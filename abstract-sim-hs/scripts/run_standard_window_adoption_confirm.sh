#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_dir="$(cd -- "$script_dir/.." && pwd)"
manifest="config/sweeps/standard-window-adoption-confirm.json"
out_dir="sweep-results/standard-window-adoption-confirm"
seed_start=700
seeds=100
slots=2000

usage() {
  cat <<'EOF'
usage: ./scripts/run_standard_window_adoption_confirm.sh [--out DIR] [--seed-start N] [--seeds N] [--slots N]

Confirm the standard-window selection under the adoption criteria, on
fresh seeds:

  historical                  announcement-diluted 20-entry window,
                              announcement-time age reset (reference)
  flat-fee                    flat-fee baseline (adoption floor)
  block-window-10-cert-reset  the configuration the CIP will specify:
                              10-block processed-block window and
                              age-escape reset at certification

The default run uses one hundred paired seeds (700-799, disjoint from the
screen's 0-99 and every other selection experiment), 2,000 slots,
independent RNG streams, the five headline loads, and the 0.1 tx/slot
trickle load.

Pre-declared pass criteria (all must hold; evaluated by the compare script):
  M1  severe congestion, EB-capacity stress, launch day: candidate overall
      retained-value ratio within 0.05 pp of historical (CI low > -0.05)
  M2  trickle: candidate standard-lane retained-value ratio within 1 pp of
      historical (CI low > -1.0)
  M3  severe congestion, EB-capacity stress, launch day: candidate overall
      retained value beats flat fee (95% CI above zero)

Superiority over historical is a declared secondary: reported, not gated.
Single attempt; the outcome is reported regardless of result.

A FAIL verdict is a completed experiment, not an execution error: the
script still prints the report paths and then exits with status 1.

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

cd "$project_dir"

if [[ -e "$out_dir" ]]; then
  echo "error: refusing to overwrite existing output directory: $out_dir" >&2
  echo "choose another directory with --out DIR" >&2
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

mkdir -p "$(dirname -- "$out_dir")"
if ! mkdir -- "$out_dir"; then
  echo "error: could not reserve output directory: $out_dir" >&2
  exit 2
fi

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
run_load trickle-0p1 profile config/loads/trickle-0p1.json

compare_status=0
python3 scripts/compare_standard_window_adoption_confirm.py \
  --root "$out_dir" \
  --manifest "$manifest" \
  --simulator-sha256 "$simulator_sha256" \
  --markdown-output "$out_dir/comparison.md" \
  --json-output "$out_dir/comparison.json" || compare_status=$?

echo "standard-window adoption confirmation complete in ${SECONDS}s:"
echo "  report: $out_dir/comparison.md"
echo "  data:   $out_dir/comparison.json"
exit "$compare_status"
