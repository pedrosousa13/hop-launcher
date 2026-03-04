# Hotkeyd X11-First Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add default daemon mode to `hop-hotkeyd` and implement real X11 global hotkey capture for launcher toggle.

**Architecture:** Keep reusable control socket client in `lib.rs` and move runtime/CLI parsing into `main.rs` with an X11 event-loop helper.

**Tech Stack:** Rust, x11rb, Unix sockets, serde_json.

## Status Snapshot (Updated 2026-03-04)

This plan is fully implemented, and the branch has progressed beyond the original X11-first scope.

### Original Plan Tasks

- [x] Task 1: Add failing command parsing tests
- [x] Task 2: Implement command parsing and runtime dispatch
- [x] Task 3: Implement X11 runtime backend and Wayland fallback
- [x] Task 4: Update docs/manual testing and verify full suite

### Completed Beyond Original Scope

- Added control socket diagnostics and health classification:
  - `hop-hotkeyd status`
  - `hop-hotkeyd doctor` (+ wait/interval/retry options)
  - `hop-hotkeyd doctor --strict` for CI/script fail-fast behavior
- Added compositor-aware tooling:
  - `hop-hotkeyd print-bindings` (+ `--compositor`, `--socket`)
  - `hop-hotkeyd setup-shortcut` (+ `--compositor`, `--shortcut`, `--socket`, `--dry-run`)
  - `status` and `doctor` now support `--compositor` override for testing
  - `status` now supports `--socket` override and embeds control probe results
  - `status` now emits `recommended_binding` for detected mode
- Added native Wayland daemon backends:
  - Sway tick backend (`send_tick hop-launcher-toggle`)
  - Hyprland event backend (`dispatch event hop-launcher-toggle`)
  - KDE DBus bridge backend (KGlobalAccel/DBus monitor path)
  - GNOME DBus bridge backend (Shell/API signal bridge path)
- Added native Wayland readiness diagnostics:
  - `native_backend_ready`
  - `native_backend_socket`
  - `native_backend_error`
  - `wayland_backend_mode` (`sway_tick`, `hyprland_event`, `kde_dbus_bridge`, `gnome_shell_bridge`, `fallback`)
- Added feasible integration-level Wayland loop coverage:
  - sway IPC frame read/write roundtrip tests
  - invalid sway frame rejection tests
  - KDE/GNOME DBus monitor line parser tests
- Added structured schema documentation:
  - `docs/HOTKEYD_STATUS_SCHEMA.md` for `status` and `doctor` outputs
- Updated manual and installer guidance:
  - `scripts/run-gtk-manual-test.sh` now prints richer hotkey diagnostics
  - installer now supports shortcut override (`HOP_LAUNCHER_SHORTCUT`) and GNOME custom-keybinding setup
  - installer output includes KDE bridge helper and dependency hints

### Recent Continuation Commits (Newest First)

- `48e2b6e` feat(hotkeyd): add strict doctor mode
- `0acae04` feat(hotkeyd): add compositor override for status and doctor
- `f752c71` feat(hotkeyd): add status socket override and probe output
- `972e472` feat(hotkeyd): support custom socket in binding snippets
- `0d02273` feat(hotkeyd): add recommended binding in status output
- `ab7a9fd` feat(hotkeyd): add wayland native readiness diagnostics
- `e6eb98b` feat(hotkeyd): add hyprland event backend for wayland toggle
- `19de3d8` feat(hotkeyd): add sway tick backend for wayland toggle

### Continuation Backlog (Next Phase Candidates)

- [x] Add KDE-native global shortcut backend (KGlobalAccel/DBus path)
- [x] Add GNOME-native global shortcut backend (Shell extension/API bridge path)
- [x] Add end-to-end integration tests for native Wayland event loops (where feasible)
- [x] Optional: expose structured machine-readable status/doctor schema in docs

---

### Task 1: Add failing command parsing tests

**Files:**
- Modify: `crates/hop-hotkeyd/src/main.rs`

**Step 1: Write failing tests**
- no args => `Run`
- `trigger --socket /tmp/x.sock` => `Trigger` with socket

**Step 2: Run test to verify it fails**
- `cd crates/hop-hotkeyd && cargo test parse_`

### Task 2: Implement command parsing and runtime dispatch

**Files:**
- Modify: `crates/hop-hotkeyd/src/main.rs`

**Step 1: Add `Command` enum + parser**
**Step 2: Dispatch `Run` vs `Trigger` in `run()`**
**Step 3: Re-run tests**
- `cd crates/hop-hotkeyd && cargo test parse_`

### Task 3: Implement X11 runtime backend and Wayland fallback

**Files:**
- Modify: `crates/hop-hotkeyd/Cargo.toml`
- Modify: `crates/hop-hotkeyd/src/main.rs`

**Step 1: Add x11rb dependency**
**Step 2: Add X11 key-grab + event loop**
- grab `Ctrl+Shift+&` key
- on key press, call `send_toggle`

**Step 3: Wayland fallback behavior**
- log fallback guidance and keep service alive (non-crashing)

**Step 4: Run tests**
- `cd crates/hop-hotkeyd && cargo test`

### Task 4: Update docs/manual testing and verify full suite

**Files:**
- Modify: `README.md`
- Modify: `apps/gtk-launcher/README.md`
- Modify: `scripts/run-gtk-manual-test.sh`

**Step 1: Document X11-first behavior and Wayland fallback**
**Step 2: Add manual probe instructions**
**Step 3: Run verification**
- `cd apps/gnome-extension && npm test`
- `cd crates/hopd && cargo test`
- `cd apps/gtk-launcher && cargo test`
- `cd apps/gtk-launcher && cargo test --features gtk_ui`
- `cd crates/hop-hotkeyd && cargo test`

**Step 4: Commit**
- `feat(hotkeyd): add x11 global hotkey runtime with wayland fallback`
