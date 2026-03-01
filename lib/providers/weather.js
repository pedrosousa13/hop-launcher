const WEATHER_QUERY_PREFIXES = ['weather ', 'wx '];
const WTTR_FORMAT = '3';

const DEFAULT_TTL_MS = 10 * 60 * 1000;
const DEFAULT_TIMEOUT_MS = 7000;

function normalize(text) {
    return (text ?? '').toString().trim();
}

function normalizeKey(text) {
    return normalize(text).toLowerCase().replace(/\s+/g, ' ');
}

function withTimeout(promise, timeoutMs) {
    return Promise.race([
        promise,
        new Promise((_, reject) => setTimeout(() => reject(new Error('weather timeout')), timeoutMs)),
    ]);
}

function buildWeatherRow(payload, key, stale = false) {
    const prettyText = payload.prettyText || `Weather for ${payload.location}`;
    const updated = payload.observedAt || new Date().toLocaleTimeString();
    const staleHint = stale ? ' (stale)' : '';
    const secondary = `wttr.in • Updated ${updated}${staleHint}`;

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
        secondaryText: 'wttr.in',
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
        secondaryText: `wttr.in ${reason}${staleHint}`,
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
            const location = normalize(raw.slice(prefix.length));
            return location ? {location} : null;
        }
    }

    if (lower.endsWith(' weather')) {
        const location = normalize(raw.slice(0, -8));
        return location ? {location} : null;
    }

    if (allowBare)
        return {location: raw};

    return null;
}

async function defaultRequestText(url) {
    if (typeof fetch !== 'function')
        throw new Error('fetch unavailable');
    const response = await fetch(url);
    if (!response.ok)
        throw new Error(`weather http ${response.status}`);
    return response.text();
}

export class WeatherProvider {
    constructor(options = {}) {
        this._requestText = options.requestText ?? defaultRequestText;
        this._ttlMs = Number.isFinite(options.ttlMs) ? options.ttlMs : DEFAULT_TTL_MS;
        this._timeoutMs = Number.isFinite(options.timeoutMs) ? options.timeoutMs : DEFAULT_TIMEOUT_MS;
        this._cache = new Map();
        this._inflight = new Map();
        this._onUpdate = null;
    }

    _isStale(entry) {
        return !entry || (Date.now() - entry.updatedAtMs) > this._ttlMs;
    }

    setUpdateCallback(callback) {
        this._onUpdate = typeof callback === 'function' ? callback : null;
    }

    _startFetch(location, key) {
        if (this._inflight.has(key))
            return;

        const run = withTimeout(this._fetchWeather(location), this._timeoutMs)
            .then(payload => {
                this._cache.set(key, {
                    payload,
                    error: null,
                    updatedAtMs: Date.now(),
                });
                this._onUpdate?.();
            })
            .catch(error => {
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
        const url = `https://wttr.in/${encodeURIComponent(location)}?format=${encodeURIComponent(WTTR_FORMAT)}`;
        const rawText = await this._requestText(url);
        const line = normalize(rawText).split('\n')[0];
        if (!line || !/\S/.test(line))
            throw new Error('weather parse miss');
        return {
            location,
            prettyText: line,
            observedAt: new Date().toLocaleTimeString(),
        };
    }

    getResults(query, mode = 'all') {
        if (mode !== 'all' && mode !== 'weather')
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
}

export {
    buildWeatherRow,
    WTTR_FORMAT,
};
