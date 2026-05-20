---
phase: 01-viz-site-mvp
plan: 02
subsystem: ingest
tags: [python, stdlib, json, csv, observable-plot, three-tier, ingest-layer]

# Dependency graph
requires:
  - "Wave 0 test harness from Plan 01-01 (synthetic mini-suite + malformed-suite + no-time-series fixtures; 11 RED tests at sim-rs/scripts/viz/tests/test_ingest.py)"
provides:
  - "sim-rs/scripts/viz/build.py — stdlib-only ingest module with the locked five-name public API: discover_suites, load_seed, run_build, parse_args, main + the LANE_FIELDS constant"
  - "Three-tier JSON layout under <output>/data/: index.json + <suite_id>.json + <suite_id>/<job>-<seed>.json (D-09)"
  - "Locked on-disk → JSON contract for Plans 01-03 (static bundle) and 01-05 (browser views)"
affects:
  - 01-03-PLAN-static-bundle
  - 01-04-PLAN-serve-entry-point
  - 01-05-PLAN-browser-views

# Tech tracking
tech-stack:
  added:
    - "Python stdlib http.server / argparse / csv / fnmatch / json / pathlib — already approved by D-08; no requirements.txt"
  patterns:
    - "Path-derived suite identifier (`str(manifest_path.parent.relative_to(source)).replace('/', '__')`) avoids collisions when two manifests share `suite-name`"
    - "Kebab-case access on `manifest.json` (`suite-name`, `started-at-utc`, `jobs[job][seed].started-at-utc`); snake_case access on `run_summary.json` (`priority_retained_value_total`, `latency_blocks_observations`)"
    - "Latency-list-to-mean reduction at build time (`sum(obs)/len(obs)` with `0.0` empty fallback) matches the Rust accessor `ComponentSummary::latency_blocks_mean()`"
    - "CSV → long-form `{slot, lane, metric, value}` records via a generator + `LANE_FIELDS` constant; browser never parses CSV"
    - "Skip-and-warn error model: every per-file failure appends to `warnings`, build never exits non-zero (D-21)"
    - "Interval-overlap sweep (+1/-1 events) as parallelism proxy when the runner doesn't persist that field (RESEARCH.md Open Q #1)"

key-files:
  created:
    - sim-rs/scripts/viz/build.py
    - sim-rs/scripts/viz/__init__.py
  modified: []

key-decisions:
  - "Stdlib-only — no off-the-shelf framework, no Cargo binary, no requirements.txt, no virtualenv (D-01 / D-02 / D-05 / D-08 of the phase CONTEXT)"
  - "`aggregates: null` unconditionally for every phase-2 suite (CRITICAL LANDMINE #2 / Pitfall 3) — no phase-2 suite emits a suite-root CSV; reserved field gated by null in the browser render path"
  - "`metrics_comparison.txt` is NEVER opened (Pitfall 4) — every field in it is also in `run_summary.json`"
  - "`max_concurrent_jobs` derived via interval-overlap sweep from manifest timestamps (RESEARCH.md Open Q #1 proxy); null when fewer than 2 (job, seed) entries have parseable timestamps"
  - "Real-world manifests in `sim-rs/output/` contain schema variants (jobs-as-list, None-valued required fields); broadened the catch to include AttributeError + TypeError so D-21 skip-and-warn engages cleanly instead of crashing"
  - "Dropped `util_priority_window_x_1e9` + `util_standard_window_x_1e9` from `LANE_FIELDS` for v1 — every chart VIZ-04 / D-14 calls out is covered by the 12 remaining entries; future iterations can extend"

patterns-established:
  - "Pattern: One module owns the entire ingest pipeline; three section comments (`# ----- discovery layer / ingest layer / emit + CLI layer`) split it for review"
  - "Pattern: Helper-prefixed underscore convention (`_iso_to_epoch`, `_max_concurrent_jobs`, `_read_time_series_long`, `_build_suite_json`, `_emit_seed_json`) — public five names are exactly the ones consumed by tests + the browser bundle"
  - "Pattern: `json.dump(..., indent=2, sort_keys=True)` everywhere so emission is deterministic across runs"

requirements-completed: [VIZ-01, VIZ-02, VIZ-03, VIZ-04, VIZ-05]

# Metrics
duration: 9min
completed: 2026-05-20
---

# Phase 01 Plan 02: viz-website Wave 2 ingest layer Summary

**Stdlib-only Python ingest module (`sim-rs/scripts/viz/build.py`) that walks `sim-rs/output/` for `manifest.json` files and emits the three-tier `index.json` / `<suite>.json` / `<suite>/<job>-<seed>.json` JSON layout the browser consumes; turns all 11 Wave 0 RED tests GREEN.**

