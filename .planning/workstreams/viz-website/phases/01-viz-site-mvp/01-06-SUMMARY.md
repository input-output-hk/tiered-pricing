---
phase: 01-viz-site-mvp
plan: 06
subsystem: docs-and-end-to-end-verification
tags: [docs, readme, claude-md-crumb, viz-06, single-documented-command, d3-peer-dep, end-to-end-verified]

# Dependency graph
requires:
  - "Plan 01-02 ingest module + three-tier JSON contract (README documents the resolved output layout)"
  - "Plan 01-03 static bundle (README documents the vendored Observable Plot + D3 bundles, the `static/<asset>` layout, and the `index.html`-at-bundle-root choice)"
  - "Plan 01-04 serve entry-point (README documents `--serve` / `--port` and the `127.0.0.1` bind)"
  - "Plan 01-05 browser views (README documents the rendered surfaces: home, suite drill-down, per-(job, seed), cross-seed overlay)"
  - "Orchestrator-level d3 vendor fix at commit ca2b2be (README documents the D3 peer-dep relationship + annual refresh covers both bundles)"
provides:
  - "sim-rs/scripts/viz/README.md (201 lines) — the single documented entry-point for VIZ-06: command, flag table, three-tier output layout, six Notes covering every Pitfall 1-8 / CRITICAL LANDMINE the workstream established, test command, annual refresh recipe"
  - "CLAUDE.md viz crumb (+22 lines) — `### Visualising suite results` subsection inside `## Running the suites`, with a single-command quickstart, a link to the README, and gitignore-transitivity + 127.0.0.1 bind notes"
  - "End-to-end visual confirmation against live `sim-rs/output/` (orchestrator pre-confirmed; see Context & history below)"
  - "Phase 01 — viz-site-mvp — operationally complete with all six VIZ-NN requirements met"
affects:
  - "Phase 01 is closed; no downstream plans in this milestone"

# Tech tracking
tech-stack:
  added:
    - "Markdown documentation surface (sim-rs/scripts/viz/README.md) — the project's first per-script README under sim-rs/scripts/"
  patterns:
    - "Module-docstring → README projection: PATTERNS.md's `### sim-rs/scripts/viz/README.md` section nominates sim-rs/scripts/generate-realistic-100-topology.py's module docstring as the structural template (one-line summary → usage block → flag table → output-layout map → notes); the README follows that shape in Markdown"
    - "Abbreviation expansions on first use per CLAUDE.md `## Conventions / gotchas`: Single-Page Application (SPA), HyperText Markup Language (HTML), Universal Module Definition (UMD), Immediately-Invoked Function Expression (IIFE), Command-Line Interface (CLI), Local Area Network (LAN), Document Object Model (DOM)"
    - "Forbidden-token scrub repeated (Plan 01-03 / 01-05 precedent): the case-insensitive 'latency by lane' regression-guard grep gate also applies to the README; the meta-discussion of why that wording is wrong uses 'per-lane-latency wording forbidden by Pitfall 5' indirection instead of inlining the gated string"

key-files:
  created:
    - sim-rs/scripts/viz/README.md
  modified:
    - CLAUDE.md
    - .planning/workstreams/viz-website/STATE.md
    - .planning/workstreams/viz-website/ROADMAP.md
    - .planning/workstreams/viz-website/REQUIREMENTS.md

key-decisions:
  - "README structure: one-line summary + Quickstart + Flags table + Output-layout tree + What-gets-rendered bullets + Notes + Tests + Updating-the-vendored-bundle recipe. 201 lines (well within the plan's 80-500 budget). Follows the PATTERNS.md analog of generate-realistic-100-topology.py's module docstring exactly, but in Markdown."
  - "What-gets-vendored section lists BOTH plot.min.js (Observable Plot 0.6.17, ~209 KB) AND d3.min.js (D3 7.9.0, ~280 KB) as required by the orchestrator's <final_smoke_already_satisfied> instruction. The peer-dep relationship is documented in prose: Plot's UMD form reads globalThis.d3 at module init, so the d3.min.js script tag MUST come first in index.html. PLOT_VERSION.txt records both pins."
  - "Latency mitigation: the README's discussion of why 'latency by lane' is wrong uses Plan 01-03 / Plan 01-05's indirection pattern ('the per-lane-latency wording forbidden by Pitfall 5') so the structural grep gate (`! grep -niE 'latency by lane'`) stays green. This is the third occurrence of this pattern in the codebase (main.js Task 2 in 01-03, renderJob in 01-05, README in 01-06)."
  - "CLAUDE.md insertion point: end of `## Running the suites` section, immediately before `## Conventions / gotchas`, as a `### Visualising suite results` subsection (no new top-level `##` heading). Two-paragraph crumb — one for the command, one for the rendering surface + gitignore-transitivity + 127.0.0.1 bind. Links the README; the README is the deep doc, this is the breadcrumb."
  - "End-to-end verification (Task 3 checkpoint) was satisfied at the orchestrator level before this executor was dispatched. The orchestrator's <final_smoke_already_satisfied> block records two smoke attempts and the d3 vendor fix in between. This SUMMARY captures the d3 fix story under 'Context & history' so the phase artefact trail is complete; no separate Task 3 verification was re-run here."

