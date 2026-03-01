import {
    filterEnabledSearchServices,
    parseWebSearchServices,
} from '../webSearchConfig.js';

const KEY_MAX_ACTIONS = 'web-search-max-actions';
const KEY_SERVICES = 'web-search-services-json';

function normalizeQuery(query) {
    return (query ?? '').toString().trim();
}

function parseHost(url) {
    try {
        return new URL(url).host;
    } catch (_) {
        return '';
    }
}

function defaultOpenUrl(url) {
    if (typeof globalThis !== 'undefined' && globalThis.Gio?.AppInfo)
        globalThis.Gio.AppInfo.launch_default_for_uri(url, null);
}

export class WebSearchProvider {
    constructor(settings, options = {}) {
        this._settings = settings;
        this._openUrl = options.openUrl ?? defaultOpenUrl;
    }

    getResults(query, mode = 'all') {
        if (mode !== 'all')
            return [];

        const q = normalizeQuery(query);
        if (!q)
            return [];

        const rawServices = this._settings?.get_string?.(KEY_SERVICES) ?? '[]';
        const services = filterEnabledSearchServices(parseWebSearchServices(rawServices, {
            fallbackToDefaults: false,
        }));
        const maxRows = Math.max(0, Number(this._settings?.get_int?.(KEY_MAX_ACTIONS) ?? 0) || 0);

        return services
            .slice(0, maxRows)
            .map(service => {
                const searchUrl = service.urlTemplate.replaceAll('%s', encodeURIComponent(q));
                return {
                    kind: 'action',
                    id: `web-search:${service.id}`,
                    primaryText: `Search ${service.name} for "${q}"`,
                    secondaryText: parseHost(searchUrl),
                    searchUrl,
                    execute: () => this._openUrl(searchUrl),
                };
            });
    }
}
