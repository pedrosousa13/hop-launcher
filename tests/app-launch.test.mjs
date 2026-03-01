import test from 'node:test';
import assert from 'node:assert/strict';

import {launchOrFocusApp} from '../lib/providers/appLaunch.js';

test('launchOrFocusApp prefers focusing an existing normal window', () => {
    let activatedWith = null;
    let launched = false;
    const now = () => 42;

    const app = {
        get_windows: () => [{
            skip_taskbar: false,
            minimized: false,
            is_override_redirect: () => false,
            activate: timestamp => {
                activatedWith = timestamp;
            },
        }],
        open_new_window: () => {
            launched = true;
        },
    };

    assert.equal(launchOrFocusApp(app, now), true);
    assert.equal(activatedWith, 42);
    assert.equal(launched, false);
});

test('launchOrFocusApp restores and focuses a minimized existing window', () => {
    let activatedWith = null;
    let unminimizedWith = null;
    let launched = false;
    const now = () => 99;

    const app = {
        get_windows: () => [{
            skip_taskbar: false,
            minimized: true,
            is_override_redirect: () => false,
            unminimize: timestamp => {
                unminimizedWith = timestamp;
            },
            activate: timestamp => {
                activatedWith = timestamp;
            },
        }],
        open_new_window: () => {
            launched = true;
        },
    };

    assert.equal(launchOrFocusApp(app, now), true);
    assert.equal(unminimizedWith, 99);
    assert.equal(activatedWith, 99);
    assert.equal(launched, false);
});

test('launchOrFocusApp focuses existing window when skip_taskbar is a method returning false', () => {
    let activatedWith = null;
    let launched = false;
    const now = () => 123;

    const app = {
        get_windows: () => [{
            skip_taskbar: () => false,
            minimized: false,
            is_override_redirect: () => false,
            activate: timestamp => {
                activatedWith = timestamp;
            },
        }],
        open_new_window: () => {
            launched = true;
        },
    };

    assert.equal(launchOrFocusApp(app, now), true);
    assert.equal(activatedWith, 123);
    assert.equal(launched, false);
});

test('launchOrFocusApp falls back to open_new_window when activate is unavailable', () => {
    let launched = false;
    const app = {
        get_windows: () => [],
        open_new_window: workspace => {
            launched = workspace === -1;
        },
    };

    assert.equal(launchOrFocusApp(app), true);
    assert.equal(launched, true);
});

test('launchOrFocusApp uses appInfo.launch as last fallback', () => {
    let launched = false;
    const app = {
        get_windows: () => [],
        get_app_info: () => ({
            launch: (files, context) => {
                launched = Array.isArray(files) && context === null;
            },
        }),
    };

    assert.equal(launchOrFocusApp(app), true);
    assert.equal(launched, true);
});

test('launchOrFocusApp uses app.launch when app is a DesktopAppInfo-like object', () => {
    let launched = false;
    const app = {
        get_windows: () => [],
        launch: (files, context) => {
            launched = Array.isArray(files) && context === null;
        },
    };

    assert.equal(launchOrFocusApp(app), true);
    assert.equal(launched, true);
});

test('launchOrFocusApp focuses matching open window when app.get_windows is empty', () => {
    const originalDisplay = global.display;
    let activatedWith = null;
    let appActivated = false;
    const now = () => 777;

    global.display = {
        get_tab_list: () => [{
            skip_taskbar: false,
            minimized: false,
            is_override_redirect: () => false,
            get_gtk_application_id: () => 'brave-browser',
            activate: timestamp => {
                activatedWith = timestamp;
            },
        }],
    };

    const app = {
        get_id: () => 'brave-browser.desktop',
        get_windows: () => [],
        activate: () => {
            appActivated = true;
        },
    };

    assert.equal(launchOrFocusApp(app, now), true);
    assert.equal(activatedWith, 777);
    assert.equal(appActivated, false);

    global.display = originalDisplay;
});

test('launchOrFocusApp does not throw when window flag methods require bound this', () => {
    const originalDisplay = global.display;
    let appActivated = false;

    const window = {
        _skipTaskbar: false,
        is_skip_taskbar() {
            if (this !== window)
                throw new TypeError('unbound method call');
            return this._skipTaskbar;
        },
        is_override_redirect() {
            if (this !== window)
                throw new TypeError('unbound method call');
            return false;
        },
    };

    global.display = {
        get_tab_list: () => [window],
    };

    const app = {
        get_id: () => 'org.test.app.desktop',
        get_windows: () => [],
        activate: () => {
            appActivated = true;
        },
    };

    assert.doesNotThrow(() => launchOrFocusApp(app, () => 1));
    assert.equal(appActivated, true);

    global.display = originalDisplay;
});
