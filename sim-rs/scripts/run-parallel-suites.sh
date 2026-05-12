#!/usr/bin/env bash
# Run the phase-2 welfare suites in parallel.
#
# Usage:
#   scripts/run-parallel-suites.sh [N] [SUITE ...]
#   M6_RUN_ID=<id> scripts/run-parallel-suites.sh [N] [SUITE ...]
#
# N defaults to the number of CPUs. SUITE list defaults to all 16
# phase-2 suites (post-M6 multi-node).
#
# Each invocation timestamps the per-suite output dirs with a single
# batch identifier so concurrent suites land under matching paths and
# successive invocations don't collide. The id is `YYYYMMDD-HHMMSS`
# UTC by default; override by exporting `M6_RUN_ID` (e.g. to resume a
# previous batch). Outputs land at
#   sim-rs/output/phase-2/<suite>-<run-id>/
#
# Each suite runs in its own process; jobs within a suite still run
# sequentially because the experiment-suite runner is single-threaded.
# So the effective parallelism is capped at min(N, len(SUITE)).
#
# Multi-node (CIP-0164 topology, 600 pools) is roughly 30x slower per
# run than the M5-era single-producer baseline. Per-suite wall time
# ranges from ~5 min (eip1559-smoothing) to ~30 min (priority-only-*).
# Expect 1-3 hours total at -P 10 on the full 16-suite batch.
#
# Invokes `experiment-suite run --run-id <id>`, which is resumable —
# re-running with the same id preserves Completed (job, seed) pairs;
# omitting it (or exporting a new id) starts a fresh dir.

set -euo pipefail

cd "$(dirname "$0")/.."

if [[ "${1:-}" =~ ^[0-9]+$ ]]; then
  parallelism="$1"
  shift
else
  parallelism="$(nproc 2>/dev/null || echo 4)"
fi

run_id="${M6_RUN_ID:-$(date -u +%Y%m%d-%H%M%S)}"

if [[ $# -gt 0 ]]; then
  suites=("$@")
else
  suites=(
    parameters/phase-2-sweep/suites/phase-2-eip1559-robustness.yaml
    parameters/phase-2-sweep/suites/phase-2-eip1559-smoothing.yaml
    parameters/phase-2-sweep/suites/phase-2-priority-only-rb-reserved.yaml
    parameters/phase-2-sweep/suites/phase-2-priority-only-unreserved.yaml
    parameters/phase-2-sweep/suites/phase-2-two-lane-both-dynamic.yaml
    parameters/phase-2-sweep/suites/phase-2-rb-scarcity.yaml
    parameters/phase-2-sweep/suites/phase-2-urgency-inversion.yaml
    parameters/phase-2-sweep/suites/phase-2-moderate-singlelane.yaml
    parameters/phase-2-sweep/suites/phase-2-moderate-priority-only.yaml
    parameters/phase-2-sweep/suites/phase-2-moderate-both-dynamic.yaml
    parameters/phase-2-sweep/suites/phase-2-congested-singlelane.yaml
    parameters/phase-2-sweep/suites/phase-2-congested-priority-only.yaml
    parameters/phase-2-sweep/suites/phase-2-congested-both-dynamic.yaml
    parameters/phase-2-sweep/suites/phase-2-sundaeswap-singlelane.yaml
    parameters/phase-2-sweep/suites/phase-2-sundaeswap-priority-only.yaml
    parameters/phase-2-sweep/suites/phase-2-sundaeswap-both-dynamic.yaml
  )
fi

if [[ ! -x ./target/release/experiment-suite ]]; then
  echo "experiment-suite binary not found; building..." >&2
  cargo build --release --bin experiment-suite
fi

echo "Running ${#suites[@]} suite(s) with parallelism=${parallelism}, run-id=${run_id}" >&2

printf '%s\n' "${suites[@]}" \
  | xargs -n1 -P "$parallelism" -I {} \
      ./target/release/experiment-suite run --run-id "$run_id" {}
