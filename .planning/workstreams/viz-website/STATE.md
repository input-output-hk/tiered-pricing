---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
current_phase: 01
current_plan: 6
status: phase-complete
stopped_at: Phase 01 (viz-site-mvp) complete — all six VIZ-NN requirements met, README + CLAUDE.md crumb landed, end-to-end visual verification confirmed
last_updated: "2026-05-20T13:00:00Z"
last_activity: 2026-05-20
progress:
  total_phases: 5
  completed_phases: 5
  total_plans: 17
  completed_plans: 21
  percent: 100
---

# Project State

## Current Position

Phase: 01 (viz-site-mvp) — COMPLETE
Plan: 6 of 6 (final plan in the phase)
**Status:** Phase 01 closed; viz site MVP shipped against `sim-rs/output/`
**Current Phase:** 01
**Last Activity:** 2026-05-20
**Last Activity Description:** Completed Plan 01-06 (Wave 4 docs + end-to-end verification): shipped `sim-rs/scripts/viz/README.md` (201 lines — single command, all six argparse flags, three-tier output layout, Notes covering every Pitfall 1-8 / CRITICAL LANDMINE, annual refresh recipe covering both vendored Observable Plot AND its D3 peer dep) and added a `### Visualising suite results` breadcrumb subsection to `CLAUDE.md` under `## Running the suites`. Orchestrator's earlier end-to-end smoke (captured in this plan's SUMMARY Context & history) had already confirmed Plot charts render against the live `sim-rs/output/` tree once the d3 vendor fix landed at ca2b2be. 18/18 viz tests green. Phase 01 is operationally complete; no follow-on phases scheduled for this workstream in this milestone.

## Progress

**Phases Complete:** 1 / 1
**Current Plan:** 6 (final)

## Plans Completed (this phase)

- [x] 01-01 — Wave 1 test harness (fixtures + 11 RED tests)
- [x] 01-02 — Wave 2 ingest module (build.py, three-tier JSON emission)
- [x] 01-03 — Wave 2 static bundle (index.html, style.css, main.js, vendored plot.min.js + d3.min.js peer dep)
- [x] 01-04 — Wave 3 serve entry-point (--serve / --port, copy_static_assets, ThreadingHTTPServer bound to 127.0.0.1, serve smoke test)
- [x] 01-05 — Wave 3 browser views (real renderHome/renderSuite/renderJob with three Plot chart panes + cross-seed overlay)
- [x] 01-06 — Wave 4 README + CLAUDE.md crumb + end-to-end checkpoint (orchestrator-confirmed)

## Decisions

- **Plot bundle form: UMD (not ESM)** — UMD's `window.Plot` global avoids a per-module import and matches Observable Plot's getting-started docs as a fully supported alternative; ESM swap remains a one-liner if a later plan prefers it.
- **Vendored Plot at `sim-rs/scripts/viz/static/plot.min.js`** — 209 KB committed; companion `PLOT_VERSION.txt` records the resolved version (0.6.17) + retrieval date for annual refresh. Upholds PROJECT.md "Local-first: must work without internet."
- **Suite-aggregates panel is conditional** — `payload.aggregates != null` gate + DOM comment marker; phase-2 suites unconditionally emit `aggregates: null` per Plan 01-02's locked contract. Plan 01-05 fleshed this out so when null is observed, NO header is rendered (just the HTML comment carrying the Pitfall 3 rationale).
- **HEADLINE_LATENCY_LABEL module constant** — single source of truth for the "Latency by demand component (blocks)" UI string; locks Pitfall 5 wording via grep gate. Plan 01-05 wired it into renderSuite (latency cell `title` attribute) + renderJob (latency section heading + table-row context).
- **`127.0.0.1` explicit bind in `serve()`** — first element of the address tuple is the literal string `"127.0.0.1"`, NOT `""` / `None` / `"0.0.0.0"`; mitigates T-01-04-01 (LAN exposure of `sim-rs/output/`). `allow_reuse_address = True` mitigates T-01-04-02 (TIME_WAIT collision on quick restarts).
- **Self-contained bundle layout: `index.html` at served root, assets under `static/`** — mirrors the SPA shell's `static/<asset>` relative href paths so the document resolves them correctly only when served from the bundle root. `copy_static_assets` enforces this layout.
- **Subprocess + free-port HTTP smoke** — `socket.bind(("127.0.0.1", 0))` picks a kernel-chosen free port; `subprocess.Popen` runs the build because `serve_forever()` blocks the caller; 5 s poll-with-deadline startup gate surfaces subprocess stderr on early exit. No hardcoded 8000, no `requests`/`httpx` dependency.
- **Cross-seed overlay uses stroke: 'seed'; lane multi-line charts use stroke: 'lane'; fees+refunds chart uses stroke: 'metric'** — three distinct `stroke` channel choices across the four chart shapes match RESEARCH.md `## Code Examples` verbatim and surface the correct grouping at the legend level.
- **Per-component latency table includes a derived 'dominant lane' column** — per RESEARCH.md Open Q #2 recommendation, surfaces lane info without mis-attributing the mean (which is computed over a mixed-lane observations list).
- **Per-suite Promise cache (Map<suiteId, Promise<payload>>)** — back-button revisits hit memory, not network. Bounded by the number of suites the user actually clicks into; no explicit eviction policy in v1.
- **D3 vendored as Observable Plot's peer dependency** — Plot's UMD bundle externalizes D3, so loading `plot.min.js` without a prior `d3.min.js` script tag yields `Cannot read properties of undefined (reading 'timeSecond')` / `Plot.ruleY is not a function`. Discovered + fixed at commit ca2b2be (`fix(01-03): vendor d3@7.9.0`); `index.html` loads `static/d3.min.js` BEFORE `static/plot.min.js`; `PLOT_VERSION.txt` records both pins; `test_d3_js_vendored_locally` (100 KB floor) locks the dep.
- **README is the canonical entry-point + CLAUDE.md is the breadcrumb** — Plan 01-06 added `sim-rs/scripts/viz/README.md` (the single documented command + full flag reference + three-tier output layout + every Pitfall 1-8 / CRITICAL LANDMINE surfaced + annual refresh recipe for both vendored bundles) and a two-paragraph `### Visualising suite results` subsection in `CLAUDE.md` under `## Running the suites`. The README is the deep doc; CLAUDE.md is the discovery breadcrumb from the primary project doc. Closes VIZ-06.

## Session Continuity

**Stopped At:** Phase 01 (viz-site-mvp) complete — all six VIZ-NN requirements met, README + CLAUDE.md crumb landed, end-to-end visual verification confirmed
**Resume File:** None
