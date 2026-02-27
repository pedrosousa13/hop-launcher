#!/usr/bin/env bash
set -euo pipefail
umask 077

UUID="hop-launcher@example.org"
EXT_DIR="${HOME}/.local/share/gnome-shell/extensions/${UUID}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v rsync >/dev/null || ! command -v gnome-extensions >/dev/null; then
  echo "Missing required tools: rsync and gnome-extensions are required." >&2
  exit 1
fi

mkdir -p "${EXT_DIR}"
rsync -a --delete \
  --exclude '.git' \
  --exclude 'node_modules' \
  --exclude '.DS_Store' \
  --exclude 'dist' \
  "${ROOT_DIR}/" "${EXT_DIR}/"

glib-compile-schemas "${EXT_DIR}/schemas"

gnome-extensions disable "${UUID}" >/dev/null 2>&1 || true
gnome-extensions enable "${UUID}"

echo "Installed and enabled ${UUID}."
echo "Open logs with: journalctl --user -f /usr/bin/gnome-shell"
