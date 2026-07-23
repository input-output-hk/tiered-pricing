#!/usr/bin/env python3
"""Render the preliminary-experiment-report results table from a sweep
summary.json.

The summary already carries per-variant aggregates (mean/min/max/stddev over
the seeded runs). This script just selects the columns the report uses and
formats them as a markdown table. It reproduces the published "mechanisms"
table exactly, and can emit either the full table or a cut-down one (best
window per family by urgent retained value, plus the single worst row, plus
the two single-lane controls).

Usage:
    results_table.py SUMMARY_JSON              # full table
    results_table.py SUMMARY_JSON --cutdown    # cut-down table
"""
import argparse
import json

# Variant-name suffix -> the "Priority signal" label used in the report.
SIGNAL_BY_SUFFIX = {
    "": "instant",
    "window3": "3-sample window",
    "windowed": "5-sample window",
    "window10": "10-sample window",
    "window20": "20-sample window",
}

# The four windowed families, in report order. Controls are handled separately.
FAMILIES = [
    "priority-only-reserved",
    "priority-only-open",
    "both-dynamic-reserved",
    "both-dynamic-open",
]

CONTROLS = ["flat-fee", "single-lane-eip1559"]


def parse_name(name):
    """Return (family, signal-label) for a variant name."""
    if name in CONTROLS:
        return name, "n/a"
    for fam in FAMILIES:
        if name == fam:
            return fam, SIGNAL_BY_SUFFIX[""]
        if name.startswith(fam + "-"):
            suffix = name[len(fam) + 1:]
            return fam, SIGNAL_BY_SUFFIX.get(suffix, suffix)
    return name, "n/a"


def mean(v, key):
    return v["aggregates"][key]["mean"]


def row_metrics(v):
    """Pull the report's columns out of a variant's aggregates."""
    prio_lat = mean(v, "latency.priority.meanBlocks")
    is_single_lane = mean(v, "inclusion.priority.submitted") == 0
    return {
        "inclusion": 100 * mean(v, "units.serviceRate"),
        "urgent_retained": 100 * mean(v, "value.urgent.retainedRatio"),
        "urgent_latency": mean(v, "latency.urgent.meanBlocks"),
        "priority_latency": None if is_single_lane else prio_lat,
        "tx_per_slot": mean(v, "throughput.txPerSlot"),
        "shock_count": mean(v, "price.shockCount"),
        "osc_cycles": mean(v, "price.oscillationCycleCount"),
        "osc_max_amp": mean(v, "price.oscillationMaxAmplitude"),
        "settled_range": mean(v, "price.settledCoefficientRange"),
    }


def fmt_row(family, signal, mt):
    pl = "n/a" if mt["priority_latency"] is None else f"{mt['priority_latency']:.2f}"
    return (
        f"| {family} | {signal} | {mt['inclusion']:.2f}% | {mt['urgent_retained']:.2f}% | "
        f"{mt['urgent_latency']:.2f} | {pl} | {mt['tx_per_slot']:.1f} | "
        f"{mt['shock_count']:.1f} | {mt['osc_cycles']:.1f} | "
        f"{mt['osc_max_amp']:.3f} | {mt['settled_range']:.3f} |"
    )


HEADER = (
    "| Family | Priority signal | Inclusion | Urgent retained | Urgent latency (blk) | "
    "Priority latency (blk) | Tx/slot | Shock count | Osc. cycles | Osc. max amp | "
    "Settled coeff. range |\n"
    "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
)


def full_table(variants):
    order = CONTROLS + [
        f"{fam}{('-' + suf) if suf else ''}"
        for fam in FAMILIES
        for suf in ["", "window3", "windowed", "window10", "window20"]
    ]
    byname = {v["name"]: v for v in variants}
    lines = [HEADER]
    for name in order:
        if name not in byname:
            continue
        fam, sig = parse_name(name)
        lines.append(fmt_row(fam, sig, row_metrics(byname[name])))
    return "\n".join(lines)


def cutdown_table(variants):
    """Controls + best window per family (by urgent retained value) + single
    worst row overall (by urgent retained value, among windowed families)."""
    byname = {v["name"]: v for v in variants}
    enriched = []
    for v in variants:
        fam, sig = parse_name(v["name"])
        if fam in CONTROLS:
            continue
        enriched.append((fam, sig, v, row_metrics(v)))

    lines = [HEADER]
    # Controls first.
    for name in CONTROLS:
        if name in byname:
            fam, sig = parse_name(name)
            lines.append(fmt_row(fam, sig, row_metrics(byname[name])))

    # Best window per family by urgent retained value.
    best_per_family = []
    for fam in FAMILIES:
        members = [e for e in enriched if e[0] == fam]
        if not members:
            continue
        best = max(members, key=lambda e: e[3]["urgent_retained"])
        best_per_family.append(best)
        lines.append(fmt_row(best[0], best[1], best[3]))

    # Single worst row overall by urgent retained value.
    worst = min(enriched, key=lambda e: e[3]["urgent_retained"])
    lines.append("")
    lines.append("Worst row overall (lowest urgent retained value):")
    lines.append(HEADER)
    lines.append(fmt_row(worst[0], worst[1], worst[3]))
    return "\n".join(lines)


def five_plus_worst_table(variants):
    """Controls + the 5-sample-window (`windowed`) row of each family, in report
    order, + the single worst row overall (lowest urgent retained value among
    windowed families). The 5-sample window is the report's recommended
    operating point, so it is shown for every family for cross-load
    comparability rather than the per-family best."""
    byname = {v["name"]: v for v in variants}
    lines = [HEADER]
    for name in CONTROLS:
        if name in byname:
            fam, sig = parse_name(name)
            lines.append(fmt_row(fam, sig, row_metrics(byname[name])))
    for fam in FAMILIES:
        name = f"{fam}-windowed"
        if name in byname:
            fam_, sig = parse_name(name)
            lines.append(fmt_row(fam_, sig, row_metrics(byname[name])))

    windowed = [
        (parse_name(v["name"]), v, row_metrics(v))
        for v in variants
        if parse_name(v["name"])[0] in FAMILIES
    ]
    worst = min(windowed, key=lambda e: e[2]["urgent_retained"])
    (fam, _), _, mt = worst
    lines.append(fmt_row(fam, parse_name(worst[1]["name"])[1] + " (worst)", mt))
    return "\n".join(lines)


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__,
                                formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("summary", help="path to a sweep-results summary.json")
    p.add_argument("--cutdown", action="store_true",
                   help="emit the cut-down table (best window per family + worst row)")
    p.add_argument("--five-plus-worst", action="store_true",
                   help="controls + the 5-sample-window row of each family + single worst row")
    args = p.parse_args(argv)

    with open(args.summary) as fh:
        summary = json.load(fh)
    variants = summary["variants"]

    if args.five_plus_worst:
        print(five_plus_worst_table(variants))
    elif args.cutdown:
        print(cutdown_table(variants))
    else:
        print(full_table(variants))


if __name__ == "__main__":
    main()
