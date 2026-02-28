import test from 'node:test';
import assert from 'node:assert/strict';

import {buildAliasContext, parseAliasesConfig} from '../lib/aliases.js';

test('parseAliasesConfig falls back to empty aliases for invalid JSON', () => {
    const parsed = parseAliasesConfig('{broken');
    assert.deepEqual(parsed, []);
});

test('buildAliasContext rewrites query from rewrite alias', () => {
    const aliases = parseAliasesConfig(JSON.stringify([
        {alias: 'gh', type: 'rewrite', target: {query: 'github'}},
    ]));

    const {rankingQuery} = buildAliasContext('gh', aliases, []);
    assert.equal(rankingQuery, 'github');
});

test('buildAliasContext boosts app alias targets on exact alias query', () => {
    const appRow = {kind: 'app', id: 'org.gnome.Terminal.desktop', primaryText: 'Terminal'};
    const aliases = parseAliasesConfig(JSON.stringify([
        {alias: 'term', type: 'app', target: {appId: 'org.gnome.Terminal.desktop'}},
    ]));

    const {boosts} = buildAliasContext('term', aliases, [appRow]);
    assert.ok((boosts.get(appRow) ?? 0) > 0);
});

test('buildAliasContext boosts only matching open window aliases', () => {
    const matchingWindow = {
        kind: 'window',
        id: 'window:1',
        primaryText: 'Daily Standup - Meet',
        windowAppId: 'org.gnome.Calendar',
    };
    const otherWindow = {
        kind: 'window',
        id: 'window:2',
        primaryText: 'Mail',
        windowAppId: 'org.gnome.Evolution',
    };
    const aliases = parseAliasesConfig(JSON.stringify([
        {
            alias: 'stand',
            type: 'window',
            target: {appId: 'org.gnome.Calendar', titleContains: 'standup'},
        },
    ]));

    const {boosts} = buildAliasContext('stand', aliases, [matchingWindow, otherWindow]);
    assert.ok((boosts.get(matchingWindow) ?? 0) > 0);
    assert.equal(boosts.get(otherWindow), undefined);
});

