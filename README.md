# Hop Launcher Monorepo

This repository now contains multiple Linux launcher components in one codebase.

## Layout

- `apps/gnome-extension` - full GNOME Shell extension implementation
- `apps/gtk-launcher` - standalone GTK4/libadwaita launcher frontend scaffold
- `crates/hopd` - Rust daemon (`hopd`) serving launcher IPC methods
- `crates/hop-hotkeyd` - companion Rust daemon for global hotkey -> GTK toggle IPC
- `docs` - shared architecture and planning docs

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

Other Wayland compositors still use the fallback one-shot command:

```bash
~/.local/bin/hop-hotkeyd trigger
```

Check detected backend/capabilities with:

```bash
~/.local/bin/hop-hotkeyd status
```

`status` now also reports native Wayland readiness details (`native_backend_ready`, `native_backend_socket`, `native_backend_error`) and a `recommended_binding` command for the detected compositor mode.

Run structured diagnostics (with optional wait/retry) with:

```bash
~/.local/bin/hop-hotkeyd doctor --wait-seconds 5 --interval-ms 200
```

Print compositor-specific binding snippets with:

```bash
~/.local/bin/hop-hotkeyd print-bindings
```

Use a custom control socket in snippets with:

```bash
~/.local/bin/hop-hotkeyd print-bindings --socket /tmp/hop-launcher-control.sock
```
