#!/usr/bin/env python3
"""Compare the four-arm default-threshold ablation across five loads."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Iterable

from compare_canonical_headlines import mean, source_provenance
from compare_cross_lane_inversion_smoke import (
    interval_text,
    load_summary,
    numeric_scalar,
    paired_interval,
    variant_runs,
)


PROJECT_DIR = Path(__file__).resolve().parent.parent
THRESHOLD_PATH = ("design", "reservationPolicy", "ebThresholdBytes")

VARIANTS = {
    "threshold-1": "config/variants/default-threshold/threshold-1.json",
    "threshold-quarter-rb": (
        "config/variants/default-threshold/threshold-quarter-rb.json"
    ),
    "threshold-half-rb": "config/variants/trickle-aging/thr-k10.json",
    "threshold-three-quarter-rb": (
        "config/variants/default-threshold/threshold-three-quarter-rb.json"
    ),
}
THRESHOLD_BYTES = {
    "threshold-1": 1,
    "threshold-quarter-rb": 22_528,
    "threshold-half-rb": 45_056,
    "threshold-three-quarter-rb": 67_584,
}
HALF_VARIANT = "threshold-half-rb"

LOADS = (
    ("low", "Low"),
    ("mid-load", "Mid load"),
    ("severe-congestion", "Severe congestion"),
    ("eb-capacity-stress", "EB-capacity stress"),
    ("launch-day", "Launch day"),
)
URGENT_LOADS = {directory for directory, _label in LOADS[:-1]}

CONTRASTS = (
    ("half-minus-1", HALF_VARIANT, "threshold-1", True),
    ("half-minus-quarter", HALF_VARIANT, "threshold-quarter-rb", False),
    (
        "half-minus-three-quarter",
        HALF_VARIANT,
        "threshold-three-quarter-rb",
        False,
    ),
)

URGENT_METRICS = (
    ("value.urgent.retainedLovelace", "Urgent retained value", "lovelace"),
    ("value.urgent.retainedRatio", "Urgent retained-value ratio", "ratio"),
    ("latency.urgent.meanBlocks", "Urgent mean latency", "blocks"),
    ("latency.standard.meanBlocks", "Standard mean latency", "blocks"),
)
LAUNCH_DAY_METRICS = (
    ("value.retainedLovelace", "Overall retained value", "lovelace"),
    ("value.retainedRatio", "Overall retained-value ratio", "ratio"),
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


def differing_paths(
    left: Any, right: Any, path: tuple[str, ...] = ()
) -> set[tuple[str, ...]]:
    """Return leaf paths whose JSON values differ, including type differences."""
    if type(left) is not type(right):
        return {path}
    if isinstance(left, dict):
        differences: set[tuple[str, ...]] = set()
        for key in left.keys() | right.keys():
            child_path = path + (str(key),)
            if key not in left or key not in right:
                differences.add(child_path)
            else:
                differences.update(differing_paths(left[key], right[key], child_path))
        return differences
    if isinstance(left, list):
        differences = set()
        for index in range(max(len(left), len(right))):
            child_path = path + (str(index),)
            if index >= len(left) or index >= len(right):
                differences.add(child_path)
            else:
                differences.update(
                    differing_paths(left[index], right[index], child_path)
                )
        return differences
    return set() if left == right else {path}


def nested_value(value: dict[str, Any], path: Iterable[str], source: Path) -> Any:
    current: Any = value
    traversed: list[str] = []
    for component in path:
        traversed.append(component)
        if not isinstance(current, dict) or component not in current:
            raise ValueError(f"missing {'.'.join(traversed)} in {source}")
        current = current[component]
    return current


def expected_load_override(directory: str) -> dict[str, str]:
    if directory == "low":
        return {"name": "low", "type": "preset"}
    return {
        "copy": "selected-load-profile.json",
        "name": directory,
        "source": f"config/loads/{directory}.json",
        "type": "profile",
    }


def validate_effective_configs(directory: Path) -> dict[str, str]:
    paths = {
        variant: directory / f"{variant}.config.json" for variant in VARIANTS
    }
    configs = {variant: json_object(path) for variant, path in paths.items()}
    half = configs[HALF_VARIANT]

    for variant, config in configs.items():
        threshold = nested_value(config, THRESHOLD_PATH, paths[variant])
        expected_threshold = THRESHOLD_BYTES[variant]
        if (
            isinstance(threshold, bool)
            or not isinstance(threshold, int)
            or threshold != expected_threshold
        ):
            raise ValueError(
                f"unexpected {'.'.join(THRESHOLD_PATH)} in {paths[variant]}: "
                f"expected={expected_threshold}, actual={threshold!r}"
            )

        differences = differing_paths(config, half)
        expected_differences = set() if variant == HALF_VARIANT else {THRESHOLD_PATH}
        if differences != expected_differences:
            rendered = [".".join(path) or "<root>" for path in sorted(differences)]
            expected = [
                ".".join(path) or "<root>" for path in sorted(expected_differences)
            ]
            raise ValueError(
                f"unexpected effective-config differences for {variant!r} in "
                f"{directory}: expected={expected!r}, actual={rendered!r}"
            )

    return {variant: str(path) for variant, path in paths.items()}


def load_runs(
    root: Path, directory: str
) -> tuple[
    Path,
    dict[str, dict[int, dict[str, float]]],
    int,
    dict[str, str],
]:
    path = root / directory / "summary.json"
    summary = load_summary(path)

    variants = summary["variants"]
    actual_configs = {
        variant.get("name"): variant.get("config") for variant in variants
    }
    if len(actual_configs) != len(variants) or actual_configs != VARIANTS:
        raise ValueError(
            f"unexpected variants/configs in {path}: "
            f"expected={VARIANTS!r}, actual={actual_configs!r}"
        )

    actual_load_override = summary.get("loadOverride")
    expected_override = expected_load_override(directory)
    if actual_load_override != expected_override:
        raise ValueError(
            f"unexpected load override in {path}: "
            f"expected={expected_override!r}, actual={actual_load_override!r}"
        )
    if summary.get("summaryOnly") is not True:
        raise ValueError(f"threshold ablation was expected to be summary-only: {path}")
    if summary.get("randomness") != "independent-streams":
        raise ValueError(f"threshold ablation requires independent RNG streams: {path}")

    slots = summary.get("slots")
    if isinstance(slots, bool) or not isinstance(slots, int) or slots < 1:
        raise ValueError(f"missing/invalid slot count in {path}")

    runs = {
        variant: variant_runs(summary, variant, path) for variant in VARIANTS
    }
    seed_sets = {variant: set(per_seed) for variant, per_seed in runs.items()}
    first_seed_set = next(iter(seed_sets.values()))
    if any(seeds != first_seed_set for seeds in seed_sets.values()):
        rendered = {
            variant: sorted(seeds) for variant, seeds in seed_sets.items()
        }
        raise ValueError(
            f"paired comparison requires identical seed sets in {path}: {rendered!r}"
        )
    if len(first_seed_set) < 2:
        raise ValueError(f"paired comparison requires at least two seeds in {path}")
    declared_seeds = summary.get("seeds")
    if (
        isinstance(declared_seeds, bool)
        or not isinstance(declared_seeds, int)
        or declared_seeds != len(first_seed_set)
    ):
        raise ValueError(
            f"declared seed count does not match runs in {path}: "
            f"declared={declared_seeds!r}, runs={len(first_seed_set)}"
        )

    effective_configs = validate_effective_configs(root / directory)
    return path, runs, slots, effective_configs


def metric_result(
    key: str,
    label: str,
    unit: str,
    candidate_name: str,
    reference_name: str,
    candidate: dict[int, dict[str, float]],
    reference: dict[int, dict[str, float]],
    seeds: list[int],
) -> dict[str, Any]:
    candidate_values = [
        numeric_scalar(candidate[seed], key, candidate_name, seed) for seed in seeds
    ]
    reference_values = [
        numeric_scalar(reference[seed], key, reference_name, seed) for seed in seeds
    ]
    estimate, low, high = paired_interval(
        candidate_value - reference_value
        for candidate_value, reference_value in zip(
            candidate_values, reference_values
        )
    )
    return {
        "key": key,
        "label": label,
        "unit": unit,
        "candidate_mean": mean(candidate_values),
        "reference_mean": mean(reference_values),
        "candidate_minus_reference": {
            "mean": estimate,
            "ci95_low": low,
            "ci95_high": high,
        },
    }


def contrast_result(
    directory: str,
    name: str,
    candidate_name: str,
    reference_name: str,
    primary: bool,
    runs: dict[str, dict[int, dict[str, float]]],
    seeds: list[int],
) -> dict[str, Any]:
    candidate = runs[candidate_name]
    reference = runs[reference_name]
    metrics = URGENT_METRICS if directory in URGENT_LOADS else LAUNCH_DAY_METRICS
    return {
        "name": name,
        "primary": primary,
        "candidate": candidate_name,
        "reference": reference_name,
        "metrics": [
            metric_result(
                key,
                label,
                unit,
                candidate_name,
                reference_name,
                candidate,
                reference,
                seeds,
            )
            for key, label, unit in metrics
        ],
        "exactly_equal_all_scalars": sum(
            candidate[seed] == reference[seed] for seed in seeds
        ),
        "paired_seeds": len(seeds),
    }


def build_report(root: Path, simulator_sha256: str) -> dict[str, Any]:
    loaded: dict[str, dict[str, dict[int, dict[str, float]]]] = {}
    sources: dict[str, dict[str, Any]] = {}
    slot_counts: set[int] = set()
    seed_sets: set[tuple[int, ...]] = set()

    for directory, _label in LOADS:
        path, runs, slots, effective_configs = load_runs(root, directory)
        loaded[directory] = runs
        sources[directory] = {
            "summary": str(path),
            "effective_configs": effective_configs,
        }
        slot_counts.add(slots)
        seed_sets.add(tuple(sorted(runs[HALF_VARIANT])))

    if len(slot_counts) != 1:
        raise ValueError(f"loads use different slot counts: {sorted(slot_counts)}")
    if len(seed_sets) != 1:
        raise ValueError("loads use different seed sets")

    seeds = list(next(iter(seed_sets)))
    results = []
    for directory, label in LOADS:
        results.append(
            {
                "load": directory,
                "label": label,
                "retained_value_scope": (
                    "urgent" if directory in URGENT_LOADS else "overall"
                ),
                "contrasts": [
                    contrast_result(
                        directory,
                        name,
                        candidate,
                        reference,
                        primary,
                        loaded[directory],
                        seeds,
                    )
                    for name, candidate, reference, primary in CONTRASTS
                ],
            }
        )

    provenance = source_provenance(root / "source.patch")
    provenance["simulator_sha256"] = simulator_sha256
    return {
        "method": (
            "paired mean difference with two-sided 95% Student-t confidence interval"
        ),
        "difference_direction": "threshold-half-rb minus reference",
        "variants": {
            variant: {
                "config": config,
                "eb_threshold_bytes": THRESHOLD_BYTES[variant],
            }
            for variant, config in VARIANTS.items()
        },
        "seeds": seeds,
        "slots": next(iter(slot_counts)),
        "sources": sources,
        "provenance": provenance,
        "loads": results,
    }


def signed_g_lovelace(value: float) -> str:
    scaled = round(value / 1.0e9, 3)
    if scaled == 0:
        scaled = 0.0
    return f"{scaled:+.3f}"


def result_text(result: dict[str, float], unit: str) -> str:
    if unit == "lovelace":
        return (
            f"{signed_g_lovelace(result['mean'])} "
            f"[{signed_g_lovelace(result['ci95_low'])}, "
            f"{signed_g_lovelace(result['ci95_high'])}]"
        )
    return interval_text(result, 3)


def seed_text(seeds: list[int]) -> str:
    if seeds == list(range(seeds[0], seeds[-1] + 1)):
        return f"{seeds[0]}–{seeds[-1]}"
    return ", ".join(str(seed) for seed in seeds)


def markdown(report: dict[str, Any]) -> str:
    seeds = report["seeds"]
    provenance = report["provenance"]
    revision = provenance["git_revision"] or "unknown"
    clean = provenance["abstract_sim_worktree_clean"]
    source_state = "clean" if clean is True else "dirty" if clean is False else "unknown"
    source_line = (
        f"Source revision: `{revision}`; `abstract-sim-hs` worktree: "
        f"{source_state}."
    )
    dirty_patch = provenance.get("dirty_patch")
    if dirty_patch:
        source_line += (
            f" Dirty source: `{dirty_patch['path']}` "
            f"(SHA-256 `{dirty_patch['sha256']}`)."
        )
    source_line += f" Simulator SHA-256: `{provenance['simulator_sha256']}`."

    lines = [
        "# Default-threshold ablation",
        "",
        (
            f"Half-RB threshold minus each comparison arm; paired seeds "
            f"{seed_text(seeds)} (n={len(seeds)}), {report['slots']:,} slots each. "
            "Cells are mean differences with two-sided 95% paired-t CIs."
        ),
        "",
        source_line,
        "",
        (
            "| Load | Contrast | Retained value (G lovelace) | Urgent latency "
            "(blocks) | Standard latency (blocks) | All scalars exactly equal |"
        ),
        "|---|---|---:|---:|---:|---:|",
    ]

    for load in report["loads"]:
        for contrast in load["contrasts"]:
            by_key = {metric["key"]: metric for metric in contrast["metrics"]}
            retained_key = (
                "value.urgent.retainedLovelace"
                if load["retained_value_scope"] == "urgent"
                else "value.retainedLovelace"
            )
            retained = result_text(
                by_key[retained_key]["candidate_minus_reference"], "lovelace"
            )
            if load["retained_value_scope"] == "urgent":
                urgent_latency = result_text(
                    by_key["latency.urgent.meanBlocks"]["candidate_minus_reference"],
                    "blocks",
                )
                standard_latency = result_text(
                    by_key["latency.standard.meanBlocks"][
                        "candidate_minus_reference"
                    ],
                    "blocks",
                )
            else:
                urgent_latency = "—"
                standard_latency = "—"
            contrast_label = contrast["name"]
            if contrast["primary"]:
                contrast_label += " (primary)"
            lines.append(
                f"| {load['label']} | {contrast_label} | {retained} | "
                f"{urgent_latency} | {standard_latency} | "
                f"{contrast['exactly_equal_all_scalars']}/{contrast['paired_seeds']} |"
            )

    lines.extend(
        [
            "",
            (
                "Retained value is urgent-class for the first four loads and overall "
                "for launch day. Positive retained-value differences favour the "
                "half-RB threshold; negative latency differences are faster. The "
                "equality column compares every recorded scalar for each paired seed."
            ),
            "",
            (
                "Low and mid load are decision-facing. The three heavier loads are "
                "no-regression checks; without a prespecified equivalence margin, "
                "they cannot establish equivalence."
            ),
            "",
            (
                "Independent streams align exogenous demand and ranking-block draws "
                "within each paired seed. At target 0.5, the two branches of the "
                "threshold max() expression both give half an RB, so this sweep cannot "
                "tell which branch should control away from the default target."
            ),
            "",
        ]
    )
    return "\n".join(lines)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root", required=True, help="directory containing the five load subdirectories"
    )
    parser.add_argument(
        "--simulator-sha256",
        required=True,
        help="SHA-256 of the executable used for every run",
    )
    parser.add_argument("--markdown-output", required=True, help="Markdown report path")
    parser.add_argument("--json-output", required=True, help="machine-readable report path")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        simulator_sha256 = args.simulator_sha256.lower()
        if len(simulator_sha256) != 64 or any(
            character not in "0123456789abcdef" for character in simulator_sha256
        ):
            raise ValueError("--simulator-sha256 must be 64 hexadecimal characters")
        report = build_report(Path(args.root), simulator_sha256)
        markdown_text = markdown(report)
        Path(args.markdown_output).write_text(markdown_text, encoding="utf-8")
        Path(args.json_output).write_text(
            json.dumps(report, indent=2) + "\n", encoding="utf-8"
        )
    except (OSError, ValueError) as error:
        raise SystemExit(f"error: {error}") from error
    print(markdown_text, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
