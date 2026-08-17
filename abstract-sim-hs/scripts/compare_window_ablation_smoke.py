#!/usr/bin/env python3
"""Compare the four-arm controller-window ablation across five loads.

The committed experiment is deliberately small: five paired seeds make it a
directional smoke test, not an equivalence test or a parameter-selection run.
The report therefore keeps absolute retained value and submission volume next
to conditional ratios, latency, throughput, retries, and price oscillation.
"""

from __future__ import annotations

import argparse
import copy
import json
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from compare_canonical_headlines import source_provenance
from compare_cross_lane_inversion_smoke import paired_interval


PROJECT_DIR = Path(__file__).resolve().parent.parent
DEFAULT_ROOT = PROJECT_DIR / "sweep-results/window-ablation-smoke"
DEFAULT_MANIFEST = PROJECT_DIR / "config/sweeps/window-ablation-smoke.json"

CURRENT = "current-w20-w5"
STANDARD_INSTANT = "standard-instant-w5"
URGENT_INSTANT = "w20-urgent-instant"
BOTH_INSTANT = "both-instant"

VARIANTS = {
    CURRENT: "config/variants/standard-target-screen/s75-u50.json",
    STANDARD_INSTANT: "config/variants/window-ablation/standard-instant-w5.json",
    URGENT_INSTANT: "config/variants/window-ablation/w20-urgent-instant.json",
    BOTH_INSTANT: "config/variants/window-ablation/both-instant.json",
}

VARIANT_LABELS = {
    CURRENT: "Current W20/W5",
    STANDARD_INSTANT: "Standard instant / urgent W5",
    URGENT_INSTANT: "Standard W20 / urgent instant",
    BOTH_INSTANT: "Both instant",
}

STANDARD_WINDOW = {"type": "capacity-weighted-window", "window": 20}
STANDARD_CURRENT_PRODUCTION = "capacity-weighted-util"
URGENT_WINDOW = {"type": "priority-reservation-window", "window": 5}
URGENT_CURRENT_PRODUCTION = "priority-reservation-util"

EXPECTED_SIGNALS = {
    CURRENT: (STANDARD_WINDOW, URGENT_WINDOW),
    STANDARD_INSTANT: (STANDARD_CURRENT_PRODUCTION, URGENT_WINDOW),
    URGENT_INSTANT: (STANDARD_WINDOW, URGENT_CURRENT_PRODUCTION),
    BOTH_INSTANT: (STANDARD_CURRENT_PRODUCTION, URGENT_CURRENT_PRODUCTION),
}

STANDARD_SIGNAL_PATH = (
    "design",
    "controllers",
    "standardController",
    "signal",
)
URGENT_SIGNAL_PATH = (
    "design",
    "controllers",
    "priorityController",
    "signal",
)

LOADS = (
    ("low", "Low"),
    ("mid-load", "Mid load"),
    ("severe-congestion", "Severe congestion"),
    ("eb-capacity-stress", "EB-capacity stress"),
    ("launch-day", "Launch day"),
)

CONTRASTS = (
    (
        "both-instant-minus-current",
        "Both instant − current",
        BOTH_INSTANT,
        CURRENT,
        True,
    ),
    (
        "standard-instant-minus-current",
        "Standard instant only − current",
        STANDARD_INSTANT,
        CURRENT,
        False,
    ),
    (
        "urgent-instant-minus-current",
        "Urgent instant only − current",
        URGENT_INSTANT,
        CURRENT,
        False,
    ),
)


@dataclass(frozen=True)
class Metric:
    key: str
    label: str
    unit: str
    scale: float
    digits: int
    preference: str | None


