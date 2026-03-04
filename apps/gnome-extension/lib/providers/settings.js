import {computeFuzzyScore} from '../fuzzy.js';

function normalize(value) {
    return (value ?? '').toString().trim().toLowerCase();
}

const SETTINGS_TERMS = [
    'settings',
    'setting',
    'preferences',
    'preference',
    'prefs',
    'config',
    'configure',
    'shortcut',
    'shortcuts',
    'keybind',
    'keybinding',
    'hotkey',
];

export function isSettingsIntent(rawQuery) {
    const q = normalize(rawQuery);
    if (!q)
        return false;

    if (SETTINGS_TERMS.some(term => q.includes(term)))
        return true;

    // Typo-tolerant fallback for "settings"-like queries (e.g. "setings").
    const collapsed = q.replace(/\s+/g, '');
    return (
        computeFuzzyScore(collapsed, 'settings') >= 26 ||
        computeFuzzyScore(collapsed, 'preferences') >= 26
    );
}

export class SettingsProvider {
    constructor(options = {}) {
        this._openHopSettings = options.openHopSettings ?? (() => {});
        this._launchDesktopApp = options.launchDesktopApp ?? (() => {});
    }

    getResults(query, mode = 'all') {
        if (mode !== 'all' && mode !== 'settings')
            return [];

        if (mode === 'all' && !isSettingsIntent(query))
            return [];

        return [
            {
                kind: 'action',
                id: 'hop-launcher-settings',
                primaryText: 'Hop Launcher Settings',
                secondaryText: 'Open extension preferences, shortcuts, and provider configuration',
                priorityBoost: 140,
                execute: () => this._openHopSettings(),
            },
            {
                kind: 'action',
                id: 'gnome-settings-app',
                primaryText: 'GNOME Settings',
                secondaryText: 'Open system settings application',
                priorityBoost: 30,
                execute: () => this._launchDesktopApp('org.gnome.Settings.desktop'),
            },
            {
                kind: 'action',
                id: 'gnome-keyboard-shortcuts',
                primaryText: 'Keyboard Shortcuts',
                secondaryText: 'Open keyboard settings to configure system hotkeys',
                priorityBoost: 20,
                execute: () => this._launchDesktopApp('org.gnome.Settings.desktop'),
            },
        ];
    }
}
