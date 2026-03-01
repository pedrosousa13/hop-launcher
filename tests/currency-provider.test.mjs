import test from 'node:test';
import assert from 'node:assert/strict';

import {
    parseCurrencyQuery,
    convertWithRates,
    isRatesCacheStale,
    CurrencyProvider,
} from '../lib/providers/currency.js';

test('parses conversion query', () => {
    assert.deepEqual(parseCurrencyQuery('100 usd to eur'), {
        amount: 100,
        from: 'USD',
        to: 'EUR',
    });
});

test('parses conversion query without space between amount and source', () => {
    assert.deepEqual(parseCurrencyQuery('100usd to eur'), {
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

test('returns conversion row for eur to chf', () => {
    const provider = new CurrencyProvider();
    const rows = provider.getResults('100eur to chf', 'currency');
    assert.equal(rows.length, 1);
    assert.match(rows[0].primaryText, /CHF$/);
});
