#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_dir="$(cd -- "$script_dir/.." && pwd)"
manifest_source="config/sweeps/window-ablation-confirm.json"
pilot_report="sweep-results/window-ablation-smoke/comparison.json"
pilot_report_sha256="97b7a5f628d74cb55f06d191998044f7d8b54a925d6fc7dca7437c730379f55f"
pilot_source_patch="sweep-results/window-ablation-smoke/source.patch"
pilot_source_patch_sha256="c658660f40d481c42bc488816621d7b68e99d0b13b69b7914437afbe5c63d24b"
out_dir="sweep-results/window-ablation-confirm"
seed_start=400
seeds=10
slots=2000

usage() {
  cat <<'EOF'
usage: ./scripts/run_window_ablation_confirm.sh [--out DIR] [--seed-start N] [--seeds N] [--slots N]

Run the held-out three-arm confirmation of the controller-window ablation:

  current                 standard W20, urgent W5
  standard instant        standard current-production signal, urgent W5
  urgent instant          standard W20, urgent current-production signal

Everything else remains fixed at S=.75/U=.50, D16, half-RB threshold, and K10.
The evidence run uses seeds 400-409, 2,000 slots, independent RNG streams, and
all five headline loads. Pilot seeds 300-304 are never pooled into its results.
Expected runtime on the development machine is roughly 15-20 minutes plus any
initial build time.

The two predeclared primary endpoints are launch-day throughput for the
standard-instant arm and severe-congestion oscillation excess travel for the
urgent-instant arm. Other reported outcomes are descriptive safety checks.

Non-default seed counts or slot counts are useful for checking the harness,
but the comparator will label those results diagnostic rather than confirmatory.
The output directory is reserved before execution and is never overwritten.
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
    --seed-start)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      non_negative_integer "$2" || { echo "error: --seed-start needs a non-negative integer" >&2; exit 2; }
      seed_start="$2"
      shift 2
      ;;
    --seeds)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      positive_integer "$2" || { echo "error: --seeds needs a positive integer" >&2; exit 2; }
      seeds="$2"
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
  echo "error: --seeds must be at least 2 for paired intervals" >&2
  exit 2
fi

seed_end=$((seed_start + seeds - 1))
if (( seed_start <= 304 && seed_end >= 300 )); then
  echo "error: requested seed range ${seed_start}-${seed_end} overlaps pilot seeds 300-304" >&2
  exit 2
fi

if (( seed_start != 400 || seeds != 10 || slots != 2000 )); then
  echo "warning: non-default run; the report will be diagnostic, not the predeclared confirmation" >&2
fi

cd "$project_dir"

if [[ -e "$out_dir" ]]; then
  echo "error: refusing to overwrite existing output directory: $out_dir" >&2
  echo "choose another directory with --out DIR" >&2
  exit 2
fi

if [[ ! -f "$pilot_report" ]]; then
  echo "error: pilot report not found: $pilot_report" >&2
  exit 2
fi
if [[ ! -f "$pilot_source_patch" ]]; then
  echo "error: pilot source patch not found: $pilot_source_patch" >&2
  exit 2
fi

pilot_report_hash_line="$(sha256sum -- "$pilot_report")"
if [[ "${pilot_report_hash_line%% *}" != "$pilot_report_sha256" ]]; then
  echo "error: pilot report hash differs from the predeclared artifact" >&2
  exit 2
fi
pilot_patch_hash_line="$(sha256sum -- "$pilot_source_patch")"
if [[ "${pilot_patch_hash_line%% *}" != "$pilot_source_patch_sha256" ]]; then
  echo "error: pilot source-patch hash differs from the predeclared artifact" >&2
  exit 2
fi

SECONDS=0
stack build
simulator="$(stack path --local-install-root)/bin/abstract-sim-hs-exe"
if [[ ! -x "$simulator" ]]; then
  echo "error: built simulator executable not found: $simulator" >&2
  exit 2
fi
simulator_hash_line="$(sha256sum -- "$simulator")"
simulator_sha256="${simulator_hash_line%% *}"
pilot_sha256="$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["provenance"]["simulator_sha256"])' "$pilot_report")"
if [[ "$simulator_sha256" != "$pilot_sha256" ]]; then
  echo "error: simulator binary differs from the pilot" >&2
  echo "  pilot:        $pilot_sha256" >&2
  echo "  confirmation: $simulator_sha256" >&2
  echo "rerun the pilot with this binary before treating a new run as its confirmation" >&2
  exit 2
fi

mkdir -p "$(dirname -- "$out_dir")"
if ! mkdir -- "$out_dir"; then
  echo "error: could not reserve output directory: $out_dir" >&2
  exit 2
fi

# Freeze the predeclared plan and its evaluator before any fresh result exists.
manifest="$out_dir/manifest.json"
cp -- "$manifest_source" "$manifest"
sha256sum -- \
  "$manifest" \
  scripts/compare_window_ablation_confirm.py \
  scripts/compare_window_ablation_smoke.py \
  scripts/compare_cross_lane_inversion_smoke.py \
  scripts/compare_canonical_headlines.py \
  scripts/run_window_ablation_confirm.sh \
  > "$out_dir/analysis-plan.sha256"

run_load() {
  local name="$1"
  local kind="$2"
  local value="$3"
  local args=(
    sweep "$manifest"
    --seed-start "$seed_start"
    --seeds "$seeds"
    --slots "$slots"
    --summary-only
    --out "$out_dir/$name"
  )
  case "$kind" in
    profile) args+=(--load-profile "$value") ;;
    preset) args+=(--load "$value") ;;
    *) echo "error: unknown load kind: $kind" >&2; exit 2 ;;
  esac
  echo "running $name..."
  "$simulator" "${args[@]}"
}

run_load low preset low
run_load mid-load profile config/loads/mid-load.json
run_load severe-congestion profile config/loads/severe-congestion.json
run_load eb-capacity-stress profile config/loads/eb-capacity-stress.json
run_load launch-day profile config/loads/launch-day.json

python3 scripts/compare_window_ablation_confirm.py \
  --root "$out_dir" \
  --manifest "$manifest" \
  --pilot-report "$pilot_report" \
  --simulator-sha256 "$simulator_sha256" \
  --markdown-output "$out_dir/comparison.md" \
  --json-output "$out_dir/comparison.json"

echo "window-ablation confirmation complete in ${SECONDS}s:"
echo "  report: $out_dir/comparison.md"
echo "  data:   $out_dir/comparison.json"
