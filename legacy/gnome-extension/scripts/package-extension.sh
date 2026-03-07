#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
METADATA_FILE="${ROOT_DIR}/metadata.json"
UUID="$(sed -n 's/^[[:space:]]*"uuid"[[:space:]]*:[[:space:]]*"\([^"]\+\)".*/\1/p' "${METADATA_FILE}" | head -n 1)"
OUT_DIR="${ROOT_DIR}/dist"
OUT_FILE="${OUT_DIR}/${UUID}.zip"

if [[ -z "${UUID}" ]]; then
  echo "Failed to read extension uuid from ${METADATA_FILE}." >&2
  exit 1
fi

mkdir -p "${OUT_DIR}"
rm -f "${OUT_FILE}"

(
  cd "${ROOT_DIR}"
  zip -r "${OUT_FILE}" \
    metadata.json extension.js prefs.js stylesheet.css LICENSE \
    lib ui schemas README.md \
    -x '*.git*' -x 'dist/*' -x 'scripts/*' -x 'tests/*' -x 'package.json'
)

echo "Created ${OUT_FILE}"
