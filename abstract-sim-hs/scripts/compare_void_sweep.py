#!/usr/bin/env python3
"""Paired per-load deltas of every arm in a void-sweep result directory
against the current-w20-w5 base arm.

Usage: python3 scripts/compare_void_sweep.py --root sweep-results/void-size-sweep-low
"""

import argparse
import json
from pathlib import Path

BASE = "current-w20-w5"
LOADS = ["low", "mid-load", "severe-congestion", "eb-capacity-stress", "launch-day"]
METRICS = [
    ("value.retainedLovelace", 1e9, "overall retained (G)"),
    ("value.retainedRatio", 1, "retained ratio"),
    ("value.urgent.retainedLovelace", 1e9, "urgent retained (G)"),
    ("throughput.txPerSlot", 1, "throughput tx/slot"),
    ("inclusion.standard.serviceRate", 1, "standard service rate"),
    ("inclusion.urgent.serviceRate", 1, "urgent service rate"),
    ("latency.standard.meanBlocks", 1, "standard mean wait (blk)"),
    ("latency.urgent.meanBlocks", 1, "urgent mean wait (blk)"),
    ("price.settledCoefficientRange", 1, "settled coeff range"),
    ("price.oscillationExcessTravel", 1, "excess quote travel"),
]


def short(name: str) -> str:
    return name.replace("standard-void-", "").replace("-w5", "")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, type=Path)
    args = parser.parse_args()

    for load in LOADS:
        summary_path = args.root / load / "summary.json"
        if not summary_path.exists():
            print(f"\n=== {load} === missing: {summary_path}")
            continue
        data = json.loads(summary_path.read_text())
        arms = {v["name"]: {r["seed"]: r["scalars"] for r in v["runs"]} for v in data["variants"]}
        if BASE not in arms:
            print(f"\n=== {load} === no {BASE} arm present")
            continue
        seeds = sorted(arms[BASE])
        others = [name for name in arms if name != BASE]
        identical = [
            name for name in others
            if all(arms[name][s] == arms[BASE][s] for s in seeds)
        ]
        print(f"\n=== {load} (n={len(seeds)}) ===  bit-identical to base: {[short(a) for a in identical] or 'none'}")
        diff_arms = [name for name in others if name not in identical]
        if not diff_arms:
            continue
        print(f"  {'metric':26s} {'base':>9s}" + "".join(f" {short(a):>15s}" for a in diff_arms))
        for key, scale, label in METRICS:
            base_mean = sum(arms[BASE][s].get(key, 0) or 0 for s in seeds) / len(seeds) / scale
            row = f"  {label:26s} {base_mean:9.3f}"
            for name in diff_arms:
                deltas = [
                    (arms[name][s].get(key, 0) or 0) - (arms[BASE][s].get(key, 0) or 0)
                    for s in seeds
                ]
                mean = sum(deltas) / len(deltas) / scale
                pos = sum(1 for x in deltas if x > 0)
                neg = sum(1 for x in deltas if x < 0)
                row += f" {mean:+9.3f}({pos}/{neg})"
            print(row)


if __name__ == "__main__":
    main()
