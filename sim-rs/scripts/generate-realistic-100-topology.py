#!/usr/bin/env python3
"""
Phase-2/3 mainnet-faithful topology generator.

Two modes, selected at the command line:

  python3 scripts/generate-realistic-100-topology.py
      → emit parameters/phase-2-sweep/topology-realistic-150.yaml
        (default — extends the committed 100-pool YAML to 150 nodes
        via mass-stratified resampling + Gaussian-jittered template clones)

  python3 scripts/generate-realistic-100-topology.py --regenerate-100
      → fetch a fresh Koios snapshot and re-emit topology-realistic-100.yaml
        to stdout. WARNING: hits a live API; produces a topology that may
        drift from the committed epoch-582 snapshot. Run only when
        explicitly re-pinning the M5 goldens.

The default path (150-node emission) is deterministic and reproducible
from in-tree data alone — it reads the committed topology-realistic-100.yaml
as the canonical witness of the epoch-582 mainnet distribution and never
calls Koios. Re-running with the same arguments produces a bit-identical
topology-realistic-150.yaml.

Design (Robustness / TEST-05 prerequisite, CONTEXT.md D-28..D-30):
  1. Read topology-realistic-100.yaml's 100 stakes as the "body"
     (mass-stratified downsample of the 1,510-pool mainnet body at
     epoch 582; carries the empirical distribution shape).
  2. Mass-stratified-downsample that 100-element body to 150 buckets
     via cumulative-sum bisect ( (i + 0.5) / 150 * total_mass ).
  3. Rescale linearly to total = 3e10 lovelace; pin residual onto
     the smallest pool. Lottery-quantization assert at the end.
  4. node-0..node-99: copy structure from the committed YAML, override
     `stake:` with the rescaled value for rank i.
  5. node-100..node-149: pick a random template node from node-0..node-99
     (uniform over the 100 base nodes, seeded by jitter_seed + i),
     deepcopy its location + producers + bandwidth, perturb each
     producer's latency-ms by a Gaussian factor with mean=1.0,
     SD=jitter_sd_pct/100. Drop tx-generation-weight on extras.
  6. YAML emission with header documenting jitter seed/SD, recipe,
     epoch-582 snapshot reference, lottery-quantization margin.

Per-extra determinism: each i in [100, 150) uses random.Random(
jitter_seed + i) so the jittered values are reproducible even if the
order of iteration changes.
"""

import argparse
import copy
import json
import random
import sys
import urllib.request
from bisect import bisect_left
from datetime import date
from pathlib import Path

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
TOTAL_STAKE = 30_000_000_000  # 3e10 lovelace, matches cip-realistic
RB_GENERATION_PROBABILITY = 0.05
LOTTERY_FLOOR = 100  # stake * rb-prob must be >= 100 to avoid u64 trunc to 0
SNAPSHOT_EPOCH = 582  # epoch-582 mainnet snapshot, retrieved 2026-05-14


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

    body = [
        int(r["active_stake"])
        for r in rows
        if int(r["active_stake"]) >= BODY_FLOOR_LOVELACE
    ]
    body.sort(reverse=True)
    return body, epoch_no, total_active


def mass_stratified_downsample(stakes_desc: list[int], n_buckets: int) -> list[int]:
    """Pick n_buckets ranks whose cumulative stake crosses
    (i + 0.5) / n_buckets * total_mass, for i in [0, n_buckets).
    Returns sampled stakes sorted descending."""
    total = sum(stakes_desc)
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


def jitter_clone(
    template: dict, stake: int, jitter_seed: int, jitter_sd_pct: float
) -> dict:
    """Deepcopy `template`, set its stake, perturb each producer's
    latency-ms by a Gaussian factor (mean=1.0, SD=jitter_sd_pct/100).
    Determinism: a separate random.Random(jitter_seed) instance per
    extra so iteration order doesn't matter."""
    rng = random.Random(jitter_seed)
    new_node = copy.deepcopy(template)
    new_node["stake"] = stake
    new_node.pop("tx-generation-weight", None)  # only node-0 generates txs
    if "producers" in new_node:
        for prod_attrs in new_node["producers"].values():
            if "latency-ms" in prod_attrs:
                factor = rng.gauss(1.0, jitter_sd_pct / 100.0)
                # Floor at 0.1 ms so a degenerate negative-factor draw can't yield
                # a non-physical latency. SD=7% makes this practically unreachable.
                prod_attrs["latency-ms"] = max(0.1, float(prod_attrs["latency-ms"]) * factor)
    return new_node


