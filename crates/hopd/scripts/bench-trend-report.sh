#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "usage: $0 <current-bench.txt> [baseline-bench.txt]" >&2
  exit 2
fi

current_report="$1"
baseline_report="${2:-}"

if [[ ! -f "$current_report" ]]; then
  echo "current benchmark report not found: $current_report" >&2
  exit 2
fi
if [[ -n "$baseline_report" && ! -f "$baseline_report" ]]; then
  echo "baseline benchmark report not found: $baseline_report" >&2
  exit 2
fi

extract_metric() {
  local file="$1"
  local label="$2"
  local field="$3"
  awk -v label="${label}:" -v field="$field" '
    index($0, label) == 1 {
      if (match($0, /warm wall mean=[0-9.]+ms p95=[0-9.]+ms max=[0-9.]+ms/)) {
        segment = substr($0, RSTART, RLENGTH)
        split(segment, parts, " ")
        for (i = 1; i <= length(parts); i++) {
          if (index(parts[i], field "=") == 1) {
            value = parts[i]
            sub(field "=", "", value)
            sub(/ms$/, "", value)
            print value
            exit
          }
        }
      }
    }
  ' "$file"
}

format_delta() {
  local current="$1"
  local previous="$2"
  awk -v current="$current" -v previous="$previous" '
    BEGIN {
      if (current == "" || previous == "") {
        print "-"
        exit
      }
      delta = current - previous
      sign = delta > 0 ? "+" : ""
      printf("%s%.3f", sign, delta)
    }
  '
}

emit_row() {
  local label="$1"
  local current_mean current_p95 current_max
  current_mean="$(extract_metric "$current_report" "$label" "mean")"
  current_p95="$(extract_metric "$current_report" "$label" "p95")"
  current_max="$(extract_metric "$current_report" "$label" "max")"

  if [[ -z "$current_mean" || -z "$current_p95" || -z "$current_max" ]]; then
    return
  fi

  if [[ -n "$baseline_report" ]]; then
    local base_mean base_p95 base_max
    base_mean="$(extract_metric "$baseline_report" "$label" "mean")"
    base_p95="$(extract_metric "$baseline_report" "$label" "p95")"
    base_max="$(extract_metric "$baseline_report" "$label" "max")"

    local delta_mean delta_p95 delta_max
    delta_mean="$(format_delta "$current_mean" "$base_mean")"
    delta_p95="$(format_delta "$current_p95" "$base_p95")"
    delta_max="$(format_delta "$current_max" "$base_max")"

    printf "| %s | %.3f | %.3f | %.3f | %s | %s | %s |\n" \
      "$label" "$current_mean" "$current_p95" "$current_max" "$delta_mean" "$delta_p95" "$delta_max"
  else
    printf "| %s | %.3f | %.3f | %.3f |\n" \
      "$label" "$current_mean" "$current_p95" "$current_max"
  fi
}

if [[ -n "$baseline_report" ]]; then
  cat <<HEADER
# hopd Benchmark Trend Report

Source reports:
- Current: $current_report
- Baseline: $baseline_report

| Scenario | mean (ms) | p95 (ms) | max (ms) | Δmean (ms) | Δp95 (ms) | Δmax (ms) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
HEADER
else
  cat <<HEADER
# hopd Benchmark Snapshot Report

Source report:
- Current: $current_report

| Scenario | mean (ms) | p95 (ms) | max (ms) |
| --- | ---: | ---: | ---: |
HEADER
fi

for scenario in apps utility files-200 files-1200; do
  emit_row "$scenario"
done
