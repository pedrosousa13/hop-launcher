import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const rootDir = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const repoRoot = path.resolve(rootDir, '..', '..');

function readMetadata() {
  const raw = fs.readFileSync(path.join(rootDir, 'metadata.json'), 'utf8');
  return JSON.parse(raw);
}

function readInstallScript() {
  return fs.readFileSync(path.join(rootDir, 'scripts/install-local.sh'), 'utf8');
}

function readCiWorkflow() {
  return fs.readFileSync(path.join(repoRoot, '.github/workflows/ci.yml'), 'utf8');
}

function readReleaseWorkflow() {
  return fs.readFileSync(path.join(repoRoot, '.github/workflows/release.yml'), 'utf8');
}

test('metadata declares support for GNOME Shell 48+', () => {
  const metadata = readMetadata();
  const majorVersions = (metadata['shell-version'] ?? []).map(v => Number.parseInt(String(v), 10));
  assert.ok(majorVersions.some(v => Number.isInteger(v) && v >= 48), 'metadata.json must include GNOME Shell version 48 or newer');
});

test('install script UUID matches metadata uuid', () => {
  const metadata = readMetadata();
  const script = readInstallScript();
  assert.match(script, /METADATA_FILE="\$\{ROOT_DIR\}\/metadata\.json"/);
  assert.match(script, /UUID="\$\(sed -n .*"\$\{METADATA_FILE\}".*\| head -n 1\)"/s);
  assert.ok(script.includes('EXT_DIR="${DATA_HOME}/gnome-shell/extensions/${UUID}"'));
  assert.equal(metadata.uuid, 'hop-launcher@hoplauncher.app');
});

test('metadata uses GNOME extension schema namespace and repo URL', () => {
  const metadata = readMetadata();
  assert.equal(metadata.uuid, 'hop-launcher@hoplauncher.app');
  assert.equal(metadata.url, 'https://github.com/pedrosousa13/hop-launcher');
  assert.equal(metadata['settings-schema'], 'org.gnome.shell.extensions.hop-launcher');
});

test('metadata omits deprecated version field', () => {
  const metadata = readMetadata();
  assert.equal(Object.hasOwn(metadata, 'version'), false);
});

test('CI workflows do not require metadata.version checks', () => {
  const ci = readCiWorkflow();
  const release = readReleaseWorkflow();
  assert.equal(ci.includes('scripts/bump-version.sh --check'), false);
  assert.equal(ci.includes('scripts/bump-version.sh --check-bump-against'), false);
  assert.equal(release.includes('scripts/bump-version.sh --check'), false);
});
