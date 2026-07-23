#!/usr/bin/env python3
"""Compare the canonical integrated run with the archived D16 references."""

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
    / "canonical-final-d16-k10-launch-day"
    / "summary.json"
)
PRE_CORRECTION_VARIANT = "bdst-tu50-d16"
CORRECTED_D16_VARIANT = "corrected-max-no-floor-d16"
CANDIDATE_VARIANT = "canonical-final-d16-k10"


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    baseline_path = Path(args.baseline)
    candidate_path = Path(args.candidate)
    baseline_summary = load_summary(baseline_path)
    candidate_summary = load_summary(candidate_path)

    pre_correction = variant_runs(
        baseline_summary, args.pre_correction_variant, baseline_path
    )
    corrected_d16 = variant_runs(
        baseline_summary, args.corrected_d16_variant, baseline_path
    )
    candidate = variant_runs(candidate_summary, args.candidate_variant, candidate_path)

    pre_correction_seeds = set(pre_correction)
    corrected_d16_seeds = set(corrected_d16)
    candidate_seeds = set(candidate)
    if not (
        pre_correction_seeds == corrected_d16_seeds == candidate_seeds
    ):
        raise ValueError(
            "paired comparison requires identical seed sets; "
            f"pre-correction={sorted(pre_correction_seeds)}, "
            f"corrected-D16={sorted(corrected_d16_seeds)}, "
            f"canonical={sorted(candidate_seeds)}"
        )
    seeds = sorted(candidate_seeds)
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
                "canonical_vs_corrected_d16": comparison(
                    metric,
                    candidate,
                    corrected_d16,
                    args.candidate_variant,
                    args.corrected_d16_variant,
                    seeds,
                ),
                "canonical_vs_pre_correction_d16": comparison(
                    metric,
                    candidate,
                    pre_correction,
                    args.candidate_variant,
                    args.pre_correction_variant,
                    seeds,
                ),
            }
        )

    return {
        "method": "paired mean difference with two-sided 95% Student-t confidence interval",
        "scope": (
            "canonical D16/D16 threshold-reservation K=10 launch-day integration "
            "check; not an equivalence test"
        ),
        "seeds": seeds,
        "references": {
            "summary": relative_source(baseline_path),
            "corrected_d16_variant": args.corrected_d16_variant,
            "pre_correction_d16_variant": args.pre_correction_variant,
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
        "# Canonical final configuration: integrated launch-day comparison",
        "",
        (
            f"Paired seeds: {', '.join(str(seed) for seed in seeds)} (n={len(seeds)}). "
            "Cells are canonical-final minus the named D16 reference, with "
            "two-sided 95% paired-t confidence intervals."
        ),
        "",
        (
            f"Archived references: `{report['references']['corrected_d16_variant']}` "
            f"and `{report['references']['pre_correction_d16_variant']}` from "
            f"`{report['references']['summary']}`."
        ),
        "",
        "| Metric | Unit | Canonical final - corrected max/no-floor D16 | Canonical final - pre-correction D16 |",
        "|---|---:|---:|---:|",
    ]
    for row in report["metrics"]:
        lines.append(
            "| {label} | {unit} | {corrected} | {pre_correction} |".format(
                label=row["label"],
                unit=row["unit"],
                corrected=interval_text(
                    row["canonical_vs_corrected_d16"], row["digits"]
                ),
                pre_correction=interval_text(
                    row["canonical_vs_pre_correction_d16"], row["digits"]
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
                "This diagnostic checks the complete recommended configuration as an "
                "integrated whole. It estimates observed differences and is not powered "
                "to establish statistical equivalence. The pre-correction reference is "
                "archived evidence whose exact code revision was not recorded."
            ),
        ]
    )
    return "\n".join(lines) + "\n"


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", default=str(DEFAULT_BASELINE))
    parser.add_argument("--candidate", default=str(DEFAULT_CANDIDATE))
    parser.add_argument(
        "--pre-correction-variant", default=PRE_CORRECTION_VARIANT
    )
    parser.add_argument("--corrected-d16-variant", default=CORRECTED_D16_VARIANT)
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