METRIC_GROUPS = (
    (
        "absolute",
        "Absolute retained value and submitted demand",
        (
            Metric(
                "value.retainedLovelace",
                "Overall retained value",
                "G lovelace",
                1e-9,
                3,
                "higher",
            ),
            Metric(
                "value.standard.retainedLovelace",
                "Standard-lane retained value",
                "G lovelace",
                1e-9,
                3,
                "higher",
            ),
            Metric(
                "value.priority.retainedLovelace",
                "Priority-lane retained value",
                "G lovelace",
                1e-9,
                3,
                "higher",
            ),
            Metric(
                "value.urgent.retainedLovelace",
                "Urgent-class retained value",
                "G lovelace",
                1e-9,
                3,
                "higher",
            ),
            Metric("units.total", "Observed submitted demand", "units", 1, 1, None),
            Metric(
                "inclusion.standard.submitted",
                "Standard-lane submissions",
                "transactions",
                1,
                1,
                None,
            ),
            Metric(
                "inclusion.priority.submitted",
                "Priority-lane submissions",
                "transactions",
                1,
                1,
                None,
            ),
            Metric(
                "inclusion.urgent.submitted",
                "Urgent-class submissions",
                "transactions",
                1,
                1,
                None,
            ),
        ),
    ),
    (
        "conditional",
        "Conditional retained-value and service ratios",
        (
            Metric(
                "value.retainedRatio",
                "Overall retained-value ratio",
                "percentage points",
                100,
                3,
                "higher",
            ),
            Metric(
                "value.standard.retainedRatio",
                "Standard-lane retained-value ratio",
                "percentage points",
                100,
                3,
                "higher",
            ),
            Metric(
                "value.priority.retainedRatio",
                "Priority-lane retained-value ratio",
                "percentage points",
                100,
                3,
                "higher",
            ),
            Metric(
                "value.urgent.retainedRatio",
                "Urgent-class retained-value ratio",
                "percentage points",
                100,
                3,
                "higher",
            ),
            Metric(
                "inclusion.standard.serviceRate",
                "Standard-lane service rate",
                "percentage points",
                100,
                3,
                "higher",
            ),
            Metric(
                "inclusion.priority.serviceRate",
                "Priority-lane service rate",
                "percentage points",
                100,
                3,
                "higher",
            ),
            Metric(
                "inclusion.urgent.serviceRate",
                "Urgent-class service rate",
                "percentage points",
                100,
                3,
                "higher",
            ),
        ),
    ),
    (
        "latency",
        "Lane and urgent-class latency",
        (
            Metric(
                "latency.standard.meanBlocks",
                "Standard-lane mean wait",
                "blocks",
                1,
                3,
                "lower",
            ),
            Metric(
                "latency.standard.p95Blocks",
                "Standard-lane p95 wait",
                "blocks",
                1,
                3,
                "lower",
            ),
            Metric(
                "latency.priority.meanBlocks",
                "Priority-lane mean wait",
                "blocks",
                1,
                3,
                "lower",
            ),
            Metric(
                "latency.priority.p95Blocks",
                "Priority-lane p95 wait",
                "blocks",
                1,
                3,
                "lower",
            ),
            Metric(
                "latency.urgent.meanBlocks",
                "Urgent-class mean wait",
                "blocks",
                1,
                3,
                "lower",
            ),
            Metric(
                "latency.urgent.p95Blocks",
                "Urgent-class p95 wait",
                "blocks",
                1,
                3,
                "lower",
            ),
        ),
    ),
    (
        "throughput-retries",
        "Throughput, capacity, and retries",
        (
            Metric(
                "throughput.txPerSlot",
                "Throughput",
                "tx/slot",
                1,
                3,
                "higher",
            ),
            Metric(
                "throughput.ebUtilization",
                "EB utilisation",
                "percentage points",
                100,
                3,
                None,
            ),
            Metric(
                "load.amplification",
                "Retry load amplification",
                "attempts/unit",
                1,
                3,
                "lower",
            ),
        ),
    ),
    (
        "oscillation",
        "Combined-lane price oscillation",
        (
            Metric(
                "price.oscillationReversalCount",
                "Oscillation reversals",
                "reversals",
                1,
                2,
                "lower",
            ),
            Metric(
                "price.oscillationCycleCount",
                "Oscillation cycles",
                "cycles",
                1,
                2,
                "lower",
            ),
            Metric(
                "price.oscillationExcessTravel",
                "Oscillation excess log travel",
                "log coefficient",
                1,
                3,
                "lower",
            ),
            Metric(
                "price.oscillationMaxAmplitude",
                "Maximum oscillation amplitude",
                "coefficient",
                1,
                3,
                "lower",
            ),
            Metric(
                "price.settledCoefficientRange",
                "Settled coefficient range",
                "coefficient",
                1,
                3,
                "lower",
            ),
        ),
    ),
)


