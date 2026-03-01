const QUERY_PATTERN = /^(\d+(?:\.\d+)?)\s*([a-z]{3})\s+to\s+([a-z]{3})$/i;

const DEFAULT_RATES = {
    USD: 1,
    EUR: 0.92,
    GBP: 0.79,
    JPY: 150.5,
    CAD: 1.36,
    AUD: 1.53,
    BRL: 4.98,
    CHF: 0.88,
};

export function parseCurrencyQuery(query) {
    const match = (query ?? '').trim().match(QUERY_PATTERN);
    if (!match)
        return null;

    return {
        amount: Number(match[1]),
        from: match[2].toUpperCase(),
        to: match[3].toUpperCase(),
    };
}

export function convertWithRates(amount, from, to, rates) {
    const fromRate = rates[from];
    const toRate = rates[to];
    if (!Number.isFinite(fromRate) || !Number.isFinite(toRate) || fromRate <= 0)
        return null;

    const usd = amount / fromRate;
    const converted = usd * toRate;
    return Math.round(converted * 100) / 100;
}

export function isRatesCacheStale(lastUpdatedMs, ttlHours) {
    if (!Number.isFinite(lastUpdatedMs))
        return true;
    const ttlMs = ttlHours * 60 * 60 * 1000;
    return Date.now() - lastUpdatedMs > ttlMs;
}

function formatTimestamp(epochMs) {
    return new Date(epochMs).toLocaleString();
}

export class CurrencyProvider {
    constructor(settings = null) {
        this._settings = settings;
        this._cache = {
            rates: {...DEFAULT_RATES},
            lastUpdatedMs: Date.now(),
        };
    }

    refresh() {
        // Offline-first v1: keep local cached rates.
        // Future improvement can refresh from network when enabled.
    }

    getResults(query, mode = 'all') {
        if (mode !== 'all' && mode !== 'currency')
            return [];

        const parsed = parseCurrencyQuery(query);
        if (!parsed)
            return [];

        const ttlHours = this._settings?.get_int?.('currency-rate-ttl-hours') ?? 12;
        const stale = isRatesCacheStale(this._cache.lastUpdatedMs, ttlHours);
        const value = convertWithRates(parsed.amount, parsed.from, parsed.to, this._cache.rates);

        if (value === null)
            return [];

        return [{
            kind: 'utility',
            id: `currency:${parsed.amount}:${parsed.from}:${parsed.to}`,
            primaryText: `${value} ${parsed.to}`,
            secondaryText: `${parsed.amount} ${parsed.from} • Rates updated ${formatTimestamp(this._cache.lastUpdatedMs)}${stale ? ' (stale)' : ''}`,
            copyText: `${value} ${parsed.to}`,
            execute: () => {},
        }];
    }
}
