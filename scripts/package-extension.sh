#!/usr/bin/env bash
set -euo pipefail

UUID="hop-launcher@example.org"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${ROOT_DIR}/dist"
OUT_FILE="${OUT_DIR}/${UUID}.zip"

mkdir -p "${OUT_DIR}"
rm -f "${OUT_FILE}"

(
  cd "${ROOT_DIR}"
  zip -r "${OUT_FILE}" \
    metadata.json extension.js prefs.js stylesheet.css \
    lib ui schemas README.md \
    -x '*.git*' -x 'dist/*' -x 'scripts/*' -x 'tests/*' -x 'package.json'
)

echo "Created ${OUT_FILE}"
