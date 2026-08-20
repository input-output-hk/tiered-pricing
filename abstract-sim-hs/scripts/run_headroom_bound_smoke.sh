#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_dir="$(cd -- "$script_dir/.." && pwd)"
manifest="config/sweeps/headroom-bound-smoke.json"
out_dir="sweep-results/headroom-bound-smoke"
seed_start=430
seeds=10
slots=2000

usage() {
  cat <<'EOF'
usage: ./scripts/run_headroom_bound_smoke.sh [--out DIR] [--seed-start N] [--seeds N] [--slots N]

Run a paired two-arm ablation of the fee-cap headroom bound:

  legacy-floor   worst-case next price bounded by max(1/D, (1-target)/(target*D))
  upward-step    worst-case next price bounded by the pure upward step (1-target)/(target*D)

The bound feeds mempool admission and EB-producer selection. The two arms
coincide at the urgent target 0.5, so only the standard lane's checks differ:
at target 0.75 the legacy floor demands 6.25% of headroom where the pure
upward step demands 2.08%. Everything else remains fixed at S=.75/U=.50, D16,
half-RB threshold, and K10.

The default run uses ten paired seeds, 2,000 slots, independent RNG streams,
and all five headline loads. It is sized as a directional smoke test on the
development machine; it is not confirmatory evidence.

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

python3 scripts/compare_headroom_bound_smoke.py \
  --root "$out_dir" \
  --manifest "$manifest" \
  --simulator-sha256 "$simulator_sha256" \
  --markdown-output "$out_dir/comparison.md" \
  --json-output "$out_dir/comparison.json"

echo "headroom-bound ablation complete in ${SECONDS}s:"
echo "  report: $out_dir/comparison.md"
echo "  data:   $out_dir/comparison.json"
