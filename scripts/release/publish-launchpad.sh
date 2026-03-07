#!/usr/bin/env bash
set -euo pipefail

# Build signed source packages and upload to Launchpad PPA via dput.
# Required env:
# - LAUNCHPAD_PPA (for example "~owner/ubuntu/hop-launcher")
# Optional env:
# - LAUNCHPAD_DPUT_HOST (default: ppa)

if [[ -z "${LAUNCHPAD_PPA:-}" ]]; then
  echo "LAUNCHPAD_PPA is required (example: ~owner/ubuntu/hop-launcher)" >&2
  exit 1
fi

DPUT_HOST="${LAUNCHPAD_DPUT_HOST:-ppa}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RELEASE_TAG="${RELEASE_TAG:-${GITHUB_REF_NAME:-}}"

if [[ -z "${RELEASE_TAG}" ]]; then
  echo "RELEASE_TAG (or GITHUB_REF_NAME) must be set" >&2
  exit 1
fi

if ! command -v dput >/dev/null 2>&1; then
  echo "dput is required but not found" >&2
  exit 1
fi

if ! command -v dpkg-buildpackage >/dev/null 2>&1; then
  echo "dpkg-buildpackage is required but not found" >&2
  exit 1
fi

upload_source_pkg() {
  local crate_rel="$1"
  local crate_dir="${ROOT_DIR}/${crate_rel}"

  pushd "${crate_dir}" >/dev/null
  dpkg-buildpackage -S -sa
  local changes_file
  changes_file="$(ls -1 ../*.changes | head -n 1)"
  if [[ -z "${changes_file}" || ! -f "${changes_file}" ]]; then
    echo "Could not find .changes file in $(pwd)/.." >&2
    exit 1
  fi

  local target="${DPUT_HOST}:${LAUNCHPAD_PPA}"
  echo "Uploading ${changes_file} to ${target}"
  dput "${target}" "${changes_file}"
  popd >/dev/null
}

upload_source_pkg "crates/hopd"
upload_source_pkg "crates/hop-hotkeyd"

