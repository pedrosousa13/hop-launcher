import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const rootDir = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');

function readSchema() {
    return fs.readFileSync(path.join(rootDir, 'schemas', 'org.example.launcher.gschema.xml'), 'utf8');
}

test('schema declares min-fuzzy-score setting with strict default', () => {
    const schema = readSchema();
    assert.match(schema, /<key name="min-fuzzy-score" type="i">/);
    assert.match(schema, /<default>30<\/default>/);
});
