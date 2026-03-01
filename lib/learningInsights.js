import {parseLearningStore} from './learning.js';

function normalizeLimit(value) {
    if (!Number.isFinite(value))
        return 10;
    return Math.min(50, Math.max(1, Math.trunc(value)));
}

function normalizeSort(value) {
    return value === 'recent' ? 'recent' : 'count';
}

export function buildLearningInsights(rawStore, {limit = 10, sort = 'count'} = {}) {
    const store = typeof rawStore === 'string'
        ? parseLearningStore(rawStore)
        : parseLearningStore('');

    const rows = [];
    for (const [query, apps] of Object.entries(store.entries ?? {})) {
        for (const [appId, entry] of Object.entries(apps ?? {})) {
            rows.push({
                query,
                appId,
                count: Math.max(0, Number(entry?.count) || 0),
                lastUsedMs: Math.max(0, Number(entry?.lastUsedMs) || 0),
            });
        }
    }

    const normalizedSort = normalizeSort(sort);
    if (normalizedSort === 'recent') {
        rows.sort((a, b) =>
            (b.lastUsedMs - a.lastUsedMs) ||
            (b.count - a.count) ||
            a.query.localeCompare(b.query) ||
            a.appId.localeCompare(b.appId));
    } else {
        rows.sort((a, b) =>
            (b.count - a.count) ||
            (b.lastUsedMs - a.lastUsedMs) ||
            a.query.localeCompare(b.query) ||
            a.appId.localeCompare(b.appId));
    }

    return rows.slice(0, normalizeLimit(limit));
}
