#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

"${ROOT_DIR}/scripts/install-local.sh"

if ! command -v gdbus >/dev/null; then
  echo "gdbus is required to reexec GNOME Shell." >&2
  exit 1
fi

gdbus call --session \
  --dest org.gnome.Shell \
  --object-path /org/gnome/Shell \
  --method org.gnome.Shell.Eval \
  "global.reexec_self(); 'ok'"

echo "Requested GNOME Shell reexec."
