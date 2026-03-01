import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const rootDir = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');

function readSchema() {
    return fs.readFileSync(path.join(rootDir, 'schemas', 'org.hoplauncher.app.gschema.xml'), 'utf8');
}

test('schema uses hoplauncher.app namespace and path', () => {
    const schema = readSchema();
    assert.match(schema, /<schema id="org\.hoplauncher\.app" path="\/org\/hoplauncher\/app\/">/);
});

test('schema declares min-fuzzy-score setting with strict default', () => {
    const schema = readSchema();
    assert.match(schema, /<key name="min-fuzzy-score" type="i">/);
    assert.match(schema, /<default>30<\/default>/);
});

test('schema declares tab and web-search settings', () => {
    const schema = readSchema();
    assert.match(schema, /<key name="web-search-enabled" type="b">/);
    assert.match(schema, /<key name="web-search-max-actions" type="i">/);
    assert.match(schema, /<key name="web-search-services-json" type="s">/);
});

test('schema declares learning insights settings', () => {
    const schema = readSchema();
    assert.match(schema, /<key name="learning-insights-limit" type="i">/);
    assert.match(schema, /<default>10<\/default>/);
    assert.match(schema, /<key name="learning-insights-sort" type="s">/);
    assert.match(schema, /<default>'count'<\/default>/);
});

test('schema declares main feature toggles', () => {
    const schema = readSchema();
    assert.match(schema, /<key name="feature-windows-enabled" type="b">/);
    assert.match(schema, /<key name="feature-apps-enabled" type="b">/);
    assert.match(schema, /<key name="feature-files-enabled" type="b">/);
    assert.match(schema, /<key name="feature-emoji-enabled" type="b">/);
    assert.match(schema, /<key name="feature-calculator-enabled" type="b">/);
    assert.match(schema, /<key name="feature-currency-enabled" type="b">/);
    assert.match(schema, /<key name="feature-timezone-enabled" type="b">/);
    assert.match(schema, /<key name="feature-weather-enabled" type="b">/);
    assert.match(schema, /<key name="feature-web-search-enabled" type="b">/);
});

test('schema declares overlay translucency key', () => {
    const schema = readSchema();
    assert.match(schema, /<key name="overlay-translucency" type="i">/);
});
