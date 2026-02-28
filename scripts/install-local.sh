#!/usr/bin/env bash
set -euo pipefail
umask 077

UUID="hop-launcher@example.org"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATA_HOME="${XDG_DATA_HOME:-${HOME}/.local/share}"
EXT_DIR="${DATA_HOME}/gnome-shell/extensions/${UUID}"
TMP_DIR=""

cleanup() {
  if [[ -n "${TMP_DIR}" ]] && [[ -d "${TMP_DIR}" ]]; then
    rm -rf "${TMP_DIR}"
  fi
}
trap cleanup EXIT

extension_discoverable() {
  gnome-extensions list 2>/dev/null | grep -Fxq "${UUID}"
}

diagnose_discovery_failure() {
  echo "=== hop-launcher discovery diagnostics ===" >&2
  echo "Desktop user: $(whoami 2>/dev/null || echo unknown) (uid $(id -u 2>/dev/null || echo unknown))" >&2
  echo "XDG_SESSION_TYPE=${XDG_SESSION_TYPE:-unset}" >&2
  echo "DBUS_SESSION_BUS_ADDRESS=${DBUS_SESSION_BUS_ADDRESS:-unset}" >&2
  echo "Install target directory: ${EXT_DIR}" >&2
  echo "GNOME Shell version: $(gnome-shell --version 2>/dev/null || echo unknown)" >&2

  if command -v stat >/dev/null; then
    echo "Target permissions:" >&2
    stat -c '%U:%G %a %n' "${EXT_DIR}" "${EXT_DIR}/metadata.json" 2>/dev/null >&2 || true
  fi

  echo "Installed extensions visible to gnome-extensions:" >&2
  gnome-extensions list >&2 || true

  echo "Direct extension info query:" >&2
  gnome-extensions info "${UUID}" >&2 || true

  if command -v gdbus >/dev/null; then
    echo "D-Bus extension registry contains UUID:" >&2
    gdbus call --session \
      --dest org.gnome.Shell.Extensions \
      --object-path /org/gnome/Shell/Extensions \
      --method org.gnome.Shell.Extensions.ListExtensions 2>/dev/null \
      | grep -F "${UUID}" >&2 || true
  fi

  if command -v journalctl >/dev/null; then
    echo "Recent GNOME Shell logs mentioning UUID:" >&2
    journalctl --user -n 200 --no-pager 2>/dev/null | grep -F "${UUID}" >&2 || true
  fi

  echo "=== end diagnostics ===" >&2
}

if ! command -v rsync >/dev/null || ! command -v gnome-extensions >/dev/null; then
  echo "Missing required tools: rsync and gnome-extensions are required." >&2
  exit 1
fi

if [[ "${EUID}" -eq 0 ]] || [[ -n "${SUDO_USER:-}" ]]; then
  echo "Do not run this script as root/sudo. Run it as your desktop user." >&2
  exit 1
fi

mkdir -p "${EXT_DIR}"
rsync -a --delete \
  --exclude '.git' \
  --exclude 'node_modules' \
  --exclude '.DS_Store' \
  --exclude 'dist' \
  "${ROOT_DIR}/" "${EXT_DIR}/"

BUILD_ID="$(date +%Y%m%d-%H%M%S)"
BUILD_HASH="$(git -C "${ROOT_DIR}" rev-parse --short=12 HEAD 2>/dev/null || true)"
cat > "${EXT_DIR}/lib/buildInfo.js" <<EOF
import {formatBuildLabel} from './buildLabel.js';

export const BUILD_ID = '${BUILD_ID}';
export const BUILD_HASH = '${BUILD_HASH}';
export const BUILD_LABEL = formatBuildLabel(BUILD_ID, BUILD_HASH);
EOF

glib-compile-schemas "${EXT_DIR}/schemas"

gnome-extensions disable "${UUID}" >/dev/null 2>&1 || true
for (( attempt = 1; attempt <= 15; attempt++ )); do
  if extension_discoverable; then
    break
  fi
  sleep 0.2
done

if ! extension_discoverable; then
  if command -v zip >/dev/null; then
    TMP_DIR="$(mktemp -d)"
    ZIP_FILE="${TMP_DIR}/${UUID}.zip"
    (
      cd "${ROOT_DIR}"
      zip -qr "${ZIP_FILE}" \
        metadata.json extension.js prefs.js stylesheet.css \
        lib ui schemas README.md \
        -x '*.git*' -x 'dist/*' -x 'scripts/*' -x 'tests/*' -x 'package.json' -x 'package-lock.json'
    )
    INSTALL_OUTPUT="$(gnome-extensions install --force "${ZIP_FILE}" 2>&1)" || true
    if [[ -n "${INSTALL_OUTPUT}" ]]; then
      echo "${INSTALL_OUTPUT}" >&2
    fi
    sleep 0.2
  fi

  if ! extension_discoverable; then
    echo "Extension ${UUID} is still not discoverable by GNOME Shell." >&2
    diagnose_discovery_failure
    echo "Check metadata.json shell-version and your session user/home alignment." >&2
    echo "If you are in a container, toolbox, distrobox, SSH, or other remote session," >&2
    echo "run this script from a terminal inside your active GNOME desktop session." >&2
    exit 1
  fi
fi

if ! gnome-extensions enable "${UUID}"; then
  echo "Failed to enable ${UUID}." >&2
  echo "If GNOME says the extension does not exist, check metadata.json shell-version" >&2
  echo "includes your GNOME Shell major version and then re-run this script." >&2
  exit 1
fi

echo "Installed and enabled ${UUID}."
echo "Open logs with: journalctl --user -f /usr/bin/gnome-shell"
