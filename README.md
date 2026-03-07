# Hop Launcher Monorepo

This repository now contains multiple Linux launcher components in one codebase.

## Layout

- `apps/gnome-extension` - full GNOME Shell extension implementation
- `apps/gtk-launcher` - standalone GTK4/libadwaita launcher frontend scaffold
- `crates/hopd` - Rust daemon (`hopd`) serving launcher IPC methods
- `crates/hop-hotkeyd` - companion Rust daemon for global hotkey -> GTK toggle IPC
- `docs` - shared architecture and planning docs

## Install options

- Local/dev install (recommended while iterating):
  - `cd apps/gnome-extension && npm run install:hopd:local`
- Release artifacts:
  - GNOME extension zip is published by GitHub Actions release workflow.
  - `hopd` and `hop-hotkeyd` Linux tarballs are also attached to release artifacts.
- Packaging/distribution docs:
  - APT + Linux packaging guide: `docs/LINUX_PACKAGING.md`
  - Maintainer release checklist: `docs/RELEASE.md`

## Per-folder commands

### GNOME extension

```bash
cd apps/gnome-extension
npm test
```

### hopd daemon

```bash
cd crates/hopd
cargo test
```

### hop-hotkeyd agent

```bash
cd crates/hop-hotkeyd
cargo test
```

### GTK launcher

```bash
cd apps/gtk-launcher
cargo test
cargo run
cargo run --features gtk_ui
```

## Local daemon install (with GNOME integration)

```bash
cd apps/gnome-extension
npm run install:hopd:local
```

This installs both `hopd` and `hop-hotkeyd` user services.
See `docs/HOPD_LOCAL_INSTALL.md` for full details.
You can override default shortcut during install with:

```bash
HOP_LAUNCHER_SHORTCUT='<Super>space' npm run install:hopd:local
```

## One-command local GTK manual test

```bash
./scripts/run-gtk-manual-test.sh
```

This command installs/updates `hopd`, checks daemon health over the Unix socket,
and launches the GTK app with `--features gtk_ui` for manual validation.

On X11 sessions, `hop-hotkeyd` runs a real global hotkey loop for `Ctrl+Shift+&`.
On Sway Wayland sessions, configure a binding with:

```bash
swaymsg -q -t send_tick hop-launcher-toggle
```

On Hyprland sessions, configure a binding with:

```bash
hyprctl dispatch event hop-launcher-toggle
```

On KDE Wayland sessions, `hop-hotkeyd` now supports a DBus bridge path.
You can manually trigger the expected action with:

```bash
qdbus org.kde.kglobalaccel /component/hoplauncher org.kde.kglobalaccel.Component.invokeShortcut hop-launcher-toggle
```

On GNOME Wayland sessions, `hop-hotkeyd` now supports a GNOME Shell/API DBus bridge path.
You can manually emit the expected bridge signal with:

```bash
gdbus emit --session --object-path /io/github/hop/Hotkeyd --signal io.github.hop.Hotkeyd.Toggle hop-launcher-toggle
```

Unknown/unsupported Wayland compositors still use the fallback one-shot command:

```bash
~/.local/bin/hop-hotkeyd trigger
```

Check detected backend/capabilities with:

```bash
~/.local/bin/hop-hotkeyd status
```

Daemon-owned shortcut config:

```bash
~/.local/bin/hop-hotkeyd config get
~/.local/bin/hop-hotkeyd config set --shortcut '<Super>space'
```

Configure desktop-native shortcut wiring (GNOME/KDE helpers) with:

```bash
~/.local/bin/hop-hotkeyd setup-shortcut
```

Auto-repair shortcut wiring based on current config/compositor with:

```bash
~/.local/bin/hop-hotkeyd repair-shortcut
```

For non-default control sockets:

```bash
~/.local/bin/hop-hotkeyd status --socket /tmp/hop-launcher-control.sock
```

Force compositor-specific status diagnostics when testing:

```bash
~/.local/bin/hop-hotkeyd status --compositor sway
~/.local/bin/hop-hotkeyd doctor --compositor hyprland
~/.local/bin/hop-hotkeyd status --compositor kde
~/.local/bin/hop-hotkeyd status --compositor gnome
```

`status` now also reports native Wayland readiness details (`native_backend_ready`, `native_backend_socket`, `native_backend_error`), a `recommended_binding` command for the detected compositor mode, plus `capability_matrix` and `shortcut_drift` diagnostics.
See `docs/HOTKEYD_STATUS_SCHEMA.md` for the structured `status`/`doctor` payload schema.

Run structured diagnostics (with optional wait/retry) with:

```bash
~/.local/bin/hop-hotkeyd doctor --wait-seconds 5 --interval-ms 200
```

Use `--strict` to make diagnostics exit non-zero when control socket is unreachable or shortcut apply state is not healthy:

```bash
~/.local/bin/hop-hotkeyd doctor --strict
```

Print compositor-specific binding snippets with:

```bash
~/.local/bin/hop-hotkeyd print-bindings
```

Use a custom control socket in snippets with:

```bash
~/.local/bin/hop-hotkeyd print-bindings --socket /tmp/hop-launcher-control.sock
```

## Maintainer release flow

Use `docs/RELEASE.md` for a complete checklist including:
- pre-release verification
- tag/release artifact flow
- Linux package publishing (APT and alternatives)
