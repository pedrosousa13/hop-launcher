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

## Milestone Status (2026-03-04)

Completed in this branch:

1. Expanded `hopd` search rows to normalized metadata (`subtitle`, `icon`, `primary_action`, `score`).
2. Added parity result-kind scaffolding for `app`, `window`, `file`, `recent`, `setting`, and `utility`.
3. Added `search.query` mode routing (`apps`, `windows`, `files`, `recents`, `settings`, plus utility modes).
4. Updated `actions.execute` response shape with execution metadata (`result_id`, `action`).
5. Updated GTK UI status flow to report contextual states (`Searching...`, `<n> results`, `No results`, `Error`).
6. Added staged GNOME convergence toggle (`feature-hopd-convergence-enabled`) for non-utility `hopd` routes.

## Next Milestones

1. Expand `search.query` providers from catalog heuristics to real data-backed provider modules.
2. Replace deterministic provider scaffolds with real desktop-backed data sources and execution handlers.
3. Add KDE transport integration (actual socket client path), not only request-contract scaffolding.
4. Add daemon benchmarks and latency telemetry assertions for non-utility + utility-intent queries.

## Local Install (Required for Adapter Integration)

- Install locally with `cd apps/gnome-extension && npm run install:hopd:local`.
- Validate daemon with `systemctl --user status hopd.service`.
- Use `echo '{"id":"1","method":"health.ping"}' | socat - UNIX-CONNECT:${XDG_RUNTIME_DIR}/hopd.sock`.
