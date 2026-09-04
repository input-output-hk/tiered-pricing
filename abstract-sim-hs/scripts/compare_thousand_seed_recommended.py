#!/usr/bin/env python3
"""Preserve the thousand-seed low/severe-congestion replication under the
adopted recommended construction as a CIP evidence record.

Reads the two 1,000-seed sweep summaries (flat fee versus the recommended
construction: 0.75 standard target, 10-block processed-block standard window,
age-escape reset at certification), computes paired statistics for the
decision-facing metrics, and writes a self-contained evidence JSON that also
preserves the per-seed values, so the CIP tables can be audited without the
gitignored sweep outputs. The metric list matches the historical record
(`thousand-seed-low-severe.json`), which this record supersedes for the
recommended configuration.
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
from compare_thousand_seed import LOW_METRICS, SEVERE_METRICS, round_value, sha256_file

FLAT_VARIANT = "flat-fee"
RECOMMENDED_VARIANT = "recommended"
PROJECT_DIR = Path(__file__).resolve().parent.parent
MANIFEST = PROJECT_DIR / "config/sweeps/recommended-headlines.json"

EXPECTED_VARIANT_CONFIGS = {
    FLAT_VARIANT: "config/variants/flat-fee.json",
    RECOMMENDED_VARIANT: "config/variants/standard-window-confirm/block-window-10-cert-reset.json",
}

RECOMMENDED_EXPECTATIONS = {
    ("design", "controllers", "standardController", "targetUtilisation"): 0.75,
    ("design", "controllers", "standardController", "maxChangeDenominator"): 16,
    (
        "design",
        "controllers",
        "standardController",
        "signal",
        "type",
    ): "capacity-weighted-block-window",
    ("design", "controllers", "standardController", "signal", "window"): 10,
    ("design", "controllers", "priorityController", "targetUtilisation"): 0.5,
    ("design", "reservationPolicy", "ebThresholdBytes"): 45_056,
    ("design", "reservationPolicy", "ebAgeEscapeRbIntervals"): 10,
    ("design", "ageResetAtCertification"): True,
}


def git_provenance(patch_output: Path) -> dict[str, Any]:
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
    provenance: dict[str, Any] = {
        "git_revision": revision,
        "abstract_sim_worktree_clean": not bool(status.strip()),
    }
    if status.strip():
        diff = subprocess.run(
            ["git", "diff", "HEAD", "--", "."],
            cwd=PROJECT_DIR,
            check=True,
            capture_output=True,
            text=True,
        ).stdout
        patch_output.write_text(diff)
        provenance["comparisonTimeSourcePatch"] = {
            "path": patch_output.name,
            "sha256": sha256_file(patch_output),
            "coversUntrackedFiles": False,
        }
    return provenance


def nested(config: dict[str, Any], path: tuple[str, ...], source: Path) -> Any:
    value: Any = config
    for part in path:
        if not isinstance(value, dict) or part not in value:
            raise ValueError(f"missing {'.'.join(path)} in {source}")
        value = value[part]
    return value


def validate_configs(directory: Path, summary: dict[str, Any], summary_path: Path) -> None:
    actual = {
        variant.get("name"): variant.get("config") for variant in summary["variants"]
    }
    if actual != EXPECTED_VARIANT_CONFIGS:
        raise ValueError(
            f"unexpected variants/configs in {summary_path}: "
            f"expected={EXPECTED_VARIANT_CONFIGS!r}, actual={actual!r}"
        )
    recommended_path = directory / f"{RECOMMENDED_VARIANT}.config.json"
    recommended = json.loads(recommended_path.read_text())
    for path, expected in RECOMMENDED_EXPECTATIONS.items():
        value = nested(recommended, path, recommended_path)
        if value != expected:
            raise ValueError(
                f"unexpected {'.'.join(path)} in {recommended_path}: "
                f"expected={expected!r}, actual={value!r}"
            )


def load_metrics(
    directory: Path, metrics: list[tuple[str, int, float, str]]
) -> dict[str, Any]:
    summary_path = directory / "summary.json"
    summary = load_summary(summary_path)
    validate_configs(directory, summary, summary_path)
    flat = variant_runs(summary, FLAT_VARIANT, summary_path)
    mech = variant_runs(summary, RECOMMENDED_VARIANT, summary_path)
    if set(flat) != set(mech):
        raise ValueError(f"{summary_path}: the two arms have different seed sets")
    seeds = sorted(set(flat) & set(mech))
    if len(seeds) != summary["seeds"]:
        raise ValueError(
            f"{summary_path}: expected {summary['seeds']} paired seeds, found {len(seeds)}"
        )

    per_metric: dict[str, Any] = {}
    for key, decimals, scale, unit in metrics:
        flat_values = [numeric_scalar(flat[s], key, FLAT_VARIANT, s) for s in seeds]
        mech_values = [
            numeric_scalar(mech[s], key, RECOMMENDED_VARIANT, s) for s in seeds
        ]
        diffs = [(m - f) * scale for f, m in zip(flat_values, mech_values)]
        mean_diff, ci_low, ci_high = paired_interval(diffs)
        per_metric[key] = {
            "unit": unit,
            "reportScale": scale,
            "flatFeeMean": sum(flat_values) / len(flat_values) * scale,
            "recommendedMean": sum(mech_values) / len(mech_values) * scale,
            "pairedMeanDifference": mean_diff,
            "ci95": [ci_low, ci_high],
            "recommendedHigherCount": sum(1 for d in diffs if d > 0),
            "perSeed": {
                FLAT_VARIANT: [round_value(v, decimals) for v in flat_values],
                RECOMMENDED_VARIANT: [round_value(v, decimals) for v in mech_values],
            },
        }

    try:
        summary_ref = str(summary_path.relative_to(PROJECT_DIR))
    except ValueError:
        summary_ref = str(summary_path)
    provenance = {
        "summary": summary_ref,
        "summarySha256": sha256_file(summary_path),
    }
    for extra in (
        f"{FLAT_VARIANT}.config.json",
        f"{RECOMMENDED_VARIANT}.config.json",
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
    record: dict[str, Any] = {
        "description": (
            "Preserved per-seed evidence for the 1,000-seed replication of the "
            "flat-fee versus recommended-construction pairing at low and "
            "severe-congestion load. The recommended construction is the adopted "
            "configuration: 0.75 standard target, 10-block processed-block standard "
            "window, age-escape reset at certification. Statistics are paired mean "
            "differences (recommended minus flat fee) with two-sided 95% paired-t "
            "confidence intervals; ratio metrics are reported in percentage points."
        ),
        "generatedAt": args.generated_at,
        "experiment": {
            "manifest": "config/sweeps/recommended-headlines.json",
            "manifestSha256": sha256_file(MANIFEST),
            "loads": {
                "low": "--load low",
                "severe-congestion": "config/loads/severe-congestion.json",
            },
            "reproduction": [
                "./scripts/run_thousand_seed_recommended.sh",
            ],
        },
        "results": {"low": low, "severe-congestion": severe},
    }
    patch_output = args.json_output.with_name(args.json_output.stem + "-source.patch")
    record["provenance"] = git_provenance(patch_output)
    record["provenance"]["sweepExecutableSha256"] = args.sweep_executable_sha256
    if args.provenance_note:
        record["provenance"]["note"] = args.provenance_note
    return record


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--low-dir",
        type=Path,
        default=PROJECT_DIR / "sweep-results/recommended-1000-low",
    )
    parser.add_argument(
        "--severe-dir",
        type=Path,
        default=PROJECT_DIR / "sweep-results/recommended-1000-severe-congestion",
    )
    parser.add_argument(
        "--json-output",
        type=Path,
        default=PROJECT_DIR.parent
        / "docs/phase-2/CIP-urgency-signalling/thousand-seed-low-severe-recommended.json",
    )
    parser.add_argument("--generated-at", default=None)
    parser.add_argument(
        "--sweep-executable-sha256",
        default=None,
        help="SHA-256 of the simulator executable that produced the sweeps",
    )
    parser.add_argument("--provenance-note", default=None)
    args = parser.parse_args(argv)
    if args.generated_at is None:
        parser.error("--generated-at is required (dates are recorded explicitly, not sampled)")
    if args.sweep_executable_sha256 is None:
        parser.error("--sweep-executable-sha256 is required (the runner records it)")
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
                f"  {key:35s} flat={stats['flatFeeMean']:9.3f} recommended={stats['recommendedMean']:9.3f} "
                f"diff={stats['pairedMeanDifference']:+8.3f} [{lo:+.3f}, {hi:+.3f}] {stats['unit']}"
            )
    print(f"wrote {args.json_output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
