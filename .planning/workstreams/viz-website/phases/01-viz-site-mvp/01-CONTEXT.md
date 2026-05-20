# Phase 1: Viz Site MVP - Context

**Gathered:** 2026-05-20
**Status:** Ready for planning

<domain>
## Phase Boundary

A local-first visualisation site that reads the existing `sim-rs/output/` tree and renders headline metrics, time-series, and in-suite cross-seed overlays for the phase-2 simulator's experiment artefacts. Built as a Python-generated static HTML+JS bundle, served locally via HTTP. v1 covers browsing, per-suite drill-down, per-(job, seed) detail, and in-suite cross-seed comparison; cross-suite comparison, paired-bootstrap CI bands, and event-stream drill-down defer to a later phase.

</domain>

<decisions>
## Implementation Decisions

### Site type & rendering

- **D-01:** Rendering model is **build-script → static HTML+JS bundle**. No committed bundle, no local dev server, no off-the-shelf framework (Observable Framework / Streamlit / Evidence.dev). Trade-off: re-build required when new suite data arrives; accepted because it keeps the tooling footprint minimal.
- **D-02:** Build script is **Python**, sitting under `sim-rs/scripts/viz/` alongside the existing `generate-realistic-100-topology.py` and `analyse-phase-3.py`. Not Rust, not Node, not a Cargo binary.
- **D-03:** Chart rendering uses **Observable Plot** loaded in the browser. ~150 KB, declarative API on top of D3, designed for the time-series and small-multiples shape the phase-2 outputs need. Whether Plot is CDN-loaded or vendored under `sim-rs/scripts/viz/static/` is at the planner's discretion (D-19); both are acceptable.
- **D-04:** The site is **served via a local HTTP server** (`python -m http.server` or a built-in helper inside the build script), not opened via `file://`. ES-module-friendly, no CORS gotchas, matches VIZ-06's "single documented command" wording.

### Repo location & build entry-point

- **D-05:** Build script and supporting assets live at **`sim-rs/scripts/viz/`** (`build.py`, `static/` for any vendored JS/CSS, templates inline in `build.py` or in adjacent `.html` files). Matches the existing `sim-rs/scripts/` convention.
- **D-06:** Built bundle output goes to **`sim-rs/output/viz/`**, **gitignored**. Nothing about the rendered bundle is committed; regenerated on demand. The `.gitignore` entry must be added to either the repo root or `sim-rs/.gitignore` as part of the plan.
- **D-07:** Entry-point command is **`python sim-rs/scripts/viz/build.py --serve`**, which builds the bundle then serves it on a local port. Drop `--serve` for build-only. This is the "single documented command" that VIZ-06 calls for; doc it in CLAUDE.md or a new `sim-rs/scripts/viz/README.md`.
- **D-08:** Python dependencies are **stdlib only** (`json`, `argparse`, `pathlib`, `http.server`, `csv`), plus PyYAML if any field requires it. **No `requirements.txt`**, no virtualenv, no Jinja2. HTML templating via f-strings or `string.Template`.

### Data plumbing

- **D-09:** Data is delivered in **three tiers**:
  - `sim-rs/output/viz/data/index.json` — list of every ingested suite with metadata (name, run date, job count, parallelism).
  - `sim-rs/output/viz/data/<suite>.json` — per-suite headline metrics for every (job, seed): `retained_value`, `net_utility`, `retained_value_ratio`, latency-by-lane, peak mempool depth.
  - `sim-rs/output/viz/data/<suite>/<job>-<seed>.json` (or similar) — per-(job, seed) time-series, **fetched on demand** only when the user opens that view.
  Browser fetches `index.json` on page load, fetches per-suite JSON on suite click, fetches per-(job, seed) JSON on job click. Bounded initial load; drill-downs cost one HTTP round-trip each.
- **D-10:** **Default ingestion scope = "every directory containing a `manifest.json`"**, found by walking `sim-rs/output/` recursively. Directories without `manifest.json` are silently skipped. Support `--include <glob>` and `--exclude <glob>` flags on `build.py` for ad-hoc scoping (e.g. `--include 'phase-2/*'` to limit to the phase-2 subtree).
- **D-11:** CSV → JSON conversion happens **at build time in Python**. The browser never parses raw CSV. The JSON shape is whatever the page needs — the planner picks the schema; a 1:1 mirror of the CSV columns is fine for v1.

### First-view priority

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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Workstream-internal

- [`.planning/workstreams/viz-website/PROJECT.md`](../../PROJECT.md) — scope boundary, in/out, core value
- [`.planning/workstreams/viz-website/REQUIREMENTS.md`](../../REQUIREMENTS.md) — VIZ-01 through VIZ-06 acceptance criteria
- [`.planning/workstreams/viz-website/ROADMAP.md`](../../ROADMAP.md) — phase 1 goal and success criteria

### Data sources

- [`sim-rs/output/`](../../../../../sim-rs/output/) — root tree the build crawls; the build script walks this recursively
- [`sim-rs/output/phase-2/`](../../../../../sim-rs/output/phase-2/) — phase-2 milestone's suite runs (~100+ as of 2026-05-20); the bulk of what the site will display
- [`sim-rs/output/tiered_plot.html`](../../../../../sim-rs/output/tiered_plot.html) — prior-art single-file viz; **reference only — do not import or reuse code**; useful for intuition on chart taxonomy

### Metrics writers (informational only — the build reads their CSV outputs, never their Rust code)

