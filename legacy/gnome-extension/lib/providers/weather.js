const WEATHER_QUERY_PREFIXES = ['weather ', 'wx '];
const OPEN_METEO_SOURCE = 'Open-Meteo';
const OPEN_METEO_GEOCODE_URL = 'https://geocoding-api.open-meteo.com/v1/search';
const OPEN_METEO_FORECAST_URL = 'https://api.open-meteo.com/v1/forecast';

const DEFAULT_TTL_MS = 10 * 60 * 1000;
const DEFAULT_TIMEOUT_MS = 4000;

function normalize(text) {
    return (text ?? '').toString().trim();
}

function normalizeKey(text) {
    return normalize(text).toLowerCase().replace(/\s+/g, ' ');
}

function normalizeLocation(text) {
    const location = normalize(text);
    return location.replace(/^in\s+/i, '').trim();
}

function runWithTimeout(run, timeoutMs) {
    return new Promise((resolve, reject) => {
        const timeoutId = setTimeout(() => reject(new Error('weather timeout')), timeoutMs);
        Promise.resolve()
            .then(() => run())
            .then(value => {
                clearTimeout(timeoutId);
                resolve(value);
            })
            .catch(error => {
                clearTimeout(timeoutId);
                reject(error);
            });
    });
}

function weatherCodeToCondition(code) {
    if (code === 0)
        return {condition: 'Clear', icon: '☀'};
    if ([1, 2].includes(code))
        return {condition: 'Partly cloudy', icon: '⛅'};
    if (code === 3)
        return {condition: 'Overcast', icon: '☁'};
    if ([45, 48].includes(code))
        return {condition: 'Fog', icon: '🌫'};
    if ([51, 53, 55, 56, 57].includes(code))
        return {condition: 'Drizzle', icon: '🌦'};
    if ([61, 63, 65, 66, 67, 80, 81, 82].includes(code))
        return {condition: 'Rain', icon: '🌧'};
    if ([71, 73, 75, 77, 85, 86].includes(code))
        return {condition: 'Snow', icon: '❄'};
    if ([95, 96, 99].includes(code))
        return {condition: 'Thunderstorm', icon: '⛈'};
    return {condition: 'Weather', icon: '🌡'};
}

function buildPlaceLabel(geoResult, fallbackLocation) {
    const segments = [
        normalize(geoResult?.name),
        normalize(geoResult?.admin1),
        normalize(geoResult?.country_code),
    ].filter(Boolean);
    return segments.length > 0 ? segments.join(', ') : fallbackLocation;
}

function buildWeatherRow(payload, key, stale = false) {
    const prettyText = payload.prettyText || `Weather for ${payload.location}`;
    const updated = payload.observedAt || new Date().toLocaleTimeString();
    const staleHint = stale ? ' (stale)' : '';
    const secondary = `${OPEN_METEO_SOURCE} • Updated ${updated}${staleHint}`;

    return {
        kind: 'utility',
        id: `weather:${key}`,
        primaryText: prettyText,
        secondaryText: secondary,
        copyText: prettyText,
        execute: () => {},
    };
}

function buildPendingRow(location, key) {
    return {
        kind: 'utility',
        id: `weather-pending:${key}`,
        primaryText: `Fetching weather for ${location}...`,
        secondaryText: OPEN_METEO_SOURCE,
        copyText: '',
        execute: () => {},
    };
}

function buildErrorRow(location, key, stale = false, reason = 'request failed') {
    const staleHint = stale ? ' (retrying...)' : '';
    return {
        kind: 'utility',
        id: `weather-error:${key}`,
        primaryText: `Weather unavailable for ${location}`,
        secondaryText: `${OPEN_METEO_SOURCE} ${reason}${staleHint}`,
        copyText: '',
        execute: () => {},
    };
}

export function shouldHandleWeatherQuery(query) {
    const q = normalize(query).toLowerCase();
    if (WEATHER_QUERY_PREFIXES.some(prefix => q.startsWith(prefix)))
        return true;
    if (q.endsWith(' weather')) {
        const location = normalize(q.slice(0, -8));
        return location.length >= 2;
    }
    return false;
}

export function parseWeatherQuery(query, {allowBare = false} = {}) {
    const raw = normalize(query);
    if (!raw)
        return null;

    const lower = raw.toLowerCase();
    for (const prefix of WEATHER_QUERY_PREFIXES) {
        if (lower.startsWith(prefix)) {
            const location = normalizeLocation(raw.slice(prefix.length));
            return location ? {location} : null;
        }
    }

    if (lower.endsWith(' weather')) {
        const location = normalizeLocation(raw.slice(0, -8));
        return location ? {location} : null;
    }

    if (allowBare) {
        const location = normalizeLocation(raw);
        return location ? {location} : null;
    }

    return null;
}

