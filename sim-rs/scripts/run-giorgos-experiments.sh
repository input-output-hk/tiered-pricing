#!/usr/bin/env bash
# Run the three smoke suites that include Giorgos' design
# (RB-reserved both-dynamic with EB-pays-standard, FIFO).
#
# Usage:
#   scripts/run-giorgos-experiments.sh [CROSS_SUITE_N] [INNER_P]
#   GIORGOS_RUN_ID=<id> scripts/run-giorgos-experiments.sh [CROSS_SUITE_N] [INNER_P]
#
# CROSS_SUITE_N defaults to 3 (all three suites concurrently).
# INNER_P defaults to 4 (intra-suite parallelism per
# `experiment-suite run --parallelism`); total concurrent (job, seed)
# workers ≈ CROSS_SUITE_N × INNER_P (here 12 by default).
#
# The defaults are tuned for the dev machine. On smaller boxes drop
# either knob to fit RSS; on larger boxes raise INNER_P toward the
# experiment-suite default of 8.
#
# Each invocation timestamps all three suites' output dirs with a
# single batch identifier (`GIORGOS_RUN_ID`, default UTC
# `YYYYMMDD-HHMMSS`) so the three runs land under matching paths and
# successive invocations don't collide. `experiment-suite run
# --run-id <id>` is resumable — re-running with the same id preserves
# Completed (job, seed) pairs.
#
# Output layout:
#   sim-rs/output/robustness/fifo-smoke-congested-<id>/<job>/<seed>/...
#   sim-rs/output/robustness/fifo-smoke-overcapacity-<id>/<job>/<seed>/...
#   sim-rs/output/robustness/fifo-smoke-sundaeswap-overcapacity-<id>/<job>/<seed>/...
#
# Each of the three suites carries seven jobs:
#   - rb_reserved_priority_only_static_fifo_x4         (menu option 1)
#   - unreserved_priority_only_static_fifo_x4          (menu option 2)
#   - rb_reserved_both_dynamic_fifo_x4                 (menu option 3)
#   - giorgos_design_rb_reserved_both_dynamic_eb_standard_fifo_x4
#   - unreserved_both_dynamic_fifo_x4                  (menu option 4)
#   - control_baseline_flat_fee
#   - control_eip1559_d8_t50_w32

set -euo pipefail

cd "$(dirname "$0")/.."

if [[ "${1:-}" =~ ^[0-9]+$ ]]; then
  cross_suite_n="$1"
  shift
else
  cross_suite_n=3
fi

if [[ "${1:-}" =~ ^[0-9]+$ ]]; then
  inner_p="$1"
  shift
else
  inner_p=4
fi

run_id="${GIORGOS_RUN_ID:-$(date -u +%Y%m%d-%H%M%S)}"

suites=(
  parameters/phase-2-sweep/suites/robustness-fifo-smoke-congested.yaml
  parameters/phase-2-sweep/suites/robustness-fifo-smoke-overcapacity.yaml
  parameters/phase-2-sweep/suites/robustness-fifo-smoke-sundaeswap-overcapacity.yaml
)

if [[ ! -x ./target/release/experiment-suite ]]; then
  echo "experiment-suite binary not found; building..." >&2
  cargo build --release --bin experiment-suite
fi

echo "Running ${#suites[@]} suite(s):" >&2
printf '  %s\n' "${suites[@]}" >&2
echo "  cross-suite parallelism = ${cross_suite_n}" >&2
echo "  intra-suite parallelism = ${inner_p} (per experiment-suite --parallelism)" >&2
echo "  total concurrent (job, seed) workers ≈ $((cross_suite_n * inner_p))" >&2
echo "  run id  = ${run_id}" >&2

printf '%s\n' "${suites[@]}" \
  | xargs -n1 -P "$cross_suite_n" -I {} \
      ./target/release/experiment-suite run \
        --parallelism "$inner_p" \
        --run-id "$run_id" \
        {}
