#!/usr/bin/env python3
"""Freeze the cert-void pilot: exploratory smoke evidence for the standard
controller's no-cert window contribution.

Reads the three-arm pilot sweep (sweep-results/cert-void-smoke: recommended
W20 base, cert-gated sample-and-hold, void backfill of a third of an EB) and
the two supporting dial sweeps, then writes comparison.json, comparison.md,
and source.patch into the pilot root. The pilot is exploratory: its seeds
(300-304) are excluded from confirmatory inference, and every interval here
is descriptive. The held-out confirmation is defined by
config/sweeps/cert-void-confirm.json and evaluated by
scripts/compare_cert_void_confirm.py.

Usage:
  python3 scripts/compare_cert_void_pilot.py --simulator-sha256 HEX64
"""

import argparse
import hashlib
import json
import math
import subprocess
from pathlib import Path
from typing import Any

PROJECT_DIR = Path(__file__).resolve().parent.parent
REPO_DIR = PROJECT_DIR.parent

PILOT_ROOT = PROJECT_DIR / "sweep-results" / "cert-void-smoke"
DIAL_ROOTS = {
    "void-size-sweep": PROJECT_DIR / "sweep-results" / "void-size-sweep",
    "void-size-sweep-low": PROJECT_DIR / "sweep-results" / "void-size-sweep-low",
}

BASE = "current-w20-w5"
CONTRASTS = (
    ("cert-only-minus-current", "Cert-gated hold − current", "standard-cert-only-w5"),
    ("void-third-minus-current", "Void third-EB − current", "standard-void-third-w5"),
)
LOADS = ["low", "mid-load", "severe-congestion", "eb-capacity-stress", "launch-day"]
METRICS = [
    ("value.retainedLovelace", 1e-9, "overall retained", "G lovelace"),
    ("value.retainedRatio", 1, "retained ratio", "ratio"),
    ("value.urgent.retainedLovelace", 1e-9, "urgent retained", "G lovelace"),
    ("throughput.txPerSlot", 1, "throughput", "tx/slot"),
    ("inclusion.standard.serviceRate", 1, "standard service rate", "ratio"),
    ("inclusion.urgent.serviceRate", 1, "urgent service rate", "ratio"),
    ("latency.standard.meanBlocks", 1, "standard mean wait", "blocks"),
    ("latency.urgent.meanBlocks", 1, "urgent mean wait", "blocks"),
    ("price.settledCoefficientRange", 1, "settled coeff range", "coeff"),
    ("price.oscillationExcessTravel", 1, "excess quote travel", "log-coeff"),
]

# t(4, 0.975): two-sided 95% interval at the pilot's df = 4.
T_975_DF4 = 2.776445105


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def artifact(path: Path) -> dict[str, Any]:
    return {
        "path": str(path.relative_to(PROJECT_DIR)),
        "sha256": sha256_file(path),
        "bytes": path.stat().st_size,
    }


def load_summary(root: Path, load: str) -> dict[str, dict[int, dict[str, float]]]:
    data = json.loads((root / load / "summary.json").read_text(encoding="utf-8"))
    return {
        variant["name"]: {run["seed"]: run["scalars"] for run in variant["runs"]}
        for variant in data["variants"]
    }


def descriptive_interval(differences: list[float]) -> tuple[float, float, float]:
    if len(differences) != 5:
        raise ValueError("the pilot design uses exactly five paired seeds")
    estimate = sum(differences) / len(differences)
    variance = sum((value - estimate) ** 2 for value in differences) / (
        len(differences) - 1
    )
    margin = T_975_DF4 * math.sqrt(variance / len(differences))
    return estimate, estimate - margin, estimate + margin


def contrast_block(
    arms: dict[str, dict[int, dict[str, float]]], candidate: str, seeds: list[int]
) -> dict[str, Any]:
    metrics: dict[str, Any] = {}
    for key, scale, label, unit in METRICS:
        differences = [
            scale
            * (
                (arms[candidate][seed].get(key, 0) or 0)
                - (arms[BASE][seed].get(key, 0) or 0)
            )
            for seed in seeds
        ]
        estimate, low, high = descriptive_interval(differences)
        metrics[key] = {
            "label": label,
            "unit": unit,
            "mean": estimate,
            "ci95_low_descriptive": low,
            "ci95_high_descriptive": high,
            "per_seed": dict(zip(map(str, seeds), differences)),
            "candidate_higher": sum(value > 0 for value in differences),
            "reference_higher": sum(value < 0 for value in differences),
            "ties": sum(value == 0 for value in differences),
        }
    return metrics


def dial_block(root: Path, seeds_expected: list[int]) -> dict[str, Any]:
    arms = load_summary(root, "launch-day")
    seeds = sorted(arms[BASE])
    if seeds != seeds_expected:
        raise ValueError(f"unexpected seeds in {root}: {seeds}")
    block: dict[str, Any] = {"summary_sha256": {}, "launch_day_overall_retained_delta_g": {}}
    for load in LOADS:
        block["summary_sha256"][load] = sha256_file(root / load / "summary.json")
    for name, by_seed in arms.items():
        if name == BASE:
            continue
        deltas = [
            1e-9
            * (
                (by_seed[seed].get("value.retainedLovelace", 0) or 0)
                - (arms[BASE][seed].get("value.retainedLovelace", 0) or 0)
            )
            for seed in seeds
        ]
        block["launch_day_overall_retained_delta_g"][name] = {
            "mean": sum(deltas) / len(deltas),
            "per_seed": dict(zip(map(str, seeds), deltas)),
        }
    return block


