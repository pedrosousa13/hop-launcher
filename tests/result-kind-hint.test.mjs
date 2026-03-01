import test from 'node:test';
import assert from 'node:assert/strict';

import {getResultHintActionLabel, getResultHintIconSpec} from '../lib/resultKindHint.js';

test('getResultHintIconSpec maps key result kinds to lucide assets and fallbacks', () => {
    assert.deepEqual(getResultHintIconSpec('window'), {
        relativePath: 'assets/icons/lucide/monitor.svg',
        fallbackIconName: 'focus-windows-symbolic',
    });
    assert.deepEqual(getResultHintIconSpec('app'), {
        relativePath: 'assets/icons/lucide/app-window.svg',
        fallbackIconName: 'application-x-executable-symbolic',
    });
    assert.deepEqual(getResultHintIconSpec('file'), {
        relativePath: 'assets/icons/lucide/files.svg',
        fallbackIconName: 'text-x-generic-symbolic',
    });
});

test('getResultHintIconSpec returns null for kinds without right-side icon', () => {
    assert.equal(getResultHintIconSpec('emoji'), null);
    assert.equal(getResultHintIconSpec('utility'), null);
    assert.equal(getResultHintIconSpec('action'), null);
});

test('getResultHintActionLabel maps copy and execute actions to specific labels', () => {
    assert.equal(getResultHintActionLabel('utility', 'copy'), 'Copy');
    assert.equal(getResultHintActionLabel('emoji', 'copy'), 'Copy');
    assert.equal(getResultHintActionLabel('app', 'execute'), 'Open');
    assert.equal(getResultHintActionLabel('file', 'execute'), 'Open');
    assert.equal(getResultHintActionLabel('window', 'execute'), 'Focus');
    assert.equal(getResultHintActionLabel('action', 'execute'), 'Run');
});

test('getResultHintActionLabel falls back to Enter when action is unknown', () => {
    assert.equal(getResultHintActionLabel('app', 'none'), 'Enter');
    assert.equal(getResultHintActionLabel('unknown', 'execute'), 'Enter');
});
