#!/usr/bin/env bash
# Cross-regime validation: test the candidate "robust default" config
# (floor=8, D=4, w=8) plus a bracket of alternatives against all three
# demand profiles (moderate, realistic, congested).
#
# Goal: identify the (floor, D) triple that's welfare-positive across
# all three demand regimes, or — if none is — establish how the
# optimal config shifts with demand intensity.
#
# Sweep:
#   floor   ∈ {4, 8, 16}
#   D       ∈ {4, 8}
#   target  = 1/2 (fixed)
#   window  = 8  (fixed; realistic-winning value)
#   variant = un-reserved both-dynamic
# = 6 sweep configurations.
#
# References per demand (5 each):
#   baseline_flat_fee, un-reserved x4/x16, partitioned x4/x16
#
# Demands (3):
#   paper_like_moderate, paper_like_realistic, paper_like_congested
#
# = (6 + 5) × 3 = 33 jobs at seed=1.
#
# Holds same fast-screen 100-node topology + per-node demand scaling
# as the prior smoke scripts.
#
# Usage:
#   scripts/run-smoke-cross-regime.sh [N] [--dry-run]

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

RUN_ID="${CROSS_REGIME_RUN_ID:-$(date -u +%Y%m%d-%H%M%S)}"
OUTPUT_ROOT="output/phase-2/smoke/cross-regime-${RUN_ID}"
TMP=$(mktemp -d -t phase2-cross-regime-XXXXXX)
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

# Scaled demand profiles per source
DEMANDS=(moderate realistic congested)
python3 - "$TMP" "$SOURCE_COUNT" "${DEMANDS[@]}" <<'PYEOF'
import os, sys, yaml, copy
tmp, sc = sys.argv[1], int(sys.argv[2])
for demand_name in sys.argv[3:]:
    with open(f"parameters/phase-2-sweep/demand/paper_like_{demand_name}.yaml") as f:
        d = yaml.safe_load(f)
    for c in d["actors"]["components"]:
        r = c["arrival-rate-per-slot"]
        if isinstance(r, (int, float)):
            c["arrival-rate-per-slot"] = r / sc
        else:
            for p in r["phases"]:
                p["rate"] = p["rate"] / sc
    with open(os.path.join(tmp, f"demand-{demand_name}-scaled.yaml"), "w") as f:
        yaml.safe_dump(d, f, sort_keys=False)
PYEOF

# Generate jobs
python3 - "$TMP" "$OUTPUT_ROOT" "$TOPOLOGY" <<'PYEOF' > "$TMP/mini-suites.txt"
import os, sys, yaml
tmp, output_root, topology = sys.argv[1], sys.argv[2], sys.argv[3]

def emit(job_name, pricing_path):
    mini = {
        "suite-name": f"cross-regime-{job_name}",
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

floors = [4, 8, 16]
Ds = [4, 8]
TNUM, TDEN, TLABEL = 1, 2, "50"
WINDOW = 8

for demand_name in ("moderate", "realistic", "congested"):
    demand_path = os.path.join(tmp, f"demand-{demand_name}-scaled.yaml")

    # Sweep configs
    for floor in floors:
        for D in Ds:
            pricing = {
                "pricing": {
                    "kind": "two-lane",
                    "variant": "unreserved-both-dynamic",
                    "priority": {
                        "initial-quote-per-byte": 176,
                        "target-num": TNUM, "target-den": TDEN,
                        "max-change-denominator": D,
                        "window-length": WINDOW,
                    },
                    "standard": {
                        "initial-quote-per-byte": 44,
                        "target-num": TNUM, "target-den": TDEN,
                        "max-change-denominator": D,
                        "window-length": WINDOW,
                    },
                    "multiplier-floor-num": floor,
                    "multiplier-floor-den": 1,
                    "lane-selection-order": "priority-first",
                }
            }
            job = f"{demand_name}--floor{floor}_d{D}_t{TLABEL}_w{WINDOW}"
            pricing_path = os.path.join(tmp, f"pricing-{job}.yaml")
            with open(pricing_path, "w") as f:
                yaml.safe_dump(pricing, f, sort_keys=False)
            emit(job, pricing_path)

    # References (5 per demand)
    for ref_name, ref_pricing in [
        ("ref_baseline_flat_fee",   "parameters/phase-2-sweep/pricing/baseline_flat_fee.yaml"),
        ("ref_unreserved_x4",       "parameters/phase-2-sweep/pricing/two_lane_both_dynamic_unreserved_x4.yaml"),
        ("ref_unreserved_x16",      "parameters/phase-2-sweep/pricing/two_lane_both_dynamic_unreserved_x16.yaml"),
        ("ref_partitioned_x4",      "parameters/phase-2-sweep/pricing/two_lane_both_dynamic_partitioned_x4.yaml"),
        ("ref_partitioned_x16",     "parameters/phase-2-sweep/pricing/two_lane_both_dynamic_partitioned_x16.yaml"),
    ]:
        emit(f"{demand_name}--{ref_name}", ref_pricing)
PYEOF

JOB_COUNT=$(wc -l < "$TMP/mini-suites.txt")
echo "Cross-regime sweep: ${JOB_COUNT} jobs at seed=1, parallelism=${PARALLELISM}" >&2
echo "Demands: moderate, realistic, congested" >&2
echo "Sweep:   floor ∈ {4, 8, 16} × D ∈ {4, 8}, fixed target=1/2 window=8" >&2
echo "Output:  ${OUTPUT_ROOT}" >&2

if [[ "$DRY_RUN" == "1" ]]; then
  cat "$TMP/mini-suites.txt"
  exit 0
fi

mkdir -p "$OUTPUT_ROOT"
cp "$TOPOLOGY" "$OUTPUT_ROOT/topology-default-actor-source.yaml"
for d in moderate realistic congested; do
  cp "$TMP/demand-${d}-scaled.yaml" "$OUTPUT_ROOT/"
done

echo "Building experiment-suite..." >&2
cargo build --release --bin experiment-suite

xargs -P "$PARALLELISM" -I {} ./target/release/experiment-suite run {} < "$TMP/mini-suites.txt"

echo "Done. Run summaries:" >&2
find "$OUTPUT_ROOT" -name run_summary.json | wc -l