def json_object(path: Path) -> dict[str, Any]:
    try:
        with path.open("r", encoding="utf-8") as handle:
            value = json.load(handle)
    except FileNotFoundError as error:
        raise ValueError(f"file does not exist: {path}") from error
    except json.JSONDecodeError as error:
        raise ValueError(f"invalid JSON in {path}: {error}") from error
    if not isinstance(value, dict):
        raise ValueError(f"expected a JSON object in {path}")
    return value


def nested_value(value: dict[str, Any], path: tuple[str, ...], source: Path) -> Any:
    current: Any = value
    traversed: list[str] = []
    for component in path:
        traversed.append(component)
        if not isinstance(current, dict) or component not in current:
            raise ValueError(f"missing {'.'.join(traversed)} in {source}")
        current = current[component]
    return current


def replace_nested(value: dict[str, Any], path: tuple[str, ...], replacement: Any) -> None:
    current: Any = value
    for component in path[:-1]:
        if not isinstance(current, dict) or component not in current:
            raise ValueError(f"missing {'.'.join(path)} while normalising config")
        current = current[component]
    if not isinstance(current, dict) or path[-1] not in current:
        raise ValueError(f"missing {'.'.join(path)} while normalising config")
    current[path[-1]] = replacement


def differing_paths(
    left: Any, right: Any, path: tuple[str, ...] = ()
) -> set[tuple[str, ...]]:
    if type(left) is not type(right):
        return {path}
    if isinstance(left, dict):
        differences: set[tuple[str, ...]] = set()
        for key in left.keys() | right.keys():
            child = path + (str(key),)
            if key not in left or key not in right:
                differences.add(child)
            else:
                differences.update(differing_paths(left[key], right[key], child))
        return differences
    if isinstance(left, list):
        differences = set()
        for index in range(max(len(left), len(right))):
            child = path + (str(index),)
            if index >= len(left) or index >= len(right):
                differences.add(child)
            else:
                differences.update(differing_paths(left[index], right[index], child))
        return differences
    return set() if left == right else {path}


def validate_manifest(path: Path) -> dict[str, Any]:
    manifest = json_object(path)
    if manifest.get("randomness") != "independent-streams":
        raise ValueError(f"manifest must use independent-streams randomness: {path}")
    if manifest.get("summaryOnly") is not True:
        raise ValueError(f"manifest must set summaryOnly=true: {path}")
    if manifest.get("seedStart") != 300:
        raise ValueError(f"manifest must default to seed 300: {path}")
    if manifest.get("seeds") != 5 or manifest.get("slots") != 2_000:
        raise ValueError(f"manifest must default to five seeds and 2,000 slots: {path}")

    variants = manifest.get("variants")
    if not isinstance(variants, list):
        raise ValueError(f"manifest is missing its variants array: {path}")
    actual: dict[str, Any] = {}
    for variant in variants:
        if not isinstance(variant, dict) or not isinstance(variant.get("name"), str):
            raise ValueError(f"malformed manifest variant in {path}")
        name = variant["name"]
        if name in actual:
            raise ValueError(f"duplicate manifest variant {name!r} in {path}")
        actual[name] = variant.get("config")
    if actual != VARIANTS:
        raise ValueError(
            f"unexpected variants/configs in {path}: expected={VARIANTS!r}, actual={actual!r}"
        )
    return manifest


def expected_load_override(directory: str) -> dict[str, str]:
    if directory == "low":
        return {"name": "low", "type": "preset"}
    return {
        "copy": "selected-load-profile.json",
        "name": directory,
        "source": f"config/loads/{directory}.json",
        "type": "profile",
    }


def variant_runs(
    summary: dict[str, Any], name: str, source: Path
) -> dict[int, dict[str, float]]:
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
            raise ValueError(f"malformed run for {name!r} in {source}")
        if seed in runs:
            raise ValueError(f"duplicate seed {seed} for {name!r} in {source}")
        runs[seed] = scalars
    if not runs:
        raise ValueError(f"variant {name!r} has no runs in {source}")
    return runs


