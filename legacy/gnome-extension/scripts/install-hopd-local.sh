#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HOPD_DIR="$ROOT_DIR/../../crates/hopd"
HOTKEYD_DIR="$ROOT_DIR/../../crates/hop-hotkeyd"
INSTALL_BIN_DIR="${HOME}/.local/bin"
INSTALL_BIN_PATH="${INSTALL_BIN_DIR}/hopd"
HOTKEYD_BIN_PATH="${INSTALL_BIN_DIR}/hop-hotkeyd"
SYSTEMD_USER_DIR="${HOME}/.config/systemd/user"
SERVICE_PATH="${SYSTEMD_USER_DIR}/hopd.service"
HOTKEYD_SERVICE_PATH="${SYSTEMD_USER_DIR}/hop-hotkeyd.service"
SOCKET_PATH="${XDG_RUNTIME_DIR:-/tmp}/hopd.sock"
CONTROL_SOCKET_PATH="${HOP_LAUNCHER_CONTROL_SOCKET:-${XDG_RUNTIME_DIR:-/tmp}/hop-launcher-control.sock}"
SHORTCUT_ACCEL="${HOP_LAUNCHER_SHORTCUT:-<Super>space}"
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

configure_gnome_shortcut() {
    local binding_path="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/hop-launcher-hotkeyd/"
    local schema="org.gnome.settings-daemon.plugins.media-keys"
    local list_key="custom-keybindings"
    local entry_schema="org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:${binding_path}"
    local trigger_cmd="${HOTKEYD_BIN_PATH} trigger --socket ${CONTROL_SOCKET_PATH}"

    if ! command -v gsettings >/dev/null 2>&1; then
        echo "GNOME detected but gsettings is not available; skipping shortcut auto-setup." >&2
        return
    fi

    local current
    current="$(gsettings get "${schema}" "${list_key}")"
    local desired="'${binding_path}'"
    if [[ "${current}" != *"${desired}"* ]]; then
        local updated
        if [[ "${current}" == "@as []" || "${current}" == "[]" ]]; then
            updated="[${desired}]"
        else
            updated="${current%]}"
            updated="${updated}, ${desired}]"
        fi
        run_cmd gsettings set "${schema}" "${list_key}" "${updated}"
    fi

    run_cmd gsettings set "${entry_schema}" name "Hop Launcher"
    run_cmd gsettings set "${entry_schema}" command "${trigger_cmd}"
    run_cmd gsettings set "${entry_schema}" binding "['${SHORTCUT_ACCEL}']"
    echo "Configured GNOME custom shortcut (${SHORTCUT_ACCEL}) -> ${trigger_cmd}"
    echo "Equivalent command form: hop-hotkeyd trigger --socket ${CONTROL_SOCKET_PATH}"
}

configure_kde_shortcut() {
    local trigger_cmd="${HOTKEYD_BIN_PATH} trigger --socket ${CONTROL_SOCKET_PATH}"
    if command -v qdbus6 >/dev/null 2>&1 || command -v qdbus >/dev/null 2>&1; then
        echo "KDE detected. Configure a global shortcut to run:"
        echo "  ${trigger_cmd}"
        echo "Optional trigger test via KGlobalAccel:"
        echo "  qdbus org.kde.kglobalaccel /component/hoplauncher org.kde.kglobalaccel.Component.invokeShortcut hop-launcher-toggle"
    else
        echo "KDE detected but qdbus/qdbus6 is missing; install qdbus (or qdbus6) for bridge diagnostics."
        echo "Configure a KDE custom shortcut command manually:"
        echo "  ${trigger_cmd}"
    fi
}

configure_desktop_shortcut() {
    local desktop
    desktop="$(printf '%s:%s' "${XDG_CURRENT_DESKTOP:-}" "${XDG_SESSION_DESKTOP:-}" | tr '[:upper:]' '[:lower:]')"
    if [[ "${desktop}" == *"gnome"* ]]; then
        configure_gnome_shortcut
    elif [[ "${desktop}" == *"kde"* || "${desktop}" == *"plasma"* ]]; then
        configure_kde_shortcut
    else
        echo "No GNOME/KDE desktop detected for shortcut auto-setup."
    fi
}

