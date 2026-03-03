# Cross-Linux Hop Launcher Plan

This branch introduces the first implementation slice of the Linux-general architecture:

1. `hopd` Rust daemon crate with IPC contract scaffolding.
2. Core methods wired for `health.ping`, `search.query`, `actions.execute`, `config.get`, and `config.set`.
3. Local install workflow for daemon development and manual testing.

## Next Milestones

1. Replace placeholder `search.query` with real provider aggregation and ranking.
2. Add Unix socket IPC contract tests for end-to-end client/server exchange.
3. Integrate GNOME adapter against daemon socket calls.
4. Add KDE adapter prototype.

## Local Install (Required for Adapter Integration)

- Install locally with `npm run install:hopd:local`.
- Validate daemon with `systemctl --user status hopd.service`.
- Use `echo '{"id":"1","method":"health.ping"}' | socat - UNIX-CONNECT:${XDG_RUNTIME_DIR}/hopd.sock`.
