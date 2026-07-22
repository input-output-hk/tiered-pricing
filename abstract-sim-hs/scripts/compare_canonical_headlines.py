#!/usr/bin/env python3
"""Build the paired canonical refresh report for the five CIP headline loads."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any

from compare_cross_lane_inversion_smoke import (
    interval_text,
    load_summary,
    numeric_scalar,
    paired_interval,
    variant_runs,
)


FLAT_VARIANT = "flat-fee"
CANONICAL_VARIANT = "canonical-final-d16-k10"
LOADS = (
    ("low", "Low"),
    ("mid-load", "Mid load"),
    ("severe-congestion", "Severe congestion"),
    ("eb-capacity-stress", "EB-capacity stress"),
    ("launch-day", "Launch day"),
)
URGENT_LOADS = LOADS[:-1]
PROJECT_DIR = Path(__file__).resolve().parent.parent


def mean(values: list[float]) -> float:
    if not values:
        raise ValueError("cannot calculate a mean from no values")
    return sum(values) / len(values)


def source_provenance(patch_path: Path) -> dict[str, Any]:
    try:
        repo_root = Path(
            subprocess.run(
                ["git", "rev-parse", "--show-toplevel"],
                cwd=PROJECT_DIR,
                check=True,
                capture_output=True,
                text=True,
            ).stdout.strip()
        )
        project_path = str(PROJECT_DIR.relative_to(repo_root))
        revision = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=repo_root,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        status = subprocess.run(
            [
                "git",
                "status",
                "--porcelain",
                "--untracked-files=all",
                "--",
                project_path,
            ],
            cwd=repo_root,
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError):
        return {
            "git_revision": None,
            "abstract_sim_worktree_clean": None,
            "dirty_patch": None,
        }

    provenance: dict[str, Any] = {
        "git_revision": revision,
        "abstract_sim_worktree_clean": not bool(status.strip()),
        "dirty_patch": None,
    }
    if provenance["abstract_sim_worktree_clean"]:
        return provenance

    try:
        tracked_patch = subprocess.run(
            ["git", "diff", "--binary", "HEAD", "--", project_path],
            cwd=repo_root,
            check=True,
            capture_output=True,
        ).stdout
        untracked_output = subprocess.run(
            [
                "git",
                "ls-files",
                "--others",
                "--exclude-standard",
                "-z",
                "--",
                project_path,
            ],
            cwd=repo_root,
            check=True,
            capture_output=True,
        ).stdout
        patch_parts = [tracked_patch]
        for raw_path in sorted(path for path in untracked_output.split(b"\0") if path):
            path = raw_path.decode("utf-8", errors="surrogateescape")
            result = subprocess.run(
                ["git", "diff", "--no-index", "--binary", "/dev/null", path],
                cwd=repo_root,
                capture_output=True,
            )
            if result.returncode not in (0, 1):
                raise OSError(result.stderr.decode("utf-8", errors="replace"))
            patch_parts.append(result.stdout)
        patch = b"".join(patch_parts)
        patch_path.write_bytes(patch)
    except (OSError, subprocess.CalledProcessError):
        return provenance

    provenance["dirty_patch"] = {
        "path": patch_path.name,
        "sha256": hashlib.sha256(patch).hexdigest(),
        "bytes": len(patch),
    }
    return provenance


def load_pair(root: Path, directory: str) -> tuple[Path, dict[int, dict[str, float]], dict[int, dict[str, float]], int]:
    path = root / directory / "summary.json"
    summary = load_summary(path)
    expected_configs = {
        FLAT_VARIANT: "config/variants/flat-fee.json",
        CANONICAL_VARIANT: "config/variants/trickle-aging/thr-k10.json",
    }
    actual_configs = {
        variant.get("name"): variant.get("config") for variant in summary["variants"]
    }
    if actual_configs != expected_configs:
        raise ValueError(
            f"unexpected variants/configs in {path}: "
            f"expected={expected_configs!r}, actual={actual_configs!r}"
        )
    load_override = summary.get("loadOverride")
    if not isinstance(load_override, dict):
        raise ValueError(f"missing load override in {path}")
    if directory == "low":
        expected_load = {"type": "preset", "name": "low"}
    else:
        expected_load = {
            "type": "profile",
            "name": directory,
            "source": f"config/loads/{directory}.json",
        }
    for key, expected in expected_load.items():
        if load_override.get(key) != expected:
            raise ValueError(
                f"unexpected {key} in load override in {path}: "
                f"expected={expected!r}, actual={load_override.get(key)!r}"
            )
    flat = variant_runs(summary, FLAT_VARIANT, path)
    canonical = variant_runs(summary, CANONICAL_VARIANT, path)
    if set(flat) != set(canonical):
        raise ValueError(
            f"paired comparison requires identical seed sets in {path}: "
            f"flat={sorted(flat)}, canonical={sorted(canonical)}"
        )
    if len(flat) < 2:
        raise ValueError(f"paired comparison requires at least two seeds in {path}")
    declared_seeds = summary.get("seeds")
    if declared_seeds != len(flat):
        raise ValueError(
            f"declared seed count does not match runs in {path}: "
            f"declared={declared_seeds!r}, runs={len(flat)}"
        )
    slots = summary.get("slots")
    if isinstance(slots, bool) or not isinstance(slots, int) or slots < 1:
        raise ValueError(f"missing/invalid slot count in {path}")
    if summary.get("summaryOnly") is not True:
        raise ValueError(f"headline refresh was expected to be summary-only: {path}")
    if summary.get("randomness") != "independent-streams":
        raise ValueError(f"headline refresh requires independent RNG streams: {path}")
    return path, flat, canonical, slots


def scalar(run: dict[str, float], key: str, variant: str, seed: int) -> float:
    return numeric_scalar(run, key, variant, seed)


def retained_row(
    directory: str,
    label: str,
    flat: dict[int, dict[str, float]],
    canonical: dict[int, dict[str, float]],
) -> dict[str, Any]:
    seeds = sorted(flat)
    flat_values: list[float] = []
    canonical_values: list[float] = []

    if directory == "launch-day":
        for seed in seeds:
            flat_run = flat[seed]
            offered = sum(
                scalar(flat_run, key, FLAT_VARIANT, seed)
                for key in (
                    "value.retainedLovelace",
                    "value.lostLovelace",
                    "value.unresolvedLovelace",
                )
            )
            if offered <= 0:
                raise ValueError(f"non-positive launch-day offered value for seed {seed}")
            flat_values.append(
                100.0
                * scalar(flat_run, "value.retainedLovelace", FLAT_VARIANT, seed)
                / offered
            )
            canonical_values.append(
                100.0
                * scalar(
                    canonical[seed],
                    "value.retainedLovelace",
                    CANONICAL_VARIANT,
                    seed,
                )
                / offered
            )
        metric = "Overall retained value (historical flat-fee denominator)"
    else:
        key = "value.urgent.retainedRatio"
        for seed in seeds:
            flat_values.append(100.0 * scalar(flat[seed], key, FLAT_VARIANT, seed))
            canonical_values.append(
                100.0 * scalar(canonical[seed], key, CANONICAL_VARIANT, seed)
            )
        metric = "Urgent retained value"

    differences = [candidate - control for candidate, control in zip(canonical_values, flat_values)]
    estimate, low, high = paired_interval(differences)
    return {
        "load": directory,
        "label": label,
        "metric": metric,
        "unit": "percentage points",
        "flat_mean_percent": mean(flat_values),
        "canonical_mean_percent": mean(canonical_values),
        "canonical_minus_flat": {
            "mean": estimate,
            "ci95_low": low,
            "ci95_high": high,
        },
        "canonical_wins": sum(candidate > control for candidate, control in zip(canonical_values, flat_values)),
        "paired_seeds": len(seeds),
    }


def latency_row(
    directory: str,
    label: str,
    flat: dict[int, dict[str, float]],
    canonical: dict[int, dict[str, float]],
) -> dict[str, Any]:
    key = "latency.urgent.meanBlocks"
    seeds = sorted(flat)
    flat_values = [scalar(flat[seed], key, FLAT_VARIANT, seed) for seed in seeds]
    canonical_values = [
        scalar(canonical[seed], key, CANONICAL_VARIANT, seed) for seed in seeds
    ]
    differences = [candidate - control for candidate, control in zip(canonical_values, flat_values)]
    estimate, low, high = paired_interval(differences)
    return {
        "load": directory,
        "label": label,
        "metric": "Urgent mean latency",
        "unit": "blocks",
        "flat_mean": mean(flat_values),
        "canonical_mean": mean(canonical_values),
        "canonical_minus_flat": {
            "mean": estimate,
            "ci95_low": low,
            "ci95_high": high,
        },
        "canonical_wins": sum(candidate < control for candidate, control in zip(canonical_values, flat_values)),
        "paired_seeds": len(seeds),
    }


def build_report(root: Path, simulator_sha256: str) -> dict[str, Any]:
    pairs: dict[str, tuple[dict[int, dict[str, float]], dict[int, dict[str, float]]]] = {}
    sources: dict[str, str] = {}
    slot_counts: set[int] = set()
    seed_sets: set[tuple[int, ...]] = set()

    for directory, _label in LOADS:
        path, flat, canonical, slots = load_pair(root, directory)
        pairs[directory] = (flat, canonical)
        sources[directory] = str(path)
        slot_counts.add(slots)
        seed_sets.add(tuple(sorted(flat)))

    if len(slot_counts) != 1:
        raise ValueError(f"loads use different slot counts: {sorted(slot_counts)}")
    if len(seed_sets) != 1:
        raise ValueError("loads use different seed sets")

    retention = [
        retained_row(directory, label, *pairs[directory])
        for directory, label in LOADS
    ]
    latency = [
        latency_row(directory, label, *pairs[directory])
        for directory, label in URGENT_LOADS
    ]
    seeds = list(next(iter(seed_sets)))
    provenance = source_provenance(root / "source.patch")
    provenance["simulator_sha256"] = simulator_sha256
    return {
        "method": "paired mean difference with two-sided 95% Student-t confidence interval",
        "variants": {"control": FLAT_VARIANT, "candidate": CANONICAL_VARIANT},
        "seeds": seeds,
        "slots": next(iter(slot_counts)),
        "sources": sources,
        "provenance": provenance,
        "retained_value": retention,
        "urgent_latency": latency,
        "launch_day_denominator": (
            "For each paired seed, both retained-value numerators are divided by "
            "the flat-fee run's retained + lost + unresolved value, matching the "
            "historical headline calculation. This accounts for demand that reached "
            "a first submission; the summary format does not record fresh samples "
            "that declined before submission, so this is a flat-fee proxy rather "
            "than a literal ledger of all offered demand."
        ),
        "refresh_scope": (
            "This is a replacement estimate for the current canonical D16/K10 model, "
            "not an exact reproduction of the published numbers. The published low, "
            "mid, EB-stress, and launch-day rows used the D8 anchor; this refresh also "
            "uses newly separated RNG streams and the committed 320/400 tx-per-slot "
            "EB-stress profile."
        ),
        "producer_policy": (
            "The simulator eagerly announces an EB when eligible payload is non-empty "
            "and either its threshold or K=10 eligibility condition is met; producer "
            "withholding is not modelled."
        ),
    }


def markdown(report: dict[str, Any]) -> str:
    seeds = report["seeds"]
    provenance = report["provenance"]
    revision = provenance["git_revision"] or "unknown"
    clean = provenance["abstract_sim_worktree_clean"]
    source_state = "clean" if clean is True else "dirty" if clean is False else "unknown"
    source_line = f"Source revision: `{revision}`; `abstract-sim-hs` worktree: {source_state}."
    dirty_patch = provenance.get("dirty_patch")
    if dirty_patch:
        source_line += (
            f" Dirty source: `{dirty_patch['path']}` "
            f"(SHA-256 `{dirty_patch['sha256']}`)."
        )
    source_line += f" Simulator SHA-256: `{provenance['simulator_sha256']}`."
    lines = [
        "# Canonical CIP headline refresh",
        "",
        (
            f"Flat fee versus canonical D16/K10; paired seeds "
            f"{seeds[0]}–{seeds[-1]} (n={len(seeds)}), {report['slots']:,} slots each. "
            "Intervals are two-sided 95% paired-t confidence intervals."
        ),
        "",
        source_line,
        "",
        "## Retained value",
        "",
        "| Load | Metric | Flat | Canonical | Canonical − flat (95% CI) | Seeds better |",
        "|---|---|---:|---:|---:|---:|",
    ]
    for row in report["retained_value"]:
        lines.append(
            "| {label} | {metric} | {flat:.2f}% | {canonical:.2f}% | {difference} pp | {wins}/{seeds} |".format(
                label=row["label"],
                metric=row["metric"],
                flat=row["flat_mean_percent"],
                canonical=row["canonical_mean_percent"],
                difference=interval_text(row["canonical_minus_flat"], 3),
                wins=row["canonical_wins"],
                seeds=row["paired_seeds"],
            )
        )

    lines.extend(
        [
            "",
            "## Urgent mean latency",
            "",
            "| Load | Flat | Canonical | Canonical − flat (95% CI) | Seeds faster |",
            "|---|---:|---:|---:|---:|",
        ]
    )
    for row in report["urgent_latency"]:
        lines.append(
            "| {label} | {flat:.3f} | {canonical:.3f} | {difference} blocks | {wins}/{seeds} |".format(
                label=row["label"],
                flat=row["flat_mean"],
                canonical=row["canonical_mean"],
                difference=interval_text(row["canonical_minus_flat"], 3),
                wins=row["canonical_wins"],
                seeds=row["paired_seeds"],
            )
        )

    lines.extend(
        [
            "",
            "## Interpretation notes",
            "",
            f"- {report['launch_day_denominator']}",
            f"- {report['refresh_scope']}",
            "- Independent seeded RNG streams keep fresh-demand samples and ranking-block opportunities common across the paired mechanisms; retry jitter remains mechanism-dependent.",
            f"- {report['producer_policy']}",
            "- Positive retained-value differences favour the canonical mechanism; negative latency differences are faster.",
            "",
        ]
    )
    return "\n".join(lines)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, help="directory containing the five load subdirectories")
    parser.add_argument("--simulator-sha256", required=True, help="SHA-256 of the executable used for every run")
    parser.add_argument("--markdown-output", required=True, help="path for the Markdown report")
    parser.add_argument("--json-output", required=True, help="path for the machine-readable report")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        simulator_sha256 = args.simulator_sha256.lower()
        if len(simulator_sha256) != 64 or any(
            character not in "0123456789abcdef" for character in simulator_sha256
        ):
            raise ValueError("--simulator-sha256 must be 64 hexadecimal characters")
        report = build_report(Path(args.root), simulator_sha256)
        markdown_text = markdown(report)
        markdown_path = Path(args.markdown_output)
        json_path = Path(args.json_output)
        markdown_path.write_text(markdown_text, encoding="utf-8")
        json_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    except (OSError, ValueError) as error:
        raise SystemExit(f"error: {error}") from error
    print(markdown_text, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
