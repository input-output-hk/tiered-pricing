# Phase 1: Viz Site MVP - Research

**Researched:** 2026-05-20
**Domain:** Python build-script that crawls `sim-rs/output/` + static HTML/JS bundle with Observable Plot in the browser
**Confidence:** HIGH (data shapes verified by file inspection; library version verified via jsDelivr; existing-script patterns read end-to-end)

## Summary

The build script is a stdlib-only Python program at `sim-rs/scripts/viz/build.py`. It walks `sim-rs/output/` recursively for directories containing `manifest.json` (the runner-emitted suite index), reads each `manifest.json` plus the per-(job, seed) `run_summary.json` and `time_series.csv`, and emits a three-tier JSON tree under `sim-rs/output/viz/data/`. A small static HTML+JS bundle reads those JSON files in the browser and draws charts with Observable Plot.

The on-disk schema is fully nailed down: `manifest.json` uses kebab-case keys (`suite-name`, `started-at-utc`, `jobs[<job>][<seed>]` → `{status, started-at-utc, completed-at-utc, output-path}`); `run_summary.json` uses snake_case with 23 top-level fields including `components: [ComponentSummary]`; `time_series.csv` has a 15-column pinned header starting with `slot,c_priority,c_standard,…`. There is **no per-suite `*comparison.csv`** in the phase-2 tree — only a Markdown-ish `metrics_comparison.txt`. VIZ-05's example file (`priority_only_fast_path_overall_comparison.csv`) lives in `sim-rs/output/analysis/` from older work and is not what the phase-2 suites emit.

**Primary recommendation:** Walk `sim-rs/output/` for directories whose **immediate child** is a `manifest.json`. Treat each such directory as one "suite run." Read `manifest.json` for the suite index; for each (job, seed) read the seed directory's `run_summary.json` (rich JSON — the headline-metrics source) and `time_series.csv` (15-column CSV — the time-series source). Convert both to JSON at build time. Skip-and-warn on missing files. Use Observable Plot 0.6.17 loaded as an ES module from `https://cdn.jsdelivr.net/npm/@observablehq/plot@0.6/+esm` (which auto-bundles d3 — no separate d3 script). For hash-based routing, use three views: `#/` (suite list), `#/suite/<suite_id>` (drill-down), `#/job/<suite_id>/<job>/<seed>` (per-(job, seed) detail).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Site type & rendering:**
- **D-01:** Rendering model is **build-script → static HTML+JS bundle**. No committed bundle, no local dev server, no off-the-shelf framework (Observable Framework / Streamlit / Evidence.dev). Trade-off: re-build required when new suite data arrives; accepted because it keeps the tooling footprint minimal.
- **D-02:** Build script is **Python**, sitting under `sim-rs/scripts/viz/` alongside the existing `generate-realistic-100-topology.py` and `analyse-phase-3.py`. Not Rust, not Node, not a Cargo binary.
- **D-03:** Chart rendering uses **Observable Plot** loaded in the browser. ~150 KB, declarative API on top of D3, designed for the time-series and small-multiples shape the phase-2 outputs need. Whether Plot is CDN-loaded or vendored under `sim-rs/scripts/viz/static/` is at the planner's discretion (D-19); both are acceptable.
- **D-04:** The site is **served via a local HTTP server** (`python -m http.server` or a built-in helper inside the build script), not opened via `file://`. ES-module-friendly, no CORS gotchas, matches VIZ-06's "single documented command" wording.

**Repo location & build entry-point:**
- **D-05:** Build script and supporting assets live at **`sim-rs/scripts/viz/`** (`build.py`, `static/` for any vendored JS/CSS, templates inline in `build.py` or in adjacent `.html` files). Matches the existing `sim-rs/scripts/` convention.
- **D-06:** Built bundle output goes to **`sim-rs/output/viz/`**, **gitignored**. Nothing about the rendered bundle is committed; regenerated on demand. The `.gitignore` entry must be added to either the repo root or `sim-rs/.gitignore` as part of the plan.
- **D-07:** Entry-point command is **`python sim-rs/scripts/viz/build.py --serve`**, which builds the bundle then serves it on a local port. Drop `--serve` for build-only. This is the "single documented command" that VIZ-06 calls for; doc it in CLAUDE.md or a new `sim-rs/scripts/viz/README.md`.
- **D-08:** Python dependencies are **stdlib only** (`json`, `argparse`, `pathlib`, `http.server`, `csv`), plus PyYAML if any field requires it. **No `requirements.txt`**, no virtualenv, no Jinja2. HTML templating via f-strings or `string.Template`.

**Data plumbing:**
- **D-09:** Data is delivered in **three tiers**:
  - `sim-rs/output/viz/data/index.json` — list of every ingested suite with metadata (name, run date, job count, parallelism).
  - `sim-rs/output/viz/data/<suite>.json` — per-suite headline metrics for every (job, seed): `retained_value`, `net_utility`, `retained_value_ratio`, latency-by-lane, peak mempool depth.
  - `sim-rs/output/viz/data/<suite>/<job>-<seed>.json` (or similar) — per-(job, seed) time-series, **fetched on demand** only when the user opens that view.
  Browser fetches `index.json` on page load, fetches per-suite JSON on suite click, fetches per-(job, seed) JSON on job click. Bounded initial load; drill-downs cost one HTTP round-trip each.
- **D-10:** **Default ingestion scope = "every directory containing a `manifest.json`"**, found by walking `sim-rs/output/` recursively. Directories without `manifest.json` are silently skipped. Support `--include <glob>` and `--exclude <glob>` flags on `build.py` for ad-hoc scoping (e.g. `--include 'phase-2/*'` to limit to the phase-2 subtree).
- **D-11:** CSV → JSON conversion happens **at build time in Python**. The browser never parses raw CSV. The JSON shape is whatever the page needs — the planner picks the schema; a 1:1 mirror of the CSV columns is fine for v1.

