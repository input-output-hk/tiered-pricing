#!/usr/bin/env python3
"""Compare low-load value attribution between flat fee and the canonical
D16/K10 mechanism.

Reads one paired sweep summary (flat-fee vs canonical-final-d16-k10) run under
the built-in low-load preset, computes paired statistics for the per-lane and
per-class value levels, entry counts, and latency, and writes comparison.md
and comparison.json.

Alongside the preserved scalars it derives, per seed:

- value.<slice>.submittedLovelace = retained + lost + unresolved for the
  overall, standard-lane, priority-lane, and urgent-class slices;
- value.residual.{retained,lost,unresolved}Lovelace = overall minus
  (standard lane + priority lane), the value component not carried by either
  lane's submitted transactions.
"""

from __future__ import annotations

import argparse
import hashlib
import json
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

# (scalar key, report scale, report unit)
METRICS = [
    ("units.total", 1.0, "units"),
    ("units.served", 1.0, "units"),
    ("units.abandoned", 1.0, "units"),
    ("units.unresolved", 1.0, "units"),
    ("inclusion.standard.submitted", 1.0, "transactions"),
    ("inclusion.standard.included", 1.0, "transactions"),
    ("inclusion.standard.serviceRate", 100.0, "pp"),
    ("inclusion.priority.submitted", 1.0, "transactions"),
    ("inclusion.priority.included", 1.0, "transactions"),
    ("inclusion.priority.serviceRate", 100.0, "pp"),
    ("inclusion.urgent.submitted", 1.0, "transactions"),
    ("inclusion.urgent.included", 1.0, "transactions"),
    ("inclusion.urgent.serviceRate", 100.0, "pp"),
    ("latency.standard.meanSlots", 1.0, "slots"),
    ("latency.standard.meanBlocks", 1.0, "blocks"),
    ("latency.priority.meanSlots", 1.0, "slots"),
    ("latency.priority.meanBlocks", 1.0, "blocks"),
    ("latency.urgent.meanSlots", 1.0, "slots"),
    ("latency.urgent.meanBlocks", 1.0, "blocks"),
    ("value.retainedLovelace", 1.0, "lovelace"),
    ("value.lostLovelace", 1.0, "lovelace"),
    ("value.unresolvedLovelace", 1.0, "lovelace"),
    ("value.retainedRatio", 100.0, "pp"),
    ("value.standard.retainedLovelace", 1.0, "lovelace"),
    ("value.standard.lostLovelace", 1.0, "lovelace"),
    ("value.standard.unresolvedLovelace", 1.0, "lovelace"),
    ("value.standard.retainedRatio", 100.0, "pp"),
    ("value.priority.retainedLovelace", 1.0, "lovelace"),
    ("value.priority.lostLovelace", 1.0, "lovelace"),
    ("value.priority.unresolvedLovelace", 1.0, "lovelace"),
    ("value.priority.retainedRatio", 100.0, "pp"),
    ("value.urgent.retainedLovelace", 1.0, "lovelace"),
    ("value.urgent.lostLovelace", 1.0, "lovelace"),
    ("value.urgent.unresolvedLovelace", 1.0, "lovelace"),
    ("value.urgent.retainedRatio", 100.0, "pp"),
]

VALUE_COMPONENTS = ["retainedLovelace", "lostLovelace", "unresolvedLovelace"]
SUBMITTED_VALUE_SLICES = {
    "value.submittedLovelace": "value",
    "value.standard.submittedLovelace": "value.standard",
    "value.priority.submittedLovelace": "value.priority",
    "value.urgent.submittedLovelace": "value.urgent",
}


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def project_relative(path: Path) -> str:
    resolved = path.resolve()
    try:
        return str(resolved.relative_to(PROJECT_DIR))
    except ValueError:
        return str(resolved)


def metric_row(
    unit: str,
    scale: float,
    flat_values: list[float],
    mech_values: list[float],
) -> dict[str, Any]:
    diffs = [(m - f) * scale for f, m in zip(flat_values, mech_values)]
    mean_diff, ci_low, ci_high = paired_interval(diffs)
    return {
        "unit": unit,
        "reportScale": scale,
        "flatFeeMean": sum(flat_values) / len(flat_values) * scale,
        "canonicalMean": sum(mech_values) / len(mech_values) * scale,
        "pairedMeanDifference": mean_diff,
        "ci95": [ci_low, ci_high],
        "canonicalHigherCount": sum(1 for d in diffs if d > 0),
        "perSeed": {
            FLAT_VARIANT: flat_values,
            CANONICAL_VARIANT: mech_values,
        },
    }


def slice_component_values(
    runs: dict[int, dict[str, float]],
    variant: str,
    seeds: list[int],
    prefix: str,
    component: str,
) -> list[float]:
    key = f"{prefix}.{component}"
    return [numeric_scalar(runs[s], key, variant, s) for s in seeds]


