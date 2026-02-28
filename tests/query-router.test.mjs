import test from 'node:test';
import assert from 'node:assert/strict';

import {extractQueryRoute} from '../lib/queryRouter.js';

test('prefix f routes to files mode', () => {
    assert.deepEqual(extractQueryRoute('f report'), {mode: 'files', query: 'report'});
});

test('prefix emoji routes to emoji mode', () => {
    assert.deepEqual(extractQueryRoute(':emoji smile'), {mode: 'emoji', query: 'smile'});
});

test('prefix w routes to windows mode', () => {
    assert.deepEqual(extractQueryRoute('w terminal'), {mode: 'windows', query: 'terminal'});
});

test('math-like queries route to calculator mode', () => {
    assert.deepEqual(extractQueryRoute('2+2'), {mode: 'calculator', query: '2+2'});
});

test('timezone keyword routes to timezone mode', () => {
    assert.deepEqual(extractQueryRoute('time tokyo'), {mode: 'timezone', query: 'tokyo'});
});

test('currency conversion text routes to currency mode', () => {
    assert.deepEqual(extractQueryRoute('100 usd to eur'), {mode: 'currency', query: '100 usd to eur'});
});

test('default route keeps all mode', () => {
    assert.deepEqual(extractQueryRoute('firefox'), {mode: 'all', query: 'firefox'});
});
