#!/usr/bin/env bash
# Paper-like NON-saturating fast-screen smoke batch.
#
# Sibling of scripts/run-smoke-paper-like-congested.sh, but covers
# both paper_like_moderate and paper_like_realistic in a single xargs
# invocation so 16 workers stay busy across the full job set.
#
# Usage:
#   scripts/run-smoke-paper-like-light.sh [N] [--include-rb-sweeps] [--dry-run]
#
# N defaults to 16. Same trimming policy as the congested fast-screen:
#   - phase-2-{moderate,realistic}-singlelane:      baseline_flat_fee only
#   - phase-2-{moderate,realistic}-priority-only:   default-protocol jobs
#   - phase-2-{moderate,realistic}-both-dynamic:    default-protocol jobs
# = 11 per-demand × 2 demands = 22 per-job mini-suites at seed=1.
# With --include-rb-sweeps the RB-reduced variants are kept too.
#
# Topology: parameters/topology.default.yaml is reused, with
# tx-generation-weight: 1 added to every node so each acts as an
# actor source. Per-component demand rates are scaled by 1/N so the
# global rate matches the parent suite. Same trick as the congested
# script.
#
# Output layout:
#   output/phase-2/smoke/paper-like-light-<run-id>/<family>/<job>/<job>/1/...
#
# PAPER_LIKE_LIGHT_RUN_ID / PAPER_LIKE_LIGHT_TOPOLOGY / PAPER_LIKE_LIGHT_SLOTS
# override the corresponding default the same way as the congested script.

set -euo pipefail
SCRIPT_PATH="$(realpath "$0")"
cd "$(dirname "$SCRIPT_PATH")/.."

PARALLELISM=16
INCLUDE_RB_SWEEPS=0
DRY_RUN=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --include-rb-sweeps)
      INCLUDE_RB_SWEEPS=1
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --help|-h)
      sed -n '1,29p' "$SCRIPT_PATH"
      exit 0
      ;;
    ''|*[!0-9]*)
      echo "Unknown argument: $1" >&2
      exit 2
      ;;
    *)
      PARALLELISM="$1"
      shift
      ;;
  esac
done

RUN_ID="${PAPER_LIKE_LIGHT_RUN_ID:-$(date -u +%Y%m%d-%H%M%S)}"
SLOTS="${PAPER_LIKE_LIGHT_SLOTS:-}"
OUTPUT_ROOT="output/phase-2/smoke/paper-like-light-${RUN_ID}"
TMP=$(mktemp -d -t phase2-paper-like-light-XXXXXX)
trap "rm -rf $TMP" EXIT

if [[ -n "$SLOTS" && ! "$SLOTS" =~ ^[0-9]+$ ]]; then
  echo "PAPER_LIKE_LIGHT_SLOTS must be a non-negative integer: $SLOTS" >&2
  exit 2
fi

GENERATED_TOPOLOGY=0
if [[ -n "${PAPER_LIKE_LIGHT_TOPOLOGY:-}" ]]; then
  TOPOLOGY="$PAPER_LIKE_LIGHT_TOPOLOGY"
else
  TOPOLOGY="$TMP/topology-default-actor-source.yaml"
  GENERATED_TOPOLOGY=1
  SOURCE_COUNT=$(python3 - parameters/topology.default.yaml "$TOPOLOGY" <<'PYEOF'
import sys
import yaml

src, dst = sys.argv[1], sys.argv[2]
with open(src) as f:
    topology = yaml.safe_load(f)

nodes = topology.get("nodes") or {}
if "node-0" not in nodes:
    raise SystemExit("parameters/topology.default.yaml does not contain node-0")

for node in nodes.values():
    node["tx-generation-weight"] = 1

with open(dst, "w") as f:
    yaml.safe_dump(topology, f, sort_keys=False)

print(len(nodes))
PYEOF
)
fi
SOURCE_COUNT="${SOURCE_COUNT:-1}"

PARENT_SUITES=(
  parameters/phase-2-sweep/suites/phase-2-moderate-singlelane.yaml
  parameters/phase-2-sweep/suites/phase-2-moderate-priority-only.yaml
  parameters/phase-2-sweep/suites/phase-2-moderate-both-dynamic.yaml
  parameters/phase-2-sweep/suites/phase-2-realistic-singlelane.yaml
  parameters/phase-2-sweep/suites/phase-2-realistic-priority-only.yaml
  parameters/phase-2-sweep/suites/phase-2-realistic-both-dynamic.yaml
)

python3 - "$TMP" "$OUTPUT_ROOT" "$TOPOLOGY" "$SLOTS" "$INCLUDE_RB_SWEEPS" "$SOURCE_COUNT" "${PARENT_SUITES[@]}" <<'PYEOF' > "$TMP/mini-suites.txt"
import copy
import os
import sys
import yaml

tmp = sys.argv[1]
output_root = sys.argv[2]
topology = sys.argv[3]
slots_override = sys.argv[4]
include_rb_sweeps = sys.argv[5] == "1"
source_count = int(sys.argv[6])
suites = sys.argv[7:]
scaled_demands = {}