requirements-completed: [VIZ-06]

# Metrics
duration: ~3min
completed: 2026-05-20
---

# Phase 01 Plan 06: viz-website Wave 4 docs + end-to-end Summary

**Closes Phase 01: ships `sim-rs/scripts/viz/README.md` (the single documented command + flag reference + three-tier output layout + every Pitfall 1-8 / CRITICAL LANDMINE surfaced in the workstream) and a `### Visualising suite results` breadcrumb subsection in `CLAUDE.md`, then records the orchestrator's already-satisfied end-to-end visual verification. After this plan, a fresh developer can clone the repo, run `python sim-rs/scripts/viz/build.py --serve`, and see the site — no tribal knowledge.**

## Performance

- **Duration:** ~3 min
- **Tasks:** 3 (Task 1 + Task 2 executed + committed atomically; Task 3 satisfied at orchestrator level — see Context & history)
- **Files created:** 1 (`sim-rs/scripts/viz/README.md` — 201 lines)
- **Files modified:** 1 (`CLAUDE.md` — +22 lines)
- **Existing tests:** 18/18 PASS (no code changes; included as gate after docs land)

## Context & history (the orchestrator-level d3 vendor fix)

The orchestrator ran the full end-to-end smoke and surfaced two real
issues before this docs executor was dispatched. Both are captured here
so the phase artefact trail is complete:

