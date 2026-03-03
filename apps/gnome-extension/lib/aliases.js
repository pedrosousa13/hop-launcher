const DIRECT_ALIAS_BOOST = 180;

function normalizeToken(value) {
    return (value ?? '')
        .toString()
        .trim()
        .toLowerCase();
}

function parseAliasRecord(raw) {
    const alias = normalizeToken(raw?.alias);
    const type = normalizeToken(raw?.type);
    if (!alias || /\s/.test(alias))
        return null;

    if (type === 'rewrite') {
        const query = (raw?.target?.query ?? '').toString().trim();
        if (!query)
            return null;
        return {alias, type, target: {query}};
    }

    if (type === 'app') {
        const appId = (raw?.target?.appId ?? '').toString().trim();
        if (!appId)
            return null;
        return {alias, type, target: {appId}};
    }

    if (type === 'window') {
        const appId = (raw?.target?.appId ?? '').toString().trim();
        const titleContains = normalizeToken(raw?.target?.titleContains);
        if (!appId && !titleContains)
            return null;
        return {alias, type, target: {appId, titleContains}};
    }

    return null;
}

export function parseAliasesConfig(raw) {
    if (!raw)
        return [];

    try {
        const parsed = JSON.parse(raw);
        if (!Array.isArray(parsed))
            return [];
        return parsed
            .map(parseAliasRecord)
            .filter(Boolean);
    } catch (_error) {
        return [];
    }
}

function windowAliasMatches(item, target) {
    if (item.kind !== 'window')
        return false;

    const appId = (item.windowAppId ?? '').toString();
    const title = normalizeToken(item.primaryText);

    if (target.appId && appId !== target.appId)
        return false;
    if (target.titleContains && !title.includes(target.titleContains))
        return false;
    return true;
}

export function buildAliasContext(query, aliases, items) {
    const aliasKey = normalizeToken(query);
    const exactMatches = aliases.filter(rule => rule.alias === aliasKey);
    const rewrite = exactMatches.find(rule => rule.type === 'rewrite');
    const rankingQuery = rewrite?.target?.query ?? query;
    const boosts = new Map();

    for (const rule of exactMatches) {
        if (rule.type === 'app') {
            for (const item of items) {
                if (item.kind === 'app' && item.id === rule.target.appId)
                    boosts.set(item, (boosts.get(item) ?? 0) + DIRECT_ALIAS_BOOST);
            }
            continue;
        }

        if (rule.type === 'window') {
            for (const item of items) {
                if (windowAliasMatches(item, rule.target))
                    boosts.set(item, (boosts.get(item) ?? 0) + DIRECT_ALIAS_BOOST);
            }
        }
    }

    return {rankingQuery, boosts};
}

