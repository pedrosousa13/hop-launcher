import test from 'node:test';
import assert from 'node:assert/strict';

import {
    parseCurrencyQuery,
    convertWithRates,
    isRatesCacheStale,
} from '../lib/providers/currency.js';

test('parses conversion query', () => {
    assert.deepEqual(parseCurrencyQuery('100 usd to eur'), {
        amount: 100,
        from: 'USD',
        to: 'EUR',
    });
});

test('converts amount with rates table', () => {
    const value = convertWithRates(100, 'USD', 'EUR', {
        USD: 1,
        EUR: 0.9,
    });
    assert.equal(value, 90);
});

test('marks old rates as stale', () => {
    const oldTimestamp = Date.now() - 13 * 60 * 60 * 1000;
    assert.equal(isRatesCacheStale(oldTimestamp, 12), true);
});