def numeric_scalar(
    scalars: dict[str, Any], key: str, variant: str, seed: int
) -> float:
    value = scalars.get(key)
    if (
        isinstance(value, bool)
        or not isinstance(value, (int, float))
        or not math.isfinite(value)
    ):
        raise ValueError(f"missing/non-finite scalar {key!r} for {variant}, seed {seed}")
    return float(value)


def validate_effective_configs(directory: Path) -> dict[str, str]:
    paths = {name: directory / f"{name}.config.json" for name in VARIANTS}
    configs = {name: json_object(path) for name, path in paths.items()}

    for name, config in configs.items():
        standard_signal = nested_value(config, STANDARD_SIGNAL_PATH, paths[name])
        urgent_signal = nested_value(config, URGENT_SIGNAL_PATH, paths[name])
        expected_standard, expected_urgent = EXPECTED_SIGNALS[name]
        if standard_signal != expected_standard or urgent_signal != expected_urgent:
            raise ValueError(
                f"unexpected controller signals in {paths[name]}: "
                f"expected={(expected_standard, expected_urgent)!r}, "
                f"actual={(standard_signal, urgent_signal)!r}"
            )

    current = configs[CURRENT]
    fixed_expectations = {
        ("design", "controllers", "standardController", "targetUtilisation"): 0.75,
        ("design", "controllers", "standardController", "maxChangeDenominator"): 16,
        ("design", "controllers", "priorityController", "targetUtilisation"): 0.5,
        ("design", "controllers", "priorityController", "maxChangeDenominator"): 16,
        ("design", "reservationPolicy", "ebThresholdBytes"): 45_056,
        ("design", "reservationPolicy", "ebAgeEscapeRbIntervals"): 10,
    }
    for path, expected in fixed_expectations.items():
        actual = nested_value(current, path, paths[CURRENT])
        if actual != expected:
            raise ValueError(
                f"unexpected {'.'.join(path)} in {paths[CURRENT]}: "
                f"expected={expected!r}, actual={actual!r}"
            )

    sentinel = "<controller-signal>"
    normalised: dict[str, dict[str, Any]] = {}
    for name, config in configs.items():
        value = copy.deepcopy(config)
        replace_nested(value, STANDARD_SIGNAL_PATH, sentinel)
        replace_nested(value, URGENT_SIGNAL_PATH, sentinel)
        normalised[name] = value
    reference = normalised[CURRENT]
    for name, config in normalised.items():
        differences = differing_paths(config, reference)
        if differences:
            rendered = [".".join(path) or "<root>" for path in sorted(differences)]
            raise ValueError(
                f"effective config {paths[name]} differs outside the two signal fields: "
                f"{rendered!r}"
            )
    return {name: str(path) for name, path in paths.items()}


def load_runs(
    root: Path, directory: str
) -> tuple[
    Path,
    dict[str, dict[int, dict[str, float]]],
    int,
    dict[str, str],
]:
    path = root / directory / "summary.json"
    summary = json_object(path)
    if not isinstance(summary.get("variants"), list):
        raise ValueError(f"not a sweep summary (missing variants array): {path}")
    if summary.get("randomness") != "independent-streams":
        raise ValueError(f"summary must use independent-streams randomness: {path}")
    if summary.get("summaryOnly") is not True:
        raise ValueError(f"summary must be summary-only: {path}")
    if summary.get("loadOverride") != expected_load_override(directory):
        raise ValueError(
            f"unexpected load override in {path}: {summary.get('loadOverride')!r}"
        )
    if directory != "low":
        profile = summary.get("loadProfile")
        if not isinstance(profile, dict) or profile.get("name") != directory:
            raise ValueError(f"missing/wrong selected load profile in {path}")

    actual_configs = {
        variant.get("name"): variant.get("config") for variant in summary["variants"]
    }
    if actual_configs != VARIANTS:
        raise ValueError(
            f"unexpected variants/configs in {path}: "
            f"expected={VARIANTS!r}, actual={actual_configs!r}"
        )

    runs = {name: variant_runs(summary, name, path) for name in VARIANTS}
    seed_sets = {name: set(by_seed) for name, by_seed in runs.items()}
    reference_seeds = seed_sets[CURRENT]
    if any(seeds != reference_seeds for seeds in seed_sets.values()):
        rendered = {name: sorted(seeds) for name, seeds in seed_sets.items()}
        raise ValueError(f"paired comparison requires identical seed sets in {path}: {rendered}")
    if len(reference_seeds) < 2:
        raise ValueError(f"paired comparison requires at least two seeds in {path}")
    if summary.get("seeds") != len(reference_seeds):
        raise ValueError(
            f"summary seed metadata disagrees with runs in {path}: "
            f"metadata={summary.get('seeds')!r}, runs={len(reference_seeds)}"
        )

    slots = summary.get("slots")
    if isinstance(slots, bool) or not isinstance(slots, int) or slots < 1:
        raise ValueError(f"invalid slot count in {path}: {slots!r}")

    scalar_keys: set[str] | None = None
    for name, by_seed in runs.items():
        for seed, scalars in by_seed.items():
            keys = set(scalars)
            if scalar_keys is None:
                scalar_keys = keys
            elif keys != scalar_keys:
                raise ValueError(
                    f"scalar-key mismatch for {name}, seed {seed} in {path}"
                )

    effective_configs = validate_effective_configs(root / directory)
    return path, runs, slots, effective_configs


