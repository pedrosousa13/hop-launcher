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
