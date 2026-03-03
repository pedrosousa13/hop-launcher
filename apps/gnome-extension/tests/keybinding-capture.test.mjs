import test from 'node:test';
import assert from 'node:assert/strict';

import {
    resolveTypedAccelerator,
    interpretKeybindingPress,
    sanitizeModifierState,
} from '../lib/keybindingCapture.js';

const KEY_NAMES = {
    escape: 9,
    backSpace: 22,
    delete: 119,
    modifiers: new Set([50, 62, 37, 105, 64, 108, 133, 134]),
};

test('resolveTypedAccelerator falls back to default when empty', () => {
    const accel = resolveTypedAccelerator('', '<Super>space', value => value === '<Super>space');
    assert.equal(accel, '<Super>space');
});

test('resolveTypedAccelerator rejects invalid accelerators', () => {
    const accel = resolveTypedAccelerator('invalid', '<Super>space', () => false);
    assert.equal(accel, null);
});

test('interpretKeybindingPress clears on backspace', () => {
    const action = interpretKeybindingPress({keyval: KEY_NAMES.backSpace, mods: 0, keyNames: KEY_NAMES});
    assert.deepEqual(action, {kind: 'clear'});
});

test('interpretKeybindingPress ignores modifiers-only', () => {
    const action = interpretKeybindingPress({keyval: 133, mods: 8, keyNames: KEY_NAMES});
    assert.deepEqual(action, {kind: 'ignore'});
});

test('interpretKeybindingPress ignores combos without modifiers', () => {
    const action = interpretKeybindingPress({keyval: 65, mods: 0, keyNames: KEY_NAMES});
    assert.deepEqual(action, {kind: 'ignore'});
});

test('interpretKeybindingPress sets for valid combo candidate', () => {
    const action = interpretKeybindingPress({keyval: 65, mods: 8, keyNames: KEY_NAMES});
    assert.deepEqual(action, {kind: 'set', keyval: 65, mods: 8});
});

test('sanitizeModifierState masks out unsupported bits', () => {
    const masked = sanitizeModifierState(0b1111, 0b0011);
    assert.equal(masked, 0b0011);
});
