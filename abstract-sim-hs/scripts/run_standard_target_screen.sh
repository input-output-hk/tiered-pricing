#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_dir="$(cd -- "$script_dir/.." && pwd)"
manifest="config/sweeps/standard-target-screen.json"
out_dir="sweep-results/standard-target-screen"
seeds=100
slots=2000
seed_start=0

usage() {
  cat <<'EOF'
usage: ./scripts/run_standard_target_screen.sh [--out DIR] [--seed-start N] [--seeds N] [--slots N]

Screen the standard-lane controller target under severe-congestion and
launch-day loads. Each load pairs flat fee, canonical S=.5/U=.5,
fixed-standard/U=.5, and standard targets .625/.75/.875 (urgent held at .5)
with a patient demand-census shadow arm.

Defaults per load: seeds 0-99, 2,000 slots, summary-only, independent RNG streams.
The output directory is reserved before execution and is never overwritten.
Use smaller --seeds/--slots values only to check the harness; those results are
not evidence.
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

run_profile() {
  local name="$1"
  local profile="$2"
  "$simulator" sweep "$manifest" \
    --seed-start "$seed_start" \
    --seeds "$seeds" \
    --slots "$slots" \
    --summary-only \
    --load-profile "$profile" \
    --out "$out_dir/$name"
}

run_profile severe-congestion config/loads/severe-congestion.json
run_profile launch-day config/loads/launch-day.json

python3 scripts/compare_standard_target_screen.py \
  --root "$out_dir/severe-congestion" \
  --load-name severe-congestion \
  --manifest "$manifest" \
  --simulator-sha256 "$simulator_sha256" \
  --markdown-output "$out_dir/comparison.md" \
  --json-output "$out_dir/comparison.json"

python3 scripts/compare_standard_target_screen.py \
  --root "$out_dir/launch-day" \
  --load-name launch-day \
  --manifest "$manifest" \
  --simulator-sha256 "$simulator_sha256" \
  --markdown-output "$out_dir/comparison-launch-day.md" \
  --json-output "$out_dir/comparison-launch-day.json"

echo "screen complete:"
echo "  severe congestion: $out_dir/comparison.md"
echo "  launch day:        $out_dir/comparison-launch-day.md"
