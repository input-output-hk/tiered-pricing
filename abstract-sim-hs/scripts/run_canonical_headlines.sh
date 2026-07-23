#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_dir="$(cd -- "$script_dir/.." && pwd)"
manifest="config/sweeps/canonical-headlines.json"
out_dir="sweep-results/canonical-headlines"
seeds=10
slots=2000

usage() {
  cat <<'EOF'
usage: ./scripts/run_canonical_headlines.sh [--out DIR] [--seeds N] [--slots N]

Refresh the CIP's five headline experiments by pairing flat fee with the
canonical D16/K10 mechanism. By default this runs seeds 0-9 for 2,000 slots
at low, mid, severe-congestion, EB-capacity-stress, and launch-day load
(100 summary-only simulations), then writes comparison.md and comparison.json.

The script refuses to overwrite an existing output directory. --seeds and
--slots are intended for quick harness checks; omit them for the headline run.
EOF
}

positive_integer() {
  [[ "$1" =~ ^[1-9][0-9]*$ ]]
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
  echo "error: --seeds must be at least 2 for the paired confidence intervals" >&2
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

run_preset() {
  local name="$1"
  local preset="$2"
  "$simulator" sweep "$manifest" \
    --seeds "$seeds" \
    --slots "$slots" \
    --summary-only \
    --load "$preset" \
    --out "$out_dir/$name"
}

run_profile() {
  local name="$1"
  local profile="$2"
  "$simulator" sweep "$manifest" \
    --seeds "$seeds" \
    --slots "$slots" \
    --summary-only \
    --load-profile "$profile" \
    --out "$out_dir/$name"
}

run_preset low low
run_profile mid-load config/loads/mid-load.json
run_profile severe-congestion config/loads/severe-congestion.json
run_profile eb-capacity-stress config/loads/eb-capacity-stress.json
run_profile launch-day config/loads/launch-day.json

python3 scripts/compare_canonical_headlines.py \
  --root "$out_dir" \
  --simulator-sha256 "$simulator_sha256" \
  --markdown-output "$out_dir/comparison.md" \
  --json-output "$out_dir/comparison.json"
