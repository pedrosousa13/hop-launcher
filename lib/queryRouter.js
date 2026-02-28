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
    return /^\d+(\.\d+)?\s+[a-z]{3}\s+to\s+[a-z]{3}$/.test(q);
}

function looksLikeTimezone(query) {
    const q = query.trim().toLowerCase();
    return q.startsWith('time ') || q.startsWith('now in ');
}

export function extractQueryRoute(rawQuery) {
    const q = (rawQuery ?? '').trimStart();

    if (q.startsWith('w '))
        return {mode: 'windows', query: q.slice(2)};
    if (q.startsWith('a '))
        return {mode: 'apps', query: q.slice(2)};
    if (q.startsWith('f '))
        return {mode: 'files', query: q.slice(2)};
    if (q.startsWith(':emoji '))
        return {mode: 'emoji', query: q.slice(7)};
    if (q.startsWith('tz '))
        return {mode: 'timezone', query: q.slice(3)};
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
        if (q.startsWith('time '))
            return {mode: 'timezone', query: q.slice(5)};
        if (q.startsWith('now in '))
            return {mode: 'timezone', query: q.slice(7)};
    }

    return {mode: 'all', query: q};
}
