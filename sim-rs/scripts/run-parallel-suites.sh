#!/usr/bin/env bash
# Run a batch of experiment-suite YAMLs in parallel.
#
# Usage:
#   scripts/run-parallel-suites.sh [N] [SUITE ...]
#
# N defaults to the number of CPUs. SUITE list defaults to the moderate +
# congested cross-mechanism sweep (6 suites covering 33 jobs × 2 demands).
#
# Each suite runs in its own process; jobs *within* a suite still run
# sequentially because the experiment-suite runner is single-threaded.
# So the effective parallelism is capped at min(N, len(SUITE)).
#
# Invokes `experiment-suite run`, which is resumable — partial output
# from a prior run is preserved and only the missing (job, seed) pairs
# are executed.

set -euo pipefail

cd "$(dirname "$0")/.."

if [[ "${1:-}" =~ ^[0-9]+$ ]]; then
  parallelism="$1"
  shift
else
  parallelism="$(nproc 2>/dev/null || echo 4)"
fi

if [[ $# -gt 0 ]]; then
  suites=("$@")
else
  suites=(
    parameters/phase-2-sweep/suites/phase-2-moderate-singlelane.yaml
    parameters/phase-2-sweep/suites/phase-2-moderate-priority-only.yaml
    parameters/phase-2-sweep/suites/phase-2-moderate-both-dynamic.yaml
    parameters/phase-2-sweep/suites/phase-2-congested-singlelane.yaml
    parameters/phase-2-sweep/suites/phase-2-congested-priority-only.yaml
    parameters/phase-2-sweep/suites/phase-2-congested-both-dynamic.yaml
  )
fi

if [[ ! -x ./target/release/experiment-suite ]]; then
  echo "experiment-suite binary not found; building..." >&2
  cargo build --release --bin experiment-suite
fi

echo "Running ${#suites[@]} suite(s) with parallelism=${parallelism}" >&2

printf '%s\n' "${suites[@]}" \
  | xargs -n1 -P "$parallelism" -I {} \
      ./target/release/experiment-suite run {}
