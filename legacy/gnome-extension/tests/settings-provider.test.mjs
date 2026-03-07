import test from 'node:test';
import assert from 'node:assert/strict';

import {isSettingsIntent, SettingsProvider} from '../lib/providers/settings.js';

test('settings intent matcher handles direct and typo queries', () => {
    assert.equal(isSettingsIntent('settings'), true);
    assert.equal(isSettingsIntent('preferences'), true);
    assert.equal(isSettingsIntent('setings'), true);
    assert.equal(isSettingsIntent('firefox'), false);
});

test('settings provider exposes hop settings row in all mode when query is settings intent', () => {
    const provider = new SettingsProvider();
    const rows = provider.getResults('setings', 'all');
    assert.equal(rows.length > 0, true);
    assert.equal(rows[0].id, 'hop-launcher-settings');
    assert.equal(rows[0].priorityBoost > 0, true);
});

test('settings provider does not pollute unrelated all-mode queries', () => {
    const provider = new SettingsProvider();
    const rows = provider.getResults('firefox', 'all');
    assert.deepEqual(rows, []);
});

test('settings provider returns rows in explicit settings mode', () => {
    let launched = '';
    const provider = new SettingsProvider({
        launchDesktopApp: id => {
            launched = id;
        },
    });
    const rows = provider.getResults('anything', 'settings');
    assert.equal(rows.length >= 2, true);
    rows[1].execute();
    assert.equal(launched, 'org.gnome.Settings.desktop');
});
