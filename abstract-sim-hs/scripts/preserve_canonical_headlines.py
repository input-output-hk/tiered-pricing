#!/usr/bin/env python3
"""Preserve the decision-facing canonical-headline evidence outside sweep-results.

The raw five-load sweep summaries are intentionally gitignored.  This script
validates their aggregate comparison, retains the per-seed scalars needed to
recompute every headline row, records hashes for the raw summaries and their
effective inputs, and preserves the exact dirty-worktree patch captured by the
comparison runner.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import statistics
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_DIR = SCRIPT_DIR.parent
REPO_ROOT = PROJECT_DIR.parent
DEFAULT_ROOT = PROJECT_DIR / "sweep-results" / "canonical-headlines"
DEFAULT_JSON_OUTPUT = (
    REPO_ROOT
    / "docs"
    / "phase-2"
    / "experiment-results"
    / "canonical-headlines.json"
)
DEFAULT_PATCH_OUTPUT = (
    REPO_ROOT
    / "docs"
    / "phase-2"
    / "experiment-results"
    / "canonical-headlines-source.patch"
)

FLAT = "flat-fee"
CANONICAL = "canonical-final-d16-k10"
LOADS = (
    ("low", "Low"),
    ("mid-load", "Mid load"),
    ("severe-congestion", "Severe congestion"),
    ("eb-capacity-stress", "EB-capacity stress"),
    ("launch-day", "Launch day"),
)
SEEDS = list(range(10))
SLOTS = 2000
# The archived comparison runner uses its table's three-decimal critical value.
T_CRITICAL_95_DF9 = 2.262
PRESERVED_SCALARS = (
    "value.urgent.retainedRatio",
    "latency.urgent.meanBlocks",
    "value.retainedLovelace",
    "value.lostLovelace",
    "value.unresolvedLovelace",
)


def repo_relative(path: Path) -> str:
    resolved = path.resolve()
    try:
        return str(resolved.relative_to(REPO_ROOT.resolve()))
    except ValueError:
        return str(resolved)


def require_file(path: Path) -> None:
    if not path.is_file():
        raise FileNotFoundError(path)


def sha256_bytes(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def file_record(path: Path) -> dict[str, Any]:
    require_file(path)
    content = path.read_bytes()
    return {
        "path": repo_relative(path),
        "bytes": len(content),
        "sha256": sha256_bytes(content),
    }


def load_json(path: Path) -> dict[str, Any]:
    require_file(path)
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise ValueError(f"{path}: invalid JSON: {error}") from error
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected a JSON object")
    return value


def numeric(value: Any, *, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"{context}: expected a number")
    result = float(value)
    if not math.isfinite(result):
        raise ValueError(f"{context}: expected a finite number")
    return result


def selected_scalars(run: dict[str, Any], *, variant: str, seed: int) -> dict[str, float]:
    scalars = run.get("scalars")
    if not isinstance(scalars, dict):
        raise ValueError(f"{variant}, seed {seed}: missing scalar object")
    return {
        key: numeric(
            scalars.get(key),
            context=f"{variant}, seed {seed}, scalar {key}",
        )
        for key in PRESERVED_SCALARS
    }


def variant_runs(
    summary: dict[str, Any],
    *,
    load: str,
) -> dict[str, dict[int, dict[str, float]]]:
    variants = summary.get("variants")
    if not isinstance(variants, list):
        raise ValueError(f"{load}: missing variants")
    result: dict[str, dict[int, dict[str, float]]] = {}
    for variant in variants:
        if not isinstance(variant, dict) or not isinstance(variant.get("name"), str):
            raise ValueError(f"{load}: malformed variant")
        name = variant["name"]
        runs = variant.get("runs")
        if not isinstance(runs, list):
            raise ValueError(f"{load}/{name}: missing runs")
        by_seed: dict[int, dict[str, float]] = {}
        for run in runs:
            if not isinstance(run, dict) or not isinstance(run.get("seed"), int):
                raise ValueError(f"{load}/{name}: malformed run")
            seed = run["seed"]
            if seed in by_seed:
                raise ValueError(f"{load}/{name}: duplicate seed {seed}")
            by_seed[seed] = selected_scalars(run, variant=name, seed=seed)
        if sorted(by_seed) != SEEDS:
            raise ValueError(
                f"{load}/{name}: expected seeds {SEEDS}, found {sorted(by_seed)}"
            )
        result[name] = by_seed
    if set(result) != {FLAT, CANONICAL}:
        raise ValueError(
            f"{load}: expected variants {FLAT!r} and {CANONICAL!r}, "
            f"found {sorted(result)}"
        )
    return result


def paired_stats(
    flat: list[float],
    canonical: list[float],
    *,
    unit: str,
    better_when: str,
) -> dict[str, Any]:
    if len(flat) != 10 or len(canonical) != 10:
        raise ValueError("headline comparisons require ten paired values")
    differences = [
        candidate - reference
        for reference, candidate in zip(flat, canonical, strict=True)
    ]
    estimate = statistics.mean(differences)
    margin = (
        T_CRITICAL_95_DF9
        * statistics.stdev(differences)
        / math.sqrt(len(differences))
    )
    if better_when == "higher":
        wins = sum(value > 0 for value in differences)
    elif better_when == "lower":
        wins = sum(value < 0 for value in differences)
    else:
        raise ValueError(f"unknown better_when value: {better_when}")
    return {
        "unit": unit,
        "flatMean": statistics.mean(flat),
        "canonicalMean": statistics.mean(canonical),
        "canonicalMinusFlat": {
            "perSeed": differences,
            "mean": estimate,
            "ci95": [estimate - margin, estimate + margin],
            "ci95HalfWidth": margin,
        },
        "canonicalBetterCount": wins,
        "ties": sum(value == 0 for value in differences),
    }


def assert_close(actual: float, expected: Any, *, context: str) -> None:
    expected_number = numeric(expected, context=context)
    if not math.isclose(actual, expected_number, rel_tol=1e-12, abs_tol=1e-12):
        raise ValueError(
            f"{context}: recomputed {actual!r}, archived {expected_number!r}"
        )


def validate_against_archived_comparison(
    load: str,
    retained: dict[str, Any],
    latency: dict[str, Any],
    comparison: dict[str, Any],
) -> None:
    retained_rows = {
        row["load"]: row
        for row in comparison.get("retained_value", [])
        if isinstance(row, dict) and isinstance(row.get("load"), str)
    }
    if load not in retained_rows:
        raise ValueError(f"archived comparison has no retained-value row for {load}")
    archived_retained = retained_rows[load]
    assert_close(
        retained["flatMean"],
        archived_retained.get("flat_mean_percent"),
        context=f"{load} retained flat mean",
    )
    assert_close(
        retained["canonicalMean"],
        archived_retained.get("canonical_mean_percent"),
        context=f"{load} retained canonical mean",
    )
    retained_difference = retained["canonicalMinusFlat"]
    archived_difference = archived_retained.get("canonical_minus_flat", {})
    assert_close(
        retained_difference["mean"],
        archived_difference.get("mean"),
        context=f"{load} retained paired mean",
    )
    assert_close(
        retained_difference["ci95"][0],
        archived_difference.get("ci95_low"),
        context=f"{load} retained CI low",
    )
    assert_close(
        retained_difference["ci95"][1],
        archived_difference.get("ci95_high"),
        context=f"{load} retained CI high",
    )
    if retained["canonicalBetterCount"] != archived_retained.get("canonical_wins"):
        raise ValueError(f"{load}: retained-value win count does not match")

    if load == "launch-day":
        return
    latency_rows = {
        row["load"]: row
        for row in comparison.get("urgent_latency", [])
        if isinstance(row, dict) and isinstance(row.get("load"), str)
    }
    if load not in latency_rows:
        raise ValueError(f"archived comparison has no latency row for {load}")
    archived_latency = latency_rows[load]
    assert_close(
        latency["flatMean"],
        archived_latency.get("flat_mean"),
        context=f"{load} latency flat mean",
    )
    assert_close(
        latency["canonicalMean"],
        archived_latency.get("canonical_mean"),
        context=f"{load} latency canonical mean",
    )
    latency_difference = latency["canonicalMinusFlat"]
    archived_difference = archived_latency.get("canonical_minus_flat", {})
    assert_close(
        latency_difference["mean"],
        archived_difference.get("mean"),
        context=f"{load} latency paired mean",
    )
    assert_close(
        latency_difference["ci95"][0],
        archived_difference.get("ci95_low"),
        context=f"{load} latency CI low",
    )
    assert_close(
        latency_difference["ci95"][1],
        archived_difference.get("ci95_high"),
        context=f"{load} latency CI high",
    )
    if latency["canonicalBetterCount"] != archived_latency.get("canonical_wins"):
        raise ValueError(f"{load}: latency win count does not match")


def effective_input_records(root: Path, load: str) -> list[dict[str, Any]]:
    load_dir = root / load
    paths = [
        load_dir / f"{FLAT}.config.json",
        load_dir / f"{CANONICAL}.config.json",
    ]
    selected_profile = load_dir / "selected-load-profile.json"
    if selected_profile.exists():
        paths.append(selected_profile)
    return [file_record(path) for path in paths]


def build_record(
    *,
    root: Path,
    generated_at: str,
    patch_output: Path,
) -> dict[str, Any]:
    comparison_path = root / "comparison.json"
    comparison = load_json(comparison_path)
    if comparison.get("seeds") != SEEDS or comparison.get("slots") != SLOTS:
        raise ValueError("archived comparison does not use seeds 0-9 and 2,000 slots")
    if comparison.get("variants") != {
        "control": FLAT,
        "candidate": CANONICAL,
    }:
        raise ValueError("archived comparison variant pair does not match")

    provenance = comparison.get("provenance")
    if not isinstance(provenance, dict):
        raise ValueError("archived comparison is missing provenance")
    source_patch = root / "source.patch"
    dirty_patch = provenance.get("dirty_patch")
    source_patch_provenance: dict[str, Any] | None
    if dirty_patch is None:
        if provenance.get("abstract_sim_worktree_clean") is not True:
            raise ValueError(
                "comparison has no dirty patch but does not record a clean worktree"
            )
        if source_patch.exists():
            raise ValueError(
                "comparison records a clean worktree but its output contains "
                "a possibly stale source.patch"
            )
        source_patch_provenance = None
    elif isinstance(dirty_patch, dict):
        source_patch_record = file_record(source_patch)
        if (
            dirty_patch.get("bytes") != source_patch_record["bytes"]
            or dirty_patch.get("sha256") != source_patch_record["sha256"]
        ):
            raise ValueError("source.patch does not match comparison provenance")
        patch_output.parent.mkdir(parents=True, exist_ok=True)
        patch_output.write_bytes(source_patch.read_bytes())
        source_patch_provenance = {
            "source": source_patch_record,
            "preserved": file_record(patch_output),
        }
    else:
        raise ValueError("comparison has malformed dirty-patch provenance")

    results: dict[str, Any] = {}
    summary_records: dict[str, Any] = {}
    for load, label in LOADS:
        summary_path = root / load / "summary.json"
        summary = load_json(summary_path)
        if summary.get("randomness") != "independent-streams":
            raise ValueError(f"{load}: expected independent-streams randomness")
        if (
            summary.get("seeds") != 10
            or summary.get("slots") != SLOTS
            or summary.get("summaryOnly") is not True
        ):
            raise ValueError(
                f"{load}: expected ten seeds, 2,000 slots, and summary-only output"
            )
        runs = variant_runs(summary, load=load)

        per_seed = []
        flat_retained: list[float] = []
        canonical_retained: list[float] = []
        flat_latency: list[float] = []
        canonical_latency: list[float] = []
        for seed in SEEDS:
            flat = runs[FLAT][seed]
            canonical = runs[CANONICAL][seed]
            if load == "launch-day":
                denominator = (
                    flat["value.retainedLovelace"]
                    + flat["value.lostLovelace"]
                    + flat["value.unresolvedLovelace"]
                )
                if denominator <= 0:
                    raise ValueError(f"launch-day seed {seed}: non-positive denominator")
                flat_retained_value = 100.0 * flat["value.retainedLovelace"] / denominator
                canonical_retained_value = (
                    100.0 * canonical["value.retainedLovelace"] / denominator
                )
            else:
                denominator = None
                flat_retained_value = 100.0 * flat["value.urgent.retainedRatio"]
                canonical_retained_value = (
                    100.0 * canonical["value.urgent.retainedRatio"]
                )
            flat_retained.append(flat_retained_value)
            canonical_retained.append(canonical_retained_value)
            flat_latency.append(flat["latency.urgent.meanBlocks"])
            canonical_latency.append(canonical["latency.urgent.meanBlocks"])
            per_seed.append(
                {
                    "seed": seed,
                    "flat": flat,
                    "canonical": canonical,
                    "derived": {
                        "retainedValueFlatPercent": flat_retained_value,
                        "retainedValueCanonicalPercent": canonical_retained_value,
                        "launchDayFlatSubmittedValueDenominator": denominator,
                    },
                }
            )

        retained = paired_stats(
            flat_retained,
            canonical_retained,
            unit="percentage points",
            better_when="higher",
        )
        latency = paired_stats(
            flat_latency,
            canonical_latency,
            unit="blocks",
            better_when="lower",
        )
        validate_against_archived_comparison(
            load,
            retained,
            latency,
            comparison,
        )
        summary_records[load] = {
            "summary": file_record(summary_path),
            "effectiveInputs": effective_input_records(root, load),
        }
        results[load] = {
            "label": label,
            "retainedValueDefinition": (
                "For each seed, both variants' overall retained-value numerators "
                "are divided by the flat-fee retained + lost + unresolved value."
                if load == "launch-day"
                else "Urgent-class retained / (retained + lost), excluding unresolved."
            ),
            "retainedValue": retained,
            "urgentLatency": latency,
            "perSeedInputs": per_seed,
        }

    simulator_sha256 = provenance.get("simulator_sha256")
    if not isinstance(simulator_sha256, str) or len(simulator_sha256) != 64:
        raise ValueError("archived comparison has no valid simulator SHA-256")

    return {
        "description": (
            "Durable evidence for the independent-stream five-load flat-fee "
            "versus canonical D16/K10 headline rerun."
        ),
        "generatedAt": generated_at,
        "experiment": {
            "variants": {"control": FLAT, "candidate": CANONICAL},
            "seeds": SEEDS,
            "slots": SLOTS,
            "randomness": "independent-streams",
            "method": (
                "Mean same-numbered-seed difference with a two-sided 95% "
                "Student-t interval (df=9, t*=2.262), matching the archived runner."
            ),
        },
        "provenance": {
            "baseGitRevision": provenance.get("git_revision"),
            "abstractSimWorktreeCleanAtComparison": provenance.get(
                "abstract_sim_worktree_clean"
            ),
            "simulatorExecutableSha256": simulator_sha256,
            "comparisonTimeSourcePatch": source_patch_provenance,
            "scopeLimitation": (
                (
                    "The runner recorded the simulator hash before the sweeps and "
                    "captured source.patch when it generated the comparison. The "
                    "original shell invocation was not separately logged, and an "
                    "intervening worktree edit would not be observable; the patch "
                    "therefore identifies the comparison-time tree while the "
                    "executable hash identifies the binary that ran."
                )
                if source_patch_provenance is not None
                else (
                    "The runner recorded the simulator hash before the sweeps "
                    "and a clean worktree at comparison time. The base revision "
                    "therefore identifies the comparison-time tree, while the "
                    "executable hash identifies the binary that ran; the original "
                    "shell invocation and any intervening edits were not logged."
                )
            ),
        },
        "sourceOutputs": {
            "aggregateComparison": file_record(comparison_path),
            "loads": summary_records,
        },
        "results": results,
        "archivedAggregateComparison": comparison,
        "reproduction": {
            "originalCommandsRecorded": False,
            "equivalentCurrentRunnerCommand": (
                "cd abstract-sim-hs && ./scripts/run_canonical_headlines.sh "
                "--out sweep-results/canonical-headlines-rerun"
            ),
            "preservationCommand": (
                "python3 abstract-sim-hs/scripts/preserve_canonical_headlines.py "
                "--root abstract-sim-hs/sweep-results/canonical-headlines-rerun "
                f"--generated-at {generated_at}"
            ),
            "note": (
                "These are reconstructed equivalent commands, not a "
                "contemporaneous invocation log. A fresh rerun estimates the "
                "current model; it is not expected to reproduce a different "
                "source state byte-for-byte."
            ),
        },
        "caveats": [
            (
                "Raw summaries remain gitignored. Their hashes identify the "
                "extraction inputs; this record preserves every per-seed scalar "
                "needed to recompute the report's retained-value and latency rows."
            ),
            (
                "The launch-day denominator is a flat-fee proxy for submitted "
                "offered value. Fresh samples that decline before first "
                "submission are not present in summary output."
            ),
            "Producer withholding is not modelled.",
        ],
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument("--json-output", type=Path, default=DEFAULT_JSON_OUTPUT)
    parser.add_argument("--patch-output", type=Path, default=DEFAULT_PATCH_OUTPUT)
    parser.add_argument("--generated-at", required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    record = build_record(
        root=args.root.resolve(),
        generated_at=args.generated_at,
        patch_output=args.patch_output.resolve(),
    )
    args.json_output.parent.mkdir(parents=True, exist_ok=True)
    args.json_output.write_text(
        json.dumps(record, indent=1) + "\n",
        encoding="utf-8",
    )
    print(f"wrote {args.json_output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
