#!/usr/bin/env python3
"""Compare the paired cross-lane fee-inversion smoke runs.

The report deliberately presents paired differences rather than treating the
three mechanisms as independent samples.  It is a small diagnostic experiment,
not an equivalence test; the confidence intervals quantify the uncertainty in
the observed mean differences.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Iterable


SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_DIR = SCRIPT_DIR.parent

DEFAULT_BASELINE = (
    PROJECT_DIR.parent
    / "docs"
    / "phase-2"
    / "experiment-results"
    / "cross-lane-inversion-smoke.json"
)
DEFAULT_CANDIDATE = (
    PROJECT_DIR
    / "sweep-results"
    / "cross-lane-inversion-smoke-launch-day"
    / "summary.json"
)

BASELINE_VARIANT = "both-dynamic-strict-threshold-rb2-windowed"
MAX_VARIANT = "corrected-max-no-floor"
FLOOR_VARIANT = "corrected-max-floor1"


@dataclass(frozen=True)
class Metric:
    key: str
    label: str
    unit: str
    transform: Callable[[float, float], float]
    digits: int


def absolute(candidate: float, reference: float) -> float:
    return candidate - reference


def percentage_points(candidate: float, reference: float) -> float:
    return 100.0 * (candidate - reference)


def relative_percent(candidate: float, reference: float) -> float:
    if reference == 0.0:
        raise ValueError("cannot calculate a relative change from zero")
    return 100.0 * (candidate / reference - 1.0)


METRICS = (
    Metric("value.retainedRatio", "Retained value", "percentage points", percentage_points, 3),
    Metric(
        "value.priority.retainedRatio",
        "Priority retained value",
        "percentage points",
        percentage_points,
        3,
    ),
    Metric("units.serviceRate", "Unit service rate", "percentage points", percentage_points, 3),
    Metric(
        "inclusion.priority.serviceRate",
        "Priority service rate",
        "percentage points",
        percentage_points,
        3,
    ),
    Metric(
        "inclusion.priority.submitted",
        "Priority submissions",
        "percent",
        relative_percent,
        2,
    ),
    Metric("latency.meanBlocks", "Mean latency", "blocks", absolute, 3),
    Metric("latency.priority.meanBlocks", "Priority latency", "blocks", absolute, 3),
    Metric("throughput.txPerSlot", "Throughput", "tx/slot", absolute, 3),
    Metric(
        "revenue.feesCollectedLovelace",
        "Fees collected",
        "percent",
        relative_percent,
        2,
    ),
)


# Two-sided 95% Student-t critical values.  Smoke sweeps currently use ten
# paired seeds, but the complete small-sample table keeps ad-hoc reruns honest.
T_975 = {
    1: 12.706,
    2: 4.303,
    3: 3.182,
    4: 2.776,
    5: 2.571,
    6: 2.447,
    7: 2.365,
    8: 2.306,
    9: 2.262,
    10: 2.228,
    11: 2.201,
    12: 2.179,
    13: 2.160,
    14: 2.145,
    15: 2.131,
    16: 2.120,
    17: 2.110,
    18: 2.101,
    19: 2.093,
    20: 2.086,
    21: 2.080,
    22: 2.074,
    23: 2.069,
    24: 2.064,
    25: 2.060,
    26: 2.056,
    27: 2.052,
    28: 2.048,
    29: 2.045,
    30: 2.042,
}


def t_critical_95(degrees_of_freedom: int) -> float:
    if degrees_of_freedom < 1:
        raise ValueError("at least two paired seeds are required for a t interval")
    if degrees_of_freedom in T_975:
        return T_975[degrees_of_freedom]
    # Cornish-Fisher expansion around z=.975; accurate well beyond the report's
    # displayed precision for df > 30.
    z = 1.959963984540054
    df = float(degrees_of_freedom)
    return (
        z
        + (z**3 + z) / (4.0 * df)
        + (5.0 * z**5 + 16.0 * z**3 + 3.0 * z) / (96.0 * df**2)
        + (3.0 * z**7 + 19.0 * z**5 + 17.0 * z**3 - 15.0 * z)
        / (384.0 * df**3)
    )


def load_summary(path: Path) -> dict[str, Any]:
    try:
        with path.open("r", encoding="utf-8") as handle:
            value = json.load(handle)
    except FileNotFoundError as error:
        raise ValueError(f"summary does not exist: {path}") from error
    except json.JSONDecodeError as error:
        raise ValueError(f"invalid JSON in {path}: {error}") from error
    if not isinstance(value, dict) or not isinstance(value.get("variants"), list):
        raise ValueError(f"not a sweep summary (missing variants array): {path}")
    return value


def variant_runs(summary: dict[str, Any], name: str, source: Path) -> dict[int, dict[str, float]]:
    matches = [variant for variant in summary["variants"] if variant.get("name") == name]
    if len(matches) != 1:
        available = ", ".join(sorted(str(v.get("name")) for v in summary["variants"]))
        raise ValueError(
            f"expected exactly one variant {name!r} in {source}; available: {available}"
        )

    runs: dict[int, dict[str, float]] = {}
    for run in matches[0].get("runs", []):
        seed = run.get("seed")
        scalars = run.get("scalars")
        if isinstance(seed, bool) or not isinstance(seed, int) or not isinstance(scalars, dict):
            raise ValueError(f"malformed run in variant {name!r} in {source}")
        if seed in runs:
            raise ValueError(f"duplicate seed {seed} in variant {name!r} in {source}")
        runs[seed] = scalars
    if not runs:
        raise ValueError(f"variant {name!r} has no runs in {source}")
    return runs


def numeric_scalar(scalars: dict[str, Any], key: str, variant: str, seed: int) -> float:
    value = scalars.get(key)
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
        raise ValueError(f"missing/non-finite scalar {key!r} for {variant}, seed {seed}")
    return float(value)


def paired_interval(differences: Iterable[float]) -> tuple[float, float, float]:
    values = list(differences)
    n = len(values)
    if n < 2:
        raise ValueError("at least two paired seeds are required")
    mean = sum(values) / n
    sample_variance = sum((value - mean) ** 2 for value in values) / (n - 1)
    margin = t_critical_95(n - 1) * math.sqrt(sample_variance / n)
    return mean, mean - margin, mean + margin


def comparison(
    metric: Metric,
    candidate: dict[int, dict[str, float]],
    reference: dict[int, dict[str, float]],
    candidate_name: str,
    reference_name: str,
    seeds: list[int],
) -> dict[str, float]:
    differences = []
    for seed in seeds:
        candidate_value = numeric_scalar(candidate[seed], metric.key, candidate_name, seed)
        reference_value = numeric_scalar(reference[seed], metric.key, reference_name, seed)
        try:
            differences.append(metric.transform(candidate_value, reference_value))
        except ValueError as error:
            raise ValueError(f"{metric.key}, seed {seed}: {error}") from error
    mean, low, high = paired_interval(differences)
    return {"mean": mean, "ci95_low": low, "ci95_high": high}


def relative_source(path: Path) -> str:
    resolved = path.resolve()
    try:
        return str(resolved.relative_to(PROJECT_DIR))
    except ValueError:
        return str(path)


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    baseline_path = Path(args.baseline)
    candidate_path = Path(args.candidate)
    baseline_summary = load_summary(baseline_path)
    candidate_summary = load_summary(candidate_path)

    baseline = variant_runs(baseline_summary, args.baseline_variant, baseline_path)
    maximum = variant_runs(candidate_summary, args.max_variant, candidate_path)
    floor = variant_runs(candidate_summary, args.floor_variant, candidate_path)

    baseline_seeds = set(baseline)
    max_seeds = set(maximum)
    floor_seeds = set(floor)
    if not (baseline_seeds == max_seeds == floor_seeds):
        raise ValueError(
            "paired comparison requires identical seed sets; "
            f"baseline={sorted(baseline_seeds)}, max={sorted(max_seeds)}, "
            f"floor1={sorted(floor_seeds)}"
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
                "max_vs_pre_correction": comparison(
                    metric,
                    maximum,
                    baseline,
                    args.max_variant,
                    args.baseline_variant,
                    seeds,
                ),
                "floor1_vs_pre_correction": comparison(
                    metric,
                    floor,
                    baseline,
                    args.floor_variant,
                    args.baseline_variant,
                    seeds,
                ),
                "floor1_vs_max": comparison(
                    metric,
                    floor,
                    maximum,
                    args.floor_variant,
                    args.max_variant,
                    seeds,
                ),
            }
        )

    return {
        "method": "paired mean difference with two-sided 95% Student-t confidence interval",
        "seeds": seeds,
        "baseline": {
            "summary": relative_source(baseline_path),
            "variant": args.baseline_variant,
        },
        "candidates": {
            "summary": relative_source(candidate_path),
            "max_no_floor_variant": args.max_variant,
            "floor1_variant": args.floor_variant,
        },
        "metrics": rows,
    }


def signed(value: float, digits: int) -> str:
    rounded = round(value, digits)
    if rounded == 0:
        rounded = 0.0
    return f"{rounded:+.{digits}f}"


def interval_text(result: dict[str, float], digits: int) -> str:
    return (
        f"{signed(result['mean'], digits)} "
        f"[{signed(result['ci95_low'], digits)}, {signed(result['ci95_high'], digits)}]"
    )


def markdown(report: dict[str, Any]) -> str:
    seeds = report["seeds"]
    lines = [
        "# Cross-lane fee-inversion smoke comparison",
        "",
        (
            f"Paired seeds: {', '.join(str(seed) for seed in seeds)} (n={len(seeds)}). "
            "Cells are mean differences with two-sided 95% paired-t CIs; positive "
            "means the left-hand mechanism is higher."
        ),
        "",
        (
            f"Baseline: `{report['baseline']['variant']}` from "
            f"`{report['baseline']['summary']}`."
        ),
        "",
        "| Metric | Unit | Max/no floor - pre-correction | Floor 1× - pre-correction | Floor 1× - max/no floor |",
        "|---|---:|---:|---:|---:|",
    ]
    for row in report["metrics"]:
        lines.append(
            "| {label} | {unit} | {maximum} | {floor} | {floor_vs_max} |".format(
                label=row["label"],
                unit=row["unit"],
                maximum=interval_text(row["max_vs_pre_correction"], row["digits"]),
                floor=interval_text(row["floor1_vs_pre_correction"], row["digits"]),
                floor_vs_max=interval_text(row["floor1_vs_max"], row["digits"]),
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
                "This smoke experiment estimates observed differences; it is not powered "
                "to establish statistical equivalence or the absence of a practically "
                "important effect."
            ),
        ]
    )
    return "\n".join(lines) + "\n"


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", default=str(DEFAULT_BASELINE), help="pre-correction summary.json")
    parser.add_argument("--candidate", default=str(DEFAULT_CANDIDATE), help="candidate summary.json")
    parser.add_argument("--baseline-variant", default=BASELINE_VARIANT)
    parser.add_argument("--max-variant", default=MAX_VARIANT)
    parser.add_argument("--floor-variant", default=FLOOR_VARIANT)
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
