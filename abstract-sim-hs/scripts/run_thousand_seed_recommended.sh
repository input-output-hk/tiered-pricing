#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_dir="$(cd -- "$script_dir/.." && pwd)"
manifest="config/sweeps/recommended-headlines.json"
low_dir="sweep-results/recommended-1000-low"
severe_dir="sweep-results/recommended-1000-severe-congestion"
seeds=1000
slots=2000

usage() {
  cat <<'EOF'
usage: ./scripts/run_thousand_seed_recommended.sh [--seeds N] [--slots N] [--low-dir DIR] [--severe-dir DIR]

Replicate the thousand-seed low/severe-congestion evidence under the adopted
recommended construction (0.75 standard target, 10-block processed-block
standard window, age-escape reset at certification), paired against flat fee.

By default this runs seeds 0-999 for 2,000 slots at low and severe-congestion
load (4,000 summary-only simulations, roughly 2 to 2.5 hours), then writes the
preserved evidence record to
docs/phase-2/CIP-urgency-signalling/thousand-seed-low-severe-recommended.json.

The historical record (thousand-seed-low-severe.json) is left in place; it
describes the historical canonical calibration.

The script refuses to overwrite existing output directories. --seeds and
--slots are intended for quick harness checks; omit them for the real run.
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
    --low-dir)
      [[ $# -ge 2 && -n "$2" ]] || { usage >&2; exit 2; }
      low_dir="$2"
      shift 2
      ;;
    --severe-dir)
      [[ $# -ge 2 && -n "$2" ]] || { usage >&2; exit 2; }
      severe_dir="$2"
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

for dir in "$low_dir" "$severe_dir"; do
  if [[ -e "$dir" ]]; then
    echo "error: refusing to overwrite existing output directory: $dir" >&2
    exit 2
  fi
done

SECONDS=0
stack build
simulator="$(stack path --local-install-root)/bin/abstract-sim-hs-exe"
if [[ ! -x "$simulator" ]]; then
  echo "error: built simulator executable not found: $simulator" >&2
  exit 2
fi
simulator_hash_line="$(sha256sum -- "$simulator")"
simulator_sha256="${simulator_hash_line%% *}"

echo "running low ($seeds seeds)..."
"$simulator" sweep "$manifest" \
  --seeds "$seeds" \
  --slots "$slots" \
  --summary-only \
  --load low \
  --out "$low_dir"

echo "running severe-congestion ($seeds seeds)..."
"$simulator" sweep "$manifest" \
  --seeds "$seeds" \
  --slots "$slots" \
  --summary-only \
  --load-profile config/loads/severe-congestion.json \
  --out "$severe_dir"

# A reduced run must never overwrite the canonical evidence record.
record_args=()
if (( seeds != 1000 )) || (( slots != 2000 )); then
  harness_record="$low_dir/thousand-seed-record-harness.json"
  echo "note: non-canonical run (seeds=$seeds, slots=$slots): writing the record to $harness_record"
  record_args=(--json-output "$harness_record")
fi

python3 scripts/compare_thousand_seed_recommended.py \
  --low-dir "$low_dir" \
  --severe-dir "$severe_dir" \
  "${record_args[@]}" \
  --sweep-executable-sha256 "$simulator_sha256" \
  --generated-at "$(date +%Y-%m-%d)"

echo "thousand-seed replication (recommended construction) complete in ${SECONDS}s"
