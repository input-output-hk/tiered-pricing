#!/usr/bin/env bash
# Multi-node noise variance experiment (M6 Phase C, Task C1).
#
# Re-runs three representative (mechanism, demand) tuples across 10
# seeds each on a 100-node multi-producer topology, then reports
# welfare mean / sd / coefficient-of-variation per tuple.
#
# Rationale: the pricing controller has no rollback on fork
# resolution, so multi-node slot battles inject noise into welfare
# numbers. Threshold: CV < 1 % per tuple ⇒ noise is bounded and
# cross-mechanism rankings can be trusted. CV ≥ 1 % ⇒ stop, the
# headline comparison needs a rollback fix or more seeds.
#
# Usage:
#   scripts/run-m6-variance.sh [THREADS]
#   M6_RUN_ID=<id> scripts/run-m6-variance.sh [THREADS]
#
# THREADS defaults to 2. The script generates a temp topology from
# parameters/topology.default.yaml, adds tx-generation-weight: 1 to all
# 100 stake-bearing nodes, scales per-source demand by 1/100 so global
# demand matches the parent profile, then generates 10 single-seed suite
# YAMLs and dispatches them with xargs -P THREADS. Re-running with the
# same M6_RUN_ID resumes Completed (job, seed) pairs.

set -euo pipefail

cd "$(dirname "$0")/.."

THREADS="${1:-2}"
RUN_ID="${M6_RUN_ID:-$(date -u +%Y%m%d-%H%M%S)}"
SLOTS=2000
SEEDS=10

SUITES_DIR="$(mktemp -d -t m6-variance-suites-XXXXXX)"
trap 'rm -rf "$SUITES_DIR"' EXIT

TOPOLOGY="$SUITES_DIR/topology-default-100-actor-sources.yaml"
PROTOCOL="$SUITES_DIR/protocol-base-100-votes.yaml"
SOURCE_COUNT=$(python3 - parameters/topology.default.yaml "$TOPOLOGY" <<'PYEOF'
import sys
import yaml

src, dst = sys.argv[1], sys.argv[2]
with open(src) as f:
    topology = yaml.safe_load(f)

nodes = topology.get("nodes") or {}
if len(nodes) != 100:
    raise SystemExit(f"expected 100 nodes in parameters/topology.default.yaml, found {len(nodes)}")

for node in nodes.values():
    node["tx-generation-weight"] = 1

with open(dst, "w") as f:
    yaml.safe_dump(topology, f, sort_keys=False)

print(len(nodes))
PYEOF
)

python3 - parameters/phase-2-sweep/protocol-base.yaml "$PROTOCOL" "$SOURCE_COUNT" <<'PYEOF'
import sys
import yaml

src, dst, source_count = sys.argv[1], sys.argv[2], int(sys.argv[3])
with open(src) as f:
    protocol = yaml.safe_load(f)

protocol["vote-generation-probability"] = float(source_count)
protocol["vote-threshold"] = int(source_count * 0.75)

with open(dst, "w") as f:
    yaml.safe_dump(protocol, f, sort_keys=False)
PYEOF

scale_demand() {
  local src="$1"
  local dst="$2"
  python3 - "$src" "$dst" "$SOURCE_COUNT" <<'PYEOF'
import copy
import sys
import yaml

src, dst, source_count = sys.argv[1], sys.argv[2], int(sys.argv[3])

def scale_rate(rate, denom):
    if isinstance(rate, (int, float)):
        return rate / denom
    if isinstance(rate, dict) and "phases" in rate:
        scaled = copy.deepcopy(rate)
        for phase in scaled["phases"]:
            phase["rate"] = phase["rate"] / denom
        return scaled
    raise TypeError(f"unsupported arrival-rate-per-slot shape: {rate!r}")

with open(src) as f:
    demand = yaml.safe_load(f)

for component in demand.get("actors", {}).get("components", []):
    component["arrival-rate-per-slot"] = scale_rate(
        component["arrival-rate-per-slot"],
        source_count,
    )

with open(dst, "w") as f:
    yaml.safe_dump(demand, f, sort_keys=False)
PYEOF
}

DEMAND_MODERATE="$SUITES_DIR/paper_like_moderate_scaled_x${SOURCE_COUNT}.yaml"
DEMAND_CONGESTED="$SUITES_DIR/paper_like_congested_scaled_x${SOURCE_COUNT}.yaml"
DEMAND_REALISTIC="$SUITES_DIR/paper_like_realistic_scaled_x${SOURCE_COUNT}.yaml"
scale_demand parameters/phase-2-sweep/demand/paper_like_moderate.yaml "$DEMAND_MODERATE"
scale_demand parameters/phase-2-sweep/demand/paper_like_congested.yaml "$DEMAND_CONGESTED"
scale_demand parameters/phase-2-sweep/demand/paper_like_realistic.yaml "$DEMAND_REALISTIC"

