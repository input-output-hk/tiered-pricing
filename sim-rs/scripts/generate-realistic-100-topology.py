#!/usr/bin/env python3
"""
Phase-2 100-node mainnet-faithful topology generator (one-shot).

Output: parameters/phase-2-sweep/topology-realistic-100.yaml

Method (spike 006, Option 1 — mass-stratified downsample):
  1. Query the Cardano mainnet on-chain pool_list view
     (active_stake>0 order=active_stake.desc) via the public
     on-chain API at https://api.koios.rest/... — two pages of
     1,000 rows, stable secondary sort by pool_id_bech32.asc,
     deduplicate.
  2. Filter to active_stake >= 1_000_000_000 lovelace (>= 1k ADA) —
     this is mainnet's "active body" of 1,510 pools (per spike 006,
     epoch 582 snapshot).
  3. Sort descending by active_stake. Build cumulative-mass array.
     For i in [0, 100), pick the rank whose cumulative stake crosses
     (i + 0.5) / 100 * total_mass via bisect.
  4. Sort the 100 sampled stakes descending. Rescale linearly to
     total = 3 * 10^10 lovelace (matches topology-cip-realistic.yaml).
     Pin the residual onto the smallest pool to make sum exact.
  5. Load topology.default.yaml (100 nodes node-0..node-99). Replace
     each node's `stake:` field. Assign stakes in descending order:
     node-0 receives the largest, node-99 the smallest. Mirrors
     topology-cip-realistic.yaml's pool-000-largest convention.
  6. Add tx-generation-weight: 1 to node-0 (the largest-stake node).
  7. Confirm min(stake) * 0.05 >= 100 (lottery-quantization check).

Run from sim-rs/:
  python3 scripts/generate-realistic-100-topology.py \\
      > parameters/phase-2-sweep/topology-realistic-100.yaml

The generator is deterministic given the same on-chain snapshot. The
bisect-on-cumsum sampling rule is closed-form (no RNG). Re-running
at a later epoch yields a *different* topology because the mainnet
on-chain state has drifted; date-stamp the YAML header accordingly.

**The committed YAML is canonical**; this generator is committed
alongside as the source of truth for the topology shape but the YAML
is what the simulator reads. Treat the YAML as a checked-in artifact,
not a build product.
"""

import json
import sys
import urllib.request
from bisect import bisect_left
from datetime import date

import yaml

KOIOS_POOL_LIST = (
    "https://api.koios.rest/api/v1/pool_list"
    "?active_stake=gt.0&order=active_stake.desc,pool_id_bech32.asc"
    "&limit=1000&offset={offset}"
)
KOIOS_EPOCH_INFO = (
    "https://api.koios.rest/api/v1/epoch_info"
    "?_include_next_epoch=false&limit=1"
)
BODY_FLOOR_LOVELACE = 1_000_000_000  # 1k ADA
N_NODES = 100
TOTAL_STAKE = 30_000_000_000  # 3e10 lovelace, matches cip-realistic
RB_GENERATION_PROBABILITY = 0.05
LOTTERY_FLOOR = 100  # stake * rb-prob must be >= 100 to avoid u64 trunc to 0


def fetch_json(url: str) -> list:
    req = urllib.request.Request(url, headers={"Accept": "application/json"})
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.loads(resp.read())


def pull_koios_snapshot() -> tuple[list[int], int, int]:
    """Returns (body_stakes_desc, epoch_no, total_active_stake_lovelace)."""
    epoch_info = fetch_json(KOIOS_EPOCH_INFO)
    epoch_no = int(epoch_info[0]["epoch_no"])
    total_active = int(epoch_info[0]["active_stake"])

    seen: set[str] = set()
    rows: list[dict] = []
    for offset in (0, 1000):
        page = fetch_json(KOIOS_POOL_LIST.format(offset=offset))
        for r in page:
            if r["pool_id_bech32"] in seen:
                continue
            seen.add(r["pool_id_bech32"])
            rows.append(r)

    body = [int(r["active_stake"]) for r in rows if int(r["active_stake"]) >= BODY_FLOOR_LOVELACE]
    body.sort(reverse=True)
    return body, epoch_no, total_active


def mass_stratified_downsample(stakes_desc: list[int], n_buckets: int) -> list[int]:
    """Pick n_buckets ranks whose cumulative stake crosses
    (i + 0.5) / n_buckets * total_mass, for i in [0, n_buckets).
    Returns sampled stakes sorted descending."""
    total = sum(stakes_desc)
    # Build cumulative-mass array over the descending-sorted body.
    cum = []
    running = 0
    for s in stakes_desc:
        running += s
        cum.append(running)
    sampled = []
    for i in range(n_buckets):
        target = (i + 0.5) / n_buckets * total
        idx = bisect_left(cum, target)
        if idx >= len(stakes_desc):
            idx = len(stakes_desc) - 1
        sampled.append(stakes_desc[idx])
    sampled.sort(reverse=True)
    return sampled


