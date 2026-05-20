#!/usr/bin/env bash
# Run the full phase-2 sweep on a 100-node multi-producer topology.
#
# Usage:
#   scripts/run-m6-full-sweep-100.sh [THREADS] [SUITE ...]
#   M6_RUN_ID=<id> scripts/run-m6-full-sweep-100.sh [THREADS] [SUITE ...]
#
# THREADS defaults to 2. SUITE defaults to every YAML file under
# parameters/phase-2-sweep/suites.
#
# The wrapper generates temporary suite copies that:
# - use parameters/topology.default.yaml's 100 nodes,
# - add tx-generation-weight: 1 to every node so all nodes run actors,
# - scale actor arrival rates by 1/100 so global demand matches the
#   parent demand profile,
# - calibrate vote-generation-probability=100 and vote-threshold=75,
# - preserve each parent suite's jobs, seeds, slots, and pricing files.
#
# Generated inputs are also copied to
# output/phase-2/m6-full-sweep-100-inputs-<run-id>/ for audit/debugging.

set -euo pipefail

cd "$(dirname "$0")/.."

if [[ "${1:-}" =~ ^[0-9]+$ ]]; then
  THREADS="$1"
  shift
else
  THREADS=2
fi

RUN_ID="${M6_RUN_ID:-$(date -u +%Y%m%d-%H%M%S)-100n-full}"
TMP="$(mktemp -d -t m6-full-sweep-100-XXXXXX)"
trap 'rm -rf "$TMP"' EXIT

if [[ $# -gt 0 ]]; then
  SUITES=("$@")
else
  mapfile -t SUITES < <(find parameters/phase-2-sweep/suites -maxdepth 1 -name '*.yaml' | sort)
fi

INPUT_DIR="output/phase-2/m6-full-sweep-100-inputs-$RUN_ID"
mkdir -p "$INPUT_DIR"

python3 - "$TMP" "$INPUT_DIR" "${SUITES[@]}" <<'PYEOF' > "$TMP/generated-suites.txt"
import copy
import os
import sys
import yaml

tmp = sys.argv[1]
input_dir = sys.argv[2]
suites = sys.argv[3:]

topology_src = "parameters/topology.default.yaml"
topology_dst = os.path.join(tmp, "topology-default-100-actor-sources.yaml")

with open(topology_src) as f:
    topology = yaml.safe_load(f)

nodes = topology.get("nodes") or {}
if len(nodes) != 100:
    raise SystemExit(f"expected 100 nodes in {topology_src}, found {len(nodes)}")

for node in nodes.values():
    node["tx-generation-weight"] = 1

with open(topology_dst, "w") as f:
    yaml.safe_dump(topology, f, sort_keys=False)

source_count = len(nodes)
vote_threshold = int(source_count * 0.75)

protocol_paths = [
    "parameters/phase-2-sweep/protocol-base.yaml",
    "parameters/phase-2-sweep/protocol-rb-reduced-half.yaml",
    "parameters/phase-2-sweep/protocol-rb-reduced-third.yaml",
    "parameters/phase-2-sweep/protocol-rb-reduced-quarter.yaml",
]
protocol_map = {}
for src in protocol_paths:
    with open(src) as f:
        protocol = yaml.safe_load(f)
    protocol["vote-generation-probability"] = float(source_count)
    protocol["vote-threshold"] = vote_threshold
    dst = os.path.join(tmp, f"100node-{os.path.basename(src)}")
    with open(dst, "w") as f:
        yaml.safe_dump(protocol, f, sort_keys=False)
    protocol_map[src] = dst

demand_map = {}

def scale_rate(rate, denom):
    if isinstance(rate, (int, float)):
        return rate / denom
    if isinstance(rate, dict) and "phases" in rate:
        scaled = copy.deepcopy(rate)
        for phase in scaled["phases"]:
            phase["rate"] = phase["rate"] / denom
        return scaled
    raise TypeError(f"unsupported arrival-rate-per-slot shape: {rate!r}")

def scaled_demand(path):
    if path in demand_map:
        return demand_map[path]
    with open(path) as f:
        demand = yaml.safe_load(f)
    for component in demand.get("actors", {}).get("components", []):
        component["arrival-rate-per-slot"] = scale_rate(
            component["arrival-rate-per-slot"],
            source_count,
        )
    dst = os.path.join(tmp, f"scaled-x{source_count}-{os.path.basename(path)}")
    with open(dst, "w") as f:
        yaml.safe_dump(demand, f, sort_keys=False)
    demand_map[path] = dst
    return dst

def remap_overrides(overrides):
    if not isinstance(overrides, dict):
        return overrides
    if "protocol" in overrides:
        overrides["protocol"] = protocol_map.get(overrides["protocol"], overrides["protocol"])
    if "demand" in overrides:
        overrides["demand"] = scaled_demand(overrides["demand"])
    if "topology" in overrides:
        overrides["topology"] = topology_dst
    return overrides

generated_suites = []
for src in suites:
    with open(src) as f:
        suite = yaml.safe_load(f)

    suite["default-topology"] = topology_dst
    suite["default-protocol"] = protocol_map.get(
        suite["default-protocol"],
        suite["default-protocol"],
    )
    suite["default-demand"] = scaled_demand(suite["default-demand"])

    for job in suite.get("jobs", []):
        if "overrides" in job:
            job["overrides"] = remap_overrides(job["overrides"])

    dst = os.path.join(tmp, os.path.basename(src))
    with open(dst, "w") as f:
        yaml.safe_dump(suite, f, sort_keys=False)
    generated_suites.append(dst)

for path in [topology_dst, *protocol_map.values(), *demand_map.values(), *generated_suites]:
    with open(path) as f:
        data = f.read()
    with open(os.path.join(input_dir, os.path.basename(path)), "w") as f:
        f.write(data)

for path in generated_suites:
    print(path)
PYEOF

SUITE_COUNT="$(wc -l < "$TMP/generated-suites.txt")"
echo "Generated ${SUITE_COUNT} 100-node suite(s); inputs copied to ${INPUT_DIR}" >&2
echo "Building experiment-suite..." >&2
cargo build --release --bin experiment-suite

echo "Running full 100-node sweep with parallelism=${THREADS}, run-id=${RUN_ID}" >&2
xargs -n1 -P "$THREADS" -I {} \
  ./target/release/experiment-suite run --run-id "$RUN_ID" {} \
  < "$TMP/generated-suites.txt"