- [`sim-rs/sim-cli/src/metrics/collector.rs`](../../../../../sim-rs/sim-cli/src/metrics/collector.rs) — `MetricsCollector`, `RunSummary`, `ComponentSummary` structs; defines headline metric field names
- [`sim-rs/sim-cli/src/metrics/comparison.rs`](../../../../../sim-rs/sim-cli/src/metrics/comparison.rs) — per-suite aggregate CSV emission (`metrics_comparison.txt` and `priority_only_*_comparison.csv` shape)
- [`sim-rs/sim-cli/src/metrics/time_series.rs`](../../../../../sim-rs/sim-cli/src/metrics/time_series.rs) — `time_series.csv` emission per (job, seed)
- [`sim-rs/sim-cli/src/metrics/paired_bootstrap.rs`](../../../../../sim-rs/sim-cli/src/metrics/paired_bootstrap.rs) — paired-bootstrap CSV emission (not displayed in v1; deferred)
- [`sim-rs/sim-cli/src/runner.rs`](../../../../../sim-rs/sim-cli/src/runner.rs) §`Manifest` — `manifest.json` schema for what the build discovers per suite

### Existing Python scripts (style reference for the build script)

- [`sim-rs/scripts/generate-realistic-100-topology.py`](../../../../../sim-rs/scripts/generate-realistic-100-topology.py) — canonical example of stdlib + PyYAML + `argparse` style; matches D-08
- [`sim-rs/scripts/analyse-phase-3.py`](../../../../../sim-rs/scripts/analyse-phase-3.py) — example of a Python script in the repo that consumes phase-2 output artefacts

### Project context

- [`CLAUDE.md`](../../../../../CLAUDE.md) — phase-2 simulator overview, output structure, conventions
- [`.planning/codebase/STACK.md`](../../../../codebase/STACK.md) — confirms Rust + Python (stdlib + PyYAML) + bash, no JS/TS infrastructure
- [`.planning/codebase/STRUCTURE.md`](../../../../codebase/STRUCTURE.md) — confirms `sim-rs/scripts/` convention for one-shot Python scripts

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **Existing Python scripts** at [`sim-rs/scripts/`](../../../../../sim-rs/scripts/) — `generate-realistic-100-topology.py`, `generate-cip-topology.py`, `analyse-phase-3.py`. Stdlib + PyYAML, `argparse`-based CLIs. The build script should mirror their argument-parsing and file-IO style.
- **`sim-rs/output/tiered_plot.html`** — earlier exploratory one-off; reference for the conceptual shape of phase-2 viz (chart types, what the dev wanted to see) but not load-bearing. Don't import code from it.
- **No existing JS/TS infrastructure** — confirmed via STACK.md. The viz site is greenfield on the web side.

### Established Patterns

- **One-shot Python under `sim-rs/scripts/`, stdlib-first.** PyYAML is the only common non-stdlib dep across the existing scripts; matches D-08.
- **CLI ergonomics use `argparse`** with sub-flags (cf. `generate-realistic-100-topology.py --num-pools …`).
- **Output artefacts live under `sim-rs/output/`** (phase-2 metric writers + tiered_plot.html). The viz bundle output (D-06) follows the same convention.

### Integration Points

- The build script **reads `sim-rs/output/<suite>/manifest.json`** (defined by `sim-cli/src/runner.rs` `Manifest`) plus the per-(job, seed) CSVs and any aggregate CSVs the suite emits.
- The build script **does NOT touch any `sim-core` or `sim-cli` Rust source files**. It reads outputs only. No coupling to the Cargo workspace.
- The build script **does NOT integrate with `experiment-suite` runner** (e.g. no live-during-a-run updates). The build is a standalone post-hoc step.

</code_context>

<specifics>
## Specific Ideas

- **Observable Plot** is the chosen chart library specifically because the phase-2 metrics are time-series-heavy (controller quote per lane, mempool size, derived_quote per block) and small-multiples-friendly. Other libraries surveyed: Plotly.js (rejected as too heavy), Chart.js (rejected as too thin for paired-bootstrap-style layered plots in future iterations), Vega-Lite (rejected as too rigorous for an MVP).
- **`python sim-rs/scripts/viz/build.py --serve`** is the literal command the planner should wire up. The flag pattern matches existing scripts' ergonomics.
- **`sim-rs/output/viz/`** is the literal output directory (per D-06). Its `.gitignore` entry needs to be added to either the repo root or `sim-rs/.gitignore` during execution.

</specifics>

<deferred>
## Deferred Ideas

These came up during discussion but are out of scope for phase 1; surface them in a phase-2 / v1.1 conversation when v1 is in:

- **Cross-suite comparison view** — multi-select suites/jobs side-by-side. Most powerful comparison; deferred per D-15 to keep MVP scope tight.
- **Paired-bootstrap CI band visualisation** — `paired_bootstrap.rs` outputs are available but not displayed in v1; relevant once the per-job patterns land.
- **Event-stream drill-down** — `TXIncluded` / `TXEvictedQuoteDrift` timelines per (job, seed). High data volume, debugging-oriented; not analysis-oriented MVP material.
- **Public hosting / GitHub Pages deployment** — only after the local-first path proves out.
- **Direct integration with the `experiment-suite` runner** — live-updating during a run.
- **Visualisation of `.planning/realism-tests/` results** — beyond `sim-rs/output/`; useful future scope.
- **Polished theming / dark mode / responsive layout** — the v1 audience is the simulator dev, not a CIP reader.
- **Schema validation of `manifest.json`** — the build skip-and-warns on malformed inputs (D-21); a stricter validation step is future hardening.

</deferred>

---

*Phase: 01-viz-site-mvp*
*Context gathered: 2026-05-20*
