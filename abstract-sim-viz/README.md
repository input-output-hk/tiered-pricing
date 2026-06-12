# abstract-sim-viz

Interactive, SVG-exportable dashboard for `abstract-sim-hs` event traces. Focuses on
price per lane, price convergence, price shock, and transaction latency per urgency class.

## Usage

```bash
# 1. Distil one or more traces into dashboard/data.js (streaming; stdlib only)
python preprocess.py ../abstract-sim-hs/events.jsonl
# e.g. a whole sweep — every variant with all its seeds:
python preprocess.py sweep-results/example/*.events.jsonl

# 2. Open the dashboard
open dashboard/index.html        # or just open the file in a browser
# (works from file://; alternatively: python -m http.server -d dashboard)
```

Options: `--shock-threshold` (default 0.10), `--band-pct` (0.05),
`--load-change-pct` (0.10), `--target-buckets` (300), `-o/--output`.

With several traces the header grows a two-level run selector: an experiment
dropdown (`[` / `]` to cycle) and, when an experiment has several seeds, a seed
dropdown (`{` / `}`). Switching experiments keeps the current seed number where
it exists. Panels re-render in place — zoom and toggles carry over between
equal-length runs — and the price panels pin a shared y-domain across runs so
coefficient excursions stay visually comparable while flipping. Grouping comes
from the trace filenames: `two-lane-open-seed3.events.jsonl` is experiment
`two-lane-open`, seed 3; names without a `-seed<N>` suffix stand alone.

## What it shows

- **Price coefficient / lane** — log overlay or per-lane small multiples, with a ±5%
  convergence band. Toggle the view top-right.
- **Price shock** — relative jump `|Δ|/old` per price update, against the 10% threshold.
- **Latency / urgency class** — median over time with a median→p95 tail band, plus a
  per-class distribution box (IQR / median / p95 / max) and summary table.
- **Load strip** — submissions/slot; brush it to zoom all time panels; double-click resets.

## Assumptions & fidelity

- Metric definitions mirror the simulator (`Metrics.Price`, `Metrics.Latency`,
  `Metrics.Accumulator`): jump `|Δ|/old`, shock threshold 0.10, convergence band ±5%,
  latency = inclusion − submission (last-wins by txId), the `quantile` rule, run length
  `1 + maxSlot`.
- **Load regimes are inferred** from observed submissions/slot, because `events.jsonl`
  does not contain the run configuration. If the simulator later emits its config, the
  preprocessor can be extended to use exact regimes.

## Tests

`python -m pytest` (covers the preprocessor; the dashboard is verified manually).