## Performance

- **Duration:** ~9 min
- **Started:** 2026-05-20T11:49:36Z
- **Completed:** 2026-05-20T11:58:41Z
- **Tasks:** 3 (+ 1 auto-fix commit, Rule 2)
- **Files created:** 2
- **`sim-rs/scripts/viz/build.py` line count:** 727 LOC (plan sanity target: ~250-450; the module came in higher because every helper carries an inline docstring linking it to the originating Pitfall + CONTEXT.md decision, and the defensive shape-guards added by the Rule 2 fix expand `_build_suite_json` and `_max_concurrent_jobs`. Substance-bearing code is ~430 LOC; docstrings + comments are ~300 LOC.)

## Public API

```python
# sim-rs/scripts/viz/build.py
LANE_FIELDS                                                          # 12-tuple list (CSV col → (lane, metric))

discover_suites(source, includes, excludes, warnings) -> list[dict]  # walk + filter + skip-and-warn
load_seed(seed_dir, warnings) -> dict | None                          # per-(job, seed) ingest
run_build(source, output, includes, excludes, warnings) -> None       # top-level orchestrator
parse_args() -> argparse.Namespace                                    # --source / --output / --include / --exclude
main() -> None                                                         # parse_args + run_build + warnings → stderr
```

## LANE_FIELDS (CSV → long-form mapping)

| CSV column                  | lane     | metric                    |
|-----------------------------|----------|---------------------------|
| `c_priority`                | priority | quote_per_byte            |
| `c_standard`                | standard | quote_per_byte            |
| `mempool_bytes_total`       | total    | mempool_bytes             |
| `mempool_bytes_priority`    | priority | mempool_bytes             |
| `mempool_bytes_standard`    | standard | mempool_bytes             |
| `included_bytes_priority`   | priority | included_bytes            |
| `included_bytes_standard`   | standard | included_bytes            |
| `included_count_priority`   | priority | included_count            |
| `included_count_standard`   | standard | included_count            |
| `fees_paid_lovelace`        | total    | fees_paid_lovelace        |
| `refund_lovelace`           | total    | refund_lovelace           |
| `evicted_quote_drift_count` | total    | evicted_quote_drift_count |

`util_priority_window_x_1e9` and `util_standard_window_x_1e9` are intentionally dropped on the floor in v1 — every VIZ-04 / D-14 pane is covered by the 12 entries above. A future iteration can extend `LANE_FIELDS` and widen the front-end metric whitelist together.

## Function → Pitfall / Decision Map

| Helper                       | Guards                                                       |
|------------------------------|--------------------------------------------------------------|
| `_iso_to_epoch`              | RESEARCH.md "Don't Hand-Roll" — stdlib `fromisoformat`      |
| `_max_concurrent_jobs`       | Open Q #1 (parallelism not persisted by the runner)         |
| `_matches_globs`             | D-10 (`--include` / `--exclude`)                            |
| `discover_suites`            | D-22 / Pitfall 2 (path-derived suite id), D-21 (skip-and-warn) |
| `_read_time_series_long`     | D-11 / Pattern F (CSV → long-form), no f64 path             |
| `load_seed`                  | Pitfall 1 (kebab/snake split), Pitfall 5 (latency mean), Pitfall 8 (missing CSV soft-fail) |
| `_seed_headline`             | VIZ-05 (cross-seed overlay from single suite JSON round-trip) |
| `_emit_seed_json`            | D-09 tier-3 path                                            |
| `_build_suite_json`          | Pitfall 3 / LANDMINE #2 (`aggregates: null`), Pitfall 4 (no `metrics_comparison.txt` open) |
| `_suite_index_entry`         | Open Q #1 (max_concurrent_jobs)                             |
| `run_build`                  | D-21 broad skip-and-warn around per-suite failures          |
| `parse_args`                 | Pattern B (argparse + `type=Path`)                          |
| `main`                       | Warnings to `sys.stderr` (Pattern A diagnostic discipline)  |

## Task Commits

Each task was committed atomically:

1. **Task 1: Discovery layer scaffold + LANE_FIELDS + `__init__.py`** — `8040ae1` (feat)
2. **Task 2: Per-(job, seed) ingest layer (`load_seed`, `_read_time_series_long`)** — `b46646c` (feat)
3. **Task 3: Three-tier emit + CLI orchestration (`_build_suite_json`, `run_build`, `parse_args`, `main`)** — `3950dab` (feat)
4. **Rule 2 fix: defensive guards for real-world manifest shape variance** — `e690085` (fix)

