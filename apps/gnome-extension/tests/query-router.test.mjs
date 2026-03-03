import test from 'node:test';
import assert from 'node:assert/strict';

import {extractQueryRoute} from '../lib/queryRouter.js';

test('prefix f routes to files mode', () => {
    assert.deepEqual(extractQueryRoute('f report'), {mode: 'files', query: 'report'});
});

test('prefix emoji routes to emoji mode', () => {
    assert.deepEqual(extractQueryRoute(':emoji smile'), {mode: 'emoji', query: 'smile'});
});

test('emoji keyword routes to emoji mode', () => {
    assert.deepEqual(extractQueryRoute('emoji smile'), {mode: 'emoji', query: 'smile'});
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

test('time in phrase routes to timezone mode', () => {
    assert.deepEqual(extractQueryRoute('time in zurich'), {mode: 'timezone', query: 'zurich'});
});

test('timezone alias routes to timezone mode', () => {
    assert.deepEqual(extractQueryRoute('pst'), {mode: 'timezone', query: 'pst'});
});

test('timezone city routes to timezone mode', () => {
    assert.deepEqual(extractQueryRoute('zurich'), {mode: 'timezone', query: 'zurich'});
});

test('city time suffix routes to timezone mode', () => {
    assert.deepEqual(extractQueryRoute('zurich time'), {mode: 'timezone', query: 'zurich'});
});

test('weather keyword routes to weather mode', () => {
    assert.deepEqual(extractQueryRoute('weather berlin'), {mode: 'weather', query: 'berlin'});
});

test('wx shorthand routes to weather mode', () => {
    assert.deepEqual(extractQueryRoute('wx 94103'), {mode: 'weather', query: '94103'});
});

test('city weather suffix routes to weather mode', () => {
    assert.deepEqual(extractQueryRoute('zurich weather'), {mode: 'weather', query: 'zurich'});
});

test('currency conversion text routes to currency mode', () => {
    assert.deepEqual(extractQueryRoute('100 usd to eur'), {mode: 'currency', query: '100 usd to eur'});
});

test('compact currency conversion routes to currency mode', () => {
    assert.deepEqual(extractQueryRoute('100usd to eur'), {mode: 'currency', query: '100usd to eur'});
});

test('default route keeps all mode', () => {
    assert.deepEqual(extractQueryRoute('firefox'), {mode: 'all', query: 'firefox'});
});
