import test from 'node:test';
import assert from 'node:assert/strict';

import {
    applyLearningBoosts,
    parseLearningStore,
    recordAppLaunch,
} from '../lib/learning.js';

test('parseLearningStore falls back to empty store for invalid JSON', () => {
    const store = parseLearningStore('{broken');
    assert.deepEqual(store, {version: 1, entries: {}});
});

test('recordAppLaunch increments app count for a query', () => {
    const store = parseLearningStore('');
    const updated = recordAppLaunch(store, 'term', 'org.gnome.Terminal.desktop', 10);
    const again = recordAppLaunch(updated, 'term', 'org.gnome.Terminal.desktop', 20);

    assert.equal(again.entries.term['org.gnome.Terminal.desktop'].count, 2);
    assert.equal(again.entries.term['org.gnome.Terminal.desktop'].lastUsedMs, 20);
});

test('applyLearningBoosts prefers frequently launched app for query', () => {
    let store = parseLearningStore('');
    for (let i = 0; i < 8; i++)
        store = recordAppLaunch(store, 'chr', 'google-chrome.desktop', 100 + i);

    const chrome = {kind: 'app', id: 'google-chrome.desktop', primaryText: 'Google Chrome'};
    const firefox = {kind: 'app', id: 'firefox.desktop', primaryText: 'Firefox'};
    const boosts = applyLearningBoosts(store, 'chr', [chrome, firefox]);

    assert.ok((boosts.get(chrome) ?? 0) > 0);
    assert.equal(boosts.get(firefox), undefined);
});

