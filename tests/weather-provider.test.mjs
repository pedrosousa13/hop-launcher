import test from 'node:test';
import assert from 'node:assert/strict';

import {
    WeatherProvider,
    buildWeatherRow,
    parseWeatherQuery,
    shouldHandleWeatherQuery,
    WTTR_FORMAT,
} from '../lib/providers/weather.js';

test('detects weather query prefixes', () => {
    assert.equal(shouldHandleWeatherQuery('weather berlin'), true);
    assert.equal(shouldHandleWeatherQuery('wx berlin'), true);
    assert.equal(shouldHandleWeatherQuery('zurich weather'), true);
    assert.equal(shouldHandleWeatherQuery('weatherberlin'), false);
});

test('parses weather query with and without prefixes', () => {
    assert.deepEqual(parseWeatherQuery('weather berlin'), {location: 'berlin'});
    assert.deepEqual(parseWeatherQuery('wx 94103'), {location: '94103'});
    assert.deepEqual(parseWeatherQuery('zurich weather'), {location: 'zurich'});
    assert.equal(parseWeatherQuery('tokyo'), null);
    assert.deepEqual(parseWeatherQuery('tokyo', {allowBare: true}), {location: 'tokyo'});
});

test('exports expected wttr format string', () => {
    assert.equal(WTTR_FORMAT, '3');
});

test('formats weather utility row', () => {
    const row = buildWeatherRow({
        location: 'zurich',
        prettyText: 'Zurich: ☀️ +12°C',
        observedAt: '2026-03-01T08:00',
    }, 'zurich');

    assert.equal(row.kind, 'utility');
    assert.equal(row.primaryText, 'Zurich: ☀️ +12°C');
    assert.match(row.secondaryText, /wttr\.in/);
    assert.match(row.secondaryText, /Updated 2026-03-01T08:00/);
});

test('returns pending row then cached weather row after fetch', async () => {
    const requestText = async () => 'Zurich: ☀️ +12°C';
    const provider = new WeatherProvider({requestText, timeoutMs: 3000});

    const pending = provider.getResults('weather zurich', 'weather');
    assert.equal(pending.length, 1);
    assert.match(pending[0].primaryText, /Fetching weather/);

    await new Promise(resolve => setTimeout(resolve, 0));

    const cached = provider.getResults('weather zurich', 'weather');
    assert.equal(cached.length, 1);
    assert.match(cached[0].primaryText, /☀️/);
    assert.match(cached[0].secondaryText, /wttr\.in/);
});

test('returns stale cached row and refreshes in background', async () => {
    let calls = 0;
    const requestText = async () => {
        calls++;
        return 'Berlin: ⛅ +8°C';
    };

    const provider = new WeatherProvider({requestText, ttlMs: 1, timeoutMs: 3000});

    provider.getResults('weather berlin', 'weather');
    await new Promise(resolve => setTimeout(resolve, 0));

    await new Promise(resolve => setTimeout(resolve, 5));
    const stale = provider.getResults('weather berlin', 'weather');
    assert.equal(stale.length, 1);
    assert.match(stale[0].secondaryText, /\(stale\)/);

    await new Promise(resolve => setTimeout(resolve, 0));
    assert.ok(calls >= 2);
});

test('timeout keeps pending/fallback behavior without throwing', async () => {
    const requestText = async () => new Promise(resolve => setTimeout(() => resolve('slow'), 100));
    const provider = new WeatherProvider({requestText, timeoutMs: 5});

    const rows = provider.getResults('weather timeout-town', 'weather');
    assert.equal(rows.length, 1);
    assert.match(rows[0].primaryText, /Fetching weather/);

    await new Promise(resolve => setTimeout(resolve, 15));
    const next = provider.getResults('weather timeout-town', 'weather');
    assert.equal(next.length, 1);
    assert.match(next[0].primaryText, /Weather unavailable/);
});

test('default timeout tolerates slower wttr responses', async () => {
    const requestText = async () => new Promise(resolve => setTimeout(() => resolve('Lisbon: ☀️ +18°C'), 3000));
    const provider = new WeatherProvider({requestText});

    const rows = provider.getResults('weather lisbon', 'weather');
    assert.equal(rows.length, 1);
    assert.match(rows[0].primaryText, /Fetching weather/);

    await new Promise(resolve => setTimeout(resolve, 3100));
    const next = provider.getResults('weather lisbon', 'weather');
    assert.equal(next.length, 1);
    assert.match(next[0].primaryText, /☀️/);
});

test('does not provide weather rows in all mode without weather intent', () => {
    const provider = new WeatherProvider();
    const rows = provider.getResults('firefox', 'all');
    assert.deepEqual(rows, []);
});
