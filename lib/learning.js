const EMPTY_STORE = Object.freeze({version: 1, entries: {}});
const MAX_QUERIES = 240;
const MAX_APPS_PER_QUERY = 24;

function normalizeQuery(query) {
    return (query ?? '')
        .toString()
        .trim()
        .toLowerCase();
}

function cloneStore(store) {
    return {
        version: 1,
        entries: {...(store?.entries ?? {})},
    };
}

function pruneStore(store) {
    const queryEntries = Object.entries(store.entries);
    if (queryEntries.length > MAX_QUERIES) {
        queryEntries.sort((a, b) => {
            const aLast = Math.max(...Object.values(a[1]).map(v => v?.lastUsedMs ?? 0), 0);
            const bLast = Math.max(...Object.values(b[1]).map(v => v?.lastUsedMs ?? 0), 0);
            return bLast - aLast;
        });
        store.entries = Object.fromEntries(queryEntries.slice(0, MAX_QUERIES));
    }

    for (const [query, apps] of Object.entries(store.entries)) {
        const appEntries = Object.entries(apps);
        if (appEntries.length <= MAX_APPS_PER_QUERY)
            continue;
        appEntries.sort((a, b) => (b[1]?.lastUsedMs ?? 0) - (a[1]?.lastUsedMs ?? 0));
        store.entries[query] = Object.fromEntries(appEntries.slice(0, MAX_APPS_PER_QUERY));
    }
}

export function parseLearningStore(raw) {
    if (!raw)
        return {version: 1, entries: {}};

    try {
        const parsed = JSON.parse(raw);
        if (!parsed || typeof parsed !== 'object' || typeof parsed.entries !== 'object' || parsed.entries === null)
            return {version: 1, entries: {}};
        return cloneStore(parsed);
    } catch (_error) {
        return {version: 1, entries: {}};
    }
}

export function serializeLearningStore(store) {
    const safeStore = cloneStore(store ?? EMPTY_STORE);
    pruneStore(safeStore);
    return JSON.stringify(safeStore);
}

export function recordAppLaunch(store, query, appId, nowMs = Date.now()) {
    const normalizedQuery = normalizeQuery(query);
    const normalizedAppId = (appId ?? '').toString().trim();
    if (!normalizedQuery || !normalizedAppId)
        return cloneStore(store ?? EMPTY_STORE);

    const next = cloneStore(store ?? EMPTY_STORE);
    const queryBucket = {...(next.entries[normalizedQuery] ?? {})};
    const prev = queryBucket[normalizedAppId] ?? {count: 0, lastUsedMs: 0};
    queryBucket[normalizedAppId] = {
        count: Math.min(100000, (prev.count ?? 0) + 1),
        lastUsedMs: Number.isFinite(nowMs) ? nowMs : Date.now(),
    };
    next.entries[normalizedQuery] = queryBucket;
    pruneStore(next);
    return next;
}

function computeBoost(entry, nowMs = Date.now()) {
    const count = Math.max(0, Number(entry?.count) || 0);
    if (count <= 0)
        return 0;

    const lastUsedMs = Number(entry?.lastUsedMs) || 0;
    const ageDays = lastUsedMs > 0 ? Math.max(0, (nowMs - lastUsedMs) / (24 * 60 * 60 * 1000)) : 365;
    const recencyFactor = Math.max(0.4, 1 - ageDays / 180);
    const base = Math.log2(count + 1) * 18;
    return Math.min(85, base * recencyFactor);
}

export function applyLearningBoosts(store, query, items) {
    const normalizedQuery = normalizeQuery(query);
    const bucket = store?.entries?.[normalizedQuery];
    if (!bucket)
        return new Map();

    const boosts = new Map();
    for (const item of items) {
        if (item.kind !== 'app' || !item.id)
            continue;
        const learned = bucket[item.id];
        const boost = computeBoost(learned);
        if (boost > 0)
            boosts.set(item, boost);
    }
    return boosts;
}

