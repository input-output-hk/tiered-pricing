---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: executing
stopped_at: Phase 1 plan 02 completed; Phase 1 complete
last_updated: "2026-05-15T13:30:00.000Z"
last_activity: 2026-05-15 -- Phase 1 plan 02 completed (realism-risks register v1 finalised)
progress:
  total_phases: 5
  completed_phases: 1
  total_plans: 2
  completed_plans: 2
  percent: 20
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-05-15)

**Core value:** A reader of the Cardano Improvement Proposal (CIP) can verify each menu-option claim against a specific simulator job and can inspect the realism-risks register to see what the simulator does and does not faithfully model, so the CIP stands on documented evidence rather than asserted authority.
**Current focus:** Phase 1 — Register Inventory

## Current Position

Phase: 1 (Register Inventory) — COMPLETE
Plan: 2 of 2 completed
Status: Phase 1 complete; ready to begin Phase 2 (Coverage Check Skeleton)
Last activity: 2026-05-15 -- Phase 1 plan 02 completed (realism-risks register v1 finalised)

Progress: [██░░░░░░░░] 20%

## Performance Metrics

**Velocity:**

- Total plans completed: 2
- Average duration: ~30 min
- Total execution time: ~1 hour

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1. Register Inventory | 2 | ~1h | ~30 min |

**Recent Trend:**

- Last 5 plans: 01-01 (~30 min), 01-02 (~30 min)
- Trend: stable execution duration; both plans documentation-only

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- Milestone init: CIP shape is single-document menu of mechanism options; user authors the CIP (Claude produces only the evidence base)
- Milestone init: Realism-risks audit pattern is register + targeted cheap tests (not register-only, not full re-audit)
- Milestone init: Pool-number sensitivity test is the prototype-pattern for the realism-risks audit (sundaeswap_moderate + 4 paper_like variants × {100, 150 pools})
- Milestone init: EIP-1559 demoted from menu to control-only, per research-stakeholder request
- Milestone init: 600-pool CIP-0164 migration superseded by the cheaper sensitivity test; M6 plan stays in tree as contingency
- 2026-05-14: Family B (chain-derived EIP-1559-faithful controller) committed for publication
- Phase 1 / plan 02: register v1 verdict distribution = 12 LIVE + 12 DISCLOSED (no DORMANT, no MITIGATED in v1); MITIGATED reserved until Phase 3 test results land
- Phase 1 / plan 02: substrate-scope verdict is LIVE-going-to-DISCLOSED (Phase 4 / DOC-01 flips it after the disclosure-paragraph folds into the refreshed audit)
- Phase 1 / plan 02: three new EXP-NN slugs surfaced — EXP-unresolved-output-read (Phase 2 / COV-06), EXP-coverage-non-welfare-columns (Phase 2 / COV-03), EXP-hash-diversity-policy-decision (Phase 3 / COV-05); none requires a new TEST-NN sub-requirement
- Phase 1 / plan 02: one new TEST sub-requirement surfaced — TEST-07a (EXP-multiplier-floor-16-companion-run) for Phase 3

### Pending Todos

None yet.

### Blockers/Concerns

None yet.

**Open questions to resolve during phase execution** (from research/SUMMARY.md, none block Phase 1):

- N for multi-seed variance bands: measure wall-clock cost before committing N=30; N=20 with explicit Bias-corrected and accelerated (BCa) coverage disclosure is the fallback (resolve in Phase 3 via TEST-02)
- Hash-diversity policy strictness: strict (re-run with different seeds) vs soft (mark WEAK with annotation); decide before Phase 3 begins
- Controller-knob anchor availability: two-hour literature search at Phase 4 start determines anchor vs disclose for window-length 32 especially

## Deferred Items

Items acknowledged and carried forward from previous milestone close (see PROJECT.md "Out of Scope" for full list):

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Modelling | Adversarial / strategic-bidder regime | Deferred; disclosed as future work | 2026-05-15 |
| Infrastructure | Cross-architecture continuous integration (CI) verification | Deferred; intra-arch sufficient | Inherited from phase-2 |
| Topology | 600-pool CIP-0164 migration (`m6-implementation-plan.md`) | Superseded by TEST-05; contingency only | 2026-05-15 |
| Substrate | Re-auditing upstream Leios simulator code paths | Disclosed as substrate-scope limitation | 2026-05-15 |

## Session Continuity

Last session: 2026-05-15T13:30:00.000Z
Stopped at: Phase 1 complete (plans 01-01 and 01-02 done); ready to begin Phase 2 (Coverage Check Skeleton)
Resume file: .planning/phases/01-register-inventory/01-02-SUMMARY.md (final phase artefact); Phase 2 begins with context-gathering for `docs/phase-2/coverage-check.md`
