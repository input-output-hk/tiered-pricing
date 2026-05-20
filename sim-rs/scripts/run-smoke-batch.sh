#!/usr/bin/env bash
# Multi-node smoke batch — informative-minimal sanity check.
#
# Usage:
#   scripts/run-smoke-batch.sh [N]
#   SMOKE_BATCH_RUN_ID=<id> scripts/run-smoke-batch.sh [N]
#
# N defaults to 16. Runs the three Sundaeswap cross-mechanism suites at
# seed=1 only, decomposed into one per-job mini-suite per (parent suite,
# job) so jobs run independently in parallel (rather than sequentially
# within each parent suite).
#
# Each invocation timestamps the per-batch output dir so successive
# invocations never collide on `manifest.json` (the experiment-suite
# runner is resumable and would otherwise skip Completed (job, seed)
# pairs from a prior batch). The id is `YYYYMMDD-HHMMSS` UTC by default;
# override by exporting `SMOKE_BATCH_RUN_ID` to resume a specific batch.
#
# Coverage:
#   - sundaeswap-singlelane     ( 8 jobs)
#   - sundaeswap-priority-only  (15 jobs)
#   - sundaeswap-both-dynamic   (10 jobs)
# = 33 per-job mini-suites at seed=1 = 33 simulator processes total,
# scheduled across N concurrent workers (true N-wide parallelism).
#
# Output layout:
#   output/phase-2/smoke/sundaeswap-batch-<run-id>/<parent-suite-base>/<job-name>/<job-name>/1/...
# The doubled <job-name> is because each per-job mini-suite has its own
# output-dir keyed by job, and the runner writes
# `<output-dir>/<job-name>/<seed>/`. To aggregate across families
# within one batch:
#   find output/phase-2/smoke/sundaeswap-batch-<run-id> -name run_summary.json | xargs jq ...

set -euo pipefail
cd "$(dirname "$0")/.."

PARALLELISM=${1:-16}
RUN_ID="${SMOKE_BATCH_RUN_ID:-$(date -u +%Y%m%d-%H%M%S)}"
OUTPUT_ROOT="output/phase-2/smoke/sundaeswap-batch-${RUN_ID}"
TMP=$(mktemp -d -t phase2-smoke-XXXXXX)
trap "rm -rf $TMP" EXIT

PARENT_SUITES=(
  parameters/phase-2-sweep/suites/phase-2-sundaeswap-singlelane.yaml
  parameters/phase-2-sweep/suites/phase-2-sundaeswap-priority-only.yaml
  parameters/phase-2-sweep/suites/phase-2-sundaeswap-both-dynamic.yaml
)

# Decompose each parent suite into per-job mini-suites in $TMP. Each
# mini-suite has a unique output-dir keyed on (run-id, parent, job), so
# concurrent writers never collide on manifest.json and successive
# invocations never resume a prior batch's Completed work.
python3 - "$TMP" "$OUTPUT_ROOT" "${PARENT_SUITES[@]}" <<'PYEOF' > "$TMP/mini-suites.txt"
import sys, os, yaml

tmp = sys.argv[1]
output_root = sys.argv[2]
suites = sys.argv[3:]

for src in suites:
    with open(src) as f:
        parent = yaml.safe_load(f)
    base = os.path.splitext(os.path.basename(src))[0].replace("phase-2-", "")
    for job in parent.get("jobs", []):
        job_name = job["name"]
        mini = {
            "suite-name": f"smoke-{base}-{job_name}",
            "output-dir": f"{output_root}/{base}/{job_name}",
            "seeds": [1],
            "default-slots": parent["default-slots"],
            "default-topology": parent["default-topology"],
            "default-protocol": parent["default-protocol"],
            "default-demand": parent["default-demand"],
            "jobs": [job],
        }
        dst = os.path.join(tmp, f"{base}--{job_name}.yaml")
        with open(dst, "w") as out:
            yaml.safe_dump(mini, out, sort_keys=False)
        print(dst)
PYEOF

JOB_COUNT=$(wc -l < "$TMP/mini-suites.txt")
echo "Smoke batch: $JOB_COUNT per-job mini-suites at seed=1, parallelism=$PARALLELISM, run-id=$RUN_ID" >&2
echo "Output root: $OUTPUT_ROOT" >&2

if [[ ! -x ./target/release/experiment-suite ]]; then
  echo "Building experiment-suite..." >&2
  cargo build --release --bin experiment-suite
fi

xargs -P "$PARALLELISM" -I {} ./target/release/experiment-suite run {} < "$TMP/mini-suites.txt"

echo "Done. Run summaries in $OUTPUT_ROOT:" >&2
find "$OUTPUT_ROOT" -name run_summary.json | wc -l
