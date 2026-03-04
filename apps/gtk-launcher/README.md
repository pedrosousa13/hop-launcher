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
- Control socket accepts `ui.toggle` requests at `${XDG_RUNTIME_DIR}/hop-launcher-control.sock`.

This phase focuses on standalone usability and IPC integration.
True compositor-wide global shortcut behavior is planned for the next phase.

## Phase 2 local toggle probe

After starting the app, trigger external toggle with:

```bash
~/.local/bin/hop-hotkeyd trigger
```