def mean(values: list[float]) -> float:
    if not values:
        raise ValueError("cannot calculate a mean from no values")
    return sum(values) / len(values)


def metric_result(
    metric: Metric,
    runs: dict[str, dict[int, dict[str, float]]],
    seeds: list[int],
) -> dict[str, Any]:
    values = {
        name: [
            metric.scale * numeric_scalar(by_seed[seed], metric.key, name, seed)
            for seed in seeds
        ]
        for name, by_seed in runs.items()
    }
    contrasts: dict[str, Any] = {}
    for contrast_id, _label, candidate, reference, primary in CONTRASTS:
        differences = [
            candidate_value - reference_value
            for candidate_value, reference_value in zip(
                values[candidate], values[reference]
            )
        ]
        estimate, low, high = paired_interval(differences)
        contrasts[contrast_id] = {
            "candidate": candidate,
            "reference": reference,
            "primary": primary,
            "mean": estimate,
            "ci95_low": low,
            "ci95_high": high,
            "candidate_higher": sum(value > 0 for value in differences),
            "reference_higher": sum(value < 0 for value in differences),
            "ties": sum(value == 0 for value in differences),
        }
    return {
        "key": metric.key,
        "label": metric.label,
        "unit": metric.unit,
        "digits": metric.digits,
        "preference": metric.preference,
        "arm_means": {name: mean(arm_values) for name, arm_values in values.items()},
        "contrasts": contrasts,
    }


def metric_is_usable_for_load(metric: Metric, load: str) -> bool:
    # The launch profile changes the urgency-rate multiplier over time, so the
    # summary export's single urgent-class slice is not a stable class there.
    return load != "launch-day" or ".urgent." not in metric.key


def exactly_equal_count(
    candidate: dict[int, dict[str, float]],
    reference: dict[int, dict[str, float]],
    seeds: list[int],
) -> int:
    return sum(candidate[seed] == reference[seed] for seed in seeds)


