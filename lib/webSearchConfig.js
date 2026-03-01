export const DEFAULT_WEB_SEARCH_SERVICES = [
    {
        id: 'google',
        name: 'Google',
        urlTemplate: 'https://www.google.com/search?q=%s',
        enabled: true,
        keyword: 'g',
    },
    {
        id: 'duckduckgo',
        name: 'DuckDuckGo',
        urlTemplate: 'https://duckduckgo.com/?q=%s',
        enabled: true,
        keyword: 'ddg',
    },
];

function normalizeString(value) {
    return (value ?? '').toString().trim();
}

function normalizeIdFromName(name) {
    const raw = normalizeString(name).toLowerCase();
    const normalized = raw.replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
    return normalized || `service-${Math.random().toString(36).slice(2, 8)}`;
}

export function validateWebSearchService(row) {
    const name = normalizeString(row?.name);
    const urlTemplate = normalizeString(row?.urlTemplate);
    const enabled = row?.enabled !== false;
    const id = normalizeString(row?.id) || normalizeIdFromName(name);
    const keyword = normalizeString(row?.keyword);

    if (!name)
        return {valid: false, reason: 'name-missing'};

    if (!urlTemplate || !urlTemplate.includes('%s'))
        return {valid: false, reason: 'template-missing-placeholder'};

    let parsed;
    try {
        parsed = new URL(urlTemplate.replace('%s', 'query'));
    } catch (_) {
        return {valid: false, reason: 'template-invalid-url'};
    }

    if (parsed.protocol !== 'https:')
        return {valid: false, reason: 'template-non-https'};

    return {
        valid: true,
        value: {
            id,
            name,
            urlTemplate,
            enabled,
            keyword,
        },
    };
}

export function parseWebSearchServices(json, options = {}) {
    const fallbackToDefaults = options.fallbackToDefaults !== false;
    let rows;

    try {
        rows = JSON.parse((json ?? '').toString());
    } catch (_) {
        return fallbackToDefaults ? [...DEFAULT_WEB_SEARCH_SERVICES] : [];
    }

    if (!Array.isArray(rows))
        return fallbackToDefaults ? [...DEFAULT_WEB_SEARCH_SERVICES] : [];

    const valid = [];
    for (const row of rows) {
        const out = validateWebSearchService(row);
        if (out.valid)
            valid.push(out.value);
    }

    if (valid.length > 0)
        return valid;

    return fallbackToDefaults ? [...DEFAULT_WEB_SEARCH_SERVICES] : [];
}

export function filterEnabledSearchServices(services) {
    if (!Array.isArray(services))
        return [];
    return services.filter(service => service && service.enabled !== false);
}
