#!/usr/bin/env python3
"""Distil one or more abstract-sim-hs events.jsonl traces into dashboard/data.js.

With several traces (e.g. one harnessed run per sweep variant), the dashboard
gets a run selector to flip between them; with one, it behaves as before.
"""
import argparse
import os

from simviz.ingest import iter_events, Accumulator
from simviz.contract import build_sim_data, run_names, write_runs_js


DEFAULT_OUTPUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "dashboard", "data.js")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("events", nargs="+", help="path(s) to events.jsonl traces, one run each")
    parser.add_argument("-o", "--output", default=DEFAULT_OUTPUT,
                        help="output JS (default: the dashboard/data.js next to this script, "
                             "so it works regardless of the directory you run from)")
    parser.add_argument("--shock-threshold", type=float, default=0.10)
    parser.add_argument("--band-pct", type=float, default=0.05)
    parser.add_argument("--load-change-pct", type=float, default=0.10)
    parser.add_argument("--target-buckets", type=int, default=300)
    parser.add_argument("--f", type=float, default=0.05,
                        help="active-slot coefficient for slot<->block conversion (default 0.05)")
    args = parser.parse_args(argv)

    runs = []
    for name, trace in zip(run_names(args.events), args.events):
        acc = Accumulator()
        for event in iter_events(trace):
            acc.ingest(event)
        sim_data = build_sim_data(
            acc,
            params={
                "shockThreshold": args.shock_threshold,
                "convergenceBandPct": args.band_pct,
                "loadChangePct": args.load_change_pct,
            },
            target_buckets=args.target_buckets,
            source=os.path.basename(trace),
            f=args.f,
        )
        runs.append((name, sim_data))
        m = sim_data["meta"]
        print(f"  {name}: {m['slotCount']} slots, {m['totalEvents']} events, "
              f"{len(m['urgencyClasses'])} urgency classes.")

    out_dir = os.path.dirname(args.output)
    if out_dir:
        os.makedirs(out_dir, exist_ok=True)
    write_runs_js(runs, args.output)
    print(f"Wrote {args.output}: {len(runs)} run(s).")


if __name__ == "__main__":
    main()
