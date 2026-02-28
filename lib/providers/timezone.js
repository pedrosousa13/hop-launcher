import {TIMEZONE_ALIASES} from '../data/timezone-aliases.js';

function normalizeToken(value) {
    return (value ?? '')
        .toString()
        .trim()
        .toLowerCase()
        .replace(/\s+/g, '_');
}

export function shouldHandleTimezoneQuery(query) {
    const q = (query ?? '').trim().toLowerCase();
    return q.startsWith('time ') || q.startsWith('now in ') || q.startsWith('tz ');
}

function extractSearchToken(query) {
    const q = (query ?? '').trim().toLowerCase();
    if (q.startsWith('tz '))
        return normalizeToken(q.slice(3));
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

export function resolveTimezoneQuery(query) {
    if (!shouldHandleTimezoneQuery(query))
        return [];

    const token = extractSearchToken(query);
    if (!token)
        return [];

    const matches = Object.entries(TIMEZONE_ALIASES)
        .filter(([alias, iana]) => alias.includes(token) || iana.toLowerCase().includes(token.replace(/_/g, '/')))
        .map(([alias, iana]) => ({alias, iana}));

    matches.sort((a, b) => {
        const aExact = a.alias === token ? 0 : 1;
        const bExact = b.alias === token ? 0 : 1;
        if (aExact !== bExact)
            return aExact - bExact;
        return a.alias.localeCompare(b.alias);
    });

    return matches;
}

export class TimezoneProvider {
    getResults(query, mode = 'all') {
        if (mode !== 'all' && mode !== 'timezone')
            return [];

        const matches = resolveTimezoneQuery(query).slice(0, 6);
        return matches.map(match => ({
            kind: 'utility',
            id: `timezone:${match.iana}`,
            primaryText: `${match.alias.toUpperCase()} • ${formatZoneTime(match.iana)}`,
            secondaryText: match.iana,
            execute: () => {},
        }));
    }
}
