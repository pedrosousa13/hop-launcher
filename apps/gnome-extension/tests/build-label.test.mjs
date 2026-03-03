import test from 'node:test';
import assert from 'node:assert/strict';

import {formatBuildLabel} from '../lib/buildLabel.js';

test('formatBuildLabel includes id and short hash when both exist', () => {
    assert.equal(formatBuildLabel('20260228-223500', 'abcdef123456'), 'dev 20260228-223500 abcdef1');
});

test('formatBuildLabel falls back when no build id/hash exists', () => {
    assert.equal(formatBuildLabel('', ''), 'dev local');
});
