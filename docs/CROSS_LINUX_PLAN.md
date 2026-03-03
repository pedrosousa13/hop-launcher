# Cross-Linux Hop Launcher Plan

This branch introduces the first implementation slice of the Linux-general architecture:

1. `hopd` Rust daemon crate with IPC contract scaffolding.
2. Core methods wired for `health.ping`, `search.query`, `actions.execute`, `config.get`, and `config.set`.
3. Local install workflow for daemon development and manual testing.

## Milestone Status (2026-03-03)

Completed in this branch:

1. Replaced placeholder `search.query` with provider-style aggregation and ranking.
2. Added Unix socket IPC contract tests for end-to-end multi-request exchange.
3. Integrated GNOME adapter against daemon socket calls.
4. Added a KDE adapter prototype contract helper.

## Next Milestones

1. Expand `search.query` providers from catalog heuristics to real data-backed provider modules.
2. Add KDE transport integration (actual socket client path), not only request-contract scaffolding.
3. Add daemon benchmarks and latency telemetry assertions for utility-intent queries.

## Local Install (Required for Adapter Integration)

- Install locally with `cd apps/gnome-extension && npm run install:hopd:local`.
- Validate daemon with `systemctl --user status hopd.service`.
- Use `echo '{"id":"1","method":"health.ping"}' | socat - UNIX-CONNECT:${XDG_RUNTIME_DIR}/hopd.sock`.
