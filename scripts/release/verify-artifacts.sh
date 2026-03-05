#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "usage: $0 <manifest-file> <artifact-path> [artifact-path ...]" >&2
  exit 2
fi

manifest_file="$1"
shift

mkdir -p "$(dirname "$manifest_file")"
: > "$manifest_file"

missing=0
for artifact in "$@"; do
  if [[ ! -e "$artifact" ]]; then
    echo "missing artifact: $artifact" >&2
    missing=1
    continue
  fi
  if [[ -d "$artifact" ]]; then
    while IFS= read -r -d '' file; do
      if [[ -s "$file" ]]; then
        echo "$file" >> "$manifest_file"
      fi
    done < <(find "$artifact" -type f -print0)
    continue
  fi
  if [[ ! -s "$artifact" ]]; then
    echo "empty artifact: $artifact" >&2
    missing=1
    continue
  fi
  echo "$artifact" >> "$manifest_file"
done

if [[ $missing -ne 0 ]]; then
  exit 1
fi

if [[ ! -s "$manifest_file" ]]; then
  echo "artifact manifest is empty: $manifest_file" >&2
  exit 1
fi

echo "verified artifacts:"
cat "$manifest_file"
