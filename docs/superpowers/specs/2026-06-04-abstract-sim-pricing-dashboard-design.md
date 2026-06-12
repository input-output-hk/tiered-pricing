# Design: Pricing & Latency Dashboard for `abstract-sim-hs`

**Date:** 2026-06-04
**Status:** Approved (pending implementation plan)
**Author:** Will Gould (with Claude)

## 1. Purpose

`abstract-sim-hs` simulates linear-Leios under candidate dynamic-pricing designs and
writes a per-event trace to `events.jsonl`. We need a **professional, interactive
visualization** of that trace to understand pricing behaviour and its effect on
transactions — with figures clean enough to export into the CIP for urgency signalling.

The dashboard focuses on four things:

1. **Price (per lane)** — the dynamic price coefficient over time, per lane.
2. **Price convergence** — whether and how fast price settles after load changes.
3. **Price shock** — how violently the price moved, against the simulator's threshold.
4. **Transaction latency (per urgency class)** — inclusion latency distribution and dynamics.

## 2. Goals & non-goals

**Goals**
- Interactive browser dashboard for exploration (zoom, hover, filter), where each panel
  renders as crisp SVG that can be exported/screenshotted into the CIP.
- Faithful to the simulator's own metric definitions (`Metrics.Price`, `Metrics.Latency`,
  `Metrics.Accumulator`) so dashboard figures and computed metrics agree.
- Robust to **arbitrary run length** and to whichever urgency classes / lanes appear in
  the trace — nothing hardcoded to the current 2,000-slot example.
- Zero build step; runs offline; no heavyweight dependencies.

**Non-goals (this iteration)**
- Visualizing every metric the simulator computes (revenue, fairness, throughput,
  invariant breaches, value retention). Out of scope; the data contract leaves room to add
  them later.
- Comparing multiple runs side by side. Single-run dashboard for now.
- Reading the run configuration (load process, band %) — the trace does not contain it;
  see §8 assumptions.
- Browser automation tests (Playwright). Explicitly skipped.

## 3. Architecture

A **two-stage pipeline** with a small JSON object as the seam between halves:

```
events.jsonl ──► preprocess.py (one streaming pass) ──► dashboard/data.js ──► index.html (Observable Plot)
   121 MB              stdlib Python                       few KB                static, file://-friendly
```

- The preprocessor does the one expensive thing — a single streaming pass over the
  ~719k-event file — and reduces it to a few KB of aggregates. The browser only ever sees
  the small summary, so the heavy file never goes near it and the page stays fast.
- The seam is the **data contract** (§5). The preprocessor's job is "produce this object
  faithfully"; the dashboard's job is "render this object." Each half is understandable
  and testable against the contract independently.

**Technology choices**
- **Preprocessor:** Python, **standard library only** (`json`, `argparse`, `collections`,
  `statistics`). No pandas/numpy. Streams line-by-line — never loads the file into memory.
- **Dashboard:** static HTML + **Observable Plot** (D3-based; clean SVG, professional
  defaults, export-friendly). Plot + D3 are **vendored** (UMD bundles in `dashboard/vendor/`)
  so the page works offline with no CDN dependency.
- **Delivery:** the preprocessor emits `dashboard/data.js` assigning
  `window.SIM_DATA = {…}`, included via a `<script>` tag. This means the dashboard works
  directly from `file://` — no local server, no CORS. (`python -m http.server` remains an
  alternative.)

## 4. Project structure

A new sibling directory, **outside `abstract-sim-hs`**:

```
abstract-sim-viz/
  README.md                 # how to generate + open; documents assumptions
  preprocess.py             # events.jsonl → dashboard/data.js  (streaming)
  pyproject.toml            # metadata + pytest dev dependency
  dashboard/
    index.html              # static page, no build step
    app.js                  # Observable Plot rendering + interactions
    style.css               # light (default) ⇄ dark themes
    vendor/                 # Observable Plot + D3 UMD bundles (vendored)
    data.js                 # GENERATED: window.SIM_DATA = {...}  (gitignored)
  tests/
    test_preprocess.py
    fixtures/tiny.jsonl     # hand-written ~20-line trace with known expected outputs
```

**How to run**
1. `python preprocess.py ../abstract-sim-hs/events.jsonl` → writes `dashboard/data.js`.
2. Open `dashboard/index.html` in a browser.

## 5. Data contract (`window.SIM_DATA`)

