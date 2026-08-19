#!/usr/bin/env python3
"""Compare the corrected D16 max-of-two run with its archived baseline."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from compare_cross_lane_inversion_smoke import (
    METRICS,
    comparison,
    interval_text,
    load_summary,
    relative_source,
    variant_runs,
)


SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_DIR = SCRIPT_DIR.parent
DEFAULT_BASELINE = (
    PROJECT_DIR.parent
    / "docs"
    / "phase-2"
    / "experiment-results"
    / "cross-lane-inversion-d16-baseline.json"
)
DEFAULT_CANDIDATE = (
    PROJECT_DIR
    / "sweep-results"
    / "cross-lane-inversion-smoke-d16-launch-day"
    / "summary.json"
)
BASELINE_VARIANT = "bdst-tu50-d16"
CANDIDATE_VARIANT = "corrected-max-no-floor-d16"


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    baseline_path = Path(args.baseline)
    candidate_path = Path(args.candidate)
    baseline_summary = load_summary(baseline_path)
    candidate_summary = load_summary(candidate_path)
    baseline = variant_runs(baseline_summary, args.baseline_variant, baseline_path)
    candidate = variant_runs(candidate_summary, args.candidate_variant, candidate_path)

    baseline_seeds = set(baseline)
    candidate_seeds = set(candidate)
    if baseline_seeds != candidate_seeds:
        raise ValueError(
            "paired comparison requires identical seed sets; "
            f"baseline={sorted(baseline_seeds)}, candidate={sorted(candidate_seeds)}"
        )
    seeds = sorted(baseline_seeds)
    if len(seeds) < 2:
        raise ValueError("paired comparison requires at least two seeds")

    rows = []
    for metric in METRICS:
        rows.append(
            {
                "key": metric.key,
                "label": metric.label,
                "unit": metric.unit,
                "digits": metric.digits,
                "candidate_vs_pre_correction": comparison(
                    metric,
                    candidate,
                    baseline,
                    args.candidate_variant,
                    args.baseline_variant,
                    seeds,
                ),
            }
        )

    return {
        "method": "paired mean difference with two-sided 95% Student-t confidence interval",
        "scope": "D16 launch-day max-of-two/no-floor diagnostic; not an equivalence test",
        "seeds": seeds,
        "baseline": {
            "summary": relative_source(baseline_path),
            "variant": args.baseline_variant,
            "provenance": "archived scalars and input hashes; exact legacy code revision unavailable",
        },
        "candidate": {
            "summary": relative_source(candidate_path),
            "variant": args.candidate_variant,
        },
        "metrics": rows,
    }


def markdown(report: dict[str, Any]) -> str:
    seeds = report["seeds"]
    lines = [
        "# D16 cross-lane fee-inversion smoke comparison",
        "",
        (
            f"Paired seeds: {', '.join(str(seed) for seed in seeds)} (n={len(seeds)}). "
            "Cells are corrected max-of-two/no-floor minus pre-correction, with "
            "two-sided 95% paired-t confidence intervals."
        ),
        "",
        (
            f"Archived baseline: `{report['baseline']['variant']}` from "
            f"`{report['baseline']['summary']}`."
        ),
        "",
        "| Metric | Unit | Corrected D16 - pre-correction D16 |",
        "|---|---:|---:|",
    ]
    for row in report["metrics"]:
        lines.append(
            "| {label} | {unit} | {result} |".format(
                label=row["label"],
                unit=row["unit"],
                result=interval_text(
                    row["candidate_vs_pre_correction"], row["digits"]
                ),
            )
        )
    lines.extend(
        [
            "",
            (
                "Percentage-point rows are absolute changes. Percent rows are the mean "
                "within-seed relative changes; other rows use the stated units."
            ),
            "",
            (
                "This diagnostic estimates observed differences. It is not powered to "
                "establish statistical equivalence or the absence of a practically "
                "important effect. The corrected candidate is replayable; the legacy "
                "half is an archived baseline whose exact code revision was not recorded."
            ),
        ]
    )
    return "\n".join(lines) + "\n"


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", default=str(DEFAULT_BASELINE))
    parser.add_argument("--candidate", default=str(DEFAULT_CANDIDATE))
    parser.add_argument("--baseline-variant", default=BASELINE_VARIANT)
    parser.add_argument("--candidate-variant", default=CANDIDATE_VARIANT)
    parser.add_argument("--format", choices=("markdown", "json"), default="markdown")
    parser.add_argument("--output", help="write the report here instead of stdout")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = build_report(args)
        rendered = (
            markdown(report)
            if args.format == "markdown"
            else json.dumps(report, indent=2, sort_keys=True) + "\n"
        )
        if args.output:
            output = Path(args.output)
            if output.exists():
                raise ValueError(f"refusing to overwrite existing report: {output}")
            output.write_text(rendered, encoding="utf-8")
            print(f"wrote {output}")
        else:
            sys.stdout.write(rendered)
    except (OSError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