def load_metrics(directory: Path) -> dict[str, Any]:
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
    for key, scale, unit in METRICS:
        flat_values = [numeric_scalar(flat[s], key, FLAT_VARIANT, s) for s in seeds]
        mech_values = [numeric_scalar(mech[s], key, CANONICAL_VARIANT, s) for s in seeds]
        per_metric[key] = metric_row(unit, scale, flat_values, mech_values)

    for derived_key, prefix in SUBMITTED_VALUE_SLICES.items():
        flat_values = [
            sum(
                slice_component_values(flat, FLAT_VARIANT, seeds, prefix, component)[i]
                for component in VALUE_COMPONENTS
            )
            for i in range(len(seeds))
        ]
        mech_values = [
            sum(
                slice_component_values(mech, CANONICAL_VARIANT, seeds, prefix, component)[i]
                for component in VALUE_COMPONENTS
            )
            for i in range(len(seeds))
        ]
        per_metric[derived_key] = metric_row("lovelace", 1.0, flat_values, mech_values)
        per_metric[derived_key]["derived"] = (
            f"sum of {prefix}.retainedLovelace, {prefix}.lostLovelace, "
            f"{prefix}.unresolvedLovelace"
        )

    for component in VALUE_COMPONENTS:
        derived_key = f"value.residual.{component}"
        flat_values = [
            numeric_scalar(flat[s], f"value.{component}", FLAT_VARIANT, s)
            - numeric_scalar(flat[s], f"value.standard.{component}", FLAT_VARIANT, s)
            - numeric_scalar(flat[s], f"value.priority.{component}", FLAT_VARIANT, s)
            for s in seeds
        ]
        mech_values = [
            numeric_scalar(mech[s], f"value.{component}", CANONICAL_VARIANT, s)
            - numeric_scalar(mech[s], f"value.standard.{component}", CANONICAL_VARIANT, s)
            - numeric_scalar(mech[s], f"value.priority.{component}", CANONICAL_VARIANT, s)
            for s in seeds
        ]
        per_metric[derived_key] = metric_row("lovelace", 1.0, flat_values, mech_values)
        per_metric[derived_key]["derived"] = (
            f"value.{component} minus value.standard.{component} "
            f"minus value.priority.{component}"
        )

    provenance = {
        "summary": project_relative(summary_path),
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


def format_number(value: float, unit: str) -> str:
    if unit == "lovelace":
        return f"{value:,.0f}"
    if unit in ("transactions", "units"):
        return f"{value:,.2f}"
    return f"{value:.3f}"


def render_markdown(record: dict[str, Any], simulator_sha256: str | None) -> str:
    result = record["results"]
    lines = [
        "# Low-load value attribution: flat fee vs canonical D16/K10",
        "",
        f"Paired over {result['seeds']} seeds, {result['slots']} slots, "
        f"randomness: {result.get('randomness')}. Differences are canonical minus "
        "flat fee with two-sided 95% paired-t confidence intervals.",
        "",
        "| Metric | Unit | Flat fee | Canonical | Paired diff | 95% CI | Canonical higher |",
        "|---|---|---:|---:|---:|---|---:|",
    ]
    for key, row in result["metrics"].items():
        unit = row["unit"]
        ci_low, ci_high = row["ci95"]
        lines.append(
            f"| {key} | {unit} "
            f"| {format_number(row['flatFeeMean'], unit)} "
            f"| {format_number(row['canonicalMean'], unit)} "
            f"| {format_number(row['pairedMeanDifference'], unit)} "
            f"| [{format_number(ci_low, unit)}, {format_number(ci_high, unit)}] "
            f"| {row['canonicalHigherCount']}/{result['seeds']} |"
        )
    lines.append("")
    lines.append(
        "value.<slice>.submittedLovelace rows are derived per seed as "
        "retained + lost + unresolved for that slice. value.residual.* rows are "
        "derived per seed as the overall value component minus the standard-lane "
        "and priority-lane components."
    )
    if simulator_sha256:
        lines.append("")
        lines.append(f"Simulator sha256: `{simulator_sha256}`")
    lines.append("")
    return "\n".join(lines)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True, help="sweep output directory")
    parser.add_argument(
        "--manifest",
        type=Path,
        default=PROJECT_DIR / "config/sweeps/canonical-headlines.json",
        help="sweep manifest the output was produced from (recorded in provenance)",
    )
    parser.add_argument("--simulator-sha256", default=None)
    parser.add_argument("--markdown-output", type=Path, required=True)
    parser.add_argument("--json-output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    manifest = args.manifest
    record: dict[str, Any] = {
        "description": (
            "Paired comparison of value attribution under low load: flat fee "
            "versus the canonical D16/K10 mechanism. Value levels, entry, and "
            "latency are reported per lane (standard, priority) and for the "
            "urgent demand class. Statistics are paired mean differences "
            "(canonical minus flat fee) with two-sided 95% paired-t confidence "
            "intervals; ratio metrics are in percentage points."
        ),
        "experiment": {
            "manifest": project_relative(manifest),
            "manifestSha256": sha256_file(manifest),
            "load": "low (built-in preset)",
        },
        "results": load_metrics(args.root),
    }
    if args.simulator_sha256:
        record["experiment"]["simulatorSha256"] = args.simulator_sha256

    args.json_output.parent.mkdir(parents=True, exist_ok=True)
    with args.json_output.open("w", encoding="utf-8") as handle:
        json.dump(record, handle, indent=2)
        handle.write("\n")
    args.markdown_output.parent.mkdir(parents=True, exist_ok=True)
    args.markdown_output.write_text(
        render_markdown(record, args.simulator_sha256), encoding="utf-8"
    )
    print(f"wrote {args.json_output}")
    print(f"wrote {args.markdown_output}")


if __name__ == "__main__":
    main()