**First-view priority:**
- **D-12:** **Landing page = suite list / browser** (the VIZ-01 view). Sortable table-or-list of every ingested suite with metadata: name, run date, job count, parallelism, perhaps a count of completed (job, seed) pairs. Clicking a row navigates to the suite drill-down view.
- **D-13:** **Suite drill-down view = sortable per-(job, seed) table** with headline metric columns (`retained_value`, `net_utility`, `retained_value_ratio`, latency-by-lane, peak mempool depth). When the suite root contains aggregate CSVs (e.g. `priority_only_fast_path_overall_comparison.csv`), they render as a separate **"Suite aggregates"** section on the same page (chart or table — planner's call).
- **D-14:** **Per-(job, seed) detail view = single scrollable page**:
  - Top: a **strip of headline metrics** for this (job, seed).
  - Below: **time-series panes** stacked vertically — controller `quote_per_byte` per lane, mempool size, `derived_quote` per block (or whatever fields the time-series CSV carries; planner schema-driven). One pane per metric, lane-coloured where applicable.
  No tabs, no side-by-side layout. Copy-paste friendly.
- **D-15:** **Comparison (VIZ-05) scope for v1 = in-suite cross-seed overlay only**. Inside the suite view, the user can pick a job and see all its seeds overlaid on the time-series chart and/or summary table. **Cross-suite comparison is deferred** to a follow-on phase / v1.1. The suite-level aggregate CSVs (per D-13) satisfy the static-rendering side of VIZ-05.

### Claude's Discretion

The planner picks these without re-asking the user:

- **D-16:** HTML templating style inside Python — f-strings vs `string.Template` vs ad-hoc concatenation. Default: f-strings unless they hurt readability.
- **D-17:** Exact column set for the suite drill-down table beyond the headline metrics listed in D-13. Add or omit columns as the data warrants.
- **D-18:** Initial sort order on the suite list (D-12). Default: most-recent run date first.
- **D-19:** Observable Plot loading strategy: CDN (`https://cdn.jsdelivr.net/npm/@observablehq/plot`) vs vendored under `sim-rs/scripts/viz/static/plot.umd.min.js`. Either is fine; pick whichever makes the resulting site easier to use offline.
- **D-20:** Empty-state copy when `sim-rs/output/` has no suites with `manifest.json` (build still succeeds; the site shows an explanatory placeholder).
- **D-21:** Error handling for malformed `manifest.json` files or missing CSVs — skip-and-warn vs fail-the-build. Default: skip-and-warn, accumulate warnings, print them at the end of the build.
- **D-22:** Suite-deduplication strategy when two `<suite>` directory names collide (e.g. re-runs at different timestamps). Default: include the full path in the suite identifier.
- **D-23:** Visual theming depth. Default: minimal CSS for readability, no design system, no dark-mode toggle, no logo. The site is a developer tool, not a polished product surface.
- **D-24:** Accessibility expectations beyond semantic HTML. Default: no explicit a11y target; the audience is internal.

### Deferred Ideas (OUT OF SCOPE)

- **Cross-suite comparison view** — multi-select suites/jobs side-by-side. Most powerful comparison; deferred per D-15 to keep MVP scope tight.
- **Paired-bootstrap CI band visualisation** — `paired_bootstrap.rs` outputs are available but not displayed in v1; relevant once the per-job patterns land.
- **Event-stream drill-down** — `TXIncluded` / `TXEvictedQuoteDrift` timelines per (job, seed). High data volume, debugging-oriented; not analysis-oriented MVP material.
- **Public hosting / GitHub Pages deployment** — only after the local-first path proves out.
- **Direct integration with the `experiment-suite` runner** — live-updating during a run.
- **Visualisation of `.planning/realism-tests/` results** — beyond `sim-rs/output/`; useful future scope.
- **Polished theming / dark mode / responsive layout** — the v1 audience is the simulator dev, not a CIP reader.
- **Schema validation of `manifest.json`** — the build skip-and-warns on malformed inputs (D-21); a stricter validation step is future hardening.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| VIZ-01 | Suite-list view with metadata (name, run date, parallelism, job count) | `manifest.json` carries `suite-name`, `started-at-utc`, `jobs` map. **Parallelism is NOT in `manifest.json`** — derive from job count or omit. Recursive walk over `sim-rs/output/` finds 249 manifest files today. |
| VIZ-02 | One-click drill-down to suite detail (manifest summary + job list + seed inventory) | Per `manifest.json`: enumerate `jobs[job_name].keys()` for seed inventory; `jobs[job_name][seed].status` for completion state; `output-path` for resolving artefact directories. |
| VIZ-03 | Headline metrics per (job, seed) | All five locked metrics are in `run_summary.json`: `priority_retained_value_total + standard_retained_value_total` → `retained_value`; per-component `net_utility_total`; ratios derived from `retained_value_total / included_value_lovelace_total`; `latency_blocks_observations` per component → mean per lane; peak mempool depth derived from `time_series.csv` (`max(mempool_bytes_total)`). |
| VIZ-04 | Time-series multi-line with lane colouring | `time_series.csv` has 15 columns, ~2000 rows per (job, seed). Render three panes: (a) `c_priority` + `c_standard` vs `slot` (controller quote per lane), (b) `mempool_bytes_priority` + `mempool_bytes_standard` + `mempool_bytes_total` vs `slot`, (c) per-block fees/refunds. Observable Plot's `stroke` channel over a long-form record gives lane colouring. |
| VIZ-05 | In-suite cross-seed overlay | All seeds for one job share the same x-axis (`slot`). Plot a single line mark with `stroke: "seed"` over the concatenated long-form records. Static aggregates: HTML table works fine; no per-suite `*comparison.csv` actually exists in the phase-2 tree (see Common Pitfalls). |
| VIZ-06 | Single documented command for fresh dev environment | `python sim-rs/scripts/viz/build.py --serve` produces a viewable site. Documented in `sim-rs/scripts/viz/README.md` (per D-07) and surfaced in root `CLAUDE.md`. |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

These are CLAUDE.md directives that apply to phase 1 work even though phase 1 touches no simulator code:

- **Abbreviations expanded on first use.** All `.planning/`, `docs/`, and project documentation must spell out abbreviations on first use with the abbreviation in parentheses (e.g., "Bias-corrected and accelerated (BCa) bootstrap"). Applies to the new `sim-rs/scripts/viz/README.md`.
- **Build command for Rust workspace is `cd sim-rs && cargo build --release`; tests are `cd sim-rs && cargo test --workspace`.** Phase 1 does not change either, but referenced for context.
- **`sim-rs/scripts/` is the canonical location for one-shot Python.** Stdlib + PyYAML; argparse-driven CLIs; no `requirements.txt`. D-02 / D-05 / D-08 inherit this directly.
- **`sim-rs/output/` is gitignored** via `sim-rs/.gitignore` line `/output`. Phase 1's bundle output to `sim-rs/output/viz/` is **already covered by this rule** — no `.gitignore` edit strictly required (see Common Pitfalls #6 for why D-06's "must be added" wording is satisfied by the existing rule).
- **No `f64` in simulation-affecting state.** Irrelevant for phase 1 (reads only; never writes to simulator artefacts). The reporting-f64 fields in `run_summary.json` (`*_retained_value_total`, `net_utility_total`, etc.) are explicit "reporting f64" per CLAUDE.md and safe to consume directly in the build script.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Crawl `sim-rs/output/` for manifest files | Build-time Python | — | Filesystem walk; cannot be done in the browser. |
| Parse `manifest.json` + `run_summary.json` (JSON) | Build-time Python | — | Already JSON; Python's `json.load` is the natural reader. |
| Parse `time_series.csv` (15-column CSV) | Build-time Python | — | D-11 explicit: "browser never parses raw CSV." Use `csv.DictReader`. |
| Emit `index.json` / `<suite>.json` / `<suite>/<job>-<seed>.json` | Build-time Python | — | D-09 explicit: three-tier JSON layout. |
| Render suite list / drill-down / detail views | Browser HTML+JS | — | D-01 / D-03; static HTML pages served over local HTTP. |
| Draw charts (multi-line time-series, cross-seed overlay) | Browser, Observable Plot | — | D-03 explicit. |
| Route between views (`#/`, `#/suite/<id>`, `#/job/<id>/<job>/<seed>`) | Browser JS (hash router) | — | Three views, no server-side routing; hash routing avoids server config (D-04 uses `http.server`). |
| Serve static files | Build-time Python (`http.server`) | — | D-04 / D-07. Single command must both build and serve. |
| Validate the rendered output | Build-time Python | Manual visual spot-check | curl asserts on served URLs cover structural; visual is for chart correctness. |

**Why this map matters:** Every data transformation happens **once at build time** in Python. The browser receives ready-to-render JSON; this keeps the JS thin (no CSV parsing, no path computation), the bundle small, and the failure modes inspectable (look at the JSON on disk before debugging the browser).

## Standard Stack

### Core

| Component | Version | Purpose | Why Standard |
|-----------|---------|---------|--------------|
| Python | 3.x stdlib | Build script, file IO, HTTP serve | D-02 / D-08; matches existing `sim-rs/scripts/` convention. [VERIFIED: scripts/analyse-phase-3.py uses pure stdlib + 3.8+ `NormalDist.inv_cdf`] |
| Observable Plot | 0.6.17 | Browser charting | D-03; current version as of 2026-05-20. [VERIFIED: `https://cdn.jsdelivr.net/npm/@observablehq/plot/package.json`, fetched 2026-05-20] |
| Browser ES modules | native | Module loading without bundler | Plot's recommended loading mode bundles d3 automatically when imported via `+esm`. [CITED: observablehq.com/plot/getting-started] |

### Supporting

