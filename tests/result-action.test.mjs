import test from 'node:test';
import assert from 'node:assert/strict';

import {resolveEnterAction} from '../lib/resultAction.js';

test('resolveEnterAction copies utility text on enter', () => {
    const action = resolveEnterAction({
        kind: 'utility',
        primaryText: '42',
        copyText: '42',
    });
    assert.deepEqual(action, {type: 'copy', text: '42'});
});

test('resolveEnterAction copies emoji glyph on enter', () => {
    const action = resolveEnterAction({
        kind: 'emoji',
        primaryText: '😀 grinning face',
        copyText: '😀',
    });
    assert.deepEqual(action, {type: 'copy', text: '😀'});
});

test('resolveEnterAction executes normal app result on enter', () => {
    const action = resolveEnterAction({
        kind: 'app',
        execute: () => {},
    });
    assert.deepEqual(action, {type: 'execute'});
});