def include_job(parent_base, job_name):
    if parent_base.endswith("-singlelane"):
        return job_name == "baseline_flat_fee"
    if parent_base.endswith("-priority-only") or parent_base.endswith("-both-dynamic"):
        if include_rb_sweeps:
            return True
        return "_rb_" not in job_name
    return False

def scale_rate(rate, denom):
    if isinstance(rate, (int, float)):
        return rate / denom
    if isinstance(rate, dict) and "phases" in rate:
        scaled = copy.deepcopy(rate)
        for phase in scaled["phases"]:
            phase["rate"] = phase["rate"] / denom
        return scaled
    raise TypeError(f"unsupported arrival-rate-per-slot shape: {rate!r}")

def demand_path_for(path):
    if source_count <= 1:
        return path
    if path not in scaled_demands:
        with open(path) as f:
            demand = yaml.safe_load(f)
        components = demand.get("actors", {}).get("components", [])
        for component in components:
            component["arrival-rate-per-slot"] = scale_rate(
                component["arrival-rate-per-slot"],
                source_count,
            )
        dst = os.path.join(
            tmp,
            f"demand-scaled-x{source_count}-{os.path.basename(path)}",
        )
        with open(dst, "w") as out:
            yaml.safe_dump(demand, out, sort_keys=False)
        scaled_demands[path] = dst
    return scaled_demands[path]

for src in suites:
    with open(src) as f:
        parent = yaml.safe_load(f)
    base = os.path.splitext(os.path.basename(src))[0].replace("phase-2-", "")
    for job in parent.get("jobs", []):
        job_name = job["name"]
        if not include_job(base, job_name):
            continue
        mini = {
            "suite-name": f"smoke-paper-like-{base}-{job_name}",
            "output-dir": f"{output_root}/{base}/{job_name}",
            "seeds": [1],
            "default-slots": int(slots_override) if slots_override else parent["default-slots"],
            "default-topology": topology,
            "default-protocol": parent["default-protocol"],
            "default-demand": demand_path_for(parent["default-demand"]),
            "jobs": [job],
        }
        dst = os.path.join(tmp, f"{base}--{job_name}.yaml")
        with open(dst, "w") as out:
            yaml.safe_dump(mini, out, sort_keys=False)
        print(dst)
PYEOF

JOB_COUNT=$(wc -l < "$TMP/mini-suites.txt")
echo "Paper-like light fast-screen smoke: ${JOB_COUNT} per-job mini-suites at seed=1, parallelism=${PARALLELISM}" >&2
if [[ "$GENERATED_TOPOLOGY" == "1" ]]; then
  echo "Topology: generated from parameters/topology.default.yaml with tx-generation-weight=1 on ${SOURCE_COUNT} nodes" >&2
  echo "Demand: generated per-source overlay scaled by 1/${SOURCE_COUNT} to preserve global arrival rates" >&2
else
  echo "Topology: ${TOPOLOGY}" >&2
fi
if [[ -n "$SLOTS" ]]; then
  echo "Slots: ${SLOTS}" >&2
fi
echo "Output root: ${OUTPUT_ROOT}" >&2

if [[ "$DRY_RUN" == "1" ]]; then
  cat "$TMP/mini-suites.txt"
  exit 0
fi

if [[ "$GENERATED_TOPOLOGY" == "1" ]]; then
  mkdir -p "$OUTPUT_ROOT"
  cp "$TOPOLOGY" "$OUTPUT_ROOT/topology-default-actor-source.yaml"
  find "$TMP" -maxdepth 1 -name 'demand-scaled-*' -exec cp {} "$OUTPUT_ROOT/" \;
fi

echo "Building experiment-suite..." >&2
cargo build --release --bin experiment-suite

xargs -P "$PARALLELISM" -I {} ./target/release/experiment-suite run {} < "$TMP/mini-suites.txt"

python3 - "$OUTPUT_ROOT" <<'PYEOF'
import json
import os
import sys

root = sys.argv[1]
zero_tx = []
summary_count = 0

for dirpath, _, filenames in os.walk(root):
    if "run_summary.json" not in filenames:
        continue
    summary_count += 1
    path = os.path.join(dirpath, "run_summary.json")
    with open(path) as f:
        summary = json.load(f)
    if int(summary.get("total_txs_submitted", 0)) == 0:
        zero_tx.append(path)

if summary_count and zero_tx:
    print("ERROR: completed smoke produced zero submitted transactions:", file=sys.stderr)
    for path in zero_tx[:10]:
        print(f"  {path}", file=sys.stderr)
    if len(zero_tx) > 10:
        print(f"  ... and {len(zero_tx) - 10} more", file=sys.stderr)
    sys.exit(3)
PYEOF

echo "Done. Run summaries under ${OUTPUT_ROOT}:" >&2
find "$OUTPUT_ROOT" -name run_summary.json | wc -l
