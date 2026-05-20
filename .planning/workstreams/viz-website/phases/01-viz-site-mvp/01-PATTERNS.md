# Phase 1: Viz Site MVP - Pattern Map

**Mapped:** 2026-05-20
**Workstream:** viz-website
**Files analyzed:** 8 new (Python build pipeline + static web bundle + tests + README) / 0 modified
**Analogs found:** 7 / 8 (the browser ES-module SPA shell has no in-repo Python or Rust analog; the planner must lean on RESEARCH.md `## Code Examples` for that single file)

## File Classification

| New File | Role | Data Flow | Closest Analog | Match Quality |
|----------|------|-----------|----------------|---------------|
| `sim-rs/scripts/viz/build.py` | CLI entry-point + orchestrator | batch (file-walk + transform + emit) | `sim-rs/scripts/analyse-phase-3.py` (consumes phase-2 artefacts) + `sim-rs/scripts/generate-realistic-100-topology.py` (argparse style) | exact (combined) |
| `sim-rs/scripts/viz/build.py` :: `discover_suites()` | utility / file-walk | filesystem-scan | `analyse-phase-3.py` :: `collect_job()` + main `Path("output/phase-3")` walk | role-match |
| `sim-rs/scripts/viz/build.py` :: `load_seed()` / `_read_time_series_long()` | ingest / transform | CSV/JSON → JSON | `analyse-phase-3.py` :: `load_run_summary()` (lines 111-122) | exact |
| `sim-rs/scripts/viz/build.py` :: `serve()` | service-wrapper | request-response (HTTP) | No analog — stdlib `http.server.ThreadingHTTPServer` boilerplate; see RESEARCH.md Pitfall 7 excerpt | no analog (stdlib boilerplate) |
| `sim-rs/scripts/viz/static/index.html` | static asset / SPA shell | static | `sim-rs/output/tiered_plot.html` (chart taxonomy intuition ONLY — do not import code per CONTEXT.md `<canonical_refs>`) | reference-only |
| `sim-rs/scripts/viz/static/main.js` | browser router + renderer | event-driven (hashchange) | None in repo (greenfield JS) — use RESEARCH.md `## Code Examples` patterns 2/3 directly | no analog |
| `sim-rs/scripts/viz/static/style.css` | static asset | static | None | no analog (minimal, D-23) |
| `sim-rs/scripts/viz/static/plot.min.js` | vendored library | static | None | no analog (vendored per RESEARCH.md Open Q #3 / A3) |
| `sim-rs/scripts/viz/tests/test_build_smoke.py` | test (unit) | request-response (assert) | None in `sim-rs/scripts/` — but `stdlib unittest` is the framework (RESEARCH.md `## Validation Architecture`) | no analog (stdlib boilerplate) |
| `sim-rs/scripts/viz/tests/test_serve_smoke.py` | test (integration) | subprocess + HTTP | None | no analog (stdlib boilerplate) |
| `sim-rs/scripts/viz/tests/fixtures/` | test data | static | None | no analog |
| `sim-rs/scripts/viz/README.md` | docs | static | `sim-rs/scripts/generate-realistic-100-topology.py` module docstring (lines 1-45) | role-match (docstring → README) |

**File NOT being created (landmine — see Common Pitfalls):**

| File | Why Skipped |
|------|-------------|
| `sim-rs/.gitignore` edit | RESEARCH.md Pitfall 6 / CLAUDE.md "`/output` is gitignored": `sim-rs/.gitignore` line 2 is `/output`, which catches everything under `sim-rs/output/` including `viz/`. D-06's "must be added" wording is satisfied transitively. **Do not add `/output/viz/`, `output/viz/`, or `viz/` lines to either gitignore.** Document the existing rule in `sim-rs/scripts/viz/README.md` instead. |

---

## Pattern Assignments

### `sim-rs/scripts/viz/build.py` (CLI entry-point, batch)

**Analog #1:** `sim-rs/scripts/generate-realistic-100-topology.py` — style baseline for shebang + module docstring + argparse `main()`.

**Analog #2:** `sim-rs/scripts/analyse-phase-3.py` — closer functional analog (consumes `sim-rs/output/<root>/<suite>/<job>/<seed>/run_summary.json`).

#### Pattern A — Shebang + module-docstring + stdlib-only imports

**Source:** `sim-rs/scripts/generate-realistic-100-topology.py` lines 1-57

```python
#!/usr/bin/env python3
"""
Phase-2/3 mainnet-faithful topology generator.

Two modes, selected at the command line:

  python3 scripts/generate-realistic-100-topology.py
      → emit parameters/phase-2-sweep/topology-realistic-150.yaml
        ...

The default path (150-node emission) is deterministic and reproducible
from in-tree data alone — ...
"""

import argparse
import copy
import json
import random
import sys
import urllib.request
from bisect import bisect_left
from datetime import date
from pathlib import Path

import yaml  # only third-party dep used across scripts/
```

**Copy verbatim style points:**
- `#!/usr/bin/env python3` shebang.
- Triple-quoted docstring opens with a one-line summary, then a usage block showing literal commands (`python3 sim-rs/scripts/viz/build.py --serve`), then a design/contract section.
- Imports split into two blocks: stdlib first (sorted), then third-party (sorted). `pathlib.Path` imported `from pathlib import Path`, never `import pathlib`.
- **For the viz build script: no third-party imports.** D-08 forbids non-stdlib deps; PyYAML is not needed because every file the viz reads is JSON or CSV.

#### Pattern B — `argparse` with `--flag`-style options + `Path` types

**Source:** `sim-rs/scripts/generate-realistic-100-topology.py` lines 356-401

```python
def main() -> None:
    parser = argparse.ArgumentParser(
        description="Mainnet-faithful topology generator (100 or 150 nodes)."
    )
    parser.add_argument(
        "--regenerate-100",
        action="store_true",
        help="Hit Koios live and re-emit topology-realistic-100.yaml to stdout. ...",
    )
    parser.add_argument(
        "--base",
        type=Path,
        default=Path("parameters/phase-2-sweep/topology-realistic-100.yaml"),
        help="Path to the committed 100-node topology YAML (input to 150-node generation).",
    )
    parser.add_argument(
        "--jitter-seed",
        type=int,
        default=582,
        help="Master RNG seed for the 50 extras (default: 582 = snapshot epoch).",
    )
    args = parser.parse_args()
    ...
    print(f"Wrote {args.out}", file=sys.stderr)


if __name__ == "__main__":
    main()
```

**Copy verbatim style points:**
- `type=Path` on every path argument; default is `Path("…")`.
- Flag names use kebab-case (`--regenerate-100`, `--jitter-seed`), exposed as snake_case attributes (`args.regenerate_100`, `args.jitter_seed`) by argparse.
- Diagnostic output goes to `sys.stderr` via `file=sys.stderr`; only the bundle's actual data outputs reach disk.
- The `if __name__ == "__main__": main()` guard is identical across all three existing scripts.

**For the viz build script:** mirror the flag set from RESEARCH.md `## Architecture Patterns / Pattern 1`:

| Flag | Type | Default | Source |
|------|------|---------|--------|
| `--source` | `Path` | `Path("sim-rs/output")` | D-10 |
| `--output` | `Path` | `Path("sim-rs/output/viz")` | D-06 |
| `--include` | `action="append"` | `[]` | D-10 |
| `--exclude` | `action="append"` | `[]` | D-10 |
| `--serve` | `action="store_true"` | `False` | D-07 |
| `--port` | `int` | `8000` | RESEARCH.md Pattern 1 |

#### Pattern C — `Path.rglob` for recursive walk + `json.load` with try/except skip-and-warn

**Source:** `sim-rs/scripts/analyse-phase-3.py` lines 111-122 (load shape) + RESEARCH.md `## Code Examples` (skip-and-warn wrapper)

```python
def load_run_summary(p: Path):
    with open(p) as f:
        d = json.load(f)
    rv = d["priority_retained_value_total"] + d["standard_retained_value_total"]
    return {
        "retained_value": rv,
        "priority_retained_value_total": d["priority_retained_value_total"],
        "standard_retained_value_total": d["standard_retained_value_total"],
        "net_utility_total": d.get("net_utility_total"),
        "retained_value_ratio": d.get("retained_value_ratio"),
        "pricing_event_stream_sha256": d["pricing_event_stream_sha256"],
    }


def collect_job(suite_dir: Path, job_name: str, seeds):
    """Return dict: seed -> run_summary fields, in seed order. Missing seeds raise."""
    out = {}
    for s in seeds:
        p = suite_dir / job_name / str(s) / "run_summary.json"
        if not p.exists():
            return None
        out[s] = load_run_summary(p)
    return out
```

**Copy verbatim style points:**
- `with open(p) as f: d = json.load(f)` — never `json.loads(open(p).read())`.
- `.get(key, default)` for **optional** fields (`net_utility_total`, `retained_value_ratio`); square-bracket `d["key"]` for **required** fields (`priority_retained_value_total`).
- Path composition uses `/` operator on `Path`, never `os.path.join`.
- Existence check via `p.exists()`, not `try/except FileNotFoundError`.

**Divergence for build.py (per D-21 skip-and-warn vs analyse-phase-3.py's None-return):**

The phase-3 analyser returns `None` from `collect_job` when a seed is missing and the caller emits a "data-missing" verdict. For the viz build, accumulate warnings into a shared list and continue (RESEARCH.md `## Architecture Patterns / Pattern 4`):

```python
def discover_suites(source: Path, warnings: list):
    suites = []
    for manifest_path in sorted(source.rglob("manifest.json")):
        try:
            with open(manifest_path) as f:
                manifest = json.load(f)
        except (json.JSONDecodeError, OSError) as e:
            warnings.append(f"skip {manifest_path}: {e}")
            continue
        suite_id = str(manifest_path.parent.relative_to(source)).replace("/", "__")
        suites.append({...})
    return suites
```

#### Pattern D — Casing-disciplined dict access (LANDMINE)

**Source:** `sim-rs/sim-cli/src/runner.rs` lines 44-74 + `sim-rs/sim-cli/src/metrics/collector.rs` lines 11-160

```rust
// runner.rs — kebab-case on disk
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct JobEntry {
    pub status: JobStatus,
    pub started_at_utc: Option<DateTime<Utc>>,
    pub completed_at_utc: Option<DateTime<Utc>>,
    pub output_path: Option<PathBuf>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Manifest {
    pub suite_name: String,
    pub started_at_utc: DateTime<Utc>,
    pub jobs: BTreeMap<String, BTreeMap<String, JobEntry>>,
}

// collector.rs — snake_case on disk (no rename_all attribute)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunSummary {
    pub components: Vec<ComponentSummary>,
    pub total_txs_submitted: u64,
    pub total_txs_included: u64,
    pub priority_retained_value_total: f64,
    pub standard_retained_value_total: f64,
    ...
    pub pricing_event_stream_sha256: String,
}
```

**Pattern to copy into build.py:** different access conventions for the two file shapes, explicit per-file constants so the casing never has to be remembered.

```python
# In manifest.json — kebab-case keys
manifest["suite-name"]            # NOT manifest["suite_name"]
manifest["started-at-utc"]        # NOT manifest["started_at_utc"]
manifest["jobs"][job][seed]["status"]
manifest["jobs"][job][seed]["completed-at-utc"]
manifest["jobs"][job][seed]["output-path"]

# In run_summary.json — snake_case keys
rs["priority_retained_value_total"]
rs["standard_retained_value_total"]
rs["pricing_event_stream_sha256"]
rs["components"][i]["component_index"]
rs["components"][i]["latency_blocks_observations"]  # LIST not float (see Pitfall E below)
```

**Landmine (RESEARCH.md Pitfall 1, CLAUDE.md "Serde rename casing is mixed by historical accident"):** Crashing on `KeyError` because the dev assumed both files share casing. The plan must call out the kebab-vs-snake split per access site.

#### Pattern E — `latency_blocks_observations` is a LIST, not a scalar (LANDMINE)

**Source:** `sim-rs/sim-cli/src/metrics/collector.rs` lines 54-81

```rust
/// Latency observations (in blocks). Mean across observations
/// becomes `latency_blocks_mean` in the comparison output.
pub latency_blocks_observations: Vec<f64>,
```

```rust
impl ComponentSummary {
    pub fn latency_blocks_mean(&self) -> f64 {
        if self.latency_blocks_observations.is_empty() {
            0.0
        } else {
            self.latency_blocks_observations.iter().sum::<f64>()
                / (self.latency_blocks_observations.len() as f64)
        }
    }
    ...
}
```

**Pattern to copy into build.py** (matches the Rust accessor that does NOT serialize):

```python
# Compute mean in Python, matching the Rust accessor
"latency_blocks_mean": (
    sum(c["latency_blocks_observations"]) / len(c["latency_blocks_observations"])
    if c["latency_blocks_observations"] else 0.0
),
```

**Landmine #1 (RESEARCH.md Pitfall 5):** The JSON carries the raw `Vec<f64>` — the accessor `latency_blocks_mean()` is a Rust method, dropped at serialisation. The build script must compute the mean.

**Landmine #2 (CONTEXT.md / RESEARCH.md note):** `latency_blocks_observations` is **per-component, not per-lane**. `ComponentSummary` mixes priority + standard inclusions into one list. The UI label in `sim-rs/scripts/viz/static/main.js` must read **"latency by demand component (blocks)"**, not "latency by lane" — even though the original VIZ-03 acceptance criterion uses the "by lane" wording. Surface the component's typical lane separately via the `priority_included` / `standard_included` counts on the same `ComponentSummary`.

#### Pattern F — CSV → long-form JSON for Observable Plot (build.py time-series transform)

**Source:** `sim-rs/sim-cli/src/metrics/time_series.rs` lines 16-20 (CSV header pin) + RESEARCH.md `## Architecture Patterns / Pattern 3`

The Rust writer pins the 15-column header:

```rust
pub const HEADER: &str = "slot,c_priority,c_standard,util_priority_window_x_1e9,\
util_standard_window_x_1e9,mempool_bytes_total,mempool_bytes_priority,\
mempool_bytes_standard,included_bytes_priority,included_bytes_standard,\
included_count_priority,included_count_standard,evicted_quote_drift_count,\
fees_paid_lovelace,refund_lovelace";
```

**Pattern to copy into build.py:**

```python
import csv

LANE_FIELDS = [
    ("c_priority", "priority", "quote_per_byte"),
    ("c_standard", "standard", "quote_per_byte"),
    ("mempool_bytes_total", "total", "mempool_bytes"),
    ("mempool_bytes_priority", "priority", "mempool_bytes"),
    ("mempool_bytes_standard", "standard", "mempool_bytes"),
]

def _read_time_series_long(csv_path):
    with open(csv_path, newline="") as f:
        for row in csv.DictReader(f):
            slot = int(row["slot"])
            for col, lane, metric in LANE_FIELDS:
                yield {"slot": slot, "lane": lane, "metric": metric, "value": int(row[col])}
```

**Copy verbatim style points:**
- `csv.DictReader` with `newline=""` (stdlib idiom for CSV files).
- Column names are **exactly** as in the Rust `HEADER` constant — copy from `time_series.rs` directly, do not paraphrase.
- All values are integers (the Rust writer formats `u64` → string with no decimal). Cast via `int(row[col])`.
- Yield long-form `{slot, lane, metric, value}` records to feed Observable Plot's `stroke: "lane"` channel without a client-side melt.

---

### `sim-rs/scripts/viz/build.py` :: `serve()` (utility / HTTP wrapper)

**No analog in repo.** Use stdlib `http.server.ThreadingHTTPServer` directly. The pattern from RESEARCH.md `## Common Pitfalls / Pitfall 7` is the planner's source of truth:

```python
from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler
import functools

def serve(output_dir, port):
    handler = functools.partial(SimpleHTTPRequestHandler, directory=str(output_dir))
    with ThreadingHTTPServer(("127.0.0.1", port), handler) as httpd:
        print(f"Serving {output_dir} at http://127.0.0.1:{port}/  (Ctrl-C to stop)", file=sys.stderr)
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            pass
```

**Landmine (RESEARCH.md `## Security Domain`):** Bind to `127.0.0.1` explicitly — not `0.0.0.0` (the default of some `http.server` constructors). Prevents accidental LAN exposure of `sim-rs/output/`. The first tuple element in `ThreadingHTTPServer(("127.0.0.1", port), handler)` is load-bearing.

---

### `sim-rs/scripts/viz/static/index.html` (static asset / SPA shell)

**Analog (reference only — DO NOT copy code):** `sim-rs/output/tiered_plot.html`. CONTEXT.md `<canonical_refs>` says explicitly: "reference only — do not import or reuse code." The Plotly.js / Chart.js code paths there are not reusable; only the chart taxonomy (`tier-prices`, `tx-volume`, `tier-delays`, `tier-utils`, `cumulative`, `inclusion-split`) is informational.

**No code-level analog.** The HTML is greenfield. Use RESEARCH.md `## Code Examples / Plot-rendering` as the planner reference. Skeleton (minimal CSS per D-23, ES-module `<script type="module">` per D-03 / D-19 — vendoring recommended per RESEARCH.md Open Q #3):

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>phase-2 sim viz</title>
  <link rel="stylesheet" href="static/style.css">
</head>
<body>
  <div id="app"></div>
  <script type="module" src="static/main.js"></script>
</body>
</html>
```

---

### `sim-rs/scripts/viz/static/main.js` (browser router + renderer)

**No analog in repo** (greenfield JS — confirmed via CONTEXT.md `## Existing Code Insights` "No existing JS/TS infrastructure"). Use RESEARCH.md `## Architecture Patterns / Pattern 2` (hash routing) + RESEARCH.md `## Code Examples / Browser-side Plot rendering` + `## Code Examples / Cross-seed overlay` as the planner reference.

Key shape (from RESEARCH.md Pattern 2):

```javascript
import * as Plot from "./plot.min.js";  // vendored per Open Q #3

async function route() {
  const hash = location.hash || "#/";
  const m = hash.match(/^#\/(?:(suite|job)\/(.+))?$/);
  if (!m || !m[1]) return renderHome();
  if (m[1] === "suite") return renderSuite(m[2]);
  if (m[1] === "job") {
    const [suite, job, seed] = m[2].split("/");
    return renderJob(suite, job, seed);
  }
}

window.addEventListener("hashchange", route);
window.addEventListener("DOMContentLoaded", route);
```

**Landmine (RESEARCH.md `## Security Domain`):** Use `textContent`, never `innerHTML`, when injecting suite/job/seed names into the DOM. HTML injection via suite name is the only realistic threat surface; `textContent` defangs it without effort.

---

### `sim-rs/scripts/viz/static/plot.min.js` (vendored Observable Plot 0.6.x)

**No analog.** Fetch once from `https://cdn.jsdelivr.net/npm/@observablehq/plot@0.6/dist/plot.umd.min.js` (UMD form) or `https://cdn.jsdelivr.net/npm/@observablehq/plot@0.6/+esm` (ESM form) and commit under `sim-rs/scripts/viz/static/`. Filename should encode version for refresh tracking: `plot.0.6.17.min.js` with a `plot.min.js` symlink, or comment the version inside the file's first line.

**Rationale (RESEARCH.md Open Q #3 / A3):** Vendoring upholds the PROJECT.md "Local-first: must work without internet" promise. ~150 KB committed; refresh annually with a one-line PR.

---

### `sim-rs/scripts/viz/static/style.css` (static asset / minimal styling)

**No analog.** Per D-23: "minimal CSS for readability, no design system, no dark-mode toggle, no logo." Plan should keep the file under ~100 lines (rough size sanity from D-23's "developer tool, not a polished product surface").

---

### `sim-rs/scripts/viz/tests/test_build_smoke.py` (unit tests)

**No analog in `sim-rs/scripts/`** (the existing scripts have zero test coverage — RESEARCH.md `## Validation Architecture`: "The existing `sim-rs/scripts/*.py` files are not currently tested at all"). Use stdlib `unittest` with `tempfile.TemporaryDirectory` + synthetic fixtures under `fixtures/`.

Pattern from RESEARCH.md `## Validation Architecture / Wave 0 Gaps`:

```python
import json
import tempfile
import unittest
from pathlib import Path

# Import the build module under test
import sys
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import build  # sim-rs/scripts/viz/build.py


class IndexBuildTest(unittest.TestCase):
    def test_index_lists_all_manifests(self):
        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp) / "src"
            (src / "suite-a").mkdir(parents=True)
            (src / "suite-a" / "manifest.json").write_text(json.dumps({
                "suite-name": "suite-a",
                "started-at-utc": "2026-05-20T10:00:00Z",
                "jobs": {},
            }))
            out = Path(tmp) / "out"
            warnings = []
            build.run_build(source=src, output=out, includes=[], excludes=[], warnings=warnings)
            index = json.loads((out / "data" / "index.json").read_text())
            self.assertEqual(len(index["suites"]), 1)


if __name__ == "__main__":
    unittest.main()
```

**Test cases the plan must cover (RESEARCH.md Phase Requirements → Test Map):**

| Req | Test class :: method |
|-----|----------------------|
| VIZ-01 | `IndexBuildTest::test_index_lists_all_manifests` |
| VIZ-02 | `SuiteJsonTest::test_jobs_match_manifest` |
| VIZ-03 | `SeedJsonTest::test_headline_fields_present` |
| VIZ-04 | `SeedJsonTest::test_time_series_long_form` |
| VIZ-05 | `SuiteJsonTest::test_seed_grouping_present` |
| D-21 | `ErrorHandlingTest::test_malformed_manifest_skipped_with_warning` |
| Pitfall 1 | `IngestTest::test_kebab_case_manifest_snake_case_run_summary` |
| Pitfall 5 | `IngestTest::test_latency_blocks_observations_aggregated_to_mean` |
| Pitfall 8 | `IngestTest::test_missing_time_series_csv_returns_empty_list_with_warning` |

---

### `sim-rs/scripts/viz/tests/test_serve_smoke.py` (integration / subprocess)

**No analog.** Spawn `build.py --serve` in a subprocess on an ephemeral port (`port=0` resolved via socket bind, or a high fixed port), `urllib.request.urlopen` the three URL shapes, assert 200 + non-empty JSON, terminate in `tearDown`. Pattern from RESEARCH.md Phase Requirements → Test Map (VIZ-06 row).

---

### `sim-rs/scripts/viz/README.md` (single documented command)

**Analog:** the module docstring of `sim-rs/scripts/generate-realistic-100-topology.py` lines 1-45 — the same shape and tone, but pulled into a markdown file rather than a docstring.

```python
# Reference (from generate-realistic-100-topology.py docstring)
"""
Phase-2/3 mainnet-faithful topology generator.

Two modes, selected at the command line:

  python3 scripts/generate-realistic-100-topology.py
      → emit parameters/phase-2-sweep/topology-realistic-150.yaml
        ...
  python3 scripts/generate-realistic-100-topology.py --regenerate-100
      → fetch a fresh Koios snapshot and re-emit ...
        ...

The default path (150-node emission) is deterministic and reproducible ...
"""
```

**Pattern to copy:** lead with a one-line summary, then a usage block with literal commands (`python sim-rs/scripts/viz/build.py --serve`), then a flag table, then a brief "what gets written where" map referencing `sim-rs/output/viz/data/{index.json,<suite>.json,<suite>/<job>-<seed>.json}` (D-09).

**CLAUDE.md compliance:** abbreviation expansion on first use applies — "Single-Page Application (SPA)", "Observable Plot", "Endorser Block (EB)" if mentioned, etc. The README is a project doc, not a comment.

**Landmine (RESEARCH.md Pitfall 6):** README must note that `sim-rs/output/viz/` is gitignored transitively via the existing `/output` rule. Do not write a doc that says "we added a new gitignore entry" — there is no new entry.

---

## Shared Patterns

### Skip-and-warn error model (D-21)

**Apply to:** every file-read site in `build.py`.

**Source:** RESEARCH.md `## Architecture Patterns / Pattern 4`.

```python
def main():
    args = parse_args()
    warnings = []
    suites = discover_suites(args.source, args.include, args.exclude, warnings)
    build_data(suites, args.output, warnings)
    copy_static_assets(args.output)
    if warnings:
        print(f"\n[warnings] {len(warnings)} issues:", file=sys.stderr)
        for w in warnings:
            print(f"  - {w}", file=sys.stderr)
    if args.serve:
        serve(args.output, args.port)
```

Every `open()`, `json.load()`, `csv.DictReader()`, `Path.exists()` failure path appends to `warnings` and continues. Exit 0 unless argparse itself fails. Tested by `ErrorHandlingTest::test_malformed_manifest_skipped_with_warning`.

### Suite identifier = path-derived, not `suite-name` derived (D-22, Pitfall 2)

**Apply to:** `discover_suites()` in `build.py`.

**Source:** RESEARCH.md `## Code Examples / Walk for manifests` line 8.

```python
suite_id = str(manifest_path.parent.relative_to(source)).replace("/", "__")
# e.g. "phase-2/eip1559-robustness-20260514-160045"
#  →   "phase-2__eip1559-robustness-20260514-160045"
```

**Landmine (Pitfall 2):** Phase-2 has two suites named `eip1559-robustness` (one bare, one timestamped). Using `manifest["suite-name"]` as the filename collapses them. Always use the relative path with `__` separator.

### Manifest parallelism is NOT a stored field (RESEARCH.md Open Q #1)

**Apply to:** the per-suite metadata block in `<suite>.json`.

**Source:** `sim-rs/sim-cli/src/runner.rs` lines 67-74 (`Manifest` struct — no `parallelism` field).

```rust
pub struct Manifest {
    pub suite_name: String,
    pub started_at_utc: DateTime<Utc>,
    pub jobs: BTreeMap<String, BTreeMap<String, JobEntry>>,
}
```

VIZ-01's acceptance criterion lists "parallelism" as a column. The runner does not persist it. Either (a) omit the column with a one-line note in the plan, or (b) derive a proxy from the (started-at-utc, completed-at-utc) interval overlap across `manifest.jobs[*].*` (recommended in RESEARCH.md Open Q #1).

### `metrics_comparison.txt` is human-only — never parsed (Pitfall 4)

**Apply to:** **negative pattern.** No file-read site in `build.py` should ever open `metrics_comparison.txt`. Every field it contains is also in `run_summary.json`.

### Phase-2 has NO `priority_only_*_comparison.csv` (Pitfall 3)

**Apply to:** the D-13 "suite aggregates" panel.

**Source:** `sim-rs/sim-cli/src/metrics/comparison.rs` writes only `metrics_comparison.txt`. The CSV named in D-13 lives at `sim-rs/output/analysis/priority_only_fast_path_overall_comparison.csv` (historical pre-phase-2). No CSV is emitted at the suite root for phase-2 runs.

The plan must:
1. Treat the "static aggregates panel" as **conditional** — render only when a `*.csv` is present at the suite root. For phase-2 suites today, this panel is empty/absent.
2. Treat the **in-suite cross-seed overlay** (D-15) as the load-bearing VIZ-05 deliverable. The overlay's data comes from per-(job, seed) `run_summary.json` aggregated **in `build.py`**, not from a suite-level CSV.

### `127.0.0.1` bind, never `0.0.0.0` (RESEARCH.md Security Domain)

**Apply to:** `serve()` in `build.py`.

```python
with ThreadingHTTPServer(("127.0.0.1", port), handler) as httpd:
    ...
```

### `textContent`, never `innerHTML` (RESEARCH.md Security Domain)

**Apply to:** every DOM-insertion site in `static/main.js` that handles a manifest-derived string (suite name, job name, seed string).

```javascript
// Good
nameEl.textContent = suite.name;

// Bad — HTML injection if a suite name ever contains '<'
nameEl.innerHTML = suite.name;
```

---

## No Analog Found

Files for which the in-repo search produced no close match (planner falls back to RESEARCH.md `## Code Examples`):

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `sim-rs/scripts/viz/static/main.js` | router/renderer | event-driven | No JS in the repo at all (CONTEXT.md "No existing JS/TS infrastructure"). |
| `sim-rs/scripts/viz/static/style.css` | static | static | No CSS in the repo (`tiered_plot.html` has inline `<style>` only). |
| `sim-rs/scripts/viz/static/plot.min.js` | vendored library | static | Third-party bundle; download once and commit. |
| `sim-rs/scripts/viz/tests/test_*.py` | tests | request-response | No `tests/` directory under `sim-rs/scripts/`. Rust tests in `sim-rs/sim-{core,cli}/tests/` use `cargo test`, not stdlib `unittest`. |
| `sim-rs/scripts/viz/build.py` :: `serve()` | HTTP wrapper | request-response | Stdlib `http.server` boilerplate; pattern from RESEARCH.md Pitfall 7. |

For all of the above, planner should reference RESEARCH.md sections by name in plan action steps, not invent.

---

## Verified Schemas (load-bearing references for plan steps)

These are exact-text references the planner can quote into plan actions without re-deriving them.

### `manifest.json` — kebab-case keys (`sim-rs/sim-cli/src/runner.rs` lines 44-74)

```
{
  "suite-name": String,
  "started-at-utc": ISO-8601 UTC with trailing Z,
  "jobs": {
    "<job_name>": {
      "<seed_string>": {
        "status": "pending" | "running" | "completed" | "failed",
        "started-at-utc": optional,
        "completed-at-utc": optional,
        "output-path": optional,
        "error": optional
      }
    }
  }
}
```

### `run_summary.json` — snake_case keys (`sim-rs/sim-cli/src/metrics/collector.rs` lines 110-160)

```
{
  "components": [ComponentSummary],
  "total_txs_submitted": u64,
  "total_txs_included": u64,
  "total_txs_evicted_quote_drift": u64,
  "total_fees_paid_lovelace": u64,
  "total_refund_lovelace": u64,
  "priority_retained_value_total": f64,
  "standard_retained_value_total": f64,
  "priority_included_value_total": u128 (serialised as number),
  "standard_included_value_total": u128 (serialised as number),
  "block_generation_probability": f64,
  "multiplier_floor_breaches": u64,
  "min_priority_over_standard_ratio": f64,
  "max_priority_over_standard_ratio": f64,
  "pricing_ticks": u64,
  "pricing_event_stream_sha256": String,
  ... (further M6 noise-metric fields, defensively `.get(...)`)
}
```

Each `ComponentSummary` (`collector.rs` lines 31-57):

```
{
  "component_index": u32,
  "txs_submitted": u64,
  "txs_included": u64,
  "txs_evicted_quote_drift": u64,
  "bytes_included": u64,
  "fees_paid_lovelace": u64,
  "refund_lovelace": u64,
  "retained_value_total": f64,
  "net_utility_total": f64,
  "included_value_lovelace_total": u128 (serialised as number),
  "priority_included": u64,
  "standard_included": u64,
  "latency_blocks_observations": [f64]   // LIST, not scalar — Pitfall 5
}
```

### `time_series.csv` — pinned 15-column header (`sim-rs/sim-cli/src/metrics/time_series.rs` lines 16-20)

```
slot,c_priority,c_standard,util_priority_window_x_1e9,util_standard_window_x_1e9,mempool_bytes_total,mempool_bytes_priority,mempool_bytes_standard,included_bytes_priority,included_bytes_standard,included_count_priority,included_count_standard,evicted_quote_drift_count,fees_paid_lovelace,refund_lovelace
```

All columns are integer-valued (`u64` in Rust).

---

## Metadata

**Analog search scope:** `sim-rs/scripts/`, `sim-rs/output/` (read-only), `sim-rs/sim-cli/src/{runner.rs,metrics/}` (schema only — no source consumption).
**Files scanned:** 5 Python scripts + 3 Rust source files + 1 HTML (reference only) + `sim-rs/.gitignore` + `sim-rs/output/` directory listing.
**Pattern extraction date:** 2026-05-20

## PATTERN MAPPING COMPLETE

**Phase:** 01 - viz-site-mvp
**Workstream:** viz-website
**Files classified:** 12 (8 net-new + 4 sub-functions inside `build.py`)
**Analogs found:** 7 strong matches; 5 files have no in-repo analog and rely on RESEARCH.md `## Code Examples` directly.

### Coverage
- Files with exact analog: 3 (`build.py` overall, `discover_suites`/`load_seed` inner pattern, README docstring)
- Files with role-match analog: 1 (argparse main in `generate-realistic-100-topology.py`)
- Files with no analog: 5 (`static/main.js`, `static/index.html`, `static/style.css`, `static/plot.min.js`, `tests/*.py`)
- Files NOT created (landmine): 1 (`.gitignore` edit — pre-existing `/output` rule covers `viz/`)

### Key Patterns Identified
- Stdlib-only Python script with `#!/usr/bin/env python3` + module docstring + argparse + `Path`-typed flags + `sys.stderr` diagnostics matches the canonical `sim-rs/scripts/` convention exactly (D-08).
- `Path.rglob('manifest.json')` is the manifest-discovery pattern; suite ID is derived from `relative_to(source)` with `/` → `__` to avoid the dual `eip1559-robustness` collision (Pitfall 2).
- The kebab-vs-snake casing split between `manifest.json` and `run_summary.json` is a load-bearing distinction enforced by mixed `#[serde(rename_all = "kebab-case")]` attributes (Pitfall 1; CLAUDE.md "Serde rename casing is mixed by historical accident").
- `latency_blocks_observations` is a list per component, **not per lane and not a scalar** — UI label must say "latency by demand component (blocks)" (Pitfall 5).
- `metrics_comparison.txt` is human-only prose-Markdown; every field is also in `run_summary.json` and the build script must not parse it (Pitfall 4).
- D-13's `priority_only_*_comparison.csv` does not exist in `phase-2/`; the aggregate panel is conditional/absent, and VIZ-05's load-bearing deliverable is the in-suite cross-seed overlay built from per-(job, seed) `run_summary.json` (Pitfall 3).
- `sim-rs/output/viz/` is gitignored transitively via `sim-rs/.gitignore` line 2 (`/output`); no new gitignore line should be added (Pitfall 6).
- Local HTTP serve binds `127.0.0.1` explicitly; DOM injection uses `textContent` not `innerHTML` (Security Domain).

### File Created
`/home/will/git/arc-tiered-pricing/.planning/workstreams/viz-website/phases/01-viz-site-mvp/01-PATTERNS.md`

### Ready for Planning
Pattern mapping complete. The planner can now reference these analog excerpts directly inside per-task action steps, with `build.py`'s shape grounded in `generate-realistic-100-topology.py` + `analyse-phase-3.py`, schema knowledge grounded in `runner.rs` + `collector.rs` + `time_series.rs`, and the five greenfield (`static/*` + `tests/*`) files routed to RESEARCH.md's `## Code Examples` + `## Validation Architecture` sections.
