import test from 'node:test';
import assert from 'node:assert/strict';

import {
    WeatherProvider,
    buildWeatherRow,
    parseWeatherQuery,
    shouldHandleWeatherQuery,
    OPEN_METEO_SOURCE,
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

test('parses leading in marker from weather locations', () => {
    assert.deepEqual(parseWeatherQuery('weather in zurich'), {location: 'zurich'});
    assert.deepEqual(parseWeatherQuery('in zurich', {allowBare: true}), {location: 'zurich'});
});

test('exports expected weather source label', () => {
    assert.equal(OPEN_METEO_SOURCE, 'Open-Meteo');
});

test('formats weather utility row', () => {
    const row = buildWeatherRow({
        location: 'zurich',
        prettyText: 'Zurich: Clear ☀ 12C Wind 10 km/h',
        observedAt: '2026-03-01T08:00',
    }, 'zurich');

    assert.equal(row.kind, 'utility');
    assert.equal(row.primaryText, 'Zurich: Clear ☀ 12C Wind 10 km/h');
    assert.match(row.secondaryText, /Open-Meteo/);
    assert.match(row.secondaryText, /Updated 2026-03-01T08:00/);
});

test('returns pending row then cached weather row after fetch', async () => {
    const requestJson = async url => {
        if (url.includes('geocoding-api.open-meteo.com')) {
            return {
                results: [{
                    name: 'Zurich',
                    latitude: 47.37,
                    longitude: 8.54,
                    admin1: 'Zurich',
                    country_code: 'CH',
                }],
            };
        }

        if (url.includes('api.open-meteo.com')) {
            return {
                current: {
                    temperature_2m: 12.3,
                    weather_code: 0,
                    wind_speed_10m: 9.6,
                },
            };
        }

        throw new Error('unexpected url');
    };

    const provider = new WeatherProvider({requestJson, timeoutMs: 3000});

    const pending = provider.getResults('weather zurich', 'weather');
    assert.equal(pending.length, 1);
    assert.match(pending[0].primaryText, /Fetching weather/);

    await new Promise(resolve => setTimeout(resolve, 0));

    const cached = provider.getResults('weather zurich', 'weather');
    assert.equal(cached.length, 1);
    assert.match(cached[0].primaryText, /Zurich, Zurich/);
    assert.match(cached[0].primaryText, /Clear/);
    assert.match(cached[0].primaryText, /12C/);
    assert.match(cached[0].primaryText, /Wind 10 km\/h/);
    assert.match(cached[0].secondaryText, /Open-Meteo/);
});

test('returns stale cached row and refreshes in background', async () => {
    let calls = 0;
    const requestJson = async url => {
        calls++;
        if (url.includes('geocoding-api.open-meteo.com')) {
            return {
                results: [{
                    name: 'Berlin',
                    latitude: 52.52,
                    longitude: 13.41,
                    country_code: 'DE',
                }],
            };
        }

        return {
            current: {
                temperature_2m: 8,
                weather_code: 2,
                wind_speed_10m: 7,
            },
        };
    };

    const provider = new WeatherProvider({requestJson, ttlMs: 1, timeoutMs: 3000});

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
    const requestJson = async () => new Promise(resolve => setTimeout(() => resolve({}), 100));
    const provider = new WeatherProvider({requestJson, timeoutMs: 5});

    const rows = provider.getResults('weather timeout-town', 'weather');
    assert.equal(rows.length, 1);
    assert.match(rows[0].primaryText, /Fetching weather/);

    await new Promise(resolve => setTimeout(resolve, 15));
    const next = provider.getResults('weather timeout-town', 'weather');
    assert.equal(next.length, 1);
    assert.match(next[0].primaryText, /Weather unavailable/);
});

test('returns unavailable row when location has no geocode match', async () => {
    const requestJson = async url => {
        if (url.includes('geocoding-api.open-meteo.com'))
            return {results: []};
        throw new Error('unexpected url');
    };
    const provider = new WeatherProvider({requestJson, timeoutMs: 3000});

    const rows = provider.getResults('weather atlantis', 'weather');
    assert.equal(rows.length, 1);
    assert.match(rows[0].primaryText, /Fetching weather/);

    await new Promise(resolve => setTimeout(resolve, 0));
    const next = provider.getResults('weather atlantis', 'weather');
    assert.equal(next.length, 1);
    assert.match(next[0].primaryText, /Weather unavailable/);
    assert.match(next[0].secondaryText, /location not found/);
});

test('does not provide weather rows in all mode without weather intent', () => {
    const provider = new WeatherProvider();
    const rows = provider.getResults('firefox', 'all');
    assert.deepEqual(rows, []);
});

test('uses soup fallback when fetch is unavailable', async () => {
    const originalFetch = globalThis.fetch;
    globalThis.fetch = undefined;

    const soupRequestJson = async url => {
        if (url.includes('geocoding-api.open-meteo.com')) {
            return {
                results: [{
                    name: 'Lisbon',
                    latitude: 38.72,
                    longitude: -9.13,
                    country_code: 'PT',
                }],
            };
        }

        if (url.includes('api.open-meteo.com')) {
            return {
                current: {
                    temperature_2m: 18,
                    weather_code: 1,
                    wind_speed_10m: 14,
                },
            };
        }

        throw new Error('unexpected url');
    };

    try {
        const provider = new WeatherProvider({soupRequestJson, timeoutMs: 3000});
        const pending = provider.getResults('weather lisbon', 'weather');
        assert.equal(pending.length, 1);
        assert.match(pending[0].primaryText, /Fetching weather/);

        await new Promise(resolve => setTimeout(resolve, 0));

        const cached = provider.getResults('weather lisbon', 'weather');
        assert.equal(cached.length, 1);
        assert.match(cached[0].primaryText, /Lisbon/);
        assert.doesNotMatch(cached[0].primaryText, /Weather unavailable/);
    } finally {
        globalThis.fetch = originalFetch;
    }
});
