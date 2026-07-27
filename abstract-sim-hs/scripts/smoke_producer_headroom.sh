#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_dir="$(cd -- "$script_dir/.." && pwd)"
manifest="config/sweeps/producer-headroom-loose.json"
out_dir="sweep-results/producer-headroom-loose"
seeds=10
slots=2000

usage() {
  cat <<'EOF'
usage: ./scripts/smoke_producer_headroom.sh [--out DIR] [--seeds N] [--slots N]

Measure how often announced EBs would fail fee validation at the
certification check if EB selection used only the current quote, with no
one-further-step producer headroom. Runs the canonical D16/K10 configuration
with producerHeadroom disabled under the severe-congestion and launch-day
load profiles (the price-moving regimes), 10 seeds x 2,000 slots each, WITH
event traces retained, then classifies every announced EB from the traces
and writes comparison.md and comparison.json.

NOTE: retaining event traces is disk-heavy - expect a few hundred MB per run
and a few GB in total. The traces can be deleted after the analysis, but the
comparison outputs alone are not independently reproducible or auditable:
they do not preserve or hash the traces or manifest, and the analyser does
not verify the expected seed set.

The script refuses to overwrite an existing output directory.
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

"$simulator" sweep "$manifest" \
  --seeds "$seeds" \
  --slots "$slots" \
  --load-profile config/loads/severe-congestion.json \
  --out "$out_dir/severe-congestion"

"$simulator" sweep "$manifest" \
  --seeds "$seeds" \
  --slots "$slots" \
  --load-profile config/loads/launch-day.json \
  --out "$out_dir/launch-day"

python3 scripts/analyze_eb_fee_failures.py \
  --root "$out_dir" \
  --simulator-sha256 "$simulator_sha256" \
  --markdown-output "$out_dir/comparison.md" \
  --json-output "$out_dir/comparison.json"
