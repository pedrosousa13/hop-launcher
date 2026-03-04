#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_SCRIPT="$ROOT_DIR/apps/gnome-extension/scripts/install-hopd-local.sh"
GTK_DIR="$ROOT_DIR/apps/gtk-launcher"
SOCKET_PATH="${HOPD_SOCKET:-${XDG_RUNTIME_DIR:-/tmp}/hopd.sock}"
CONTROL_SOCKET_PATH="${HOP_LAUNCHER_CONTROL_SOCKET:-${XDG_RUNTIME_DIR:-/tmp}/hop-launcher-control.sock}"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Required command not found: $1" >&2
    exit 1
  fi
}

wait_for_socket() {
  local attempts=50
  local delay_s=0.1
  local i
  for ((i = 1; i <= attempts; i++)); do
    if [[ -S "$SOCKET_PATH" ]]; then
      return 0
    fi
    sleep "$delay_s"
  done

  echo "hopd socket not found at: $SOCKET_PATH" >&2
  echo "Try: systemctl --user status hopd.service" >&2
  return 1
}

check_hopd_health() {
  if command -v python3 >/dev/null 2>&1; then
    python3 - "$SOCKET_PATH" <<'PY'
import json
import socket
import sys

socket_path = sys.argv[1]
request = {"id": "manual-test", "method": "health.ping"}

sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
try:
    sock.settimeout(2.0)
    sock.connect(socket_path)
    payload = (json.dumps(request) + "\n").encode("utf-8")
    sock.sendall(payload)
    response = sock.recv(4096).decode("utf-8").strip()
    parsed = json.loads(response)
    ok = parsed.get("result", {}).get("ok") is True
    if not ok:
        raise RuntimeError(f"unexpected response: {response}")
    print("hopd health check OK")
finally:
    sock.close()
PY
    return
  fi

  if command -v socat >/dev/null 2>&1; then
    local response
    response="$(printf '{"id":"manual-test","method":"health.ping"}\n' | socat - UNIX-CONNECT:"$SOCKET_PATH")"
    if [[ "$response" != *'"ok":true'* ]]; then
      echo "hopd health check failed: $response" >&2
      return 1
    fi
    echo "hopd health check OK"
    return
  fi

  echo "Need python3 or socat to run hopd health check" >&2
  return 1
}

require_cmd cargo
require_cmd systemctl

if [[ ! -x "$INSTALL_SCRIPT" ]]; then
  echo "Install script not found or not executable: $INSTALL_SCRIPT" >&2
  exit 1
fi

echo "Installing/updating local hopd service..."
"$INSTALL_SCRIPT"

echo "Waiting for hopd socket at $SOCKET_PATH..."
wait_for_socket

echo "Checking hopd health..."
check_hopd_health

echo "Checking hop-hotkeyd service..."
systemctl --user status hop-hotkeyd.service >/dev/null 2>&1 || {
  echo "hop-hotkeyd.service is not active" >&2
  echo "Try: systemctl --user status hop-hotkeyd.service" >&2
  exit 1
}

echo "hotkey backend status:"
~/.local/bin/hop-hotkeyd status || true

echo
echo "Manual test checklist:"
echo "- App starts visible."
echo "- X11: Ctrl+Shift+& should toggle globally via hop-hotkeyd.service."
echo "- Wayland fallback: ~/.local/bin/hop-hotkeyd trigger --socket $CONTROL_SOCKET_PATH"
echo "- Type: weather zurich / time in tokyo / emoji smile"
echo "- Press Enter on a result to trigger actions.execute"
echo

echo "Launching GTK app..."
cd "$GTK_DIR"
exec cargo run --features gtk_ui
