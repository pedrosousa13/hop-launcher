import test from 'node:test';
import assert from 'node:assert/strict';

import {searchEmoji, shouldHandleEmojiQuery} from '../lib/providers/emoji.js';

test('handles explicit emoji prefix', () => {
    assert.equal(shouldHandleEmojiQuery(':emoji smile'), true);
    assert.equal(shouldHandleEmojiQuery('emoji smile'), true);
    assert.equal(shouldHandleEmojiQuery('smile'), true);
});

test('finds smile emoji by keyword', () => {
    const matches = searchEmoji('smile');
    assert.equal(matches[0].emoji, '😀');
});

test('finds smile emoji with natural language prefix', () => {
    const matches = searchEmoji('emoji smile');
    assert.equal(matches[0].emoji, '😀');
});
