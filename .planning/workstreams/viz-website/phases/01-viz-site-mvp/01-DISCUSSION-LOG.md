# Phase 1: Viz Site MVP - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in [`01-CONTEXT.md`](01-CONTEXT.md) — this log preserves the alternatives considered.

**Date:** 2026-05-20
**Phase:** 01-viz-site-mvp
**Areas discussed:** Site type & rendering, Repo location & build entry-point, Data plumbing, First-view priority

---

## Site type & rendering

### Q1: What's the rendering model for the v1 viz site?

| Option | Description | Selected |
|--------|-------------|----------|
| Build script → static HTML | Python script reads sim-rs/output/, emits a self-contained static HTML+JS bundle. No server. Re-run on new suites. | ✓ |
| Static HTML committed | Hand-authored HTML/JS files committed; data baked in by re-saving JSON snapshots. Burden on every new suite. | |
| Local dev server | Rust/Node server reads sim-rs/output/ live, serves an SPA. Best interactivity; more moving parts. | |
| Off-the-shelf framework | Observable Framework / Streamlit / Evidence.dev. Ready-made charts; adds a toolchain dep. | |

**User's choice:** Build script → static HTML.

### Q2: What writes the static bundle?

| Option | Description | Selected |
|--------|-------------|----------|
| Python script under sim-rs/scripts/ | Matches existing generate-*-topology.py pattern; stdlib + PyYAML. | ✓ |
| Rust binary in the workspace | Strict typing against metrics structs; Rust-for-HTML/JS templating is awkward. | |
| Node-based static site generator | Astro / 11ty / similar; adds npm/Node toolchain. | |
| Pure static, no build script | Revisit option — hand-author files, hardcode JSON. | |

**User's choice:** Python script.

### Q3: What renders the charts in the browser?

| Option | Description | Selected |
|--------|-------------|----------|
| Observable Plot | ~150 KB, declarative on top of D3, excellent for time series and small multiples. | ✓ |
| Plotly.js | Hover/zoom out of the box, ~3 MB. Better if exploratory interactivity matters. | |
| Chart.js | ~200 KB, simpler taxonomy, less powerful for layered plots. | |
| Vega-Lite | Grammar of graphics; steeper learning curve, ~500 KB+ runtime. | |

**User's choice:** Observable Plot.

### Q4: How does the user open the built site?

