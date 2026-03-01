import test from 'node:test';
import assert from 'node:assert/strict';

import {makeSettingsGatedProvider} from '../lib/providerToggleWrapper.js';

function makeSettings(overrides = {}) {
    const values = {
        'feature-weather-enabled': true,
        ...overrides,
    };
    return {
        get_boolean: key => Boolean(values[key]),
    };
}

test('settings-gated provider returns no rows when feature is disabled', () => {
    const provider = {
        getResults: () => [{id: 'a'}],
    };

    const gated = makeSettingsGatedProvider(provider, makeSettings({'feature-weather-enabled': false}), 'feature-weather-enabled');
    assert.deepEqual(gated.getResults('query', 'all'), []);
});

test('settings-gated provider passes through rows when feature is enabled', () => {
    const provider = {
        getResults: () => [{id: 'a'}],
    };

    const gated = makeSettingsGatedProvider(provider, makeSettings(), 'feature-weather-enabled');
    assert.deepEqual(gated.getResults('query', 'all'), [{id: 'a'}]);
});

test('settings-gated provider preserves provider metadata methods', () => {
    let refreshed = 0;
    const provider = {
        refreshOnOpen: true,
        refresh: () => {
            refreshed += 1;
        },
    };

    const gated = makeSettingsGatedProvider(provider, makeSettings(), 'feature-weather-enabled');
    assert.equal(gated.refreshOnOpen, true);
    gated.refresh();
    assert.equal(refreshed, 1);
});
