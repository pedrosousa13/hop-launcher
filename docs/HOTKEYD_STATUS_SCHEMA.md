# hop-hotkeyd Status and Doctor Schema

This document defines the structured JSON output fields for:

- `hop-hotkeyd status`
- `hop-hotkeyd doctor`

## `status` payload

### Common fields

- `session_type` (`string`): raw `XDG_SESSION_TYPE` input.
- `backend` (`"x11" | "wayland" | "unknown"`): selected high-level backend family.
- `configured_shortcut` (`string`): daemon-owned canonical accelerator value.
- `global_hotkey_supported` (`boolean`): whether native global capture/bridge is currently available.
- `applied` (`boolean`): whether the current backend can apply/use the configured shortcut now.
- `requires_manual_step` (`boolean`): whether operator action is required to finish wiring.
- `warnings` (`string[]`): non-fatal issues (config read, backend readiness, etc.).

### X11-specific fields

- `mode` (`"daemon"`): daemon runtime mode.

### Wayland-specific fields

- `fallback` (`string`): fallback one-shot command name (`hop-hotkeyd trigger`).
- `wayland_compositor` (`"sway" | "hyprland" | "kde" | "gnome" | "unknown"`): detected or overridden compositor.
- `wayland_backend_mode` (`"sway_tick" | "hyprland_event" | "kde_dbus_bridge" | "gnome_shell_bridge" | "fallback"`): selected Wayland backend path.
- `native_backend_ready` (`boolean`): backend prerequisites available now.
- `native_backend_socket` (`string | null`): socket path for socket-based native modes (sway/hyprland), else `null`.
- `native_backend_error` (`string | null`): readiness/probe error details when unavailable.
- `recommended_binding` (`string | null`): compositor-specific shortcut/bridge command hint.
- `next_step` (`string`): operator guidance for enabling native behavior.

### Control probe fields (attached by `status --socket ...` path)

- `control_socket_path` (`string`): resolved GTK control socket path.
- `control_socket_reachable` (`boolean`): socket connect/probe reachability.
- `control_ping_supported` (`boolean`): whether `ui.ping` is supported.
- `control_probe_status` (`"healthy" | "reachable_no_ping" | "unreachable"`).
- `control_probe_error` (`string | null`): reachability/probe error.

## `doctor` payload

`doctor` wraps a status snapshot and probe metadata:

- `status` (`object`): same structure as `status`.
- `control_socket_path` (`string`)
- `control_socket_reachable` (`boolean`)
- `control_ping_supported` (`boolean`)
- `control_probe_status` (`"healthy" | "reachable_no_ping" | "unreachable"`)
- `control_probe_error` (`string | null`)
- `doctor_wait_seconds` (`number`)
- `doctor_interval_ms` (`number`)
- `doctor_strict` (`boolean`)

If `--strict` is enabled and either:

- control socket is unreachable, or
- status reports `applied: false`

then `doctor` exits non-zero.
