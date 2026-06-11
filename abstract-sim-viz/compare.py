#!/usr/bin/env python3
"""Embed a sweep summary.json as dashboard/compare-data.js for the
cross-variant comparison page (dashboard/compare.html). No distillation
happens here: the sweep summary already carries per-run scalars and
per-variant aggregates in chart-ready form."""
import argparse
import json
import os
from datetime import datetime, timezone

DEFAULT_OUTPUT = os.path.join(
    os.path.dirname(os.path.abspath(__file__)), "dashboard", "compare-data.js")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("summary", help="path to a sweep-results summary.json")
    parser.add_argument("-o", "--output", default=DEFAULT_OUTPUT,
                        help="output JS (default: the dashboard/compare-data.js next to "
                             "this script, so it works regardless of the directory you "
                             "run from)")
    args = parser.parse_args(argv)

    with open(args.summary) as fh:
        summary = json.load(fh)

    payload = {
        "source": os.path.basename(os.path.dirname(os.path.abspath(args.summary)))
                  + "/" + os.path.basename(args.summary),
        "generatedAt": datetime.now(timezone.utc).isoformat(),
        "summary": summary,
    }
    out_dir = os.path.dirname(args.output)
    if out_dir:
        os.makedirs(out_dir, exist_ok=True)
    with open(args.output, "w") as fh:
        fh.write("window.SWEEP_DATA = " + json.dumps(payload, separators=(",", ":")) + ";\n")

    variants = summary.get("variants", [])
    print(f"Wrote {args.output}: {len(variants)} variants × {summary.get('seeds')} seeds, "
          f"{summary.get('slots')} slots.")


if __name__ == "__main__":
    main()
