# Global Hotkey Agent Design

## Goal
Implement Phase 2 global launcher activation for both Wayland and X11 by introducing a companion hotkey agent that can toggle the GTK launcher window.

## Scope
- Add a new Rust companion daemon crate: `crates/hop-hotkeyd`.
- Keep `apps/gtk-launcher` focused on UI + `hopd` integration.
- Add a local Unix socket control channel between agent and GTK app.
- Support backend selection for Wayland/X11 session types.
- Extend local install/testing workflow to start both `hopd` and `hop-hotkeyd`.

## Non-Goals
- Compositor-specific deep integrations beyond basic backend abstraction.
- Replacing existing GNOME extension shortcut flow.
- Advanced multi-seat keyboard handling.

## Architecture
### Components
1. `crates/hop-hotkeyd`
- Captures global shortcut events via backend abstraction.
- Emits `ui.toggle` control message to GTK control socket.
- Runs as user daemon (`hop-hotkeyd.service`).

2. `apps/gtk-launcher`
- Hosts control socket server and handles control requests.
- Maintains app-local fallback shortcut for development.
- On `ui.toggle`, toggles launcher window visibility.

3. Installer/testing scripts
- Install/update `hop-hotkeyd` binary and service.
- Validate both daemons and launcher control path.

### IPC Contract (hotkey agent -> GTK)
Unix socket: `${XDG_RUNTIME_DIR}/hop-launcher-control.sock`

Request:
```json
{"id":"hk-1","method":"ui.toggle"}
```

Response:
```json
{"id":"hk-1","result":{"ok":true}}
```

Error shape mirrors existing JSON-RPC-like style used by `hopd` integration.

## Data Flow
1. `hop-hotkeyd` starts and selects backend from session type (`wayland` or `x11`).
2. User presses configured shortcut.
3. `hop-hotkeyd` sends `ui.toggle` request to GTK control socket.
4. GTK app toggles window visibility and returns `ok` response.

## Backend Strategy
- Define `GlobalHotkeyBackend` trait in `hop-hotkeyd`.
- Start with stub backend event source for deterministic tests.
- Add platform adapters behind feature flags:
  - `wayland_backend`
  - `x11_backend`
- Runtime backend choice based on `XDG_SESSION_TYPE`, with clear logs/errors.

## Error Handling
- Missing control socket: agent logs and retries with backoff.
- Invalid control response: agent reports error and continues listening.
- Unsupported session type: agent exits with actionable message.
- GTK control socket bind failure: launcher exits with explicit error.

## Testing Strategy
- Unit tests for control message encode/decode.
- Unit tests for backend selection logic.
- GTK launcher tests for control request parsing and toggle action handling.
- Integration test for agent sending `ui.toggle` to a fake control socket server.
- Existing project verification suites remain required.

## Rollout Plan
1. Ship local/dev path first (manual test command).
2. Keep app-local shortcut fallback available.
3. Iterate backend implementations while preserving IPC contract.
