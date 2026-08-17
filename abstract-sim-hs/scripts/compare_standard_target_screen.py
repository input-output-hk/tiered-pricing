#!/usr/bin/env python3
"""Compare one load from the independent standard-lane target screen.

The patient demand-census arm first-submits every generated demand sample. In
independent-stream mode it therefore supplies, seed by seed, the common offered
unit and gross-value denominators for every honest-actor arm. Candidate results
are paired against flat fee and the canonical S=.5/U=.5 mechanism.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from compare_cross_lane_inversion_smoke import (
    load_summary,
    numeric_scalar,
    paired_interval,
    variant_runs,
)


PROJECT_DIR = Path(__file__).resolve().parent.parent
FLAT_VARIANT = "flat-fee"
CENSUS_VARIANT = "demand-census"
CANONICAL_VARIANT = "canonical-s50-u50"
CANDIDATE_VARIANTS = [
    CANONICAL_VARIANT,
    "priority-only-u50",
    "s625-u50",
    "s75-u50",
    "s875-u50",
]

VARIANT_LABELS = {
    CANONICAL_VARIANT: "canonical S=.50/U=.50",
    "priority-only-u50": "fixed standard/U=.50",
    "s625-u50": "S=.625/U=.50",
    "s75-u50": "S=.75/U=.50",
    "s875-u50": "S=.875/U=.50",
}

TOTAL_VALUE_COMPONENTS = (
    "value.retainedLovelace",
    "value.lostLovelace",
    "value.unresolvedLovelace",
)
URGENT_VALUE_COMPONENTS = (
    "value.urgent.retainedLovelace",
    "value.urgent.lostLovelace",
    "value.urgent.unresolvedLovelace",
)


@dataclass(frozen=True)
class Metric:
    key: str
    label: str
    unit: str
    digits: int


METRICS = [
    Metric("units.submitted", "First-submitted units", "units", 1),
    Metric("units.submittedShare", "Units entering / offered", "pp", 3),
    Metric("units.initialDeclined", "Units declining before first submission", "units", 1),
    Metric("value.submitted", "First-submitted gross value", "G lovelace", 3),
    Metric("value.submittedShare", "Submitted / offered gross value", "pp", 3),
    Metric("value.initialDeclined", "Gross value declining before first submission", "G lovelace", 3),
    Metric("value.retained", "Absolute gross retained value", "G lovelace", 3),
    Metric("value.lost", "Absolute gross value lost", "G lovelace", 3),
    Metric("value.unresolved", "Absolute unresolved gross value", "G lovelace", 3),
    Metric("value.nonurgent.retained", "Non-urgent absolute gross retained value", "G lovelace", 3),
    Metric("value.retainedOffered", "Gross retained / common offered value", "pp", 3),
    Metric("value.retainedConditional", "Conditional retained ratio", "pp", 3),
    Metric("fees.realised", "Realised fees", "G lovelace", 3),
    Metric("value.userNet", "Gross retained minus realised fees", "G lovelace", 3),
    Metric("urgent.units.submitted", "Urgent-class first-submitted units", "units", 1),
    Metric("urgent.units.initialDeclined", "Urgent units declining before first submission", "units", 1),
    Metric("urgent.value.submitted", "Urgent first-submitted gross value", "G lovelace", 3),
    Metric("urgent.value.initialDeclined", "Urgent gross value declining before first submission", "G lovelace", 3),
    Metric("urgent.value.retained", "Urgent absolute gross retained value", "G lovelace", 3),
    Metric("urgent.value.lost", "Urgent absolute gross value lost", "G lovelace", 3),
    Metric("urgent.value.unresolved", "Urgent absolute unresolved gross value", "G lovelace", 3),
    Metric("urgent.value.retainedOffered", "Urgent retained / common urgent offered value", "pp", 3),
    Metric("urgent.value.retainedConditional", "Urgent conditional retained ratio", "pp", 3),
    Metric("urgent.latency.meanBlocks", "Urgent mean inclusion delay", "blocks", 3),
    Metric("urgent.latency.p95Blocks", "Urgent p95 inclusion delay", "blocks", 3),
    Metric("throughput.txPerSlot", "Included throughput", "tx/slot", 3),
    Metric("throughput.ebUtilization", "EB utilisation", "pp", 3),
    Metric("price.shockCount", "Price shocks", "count", 2),
]


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def project_relative(path: Path) -> str:
    resolved = path.resolve()
    try:
        return str(resolved.relative_to(PROJECT_DIR))
    except ValueError:
        return str(resolved)


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as error:
        raise ValueError(f"required file does not exist: {path}") from error
    except json.JSONDecodeError as error:
        raise ValueError(f"invalid JSON in {path}: {error}") from error
    if not isinstance(value, dict):
        raise ValueError(f"expected a JSON object in {path}")
    return value


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def validate_effective_configs(root: Path, candidates: list[str]) -> dict[str, str]:
    paths = {
        name: root / f"{name}.config.json"
        for name in [CENSUS_VARIANT, FLAT_VARIANT, *candidates]
    }
    configs = {name: load_json(path) for name, path in paths.items()}

    expected_census = copy.deepcopy(configs[FLAT_VARIANT])
    expected_census["actors"][0]["type"] = "patient"
    require(
        configs[CENSUS_VARIANT] == expected_census,
        "demand-census must differ from effective flat-fee only by actor type=patient",
    )

    canonical = configs[CANONICAL_VARIANT]
    controllers = canonical["design"]["controllers"]
    standard = controllers["standardController"]
    urgent = controllers["priorityController"]
    reservation = canonical["design"]["reservationPolicy"]
    require(standard["targetUtilisation"] == 0.5, "canonical standard target must be 0.5")
    require(standard["maxChangeDenominator"] == 16, "canonical standard controller must use D16")
    require(standard["signal"] == {"type": "capacity-weighted-window", "window": 20}, "canonical standard signal must be the 20-block capacity-weighted window")
    require(urgent["targetUtilisation"] == 0.5, "canonical urgent target must be 0.5")
    require(urgent["maxChangeDenominator"] == 16, "canonical urgent controller must use D16")
    require(urgent["signal"] == {"type": "priority-reservation-window", "window": 5}, "canonical urgent signal must be the 5-sample reservation window")
    require(reservation["ebThresholdBytes"] == 45056, "EB threshold must stay at half an RB")
    require(reservation["ebAgeEscapeRbIntervals"] == 10, "EB age escape must stay at K10")

    if "priority-only-u50" in configs:
        expected_priority_only = copy.deepcopy(canonical)
        del expected_priority_only["design"]["controllers"]["standardController"]
        require(
            configs["priority-only-u50"] == expected_priority_only,
            "priority-only-u50 must differ from canonical only by removing standardController",
        )

    for name, target in (("s625-u50", 0.625), ("s75-u50", 0.75), ("s875-u50", 0.875)):
        if name not in configs:
            continue
        expected = copy.deepcopy(canonical)
        expected["design"]["controllers"]["standardController"]["targetUtilisation"] = target
        require(
            configs[name] == expected,
            f"{name} must differ from canonical only at standard targetUtilisation={target}",
        )

    return {name: sha256_file(path) for name, path in paths.items()}


def scalar(run: dict[str, float], key: str, variant: str, seed: int) -> float:
    return numeric_scalar(run, key, variant, seed)


def sum_scalars(
    run: dict[str, float], keys: tuple[str, ...], variant: str, seed: int
) -> float:
    return sum(scalar(run, key, variant, seed) for key in keys)


def ratio(numerator: float, denominator: float, context: str) -> float:
    if denominator <= 0:
        raise ValueError(f"non-positive denominator for {context}: {denominator}")
    return 100.0 * numerator / denominator


def derive_seed_metrics(
    arm: dict[str, float],
    census: dict[str, float],
    variant: str,
    seed: int,
) -> dict[str, float]:
    offered_units = scalar(census, "units.total", CENSUS_VARIANT, seed)
    offered_value = sum_scalars(census, TOTAL_VALUE_COMPONENTS, CENSUS_VARIANT, seed)
    offered_urgent_units = scalar(census, "inclusion.urgent.submitted", CENSUS_VARIANT, seed)
    offered_urgent_value = sum_scalars(census, URGENT_VALUE_COMPONENTS, CENSUS_VARIANT, seed)

    submitted_units = scalar(arm, "units.total", variant, seed)
    retained = scalar(arm, "value.retainedLovelace", variant, seed)
    lost = scalar(arm, "value.lostLovelace", variant, seed)
    unresolved = scalar(arm, "value.unresolvedLovelace", variant, seed)
    submitted_value = retained + lost + unresolved
    urgent_units = scalar(arm, "inclusion.urgent.submitted", variant, seed)
    urgent_retained = scalar(arm, "value.urgent.retainedLovelace", variant, seed)
    urgent_lost = scalar(arm, "value.urgent.lostLovelace", variant, seed)
    urgent_unresolved = scalar(arm, "value.urgent.unresolvedLovelace", variant, seed)
    urgent_submitted_value = urgent_retained + urgent_lost + urgent_unresolved
    realised_fees = scalar(arm, "revenue.feesCollectedLovelace", variant, seed) - scalar(
        arm, "revenue.refundsPaidLovelace", variant, seed
    )

    tolerance = 0.5
    require(
        submitted_units <= offered_units + tolerance,
        f"{variant}, seed {seed}: submitted units exceed census offered units",
    )
    require(
        submitted_value <= offered_value + tolerance,
        f"{variant}, seed {seed}: submitted value exceeds census offered value",
    )
    require(
        urgent_units <= offered_urgent_units + tolerance,
        f"{variant}, seed {seed}: urgent submitted units exceed census offered units",
    )
    require(
        urgent_submitted_value <= offered_urgent_value + tolerance,
        f"{variant}, seed {seed}: urgent submitted value exceeds census offered value",
    )

    g = 1_000_000_000.0
    return {
        "units.submitted": submitted_units,
        "units.submittedShare": ratio(submitted_units, offered_units, f"{variant} units, seed {seed}"),
        "units.initialDeclined": offered_units - submitted_units,
        "value.submitted": submitted_value / g,
        "value.submittedShare": ratio(submitted_value, offered_value, f"{variant} value, seed {seed}"),
        "value.initialDeclined": (offered_value - submitted_value) / g,
        "value.retained": retained / g,
        "value.lost": lost / g,
        "value.unresolved": unresolved / g,
        "value.nonurgent.retained": (retained - urgent_retained) / g,
        "value.retainedOffered": ratio(retained, offered_value, f"{variant} retained/offered, seed {seed}"),
        "value.retainedConditional": 100.0 * scalar(arm, "value.retainedRatio", variant, seed),
        "fees.realised": realised_fees / g,
        "value.userNet": (retained - realised_fees) / g,
        "urgent.units.submitted": urgent_units,
        "urgent.units.initialDeclined": offered_urgent_units - urgent_units,
        "urgent.value.submitted": urgent_submitted_value / g,
        "urgent.value.initialDeclined": (offered_urgent_value - urgent_submitted_value) / g,
        "urgent.value.retained": urgent_retained / g,
        "urgent.value.lost": urgent_lost / g,
        "urgent.value.unresolved": urgent_unresolved / g,
        "urgent.value.retainedOffered": ratio(urgent_retained, offered_urgent_value, f"{variant} urgent retained/offered, seed {seed}"),
        "urgent.value.retainedConditional": 100.0 * scalar(arm, "value.urgent.retainedRatio", variant, seed),
        "urgent.latency.meanBlocks": scalar(arm, "latency.urgent.meanBlocks", variant, seed),
        "urgent.latency.p95Blocks": scalar(arm, "latency.urgent.p95Blocks", variant, seed),
        "throughput.txPerSlot": scalar(arm, "throughput.txPerSlot", variant, seed),
        "throughput.ebUtilization": 100.0 * scalar(arm, "throughput.ebUtilization", variant, seed),
        "price.shockCount": scalar(arm, "price.shockCount", variant, seed),
    }


def summarize_series(values: list[float]) -> dict[str, Any]:
    return {"mean": sum(values) / len(values), "perSeed": values}


def paired_summary(candidate: list[float], reference: list[float]) -> dict[str, Any]:
    differences = [left - right for left, right in zip(candidate, reference)]
    mean, low, high = paired_interval(differences)
    return {
        "meanDifference": mean,
        "ci95": [low, high],
        "candidateHigherCount": sum(1 for value in differences if value > 0),
        "perSeedDifference": differences,
    }


def build_report(
    root: Path,
    manifest: Path,
    simulator_sha256: str | None,
    load_name: str,
    stage: str = "screen",
) -> dict[str, Any]:
    summary_path = root / "summary.json"
    summary = load_summary(summary_path)
    require(summary.get("summaryOnly") is True, "screen must be summary-only")
    require(summary.get("randomness") == "independent-streams", "screen must use independent-streams randomness")
    require(summary.get("loadOverride", {}).get("type") == "profile", "screen must use an explicit load profile")
    require(
        summary.get("loadProfile", {}).get("name") == load_name,
        f"screen must use the {load_name} profile",
    )

    summary_variant_names = {v.get("name") for v in summary["variants"]}
    candidates = [name for name in CANDIDATE_VARIANTS if name in summary_variant_names]
    require(
        CANONICAL_VARIANT in candidates,
        "the canonical-s50-u50 arm must be present for vs-canonical pairing",
    )
    names = [CENSUS_VARIANT, FLAT_VARIANT, *candidates]
    runs = {name: variant_runs(summary, name, summary_path) for name in names}
    seed_sets = {name: set(by_seed) for name, by_seed in runs.items()}
    expected_seeds = set(runs[CENSUS_VARIANT])
    require(len(expected_seeds) >= 2, "at least two paired seeds are required")
    require(
        all(seeds == expected_seeds for seeds in seed_sets.values()),
        "all arms must contain identical seed sets",
    )
    require(
        len(expected_seeds) == summary.get("seeds"),
        f"summary declares {summary.get('seeds')} seeds but contains {len(expected_seeds)}",
    )
    declared_seed_start = summary.get("seedStart", 0)
    require(
        expected_seeds
        == set(range(declared_seed_start, declared_seed_start + summary["seeds"])),
        "summary seed set does not match its recorded seedStart and seed count",
    )
    seeds = sorted(expected_seeds)

    config_hashes = validate_effective_configs(root, candidates)
    arm_names = [FLAT_VARIANT, *candidates]
    per_arm: dict[str, dict[str, list[float]]] = {}
    for name in arm_names:
        derived = [
            derive_seed_metrics(runs[name][seed], runs[CENSUS_VARIANT][seed], name, seed)
            for seed in seeds
        ]
        per_arm[name] = {
            metric.key: [row[metric.key] for row in derived] for metric in METRICS
        }

    offered_units = [scalar(runs[CENSUS_VARIANT][seed], "units.total", CENSUS_VARIANT, seed) for seed in seeds]
    offered_value = [sum_scalars(runs[CENSUS_VARIANT][seed], TOTAL_VALUE_COMPONENTS, CENSUS_VARIANT, seed) / 1_000_000_000.0 for seed in seeds]
    offered_urgent_units = [scalar(runs[CENSUS_VARIANT][seed], "inclusion.urgent.submitted", CENSUS_VARIANT, seed) for seed in seeds]
    offered_urgent_value = [sum_scalars(runs[CENSUS_VARIANT][seed], URGENT_VALUE_COMPONENTS, CENSUS_VARIANT, seed) / 1_000_000_000.0 for seed in seeds]

    report_metrics = [
        metric
        for metric in METRICS
        if load_name == "severe-congestion"
        or (
            not metric.key.startswith("urgent.")
            and metric.key != "value.nonurgent.retained"
        )
    ]
    results: dict[str, Any] = {}
    for name in arm_names:
        metrics: dict[str, Any] = {}
        for metric in report_metrics:
            values = per_arm[name][metric.key]
            entry = summarize_series(values)
            if name != FLAT_VARIANT:
                entry["vsFlatFee"] = paired_summary(values, per_arm[FLAT_VARIANT][metric.key])
            if name not in (FLAT_VARIANT, CANONICAL_VARIANT):
                entry["vsCanonical"] = paired_summary(values, per_arm[CANONICAL_VARIANT][metric.key])
            metrics[metric.key] = entry
        results[name] = {
            "label": VARIANT_LABELS.get(name, "flat fee"),
            "metrics": metrics,
        }

    provenance: dict[str, Any] = {
        "summary": project_relative(summary_path),
        "summarySha256": sha256_file(summary_path),
        "manifest": project_relative(manifest),
        "manifestSha256": sha256_file(manifest),
        "selectedLoadProfileSha256": sha256_file(root / "selected-load-profile.json"),
        "effectiveConfigSha256": config_hashes,
    }
    if simulator_sha256:
        provenance["simulatorSha256"] = simulator_sha256

    offered_demand: dict[str, Any] = {
        "units": summarize_series(offered_units),
        "valueG": summarize_series(offered_value),
    }
    if load_name == "severe-congestion":
        offered_demand.update(
            {
                "urgentUnits": summarize_series(offered_urgent_units),
                "urgentValueG": summarize_series(offered_urgent_value),
            }
        )

    if stage == "confirm":
        description = f"Confirmatory paired rerun of the pre-selected independent standard-lane target under {load_name}. Deltas use two-sided 95% paired-t intervals and are conditional on this simulator calibration."
        role = "confirmation of the pre-selected arm on a disjoint seed range"
    else:
        description = f"Exploratory paired screen of independent standard-lane targets under {load_name}. Deltas use two-sided 95% paired-t intervals and are conditional on this simulator calibration."
        role = "candidate screening; use held-out seeds for confirmation"
    return {
        "description": description,
        "method": {
            "randomness": summary.get("randomness"),
            "seeds": seeds,
            "seedStart": declared_seed_start,
            "slots": summary.get("slots"),
            "load": load_name,
            "stage": stage,
            "candidates": candidates,
            "role": role,
            "demandCensus": "patient fixed-fee shadow arm with aligned fresh-demand stream; only its offered count/value totals are used",
        },
        "offeredDemand": offered_demand,
        "metrics": {
            metric.key: {
                "label": metric.label,
                "unit": metric.unit,
                "digits": metric.digits,
            }
            for metric in report_metrics
        },
        "results": results,
        "provenance": provenance,
    }


def signed(value: float, digits: int) -> str:
    rounded = round(value, digits)
    if rounded == 0:
        rounded = 0.0
    return f"{rounded:+,.{digits}f}"


def plain(value: float, digits: int) -> str:
    return f"{value:,.{digits}f}"


def delta_cell(row: dict[str, Any], digits: int) -> str:
    delta = row["vsFlatFee"]
    low, high = delta["ci95"]
    return f"{signed(delta['meanDifference'], digits)} [{signed(low, digits)}, {signed(high, digits)}]"


def canonical_delta_cell(row: dict[str, Any], digits: int) -> str:
    delta = row.get("vsCanonical")
    if delta is None:
        return "—"
    low, high = delta["ci95"]
    return f"{signed(delta['meanDifference'], digits)} [{signed(low, digits)}, {signed(high, digits)}]"


def render_markdown(report: dict[str, Any]) -> str:
    method = report["method"]
    offered = report["offeredDemand"]
    results = report["results"]
    n = len(method["seeds"])
    load_name = method["load"]
    stage = method.get("stage", "screen")
    candidates = method.get("candidates", CANDIDATE_VARIANTS)
    load_label = "Severe congestion" if load_name == "severe-congestion" else "Launch day"
    offered_line = (
        f"Mean offered demand: **{offered['units']['mean']:,.1f} units**, "
        f"**{offered['valueG']['mean']:,.3f} G lovelace**."
    )
    if load_name == "severe-congestion":
        offered_line = (
            offered_line[:-1]
            + f"; urgent class: **{offered['urgentUnits']['mean']:,.1f} units**, "
            + f"**{offered['urgentValueG']['mean']:,.3f} G lovelace**."
        )
    if stage == "confirm":
        title = f"# Independent standard-target confirmation: {load_label.lower()}"
        intro = (
            f"Confirmatory rerun over {n} paired seeds and {method['slots']} slots using "
            "independent fresh-demand, block-production, and retry streams. Deltas are "
            "candidate minus flat fee with two-sided 95% paired-t confidence intervals."
        )
    else:
        title = f"# Independent standard-target screen: {load_label.lower()}"
        intro = (
            f"Exploratory screen over {n} paired seeds and {method['slots']} slots using "
            "independent fresh-demand, block-production, and retry streams. Deltas are "
            "candidate minus flat fee with two-sided 95% paired-t confidence intervals."
        )
    lines = [
        title,
        "",
        intro,
        "",
        "The patient shadow arm records the common generated-demand census. Its own "
        "service outcomes are not treated as a candidate.",
        "",
        offered_line,
        "",
        "## Overall entry and retained value",
        "",
        "| Candidate | Retained (G) | Δ retained vs flat (95% CI) | Δ retained vs canonical (95% CI) | Submitted units | Δ units vs flat (95% CI) | Initial-declined value (G) | Retained / offered | Conditional ratio |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for name in candidates:
        metrics = results[name]["metrics"]
        lines.append(
            f"| {VARIANT_LABELS[name]} "
            f"| {plain(metrics['value.retained']['mean'], 3)} "
            f"| {delta_cell(metrics['value.retained'], 3)} "
            f"| {canonical_delta_cell(metrics['value.retained'], 3)} "
            f"| {plain(metrics['units.submitted']['mean'], 1)} "
            f"| {delta_cell(metrics['units.submitted'], 1)} "
            f"| {plain(metrics['value.initialDeclined']['mean'], 3)} "
            f"| {plain(metrics['value.retainedOffered']['mean'], 3)}% "
            f"| {plain(metrics['value.retainedConditional']['mean'], 3)}% |"
        )

    if load_name == "severe-congestion":
        lines.extend(
            [
                "",
                "## Urgent-class outcome",
                "",
                "| Candidate | Urgent retained (G) | Δ retained vs flat (95% CI) | Conditional retained | Mean delay (blocks) | Δ mean delay | p95 delay (blocks) | Δ p95 delay |",
                "|---|---:|---:|---:|---:|---:|---:|---:|",
            ]
        )
        for name in candidates:
            metrics = results[name]["metrics"]
            lines.append(
                f"| {VARIANT_LABELS[name]} "
                f"| {plain(metrics['urgent.value.retained']['mean'], 3)} "
                f"| {delta_cell(metrics['urgent.value.retained'], 3)} "
                f"| {plain(metrics['urgent.value.retainedConditional']['mean'], 3)}% "
                f"| {plain(metrics['urgent.latency.meanBlocks']['mean'], 3)} "
                f"| {delta_cell(metrics['urgent.latency.meanBlocks'], 3)} "
                f"| {plain(metrics['urgent.latency.p95Blocks']['mean'], 3)} "
                f"| {delta_cell(metrics['urgent.latency.p95Blocks'], 3)} |"
            )
    else:
        lines.extend(
            [
                "",
                "Urgent-class summary rows are intentionally omitted for launch day: "
                "the load's time-varying urgency multipliers fragment the simulator's "
                "urgency bands. Overall offered, submitted, and retained values remain valid.",
            ]
        )

    lines.extend(
        [
            "",
            "## Fee and load diagnostics",
            "",
            "| Candidate | Retained − realised fees (G) | Δ vs flat (95% CI) | Realised fees (G) | EB utilisation | Price shocks |",
            "|---|---:|---:|---:|---:|---:|",
        ]
    )
    for name in candidates:
        metrics = results[name]["metrics"]
        lines.append(
            f"| {VARIANT_LABELS[name]} "
            f"| {plain(metrics['value.userNet']['mean'], 3)} "
            f"| {delta_cell(metrics['value.userNet'], 3)} "
            f"| {plain(metrics['fees.realised']['mean'], 3)} "
            f"| {plain(metrics['throughput.ebUtilization']['mean'], 3)}% "
            f"| {plain(metrics['price.shockCount']['mean'], 2)} |"
        )

    if stage == "confirm":
        closing = (
            "This is the confirmatory rerun of the pre-selected arm against flat fee "
            "and canonical on a disjoint seed range. The selection rule and this arm "
            "were fixed before these seeds were inspected."
        )
    else:
        closing = (
            "This is a candidate-selection screen, not confirmatory evidence. Select "
            "the decision rule before inspecting held-out seeds, then rerun only the "
            "selected arm against flat fee and canonical on a disjoint seed range."
        )
    lines.extend(
        [
            "",
            closing,
            "",
            "`retained - realised fees` is user-side model value. Gross retained value "
            "is the appropriate total if fees are treated entirely as transfers; neither "
            "quantity is an externally calibrated welfare estimate.",
            "",
        ]
    )
    return "\n".join(lines)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True, help="single-load sweep output")
    parser.add_argument(
        "--load-name",
        choices=("severe-congestion", "launch-day"),
        default="severe-congestion",
    )
    parser.add_argument(
        "--manifest",
        type=Path,
        default=PROJECT_DIR / "config/sweeps/standard-target-screen.json",
    )
    parser.add_argument("--simulator-sha256", default=None)
    parser.add_argument(
        "--stage",
        choices=("screen", "confirm"),
        default="screen",
        help="screen: exploratory candidate screen; confirm: confirmatory rerun of the pre-selected arm",
    )
    parser.add_argument("--markdown-output", type=Path, required=True)
    parser.add_argument("--json-output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    report = build_report(args.root, args.manifest, args.simulator_sha256, args.load_name, args.stage)
    args.json_output.parent.mkdir(parents=True, exist_ok=True)
    args.json_output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    args.markdown_output.parent.mkdir(parents=True, exist_ok=True)
    args.markdown_output.write_text(render_markdown(report), encoding="utf-8")
    print(f"wrote {args.json_output}")
    print(f"wrote {args.markdown_output}")


if __name__ == "__main__":
    main()
