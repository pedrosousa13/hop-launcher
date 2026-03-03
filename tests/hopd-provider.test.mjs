import test from 'node:test';
import assert from 'node:assert/strict';

import {HopdProvider} from '../lib/providers/hopd.js';

test('HopdProvider returns no rows for empty query', () => {
    const provider = new HopdProvider({
        requestIpc: async () => ({result: {results: []}}),
    });

    assert.deepEqual(provider.getResults('', 'all'), []);
});

test('HopdProvider returns pending row then cached hopd rows', async () => {
    const calls = [];
    const provider = new HopdProvider({
        requestIpc: async request => {
            calls.push(request);
            return {
                result: {
                    results: [
                        {id: 'utility:weather', kind: 'weather', title: 'Weather'},
                    ],
                },
            };
        },
    });

    const pending = provider.getResults('weather', 'all');
    assert.equal(pending.length, 1);
    assert.match(pending[0].primaryText, /Searching via hopd/i);

    await new Promise(resolve => setTimeout(resolve, 0));

    const cached = provider.getResults('weather', 'all');
    assert.equal(cached.length, 1);
    assert.equal(cached[0].kind, 'utility');
    assert.equal(cached[0].primaryText, 'Weather');
    assert.equal(calls.length, 1);
    assert.equal(calls[0].method, 'search.query');
});

test('HopdProvider execute action forwards actions.execute IPC request', async () => {
    const calls = [];
    const provider = new HopdProvider({
        requestIpc: async request => {
            calls.push(request);
            if (request.method === 'search.query') {
                return {
                    result: {
                        results: [
                            {id: 'utility:emoji', kind: 'emoji', title: 'Emoji'},
                        ],
                    },
                };
            }
            return {result: {ok: true, executed: true}};
        },
    });

    provider.getResults('emoji', 'all');
    await new Promise(resolve => setTimeout(resolve, 0));
    const rows = provider.getResults('emoji', 'all');
    assert.equal(rows.length, 1);
    assert.equal(typeof rows[0].execute, 'function');

    await rows[0].execute();

    const actionCall = calls.find(call => call.method === 'actions.execute');
    assert.ok(actionCall);
    assert.equal(actionCall.params.result_id, 'utility:emoji');
    assert.equal(actionCall.params.action, 'enter');
});

test('HopdProvider returns error row when IPC fails', async () => {
    const provider = new HopdProvider({
        requestIpc: async () => {
            throw new Error('socket unavailable');
        },
    });

    provider.getResults('timezone', 'all');
    await new Promise(resolve => setTimeout(resolve, 0));
    const rows = provider.getResults('timezone', 'all');
    assert.equal(rows.length, 1);
    assert.match(rows[0].primaryText, /hopd unavailable/i);
});