def emit_150_from_committed_100(
    base_path: Path,
    out_path: Path,
    jitter_seed: int = 582,
    jitter_sd_pct: float = 7.0,
) -> None:
    """Read base_path (topology-realistic-100.yaml), extend to 150 nodes
    via mass-stratified resampling + Gaussian-jittered template clones,
    write out_path."""
    with open(base_path) as fh:
        base = yaml.safe_load(fh)
    base_nodes = base["nodes"]
    base_names = sorted(base_nodes.keys(), key=lambda n: int(n.split("-")[1]))
    assert len(base_names) == 100, f"expected 100 base nodes, got {len(base_names)}"
    base_stakes_desc = sorted(
        (base_nodes[n]["stake"] for n in base_names), reverse=True
    )

    # Mass-stratified downsample the 100-element body to 150 buckets.
    sampled = mass_stratified_downsample(base_stakes_desc, 150)
    stakes = rescale_to_total(sampled, TOTAL_STAKE)
    assert stakes == sorted(stakes, reverse=True)

    # node-0..node-99: keep template structure, override stake by rank.
    new_nodes: dict = {}
    for i, name in enumerate(base_names):
        node = copy.deepcopy(base_nodes[name])
        node["stake"] = stakes[i]
        new_nodes[name] = node

    # node-100..node-149: jitter-clone from a uniformly-sampled base template.
    master_rng = random.Random(jitter_seed)
    for i in range(50):
        idx = 100 + i
        new_name = f"node-{idx}"
        template_name = master_rng.choice(base_names)
        template = base_nodes[template_name]
        new_nodes[new_name] = jitter_clone(
            template,
            stake=stakes[idx],
            jitter_seed=jitter_seed + idx,
            jitter_sd_pct=jitter_sd_pct,
        )

    assert len(new_nodes) == 150
    min_stake = min(stakes)
    lottery_margin = min_stake * RB_GENERATION_PROBABILITY
    assert lottery_margin >= LOTTERY_FLOOR, (
        f"lottery-quantization fail at N=150: min_stake={min_stake} "
        f"* rb_prob={RB_GENERATION_PROBABILITY} = {lottery_margin:.1f} < {LOTTERY_FLOOR}"
    )

    cumulative = 0
    nak = 0
    for s in stakes:
        cumulative += s
        nak += 1
        if cumulative >= TOTAL_STAKE / 2:
            break
    top1_share = stakes[0] * 100.0 / TOTAL_STAKE

    header = (
        "# Robustness 150-node mainnet-faithful topology (extension of the 100-node\n"
        "# Option 1 mass-stratified topology from spike 006).\n"
        "#\n"
        "# Generated by sim-rs/scripts/generate-realistic-100-topology.py from\n"
        "# parameters/phase-2-sweep/topology-realistic-100.yaml (the canonical\n"
        f"# witness of the Cardano mainnet on-chain snapshot at epoch {SNAPSHOT_EPOCH},\n"
        "# retrieved 2026-05-14).\n"
        "#\n"
        "# This generator runs without network access — it reads the committed\n"
        "# 100-node YAML as input. The mass-stratified downsample at N=150\n"
        "# treats the 100 base stakes as the body distribution. Reproducible\n"
        "# bit-for-bit on re-run given the same base file and CLI arguments.\n"
        "#\n"
        "# Reproduction recipe:\n"
        "#   python3 sim-rs/scripts/generate-realistic-100-topology.py\n"
        "#       --jitter-seed 582 --jitter-sd-pct 7.0\n"
        "#       --base parameters/phase-2-sweep/topology-realistic-100.yaml\n"
        "#       --out  parameters/phase-2-sweep/topology-realistic-150.yaml\n"
        "#\n"
        f"# Snapshot:           epoch-{SNAPSHOT_EPOCH} (mainnet, retrieved 2026-05-14)\n"
        f"# Total stake:        {TOTAL_STAKE} lovelace (matches topology-realistic-100.yaml)\n"
        f"# Per-pool average:   {TOTAL_STAKE // 150} lovelace (3e10/150; drops from 3e10/100 by design — CONTEXT.md D-30)\n"
        f"# Top-1 share:        {top1_share:.2f}%\n"
        f"# Nakamoto coefficient: {nak} (50% of distribution total)\n"
        f"# Min stake:          {min_stake} lovelace\n"
        f"# Lottery-quant check: min * rb-prob ({RB_GENERATION_PROBABILITY}) = {lottery_margin:.0f} (>= {LOTTERY_FLOOR}, passes)\n"
        "#\n"
        "# Extras methodology (node-100..node-149 are template-cloned + Gaussian-perturbed):\n"
        f"#   - jitter_seed:       582 (= epoch number)\n"
        f"#   - jitter_sd_pct:     7.0% (midpoint of CONTEXT.md D-29's ±5-10% range)\n"
        "#   - per-extra random.Random(jitter_seed + i) instance for determinism\n"
        "#   - latency-ms perturbed by Gaussian(mean=1.0, SD=0.07), floored at 0.1 ms\n"
        "#   - location, bandwidth-bytes-per-second, producer set: copied verbatim\n"
        "#     from a uniformly-sampled base template node (no jitter on these)\n"
        "#   - tx-generation-weight dropped on extras (only node-0 generates txs)\n"
        "#\n"
        "# DO NOT HAND-EDIT. This YAML is reproducible from the recipe above; the\n"
        "# generator is the source of truth. See CONTEXT.md D-28/D-29/D-30 for\n"
        "# the design rationale and CLAUDE.md Calibration choices for operational\n"
        "# implications. The 150-node topology is the TEST-05 input (pool-number\n"
        "# sensitivity test). Robustness suites are NOT goldens-pinned, so re-running\n"
        "# this generator does not invalidate M5 suite goldens.\n"
    )

    out = dict(base)
    out["nodes"] = new_nodes
    with open(out_path, "w") as fh:
        fh.write(header)
        yaml.safe_dump(out, fh, sort_keys=True, default_flow_style=False)


