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

test('resolves bare timezone token without prefix', () => {
    const matches = resolveTimezoneQuery('pst');
    assert.equal(matches[0].iana, 'America/Los_Angeles');
});

test('resolves city query to matching timezone', () => {
    const matches = resolveTimezoneQuery('time zurich');
    assert.equal(matches[0].iana, 'Europe/Zurich');
});

test('resolves time in city phrasing', () => {
    const matches = resolveTimezoneQuery('time in zurich');
    assert.equal(matches[0].iana, 'Europe/Zurich');
});
