import test from 'node:test';
import assert from 'node:assert/strict';

import {
    DEFAULT_WEB_SEARCH_SERVICES,
    filterEnabledSearchServices,
    parseWebSearchServices,
    validateWebSearchService,
} from '../lib/webSearchConfig.js';

test('parseWebSearchServices falls back to defaults for malformed JSON', () => {
    const out = parseWebSearchServices('not-json');
    assert.deepEqual(out.map(row => row.name), DEFAULT_WEB_SEARCH_SERVICES.map(row => row.name));
});

test('validateWebSearchService rejects non-https or templates without %s', () => {
    const missingPlaceholder = validateWebSearchService({
        id: 'x',
        name: 'No Placeholder',
        urlTemplate: 'https://example.com/search?q=query',
        enabled: true,
    });
    assert.equal(missingPlaceholder.valid, false);

    const nonHttps = validateWebSearchService({
        id: 'x',
        name: 'Insecure',
        urlTemplate: 'http://example.com/search?q=%s',
        enabled: true,
    });
    assert.equal(nonHttps.valid, false);
});

test('parseWebSearchServices keeps valid custom rows and skips invalid rows', () => {
    const json = JSON.stringify([
        {name: 'Kagi', urlTemplate: 'https://kagi.com/search?q=%s', enabled: true},
        {name: 'Bad', urlTemplate: 'https://bad.example.com/search?q=query', enabled: true},
    ]);

    const out = parseWebSearchServices(json, {fallbackToDefaults: false});
    assert.equal(out.length, 1);
    assert.equal(out[0].name, 'Kagi');
});

test('filterEnabledSearchServices only returns enabled rows', () => {
    const out = filterEnabledSearchServices([
        {id: 'g', name: 'Google', urlTemplate: 'https://google.com/search?q=%s', enabled: true},
        {id: 'd', name: 'DDG', urlTemplate: 'https://duckduckgo.com/?q=%s', enabled: false},
    ]);

    assert.equal(out.length, 1);
    assert.equal(out[0].id, 'g');
});
