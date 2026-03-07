# Global Shortcut Solidness Design

Date: 2026-03-07
Status: Approved

## Goal

Deliver reliable global shortcut behavior across all supported Linux environments with `hop-hotkeyd` as the single source of truth for shortcut configuration.

## Scope

- In scope:
  - Daemon-owned shortcut config (`hop-hotkeyd` authoritative)
  - Cross-compositor backend application paths (GNOME, KDE, Sway, Hyprland, X11)
  - Health and diagnostics hardening (`status`, `doctor --strict`)
  - CI/release gates for shortcut reliability
- Out of scope:
  - New UI surfaces beyond wiring existing clients to daemon config APIs
  - Non-Linux platforms

## Architecture

`hop-hotkeyd` is the authoritative shortcut service. Shortcut state is persisted in daemon config (for example `~/.config/hop/hotkeyd.toml`) and exposed via daemon RPC:

- `hotkey.config.get`
- `hotkey.config.set`

Backends consume the canonical shortcut and apply compositor-native binding/bridge behavior:

- `x11`: key grab loop with updated key/modifier mapping
- `gnome`/`kde`/`sway`/`hyprland`: bridge or compositor binding path that ultimately triggers daemon toggle flow

GNOME extension preferences become a client of daemon config APIs instead of owning shortcut state.

## Components and Data Flow

1. Shortcut write path
- Client sends `hotkey.config.set`
- Daemon validates + normalizes accelerator
- Daemon atomically persists config
- Daemon applies backend wiring
- Response includes structured state:
  - `applied` (boolean)
  - `requires_manual_step` (boolean)
  - `warnings` (string list)

2. Shortcut read path
- Client sends `hotkey.config.get`
- Returned value is rendered directly by UI/CLI clients

3. Trigger path
- Backend receives global shortcut event
- Backend emits internal trigger event
- Daemon sends launcher toggle request to control socket (`ui.toggle`)
- Daemon records structured trigger metadata (backend/source/latency)

4. Health path
- `status` reports backend, configured shortcut, and bind/apply state
- `doctor --strict` returns non-zero when shortcut is invalid/unapplied or control socket is unreachable

## Error Handling and Reliability Rules

- `hotkey.config.set` hard-fails on invalid accelerator input
- Modifier-only or unsupported combinations are rejected
- Backend apply failures are explicit and never silent
- Daemon keeps last known good active binding on apply failure
- Config writes are atomic (temp file + fsync + rename)
- Config read corruption falls back safely and surfaces warning state
- Bridge/runtime failures are retried via backend watchdog logic
- Status surfaces `requires_manual_step` when fully automatic apply is not possible

## Testing and Release Gates

Unit tests:
- accelerator parsing/normalization/round-trip
- config load/save/corruption fallback
- backend mapping from canonical shortcut to compositor form

Integration tests:
- `hotkey.config.set/get` contract
- apply-path structured outcome assertions
- `status` and `doctor --strict` schema + behavior assertions

End-to-end tests:
- daemon trigger-to-toggle IPC path
- mocked bridge paths for GNOME/KDE/Sway/Hyprland
- X11 keygrab modifier variant behavior

CI/release policy:
- Hotkey test matrix is required for merge/release
- Release smoke checks include `doctor --strict` in target environments

## Accepted Decisions

- Source of truth: daemon-owned (`hop-hotkeyd`)
- “Works” definition allows compositor bridge/binding paths when native capture is unavailable
- Reliability bar includes automated CI matrix for config, apply, diagnostics, and end-to-end trigger flow
