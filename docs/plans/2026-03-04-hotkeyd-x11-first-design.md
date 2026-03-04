# Hotkeyd X11-First Design

## Goal
Deliver real global hotkey capture on X11 in `hop-hotkeyd` while keeping Wayland on explicit fallback mode.

## Scope
- Add daemon runtime mode to `hop-hotkeyd` (default command).
- Implement X11 key grab/event loop for `Ctrl+Shift+&` hotkey.
- On hotkey press, send `ui.toggle` to GTK control socket.
- Keep existing `trigger` command for manual and Wayland fallback testing.

## Non-Goals
- Native Wayland global key capture in this slice.
- Advanced shortcut customization UI.

## Architecture
- `hop-hotkeyd` command parser returns either:
  - `Run` (default, daemon mode)
  - `Trigger` (one-shot existing behavior)
- Backend selection uses `XDG_SESSION_TYPE`.
  - `x11` => start X11 grab/event loop.
  - `wayland` => fallback mode logs guidance and stays alive.
- X11 event loop sends `ui.toggle` through existing control socket client.

## Testing
- Unit tests for command parsing (`Run` default / `Trigger` override).
- Unit tests for session/backend mapping remain in place.
- Manual verification on X11 via running `hop-hotkeyd` and pressing shortcut.
