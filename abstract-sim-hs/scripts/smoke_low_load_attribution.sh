#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_dir="$(cd -- "$script_dir/.." && pwd)"
manifest="config/sweeps/canonical-headlines.json"
out_dir="sweep-results/low-load-attribution-smoke"
seeds=100
slots=2000

usage() {
  cat <<'EOF'
usage: ./scripts/smoke_low_load_attribution.sh [--out DIR] [--seeds N] [--slots N] [--manifest FILE]

Attribute the low-load overall retained-value difference by pairing flat fee
with the canonical D16/K10 mechanism under the built-in low-load preset and
recording value levels (retained, lost, unresolved lovelace) per lane and for
the urgent demand class, alongside entry counts and latency. By default this
runs seeds 0-99 for 2,000 slots (200 summary-only simulations), then writes
comparison.md and comparison.json, including derived per-slice submitted
value and a residual row (overall minus both lanes).

--manifest selects a different variant-pair manifest (default
config/sweeps/canonical-headlines.json); the manifest's variants must be
named flat-fee and canonical-final-d16-k10 for the comparison step.

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
    --manifest)
      [[ $# -ge 2 ]] || { usage >&2; exit 2; }
      [[ -n "$2" ]] || { echo "error: --manifest must not be empty" >&2; exit 2; }
      manifest="$2"
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

"$simulator" sweep "$manifest" \
  --seeds "$seeds" \
  --slots "$slots" \
  --summary-only \
  --load low \
  --out "$out_dir/low"

python3 scripts/compare_low_load_attribution.py \
  --root "$out_dir/low" \
  --manifest "$manifest" \
  --simulator-sha256 "$simulator_sha256" \
  --markdown-output "$out_dir/comparison.md" \
  --json-output "$out_dir/comparison.json"
