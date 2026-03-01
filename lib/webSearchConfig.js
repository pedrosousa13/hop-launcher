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

function parseHttpsHostFallback(candidate) {
    const match = (candidate ?? '').toString().match(/^https:\/\/([^/\s?#]+)(?:[/?#]|$)/i);
    return match?.[1] ?? '';
}

export function validateWebSearchService(row) {
    const name = normalizeString(row?.name);
    const urlTemplate = normalizeString(row?.urlTemplate ?? row?.url ?? row?.template);
    const enabled = row?.enabled !== false;
    const id = normalizeString(row?.id) || normalizeIdFromName(name);
    const keyword = normalizeString(row?.keyword);

    if (!name)
        return {valid: false, reason: 'name-missing'};

    if (!urlTemplate || !urlTemplate.includes('%s'))
        return {valid: false, reason: 'template-missing-placeholder'};

    const candidate = urlTemplate.replace('%s', 'query');
    if (/^http:\/\//i.test(candidate))
        return {valid: false, reason: 'template-non-https'};

    if (typeof URL === 'function') {
        let parsed;
        try {
            parsed = new URL(candidate);
        } catch (_) {
            return {valid: false, reason: 'template-invalid-url'};
        }

        if (parsed.protocol !== 'https:')
            return {valid: false, reason: 'template-non-https'};

        if (!parsed.host)
            return {valid: false, reason: 'template-invalid-url'};
    } else if (!parseHttpsHostFallback(candidate)) {
        return {valid: false, reason: 'template-invalid-url'};
    }

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

export function serializeWebSearchServices(rows, options = {}) {
    const fallbackToDefaults = options.fallbackToDefaults !== false;
    const nextRows = Array.isArray(rows) ? rows : [];
    const valid = [];

    for (const row of nextRows) {
        const out = validateWebSearchService(row);
        if (out.valid)
            valid.push(out.value);
    }

    if (valid.length > 0)
        return JSON.stringify(valid);

    if (fallbackToDefaults)
        return JSON.stringify(DEFAULT_WEB_SEARCH_SERVICES);

    return '[]';
}

export function addWebSearchProvider(currentRows, nextRow) {
    const existing = Array.isArray(currentRows) ? currentRows : [];
    const out = validateWebSearchService(nextRow);
    if (!out.valid)
        return [...existing];
    return [...existing, out.value];
}

export function filterEnabledSearchServices(services) {
    if (!Array.isArray(services))
        return [];
    return services.filter(service => service && service.enabled !== false);
}
