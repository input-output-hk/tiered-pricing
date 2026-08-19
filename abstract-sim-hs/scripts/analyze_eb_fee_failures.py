#!/usr/bin/env python3
"""Measure how often announced EBs fail fee validation at the certification
check when the producer's one-further-step headroom is disabled.

Reads the event traces of a sweep run with the loose-producer variant and
classifies every announced EB:

- certified: an EndorserBlockCertified event with the same id exists;
- fee-failed: the next ranking-block production came at least D slots after
  the announcement (so the age check passed) but the EB was not certified -
  in the simulator's pipeline the only remaining reason is fee invalidity;
- timing-replaced: the next production came before D slots elapsed, so the
  pending EB was dropped regardless of fees;
- unresolved: no production followed the announcement before the horizon.

The fee-failure rate is fee-failed / announced, reported per seed and as a
mean with a two-sided 95% t confidence interval across seeds.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Any

from compare_cross_lane_inversion_smoke import paired_interval

PROJECT_DIR = Path(__file__).resolve().parent.parent
TRACE_PATTERN = re.compile(r"^(?P<variant>.+)-seed(?P<seed>\d+)\.events\.jsonl$")


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def project_relative(path: Path) -> str:
    resolved = path.resolve()
    try:
        return str(resolved.relative_to(PROJECT_DIR))
    except ValueError:
        return str(resolved)


def classify_run(trace_path: Path, certification_delay: int) -> dict[str, Any]:
    production_slots: list[int] = []
    announcements: list[tuple[int, Any]] = []
    certified_ids: set[Any] = set()

    with trace_path.open("r", encoding="utf-8") as handle:
        for line in handle:
            if '"BlockProduced"' not in line:
                continue
            event = json.loads(line).get("event", {})
            if event.get("tag") != "BlockProduced":
                continue
            summary = event["summary"]
            tag = summary.get("tag")
            if tag == "RankingBlockProduced":
                production_slots.append(event["slot"])
            elif tag == "EndorserBlockAnnounced":
                announcements.append((event["slot"], summary["summary"]["id"]))
            elif tag == "EndorserBlockCertified":
                certified_ids.add(summary["summary"]["id"])

    production_slots.sort()
    counts = {"announced": 0, "certified": 0, "feeFailed": 0, "timingReplaced": 0, "unresolved": 0}
    for slot, eb_id in announcements:
        counts["announced"] += 1
        if eb_id in certified_ids:
            counts["certified"] += 1
            continue
        next_production = next((s for s in production_slots if s > slot), None)
        if next_production is None:
            counts["unresolved"] += 1
        elif next_production - slot >= certification_delay:
            counts["feeFailed"] += 1
        else:
            counts["timingReplaced"] += 1
    return counts


def mean_with_ci(values: list[float]) -> dict[str, Any]:
    if len(values) < 2:
        return {"mean": values[0] if values else None, "ci95": None}
    # paired_interval computes a plain one-sample t interval over the list.
    mean, ci_low, ci_high = paired_interval(values)
    return {"mean": mean, "ci95": [ci_low, ci_high]}


def analyze_load(load_dir: Path, variant: str) -> dict[str, Any]:
    config_path = load_dir / f"{variant}.config.json"
    with config_path.open("r", encoding="utf-8") as handle:
        config = json.load(handle)
    certification_delay = config["D"]
    if config["design"].get("producerHeadroom", True):
        raise ValueError(
            f"{config_path}: design has producerHeadroom enabled; this analysis "
            "measures the loose-producer counterfactual and needs it disabled"
        )

    per_seed: dict[int, dict[str, Any]] = {}
    for trace_path in sorted(load_dir.iterdir()):
        match = TRACE_PATTERN.match(trace_path.name)
        if match is None or match.group("variant") != variant:
            continue
        seed = int(match.group("seed"))
        counts = classify_run(trace_path, certification_delay)
        counts["feeFailureRatePct"] = (
            100.0 * counts["feeFailed"] / counts["announced"] if counts["announced"] else None
        )
        per_seed[seed] = counts
    if not per_seed:
        raise ValueError(f"no {variant} event traces found in {load_dir}")

    rates = [row["feeFailureRatePct"] for row in per_seed.values() if row["feeFailureRatePct"] is not None]
    totals = {
        key: sum(row[key] for row in per_seed.values())
        for key in ("announced", "certified", "feeFailed", "timingReplaced", "unresolved")
    }
    return {
        "certificationDelaySlots": certification_delay,
        "seeds": len(per_seed),
        "perSeed": {seed: per_seed[seed] for seed in sorted(per_seed)},
        "totals": totals,
        "pooledFeeFailureRatePct": (
            100.0 * totals["feeFailed"] / totals["announced"] if totals["announced"] else None
        ),
        "meanFeeFailureRatePct": mean_with_ci(rates),
        "provenance": {
            "config": project_relative(config_path),
            "configSha256": sha256_file(config_path),
        },
    }


def render_markdown(record: dict[str, Any]) -> str:
    lines = [
        "# EB fee-failure rate without producer headroom",
        "",
        "Canonical D16/K10 configuration with the one-further-step EB selection",
        "headroom disabled; EB selection takes any transaction covering the",
        "current quote. Fee-failed = announced EBs dropped at a certification",
        "check that their age had passed, which in the simulator pipeline can",
        "only mean a transaction no longer covered the quote at that time.",
        "",
        "| Load | Announced | Certified | Fee-failed | Timing-replaced | Unresolved | Pooled fee-failure % | Mean per-seed % (95% CI) |",
        "|---|---:|---:|---:|---:|---:|---:|---|",
    ]
    for load, result in record["results"].items():
        totals = result["totals"]
        mean = result["meanFeeFailureRatePct"]
        ci = mean["ci95"]
        ci_text = f"{mean['mean']:.2f} [{ci[0]:.2f}, {ci[1]:.2f}]" if ci else str(mean["mean"])
        pooled = result["pooledFeeFailureRatePct"]
        lines.append(
            f"| {load} | {totals['announced']} | {totals['certified']} "
            f"| {totals['feeFailed']} | {totals['timingReplaced']} | {totals['unresolved']} "
            f"| {pooled:.2f} | {ci_text} |"
        )
    lines.append("")
    if record.get("simulatorSha256"):
        lines.append(f"Simulator sha256: `{record['simulatorSha256']}`")
        lines.append("")
    return "\n".join(lines)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        required=True,
        help="sweep output directory containing one subdirectory per load",
    )
    parser.add_argument("--variant", default="thr-k10-loose-producer")
    parser.add_argument("--simulator-sha256", default=None)
    parser.add_argument("--markdown-output", type=Path, required=True)
    parser.add_argument("--json-output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    load_dirs = sorted(d for d in args.root.iterdir() if d.is_dir())
    if not load_dirs:
        raise ValueError(f"no load subdirectories under {args.root}")

    record: dict[str, Any] = {
        "description": (
            "Rate at which announced EBs fail fee validation at the "
            "certification check when EB selection uses only the current "
            "quote (producer one-further-step headroom disabled). Fee "
            "failures are inferred from event traces: an announced EB that "
            "was not certified, whose next ranking-block production came at "
            "least the certification delay after the announcement."
        ),
        "variant": args.variant,
        "results": {d.name: analyze_load(d, args.variant) for d in load_dirs},
    }
    if args.simulator_sha256:
        record["simulatorSha256"] = args.simulator_sha256

    args.json_output.parent.mkdir(parents=True, exist_ok=True)
    with args.json_output.open("w", encoding="utf-8") as handle:
        json.dump(record, handle, indent=2)
        handle.write("\n")
    args.markdown_output.parent.mkdir(parents=True, exist_ok=True)
    args.markdown_output.write_text(render_markdown(record), encoding="utf-8")
    print(f"wrote {args.json_output}")
    print(f"wrote {args.markdown_output}")


if __name__ == "__main__":
    main()
