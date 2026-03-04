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
