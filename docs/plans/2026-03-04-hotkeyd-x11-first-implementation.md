# Hotkeyd X11-First Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add default daemon mode to `hop-hotkeyd` and implement real X11 global hotkey capture for launcher toggle.

**Architecture:** Keep reusable control socket client in `lib.rs` and move runtime/CLI parsing into `main.rs` with an X11 event-loop helper.

**Tech Stack:** Rust, x11rb, Unix sockets, serde_json.

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
