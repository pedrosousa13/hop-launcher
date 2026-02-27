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
