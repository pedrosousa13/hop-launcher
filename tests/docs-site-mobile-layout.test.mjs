import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const rootDir = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');

function readGlobalCss() {
    return fs.readFileSync(path.join(rootDir, 'docs-site', 'src', 'styles', 'global.css'), 'utf8');
}

test('docs-site mobile css includes launcher demo overflow guards', () => {
    const css = readGlobalCss();
    assert.match(css, /@media\s*\(max-width:\s*480px\)/);
    assert.match(css, /\.launcher-shell\s*\{[^}]*min-width:\s*0;/s);
    assert.match(css, /\.launcher-results\s*\{[^}]*overflow-x:\s*hidden;/s);
    assert.match(css, /\.row-action\s+span\s*\{[^}]*display:\s*none;/s);
});
