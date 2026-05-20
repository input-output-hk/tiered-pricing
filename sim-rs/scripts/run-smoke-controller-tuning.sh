#!/usr/bin/env bash
# Controller-tuning sweep at multiplier_floor = 1.
#
# Goal: test whether a well-tuned both-dynamic controller pair can
# replace the multiplier-floor lever. Hypothesis: with floor=1, an
# appropriate (D, target, window) configuration discovers the right
# priority-vs-standard ratio organically and delivers welfare-positive
# results without needing the operator to pre-set the floor per epoch.
#
# Sweep (× both un-reserved and partitioned placements):
#   D       ∈ {4, 8, 16}
#   target  ∈ {1/4, 1/2, 3/4}
#   window  ∈ {8, 16, 32, 64}    (priority window is forced to 1 for
#                                 partitioned variant per M5 calibration)
# = 3 × 3 × 4 × 2 = 72 controller configurations.
# Plus 5 reference points (existing floor-based mechanisms + baseline)
# = 77 total jobs at seed = 1.
#
# Holds demand at paper_like_realistic per the experimental design.
#
# Usage:
#   scripts/run-smoke-controller-tuning.sh [N] [--dry-run]
# N defaults to 16.
#
# Output layout:
#   output/phase-2/smoke/controller-tuning-<run-id>/<job>/<job>/1/...

set -euo pipefail
SCRIPT_PATH="$(realpath "$0")"
cd "$(dirname "$SCRIPT_PATH")/.."

PARALLELISM=16
DRY_RUN=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=1; shift ;;
    --help|-h) sed -n '1,32p' "$SCRIPT_PATH"; exit 0 ;;
    ''|*[!0-9]*) echo "Unknown argument: $1" >&2; exit 2 ;;
    *) PARALLELISM="$1"; shift ;;
  esac
done

RUN_ID="${CONTROLLER_TUNING_RUN_ID:-$(date -u +%Y%m%d-%H%M%S)}"
OUTPUT_ROOT="output/phase-2/smoke/controller-tuning-${RUN_ID}"
TMP=$(mktemp -d -t phase2-controller-tuning-XXXXXX)
trap "rm -rf $TMP" EXIT

# Generate fast-screen topology (every node = actor source)
TOPOLOGY="$TMP/topology-default-actor-source.yaml"
SOURCE_COUNT=$(python3 - parameters/topology.default.yaml "$TOPOLOGY" <<'PYEOF'
import sys, yaml
src, dst = sys.argv[1], sys.argv[2]
with open(src) as f:
    t = yaml.safe_load(f)
for n in t.get("nodes", {}).values():
    n["tx-generation-weight"] = 1
with open(dst, "w") as f:
    yaml.safe_dump(t, f, sort_keys=False)
print(len(t.get("nodes", {})))
PYEOF
)

# Generate scaled paper_like_realistic demand (per-source rate)
python3 - "$TMP" "$SOURCE_COUNT" <<'PYEOF'
import os, sys, yaml, copy
tmp, sc = sys.argv[1], int(sys.argv[2])
src = "parameters/phase-2-sweep/demand/paper_like_realistic.yaml"
with open(src) as f:
    d = yaml.safe_load(f)
for c in d["actors"]["components"]:
    r = c["arrival-rate-per-slot"]
    if isinstance(r, (int, float)):
        c["arrival-rate-per-slot"] = r / sc
    else:
        for p in r["phases"]:
            p["rate"] = p["rate"] / sc
with open(os.path.join(tmp, "demand-realistic-scaled.yaml"), "w") as f:
    yaml.safe_dump(d, f, sort_keys=False)
PYEOF

# Generate the 72 floor=1 controller-sweep pricing configs + mini-suites,
# plus 5 reference points (existing floor mechanisms + baseline).
python3 - "$TMP" "$OUTPUT_ROOT" "$TOPOLOGY" <<'PYEOF' > "$TMP/mini-suites.txt"
import os, sys, yaml
tmp, output_root, topology = sys.argv[1], sys.argv[2], sys.argv[3]
demand_path = os.path.join(tmp, "demand-realistic-scaled.yaml")

