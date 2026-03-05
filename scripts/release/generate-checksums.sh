#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "usage: $0 <output-file> <artifact-path> [artifact-path ...]" >&2
  exit 2
fi

output_file="$1"
shift

mkdir -p "$(dirname "$output_file")"

files=()
for artifact in "$@"; do
  if [[ ! -e "$artifact" ]]; then
    echo "missing artifact for checksums: $artifact" >&2
    exit 1
  fi
  if [[ -d "$artifact" ]]; then
    while IFS= read -r -d '' file; do
      [[ -f "$file" ]] && files+=("$file")
    done < <(find "$artifact" -type f -print0)
  else
    files+=("$artifact")
  fi
done

if [[ ${#files[@]} -eq 0 ]]; then
  echo "no files found to checksum" >&2
  exit 1
fi

IFS=$'\n' sorted=($(printf '%s\n' "${files[@]}" | sort))
unset IFS

: > "$output_file"
for file in "${sorted[@]}"; do
  sha256sum "$file" >> "$output_file"
done

echo "wrote checksums to $output_file"
