#!/usr/bin/env python3
"""
CIP-0164 Table 7-aligned phase-2 topology generator (one-shot).

Output: parameters/phase-2-sweep/topology-cip-realistic.yaml

Properties (CIP-0164 Table 7 + Cardano mainnet realism):
  - 600 stake pools (matches mean committee n = 600).
  - Pareto(alpha=1.4) stake distribution, total ~= 3e10 (mainnet ~22B
    ADA + headroom).
  - 4 regions: NA-East 30%, EU 30%, NA-West 20%, Asia 20%.
  - 25 ms +/- 5 ms intra-region; 80-150 ms inter-region by region pair.
  - 20 peers/node, 70% intra-region 30% inter-region.
  - cpu-core-count: 4 everywhere; one pool (largest stake) carries
    tx-generation-weight: 1.
  - Bandwidth = 1,024,000 B/s (matches existing topology.default.yaml).

Run from sim-rs/:
  python3 scripts/generate-cip-topology.py \\
      > parameters/phase-2-sweep/topology-cip-realistic.yaml

The generator is deterministic (seed pinned). **The committed YAML
is canonical**; the generator is committed alongside as the source of
truth for the topology shape but the YAML is what the simulator
reads. Regenerating is only safe if you intend to also rerun the
full multi-node compute sweep and regenerate the suite goldens — a
new Python or CPython point release could in principle perturb
`random.paretovariate` output and silently desync the goldens.
Treat the YAML as a checked-in artifact, not a build product.
"""

import random
import sys
import yaml

SEED = 0xC1F0164      # pinned; treat as opaque
N_POOLS = 600
PARETO_ALPHA = 1.4
TOTAL_STAKE = 30_000_000_000  # 3e10 lovelace-equivalent
PEERS_PER_NODE = 20
INTRA_REGION_PEER_FRACTION = 0.70
BANDWIDTH_BPS = 1_024_000

# Region shares sum to 1.0. Centroids are (lon, lat) — only used for
# the `location:` field; not consumed by the simulator for latency
# computation. Latency is set explicitly per peer edge below.
REGIONS = [
    ("na-east", 0.30, (-75.0, 40.0)),
    ("eu",      0.30, ( 10.0, 50.0)),
    ("na-west", 0.20, (-120.0, 37.0)),
    ("asia",    0.20, (140.0, 35.0)),
]

INTRA_REGION_MS_MEAN = 25.0
INTRA_REGION_MS_STD = 5.0
INTER_REGION_MS = {
    ("na-east", "eu"):      90.0,
    ("na-east", "na-west"): 70.0,
    ("na-east", "asia"):    150.0,
    ("eu",      "na-west"): 130.0,
    ("eu",      "asia"):    150.0,
    ("na-west", "asia"):    110.0,
}


def inter_region_latency(a: str, b: str, rng: random.Random) -> float:
    if a == b:
        return max(1.0, rng.gauss(INTRA_REGION_MS_MEAN, INTRA_REGION_MS_STD))
    key = tuple(sorted([a, b]))
    base = INTER_REGION_MS.get(key)
    if base is None:
        base = INTER_REGION_MS[(key[1], key[0])]
    return max(1.0, rng.gauss(base, 15.0))