| Component | Version | Purpose | When to Use |
|-----------|---------|---------|-------------|
| `python -m http.server` (stdlib) | 3.x | Serve the built bundle | When `--serve` is passed (D-07). Single-threaded; fine for one local dev. |
| `argparse` (stdlib) | 3.x | CLI flag parsing | `--serve`, `--include`, `--exclude`, `--port`, `--output-dir` (D-07/D-10). Matches `generate-realistic-100-topology.py` style. |
| `csv.DictReader` (stdlib) | 3.x | Parse `time_series.csv` | One call per (job, seed). 2000 rows × 15 cols × ~100 suites × ~5 jobs × 3 seeds ≈ 4.5M cells; stdlib `csv` handles this in seconds. |
| `pathlib.Path` (stdlib) | 3.x | All file walks and path arithmetic | Matches `analyse-phase-3.py` style. `Path.rglob('manifest.json')` does the recursive scan in one line. |
| `string.Template` or f-strings | 3.x | HTML emission | D-16: f-strings as default; switch to `string.Template` only if HTML's `{` / `}` get hostile. |
| `fnmatch` (stdlib) | 3.x | `--include` / `--exclude` glob matching | D-10. |

### Alternatives Considered (and why rejected)

| Instead of | Could Use | Rejected Because |
|------------|-----------|------------------|
| Observable Plot | Plotly.js | D-03 already excluded; heavy (~3.5 MB vs Plot's ~150 KB). |
| Observable Plot | Chart.js | D-03 already excluded; too thin for the layered comparison use case future iterations will want. |
| Observable Plot | Vega-Lite | D-03 already excluded; too rigorous for an MVP. |
| Python build-script | Observable Framework / Evidence.dev | D-01 already excluded; brings a Node toolchain. |
| Python build-script | Streamlit | D-01 already excluded; long-running process, not local-first static. |
| `python -m http.server` | `caddy file-server` / `npx serve` | D-08 forbids non-stdlib; D-02 forbids Node. |
| ES module CDN | UMD `plot.umd.min.js` script tags | UMD is valid too (D-19 leaves it open); choose ESM because Plot's docs treat it as preferred and it auto-bundles d3 (no separate `<script src="d3">` tag). |

**Installation:**

No installation step. Python 3.x is already on the dev machine (used by existing `sim-rs/scripts/*.py`). The browser fetches Plot from jsDelivr at first page load (or, if vendored per D-19, from a local file under `sim-rs/scripts/viz/static/`).

**Version verification:**

- `@observablehq/plot@0.6.17` — verified via `https://cdn.jsdelivr.net/npm/@observablehq/plot/package.json` at 2026-05-20. The major.minor pin `@0.6` in the CDN URL is the recommended form per Plot's docs; it auto-tracks patch releases within 0.6.x.
- Python 3.x — version requirement inherited from `analyse-phase-3.py` (uses `statistics.NormalDist.inv_cdf`, Python 3.8+). No tighter pin needed for the viz build.

## System Architecture Diagram

```
┌──────────────────────────────────────────────────────────────────────┐
│                    sim-rs/output/                                    │
│                                                                      │
│   phase-2/<suite-run>/                                               │
│     ├── manifest.json           ← suite index (kebab-case keys)      │
│     ├── metrics_comparison.txt  ← prose-Markdown aggregate           │
│     └── <job_name>/                                                  │
│         └── <seed>/                                                  │
│             ├── run_summary.json     ← rich snake_case JSON          │
│             ├── time_series.csv      ← 15-col CSV, ~2000 rows        │
│             ├── pricing_event_stream.sha256                          │
│             └── diagnostics.log                                      │
│                                                                      │
│   phase-3/<suite-run>/    ← same shape                               │
│   <other roots>/<suite-run>/    ← same shape                         │
└────────────────────────────┬─────────────────────────────────────────┘
                             │
                             │  `python sim-rs/scripts/viz/build.py [--serve]`
                             ▼
┌──────────────────────────────────────────────────────────────────────┐
│  Build-time pipeline (Python stdlib)                                 │
│                                                                      │
│   1. Walk: Path('sim-rs/output').rglob('manifest.json')              │
│      → list of (suite_dir, manifest_path)                            │
│   2. For each suite_dir:                                             │
│      a. Read manifest.json (kebab-case)                              │
│      b. For each (job, seed) in manifest.jobs:                       │
│         - Read run_summary.json → headline metrics                   │
│         - Read time_series.csv → per-slot rows                       │
│         - Apply --include / --exclude globs                          │
│      c. Build per-suite headline JSON (Tier 2 — D-09)                │
│      d. Build per-(job, seed) time-series JSONs (Tier 3 — D-09)      │
│   3. Build cross-suite index.json (Tier 1 — D-09)                    │
│   4. Copy static assets (HTML, CSS, JS, optionally vendored Plot)    │
│   5. If --serve: cd output/viz/ && http.server on --port             │
└────────────────────────────┬─────────────────────────────────────────┘
                             │
                             ▼
┌──────────────────────────────────────────────────────────────────────┐
│  sim-rs/output/viz/  (gitignored)                                    │
│                                                                      │
│   ├── index.html              ← entry; loads main.js                 │
│   ├── static/                                                        │
│   │   ├── main.js             ← hash router + view renderers         │
│   │   ├── style.css           ← minimal styling (D-23)               │
│   │   └── plot.min.js?        ← only if D-19 picks vendored          │
│   └── data/                                                          │
│       ├── index.json          ← Tier 1: all suites                   │
│       ├── <suite_id>.json     ← Tier 2: per-suite headline metrics   │
│       └── <suite_id>/                                                │
│           └── <job>-<seed>.json  ← Tier 3: time-series, on-demand    │
└────────────────────────────┬─────────────────────────────────────────┘
                             │
                             │  http://localhost:<port>/
                             ▼
┌──────────────────────────────────────────────────────────────────────┐
│  Browser (ESM, Observable Plot 0.6.17)                               │
│                                                                      │
│  On load:  fetch('/data/index.json') → render suite list             │
│  On #/suite/<id>:  fetch('/data/<id>.json') → render drill-down      │
│  On #/job/<id>/<j>/<s>:  fetch('/data/<id>/<j>-<s>.json') → detail   │
│                                                                      │
│  Plot.plot({ marks: [Plot.line(rows, {x:'slot', y:'c_priority',      │
│              stroke: 'lane'})] })  ← VIZ-04                          │
└──────────────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure

```
sim-rs/
├── scripts/
│   ├── viz/                              # NEW — phase 1
│   │   ├── build.py                      # main entrypoint (D-07)
│   │   ├── README.md                     # "single documented command" (D-07)
│   │   ├── static/
│   │   │   ├── index.html                # SPA shell
│   │   │   ├── main.js                   # router + render functions
│   │   │   ├── style.css                 # minimal (D-23)
│   │   │   └── plot.min.js               # optional vendored Plot (D-19)
│   │   └── tests/                        # smoke tests (see Validation Architecture)
│   │       └── test_build_smoke.py
│   ├── generate-realistic-100-topology.py  # existing — style reference
│   └── analyse-phase-3.py                  # existing — style reference
├── output/                               # gitignored at sim-rs/.gitignore /output
│   ├── phase-2/                          # source suites
│   ├── phase-3/                          # source suites
│   └── viz/                              # NEW — build target (D-06)
│       ├── index.html
│       ├── static/
│       └── data/
└── …
```

## Architecture Patterns

### Pattern 1: Single-shot CLI with shared core, optional --serve subcommand-equivalent

Existing scripts (`generate-realistic-100-topology.py`, `analyse-phase-3.py`) use a flat-`main()` argparse pattern, not subcommands. Match that for VIZ-06's "single documented command" clarity.

```python
# sim-rs/scripts/viz/build.py
"""
Build the phase-2 visualisation site against sim-rs/output/.

Usage:
  python3 sim-rs/scripts/viz/build.py                    # build only
  python3 sim-rs/scripts/viz/build.py --serve            # build + serve
  python3 sim-rs/scripts/viz/build.py --include 'phase-2/*'
"""

import argparse
import json
import sys
from pathlib import Path


def parse_args():
    p = argparse.ArgumentParser(description="...")
    p.add_argument("--source", type=Path, default=Path("sim-rs/output"),
                   help="Root to walk for manifest.json files (default: sim-rs/output)")
    p.add_argument("--output", type=Path, default=Path("sim-rs/output/viz"),
                   help="Build output directory (default: sim-rs/output/viz)")
    p.add_argument("--include", action="append", default=[],
                   help="Glob to include (repeatable; matched against suite path)")
    p.add_argument("--exclude", action="append", default=[],
                   help="Glob to exclude (repeatable; matched against suite path)")
    p.add_argument("--serve", action="store_true",
                   help="After build, serve via http.server")
    p.add_argument("--port", type=int, default=8000)
    return p.parse_args()


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


if __name__ == "__main__":
    main()
```

**When to use:** This is the only entrypoint. Don't split build/serve into two scripts — VIZ-06 wants one command. [CITED: existing scripts in `sim-rs/scripts/`]

### Pattern 2: Hash-based SPA routing without a router library

Three views, no server-side routing (D-04 uses `http.server`). Listen for `hashchange` and dispatch.

```javascript
// sim-rs/scripts/viz/static/main.js
import * as Plot from "https://cdn.jsdelivr.net/npm/@observablehq/plot@0.6/+esm";

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

async function renderHome() {
  const data = await fetch("data/index.json").then(r => r.json());
  // sortable table, default sort by run-date desc (D-18)
}

async function renderSuite(suiteId) {
  const data = await fetch(`data/${suiteId}.json`).then(r => r.json());
  // per-(job, seed) table + cross-seed overlay charts (D-13, D-15)
}

async function renderJob(suiteId, job, seed) {
  const data = await fetch(`data/${suiteId}/${job}-${seed}.json`).then(r => r.json());
  // headline strip + stacked time-series panes (D-14)
}
```

**When to use:** Phase 1's three views. No library. [CITED: hashchange is a standard browser event since 2009.]

### Pattern 3: Build-time CSV-to-JSON with a long-form schema

For VIZ-04 (multi-line, lane-coloured) and VIZ-05 (cross-seed overlay), pre-shape the time-series into the *long-form* records that Observable Plot consumes natively. Don't ship wide CSV-shaped JSON to the browser; the JS would need a melt step.

```python
# sim-rs/scripts/viz/build.py (sketch)
import csv

LANE_FIELDS = [
    ("c_priority", "priority", "quote_per_byte"),
    ("c_standard", "standard", "quote_per_byte"),
    ("mempool_bytes_priority", "priority", "mempool_bytes"),
    ("mempool_bytes_standard", "standard", "mempool_bytes"),
]

def time_series_long(csv_path):
    """Return list of {slot, lane, metric, value} records (long form)."""
    out = []
    with open(csv_path, newline="") as f:
        for row in csv.DictReader(f):
            slot = int(row["slot"])
            for col, lane, metric in LANE_FIELDS:
                out.append({
                    "slot": slot,
                    "lane": lane,
                    "metric": metric,
                    "value": int(row[col]),
                })
    return out
```

Then in the browser:

```javascript
Plot.plot({
  color: { legend: true },
  marks: [
    Plot.line(records.filter(r => r.metric === "quote_per_byte"),
              {x: "slot", y: "value", stroke: "lane"}),
  ],
});
```

**Why long-form:** Plot's `stroke: "lane"` and faceting both expect long-form. Sub-100 KB per (job, seed) at 2000 slots × 4 fields × ~40 bytes ≈ 320 KB JSON; trim to only the fields a chart needs. [CITED: observablehq.com/plot/marks/line — z/stroke channel groups data into series.]

### Pattern 4: Skip-and-warn error model

Per D-21, accumulate warnings, print at end of build, exit 0.

```python
def discover_suites(source, includes, excludes, warnings):
    suites = []
    for manifest_path in source.rglob("manifest.json"):
        try:
            with open(manifest_path) as f:
                manifest = json.load(f)
        except (json.JSONDecodeError, OSError) as e:
            warnings.append(f"skip {manifest_path}: {e}")
            continue
        # ... include/exclude globs against str(manifest_path.parent.relative_to(source))
        suites.append((manifest_path.parent, manifest))
    return suites
```

**When to use:** Default for v1; D-21 already locks this in. [CITED: D-21]

### Anti-Patterns to Avoid

- **Reading CSVs in the browser.** D-11 explicit: convert at build time. Avoid `d3.csv()` even though Plot has access to it.
- **Bundling all time-series into one big JSON.** With ~100 suites × ~5 jobs × 3 seeds × 2000 slots × 15 cols, a monolithic JSON would be tens of MB. D-09 splits Tier 3 per (job, seed) precisely to bound the page load. Honour the split.
- **Bundler / Node toolchain.** D-01, D-02, D-08 all forbid it. f-strings template the HTML.
- **Computing parallelism from `manifest.json`.** It's not in there — the field doesn't exist. Either omit from VIZ-01's column list or derive a proxy (e.g. count of completed jobs / wall-clock duration). See Open Questions #1.
- **Long-running serve coupled to a watcher.** Single-threaded `http.server` after a one-shot build is enough. No file-system watcher; the user re-runs `build.py --serve` when new suites land. This matches D-01's "re-build required when new suite data arrives; accepted because it keeps the tooling footprint minimal."

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CSV parsing | Hand-written split-on-comma | `csv.DictReader` | Stdlib handles quoting, BOM, mixed line endings. The `time_series.csv` is well-formed (no quoting needed) but `DictReader` is one line either way. |
| HTTP server | `socket.socket` loop | `python -m http.server` (or `http.server.HTTPServer` in-process) | Stdlib. Single-threaded fine for one local dev. |
| Hash routing | Custom history API + URL parser | Simple `location.hash` + `hashchange` event | Three views. Routers (e.g. `wouter`, `page.js`) are overkill and pull in modules. |
| Multi-line charts | Hand-drawn SVG | `Plot.line({stroke: 'lane'})` | Plot's grouping/legend/scales/axes are battle-tested. |
| Path arithmetic / globbing | Manual `os.path.join` + regex | `pathlib.Path` + `fnmatch.fnmatch` | `analyse-phase-3.py` already uses `pathlib`; match style. |
| JSON formatting | String concatenation | `json.dump(..., indent=2)` | Stdlib, deterministic output (key-ordered when `sort_keys=True`). |
| Date parsing | `re` regex | `datetime.datetime.fromisoformat` (Python 3.11+ for the trailing-Z form, 3.7+ otherwise with manual `replace("Z", "+00:00")`) | `started-at-utc` is ISO-8601 RFC-3339 with trailing `Z`. |

**Key insight:** Everything phase 1 needs is in Python stdlib or Observable Plot. The "don't hand-roll" list reduces to "use stdlib for IO, use Plot for charts, write the rest as straight-line procedural Python."

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Python 3 | Build script | ✓ | `python3` available; existing scripts run | — |
| `python -m http.server` | `--serve` mode | ✓ | Stdlib | — |
| Network access (jsDelivr CDN) | Browser ESM load of Plot | Unknown at run-time | — | Vendor `plot.min.js` per D-19 → fully offline |
| Modern browser (ES modules) | All views | Assumed (dev machine) | — | Document Chrome/Firefox/Safari version in README; UMD fallback if needed |
| `git` for `.gitignore` edits | D-06 | ✓ | — | — |

**Missing dependencies with no fallback:** None.

**Missing dependencies with fallback:**
- Internet at first load: vendor Plot under `sim-rs/scripts/viz/static/plot.min.js` (~150 KB). D-19 leaves this open; **recommend vendoring** so the local-first promise (VIZ-06, PROJECT.md "Local-first: must work without internet") holds even when the dev is offline. Vendoring adds ~150 KB to the committed `static/` directory but eliminates the only network dependency.

## Common Pitfalls

### Pitfall 1: Casing mismatch between `manifest.json` and `run_summary.json`

**What goes wrong:** Build script crashes with `KeyError` because the dev assumed both files use the same casing.
**Why it happens:** The runner's `Manifest`/`JobEntry` structs use `#[serde(rename_all = "kebab-case")]`; `RunSummary` does not. The two files coexist with different conventions. [VERIFIED: `sim-rs/sim-cli/src/runner.rs` lines 45/54/68; `sim-rs/sim-cli/src/metrics/collector.rs` line 112]
**How to avoid:** In the build script, document the casing explicitly per field. Treat `manifest.json` as kebab-case (`suite-name`, `started-at-utc`, `output-path`, `completed-at-utc`, `status`); treat `run_summary.json` as snake_case (`total_txs_submitted`, `priority_retained_value_total`, `pricing_event_stream_sha256`, `components`). Defensive: `manifest["suite-name"]` (square brackets, exact key); never `manifest["suite_name"]`. [CITED: CLAUDE.md "Serde rename casing is mixed by historical accident"]
**Warning signs:** KeyError on a key that "obviously" exists. Always re-check casing.

### Pitfall 2: Suite directory name collisions across timestamped reruns

**What goes wrong:** Two suite directories called `eip1559-robustness` (one timestamped, one not) overwrite each other in the `<suite>.json` Tier-2 layer; the second clobbers the first.
**Why it happens:** Suite directories are named by suite name; reruns add a timestamp suffix (e.g. `eip1559-robustness-20260514-160045`). But the bare name `eip1559-robustness` also exists (no timestamp) for older or one-off runs. Same name → same JSON filename. [VERIFIED: `ls sim-rs/output/phase-2/` shows both `eip1559-robustness` and `eip1559-robustness-20260514-160045`.]
**How to avoid:** Per D-22 default, build the suite identifier from the **path relative to `--source`**: `phase-2/eip1559-robustness` vs `phase-2/eip1559-robustness-20260514-160045`. Use a sanitised version (e.g. `phase-2__eip1559-robustness-20260514-160045`) as the Tier-2 JSON filename. Surface the original path in the suite's `id` field in `index.json` for UI display.
**Warning signs:** `index.json` has the right count but two entries show identical content; one of the rebuilds shadowed an earlier suite.

### Pitfall 3: The `priority_only_fast_path_overall_comparison.csv` example does not exist in `phase-2/`

**What goes wrong:** The planner reads VIZ-05 / CONTEXT.md D-13 and writes a task that opens `sim-rs/output/phase-2/<suite>/priority_only_fast_path_overall_comparison.csv`. It's not there.
**Why it happens:** That filename is **historical** — it lives at `sim-rs/output/analysis/priority_only_fast_path_overall_comparison.csv` from older (pre-phase-2) tiered-pricing analysis. The phase-2 metrics writer (`sim-rs/sim-cli/src/metrics/comparison.rs`) emits **only** `metrics_comparison.txt` — a Markdown-ish prose file, not CSV. [VERIFIED: `find sim-rs/output/ -name '*comparison.csv'` returns one file under `output/analysis/`; `find sim-rs/output/phase-2/ -name '*.csv'` returns only `time_series.csv` files.]
**How to avoid:** Treat the "suite-level aggregate CSV" mention in D-13 as **conditional**: render an aggregates panel only when a `*.csv` file is present at the suite root. For phase-2 suites, that panel is empty (or absent). For VIZ-05's in-suite cross-seed overlay (the actual v1 scope per D-15), the data comes from per-(job, seed) `run_summary.json` aggregated **by the build script**, not from a suite-level CSV.
**Warning signs:** Empty aggregate panel on every phase-2 suite; the file the task expected isn't on disk.

### Pitfall 4: `metrics_comparison.txt` is Markdown, not structured

**What goes wrong:** Task tries to parse `metrics_comparison.txt` as a CSV or table, gets garbled output.
**Why it happens:** `comparison.rs::write_run` writes prose-Markdown bullet lists per (job, seed): `## job=<j> seed=<s>` headings, `- field: value` bullets, `- per-component:` sub-bullets. [VERIFIED: read 80 lines of `sim-rs/output/phase-2/eip1559-robustness-20260514-160045/metrics_comparison.txt`.]
**How to avoid:** **Don't parse `metrics_comparison.txt`.** Every field it contains is also in `run_summary.json` (the same `RunSummary` struct populates both writers). Read `run_summary.json` and compute aggregates in Python. Leave `metrics_comparison.txt` for human reading; it's not part of the data pipeline.
**Warning signs:** Regex-heavy parsing in `build.py`; "stripped the colon" type bugs.

### Pitfall 5: `latency_blocks_observations` is a list per component, not a scalar

**What goes wrong:** Headline-metrics task expects `latency_blocks_mean: f64` per component but the JSON carries `latency_blocks_observations: [f64]` with hundreds of entries.
**Why it happens:** `ComponentSummary` stores raw observations and exposes `latency_blocks_mean()` as a method in Rust; the JSON serialisation drops the method and persists the underlying `Vec`. [VERIFIED: `collector.rs` line 56, observed in `run_summary.json` at offset 16+.]
**How to avoid:** In `build.py`, compute `sum(obs) / len(obs)` per component in Python (matching the Rust accessor). Per-lane breakdown: there's **no per-lane latency field** — `ComponentSummary` mixes priority+standard observations into one list per component. To get latency-by-lane for VIZ-03 either:
  - (a) report latency per **component** (3 numbers for the typical 3-component demand profile), labelled by the component's typical posted lane (the `priority_included` vs `standard_included` ratio shows which lane dominates), or
  - (b) accept that the field set is "latency per component" and rename the UI label accordingly.
  Recommend (b) for v1 honesty. See Open Questions #2.
**Warning signs:** A latency field whose value is `[0.9, 0.9, 0.9, 4.7, ...]` instead of a single float.

### Pitfall 6: D-06's `.gitignore` requirement is already satisfied

**What goes wrong:** Plan adds `/output/viz/` to `sim-rs/.gitignore`, but `sim-rs/.gitignore` already has `/output` (catches everything under `sim-rs/output/`, including `viz/`). The new line is dead.
**Why it happens:** D-06 reads "must be added" as imperative; the writer didn't check the existing `.gitignore`. [VERIFIED: `cat sim-rs/.gitignore` shows `/output` on line 2.]
**How to avoid:** **Skip the `.gitignore` edit unless adding a more-specific allowlist.** If anyone wants `sim-rs/output/viz/data/` excluded from a future `output/` un-ignore, that's a separate edit. For v1, document in the build-script README that `sim-rs/output/viz/` is gitignored transitively via `/output`, and that's sufficient. No-op task.
**Warning signs:** A line `output/viz/` or `viz/` appearing in either `.gitignore` despite the parent rule.

### Pitfall 7: Single-threaded `http.server` blocks build-then-serve

**What goes wrong:** If `--serve` is `http.server.HTTPServer.serve_forever()` called *after* the build returns, fine. If it's a background thread spawned mid-build and the build crashes, the server is orphaned.
**Why it happens:** Misordering. The simplest pattern is **strictly sequential**: build → print URL → call `serve_forever()` synchronously. The user Ctrl-Cs to stop.
**How to avoid:** Use `http.server.ThreadingHTTPServer` if you want graceful Ctrl-C handling on every request; otherwise plain `HTTPServer` is fine. Pattern:

```python
from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler
import functools, os

def serve(output_dir, port):
    handler = functools.partial(SimpleHTTPRequestHandler, directory=str(output_dir))
    with ThreadingHTTPServer(("127.0.0.1", port), handler) as httpd:
        print(f"Serving {output_dir} at http://127.0.0.1:{port}/  (Ctrl-C to stop)")
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            pass
```

`directory=` keyword is Python 3.7+ on `SimpleHTTPRequestHandler`. Already met. [CITED: docs.python.org/3/library/http.server.html]
**Warning signs:** "Address already in use" on a re-run; port reuse needs `httpd.allow_reuse_address = True` or just bumping the port.

### Pitfall 8: Phase-3 suites have a different shape than phase-2

**What goes wrong:** Build skips phase-3 suites silently or crashes on a slightly-different file layout.
**Why it happens:** `sim-rs/output/phase-3/` exists (read end-to-end in `analyse-phase-3.py`) with the same `manifest.json` + `<job>/<seed>/run_summary.json` layout — but the field set in `run_summary.json` may differ over time (e.g., phase-3 doesn't necessarily emit a `time_series.csv` per (job, seed)). [VERIFIED partially: `sim-rs/output/phase-3/canonical-variance-20260518-084846/` exists; field-by-field inventory would require a sweep.]
**How to avoid:** Per D-21 skip-and-warn: in `build.py`, if `time_series.csv` is missing for a (job, seed), build the per-(job, seed) JSON with empty `time_series: []` and emit a warning. The browser's job-detail view shows the headline strip but an "(no time-series available)" placeholder for the chart panes. Same defensive default for unexpected fields in `run_summary.json` — use `.get(key, default)` everywhere.
**Warning signs:** Phase-3 suites appear in the index but their detail pages are blank or 404. (Either is a soft failure with skip-and-warn; not a build crash.)

## Runtime State Inventory

Phase 1 is **greenfield** (new build script, new bundle directory; no rename, refactor, or migration). No runtime-state inventory required.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — this is read-only consumption of existing artefacts | — |
| Live service config | None | — |
| OS-registered state | None | — |
| Secrets and env vars | None | — |
| Build artifacts | None — only new files under `sim-rs/scripts/viz/` (committed) and `sim-rs/output/viz/` (gitignored) | — |

## Code Examples

### Walk for manifests and parse with skip-and-warn

```python
# Source: sim-rs/scripts/analyse-phase-3.py (style reference) + new code
import json
from pathlib import Path

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
        suites.append({
            "id": suite_id,
            "dir": manifest_path.parent,
            "name": manifest.get("suite-name", manifest_path.parent.name),
            "started_at": manifest.get("started-at-utc", ""),
            "jobs": manifest.get("jobs", {}),
        })
    return suites
```

### Read run_summary.json and time_series.csv for one (job, seed)

```python
# Source: combined from sim-rs/scripts/analyse-phase-3.py and the verified RunSummary schema
import csv
import json
from pathlib import Path

def load_seed(seed_dir: Path, warnings: list):
    rs_path = seed_dir / "run_summary.json"
    ts_path = seed_dir / "time_series.csv"
    if not rs_path.exists():
        warnings.append(f"missing run_summary.json at {seed_dir}")
        return None
    with open(rs_path) as f:
        rs = json.load(f)
    out = {
        "retained_value": rs["priority_retained_value_total"] + rs["standard_retained_value_total"],
        "priority_retained_value": rs["priority_retained_value_total"],
        "standard_retained_value": rs["standard_retained_value_total"],
        "total_txs_submitted": rs["total_txs_submitted"],
        "total_txs_included": rs["total_txs_included"],
        "total_fees_paid_lovelace": rs["total_fees_paid_lovelace"],
        "total_refund_lovelace": rs["total_refund_lovelace"],
        "pricing_event_stream_sha256": rs.get("pricing_event_stream_sha256", ""),
        "components": [
            {
                "index": c["component_index"],
                "txs_submitted": c["txs_submitted"],
                "txs_included": c["txs_included"],
                "bytes_included": c["bytes_included"],
                "retained_value": c["retained_value_total"],
                "net_utility": c["net_utility_total"],
                "latency_blocks_mean": (
                    sum(c["latency_blocks_observations"]) / len(c["latency_blocks_observations"])
                    if c["latency_blocks_observations"] else 0.0
                ),
                "priority_included": c["priority_included"],
                "standard_included": c["standard_included"],
            }
            for c in rs.get("components", [])
        ],
    }
    if ts_path.exists():
        out["time_series"] = list(_read_time_series_long(ts_path))
        out["peak_mempool_bytes"] = max(
            (r["value"] for r in out["time_series"]
             if r["metric"] == "mempool_bytes" and r["lane"] == "total"),
            default=0,
        )
    else:
        out["time_series"] = []
        out["peak_mempool_bytes"] = None
        warnings.append(f"missing time_series.csv at {seed_dir}")
    return out


def _read_time_series_long(csv_path: Path):
    LANE_FIELDS = [
        ("c_priority", "priority", "quote_per_byte"),
        ("c_standard", "standard", "quote_per_byte"),
        ("mempool_bytes_total", "total", "mempool_bytes"),
        ("mempool_bytes_priority", "priority", "mempool_bytes"),
        ("mempool_bytes_standard", "standard", "mempool_bytes"),
    ]
    with open(csv_path, newline="") as f:
        for row in csv.DictReader(f):
            slot = int(row["slot"])
            for col, lane, metric in LANE_FIELDS:
                yield {"slot": slot, "lane": lane, "metric": metric, "value": int(row[col])}
```

### Browser-side Plot rendering for VIZ-04

```javascript
// Source: derived from observablehq.com/plot/getting-started + observablehq.com/plot/marks/line
import * as Plot from "https://cdn.jsdelivr.net/npm/@observablehq/plot@0.6/+esm";

function renderQuoteChart(records, container) {
  const filtered = records.filter(r => r.metric === "quote_per_byte");
  const chart = Plot.plot({
    width: 800, height: 240,
    color: { legend: true },
    x: { label: "slot" },
    y: { label: "controller quote (lovelace/byte)" },
    marks: [
      Plot.ruleY([0]),
      Plot.line(filtered, { x: "slot", y: "value", stroke: "lane" }),
    ],
  });
  container.replaceChildren(chart);
}
```

### Browser-side cross-seed overlay for VIZ-05

```javascript
// Source: derived from observablehq.com/@observablehq/plot-multi-series-line-chart-interactive-tips
function renderCrossSeedOverlay(seedRecords, container) {
  // seedRecords: [{seed, records: [{slot, value}, ...]}, ...]
  const flat = seedRecords.flatMap(({seed, records}) =>
    records.map(r => ({...r, seed: String(seed)})));
  const chart = Plot.plot({
    color: { legend: true, type: "ordinal" },
    marks: [Plot.line(flat, {x: "slot", y: "value", stroke: "seed"})],
  });
  container.replaceChildren(chart);
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `sim-rs/output/tiered_plot.html` with Plotly.js 2.30 CDN | Observable Plot 0.6.17 via ESM | New for v1 | Smaller bundle (~150 KB vs ~3.5 MB), modern declarative API, d3 auto-bundled. |
| Per-suite ad-hoc HTML | Reusable build pipeline driven by `manifest.json` | New for v1 | Every suite gets the same UI for free; no per-suite hand-editing. |
| Notebook-based inspection (Jupyter) | Browser-based static site | New for v1 | No Python kernel needed to view results; share by sharing a directory. |

**Deprecated / outdated:**
- `tiered_plot.html` — keep on disk as a reference but do not import or extend. (Stated explicitly in CONTEXT.md canonical-refs.)

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Phase-3 suites use the same `manifest.json` + `<job>/<seed>/run_summary.json` layout as phase-2 | Pitfall #8 | Build skips phase-3 silently (acceptable per D-21) or shows partial data. Confirm with a quick `ls` of one phase-3 suite during the planning step; recommend the planner adds a "phase-3 spot-check" sub-task. |
| A2 | Latency-by-lane in VIZ-03 means "latency per component" given the field set | Pitfall #5, Open Questions #2 | User expected per-lane numbers and didn't get them. Lock in via a question in the discuss-phase if revisited. |
| A3 | A single committed/vendored `plot.min.js` is preferable to the CDN for the "local-first" promise | Environment Availability | If the dev has no internet on first run, CDN fails; vendored Plot is the safety net. Trade ~150 KB of committed assets for offline guarantee. |
| A4 | The Markdown-ish `metrics_comparison.txt` is human-readable only and not part of the data pipeline | Pitfall #4 | If someone needs a field that exists only there, the build won't see it. Cross-checked: every field in the .txt is also in `run_summary.json`. |
| A5 | Two-tier hash routing (`#/suite/<id>`, `#/job/<id>/<job>/<seed>`) is sufficient — no query-string filters | Pattern 2 | Future sort/filter state isn't bookmarkable. Acceptable for MVP; revisit in v1.1. |

If A1–A5 turn out wrong, none cause data loss or simulator-side regression. They affect only the display layer; iteration cost is low.

## Open Questions (RESOLVED)

1. **Parallelism column in the VIZ-01 suite list** — `manifest.json` does not carry a `parallelism` field. The `experiment-suite` runner uses parallelism at run-time but persists no record. Options:
   - Omit the column. The user-facing VIZ-01 acceptance criterion says "name, run date, parallelism, job count" — dropping parallelism mildly contradicts it.
   - Derive a proxy: max overlap of (started-at-utc, completed-at-utc) intervals across (job, seed) entries in `manifest.json`. Surfaces effective concurrency at the time of the run.
   - Add a column to `Manifest` in a follow-on simulator change. Out of scope for phase 1.
   - **Recommendation:** Compute the overlap-based proxy in `build.py` and label it "max-concurrent-jobs" in the UI. Two-line algorithm; matches VIZ-01's intent.
   - **RESOLVED:** Derived `max_concurrent_jobs` proxy implemented in Plan 02 (`_max_concurrent_jobs`).

2. **Latency-by-lane vs latency-by-component in VIZ-03** — the `ComponentSummary.latency_blocks_observations` field mixes both lanes' observations into one list per component. There is no clean per-lane latency aggregate at the (job, seed) level today. Options:
   - Report latency per component, label appropriately, drop the "by lane" wording from the UI.
   - Split observations by `priority_included` / `standard_included` ratio per component (proxy, not exact).
   - Modify the metrics collector to emit per-(component, lane) latency separately — out of scope for phase 1.
   - **Recommendation:** Per-component latency, UI label "Latency by demand component (blocks)" with each component's typical lane noted (priority-dominant vs standard-dominant from `priority_included` / `standard_included` counts).
   - **RESOLVED:** UI label "Latency by demand component (blocks)" wired in Plans 03/05/06; grep gates in 03/05/06.

3. **Vendor Plot or use CDN?** D-19 leaves the choice to the planner. The local-first promise in PROJECT.md and the VIZ-06 "without internet for the simulator developer" wording argue for vendoring. ~150 KB committed asset. **Recommendation:** vendor as `sim-rs/scripts/viz/static/plot.min.js`, document the version in a comment, refresh annually.
   - **RESOLVED:** Vendored `static/plot.min.js` per Plan 03 Task 1; offline-friendly per VIZ-06.

4. **Sort key for the suite list** — D-18 defaults to "most-recent run date first." `manifest.json` has `started-at-utc` for the suite and per-(job, seed) `started-at-utc` / `completed-at-utc`. Use the suite-level `started-at-utc`. Confirmed sufficient; no follow-up needed.
   - **RESOLVED:** `started-at-utc` desc default, implemented in Plan 05 `renderHome`.

5. **Index.json size at full scale** — 249 manifests today. If every suite contributes ~200 bytes of metadata (name, run date, job/seed counts), `index.json` is ~50 KB. Fine. No streaming or paging needed for v1.
   - **RESOLVED:** ~50 KB for 249 suites; no streaming needed; documented in Plan 06 README.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Python stdlib `unittest` (no pytest install per D-08) |
| Config file | None — `python -m unittest discover sim-rs/scripts/viz/tests` from the repo root |
| Quick run command | `python -m unittest discover sim-rs/scripts/viz/tests` |
| Full suite command | `python -m unittest discover sim-rs/scripts/viz/tests -v` |

**Rationale:** D-08 forbids `requirements.txt` / virtualenvs / non-stdlib deps. `unittest` is stdlib. The existing `sim-rs/scripts/*.py` files are not currently tested at all (manual scripts), so this is a marginal increase in test infrastructure.

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| VIZ-01 | Build emits `index.json` listing every discovered manifest | unit | `python -m unittest sim-rs/scripts/viz/tests/test_build_smoke.py::IndexBuildTest::test_index_lists_all_manifests` | ❌ Wave 0 |
| VIZ-02 | Per-suite JSON contains all (job, seed) entries from `manifest.json` | unit | `python -m unittest sim-rs/scripts/viz/tests/test_build_smoke.py::SuiteJsonTest::test_jobs_match_manifest` | ❌ Wave 0 |
| VIZ-03 | Per-(job, seed) JSON contains the five headline metric fields | unit | `python -m unittest sim-rs/scripts/viz/tests/test_build_smoke.py::SeedJsonTest::test_headline_fields_present` | ❌ Wave 0 |
| VIZ-04 | Per-(job, seed) JSON contains long-form time-series records | unit | `python -m unittest sim-rs/scripts/viz/tests/test_build_smoke.py::SeedJsonTest::test_time_series_long_form` | ❌ Wave 0 |
| VIZ-05 | Suite JSON exposes per-job seed grouping for cross-seed overlay | unit | `python -m unittest sim-rs/scripts/viz/tests/test_build_smoke.py::SuiteJsonTest::test_seed_grouping_present` | ❌ Wave 0 |
| VIZ-06 | `python sim-rs/scripts/viz/build.py --serve` serves HTTP 200 on `/`, `/data/index.json`, and at least one Tier-2 / Tier-3 JSON | integration (subprocess + urllib) | `python -m unittest sim-rs/scripts/viz/tests/test_serve_smoke.py::ServeSmokeTest` | ❌ Wave 0 |
| All | Skip-and-warn on a synthetic malformed manifest | unit | `python -m unittest sim-rs/scripts/viz/tests/test_build_smoke.py::ErrorHandlingTest::test_malformed_manifest_skipped_with_warning` | ❌ Wave 0 |

**Manual-only (not automated):**
- **Visual correctness of charts.** Open `http://localhost:8000/`, click into a phase-2 suite, click into a (job, seed), confirm three time-series panes render and show lane-coloured lines. No automated visual regression for v1 — Plot output is well-defined enough that the unit tests on the underlying JSON are the load-bearing gate; visual is for spot-check only.
- **Cross-suite spot-check.** Open a phase-3 suite (likely missing `time_series.csv`) and confirm the empty-state placeholder renders rather than a JavaScript error.

### Sampling Rate

- **Per task commit:** `python -m unittest discover sim-rs/scripts/viz/tests` (all tests; sub-second runtime expected since fixtures are tiny synthetic manifests).
- **Per wave merge:** Same — there's no slower "full suite" tier.
- **Phase gate:** Manual visual spot-check on the real `sim-rs/output/phase-2/` tree before `/gsd-verify-work`, plus the automated unit suite green.

### Wave 0 Gaps

- [ ] `sim-rs/scripts/viz/tests/__init__.py` — empty package marker
- [ ] `sim-rs/scripts/viz/tests/test_build_smoke.py` — synthetic-fixture tests for the build pipeline (unit-level; create a `tempfile.TemporaryDirectory` with a minimal fake `manifest.json` + fake `run_summary.json` + fake `time_series.csv`, run the build, assert outputs)
- [ ] `sim-rs/scripts/viz/tests/test_serve_smoke.py` — spawn `build.py --serve` in a subprocess on a random high port, `urllib.request.urlopen` against the three URL shapes, assert HTTP 200 + non-empty JSON, terminate subprocess in `tearDown`
- [ ] `sim-rs/scripts/viz/tests/fixtures/` — checked-in synthetic mini-suite (one suite, one job, two seeds, hand-written `manifest.json` + `run_summary.json` + `time_series.csv`); ~5 KB total, lets the unit tests run without needing the real `sim-rs/output/` populated

*(Existing test infrastructure (Rust `cargo test`) is not relevant — phase 1 changes no Rust.)*

## Security Domain

The viz site is a local-first developer tool (D-04 / D-23 / D-24). It serves the dev's own files on `127.0.0.1` and reads only artefacts already on disk. The user is the sole audience.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No auth surface — local-only, no users. |
| V3 Session Management | no | No sessions. |
| V4 Access Control | no | Single-user local tool. |
| V5 Input Validation | partial | `--include` / `--exclude` globs and `--output` path. Use `pathlib.Path` resolution; reject `..` traversal outside the source dir if the build crosses into user-supplied input territory. Low risk: the dev runs their own script against their own filesystem. |
| V6 Cryptography | no | None used. |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| HTML injection via suite name displayed in browser | Tampering | Escape suite names / job names in HTML output. Use `html.escape()` (stdlib) at build time, or set them as text content in JS (not `innerHTML`). |
| Path traversal via crafted `--output` argument | Tampering | Resolve via `Path(args.output).resolve()`; refuse paths outside the repo root if paranoia is warranted. Lowest priority — the dev is the attacker, and they own the machine. |
| Serving on `0.0.0.0` instead of `127.0.0.1` | Information Disclosure | Bind `http.server` to `127.0.0.1` explicitly, not the default `0.0.0.0`. Stops accidental LAN exposure of `sim-rs/output/` (which may include experimental results not yet ready for sharing). |

**Recommendation:** Bind to `127.0.0.1` (not `0.0.0.0`), `html.escape()` all interpolated suite/job/seed names in any Python-emitted HTML, and use `textContent` (never `innerHTML`) when JS injects suite names into the DOM. The rest of ASVS does not apply.

## Sources

### Primary (HIGH confidence — verified by direct inspection)

- `sim-rs/sim-cli/src/runner.rs` lines 44–74 — `Manifest`, `JobEntry`, `JobStatus` schemas with `#[serde(rename_all = "kebab-case")]`
- `sim-rs/sim-cli/src/metrics/collector.rs` lines 11–157 — `TimeSeriesRow`, `ComponentSummary`, `RunSummary` schemas
- `sim-rs/sim-cli/src/metrics/time_series.rs` lines 1–50 — `time_series.csv` 15-column header pinned
- `sim-rs/sim-cli/src/metrics/comparison.rs` lines 16–106 — `metrics_comparison.txt` is prose-Markdown, no `*comparison.csv` emitted by phase-2
- `sim-rs/output/phase-2/eip1559-robustness-20260514-160045/manifest.json` lines 1–106 — verified kebab-case schema in real artefact
- `sim-rs/output/phase-2/eip1559-robustness-20260514-160045/d8_target0.5_window32/1/run_summary.json` — verified 23 top-level keys via `python3 json.load`
- `sim-rs/output/phase-2/eip1559-robustness-20260514-160045/d8_target0.5_window32/1/time_series.csv` lines 1–3 — verified header and example rows
- `sim-rs/output/phase-2/eip1559-robustness-20260514-160045/metrics_comparison.txt` lines 1–80 — verified prose-Markdown format
- `sim-rs/scripts/analyse-phase-3.py` lines 1–310 — style reference, stdlib-only, `pathlib`, `argparse`, `json`
- `sim-rs/scripts/generate-realistic-100-topology.py` lines 1–80 — argparse + module docstring style
- `sim-rs/.gitignore` — confirms `/output` already covers `output/viz/`
- `sim-rs/output/analysis/priority_only_fast_path_overall_comparison.csv` — historical file location proves the CSV is not in `phase-2/`
- `CLAUDE.md` "Conventions / gotchas" — abbreviation expansion, no f64 in simulation state (irrelevant here but noted)
- `https://cdn.jsdelivr.net/npm/@observablehq/plot/package.json` — version 0.6.17 as of 2026-05-20

### Secondary (MEDIUM confidence — official docs)

- `observablehq.com/plot/getting-started` — ESM + CDN load pattern, d3 auto-bundling
- `observablehq.com/plot/marks/line` — `stroke` channel groups data into series
- `observablehq.com/plot/features/legends` — `color: { legend: true }` for categorical legends
- `docs.python.org/3/library/http.server.html` — `SimpleHTTPRequestHandler(directory=...)` keyword
- `docs.python.org/3/library/csv.html` — `csv.DictReader`

### Tertiary (LOW confidence — informational only)

- `talk.observablehq.com/t/legends-on-multi-line-charts/10305` — community discussion confirming legend caveats
- `github.com/observablehq/plot/discussions/2007` — direct-colour-spec legend caveat noted

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — Plot version verified via jsDelivr; Python is stdlib; existing scripts confirm idioms.
- Architecture: HIGH — three-view SPA with hash routing and pre-shaped JSON is a well-understood pattern with no novel choices.
- Pitfalls: HIGH — all eight pitfalls grounded in directly-verified file contents, not training data.
- Data schema: HIGH — every field referenced is read from a real artefact on disk or a Rust struct definition.
- Validation: MEDIUM — unit/integration test layout is straightforward stdlib `unittest`; no precedent in this repo for a viz-test pattern, but the pattern is conventional.
- Security: HIGH — local-only tool; threat surface is small and well-characterised.

**Research date:** 2026-05-20
**Valid until:** 2026-06-20 (Observable Plot patch releases land regularly; pin via `@0.6` major.minor in URL — patch drift is non-breaking)

## RESEARCH COMPLETE

**Phase:** 01 - Viz Site MVP
**Confidence:** HIGH

### Key Findings

1. **The on-disk schema is fully verified by direct inspection.** `manifest.json` is kebab-case, `run_summary.json` is snake_case, `time_series.csv` has a pinned 15-column header. The planner can write field-exact accessors without guessing.
2. **The `priority_only_fast_path_overall_comparison.csv` filename cited in CONTEXT.md / REQUIREMENTS.md does NOT exist in the phase-2 suite tree.** It is a historical artefact under `sim-rs/output/analysis/`. The phase-2 metrics writer (`comparison.rs`) emits only `metrics_comparison.txt` (Markdown-ish prose). VIZ-05's "static aggregate" panel will be conditional or absent for phase-2 suites; cross-seed overlay (D-15) drives VIZ-05 in practice.
3. **`sim-rs/output/` is already in `sim-rs/.gitignore` as `/output`.** D-06's "must add `.gitignore` entry" is satisfied transitively. No `.gitignore` edit needed.
4. **Observable Plot 0.6.17 via ESM (`+esm` form on jsDelivr) auto-bundles d3** — no separate d3 script tag. For local-first guarantee under PROJECT.md, recommend vendoring `plot.min.js` (~150 KB) under `sim-rs/scripts/viz/static/`.
5. **`latency_blocks_observations` is per-component, not per-lane.** VIZ-03's "latency by lane" wording is achievable only as a proxy. Recommend renaming the UI label to "latency by demand component" and noting each component's typical lane via `priority_included` / `standard_included` ratios. (Open Question #2.)

### File Created

`/home/will/git/arc-tiered-pricing/.planning/workstreams/viz-website/phases/01-viz-site-mvp/01-RESEARCH.md`

### Confidence Assessment

| Area | Level | Reason |
|------|-------|--------|
| Standard Stack | HIGH | Plot version verified via jsDelivr package.json; Python is stdlib. |
| Architecture | HIGH | Three-view static SPA with hash routing — standard pattern; no novel choices. |
| Data Schema | HIGH | All five files (manifest.json, run_summary.json, time_series.csv, metrics_comparison.txt, analysis-historical CSV) inspected directly. |
| Pitfalls | HIGH | All eight grounded in verified artefacts, not training data. |
| Validation | MEDIUM | `unittest` is stdlib but no precedent in repo for testing Python viz scripts; pattern is conventional. |
| Security | HIGH | Local-only developer tool; threat surface small and characterised. |

### Open Questions

1. Suite-list "parallelism" column — `manifest.json` does not carry the field. Recommend a derived overlap-of-intervals proxy.
2. Latency-by-lane vs latency-by-component — the field set supports only per-component; recommend the UI label match.
3. Vendor Plot vs CDN — recommend vendoring for the local-first guarantee.
4. None blocking — all five Open Questions have a clear recommendation.

### Ready for Planning

Research complete. Planner can now create PLAN.md files. The plan should account for the seven verified data-layer realities, the five Claude's-discretion choices recommended here (vendor Plot, use overlap-proxy for parallelism, drop the `.gitignore` no-op, label latency per-component, hash-routing without query strings), and the eight pitfalls — most of which become small Wave-0 / per-task verifications rather than risks.
