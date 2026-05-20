---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
current_phase: 01
current_plan: 6
status: executing
stopped_at: Plan 01-05 complete; ready for Plan 01-06 (README + end-to-end checkpoint)
last_updated: "2026-05-20T12:33:39Z"
last_activity: 2026-05-20
progress:
  total_phases: 5
  completed_phases: 5
  total_plans: 17
  completed_plans: 20
  percent: 100
---

# Project State

## Current Position

Phase: 01 (viz-site-mvp) — EXECUTING
Plan: 6 of 6
**Status:** Plan 01-05 complete; ready for Plan 01-06
**Current Phase:** 01
**Last Activity:** 2026-05-20
**Last Activity Description:** Completed Plan 01-05 (Wave 3 browser views): replaced the Plan 01-03 stub renderers in `sim-rs/scripts/viz/static/main.js` with real renderHome (sortable suite list, default sort started_at desc), renderSuite (manifest summary + sortable (job, seed) table + conditional aggregates panel + cross-seed time-series overlay), and renderJob (six-card headline strip + per-component latency table + three Observable Plot chart panes). 362 → 988 lines; security grep gates green (no innerHTML, no "latency by lane"); 17/17 existing tests still pass; HTTP smoke against mini-suite green.

## Progress

**Phases Complete:** 0 / 1
**Current Plan:** 6

## Plans Completed (this phase)

- [x] 01-01 — Wave 1 test harness (fixtures + 11 RED tests)
- [x] 01-02 — Wave 2 ingest module (build.py, three-tier JSON emission)
- [x] 01-03 — Wave 2 static bundle (index.html, style.css, main.js, vendored plot.min.js)
- [x] 01-04 — Wave 3 serve entry-point (--serve / --port, copy_static_assets, ThreadingHTTPServer bound to 127.0.0.1, serve smoke test)
- [x] 01-05 — Wave 3 browser views (real renderHome/renderSuite/renderJob with three Plot chart panes + cross-seed overlay)
- [ ] 01-06 — Wave 4 README + end-to-end checkpoint

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

## Session Continuity

**Stopped At:** Plan 01-05 complete; ready for Plan 01-06
**Resume File:** None
