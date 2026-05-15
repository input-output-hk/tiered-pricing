---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: executing
stopped_at: Phase 1 context gathered
last_updated: "2026-05-15T12:13:38.287Z"
last_activity: 2026-05-15 -- Phase 1 execution started
progress:
  total_phases: 5
  completed_phases: 0
  total_plans: 2
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-05-15)

**Core value:** A reader of the Cardano Improvement Proposal (CIP) can verify each menu-option claim against a specific simulator job and can inspect the realism-risks register to see what the simulator does and does not faithfully model, so the CIP stands on documented evidence rather than asserted authority.
**Current focus:** Phase 1 — Register Inventory

## Current Position

Phase: 1 (Register Inventory) — EXECUTING
Plan: 1 of 2
Status: Executing Phase 1
Last activity: 2026-05-15 -- Phase 1 execution started

Progress: [░░░░░░░░░░] 0%

## Performance Metrics

**Velocity:**

- Total plans completed: 0
- Average duration: —
- Total execution time: —

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| — | — | — | — |

**Recent Trend:**

- Last 5 plans: —
- Trend: —

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

Last session: 2026-05-15T11:24:35.407Z
Stopped at: Phase 1 context gathered
Resume file: .planning/phases/01-register-inventory/01-CONTEXT.md
