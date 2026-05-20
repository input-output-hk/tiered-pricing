---
phase: 01-viz-site-mvp
plan: 05
subsystem: browser-views
tags: [browser, spa, observable-plot, hash-router, viz-04, viz-05, headline-strip, cross-seed-overlay, security-textContent]

# Dependency graph
requires:
  - "Plan 01-02 three-tier JSON contract (data/index.json + data/<id>.json + data/<id>/<job>-<seed>.json) — consumed by all three renderers"
  - "Plan 01-03 SPA shell + vendored Observable Plot 0.6.17 (window.Plot global) + DOM helpers (el, HEADLINE_LATENCY_LABEL constant) — extended in-place"
provides:
  - "Real renderHome — sortable suite-list table with 8 columns (D-12), default sort started_at descending (D-18), suite-name links → #/suite/<id>"
  - "Real renderSuite — manifest <dl> + sortable (job, seed) table with headline-metric columns + conditional aggregates panel (omitted when null, Pitfall 3) + cross-seed time-series overlay (VIZ-05 / D-15)"
  - "Real renderJob — six-card headline strip (VIZ-03) + per-component latency table with dominant-lane derivation + three Plot.line chart panes (VIZ-04: controller quote per lane, mempool bytes per lane, fees+refunds per slot)"
  - "renderChartPane helper — single Plot.plot+Plot.line wrapper with try/catch isolation per pane so one broken chart can't kill the view"
  - "renderCrossSeedSection helper — job+lane select pair, Promise.all parallel fetch of all (job, seed) JSONs, Plot.line with stroke: 'seed' overlay"
  - "Module-level format helpers — fmtInt, fmtRatio, fmtComponents, sortBy (null-safe); suiteCache (Promise cache for re-visits)"
affects:
  - 01-06-PLAN-readme-and-end-to-end

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Click-to-sort table header pattern with closure-scope sort state (sortKey + sortDir), header rebuild + body rebuild functions, ▲/▼ indicators"
    - "Per-suite Promise cache (Map<suiteId, Promise<payload>>) — back-button revisits hit memory, not network"
    - "Long-form filter pattern — chart panes filter time_series on (metric, lane) before passing to Plot.line; no client-side melt because build.py already long-formed at ingest"
    - "HTML comment marker for null aggregates (document.createComment) — DevTools observers see the Pitfall 3 rationale without a visible header"
    - "try/catch around each Plot.plot call inside renderChartPane — one broken chart pane renders a muted error message instead of killing the view"
    - "stroke: 'metric' for fees+refunds chart (both records are lane='total'; the metric field is what distinguishes the two lines) — distinct from stroke: 'lane' (chart 1, 2) and stroke: 'seed' (cross-seed overlay)"

key-files:
  created: []
  modified:
    - sim-rs/scripts/viz/static/main.js

key-decisions:
  - "Single source file — main.js stayed as one module rather than splitting into renderers/, helpers/, plot/ — matches D-23 minimalism and the SPA scope. At 988 lines (727 substance + 261 comments) it stays scannable."
  - "Cross-seed overlay defaults to priority lane on first render — user can flip to standard via the lane select; no third 'total' option because the cross-seed comparison is about controller quote (priority/standard only)."
  - "Aggregates panel HTML comment text spells out the Pitfall 3 rationale verbatim — DevTools-readable audit trail per the plan's must_haves."
  - "Per-component latency table includes a derived 'dominant lane' column per RESEARCH.md Open Q #2 recommendation — surfaces lane info without mis-attributing the mean."
  - "Event-stream hash card shows first 8 hex chars with full sha256 via setAttribute('title', ...); the unsafe HTML-string sink stays grep-gated out of the file."
  - "Comment-level forbidden-token scrub repeated — two comments in renderJob initially used the grep-banned tokens to describe the rules themselves; rewrote them per Plan 01-03's same precedent ('grep-gated out of this file' instead of inlining the token)."

requirements-completed: [VIZ-01, VIZ-02, VIZ-03, VIZ-04, VIZ-05]

# Metrics
duration: ~7min
completed: 2026-05-20
---

# Phase 01 Plan 05: viz-website Wave 3 browser views Summary

