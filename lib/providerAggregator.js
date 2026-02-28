export function collectProviderItems(providers, query, mode, onError = null) {
    const items = [];

    for (const provider of providers) {
        try {
            const rows = provider.getResults(query, mode);
            if (Array.isArray(rows))
                items.push(...rows);
        } catch (error) {
            if (onError)
                onError(error, provider);
        }
    }

    return items;
}