INPUT_DIR="output/phase-2/m6-variance-inputs-$RUN_ID"
mkdir -p "$INPUT_DIR"
cp "$TOPOLOGY" "$INPUT_DIR/topology-default-100-actor-sources.yaml"
cp "$PROTOCOL" "$INPUT_DIR/protocol-base-100-votes.yaml"
cp "$DEMAND_MODERATE" "$INPUT_DIR/"
cp "$DEMAND_CONGESTED" "$INPUT_DIR/"
cp "$DEMAND_REALISTIC" "$INPUT_DIR/"

# (job-name | pricing | demand) — one tuple per line.
TUPLES=(
  "baseline_moderate|baseline_flat_fee.yaml|$DEMAND_MODERATE"
  "priority_unreserved_x4_congested|two_lane_priority_only_unreserved_x4.yaml|$DEMAND_CONGESTED"
  "priority_static_x4_realistic|two_lane_priority_only_static_x4.yaml|$DEMAND_REALISTIC"
)

for seed in $(seq 1 "$SEEDS"); do
  suite="$SUITES_DIR/variance-seed-$seed.yaml"
  {
    echo "suite-name: m6-variance-seed-$seed"
    echo "output-dir: output/phase-2/m6-variance-seed-$seed"
    echo "seeds: [$seed]"
    echo "default-slots: $SLOTS"
    echo "default-topology: $TOPOLOGY"
    echo "default-protocol: $PROTOCOL"
    echo "default-demand: $DEMAND_MODERATE"
    echo "jobs:"
    for tup in "${TUPLES[@]}"; do
      IFS='|' read -r name pricing demand <<<"$tup"
      echo "  - name: $name"
      echo "    pricing: parameters/phase-2-sweep/pricing/$pricing"
      if [[ "$demand" != "$DEMAND_MODERATE" ]]; then
        echo "    overrides:"
        echo "      demand: $demand"
      fi
    done
  } >"$suite"
done

echo "Building experiment-suite..." >&2
cargo build --release --bin experiment-suite

echo "Generated 100-node actor-source topology and 100-vote protocol inputs in $INPUT_DIR" >&2
echo "Running 10 single-seed suites (3 tuples × 10 seeds = 30 runs) at -P $THREADS, run-id=$RUN_ID" >&2

find "$SUITES_DIR" -maxdepth 1 -name 'variance-seed-*.yaml' -print0 |
  xargs -0 -n1 -P "$THREADS" -I {} \
    ./target/release/experiment-suite run --run-id "$RUN_ID" {}

RUN_ID="$RUN_ID" python3 - <<'PYEOF'
import glob, json, os, statistics, sys

runid = os.environ["RUN_ID"]
tuples = [
    "baseline_moderate",
    "priority_unreserved_x4_congested",
    "priority_static_x4_realistic",
]

print()
print(f"=== M6 variance summary (run-id: {runid}) ===")
print()
hdr = ["tuple", "n", "welfare_mean", "welfare_sd", "cv_%", "battles_mean", "orphans_mean"]
print(f"{hdr[0]:<36} {hdr[1]:>2}  {hdr[2]:>14}  {hdr[3]:>14}  {hdr[4]:>6}  {hdr[5]:>12}  {hdr[6]:>12}")

any_fail = False
for t in tuples:
    paths = sorted(glob.glob(
        f"output/phase-2/m6-variance-seed-*-{runid}/{t}/*/run_summary.json"
    ))
    if not paths:
        print(f"{t:<36} -- no results found", file=sys.stderr)
        continue
    welfares, battles, orphans = [], [], []
    for p in paths:
        with open(p) as f:
            s = json.load(f)
        welfares.append(
            s.get("priority_retained_value_total", 0.0)
            + s.get("standard_retained_value_total", 0.0)
        )
        battles.append(s.get("slot_battles_count", 0))
        orphans.append(s.get("orphaned_pricing_samples", 0))
    mean = statistics.mean(welfares)
    sd = statistics.stdev(welfares) if len(welfares) > 1 else 0.0
    cv = (sd / mean * 100) if mean else 0.0
    bmean = statistics.mean(battles)
    omean = statistics.mean(orphans)
    flag = "" if cv < 1.0 else "  ← FAIL (CV ≥ 1%)"
    if cv >= 1.0:
        any_fail = True
    print(
        f"{t:<36} {len(welfares):>2}  "
        f"{mean:>14.4e}  {sd:>14.4e}  {cv:>5.3f}%  "
        f"{bmean:>12.1f}  {omean:>12.1f}{flag}"
    )

print()
if any_fail:
    print("RESULT: FAIL — at least one tuple has CV ≥ 1 %. Multi-node "
          "noise floor is too high for headline welfare claims; "
          "consider rollback work or more seeds.")
    sys.exit(1)
else:
    print("RESULT: PASS — all tuples have CV < 1 %. Multi-node noise is "
          "bounded; mechanism rankings on the full sweep can be trusted.")
PYEOF