**Replaced the Plan 01-03 stub view-renderers in `sim-rs/scripts/viz/static/main.js` with complete implementations of `renderHome`, `renderSuite`, and `renderJob` against the Plan 01-02 three-tier JSON contract. Wired Observable Plot 0.6.17 figures (the vendored `window.Plot` global from Plan 01-03) into three chart panes per job view plus a cross-seed overlay per suite view. Held the load-bearing landmines (no `innerHTML`, no "latency by lane", aggregates-section-omitted-when-null) via grep-gated structural checks.**

## Performance

- **Duration:** ~7 min
- **Started:** 2026-05-20T12:26:25Z
- **Completed:** 2026-05-20T12:33:39Z
- **Tasks:** 2 atomic auto-commits (Task 3 is a human-verify checkpoint — structural gates green, visual confirmation deferred per the same pattern as Plan 01-03 / Plan 01-04)
- **Files modified:** 1 (`sim-rs/scripts/viz/static/main.js` — 362 → 988 lines, +552 / -169 in Task 1, +251 / -8 in Task 2)
- **Files created:** 0
- **Existing tests:** 17/17 PASS (unchanged from baseline)

## Line counts

| File | Before | After |
|------|--------|-------|
| `sim-rs/scripts/viz/static/main.js` | 362 | 988 |

Of the 988 lines, ~727 are substance and ~261 are comments + the JSON-contract schema block at the top of the file. The schema comment block is intentional: future executors should not have to chase the Plan 01-02 schema across files.

## Chart-pane shapes (the three VIZ-04 panes)

| Pane | Filter | Marks | Y-axis label |
|------|--------|-------|--------------|
| Controller quote per lane (c_priority, c_standard) | `metric === "quote_per_byte" && (lane === "priority" \|\| lane === "standard")` | `Plot.ruleY([0])`, `Plot.line({x: "slot", y: "value", stroke: "lane"})` | `controller quote (lovelace/byte)` |
| Mempool bytes (priority, standard, total) | `metric === "mempool_bytes"` | `Plot.ruleY([0])`, `Plot.line({x: "slot", y: "value", stroke: "lane"})` | `mempool bytes` |
| Fees paid + refunds per slot (lovelace) | `metric === "fees_paid_lovelace" \|\| metric === "refund_lovelace"` | `Plot.ruleY([0])`, `Plot.line({x: "slot", y: "value", stroke: "metric"})` | `lovelace per slot` |

The cross-seed overlay (VIZ-05) on the suite view uses `Plot.line(flat, {x: "slot", y: "value", stroke: "seed"})` with `color: {legend: true, type: "ordinal"}` and a `Plot.ruleY([0])` baseline.

All four chart shapes match RESEARCH.md `## Code Examples` verbatim modulo the y-axis labels (specialised per pane) and the metric filter (per pane). Each `Plot.plot` call is wrapped in try/catch so one broken pane (e.g. an unexpected NaN in `value`) replaces its container's children with a muted error message instead of killing the rest of the view.

## Public API (locked surface)

```javascript
// sim-rs/scripts/viz/static/main.js
export const HEADLINE_LATENCY_LABEL;  // "Latency by demand component (blocks)"
export function fmtInt(n);            // Intl.NumberFormat, maxFractionDigits: 0, em-dash for null
export function fmtRatio(n);          // Intl.NumberFormat, maxFractionDigits: 4, em-dash for null
export function fmtComponents(comps); // "0.93 / 4.21 / 1.10" (2-decimal, up to 3 components)
export function sortBy(rows, key, dir); // null-safe sort (nulls always to end)
export async function route();        // hashchange dispatch
export async function renderHome();   // VIZ-01 — sortable suite list
export async function renderSuite(id); // VIZ-02 + VIZ-05 — manifest + (job, seed) table + cross-seed overlay
export async function renderJob(suite, job, seed); // VIZ-03 + VIZ-04 — headline strip + latency table + 3 charts
```

## Security gates (Task 1 + Task 2 acceptance — STRUCTURAL, not advisory)

```
$ grep -c 'innerHTML' sim-rs/scripts/viz/static/main.js
0

$ grep -ci 'latency by lane' sim-rs/scripts/viz/static/main.js
0

$ grep -c 'Latency by demand component (blocks)' sim-rs/scripts/viz/static/main.js
1   # the single canonical literal in HEADLINE_LATENCY_LABEL declaration

$ grep -c 'HEADLINE_LATENCY_LABEL' sim-rs/scripts/viz/static/main.js
7   # 1 declaration + 6 usages (suite latency cell title, job latency section heading, latency table header, etc.)

$ grep -c 'textContent' sim-rs/scripts/viz/static/main.js
2   # el() helper's textContent assignment + one .textContent assignment inside the retained-value subtotals card
```

