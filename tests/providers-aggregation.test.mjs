import test from 'node:test';
import assert from 'node:assert/strict';

import {collectProviderItems} from '../lib/providerAggregator.js';

test('collectProviderItems keeps results from healthy providers when one throws', () => {
    const providers = [
        {getResults: () => { throw new Error('boom'); }},
        {getResults: () => [{kind: 'app', primaryText: 'Brave', secondaryText: ''}]},
    ];

    const out = collectProviderItems(providers, 'brave', 'all');
    assert.equal(out.length, 1);
    assert.equal(out[0].primaryText, 'Brave');
});
