# Hop Launcher Monorepo

This repository now contains multiple Linux launcher components in one codebase.

## Layout

- `apps/gnome-extension` - full GNOME Shell extension implementation
- `apps/gtk-launcher` - standalone GTK4/libadwaita launcher frontend scaffold
- `crates/hopd` - Rust daemon (`hopd`) serving launcher IPC methods
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

### GTK launcher

```bash
cd apps/gtk-launcher
cargo test
cargo run
cargo run --features gtk_ui
```

## Local hopd install (with GNOME integration)

```bash
cd apps/gnome-extension
npm run install:hopd:local
```

See `docs/HOPD_LOCAL_INSTALL.md` for full details.
