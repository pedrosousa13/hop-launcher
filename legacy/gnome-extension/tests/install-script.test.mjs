import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const rootDir = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const scriptPath = path.join(rootDir, 'scripts/install-local.sh');
const reloadScriptPath = path.join(rootDir, 'scripts/reload-shell.sh');

function readScript() {
  return fs.readFileSync(scriptPath, 'utf8');
}

function readReloadScript() {
  return fs.readFileSync(reloadScriptPath, 'utf8');
}

test('install script waits until extension appears in gnome-extensions list before enabling', () => {
  const script = readScript();
  assert.match(script, /gnome-extensions list/);
  assert.match(script, /grep -Fxq "\$\{UUID\}"/);
  assert.match(script, /for\s+\(\(\s*attempt\s*=\s*1;\s*attempt\s*<=\s*\d+;\s*attempt\+\+\s*\)\)/);
  assert.match(script, /sleep\s+0\.[0-9]+/);
});

test('install script prints diagnostics when extension is still not discoverable', () => {
  const script = readScript();
  assert.match(script, /gnome-shell --version/);
  assert.match(script, /gnome-extensions list/);
  assert.match(script, /not discoverable/);
});

test('install script uses XDG data home and has force-install fallback', () => {
  const script = readScript();
  assert.match(script, /XDG_DATA_HOME/);
  assert.match(script, /gnome-extensions install --force/);
});

test('install script includes container or remote-session hint in final diagnostics', () => {
  const script = readScript();
  assert.match(script, /container, toolbox, distrobox, SSH, or other remote session/i);
});

test('install script includes detailed discovery diagnostics for session, dbus, and logs', () => {
  const script = readScript();
  assert.match(script, /diagnose_discovery_failure/);
  assert.match(script, /XDG_SESSION_TYPE/);
  assert.match(script, /DBUS_SESSION_BUS_ADDRESS/);
  assert.match(script, /ListExtensions/);
  assert.match(script, /journalctl --user/);
});

test('reload script installs extension then requests GNOME Shell reexec', () => {
  const script = readReloadScript();
  assert.match(script, /install-local\.sh/);
  assert.match(script, /org\.gnome\.Shell/);
  assert.match(script, /global\.reexec_self\(\)/);
});

test('install script also installs hop-hotkeyd companion service', () => {
  const script = fs.readFileSync(path.join(rootDir, 'scripts/install-hopd-local.sh'), 'utf8');
  assert.match(script, /hop-hotkeyd/);
  assert.match(script, /hop-hotkeyd\.service/);
  assert.match(script, /HOP_LAUNCHER_CONTROL_SOCKET/);
});

test('install script prints compositor binding helper guidance', () => {
  const script = fs.readFileSync(path.join(rootDir, 'scripts/install-hopd-local.sh'), 'utf8');
  assert.match(script, /print-bindings/);
  assert.match(script, /compositor-specific/i);
});

test('install script supports configurable global shortcut env override', () => {
  const script = fs.readFileSync(path.join(rootDir, 'scripts/install-hopd-local.sh'), 'utf8');
  assert.match(script, /HOP_LAUNCHER_SHORTCUT/);
});

test('install script configures GNOME custom keybinding for hop-hotkeyd trigger', () => {
  const script = fs.readFileSync(path.join(rootDir, 'scripts/install-hopd-local.sh'), 'utf8');
  assert.match(script, /org\.gnome\.settings-daemon\.plugins\.media-keys/);
  assert.match(script, /custom-keybindings/);
  assert.match(script, /hop-hotkeyd trigger --socket/);
});

test('install script includes KDE shortcut helper guidance', () => {
  const script = fs.readFileSync(path.join(rootDir, 'scripts/install-hopd-local.sh'), 'utf8');
  assert.match(script, /KDE/i);
  assert.match(script, /qdbus|qdbus6/);
});
