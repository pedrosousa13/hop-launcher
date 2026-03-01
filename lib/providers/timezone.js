import {findTimezoneMatches, hasExactTimezoneToken} from '../timezoneLookup.js';

const DEFAULT_FALLBACK_TTL_MS = 24 * 60 * 60 * 1000;
const DEFAULT_FALLBACK_TIMEOUT_MS = 2500;

function normalizeToken(value) {
    return (value ?? '')
        .toString()
        .trim()
        .toLowerCase()
        .replace(/\s+/g, '_');
}

function normalizeQuery(query) {
    return (query ?? '').toString().trim();
}

function withTimeout(promise, timeoutMs) {
    return Promise.race([
        promise,
        new Promise((_, reject) => setTimeout(() => reject(new Error('timezone timeout')), timeoutMs)),
    ]);
}

function formatAliasLabel(alias) {
    const normalized = normalizeToken(alias);
    if (!normalized)
        return '';

    if (/^[a-z]{2,5}$/.test(normalized))
        return normalized.toUpperCase();

    return normalized
        .split('_')
        .filter(Boolean)
        .map(part => part.charAt(0).toUpperCase() + part.slice(1))
        .join(' ');
}

export function shouldHandleTimezoneQuery(query) {
    const q = normalizeQuery(query).toLowerCase();
    if (q.startsWith('time ') || q.startsWith('time in ') || q.startsWith('now in ') || q.startsWith('tz '))
        return true;

    const token = normalizeToken(q);
    if (token.length < 2)
        return false;

    return hasExactTimezoneToken(token);
}

function extractSearchToken(query) {
    const q = normalizeQuery(query).toLowerCase();
    if (q.startsWith('tz '))
        return normalizeToken(q.slice(3));
    if (q.startsWith('time in '))
        return normalizeToken(q.slice(8));
    if (q.startsWith('time '))
        return normalizeToken(q.slice(5));
    if (q.startsWith('now in '))
        return normalizeToken(q.slice(7));
    return normalizeToken(q);
}

function formatZoneTime(iana) {
    return new Intl.DateTimeFormat('en-US', {
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
        hour12: false,
        timeZone: iana,
    }).format(new Date());
}

function buildTimezoneRow(match) {
    const label = formatAliasLabel(match.alias) || match.alias;
    const current = formatZoneTime(match.iana);
    return {
        kind: 'utility',
        id: `timezone:${match.iana}`,
        primaryText: `${label} • ${current}`,
        secondaryText: match.iana,
        copyText: `${label} • ${current}`,
        execute: () => {},
    };
}

function buildPendingRow(token) {
    return {
        kind: 'utility',
        id: `timezone-pending:${token}`,
        primaryText: `Resolving timezone for ${token.replaceAll('_', ' ')}...`,
        secondaryText: 'Local city index',
        copyText: '',
        execute: () => {},
    };
}

function buildErrorRow(token, reason = 'lookup failed') {
    return {
        kind: 'utility',
        id: `timezone-error:${token}`,
        primaryText: `Timezone unavailable for ${token.replaceAll('_', ' ')}`,
        secondaryText: `Local city index ${reason}`,
        copyText: '',
        execute: () => {},
    };
}

export function resolveTimezoneQuery(query) {
    if (!shouldHandleTimezoneQuery(query))
        return [];

    const token = extractSearchToken(query);
    if (!token)
        return [];

    return findTimezoneMatches(token);
}

export class TimezoneProvider {
    constructor(options = {}) {
        this._ttlMs = Number.isFinite(options.ttlMs) ? options.ttlMs : DEFAULT_FALLBACK_TTL_MS;
        this._timeoutMs = Number.isFinite(options.timeoutMs) ? options.timeoutMs : DEFAULT_FALLBACK_TIMEOUT_MS;
        this._fallbackCache = new Map();
        this._inflight = new Map();
        this._onUpdate = null;
        this._cityLookupLoader = options.cityLookupLoader ?? (async () => {
            const module = await import('../data/city-timezones.js');
            return module.CITY_TIMEZONE_LOOKUP ?? {};
        });
        this._cityLookupPromise = null;
    }

    _isStale(entry) {
        return !entry || (Date.now() - entry.updatedAtMs) > this._ttlMs;
    }

    setUpdateCallback(callback) {
        this._onUpdate = typeof callback === 'function' ? callback : null;
    }

    async _loadCityLookup() {
        if (!this._cityLookupPromise)
            this._cityLookupPromise = Promise.resolve(this._cityLookupLoader());
        return this._cityLookupPromise;
    }

    async _resolveCityTimezone(token) {
        const lookup = await this._loadCityLookup();
        const timezoneId = normalizeQuery(lookup?.[token]);
        if (!timezoneId)
            throw new Error('city not found');

        return {
            alias: token,
            iana: timezoneId,
        };
    }

    _startFallbackLookup(token) {
        if (this._inflight.has(token))
            return;

        const run = withTimeout(this._resolveCityTimezone(token), this._timeoutMs)
            .then(payload => {
                this._fallbackCache.set(token, {
                    payload,
                    error: null,
                    updatedAtMs: Date.now(),
                });
                this._onUpdate?.();
            })
            .catch(error => {
                const reason = normalizeQuery(error?.message) || 'lookup failed';
                this._fallbackCache.set(token, {
                    payload: null,
                    error: reason,
                    updatedAtMs: Date.now(),
                });
                this._onUpdate?.();
            })
            .finally(() => {
                this._inflight.delete(token);
            });

        this._inflight.set(token, run);
    }

    getResults(query, mode = 'all') {
        if (mode !== 'all' && mode !== 'timezone')
            return [];

        const matches = resolveTimezoneQuery(query).slice(0, 6);
        if (matches.length > 0)
            return matches.map(match => buildTimezoneRow(match));

        const timezoneIntent = mode === 'timezone' || shouldHandleTimezoneQuery(query);
        if (!timezoneIntent)
            return [];

        const token = extractSearchToken(query);
        if (!token)
            return [];

        const cached = this._fallbackCache.get(token);
        const stale = this._isStale(cached);
        if (!stale && cached) {
            if (cached.payload)
                return [buildTimezoneRow(cached.payload)];
            return [buildErrorRow(token, cached.error)];
        }

        this._startFallbackLookup(token);
        if (cached) {
            if (cached.payload)
                return [buildTimezoneRow(cached.payload)];
            return [buildErrorRow(token, cached.error)];
        }

        return [buildPendingRow(token)];
    }
}
