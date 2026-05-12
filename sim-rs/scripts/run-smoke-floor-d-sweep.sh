#!/usr/bin/env bash
# Floor × D × window middle-ground sweep.
#
# Goal: find a configuration that gets close to the floor=1, D=4, w=8
# winner's welfare (+525M) without its 1000×-range price volatility.
# Hypothesis: a modest non-1 floor (e.g. 4 or 8) bounds the priority/
# standard ratio while fast controllers still adapt within that band.
#
# Sweep:
#   floor   ∈ {1, 2, 4, 8}
#   D       ∈ {4, 8}
#   target  = 1/2 (fixed at the canonical EIP-1559 target)
#   window  ∈ {8, 16, 32}
#   variant = un-reserved both-dynamic (best family from prior runs)
# = 4 × 2 × 3 = 24 sweep configurations, plus 5 reference points
# (baseline + x4/x16 floor-based for un-reserved and partitioned).
#
# Holds demand at paper_like_realistic and the same fast-screen 100-node
# topology + per-node demand scaling as the other smoke batches.
#
# Usage:
#   scripts/run-smoke-floor-d-sweep.sh [N] [--dry-run]
# N defaults to 16.
#
# Output: output/phase-2/smoke/floor-d-sweep-<run-id>/<job>/<job>/1/...

set -euo pipefail
SCRIPT_PATH="$(realpath "$0")"
cd "$(dirname "$SCRIPT_PATH")/.."

PARALLELISM=16
DRY_RUN=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=1; shift ;;
    --help|-h) sed -n '1,30p' "$SCRIPT_PATH"; exit 0 ;;
    ''|*[!0-9]*) echo "Unknown argument: $1" >&2; exit 2 ;;
    *) PARALLELISM="$1"; shift ;;
  esac
done

RUN_ID="${FLOOR_D_SWEEP_RUN_ID:-$(date -u +%Y%m%d-%H%M%S)}"
OUTPUT_ROOT="output/phase-2/smoke/floor-d-sweep-${RUN_ID}"
TMP=$(mktemp -d -t phase2-floor-d-sweep-XXXXXX)
trap "rm -rf $TMP" EXIT

# Fast-screen topology (every node = actor source)
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

# Per-source-scaled paper_like_realistic demand
python3 - "$TMP" "$SOURCE_COUNT" <<'PYEOF'
import os, sys, yaml
tmp, sc = sys.argv[1], int(sys.argv[2])
with open("parameters/phase-2-sweep/demand/paper_like_realistic.yaml") as f:
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

# Generate the grid + references
python3 - "$TMP" "$OUTPUT_ROOT" "$TOPOLOGY" <<'PYEOF' > "$TMP/mini-suites.txt"
import os, sys, yaml
tmp, output_root, topology = sys.argv[1], sys.argv[2], sys.argv[3]
demand_path = os.path.join(tmp, "demand-realistic-scaled.yaml")

def emit(job_name, pricing_path):
    mini = {
        "suite-name": f"floor-d-sweep-{job_name}",
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

# Sweep grid
floors = [1, 2, 4, 8]
Ds = [4, 8]
windows = [8, 16, 32]
TNUM, TDEN, TLABEL = 1, 2, "50"  # target = 1/2 fixed

for floor in floors:
    for D in Ds:
        for w in windows:
            pricing = {
                "pricing": {
                    "kind": "two-lane",
                    "variant": "unreserved-both-dynamic",
                    "priority": {
                        "initial-quote-per-byte": 176,
                        "target-num": TNUM,
                        "target-den": TDEN,
                        "max-change-denominator": D,
                        "window-length": w,
                    },
                    "standard": {
                        "initial-quote-per-byte": 44,
                        "target-num": TNUM,
                        "target-den": TDEN,
                        "max-change-denominator": D,
                        "window-length": w,
                    },
                    "multiplier-floor-num": floor,
                    "multiplier-floor-den": 1,
                    "lane-selection-order": "priority-first",
                }
            }
            job = f"floor{floor}_unreserved_d{D}_t{TLABEL}_w{w}"
            pricing_path = os.path.join(tmp, f"pricing-{job}.yaml")
            with open(pricing_path, "w") as f:
                yaml.safe_dump(pricing, f, sort_keys=False)
            emit(job, pricing_path)

# Reference points (same as controller-tuning sweep for direct comparability)
for job, pricing in [
    ("ref_baseline_flat_fee",   "parameters/phase-2-sweep/pricing/baseline_flat_fee.yaml"),
    ("ref_unreserved_x4",       "parameters/phase-2-sweep/pricing/two_lane_both_dynamic_unreserved_x4.yaml"),
    ("ref_unreserved_x16",      "parameters/phase-2-sweep/pricing/two_lane_both_dynamic_unreserved_x16.yaml"),
    ("ref_partitioned_x4",      "parameters/phase-2-sweep/pricing/two_lane_both_dynamic_partitioned_x4.yaml"),
    ("ref_partitioned_x16",     "parameters/phase-2-sweep/pricing/two_lane_both_dynamic_partitioned_x16.yaml"),
]:
    emit(job, pricing)
PYEOF

JOB_COUNT=$(wc -l < "$TMP/mini-suites.txt")
echo "Floor × D sweep: ${JOB_COUNT} jobs at seed=1, parallelism=${PARALLELISM}" >&2
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
