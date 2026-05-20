---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
current_phase: 01
current_plan: 5
status: executing
stopped_at: Plan 01-04 complete; ready for Plan 01-05 (browser views)
last_updated: "2026-05-20T12:20:26Z"
last_activity: 2026-05-20
progress:
  total_phases: 5
  completed_phases: 4
  total_plans: 17
  completed_plans: 19
  percent: 100
---

# Project State

## Current Position

Phase: 01 (viz-site-mvp) — EXECUTING
Plan: 5 of 6
**Status:** Plan 01-04 complete; ready for Plan 01-05
**Current Phase:** 01
**Last Activity:** 2026-05-20
**Last Activity Description:** Completed Plan 01-04 (Wave 3 serve entry-point): extended build.py with `--serve` / `--port` / `copy_static_assets` / `serve()`; explicit 127.0.0.1 bind on ThreadingHTTPServer + allow_reuse_address; 6-method ServeSmokeTest spawns the entry-point as a subprocess against the mini-suite fixture.

## Progress

**Phases Complete:** 0 / 1
**Current Plan:** 5

## Plans Completed (this phase)

- [x] 01-01 — Wave 1 test harness (fixtures + 11 RED tests)
- [x] 01-02 — Wave 2 ingest module (build.py, three-tier JSON emission)
- [x] 01-03 — Wave 2 static bundle (index.html, style.css, main.js, vendored plot.min.js)
- [x] 01-04 — Wave 3 serve entry-point (--serve / --port, copy_static_assets, ThreadingHTTPServer bound to 127.0.0.1, serve smoke test)
- [ ] 01-05 — Wave 3 browser views (real Plot figures + cross-seed overlay)
- [ ] 01-06 — Wave 4 README + end-to-end checkpoint

## Decisions

- **Plot bundle form: UMD (not ESM)** — UMD's `window.Plot` global avoids a per-module import and matches Observable Plot's getting-started docs as a fully supported alternative; ESM swap remains a one-liner if a later plan prefers it.
- **Vendored Plot at `sim-rs/scripts/viz/static/plot.min.js`** — 209 KB committed; companion `PLOT_VERSION.txt` records the resolved version (0.6.17) + retrieval date for annual refresh. Upholds PROJECT.md "Local-first: must work without internet."
- **Suite-aggregates panel is conditional** — `payload.aggregates != null` gate + DOM comment marker; phase-2 suites unconditionally emit `aggregates: null` per Plan 01-02's locked contract.
- **HEADLINE_LATENCY_LABEL module constant** — single source of truth for the "Latency by demand component (blocks)" UI string; locks Pitfall 5 wording via grep gate.
- **`127.0.0.1` explicit bind in `serve()`** — first element of the address tuple is the literal string `"127.0.0.1"`, NOT `""` / `None` / `"0.0.0.0"`; mitigates T-01-04-01 (LAN exposure of `sim-rs/output/`). `allow_reuse_address = True` mitigates T-01-04-02 (TIME_WAIT collision on quick restarts).
- **Self-contained bundle layout: `index.html` at served root, assets under `static/`** — mirrors the SPA shell's `static/<asset>` relative href paths so the document resolves them correctly only when served from the bundle root. `copy_static_assets` enforces this layout.
- **Subprocess + free-port HTTP smoke** — `socket.bind(("127.0.0.1", 0))` picks a kernel-chosen free port; `subprocess.Popen` runs the build because `serve_forever()` blocks the caller; 5 s poll-with-deadline startup gate surfaces subprocess stderr on early exit. No hardcoded 8000, no `requests`/`httpx` dependency.

## Session Continuity

**Stopped At:** Plan 01-04 complete; ready for Plan 01-05
**Resume File:** None
