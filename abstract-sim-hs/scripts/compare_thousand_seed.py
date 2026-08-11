#!/usr/bin/env python3
"""Preserve the thousand-seed low/severe-congestion replication as a CIP evidence record.

Reads the two 1,000-seed sweep summaries (flat fee vs canonical D16/K10), computes
paired statistics for the decision-facing metrics, and writes a self-contained
evidence JSON that also preserves the per-seed values, so the CIP tables can be
audited without the gitignored sweep outputs.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any

from compare_cross_lane_inversion_smoke import (
    load_summary,
    numeric_scalar,
    paired_interval,
    variant_runs,
)

FLAT_VARIANT = "flat-fee"
CANONICAL_VARIANT = "canonical-final-d16-k10"
PROJECT_DIR = Path(__file__).resolve().parent.parent

# (scalar key, per-seed rounding decimals, report scale, report unit)
LOW_METRICS = [
    ("value.urgent.retainedRatio", 9, 100.0, "pp"),
    ("value.retainedRatio", 9, 100.0, "pp"),
    ("value.standard.retainedRatio", 9, 100.0, "pp"),
    ("inclusion.urgent.serviceRate", 9, 100.0, "pp"),
    ("latency.urgent.meanBlocks", 6, 1.0, "blocks"),
    ("latency.urgent.meanSlots", 6, 1.0, "slots"),
    ("latency.urgent.p50Slots", 6, 1.0, "slots"),
    ("latency.urgent.p95Slots", 6, 1.0, "slots"),
    ("latency.urgent.p50Blocks", 6, 1.0, "blocks"),
    ("latency.urgent.p95Blocks", 6, 1.0, "blocks"),
    ("latency.standard.meanBlocks", 6, 1.0, "blocks"),
    ("latency.standard.meanSlots", 6, 1.0, "slots"),
    ("latency.standard.p50Slots", 6, 1.0, "slots"),
    ("latency.standard.p95Slots", 6, 1.0, "slots"),
    ("latency.standard.p50Blocks", 6, 1.0, "blocks"),
    ("latency.standard.p95Blocks", 6, 1.0, "blocks"),
    # No latency.priority.* rows: flat fee routes nothing through the priority
    # lane, so its scalars are a structural zero and the paired difference would
    # report the mechanism's own level as if it were an effect.
    ("latency.meanSlots", 6, 1.0, "slots"),
    ("value.retainedLovelace", 0, 1.0, "lovelace"),
    ("value.lostLovelace", 0, 1.0, "lovelace"),
    ("value.priority.retainedLovelace", 0, 1.0, "lovelace"),
    ("value.priority.lostLovelace", 0, 1.0, "lovelace"),
    ("inclusion.urgent.submitted", 0, 1.0, "transactions"),
    ("inclusion.priority.submitted", 0, 1.0, "transactions"),
    ("inclusion.standard.submitted", 0, 1.0, "transactions"),
    ("throughput.ebUtilization", 9, 100.0, "percent"),
]
SEVERE_METRICS = [
    ("value.urgent.retainedRatio", 9, 100.0, "pp"),
    ("value.retainedRatio", 9, 100.0, "pp"),
    ("inclusion.urgent.serviceRate", 9, 100.0, "pp"),
    ("latency.urgent.meanBlocks", 6, 1.0, "blocks"),
    ("latency.urgent.meanSlots", 6, 1.0, "slots"),
    ("latency.urgent.p50Slots", 6, 1.0, "slots"),
    ("latency.urgent.p95Slots", 6, 1.0, "slots"),
    ("latency.urgent.p50Blocks", 6, 1.0, "blocks"),
    ("latency.urgent.p95Blocks", 6, 1.0, "blocks"),
    ("latency.standard.meanBlocks", 6, 1.0, "blocks"),
    ("latency.standard.meanSlots", 6, 1.0, "slots"),
    ("latency.standard.p50Slots", 6, 1.0, "slots"),
    ("latency.standard.p95Slots", 6, 1.0, "slots"),
    ("latency.standard.p50Blocks", 6, 1.0, "blocks"),
    ("latency.standard.p95Blocks", 6, 1.0, "blocks"),
]


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git_provenance() -> dict[str, Any]:
    try:
        revision = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=PROJECT_DIR,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        status = subprocess.run(
            ["git", "status", "--porcelain", "--untracked-files=all", "--", "."],
            cwd=PROJECT_DIR,
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError):
        return {"git_revision": None, "abstract_sim_worktree_clean": None}
    return {
        "git_revision": revision,
        "abstract_sim_worktree_clean": not bool(status.strip()),
    }


def round_value(value: float, decimals: int) -> Any:
    if decimals == 0:
        return int(round(value))
    return round(value, decimals)


def load_metrics(
    directory: Path, metrics: list[tuple[str, int, float, str]]
) -> dict[str, Any]:
    summary_path = directory / "summary.json"
    summary = load_summary(summary_path)
    flat = variant_runs(summary, FLAT_VARIANT, summary_path)
    mech = variant_runs(summary, CANONICAL_VARIANT, summary_path)
    seeds = sorted(set(flat) & set(mech))
    if len(seeds) != summary["seeds"]:
        raise ValueError(
            f"{summary_path}: expected {summary['seeds']} paired seeds, found {len(seeds)}"
        )

    per_metric: dict[str, Any] = {}
    for key, decimals, scale, unit in metrics:
        flat_values = [numeric_scalar(flat[s], key, FLAT_VARIANT, s) for s in seeds]
        mech_values = [numeric_scalar(mech[s], key, CANONICAL_VARIANT, s) for s in seeds]
        diffs = [(m - f) * scale for f, m in zip(flat_values, mech_values)]
        mean_diff, ci_low, ci_high = paired_interval(diffs)
        per_metric[key] = {
            "unit": unit,
            "reportScale": scale,
            "flatFeeMean": sum(flat_values) / len(flat_values) * scale,
            "canonicalMean": sum(mech_values) / len(mech_values) * scale,
            "pairedMeanDifference": mean_diff,
            "ci95": [ci_low, ci_high],
            "canonicalHigherCount": sum(1 for d in diffs if d > 0),
            "perSeed": {
                FLAT_VARIANT: [round_value(v, decimals) for v in flat_values],
                CANONICAL_VARIANT: [round_value(v, decimals) for v in mech_values],
            },
        }

    provenance = {
        "summary": str(summary_path.relative_to(PROJECT_DIR)),
        "summarySha256": sha256_file(summary_path),
    }
    for extra in (
        f"{FLAT_VARIANT}.config.json",
        f"{CANONICAL_VARIANT}.config.json",
        "selected-load-profile.json",
    ):
        extra_path = directory / extra
        if extra_path.exists():
            provenance[extra] = sha256_file(extra_path)

    return {
        "seeds": len(seeds),
        "slots": summary["slots"],
        "randomness": summary.get("randomness"),
        "metrics": per_metric,
        "provenance": provenance,
    }


def build_record(args: argparse.Namespace) -> dict[str, Any]:
    low = load_metrics(args.low_dir, LOW_METRICS)
    severe = load_metrics(args.severe_dir, SEVERE_METRICS)
    manifest = PROJECT_DIR / "config/sweeps/canonical-headlines.json"
    record: dict[str, Any] = {
        "description": (
            "Preserved per-seed evidence for the 1,000-seed replication of the "
            "flat-fee versus canonical D16/K10 pairing at low and severe-congestion "
            "load. Statistics are paired mean differences (canonical minus flat fee) "
            "with two-sided 95% paired-t confidence intervals; ratio metrics are "
            "reported in percentage points."
        ),
        "generatedAt": args.generated_at,
        "experiment": {
            "manifest": "config/sweeps/canonical-headlines.json",
            "manifestSha256": sha256_file(manifest),
            "loads": {"low": "--load low", "severe-congestion": "config/loads/severe-congestion.json"},
            "reproduction": [
                'sim="$(stack path --local-install-root)/bin/abstract-sim-hs-exe"',
                '"$sim" sweep config/sweeps/canonical-headlines.json --seeds 1000 --slots 2000 --summary-only --load low --out sweep-results/canonical-1000-low',
                '"$sim" sweep config/sweeps/canonical-headlines.json --seeds 1000 --slots 2000 --summary-only --load-profile config/loads/severe-congestion.json --out sweep-results/canonical-1000-severe-congestion',
                "python3 scripts/compare_thousand_seed.py",
            ],
        },
        "results": {"low": low, "severe-congestion": severe},
    }
    record["provenance"] = git_provenance()
    if args.provenance_note:
        record["provenance"]["note"] = args.provenance_note
    return record


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--low-dir",
        type=Path,
        default=PROJECT_DIR / "sweep-results/canonical-1000-low",
    )
    parser.add_argument(
        "--severe-dir",
        type=Path,
        default=PROJECT_DIR / "sweep-results/canonical-1000-severe-congestion",
    )
    parser.add_argument(
        "--json-output",
        type=Path,
        default=PROJECT_DIR.parent
        / "docs/phase-2/CIP-urgency-signalling/thousand-seed-low-severe.json",
    )
    parser.add_argument("--generated-at", default=None)
    parser.add_argument("--provenance-note", default=None)
    args = parser.parse_args(argv)
    if args.generated_at is None:
        parser.error("--generated-at is required (dates are recorded explicitly, not sampled)")
    return args


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    record = build_record(args)
    args.json_output.write_text(json.dumps(record, indent=1) + "\n")
    for load, result in record["results"].items():
        print(f"== {load} ({result['seeds']} paired seeds) ==")
        for key, stats in result["metrics"].items():
            lo, hi = stats["ci95"]
            print(
                f"  {key:35s} flat={stats['flatFeeMean']:9.3f} canonical={stats['canonicalMean']:9.3f} "
                f"diff={stats['pairedMeanDifference']:+8.3f} [{lo:+.3f}, {hi:+.3f}] {stats['unit']}"
            )
    print(f"wrote {args.json_output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
