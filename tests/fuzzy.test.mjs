import test from 'node:test';
import assert from 'node:assert/strict';

import {computeFuzzyScore, rankResults} from '../lib/fuzzy.js';

test('fuzzy scoring tolerates crome typo for Google Chrome', () => {
    const typo = computeFuzzyScore('crome', 'Google Chrome');
    const unrelated = computeFuzzyScore('crome', 'Terminal');
    assert.ok(typo > unrelated);
});

test('ordered short query chr matches Google Chrome strongly', () => {
    const score = computeFuzzyScore('chr', 'Google Chrome');
    assert.ok(score > 20);
});

test('ranking prefers windows over apps over recents on tie', () => {
    const items = [
        {kind: 'recent', primaryText: 'Chrome Notes', secondaryText: ''},
        {kind: 'app', primaryText: 'Chrome', secondaryText: ''},
        {kind: 'window', primaryText: 'Chrome', secondaryText: 'Workspace 1'},
    ];

    const ranked = rankResults('chrome', items, {
        weightWindows: 30,
        weightApps: 20,
        weightRecents: 10,
        maxResults: 10,
    });

    assert.equal(ranked[0].kind, 'window');
});

test('window title substring can win', () => {
    const items = [
        {kind: 'window', primaryText: 'Fix launcher ranking bug', secondaryText: 'Code - Workspace 2'},
        {kind: 'app', primaryText: 'Files', secondaryText: ''},
    ];

    const ranked = rankResults('ranking', items, {maxResults: 5});
    assert.equal(ranked[0].kind, 'window');
});


test('empty query falls back to source weighting order', () => {
    const items = [
        {kind: 'app', primaryText: 'Calculator', secondaryText: ''},
        {kind: 'window', primaryText: 'Terminal', secondaryText: ''},
        {kind: 'recent', primaryText: 'notes.txt', secondaryText: ''},
    ];

    const ranked = rankResults('', items, {maxResults: 10});
    assert.equal(ranked[0].kind, 'window');
});

test('new smart-provider kinds stay below windows and apps with default weights', () => {
    const items = [
        {kind: 'file', primaryText: 'terminal.md', secondaryText: ''},
        {kind: 'emoji', primaryText: 'Terminal Face', secondaryText: ''},
        {kind: 'utility', primaryText: 'terminal = 1', secondaryText: ''},
        {kind: 'app', primaryText: 'Terminal', secondaryText: ''},
        {kind: 'window', primaryText: 'Terminal', secondaryText: ''},
    ];

    const ranked = rankResults('terminal', items, {
        weightWindows: 30,
        weightApps: 20,
        weightRecents: 10,
        weightFiles: 12,
        weightEmoji: 8,
        weightUtility: 6,
        maxResults: 10,
    });

    assert.equal(ranked[0].kind, 'window');
    assert.equal(ranked[1].kind, 'app');
});

test('ranking clamps maxResults to at least one', () => {
    const items = [
        {kind: 'app', primaryText: 'Calculator', secondaryText: ''},
        {kind: 'window', primaryText: 'Terminal', secondaryText: ''},
    ];
    const ranked = rankResults('', items, {maxResults: 0});
    assert.equal(ranked.length, 1);
    assert.equal(ranked[0].kind, 'window');
});

test('ranking caches normalized haystack for reused scoring', () => {
    const item = {kind: 'app', primaryText: 'Brave Browser', secondaryText: 'Web browser'};
    rankResults('br', [item], {maxResults: 5});
    assert.equal(item._searchHaystack, 'Brave Browser Web browser');
    assert.equal(item._searchHaystackLower, 'brave browser web browser');
});

test('ranking deduplicates duplicate result identities', () => {
    const items = [
        {kind: 'app', id: 'org.gnome.Terminal.desktop', primaryText: 'Terminal', secondaryText: 'Application'},
        {kind: 'app', id: 'org.gnome.Terminal.desktop', primaryText: 'Terminal', secondaryText: 'Application'},
        {kind: 'window', id: 'window:1', primaryText: 'Terminal', secondaryText: 'Terminal • Workspace 1'},
    ];

    const ranked = rankResults('terminal', items, {maxResults: 10});
    const appRows = ranked.filter(row => row.kind === 'app' && row.id === 'org.gnome.Terminal.desktop');
    assert.equal(appRows.length, 1);
});

test('ranking deduplicates app rows with different ids but same visible text', () => {
    const items = [
        {kind: 'app', id: 'brave-browser.desktop', primaryText: 'Brave Browser', secondaryText: 'Web browser'},
        {kind: 'app', id: 'brave-browser-alt.desktop', primaryText: 'Brave Browser', secondaryText: 'Web browser'},
    ];

    const ranked = rankResults('brave', items, {maxResults: 10});
    assert.equal(ranked.length, 1);
    assert.equal(ranked[0].primaryText, 'Brave Browser');
});

test('ranking applies external item score boosts', () => {
    const app = {kind: 'app', id: 'firefox.desktop', primaryText: 'Firefox', secondaryText: ''};
    const window = {kind: 'window', id: 'window:1', primaryText: 'Firefox', secondaryText: ''};
    const boosts = new Map([[app, 80]]);

    const ranked = rankResults('fire', [window, app], {
        maxResults: 10,
        scoreBoost: item => boosts.get(item) ?? 0,
    });

    assert.equal(ranked[0].kind, 'app');
});

test('ranking filters non-empty query matches under minFuzzyScore', () => {
    const items = [
        {kind: 'app', primaryText: 'Terminal', secondaryText: ''},
        {kind: 'app', primaryText: 'Mozilla Firefox', secondaryText: ''},
    ];

    const ranked = rankResults('ter', items, {
        maxResults: 10,
        minFuzzyScore: 999,
    });

    assert.equal(ranked.length, 0);
});

test('ranking keeps empty-query ordering even with minFuzzyScore', () => {
    const items = [
        {kind: 'window', primaryText: 'Terminal', secondaryText: ''},
        {kind: 'app', primaryText: 'Calculator', secondaryText: ''},
    ];

    const ranked = rankResults('', items, {
        maxResults: 10,
        minFuzzyScore: 40,
    });

    assert.equal(ranked.length, 2);
    assert.equal(ranked[0].kind, 'window');
});

test('ranking rejects dispersed long-query letter matches', () => {
    const items = [
        {kind: 'app', primaryText: 'Brave Web Browser', secondaryText: 'Access the Internet'},
        {kind: 'app', primaryText: 'Bluetooth Transfer', secondaryText: 'Send files via Bluetooth'},
        {kind: 'app', primaryText: 'Report a problem...', secondaryText: 'Report a malfunction to developers'},
        {kind: 'app', primaryText: 'WebStorm', secondaryText: 'The smartest JavaScript IDE'},
    ];

    const ranked = rankResults('brave', items, {
        maxResults: 10,
        minFuzzyScore: 30,
    });

    assert.deepEqual(
        ranked.map(item => item.primaryText),
        ['Brave Web Browser']
    );
});

test('fuzzy scoring makes dispersed brave matches non-positive', () => {
    const loose1 = computeFuzzyScore('brave', 'Oracle VirtualBox Run several virtual systems on a single host computer');
    const loose2 = computeFuzzyScore('brave', 'Keyboard Layout Preview keyboard layouts');
    assert.ok(loose1 <= 0);
    assert.ok(loose2 <= 0);
});