def emit(job_name, pricing_path):
    mini = {
        "suite-name": f"controller-tuning-{job_name}",
        "output-dir": f"{output_root}/{job_name}",
        "seeds": [1],
        "default-slots": 2000,
        "default-topology": topology,
        "default-protocol": "parameters/phase-2-sweep/protocol-base.yaml",
        "default-demand": demand_path,
        "jobs": [{"name": job_name, "pricing": pricing_path}],
    }
    dst = os.path.join(tmp, f"suite-{job_name}.yaml")
    with open(dst, "w") as f:
        yaml.safe_dump(mini, f, sort_keys=False)
    print(dst)

# Sweep dimensions
Ds = [4, 8, 16]
targets = [(1, 4, "25"), (1, 2, "50"), (3, 4, "75")]
windows = [8, 16, 32, 64]
variants = [
    ("unreserved-both-dynamic", "unreserved", False),
    ("rb-reserved-both-dynamic", "partitioned", True),  # partitioned priority window forced to 1
]

for v_kind, v_label, partitioned in variants:
    for D in Ds:
        for tnum, tden, tlabel in targets:
            for w in windows:
                pw = 1 if partitioned else w
                pricing = {
                    "pricing": {
                        "kind": "two-lane",
                        "variant": v_kind,
                        "priority": {
                            "initial-quote-per-byte": 176,
                            "target-num": tnum,
                            "target-den": tden,
                            "max-change-denominator": D,
                            "window-length": pw,
                        },
                        "standard": {
                            "initial-quote-per-byte": 44,
                            "target-num": tnum,
                            "target-den": tden,
                            "max-change-denominator": D,
                            "window-length": w,
                        },
                        "multiplier-floor-num": 1,
                        "multiplier-floor-den": 1,
                        "lane-selection-order": "priority-first",
                    }
                }
                job = f"floor1_{v_label}_d{D}_t{tlabel}_w{w}"
                pricing_path = os.path.join(tmp, f"pricing-{job}.yaml")
                with open(pricing_path, "w") as f:
                    yaml.safe_dump(pricing, f, sort_keys=False)
                emit(job, pricing_path)

# Reference points: existing floor mechanisms + baseline
refs = [
    ("ref_baseline_flat_fee",          "parameters/phase-2-sweep/pricing/baseline_flat_fee.yaml"),
    ("ref_unreserved_x4",              "parameters/phase-2-sweep/pricing/two_lane_both_dynamic_unreserved_x4.yaml"),
    ("ref_unreserved_x16",             "parameters/phase-2-sweep/pricing/two_lane_both_dynamic_unreserved_x16.yaml"),
    ("ref_partitioned_x4",             "parameters/phase-2-sweep/pricing/two_lane_both_dynamic_partitioned_x4.yaml"),
    ("ref_partitioned_x16",            "parameters/phase-2-sweep/pricing/two_lane_both_dynamic_partitioned_x16.yaml"),
]
for job, pricing in refs:
    emit(job, pricing)
PYEOF

JOB_COUNT=$(wc -l < "$TMP/mini-suites.txt")
echo "Controller-tuning sweep: ${JOB_COUNT} jobs at seed=1, parallelism=${PARALLELISM}" >&2
echo "Topology: 100-node default with every node = actor source" >&2
echo "Demand: paper_like_realistic, scaled by 1/${SOURCE_COUNT} per node" >&2
echo "Output: ${OUTPUT_ROOT}" >&2

if [[ "$DRY_RUN" == "1" ]]; then
  cat "$TMP/mini-suites.txt"
  exit 0
fi

mkdir -p "$OUTPUT_ROOT"
cp "$TOPOLOGY" "$OUTPUT_ROOT/topology-default-actor-source.yaml"
cp "$TMP/demand-realistic-scaled.yaml" "$OUTPUT_ROOT/"

echo "Building experiment-suite..." >&2
cargo build --release --bin experiment-suite

xargs -P "$PARALLELISM" -I {} ./target/release/experiment-suite run {} < "$TMP/mini-suites.txt"

echo "Done. Run summaries:" >&2
find "$OUTPUT_ROOT" -name run_summary.json | wc -l
