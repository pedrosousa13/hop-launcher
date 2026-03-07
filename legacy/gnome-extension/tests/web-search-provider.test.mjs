import test from 'node:test';
import assert from 'node:assert/strict';

import {WebSearchProvider} from '../lib/providers/webSearch.js';

function makeSettings(overrides = {}) {
    const values = {
        'web-search-enabled': true,
        'web-search-max-actions': 5,
        'web-search-services-json': JSON.stringify([
            {id: 'google', name: 'Google', urlTemplate: 'https://www.google.com/search?q=%s', enabled: true},
            {id: 'ddg', name: 'DuckDuckGo', urlTemplate: 'https://duckduckgo.com/?q=%s', enabled: true},
        ]),
        ...overrides,
    };
    return {
        get_boolean: key => Boolean(values[key]),
        get_int: key => Number(values[key]),
        get_string: key => String(values[key]),
    };
}

test('WebSearchProvider returns no rows for empty query', () => {
    const provider = new WebSearchProvider(makeSettings());
    assert.deepEqual(provider.getResults('', 'all'), []);
    assert.deepEqual(provider.getResults('   ', 'all'), []);
});

test('WebSearchProvider returns enabled services with URL-encoded query', () => {
    const provider = new WebSearchProvider(makeSettings());
    const rows = provider.getResults('gnome shell', 'all');

    assert.equal(rows.length, 2);
    assert.equal(rows[0].kind, 'action');
    assert.match(rows[0].primaryText, /Search Google/);
    assert.match(rows[0].secondaryText, /google\.com/);
    assert.equal(rows[0].searchUrl, 'https://www.google.com/search?q=gnome%20shell');
});

test('WebSearchProvider honors max actions', () => {
    const provider = new WebSearchProvider(makeSettings({'web-search-max-actions': 1}));
    const rows = provider.getResults('query', 'all');
    assert.equal(rows.length, 1);
});

test('WebSearchProvider ignores deprecated web-search-enabled gate', () => {
    const provider = new WebSearchProvider(makeSettings({'web-search-enabled': false}));
    const rows = provider.getResults('query', 'all');
    assert.equal(rows.length, 2);
});

test('WebSearchProvider preserves configured provider order', () => {
    const provider = new WebSearchProvider(makeSettings({
        'web-search-services-json': JSON.stringify([
            {id: 'ddg', name: 'DuckDuckGo', urlTemplate: 'https://duckduckgo.com/?q=%s', enabled: true},
            {id: 'google', name: 'Google', urlTemplate: 'https://www.google.com/search?q=%s', enabled: true},
        ]),
    }));

    const rows = provider.getResults('query', 'all');
    assert.equal(rows.length, 2);
    assert.match(rows[0].primaryText, /DuckDuckGo/);
    assert.match(rows[1].primaryText, /Google/);
});

test('WebSearchProvider returns no rows when provider list is empty', () => {
    const provider = new WebSearchProvider(makeSettings({
        'web-search-services-json': '[]',
    }));

    const rows = provider.getResults('query', 'all');
    assert.deepEqual(rows, []);
});
