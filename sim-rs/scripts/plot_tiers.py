#!/usr/bin/env python3
"""
Plot tiered pricing time-series CSV into an interactive HTML report.

Usage:
  python scripts/plot_tiers.py output/time_series.csv
  python scripts/plot_tiers.py output/time_series.csv --output output/tiered_plot.html
"""

from __future__ import annotations

import argparse
import csv
import json
import os
from typing import Callable, List


def parse_list(value: str, cast: Callable[[str], float]) -> List[float]:
    value = value.strip()
    if not value:
        return []
    return [cast(item) for item in value.split(";") if item]


def parse_int(value: str | None, default: int = 0) -> int:
    if value is None:
        return default
    value = value.strip()
    if not value:
        return default
    return int(value)


def load_points(path: str):
    rows = []
    with open(path, "r", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            rows.append(
                {
                    "slot": parse_int(row.get("slot")),
                    "tier_count": parse_int(row.get("tier_count")),
                    "prices": parse_list(row.get("tier_prices", ""), int),
                    "delays": parse_list(row.get("tier_delays", ""), int),
                    "utils": parse_list(row.get("tier_utilisations", ""), float),
                    "cumulative_inclusions": parse_int(
                        row.get("cumulative_inclusions")
                    ),
                    "cumulative_rb_inclusions": parse_int(
                        row.get("cumulative_rb_inclusions")
                    ),
                    "cumulative_eb_inclusions": parse_int(
                        row.get("cumulative_eb_inclusions")
                    ),
                    "cumulative_block_inclusions_total": parse_int(
                        row.get("cumulative_block_inclusions_total")
                    ),
                    "cumulative_block_inclusions_with_delay": parse_int(
                        row.get("cumulative_block_inclusions_with_delay")
                    ),
                    "cumulative_settled_inclusions_with_delay": parse_int(
                        row.get("cumulative_settled_inclusions_with_delay")
                    ),
                    "pending_delayed_inclusions": parse_int(
                        row.get("pending_delayed_inclusions")
                    ),
                    "cumulative_submitted_bytes": parse_int(
                        row.get("cumulative_submitted_bytes")
                    ),
                    "cumulative_included_bytes": parse_int(
                        row.get("cumulative_included_bytes")
                    ),
                    "cumulative_fees": parse_int(row.get("cumulative_fees")),
                }
            )
    return rows


def saturating_delta(current: int, previous: int) -> int:
    if current >= previous:
        return current - previous
    return 0


def build_series(rows):
    if not rows:
        return None

    max_tiers = max(len(row["prices"]) for row in rows)

    slots = [row["slot"] for row in rows]
    tier_counts = [row["tier_count"] for row in rows]

    cumulative_inclusions = [row["cumulative_inclusions"] for row in rows]
    cumulative_rb_inclusions = [row["cumulative_rb_inclusions"] for row in rows]
    cumulative_eb_inclusions = [row["cumulative_eb_inclusions"] for row in rows]
    cumulative_block_inclusions_total = [
        row["cumulative_block_inclusions_total"] for row in rows
    ]
    cumulative_block_inclusions_with_delay = [
        row["cumulative_block_inclusions_with_delay"] for row in rows
    ]
    cumulative_settled_inclusions_with_delay = [
        row["cumulative_settled_inclusions_with_delay"] for row in rows
    ]
    pending_delayed_inclusions = [row["pending_delayed_inclusions"] for row in rows]
    cumulative_submitted_bytes = [row["cumulative_submitted_bytes"] for row in rows]
    cumulative_included_bytes = [row["cumulative_included_bytes"] for row in rows]

    attempted_bytes_delta = []
    included_bytes_delta = []
    rb_inclusions_delta = []
    eb_inclusions_delta = []
    block_inclusions_total_delta = []
    block_inclusions_with_delay_delta = []
    settled_inclusions_total_delta = []
    settled_inclusions_with_delay_delta = []

    previous_cumulative_attempted_bytes = 0
    previous_cumulative_bytes = 0
    previous_cumulative_rb = 0
    previous_cumulative_eb = 0
    previous_block_inclusions_total = 0
    previous_block_inclusions_with_delay = 0
    previous_settled_inclusions_total = 0
    previous_settled_inclusions_with_delay = 0

    for row in rows:
        attempted_bytes_delta.append(
            saturating_delta(
                row["cumulative_submitted_bytes"], previous_cumulative_attempted_bytes
            )
        )
        previous_cumulative_attempted_bytes = row["cumulative_submitted_bytes"]

        included_bytes_delta.append(
            saturating_delta(row["cumulative_included_bytes"], previous_cumulative_bytes)
        )
        previous_cumulative_bytes = row["cumulative_included_bytes"]

        rb_inclusions_delta.append(
            saturating_delta(row["cumulative_rb_inclusions"], previous_cumulative_rb)
        )
        previous_cumulative_rb = row["cumulative_rb_inclusions"]

        eb_inclusions_delta.append(
            saturating_delta(row["cumulative_eb_inclusions"], previous_cumulative_eb)
        )
        previous_cumulative_eb = row["cumulative_eb_inclusions"]

        block_inclusions_total_delta.append(
            saturating_delta(
                row["cumulative_block_inclusions_total"],
                previous_block_inclusions_total,
            )
        )
        previous_block_inclusions_total = row["cumulative_block_inclusions_total"]

        block_inclusions_with_delay_delta.append(
            saturating_delta(
                row["cumulative_block_inclusions_with_delay"],
                previous_block_inclusions_with_delay,
            )
        )
        previous_block_inclusions_with_delay = row[
            "cumulative_block_inclusions_with_delay"
        ]

        settled_inclusions_total_delta.append(
            saturating_delta(
                row["cumulative_inclusions"], previous_settled_inclusions_total
            )
        )
        previous_settled_inclusions_total = row["cumulative_inclusions"]

        settled_inclusions_with_delay_delta.append(
            saturating_delta(
                row["cumulative_settled_inclusions_with_delay"],
                previous_settled_inclusions_with_delay,
            )
        )
        previous_settled_inclusions_with_delay = row[
            "cumulative_settled_inclusions_with_delay"
        ]

    cumulative_fees = [float(row["cumulative_fees"]) for row in rows]

    prices_by_tier = [[] for _ in range(max_tiers)]
    delays_by_tier = [[] for _ in range(max_tiers)]
    utils_by_tier = [[] for _ in range(max_tiers)]

    for row in rows:
        for tier_index in range(max_tiers):
            prices_by_tier[tier_index].append(
                row["prices"][tier_index] if tier_index < len(row["prices"]) else None
            )
            delays_by_tier[tier_index].append(
                row["delays"][tier_index] if tier_index < len(row["delays"]) else None
            )
            utils_by_tier[tier_index].append(
                row["utils"][tier_index] if tier_index < len(row["utils"]) else None
            )

    return {
        "slots": slots,
        "tier_counts": tier_counts,
        "prices_by_tier": prices_by_tier,
        "delays_by_tier": delays_by_tier,
        "utils_by_tier": utils_by_tier,
        "cumulative_inclusions": cumulative_inclusions,
        "cumulative_rb_inclusions": cumulative_rb_inclusions,
        "cumulative_eb_inclusions": cumulative_eb_inclusions,
        "cumulative_block_inclusions_total": cumulative_block_inclusions_total,
        "cumulative_block_inclusions_with_delay": cumulative_block_inclusions_with_delay,
        "cumulative_settled_inclusions_with_delay": cumulative_settled_inclusions_with_delay,
        "pending_delayed_inclusions": pending_delayed_inclusions,
        "cumulative_submitted_bytes": cumulative_submitted_bytes,
        "attempted_bytes_delta": attempted_bytes_delta,
        "cumulative_included_bytes": cumulative_included_bytes,
        "included_bytes_delta": included_bytes_delta,
        "rb_inclusions_delta": rb_inclusions_delta,
        "eb_inclusions_delta": eb_inclusions_delta,
        "block_inclusions_total_delta": block_inclusions_total_delta,
        "block_inclusions_with_delay_delta": block_inclusions_with_delay_delta,
        "settled_inclusions_total_delta": settled_inclusions_total_delta,
        "settled_inclusions_with_delay_delta": settled_inclusions_with_delay_delta,
        "cumulative_fees": cumulative_fees,
    }


def build_html(series, title: str, include_cumulative: bool) -> str:
    payload = {
        "title": title,
        "slots": series["slots"],
        "tier_counts": series["tier_counts"],
        "prices_by_tier": series["prices_by_tier"],
        "delays_by_tier": series["delays_by_tier"],
        "utils_by_tier": series["utils_by_tier"],
        "cumulative_inclusions": series["cumulative_inclusions"],
        "cumulative_rb_inclusions": series["cumulative_rb_inclusions"],
        "cumulative_eb_inclusions": series["cumulative_eb_inclusions"],
        "cumulative_block_inclusions_total": series["cumulative_block_inclusions_total"],
        "cumulative_block_inclusions_with_delay": series[
            "cumulative_block_inclusions_with_delay"
        ],
        "cumulative_settled_inclusions_with_delay": series[
            "cumulative_settled_inclusions_with_delay"
        ],
        "pending_delayed_inclusions": series["pending_delayed_inclusions"],
        "cumulative_submitted_bytes": series["cumulative_submitted_bytes"],
        "attempted_bytes_delta": series["attempted_bytes_delta"],
        "cumulative_included_bytes": series["cumulative_included_bytes"],
        "included_bytes_delta": series["included_bytes_delta"],
        "rb_inclusions_delta": series["rb_inclusions_delta"],
        "eb_inclusions_delta": series["eb_inclusions_delta"],
        "block_inclusions_total_delta": series["block_inclusions_total_delta"],
        "block_inclusions_with_delay_delta": series[
            "block_inclusions_with_delay_delta"
        ],
        "settled_inclusions_total_delta": series["settled_inclusions_total_delta"],
        "settled_inclusions_with_delay_delta": series[
            "settled_inclusions_with_delay_delta"
        ],
        "cumulative_fees": series["cumulative_fees"],
        "include_cumulative": include_cumulative,
    }

    data_json = json.dumps(payload)

    return f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Tiered Pricing Time Series</title>
  <script src="https://cdn.plot.ly/plotly-2.30.0.min.js"></script>
  <style>
    body {{
      font-family: system-ui, -apple-system, Segoe UI, sans-serif;
      margin: 0;
      padding: 16px;
      background: #f7f7f7;
    }}
    .chart {{
      background: #fff;
      border: 1px solid #e0e0e0;
      border-radius: 8px;
      margin: 16px 0;
      padding: 8px;
    }}
    h1 {{
      margin: 0 0 8px 0;
      font-size: 20px;
    }}
  </style>
</head>
<body>
  <h1>Tiered Pricing Time Series</h1>
  <div id="tier-count" class="chart"></div>
  <div id="tier-prices" class="chart"></div>
  <div id="tx-volume" class="chart"></div>
  <div id="tier-delays" class="chart"></div>
  <div id="tier-utils" class="chart"></div>
  <div id="cumulative" class="chart"></div>
  <div id="inclusion-split" class="chart"></div>
  <div id="delay-settlement" class="chart"></div>

  <script>
    const payload = {data_json};
    const slots = payload.slots;

    function tierTraces(seriesKey) {{
      const traces = [];
      payload[seriesKey].forEach((series, tierIndex) => {{
        traces.push({{
          x: slots,
          y: series,
          mode: "lines",
          name: `Tier ${{tierIndex}}`,
        }});
      }});
      return traces;
    }}

    function movingAverage(values, windowSize) {{
      const safeWindow = Math.max(1, windowSize | 0);
      const output = [];
      const rolling = [];
      let sum = 0;
      values.forEach((value) => {{
        const numeric = Number(value) || 0;
        rolling.push(numeric);
        sum += numeric;
        if (rolling.length > safeWindow) {{
          sum -= rolling.shift();
        }}
        output.push(sum / rolling.length);
      }});
      return output;
    }}

    const volumeWindow = Math.max(3, Math.min(15, Math.round(slots.length / 30)));
    const smoothedAttemptedBytes = movingAverage(payload.attempted_bytes_delta, volumeWindow);
    const smoothedIncludedBytes = movingAverage(payload.included_bytes_delta, volumeWindow);

    Plotly.newPlot("tier-count", [
      {{
        x: slots,
        y: payload.tier_counts,
        mode: "lines",
        name: "Tier count",
        line: {{ width: 2 }},
      }}
    ], {{
      title: "Tier Count",
      xaxis: {{ title: "Slot" }},
      yaxis: {{ title: "Count" }},
      margin: {{ t: 40, r: 20, b: 40, l: 50 }},
    }});

    Plotly.newPlot("tier-prices", tierTraces("prices_by_tier"), {{
      title: "Tier Prices (per byte)",
      xaxis: {{ title: "Slot" }},
      yaxis: {{ title: "Price" }},
      margin: {{ t: 40, r: 20, b: 40, l: 50 }},
    }});

    Plotly.newPlot("inclusion-split", [
      {{
        x: slots,
        y: payload.rb_inclusions_delta,
        type: "bar",
        name: "RB inclusions",
        marker: {{ color: "rgba(255, 127, 14, 0.80)" }},
      }},
      {{
        x: slots,
        y: payload.eb_inclusions_delta,
        type: "bar",
        name: "EB inclusions",
        marker: {{ color: "rgba(44, 160, 44, 0.80)" }},
      }},
    ], {{
      title: "Transaction Inclusions per Update (RB vs EB)",
      barmode: "stack",
      xaxis: {{ title: "Slot" }},
      yaxis: {{ title: "Inclusions" }},
      margin: {{ t: 40, r: 20, b: 40, l: 50 }},
    }});

    Plotly.newPlot("delay-settlement", [
      {{
        x: slots,
        y: payload.block_inclusions_with_delay_delta,
        type: "bar",
        name: "Block inclusions (delay > 1)",
        marker: {{ color: "rgba(255, 127, 14, 0.70)" }},
      }},
      {{
        x: slots,
        y: payload.settled_inclusions_with_delay_delta,
        type: "bar",
        name: "Settled inclusions (delay > 1)",
        marker: {{ color: "rgba(44, 160, 44, 0.70)" }},
      }},
      {{
        x: slots,
        y: payload.block_inclusions_total_delta,
        mode: "lines",
        name: "All block inclusions",
        line: {{ width: 1.5, color: "rgb(255, 127, 14)", dash: "dot" }},
        visible: "legendonly",
      }},
      {{
        x: slots,
        y: payload.settled_inclusions_total_delta,
        mode: "lines",
        name: "All settled inclusions",
        line: {{ width: 1.5, color: "rgb(31, 119, 180)", dash: "dot" }},
        visible: "legendonly",
      }},
      {{
        x: slots,
        y: payload.pending_delayed_inclusions,
        mode: "lines",
        name: "Pending delayed queue",
        line: {{ width: 2.5, color: "rgb(214, 39, 40)" }},
        yaxis: "y2",
      }},
    ], {{
      title: "Delayed-Tier Inclusion and Settlement",
      barmode: "group",
      xaxis: {{ title: "Slot" }},
      yaxis: {{ title: "Inclusions per update", rangemode: "tozero" }},
      yaxis2: {{
        title: "Pending delayed txs",
        overlaying: "y",
        side: "right",
      }},
      margin: {{ t: 40, r: 60, b: 40, l: 50 }},
    }});

    Plotly.newPlot("tx-volume", [
      {{
        x: slots,
        y: payload.attempted_bytes_delta,
        type: "bar",
        name: "Attempted bytes per update",
        marker: {{ color: "rgba(214, 39, 40, 0.20)" }},
      }},
      {{
        x: slots,
        y: payload.included_bytes_delta,
        type: "bar",
        name: "Included bytes per update",
        marker: {{ color: "rgba(31, 119, 180, 0.25)" }},
      }},
      {{
        x: slots,
        y: smoothedAttemptedBytes,
        mode: "lines",
        name: `Attempted moving average (${{volumeWindow}} points)`,
        line: {{ width: 3, color: "rgb(214, 39, 40)" }},
      }},
      {{
        x: slots,
        y: smoothedIncludedBytes,
        mode: "lines",
        name: `Included moving average (${{volumeWindow}} points)`,
        line: {{ width: 3, color: "rgb(31, 119, 180)" }},
      }},
    ], {{
      title: "Attempted vs Included Transaction Volume (bytes)",
      barmode: "overlay",
      xaxis: {{ title: "Slot" }},
      yaxis: {{ title: "Bytes" }},
      margin: {{ t: 40, r: 20, b: 40, l: 50 }},
    }});

    Plotly.newPlot("tier-delays", tierTraces("delays_by_tier"), {{
      title: "Tier Delays (slots)",
      xaxis: {{ title: "Slot" }},
      yaxis: {{ title: "Delay" }},
      margin: {{ t: 40, r: 20, b: 40, l: 50 }},
    }});

    Plotly.newPlot("tier-utils", tierTraces("utils_by_tier"), {{
      title: "Tier Utilisation",
      xaxis: {{ title: "Slot" }},
      yaxis: {{ title: "Utilisation" }},
      margin: {{ t: 40, r: 20, b: 40, l: 50 }},
    }});

    if (payload.include_cumulative) {{
      Plotly.newPlot("cumulative", [
        {{
          x: slots,
          y: payload.cumulative_inclusions,
          mode: "lines",
          name: "Cumulative inclusions (total)",
          line: {{ width: 2, color: "rgb(31, 119, 180)" }},
        }},
        {{
          x: slots,
          y: payload.cumulative_rb_inclusions,
          mode: "lines",
          name: "Cumulative RB inclusions",
          line: {{ width: 2, color: "rgb(255, 127, 14)" }},
        }},
        {{
          x: slots,
          y: payload.cumulative_eb_inclusions,
          mode: "lines",
          name: "Cumulative EB inclusions",
          line: {{ width: 2, color: "rgb(44, 160, 44)" }},
        }},
        {{
          x: slots,
          y: payload.cumulative_submitted_bytes,
          mode: "lines",
          name: "Cumulative attempted bytes",
          line: {{ width: 2, color: "rgb(214, 39, 40)" }},
        }},
        {{
          x: slots,
          y: payload.cumulative_included_bytes,
          mode: "lines",
          name: "Cumulative included bytes",
        }},
        {{
          x: slots,
          y: payload.cumulative_fees,
          mode: "lines",
          name: "Cumulative fees",
        }},
      ], {{
        title: "Cumulative Inclusions / Bytes / Fees",
        xaxis: {{ title: "Slot" }},
        yaxis: {{ title: "Value" }},
        margin: {{ t: 40, r: 20, b: 40, l: 50 }},
      }});
    }}
  </script>
</body>
</html>
"""


def main() -> int:
    parser = argparse.ArgumentParser(description="Plot tiered pricing time-series CSV.")
    parser.add_argument("csv", help="Path to time_series.csv")
    parser.add_argument(
        "--output",
        "-o",
        help="Output HTML path (default: alongside CSV as tiered_plot.html)",
    )
    parser.add_argument("--title", default="Tiered Pricing Time Series")
    parser.add_argument(
        "--include-cumulative",
        dest="include_cumulative",
        action="store_true",
        default=True,
        help="Include cumulative inclusions/fees subplot (default: enabled)",
    )
    parser.add_argument(
        "--no-cumulative",
        dest="include_cumulative",
        action="store_false",
        help="Disable cumulative subplot",
    )
    args = parser.parse_args()

    rows = load_points(args.csv)
    if not rows:
        raise SystemExit("No rows found in CSV.")

    series = build_series(rows)
    if series is None:
        raise SystemExit("No data available to plot.")

    output_path = args.output
    if output_path is None:
        output_path = os.path.join(os.path.dirname(args.csv), "tiered_plot.html")

    html = build_html(series, args.title, args.include_cumulative)
    with open(output_path, "w", encoding="utf-8") as handle:
        handle.write(html)
    print(f"Wrote {output_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
