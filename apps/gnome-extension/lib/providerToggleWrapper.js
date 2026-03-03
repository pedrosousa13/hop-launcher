export function makeSettingsGatedProvider(provider, settings, key) {
    const source = provider ?? {};

    return {
        ...source,
        getResults(query, mode) {
            if (!settings?.get_boolean?.(key))
                return [];

            if (!source.getResults)
                return [];

            return source.getResults(query, mode);
        },
    };
}