```jsonc
{
  "meta": {
    "source": "events.jsonl",
    "generatedAt": "<ISO-8601>",
    "slotCount": <int>,            // 1 + max slot observed (mirrors observedSlots)
    "totalEvents": <int>,
    "lanes": ["Standard", "Priority"],          // lanes actually present, sim order
    "urgencyClasses": [                          // ordered by rate, low → high
      { "id": "Exponential:5e-4", "tag": "Exponential", "rate": 0.0005, "label": "Exp λ=5e-4" }
    ]
  },
  "params": { "shockThreshold": 0.10, "convergenceBandPct": 0.05, "loadChangePct": 0.10 },

  "price": {
    "byLane": {
      "<lane>": [ { "slot", "oldCoeff", "newCoeff", "utilisation", "jump" } ]   // one per PriceUpdated, in slot order
    }
  },

  "shock": {
    "byLane": { "<lane>": { "maxJump": <float>, "shockCount": <int> } }
  },

  "convergence": {
    "loadRegimes": [ { "start", "end", "meanArrival" } ],   // inferred from submissions/slot
    "byLane": {
      "<lane>": {
        "convergenceTime": <int|null>,        // max across regimes of (convergenceSlot − regimeStart); null = never
        "oscillationAmplitude": <float>,      // peak-to-peak of this lane's coeffs across the run (max − min)
        "regimes": [
          { "start", "end", "reference", "band": [<lo>, <hi>], "convergenceSlot": <int|null> }
        ]
      }
    }
  },

  "latency": {
    "byClass": {
      "<class id>": {
        "count": <int>,
        "mean": <float>,                                                           // slots
        "median": <int>, "p25": <int>, "p75": <int>, "p95": <int>, "max": <int>,   // box/violin quantiles
        "histogram": { "binWidth": <int>, "bins": [ { "lo", "hi", "n" } ] },        // binWidth shared across classes
        "overTime": [ { "slot": <bucketStart>, "median": <int>, "p95": <int>, "n": <int> } ]  // bucketed by submit slot
      }
    }
  },

  "load": {
    "bucketWidth": <int>,
    "buckets": [ { "slot": <bucketStart>, "submissions": <int>, "inclusions": <int> } ]
  }
}
```

**Adaptive bucketing.** Time series (`latency.overTime`, `load.buckets`) target ~300
buckets across the run: `bucketWidth = max(1, ceil(slotCount / 300))`. This keeps series
crisp whether the run is 200 or 200,000 slots.

## 6. Preprocessing algorithm (single streaming pass)

For each JSONL line, dispatch on `event.tag`:

- **`TxSubmitted`** — record `submittedAt[txId] = slot` and `txMeta[txId] = (urgency, lane)`
  (last-write-wins, mirroring `Map.insert`); increment `submissions` for the slot's bucket.
- **`TxIncluded`** — record `includedAt[txId] = slot` (last-write-wins); increment
  `inclusions` for the slot's bucket.
- **`PriceUpdated`** — append `{slot, oldCoeff, newCoeff, utilisation, jump}` to that lane's
  price series, where `jump = |newCoeff − oldCoeff| / oldCoeff` (0 if `oldCoeff ≤ 0`),
  mirroring `relativeJump`.
- Other tags ignored (this iteration).

Two-pass note: bucket count depends on `slotCount`, which we only know after the first
pass. Implementation may either (a) buffer raw per-event minimal tuples and bucket after
determining `slotCount`, or (b) accumulate per-slot counts in a dict and bucket at the
end. Option (b) keeps memory bounded by slot count, not event count — preferred.

**After the pass**, compute:
- `latency[txId] = includedAt − submittedAt` for every txId present in both maps; group by
  urgency class → per-class list of latencies. From each list compute count/mean and the
  quantiles median/p25/p75/p95/max (via `quantile q xs = xs[min(n-1, ceil(q*n)-1)]`, mirroring
  `Metrics.Latency`), a histogram using a **bin width shared across all classes** (so the
  distribution panel is comparable — e.g. a nice-rounded `globalP99 / 30`), and the `overTime`
  series (bucket each latency by its **submit** slot; per bucket emit median, p95, n).
- Per-lane `maxJump` and `shockCount = #{jump > shockThreshold}` from the price series.
- **Load regimes:** smooth `submissions`/slot, walk slots detecting material change
  (`|new−old|/old > loadChangePct`, with the `old ≤ 0` edge case from `materialLoadChange`),
  emit `{start, end, meanArrival}` regimes.
- **Convergence (per lane, per regime):** within each regime, take the reference = price at
  the slot before the regime end; find the first candidate slot from which the price and all
  later in-regime prices stay within `±convergenceBandPct` of the reference (its `band` is
  `reference·(1±bandPct)`); record that `convergenceSlot`. The lane's `convergenceTime` is the
  **max across regimes** of `convergenceSlot − regimeStart` (`null` if any regime never
  converges), mirroring `convergenceTimeFrom`. `oscillationAmplitude` = peak-to-peak (max − min)
  of the lane's old+new coeffs across the run, mirroring `amplitude`; the run-level KPI is the
  max across lanes.

## 7. Dashboard panels & interactions

