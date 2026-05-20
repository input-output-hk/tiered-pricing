---
workstream: viz-website
milestone: v1.0
---

# REQUIREMENTS.md — Visualisation Website (v1)

The v1 scope. Each requirement is capability-level — it captures WHAT the site does, not HOW. Implementation choices (static vs dynamic, tech stack, chart library, layout) are deliberately deferred to [`/gsd-discuss-phase`](../phases/) and the plan that follows.

REQ-ID prefix: **VIZ-NN** — Visualisation Website work.

---

## v1 Requirements

### Browsing

- [x] **VIZ-01** — A user can open the site locally and see a navigable list of suite runs that exist under [`sim-rs/output/`](../../../sim-rs/output/), with enough metadata per row (suite name, run date, parallelism, job count) to pick the right one. Acceptance: opening the site lists the existing ~100 suite runs without crawling output directories by hand.

- [x] **VIZ-02** — A user can drill into a single suite run and see its `manifest.json` summary, the suite's job list, and per-job seed inventory in a structured view. Acceptance: from the list in VIZ-01, one click reaches a "this suite ran X jobs at Y seeds, here is the manifest" view.

### Per-job visualisation

- [x] **VIZ-03** — A user can view the headline metrics for a selected (job, seed) pair: retained value, net utility, retained-value ratio, latency-by-lane, mempool depth. Acceptance: the same numbers that appear in the `metrics_comparison.txt` / per-job CSVs are rendered in a readable layout.

- [x] **VIZ-04** — A user can view time-series plots for a selected (job, seed): controller `quote_per_byte` per lane, mempool size, `derived_quote` per block. Acceptance: opening the `time_series.csv` for a job renders as a multi-line chart with lane colouring rather than requiring a notebook.

### Comparison

- [x] **VIZ-05** — A user can compare headline metrics across (job, seed) pairs within a suite, or across suites. Acceptance: the suite-level aggregate CSVs (e.g. `priority_only_fast_path_overall_comparison.csv`) are rendered as charts or tables alongside the relevant jobs.

### Local-first hosting

- [ ] **VIZ-06** — The site runs locally on the developer's machine. Whether that means a static bundle, a build script, or a local dev server is a discuss-phase decision; the requirement is that one documented command produces a browsable site. Acceptance: a single command (in `CLAUDE.md` or workstream README) gets a fresh dev environment to a viewable viz site against the current `sim-rs/output/` tree.

## Future Requirements

Deferred to later phases or workstream iteration:

- Paired-bootstrap CI band / BCa visualisation (depends on VIZ-03/04 patterns landing first).
- Event-stream drill-down (TXIncluded / TXEvictedQuoteDrift timelines) — high data volume; debugging-oriented not analysis-oriented.
- Public hosting / GitHub Pages deployment — only after the local-first path proves out.
- Direct integration with the experiment-suite runner (e.g. live-updating during a run).

## Out of Scope (v1)

- Changes to the simulator's output format. The viz site adapts to what's written, not the other way around.
- Multi-user, auth, server-side state. Local-first only.
- CIP-author-facing polish. The immediate consumer is the simulator developer; CIP-facing presentation is a possible future iteration once v1 lands.
- Anything outside `sim-rs/output/` (e.g. `.planning/realism-tests/` results) — listed as a future iteration if useful.

## Traceability

Filled by ROADMAP.md.

| REQ-ID | Phase |
|--------|-------|
| VIZ-01 | Phase 1 |
| VIZ-02 | Phase 1 |
| VIZ-03 | Phase 1 |
| VIZ-04 | Phase 1 |
| VIZ-05 | Phase 1 |
| VIZ-06 | Phase 1 |