def regenerate_100_from_koios() -> None:
    """Original Phase-2 100-node generator. Fetches Koios live and writes
    to stdout. Preserved for completeness; re-running this AT A LATER
    EPOCH drifts away from the committed topology-realistic-100.yaml
    and requires a full M5 goldens re-pinning. Cardano Improvement
    Proposal (CIP) evidence work in the robustness suites uses the committed 100-node
    YAML as-is.
    """
    body, epoch_no, total_active = pull_koios_snapshot()
    sampled = mass_stratified_downsample(body, 100)
    stakes = rescale_to_total(sampled, TOTAL_STAKE)
    assert len(stakes) == 100
    assert stakes == sorted(stakes, reverse=True)
    assert stakes[-1] * RB_GENERATION_PROBABILITY >= LOTTERY_FLOOR, (
        f"smallest stake {stakes[-1]} truncates target_vrf_stake below {LOTTERY_FLOOR}"
    )

    with open("parameters/topology.default.yaml", "r") as fh:
        topo = yaml.safe_load(fh)

    node_names = sorted(topo["nodes"].keys(), key=lambda n: int(n.split("-")[1]))
    assert len(node_names) == 100
    for i, name in enumerate(node_names):
        topo["nodes"][name]["stake"] = stakes[i]

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


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Mainnet-faithful topology generator (100 or 150 nodes)."
    )
    parser.add_argument(
        "--regenerate-100",
        action="store_true",
        help="Hit Koios live and re-emit topology-realistic-100.yaml to stdout. "
        "Drifts from the committed epoch-582 snapshot; requires M5 goldens re-pinning.",
    )
    parser.add_argument(
        "--base",
        type=Path,
        default=Path("parameters/phase-2-sweep/topology-realistic-100.yaml"),
        help="Path to the committed 100-node topology YAML (input to 150-node generation).",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=Path("parameters/phase-2-sweep/topology-realistic-150.yaml"),
        help="Output path for the 150-node topology YAML.",
    )
    parser.add_argument(
        "--jitter-seed",
        type=int,
        default=582,
        help="Master RNG seed for the 50 extras (default: 582 = snapshot epoch).",
    )
    parser.add_argument(
        "--jitter-sd-pct",
        type=float,
        default=7.0,
        help="Standard deviation of per-producer latency-ms Gaussian factor, in percent (default: 7.0).",
    )
    args = parser.parse_args()

    if args.regenerate_100:
        regenerate_100_from_koios()
    else:
        emit_150_from_committed_100(
            base_path=args.base,
            out_path=args.out,
            jitter_seed=args.jitter_seed,
            jitter_sd_pct=args.jitter_sd_pct,
        )
        print(f"Wrote {args.out}", file=sys.stderr)


if __name__ == "__main__":
    main()
