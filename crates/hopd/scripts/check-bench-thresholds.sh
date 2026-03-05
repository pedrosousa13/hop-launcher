#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <hopd-bench-output-file>" >&2
  exit 2
fi

report_file="$1"
if [[ ! -f "$report_file" ]]; then
  echo "benchmark report file not found: $report_file" >&2
  exit 2
fi

apps_wall_p95_max="${HOPD_BENCH_APPS_WALL_P95_MAX:-40.0}"
utility_wall_p95_max="${HOPD_BENCH_UTILITY_WALL_P95_MAX:-90.0}"
files1200_wall_p95_max="${HOPD_BENCH_FILES1200_WALL_P95_MAX:-120.0}"

extract_wall_p95() {
  local label="$1"
  awk -v label="${label}:" '
    index($0, label) == 1 {
      if (match($0, /warm wall mean=[0-9.]+ms p95=[0-9.]+ms/)) {
        segment = substr($0, RSTART, RLENGTH)
        sub(/^.*p95=/, "", segment)
        sub(/ms$/, "", segment)
        print segment
        exit
      }
    }
  ' "$report_file"
}

compare_threshold() {
  local name="$1"
  local value="$2"
  local max="$3"
  awk -v name="$name" -v value="$value" -v max="$max" '
    BEGIN {
      if (value == "") {
        printf("missing benchmark value for %s\n", name) > "/dev/stderr";
        exit 2;
      }
      if ((value + 0) > (max + 0)) {
        printf("%s p95 %.3fms exceeds threshold %.3fms\n", name, value + 0, max + 0) > "/dev/stderr";
        exit 1;
      }
      printf("%s p95 %.3fms within threshold %.3fms\n", name, value + 0, max + 0);
    }
  '
}

apps_wall_p95="$(extract_wall_p95 "apps")"
utility_wall_p95="$(extract_wall_p95 "utility")"
files1200_wall_p95="$(extract_wall_p95 "files-1200")"

compare_threshold "apps" "$apps_wall_p95" "$apps_wall_p95_max"
compare_threshold "utility" "$utility_wall_p95" "$utility_wall_p95_max"
compare_threshold "files-1200" "$files1200_wall_p95" "$files1200_wall_p95_max"
