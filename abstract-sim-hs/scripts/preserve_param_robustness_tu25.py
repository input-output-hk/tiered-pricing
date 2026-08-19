#!/usr/bin/env python3
"""Preserve the corrected target-utilisation-0.25 parameter-sweep evidence.

The historical sweep outputs are intentionally gitignored and include roughly
17 GiB of event traces.  This script validates those outputs, keeps the
per-seed scalars needed by every corrected target-0.25 table and confidence
interval in the phase-2 report, and writes a compact evidence record.

The replay used the simulator at a historical revision plus one small source
patch.  Its sweep runner predates explicit randomness modes and uses one shared
PRNG stream.  Comparisons whose retry paths diverge are therefore descriptive,
but the record audits and preserves the low-load target-0.25 formula/fixed pair
as an aligned exception.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import statistics
import subprocess
from pathlib import Path
from typing import Any, Iterable

from compare_cross_lane_inversion_smoke import load_summary, numeric_scalar, variant_runs


PROJECT_DIR = Path(__file__).resolve().parent.parent
REPO_ROOT = PROJECT_DIR.parent

BASE_REVISION = "4d5b5abc40a8ea2942d8de8bb06ceee73c401b63"
PATCH_SHA256 = "bd5c099c1a25e13c997dde57872a5710205c2c02fd6e3252337aa97dd7298cf6"
PATCH_BYTES = 829
MANIFEST_SHA256 = "69da0d6c5c77c8ac1ccf1614653a7953ef8b9e831718144e4045d36a528f5800"
SOURCE_TREE_SHA256 = "046999f00c2defa598b9be069934d5339ec42834084acf7b84fe87a5a9b5d2f2"
SIMULATOR_SHA256 = "879d2db6f0356d241577ba70af4c1dddb7f012b07802d78d50a314c862c3ea16"
T_CRITICAL_95_DF9 = 2.2621571627409915

PATCH_PATH = (
    REPO_ROOT
    / "docs/phase-2/experiment-results/param-robustness-tu25-headroom.patch"
)
MANIFEST_PATH = PROJECT_DIR / "config/sweeps/param-robustness-tu25-correction.json"

VARIANTS = (
    "bdst-tu25-d4",
    "bdst-tu25-d8",
    "bdst-tu25-d16",
    "bdst-tu25-d8-fixed-thr",
)
EXPECTED_CONFIGS = {
    name: f"config/variants/param-robustness/{name}.json" for name in VARIANTS
}

# key, report scale, displayed unit
COMMON_METRICS = (
    ("units.serviceRate", 100.0, "percent"),
    ("value.urgent.retainedRatio", 100.0, "percent"),
    ("latency.urgent.meanBlocks", 1.0, "blocks"),
    ("throughput.txPerSlot", 1.0, "transactions/slot"),
    ("price.shockCount", 1.0, "count"),
    ("price.oscillationCycleCount", 1.0, "count"),
    ("price.oscillationMaxAmplitude", 1.0, "coefficient ratio"),
)
LAUNCH_METRICS = (
    ("value.retainedLovelace", 1.0e-9, "G lovelace"),
    ("units.serviceRate", 100.0, "percent"),
    ("throughput.txPerSlot", 1.0, "transactions/slot"),
    ("latency.priority.meanBlocks", 1.0, "blocks"),
    ("latency.standard.meanBlocks", 1.0, "blocks"),
    ("price.shockCount", 1.0, "count"),
    ("price.oscillationMaxAmplitude", 1.0, "coefficient ratio"),
)
LOAD_METRICS = {
    "low": COMMON_METRICS,
    "severe-congestion": COMMON_METRICS,
    "launch-day": LAUNCH_METRICS,
    "eb-capacity-stress": COMMON_METRICS,
}
LOAD_OVERRIDES = {
    "low": {"type": "preset", "name": "low"},
    "severe-congestion": {
        "type": "profile",
        "name": "severe-congestion",
        "source": "config/loads/severe-congestion.json",
    },
    "launch-day": {
        "type": "profile",
        "name": "launch-day",
        "source": "config/loads/launch-day.json",
    },
    "eb-capacity-stress": {
        "type": "profile",
        "name": "eb-capacity-stress",
        "source": "config/loads/eb-capacity-stress.json",
    },
}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def repo_relative(path: Path) -> str:
    try:
        return str(path.resolve().relative_to(REPO_ROOT.resolve()))
    except ValueError:
        return str(path)


def require_file(path: Path) -> None:
    if not path.is_file():
        raise ValueError(f"required file does not exist: {path}")


def require_hash(path: Path, expected: str) -> None:
    require_file(path)
    actual = sha256_file(path)
    if actual != expected:
        raise ValueError(
            f"unexpected SHA-256 for {path}: expected {expected}, found {actual}"
        )


def source_tree_sha256(worktree: Path) -> str:
    """Match: find app src -name '*.hs' | sort | xargs sha256sum | sha256sum."""
    project = worktree / "abstract-sim-hs"
    paths = sorted(
        (
            path
            for source_dir in (project / "app", project / "src")
            for path in source_dir.rglob("*.hs")
        ),
        key=lambda path: str(path.relative_to(project)),
    )
    if not paths:
        raise ValueError(f"no Haskell source files found below {project}")
    digest = hashlib.sha256()
    for path in paths:
        relative = path.relative_to(project)
        record = f"{sha256_file(path)}  {relative}\n"
        digest.update(record.encode("utf-8"))
    return digest.hexdigest()


def verify_historical_source(worktree: Path) -> None:
    if not worktree.is_dir():
        raise ValueError(f"historical worktree does not exist: {worktree}")
    revision = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=worktree,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    if revision != BASE_REVISION:
        raise ValueError(
            f"historical worktree HEAD is {revision}, expected {BASE_REVISION}"
        )
    patch = subprocess.run(
        [
            "git",
            "diff",
            "--binary",
            "--",
            "abstract-sim-hs/src/Pricing.hs",
        ],
        cwd=worktree,
        check=True,
        capture_output=True,
    ).stdout
    preserved_patch = PATCH_PATH.read_bytes()
    if patch != preserved_patch:
        raise ValueError(
            "historical worktree Pricing.hs diff does not equal the preserved patch"
        )
    worktree_manifest = worktree / MANIFEST_PATH.relative_to(REPO_ROOT)
    if worktree_manifest.read_bytes() != MANIFEST_PATH.read_bytes():
        raise ValueError(
            "historical worktree sweep manifest does not equal the preserved manifest"
        )
    actual_tree_hash = source_tree_sha256(worktree)
    if actual_tree_hash != SOURCE_TREE_SHA256:
        raise ValueError(
            "historical worktree source hash mismatch: "
            f"expected {SOURCE_TREE_SHA256}, found {actual_tree_hash}"
        )


def number(value: Any, context: str) -> int | float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"{context}: expected a numeric scalar, found {value!r}")
    if not math.isfinite(float(value)):
        raise ValueError(f"{context}: expected a finite scalar, found {value!r}")
    return value


def mean(values: Iterable[int | float]) -> float:
    copied = [float(value) for value in values]
    if not copied:
        raise ValueError("cannot calculate a mean from no values")
    return statistics.mean(copied)


def checked_variants(
    summary: dict[str, Any], source: Path
) -> dict[str, dict[int, dict[str, Any]]]:
    actual_configs = {
        variant.get("name"): variant.get("config")
        for variant in summary.get("variants", [])
    }
    if actual_configs != EXPECTED_CONFIGS:
        raise ValueError(
            f"{source}: unexpected variants/configs: "
            f"expected {EXPECTED_CONFIGS!r}, found {actual_configs!r}"
        )
    checked: dict[str, dict[int, dict[str, Any]]] = {}
    for name in VARIANTS:
        checked[name] = variant_runs(summary, name, source)
    expected_seeds = set(range(10))
    for name, runs in checked.items():
        if set(runs) != expected_seeds:
            raise ValueError(
                f"{source}: {name} seeds are {sorted(runs)}, expected 0..9"
            )
    return checked


def validate_summary(
    summary: dict[str, Any], source: Path, load_name: str
) -> dict[str, dict[int, dict[str, Any]]]:
    if summary.get("seeds") != 10 or summary.get("slots") != 2000:
        raise ValueError(
            f"{source}: expected 10 seeds and 2,000 slots, found "
            f"{summary.get('seeds')!r} and {summary.get('slots')!r}"
        )
    # The historical schema has neither field.  Their absence is material:
    # this runner always emitted traces and used its one shared random stream.
    if "summaryOnly" in summary or "randomness" in summary:
        raise ValueError(
            f"{source}: expected the historical sweep schema without "
            "summaryOnly/randomness fields"
        )
    expected_override = LOAD_OVERRIDES[load_name]
    actual_override = summary.get("loadOverride")
    for key, expected in expected_override.items():
        if actual_override.get(key) != expected:
            raise ValueError(
                f"{source}: loadOverride.{key} is {actual_override.get(key)!r}, "
                f"expected {expected!r}"
            )
    return checked_variants(summary, source)


def metric_record(
    summary: dict[str, Any],
    runs: dict[str, dict[int, dict[str, Any]]],
    key: str,
    report_scale: float,
    unit: str,
) -> dict[str, Any]:
    per_seed: dict[str, list[int | float]] = {}
    reported_means: dict[str, float] = {}
    for variant in VARIANTS:
        values = [
            number(runs[variant][seed].get(key), f"{variant} seed {seed} {key}")
            for seed in range(10)
        ]
        per_seed[variant] = values
        matching = [
            item for item in summary["variants"] if item.get("name") == variant
        ]
        aggregate = matching[0].get("aggregates", {}).get(key, {})
        reported_mean = float(
            number(aggregate.get("mean"), f"{variant} aggregate {key}")
        )
        calculated_mean = mean(values)
        if not math.isclose(
            reported_mean, calculated_mean, rel_tol=1.0e-12, abs_tol=1.0e-9
        ):
            raise ValueError(
                f"{variant} {key}: aggregate mean {reported_mean} does not match "
                f"per-seed mean {calculated_mean}"
            )
        reported_means[variant] = reported_mean
    return {
        "unit": unit,
        "reportScale": report_scale,
        "means": reported_means,
        "perSeed": per_seed,
    }


def source_bundle(directory: Path) -> dict[str, Any]:
    summary_path = directory / "summary.json"
    require_file(summary_path)
    effective_configs = {}
    for name in VARIANTS:
        path = directory / f"{name}.config.json"
        require_file(path)
        effective_configs[name] = {
            "path": repo_relative(path),
            "sha256": sha256_file(path),
        }
    bundle: dict[str, Any] = {
        "summary": {
            "path": repo_relative(summary_path),
            "sha256": sha256_file(summary_path),
        },
        "effectiveConfigs": effective_configs,
    }
    selected_profile = directory / "selected-load-profile.json"
    if selected_profile.exists():
        bundle["selectedLoadProfile"] = {
            "path": repo_relative(selected_profile),
            "sha256": sha256_file(selected_profile),
        }
    return bundle


def reference_source(
    summary_path: Path, variant_name: str, metrics: tuple[str, ...]
) -> tuple[dict[str, Any], dict[str, list[int | float]]]:
    summary = load_summary(summary_path)
    runs = variant_runs(summary, variant_name, summary_path)
    if set(runs) != set(range(10)):
        raise ValueError(
            f"{summary_path}: {variant_name} does not contain seeds 0..9"
        )
    values = {
        key: [
            number(runs[seed].get(key), f"{variant_name} seed {seed} {key}")
            for seed in range(10)
        ]
        for key in metrics
    }
    directory = summary_path.parent
    config_path = directory / f"{variant_name}.config.json"
    require_file(config_path)
    provenance: dict[str, Any] = {
        "variant": variant_name,
        "summary": {
            "path": repo_relative(summary_path),
            "sha256": sha256_file(summary_path),
        },
        "effectiveConfig": {
            "path": repo_relative(config_path),
            "sha256": sha256_file(config_path),
        },
    }
    selected_profile = directory / "selected-load-profile.json"
    if selected_profile.exists():
        provenance["selectedLoadProfile"] = {
            "path": repo_relative(selected_profile),
            "sha256": sha256_file(selected_profile),
        }
    return provenance, values


def paired_stats(
    differences: list[float],
    *,
    candidate: str,
    reference: str,
    metric: str,
    unit: str,
    better_when: str = "higher",
    aligned_exogenous_streams: bool = False,
) -> dict[str, Any]:
    if len(differences) != 10:
        raise ValueError(
            f"{candidate} vs {reference} {metric}: expected 10 pairs, "
            f"found {len(differences)}"
        )
    estimate = statistics.mean(differences)
    half_width = (
        T_CRITICAL_95_DF9
        * statistics.stdev(differences)
        / math.sqrt(len(differences))
    )
    if better_when == "higher":
        better_count = sum(value > 0 for value in differences)
    elif better_when == "lower":
        better_count = sum(value < 0 for value in differences)
    else:
        raise ValueError(f"unknown better_when value: {better_when}")
    randomness_assessment = (
        {
            "status": "aligned-exception",
            "basis": (
                "All 20 low-load target-0.25 formula/fixed traces contain zero "
                "TxRejected events, zero TxEvicted events, and zero TxSubmitted "
                "events with attempt > 1. The historical draw schedule therefore "
                "consumes no conditional retry-jitter draws, while the observed "
                "Ranking Block opportunity-slot sequence matches within every "
                "same-seed pair."
            ),
            "interpretation": (
                "The paired statistic retains aligned exogenous raw-demand and "
                "Ranking Block draws within this audited comparison."
            ),
        }
        if aligned_exogenous_streams
        else {
            "status": "potentially-confounded",
            "basis": (
                "The historical runner uses one shared PRNG stream. If compared "
                "paths consume different conditional retry-jitter draws, later "
                "fresh-demand and Ranking Block draws can diverge."
            ),
            "interpretation": (
                "Descriptive full-run comparison unless retry-path alignment is "
                "established separately."
            ),
        }
    )
    return {
        "candidate": candidate,
        "reference": reference,
        "metric": metric,
        "unit": unit,
        "direction": f"{candidate} minus {reference}",
        "method": (
            "Mean same-numbered-seed difference with a two-sided 95% "
            "Student-t interval (df=9, t*=2.2621571627409915)."
        ),
        "randomnessAssessment": randomness_assessment,
        "perSeedDifferences": differences,
        "meanDifference": estimate,
        "ci95": [estimate - half_width, estimate + half_width],
        "ci95HalfWidth": half_width,
        "candidateBetterCount": better_count,
        "ties": sum(value == 0 for value in differences),
    }


def corrected_comparisons(
    results: dict[str, Any],
    corrected_runs: dict[str, dict[str, dict[int, dict[str, Any]]]],
    references: dict[str, Any],
) -> None:
    low = corrected_runs["low"]
    low_comparisons = []
    for key, scale, unit, better_when in (
        ("value.urgent.retainedRatio", 100.0, "percentage points", "higher"),
        ("latency.urgent.meanBlocks", 1.0, "blocks", "lower"),
    ):
        differences = [
            scale
            * (
                numeric_scalar(low["bdst-tu25-d8"][seed], key, "bdst-tu25-d8", seed)
                - numeric_scalar(
                    low["bdst-tu25-d8-fixed-thr"][seed],
                    key,
                    "bdst-tu25-d8-fixed-thr",
                    seed,
                )
            )
            for seed in range(10)
        ]
        low_comparisons.append(
            paired_stats(
                differences,
                candidate="bdst-tu25-d8",
                reference="bdst-tu25-d8-fixed-thr",
                metric=key,
                unit=unit,
                better_when=better_when,
                aligned_exogenous_streams=True,
            )
        )
    results["low"]["comparisons"] = low_comparisons

    severe_anchor = references["severeAnchor"]["perSeed"][
        "value.urgent.retainedRatio"
    ]
    severe_comparisons = []
    for candidate_name in VARIANTS[:3]:
        differences = [
            100.0
            * (
                numeric_scalar(
                    corrected_runs["severe-congestion"][candidate_name][seed],
                    "value.urgent.retainedRatio",
                    candidate_name,
                    seed,
                )
                - float(severe_anchor[seed])
            )
            for seed in range(10)
        ]
        severe_comparisons.append(
            paired_stats(
                differences,
                candidate=candidate_name,
                reference="bdst-tu50-d8",
                metric="value.urgent.retainedRatio",
                unit="percentage points",
            )
        )
    results["severe-congestion"]["comparisons"] = severe_comparisons

    flat = references["launchDayFlatFee"]["perSeed"]
    flat_offered = [
        sum(
            float(flat[key][seed])
            for key in (
                "value.retainedLovelace",
                "value.lostLovelace",
                "value.unresolvedLovelace",
            )
        )
        for seed in range(10)
    ]
    retained_percent: dict[str, list[float]] = {}
    launch_comparisons = []
    for candidate_name in VARIANTS:
        candidate_values = [
            numeric_scalar(
                corrected_runs["launch-day"][candidate_name][seed],
                "value.retainedLovelace",
                candidate_name,
                seed,
            )
            for seed in range(10)
        ]
        retained_percent[candidate_name] = [
            100.0 * candidate_values[seed] / flat_offered[seed]
            for seed in range(10)
        ]
        if candidate_name != "bdst-tu25-d8-fixed-thr":
            differences = [
                100.0
                * (candidate_values[seed] - float(flat["value.retainedLovelace"][seed]))
                / flat_offered[seed]
                for seed in range(10)
            ]
            launch_comparisons.append(
                paired_stats(
                    differences,
                    candidate=candidate_name,
                    reference="flat-fee",
                    metric="value.retainedLovelace / flat-fee submitted value",
                    unit="percentage points",
                )
            )
    results["launch-day"]["derivedMetrics"] = {
        "retainedPercentOfFlatFeeSubmittedValue": {
            "unit": "percent",
            "definition": (
                "100 * candidate value.retainedLovelace / "
                "(flat-fee value.retainedLovelace + value.lostLovelace + "
                "value.unresolvedLovelace), within the same numbered seed."
            ),
            "means": {
                name: statistics.mean(values)
                for name, values in retained_percent.items()
            },
            "perSeed": retained_percent,
        }
    }
    results["launch-day"]["comparisons"] = launch_comparisons

    eb_anchor = references["ebCapacityStressAnchor"]["perSeed"][
        "value.urgent.retainedRatio"
    ]
    candidate_name = "bdst-tu25-d16"
    differences = [
        100.0
        * (
            numeric_scalar(
                corrected_runs["eb-capacity-stress"][candidate_name][seed],
                "value.urgent.retainedRatio",
                candidate_name,
                seed,
            )
            - float(eb_anchor[seed])
        )
        for seed in range(10)
    ]
    results["eb-capacity-stress"]["comparisons"] = [
        paired_stats(
            differences,
            candidate=candidate_name,
            reference="bdst-tu50-d8",
            metric="value.urgent.retainedRatio",
            unit="percentage points",
        )
    ]


def fresh_submission_fingerprint(event: dict[str, Any]) -> list[Any]:
    tx = event["tx"]
    body = tx["body"]
    script = body["script"]
    return [
        body["number"],
        event["slot"],
        event["actorId"],
        body["dependsOn"],
        body["sizeBytes"],
        script["exUnits"],
        script["sizeBytes"],
        tx["urgency"],
        tx["value"],
    ]


def scan_low_trace(
    path: Path,
) -> tuple[dict[str, Any], list[list[Any]], dict[int, list[Any]]]:
    digest = hashlib.sha256()
    fresh_digest = hashlib.sha256()
    ranking_block_slots_digest = hashlib.sha256()
    fresh: list[list[Any]] = []
    fresh_by_raw_demand_number: dict[int, list[Any]] = {}
    ranking_block_slots: list[int] = []
    tx_rejected = 0
    tx_evicted = 0
    retry_submissions = 0
    certificate_rbs = 0
    non_certificate_rbs = 0
    urgent_bytes = 0
    urgent_capacity_bytes = 0
    eb_announcements = 0
    announced_eb_payload_bytes = 0
    with path.open("rb") as stream:
        for line_number, raw in enumerate(stream, 1):
            digest.update(raw)
            try:
                wrapper = json.loads(raw)
            except json.JSONDecodeError as error:
                raise ValueError(f"{path}:{line_number}: invalid JSON: {error}") from error
            event = wrapper.get("event", {})
            event_tag = event.get("tag")
            if event_tag == "TxRejected":
                tx_rejected += 1
            elif event_tag == "TxEvicted":
                tx_evicted += 1
            elif event_tag == "TxSubmitted":
                tx = event.get("tx", {})
                attempt = tx.get("attempt")
                if isinstance(attempt, int) and attempt > 1:
                    retry_submissions += 1
                if tx.get("attempt") == 1:
                    fingerprint = fresh_submission_fingerprint(event)
                    fresh.append(fingerprint)
                    raw_demand_number = int(tx["body"]["number"])
                    if raw_demand_number in fresh_by_raw_demand_number:
                        raise ValueError(
                            f"{path}:{line_number}: duplicate first-attempt raw "
                            f"demand number {raw_demand_number}"
                        )
                    fresh_by_raw_demand_number[raw_demand_number] = fingerprint
                    encoded = json.dumps(
                        fingerprint,
                        sort_keys=True,
                        separators=(",", ":"),
                    ).encode("utf-8")
                    fresh_digest.update(encoded + b"\n")
            if event.get("tag") != "BlockProduced":
                continue
            outer = event["summary"]
            block_tag = outer["tag"]
            summary = outer["summary"]
            if block_tag == "RankingBlockProduced":
                slot = int(event["slot"])
                ranking_block_slots.append(slot)
                ranking_block_slots_digest.update(
                    json.dumps(slot, separators=(",", ":")).encode("utf-8") + b"\n"
                )
                if summary["block"]["tag"] == "CertifyingBlock":
                    certificate_rbs += 1
                else:
                    non_certificate_rbs += 1
                    urgent_bytes += int(summary["priorityBytes"])
                    urgent_capacity_bytes += int(summary["priorityCapacityBytes"])
            elif block_tag == "EndorserBlockAnnounced":
                eb_announcements += 1
                announced_eb_payload_bytes += int(summary["usedBytes"])
    return (
        {
            "source": {
                "path": repo_relative(path),
                "bytes": path.stat().st_size,
                "sha256": digest.hexdigest(),
            },
            "observedFirstAttemptSubmissions": {
                "count": len(fresh),
                "sequenceSha256": fresh_digest.hexdigest(),
            },
            "failureAndRetryEvents": {
                "txRejected": tx_rejected,
                "txEvicted": tx_evicted,
                "txSubmittedAttemptGreaterThanOne": retry_submissions,
            },
            "rankingBlockOpportunitySlots": {
                "count": len(ranking_block_slots),
                "sequenceSha256": ranking_block_slots_digest.hexdigest(),
            },
            "certificateRankingBlocks": certificate_rbs,
            "nonCertificateRankingBlocks": non_certificate_rbs,
            "urgentBytesInNonCertificateRankingBlocks": urgent_bytes,
            "urgentCapacityBytesInNonCertificateRankingBlocks": urgent_capacity_bytes,
            "endorserBlocksAnnounced": eb_announcements,
            "announcedEndorserBlockPayloadBytes": announced_eb_payload_bytes,
        },
        fresh,
        fresh_by_raw_demand_number,
    )


def first_sequence_mismatch(
    left: list[list[Any]], right: list[list[Any]]
) -> int | None:
    for index, (left_item, right_item) in enumerate(zip(left, right)):
        if left_item != right_item:
            return index
    if len(left) != len(right):
        return min(len(left), len(right))
    return None


def low_event_evidence(corrected_root: Path) -> dict[str, Any]:
    load_dir = corrected_root / "low"
    names = ("bdst-tu25-d8", "bdst-tu25-d8-fixed-thr")
    scans: dict[str, list[dict[str, Any]]] = {name: [] for name in names}
    sequences: dict[str, list[list[list[Any]]]] = {name: [] for name in names}
    submissions_by_raw_demand: dict[str, list[dict[int, list[Any]]]] = {
        name: [] for name in names
    }
    for name in names:
        for seed in range(10):
            path = load_dir / f"{name}-seed{seed}.events.jsonl"
            require_file(path)
            record, fresh, fresh_by_raw_demand_number = scan_low_trace(path)
            record["seed"] = seed
            scans[name].append(record)
            sequences[name].append(fresh)
            submissions_by_raw_demand[name].append(fresh_by_raw_demand_number)

    nonzero_failure_or_retry_runs = [
        {
            "variant": name,
            "seed": run["seed"],
            **run["failureAndRetryEvents"],
        }
        for name in names
        for run in scans[name]
        if any(run["failureAndRetryEvents"].values())
    ]
    if nonzero_failure_or_retry_runs:
        raise ValueError(
            "low formula/fixed stream-alignment audit found a rejection, "
            f"eviction, or retry submission: {nonzero_failure_or_retry_runs}"
        )

    aggregates: dict[str, Any] = {}
    for name in names:
        runs = scans[name]
        certs = sum(run["certificateRankingBlocks"] for run in runs)
        non_certs = sum(run["nonCertificateRankingBlocks"] for run in runs)
        urgent_bytes = sum(
            run["urgentBytesInNonCertificateRankingBlocks"] for run in runs
        )
        urgent_capacity = sum(
            run["urgentCapacityBytesInNonCertificateRankingBlocks"] for run in runs
        )
        announcements = sum(run["endorserBlocksAnnounced"] for run in runs)
        payload = sum(run["announcedEndorserBlockPayloadBytes"] for run in runs)
        aggregates[name] = {
            "certificateRankingBlocksMean": certs / 10.0,
            "nonCertificateRankingBlocksMean": non_certs / 10.0,
            "certificateShareOfRankingBlocks": certs / (certs + non_certs),
            "urgentByteFillOfNonCertificateRankingBlocks": (
                urgent_bytes / urgent_capacity
            ),
            "endorserBlocksAnnouncedMean": announcements / 10.0,
            "meanAnnouncedEndorserBlockPayloadKilobytes": (
                payload / announcements / 1000.0
            ),
        }

    pairs = []
    for seed in range(10):
        formula = scans[names[0]][seed]
        fixed = scans[names[1]][seed]
        formula_by_raw_demand = submissions_by_raw_demand[names[0]][seed]
        fixed_by_raw_demand = submissions_by_raw_demand[names[1]][seed]
        common_raw_demand_numbers = sorted(
            formula_by_raw_demand.keys() & fixed_by_raw_demand.keys()
        )
        formula_only_raw_demand_numbers = (
            formula_by_raw_demand.keys() - fixed_by_raw_demand.keys()
        )
        fixed_only_raw_demand_numbers = (
            fixed_by_raw_demand.keys() - formula_by_raw_demand.keys()
        )
        common_attribute_mismatches = [
            raw_demand_number
            for raw_demand_number in common_raw_demand_numbers
            if formula_by_raw_demand[raw_demand_number]
            != fixed_by_raw_demand[raw_demand_number]
        ]
        if common_attribute_mismatches:
            raise ValueError(
                f"seed {seed}: exogenous attributes differ for common observed "
                f"raw-demand numbers {common_attribute_mismatches[:10]}"
            )
        common_digest = hashlib.sha256()
        for raw_demand_number in common_raw_demand_numbers:
            encoded = json.dumps(
                formula_by_raw_demand[raw_demand_number],
                sort_keys=True,
                separators=(",", ":"),
            ).encode("utf-8")
            common_digest.update(encoded + b"\n")
        mismatch = first_sequence_mismatch(
            sequences[names[0]][seed], sequences[names[1]][seed]
        )
        ranking_block_slots_match = (
            formula["rankingBlockOpportunitySlots"]
            == fixed["rankingBlockOpportunitySlots"]
        )
        if not ranking_block_slots_match:
            raise ValueError(
                f"seed {seed}: formula/fixed Ranking Block opportunity slots differ"
            )
        pairs.append(
            {
                "seed": seed,
                "observedSubmissionSequencesMatch": mismatch is None,
                "firstObservedSubmissionMismatchIndex": mismatch,
                "rankingBlockOpportunitySlotsMatch": ranking_block_slots_match,
                "commonObservedRawDemandNumbers": len(common_raw_demand_numbers),
                "formulaOnlyObservedRawDemandNumbers": len(
                    formula_only_raw_demand_numbers
                ),
                "fixedOnlyObservedRawDemandNumbers": len(
                    fixed_only_raw_demand_numbers
                ),
                "commonObservedExogenousAttributeMismatches": len(
                    common_attribute_mismatches
                ),
                "commonObservedSubmissionFingerprintSha256": (
                    common_digest.hexdigest()
                ),
                "formula": {
                    "eventTraceSha256": formula["source"]["sha256"],
                    **formula["observedFirstAttemptSubmissions"],
                    "failureAndRetryEvents": formula["failureAndRetryEvents"],
                    "rankingBlockOpportunitySlots": (
                        formula["rankingBlockOpportunitySlots"]
                    ),
                },
                "fixed": {
                    "eventTraceSha256": fixed["source"]["sha256"],
                    **fixed["observedFirstAttemptSubmissions"],
                    "failureAndRetryEvents": fixed["failureAndRetryEvents"],
                    "rankingBlockOpportunitySlots": (
                        fixed["rankingBlockOpportunitySlots"]
                    ),
                },
            }
        )

    certificate_differences = [
        float(scans[names[0]][seed]["certificateRankingBlocks"])
        - float(scans[names[1]][seed]["certificateRankingBlocks"])
        for seed in range(10)
    ]
    return {
        "method": (
            "Stream each low-load JSONL trace. A certificate Ranking Block is a "
            "RankingBlockProduced event whose nested block tag is "
            "CertifyingBlock; every other RankingBlockProduced event is "
            "non-certificate. Urgent fill is total priorityBytes divided by "
            "total priorityCapacityBytes across non-certificate RBs. Mean "
            "announced EB payload is total usedBytes divided by the number of "
            "EndorserBlockAnnounced events, using decimal kilobytes."
        ),
        "perSeed": scans,
        "aggregates": aggregates,
        "certificateRankingBlockFormulaMinusFixed": paired_stats(
            certificate_differences,
            candidate=names[0],
            reference=names[1],
            metric="certificate Ranking Blocks",
            unit="blocks",
            better_when="lower",
            aligned_exogenous_streams=True,
        ),
        "streamAlignmentAndObservedSubmissionResponse": {
            "scope": "low-load target-0.25 formula-versus-fixed pair",
            "status": "aligned-exception",
            "drawScheduleAudit": {
                "sourceState": (
                    f"historical base {BASE_REVISION} plus the recorded "
                    f"{PATCH_SHA256} patch; patched app/src digest "
                    f"{SOURCE_TREE_SHA256}"
                ),
                "freshDemand": (
                    "In Sim.actorStep, sampleArrivalCount first consumes draws "
                    "based only on the configured workload rate and PRNG values. "
                    "Each resulting raw arrival then consumes one pickActor draw "
                    "and the five drawTxSample draws before generateTransaction "
                    "may make the price-dependent decision to submit or decline. "
                    "That decision consumes no PRNG draw."
                ),
                "rankingBlocks": (
                    "In Sim.blockStep, rollRbProduction consumes exactly one "
                    "Ranking Block draw per slot, independent of the mechanism "
                    "outcome."
                ),
                "conditionalDraws": (
                    "The remaining mechanism-dependent draw site is retry jitter "
                    "in captureRetries, once per queued retry."
                ),
            },
            "observedConditionalPathCounts": {
                "traces": 20,
                "txRejected": sum(
                    run["failureAndRetryEvents"]["txRejected"]
                    for name in names
                    for run in scans[name]
                ),
                "txEvicted": sum(
                    run["failureAndRetryEvents"]["txEvicted"]
                    for name in names
                    for run in scans[name]
                ),
                "txSubmittedAttemptGreaterThanOne": sum(
                    run["failureAndRetryEvents"][
                        "txSubmittedAttemptGreaterThanOne"
                    ]
                    for name in names
                    for run in scans[name]
                ),
            },
            "rankingBlockOpportunityMatchingPairs": sum(
                pair["rankingBlockOpportunitySlotsMatch"] for pair in pairs
            ),
            "alignmentInference": (
                "The same seed, fixed per-arrival and per-slot draw schedule, "
                "zero observed failure/retry paths, and matching Ranking Block "
                "opportunity slots establish that raw-demand and Ranking Block "
                "streams remained aligned in all ten pairs."
            ),
            "observedSubmissionFingerprintMethod": (
                "For each first-attempt TxSubmitted event, hash the ordered "
                "canonical-JSON sequence of [raw-demand body.number, slot, "
                "actorId, dependsOn, tx size, script ex-units, script bytes, "
                "urgency, value]. Mechanism-dependent tx id, lane, fee, and "
                "retry metadata are excluded. Common body.number values have "
                "zero exogenous-attribute mismatches in every pair."
            ),
            "observedSubmissionInterpretation": (
                "The eight differing observed-submission subsequences reflect "
                "price-dependent submit/decline decisions: declined demand has "
                "no TxSubmitted event. They do not show RNG divergence."
            ),
            "observedSubmissionMatchingPairs": sum(
                pair["observedSubmissionSequencesMatch"] for pair in pairs
            ),
            "observedSubmissionDifferingPairs": sum(
                not pair["observedSubmissionSequencesMatch"] for pair in pairs
            ),
            "commonObservedExogenousAttributeMismatches": sum(
                pair["commonObservedExogenousAttributeMismatches"] for pair in pairs
            ),
            "pairs": pairs,
        },
    }


def announced_eb_diagnostic(path: Path) -> dict[str, Any]:
    announcements = []
    with path.open("rb") as stream:
        for line_number, raw in enumerate(stream, 1):
            try:
                wrapper = json.loads(raw)
            except json.JSONDecodeError as error:
                raise ValueError(f"{path}:{line_number}: invalid JSON: {error}") from error
            event = wrapper.get("event", {})
            if event.get("tag") != "BlockProduced":
                continue
            outer = event["summary"]
            if outer["tag"] != "EndorserBlockAnnounced":
                continue
            summary = outer["summary"]
            announcements.append(
                {
                    "slot": event["slot"],
                    "eventNo": wrapper["eventNo"],
                    "endorserBlockId": summary["id"],
                    "usedBytes": summary["usedBytes"],
                }
            )
    return {
        "count": len(announcements),
        "first": announcements[0] if announcements else None,
        "meanPayloadBytes": (
            statistics.mean(item["usedBytes"] for item in announcements)
            if announcements
            else None
        ),
    }


def severe_trace_identity(
    corrected_root: Path,
    runs: dict[str, dict[int, dict[str, Any]]],
) -> dict[str, Any]:
    load_dir = corrected_root / "severe-congestion"
    formula_name = "bdst-tu25-d8"
    fixed_name = "bdst-tu25-d8-fixed-thr"
    pairs = []
    for seed in range(10):
        formula_path = load_dir / f"{formula_name}-seed{seed}.events.jsonl"
        fixed_path = load_dir / f"{fixed_name}-seed{seed}.events.jsonl"
        require_file(formula_path)
        require_file(fixed_path)
        formula_hash = sha256_file(formula_path)
        fixed_hash = sha256_file(fixed_path)
        scalar_differences = {
            key: {
                "formula": runs[formula_name][seed][key],
                "fixed": runs[fixed_name][seed][key],
            }
            for key in sorted(runs[formula_name][seed])
            if runs[formula_name][seed][key] != runs[fixed_name][seed][key]
        }
        pair: dict[str, Any] = {
            "seed": seed,
            "bitIdentical": (
                formula_path.stat().st_size == fixed_path.stat().st_size
                and formula_hash == fixed_hash
            ),
            "formula": {
                "path": repo_relative(formula_path),
                "bytes": formula_path.stat().st_size,
                "sha256": formula_hash,
            },
            "fixed": {
                "path": repo_relative(fixed_path),
                "bytes": fixed_path.stat().st_size,
                "sha256": fixed_hash,
            },
            "summaryScalarDifferences": scalar_differences,
        }
        if not pair["bitIdentical"]:
            pair["announcedEndorserBlocks"] = {
                "formula": announced_eb_diagnostic(formula_path),
                "fixed": announced_eb_diagnostic(fixed_path),
            }
        pairs.append(pair)
    return {
        "method": (
            "Compare byte length and SHA-256 of each same-seed formula/fixed "
            "JSONL pair; compare all 55 summary scalars directly. For a "
            "non-identical pair, also preserve the announcement count, first "
            "announcement witness, and mean announced payload."
        ),
        "bitIdenticalPairs": sum(pair["bitIdentical"] for pair in pairs),
        "differingPairs": sum(not pair["bitIdentical"] for pair in pairs),
        "pairs": pairs,
    }


def equivalent_reproduction_commands() -> list[str]:
    patch = "docs/phase-2/experiment-results/param-robustness-tu25-headroom.patch"
    manifest = "abstract-sim-hs/config/sweeps/param-robustness-tu25-correction.json"
    return [
        "set -eu",
        'repo_root="$(git rev-parse --show-toplevel)"',
        'legacy_root="$repo_root/abstract-sim-hs/sweep-results"',
        'corrected_root="$legacy_root/param-robustness-tu25-correction"',
        'replay_parent="$(mktemp -d /tmp/arc-tu25-replay.XXXXXX)"',
        'replay_root="$replay_parent/worktree"',
        'test ! -e "$legacy_root/param-robustness-severe-congestion"',
        'test ! -e "$legacy_root/param-robustness-eb-capacity-stress"',
        'test ! -e "$legacy_root/launch-day"',
        'test ! -e "$corrected_root"',
        'git -C "$repo_root" worktree add --detach "$replay_root" '
        + BASE_REVISION,
        'cd "$replay_root/abstract-sim-hs"',
        "stack build",
        'legacy_simulator="$(stack path --local-install-root)/bin/abstract-sim-hs-exe"',
        '"$legacy_simulator" sweep config/sweeps/param-robustness.json '
        "--load-profile config/loads/severe-congestion.json "
        '--out "$legacy_root/param-robustness-severe-congestion"',
        '"$legacy_simulator" sweep config/sweeps/param-robustness.json '
        "--load-profile config/loads/eb-capacity-stress.json "
        '--out "$legacy_root/param-robustness-eb-capacity-stress"',
        '"$legacy_simulator" sweep config/sweeps/launch-day.json '
        "--load-profile config/loads/launch-day.json "
        '--out "$legacy_root/launch-day"',
        f'git -C "$replay_root" apply "$repo_root/{patch}"',
        f'cp "$repo_root/{manifest}" "$replay_root/{manifest}"',
        "stack build",
        'simulator="$(stack path --local-install-root)/bin/abstract-sim-hs-exe"',
        '"$simulator" sweep config/sweeps/param-robustness-tu25-correction.json '
        '--load low --out "$corrected_root/low"',
        '"$simulator" sweep config/sweeps/param-robustness-tu25-correction.json '
        "--load-profile config/loads/severe-congestion.json "
        '--out "$corrected_root/severe-congestion"',
        '"$simulator" sweep config/sweeps/param-robustness-tu25-correction.json '
        "--load-profile config/loads/launch-day.json "
        '--out "$corrected_root/launch-day"',
        '"$simulator" sweep config/sweeps/param-robustness-tu25-correction.json '
        "--load-profile config/loads/eb-capacity-stress.json "
        '--out "$corrected_root/eb-capacity-stress"',
        'python3 "$repo_root/abstract-sim-hs/scripts/'
        'preserve_param_robustness_tu25.py" --generated-at 2026-07-29 '
        '--corrected-root "$corrected_root" --legacy-root "$legacy_root" '
        '--historical-worktree "$replay_root" --simulator "$simulator" '
        "--json-output "
        "/tmp/param-robustness-tu25-correction.reproduced.json",
        'cmp "$repo_root/docs/phase-2/experiment-results/'
        'param-robustness-tu25-correction.json" '
        "/tmp/param-robustness-tu25-correction.reproduced.json",
    ]


def build_record(args: argparse.Namespace) -> dict[str, Any]:
    require_hash(PATCH_PATH, PATCH_SHA256)
    if PATCH_PATH.stat().st_size != PATCH_BYTES:
        raise ValueError(
            f"preserved patch is {PATCH_PATH.stat().st_size} bytes, "
            f"expected {PATCH_BYTES}"
        )
    require_hash(MANIFEST_PATH, MANIFEST_SHA256)
    if args.historical_worktree is not None:
        verify_historical_source(args.historical_worktree)
    if args.simulator is not None:
        require_hash(args.simulator, SIMULATOR_SHA256)

    corrected_runs: dict[str, dict[str, dict[int, dict[str, Any]]]] = {}
    results: dict[str, Any] = {}
    source_outputs: dict[str, Any] = {}
    for load_name, metrics in LOAD_METRICS.items():
        directory = args.corrected_root / load_name
        summary_path = directory / "summary.json"
        summary = load_summary(summary_path)
        runs = validate_summary(summary, summary_path, load_name)
        corrected_runs[load_name] = runs
        results[load_name] = {
            "seeds": list(range(10)),
            "metrics": {
                key: metric_record(summary, runs, key, scale, unit)
                for key, scale, unit in metrics
            },
        }
        source_outputs[load_name] = source_bundle(directory)

    severe_source, severe_values = reference_source(
        args.legacy_root / "param-robustness-severe-congestion/summary.json",
        "bdst-tu50-d8",
        ("value.urgent.retainedRatio",),
    )
    eb_source, eb_values = reference_source(
        args.legacy_root / "param-robustness-eb-capacity-stress/summary.json",
        "bdst-tu50-d8",
        ("value.urgent.retainedRatio",),
    )
    launch_source, launch_values = reference_source(
        args.legacy_root / "launch-day/summary.json",
        "flat-fee",
        (
            "value.retainedLovelace",
            "value.lostLovelace",
            "value.unresolvedLovelace",
            "units.serviceRate",
        ),
    )
    references = {
        "severeAnchor": {
            "source": severe_source,
            "perSeed": severe_values,
        },
        "ebCapacityStressAnchor": {
            "source": eb_source,
            "perSeed": eb_values,
        },
        "launchDayFlatFee": {
            "source": launch_source,
            "perSeed": launch_values,
        },
    }
    corrected_comparisons(results, corrected_runs, references)

    event_evidence = {
        "lowThresholdPair": low_event_evidence(args.corrected_root),
        "severeThresholdPairTraceIdentity": severe_trace_identity(
            args.corrected_root, corrected_runs["severe-congestion"]
        ),
    }

    return {
        "description": (
            "Compact preserved evidence for the corrected target-utilisation-0.25 "
            "subset of the historical parameter-robustness sweep. It retains the "
            "per-seed inputs needed to recompute every corrected report row and "
            "paired interval, plus event-derived threshold accounting and source "
            "hashes, without committing the roughly 17 GiB raw traces."
        ),
        "generatedAt": args.generated_at,
        "experiment": {
            "seeds": list(range(10)),
            "slots": 2000,
            "loads": list(LOAD_METRICS),
            "variants": list(VARIANTS),
            "manifest": {
                "path": repo_relative(MANIFEST_PATH),
                "bytes": MANIFEST_PATH.stat().st_size,
                "sha256": MANIFEST_SHA256,
            },
            "historicalSource": {
                "baseGitRevision": BASE_REVISION,
                "patch": {
                    "path": repo_relative(PATCH_PATH),
                    "bytes": PATCH_BYTES,
                    "sha256": PATCH_SHA256,
                    "scope": (
                        "Only the seven-line worstCaseNextPrices scale-formula "
                        "hunk preserved in this file; it predates the later "
                        "before/after price-floor refinement and its tests."
                    ),
                },
                "simulatorSourceTreeSha256": SOURCE_TREE_SHA256,
                "sourceTreeHashMethod": (
                    "SHA-256 of the sorted GNU sha256sum records for every .hs "
                    "file under abstract-sim-hs/app and abstract-sim-hs/src."
                ),
                "simulatorExecutableSha256": SIMULATOR_SHA256,
            },
            "randomness": {
                "mode": "shared",
                "explicitInManifest": False,
                "explicitInSummary": False,
                "basis": (
                    "At the historical base revision, the sweep runner calls "
                    "runWithSeedToFile and has no randomness-mode field. Shared "
                    "is therefore the runner's only, implicit mode."
                ),
                "consequence": (
                    "If mechanism paths consume different conditional retry-jitter "
                    "draws, same-numbered arms can receive different later fresh "
                    "demand or Ranking Block opportunities; comparisons without a "
                    "path-alignment audit are therefore descriptive. The low-load "
                    "target-0.25 formula/fixed comparison is an explicit aligned "
                    "exception: all 20 traces have zero rejection, eviction, and "
                    "attempt-greater-than-one events, and every same-seed pair has "
                    "the same Ranking Block opportunity slots."
                ),
            },
            "invocationProvenance": {
                "originalCommandsRecorded": False,
                "cleanOutputDirectoryAssumption": (
                    "The three gitignored legacy output directories and the "
                    "target-0.25 correction output directory named in the "
                    "commands must not exist before the sequence starts. The "
                    "test commands enforce that assumption so stale files "
                    "cannot be mistaken for regenerated inputs."
                ),
                "note": (
                    "The commands below are equivalent reproduction commands "
                    "reconstructed from the preserved manifest, effective inputs, "
                    "historical source state, and output metadata. They are not "
                    "claimed to be a contemporaneous log of the original shell "
                    "invocations."
                ),
                "equivalentReproductionCommands": equivalent_reproduction_commands(),
            },
        },
        "sourceOutputs": source_outputs,
        "references": references,
        "results": results,
        "eventEvidence": event_evidence,
        "caveats": [
            (
                "The raw summaries and traces are gitignored. Their hashes identify "
                "the extraction inputs, while this record preserves the selected "
                "per-seed scalars and event totals needed by the report."
            ),
            (
                "The low-load target-0.25 formula/fixed event audit establishes "
                "aligned exogenous streams from the historical draw schedule, zero "
                "failure/retry events in all 20 traces, and matching Ranking Block "
                "opportunity slots. First-attempt TxSubmitted subsequences still "
                "differ in eight pairs because submission is a price-dependent "
                "filter over aligned raw demand; declined demand is not traced. "
                "Those differences do not demonstrate RNG divergence."
            ),
            (
                "The historical replay patch is narrower than the final source "
                "correction: it changes only the target-aware upward multiplier. "
                "The exact patch, source-tree hash, and executable hash are recorded "
                "to avoid representing later code as the run state."
            ),
        ],
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--corrected-root",
        type=Path,
        default=PROJECT_DIR / "sweep-results/param-robustness-tu25-correction",
    )
    parser.add_argument(
        "--legacy-root",
        type=Path,
        default=PROJECT_DIR / "sweep-results",
    )
    parser.add_argument(
        "--json-output",
        type=Path,
        default=(
            REPO_ROOT
            / "docs/phase-2/experiment-results/"
            "param-robustness-tu25-correction.json"
        ),
    )
    parser.add_argument(
        "--historical-worktree",
        type=Path,
        help=(
            "optional detached 4d5b5abc worktree; when supplied, verify HEAD, "
            "the exact patch, manifest, and patched app/src tree hash"
        ),
    )
    parser.add_argument(
        "--simulator",
        type=Path,
        help="optional historical replay executable to verify by SHA-256",
    )
    parser.add_argument("--generated-at", required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    record = build_record(args)
    args.json_output.parent.mkdir(parents=True, exist_ok=True)
    args.json_output.write_text(json.dumps(record, indent=1) + "\n", encoding="utf-8")
    diagnostic = record["eventEvidence"]["lowThresholdPair"][
        "streamAlignmentAndObservedSubmissionResponse"
    ]
    print(
        f"wrote {args.json_output} "
        "(low formula/fixed exogenous streams aligned; "
        f"{diagnostic['observedSubmissionDifferingPairs']}/10 endogenous "
        "observed-submission subsequences differ)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
