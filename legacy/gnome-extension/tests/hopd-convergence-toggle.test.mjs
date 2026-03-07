import test from 'node:test';
import assert from 'node:assert/strict';

import {HopdProvider} from '../lib/providers/hopd.js';

test('HopdProvider convergence toggle gates non-utility mode routing', async () => {
    let calls = 0;
    const provider = new HopdProvider({
        requestIpc: async () => {
            calls++;
            return {result: {results: []}};
        },
        convergenceEnabled: false,
    });

    const rows = provider.getResults('terminal', 'apps');
    assert.deepEqual(rows, []);
    assert.equal(calls, 0);

    provider.setConvergenceEnabled(true);
    const pending = provider.getResults('terminal', 'apps');
    assert.equal(pending.length, 1);
    assert.match(pending[0].primaryText, /Searching via hopd/i);

    await new Promise(resolve => setTimeout(resolve, 0));
    assert.equal(calls, 1);
});
