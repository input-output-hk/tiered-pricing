---
phase: 01-viz-site-mvp
plan: 01
subsystem: testing
tags: [python, unittest, fixtures, json, csv, observable-plot, test-harness]

# Dependency graph
requires: []
provides:
  - "Synthetic fixture trees that encode the kebab-case manifest.json and snake_case run_summary.json verified schemas plus the pinned 15-column time_series.csv header"
  - "Failing test scaffolds (5 classes, 11 methods) covering VIZ-01..VIZ-05 and Pitfalls 1, 2, 3, 5, 8 plus D-21 / D-22"
  - "Locked Phase Requirements -> Test Map: Plan 01-02 (ingest) and Plan 01-03 (static bundle) build against this harness"
affects:
  - 01-02-PLAN-ingest
  - 01-03-PLAN-static-bundle

# Tech tracking
tech-stack:
  added:
    - "Python stdlib unittest (no precedent in repo for Python tests)"
  patterns:
    - "Fixtures co-located under sim-rs/scripts/viz/tests/fixtures/ — checked-in synthetic data, no shutil.copy from sim-rs/output/"
    - "Try/except import guard around build module so unittest discover prints skipped tests rather than crashing during Wave 0"
    - "FIXTURES = Path(__file__).resolve().parent / 'fixtures' (cwd-independent fixture lookup)"

key-files:
  created:
    - sim-rs/scripts/viz/tests/__init__.py
    - sim-rs/scripts/viz/tests/fixtures/mini-suite/manifest.json
    - sim-rs/scripts/viz/tests/fixtures/mini-suite/d8_target0.5_window32/1/run_summary.json
    - sim-rs/scripts/viz/tests/fixtures/mini-suite/d8_target0.5_window32/1/time_series.csv
    - sim-rs/scripts/viz/tests/fixtures/mini-suite/d8_target0.5_window32/2/run_summary.json
    - sim-rs/scripts/viz/tests/fixtures/mini-suite/d8_target0.5_window32/2/time_series.csv
    - sim-rs/scripts/viz/tests/fixtures/malformed-suite/manifest.json
    - sim-rs/scripts/viz/tests/fixtures/no-time-series/manifest.json
    - sim-rs/scripts/viz/tests/fixtures/no-time-series/job_a/1/run_summary.json
    - sim-rs/scripts/viz/tests/test_ingest.py
  modified: []

key-decisions:
  - "Fixtures are checked-in JSON/CSV under tests/fixtures/, not copies from sim-rs/output/ — keeps tests deterministic and runnable in a fresh checkout."
  - "Mini-suite seed 2 has different numeric values from seed 1 (mempool sizes, c_priority deltas, fees) so cross-seed overlay tests can assert lines do not collapse into one."
  - "Latency observations list lengths differ across seeds and components (8-11 entries) so the mean computation Pitfall 5 codifies is non-trivial."
  - "Suite-id-from-path test (Pitfall 2 / D-22) builds two transient suites whose suite-name keys collide, then asserts the build emits two distinct ids containing '__' as the path-derived separator."
  - "Test for Pitfall 3 (no priority_only_fast_path_overall_comparison.csv in phase-2 tree) accepts None or empty list/dict as the aggregates field — gives Plan 01-02 implementation flexibility."

patterns-established:
  - "Pattern: Wave 0 = checked-in synthetic fixtures + try/except-guarded import of the target module + skipped tests until Wave 1 lands."
  - "Pattern: Each test docstring names the Pitfall number from RESEARCH.md so failures point straight at the documented landmine."
  - "Pattern: time_series.csv header copied verbatim from sim-cli/src/metrics/time_series.rs lines 16-20 — paraphrasing would break the kebab-vs-snake casing landmine."

requirements-completed: [VIZ-01, VIZ-02, VIZ-03, VIZ-04, VIZ-05]

# Metrics
duration: 3min
completed: 2026-05-20
---

# Phase 01 Plan 01: viz-website Wave 0 test harness Summary

**Failing unittest scaffolds plus synthetic kebab/snake manifest + 15-col time-series fixtures that lock the Phase Requirements -> Test Map for VIZ-01..VIZ-05 before any business code lands**

## Performance

- **Duration:** 3 min
- **Started:** 2026-05-20T11:41:44Z
- **Completed:** 2026-05-20T11:44:53Z
- **Tasks:** 2
- **Files created:** 10

