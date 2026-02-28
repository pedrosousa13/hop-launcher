import test from 'node:test';
import assert from 'node:assert/strict';

import {evaluateExpression, shouldHandleCalculatorQuery} from '../lib/providers/calculator.js';

test('detects calculator-friendly query', () => {
    assert.equal(shouldHandleCalculatorQuery('2 + 2'), true);
    assert.equal(shouldHandleCalculatorQuery('open terminal'), false);
});

test('evaluates basic arithmetic expression', () => {
    assert.equal(evaluateExpression('2+2'), '4');
    assert.equal(evaluateExpression('10 / 4'), '2.5');
});

test('returns null for invalid expression', () => {
    assert.equal(evaluateExpression('2++'), null);
    assert.equal(evaluateExpression('abc'), null);
});
