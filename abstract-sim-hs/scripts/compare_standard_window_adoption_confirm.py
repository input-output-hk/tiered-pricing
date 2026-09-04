#!/usr/bin/env python3
"""Confirm the standard-window selection under the adoption criteria.

The candidate arm is the exact configuration the CIP will specify: the
standard controller's utilisation window taken over the 10 most recent
processed-block summaries (announcements excluded), and the EB age-escape
count reset at certified-EB application rather than at announcement. It runs
against the historical announcement-diluted 20-entry window and flat fee.

Adoption requires the enforceable configuration to match the historical
behaviour and to keep the mechanism's margin over flat fee. It does not
require superiority over the historical behaviour, so superiority is
reported as a declared secondary and is not gated.

Pre-declared pass criteria, all of which must hold (conjunctive gating, so
no multiplicity adjustment is applied):

  M1  severe congestion, EB-capacity stress, and launch day:
      candidate-minus-historical overall retained-value ratio has a
      95% CI lower bound above -0.05 percentage points
  M2  trickle: candidate-minus-historical standard-lane retained-value
      ratio has a 95% CI lower bound above -1.0 percentage points
  M3  severe congestion, EB-capacity stress, and launch day:
      candidate-minus-flat-fee overall retained value has a 95% CI above
      zero

Declared secondaries, reported and never gated:

  S1  launch day: candidate-minus-historical overall retained value with
      its 95% CI and seed-pair win count

Low and mid-load deltas and exact-tie counts are reported in the per-load
tables. Mid-load bit-identity to historical is expected, not gated.
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
    differing_paths,
    exactly_equal_count,
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
DEFAULT_ROOT = PROJECT_DIR / "sweep-results/standard-window-adoption-confirm"
DEFAULT_MANIFEST = PROJECT_DIR / "config/sweeps/standard-window-adoption-confirm.json"

HISTORICAL = "historical"
FLAT_FEE = "flat-fee"
CANDIDATE = "block-window-10-cert-reset"

VARIANTS = {
    HISTORICAL: "config/variants/standard-target-screen/s75-u50.json",
    FLAT_FEE: "config/variants/flat-fee.json",
    CANDIDATE: "config/variants/standard-window-confirm/block-window-10-cert-reset.json",
}

VARIANT_LABELS = {
    HISTORICAL: "Historical announcement-diluted window (20 entries)",
    FLAT_FEE: "Flat fee",
    CANDIDATE: "Specified semantics (10-block window, cert reset)",
}

SIGNAL_TYPE_PATH = ("design", "controllers", "standardController", "signal", "type")
SIGNAL_WINDOW_PATH = ("design", "controllers", "standardController", "signal", "window")
AGE_RESET_PATH = ("design", "ageResetAtCertification")

CONTRASTS = {
    "candidate-minus-historical": {
        "candidate": CANDIDATE,
        "reference": HISTORICAL,
        "primary": True,
    },
    "candidate-minus-flat-fee": {
        "candidate": CANDIDATE,
        "reference": FLAT_FEE,
        "primary": False,
    },
    "historical-minus-flat-fee": {
        "candidate": HISTORICAL,
        "reference": FLAT_FEE,
        "primary": False,
    },
}

LOADS = (
    ("low", "Low"),
    ("mid-load", "Mid load"),
    ("severe-congestion", "Severe congestion"),
    ("eb-capacity-stress", "EB-capacity stress"),
    ("launch-day", "Launch day"),
    ("trickle-0p1", "Trickle 0.1 tx/slot"),
)

MATCHING_LOADS = ("severe-congestion", "eb-capacity-stress", "launch-day")

# The trickle profile's embedded name uses a dot where its filename uses "p".
PROFILE_NAMES = {directory: directory for directory, _ in LOADS if directory != "low"}
PROFILE_NAMES["trickle-0p1"] = "trickle-0.1"


def expected_override(directory: str) -> dict[str, str]:
    if directory == "low":
        return {"name": "low", "type": "preset"}
    return {
        "copy": "selected-load-profile.json",
        "name": PROFILE_NAMES[directory],
        "source": f"config/loads/{directory}.json",
        "type": "profile",
    }


def validate_manifest(path: Path) -> dict[str, Any]:
    manifest = json_object(path)
    if manifest.get("randomness") != "independent-streams":
        raise ValueError(f"manifest must use independent-streams randomness: {path}")
    if manifest.get("summaryOnly") is not True:
        raise ValueError(f"manifest must set summaryOnly=true: {path}")
    if manifest.get("seedStart") != 700:
        raise ValueError(f"manifest must default to seed 700: {path}")
    if manifest.get("seeds") != 100 or manifest.get("slots") != 2_000:
        raise ValueError(f"manifest must default to 100 seeds and 2,000 slots: {path}")

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

    historical_signal = nested_value(configs[HISTORICAL], SIGNAL_TYPE_PATH, paths[HISTORICAL])
    if historical_signal != "capacity-weighted-window":
        raise ValueError(
            f"historical arm must use the capacity-weighted-window signal in "
            f"{paths[HISTORICAL]}: actual={historical_signal!r}"
        )
    historical_window = nested_value(configs[HISTORICAL], SIGNAL_WINDOW_PATH, paths[HISTORICAL])
    if historical_window != 20:
        raise ValueError(
            f"historical arm must use a 20-entry window in {paths[HISTORICAL]}: "
            f"actual={historical_window!r}"
        )
    historical_design = nested_value(configs[HISTORICAL], ("design",), paths[HISTORICAL])
    historical_reset = historical_design.get(AGE_RESET_PATH[-1], False)
    if historical_reset is not False:
        raise ValueError(
            f"historical arm must reset at announcement (absent or false) in "
            f"{paths[HISTORICAL]}: actual={historical_reset!r}"
        )

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

    fixed_expectations = {
        ("design", "controllers", "standardController", "targetUtilisation"): 0.75,
        ("design", "controllers", "standardController", "maxChangeDenominator"): 16,
        ("design", "controllers", "priorityController", "targetUtilisation"): 0.5,
        ("design", "controllers", "priorityController", "maxChangeDenominator"): 16,
        ("design", "reservationPolicy", "ebThresholdBytes"): 45_056,
        ("design", "reservationPolicy", "ebAgeEscapeRbIntervals"): 10,
    }
    for name in (HISTORICAL, CANDIDATE):
        for path, expected in fixed_expectations.items():
            actual = nested_value(configs[name], path, paths[name])
            if actual != expected:
                raise ValueError(
                    f"unexpected {'.'.join(path)} in {paths[name]}: "
                    f"expected={expected!r}, actual={actual!r}"
                )

    candidate_signal = nested_value(configs[CANDIDATE], SIGNAL_TYPE_PATH, paths[CANDIDATE])
    if candidate_signal != "capacity-weighted-block-window":
        raise ValueError(
            f"candidate arm must use the capacity-weighted-block-window signal in "
            f"{paths[CANDIDATE]}: actual={candidate_signal!r}"
        )
    candidate_window = nested_value(configs[CANDIDATE], SIGNAL_WINDOW_PATH, paths[CANDIDATE])
    if candidate_window != 10:
        raise ValueError(
            f"candidate arm must use a 10-block window in {paths[CANDIDATE]}: "
            f"actual={candidate_window!r}"
        )
    candidate_reset = nested_value(configs[CANDIDATE], AGE_RESET_PATH, paths[CANDIDATE])
    if candidate_reset is not True:
        raise ValueError(
            f"candidate arm must set ageResetAtCertification=true in {paths[CANDIDATE]}: "
            f"actual={candidate_reset!r}"
        )

    expected_diff = {SIGNAL_TYPE_PATH, SIGNAL_WINDOW_PATH, AGE_RESET_PATH}
    differences = differing_paths(configs[CANDIDATE], configs[HISTORICAL])
    if differences != expected_diff:
        rendered = [".".join(path) or "<root>" for path in sorted(differences)]
        expected = [".".join(path) for path in sorted(expected_diff)]
        raise ValueError(
            f"effective configs for {CANDIDATE} must differ from {HISTORICAL} only at "
            f"{expected!r}: actual differences={rendered!r}"
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
        if not isinstance(profile, dict) or profile.get("name") != PROFILE_NAMES[directory]:
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
    reference_seeds = seed_sets[HISTORICAL]
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
    contrasts: dict[str, Any] = {}
    for cid, spec in CONTRASTS.items():
        candidate, reference = spec["candidate"], spec["reference"]
        differences = [
            candidate_value - reference_value
            for candidate_value, reference_value in zip(
                values[candidate], values[reference]
            )
        ]
        estimate, low, high = paired_interval(differences)
        contrasts[cid] = {
            "candidate": candidate,
            "reference": reference,
            "primary": spec["primary"],
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


def paired_contrast(
    runs: dict[str, dict[int, dict[str, float]]],
    seeds: list[int],
    key: str,
    candidate: str,
    reference: str,
    scale: float,
) -> dict[str, Any]:
    differences = [
        scale
        * (
            numeric_scalar(runs[candidate][seed], key, candidate, seed)
            - numeric_scalar(runs[reference][seed], key, reference, seed)
        )
        for seed in seeds
    ]
    estimate, low, high = paired_interval(differences)
    return {
        "mean": estimate,
        "ci95_low": low,
        "ci95_high": high,
        "candidate_higher": sum(value > 0 for value in differences),
        "n": len(differences),
    }


def evaluate_criteria(
    runs_by_load: dict[str, dict[str, dict[int, dict[str, float]]]],
    seeds: list[int],
) -> list[dict[str, Any]]:
    criteria: list[dict[str, Any]] = []

    for directory in MATCHING_LOADS:
        m1 = paired_contrast(
            runs_by_load[directory], seeds, "value.retainedRatio", CANDIDATE, HISTORICAL, 100.0
        )
        criteria.append(
            {
                "id": f"M1 ({directory})",
                "description": (
                    f"{directory}: candidate-minus-historical overall retained-value "
                    "ratio, 95% CI lower bound above -0.05 pp"
                ),
                "observed": (
                    f"{m1['mean']:+.4f} pp [{m1['ci95_low']:+.4f}, {m1['ci95_high']:+.4f}]"
                ),
                "pass": m1["ci95_low"] > -0.05,
            }
        )

    m2 = paired_contrast(
        runs_by_load["trickle-0p1"], seeds, "value.standard.retainedRatio", CANDIDATE, HISTORICAL, 100.0
    )
    criteria.append(
        {
            "id": "M2",
            "description": (
                "Trickle: candidate-minus-historical standard-lane retained-value "
                "ratio, 95% CI lower bound above -1.0 pp"
            ),
            "observed": (
                f"{m2['mean']:+.3f} pp [{m2['ci95_low']:+.3f}, {m2['ci95_high']:+.3f}]"
            ),
            "pass": m2["ci95_low"] > -1.0,
        }
    )

    for directory in MATCHING_LOADS:
        m3 = paired_contrast(
            runs_by_load[directory], seeds, "value.retainedLovelace", CANDIDATE, FLAT_FEE, 1e-9
        )
        criteria.append(
            {
                "id": f"M3 ({directory})",
                "description": (
                    f"{directory}: candidate-minus-flat-fee overall retained value, "
                    "95% CI above zero"
                ),
                "observed": (
                    f"{m3['mean']:+.3f} G [{m3['ci95_low']:+.3f}, {m3['ci95_high']:+.3f}]"
                ),
                "pass": m3["ci95_low"] > 0,
            }
        )
    return criteria


def evaluate_secondaries(
    runs_by_load: dict[str, dict[str, dict[int, dict[str, float]]]],
    seeds: list[int],
) -> list[dict[str, Any]]:
    s1 = paired_contrast(
        runs_by_load["launch-day"], seeds, "value.retainedLovelace", CANDIDATE, HISTORICAL, 1e-9
    )
    return [
        {
            "id": "S1",
            "description": (
                "Launch day: candidate-minus-historical overall retained value, "
                "with seed-pair win count (reported, not gated)"
            ),
            "observed": (
                f"{s1['mean']:+.3f} G [{s1['ci95_low']:+.3f}, {s1['ci95_high']:+.3f}], "
                f"candidate higher in {s1['candidate_higher']}/{s1['n']}"
            ),
        }
    ]


def build_report(
    root: Path, manifest_path: Path, simulator_sha256: str
) -> dict[str, Any]:
    validate_manifest(manifest_path)
    load_data: list[dict[str, Any]] = []
    runs_by_load: dict[str, dict[str, dict[int, dict[str, float]]]] = {}
    common_seeds: tuple[int, ...] | None = None
    common_slots: int | None = None

    for directory, label in LOADS:
        summary_path, runs, slots, effective_configs = load_runs(root, directory)
        runs_by_load[directory] = runs
        seeds = tuple(sorted(runs[HISTORICAL]))
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
                "exactly_equal_all_scalars": {
                    "candidate-minus-historical": exactly_equal_count(
                        runs[CANDIDATE], runs[HISTORICAL], seed_list
                    )
                },
                "metric_groups": groups,
            }
        )

    if common_seeds is None or common_slots is None:
        raise ValueError("no load results found")
    seed_list = list(common_seeds)
    criteria = evaluate_criteria(runs_by_load, seed_list)
    secondaries = evaluate_secondaries(runs_by_load, seed_list)
    # The verdict is only meaningful for the preregistered design: seeds
    # 700-799 and 2,000 slots. Any other input still gets its criteria
    # reported, but no PASS/FAIL is issued in the preregistered sense.
    preregistered = seed_list == list(range(700, 800)) and common_slots == 2_000
    if not preregistered:
        verdict = "NOT-PREREGISTERED"
    elif all(criterion["pass"] for criterion in criteria):
        verdict = "PASS"
    else:
        verdict = "FAIL"

    provenance = source_provenance(root / "source.patch")
    provenance["simulator_sha256"] = simulator_sha256
    return {
        "schema_version": 1,
        "experiment": "standard-window-adoption-confirm",
        "method": "paired mean difference with two-sided 95% Student-t confidence interval",
        "evidence_scope": (
            "Confirmatory run against pre-declared criteria on seeds disjoint from the "
            "selecting screen. All criteria must hold (conjunctive gating, so no "
            "multiplicity adjustment is applied). The criteria gate matching and the "
            "flat-fee margin, the requirements the adoption decision rests on; "
            "superiority over the historical behaviour is a declared secondary, "
            "reported and never gated. The run cannot establish equivalence beyond "
            "the declared margins."
        ),
        "difference_direction": "candidate arm minus reference arm, per contrast",
        "manifest": str(manifest_path),
        "variants": {
            name: {"label": VARIANT_LABELS[name], "config": path}
            for name, path in VARIANTS.items()
        },
        "contrasts": {
            cid: {
                "label": f"{VARIANT_LABELS[spec['candidate']]} − {VARIANT_LABELS[spec['reference']]}",
                **spec,
            }
            for cid, spec in CONTRASTS.items()
        },
        "seeds": seed_list,
        "slots": common_slots,
        "provenance": provenance,
        "preregistered_design": {"seeds": "700-799", "slots": 2_000, "matched": preregistered},
        "criteria": criteria,
        "secondaries": secondaries,
        "verdict": verdict,
        "loads": load_data,
        "notes": [
            (
                "The candidate is the configuration the CIP will specify: the standard "
                "controller's window is the 10 most recent processed-block summaries "
                "(non-certificate RBs, certificate-carrying RBs, certified EBs; "
                "announcements excluded), and the EB age-escape count resets when a "
                "certified EB is applied rather than at announcement."
            ),
            (
                "The 10-block length was selected by the standard-window screen "
                "(seeds 0-99); this run uses disjoint seeds 700-799. The cert-reset "
                "change was separately bounded by the announcement-semantics smoke "
                "(seeds 440-449), which found it within noise everywhere and "
                "directionally favourable at trickle."
            ),
            (
                "Mid-load bit-identity between candidate and historical is expected "
                "(the standard coefficient rests on its floor and the age escape is "
                "not consulted); it is reported in the per-load tie counts, not gated."
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
        "# Standard-window adoption confirmation",
        "",
        (
            f"The specified configuration (10-block processed-block window, age reset "
            f"at certification) versus the historical window and flat fee; paired "
            f"seeds {seed_text} (n={len(seeds)}), {report['slots']:,} slots per load. "
            "Cells are candidate-minus-reference mean differences with two-sided 95% "
            "paired-t confidence intervals."
        ),
        "",
        f"## Verdict: {report['verdict']}",
        "",
        "Pre-declared criteria (all must hold; conjunctive gating, no multiplicity adjustment):",
        "",
        "| Criterion | Requirement | Observed | Result |",
        "|---|---|---|---|",
    ]
    for criterion in report["criteria"]:
        result = "PASS" if criterion["pass"] else "FAIL"
        lines.append(
            f"| {criterion['id']} | {criterion['description']} "
            f"| {criterion['observed']} | {result} |"
        )
    lines.extend(
        [
            "",
            "Declared secondaries (reported, not gated):",
            "",
            "| Secondary | Description | Observed |",
            "|---|---|---|",
        ]
    )
    for secondary in report["secondaries"]:
        lines.append(
            f"| {secondary['id']} | {secondary['description']} "
            f"| {secondary['observed']} |"
        )
    lines.extend(["", source_line])

    for load in report["loads"]:
        lines.extend(["", f"## {load['label']}"])
        equality = load["exactly_equal_all_scalars"]["candidate-minus-historical"]
        lines.extend(
            [
                "",
                f"Exact all-scalar ties, candidate versus historical: "
                f"{equality}/{load['paired_seeds']} paired seeds.",
            ]
        )
        for group in load["metric_groups"]:
            lines.extend(
                [
                    "",
                    f"### {group['label']}",
                    "",
                    (
                        "| Metric | Unit | Historical mean | Flat-fee mean "
                        "| Candidate − hist | Candidate − flat | Hist − flat |"
                    ),
                    "|---|---:|---:|---:|---:|---:|---:|",
                ]
            )
            for metric in group["metrics"]:
                lines.append(
                    f"| {metric['label']} | {metric['unit']} "
                    f"| {plain(metric['arm_means'][HISTORICAL], metric['digits'])} "
                    f"| {plain(metric['arm_means'][FLAT_FEE], metric['digits'])} "
                    f"| {interval_text(metric['contrasts']['candidate-minus-historical'], metric['digits'])} "
                    f"| {interval_text(metric['contrasts']['candidate-minus-flat-fee'], metric['digits'])} "
                    f"| {interval_text(metric['contrasts']['historical-minus-flat-fee'], metric['digits'])} |"
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
        help="directory containing the six load subdirectories",
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
    print(f"verdict: {report['verdict']}")
    return 0 if report["verdict"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
