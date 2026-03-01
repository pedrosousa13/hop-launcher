export function buildProviderFeatureMap(providers) {
    return [
        [providers.windows, 'feature-windows-enabled'],
        [providers.apps, 'feature-apps-enabled'],
        [providers.recents, 'feature-files-enabled'],
        [providers.files, 'feature-files-enabled'],
        [providers.emoji, 'feature-emoji-enabled'],
        [providers.calculator, 'feature-calculator-enabled'],
        [providers.timezone, 'feature-timezone-enabled'],
        [providers.currency, 'feature-currency-enabled'],
        [providers.weather, 'feature-weather-enabled'],
        [providers.webSearch, 'feature-web-search-enabled'],
    ];
}
