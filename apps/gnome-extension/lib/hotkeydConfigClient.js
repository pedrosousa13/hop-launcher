export function createHotkeydConfigClient({runJsonCommand}) {
    if (typeof runJsonCommand !== 'function')
        throw new Error('runJsonCommand is required');

    return {
        getShortcut() {
            const payload = runJsonCommand(['config', 'get']);
            const shortcut = (payload?.shortcut ?? '').toString().trim();
            if (!shortcut)
                throw new Error('hotkeyd config get returned empty shortcut');
            return shortcut;
        },

        setShortcut(shortcut) {
            const value = (shortcut ?? '').toString().trim();
            if (!value)
                throw new Error('shortcut must not be empty');

            const payload = runJsonCommand(['config', 'set', '--shortcut', value]);
            return {
                shortcut: (payload?.shortcut ?? value).toString().trim(),
                applied: payload?.applied !== false,
                requiresManualStep: payload?.requires_manual_step === true,
                warnings: Array.isArray(payload?.warnings) ? payload.warnings : [],
            };
        },
    };
}
