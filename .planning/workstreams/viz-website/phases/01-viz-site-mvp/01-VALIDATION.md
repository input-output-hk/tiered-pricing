---
phase: 1
slug: viz-site-mvp
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-20
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Python stdlib `unittest` (no precedent in repo for Python tests — see RESEARCH.md §Validation Architecture) |
| **Config file** | none — tests under `sim-rs/scripts/viz/tests/` discovered via `python -m unittest discover` |
| **Quick run command** | `python -m unittest discover -s sim-rs/scripts/viz/tests -t sim-rs/scripts/viz` |
| **Full suite command** | `python -m unittest discover -s sim-rs/scripts/viz/tests -t sim-rs/scripts/viz` + HTTP smoke (`scripts/viz/tests/smoke.sh`) |
| **Estimated runtime** | ~10 seconds (build script unit tests) + ~5 seconds (HTTP smoke) |

---

## Sampling Rate

- **After every task commit:** Run quick command
- **After every plan wave:** Run full suite command
- **Before `/gsd-verify-work`:** Full suite must be green AND a manual visual spot-check of all three views has been performed
- **Max feedback latency:** ~15 seconds

---

## Per-Task Verification Map

Per-task entries populated by the planner — each plan task should declare a row here mapping (Task ID, Plan, Wave, Requirement, Test Type, Command). The planner fills this when emitting PLAN.md files.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | — | — | — | — | — | — | — | — | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `sim-rs/scripts/viz/tests/__init__.py` — package marker
- [ ] `sim-rs/scripts/viz/tests/test_ingest.py` — schema-parse stubs covering manifest.json (kebab-case), run_summary.json (snake_case), time_series.csv columns
- [ ] `sim-rs/scripts/viz/tests/fixtures/` — one minimal real suite directory snapshot (or a curated synthetic one) so tests don't depend on `sim-rs/output/` state
- [ ] `sim-rs/scripts/viz/tests/smoke.sh` — HTTP smoke: builds bundle against a fixture, starts server on a fixed port, curls `/`, `/index.json`, asserts HTTP 200 + non-empty JSON; tears down

*Wave 0 establishes the test harness before any business code lands. The build script is pure functions plus IO seams — easy to unit-test once the harness exists.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Suite list renders ~100 suites with readable metadata | VIZ-01 | Visual layout / readability is subjective | Run `python sim-rs/scripts/viz/build.py --serve`, open `http://localhost:<port>/`, confirm suite count matches `find sim-rs/output/phase-2 -name manifest.json \| wc -l` |
| Suite drill-down shows manifest summary + per-job/per-seed inventory | VIZ-02 | Layout judgement | Click any suite row, confirm manifest fields render and job × seed table is sortable |
| Headline metrics page renders 5 numbers (retained_value, net_utility, retained_value_ratio, latency-by-component, peak mempool depth) | VIZ-03 | Numeric correctness vs source is subjective per-row check | Open a (job, seed) detail page; cross-check 1–2 numbers against `cat run_summary.json` |
| Time-series charts render multi-line with lane colouring | VIZ-04 | Visual chart correctness | Open per-(job, seed) detail; confirm 3+ panes (controller `quote_per_byte` per lane, mempool size, `derived_quote`) render with distinguishable lane colours and legend |
| In-suite cross-seed overlay renders | VIZ-05 (per RESEARCH.md correction) | Visual chart correctness | Open suite drill-down; trigger cross-seed overlay view; confirm N seeds overlay on time-series |
| Single command produces a viewable site against the live `sim-rs/output/` tree | VIZ-06 | End-to-end UX is a manual confirmation | Run `python sim-rs/scripts/viz/build.py --serve` from a fresh clone (or after `git clean -fdx sim-rs/output/viz/`); confirm site is reachable in <60s |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
