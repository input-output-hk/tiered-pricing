# abstract-sim-viz Preprocessor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a streaming Python preprocessor that distills `abstract-sim-hs/events.jsonl` into a compact `dashboard/data.js` (`window.SIM_DATA = {…}`) matching the data contract, faithful to the simulator's metric definitions.

**Architecture:** A `simviz` stdlib-only package of focused, pure modules (stats, ingest, price, load, latency, contract) driven by a thin `preprocess.py` CLI. One streaming pass over the trace builds an `Accumulator`; pure functions derive price/shock/convergence/latency/load aggregates; `contract.build_sim_data` assembles the contract object and `write_data_js` serialises it.

**Tech Stack:** Python 3.10+, standard library only (`json`, `argparse`, `collections`, `statistics`, `math`, `datetime`). pytest for tests. No pandas/numpy.

**Spec:** `docs/superpowers/specs/2026-06-04-abstract-sim-pricing-dashboard-design.md` (§5 contract, §6 algorithm, §8 fidelity).

---

## File structure

```
abstract-sim-viz/
  .gitignore                # dashboard/data.js, __pycache__/, .pytest_cache/
  pyproject.toml            # metadata, pytest config (pythonpath=".")
  preprocess.py             # CLI orchestrator (thin)
  simviz/
    __init__.py
    stats.py                # quantile, mean, relative_jump, histogram_bins
    ingest.py               # iter_events, Accumulator
    price.py                # price_series, shock_stats, price_at_or_before, convergence_for_lane, oscillation_amplitude
    load.py                 # bucket_width, load_buckets, smooth_rate, detect_regimes
    latency.py              # class_id, join_latencies, class_stats, over_time
    contract.py             # urgency_classes, build_sim_data, write_data_js
  tests/
    test_stats.py
    test_ingest.py
    test_price.py
    test_load.py
    test_latency.py
    test_contract.py
    test_preprocess_integration.py
    fixtures/tiny.jsonl
```

Each `simviz/*.py` has one responsibility and is imported by `contract.py` / `preprocess.py`. Tests mirror modules 1:1, plus one end-to-end integration test.

---

## Task 1: Project scaffold

**Files:**
- Create: `abstract-sim-viz/pyproject.toml`
- Create: `abstract-sim-viz/.gitignore`
- Create: `abstract-sim-viz/simviz/__init__.py` (empty)
- Create: `abstract-sim-viz/tests/__init__.py` (empty)
- Create: `abstract-sim-viz/tests/test_smoke.py`

- [ ] **Step 1: Create `pyproject.toml`**

```toml
[project]
name = "abstract-sim-viz"
version = "0.1.0"
description = "Visualization preprocessor for abstract-sim-hs event traces"
requires-python = ">=3.10"

[project.optional-dependencies]
dev = ["pytest>=7"]

[tool.pytest.ini_options]
pythonpath = ["."]
testpaths = ["tests"]
```

- [ ] **Step 2: Create `.gitignore`**

```gitignore
dashboard/data.js
__pycache__/
.pytest_cache/
*.pyc
```

- [ ] **Step 3: Create empty `simviz/__init__.py` and `tests/__init__.py`**

Both files are empty (package markers).

- [ ] **Step 4: Write a smoke test** in `tests/test_smoke.py`

```python
def test_python_environment():
    assert 1 + 1 == 2
```

- [ ] **Step 5: Run the smoke test**

Run (from `abstract-sim-viz/`): `python -m pytest -q`
Expected: `1 passed`.

- [ ] **Step 6: Commit**

```bash
git add abstract-sim-viz/pyproject.toml abstract-sim-viz/.gitignore abstract-sim-viz/simviz/__init__.py abstract-sim-viz/tests/__init__.py abstract-sim-viz/tests/test_smoke.py
git commit -m "chore(viz): scaffold abstract-sim-viz preprocessor project"
```

---

## Task 2: `stats.py` — quantile, mean, relative_jump

**Files:**
- Create: `abstract-sim-viz/simviz/stats.py`
- Test: `abstract-sim-viz/tests/test_stats.py`

- [ ] **Step 1: Write the failing tests** in `tests/test_stats.py`

```python
from simviz.stats import quantile, mean, relative_jump


def test_quantile_matches_haskell_rule():
    # quantile q xs = xs[min(n-1, ceil(q*n)-1)]
    assert quantile(0.50, [1, 2, 3, 4]) == 2        # ceil(2)-1 = 1
    assert quantile(0.95, [1, 2, 3, 4]) == 4        # ceil(3.8)-1 = 3
    assert quantile(0.50, [10, 20, 30]) == 20       # ceil(1.5)-1 = 1
    assert quantile(0.25, [1, 2, 3, 4]) == 1        # ceil(1)-1 = 0


def test_quantile_empty_is_zero():
    assert quantile(0.5, []) == 0


def test_mean():
    assert mean([]) == 0.0
    assert mean([1, 2, 3]) == 2.0


def test_relative_jump():
    assert relative_jump(16, 10) == 0.375
    assert relative_jump(0, 5) == 0.0      # old <= 0 -> 0
    assert relative_jump(4, 4) == 0.0
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest tests/test_stats.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'simviz.stats'`.

- [ ] **Step 3: Implement `simviz/stats.py`**

```python
import math


def mean(xs):
    return sum(xs) / len(xs) if xs else 0.0


def relative_jump(old_coeff, new_coeff):
    """Mirror Metrics.Accumulator.relativeJump: |new-old|/old, 0 if old <= 0."""
    if old_coeff <= 0:
        return 0.0
    return abs(new_coeff - old_coeff) / old_coeff


def quantile(q, sorted_xs):
    """Mirror Metrics.Latency.quantile: xs[min(n-1, max(0, ceil(q*n)-1))]."""
    n = len(sorted_xs)
    if n == 0:
        return 0
    idx = min(n - 1, max(0, math.ceil(q * n) - 1))
    return sorted_xs[idx]
```

- [ ] **Step 4: Run to verify pass**

Run: `python -m pytest tests/test_stats.py -q`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add abstract-sim-viz/simviz/stats.py abstract-sim-viz/tests/test_stats.py
git commit -m "feat(viz): add quantile, mean, relative_jump matching sim definitions"
```

---

## Task 3: `stats.py` — histogram_bins

**Files:**
- Modify: `abstract-sim-viz/simviz/stats.py`
- Test: `abstract-sim-viz/tests/test_stats.py`

- [ ] **Step 1: Add the failing test** to `tests/test_stats.py`

```python
from simviz.stats import histogram_bins


def test_histogram_bins_basic():
    bins = histogram_bins([0, 1, 5, 9], 5)
    assert bins == [
        {"lo": 0, "hi": 5, "n": 2},    # 0, 1
        {"lo": 5, "hi": 10, "n": 2},   # 5, 9
    ]