def build_report(
    root: Path, manifest_path: Path, simulator_sha256: str
) -> dict[str, Any]:
    validate_manifest(manifest_path)
    load_data: list[dict[str, Any]] = []
    common_seeds: tuple[int, ...] | None = None
    common_slots: int | None = None

    for directory, label in LOADS:
        summary_path, runs, slots, effective_configs = load_runs(root, directory)
        seeds = tuple(sorted(runs[CURRENT]))
        if common_seeds is None:
            common_seeds = seeds
        elif seeds != common_seeds:
            raise ValueError(
                f"loads use different paired seed sets: first={list(common_seeds)}, "
                f"{directory}={list(seeds)}"
            )
        if common_slots is None:
            common_slots = slots
        elif slots != common_slots:
            raise ValueError(
                f"loads use different slot counts: first={common_slots}, "
                f"{directory}={slots}"
            )

        seed_list = list(seeds)
        groups = [
            {
                "id": group_id,
                "label": group_label,
                "metrics": [
                    metric_result(metric, runs, seed_list)
                    for metric in metrics
                    if metric_is_usable_for_load(metric, directory)
                ],
            }
            for group_id, group_label, metrics in METRIC_GROUPS
        ]
        equalities = {
            contrast_id: exactly_equal_count(
                runs[candidate], runs[reference], seed_list
            )
            for contrast_id, _contrast_label, candidate, reference, _primary in CONTRASTS
        }
        load_data.append(
            {
                "load": directory,
                "label": label,
                "source": {
                    "summary": str(summary_path),
                    "effective_configs": effective_configs,
                },
                "paired_seeds": len(seed_list),
                "exactly_equal_all_scalars": equalities,
                "metric_groups": groups,
            }
        )

    if common_seeds is None or common_slots is None:
        raise ValueError("no load results found")
    provenance = source_provenance(root / "source.patch")
    provenance["simulator_sha256"] = simulator_sha256
    return {
        "schema_version": 1,
        "experiment": "controller-window-ablation-smoke",
        "method": "paired mean difference with two-sided 95% Student-t confidence interval",
        "evidence_scope": (
            "Exploratory directional smoke test. The committed default is five paired "
            "seeds; this run is not powered to establish equivalence, select protocol "
            "parameters, or show that an unobserved practically important effect is absent."
        ),
        "difference_direction": "instantaneous signal minus current W20/W5 signal",
        "manifest": str(manifest_path),
        "variants": {
            name: {
                "label": VARIANT_LABELS[name],
                "config": path,
                "standard_signal": EXPECTED_SIGNALS[name][0],
                "urgent_signal": EXPECTED_SIGNALS[name][1],
            }
            for name, path in VARIANTS.items()
        },
        "contrasts": {
            contrast_id: {
                "label": label,
                "candidate": candidate,
                "reference": reference,
                "primary": primary,
            }
            for contrast_id, label, candidate, reference, primary in CONTRASTS
        },
        "seeds": list(common_seeds),
        "slots": common_slots,
        "provenance": provenance,
        "loads": load_data,
        "notes": [
            (
                "Independent streams align fresh-demand samples and Ranking Block "
                "opportunities within a seed; retry jitter remains mechanism-dependent."
            ),
            (
                "Observed demand and submission counts cover demand that reached a first "
                "submission, not every generated demand sample. Absolute retained values "
                "must therefore be read beside the conditional retained-value ratios."
            ),
            (
                "The standard instantaneous arm aggregates the current block-producing "
                "slot's payload and capacity summaries. It is not a one-summary rolling "
                "window, which could end on a payload-free EB announcement."
            ),
            (
                "Price-oscillation scalars combine both lanes. The single-signal arms "
                "isolate which lane changed, while the primary both-instant contrast does not."
            ),
            (
                "Launch-day urgent-class slices are omitted because that profile changes "
                "the urgency-rate multiplier over time; priority-lane metrics remain usable."
            ),
        ],
    }


def signed(value: float, digits: int) -> str:
    rounded = round(value, digits)
    if rounded == 0:
        rounded = 0.0
    return f"{rounded:+.{digits}f}"


def interval_text(result: dict[str, Any], digits: int) -> str:
    return (
        f"{signed(result['mean'], digits)} "
        f"[{signed(result['ci95_low'], digits)}, "
        f"{signed(result['ci95_high'], digits)}]"
    )


def plain(value: float, digits: int) -> str:
    return f"{value:.{digits}f}"