## Files Created/Modified

- `sim-rs/scripts/viz/__init__.py` — empty package marker so the test harness's `import build` (via `sys.path.insert(0, scripts/viz)`) resolves cleanly
- `sim-rs/scripts/viz/build.py` — the entire ingest pipeline (727 LOC)

## Wave 0 Test Status — RED → GREEN

```
cd sim-rs && python3 -m unittest discover -s scripts/viz/tests -t scripts/viz
Ran 11 tests in 0.014s
OK
```

| Test class :: method                                                       | Pitfall / Req | Status |
|----------------------------------------------------------------------------|---------------|--------|
| `IndexBuildTest::test_index_lists_all_manifests`                          | VIZ-01        | PASS   |
| `SuiteJsonTest::test_jobs_match_manifest`                                 | VIZ-02        | PASS   |
| `SuiteJsonTest::test_seed_grouping_present`                               | VIZ-05        | PASS   |
| `SeedJsonTest::test_headline_fields_present`                              | VIZ-03        | PASS   |
| `SeedJsonTest::test_time_series_long_form`                                | VIZ-04        | PASS   |
| `IngestTest::test_kebab_case_manifest_snake_case_run_summary`             | Pitfall 1     | PASS   |
| `IngestTest::test_suite_id_derived_from_path_not_suite_name`              | Pitfall 2 / D-22 | PASS |
| `IngestTest::test_phase_2_has_no_priority_only_fast_path_csv`             | Pitfall 3     | PASS   |
| `IngestTest::test_latency_blocks_observations_aggregated_to_mean`         | Pitfall 5     | PASS   |
| `IngestTest::test_missing_time_series_csv_returns_empty_list_with_warning` | Pitfall 8    | PASS   |
| `ErrorHandlingTest::test_malformed_manifest_skipped_with_warning`         | D-21          | PASS   |

## End-to-End Verification

### Mini-suite fixture smoke

```bash
cd sim-rs && rm -rf /tmp/viz-out-check && \
  python3 scripts/viz/build.py \
    --source scripts/viz/tests/fixtures/mini-suite \
    --output /tmp/viz-out-check
# Wrote /tmp/viz-out-check/data/index.json (source=scripts/viz/tests/fixtures/mini-suite)

ls /tmp/viz-out-check/data/
# index.json  mini-suite  mini-suite.json

find /tmp/viz-out-check/data -type f -name '*.json' | sort
# /tmp/viz-out-check/data/index.json
# /tmp/viz-out-check/data/mini-suite.json
# /tmp/viz-out-check/data/mini-suite/d8_target0.5_window32-1.json
# /tmp/viz-out-check/data/mini-suite/d8_target0.5_window32-2.json
```

Per-suite JSON:
- `aggregates: null` — Pitfall 3 / LANDMINE #2 enforced
- `jobs.d8_target0.5_window32.seeds.{1,2}.headline.{retained_value, net_utility, retained_value_ratio, peak_mempool_bytes, components}`
- `manifest` round-tripped verbatim (kebab-case keys intact)

Per-(job, seed) JSON:
- `time_series` = 60 long-form records (5 CSV rows × 12 LANE_FIELDS)
- `components[i].latency_blocks_mean` computed from observations list
- `peak_mempool_bytes = 316505` (max over mempool_bytes/total)
- `retained_value_ratio = 0.5` (priority+standard retained / priority+standard included lovelace)

### Live `sim-rs/output/` tree smoke (1884 manifests)

