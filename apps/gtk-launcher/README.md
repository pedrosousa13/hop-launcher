# Hop Launcher GTK

Standalone GTK4/libadwaita launcher frontend scaffold for the `hopd` daemon.

## Commands

```bash
cargo test
cargo run
cargo run --features gtk_ui
```

`cargo run` without features prints a scaffold message (no GTK system dependency required).
Use `--features gtk_ui` to launch the native GTK/libadwaita window.
