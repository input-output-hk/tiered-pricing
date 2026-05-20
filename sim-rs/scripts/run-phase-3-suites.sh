#!/usr/bin/env bash
# Run the Phase-3 (Cardano Improvement Proposal evidence) suites.
#
# Usage:
#   scripts/run-phase-3-suites.sh [N] [SUITE ...]
#   PHASE3_RUN_ID=<id> scripts/run-phase-3-suites.sh [N] [SUITE ...]
#
# N defaults to 1 (cross-suite SEQUENTIAL; one suite at a time, each
# parallelised internally by experiment-suite's `-P 8` default). Per the
# 03-02-PLAN.md parallelism advisory: on the dev machine prefer
# cross-suite-sequential because the cross-suite K × intra-suite P
# product otherwise oversubscribes cores. Raise N (e.g. 2 or 3) only on
# hardware with ≥ 16 cores AND > 64 GB RSS — and consider lowering
# `--parallelism` on each suite when you do.
#
# SUITE list defaults to all Phase-3 suites (Wave 1 scoping + all five
# Wave 2 suites). Pass an explicit suite list to run only a subset, e.g.
# `scripts/run-phase-3-suites.sh 1 parameters/phase-2-sweep/suites/phase-3-multiplier-floor-16-companion.yaml`.
#
# Each invocation timestamps the per-suite output dirs with a single
# batch identifier (`PHASE3_RUN_ID`, default UTC `YYYYMMDD-HHMMSS`)
# so concurrent suites land under matching paths and successive runs
# don't collide. `experiment-suite run --run-id <id>` is resumable —
# re-running with the same id preserves Completed (job, seed) pairs.
#
# Output layout:
#   sim-rs/output/phase-3/<suite-base>-<run-id>/<job>/<seed>/...
#
# Phase-3 suites are NOT goldens-pinned (per CONTEXT.md D-25). The
# pricing event stream determinism is still enforced per-(job, seed)
# via `experiment-suite verify`, but no entries are added to
# parameters/phase-2-sweep/suites/.goldens/ or
# sim-cli/tests/determinism.rs.

set -euo pipefail
cd "$(dirname "$0")/.."

if [[ "${1:-}" =~ ^[0-9]+$ ]]; then
  parallelism="$1"
  shift
else
  parallelism=1
fi

run_id="${PHASE3_RUN_ID:-$(date -u +%Y%m%d-%H%M%S)}"

if [[ $# -gt 0 ]]; then
  suites=("$@")
else
  # Wave 2 suite ordering: light suites first (fast feedback), then
  # heavy compute (TEST-05 pool-number-sensitivity is ~1650 runs and
  # dominates total wall-clock). The cross-suite default of N=1 means
  # these run sequentially, so order = wall-clock priority.
  suites=(
    # Wave 1 (already runnable; resumable so re-execution skips done seeds):
    parameters/phase-2-sweep/suites/phase-3-scoping.yaml
    # Wave 2 (gated on Wave 1; seed counts set per scoping-results.md):
    parameters/phase-2-sweep/suites/phase-3-multiplier-floor-16-companion.yaml   # TEST-07a — 6 jobs × 5 seeds (fastest)
    parameters/phase-2-sweep/suites/phase-3-canonical-variance.yaml              # TEST-04 — 5 jobs × 20 seeds
    parameters/phase-2-sweep/suites/phase-3-sign-flip-variance.yaml              # TEST-03 — 6 jobs × 20 seeds
    parameters/phase-2-sweep/suites/phase-3-run-length.yaml                      # TEST-06 — 12 jobs × 10 seeds, 2000/4000/8000 slots
    parameters/phase-2-sweep/suites/phase-3-pool-number-sensitivity.yaml         # TEST-05 — 330 jobs × 5 seeds (heaviest)
  )
fi

if [[ ! -x ./target/release/experiment-suite ]]; then
  echo "experiment-suite binary not found; building..." >&2
  cargo build --release --bin experiment-suite
fi

echo "Running ${#suites[@]} suite(s) with cross-suite parallelism=${parallelism}, run-id=${run_id}" >&2
echo "  Each suite uses experiment-suite's intra-suite default (-P 8) unless its own header overrides." >&2

printf '%s\n' "${suites[@]}" \
  | xargs -n1 -P "$parallelism" -I {} \
      ./target/release/experiment-suite run --run-id "$run_id" {}

echo "" >&2
echo "All ${#suites[@]} suite(s) finished. Manifests:" >&2
for s in "${suites[@]}"; do
  base="$(basename "$s" .yaml)"
  echo "  output/phase-3/${base}-${run_id}/manifest.json" >&2
done
