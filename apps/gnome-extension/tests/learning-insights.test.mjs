import test from 'node:test';
import assert from 'node:assert/strict';

import {buildLearningInsights} from '../lib/learningInsights.js';

test('buildLearningInsights limits rows and sorts by most used', () => {
    const rows = buildLearningInsights('{"version":1,"entries":{"brave":{"brave.desktop":{"count":4,"lastUsedMs":100}},"notes":{"notes.desktop":{"count":2,"lastUsedMs":300}},"term":{"term.desktop":{"count":1,"lastUsedMs":500}}}}', {
        limit: 2,
        sort: 'count',
    });

    assert.equal(rows.length, 2);
    assert.deepEqual(rows.map(r => r.query), ['brave', 'notes']);
});

test('buildLearningInsights sorts by most recent', () => {
    const rows = buildLearningInsights('{"version":1,"entries":{"brave":{"brave.desktop":{"count":4,"lastUsedMs":100}},"notes":{"notes.desktop":{"count":2,"lastUsedMs":300}},"term":{"term.desktop":{"count":1,"lastUsedMs":500}}}}', {
        limit: 3,
        sort: 'recent',
    });

    assert.deepEqual(rows.map(r => r.query), ['term', 'notes', 'brave']);
});

