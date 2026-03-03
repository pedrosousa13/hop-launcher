import {hasExactTimezoneToken} from './timezoneLookup.js';

function looksLikeMath(query) {
    const q = query.trim();
    if (!q)
        return false;
    if (!/[0-9]/.test(q))
        return false;
    return /^[0-9+\-*/().\s%]+$/.test(q);
}

function looksLikeCurrency(query) {
    const q = query.trim().toLowerCase();
    return /^\d+(\.\d+)?\s*[a-z]{3}\s+to\s+[a-z]{3}$/.test(q);
}

function looksLikeTimezone(query) {
    const q = query.trim().toLowerCase();
    if (q.startsWith('time ') || q.startsWith('time in ') || q.startsWith('now in ') || q.startsWith('timezone '))
        return true;
    if (q.endsWith(' time')) {
        const token = q.slice(0, -5).trim().replace(/\s+/g, '_');
        return token.length >= 2 && hasExactTimezoneToken(token);
    }

    const token = q.replace(/\s+/g, '_');
    if (token.length < 2)
        return false;

    return hasExactTimezoneToken(token);
}

export function extractQueryRoute(rawQuery) {
    const q = (rawQuery ?? '').trimStart();
    const qLower = q.toLowerCase();

    if (qLower.startsWith('w '))
        return {mode: 'windows', query: q.slice(2)};
    if (qLower.startsWith('a '))
        return {mode: 'apps', query: q.slice(2)};
    if (qLower.startsWith('f '))
        return {mode: 'files', query: q.slice(2)};
    if (qLower.startsWith(':emoji '))
        return {mode: 'emoji', query: q.slice(7)};
    if (qLower.startsWith('emoji '))
        return {mode: 'emoji', query: q.slice(6)};
    if (qLower.startsWith('tz '))
        return {mode: 'timezone', query: q.slice(3)};
    if (qLower.startsWith('timezone '))
        return {mode: 'timezone', query: q.slice(9)};
    if (qLower.startsWith('weather '))
        return {mode: 'weather', query: q.slice(8)};
    if (qLower.startsWith('wx '))
        return {mode: 'weather', query: q.slice(3)};
    if (qLower.endsWith(' weather'))
        return {mode: 'weather', query: q.slice(0, -8).trim()};
    if (q.startsWith('$'))
        return {mode: 'currency', query: q.slice(1).trimStart()};
    if (q.startsWith('='))
        return {mode: 'calculator', query: q.slice(1).trimStart()};
    if (q.startsWith('>'))
        return {mode: 'actions', query: q.slice(1)};

    if (looksLikeMath(q))
        return {mode: 'calculator', query: q};
    if (looksLikeCurrency(q))
        return {mode: 'currency', query: q};
    if (looksLikeTimezone(q)) {
        if (qLower.startsWith('time in '))
            return {mode: 'timezone', query: q.slice(8)};
        if (qLower.startsWith('time '))
            return {mode: 'timezone', query: q.slice(5)};
        if (qLower.startsWith('now in '))
            return {mode: 'timezone', query: q.slice(7)};
        if (qLower.startsWith('timezone '))
            return {mode: 'timezone', query: q.slice(9)};
        if (qLower.endsWith(' time'))
            return {mode: 'timezone', query: q.slice(0, -5).trim()};
        return {mode: 'timezone', query: q};
    }

    return {mode: 'all', query: q};
}