def test_histogram_bins_empty():
    assert histogram_bins([], 5) == []
    assert histogram_bins([1, 2], 0) == []
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest tests/test_stats.py::test_histogram_bins_basic -q`
Expected: FAIL — `ImportError: cannot import name 'histogram_bins'`.

- [ ] **Step 3: Append to `simviz/stats.py`**

```python
def histogram_bins(values, bin_width):
    """Fixed-width bins over [0, max(values)]; returns [{lo, hi, n}]."""
    if not values or bin_width <= 0:
        return []
    n_bins = int(max(values) // bin_width) + 1
    counts = [0] * n_bins
    for v in values:
        idx = int(v // bin_width)
        idx = 0 if idx < 0 else min(idx, n_bins - 1)
        counts[idx] += 1
    return [
        {"lo": i * bin_width, "hi": (i + 1) * bin_width, "n": counts[i]}
        for i in range(n_bins)
    ]
```

- [ ] **Step 4: Run to verify pass**

Run: `python -m pytest tests/test_stats.py -q`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add abstract-sim-viz/simviz/stats.py abstract-sim-viz/tests/test_stats.py
git commit -m "feat(viz): add histogram_bins"
```

---

## Task 4: `ingest.py` — iter_events

**Files:**
- Create: `abstract-sim-viz/simviz/ingest.py`
- Test: `abstract-sim-viz/tests/test_ingest.py`

- [ ] **Step 1: Write the failing test** in `tests/test_ingest.py`

```python
from simviz.ingest import iter_events


def test_iter_events_yields_inner_event_objects(tmp_path):
    f = tmp_path / "trace.jsonl"
    f.write_text(
        '{"event":{"tag":"TxAdmitted","slot":0,"txId":1},"eventNo":0}\n'
        '\n'  # blank line is skipped
        '{"event":{"tag":"TxAdmitted","slot":1,"txId":2},"eventNo":1}\n'
    )
    events = list(iter_events(str(f)))
    assert events == [
        {"tag": "TxAdmitted", "slot": 0, "txId": 1},
        {"tag": "TxAdmitted", "slot": 1, "txId": 2},
    ]
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest tests/test_ingest.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'simviz.ingest'`.

- [ ] **Step 3: Create `simviz/ingest.py`** (iter_events only)

```python
import json
from collections import defaultdict


def iter_events(path):
    """Stream a JSONL trace, yielding each line's inner `event` object in order."""
    with open(path, "r") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            yield json.loads(line)["event"]
```

- [ ] **Step 4: Run to verify pass**

Run: `python -m pytest tests/test_ingest.py -q`
Expected: PASS (1 test).

- [ ] **Step 5: Commit**

```bash
git add abstract-sim-viz/simviz/ingest.py abstract-sim-viz/tests/test_ingest.py
git commit -m "feat(viz): stream events.jsonl with iter_events"
```

---

## Task 5: `ingest.py` — Accumulator

**Files:**
- Modify: `abstract-sim-viz/simviz/ingest.py`
- Test: `abstract-sim-viz/tests/test_ingest.py`

- [ ] **Step 1: Add the failing tests** to `tests/test_ingest.py`

```python
from simviz.ingest import Accumulator


def _submitted(tx_id, slot, lane, rate, tag="Exponential"):
    return {
        "tag": "TxSubmitted", "slot": slot, "actorId": 0,
        "tx": {
            "id": tx_id, "lane": lane, "submitted": slot, "value": 100,
            "urgency": {"tag": tag, "rate": rate},
            "body": {"sizeBytes": 1, "script": {"sizeBytes": 0, "exUnits": 0},
                     "dependsOn": [], "fee": 1},
        },
    }


def _included(tx_id, slot):
    return {"tag": "TxIncluded", "slot": slot, "txId": tx_id,
            "inclusionPoint": {"tag": "IncludedInRb"}}


def _price(lane, slot, old, new, util):
    return {"tag": "PriceUpdated", "slot": slot, "lane": lane,
            "oldCoeff": old, "newCoeff": new, "utilisation": util}


def test_accumulator_records_state():
    acc = Accumulator()
    for e in [
        _submitted(1, 0, "Standard", 5.0e-4),
        _included(1, 2),
        _price("Priority", 1, 16, 10, 0.4),
    ]:
        acc.ingest(e)
    assert acc.submitted_at == {1: 0}
    assert acc.included_at == {1: 2}
    assert acc.tx_meta[1] == {"tag": "Exponential", "rate": 5.0e-4, "lane": "Standard"}
    assert acc.submissions_per_slot[0] == 1
    assert acc.inclusions_per_slot[2] == 1
    assert acc.price_changes["Priority"][0]["newCoeff"] == 10
    assert acc.slot_count == 3          # max slot 2 -> +1
    assert acc.total_events == 3


def test_accumulator_last_wins_on_duplicate_txid():
    acc = Accumulator()
    acc.ingest(_submitted(1, 0, "Standard", 5.0e-4))
    acc.ingest(_submitted(1, 5, "Priority", 6.0e-3))   # resubmission, same id
    acc.ingest(_included(1, 3))
    acc.ingest(_included(1, 9))                         # later inclusion wins
    assert acc.submitted_at[1] == 5
    assert acc.tx_meta[1]["lane"] == "Priority"
    assert acc.included_at[1] == 9
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest tests/test_ingest.py -q`
Expected: FAIL — `ImportError: cannot import name 'Accumulator'`.

- [ ] **Step 3: Append the `Accumulator` class** to `simviz/ingest.py`

```python
class Accumulator:
    """Single-pass accumulator over a SimEvent trace. last-write-wins by txId,
    mirroring the simulator's Map.insert semantics."""

    def __init__(self):
        self.submitted_at = {}              # txId -> submit slot
        self.tx_meta = {}                   # txId -> {"tag", "rate", "lane"}
        self.included_at = {}               # txId -> inclusion slot
        self.price_changes = defaultdict(list)     # lane -> [PriceUpdated event]
        self.submissions_per_slot = defaultdict(int)
        self.inclusions_per_slot = defaultdict(int)
        self.max_slot = 0
        self.total_events = 0

    def ingest(self, event):
        self.total_events += 1
        tag = event["tag"]
        slot = event.get("slot", 0)
        if slot > self.max_slot:
            self.max_slot = slot
        if tag == "TxSubmitted":
            tx = event["tx"]
            tx_id = tx["id"]
            self.submitted_at[tx_id] = tx["submitted"]
            self.tx_meta[tx_id] = {
                "tag": tx["urgency"]["tag"],
                "rate": tx["urgency"]["rate"],
                "lane": tx["lane"],
            }
            self.submissions_per_slot[tx["submitted"]] += 1
        elif tag == "TxIncluded":
            self.included_at[event["txId"]] = slot
            self.inclusions_per_slot[slot] += 1
        elif tag == "PriceUpdated":
            self.price_changes[event["lane"]].append(event)
        # all other tags ignored (this iteration)

    @property
    def slot_count(self):
        return self.max_slot + 1
```

- [ ] **Step 4: Run to verify pass**

Run: `python -m pytest tests/test_ingest.py -q`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add abstract-sim-viz/simviz/ingest.py abstract-sim-viz/tests/test_ingest.py
git commit -m "feat(viz): Accumulator with last-wins txId semantics"
```

---

## Task 6: `price.py` — price_series & shock_stats

**Files:**
- Create: `abstract-sim-viz/simviz/price.py`
- Test: `abstract-sim-viz/tests/test_price.py`

- [ ] **Step 1: Write the failing tests** in `tests/test_price.py`

```python
from simviz.ingest import Accumulator
from simviz.price import price_series, shock_stats


def _price(lane, slot, old, new, util=0.0):
    return {"tag": "PriceUpdated", "slot": slot, "lane": lane,
            "oldCoeff": old, "newCoeff": new, "utilisation": util}


def test_price_series_sorted_with_jumps():
    acc = Accumulator()
    acc.ingest(_price("Priority", 2, 10, 10.5, 0.5))
    acc.ingest(_price("Priority", 1, 16, 10, 0.4))     # out of order on purpose
    series = price_series(acc, "Priority")
    assert [p["slot"] for p in series] == [1, 2]
    assert series[0]["jump"] == 0.375                  # |10-16|/16
    assert round(series[1]["jump"], 3) == 0.05         # |10.5-10|/10


def test_shock_stats():
    series = [{"jump": 0.375}, {"jump": 0.05}, {"jump": 0.2}]
    assert shock_stats(series, 0.10) == {"maxJump": 0.375, "shockCount": 2}
    assert shock_stats([], 0.10) == {"maxJump": 0.0, "shockCount": 0}
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest tests/test_price.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'simviz.price'`.

- [ ] **Step 3: Create `simviz/price.py`** (series + shock)

```python
from simviz.stats import relative_jump


def price_series(acc, lane):
    """Ordered price-update trace for a lane, with relative jump per step."""
    changes = sorted(acc.price_changes.get(lane, []), key=lambda e: e["slot"])
    return [
        {
            "slot": e["slot"],
            "oldCoeff": e["oldCoeff"],
            "newCoeff": e["newCoeff"],
            "utilisation": e["utilisation"],
            "jump": relative_jump(e["oldCoeff"], e["newCoeff"]),
        }
        for e in changes
    ]


def shock_stats(series, threshold):
    """Mirror priceShockFrom: max jump and count of jumps strictly over threshold."""
    jumps = [p["jump"] for p in series]
    return {
        "maxJump": max(jumps) if jumps else 0.0,
        "shockCount": sum(1 for j in jumps if j > threshold),
    }
```

- [ ] **Step 4: Run to verify pass**

Run: `python -m pytest tests/test_price.py -q`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add abstract-sim-viz/simviz/price.py abstract-sim-viz/tests/test_price.py
git commit -m "feat(viz): price_series and shock_stats"
```

---

## Task 7: `price.py` — convergence & oscillation

**Files:**
- Modify: `abstract-sim-viz/simviz/price.py`
- Test: `abstract-sim-viz/tests/test_price.py`

- [ ] **Step 1: Add the failing tests** to `tests/test_price.py`

```python
from simviz.price import price_at_or_before, convergence_for_lane, oscillation_amplitude


def test_price_at_or_before():
    series = [{"slot": 1, "oldCoeff": 16, "newCoeff": 10},
              {"slot": 5, "oldCoeff": 10, "newCoeff": 12}]
    assert price_at_or_before(series, 0) == 16     # before first change -> first oldCoeff
    assert price_at_or_before(series, 1) == 10     # newCoeff of change at slot 1
    assert price_at_or_before(series, 4) == 10
    assert price_at_or_before(series, 9) == 12
    assert price_at_or_before([], 3) is None


def test_convergence_converges_within_band():
    # one regime [0, 10). Price settles at 10 (reference at slot 9 = 10).
    series = [{"slot": 1, "oldCoeff": 16, "newCoeff": 10},
              {"slot": 2, "oldCoeff": 10, "newCoeff": 10}]
    regimes = [{"start": 0, "end": 10, "meanArrival": 2.0}]
    results, conv_time = convergence_for_lane(series, regimes, 0.05)
    assert results[0]["convergenceSlot"] == 1      # from slot 1 onward all within 5% of 10
    assert conv_time == 1                           # 1 - regimeStart(0)


def test_convergence_none_without_price_data():
    # The reference is the regime's final settled price, so any regime WITH price data
    # always converges (at worst at its last change). convergence_time is therefore None
    # only when there is no price data (no reference) or a degenerate regime.
    results, conv_time = convergence_for_lane(
        [], [{"start": 0, "end": 5, "meanArrival": 2.0}], 0.05)
    assert conv_time is None
    assert results[0]["convergenceSlot"] is None


def test_oscillation_amplitude():
    series = [{"oldCoeff": 16, "newCoeff": 10}, {"oldCoeff": 10, "newCoeff": 12}]
    assert oscillation_amplitude(series) == 6      # max(16,10,10,12) - min(...) = 16 - 10
    assert oscillation_amplitude([]) == 0.0
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest tests/test_price.py -q`
Expected: FAIL — `ImportError: cannot import name 'price_at_or_before'`.

- [ ] **Step 3: Append to `simviz/price.py`**

```python
def price_at_or_before(series, slot):
    """Mirror priceAtOrBefore: newCoeff of last change <= slot; else first oldCoeff; else None."""
    prior = [p for p in series if p["slot"] <= slot]
    if prior:
        return prior[-1]["newCoeff"]
    if series:
        return series[0]["oldCoeff"]
    return None


def _within_band(band_pct, reference, price):
    return abs(price - reference) <= abs(reference) * max(0.0, band_pct)


def convergence_for_lane(series, regimes, band_pct):
    """Per-regime convergence + lane summary, mirroring convergenceTimeFrom/convergenceInRegime.

    Returns (regime_results, convergence_time) where convergence_time is the max across
    regimes of (convergenceSlot - regimeStart), or None if any regime never converges.
    """
    regime_results = []
    times = []
    any_unconverged = False
    for regime in regimes:
        start, end = regime["start"], regime["end"]
        reference = price_at_or_before(series, max(0, end - 1)) if end > start else None
        band = None
        conv_slot = None
        if reference is not None and end > start:
            band = [reference * (1 - band_pct), reference * (1 + band_pct)]
            in_regime = [p for p in series if start <= p["slot"] < end]
            candidates = [start] + [p["slot"] for p in in_regime]
            for cand in candidates:
                cand_price = price_at_or_before(series, cand)
                if cand_price is None:
                    continue
                future = [p["newCoeff"] for p in in_regime if p["slot"] > cand]
                if all(_within_band(band_pct, reference, x) for x in [cand_price] + future):
                    conv_slot = cand
                    break
        regime_results.append({
            "start": start, "end": end,
            "reference": reference, "band": band,
            "convergenceSlot": conv_slot,
        })
        if conv_slot is None:
            any_unconverged = True
        else:
            times.append(conv_slot - start)
    convergence_time = None if (any_unconverged or not times) else max(times)
    return regime_results, convergence_time


def oscillation_amplitude(series):
    """Mirror amplitude: peak-to-peak (max-min) of all old+new coeffs across the run."""
    coeffs = []
    for p in series:
        coeffs.append(p["oldCoeff"])
        coeffs.append(p["newCoeff"])
    return (max(coeffs) - min(coeffs)) if coeffs else 0.0
```

- [ ] **Step 4: Run to verify pass**

Run: `python -m pytest tests/test_price.py -q`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add abstract-sim-viz/simviz/price.py abstract-sim-viz/tests/test_price.py
git commit -m "feat(viz): per-regime convergence and oscillation amplitude"
```

---

## Task 8: `load.py` — bucket_width & load_buckets

**Files:**
- Create: `abstract-sim-viz/simviz/load.py`
- Test: `abstract-sim-viz/tests/test_load.py`

- [ ] **Step 1: Write the failing tests** in `tests/test_load.py`

```python
from collections import defaultdict
from simviz.load import bucket_width, load_buckets


def test_bucket_width_adaptive():
    assert bucket_width(6, target_buckets=300) == 1
    assert bucket_width(2000, target_buckets=300) == 7      # ceil(2000/300)
    assert bucket_width(0, target_buckets=300) == 1         # never below 1


def test_load_buckets_counts_per_window():
    subs = defaultdict(int, {0: 2, 1: 1, 3: 4})
    incs = defaultdict(int, {1: 1, 2: 2})
    buckets = load_buckets(subs, incs, slot_count=4, width=2)
    assert buckets == [
        {"slot": 0, "submissions": 3, "inclusions": 1},     # slots 0,1
        {"slot": 2, "submissions": 4, "inclusions": 2},     # slots 2,3
    ]
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest tests/test_load.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'simviz.load'`.

- [ ] **Step 3: Create `simviz/load.py`** (bucket_width + load_buckets)

```python
import math
from simviz.stats import mean


def bucket_width(slot_count, target_buckets=300):
    return max(1, math.ceil(slot_count / target_buckets)) if slot_count > 0 else 1


def load_buckets(submissions_per_slot, inclusions_per_slot, slot_count, width):
    buckets = []
    for start in range(0, slot_count, width):
        end = min(start + width, slot_count)
        subs = sum(submissions_per_slot.get(s, 0) for s in range(start, end))
        incs = sum(inclusions_per_slot.get(s, 0) for s in range(start, end))
        buckets.append({"slot": start, "submissions": subs, "inclusions": incs})
    return buckets
```

- [ ] **Step 4: Run to verify pass**

Run: `python -m pytest tests/test_load.py -q`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add abstract-sim-viz/simviz/load.py abstract-sim-viz/tests/test_load.py
git commit -m "feat(viz): adaptive bucket_width and load_buckets"
```

---

## Task 9: `load.py` — smooth_rate & detect_regimes

**Files:**
- Modify: `abstract-sim-viz/simviz/load.py`
- Test: `abstract-sim-viz/tests/test_load.py`

- [ ] **Step 1: Add the failing tests** to `tests/test_load.py`

```python
from collections import defaultdict
from simviz.load import smooth_rate, detect_regimes


def test_smooth_rate_trailing_average():
    subs = defaultdict(int, {0: 4, 1: 0, 2: 2})
    # window 2: slot0 -> 4/1, slot1 -> (4+0)/2, slot2 -> (0+2)/2
    assert smooth_rate(subs, slot_count=3, window=2) == [4.0, 2.0, 1.0]


def test_detect_regimes_step_change():
    rate = [2.0] * 5 + [40.0] * 5
    regimes = detect_regimes(rate, change_pct=0.10)
    assert [(r["start"], r["end"]) for r in regimes] == [(0, 5), (5, 10)]
    assert regimes[0]["meanArrival"] == 2.0
    assert regimes[1]["meanArrival"] == 40.0


def test_detect_regimes_single_when_flat():
    regimes = detect_regimes([2.0, 2.0, 2.0], change_pct=0.10)
    assert [(r["start"], r["end"]) for r in regimes] == [(0, 3)]


def test_detect_regimes_empty():
    assert detect_regimes([], change_pct=0.10) == []
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest tests/test_load.py -q`
Expected: FAIL — `ImportError: cannot import name 'smooth_rate'`.

- [ ] **Step 3: Append to `simviz/load.py`**

```python
def smooth_rate(submissions_per_slot, slot_count, window):
    """Trailing moving average of submissions/slot over `window` slots."""
    window = max(1, window)
    rate = []
    run = 0
    for s in range(slot_count):
        run += submissions_per_slot.get(s, 0)
        if s >= window:
            run -= submissions_per_slot.get(s - window, 0)
        denom = min(s + 1, window)
        rate.append(run / denom)
    return rate


def _material_change(change_pct, old_rate, new_rate):
    """Mirror materialLoadChange."""
    if old_rate == new_rate:
        return False
    if old_rate <= 0:
        return new_rate > 0
    return abs(new_rate - old_rate) / old_rate > max(0.0, change_pct)


def detect_regimes(rate_series, change_pct):
    """Segment a (smoothed) rate series into load regimes.

    Mirrors loadRegimes' material-change logic, but compares each slot's rate against
    the rate at the START of the current regime (not the immediately previous slot) so
    that Poisson slot-to-slot noise in the observed series doesn't over-segment.
    """
    n = len(rate_series)
    if n == 0:
        return []
    regimes = []
    start = 0
    base = rate_series[0]
    for s in range(1, n):
        if _material_change(change_pct, base, rate_series[s]):
            regimes.append({"start": start, "end": s,
                            "meanArrival": mean(rate_series[start:s])})
            start = s
            base = rate_series[s]
    regimes.append({"start": start, "end": n,
                    "meanArrival": mean(rate_series[start:n])})
    return regimes
```

- [ ] **Step 4: Run to verify pass**

Run: `python -m pytest tests/test_load.py -q`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add abstract-sim-viz/simviz/load.py abstract-sim-viz/tests/test_load.py
git commit -m "feat(viz): smooth_rate and load-regime detection"
```

---

## Task 10: `latency.py` — class_id & join_latencies

**Files:**
- Create: `abstract-sim-viz/simviz/latency.py`
- Test: `abstract-sim-viz/tests/test_latency.py`

- [ ] **Step 1: Write the failing tests** in `tests/test_latency.py`

```python
from simviz.ingest import Accumulator
from simviz.latency import class_id, join_latencies


def _submitted(tx_id, slot, lane, rate, tag="Exponential"):
    return {"tag": "TxSubmitted", "slot": slot, "actorId": 0,
            "tx": {"id": tx_id, "lane": lane, "submitted": slot, "value": 1,
                   "urgency": {"tag": tag, "rate": rate},
                   "body": {"sizeBytes": 1, "script": {"sizeBytes": 0, "exUnits": 0},
                            "dependsOn": [], "fee": 1}}}


def _included(tx_id, slot):
    return {"tag": "TxIncluded", "slot": slot, "txId": tx_id,
            "inclusionPoint": {"tag": "IncludedInRb"}}


def test_class_id():
    assert class_id("Exponential", 5.0e-4) == "Exponential:0.0005"


def test_join_latencies_groups_by_class_and_skips_unincluded():
    acc = Accumulator()
    for e in [
        _submitted(1, 0, "Standard", 5.0e-4), _included(1, 2),   # latency 2
        _submitted(2, 0, "Priority", 6.0e-3), _included(2, 3),   # latency 3
        _submitted(3, 4, "Standard", 5.0e-4), _included(3, 5),   # latency 1
        _submitted(4, 1, "Standard", 5.0e-4),                    # never included -> skipped
    ]:
        acc.ingest(e)
    grouped = join_latencies(acc)
    assert sorted(grouped["Exponential:0.0005"]) == [(0, 2), (4, 1)]   # tx1, tx3 (tx4 never included)
    assert grouped["Exponential:0.006"] == [(0, 3)]
    assert len(grouped) == 2   # only 2 classes present; tx4 (never included) excluded
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest tests/test_latency.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'simviz.latency'`.

- [ ] **Step 3: Create `simviz/latency.py`** (class_id + join)

```python
def class_id(tag, rate):
    """Stable id for an urgency class, e.g. 'Exponential:0.0005'."""
    return f"{tag}:{rate}"


def join_latencies(acc):
    """Map class_id -> list of (submit_slot, latency_slots) for txs with both events."""
    out = {}
    for tx_id, submit_slot in acc.submitted_at.items():
        inc = acc.included_at.get(tx_id)
        meta = acc.tx_meta.get(tx_id)
        if inc is None or meta is None:
            continue
        cid = class_id(meta["tag"], meta["rate"])
        out.setdefault(cid, []).append((submit_slot, inc - submit_slot))
    return out
```

- [ ] **Step 4: Run to verify pass**

Run: `python -m pytest tests/test_latency.py -q`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add abstract-sim-viz/simviz/latency.py abstract-sim-viz/tests/test_latency.py
git commit -m "feat(viz): latency join grouped by urgency class"
```

---

## Task 11: `latency.py` — class_stats & over_time

**Files:**
- Modify: `abstract-sim-viz/simviz/latency.py`
- Test: `abstract-sim-viz/tests/test_latency.py`

- [ ] **Step 1: Add the failing tests** to `tests/test_latency.py`

```python
from simviz.latency import class_stats, over_time


def test_class_stats():
    stats = class_stats([1, 2])
    assert stats["count"] == 2
    assert stats["mean"] == 1.5
    assert stats["median"] == 1          # quantile(0.5, [1,2]) -> idx 0
    assert stats["p95"] == 2
    assert stats["max"] == 2
    empty = class_stats([])
    assert empty["count"] == 0 and empty["max"] == 0


def test_over_time_buckets_by_submit_slot():
    pairs = [(0, 5), (1, 7), (4, 1), (5, 3)]   # (submit_slot, latency)
    out = over_time(pairs, width=2, slot_count=6)
    assert out == [
        {"slot": 0, "median": 5, "p95": 7, "n": 2},   # bucket 0: latencies [5,7]
        {"slot": 4, "median": 1, "p95": 3, "n": 2},   # bucket 4: latencies [1,3]
    ]
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest tests/test_latency.py -q`
Expected: FAIL — `ImportError: cannot import name 'class_stats'`.

- [ ] **Step 3: Append to `simviz/latency.py`**

```python
from simviz.stats import quantile, mean


def class_stats(latencies):
    xs = sorted(latencies)
    n = len(xs)
    if n == 0:
        return {"count": 0, "mean": 0.0, "median": 0,
                "p25": 0, "p75": 0, "p95": 0, "max": 0}
    return {
        "count": n,
        "mean": mean(xs),
        "median": quantile(0.50, xs),
        "p25": quantile(0.25, xs),
        "p75": quantile(0.75, xs),
        "p95": quantile(0.95, xs),
        "max": xs[-1],
    }


def over_time(pairs, width, slot_count):
    """Bucket (submit_slot, latency) pairs by submit slot; per bucket emit median/p95/n."""
    buckets = {}
    for submit_slot, lat in pairs:
        key = (submit_slot // width) * width
        buckets.setdefault(key, []).append(lat)
    out = []
    for start in sorted(buckets):
        xs = sorted(buckets[start])
        out.append({"slot": start, "median": quantile(0.5, xs),
                    "p95": quantile(0.95, xs), "n": len(xs)})
    return out
```

- [ ] **Step 4: Run to verify pass**

Run: `python -m pytest tests/test_latency.py -q`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add abstract-sim-viz/simviz/latency.py abstract-sim-viz/tests/test_latency.py
git commit -m "feat(viz): per-class latency stats and over-time bucketing"
```

---

## Task 12: `contract.py` — urgency_classes

**Files:**
- Create: `abstract-sim-viz/simviz/contract.py`
- Test: `abstract-sim-viz/tests/test_contract.py`

- [ ] **Step 1: Write the failing test** in `tests/test_contract.py`

```python
from simviz.ingest import Accumulator
from simviz.contract import urgency_classes


def _submitted(tx_id, rate, tag="Exponential"):
    return {"tag": "TxSubmitted", "slot": 0, "actorId": 0,
            "tx": {"id": tx_id, "lane": "Standard", "submitted": 0, "value": 1,
                   "urgency": {"tag": tag, "rate": rate},
                   "body": {"sizeBytes": 1, "script": {"sizeBytes": 0, "exUnits": 0},
                            "dependsOn": [], "fee": 1}}}


def test_urgency_classes_ordered_low_to_high_rate():
    acc = Accumulator()
    acc.ingest(_submitted(1, 6.0e-3))
    acc.ingest(_submitted(2, 5.0e-4))
    acc.ingest(_submitted(3, 5.0e-4))   # duplicate class
    classes = urgency_classes(acc)
    assert [c["rate"] for c in classes] == [5.0e-4, 6.0e-3]
    assert classes[0]["id"] == "Exponential:0.0005"
    assert classes[0]["label"] == "Exp λ=0.0005"
    assert classes[0]["tag"] == "Exponential"
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest tests/test_contract.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'simviz.contract'`.

- [ ] **Step 3: Create `simviz/contract.py`** (urgency_classes + helpers)

```python
import json
import math
from datetime import datetime, timezone

from simviz import price as price_mod
from simviz import load as load_mod
from simviz import latency as latency_mod
from simviz.stats import quantile, histogram_bins

DEFAULT_PARAMS = {"shockThreshold": 0.10, "convergenceBandPct": 0.05, "loadChangePct": 0.10}


def _format_rate(rate):
    return f"{rate:g}"


def urgency_label(tag, rate):
    short = {"Exponential": "Exp", "Linear": "Lin"}.get(tag, tag)
    return f"{short} λ={_format_rate(rate)}"


def urgency_classes(acc):
    """Distinct urgency classes present, ordered by rate low -> high."""
    keys = {(m["tag"], m["rate"]) for m in acc.tx_meta.values()}
    classes = []
    for tag, rate in sorted(keys, key=lambda k: k[1]):
        classes.append({
            "id": latency_mod.class_id(tag, rate),
            "tag": tag, "rate": rate,
            "label": urgency_label(tag, rate),
        })
    return classes
```

- [ ] **Step 4: Run to verify pass**

Run: `python -m pytest tests/test_contract.py -q`
Expected: PASS (1 test).

- [ ] **Step 5: Commit**

```bash
git add abstract-sim-viz/simviz/contract.py abstract-sim-viz/tests/test_contract.py
git commit -m "feat(viz): ordered urgency-class metadata"
```

---

## Task 13: `contract.py` — build_sim_data

**Files:**
- Modify: `abstract-sim-viz/simviz/contract.py`
- Test: `abstract-sim-viz/tests/test_contract.py`

- [ ] **Step 1: Add the failing test** to `tests/test_contract.py`

```python
from simviz.contract import build_sim_data


def _price(lane, slot, old, new, util=0.0):
    return {"tag": "PriceUpdated", "slot": slot, "lane": lane,
            "oldCoeff": old, "newCoeff": new, "utilisation": util}


def _included(tx_id, slot):
    return {"tag": "TxIncluded", "slot": slot, "txId": tx_id,
            "inclusionPoint": {"tag": "IncludedInRb"}}


def test_build_sim_data_structure_and_values():
    acc = Accumulator()
    for e in [
        _submitted(1, 5.0e-4), _included(1, 2),
        _price("Priority", 1, 16, 10, 0.4),     # jump 0.375 -> shock
        _price("Standard", 1, 1, 1, 0.2),
    ]:
        acc.ingest(e)
    data = build_sim_data(acc, source="trace.jsonl")

    # meta
    assert data["meta"]["slotCount"] == 3       # max slot 2 + 1
    assert data["meta"]["totalEvents"] == 4
    assert data["meta"]["lanes"] == ["Standard", "Priority"]
    assert len(data["meta"]["urgencyClasses"]) == 1
    assert "generatedAt" in data["meta"]

    # params
    assert data["params"]["shockThreshold"] == 0.10

    # price + shock
    assert data["price"]["byLane"]["Priority"][0]["jump"] == 0.375
    assert data["shock"]["byLane"]["Priority"]["shockCount"] == 1
    assert data["shock"]["byLane"]["Standard"]["shockCount"] == 0

    # convergence + load + latency keys present
    assert "loadRegimes" in data["convergence"]
    assert "Priority" in data["convergence"]["byLane"]
    assert data["load"]["bucketWidth"] >= 1
    cls_id = data["meta"]["urgencyClasses"][0]["id"]
    assert data["latency"]["byClass"][cls_id]["count"] == 1
    assert data["latency"]["byClass"][cls_id]["max"] == 2
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest tests/test_contract.py::test_build_sim_data_structure_and_values -q`
Expected: FAIL — `ImportError: cannot import name 'build_sim_data'`.

- [ ] **Step 3: Append to `simviz/contract.py`**

```python
def _shared_bin_width(all_latencies):
    if not all_latencies:
        return 1
    p99 = quantile(0.99, sorted(all_latencies))
    return max(1, math.ceil(p99 / 30))


def build_sim_data(acc, params=None, target_buckets=300, source="events.jsonl"):
    params = {**DEFAULT_PARAMS, **(params or {})}
    slot_count = acc.slot_count
    width = load_mod.bucket_width(slot_count, target_buckets)

    present = set(acc.price_changes.keys())
    lanes = [l for l in ["Standard", "Priority"] if l in present] or sorted(present)
    classes = urgency_classes(acc)

    price_by_lane = {lane: price_mod.price_series(acc, lane) for lane in lanes}
    shock_by_lane = {
        lane: price_mod.shock_stats(price_by_lane[lane], params["shockThreshold"])
        for lane in lanes
    }

    rate = load_mod.smooth_rate(acc.submissions_per_slot, slot_count, width)
    regimes = load_mod.detect_regimes(rate, params["loadChangePct"])
    load_obj = {
        "bucketWidth": width,
        "buckets": load_mod.load_buckets(
            acc.submissions_per_slot, acc.inclusions_per_slot, slot_count, width),
    }

    conv_by_lane = {}
    for lane in lanes:
        regime_results, conv_time = price_mod.convergence_for_lane(
            price_by_lane[lane], regimes, params["convergenceBandPct"])
        conv_by_lane[lane] = {
            "convergenceTime": conv_time,
            "oscillationAmplitude": price_mod.oscillation_amplitude(price_by_lane[lane]),
            "regimes": regime_results,
        }

    grouped = latency_mod.join_latencies(acc)
    all_lat = [lat for pairs in grouped.values() for (_, lat) in pairs]
    bin_w = _shared_bin_width(all_lat)
    latency_by_class = {}
    for cls in classes:
        cid = cls["id"]
        pairs = grouped.get(cid, [])
        lats = [lat for (_, lat) in pairs]
        stats = latency_mod.class_stats(lats)
        stats["histogram"] = {"binWidth": bin_w, "bins": histogram_bins(lats, bin_w)}
        stats["overTime"] = latency_mod.over_time(pairs, width, slot_count)
        latency_by_class[cid] = stats

    return {
        "meta": {
            "source": source,
            "generatedAt": datetime.now(timezone.utc).isoformat(),
            "slotCount": slot_count,
            "totalEvents": acc.total_events,
            "lanes": lanes,
            "urgencyClasses": classes,
        },
        "params": params,
        "price": {"byLane": price_by_lane},
        "shock": {"byLane": shock_by_lane},
        "convergence": {"loadRegimes": regimes, "byLane": conv_by_lane},
        "latency": {"byClass": latency_by_class},
        "load": load_obj,
    }
```

- [ ] **Step 4: Run to verify pass**

Run: `python -m pytest tests/test_contract.py -q`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add abstract-sim-viz/simviz/contract.py abstract-sim-viz/tests/test_contract.py
git commit -m "feat(viz): assemble full SIM_DATA contract object"
```

---

## Task 14: `contract.py` — write_data_js

**Files:**
- Modify: `abstract-sim-viz/simviz/contract.py`
- Test: `abstract-sim-viz/tests/test_contract.py`

- [ ] **Step 1: Add the failing test** to `tests/test_contract.py`

```python
import json
from simviz.contract import write_data_js


def test_write_data_js_roundtrip(tmp_path):
    sim_data = {"meta": {"slotCount": 3}, "price": {"byLane": {}}}
    out = tmp_path / "data.js"
    write_data_js(sim_data, str(out))
    text = out.read_text()
    assert text.startswith("window.SIM_DATA = ")
    assert text.rstrip().endswith(";")
    payload = text[len("window.SIM_DATA = "):].rstrip().rstrip(";")
    assert json.loads(payload) == sim_data
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest tests/test_contract.py::test_write_data_js_roundtrip -q`
Expected: FAIL — `ImportError: cannot import name 'write_data_js'`.

- [ ] **Step 3: Append to `simviz/contract.py`**

```python
def write_data_js(sim_data, path):
    """Serialise SIM_DATA as a JS global so the dashboard works from file://."""
    payload = json.dumps(sim_data, separators=(",", ":"))
    with open(path, "w") as fh:
        fh.write("window.SIM_DATA = " + payload + ";\n")
```

- [ ] **Step 4: Run to verify pass**

Run: `python -m pytest tests/test_contract.py -q`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add abstract-sim-viz/simviz/contract.py abstract-sim-viz/tests/test_contract.py
git commit -m "feat(viz): write_data_js serialiser"
```

---

## Task 15: `preprocess.py` — CLI

**Files:**
- Create: `abstract-sim-viz/preprocess.py`
- Test: `abstract-sim-viz/tests/test_preprocess_integration.py`

- [ ] **Step 1: Write the failing test** in `tests/test_preprocess_integration.py`

```python
import json
from preprocess import main


def _line(obj, n):
    return json.dumps({"event": obj, "eventNo": n})


def test_cli_writes_data_js(tmp_path):
    trace = tmp_path / "events.jsonl"
    trace.write_text("\n".join([
        _line({"tag": "TxSubmitted", "slot": 0, "actorId": 0,
               "tx": {"id": 1, "lane": "Standard", "submitted": 0, "value": 1,
                      "urgency": {"tag": "Exponential", "rate": 5.0e-4},
                      "body": {"sizeBytes": 1, "script": {"sizeBytes": 0, "exUnits": 0},
                               "dependsOn": [], "fee": 1}}}, 0),
        _line({"tag": "TxIncluded", "slot": 2, "txId": 1,
               "inclusionPoint": {"tag": "IncludedInRb"}}, 1),
    ]) + "\n")
    out = tmp_path / "data.js"
    main([str(trace), "-o", str(out)])
    text = out.read_text()
    assert text.startswith("window.SIM_DATA = ")
    payload = json.loads(text[len("window.SIM_DATA = "):].rstrip().rstrip(";"))
    assert payload["meta"]["slotCount"] == 3
    assert payload["meta"]["totalEvents"] == 2
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest tests/test_preprocess_integration.py -q`
Expected: FAIL — `ModuleNotFoundError: No module named 'preprocess'`.

- [ ] **Step 3: Create `abstract-sim-viz/preprocess.py`**

```python
#!/usr/bin/env python3
"""Distil an abstract-sim-hs events.jsonl trace into dashboard/data.js."""
import argparse
import os

from simviz.ingest import iter_events, Accumulator
from simviz.contract import build_sim_data, write_data_js


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("events", help="path to events.jsonl")
    parser.add_argument("-o", "--output", default=os.path.join("dashboard", "data.js"))
    parser.add_argument("--shock-threshold", type=float, default=0.10)
    parser.add_argument("--band-pct", type=float, default=0.05)
    parser.add_argument("--load-change-pct", type=float, default=0.10)
    parser.add_argument("--target-buckets", type=int, default=300)
    args = parser.parse_args(argv)

    acc = Accumulator()
    for event in iter_events(args.events):
        acc.ingest(event)

    sim_data = build_sim_data(
        acc,
        params={
            "shockThreshold": args.shock_threshold,
            "convergenceBandPct": args.band_pct,
            "loadChangePct": args.load_change_pct,
        },
        target_buckets=args.target_buckets,
        source=os.path.basename(args.events),
    )
    out_dir = os.path.dirname(args.output)
    if out_dir:
        os.makedirs(out_dir, exist_ok=True)
    write_data_js(sim_data, args.output)
    m = sim_data["meta"]
    print(f"Wrote {args.output}: {m['slotCount']} slots, {m['totalEvents']} events, "
          f"{len(m['urgencyClasses'])} urgency classes.")


if __name__ == "__main__":
    main()
```

- [ ] **Step 4: Run to verify pass**

Run: `python -m pytest tests/test_preprocess_integration.py -q`
Expected: PASS (1 test).

- [ ] **Step 5: Commit**

```bash
git add abstract-sim-viz/preprocess.py abstract-sim-viz/tests/test_preprocess_integration.py
git commit -m "feat(viz): preprocess.py CLI"
```

---

## Task 16: Integration fixture & full assertions

**Files:**
- Create: `abstract-sim-viz/tests/fixtures/tiny.jsonl`
- Modify: `abstract-sim-viz/tests/test_preprocess_integration.py`

- [ ] **Step 1: Create `tests/fixtures/tiny.jsonl`** (hand-built trace with known outputs)

```jsonl
{"event":{"actorId":0,"slot":0,"tag":"TxSubmitted","tx":{"body":{"dependsOn":[],"fee":100,"script":{"exUnits":0,"sizeBytes":0},"sizeBytes":300},"id":1,"lane":"Standard","submitted":0,"urgency":{"rate":5.0e-4,"tag":"Exponential"},"value":1000}},"eventNo":0}
{"event":{"actorId":0,"slot":0,"tag":"TxSubmitted","tx":{"body":{"dependsOn":[],"fee":200,"script":{"exUnits":0,"sizeBytes":0},"sizeBytes":400},"id":2,"lane":"Priority","submitted":0,"urgency":{"rate":6.0e-3,"tag":"Exponential"},"value":2000}},"eventNo":1}
{"event":{"lane":"Standard","newCoeff":1,"oldCoeff":1,"slot":1,"tag":"PriceUpdated","utilisation":0.2},"eventNo":2}
{"event":{"lane":"Priority","newCoeff":10,"oldCoeff":16,"slot":1,"tag":"PriceUpdated","utilisation":0.4},"eventNo":3}
{"event":{"inclusionPoint":{"tag":"IncludedInRb"},"slot":2,"tag":"TxIncluded","txId":1},"eventNo":4}
{"event":{"lane":"Priority","newCoeff":10.5,"oldCoeff":10,"slot":2,"tag":"PriceUpdated","utilisation":0.5},"eventNo":5}
{"event":{"inclusionPoint":{"tag":"IncludedInRb"},"slot":3,"tag":"TxIncluded","txId":2},"eventNo":6}
{"event":{"actorId":1,"slot":4,"tag":"TxSubmitted","tx":{"body":{"dependsOn":[],"fee":100,"script":{"exUnits":0,"sizeBytes":0},"sizeBytes":300},"id":3,"lane":"Standard","submitted":4,"urgency":{"rate":5.0e-4,"tag":"Exponential"},"value":1000}},"eventNo":7}
{"event":{"inclusionPoint":{"tag":"IncludedInRb"},"slot":5,"tag":"TxIncluded","txId":3},"eventNo":8}
```

Known outputs: slotCount = 6 (max slot 5 + 1); totalEvents = 9; lanes = ["Standard","Priority"]; 2 urgency classes (`Exponential:0.0005`, `Exponential:0.006`); Priority price series jumps = [0.375, 0.05] → maxJump 0.375, shockCount 1; Standard shockCount 0; class `Exponential:0.0005` latencies = [2 (id1), 1 (id3)] → count 2, median 1, max 2; class `Exponential:0.006` latency [3] → count 1, median 3.

- [ ] **Step 2: Add the failing integration test** to `tests/test_preprocess_integration.py`

```python
import os
from preprocess import main as run_main


FIXTURE = os.path.join(os.path.dirname(__file__), "fixtures", "tiny.jsonl")


def test_tiny_fixture_end_to_end(tmp_path):
    import json
    out = tmp_path / "data.js"
    run_main([FIXTURE, "-o", str(out)])
    data = json.loads(out.read_text()[len("window.SIM_DATA = "):].rstrip().rstrip(";"))

    assert data["meta"]["slotCount"] == 6
    assert data["meta"]["totalEvents"] == 9
    assert data["meta"]["lanes"] == ["Standard", "Priority"]
    assert [c["id"] for c in data["meta"]["urgencyClasses"]] == \
        ["Exponential:0.0005", "Exponential:0.006"]

    assert data["shock"]["byLane"]["Priority"]["maxJump"] == 0.375
    assert data["shock"]["byLane"]["Priority"]["shockCount"] == 1
    assert data["shock"]["byLane"]["Standard"]["shockCount"] == 0

    lat = data["latency"]["byClass"]
    assert lat["Exponential:0.0005"]["count"] == 2
    assert lat["Exponential:0.0005"]["median"] == 1
    assert lat["Exponential:0.0005"]["max"] == 2
    assert lat["Exponential:0.006"]["count"] == 1
    assert lat["Exponential:0.006"]["median"] == 3

    assert len(data["convergence"]["loadRegimes"]) >= 1   # regimes present (noisy at width=1)
```

- [ ] **Step 3: Run to verify it passes** (implementation already exists)

Run: `python -m pytest tests/test_preprocess_integration.py -q`
Expected: PASS (2 tests). If any assertion fails, fix the relevant module — do not change the asserted expected values, which are hand-computed.

- [ ] **Step 4: Run the whole suite**

Run: `python -m pytest -q`
Expected: PASS (all tests across all modules).

- [ ] **Step 5: Commit**

```bash
git add abstract-sim-viz/tests/fixtures/tiny.jsonl abstract-sim-viz/tests/test_preprocess_integration.py
git commit -m "test(viz): end-to-end fixture with hand-computed expectations"
```

---

## Task 17: Smoke-run against the real trace

**Files:** none (verification only).

- [ ] **Step 1: Run the preprocessor on the real trace**

Run (from `abstract-sim-viz/`):
`python preprocess.py ../abstract-sim-hs/events.jsonl -o /tmp/data.js`
Expected: prints e.g. `Wrote /tmp/data.js: 2000 slots, 719469 events, 4 urgency classes.` and completes in a few seconds without loading errors.

- [ ] **Step 2: Sanity-check the output size and shape**

Run: `head -c 300 /tmp/data.js`
Expected: starts with `window.SIM_DATA = {"meta":{"source":"events.jsonl","generatedAt":...`. File should be small (tens–hundreds of KB), confirming the 121 MB trace was distilled correctly.

- [ ] **Step 3: No commit** (this is a verification step; `/tmp/data.js` is throwaway).

---

## Self-review notes (completed by plan author)

- **Spec coverage:** price series/shock (Tasks 6, 13) → §5 price/shock; convergence + oscillation (Task 7, 13) → §5 convergence, §8; latency join/stats/over-time/histogram (Tasks 10–11, 13) → §5 latency; load buckets + regimes (Tasks 8–9, 13) → §5 load, §8 assumption; streaming + last-wins (Tasks 4–5) → §3, §6, §8; contract assembly + data.js (Tasks 12–14) → §3, §5; CLI + run length (Task 15, 17) → §4, §8; tests incl. duplicate-txId, percentile rule, regime detection (throughout) → §9. All §5/§6/§8 items map to a task.
- **Placeholder scan:** no TBD/TODO; every code step shows complete code.
- **Type/name consistency:** `class_id` defined in `latency.py` (Task 10), reused in `contract.urgency_classes` (Task 12); `Accumulator` attributes set in Task 5 and consumed unchanged by Tasks 6–13; `build_sim_data`/`write_data_js` signatures consistent between Tasks 13–15 and the CLI.

---

## Execution handoff

This is **Plan 1 of 2**. Plan 2 (`2026-06-04-abstract-sim-viz-dashboard.md`) builds the Observable Plot dashboard that consumes `data.js`. Execute this plan first; the dashboard plan depends only on the data contract it produces.
