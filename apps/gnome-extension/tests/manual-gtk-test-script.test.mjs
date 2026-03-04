import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const extensionRoot = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const monorepoRoot = path.resolve(extensionRoot, '..', '..');
const scriptPath = path.join(monorepoRoot, 'scripts/run-gtk-manual-test.sh');

function readScript() {
  return fs.readFileSync(scriptPath, 'utf8');
}

test('manual gtk test script exists at monorepo root scripts directory', () => {
  assert.equal(fs.existsSync(scriptPath), true);
});

test('manual gtk test script installs hopd locally before launch', () => {
  const script = readScript();
  assert.match(script, /apps\/gnome-extension\/scripts\/install-hopd-local\.sh/);
});

test('manual gtk test script checks hopd health over unix socket', () => {
  const script = readScript();
  assert.match(script, /health\.ping/);
  assert.match(script, /python3|socat/);
});

test('manual gtk test script launches gtk_ui app', () => {
  const script = readScript();
  assert.match(script, /cargo run --features gtk_ui/);
});
