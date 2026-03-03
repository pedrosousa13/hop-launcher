#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOPD_DIR="$ROOT_DIR/hopd"
INSTALL_BIN_DIR="${HOME}/.local/bin"
INSTALL_BIN_PATH="${INSTALL_BIN_DIR}/hopd"
SYSTEMD_USER_DIR="${HOME}/.config/systemd/user"
SERVICE_PATH="${SYSTEMD_USER_DIR}/hopd.service"
SOCKET_PATH="${XDG_RUNTIME_DIR:-/tmp}/hopd.sock"
ENABLE_SERVICE=1
DRY_RUN=0

print_usage() {
    cat <<'USAGE'
Usage: scripts/install-hopd-local.sh [options]

Options:
  --dry-run      Print actions without changing the system
  --no-enable    Install service but do not enable/start it
  -h, --help     Show this help
USAGE
}

run_cmd() {
    if [[ "$DRY_RUN" -eq 1 ]]; then
        echo "[dry-run] $*"
    else
        "$@"
    fi
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run)
            DRY_RUN=1
            shift
            ;;
        --no-enable)
            ENABLE_SERVICE=0
            shift
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            print_usage
            exit 1
            ;;
    esac
done

if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is required but not found in PATH" >&2
    exit 1
fi

run_cmd cargo build --release --manifest-path "${HOPD_DIR}/Cargo.toml"
run_cmd mkdir -p "${INSTALL_BIN_DIR}"
run_cmd install -m 0755 "${HOPD_DIR}/target/release/hopd" "${INSTALL_BIN_PATH}"
run_cmd mkdir -p "${SYSTEMD_USER_DIR}"

if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "[dry-run] write ${SERVICE_PATH}"
else
    cat > "${SERVICE_PATH}" <<EOF
[Unit]
Description=Hop Launcher Daemon (hopd)
After=graphical-session.target

[Service]
Type=simple
ExecStart=${INSTALL_BIN_PATH} --socket ${SOCKET_PATH}
Restart=on-failure
RestartSec=1
Environment=HOPD_SOCKET=${SOCKET_PATH}

[Install]
WantedBy=default.target
EOF
fi

run_cmd systemctl --user daemon-reload
if [[ "$ENABLE_SERVICE" -eq 1 ]]; then
    run_cmd systemctl --user enable --now hopd.service
else
    echo "Installed hopd.service but did not enable/start it (--no-enable)."
fi

echo "hopd installed at ${INSTALL_BIN_PATH}"
echo "systemd unit installed at ${SERVICE_PATH}"
echo "socket path: ${SOCKET_PATH}"
