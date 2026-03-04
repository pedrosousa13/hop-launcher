# Global Hotkey Agent Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add Phase 2 companion hotkey agent plumbing and GTK control IPC so launcher visibility can be toggled externally.

**Architecture:** Add a small control IPC contract to the GTK launcher library and run a control socket listener in the GTK app; introduce a dedicated `hop-hotkeyd` Rust crate that selects Wayland/X11 backend mode and can send `ui.toggle` requests.

**Tech Stack:** Rust, GTK4/libadwaita, Unix sockets, serde/serde_json, systemd user units (local install scripts).

---

### Task 1: Add failing GTK control IPC tests

**Files:**
- Modify: `apps/gtk-launcher/src/lib.rs`

**Step 1: Write failing tests**
- Add tests for:
  - default control socket path construction
  - parsing `ui.toggle` control request
  - building success/error response payloads

**Step 2: Run test to verify it fails**
- Run: `cd apps/gtk-launcher && cargo test control_`

### Task 2: Implement GTK control IPC contract helpers

**Files:**
- Modify: `apps/gtk-launcher/src/lib.rs`

**Step 1: Add control IPC data types and helpers**
- `default_control_socket_path()`
- request parser for `ui.toggle`
- response builders for `ok` and JSON-RPC-style error

**Step 2: Re-run tests**
- Run: `cd apps/gtk-launcher && cargo test control_`

### Task 3: Wire GTK app control socket listener

**Files:**
- Modify: `apps/gtk-launcher/src/main.rs`

**Step 1: Add background Unix listener**
- Bind control socket path at startup.
- Parse incoming request and dispatch toggle command to GTK main thread.

**Step 2: Handle cleanup and non-fatal socket errors**
- Remove stale socket before bind.
- Keep app responsive if request parse fails.

**Step 3: Verify compile/tests**
- Run:
  - `cd apps/gtk-launcher && cargo test`
  - `cd apps/gtk-launcher && cargo test --features gtk_ui`

### Task 4: Add failing `hop-hotkeyd` agent tests

**Files:**
- Create: `crates/hop-hotkeyd/Cargo.toml`
- Create: `crates/hop-hotkeyd/src/lib.rs`
- Create: `crates/hop-hotkeyd/src/main.rs`

**Step 1: Write failing tests**
- backend selection from `XDG_SESSION_TYPE`
- `ui.toggle` request generation
- Unix socket roundtrip helper handles success response

**Step 2: Run test to verify it fails**
- Run: `cd crates/hop-hotkeyd && cargo test`

### Task 5: Implement minimal `hop-hotkeyd` agent core

**Files:**
- Modify: `crates/hop-hotkeyd/src/lib.rs`
- Modify: `crates/hop-hotkeyd/src/main.rs`

**Step 1: Implement backend mode selection**
- Map session to `Wayland`, `X11`, or unsupported error.

**Step 2: Implement control socket client**
- Send `ui.toggle` JSON payload and verify response `ok`.

**Step 3: Implement CLI entrypoint**
- `hop-hotkeyd trigger --socket <path>` sends one toggle request.

**Step 4: Re-run tests**
- Run: `cd crates/hop-hotkeyd && cargo test`

### Task 6: Wire local installer + docs + end-to-end checks

**Files:**
- Modify: `apps/gnome-extension/scripts/install-hopd-local.sh`
- Modify: `scripts/run-gtk-manual-test.sh`
- Modify: `README.md`
- Modify: `apps/gtk-launcher/README.md`

**Step 1: Extend local install script**
- Build/install `hop-hotkeyd` binary.
- Install `hop-hotkeyd.service` with environment for control socket path.

**Step 2: Extend manual runner flow**
- Verify `hop-hotkeyd` binary/service availability.
- Print command to test toggle via `hop-hotkeyd trigger`.

**Step 3: Run all verification commands**
- `cd apps/gnome-extension && npm test`
- `cd crates/hopd && cargo test`
- `cd apps/gtk-launcher && cargo test`
- `cd apps/gtk-launcher && cargo test --features gtk_ui`
- `cd crates/hop-hotkeyd && cargo test`

**Step 4: Commit checkpoint**
- Commit message: `feat(gtk): add phase-2 control socket and hotkey agent scaffold`