## Accomplishments

- **Three checked-in fixture trees under `sim-rs/scripts/viz/tests/fixtures/`** encoding (a) a valid mini-suite (kebab-case manifest, snake_case run_summary, pinned 15-column time_series.csv, one job, two seeds with distinct numeric values), (b) a malformed-suite (truncated JSON to exercise the D-21 skip-and-warn path), (c) a no-time-series fixture (valid manifest + run_summary, no time_series.csv) to exercise the Pitfall 8 soft-failure path.
- **Failing-test scaffold `tests/test_ingest.py`** with 5 classes and 11 test methods covering the Phase Requirements -> Test Map locked in 01-RESEARCH.md and the five documented landmines (Pitfalls 1, 2, 3, 5, 8) plus D-21 and D-22.
- **Wave 0 RED state achieved cleanly:** `python -m unittest discover` produces `Ran 11 tests in 0.000s / OK (skipped=11)` with no import error — build module import is guarded by `try/except ImportError` so Plan 01-02 lands `build.run_build()` and converts skipped -> green incrementally.

## Task Commits

Each task was committed atomically:

1. **Task 1: Author three fixture trees with verified-schema files** - `afd31d0` (test)
2. **Task 2: Write failing test scaffolds in test_ingest.py** - `21382ab` (test)

## Files Created/Modified

- `sim-rs/scripts/viz/tests/__init__.py` - empty Python package marker
- `sim-rs/scripts/viz/tests/fixtures/mini-suite/manifest.json` - kebab-case suite manifest (one job, two seeds)
- `sim-rs/scripts/viz/tests/fixtures/mini-suite/d8_target0.5_window32/1/run_summary.json` - snake_case run_summary with 2-component latency_blocks_observations lists (8 + 10 floats)
- `sim-rs/scripts/viz/tests/fixtures/mini-suite/d8_target0.5_window32/1/time_series.csv` - 15-column header verbatim + 5 integer rows
- `sim-rs/scripts/viz/tests/fixtures/mini-suite/d8_target0.5_window32/2/run_summary.json` - seed 2 with distinct numeric values (9 + 11 floats)
- `sim-rs/scripts/viz/tests/fixtures/mini-suite/d8_target0.5_window32/2/time_series.csv` - seed 2 time-series with distinct values for cross-seed overlay
- `sim-rs/scripts/viz/tests/fixtures/malformed-suite/manifest.json` - bare opening brace; raises json.JSONDecodeError
- `sim-rs/scripts/viz/tests/fixtures/no-time-series/manifest.json` - valid manifest (one job, one seed)
- `sim-rs/scripts/viz/tests/fixtures/no-time-series/job_a/1/run_summary.json` - valid run_summary; no sibling time_series.csv
- `sim-rs/scripts/viz/tests/test_ingest.py` - 5 classes, 11 test methods, try/except-guarded `import build`

## Tests -> Pitfall/Requirement Map

| Test class :: method | Codifies | Source |
|---------------------|----------|--------|
| `IndexBuildTest::test_index_lists_all_manifests` | VIZ-01 | RESEARCH.md Phase Requirements -> Test Map |
| `SuiteJsonTest::test_jobs_match_manifest` | VIZ-02 | RESEARCH.md Phase Requirements -> Test Map |
| `SuiteJsonTest::test_seed_grouping_present` | VIZ-05 (cross-seed overlay) | RESEARCH.md Phase Requirements -> Test Map |
| `SeedJsonTest::test_headline_fields_present` | VIZ-03 | RESEARCH.md Phase Requirements -> Test Map |
| `SeedJsonTest::test_time_series_long_form` | VIZ-04 | RESEARCH.md Phase Requirements -> Test Map |
| `IngestTest::test_kebab_case_manifest_snake_case_run_summary` | Pitfall 1 (casing landmine) | RESEARCH.md Common Pitfalls |
| `IngestTest::test_suite_id_derived_from_path_not_suite_name` | Pitfall 2 / D-22 (suite-id collision) | RESEARCH.md Common Pitfalls, CONTEXT.md D-22 |
| `IngestTest::test_phase_2_has_no_priority_only_fast_path_csv` | Pitfall 3 (no per-suite aggregate CSV in phase-2) | RESEARCH.md Common Pitfalls |
| `IngestTest::test_latency_blocks_observations_aggregated_to_mean` | Pitfall 5 (list-not-scalar) | RESEARCH.md Common Pitfalls |
| `IngestTest::test_missing_time_series_csv_returns_empty_list_with_warning` | Pitfall 8 (phase-3 has no time-series) | RESEARCH.md Common Pitfalls |
| `ErrorHandlingTest::test_malformed_manifest_skipped_with_warning` | D-21 (skip-and-warn) | CONTEXT.md D-21 |

