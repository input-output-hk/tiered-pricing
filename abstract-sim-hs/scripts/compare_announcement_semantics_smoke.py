#!/usr/bin/env python3
"""Compare the four-arm announcement-semantics ablation across six loads.

The historical simulator differs from the CIP's specified mechanism in two
announcement-related behaviours. First, the standard controller's 20-entry
window is taken over all block summaries, so a zero-width EbAnnounced entry
occupies a window position; the specified window contains processed blocks
only. Second, the EB age-escape count resets at every announcement, including
announcements whose EB is later dropped without certifying; the specified
count resets only when an EB certificate enters the chain. The block-window
and cert-reset arms apply the specified semantics one at a time, and the
combined arm applies both. Ten paired seeds make this a directional smoke
test, not an equivalence test.
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
DEFAULT_ROOT = PROJECT_DIR / "sweep-results/announcement-semantics-smoke"
DEFAULT_MANIFEST = PROJECT_DIR / "config/sweeps/announcement-semantics-smoke.json"

HISTORICAL = "historical"
BLOCK_WINDOW = "block-window"
CERT_RESET = "cert-reset"
COMBINED = "block-window-cert-reset"

VARIANTS = {
    HISTORICAL: "config/variants/standard-target-screen/s75-u50.json",
    BLOCK_WINDOW: "config/variants/announcement-semantics/block-window.json",
    CERT_RESET: "config/variants/announcement-semantics/cert-reset.json",
    COMBINED: "config/variants/announcement-semantics/block-window-cert-reset.json",
}

VARIANT_LABELS = {
    HISTORICAL: "Historical announcement semantics",
    BLOCK_WINDOW: "Processed-block standard window",
    CERT_RESET: "Age-escape reset at certification",
    COMBINED: "Both specified semantics",
}

CANDIDATES = (BLOCK_WINDOW, CERT_RESET, COMBINED)

SIGNAL_TYPE_PATH = ("design", "controllers", "standardController", "signal", "type")
AGE_RESET_PATH = ("design", "ageResetAtCertification")

EXPECTED_DIFFERENCES = {
    BLOCK_WINDOW: {SIGNAL_TYPE_PATH},
    CERT_RESET: {AGE_RESET_PATH},
    COMBINED: {SIGNAL_TYPE_PATH, AGE_RESET_PATH},
}

LOADS = (
    ("low", "Low"),
    ("mid-load", "Mid load"),
    ("severe-congestion", "Severe congestion"),
    ("eb-capacity-stress", "EB-capacity stress"),
    ("launch-day", "Launch day"),
    ("trickle-0p1", "Trickle 0.1 tx/slot"),
)

# The trickle profile's embedded name uses a dot where its filename uses "p".
PROFILE_NAMES = {directory: directory for directory, _ in LOADS if directory != "low"}
PROFILE_NAMES["trickle-0p1"] = "trickle-0.1"


def contrast_id(candidate: str) -> str:
    return f"{candidate}-minus-{HISTORICAL}"


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
    if manifest.get("seedStart") != 440:
        raise ValueError(f"manifest must default to seed 440: {path}")
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

    historical_design = nested_value(configs[HISTORICAL], ("design",), paths[HISTORICAL])
    historical_reset = historical_design.get(AGE_RESET_PATH[-1], False)
    if historical_reset is not False:
        raise ValueError(
            f"historical arm must reset at announcement (absent or false) in "
            f"{paths[HISTORICAL]}: actual={historical_reset!r}"
        )
    historical_signal = nested_value(configs[HISTORICAL], SIGNAL_TYPE_PATH, paths[HISTORICAL])
    if historical_signal != "capacity-weighted-window":
        raise ValueError(
            f"historical arm must use the capacity-weighted-window signal in "
            f"{paths[HISTORICAL]}: actual={historical_signal!r}"
        )

    for name in (BLOCK_WINDOW, COMBINED):
        signal = nested_value(configs[name], SIGNAL_TYPE_PATH, paths[name])
        if signal != "capacity-weighted-block-window":
            raise ValueError(
                f"{name} arm must use the capacity-weighted-block-window signal in "
                f"{paths[name]}: actual={signal!r}"
            )
    for name in (CERT_RESET, COMBINED):
        reset = nested_value(configs[name], AGE_RESET_PATH, paths[name])
        if reset is not True:
            raise ValueError(
                f"{name} arm must set ageResetAtCertification=true in {paths[name]}: "
                f"actual={reset!r}"
            )

    fixed_expectations = {
        ("design", "controllers", "standardController", "targetUtilisation"): 0.75,
        ("design", "controllers", "standardController", "maxChangeDenominator"): 16,
        ("design", "controllers", "standardController", "signal", "window"): 20,
        ("design", "controllers", "priorityController", "targetUtilisation"): 0.5,
        ("design", "controllers", "priorityController", "maxChangeDenominator"): 16,
        ("design", "reservationPolicy", "ebThresholdBytes"): 45_056,
        ("design", "reservationPolicy", "ebAgeEscapeRbIntervals"): 10,
    }
    for name, config in configs.items():
        for path, expected in fixed_expectations.items():
            actual = nested_value(config, path, paths[name])
            if actual != expected:
                raise ValueError(
                    f"unexpected {'.'.join(path)} in {paths[name]}: "
                    f"expected={expected!r}, actual={actual!r}"
                )

    for name in CANDIDATES:
        differences = differing_paths(configs[name], configs[HISTORICAL])
        if differences != EXPECTED_DIFFERENCES[name]:
            rendered = [".".join(path) or "<root>" for path in sorted(differences)]
            expected = [".".join(path) for path in sorted(EXPECTED_DIFFERENCES[name])]
            raise ValueError(
                f"effective configs for {name} must differ from {HISTORICAL} only at "
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
    for candidate in CANDIDATES:
        differences = [
            candidate_value - historical_value
            for candidate_value, historical_value in zip(
                values[candidate], values[HISTORICAL]
            )
        ]
        estimate, low, high = paired_interval(differences)
        contrasts[contrast_id(candidate)] = {
            "candidate": candidate,
            "reference": HISTORICAL,
            "primary": candidate != COMBINED,
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


def build_report(
    root: Path, manifest_path: Path, simulator_sha256: str
) -> dict[str, Any]:
    validate_manifest(manifest_path)
    load_data: list[dict[str, Any]] = []
    common_seeds: tuple[int, ...] | None = None
    common_slots: int | None = None

    for directory, label in LOADS:
        summary_path, runs, slots, effective_configs = load_runs(root, directory)
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
                    contrast_id(candidate): exactly_equal_count(
                        runs[candidate], runs[HISTORICAL], seed_list
                    )
                    for candidate in CANDIDATES
                },
                "metric_groups": groups,
            }
        )

    if common_seeds is None or common_slots is None:
        raise ValueError("no load results found")
    provenance = source_provenance(root / "source.patch")
    provenance["simulator_sha256"] = simulator_sha256
    return {
        "schema_version": 1,
        "experiment": "announcement-semantics-smoke",
        "method": "paired mean difference with two-sided 95% Student-t confidence interval",
        "evidence_scope": (
            "Exploratory directional smoke test. The committed default is ten paired "
            "seeds; this run is not powered to establish equivalence, select protocol "
            "parameters, or show that an unobserved practically important effect is absent."
        ),
        "difference_direction": "specified-semantics arm minus historical arm",
        "manifest": str(manifest_path),
        "variants": {
            name: {"label": VARIANT_LABELS[name], "config": path}
            for name, path in VARIANTS.items()
        },
        "contrasts": {
            contrast_id(candidate): {
                "label": f"{VARIANT_LABELS[candidate]} − historical",
                "candidate": candidate,
                "reference": HISTORICAL,
                "primary": candidate != COMBINED,
            }
            for candidate in CANDIDATES
        },
        "seeds": list(common_seeds),
        "slots": common_slots,
        "provenance": provenance,
        "loads": load_data,
        "notes": [
            (
                "block-window changes only the standard controller's utilisation "
                "signal: its 20-entry window is taken over processed-block summaries "
                "(non-certificate RBs, certificate-carrying RBs, certified EBs), so a "
                "zero-width announcement entry cannot occupy a window position. "
                "Certificate-carrying RBs remain zero-capacity window entries in both "
                "arms."
            ),
            (
                "cert-reset changes only when the EB age-escape count restarts: at "
                "certified-EB application instead of at every announcement. Under the "
                "historical policy an announcement whose EB is later dropped without "
                "certifying still resets the count; with f=0.05 and D=13 roughly 46% "
                "of announcements drop this way."
            ),
            (
                "The age escape binds at trickle load, so trickle-0p1 is the "
                "decision-facing load for cert-reset; the standard window shape "
                "matters wherever the standard coefficient leaves its floor, so "
                "severe congestion, EB-capacity stress, and launch day are the "
                "decision-facing loads for block-window."
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
        "# Announcement-semantics ablation smoke",
        "",
        (
            f"Historical announcement semantics versus the CIP's specified semantics, "
            f"one knob at a time and combined; paired seeds {seed_text} "
            f"(n={len(seeds)}), {report['slots']:,} slots per load. Cells are "
            "arm-minus-historical mean differences with two-sided 95% paired-t "
            "confidence intervals."
        ),
        "",
        (
            "**Exploratory/directional only.** This smoke run cannot establish "
            "equivalence, select a protocol default, or show that a practically "
            "important effect is absent."
        ),
        "",
        source_line,
    ]

    for load in report["loads"]:
        lines.extend(["", f"## {load['label']}"])
        equalities = load["exactly_equal_all_scalars"]
        equality_text = ", ".join(
            f"{VARIANT_LABELS[candidate]}: {equalities[contrast_id(candidate)]}"
            f"/{load['paired_seeds']}"
            for candidate in CANDIDATES
        )
        lines.extend(["", f"Exact all-scalar ties versus historical: {equality_text}."])
        for group in load["metric_groups"]:
            lines.extend(
                [
                    "",
                    f"### {group['label']}",
                    "",
                    (
                        "| Metric | Unit | Historical mean "
                        "| Block window − hist | Cert reset − hist | Both − hist |"
                    ),
                    "|---|---:|---:|---:|---:|---:|",
                ]
            )
            for metric in group["metrics"]:
                cells = " | ".join(
                    interval_text(metric["contrasts"][contrast_id(candidate)], metric["digits"])
                    for candidate in CANDIDATES
                )
                lines.append(
                    f"| {metric['label']} | {metric['unit']} "
                    f"| {plain(metric['arm_means'][HISTORICAL], metric['digits'])} "
                    f"| {cells} |"
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
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