```bash
cd sim-rs && python3 scripts/viz/build.py --source output --output /tmp/viz-smoke-real
# 1884 suites discovered; ~50 warnings for schema-variant manifests
# (jobs-as-list, None-valued required fields), zero crashes
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical handling] Defensive guards against real-world manifest shape variance**
- **Found during:** Post-Task-3 smoke against the live `sim-rs/output/` tree
- **Issue:** Live tree contains manifests with `jobs` as a list (legacy / hand-edited at audit time under `sim-rs/output/v11-test/`) and `run_summary.json` files with `None` in nominally-required fields. `_build_suite_json` and `_max_concurrent_jobs` walked these shapes with `.items()` / `.values()` and crashed with `AttributeError` / `TypeError`. The outer try/except in `run_build` caught only `(OSError, KeyError, ValueError)` — the new exception types leaked through and aborted the entire build.
- **Why this is a correctness requirement (Rule 2):** D-21 mandates skip-and-warn — the build must never exit non-zero on data errors. A single bad manifest taking down a 1884-suite build is a hard violation of that contract.
- **Fix:** (a) Type-guard each level of the jobs walk inside `_max_concurrent_jobs` and `_build_suite_json`, emitting a warning and skipping the affected suite / job / entry instead of crashing. (b) Broadened both try/except clauses in `run_build` to include `AttributeError` and `TypeError`. (c) Added a separate try/except around the per-suite emit + index-entry append so a failure there doesn't drop earlier successful work.
- **Files modified:** `sim-rs/scripts/viz/build.py`
- **Commit:** `e690085`

**2. [Plan-action drift] Removed unused `import functools`**
- **Found during:** Task 3 cleanup
- **Issue:** Task 1's plan said to include `functools` because the eventual `serve()` helper (lands in Plan 01-04) uses `functools.partial`. Importing it without using it would leave a dead import in the module on plan completion.
- **Fix:** Dropped the `import functools` line. Plan 01-04 will add it back when it ships `serve()`.
- **Files modified:** `sim-rs/scripts/viz/build.py`
- **Commit:** `3950dab` (rolled into the Task 3 commit since the change is mechanical)

## Issues Encountered

None beyond the auto-fixed schema-variance issue above. The plan as written assumed the runner's `Manifest` schema held universally across `sim-rs/output/`; the live tree contains older / non-canonical manifests that don't. The plan's own `<deviation_rules>` Rule 2 covers this case cleanly.

## User Setup Required

None — stdlib-only, no virtualenv, no API keys, no external services.

## Sanity Check vs Plan Targets

| Item                        | Target           | Actual                                                    |
|-----------------------------|------------------|-----------------------------------------------------------|
| `build.py` line count       | ~250-450 LOC     | 727 LOC (substance ~430, docstrings/comments ~300)        |
| Public API names exported   | 5 + LANE_FIELDS  | 5 + LANE_FIELDS                                           |
| `LANE_FIELDS` entries       | ≥ 12             | 12 (util_*_window_x_1e9 dropped per design)               |
| Wave 0 tests passing        | 11/11 green      | 11/11 PASS                                                |
| Mini-suite smoke succeeds   | three-tier emit  | three-tier emit verified                                  |
| `aggregates: null` everywhere | yes            | yes (unconditional in `_build_suite_json`)                |
| `.gitignore` untouched      | yes              | yes (`git diff sim-rs/.gitignore` = empty)                |
| Stdlib-only                 | yes              | yes (`grep '^import (yaml|requests|pytest|jinja2|flask)' = empty`) |

## Next Phase Readiness

- **Plan 01-03 (Wave 2 — static bundle, parallel-eligible)** lands the HTML/CSS/JS browser shell. The JSON contract this plan emits is locked: `index.json` + `<suite>.json` + `<suite>/<job>-<seed>.json` shapes are exactly as the test harness asserts. The static bundle can be developed against the mini-suite fixture output (`/tmp/viz-out-check/data/`) before Plan 01-04 wires the serve helper.
- **Plan 01-04 (Wave 3 — serve entry-point)** adds `--serve` + `--port` flags and the `copy_static_assets` step. The argparse stub in `parse_args()` is structured so that wave just adds two flags; the `main()` body adds a `--serve` branch that calls a new `serve()` helper. The TODO comment `# Plan 01-04 lands the ``--serve`` branch` in `main()`'s docstring is the seam.
- **Plan 01-05 (Wave 3 — browser views)** consumes the JSON shapes exactly as emitted today. The schema's `aggregates: null` gate is the explicit signal that the browser must NOT render a suite-aggregates panel in v1.
- **No blockers.** Wave 2 ingest is complete; Wave 0 tests are green; the live-tree smoke confirms D-21 skip-and-warn is operationally correct.

## Self-Check: PASSED

**Created files (verified present on disk):**
- FOUND: sim-rs/scripts/viz/__init__.py
- FOUND: sim-rs/scripts/viz/build.py (727 LOC)

**Commits (verified in git log):**
- FOUND: 8040ae1 (Task 1 — discovery layer scaffold)
- FOUND: b46646c (Task 2 — per-(job, seed) ingest)
- FOUND: 3950dab (Task 3 — three-tier emit + CLI)
- FOUND: e690085 (Rule 2 fix — schema-variance hardening)

**Tests (verified green):**
- 11/11 Wave 0 tests PASS (`python3 -m unittest discover -s scripts/viz/tests -t scripts/viz`)

---
*Phase: 01-viz-site-mvp*
*Completed: 2026-05-20*