def main() -> None:
    rng = random.Random(SEED)

    # 1. Stake distribution: Pareto(alpha) draws, scaled to TOTAL_STAKE.
    raw = [rng.paretovariate(PARETO_ALPHA) for _ in range(N_POOLS)]
    raw_sum = sum(raw)
    stakes = sorted(
        (int(round(r * TOTAL_STAKE / raw_sum)) for r in raw),
        reverse=True,
    )
    # Pin total exactly: dump residual onto the smallest pool.
    delta = TOTAL_STAKE - sum(stakes)
    stakes[-1] += delta
    assert sum(stakes) == TOTAL_STAKE
    # VRF stake quantization: at rb-prob=0.05 the lottery computes
    # target_vrf_stake = stake * 0.05; truncates to u64. Smallest pool
    # must satisfy stake * 0.05 >= 100 to avoid target-zero.
    assert stakes[-1] * 0.05 >= 100, (
        f"smallest pool stake {stakes[-1]} truncates to target_vrf_stake < 100"
    )

    # 2. Region assignment: round shares to integer pool counts; pad
    # any rounding shortfall onto na-east.
    target = {r[0]: int(round(r[1] * N_POOLS)) for r in REGIONS}
    while sum(target.values()) < N_POOLS:
        target["na-east"] += 1
    while sum(target.values()) > N_POOLS:
        target["na-east"] -= 1
    region_assignment = []
    for name, count in target.items():
        region_assignment.extend([name] * count)
    rng.shuffle(region_assignment)
    assert len(region_assignment) == N_POOLS

    # 3. Build per-node records (stake desc; pool-000 = largest).
    region_centroids = {r[0]: r[2] for r in REGIONS}
    records = []
    for i in range(N_POOLS):
        region = region_assignment[i]
        lon, lat = region_centroids[region]
        records.append({
            "name": f"pool-{i:03d}",
            "stake": stakes[i],
            "region": region,
            "location": [
                round(lon + rng.gauss(0, 5.0), 4),
                round(lat + rng.gauss(0, 5.0), 4),
            ],
        })

    # 4. Peer graph: 70/30 intra/inter-region weighted, 20 peers/node.
    intra_count = int(round(PEERS_PER_NODE * INTRA_REGION_PEER_FRACTION))
    inter_count = PEERS_PER_NODE - intra_count
    by_region: dict[str, list[str]] = {r[0]: [] for r in REGIONS}
    for n in records:
        by_region[n["region"]].append(n["name"])
    region_lookup = {n["name"]: n["region"] for n in records}

    nodes_out: dict[str, dict] = {}
    for n in records:
        same = [p for p in by_region[n["region"]] if p != n["name"]]
        other = [
            p
            for p in (name for names in by_region.values() for name in names)
            if region_lookup[p] != n["region"]
        ]
        chosen_intra = rng.sample(same, min(intra_count, len(same)))
        chosen_inter = rng.sample(other, min(inter_count, len(other)))
        producers: dict[str, dict] = {}
        for peer in chosen_intra + chosen_inter:
            producers[peer] = {
                "bandwidth-bytes-per-second": BANDWIDTH_BPS,
                "latency-ms": round(
                    inter_region_latency(n["region"], region_lookup[peer], rng),
                    3,
                ),
            }
        nodes_out[n["name"]] = {
            "stake": n["stake"],
            "cpu-core-count": 4,
            "location": n["location"],
            "producers": producers,
        }

    # 5. Single tx-generation source: pool-000 (largest stake).
    nodes_out["pool-000"]["tx-generation-weight"] = 1

    header = (
        "# CIP-0164 Table 7-aligned phase-2 topology (600 pools).\n"
        "#\n"
        "# Generated by sim-rs/scripts/generate-cip-topology.py (seed pinned).\n"
        "# Total stake 3e10 (mainnet ~22B ADA + headroom). Pareto(alpha=1.4)\n"
        "# stake distribution. 4 regions (NA-East 30%, EU 30%, NA-West 20%,\n"
        "# Asia 20%). 20 peers/node, 70% intra-region. cpu-core-count: 4.\n"
        "# Single tx-generation source = pool-000 (largest stake).\n"
        "#\n"
        "# Calibration (set in protocol-base.yaml):\n"
        "#   vote-generation-probability: 600  (sum_i share_i * p = p)\n"
        "#   vote-threshold:              450  (75 % of n = 600)\n"
        "#\n"
        "# Smallest pool stake * rb-generation-probability (0.05) >= 100,\n"
        "# so no target_vrf_stake -> 0 truncation in the RB lottery.\n"
        "#\n"
        "# DO NOT HAND-EDIT. This YAML is the canonical topology;\n"
        "# the generator is reference. Regenerating is only safe\n"
        "# alongside a full multi-node sweep + goldens re-tag.\n"
    )
    sys.stdout.write(header)
    yaml.safe_dump(
        {"nodes": nodes_out},
        sys.stdout,
        sort_keys=True,
        default_flow_style=False,
    )


if __name__ == "__main__":
    main()
