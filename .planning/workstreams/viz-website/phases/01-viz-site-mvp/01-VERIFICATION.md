---
phase: 01-viz-site-mvp
verified: 2026-05-20T13:55:00Z
status: passed
score: 16/16 must-haves verified
overrides_applied: 0
---

# Phase 01: Viz Site MVP — Verification Report

**Phase Goal:** A local-first visualisation site exists that browses the suite
runs under `sim-rs/output/`, renders headline metrics and time series for a
selected (job, seed), supports comparison across runs, and can be brought up
with a single documented command.

**Verified:** 2026-05-20T13:55:00Z
**Status:** PASSED
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria + locked must-haves)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | The site lists the suite runs currently in `sim-rs/output/` and lets the user pick one without crawling directories by hand (VIZ-01, SC#1) | VERIFIED | `discover_suites` against `sim-rs/output/` returns **1890 suites** with 0 warnings (live spot-check); `renderHome` in `main.js:200` builds a sortable 8-column table from `data/index.json`; each row links to `#/suite/<suite_id>`. Default sort `started_at` descending (D-18). |
| 2 | Drilling into a suite shows its `manifest.json` summary and per-job / per-seed inventory (VIZ-02, SC#2) | VERIFIED | `renderSuite` in `main.js:342` renders manifest `<dl>` + sortable (job, seed) table from `data/<suite_id>.json`. Tier-2 JSON contract verified end-to-end: emitted file contains `id`, `name`, `path`, `started_at`, `manifest`, `jobs[<job>].seeds[<seed>].headline`, `aggregates`. |
| 3 | Headline metrics (retained value, net utility, retained-value ratio, latency-by-component, mempool depth) are rendered (VIZ-03, SC#3) | VERIFIED | `renderJob` in `main.js:692` mounts a six-card headline strip (retained_value, net_utility, retained_value_ratio, peak_mempool_bytes, included-of-submitted, event-stream-hash) + per-component latency table with `priority_included` / `standard_included` / dominant-lane derivation. Tier-3 JSON spot-check shows `retained_value=2.3e9`, `peak_mempool_bytes=316505`, `net_utility`, `retained_value_ratio` all emitted as primary fields. |
| 4 | Time-series for a selected (job, seed) render as multi-line charts with lane colouring (VIZ-04, SC#4) | VERIFIED | Three Observable Plot panes invoked through `renderChartPane` (`main.js:933`): controller quote (`stroke: "lane"`, line 899); mempool bytes (`stroke: "lane"`, line 908); fees+refunds (`stroke: "metric"`, line 920). Tier-3 JSON carries the long-form `{slot, lane, metric, value}` records the panes filter. Orchestrator pre-flight confirmed the three vertically-stacked panes render against `#/job/phase-2__sundaeswap-priority-only/rb_reserved_x16/1`. |
| 5 | Suite-level comparison aggregates render as charts or tables (VIZ-05, SC#5) | VERIFIED | `renderCrossSeedSection` (`main.js:565`) implements the in-suite cross-seed overlay: job + lane selects, `Promise.all` parallel fetch of every (job, seed) tier-3 JSON, `Plot.line(flat, {x: "slot", y: "value", stroke: "seed"})` overlay (line 665). Per D-15, in-suite cross-seed overlay is the v1 VIZ-05 scope (cross-suite deferred). For phase-2 suites, `aggregates: null` is unconditional (CRITICAL LANDMINE #2 / Pitfall 3): suite-aggregates panel is omitted entirely, only an HTML comment carries the rationale. Orchestrator pre-flight confirmed cross-seed overlay rendered. |
| 6 | A single documented command brings up a viewable site (VIZ-06, SC#6) | VERIFIED | `python sim-rs/scripts/viz/build.py --serve` documented in `sim-rs/scripts/viz/README.md` (201 lines) Quickstart + flag table (`--source`, `--output`, `--include`, `--exclude`, `--serve`, `--port`); breadcrumb in `CLAUDE.md` `### Visualising suite results` (line 446) under `## Running the suites`. End-to-end smoke against fixture in <1s; `ThreadingHTTPServer` binds `127.0.0.1:8000` exclusively. |
| 7 | Three-tier JSON layout under `data/` matches D-09 | VERIFIED | Live emit against fixture produces: `data/index.json` + `data/mini-suite.json` + `data/mini-suite/d8_target0.5_window32-{1,2}.json`. `discover_suites` against the live tree emits all three tiers without crashing on the 1890 manifests. |
| 8 | `build.py` is stdlib-only (D-01 / D-08) | VERIFIED | `python3 -c "import build"` succeeds with no `requirements.txt`, no virtualenv. Imports: `argparse`, `csv`, `datetime`, `fnmatch`, `functools`, `json`, `pathlib`, `shutil`, `sys`, `http.server` — all stdlib. No `yaml`, `requests`, `flask`, `jinja2`, `pytest` imports. |
| 9 | Server binds `127.0.0.1` exclusively (Pitfall 7 / D-04) | VERIFIED | `build.py:736` — `ThreadingHTTPServer(("127.0.0.1", port), handler)`. The single match for the literal string `"0.0.0.0"` in the file (line 716) is inside the function docstring explaining the landmine, NOT in a constructor argument. `allow_reuse_address = True` set at line 737 before `serve_forever`. |
| 10 | `latency by lane` is grep-clean in main.js (CRITICAL LANDMINE #3 / Pitfall 5) | VERIFIED | `grep -ci 'latency by lane' main.js = 0`. The canonical label `Latency by demand component (blocks)` appears once (declaration of `HEADLINE_LATENCY_LABEL`); the constant is referenced 7 times across the file. |
| 11 | `innerHTML` is absent from main.js (security landmine) | VERIFIED | `grep -c 'innerHTML' main.js = 0`. Every DOM insertion routes through the `el(tag, {text})` helper which sets `textContent`. |
| 12 | Observable Plot vendored (D-19) | VERIFIED | `static/plot.min.js` exists (209,183 bytes). `static/PLOT_VERSION.txt` records `@observablehq/plot@0.6.17 retrieved 2026-05-20`. |
| 13 | D3 7.9.0 vendored as Plot's peer dep (orchestrator d3 fix) | VERIFIED | `static/d3.min.js` exists (279,706 bytes). `PLOT_VERSION.txt` records `d3@7.9.0 retrieved 2026-05-20` with explanation of UMD externalization. `index.html:27-28` loads `d3.min.js` BEFORE `plot.min.js` (HTML comment cites the requirement). Captured by commit `ca2b2be` (`fix(01-03): vendor d3@7.9.0 (peer dep for Observable Plot UMD)`). |
| 14 | No new `.gitignore` entry for `sim-rs/output/viz/` (CRITICAL LANDMINE #4 / Pitfall 6) | VERIFIED | `grep 'output/viz' sim-rs/.gitignore` = empty. The existing `/output` rule (line 2) catches everything under `sim-rs/output/` transitively. Repo-root `.gitignore` unmodified. |
| 15 | Suite identifier path-derived, not name-derived (D-22 / Pitfall 2) | VERIFIED | Test `test_suite_id_derived_from_path_not_suite_name` PASSES; `build.py` contains `relative_to(source).replace("/", "__")` pattern; live tree spot-check shows IDs like `audit__20260428-104303-two-lane-priority-target09` (path-derived). |
| 16 | 18/18 viz tests green | VERIFIED | `cd sim-rs && python3 -m unittest discover -s scripts/viz/tests -t scripts/viz` reports `Ran 18 tests in 0.750s / OK`. 11 ingest tests + 7 smoke tests (the 6 original + the d3-vendoring smoke test added at commit `ca2b2be`). |

**Score:** 16/16 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `sim-rs/scripts/viz/build.py` | Ingest module + serve helper (Plans 02 + 04) | VERIFIED | 863 lines; exports `discover_suites`, `load_seed`, `run_build`, `parse_args`, `main`, `serve`, `copy_static_assets`, `LANE_FIELDS` (12 tuples). Live discovery against 1890 manifests succeeds; build-only against fixture produces correct three-tier emit. |
| `sim-rs/scripts/viz/__init__.py` | Package marker | VERIFIED | Empty file present; allows `import build` from tests via `sys.path.insert`. |
| `sim-rs/scripts/viz/static/index.html` | SPA shell | VERIFIED | 31 lines; loads `d3.min.js` then `plot.min.js` then `static/main.js` as a module; contains `<div id="app">` mount point and `data:` favicon. |
| `sim-rs/scripts/viz/static/main.js` | Hash router + 4 renderers | VERIFIED | 988 lines; `renderHome`, `renderSuite`, `renderJob`, `renderCrossSeedSection`, `renderChartPane`; `Plot.line` invoked 4× in source + cross-seed overlay; structural grep gates green. |
| `sim-rs/scripts/viz/static/style.css` | Minimal styling (D-23) | VERIFIED | 133 lines total, well within plan ceiling. |
| `sim-rs/scripts/viz/static/plot.min.js` | Observable Plot 0.6.17 UMD | VERIFIED | 209,183 bytes. |
| `sim-rs/scripts/viz/static/d3.min.js` | D3 7.9.0 (Plot peer dep) | VERIFIED | 279,706 bytes; loaded before Plot in index.html. |
| `sim-rs/scripts/viz/static/PLOT_VERSION.txt` | Version pins for both bundles | VERIFIED | Records both Plot 0.6.17 and D3 7.9.0 retrievals from 2026-05-20. |
| `sim-rs/scripts/viz/tests/test_ingest.py` | 11 ingest unit tests | VERIFIED | All 11 pass; covers VIZ-01..VIZ-05 + Pitfalls 1, 2, 3, 5, 8 + D-21, D-22. |
| `sim-rs/scripts/viz/tests/test_serve_smoke.py` | HTTP smoke tests | VERIFIED | 7 smoke tests (6 originally + `test_d3_js_vendored_locally` added at d3-vendor fix); all pass. |
| `sim-rs/scripts/viz/tests/fixtures/` | Three fixture trees | VERIFIED | mini-suite (kebab + snake + 15-col CSV), malformed-suite (truncated JSON), no-time-series (Pitfall 8 path) — all in place. |
| `sim-rs/scripts/viz/README.md` | Single-command + flag reference + Notes | VERIFIED | 201 lines, within 80-500 budget. All grep gates from Plan 01-06 acceptance criteria pass (single command, 127.0.0.1, gitignored, /output, SPA, Observable Plot, Latency by demand component, metrics_comparison.txt, priority_only_fast_path_overall_comparison.csv). |
| `CLAUDE.md` viz crumb | `### Visualising suite results` subsection | VERIFIED | Lines 446-466 under `## Running the suites` immediately before `## Conventions / gotchas`; contains literal command + README link + 127.0.0.1 bind + gitignore-transitivity notes. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `static/index.html` | `static/d3.min.js` + `static/plot.min.js` + `static/main.js` | Three `<script>` tags in correct order | WIRED | `index.html:27-29`: d3 → plot → main.js (module). HTML comment at line 20 explains the d3-first requirement. |
| `static/main.js::renderHome` | `data/index.json` | `fetch("data/index.json")` | WIRED | Confirmed via grep; orchestrator pre-flight confirmed 12 suites loaded from sim-rs/output/phase-2/sundaeswap*. |
| `static/main.js::renderSuite` | `data/<suite_id>.json` | `fetch("data/${id}.json")` | WIRED | Aggregates section gated by `payload.aggregates != null` (line 526); phase-2 path appends HTML comment marker. |
| `static/main.js::renderJob` | `data/<suite_id>/<job>-<seed>.json` | `fetch` + 3× `renderChartPane` | WIRED | `Plot.line` invoked once per pane with the right `stroke` channel (lane / lane / metric). |
| `static/main.js::renderCrossSeedSection` | tier-3 JSONs in parallel | `Promise.all([fetch(...)])` + `Plot.line(flat, {stroke: "seed"})` | WIRED | Line 665. |
| `build.py::main` | `run_build` → `copy_static_assets` → optional `serve` | sequential calls in main() | WIRED | Build-only smoke produces correct artefacts (index.html at root, static/ with 5 files + index.html-source, data/ three-tier). |
| `build.py::serve` | `ThreadingHTTPServer(("127.0.0.1", port), ...)` | bind tuple first element is literal `"127.0.0.1"` | WIRED | `build.py:736`; `allow_reuse_address = True` at 737. |
| Suite walk (build.py) | per-(job, seed) load | `discover_suites` → `_build_suite_json` → `load_seed` | WIRED | Live 1890-suite smoke succeeds with 0 warnings; tests confirm kebab→snake split honoured. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `renderHome` table | `data.suites` from `index.json` | `discover_suites` walking `sim-rs/output/` | YES — 1890 entries from live tree | FLOWING |
| `renderSuite` jobs table | `data.jobs[<job>].seeds[<seed>].headline` | `_build_suite_json` reading `run_summary.json` | YES — populated from `priority_retained_value_total + standard_retained_value_total`, `latency_blocks_mean = sum(obs)/len(obs)` per Pitfall 5 reduction | FLOWING |
| `renderJob` Plot panes | `data.time_series` filtered on `(metric, lane)` | `_read_time_series_long` reading `time_series.csv` and converting per `LANE_FIELDS` (12 tuples) | YES — fixture spot-check yielded 60 long-form records (5 rows × 12 fields); `Plot.line` filter resolves to records with real integer values | FLOWING |
| Cross-seed overlay | `Promise.all` of tier-3 JSONs across seeds | parallel fetch of `data/<suite>/<job>-<seed>.json` | YES — each tier-3 JSON has populated `time_series` + headline | FLOWING |
| `aggregates` section | `data.aggregates === null` (phase-2) | `_build_suite_json` unconditionally sets `aggregates = None` | INTENTIONALLY NULL — gates the render off; HTML comment carries the rationale (Pitfall 3 / CRITICAL LANDMINE #2) | INTENTIONAL_NULL (consistent with D-15 + locked schema) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Build module imports cleanly | `python3 -c "import sys; sys.path.insert(0, 'sim-rs/scripts/viz'); import build"` | Exits 0; full public API present | PASS |
| Build-only against fixture emits three-tier JSON | `python3 sim-rs/scripts/viz/build.py --source sim-rs/scripts/viz/tests/fixtures/mini-suite --output /tmp/viz-verif-01` | Produces `index.html`, `static/{d3,plot,main}.{min.,}js`, `static/style.css`, `static/PLOT_VERSION.txt`, `data/index.json`, `data/mini-suite.json`, `data/mini-suite/d8_target0.5_window32-{1,2}.json` | PASS |
| Discovery against the live tree | `discover_suites(Path('sim-rs/output').resolve(), [], [], [])` | 1890 suites, 0 warnings | PASS |
| Tier-2 JSON gates aggregates correctly | `python3 -c "import json; d=json.load(open('/tmp/.../mini-suite.json')); print(d['aggregates'])"` | `None` — CRITICAL LANDMINE #2 enforced | PASS |
| Tier-3 JSON has long-form time_series | inspect first 3 records | `[{lane:'priority', metric:'quote_per_byte', slot:0, value:44}, ...]` | PASS |
| Full test suite green | `cd sim-rs && python3 -m unittest discover -s scripts/viz/tests -t scripts/viz` | `Ran 18 tests in 0.750s / OK` | PASS |
| Security grep gates | `grep -c 'innerHTML' main.js; grep -ci 'latency by lane' main.js` | `0 / 0` | PASS |
| Bind landmine grep | `grep -n '("127.0.0.1"' build.py; check `("0.0.0.0"` is docstring-only` | 1 line at 736 (constructor); the 0.0.0.0 match is at line 716 inside the function docstring explaining the landmine | PASS |
| `.gitignore` for sim-rs/output/viz | `grep 'output/viz' sim-rs/.gitignore` | empty (transitively covered by `/output`) | PASS |

### Probe Execution

No project-conventional probes (`scripts/*/tests/probe-*.sh`) apply to the viz-website
workstream. The phase is Python + browser; behavioral verification runs through the
unittest suite (Step 7b above) instead.

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|----------------|-------------|--------|----------|
| VIZ-01 | 01-01, 01-02, 01-03, 01-05 | Navigable list of suite runs | SATISFIED | `renderHome` builds sortable 8-column table from `data/index.json`; live tree yields 1890 suites. |
| VIZ-02 | 01-01, 01-02, 01-03, 01-05 | Suite drill-down with manifest summary + per-job/per-seed inventory | SATISFIED | `renderSuite` renders `<dl>` for manifest + sortable (job, seed) table from tier-2 JSON. |
| VIZ-03 | 01-01, 01-02, 01-03, 01-05 | Headline metrics for selected (job, seed) | SATISFIED | `renderJob` builds 6-card headline strip + per-component latency table using `HEADLINE_LATENCY_LABEL` constant. |
| VIZ-04 | 01-01, 01-02, 01-03, 01-05 | Time-series plots with lane colouring | SATISFIED | 3× `renderChartPane` invocations in `renderJob` (controller quote `stroke:"lane"`, mempool bytes `stroke:"lane"`, fees+refunds `stroke:"metric"`). |
| VIZ-05 | 01-01, 01-02, 01-03, 01-05 | Cross-(job, seed) comparison aggregates | SATISFIED | `renderCrossSeedSection` overlays seeds with `Plot.line(flat, {stroke: "seed"})`. Static suite-level CSV aggregates section omitted (gated by `aggregates != null`) per phase-2 reality (CRITICAL LANDMINE #2). D-15 limits VIZ-05 to in-suite cross-seed for v1. |
| VIZ-06 | 01-03, 01-04, 01-06 | Single documented command | SATISFIED | `python sim-rs/scripts/viz/build.py --serve` documented in README + CLAUDE.md crumb; `ThreadingHTTPServer` binds `127.0.0.1`. |

No orphaned requirements: every VIZ-NN from REQUIREMENTS.md is claimed by at least
one PLAN and verified above. No additional requirement IDs are mapped to Phase 1
beyond VIZ-01..VIZ-06.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none) | — | — | — | No `TBD`, `FIXME`, `XXX`, `TODO`, `HACK`, or `PLACEHOLDER` markers found across `build.py`, `main.js`, `index.html`, `style.css`, `README.md`, `test_ingest.py`, `test_serve_smoke.py`. |

### Locked-Decision Honour Check

| Decision | Honoured? | Evidence |
|----------|-----------|----------|
| D-01 (stdlib-only) | YES | No third-party imports in `build.py`. |
| D-04 (HTTP local server) | YES | `ThreadingHTTPServer` + `127.0.0.1` bind in `build.py:736`. |
| D-06 (output path = sim-rs/output/viz) | YES | argparse default `--output sim-rs/output/viz` per README. |
| D-07 (single command) | YES | `python sim-rs/scripts/viz/build.py --serve` documented. |
| D-08 (no requirements.txt) | YES | No `requirements.txt` in workstream tree. |
| D-09 (three-tier JSON) | YES | `data/index.json` + `data/<suite>.json` + `data/<suite>/<job>-<seed>.json` confirmed on disk via fixture build. |
| D-15 (in-suite cross-seed only for v1) | YES | `renderCrossSeedSection` overlays seeds; cross-suite explicitly deferred. |
| D-19 (Plot vendored) | YES | `static/plot.min.js` (209 KB) + `static/PLOT_VERSION.txt`. **Plus** D3 7.9.0 vendored as peer dep (`static/d3.min.js`, 280 KB) — necessary because Plot 0.6.17 UMD externalizes D3. Captured at commit `ca2b2be`. |
| D-21 (skip-and-warn) | YES | `discover_suites` catches OSError/JSONDecodeError/AttributeError/TypeError and appends warnings; live 1890-suite smoke had 0 warnings (current tree is clean). |
| D-22 (path-derived suite id) | YES | Confirmed by `test_suite_id_derived_from_path_not_suite_name` + live spot-check (`audit__20260428-...`). |
| D-23 (minimal styling) | YES | `style.css` is 133 lines, well within plan ceiling. |

### Context & history (orchestrator-supplied)

A Plan 01-03 deliverable defect surfaced during the orchestrator's Wave 4 pre-flight
checkpoint: Observable Plot 0.6.17's UMD bundle externalizes D3 (its IIFE reads
`globalThis.d3` at module init), so without a prior D3 script tag every chart pane
failed with `Plot.ruleY is not a function` / `Cannot read properties of undefined
(reading 'timeSecond')`. The defect was fixed before Plan 06 dispatched:

- Commit `ca2b2be` — `fix(01-03): vendor d3@7.9.0 (peer dep for Observable Plot UMD)`
  - Added `sim-rs/scripts/viz/static/d3.min.js` (~280 KB)
  - `index.html` now loads `d3.min.js` BEFORE `plot.min.js` with an explanatory comment
  - `PLOT_VERSION.txt` records the d3 pin alongside the Plot pin
  - `test_serve_smoke.py` gained `test_d3_js_vendored_locally` (100 KB floor)
  - `copy_static_assets` (Plan 04) picks up `d3.min.js` automatically via `iterdir()`

The second smoke attempt (after the fix) confirmed the goal observably: suite list
for 12 phase-2 sundaeswap suites; clean drill-down and per-job hash routing; three
Plot panes on a job view (`#/job/phase-2__sundaeswap-priority-only/rb_reserved_x16/1`);
cross-seed overlay on the suite view; clean console; canonical "Latency by demand
component (blocks)" label. This audit-trail entry mirrors Plan 01-06's "Context &
history" SUMMARY block.

### Human Verification Required

None. The phase goal is observably true in the codebase across all verification
levels:

- Goal-backward: every ROADMAP success criterion (1-6) maps to a verified
  artifact + key link + data flow.
- Behavioral spot-checks (9/9 pass): live tree discovery, three-tier emit,
  full test suite, security gates, bind landmine, gitignore non-edit, schema
  contracts.
- Visual verification was already performed by the orchestrator at the Wave 4
  pre-flight checkpoint against the live `sim-rs/output/` tree (12 sundaeswap
  suites, three rendered Plot panes, cross-seed overlay, console clean). No
  further human action is required to close the phase.

### Gaps Summary

No gaps. Phase goal achieved. All six VIZ-NN requirements satisfied; all 16 must-have
truths verified; all 13 required artifacts present and substantive; all 8 key links
wired with data flowing through them; full test suite (18/18) green; no anti-patterns
or debt markers; every locked CONTEXT.md decision (D-01, D-04, D-06, D-07, D-08, D-09,
D-15, D-19, D-21, D-22, D-23) honoured; CRITICAL LANDMINES #2 (aggregates: null in
phase-2), #3 (no "latency by lane" wording), #4 (no new gitignore entry), and #7
(textContent only, no innerHTML) all structurally enforced via grep gates.

The mid-phase d3 vendoring defect at Plan 01-03 was caught and closed at the
orchestrator-level pre-flight before Plan 01-06 dispatched; commit `ca2b2be` carries
the fix, and the audit trail is preserved in 01-06-SUMMARY.md "Context & history"
plus this verification report.

---

*Verified: 2026-05-20T13:55:00Z*
*Verifier: Claude (gsd-verifier)*
