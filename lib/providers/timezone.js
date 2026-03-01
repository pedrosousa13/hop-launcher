import {findTimezoneMatches, hasExactTimezoneToken} from '../timezoneLookup.js';

function normalizeToken(value) {
    return (value ?? '')
        .toString()
        .trim()
        .toLowerCase()
        .replace(/\s+/g, '_');
}

export function shouldHandleTimezoneQuery(query) {
    const q = (query ?? '').trim().toLowerCase();
    if (q.startsWith('time ') || q.startsWith('time in ') || q.startsWith('now in ') || q.startsWith('tz '))
        return true;

    const token = normalizeToken(q);
    if (token.length < 2)
        return false;

    return hasExactTimezoneToken(token);
}

function extractSearchToken(query) {
    const q = (query ?? '').trim().toLowerCase();
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

export function resolveTimezoneQuery(query) {
    if (!shouldHandleTimezoneQuery(query))
        return [];

    const token = extractSearchToken(query);
    if (!token)
        return [];

    return findTimezoneMatches(token);
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
            copyText: `${match.alias.toUpperCase()} • ${formatZoneTime(match.iana)}`,
            execute: () => {},
        }));
    }
}