**Layout:** stacked, shared-time-axis (left column) + latency distribution (right column),
with a KPI strip on top. Three of four metrics are slot-indexed, so the shared axis lets
shock → latency causation be read directly.

1. **Price coefficient / lane** — toggle between *overlaid log-y* (default; ticks 1·2·4·8·16)
   and *per-lane small multiples* (each own linear y). The ±5% convergence band is shaded
   behind each lane's settling price; convergence is read on this panel, not a separate one.
2. **Price shock** — a stem per `PriceUpdated` at `jump`, colored by lane, with the 10%
   threshold as a dashed rule; stems above it get a marker.
3. **Latency / urgency class (over time)** — one median line per class (ordered blue ramp,
   low→high decay), bucketed by submit slot, with a toggleable translucent median→p95 band
   (the band's upper edge tracks the worst-affected 5% — tail latency — over time).
4. **Latency distribution** (right, non-time) — box/violin per class + a compact
   median/p95/n table. The headline "do urgent txs clear faster?" comparison.

Plus a thin **load strip** (submissions & inclusions per bucket) with inferred regime
boundaries, and a **KPI strip** (per-lane convergence time, max price jump, shock count,
oscillation).

**Interactions:** shared crosshair across the time panels; brush-to-zoom on the time axis
(rescales all time panels together); hover tooltips; legend click to toggle lanes/classes;
price view toggle (log ⇄ per-lane); light⇄dark theme toggle (light default); per-panel
**"export SVG"** button (serializes the Plot SVG node to a download).

**Colour:** colourblind-safe categorical hues for the two lanes; an **ordered** blue ramp
for the urgency classes (encodes the natural low→high decay ordering).

## 8. Fidelity & stated assumptions

All computations mirror the Haskell metric code:

| Concept | Definition | Source |
|---|---|---|
| Price jump | `|new−old|/old`, 0 if `old ≤ 0` | `relativeJump` (Accumulator) |
| Shock threshold | `0.10` | `priceShockThreshold` (Price) |
| maxJump / shockCount | max jump; `#{jump > 0.10}` | `priceShockFrom` |
| Convergence band | `±0.05` | `metricsConfigDefault` |
| Material load change | `>0.10` relative | `materialLoadChange` / default |
| Convergence time | first slot price enters & stays in band per regime; max across regimes | `convergenceTimeFrom` |
| Oscillation amplitude | peak-to-peak (max−min) of a lane's coeffs; run KPI = max across lanes | `amplitude` / `priceStabilityFrom` |

> **Update (2026-06-12):** the simulator's own metrics no longer use load
> regimes. `convergenceTime` is now the settling time against each lane's
> *final* coefficient (max across lanes; `null` if a lane was still out of
> band at its last update), and `oscillationAmplitude` is the peak-to-peak
> ripple *after* settling (full-run swing for a never-settling lane). The
> dashboard's inferred load regimes remain a display concern; its per-regime
> convergence summaries no longer mirror a simulator metric.
| Latency | `includedAt − submittedAt`, last-wins by txId, both events required | `includedLatency` |
| Percentiles | `xs[min(n-1, ceil(q*n)-1)]` | `quantile` (Latency) |
| Run length | `1 + max slot` | `observedSlots` |
| Price quantity | dynamic **coefficient** (multiplier on min-fee), not absolute fee | `PriceUpdated` |

**Key assumption — load regimes are inferred.** `events.jsonl` is the only artifact the
simulator writes; the run config (arrival process, band %) is *not* in the trace. Observed
submissions-per-slot are the realization of the arrival process, so we infer regimes from
them. This is documented prominently in the README and noted on the load strip. The data
contract leaves room to later accept an optional explicit `--load-config` if the simulator
starts emitting one (deferred — YAGNI now).

## 9. Testing

- **`pytest` on `preprocess.py`** against a hand-written `tests/fixtures/tiny.jsonl`
  (~20 lines with known expected outputs), asserting:
  - slot-count derivation (`1 + max slot`);
  - price-series extraction and `jump` values;
  - `shockCount` vs the 10% threshold and `maxJump`;
  - latency join correctness, **including duplicate-`txId` last-wins**;
  - percentile + histogram correctness against the `quantile` rule;
  - load-regime detection across a known material load change;
  - convergence time on a constructed converging vs never-converging series.
- **Dashboard:** manual visual verification — open `index.html`, confirm all panels render
  from `data.js`, toggles (price view, theme, legend, p95 band) work, brush-zoom syncs the
  time panels, and SVG export downloads a valid file. (Playwright smoke test explicitly out
  of scope.)

## 10. Out of scope / future

- Additional metrics (revenue, fairness, throughput, value retention, invariant breaches).
- Multi-run comparison.
- Reading an explicit run-config sidecar for exact (non-inferred) load regimes.
