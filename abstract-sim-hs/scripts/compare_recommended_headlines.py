#!/usr/bin/env python3
"""Compare flat fee with the adopted recommended construction across the five
headline loads.

The recommended construction is the configuration the CIP specifies: the 0.75
standard target, the 10-block processed-block standard window, and the
age-escape reset at certification. The window length and the reset rule were
selected by the standard-window screen (seeds 0-99), bounded by the
announcement-semantics smoke (seeds 440-449), and confirmed on disjoint seeds
(700-799). This refresh regenerates the headline mechanism-versus-flat-fee
figures under that configuration.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from compare_canonical_headlines import source_provenance
from compare_cross_lane_inversion_smoke import paired_interval
from compare_window_ablation_smoke import (
    METRIC_GROUPS,
    Metric,
    interval_text,
    json_object,
    mean,
    metric_is_usable_for_load,
    nested_value,
    numeric_scalar,
    plain,
    variant_runs,
)

PROJECT_DIR = Path(__file__).resolve().parent.parent
DEFAULT_ROOT = PROJECT_DIR / "sweep-results/recommended-headlines"
DEFAULT_MANIFEST = PROJECT_DIR / "config/sweeps/recommended-headlines.json"

FLAT_FEE = "flat-fee"
RECOMMENDED = "recommended"

VARIANTS = {
    FLAT_FEE: "config/variants/flat-fee.json",
    RECOMMENDED: "config/variants/standard-window-confirm/block-window-10-cert-reset.json",
}

VARIANT_LABELS = {
    FLAT_FEE: "Flat fee",
    RECOMMENDED: "Recommended construction",
}

CONTRAST_ID = "recommended-minus-flat-fee"

LOADS = (
    ("low", "Low"),
    ("mid-load", "Mid load"),
    ("severe-congestion", "Severe congestion"),
    ("eb-capacity-stress", "EB-capacity stress"),
    ("launch-day", "Launch day"),
)


def expected_override(directory: str) -> dict[str, str]:
    if directory == "low":
        return {"name": "low", "type": "preset"}
    return {
        "copy": "selected-load-profile.json",
        "name": directory,
        "source": f"config/loads/{directory}.json",
        "type": "profile",
    }


def validate_manifest(path: Path) -> dict[str, Any]:
    manifest = json_object(path)
    if manifest.get("randomness") != "independent-streams":
        raise ValueError(f"manifest must use independent-streams randomness: {path}")
    if manifest.get("summaryOnly") is not True:
        raise ValueError(f"manifest must set summaryOnly=true: {path}")
    if manifest.get("seeds") != 10 or manifest.get("slots") != 2_000:
        raise ValueError(f"manifest must default to ten seeds and 2,000 slots: {path}")

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


def validate_effective_configs(directory: Path) -> dict[str, str]:
    paths = {name: directory / f"{name}.config.json" for name in VARIANTS}
    configs = {name: json_object(path) for name, path in paths.items()}

    flat_expectations = {
        ("design", "laneStructure"): "one",
        ("design", "reservationPolicy", "type"): "no-reservation",
        ("design", "feeSemantics"): "fixed-fee",
    }
    for path, expected in flat_expectations.items():
        actual = nested_value(configs[FLAT_FEE], path, paths[FLAT_FEE])
        if actual != expected:
            raise ValueError(
                f"unexpected {'.'.join(path)} in {paths[FLAT_FEE]}: "
                f"expected={expected!r}, actual={actual!r}"
            )

    recommended_expectations = {
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
        ("design", "controllers", "priorityController", "maxChangeDenominator"): 16,
        ("design", "reservationPolicy", "ebThresholdBytes"): 45_056,
        ("design", "reservationPolicy", "ebAgeEscapeRbIntervals"): 10,
        ("design", "ageResetAtCertification"): True,
    }
    for path, expected in recommended_expectations.items():
        actual = nested_value(configs[RECOMMENDED], path, paths[RECOMMENDED])
        if actual != expected:
            raise ValueError(
                f"unexpected {'.'.join(path)} in {paths[RECOMMENDED]}: "
                f"expected={expected!r}, actual={actual!r}"
            )
    return {name: str(path) for name, path in paths.items()}


def load_runs(
    root: Path, directory: str
) -> tuple[Path, dict[str, dict[int, dict[str, float]]], int, dict[str, str]]:
    path = root / directory / "summary.json"
    summary = json_object(path)
    if not isinstance(summary.get("variants"), list):
        raise ValueError(f"not a sweep summary (missing variants array): {path}")
    if summary.get("randomness") != "independent-streams":
        raise ValueError(f"summary must use independent-streams randomness: {path}")
    if summary.get("summaryOnly") is not True:
        raise ValueError(f"summary must be summary-only: {path}")
    if summary.get("loadOverride") != expected_override(directory):
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
    reference_seeds = seed_sets[FLAT_FEE]
    if any(seeds != reference_seeds for seeds in seed_sets.values()):
        rendered = {name: sorted(seeds) for name, seeds in seed_sets.items()}
        raise ValueError(
            f"paired comparison requires identical seed sets in {path}: {rendered}"
        )
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
                raise ValueError(f"scalar-key mismatch for {name}, seed {seed} in {path}")

    effective_configs = validate_effective_configs(root / directory)
    return path, runs, slots, effective_configs


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
    differences = [
        recommended_value - flat_value
        for recommended_value, flat_value in zip(values[RECOMMENDED], values[FLAT_FEE])
    ]
    estimate, low, high = paired_interval(differences)
    return {
        "key": metric.key,
        "label": metric.label,
        "unit": metric.unit,
        "digits": metric.digits,
        "preference": metric.preference,
        "arm_means": {name: mean(arm_values) for name, arm_values in values.items()},
        "contrasts": {
            CONTRAST_ID: {
                "candidate": RECOMMENDED,
                "reference": FLAT_FEE,
                "primary": True,
                "mean": estimate,
                "ci95_low": low,
                "ci95_high": high,
                "candidate_higher": sum(value > 0 for value in differences),
                "reference_higher": sum(value < 0 for value in differences),
                "ties": sum(value == 0 for value in differences),
            }
        },
    }


def build_report(
    root: Path, manifest_path: Path, simulator_sha256: str
) -> dict[str, Any]:
    validate_manifest(manifest_path)
    load_data: list[dict[str, Any]] = []
    common_seeds: tuple[int, ...] | None = None
    common_slots: int | None = None

    for directory, label in LOADS:
        summary_path, runs, slots, effective_configs = load_runs(root, directory)
        seeds = tuple(sorted(runs[FLAT_FEE]))
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
        load_data.append(
            {
                "load": directory,
                "label": label,
                "source": {
                    "summary": str(summary_path),
                    "effective_configs": effective_configs,
                },
                "paired_seeds": len(seed_list),
                "metric_groups": groups,
            }
        )

    if common_seeds is None or common_slots is None:
        raise ValueError("no load results found")
    provenance = source_provenance(root / "source.patch")
    provenance["simulator_sha256"] = simulator_sha256
    return {
        "schema_version": 1,
        "experiment": "recommended-headlines",
        "method": "paired mean difference with two-sided 95% Student-t confidence interval",
        "evidence_scope": (
            "Headline refresh under the adopted recommended construction. Ten paired "
            "seeds match the original headline design. The thousand-seed replication "
            "bounds seed-sampling error separately at low and severe-congestion load; "
            "its record for this configuration is thousand-seed-low-severe-recommended.json."
        ),
        "difference_direction": "recommended construction minus flat fee",
        "manifest": str(manifest_path),
        "variants": {
            name: {"label": VARIANT_LABELS[name], "config": path}
            for name, path in VARIANTS.items()
        },
        "contrasts": {
            CONTRAST_ID: {
                "label": "Recommended construction − flat fee",
                "candidate": RECOMMENDED,
                "reference": FLAT_FEE,
                "primary": True,
            }
        },
        "seeds": list(common_seeds),
        "slots": common_slots,
        "provenance": provenance,
        "loads": load_data,
        "notes": [
            (
                "The recommended construction here is the full adopted configuration: "
                "0.75 standard target, 0.5 urgent target, D = 16, half-RB threshold, "
                "K = 10, the 10-block processed-block standard window, and the "
                "age-escape reset at certification."
            ),
            (
                "The window length and the reset rule were selected on seeds 0-99, "
                "bounded on seeds 440-449, and confirmed on seeds 700-799. This "
                "refresh reuses the original headline seeds 0-9, which overlap the "
                "selecting screen's seed range; the disjoint-seed confirmation is the "
                "guard against selection bias, not this refresh."
            ),
            (
                "Independent streams align fresh-demand samples and Ranking Block "
                "opportunities within a seed; retry jitter remains mechanism-dependent."
            ),
            (
                "Launch-day urgent-class slices are omitted because that profile changes "
                "the urgency-rate multiplier over time; priority-lane metrics remain usable."
            ),
        ],
    }


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
        "# Recommended-construction headline refresh",
        "",
        (
            f"Flat fee versus the adopted recommended construction; paired seeds "
            f"{seed_text} (n={len(seeds)}), {report['slots']:,} slots per load. "
            "Cells are recommended-minus-flat-fee mean differences with two-sided "
            "95% paired-t confidence intervals."
        ),
        "",
        source_line,
    ]

    for load in report["loads"]:
        lines.extend(["", f"## {load['label']}"])
        for group in load["metric_groups"]:
            lines.extend(
                [
                    "",
                    f"### {group['label']}",
                    "",
                    "| Metric | Unit | Flat-fee mean | Recommended mean | Recommended − flat fee |",
                    "|---|---:|---:|---:|---:|",
                ]
            )
            for metric in group["metrics"]:
                result = metric["contrasts"][CONTRAST_ID]
                lines.append(
                    f"| {metric['label']} | {metric['unit']} "
                    f"| {plain(metric['arm_means'][FLAT_FEE], metric['digits'])} "
                    f"| {plain(metric['arm_means'][RECOMMENDED], metric['digits'])} "
                    f"| {interval_text(result, metric['digits'])} |"
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