1. **First smoke attempt** — `python sim-rs/scripts/viz/build.py
   --include 'phase-2/sundaeswap*' --serve` against
   `sim-rs/output/` succeeded in loading 12 real suites, but every
   `renderJob` Observable Plot pane failed with:

       Uncaught TypeError: Cannot read properties of undefined
         (reading 'timeSecond')
       Plot.ruleY is not a function

   **Root cause:** Observable Plot 0.6.17's UMD bundle externalizes D3 —
   its IIFE reads `globalThis.d3` at module init, so loading
   `plot.min.js` without a prior `d3.min.js` script tag yields the above
   errors. This was a Plan 01-03 deliverable defect (D-19 "vendored
   Observable Plot" implies vendoring the peer dependency too).

2. **Fix committed at `ca2b2be`** —
   `fix(01-03): vendor d3@7.9.0 (peer dep for Observable Plot UMD)`:
   - Added `sim-rs/scripts/viz/static/d3.min.js` (~280 KB)
   - `index.html` now loads `static/d3.min.js` BEFORE
     `static/plot.min.js` (a comment at the script-tag block cites the
     UMD externalization)
   - `PLOT_VERSION.txt` records the d3 pin alongside the Plot pin
   - `test_serve_smoke.py` gained `test_d3_js_vendored_locally` (100 KB
     floor) to lock the dep
   - All 18 viz tests green (the 17 from Plan 04 + this new smoke test)
   - `copy_static_assets` (Plan 04) uses `iterdir()` so it picks up
     `d3.min.js` automatically — no build.py change required

3. **Second smoke attempt** (re-run after fix) — User confirmed Plot
   charts render: three vertically-stacked time-series panes on a job
   view (`#/job/phase-2__sundaeswap-priority-only/rb_reserved_x16/1`),
   cross-seed overlay on the suite view, canonical latency label, no
   console errors.

**Implications captured in the README:**

- The "what gets vendored" Notes section lists BOTH `plot.min.js`
  AND `d3.min.js` (the peer-dep relationship is non-obvious and the
  README is the durable place to record it).
- The output-layout tree shows all six files under `static/`:
  `index.html` at bundle root + `static/main.js`, `static/style.css`,
  `static/plot.min.js`, `static/d3.min.js`, `static/PLOT_VERSION.txt`.
- The annual refresh recipe vendors both bundles together with an
  explicit "do not refresh Plot without refreshing D3 in the same pull
  request" note.

## VIZ-NN traceability (final phase close-out)

| Req     | View / surface | Where satisfied | Numeric / structural confirmation |
|---------|----------------|-----------------|------------------------------------|
| VIZ-01  | Suite list (sortable, default sort started_at desc) | Plan 01-05 `renderHome` | Orchestrator smoke: 12 suites loaded against `--include 'phase-2/sundaeswap*'`; full `--source sim-rs/output` walk discovers 249 manifest.json files |
| VIZ-02  | Suite drill-down (manifest summary + (job, seed) table) | Plan 01-05 `renderSuite` | Orchestrator smoke confirmed manifest <dl> + sortable jobs table render |
| VIZ-03  | Per-(job, seed) headline strip (6 cards) + per-component latency table | Plan 01-05 `renderJob` headline block | Headline cards: retained_value, net_utility, retained_value_ratio, peak mempool, multiplier-floor breaches, pricing_event_stream_sha256 (first 8 hex chars + full sha256 via title attr) |
| VIZ-04  | Three Observable Plot chart panes (controller quote, mempool bytes, fees+refunds per slot) | Plan 01-05 `renderJob` → `renderChartPane` (×3) | Orchestrator second smoke: three vertically-stacked time-series panes rendered against `#/job/phase-2__sundaeswap-priority-only/rb_reserved_x16/1`, console clean |
| VIZ-05  | In-suite cross-seed time-series overlay | Plan 01-05 `renderCrossSeedSection` (suite view) | Orchestrator smoke confirmed; `stroke: "seed"` channel, one coloured line per seed, job + lane selects |
| VIZ-06  | A single documented command produces a viewable site | Plan 01-04 `--serve` + this plan's README + CLAUDE.md crumb | `python sim-rs/scripts/viz/build.py --serve` — documented verbatim in README Quickstart + CLAUDE.md `### Visualising suite results` |

The README is now the canonical entry-point for a fresh developer; the
CLAUDE.md crumb is the discovery breadcrumb from the primary project
doc.

## Task commits

Each task was committed atomically:

1. **Task 1 — `sim-rs/scripts/viz/README.md` (single command + flags + output layout + Notes + tests + refresh recipe)** — `2654ce4` (docs)
2. **Task 2 — `### Visualising suite results` subsection in `CLAUDE.md`** — `016b5bf` (docs)
3. **Task 3 — end-to-end visual verification** — satisfied at orchestrator level before this executor was dispatched (see Context & history; no commit, gate is observational)

No fix / refactor commits, no Rule 1/2/3/4 deviations triggered during
this plan. The one deviation-shaped clarification (forbidden-token
scrub in the README) is documented under "Deviations from Plan" below
and matches the Plan 01-03 / Plan 01-05 precedent exactly.

## Files Created / Modified

- **`sim-rs/scripts/viz/README.md`** (created, 201 lines) — phase-2
  simulator visualisation site documentation. Sections: opening summary
  with abbreviation expansions, Quickstart (the literal
  `python sim-rs/scripts/viz/build.py --serve`), Flags table for all
  six argparse options, Output-layout tree mapping the three tiers to
  the six VIZ-NN views, What-gets-rendered bullets, Notes (eight
  bullets covering gitignore-is-transitive, stdlib-only, Plot vendored,
  D3 vendored as peer dep, metrics_comparison.txt is human-only,
  priority_only_fast_path_overall_comparison.csv is historical,
  latency-per-component-not-per-lane, 127.0.0.1 bind, textContent not
  innerHTML), Tests command, Updating-the-vendored-bundle annual recipe
  covering both Plot AND D3.
- **`CLAUDE.md`** (modified, +22/-0) — added a single `### Visualising
  suite results` subsection at the end of `## Running the suites`,
  immediately before `## Conventions / gotchas`. Two paragraphs: one
  with the literal command, one with the rendering surface + the
  gitignore-transitivity note + the 127.0.0.1 bind note + a link to the
  README for the deep reference. No other section of CLAUDE.md was
  modified; `git diff CLAUDE.md` confirms the change is the +22-line
  insertion only.

## Verification

### Task 1 (README) — 14 acceptance gates, all green

```
$ test -f sim-rs/scripts/viz/README.md                                       # OK
$ grep -q 'python sim-rs/scripts/viz/build.py --serve' README.md             # OK
$ grep -q '127\.0\.0\.1' README.md                                            # OK
$ grep -q 'gitignored' README.md                                              # OK
$ grep -q '/output' README.md                                                 # OK
$ grep -qE 'Single-Page Application \(SPA\)|SPA' README.md                    # OK
$ grep -q 'Observable Plot' README.md                                         # OK
$ grep -q 'Latency by demand component' README.md                             # OK
$ ! grep -niE 'latency by lane' README.md                                     # OK (count = 0 after scrub)
$ grep -q 'metrics_comparison\.txt' README.md                                 # OK
$ grep -q 'priority_only_fast_path_overall_comparison\.csv' README.md         # OK
$ test "$(wc -l < README.md)" -ge 80                                          # OK (201 >= 80)
$ test "$(wc -l < README.md)" -le 500                                         # OK (201 <= 500)
$ grep -c -- '--source|--output|--include|--exclude|--serve|--port' README.md # 10 matches (all six flags + repeats)
```

### Task 2 (CLAUDE.md) — 5 acceptance gates, all green

```
$ grep -q 'sim-rs/scripts/viz' CLAUDE.md                                      # OK
$ grep -q 'python sim-rs/scripts/viz/build.py' CLAUDE.md                      # OK
$ grep -qE 'Visualising suite results' CLAUDE.md                              # OK
$ grep -q 'sim-rs/output/viz' CLAUDE.md                                       # OK
$ ! grep -nE 'latency by lane' CLAUDE.md                                      # OK (count = 0)
$ git diff --stat CLAUDE.md                                                   # +22/-0, single +22-line insertion
```

### Plan-level final verification

```
$ test -f sim-rs/scripts/viz/README.md                                        # OK
$ grep -q 'python sim-rs/scripts/viz/build.py' CLAUDE.md                      # OK
$ cd sim-rs && python3 -m unittest discover -s scripts/viz/tests -t scripts/viz
... (18 test methods listed) ...
Ran 18 tests in 0.759s
OK
```

Test count: 18 (the 17 from Plan 04 + 1 added by the orchestrator's d3
vendor fix at ca2b2be: `test_d3_js_vendored_locally`). All green in
under 1 second.

## Deviations from Plan

### Plan-action clarifications (Plan 01-03 / 01-05 precedent)

**1. [Comment-token scrub — same class as Plan 01-03 clarification #1 and Plan 01-05 clarification #1] README's meta-discussion of why "latency by lane" is wrong initially inlined that string inside quotes**

- **Found during:** Task 1 verification gate run (the
  `! grep -niE 'latency by lane'` regression-guard in the plan's
  acceptance criteria fired on the README's own discussion of the
  forbidden phrase).
- **Fix:** Rewrote the relevant Notes bullet to describe the rule
  without inlining the forbidden tokens, matching Plan 01-03 / Plan
  01-05's same precedent — "the per-lane-latency wording forbidden by
  Pitfall 5 is grep-gated out of `static/main.js`" instead of inlining
  the gated string. Semantically equivalent; grep-clean.
- **Why this is a clarification not a Rule 2 deviation:** the plan's
  acceptance criteria explicitly chose a case-insensitive grep gate
  (`! grep -niE 'latency by lane'`) over context-aware checks. The
  behaviour was always to honour the gate; the README's first draft
  inlined the phrase to make the explanation more direct, but that
  trips the structural gate. The same precedent applied to main.js
  twice already (Plan 01-03 Task 2 and Plan 01-05 Task 2).
- **Files modified:** `sim-rs/scripts/viz/README.md` (edited in-place
  before the Task 1 commit; not a separate commit).

### Auto-fixed issues (Rules 1/2/3)

None — both tasks ran cleanly to verified completion with one in-place
scrub.

### Auth gates

None — pure documentation surface, no network, no external services.

### Auto-mode triggers

None — `auto_advance: false` in `.planning/config.json`. Task 3 (end-to-
end checkpoint) was already satisfied at the orchestrator level before
this executor was dispatched, so no checkpoint stop was required here.

## Stubs introduced

None. The README is production-ready documentation; the CLAUDE.md crumb
is a finished two-paragraph subsection. No `TBD` markers, no
placeholders, no future-plan references in either output.

## Sanity check vs plan targets

| Item                                                  | Target / Expected                                              | Actual                                                                                       |
|-------------------------------------------------------|----------------------------------------------------------------|----------------------------------------------------------------------------------------------|
| `README.md` line count                                | 80-500 (plan budget); 150-300 (planner recommendation)         | 201 (within both ranges)                                                                     |
| All six argparse flags documented                     | yes                                                            | yes (`--source`, `--output`, `--include`, `--exclude`, `--serve`, `--port` in the Flags table) |
| Single command literal present                        | `python sim-rs/scripts/viz/build.py --serve`                   | yes (Quickstart code block + CLAUDE.md crumb code block)                                     |
| `127.0.0.1` literal present                           | yes (security bind statement)                                  | yes (Notes section)                                                                          |
| Gitignore-transitivity surfaced                       | both `gitignored` + `/output` mentioned in the same Notes      | yes ("sim-rs/.gitignore line 2 (`/output`) catches everything under `sim-rs/output/`")        |
| SPA abbreviation expansion on first use               | yes                                                            | yes ("Single-Page Application (SPA)" in opening paragraph)                                   |
| Observable Plot spelled in full on first use          | yes                                                            | yes (opening paragraph)                                                                      |
| `Latency by demand component (blocks)` literal        | yes (Pitfall 5 surfaced)                                       | yes ("What gets rendered" + Notes)                                                           |
| `latency by lane` case-insensitive absent             | yes (regression guard)                                         | yes (count = 0 after in-place scrub)                                                         |
| `metrics_comparison.txt` mentioned + explained        | yes (Pitfall 4 surfaced)                                       | yes (Notes section)                                                                          |
| `priority_only_fast_path_overall_comparison.csv` mentioned + located | yes (CRITICAL LANDMINE #2 / Pitfall 3 surfaced) | yes (Notes section, "lives under sim-rs/output/analysis/")                                   |
| Both plot.min.js AND d3.min.js documented as vendored | yes (orchestrator's <final_smoke_already_satisfied>)           | yes (Notes section + Output-layout tree + Updating-the-vendored-bundle recipe)               |
| CLAUDE.md `###`-level subsection (not new `##`)       | yes                                                            | yes (`### Visualising suite results` under `## Running the suites`)                          |
| CLAUDE.md change localised to the new block           | yes (no other content modified)                                | yes (+22/-0, single insertion confirmed by `git diff --stat`)                                |
| 18/18 viz tests green                                 | yes                                                            | yes (0.76 s wall-clock)                                                                      |

## Next Phase Readiness

- **Phase 01 (viz-site-mvp)** is operationally complete. All six VIZ-NN
  requirements (VIZ-01 through VIZ-06) are met; the README and CLAUDE.md
  crumb close VIZ-06's "single documented command" wording explicitly.
- **No follow-on phases** are scheduled for the viz-website workstream
  in this milestone (the ROADMAP has a single phase). Future work
  (cross-suite comparison, paired-bootstrap CI bands, event-stream
  drill-down, public hosting) is captured under CONTEXT.md's `<deferred>`
  block for a hypothetical v1.1 phase.
- **No blockers, no carry-forward.** A fresh developer can clone the
  repo, run `python sim-rs/scripts/viz/build.py --serve`, and see the
  full set of six VIZ-NN views in the browser.

## Self-Check: PASSED

**Created files (verified present on disk):**

- FOUND: `sim-rs/scripts/viz/README.md` (201 lines)
- FOUND: `.planning/workstreams/viz-website/phases/01-viz-site-mvp/01-06-SUMMARY.md` (this file)

**Modified files (verified diff):**

- FOUND: `CLAUDE.md` (+22/-0, localised insertion under `## Running the suites`)

**Commits (verified in `git log --oneline -3`):**

- FOUND: `2654ce4` docs(01-06): add sim-rs/scripts/viz/README.md (single command + flags + output layout)
- FOUND: `016b5bf` docs(01-06): add viz-site crumb to CLAUDE.md (Running the suites)

**Plan-level verification block (verified):**

- 18/18 viz tests PASS in 0.76 s
- All 14 README acceptance gates green
- All 5 CLAUDE.md acceptance gates green
- All six VIZ-NN requirements (VIZ-01 through VIZ-06) have a documented
  surface in the README + a discovery breadcrumb in CLAUDE.md

---
*Phase: 01-viz-site-mvp*
*Completed: 2026-05-20*