def rescale_to_total(stakes_desc: list[int], target_total: int) -> list[int]:
    raw_sum = sum(stakes_desc)
    rescaled = [int(round(s * target_total / raw_sum)) for s in stakes_desc]
    rescaled.sort(reverse=True)
    delta = target_total - sum(rescaled)
    rescaled[-1] += delta  # pin residual onto smallest
    assert sum(rescaled) == target_total
    return rescaled


def main() -> None:
    body, epoch_no, total_active = pull_koios_snapshot()

    sampled = mass_stratified_downsample(body, N_NODES)
    stakes = rescale_to_total(sampled, TOTAL_STAKE)
    assert len(stakes) == N_NODES
    assert stakes == sorted(stakes, reverse=True)
    assert stakes[-1] * RB_GENERATION_PROBABILITY >= LOTTERY_FLOOR, (
        f"smallest stake {stakes[-1]} truncates target_vrf_stake below {LOTTERY_FLOOR}"
    )

    # Load the canonical topology.default.yaml structure.
    with open("parameters/topology.default.yaml", "r") as fh:
        topo = yaml.safe_load(fh)

    node_names = sorted(topo["nodes"].keys(), key=lambda n: int(n.split("-")[1]))
    assert len(node_names) == N_NODES
    # node-0 receives the largest stake (stakes[0]); node-99 the smallest.
    for i, name in enumerate(node_names):
        topo["nodes"][name]["stake"] = stakes[i]

    # Tx-generation source: node-0 (the largest stake), mirroring
    # topology-cip-realistic.yaml's "largest stake is the source"
    # convention. Only added if topology.default.yaml doesn't already
    # carry one or more tx-generation-weight fields.
    already_has_txgen = any(
        "tx-generation-weight" in n for n in topo["nodes"].values()
    )
    if not already_has_txgen:
        topo["nodes"]["node-0"]["tx-generation-weight"] = 1

    retrieval = date.today().isoformat()
    top1_share = stakes[0] * 100.0 / TOTAL_STAKE
    cumulative = 0
    nak = 0
    for s in stakes:
        cumulative += s
        nak += 1
        if cumulative >= TOTAL_STAKE / 2:
            break

    header = (
        "# Phase-2 100-node mainnet-faithful topology (Option 1 from spike 006).\n"
        "#\n"
        f"# Generated by sim-rs/scripts/generate-realistic-100-topology.py from\n"
        f"# Cardano mainnet on-chain state, retrieved {retrieval}.\n"
        f"# Epoch:              {epoch_no}\n"
        f"# Total active stake: {total_active} lovelace (~{total_active/1e15:.2f} B ADA)\n"
        f"# Body filter:        active_stake >= 1k ADA -> {len(body)} pools\n"
        "#\n"
        "# Curve method (spike 006 Option 1): mass-stratified downsample of\n"
        "# the 1,510-pool mainnet body. For i in [0, 100), pick the rank whose\n"
        "# cumulative stake crosses (i + 0.5) / 100 * total_mass. Rescale\n"
        "# linearly to total = 3e10 lovelace; pin residual on smallest pool.\n"
        "# Defensibility statement (spike 006):\n"
        "#   \"Stakes are a mass-stratified downsample of the Cardano mainnet\n"
        "#    pools with >= 1k ADA active stake as of mainnet on-chain\n"
        f"#    snapshot epoch {epoch_no} (retrieved {retrieval}), rescaled linearly to total = 3e10\n"
        "#    lovelace to match the CIP-0164 reference topology's headroom.\"\n"
        "#\n"
        f"# Top-1 share:  {top1_share:.2f}%\n"
        f"# Nakamoto:     {nak} (50% of distribution total)\n"
        f"# Min stake:    {stakes[-1]} lovelace\n"
        f"# min * rb-prob (0.05) = {stakes[-1] * RB_GENERATION_PROBABILITY:.0f} (>= 100, lottery check passes)\n"
        "#\n"
        "# Locations/latencies/producers/bandwidth values are copied verbatim\n"
        "# from parameters/topology.default.yaml; only stake values change\n"
        "# (and one tx-generation-weight: 1 on node-0).\n"
        "#\n"
        "# DO NOT HAND-EDIT. This YAML is the canonical topology; this\n"
        "# generator is the reference recipe. Regenerating at a later mainnet\n"
        "# epoch is only safe alongside a full M5 goldens re-pinning.\n"
        "# See .planning/spikes/006-curve-design/README.md for the curve\n"
        "# rationale and CLAUDE.md Calibration choices for the operational\n"
        "# implications.\n"
    )
    sys.stdout.write(header)
    yaml.safe_dump(
        topo,
        sys.stdout,
        sort_keys=True,
        default_flow_style=False,
    )


if __name__ == "__main__":
    main()