| Option | Description | Selected |
|--------|-------------|----------|
| Local HTTP server | One-liner serves the output (python -m http.server or built-in helper); ES-module-friendly. | ✓ |
| Double-click index.html (file://) | No process to manage; requires Plot to be vendored as UMD to dodge file:// CORS. | |
| You decide | Planner picks. | |

**User's choice:** Local HTTP server.

---

## Repo location & build entry-point

### Q1: Where do the build script and its supporting files live?

| Option | Description | Selected |
|--------|-------------|----------|
| sim-rs/scripts/viz/ | Build script and assets sit alongside the existing generate-*-topology scripts. | ✓ |
| Top-level scripts/viz/ | Promotes viz tooling out of sim-rs/; introduces a new top-level dir. | |
| Top-level viz/ | First-class subproject dir; more repo-root churn. | |

**User's choice:** sim-rs/scripts/viz/.

### Q2: Where does the built bundle (HTML+JS+data) go, and is it committed?

| Option | Description | Selected |
|--------|-------------|----------|
| sim-rs/output/viz/, gitignored | Alongside the experiment artefacts the site reads; regenerated on demand. | ✓ |
| sim-rs/scripts/viz/dist/, gitignored | Alongside the script; sim-rs/output/ stays purely simulator artefacts. | |
| sim-rs/scripts/viz/dist/, committed | Rendered HTML/JS in git like the existing tiered_plot.html; bundle churn in history. | |

**User's choice:** sim-rs/output/viz/, gitignored.

### Q3: What does the 'single documented command' look like (VIZ-06)?

| Option | Description | Selected |
|--------|-------------|----------|
| Python script with --serve flag | `python sim-rs/scripts/viz/build.py --serve` builds + serves; drop flag for build-only. | ✓ |
| Shell wrapper | sim-rs/scripts/viz/run.sh wraps build + serve; another shell-vs-python decision. | |
| Makefile target | `make -C sim-rs viz`; introduces a Makefile to a Cargo workspace. | |
| Two separate commands, doc'd together | Honest about two steps; violates 'single command' spirit of VIZ-06. | |

**User's choice:** Python script with --serve flag.

### Q4: Python deps for the build script?

| Option | Description | Selected |
|--------|-------------|----------|
| Stdlib only | json/argparse/http.server/pathlib/csv + maybe PyYAML; no Jinja2, no requirements.txt. | ✓ |
| Allow Jinja2 + tiny requirements.txt | Cleaner templating but introduces first requirements.txt to the repo. | |
| You decide in the plan | Defer to planner; default would be stdlib. | |

**User's choice:** Stdlib only.

---

## Data plumbing

### Q1: How does sim-rs/output/ data reach the page?

| Option | Description | Selected |
|--------|-------------|----------|
| Build emits per-suite JSON + an index | index.json + per-suite JSON; JS fetches on demand. Bounded bundle, one round-trip per drill-down. | ✓ |
| Bake everything into one bundle | All suites embedded in index.html; could grow to multi-MB; full rebuild on every suite addition. | |
| Serve raw CSVs directly | JS fetches and parses CSV client-side; no Python conversion step; CSV parsing fiddly. | |
| Lazy directory listing via fetch | JS uses http.server's directory listing; brittle, hard to display metadata before fetch. | |

**User's choice:** Build emits per-suite JSON + an index.

### Q2: Time series is heavy (multi-MB per (job, seed)). How should it be handled?

| Option | Description | Selected |
|--------|-------------|----------|
| Tiered: index + headlines baked, time series fetched on demand | Three tiers — index.json, per-suite headlines, per-(job, seed) time series fetched only on click. | ✓ |
| Bake everything into per-suite JSON | Single per-suite JSON contains headline + every time series. Opens any suite = download multi-MB. | |
| Convert CSVs only on click (Python http handler) | http.server becomes a tiny WSGI app; cheapest in disk, costliest in runtime complexity. | |
| Bake summary statistics, leave raw CSVs untouched | Aggregated/downsampled time series only; loses fidelity. | |

**User's choice:** Tiered ingestion.

### Q3: Which suites does the build ingest by default?

| Option | Description | Selected |
|--------|-------------|----------|
| Every dir with manifest.json | Walk sim-rs/output/ recursively; --include/--exclude globs for scoping. Resilient. | ✓ |
| Only sim-rs/output/phase-2/ | Cleaner default; ad-hoc dev runs at sim-rs/output/ root aren't visible without --include. | |
| Explicit allowlist in config | sim-rs/scripts/viz/suites.yaml lists suite roots; curated but new suites are invisible until added. | |

**User's choice:** Every dir with manifest.json.

---

## First-view priority

### Q1: What does the user see first when opening the site?

| Option | Description | Selected |
|--------|-------------|----------|
| Suite list / browser | Landing is the VIZ-01 view — every ingested suite with metadata, click a row → drill-down. | ✓ |
| Most-recent-suite dashboard | Landing jumps to most-recently-built suite's overview; faster for the 'just ran X' case. | |
| Cross-suite comparison view | Landing is a comparison dashboard; powerful but hard to make useful without filters. | |
| Empty-state explainer | Landing is a 'what this is, how to navigate' page; extra click for return visitors. | |

**User's choice:** Suite list / browser.

### Q2: Inside a suite, which view is the primary one (loaded first)?

| Option | Description | Selected |
|--------|-------------|----------|
| Per-suite summary table | Sortable per-(job, seed) table with headline metric columns; click row → per-(job, seed) detail. | ✓ |
| Per-suite charts (multi-job overlay) | Multi-job time-series overlays as primary; noisy with 10+ jobs without filters. | |
| Suite comparison aggregates | Aggregate CSVs as charts/tables; job-level detail one click deeper; aggregates don't exist for every suite. | |

**User's choice:** Per-suite summary table.

### Q3: Per-(job, seed) detail — what's the layout?

| Option | Description | Selected |
|--------|-------------|----------|
| Headline + time series stacked | Headline metric strip on top, time-series panes below; single scrollable page. | ✓ |
| Tabbed: Headline / Time series / Comparison | Same content split into tabs; tab UX is more state. | |
| Side-by-side: chart on left, table on right | Notebook-flavored two-pane; needs more horizontal width. | |

**User's choice:** Headline + time series stacked.

### Q4: Cross-job / cross-suite comparison (VIZ-05) — where does it live in v1?

| Option | Description | Selected |
|--------|-------------|----------|
| Cross-seed overlay inside the suite view | Pick a job in the suite view, see all seeds overlaid; suite aggregate CSVs as 'Suite aggregates' section. Cross-suite deferred to v1.1. | ✓ |
| Dedicated cross-suite Compare view | Top-level Compare route with multi-select. Most powerful; multi-select UX is risky for MVP. | |
| Both: in-suite + cross-suite | Most comprehensive; most surface area for an MVP. | |
| Defer all comparison to v1.1 | v1 covers browse + per-job only; leaves a gap against the explicit VIZ-05 requirement. | |

**User's choice:** Cross-seed overlay inside the suite view.

---

## Claude's Discretion

The user explicitly deferred these to the planner / implementer:

- HTML templating style (f-strings vs `string.Template`) within stdlib-only Python — D-16
- Exact column set in the suite drill-down table beyond the named headline metrics — D-17
- Initial sort order on the suite list — D-18
- Observable Plot loading strategy: CDN vs vendored UMD — D-19
- Empty-state copy when sim-rs/output/ has no suites — D-20
- Skip-and-warn vs fail-build for malformed manifest.json — D-21
- Suite-deduplication when directory names collide — D-22
- Visual theming depth — D-23
- Accessibility expectations — D-24

## Deferred Ideas

Surfaced during discussion as future work, not v1 scope:

- Cross-suite comparison view (`Compare` route) — defers because in-suite cross-seed overlay covers v1's VIZ-05.
- Paired-bootstrap CI band visualisation — `paired_bootstrap.rs` outputs are available but not displayed in v1.
- Event-stream drill-down (`TXIncluded` / `TXEvictedQuoteDrift` timelines).
- Public hosting / GitHub Pages deployment.
- Direct integration with the `experiment-suite` runner (live updates during a run).
- Visualisation of `.planning/realism-tests/` results (beyond `sim-rs/output/`).
- Polished theming, dark mode, responsive layout.
- Schema validation of `manifest.json` (vs the current skip-and-warn default).