def write_source_patch() -> dict[str, Any]:
    patch_path = PILOT_ROOT / "source.patch"
    patch = subprocess.run(
        ["git", "-C", str(REPO_DIR), "diff", "HEAD", "--", "abstract-sim-hs"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    patch_path.write_text(patch, encoding="utf-8")
    return artifact(patch_path)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--simulator-sha256", required=True)
    args = parser.parse_args()
    simulator_sha256 = args.simulator_sha256.lower()
    if len(simulator_sha256) != 64 or any(
        character not in "0123456789abcdef" for character in simulator_sha256
    ):
        raise SystemExit("error: --simulator-sha256 must be 64 hexadecimal characters")

    seeds = [300, 301, 302, 303, 304]
    loads: dict[str, Any] = {}
    inputs: dict[str, Any] = {}
    for load in LOADS:
        arms = load_summary(PILOT_ROOT, load)
        if sorted(arms[BASE]) != seeds:
            raise ValueError(f"unexpected pilot seeds under {load}")
        loads[load] = {
            contrast_id: contrast_block(arms, candidate, seeds)
            for contrast_id, _label, candidate in CONTRASTS
        }
        inputs[load] = {
            "summary": artifact(PILOT_ROOT / load / "summary.json"),
            "effective_configs": {
                config.name: sha256_file(config)
                for config in sorted((PILOT_ROOT / load).glob("*.config.json"))
            },
        }

    git_revision = subprocess.run(
        ["git", "-C", str(REPO_DIR), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    worktree_dirty = bool(
        subprocess.run(
            ["git", "-C", str(REPO_DIR), "status", "--porcelain", "--", "abstract-sim-hs"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    )
    dirty_patch = write_source_patch()

    report = {
        "schema_version": 1,
        "experiment": "cert-void-pilot",
        "method": (
            "Exploratory three-arm paired smoke on seeds 300-304, 2,000 slots, "
            "independent random streams, five headline loads. Arms: the recommended "
            "W20/W5 signals; a cert-gated sample-and-hold standard signal (updates "
            "only in a block production that applies a certified EB); and a W20 "
            "window whose no-cert contribution is a third of an empty EB "
            "(4,000,000 B / 3,166,377,816 exunits) in place of the Ranking Block's "
            "own capacity. All intervals in this report are descriptive; pilot "
            "seeds are excluded from confirmatory inference."
        ),
        "difference_direction": "alternative signal minus recommended W20/W5",
        "seeds": seeds,
        "slots": 2000,
        "manifest": artifact(PROJECT_DIR / "config" / "sweeps" / "cert-void-smoke.json"),
        "contrasts": [
            {"id": contrast_id, "label": label, "candidate": candidate, "reference": BASE}
            for contrast_id, label, candidate in CONTRASTS
        ],
        "loads": loads,
        "supporting_dial_sweeps": {
            "note": (
                "Descriptive context only: the same base arm against void sizes "
                "RB caps, EB/32, EB/16, EB/8, EB/4, EB/3 (void-size-sweep) and "
                "RB/2, RB/4, RB/8, zero (void-size-sweep-low), seeds 300-304. "
                "The rbcaps arm reproduced the recommended arm bit-for-bit on "
                "every load. Launch-day overall retained value falls "
                "monotonically above the RB-sized contribution and moves by "
                "less than half a percent below it."
            ),
            "sweeps": {
                name: dial_block(root, seeds) for name, root in DIAL_ROOTS.items()
            },
        },
        "provenance": {
            "git_revision": git_revision,
            "abstract_sim_worktree_clean": not worktree_dirty,
            "dirty_patch": dirty_patch,
            "simulator_sha256": simulator_sha256,
            "inputs": inputs,
        },
        "notes": [
            "The cert-gated arm's per-seed results reproduce the earlier two-arm "
            "cert-only smoke exactly (same seeds, same configuration).",
            "Low and mid load are bit-identical across all arms at every tested "
            "void size: the standard coefficient rests on its 1.0 floor either way.",
        ],
    }

    json_path = PILOT_ROOT / "comparison.json"
    json_path.write_text(
        json.dumps(report, indent=2, sort_keys=False) + "\n", encoding="utf-8"
    )

    lines = [
        "# Cert-void pilot (exploratory; descriptive only)",
        "",
        report["method"],
        "",
    ]
    for load in LOADS:
        lines.append(f"## {load}")
        lines.append("")
        lines.append(
            "| metric | unit | "
            + " | ".join(label for _id, label, _cand in CONTRASTS)
            + " |"
        )
        lines.append("|---|---|" + "---|" * len(CONTRASTS))
        for key, _scale, label, unit in METRICS:
            cells = []
            for contrast_id, _label, _cand in CONTRASTS:
                entry = loads[load][contrast_id][key]
                cells.append(
                    f"{entry['mean']:+.4f} "
                    f"[{entry['ci95_low_descriptive']:+.4f}, {entry['ci95_high_descriptive']:+.4f}] "
                    f"(+{entry['candidate_higher']}/-{entry['reference_higher']})"
                )
            lines.append(f"| {label} | {unit} | " + " | ".join(cells) + " |")
        lines.append("")
    lines.append(
        f"Simulator SHA-256 `{simulator_sha256}`; git revision "
        f"`{git_revision}` (worktree {'clean' if not worktree_dirty else 'dirty; see source.patch'})."
    )
    (PILOT_ROOT / "comparison.md").write_text("\n".join(lines) + "\n", encoding="utf-8")

    print(f"wrote {json_path}")
    print(f"pilot report sha256: {sha256_file(json_path)}")
    print(f"source patch sha256: {dirty_patch['sha256']}")


if __name__ == "__main__":
    main()