print_desktop_dependency_hints() {
    local desktop
    desktop="$(printf '%s:%s' "${XDG_CURRENT_DESKTOP:-}" "${XDG_SESSION_DESKTOP:-}" | tr '[:upper:]' '[:lower:]')"
    if [[ "${desktop}" == *"gnome"* ]]; then
        command -v gsettings >/dev/null 2>&1 || echo "hint: install gsettings for GNOME shortcut automation"
        command -v dbus-monitor >/dev/null 2>&1 || echo "hint: install dbus-monitor for GNOME bridge backend readiness"
    elif [[ "${desktop}" == *"kde"* || "${desktop}" == *"plasma"* ]]; then
        (command -v qdbus6 >/dev/null 2>&1 || command -v qdbus >/dev/null 2>&1) || echo "hint: install qdbus/qdbus6 for KDE bridge diagnostics"
        command -v dbus-monitor >/dev/null 2>&1 || echo "hint: install dbus-monitor for KDE bridge backend readiness"
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
run_cmd cargo build --release --manifest-path "${HOTKEYD_DIR}/Cargo.toml"
run_cmd mkdir -p "${INSTALL_BIN_DIR}"
run_cmd install -m 0755 "${HOPD_DIR}/target/release/hopd" "${INSTALL_BIN_PATH}"
run_cmd install -m 0755 "${HOTKEYD_DIR}/target/release/hop-hotkeyd" "${HOTKEYD_BIN_PATH}"
run_cmd mkdir -p "${SYSTEMD_USER_DIR}"

if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "[dry-run] write ${SERVICE_PATH}"
    echo "[dry-run] write ${HOTKEYD_SERVICE_PATH}"
else
    cat > "${SERVICE_PATH}" <<EOF
[Unit]
Description=Hop Launcher Daemon (hopd)
Wants=graphical-session.target
After=graphical-session.target dbus.service

[Service]
Type=simple
ExecStart=${INSTALL_BIN_PATH} --socket ${SOCKET_PATH}
Restart=on-failure
RestartSec=1
Environment=HOPD_SOCKET=${SOCKET_PATH}

[Install]
WantedBy=default.target
EOF

    cat > "${HOTKEYD_SERVICE_PATH}" <<EOF
[Unit]
Description=Hop Launcher Global Hotkey Agent (hop-hotkeyd)
Wants=graphical-session.target
After=graphical-session.target dbus.service

[Service]
Type=simple
ExecStart=${HOTKEYD_BIN_PATH}
Restart=on-failure
RestartSec=1
Environment=HOP_LAUNCHER_CONTROL_SOCKET=${CONTROL_SOCKET_PATH}
Environment=XDG_RUNTIME_DIR=%t

[Install]
WantedBy=default.target
EOF
fi

run_cmd systemctl --user daemon-reload
if [[ "$ENABLE_SERVICE" -eq 1 ]]; then
    run_cmd systemctl --user enable --now hopd.service
    run_cmd systemctl --user enable --now hop-hotkeyd.service
    configure_desktop_shortcut
    print_desktop_dependency_hints
else
    echo "Installed hopd.service and hop-hotkeyd.service but did not enable/start them (--no-enable)."
fi

echo "hopd installed at ${INSTALL_BIN_PATH}"
echo "hop-hotkeyd installed at ${HOTKEYD_BIN_PATH}"
echo "systemd unit installed at ${SERVICE_PATH}"
echo "systemd unit installed at ${HOTKEYD_SERVICE_PATH}"
echo "socket path: ${SOCKET_PATH}"
echo "control socket path: ${CONTROL_SOCKET_PATH}"
echo "shortcut accelerator: ${SHORTCUT_ACCEL}"
echo "compositor-specific binding helper:"
echo "  ${HOTKEYD_BIN_PATH} print-bindings"
