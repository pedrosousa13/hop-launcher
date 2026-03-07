import test from 'node:test';
import assert from 'node:assert/strict';

import {scoreFileMatch, filterIndexedEntries} from '../lib/index/fileIndexer.js';

test('filename score outranks path-only score', () => {
    const byName = scoreFileMatch('report', 'report-q1.pdf', '/docs/finance/report-q1.pdf');
    const byPath = scoreFileMatch('report', 'q1.pdf', '/docs/finance/report-q1.pdf');
    assert.ok(byName > byPath);
});

test('filterIndexedEntries returns best matches first', () => {
    const entries = [
        {name: 'notes.txt', path: '/tmp/notes.txt'},
        {name: 'report-q1.pdf', path: '/docs/finance/report-q1.pdf'},
        {name: 'roadmap.md', path: '/work/roadmap.md'},
    ];

    const matched = filterIndexedEntries(entries, 'report', 2);
    assert.equal(matched[0].name, 'report-q1.pdf');
});
