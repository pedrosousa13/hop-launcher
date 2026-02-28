import test from 'node:test';
import assert from 'node:assert/strict';

import {resolveTimezoneQuery, shouldHandleTimezoneQuery} from '../lib/providers/timezone.js';

test('detects timezone query intent', () => {
    assert.equal(shouldHandleTimezoneQuery('time tokyo'), true);
    assert.equal(shouldHandleTimezoneQuery('tz pst'), true);
    assert.equal(shouldHandleTimezoneQuery('notes.txt'), false);
});

test('resolves pst alias to los angeles zone', () => {
    const matches = resolveTimezoneQuery('tz pst');
    assert.equal(matches[0].iana, 'America/Los_Angeles');
});
