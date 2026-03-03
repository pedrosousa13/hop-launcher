import test from 'node:test';
import assert from 'node:assert/strict';

import {
    TimezoneProvider,
    resolveTimezoneQuery,
    shouldHandleTimezoneQuery,
} from '../lib/providers/timezone.js';

test('detects timezone query intent', () => {
    assert.equal(shouldHandleTimezoneQuery('time tokyo'), true);
    assert.equal(shouldHandleTimezoneQuery('tz pst'), true);
    assert.equal(shouldHandleTimezoneQuery('notes.txt'), false);
});

test('resolves pst alias to los angeles zone', () => {
    const matches = resolveTimezoneQuery('tz pst');
    assert.equal(matches[0].iana, 'America/Los_Angeles');
});

test('resolves bare timezone token without prefix', () => {
    const matches = resolveTimezoneQuery('pst');
    assert.equal(matches[0].iana, 'America/Los_Angeles');
});

test('resolves city query to matching timezone', () => {
    const matches = resolveTimezoneQuery('time zurich');
    assert.equal(matches[0].iana, 'Europe/Zurich');
});

test('resolves time in city phrasing', () => {
    const matches = resolveTimezoneQuery('time in zurich');
    assert.equal(matches[0].iana, 'Europe/Zurich');
});

test('TimezoneProvider resolves unknown city via local fallback in timezone mode', async () => {
    let loads = 0;
    const cityLookupLoader = async () => {
        loads++;
        return {
            boston: 'America/New_York',
        };
    };
    const provider = new TimezoneProvider({cityLookupLoader, timeoutMs: 1000});

    const pending = provider.getResults('boston', 'timezone');
    assert.equal(pending.length, 1);
    assert.match(pending[0].primaryText, /Resolving timezone/);

    await new Promise(resolve => setTimeout(resolve, 0));

    const resolved = provider.getResults('boston', 'timezone');
    assert.equal(resolved.length, 1);
    assert.match(resolved[0].secondaryText, /America\/New_York/);
    assert.equal(loads, 1);
});

test('TimezoneProvider default city dataset resolves boston', async () => {
    const provider = new TimezoneProvider({timeoutMs: 1000});

    const pending = provider.getResults('boston', 'timezone');
    assert.equal(pending.length, 1);
    assert.match(pending[0].primaryText, /Resolving timezone/);

    let resolved = provider.getResults('boston', 'timezone');
    for (let i = 0; i < 20 && /Resolving timezone/.test(resolved[0]?.primaryText ?? ''); i++) {
        await new Promise(resolve => setTimeout(resolve, 25));
        resolved = provider.getResults('boston', 'timezone');
    }

    assert.equal(resolved.length, 1);
    assert.match(resolved[0].secondaryText, /America\/New_York/);
});

test('destroy prevents late update callbacks from inflight timezone lookup', async () => {
    let resolveLookup;
    const cityLookupLoader = async () => new Promise(resolve => {
        resolveLookup = resolve;
    });
    const provider = new TimezoneProvider({cityLookupLoader, timeoutMs: 3000});
    let updates = 0;
    provider.setUpdateCallback(() => {
        updates++;
    });

    provider.getResults('new-town', 'timezone');
    await new Promise(resolve => setTimeout(resolve, 0));
    provider.destroy();
    resolveLookup?.({'new-town': 'America/New_York'});
    await new Promise(resolve => setTimeout(resolve, 0));

    assert.equal(updates, 0);
});
