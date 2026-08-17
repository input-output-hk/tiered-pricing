#!/usr/bin/env python3
"""Compare the held-out three-arm controller-window confirmation.

The manifest predeclares two co-primary paired endpoints.  Their two-sided
97.5% Student-t intervals apply a Bonferroni correction across the pair, and
each must agree with the expected direction in at least nine of ten seeds.
Every other table entry is an ordinary descriptive 95% paired interval.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
from pathlib import Path
from typing import Any, Iterable

from compare_canonical_headlines import source_provenance
from compare_cross_lane_inversion_smoke import paired_interval
from compare_window_ablation_smoke import (
    METRIC_GROUPS as SMOKE_METRIC_GROUPS,
    Metric,
    differing_paths,
    exactly_equal_count,
    json_object,
    mean,
    metric_is_usable_for_load,
    nested_value,
    numeric_scalar,
    plain,
    replace_nested,
    signed,
)


PROJECT_DIR = Path(__file__).resolve().parent.parent
DEFAULT_ROOT = PROJECT_DIR / "sweep-results/window-ablation-confirm"
DEFAULT_MANIFEST = PROJECT_DIR / "config/sweeps/window-ablation-confirm.json"
DEFAULT_PILOT = PROJECT_DIR / "sweep-results/window-ablation-smoke/comparison.json"
PILOT_REPORT_SHA256 = "97b7a5f628d74cb55f06d191998044f7d8b54a925d6fc7dca7437c730379f55f"
PILOT_SOURCE_PATCH_SHA256 = (
    "c658660f40d481c42bc488816621d7b68e99d0b13b69b7914437afbe5c63d24b"
)

CURRENT = "current-w20-w5"
STANDARD_INSTANT = "standard-instant-w5"
URGENT_INSTANT = "w20-urgent-instant"

VARIANTS = {
    CURRENT: "config/variants/standard-target-screen/s75-u50.json",
    STANDARD_INSTANT: "config/variants/window-ablation/standard-instant-w5.json",
    URGENT_INSTANT: "config/variants/window-ablation/w20-urgent-instant.json",
}

VARIANT_LABELS = {
    CURRENT: "Current W20/W5",
    STANDARD_INSTANT: "Standard instant / urgent W5",
    URGENT_INSTANT: "Standard W20 / urgent instant",
}

STANDARD_WINDOW = {"type": "capacity-weighted-window", "window": 20}
STANDARD_CURRENT_PRODUCTION = "capacity-weighted-util"
URGENT_WINDOW = {"type": "priority-reservation-window", "window": 5}
URGENT_CURRENT_PRODUCTION = "priority-reservation-util"

EXPECTED_SIGNALS = {
    CURRENT: (STANDARD_WINDOW, URGENT_WINDOW),
    STANDARD_INSTANT: (STANDARD_CURRENT_PRODUCTION, URGENT_WINDOW),
    URGENT_INSTANT: (STANDARD_WINDOW, URGENT_CURRENT_PRODUCTION),
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
        "standard-instant-minus-current",
        "Standard instant − current",
        STANDARD_INSTANT,
        CURRENT,
    ),
    (
        "urgent-instant-minus-current",
        "Urgent instant − current",
        URGENT_INSTANT,
        CURRENT,
    ),
)

EXPECTED_DESCRIPTION = (
    "Held-out confirmation of the controller-window ablation. Uses three arms "
    "to confirm the two effects selected before this run: launch-day throughput "
    "under an instantaneous standard signal, and severe-congestion price-path "
    "oscillation under an instantaneous urgent signal. Pilot seeds 300-304 are "
    "excluded from confirmatory inference."
)

EXPECTED_ANALYSIS_PLAN = {
    "stage": "held-out-confirmation",
    "pilot": {
        "report": "sweep-results/window-ablation-smoke/comparison.json",
        "reportSha256": PILOT_REPORT_SHA256,
        "sourcePatchSha256": PILOT_SOURCE_PATCH_SHA256,
        "seeds": [300, 301, 302, 303, 304],
        "excludedFromInference": True,
    },
    "primaryProcedure": (
        "Two co-primary paired contrasts use two-sided 97.5% Student-t confidence "
        "intervals (Bonferroni-adjusted across the two endpoints). Each endpoint "
        "must also agree in the expected direction in at least 9 of 10 paired seeds. "
        "Both endpoints must pass."
    ),
    "primaryEndpoints": [
        {
            "id": "launch-standard-throughput",
            "load": "launch-day",
            "contrast": "standard-instant-minus-current",
            "metric": "throughput.txPerSlot",
            "expectedDirection": "negative",
        },
        {
            "id": "severe-urgent-oscillation-travel",
            "load": "severe-congestion",
            "contrast": "urgent-instant-minus-current",
            "metric": "price.oscillationExcessTravel",
            "expectedDirection": "positive",
        },
    ],
    "secondaryProcedure": (
        "All other load, metric, and contrast results are descriptive coherence "
        "and safety checks with ordinary two-sided 95% paired-t confidence intervals. "
        "They are not additional confirmatory tests."
    ),
    "scope": (
        "Passing supports standard windowing for launch-day throughput and urgent "
        "windowing for severe-load price-path stability over the tested instantaneous "
        "alternatives under this simulator calibration. It does not establish welfare, "
        "real-world optimality, or that window lengths 20 and 5 are globally optimal."
    ),
}

CANONICAL_SEEDS = tuple(range(400, 410))
CANONICAL_SLOTS = 2_000
REQUIRED_DIRECTION_COUNT = 9

PRIMARY_ENDPOINTS = tuple(EXPECTED_ANALYSIS_PLAN["primaryEndpoints"])

ACCOUNTING_GROUP = (
    "accounting",
    "Additional fee and value accounting (not welfare)",
    (
        Metric(
            "derived.submittedGrossLovelace",
            "Submitted gross value",
            "G lovelace",
            1e-9,
            3,
            None,
        ),
        Metric(
            "derived.realisedFeesLovelace",
            "Realised fees",
            "G lovelace",
            1e-9,
            3,
            None,
        ),
        Metric(
            "derived.userNetLovelace",
            "Gross retained minus realised fees",
            "G lovelace",
            1e-9,
            3,
            "higher",
        ),
    ),
)

METRIC_GROUPS = SMOKE_METRIC_GROUPS + (ACCOUNTING_GROUP,)
METRICS_BY_KEY = {
    metric.key: metric
    for _group_id, _group_label, metrics in METRIC_GROUPS
    for metric in metrics
}


# Quantiles t(df, 0.9875), giving a two-sided 97.5% interval.  The evidence
# design uses df=9; the small-sample table also keeps diagnostic reruns honest.
T_9875 = {
    1: 25.451699579,
    2: 6.205346817,
    3: 4.176534846,
    4: 3.495405933,
    5: 3.163381450,
    6: 2.968686684,
    7: 2.841244249,
    8: 2.751523596,
    9: 2.685010847,
    10: 2.633766916,
    11: 2.593092683,
    12: 2.560032959,
    13: 2.532637815,
    14: 2.509569411,
    15: 2.489879703,
    16: 2.472878322,
    17: 2.458050720,
    18: 2.445005617,
    19: 2.433440211,
    20: 2.423116540,
    21: 2.413845017,
    22: 2.405472746,
    23: 2.397875065,
    24: 2.390949315,
    25: 2.384610201,
    26: 2.378786266,
    27: 2.373417201,
    28: 2.368451749,
    29: 2.363846073,
    30: 2.359562459,
}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def artifact(path: Path) -> dict[str, Any]:
    return {
        "path": str(path),
        "sha256": sha256_file(path),
        "bytes": path.stat().st_size,
    }


def validate_pre_run_hashes(root: Path, paths: list[Path]) -> dict[str, Any]:
    ledger = root / "analysis-plan.sha256"
    try:
        lines = ledger.read_text(encoding="utf-8").splitlines()
    except FileNotFoundError as error:
        raise ValueError(f"pre-run analysis-plan hash ledger is missing: {ledger}") from error
    recorded: dict[str, str] = {}
    for line in lines:
        pieces = line.split(maxsplit=1)
        if len(pieces) != 2:
            raise ValueError(f"malformed line in pre-run hash ledger: {line!r}")
        digest, name = pieces
        if name in recorded:
            raise ValueError(f"duplicate path in pre-run hash ledger: {name}")
        recorded[name] = digest
    project_dir = PROJECT_DIR.resolve()
    expected: dict[str, str] = {}
    for path in paths:
        resolved = path.resolve()
        try:
            name = str(resolved.relative_to(project_dir))
        except ValueError:
            name = str(resolved)
        expected[name] = sha256_file(resolved)
    if recorded != expected:
        raise ValueError(
            f"pre-run analysis-plan hashes differ from the files used for comparison: "
            f"recorded={recorded!r}, expected={expected!r}"
        )
    return {
        "artifact": artifact(ledger),
        "recorded_before_simulation": True,
        "files": recorded,
    }


def validate_manifest(path: Path) -> dict[str, Any]:
    manifest = json_object(path)
    expected_scalars = {
        "description": EXPECTED_DESCRIPTION,
        "analysisPlan": EXPECTED_ANALYSIS_PLAN,
        "seedStart": 400,
        "seeds": 10,
        "slots": CANONICAL_SLOTS,
        "summaryOnly": True,
        "randomness": "independent-streams",
    }
    for key, expected in expected_scalars.items():
        if manifest.get(key) != expected:
            raise ValueError(
                f"unexpected manifest {key} in {path}: "
                f"expected={expected!r}, actual={manifest.get(key)!r}"
            )

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


def validate_effective_configs(directory: Path) -> tuple[dict[str, str], list[dict[str, Any]]]:
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

    return (
        {name: str(path) for name, path in paths.items()},
        [artifact(path) for path in paths.values()],
    )


def load_runs(
    root: Path, directory: str, manifest: dict[str, Any]
) -> tuple[
    Path,
    dict[str, dict[int, dict[str, float]]],
    int,
    int,
    dict[str, str],
    list[dict[str, Any]],
]:
    path = root / directory / "summary.json"
    summary = json_object(path)
    if not isinstance(summary.get("variants"), list):
        raise ValueError(f"not a sweep summary (missing variants array): {path}")
    if summary.get("description") != manifest["description"]:
        raise ValueError(f"summary description does not match the manifest: {path}")
    if summary.get("randomness") != "independent-streams":
        raise ValueError(f"summary must use independent-streams randomness: {path}")
    if summary.get("summaryOnly") is not True:
        raise ValueError(f"summary must be summary-only: {path}")
    if summary.get("loadOverride") != expected_load_override(directory):
        raise ValueError(
            f"unexpected load override in {path}: {summary.get('loadOverride')!r}"
        )

    artifacts = [artifact(path)]
    if directory != "low":
        profile = summary.get("loadProfile")
        if not isinstance(profile, dict) or profile.get("name") != directory:
            raise ValueError(f"missing/wrong selected load profile in {path}")
        selected_profile = root / directory / "selected-load-profile.json"
        selected = json_object(selected_profile)
        source_profile = PROJECT_DIR / f"config/loads/{directory}.json"
        source = json_object(source_profile)
        if selected != source:
            raise ValueError(f"selected load-profile copy differs from its source: {path}")
        if profile.get("description") != selected.get("description"):
            raise ValueError(f"summary load-profile description differs from its copy: {path}")
        artifacts.extend((artifact(selected_profile), artifact(source_profile)))

    actual_configs = {
        variant.get("name"): variant.get("config") for variant in summary["variants"]
    }
    if actual_configs != VARIANTS:
        raise ValueError(
            f"unexpected variants/configs in {path}: expected={VARIANTS!r}, "
            f"actual={actual_configs!r}"
        )

    runs = {name: variant_runs(summary, name, path) for name in VARIANTS}
    seed_sets = {name: set(by_seed) for name, by_seed in runs.items()}
    reference_seeds = seed_sets[CURRENT]
    if any(seeds != reference_seeds for seeds in seed_sets.values()):
        rendered = {name: sorted(seeds) for name, seeds in seed_sets.items()}
        raise ValueError(f"paired comparison requires identical seed sets in {path}: {rendered}")

    declared_seed_start = summary.get("seedStart")
    declared_seed_count = summary.get("seeds")
    if (
        isinstance(declared_seed_start, bool)
        or not isinstance(declared_seed_start, int)
        or declared_seed_start < 0
    ):
        raise ValueError(f"invalid seedStart in {path}: {declared_seed_start!r}")
    if (
        isinstance(declared_seed_count, bool)
        or not isinstance(declared_seed_count, int)
        or declared_seed_count < 2
    ):
        raise ValueError(f"invalid seed count in {path}: {declared_seed_count!r}")
    expected_seeds = set(
        range(declared_seed_start, declared_seed_start + declared_seed_count)
    )
    if reference_seeds != expected_seeds:
        raise ValueError(
            f"summary seed range disagrees with runs in {path}: "
            f"declared={sorted(expected_seeds)}, actual={sorted(reference_seeds)}"
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

    effective_configs, config_artifacts = validate_effective_configs(root / directory)
    artifacts.extend(config_artifacts)
    return (
        path,
        runs,
        slots,
        declared_seed_start,
        effective_configs,
        artifacts,
    )


def scalar_value(
    scalars: dict[str, Any], key: str, variant: str, seed: int
) -> float:
    if key == "derived.submittedGrossLovelace":
        return sum(
            numeric_scalar(scalars, component, variant, seed)
            for component in (
                "value.retainedLovelace",
                "value.lostLovelace",
                "value.unresolvedLovelace",
            )
        )
    if key == "derived.realisedFeesLovelace":
        return numeric_scalar(
            scalars, "revenue.feesCollectedLovelace", variant, seed
        ) - numeric_scalar(scalars, "revenue.refundsPaidLovelace", variant, seed)
    if key == "derived.userNetLovelace":
        realised_fees = scalar_value(
            scalars, "derived.realisedFeesLovelace", variant, seed
        )
        return numeric_scalar(
            scalars, "value.retainedLovelace", variant, seed
        ) - realised_fees
    return numeric_scalar(scalars, key, variant, seed)


def metric_result(
    metric: Metric,
    runs: dict[str, dict[int, dict[str, float]]],
    seeds: list[int],
) -> dict[str, Any]:
    values = {
        name: [
            metric.scale * scalar_value(by_seed[seed], metric.key, name, seed)
            for seed in seeds
        ]
        for name, by_seed in runs.items()
    }
    contrasts: dict[str, Any] = {}
    for contrast_id, _label, candidate, reference in CONTRASTS:
        differences = [
            values[candidate][index] - values[reference][index]
            for index in range(len(seeds))
        ]
        estimate, low, high = paired_interval(differences)
        contrasts[contrast_id] = {
            "candidate": candidate,
            "reference": reference,
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


def t_critical_9875(degrees_of_freedom: int) -> float:
    if degrees_of_freedom in T_9875:
        return T_9875[degrees_of_freedom]
    if degrees_of_freedom < 1:
        raise ValueError("paired interval needs at least two observations")
    # Cornish-Fisher expansion around z(.9875), accurate in the df>30 branch.
    z = 2.241402727604947
    df = float(degrees_of_freedom)
    return (
        z
        + (z**3 + z) / (4 * df)
        + (5 * z**5 + 16 * z**3 + 3 * z) / (96 * df**2)
        + (3 * z**7 + 19 * z**5 + 17 * z**3 - 15 * z) / (384 * df**3)
    )


def paired_interval_97_5(differences: Iterable[float]) -> tuple[float, float, float]:
    values = list(differences)
    if len(values) < 2:
        raise ValueError("paired interval needs at least two observations")
    estimate = mean(values)
    variance = sum((value - estimate) ** 2 for value in values) / (len(values) - 1)
    margin = t_critical_9875(len(values) - 1) * math.sqrt(variance / len(values))
    return estimate, estimate - margin, estimate + margin


def exact_two_sided_sign_p(positive: int, negative: int) -> float:
    non_ties = positive + negative
    if non_ties == 0:
        return 1.0
    extreme = max(positive, negative)
    tail = sum(math.comb(non_ties, count) for count in range(extreme, non_ties + 1))
    return min(1.0, 2.0 * tail / (2**non_ties))


def primary_result(
    specification: dict[str, str],
    runs_by_load: dict[str, dict[str, dict[int, dict[str, float]]]],
    seeds: list[int],
    canonical_confirmation: bool,
) -> dict[str, Any]:
    contrast = next(
        item for item in CONTRASTS if item[0] == specification["contrast"]
    )
    _contrast_id, contrast_label, candidate, reference = contrast
    metric = METRICS_BY_KEY[specification["metric"]]
    runs = runs_by_load[specification["load"]]
    candidate_values = [
        metric.scale
        * scalar_value(runs[candidate][seed], metric.key, candidate, seed)
        for seed in seeds
    ]
    reference_values = [
        metric.scale
        * scalar_value(runs[reference][seed], metric.key, reference, seed)
        for seed in seeds
    ]
    differences = [
        candidate_value - reference_value
        for candidate_value, reference_value in zip(candidate_values, reference_values)
    ]
    estimate, low, high = paired_interval_97_5(differences)
    positive = sum(value > 0 for value in differences)
    negative = sum(value < 0 for value in differences)
    ties = sum(value == 0 for value in differences)
    expected_direction = specification["expectedDirection"]
    expected_count = positive if expected_direction == "positive" else negative
    interval_pass = low > 0 if expected_direction == "positive" else high < 0
    direction_pass = expected_count >= REQUIRED_DIRECTION_COUNT
    if not canonical_confirmation:
        status = "diagnostic-only"
    elif interval_pass and direction_pass:
        status = "pass"
    else:
        status = "not-confirmed"
    return {
        **specification,
        "label": f"{metric.label} under {dict(LOADS)[specification['load']]}",
        "contrast_label": contrast_label,
        "candidate": candidate,
        "reference": reference,
        "unit": metric.unit,
        "digits": metric.digits,
        "candidate_mean": mean(candidate_values),
        "reference_mean": mean(reference_values),
        "mean": estimate,
        "ci97_5_low": low,
        "ci97_5_high": high,
        "candidate_higher": positive,
        "reference_higher": negative,
        "ties": ties,
        "expected_direction_count": expected_count,
        "required_direction_count": REQUIRED_DIRECTION_COUNT,
        "exact_two_sided_sign_p": exact_two_sided_sign_p(positive, negative),
        "adjusted_interval_pass": interval_pass,
        "direction_count_pass": direction_pass,
        "status": status,
    }


def resolve_project_path(path: str) -> Path:
    candidate = Path(path)
    return candidate if candidate.is_absolute() else PROJECT_DIR / candidate


def validate_pilot(
    path: Path, simulator_sha256: str
) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    if sha256_file(path) != PILOT_REPORT_SHA256:
        raise ValueError(f"pilot report hash differs from the predeclared artifact: {path}")
    pilot = json_object(path)
    if pilot.get("experiment") != "controller-window-ablation-smoke":
        raise ValueError(f"unexpected pilot experiment in {path}")
    if pilot.get("seeds") != [300, 301, 302, 303, 304]:
        raise ValueError(f"pilot must contain only seeds 300-304: {path}")
    if pilot.get("slots") != CANONICAL_SLOTS:
        raise ValueError(f"pilot must use 2,000 slots: {path}")
    provenance = pilot.get("provenance")
    if not isinstance(provenance, dict):
        raise ValueError(f"pilot is missing provenance: {path}")
    pilot_sha256 = provenance.get("simulator_sha256")
    if pilot_sha256 != simulator_sha256:
        raise ValueError(
            f"confirmation simulator hash does not match pilot: "
            f"pilot={pilot_sha256!r}, confirmation={simulator_sha256!r}"
        )
    pilot_variants = pilot.get("variants")
    if not isinstance(pilot_variants, dict):
        raise ValueError(f"pilot is missing variant metadata: {path}")
    for name, config in VARIANTS.items():
        variant = pilot_variants.get(name)
        if not isinstance(variant, dict) or variant.get("config") != config:
            raise ValueError(f"pilot variant {name!r} does not match confirmation: {path}")

    dirty_patch = provenance.get("dirty_patch")
    if not isinstance(dirty_patch, dict):
        raise ValueError(f"pilot is missing its dirty-source patch metadata: {path}")
    pilot_patch = path.parent / str(dirty_patch.get("path"))
    if (
        dirty_patch.get("sha256") != PILOT_SOURCE_PATCH_SHA256
        or sha256_file(pilot_patch) != PILOT_SOURCE_PATCH_SHA256
    ):
        raise ValueError(f"pilot source-patch hash differs from the predeclared artifact: {path}")

    loads = pilot.get("loads")
    if not isinstance(loads, list):
        raise ValueError(f"pilot is missing load metadata: {path}")
    pilot_inputs: dict[str, dict[str, Any]] = {}
    input_artifacts: list[dict[str, Any]] = [artifact(pilot_patch)]
    for directory, _label in LOADS:
        matches = [load for load in loads if load.get("load") == directory]
        if len(matches) != 1:
            raise ValueError(f"pilot must contain exactly one {directory!r} load: {path}")
        source = matches[0].get("source")
        if not isinstance(source, dict) or not isinstance(source.get("effective_configs"), dict):
            raise ValueError(f"pilot is missing effective configs for {directory}: {path}")
        configs: dict[str, Path] = {}
        for name in VARIANTS:
            configured_path = source["effective_configs"].get(name)
            if not isinstance(configured_path, str):
                raise ValueError(f"pilot is missing {name!r} config for {directory}: {path}")
            config_path = resolve_project_path(configured_path)
            json_object(config_path)
            configs[name] = config_path
            input_artifacts.append(artifact(config_path))
        selected_profile: Path | None = None
        if directory != "low":
            selected_profile = configs[CURRENT].parent / "selected-load-profile.json"
            json_object(selected_profile)
            input_artifacts.append(artifact(selected_profile))
        pilot_inputs[directory] = {
            "configs": configs,
            "selected_profile": selected_profile,
        }

    return ({
        "artifact": artifact(path),
        "source_patch": artifact(pilot_patch),
        "input_artifacts": input_artifacts,
        "seeds": pilot["seeds"],
        "slots": pilot["slots"],
        "simulator_sha256": pilot_sha256,
        "excluded_from_confirmation": True,
    }, pilot_inputs)


def validate_against_pilot(
    directory: str,
    effective_configs: dict[str, str],
    confirmation_root: Path,
    pilot_inputs: dict[str, dict[str, Any]],
) -> None:
    pilot_load = pilot_inputs[directory]
    for name, confirmation_path in effective_configs.items():
        confirmation = json_object(Path(confirmation_path))
        pilot = json_object(pilot_load["configs"][name])
        if confirmation != pilot:
            differences = differing_paths(confirmation, pilot)
            rendered = [".".join(path) or "<root>" for path in sorted(differences)]
            raise ValueError(
                f"confirmation config differs from pilot for {directory}/{name}: {rendered!r}"
            )
    if directory != "low":
        confirmation_profile = confirmation_root / directory / "selected-load-profile.json"
        pilot_profile = pilot_load["selected_profile"]
        if json_object(confirmation_profile) != json_object(pilot_profile):
            raise ValueError(f"confirmation load profile differs from pilot for {directory}")


def build_report(
    root: Path,
    manifest_path: Path,
    pilot_path: Path,
    simulator_sha256: str,
) -> dict[str, Any]:
    manifest = validate_manifest(manifest_path)
    pilot, pilot_inputs = validate_pilot(pilot_path, simulator_sha256)
    analysis_files = [
        manifest_path,
        Path(__file__),
        PROJECT_DIR / "scripts/compare_window_ablation_smoke.py",
        PROJECT_DIR / "scripts/compare_cross_lane_inversion_smoke.py",
        PROJECT_DIR / "scripts/compare_canonical_headlines.py",
        PROJECT_DIR / "scripts/run_window_ablation_confirm.sh",
    ]
    pre_run_hashes = validate_pre_run_hashes(root, analysis_files)
    load_data: list[dict[str, Any]] = []
    runs_by_load: dict[str, dict[str, dict[int, dict[str, float]]]] = {}
    common_seeds: tuple[int, ...] | None = None
    common_slots: int | None = None
    common_seed_start: int | None = None
    run_artifacts: list[dict[str, Any]] = []

    for directory, label in LOADS:
        (
            summary_path,
            runs,
            slots,
            seed_start,
            effective_configs,
            artifacts,
        ) = load_runs(root, directory, manifest)
        validate_against_pilot(directory, effective_configs, root, pilot_inputs)
        seeds = tuple(sorted(runs[CURRENT]))
        if common_seeds is None:
            common_seeds = seeds
            common_slots = slots
            common_seed_start = seed_start
        elif seeds != common_seeds or slots != common_slots or seed_start != common_seed_start:
            raise ValueError(
                "all loads must use exactly the same paired seeds, seedStart, and slots"
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
            for contrast_id, _label, candidate, reference in CONTRASTS
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
        runs_by_load[directory] = runs
        run_artifacts.extend(artifacts)

    if common_seeds is None or common_slots is None or common_seed_start is None:
        raise ValueError("no load results found")

    canonical_confirmation = (
        common_seeds == CANONICAL_SEEDS
        and common_seed_start == CANONICAL_SEEDS[0]
        and common_slots == CANONICAL_SLOTS
    )
    seed_list = list(common_seeds)
    primary_results = [
        primary_result(specification, runs_by_load, seed_list, canonical_confirmation)
        for specification in PRIMARY_ENDPOINTS
    ]
    if canonical_confirmation:
        global_status = (
            "pass"
            if all(result["status"] == "pass" for result in primary_results)
            else "not-confirmed"
        )
    else:
        global_status = "diagnostic-only"

    provenance = source_provenance(root / "source.patch")
    provenance["simulator_sha256"] = simulator_sha256
    return {
        "schema_version": 1,
        "experiment": "controller-window-ablation-confirmation",
        "method": (
            "paired mean differences; co-primary two-sided 97.5% Student-t "
            "confidence intervals and descriptive two-sided 95% intervals"
        ),
        "evidence_scope": EXPECTED_ANALYSIS_PLAN["scope"],
        "canonical_confirmation": canonical_confirmation,
        "global_primary_status": global_status,
        "difference_direction": "instantaneous signal minus current W20/W5 signal",
        "manifest": artifact(manifest_path),
        "analysis_plan": EXPECTED_ANALYSIS_PLAN,
        "pre_run_analysis_hashes": pre_run_hashes,
        "pilot": pilot,
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
            }
            for contrast_id, label, candidate, reference in CONTRASTS
        },
        "seeds": seed_list,
        "slots": common_slots,
        "primary_results": primary_results,
        "provenance": provenance,
        "artifacts": {
            "analysis_files": [artifact(path) for path in analysis_files],
            "run_inputs_and_summaries": run_artifacts,
        },
        "loads": load_data,
        "notes": [
            (
                "Pilot seeds 300-304 are retained only as provenance for endpoint "
                "selection and are not pooled into any confirmation estimate."
            ),
            (
                "Independent streams align fresh-demand samples and Ranking Block "
                "opportunities within a seed; retry jitter remains mechanism-dependent."
            ),
            (
                "Observed demand and submission counts cover demand that reached a first "
                "submission, not every generated demand sample. Absolute retained values "
                "must therefore be read beside conditional ratios and submission volume."
            ),
            (
                "Gross retained value treats fees as transfers. Gross retained minus "
                "realised fees is a model user-side accounting view. Neither is welfare."
            ),
            (
                "The standard instantaneous arm aggregates all payload and capacity "
                "summaries from the current block-producing slot; it is not a one-summary "
                "rolling window that may end on a payload-free announcement."
            ),
            (
                "Price-oscillation scalars combine both lanes. The isolated urgent-signal "
                "intervention attributes its paired change to that signal choice."
            ),
            (
                "Launch-day urgent-class slices are omitted because the profile changes "
                "the urgency-rate multiplier over time; priority-lane metrics remain usable."
            ),
            (
                "Secondary 95% intervals are descriptive. CI exclusion outside the two "
                "predeclared endpoints is not treated as a confirmatory finding."
            ),
        ],
    }


def interval_text(result: dict[str, Any], digits: int, level: str = "95") -> str:
    low_key = f"ci{level}_low"
    high_key = f"ci{level}_high"
    return (
        f"{signed(result['mean'], digits)} "
        f"[{signed(result[low_key], digits)}, {signed(result[high_key], digits)}]"
    )


def direction_text(result: dict[str, Any], total: int) -> str:
    return (
        f"+ {result['candidate_higher']}/{total}; "
        f"− {result['reference_higher']}/{total}; "
        f"ties {result['ties']}"
    )


def status_text(status: str) -> str:
    return status.replace("-", " ").upper()


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
    canonical_text = (
        "This is the predeclared held-out confirmation."
        if report["canonical_confirmation"]
        else "**Diagnostic run only:** seed range and/or slot count differ from the predeclared design."
    )
    lines = [
        "# Controller-window ablation: held-out confirmation",
        "",
        (
            f"Current W20/W5 versus one instantaneous signal at a time; paired seeds "
            f"{seed_text} (n={len(seeds)}), {report['slots']:,} slots per load. "
            "Pilot seeds 300–304 are excluded."
        ),
        "",
        canonical_text,
        "",
        source_line,
        "",
        "## Predeclared primary results",
        "",
        (
            "Two co-primary endpoints use Bonferroni-adjusted, two-sided 97.5% "
            "paired-t intervals. Each must exclude zero in the expected direction "
            "and agree in at least 9/10 seeds. Both must pass."
        ),
        "",
        f"**Global primary result: {status_text(report['global_primary_status'])}.**",
        "",
        (
            "| Endpoint | Contrast | Current mean | Instant mean | Difference "
            "(97.5% CI) | Expected-direction seeds | Raw exact sign p | Result |"
        ),
        "|---|---|---:|---:|---:|---:|---:|---:|",
    ]
    for result in report["primary_results"]:
        expected = "negative" if result["expectedDirection"] == "negative" else "positive"
        lines.append(
            f"| {result['label']} | {result['contrast_label']} "
            f"| {plain(result['reference_mean'], result['digits'])} "
            f"| {plain(result['candidate_mean'], result['digits'])} "
            f"| {interval_text(result, result['digits'], '97_5')} "
            f"| {result['expected_direction_count']}/{len(seeds)} {expected} "
            f"| {result['exact_two_sided_sign_p']:.4f} "
            f"| {status_text(result['status'])} |"
        )

    primary_locations = {
        (result["load"], result["contrast"], result["metric"])
        for result in report["primary_results"]
    }
    contrast_ids = [contrast_id for contrast_id, *_rest in CONTRASTS]
    for load in report["loads"]:
        lines.extend(["", f"## {load['label']} descriptive checks"])
        equality = load["exactly_equal_all_scalars"]
        lines.extend(
            [
                "",
                (
                    "Exact all-scalar ties with current: "
                    f"standard instant {equality[contrast_ids[0]]}/{load['paired_seeds']}; "
                    f"urgent instant {equality[contrast_ids[1]]}/{load['paired_seeds']}."
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
                        "| Metric | Unit | Current | Standard instant | Standard − current "
                        "(95% CI) | Directions | Urgent instant | Urgent − current "
                        "(95% CI) | Directions |"
                    ),
                    "|---|---:|---:|---:|---:|---:|---:|---:|---:|",
                ]
            )
            for metric in group["metrics"]:
                standard = metric["contrasts"][contrast_ids[0]]
                urgent = metric["contrasts"][contrast_ids[1]]
                marker = "† " if any(
                    (load["load"], contrast_id, metric["key"]) in primary_locations
                    for contrast_id in contrast_ids
                ) else ""
                lines.append(
                    f"| {marker}{metric['label']} | {metric['unit']} "
                    f"| {plain(metric['arm_means'][CURRENT], metric['digits'])} "
                    f"| {plain(metric['arm_means'][STANDARD_INSTANT], metric['digits'])} "
                    f"| {interval_text(standard, metric['digits'])} "
                    f"| {direction_text(standard, load['paired_seeds'])} "
                    f"| {plain(metric['arm_means'][URGENT_INSTANT], metric['digits'])} "
                    f"| {interval_text(urgent, metric['digits'])} "
                    f"| {direction_text(urgent, load['paired_seeds'])} |"
                )

    lines.extend(
        [
            "",
            "## Interpretation boundaries",
            "",
            *[f"- {note}" for note in report["notes"]],
            "- † marks a predeclared endpoint; its adjusted result is in the primary table.",
            "- NOT CONFIRMED means the predeclared gate was not passed; it is not evidence of equivalence or absence of an effect.",
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
        help="predeclared confirmation manifest",
    )
    parser.add_argument(
        "--pilot-report",
        type=Path,
        default=DEFAULT_PILOT,
        help="five-seed pilot comparison (provenance only; never pooled)",
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
        report = build_report(
            args.root,
            args.manifest,
            args.pilot_report,
            simulator_sha256,
        )
        rendered = markdown(report)
        markdown_output.write_text(rendered, encoding="utf-8")
        json_output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    except (OSError, ValueError, StopIteration) as error:
        raise SystemExit(f"error: {error}") from error
    print(f"wrote {markdown_output}")
    print(f"wrote {json_output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
