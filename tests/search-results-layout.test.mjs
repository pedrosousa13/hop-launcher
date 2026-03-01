import test from 'node:test';
import assert from 'node:assert/strict';

import {splitTailItems, combineRankedWithTail} from '../lib/searchResultsLayout.js';

test('splitTailItems separates appendToEnd rows from ranked rows', () => {
    const rows = [
        {kind: 'app', primaryText: 'Terminal'},
        {kind: 'action', primaryText: 'Search Google', appendToEnd: true},
        {kind: 'tab', primaryText: 'Docs'},
    ];

    const {rankedItems, tailItems} = splitTailItems(rows);
    assert.equal(rankedItems.length, 2);
    assert.equal(tailItems.length, 1);
    assert.equal(tailItems[0].primaryText, 'Search Google');
});

test('combineRankedWithTail appends tail rows after ranked rows', () => {
    const ranked = [
        {kind: 'tab', primaryText: 'Docs'},
        {kind: 'app', primaryText: 'Terminal'},
    ];
    const tail = [
        {kind: 'action', primaryText: 'Search Google', appendToEnd: true},
        {kind: 'action', primaryText: 'Search DuckDuckGo', appendToEnd: true},
    ];

    const out = combineRankedWithTail(ranked, tail, 10);
    assert.deepEqual(out.map(row => row.primaryText), [
        'Docs',
        'Terminal',
        'Search Google',
        'Search DuckDuckGo',
    ]);
});

test('combineRankedWithTail reserves space for tail rows within max results', () => {
    const ranked = [
        {kind: 'tab', primaryText: 'Docs'},
        {kind: 'app', primaryText: 'Terminal'},
        {kind: 'app', primaryText: 'Calendar'},
    ];
    const tail = [
        {kind: 'action', primaryText: 'Search Google', appendToEnd: true},
        {kind: 'action', primaryText: 'Search DuckDuckGo', appendToEnd: true},
    ];

    const out = combineRankedWithTail(ranked, tail, 3);
    assert.deepEqual(out.map(row => row.primaryText), [
        'Docs',
        'Search Google',
        'Search DuckDuckGo',
    ]);
});
