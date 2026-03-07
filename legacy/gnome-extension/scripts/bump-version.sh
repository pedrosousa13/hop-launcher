#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
METADATA_FILE="${ROOT_DIR}/metadata.json"

usage() {
  cat <<'USAGE'
Usage:
  scripts/bump-version.sh                # increment metadata.json version by 1
  scripts/bump-version.sh --set <n>      # set metadata.json version to n (integer > 0)
  scripts/bump-version.sh --check        # validate metadata.json version is integer > 0
  scripts/bump-version.sh --check-bump-against <git-ref>
                                         # fail unless current version > version at git-ref
USAGE
}

read_version_from_file() {
  local path="$1"
  python - "$path" <<'PY'
import json, pathlib, sys
p = pathlib.Path(sys.argv[1])
obj = json.loads(p.read_text())
v = obj.get('version')
if not isinstance(v, int):
    raise SystemExit('metadata.version must be an integer')
if v <= 0:
    raise SystemExit('metadata.version must be > 0')
print(v)
PY
}

write_version_to_file() {
  local path="$1"
  local new_version="$2"
  python - "$path" "$new_version" <<'PY'
import json, pathlib, sys
p = pathlib.Path(sys.argv[1])
new_version = int(sys.argv[2])
obj = json.loads(p.read_text())
obj['version'] = new_version
p.write_text(json.dumps(obj, indent=2) + '\n')
PY
}

check() {
  local current
  current="$(read_version_from_file "${METADATA_FILE}")"
  echo "metadata.json version is valid: ${current}"
}

check_bump_against() {
  local ref="$1"
  local current base_json base
  current="$(read_version_from_file "${METADATA_FILE}")"

  base_json="$(git -C "${ROOT_DIR}" show "${ref}:metadata.json" 2>/dev/null || true)"
  if [[ -z "${base_json}" ]]; then
    echo "Could not read metadata.json from ref: ${ref}" >&2
    exit 1
  fi

  base="$(python - "${base_json}" <<'PY'
import json,sys
obj=json.loads(sys.argv[1])
v=obj.get('version')
if not isinstance(v,int):
    raise SystemExit('base metadata.version must be an integer')
print(v)
PY
)"

  if [[ "${current}" -le "${base}" ]]; then
    echo "Version guard failed: current metadata version (${current}) must be greater than ${ref} (${base})." >&2
    echo "Run: scripts/bump-version.sh" >&2
    exit 1
  fi

  echo "Version guard passed: current ${current} > ${ref} ${base}"
}

if [[ ! -f "${METADATA_FILE}" ]]; then
  echo "metadata.json not found at ${METADATA_FILE}" >&2
  exit 1
fi

case "${1:-}" in
  "")
    current="$(read_version_from_file "${METADATA_FILE}")"
    next=$((current + 1))
    write_version_to_file "${METADATA_FILE}" "${next}"
    echo "Bumped metadata version: ${current} -> ${next}"
    ;;
  --set)
    if [[ $# -ne 2 ]] || ! [[ "${2}" =~ ^[0-9]+$ ]] || [[ "${2}" -le 0 ]]; then
      echo "--set requires a positive integer" >&2
      usage
      exit 1
    fi
    write_version_to_file "${METADATA_FILE}" "${2}"
    echo "Set metadata version to ${2}"
    ;;
  --check)
    if [[ $# -ne 1 ]]; then
      usage
      exit 1
    fi
    check
    ;;
  --check-bump-against)
    if [[ $# -ne 2 ]]; then
      usage
      exit 1
    fi
    check_bump_against "${2}"
    ;;
  -h|--help)
    usage
    ;;
  *)
    usage
    exit 1
    ;;
esac
