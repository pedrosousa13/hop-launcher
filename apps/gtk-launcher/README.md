# Hop Launcher GTK

Standalone GTK4/libadwaita launcher frontend scaffold for the `hopd` daemon.

## Commands

```bash
cargo test
cargo run
cargo run --features gtk_ui
```

`cargo run` without features prints a scaffold message (no GTK system dependency required).
Use `--features gtk_ui` to launch the native GTK/libadwaita app.

## Phase 1 behavior

- Background-style app process with a toggleable launcher window.
- Fallback accelerator: `Ctrl+Shift+&` (`<Primary><Shift>ampersand`) registered as app action.
- Phase 1 starts with the window visible (app-local shortcut; not compositor-global yet).
- Query typing calls `hopd` `search.query`.
- Enter (or row activation) calls `hopd` `actions.execute`.
- Minimal status chrome: only transient `Searching...` and explicit errors are shown.
- Settings shortcut parity: `Ctrl+,` / `Super+,` / `Meta+,` opens launcher settings.
- Control socket accepts `ui.toggle` requests at `${XDG_RUNTIME_DIR}/hop-launcher-control.sock`.

This phase focuses on standalone usability and IPC integration.
X11 global shortcut capture is now provided by the companion `hop-hotkeyd` service.
Sway and Hyprland Wayland support daemon-driven toggle via compositor event sockets.
KDE and GNOME Wayland support DBus bridge paths.
Unknown/unsupported Wayland compositors remain on trigger fallback.

## Phase 2 local toggle probe

After starting the app, trigger external toggle with:

```bash
~/.local/bin/hop-hotkeyd trigger
```

Inspect active backend mode/capabilities with:

```bash
~/.local/bin/hop-hotkeyd status
```

Configure compositor shortcut wiring (GNOME/KDE helper path) with:

```bash
~/.local/bin/hop-hotkeyd setup-shortcut
```

If your control socket path is customized:

```bash
~/.local/bin/hop-hotkeyd status --socket /tmp/hop-launcher-control.sock
```

For compositor-specific testing overrides:

```bash
~/.local/bin/hop-hotkeyd status --compositor sway
~/.local/bin/hop-hotkeyd status --compositor kde
~/.local/bin/hop-hotkeyd status --compositor gnome
```

Fail fast in scripts when control socket should already be available:

```bash
~/.local/bin/hop-hotkeyd doctor --strict
```

Print compositor-specific binding snippets (for Wayland setups) with:

```bash
~/.local/bin/hop-hotkeyd print-bindings
```

If you run the GTK control socket on a custom path:

```bash
~/.local/bin/hop-hotkeyd print-bindings --socket /tmp/hop-launcher-control.sock
```
