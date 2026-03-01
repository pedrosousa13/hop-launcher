import test from 'node:test';
import assert from 'node:assert/strict';

import {getResultHintIconName} from '../lib/resultKindHint.js';

test('getResultHintIconName maps key result kinds to right-side icons', () => {
    assert.equal(getResultHintIconName('window'), 'focus-windows-symbolic');
    assert.equal(getResultHintIconName('app'), 'application-x-executable-symbolic');
    assert.equal(getResultHintIconName('file'), 'text-x-generic-symbolic');
});

test('getResultHintIconName returns null for kinds without right-side icon', () => {
    assert.equal(getResultHintIconName('emoji'), null);
    assert.equal(getResultHintIconName('utility'), null);
    assert.equal(getResultHintIconName('action'), null);
});
