import test from 'node:test';
import assert from 'node:assert/strict';

import {createHotkeydConfigClient} from '../lib/hotkeydConfigClient.js';

test('getShortcut reads daemon shortcut value', () => {
    const client = createHotkeydConfigClient({
        runJsonCommand(args) {
            assert.deepEqual(args, ['config', 'get']);
            return {shortcut: '<Super>space'};
        },
    });
    assert.equal(client.getShortcut(), '<Super>space');
});

test('setShortcut sends config set command and returns normalized response', () => {
    const client = createHotkeydConfigClient({
        runJsonCommand(args) {
            assert.deepEqual(args, ['config', 'set', '--shortcut', '<Super>Return']);
            return {
                shortcut: '<Super>Return',
                applied: true,
                requires_manual_step: false,
                warnings: [],
            };
        },
    });

    const result = client.setShortcut('<Super>Return');
    assert.deepEqual(result, {
        shortcut: '<Super>Return',
        applied: true,
        requiresManualStep: false,
        warnings: [],
    });
});

test('setShortcut rejects empty values', () => {
    const client = createHotkeydConfigClient({
        runJsonCommand() {
            return {};
        },
    });

    assert.throws(() => client.setShortcut('  '), /must not be empty/);
});