def markdown(report: dict[str, Any]) -> str:
    seeds = report["seeds"]
    provenance = report["provenance"]
    revision = provenance.get("git_revision") or "unknown"
    source_state = (
        "clean"
        if provenance.get("abstract_sim_worktree_clean") is True
        else "dirty"
        if provenance.get("abstract_sim_worktree_clean") is False
        else "unknown"
    )
    source_line = (
        f"Source revision: `{revision}`; `abstract-sim-hs` worktree: {source_state}; "
        f"simulator SHA-256: `{provenance['simulator_sha256']}`."
    )
    dirty_patch = provenance.get("dirty_patch")
    if dirty_patch:
        source_line += (
            f" Dirty-source patch: `{dirty_patch['path']}` "
            f"(SHA-256 `{dirty_patch['sha256']}`)."
        )

    if seeds == list(range(seeds[0], seeds[-1] + 1)):
        seed_text = f"{seeds[0]}–{seeds[-1]}"
    else:
        seed_text = ", ".join(str(seed) for seed in seeds)
    lines = [
        "# Controller-window ablation smoke",
        "",
        (
            f"Current W20/W5 versus instantaneous controller signals; paired seeds "
            f"{seed_text} (n={len(seeds)}), {report['slots']:,} slots per load. "
            "Cells are candidate-minus-current mean differences with two-sided 95% "
            "paired-t confidence intervals."
        ),
        "",
        (
            "**Exploratory/directional only.** The committed design uses five seeds. "
            "This smoke run cannot establish equivalence, select a protocol default, "
            "or show that a practically important effect is absent."
        ),
        "",
        source_line,
    ]

    contrast_ids = [contrast_id for contrast_id, *_rest in CONTRASTS]
    for load in report["loads"]:
        lines.extend(["", f"## {load['label']}"])
        equality = load["exactly_equal_all_scalars"]
        lines.extend(
            [
                "",
                (
                    "Exact all-scalar ties: "
                    f"both instant {equality[contrast_ids[0]]}/{load['paired_seeds']}; "
                    f"standard instant only {equality[contrast_ids[1]]}/{load['paired_seeds']}; "
                    f"urgent instant only {equality[contrast_ids[2]]}/{load['paired_seeds']}."
                ),
            ]
        )
        for group in load["metric_groups"]:
            lines.extend(
                [
                    "",
                    f"### {group['label']}",
                    "",
                    (
                        "| Metric | Unit | Current mean | Both-instant mean | "
                        "Both instant − current | Standard instant only − current | "
                        "Urgent instant only − current |"
                    ),
                    "|---|---:|---:|---:|---:|---:|---:|",
                ]
            )
            for metric in group["metrics"]:
                results = metric["contrasts"]
                lines.append(
                    f"| {metric['label']} | {metric['unit']} "
                    f"| {plain(metric['arm_means'][CURRENT], metric['digits'])} "
                    f"| {plain(metric['arm_means'][BOTH_INSTANT], metric['digits'])} "
                    f"| {interval_text(results[contrast_ids[0]], metric['digits'])} "
                    f"| {interval_text(results[contrast_ids[1]], metric['digits'])} "
                    f"| {interval_text(results[contrast_ids[2]], metric['digits'])} |"
                )

    lines.extend(
        [
            "",
            "## Interpretation notes",
            "",
            *[f"- {note}" for note in report["notes"]],
            "- Positive retained-value, service-rate, and throughput differences are favourable; negative latency, retry, and oscillation differences are favourable.",
            "- Percentage-point rows are absolute changes in ratios, not relative percentages.",
            "",
        ]
    )
    return "\n".join(lines)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=DEFAULT_ROOT,
        help="directory containing the five load subdirectories",
    )
    parser.add_argument(
        "--manifest",
        type=Path,
        default=DEFAULT_MANIFEST,
        help="committed sweep manifest",
    )
    parser.add_argument(
        "--simulator-sha256",
        required=True,
        help="SHA-256 of the executable used for every run",
    )
    parser.add_argument(
        "--markdown-output",
        type=Path,
        help="Markdown report path (default: ROOT/comparison.md)",
    )
    parser.add_argument(
        "--json-output",
        type=Path,
        help="machine-readable report path (default: ROOT/comparison.json)",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    simulator_sha256 = args.simulator_sha256.lower()
    if len(simulator_sha256) != 64 or any(
        character not in "0123456789abcdef" for character in simulator_sha256
    ):
        raise SystemExit("error: --simulator-sha256 must be 64 hexadecimal characters")

    markdown_output = args.markdown_output or args.root / "comparison.md"
    json_output = args.json_output or args.root / "comparison.json"
    try:
        report = build_report(args.root, args.manifest, simulator_sha256)
        rendered = markdown(report)
        markdown_output.write_text(rendered, encoding="utf-8")
        json_output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    except (OSError, ValueError) as error:
        raise SystemExit(f"error: {error}") from error
    print(f"wrote {markdown_output}")
    print(f"wrote {json_output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
