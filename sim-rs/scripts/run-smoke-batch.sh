#!/usr/bin/env bash
# Multi-node smoke batch — informative-minimal sanity check.
#
# Usage:
#   scripts/run-smoke-batch.sh [N]
#
# N defaults to 16. Runs the three Sundaeswap cross-mechanism suites at
# seed=1 only, decomposed into one per-job mini-suite per (parent suite,
# job) so jobs run independently in parallel (rather than sequentially
# within each parent suite).
#
# Coverage:
#   - sundaeswap-singlelane     ( 8 jobs)
#   - sundaeswap-priority-only  (15 jobs)
#   - sundaeswap-both-dynamic   (10 jobs)
# = 33 per-job mini-suites at seed=1 = 33 simulator processes total,
# scheduled across N concurrent workers (true N-wide parallelism).
#
# Output layout:
#   output/phase-2/smoke/<parent-suite-base>/<job-name>/<job-name>/1/...
# The doubled <job-name> is because each per-job mini-suite has its own
# output-dir keyed by job, and the runner writes
# `<output-dir>/<job-name>/<seed>/`. To aggregate across families:
#   find output/phase-2/smoke -name run_summary.json | xargs jq ...

set -euo pipefail
cd "$(dirname "$0")/.."

PARALLELISM=${1:-16}
TMP=$(mktemp -d -t phase2-smoke-XXXXXX)
trap "rm -rf $TMP" EXIT

PARENT_SUITES=(
  parameters/phase-2-sweep/suites/phase-2-sundaeswap-singlelane.yaml
  parameters/phase-2-sweep/suites/phase-2-sundaeswap-priority-only.yaml
  parameters/phase-2-sweep/suites/phase-2-sundaeswap-both-dynamic.yaml
)

# Decompose each parent suite into per-job mini-suites in $TMP. Each
# mini-suite has a unique output-dir keyed on (parent, job), so
# concurrent writers never collide on manifest.json.
python3 - "$TMP" "${PARENT_SUITES[@]}" <<'PYEOF' > "$TMP/mini-suites.txt"
import sys, os, yaml

tmp = sys.argv[1]
suites = sys.argv[2:]

for src in suites:
    with open(src) as f:
        parent = yaml.safe_load(f)
    base = os.path.splitext(os.path.basename(src))[0].replace("phase-2-", "")
    for job in parent.get("jobs", []):
        job_name = job["name"]
        mini = {
            "suite-name": f"smoke-{base}-{job_name}",
            "output-dir": f"output/phase-2/smoke/{base}/{job_name}",
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
echo "Smoke batch: $JOB_COUNT per-job mini-suites at seed=1, parallelism=$PARALLELISM" >&2

if [[ ! -x ./target/release/experiment-suite ]]; then
  echo "Building experiment-suite..." >&2
  cargo build --release --bin experiment-suite
fi

xargs -P "$PARALLELISM" -I {} ./target/release/experiment-suite run {} < "$TMP/mini-suites.txt"

echo "Done. Run summaries:" >&2
find output/phase-2/smoke -name run_summary.json | wc -l
