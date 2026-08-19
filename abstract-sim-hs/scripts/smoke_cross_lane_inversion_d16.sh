#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_dir="$(cd -- "$script_dir/.." && pwd)"
default_out="sweep-results/cross-lane-inversion-smoke-d16-launch-day"

usage() {
  cat <<'EOF'
usage: ./scripts/smoke_cross_lane_inversion_d16.sh [--out DIR]

Run the ten-seed, 2,000-slot D16 max-of-two/no-floor launch-day diagnostic
without retaining event traces, then write DIR/comparison.md. The comparison
uses preserved pre-correction D16 scalars as its paired baseline.

Only the corrected candidate is rerun. The script refuses to overwrite an
existing output directory.
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
  echo "error: paired baseline does not exist: $baseline" >&2
  exit 2
fi
if [[ -e "$out_dir" ]]; then
  echo "error: refusing to overwrite existing output directory: $out_dir" >&2
  echo "choose another directory with --out DIR" >&2
  exit 2
fi

stack run -- sweep config/sweeps/cross-lane-inversion-smoke-d16.json \
  --load-profile config/loads/launch-day.json \
  --summary-only \
  --out "$out_dir"

python3 scripts/compare_cross_lane_inversion_d16.py \
  --baseline "$baseline" \
  --candidate "$out_dir/summary.json" \
  --output "$out_dir/comparison.md"

cat "$out_dir/comparison.md"
