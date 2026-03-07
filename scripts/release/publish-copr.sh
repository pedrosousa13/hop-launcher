#!/usr/bin/env bash
set -euo pipefail

# Trigger COPR SCM builds for hopd and hop-hotkeyd from a release tag.
# Required env:
# - COPR_OWNER
# - COPR_PROJECT
# Optional env:
# - COPR_CHROOTS (space-delimited list, default fedora-41-x86_64)
# - COPR_SOURCE_REPO (default repository URL)
# - RELEASE_TAG (defaults to GITHUB_REF_NAME)

if [[ -z "${COPR_OWNER:-}" || -z "${COPR_PROJECT:-}" ]]; then
  echo "COPR_OWNER and COPR_PROJECT are required" >&2
  exit 1
fi

if ! command -v copr-cli >/dev/null 2>&1; then
  echo "copr-cli is required but not found" >&2
  exit 1
fi

RELEASE_TAG="${RELEASE_TAG:-${GITHUB_REF_NAME:-}}"
if [[ -z "${RELEASE_TAG}" ]]; then
  echo "RELEASE_TAG (or GITHUB_REF_NAME) must be set" >&2
  exit 1
fi

COPR_CHROOTS="${COPR_CHROOTS:-fedora-41-x86_64}"
COPR_SOURCE_REPO="${COPR_SOURCE_REPO:-https://github.com/pedrosousa13/hop-launcher.git}"
COPR_TARGET="${COPR_OWNER}/${COPR_PROJECT}"

build_package() {
  local package_name="$1"
  local spec_path="$2"
  echo "Triggering COPR build for ${package_name} on ${COPR_TARGET} from tag ${RELEASE_TAG}"

  local output
  # Use SCM build so COPR checks out the tag directly from the repository.
  output="$(copr-cli buildscm "${COPR_TARGET}" \
    --clone-url "${COPR_SOURCE_REPO}" \
    --commit "${RELEASE_TAG}" \
    --method tito \
    --spec "${spec_path}" \
    --chroots ${COPR_CHROOTS} 2>&1)"

  echo "${output}"
}

build_package "hopd" "packaging/rpm/hopd.spec"
build_package "hop-hotkeyd" "packaging/rpm/hop-hotkeyd.spec"

