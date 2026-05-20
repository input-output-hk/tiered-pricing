---
workstream: viz-website
milestone: v1.0
---

# Roadmap: Visualisation Website (v1)

## Overview

A single-phase milestone that stands up the viz website's first usable version. Phase 1 builds the MVP against the existing [`sim-rs/output/`](../../../sim-rs/output/) tree: browsing, per-job metrics, per-job time series, cross-job comparison, local-first hosting. Tech-stack and layout decisions are deliberately deferred to the phase-1 discussion; this roadmap captures scope and success criteria only.

If v1 lands cleanly and additional iteration is wanted (paired-bootstrap CI bands, event-stream drill-down, public hosting, etc.), a phase 2 will be added then — not now.

Granularity: minimal. One phase, six v1 requirements, 100% coverage.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, ...): planned workstream work
- Decimal phases (e.g. 1.1): urgent insertions

- [ ] **Phase 1: Viz Site MVP** — Stand up the first usable version of the visualisation site against `sim-rs/output/`

## Phase Details

### Phase 1: Viz Site MVP
**Goal**: A local-first visualisation site exists that browses the suite runs under [`sim-rs/output/`](../../../sim-rs/output/), renders headline metrics and time series for a selected (job, seed), supports comparison across runs, and can be brought up with a single documented command. Tech-stack, layout, location-in-repo, and chart-library decisions are made during the phase discussion and locked in CONTEXT.md before planning.
**Depends on**: Nothing (first phase in the workstream)
**Requirements**: VIZ-01, VIZ-02, VIZ-03, VIZ-04, VIZ-05, VIZ-06
**Success Criteria** (what must be TRUE):
  1. The site lists the suite runs currently in `sim-rs/output/` and lets the user pick one without crawling directories by hand (VIZ-01)
  2. Drilling into a suite shows its `manifest.json` summary and per-job / per-seed inventory (VIZ-02)
  3. The headline metrics (retained value, net utility, retained-value ratio, latency-by-lane, mempool depth) for a selected (job, seed) are rendered in a readable layout (VIZ-03)
  4. Time-series for a selected (job, seed) — controller quotes per lane, mempool size, derived_quote per block — render as multi-line charts with lane colouring (VIZ-04)
  5. Suite-level comparison aggregates (e.g. `priority_only_fast_path_overall_comparison.csv`) render as charts or tables (VIZ-05)
  6. A single documented command gets a fresh dev environment to a viewable viz site against the current `sim-rs/output/` tree (VIZ-06)

**Canonical refs**:
- [`sim-rs/output/`](../../../sim-rs/output/) — source data tree
- [`sim-rs/output/phase-2/`](../../../sim-rs/output/phase-2/) — the phase-2 suites' run directories (~100+ as of workstream creation)
- [`sim-rs/sim-cli/src/metrics/`](../../../sim-rs/sim-cli/src/metrics/) — the writers that produce the metrics the site reads (collector.rs, comparison.rs, time_series.rs, paired_bootstrap.rs)
- [`sim-rs/output/tiered_plot.html`](../../../sim-rs/output/tiered_plot.html) — earlier exploratory single-file viz (prior-art; not load-bearing)

**Plans:** 6 plans across 4 waves.

Plans:
- [x] `phases/01-viz-site-mvp/01-01-PLAN.md` — Wave 1: Test harness (fixtures + failing test scaffolds for every Pitfall 1–8 + every VIZ-NN)
- [x] `phases/01-viz-site-mvp/01-02-PLAN.md` — Wave 2: Ingest module (`build.py` core — discover_suites, load_seed, three-tier JSON emission, kebab/snake split, latency-list-to-mean, aggregates:null)
- [x] `phases/01-viz-site-mvp/01-03-PLAN.md` — Wave 2: Static bundle skeleton (`index.html`, `style.css`, `main.js` router + stubs, vendored Observable Plot 0.6.x) — parallel with 01-02
- [x] `phases/01-viz-site-mvp/01-04-PLAN.md` — Wave 3: Build entry-point (--serve / --port, copy_static_assets, ThreadingHTTPServer bound to 127.0.0.1, serve smoke test)
- [ ] `phases/01-viz-site-mvp/01-05-PLAN.md` — Wave 3: Browser views (real renderHome/renderSuite/renderJob with three Plot chart panes + cross-seed overlay) — parallel with 01-04
- [ ] `phases/01-viz-site-mvp/01-06-PLAN.md` — Wave 4: README + CLAUDE.md crumb + end-to-end checkpoint against the live `sim-rs/output/` tree

**Wave structure** (no `files_modified` overlap within a wave):
- Wave 1: 01-01 alone — establishes the test harness.
- Wave 2: 01-02 (Python build.py) + 01-03 (static/* HTML/CSS/JS) — parallel; touch disjoint files.
- Wave 3: 01-04 (build.py serve helper + tests/test_serve_smoke.py) + 01-05 (static/main.js views) — parallel; touch disjoint files.
- Wave 4: 01-06 alone — docs + final E2E check.
