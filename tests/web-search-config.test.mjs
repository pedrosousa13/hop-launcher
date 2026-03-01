import test from 'node:test';
import assert from 'node:assert/strict';

import {
    addWebSearchProvider,
    DEFAULT_WEB_SEARCH_SERVICES,
    filterEnabledSearchServices,
    parseWebSearchServices,
    serializeWebSearchServices,
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

test('parseWebSearchServices accepts legacy url field as template', () => {
    const json = JSON.stringify([
        {id: 'kagi', name: 'Kagi', url: 'https://kagi.com/search?q=%s', enabled: true},
    ]);

    const out = parseWebSearchServices(json, {fallbackToDefaults: false});
    assert.equal(out.length, 1);
    assert.equal(out[0].id, 'kagi');
    assert.equal(out[0].urlTemplate, 'https://kagi.com/search?q=%s');
});

test('filterEnabledSearchServices only returns enabled rows', () => {
    const out = filterEnabledSearchServices([
        {id: 'g', name: 'Google', urlTemplate: 'https://google.com/search?q=%s', enabled: true},
        {id: 'd', name: 'DDG', urlTemplate: 'https://duckduckgo.com/?q=%s', enabled: false},
    ]);

    assert.equal(out.length, 1);
    assert.equal(out[0].id, 'g');
});

test('serializeWebSearchServices accepts row-style name + url inputs', () => {
    const json = serializeWebSearchServices([
        {name: 'Kagi', urlTemplate: 'https://kagi.com/search?q=%s'},
        {name: 'Brave', urlTemplate: 'https://search.brave.com/search?q=%s'},
    ], {fallbackToDefaults: false});

    const out = JSON.parse(json);
    assert.equal(out.length, 2);
    assert.equal(out[0].name, 'Kagi');
    assert.equal(out[1].name, 'Brave');
});

test('serializeWebSearchServices falls back to defaults when all rows are invalid', () => {
    const json = serializeWebSearchServices([
        {name: 'Bad', urlTemplate: 'http://example.com/?q=%s'},
        {name: '', urlTemplate: 'https://example.com/?q=%s'},
    ]);

    const out = JSON.parse(json);
    assert.deepEqual(out.map(row => row.name), DEFAULT_WEB_SEARCH_SERVICES.map(row => row.name));
});

test('serializeWebSearchServices keeps optional keyword field', () => {
    const json = serializeWebSearchServices([
        {name: 'Google', urlTemplate: 'https://www.google.com/search?q=%s', keyword: 'g'},
    ], {fallbackToDefaults: false});

    const out = JSON.parse(json);
    assert.equal(out.length, 1);
    assert.equal(out[0].keyword, 'g');
});

test('addWebSearchProvider appends a valid provider to current rows', () => {
    const out = addWebSearchProvider([
        {id: 'google', name: 'Google', urlTemplate: 'https://www.google.com/search?q=%s', enabled: true},
    ], {
        id: 'kagi',
        name: 'Kagi',
        urlTemplate: 'https://kagi.com/search?q=%s',
        enabled: true,
        keyword: 'kg',
    });

    assert.equal(out.length, 2);
    assert.equal(out[1].id, 'kagi');
    assert.equal(out[1].name, 'Kagi');
});