function createDefaultRequestJson(options = {}) {
    const requestJson = typeof options.requestJson === 'function'
        ? options.requestJson
        : typeof options.soupRequestJson === 'function'
            ? options.soupRequestJson
            : null;

    if (!requestJson) {
        return async () => {
            throw new Error('weather transport unavailable');
        };
    }

    return async url => {
        return requestJson(url);
    };
}

export class WeatherProvider {
    constructor(options = {}) {
        this._requestJson = createDefaultRequestJson(options);
        this._ttlMs = Number.isFinite(options.ttlMs) ? options.ttlMs : DEFAULT_TTL_MS;
        this._timeoutMs = Number.isFinite(options.timeoutMs) ? options.timeoutMs : DEFAULT_TIMEOUT_MS;
        this._cache = new Map();
        this._inflight = new Map();
        this._onUpdate = null;
        this._destroyed = false;
    }

    _isStale(entry) {
        return !entry || (Date.now() - entry.updatedAtMs) > this._ttlMs;
    }

    setUpdateCallback(callback) {
        this._onUpdate = typeof callback === 'function' ? callback : null;
    }

    _startFetch(location, key) {
        if (this._destroyed || this._inflight.has(key))
            return;

        const run = runWithTimeout(() => this._fetchWeather(location), this._timeoutMs)
            .then(payload => {
                if (this._destroyed)
                    return;
                this._cache.set(key, {
                    payload,
                    error: null,
                    updatedAtMs: Date.now(),
                });
                this._onUpdate?.();
            })
            .catch(error => {
                if (this._destroyed)
                    return;
                const reason = normalize(error?.message) || 'request failed';
                this._cache.set(key, {
                    payload: null,
                    error: reason,
                    updatedAtMs: Date.now(),
                });
                this._onUpdate?.();
            })
            .finally(() => {
                this._inflight.delete(key);
            });

        this._inflight.set(key, run);
    }

    async _fetchWeather(location) {
        if (this._destroyed)
            throw new Error('weather canceled');

        const geocodeUrl = `${OPEN_METEO_GEOCODE_URL}?name=${encodeURIComponent(location)}&count=1&language=en&format=json`;
        const geocode = await this._requestJson(geocodeUrl);
        if (this._destroyed)
            throw new Error('weather canceled');

        const geoResult = geocode?.results?.[0];
        if (!geoResult)
            throw new Error('location not found');

        const latitude = Number(geoResult.latitude);
        const longitude = Number(geoResult.longitude);
        if (!Number.isFinite(latitude) || !Number.isFinite(longitude))
            throw new Error('weather parse miss');

        const forecastUrl = `${OPEN_METEO_FORECAST_URL}?latitude=${latitude}&longitude=${longitude}&current=temperature_2m,weather_code,wind_speed_10m&temperature_unit=celsius&wind_speed_unit=kmh`;
        const forecast = await this._requestJson(forecastUrl);
        if (this._destroyed)
            throw new Error('weather canceled');

        const current = forecast?.current;
        if (!current)
            throw new Error('weather parse miss');

        const temperature = Number(current.temperature_2m);
        const weatherCode = Number(current.weather_code);
        const windSpeed = Number(current.wind_speed_10m);
        if (!Number.isFinite(temperature) || !Number.isFinite(weatherCode) || !Number.isFinite(windSpeed))
            throw new Error('weather parse miss');

        const {condition, icon} = weatherCodeToCondition(weatherCode);
        const placeLabel = buildPlaceLabel(geoResult, location);
        const line = `${placeLabel}: ${condition} ${icon} ${Math.round(temperature)}C Wind ${Math.round(windSpeed)} km/h`;

        return {
            location,
            prettyText: line,
            observedAt: new Date().toLocaleTimeString(),
        };
    }

    getResults(query, mode = 'all') {
        if (this._destroyed || (mode !== 'all' && mode !== 'weather'))
            return [];

        const parsed = mode === 'weather'
            ? parseWeatherQuery(query, {allowBare: true})
            : parseWeatherQuery(query);
        if (!parsed)
            return [];

        const key = normalizeKey(parsed.location);
        if (!key)
            return [];

        const cached = this._cache.get(key);
        const stale = this._isStale(cached);
        if (!stale && cached) {
            if (cached.payload)
                return [buildWeatherRow(cached.payload, key, false)];
            return [buildErrorRow(parsed.location, key, false, cached.error)];
        }

        this._startFetch(parsed.location, key);

        if (cached) {
            if (cached.payload)
                return [buildWeatherRow(cached.payload, key, true)];
            return [buildErrorRow(parsed.location, key, true, cached.error)];
        }

        return [buildPendingRow(parsed.location, key)];
    }

    destroy() {
        this._destroyed = true;
        this._onUpdate = null;
        this._inflight.clear();
    }
}

export {
    buildWeatherRow,
    OPEN_METEO_SOURCE,
};
