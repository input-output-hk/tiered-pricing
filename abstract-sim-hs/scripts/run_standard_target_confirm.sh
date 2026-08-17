#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_dir="$(cd -- "$script_dir/.." && pwd)"
manifest="config/sweeps/standard-target-confirm.json"
out_dir="sweep-results/standard-target-confirm"
seeds=100
slots=2000
seed_start=200

usage() {
  cat <<'EOF'
usage: ./scripts/run_standard_target_confirm.sh [--out DIR] [--seed-start N] [--seeds N] [--slots N]

Confirmatory rerun of the pre-selected S=.75/U=.50 arm from the independent
standard-target screen, paired against flat fee and canonical S=.5/U=.5 with
the patient demand-census shadow arm.

Stages: severe-congestion and launch-day (the screened loads, compared with
confirm-stage reports), then low (built-in low preset via --load), mid-load,
and eb-capacity-stress as regression stages (summaries only; the comparison
script does not yet cover these loads). The variant configs carry an internal
"load": "severe-congestion" default, so the low stage must pass --load low
explicitly; omitting it silently reruns severe congestion.

Defaults: seeds 200-299 (disjoint from the screen's 0-199), 2,000 slots,
summary-only, independent RNG streams. The output directory is reserved before
execution and is never overwritten. Use smaller --seeds/--slots values only to
check the harness; those results are not evidence.
EOF
}

positive_integer() {
  [[ "$1" =~ ^[1-9][0-9]*$ ]]
}

non_negative_integer() {
  [[ "$1" =~ ^[0-9]+$ ]]
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --out)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      [[ -n "$2" ]] || { echo "error: --out must not be empty" >&2; exit 2; }
      out_dir="$2"
      shift 2
      ;;
    --seeds)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      positive_integer "$2" || { echo "error: --seeds needs a positive integer" >&2; exit 2; }
      seeds="$2"
      shift 2
      ;;
    --seed-start)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      non_negative_integer "$2" || { echo "error: --seed-start needs a non-negative integer" >&2; exit 2; }
      seed_start="$2"
      shift 2
      ;;
    --slots)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      positive_integer "$2" || { echo "error: --slots needs a positive integer" >&2; exit 2; }
      slots="$2"
      shift 2
      ;;
    *)
      usage >&2
      exit 2
      ;;
  esac
done

if (( seeds < 2 )); then
  echo "error: --seeds must be at least 2 for paired confidence intervals" >&2
  exit 2
fi

if (( seed_start < 200 )); then
  echo "warning: seeds below 200 overlap the screen's 0-199 range; the result is not a disjoint-seed confirmation" >&2
fi

cd "$project_dir"

if [[ -e "$out_dir" ]]; then
  echo "error: refusing to overwrite existing output directory: $out_dir" >&2
  echo "choose another directory with --out DIR" >&2
  exit 2
fi

stack build
simulator="$(stack path --local-install-root)/bin/abstract-sim-hs-exe"
if [[ ! -x "$simulator" ]]; then
  echo "error: built simulator executable not found: $simulator" >&2
  exit 2
fi
simulator_hash_line="$(sha256sum -- "$simulator")"
simulator_sha256="${simulator_hash_line%% *}"

out_parent="$(dirname -- "$out_dir")"
mkdir -p "$out_parent"
if ! mkdir -- "$out_dir"; then
  echo "error: could not reserve output directory: $out_dir" >&2
  exit 2
fi

run_load() {
  local name="$1"
  local kind="$2"
  local value="$3"
  local args=(sweep "$manifest"
    --seed-start "$seed_start"
    --seeds "$seeds"
    --slots "$slots"
    --summary-only
    --out "$out_dir/$name")
  case "$kind" in
    profile) args+=(--load-profile "$value") ;;
    preset) args+=(--load "$value") ;;
    *) echo "error: unknown load kind: $kind" >&2; exit 2 ;;
  esac
  "$simulator" "${args[@]}"
}

run_load severe-congestion profile config/loads/severe-congestion.json
run_load launch-day profile config/loads/launch-day.json
run_load low preset low
run_load mid-load profile config/loads/mid-load.json
run_load eb-capacity-stress profile config/loads/eb-capacity-stress.json

python3 scripts/compare_standard_target_screen.py \
  --root "$out_dir/severe-congestion" \
  --load-name severe-congestion \
  --stage confirm \
  --manifest "$manifest" \
  --simulator-sha256 "$simulator_sha256" \
  --markdown-output "$out_dir/comparison.md" \
  --json-output "$out_dir/comparison.json"

python3 scripts/compare_standard_target_screen.py \
  --root "$out_dir/launch-day" \
  --load-name launch-day \
  --stage confirm \
  --manifest "$manifest" \
  --simulator-sha256 "$simulator_sha256" \
  --markdown-output "$out_dir/comparison-launch-day.md" \
  --json-output "$out_dir/comparison-launch-day.json"

echo "confirmation complete:"
echo "  severe congestion:  $out_dir/comparison.md"
echo "  launch day:         $out_dir/comparison-launch-day.md"
echo "  regression stages (summaries only, no comparison reports yet):"
echo "    low (default):    $out_dir/low/summary.json"
echo "    mid load:         $out_dir/mid-load/summary.json"
echo "    EB stress:        $out_dir/eb-capacity-stress/summary.json"
