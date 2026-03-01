export function normalizeText(text) {
    return (text ?? '').toString();
}

export function toSearchable(text) {
    return normalizeText(text).toLowerCase();
}

export function computeFuzzyScore(query, candidate) {
    const q = toSearchable(query).trim();
    const cRaw = normalizeText(candidate);
    const c = cRaw.toLowerCase();
    return computeFuzzyScorePrepared(q, cRaw, c);
}

function computeFuzzyScorePrepared(queryLower, candidateRaw, candidateLower) {
    const q = queryLower;
    const cRaw = candidateRaw;
    const c = candidateLower;

    if (!q)
        return 0;
    if (!c)
        return Number.NEGATIVE_INFINITY;

    let score = 0;
    let qIndex = 0;
    let lastMatch = -2;
    let contiguousRun = 0;

    for (let i = 0; i < c.length && qIndex < q.length; i++) {
        if (c[i] !== q[qIndex])
            continue;

        score += 10;

        if (i === lastMatch + 1) {
            contiguousRun++;
            score += 8 + contiguousRun;
        } else {
            contiguousRun = 0;
        }

        if (i === 0 || /[\s_\-./]/.test(c[i - 1]))
            score += 12;

        if (i > 0 && cRaw[i] === cRaw[i].toUpperCase() && cRaw[i - 1] === cRaw[i - 1].toLowerCase())
            score += 8;

        if (i < 6)
            score += 6 - i;

        lastMatch = i;
        qIndex++;
    }

    if (qIndex !== q.length) {
        const alt = Math.max(0, score - levenshteinDistanceWithin(c, q, 1) * 14);
        return alt > 0 ? alt - 20 : Number.NEGATIVE_INFINITY;
    }

    score -= Math.max(0, c.length - q.length) * 0.15;
    return score;
}

function levenshteinDistanceWithin(haystack, needle, maxDistance) {
    if (!needle)
        return 0;

    let best = maxDistance + 1;
    const window = Math.max(needle.length + maxDistance, needle.length);

    for (let start = 0; start < haystack.length; start++) {
        const sub = haystack.slice(start, start + window);
        if (!sub)
            break;
        const d = levenshtein(sub, needle, maxDistance);
        if (d < best)
            best = d;
        if (best === 0)
            break;
    }

    return best;
}

function levenshtein(a, b, maxDistance) {
    const dp = Array.from({length: b.length + 1}, (_, i) => i);

    for (let i = 1; i <= a.length; i++) {
        let prev = i - 1;
        dp[0] = i;
        let rowMin = dp[0];

        for (let j = 1; j <= b.length; j++) {
            const tmp = dp[j];
            const cost = a[i - 1] === b[j - 1] ? 0 : 1;
            dp[j] = Math.min(dp[j] + 1, dp[j - 1] + 1, prev + cost);
            prev = tmp;
            rowMin = Math.min(rowMin, dp[j]);
        }

        if (rowMin > maxDistance)
            return maxDistance + 1;
    }

    return dp[b.length];
}

export function rankResults(query, items, options = {}) {
    const {
        weightWindows = 30,
        weightApps = 20,
        weightRecents = 10,
        weightFiles = 12,
        weightEmoji = 8,
        weightUtility = 6,
        maxResults = 10,
        minFuzzyScore = 0,
        scoreBoost = null,
    } = options;
    const limit = Number.isFinite(maxResults) ? Math.max(1, Math.floor(maxResults)) : 10;
    const minimumFuzzy = Number.isFinite(minFuzzyScore) ? minFuzzyScore : 0;

    const sourceWeight = {
        window: weightWindows,
        app: weightApps,
        recent: weightRecents,
        file: weightFiles,
        emoji: weightEmoji,
        utility: weightUtility,
        action: 25,
    };

    const normalizedQuery = toSearchable(query).trim();
    const hasQuery = normalizedQuery.length > 0;
    const scored = [];

    for (const item of items) {
        if (!item._searchHaystack) {
            item._searchHaystack = `${item.primaryText ?? ''} ${item.secondaryText ?? ''}`.trim();
            item._searchHaystackLower = item._searchHaystack.toLowerCase();
        } else if (!item._searchHaystackLower) {
            item._searchHaystackLower = item._searchHaystack.toLowerCase();
        }

        const fuzzy = hasQuery
            ? computeFuzzyScorePrepared(normalizedQuery, item._searchHaystack, item._searchHaystackLower)
            : 0;

        if (hasQuery && fuzzy < minimumFuzzy)
            continue;

        const extra = typeof scoreBoost === 'function' ? Number(scoreBoost(item)) || 0 : 0;
        const score = fuzzy + (sourceWeight[item.kind] ?? 0) + extra;
        if (!Number.isFinite(score) || score <= Number.NEGATIVE_INFINITY)
            continue;

        scored.push({...item, score});
    }

    scored
        .sort((a, b) => {
            if (b.score !== a.score)
                return b.score - a.score;
            if (a.kind !== b.kind)
                return (sourceWeight[b.kind] ?? 0) - (sourceWeight[a.kind] ?? 0);
            return (a.primaryText ?? '').localeCompare(b.primaryText ?? '');
        });

    const seen = new Set();
    const unique = [];

    for (const item of scored) {
        const textKey = `${item.primaryText ?? ''}:${item.secondaryText ?? ''}`.toLowerCase();
        const key = item.kind === 'app'
            ? `app:${textKey}`
            : `${item.kind}:${item.id ?? ''}:${textKey}`;
        if (seen.has(key))
            continue;
        seen.add(key);
        unique.push(item);
        if (unique.length >= limit)
            break;
    }

    return unique;
}
