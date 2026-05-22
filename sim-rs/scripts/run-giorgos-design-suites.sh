#!/usr/bin/env bash
# Run the four standalone Giorgos-design smoke suites in parallel.
#
# These are the single-(job, seed) suites — one Giorgos design job
# (RB-reserved both-dynamic, EB pays standard, FIFO) per demand
# regime: moderate (sundaeswap_moderate), congested
# (paper_like_congested), overcapacity (paper_like_overcapacity), and
# sundaeswap-overcapacity (sundaeswap_overcapacity). Each suite has
# just one (job, seed) pair, so intra-suite `--parallelism` has nothing
# to chew on; only cross-suite parallelism matters here.
#
# For the larger fifo-smoke suites (7 jobs each including Giorgos as
# one menu arm) see scripts/run-giorgos-experiments.sh.
#
# Usage:
#   scripts/run-giorgos-design-suites.sh [N]
#   GIORGOS_RUN_ID=<id> scripts/run-giorgos-design-suites.sh [N]
#
# N defaults to 4 (all suites concurrently). Each suite is effectively
# single-CPU (the simulator uses a per-thread current_thread tokio
# runtime per CLAUDE.md), so running 4 in parallel uses ~4 cores.
#
# Each invocation timestamps the per-suite output dirs with a single
# batch identifier so concurrent suites land under matching paths and
# successive invocations don't collide. Default id is `YYYYMMDD-HHMMSS`
# UTC; override by exporting `GIORGOS_RUN_ID` (e.g. to resume a
# previous batch — `experiment-suite run --run-id <id>` is resumable
# and preserves Completed (job, seed) pairs).
#
# Output layout:
#   sim-rs/output/robustness/giorgos-design-smoke-<id>/...
#   sim-rs/output/robustness/giorgos-design-smoke-congested-<id>/...
#   sim-rs/output/robustness/giorgos-design-smoke-overcapacity-<id>/...
#   sim-rs/output/robustness/giorgos-design-smoke-sundaeswap-overcapacity-<id>/...

set -euo pipefail

cd "$(dirname "$0")/.."

if [[ "${1:-}" =~ ^[0-9]+$ ]]; then
  parallelism="$1"
  shift
else
  parallelism=4
fi

run_id="${GIORGOS_RUN_ID:-$(date -u +%Y%m%d-%H%M%S)}"

suites=(
  parameters/phase-2-sweep/suites/robustness-giorgos-design-smoke.yaml
  parameters/phase-2-sweep/suites/robustness-giorgos-design-smoke-congested.yaml
  parameters/phase-2-sweep/suites/robustness-giorgos-design-smoke-overcapacity.yaml
  parameters/phase-2-sweep/suites/robustness-giorgos-design-smoke-sundaeswap-overcapacity.yaml
)

if [[ ! -x ./target/release/experiment-suite ]]; then
  echo "experiment-suite binary not found; building..." >&2
  cargo build --release --bin experiment-suite
fi

echo "Running ${#suites[@]} Giorgos design smoke suite(s):" >&2
printf '  %s\n' "${suites[@]}" >&2
echo "  cross-suite parallelism = ${parallelism}" >&2
echo "  run id  = ${run_id}" >&2

printf '%s\n' "${suites[@]}" \
  | xargs -n1 -P "$parallelism" -I {} \
      ./target/release/experiment-suite run --run-id "$run_id" {}
