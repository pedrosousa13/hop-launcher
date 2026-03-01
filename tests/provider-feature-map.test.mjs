import test from 'node:test';
import assert from 'node:assert/strict';

import {buildProviderFeatureMap} from '../lib/providerFeatureMap.js';

test('buildProviderFeatureMap returns expected provider keys and ordering', () => {
    const mapping = buildProviderFeatureMap({
        windows: 'windows-provider',
        apps: 'apps-provider',
        recents: 'recents-provider',
        files: 'files-provider',
        emoji: 'emoji-provider',
        calculator: 'calculator-provider',
        timezone: 'timezone-provider',
        currency: 'currency-provider',
        weather: 'weather-provider',
        webSearch: 'web-search-provider',
    });

    assert.deepEqual(mapping, [
        ['windows-provider', 'feature-windows-enabled'],
        ['apps-provider', 'feature-apps-enabled'],
        ['recents-provider', 'feature-files-enabled'],
        ['files-provider', 'feature-files-enabled'],
        ['emoji-provider', 'feature-emoji-enabled'],
        ['calculator-provider', 'feature-calculator-enabled'],
        ['timezone-provider', 'feature-timezone-enabled'],
        ['currency-provider', 'feature-currency-enabled'],
        ['weather-provider', 'feature-weather-enabled'],
        ['web-search-provider', 'feature-web-search-enabled'],
    ]);
});