## Decisions Made

- **Mini-suite is one job × two seeds, not multiple jobs × multiple seeds.** Minimum data to exercise the kebab/snake split, the latency-list-to-mean computation, and the cross-seed overlay (VIZ-05). Larger fixtures would not catch additional bugs.
- **Latency observation lengths differ across components and across seeds (8, 10, 9, 11).** Makes the Pitfall 5 mean computation non-trivial — a buggy implementation that returns the first observation or the length of the list would still fail the float-equality check.
- **Malformed-suite is a bare opening brace `{`.** Simplest input that reliably raises `json.JSONDecodeError` on Python 3.10+ regardless of decoder version.
- **No-time-series fixture has only one (job, seed) pair.** Exercises Pitfall 8 without ballooning fixture count — the assertion in `test_missing_time_series_csv_returns_empty_list_with_warning` only needs one seed file to verify the empty-list + None-peak + warning string contract.
- **`FIXTURES = Path(__file__).resolve().parent / "fixtures"`.** Fixture lookup is independent of the caller's working directory so `cd sim-rs && python -m unittest discover -s scripts/viz/tests -t scripts/viz` works regardless of where the user runs it from.

## Deviations from Plan

None at the task level — plan executed exactly as written.

**Note on requirement traceability:** The plan's frontmatter declares
`requirements: [VIZ-01, VIZ-02, VIZ-03, VIZ-04, VIZ-05]` so the SDK
checks those boxes in REQUIREMENTS.md on plan completion. Wave 0 only
locks the test harness; the user-facing UI behaviour those VIZ-NN
acceptance criteria describe is delivered by Plans 01-02 (ingest),
01-03 (static bundle), 01-04 (serve), 01-05 (browser views), 01-06
(end-to-end). The checkboxes are a planner-declared traceability
ownership signal, not a behavioural assertion. Downstream plans will
re-validate against the live `sim-rs/output/` tree.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **Plan 01-02 (Wave 1 ingest)** can now `import build` from the same test path and incrementally turn 11 skipped tests into 11 passing tests. The contract for `build.run_build(source, output, includes, excludes, warnings)` is locked by these tests' call sites.
- **Plan 01-03 (Wave 1 static bundle)** has no direct dependency on this harness — it ships HTML/JS/CSS that the build script copies. The harness does pin the per-(job, seed) JSON shape (long-form `{slot, lane, metric, value}` records) that `static/main.js` will consume.
- **No blockers.** Wave 0 RED state is the expected outcome; Plan 01-02 lands the build module to convert RED -> GREEN.

## Self-Check: PASSED

**Created files (verified present on disk):**
- FOUND: sim-rs/scripts/viz/tests/__init__.py
- FOUND: sim-rs/scripts/viz/tests/fixtures/mini-suite/manifest.json
- FOUND: sim-rs/scripts/viz/tests/fixtures/mini-suite/d8_target0.5_window32/1/run_summary.json
- FOUND: sim-rs/scripts/viz/tests/fixtures/mini-suite/d8_target0.5_window32/1/time_series.csv
- FOUND: sim-rs/scripts/viz/tests/fixtures/mini-suite/d8_target0.5_window32/2/run_summary.json
- FOUND: sim-rs/scripts/viz/tests/fixtures/mini-suite/d8_target0.5_window32/2/time_series.csv
- FOUND: sim-rs/scripts/viz/tests/fixtures/malformed-suite/manifest.json
- FOUND: sim-rs/scripts/viz/tests/fixtures/no-time-series/manifest.json
- FOUND: sim-rs/scripts/viz/tests/fixtures/no-time-series/job_a/1/run_summary.json
- FOUND: sim-rs/scripts/viz/tests/test_ingest.py

**Commits (verified in git log):**
- FOUND: afd31d0 (Task 1 — fixtures)
- FOUND: 21382ab (Task 2 — test scaffolds)

---
*Phase: 01-viz-site-mvp*
*Completed: 2026-05-20*
