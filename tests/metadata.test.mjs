import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const rootDir = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');

function readMetadata() {
  const raw = fs.readFileSync(path.join(rootDir, 'metadata.json'), 'utf8');
  return JSON.parse(raw);
}

function readInstallScript() {
  return fs.readFileSync(path.join(rootDir, 'scripts/install-local.sh'), 'utf8');
}

test('metadata declares support for GNOME Shell 48+', () => {
  const metadata = readMetadata();
  const majorVersions = (metadata['shell-version'] ?? []).map(v => Number.parseInt(String(v), 10));
  assert.ok(majorVersions.some(v => Number.isInteger(v) && v >= 48), 'metadata.json must include GNOME Shell version 48 or newer');
});

test('install script UUID matches metadata uuid', () => {
  const metadata = readMetadata();
  const script = readInstallScript();
  const match = script.match(/UUID="([^"]+)"/);
  assert.ok(match, 'install-local.sh must define UUID');
  assert.equal(match[1], metadata.uuid);
});
