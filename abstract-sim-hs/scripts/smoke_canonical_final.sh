#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_dir="$(cd -- "$script_dir/.." && pwd)"
default_out="sweep-results/canonical-final-d16-k10-launch-day"

usage() {
  cat <<'EOF'
usage: ./scripts/smoke_canonical_final.sh [--out DIR]

Run the ten-seed, 2,000-slot canonical D16/D16, threshold-reservation, K=10
launch-day integration check without retaining event traces, then write
DIR/comparison.md. The comparison uses preserved corrected and pre-correction
D16 scalars as paired references.

The script refuses to overwrite an existing output directory.
EOF
}

out_dir="$default_out"
case "${1-}" in
  "") ;;
  -h|--help)
    usage
    exit 0
    ;;
  --out)
    if [[ $# -ne 2 ]]; then
      usage >&2
      exit 2
    fi
    out_dir="$2"
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac

cd "$project_dir"

baseline="../docs/phase-2/experiment-results/cross-lane-inversion-d16-baseline.json"
if [[ ! -f "$baseline" ]]; then
  echo "error: paired reference evidence does not exist: $baseline" >&2
  exit 2
fi
if [[ -e "$out_dir" ]]; then
  echo "error: refusing to overwrite existing output directory: $out_dir" >&2
  echo "choose another directory with --out DIR" >&2
  exit 2
fi

stack run -- sweep config/sweeps/canonical-final-smoke.json \
  --load-profile config/loads/launch-day.json \
  --summary-only \
  --out "$out_dir"

python3 scripts/compare_canonical_final.py \
  --baseline "$baseline" \
  --candidate "$out_dir/summary.json" \
  --output "$out_dir/comparison.md"

cat "$out_dir/comparison.md"
