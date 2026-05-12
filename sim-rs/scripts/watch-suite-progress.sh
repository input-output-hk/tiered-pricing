#!/usr/bin/env bash
# Print a compact progress summary across the phase-2 suites.
#
# Usage:
#   scripts/watch-suite-progress.sh [RUN_ID]
#   watch -n 5 scripts/watch-suite-progress.sh [RUN_ID]
#
# RUN_ID defaults to "*" (matches any suffix). Pass the same ID you
# passed to scripts/run-parallel-suites.sh to scope to that batch.
#
# Reads sim-rs/output/phase-2/<suite>-<run-id>/manifest.json and tallies
# pending / running / completed / failed (job, seed) pairs per suite.

set -euo pipefail

cd "$(dirname "$0")/.."

run_id="${1:-*}"

shopt -s nullglob
manifests=(output/phase-2/*-${run_id}/manifest.json)
shopt -u nullglob

if [[ ${#manifests[@]} -eq 0 ]]; then
  echo "no manifests at output/phase-2/*-${run_id}/manifest.json" >&2
  exit 1
fi

printf '%-50s %8s %8s %8s %8s %8s\n' SUITE PENDING RUNNING COMPLETED FAILED TOTAL

total_p=0; total_r=0; total_c=0; total_f=0; total_t=0

for m in "${manifests[@]}"; do
  suite_dir="$(dirname "$m")"
  suite_name="$(basename "$suite_dir")"
  counts="$(jq '
    [.jobs[][].status]
    | group_by(.)
    | map({key: .[0], value: length})
    | from_entries
  ' "$m")"
  p=$(jq -r '."pending"  // 0' <<<"$counts")
  r=$(jq -r '."running"  // 0' <<<"$counts")
  c=$(jq -r '."completed" // 0' <<<"$counts")
  f=$(jq -r '."failed"   // 0' <<<"$counts")
  t=$((p + r + c + f))
  printf '%-50s %8d %8d %8d %8d %8d\n' "$suite_name" "$p" "$r" "$c" "$f" "$t"
  total_p=$((total_p + p))
  total_r=$((total_r + r))
  total_c=$((total_c + c))
  total_f=$((total_f + f))
  total_t=$((total_t + t))
done

printf -- '----------------------------------------------------------------------------------------\n'
printf '%-50s %8d %8d %8d %8d %8d' TOTAL "$total_p" "$total_r" "$total_c" "$total_f" "$total_t"
if [[ $total_t -gt 0 ]]; then
  pct=$(( (total_c * 100) / total_t ))
  printf '   (%d%% complete)' "$pct"
fi
printf '\n'