The single literal `"Latency by demand component (blocks)"` is in `const HEADLINE_LATENCY_LABEL = ...` at the top of the file; every other place that displays the label references the constant by name. Comments describing the security rules use the indirection "the unsafe HTML-string sink is grep-gated out of this file" / "the per-lane wording forbidden by Pitfall 5 is grep-gated out of this file" to avoid tripping their own grep gates — the same precedent Plan 01-03 set.

## Task commits

Each task was committed atomically. Both commits are on `dynamic-experiment` (the active branch — `use_worktrees: false` per the workstream's PROJECT.md choice).

1. **Task 1 — real renderHome + renderSuite with sortable tables and cross-seed overlay** — `d66eba7` (feat)
2. **Task 2 — real renderJob with headline strip + per-component latency + 3 Plot chart panes** — `ea631f6` (feat)

No fix / refactor commits, no Rule 1/2/3/4 deviations triggered (see "Deviations from Plan" below for the comment-token clarification that mirrors Plan 01-03's same precedent).

## Files Modified

- `sim-rs/scripts/viz/static/main.js` — replaced the four stub renderers with real implementations. The shell wiring locked by Plan 01-03 (router, `el()` helper, `HEADLINE_LATENCY_LABEL` constant, `<script type="module">` load) is untouched; only the inner bodies of `renderHome`, `renderSuite`, and `renderJob` changed, plus the JSON-contract schema comment block at the top and the new formatting / fetch-cache / chart-pane helpers.

## Verification

### Task 1 (renderHome + renderSuite)

```
$ grep -n 'innerHTML' main.js                         # 0 matches — OK
$ grep -niE 'latency by lane' main.js                 # 0 matches — OK
$ grep -q 'Latency by demand component (blocks)'      # OK
$ grep -q 'renderHome'                                # OK
$ grep -q 'renderSuite'                               # OK
$ grep -q 'renderJob'                                 # OK (stub, replaced by Task 2)
$ grep -q 'Pitfall 3'                                 # OK
$ grep -q "fetch.*data/index\.json"                   # OK
$ node --check (via .mjs copy)                        # OK
```

### Task 2 (renderJob)

```
$ grep -n 'innerHTML' main.js                         # 0 matches — OK (after comment scrub)
$ grep -niE 'latency by lane' main.js                 # 0 matches — OK (after comment scrub)
$ grep -q 'Plot.line'                                 # 6 matches — OK (4 in source: Task 1's cross-seed + Task 2's renderChartPane; 2 in JSDoc-style comments)
$ grep -q "stroke:.*lane"                             # OK (charts 1 + 2 + comments)
$ grep -q "stroke:.*seed"                             # OK (cross-seed overlay)
$ grep -q 'pricing_event_stream_sha256'               # OK (Event-stream hash card)
$ grep -q 'priority_included'                         # OK (latency table)
$ grep -q 'no time-series available'                  # OK (Pitfall 8 placeholder)
$ node --check                                        # OK
```

### Plan-level verification block (end-to-end smoke)

```
$ cd sim-rs && python3 scripts/viz/build.py \
    --source scripts/viz/tests/fixtures/mini-suite --output /tmp/viz-05-final \
    --serve --port 8888 &
$ curl -sI http://127.0.0.1:8888/                     # HTTP/1.0 200 OK
$ curl -sI http://127.0.0.1:8888/data/index.json      # HTTP/1.0 200 OK
$ curl -s http://127.0.0.1:8888/static/main.js | grep -c 'Plot.line'   # 6
$ grep -nE 'innerHTML|latency by lane' scripts/viz/static/main.js      # 0 matches
```

### Existing tests (no regressions)

```
$ cd sim-rs && python3 -m unittest discover -s scripts/viz/tests -t scripts/viz
Ran 17 tests in 0.656s

OK
```

## Cross-seed overlay UX (VIZ-05 / D-15)

The suite view's "Cross-seed time-series overlay" section is interactive: two `<select>` controls (job + lane), default to no-job-selected (showing a "pick a job" placeholder). When the user picks a job, the section fetches every `data/<suite>/<job>-<seed>.json` for that job in parallel via `Promise.all`, filters each payload's `time_series` to `metric === "quote_per_byte" && lane === <chosen>`, flattens the records with `seed` lifted onto each one, and calls `Plot.line(flat, {x: "slot", y: "value", stroke: "seed"})` so each seed becomes a coloured line on the same x-axis. Errors (HTTP failure on any seed fetch, no records for the chosen lane) replace the chart container with a muted explanatory message instead of crashing the view.

Loading-state copy is honest about the count: "(loading 3 seeds...)" for a 3-seed job. Empty-state copy explains why a chart isn't shown: "(no quote_per_byte records for lane=standard across the 3 seeds of this job)".

## Aggregates section behaviour (CRITICAL LANDMINE #2 / Pitfall 3)

When `data.aggregates === null` (every phase-2 suite per Plan 01-02's locked contract), the suite view renders:

- NO `<h2>Suite aggregates</h2>` heading.
- NO `<p class="muted">aggregates pending</p>` placeholder.
- NO `<section class="aggregates">` empty container.

It appends a single HTML comment node (via `document.createComment(...)`) at the same position the section would have lived, carrying the full Pitfall 3 rationale so a future executor opening DevTools sees the why:

```
<!-- Pitfall 3: aggregates is null — phase-2 metrics writer (comparison.rs) emits no
 suite-level *.csv; the historical priority_only_fast_path_overall_comparison.csv lives
 under sim-rs/output/analysis/ and is not generated by current phase-2 suites. Render
 only when aggregates is non-null. See Plan 01-02 / Plan 01-05 / Pitfall 3 in RESEARCH.md. -->
```

When `data.aggregates` is non-null (a future enhancement for suites that DO emit a suite-root CSV), the render path is reserved: it appends an `<h2>Suite aggregates</h2>` followed by a key/value table. This branch is dead code today but parses + tests cleanly.

## Deviations from Plan

### Auto-fixed issues (Rules 1/2/3)

None.

### Plan-action clarifications (Plan 01-03 precedent)

**1. [Comment-token scrub — same class as Plan 01-03's clarification #1] Two comments in `renderJob` initially used the grep-gated tokens to describe the rules themselves**

- **Found during:** Task 2 verification gate run (initial `grep -nE 'innerHTML|latency by lane'` returned two hits, both inside explanatory comments — `// title attribute set via setAttribute, NOT innerHTML` and `// The "latency by lane" wording is intentionally NEVER used here`).
- **Fix:** Rewrote both comments to describe the rule without inlining the forbidden tokens, matching Plan 01-03's same precedent — "the unsafe HTML-string sink is grep-gated out of this file" and "the per-lane wording forbidden by Pitfall 5 is grep-gated out of this file". Semantically equivalent; grep-clean.
- **Why this is a clarification not a Rule 2 deviation:** the plan's acceptance criteria explicitly chose grep gates over context-aware checks. The behaviour was always to honour the gate; the comment phrasing was a momentary inattention.
- **Files modified:** `sim-rs/scripts/viz/static/main.js` (edited in-place before the Task 2 commit; not a separate commit).

### Auth gates

None.

### Auto-mode triggers

None — `auto_advance: false` in `.planning/config.json`. The Task 3 human-verify checkpoint is documented in the next section as pending visual verification.

## Task 3 — human-verify checkpoint status

Task 3 of the plan is a `checkpoint:human-verify` that walks all six VIZ requirements against the live `sim-rs/output/` tree. Structural gates are GREEN (above); visual verification is **pending** and is the user's call to invoke after merging:

```bash
cd sim-rs && python3 scripts/viz/build.py --serve --port 8765
# Open http://127.0.0.1:8765/ in a browser. Walk through:
#  - VIZ-01: suite list (default sort started_at desc, click columns to re-sort).
#  - VIZ-02: click a suite → manifest + (job, seed) table.
#  - VIZ-03: click a (job, seed) → headline strip with 6 cards + latency table.
#           Confirm the latency section heading reads "Latency by demand component (blocks)".
#  - VIZ-04: confirm 3 chart panes render — controller quote per lane (2 lines + legend),
#           mempool bytes (3 lines + legend), fees+refunds (2 lines + legend).
#  - VIZ-05: back on the suite view, pick a job in the overlay select → confirm one
#           coloured line per seed.
#  - Confirm DevTools shows NO red console errors and NO external network requests.
#  - Confirm an HTML comment with "Pitfall 3" is present where the aggregates panel would be.
```

Plan 01-03 and Plan 01-04 set the same precedent of deferring the visual checkpoint to a later confirmation step rather than blocking the SUMMARY. Plan 01-06 (Wave 4 — README + CLAUDE.md crumb + end-to-end checkpoint) is the next plan in the sequence and re-runs the live-tree verification.

## Stubs introduced

None — every stub from Plan 01-03 (`TBD - Plan 01-05` placeholders in `renderSuite`'s headline column + `renderJob`'s headline strip + three chart pane placeholders + `derived_quote per block` label) is replaced by this plan's real implementation. The verifier should observe zero `TBD` strings in `main.js`:

```
$ grep -c 'TBD' sim-rs/scripts/viz/static/main.js
0
```

## Sanity check vs plan targets

| Item | Target | Actual |
|------|--------|--------|
| `main.js` line count | grow from 362, no hard cap | 988 lines (+626; ~727 substance, ~261 comments + schema block) |
| `renderHome` sortable by 8 columns | yes (D-12, D-17) | yes (name, path, started_at, jobs, seeds, completed, max-concurrent-jobs, id) |
| Default home sort | started_at desc (D-18) | started_at desc (initial sortKey + sortDir constants) |
| `renderSuite` aggregates omitted when null | yes (CRITICAL LANDMINE #2 / Pitfall 3) | yes (no `<h2>`; HTML comment marker only) |
| `renderJob` 3 chart panes via Plot | yes (VIZ-04) | yes (`renderChartPane` invoked 3 times + 1 cross-seed Plot.plot call) |
| Cross-seed overlay uses `stroke: 'seed'` | yes (VIZ-05) | yes (`renderCrossSeedSection` → `Plot.line(flat, {stroke: "seed"})`) |
| Chart 1+2 use `stroke: 'lane'` | yes | yes |
| Chart 3 uses `stroke: 'metric'` | implied (both records lane=total) | yes |
| Empty time-series placeholder | yes (Pitfall 8) | yes ("(no time-series available for this seed — see build warnings)") |
| `innerHTML` count | 0 | 0 |
| `latency by lane` count (case-insensitive) | 0 | 0 |
| `Latency by demand component (blocks)` count | ≥ 1 | 1 (single canonical literal in the const declaration; 6 references via constant name) |
| `node --check` clean | yes | yes |
| Existing 17 tests pass | yes | yes |
| HTTP smoke (mini-suite, port 8888) | 200 OK for /, /data/index.json, /static/main.js | yes; served main.js has 6 Plot.line references |

## Next Phase Readiness

- **Plan 01-06 (Wave 4 — README + CLAUDE.md crumb + end-to-end checkpoint)** is the final plan in this phase. It documents the single `python sim-rs/scripts/viz/build.py --serve` command, adds a CLAUDE.md crumb, and runs the live-tree end-to-end visual checkpoint that this plan defers. The static bundle is operationally complete at this point.
- **No blockers.** Structural gates green; existing tests green; live HTTP smoke green; the only remaining gate is the visual confirmation in Plan 01-06's checkpoint or via the user's standalone browser session.

## Self-Check: PASSED

**Modified files (verified present on disk + line count):**
- FOUND: `sim-rs/scripts/viz/static/main.js` (988 lines)

**Commits (verified in `git log --oneline -3`):**
- FOUND: `d66eba7` feat(01-05): real renderHome + renderSuite with sortable tables and cross-seed overlay
- FOUND: `ea631f6` feat(01-05): real renderJob — headline strip + per-component latency + 3 Plot chart panes

**Security grep gates (verified):**
- `grep 'innerHTML' main.js` → 0 occurrences
- `grep -i 'latency by lane' main.js` → 0 occurrences
- `grep 'Latency by demand component (blocks)' main.js` → 1 occurrence (canonical literal)
- `grep 'HEADLINE_LATENCY_LABEL' main.js` → 7 occurrences (1 declaration + 6 usages)

**Plan-level verification block (verified):**
- HTTP/1.0 200 OK on /, /data/index.json, /static/main.js against `build.py --serve --port 8888`
- 6 `Plot.line` references in the served main.js
- 17/17 existing viz tests still pass

---
*Phase: 01-viz-site-mvp*
*Completed: 2026-05-20*
