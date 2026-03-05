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

## Milestone Status (2026-03-05)

Completed in this branch:

1. Extended `actions.execute` contract with explicit command-resolution metadata (`resolved_command`, `resolved_args`) plus `success` semantics in integration tests.
2. Added real KDE-side socket transport helper (`request_hopd_search_over_socket`) and integration tests against a Unix socket listener.
3. Added telemetry assertions that validate `elapsed_ms` for both non-utility and utility-intent `search.query` paths.
4. Added local daemon benchmark entrypoint (`cargo run --bin hopd-bench -- <iterations>`) reporting mean/p95/max search latency for utility and app queries.
5. Expanded benchmark coverage with cold + warm latency reporting and file-search scale scenarios (`files-200`, `files-1200`) using indexed-folder test corpora.
6. Added CI benchmark artifact upload (`hopd-bench`) from `ci.yml` so each run stores a searchable latency snapshot.
7. Reworked settings provider from static catalog to desktop- and command-aware rows (`gnome-control-center`, `systemsettings5/systemsettings`) with fallback behavior.
8. Improved app provider desktop-entry parsing with XDG data directory discovery, hidden/no-display filtering, localized-name fallback, and cleaned `Exec` keyword extraction.
9. Improved recents provider parsing to consume bookmark metadata and order rows by `modified` timestamp (most recent first).
10. Replaced utility placeholder provider with explicit utility catalog rows (calculator/currency/weather/timezone/emoji) plus intent-term filtering.
11. Extended KDE adapter transport with `actions.execute` socket requests and added CLI execute mode (`kde-hopd-query --execute <result_id>`).
12. Added CI benchmark guardrail enforcement (`check-bench-thresholds.sh`) so p95 latency regressions fail the `hopd` job.
13. Replaced static settings rows with desktop-entry-backed settings discovery and `settingcmd:` execution wiring for real settings launch commands.
14. Extended KDE adapter query transport with explicit route-mode support (`kde-hopd-query --mode <mode>`) so KDE surfaces can call full `hopd` providers, not utility-only intent.
15. Improved utility-intent coverage with dynamic weather/time location parsing (`weather <city>`, `<city> weather`, `<city> time`) and weather-location execution URLs.
16. Added benchmark summary/trend reporting script (`bench-trend-report.sh`) and CI artifact publication for markdown latency snapshots.
17. Wired CI baseline retrieval for `hopd-bench` artifacts so summary reports can include cross-run deltas automatically when prior artifacts exist.
18. Extended KDE helper CLI with runner output format (`--format runner`) for adapter-friendly tabular result consumption.
19. Upgraded explicit `utility ...` provider behavior from static filtering to intent-aware rows (calculator/currency/weather/timezone/emoji with location parsing).
20. Included `kde-hopd-query` in Debian package install targets so KDE adapter helper ships as an installable artifact.
21. Applied GTK minimal-noise status policy (silent steady-state status text; transient searching/errors only) and updated manual polish checklist.
22. Hardened CI/release artifact validation with reusable manifest verification script and enabled `gtk_ui` test coverage in workflow gates.

## Next Milestones

1. Expand `search.query` providers from catalog heuristics to real data-backed provider modules.
2. Replace remaining deterministic provider scaffolds with real desktop-backed data sources and execution handlers (utility catalog + provider data parity with GNOME).
3. Integrate KDE transport helper into a packaged KDE launcher/runner adapter surface (Plasma-facing artifact + install docs).
4. Add packaged KDE runner/launcher adapter artifact and install path (beyond CLI helper) for Plasma integration.

## Parity Matrix

- Locked cross-frontend parity matrix: `docs/plans/2026-03-05-parity-matrix.md`

## Local Install (Required for Adapter Integration)

- Install locally with `cd apps/gnome-extension && npm run install:hopd:local`.
- Validate daemon with `systemctl --user status hopd.service`.
- Use `echo '{"id":"1","method":"health.ping"}' | socat - UNIX-CONNECT:${XDG_RUNTIME_DIR}/hopd.sock`.
