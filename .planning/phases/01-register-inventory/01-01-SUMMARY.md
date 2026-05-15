---
phase: 01-register-inventory
plan: 01
subsystem: documentation
tags: [realism-risks, register, cip, wohlin, inventory]

# Dependency graph
requires: []
provides:
  - "docs/phase-2/realism-risks-register.md skeleton with 24 RSK-NN entries (all required fields populated; judgement fields placeholdered TBD plan 02 for plan 01-02)"
  - "Mandatory four LIVE entries by canonical name: RSK-pool-count, RSK-single-seed-precision, RSK-un-anchored-controller-knobs (umbrella naming four sub-points), RSK-substrate-scope (umbrella naming three sub-points)"
  - "Locked scope-of-resolution text for RSK-pool-count per REG-05"
  - "Index table mapping every RSK-NN to title and initial Wohlin category"
affects:
  - "01-02 (plan 02: judgement-field finalisation): plan 02 greps TBD plan 02 to find every site"
  - "02-coverage-check (Phase 2): coverage-check rows reference RSK-NN identifiers in related-RSK-ids column"
  - "03-targeted-cheap-tests (Phase 3): LIVE entries flag EXP-NN cheap tests for execution"
  - "04-refresh-and-anchor (Phase 4): DOC-01 / DOC-02 / DOC-03 use register entries as the source of truth for refresh"
  - "05-handoff (Phase 5): CIP author summary cites RSK-NN identifiers and disclosure paragraphs"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "RSK-NN identifier scheme (Leios docs/ImpactAnalysis.md precedent; append-only)"
    - "Wohlin four-fold categorisation (construct / internal / external / conclusion) with multi-tagging for borderline cases per D-07"
    - "Umbrella-entry pattern: one RSK-NN with description naming sub-points, used twice (RSK-un-anchored-controller-knobs covers four knobs; RSK-substrate-scope covers three sub-points)"
    - "TBD plan 02 placeholder convention so judgement fields are greppable for plan 02"

key-files:
  created:
    - "docs/phase-2/realism-risks-register.md"
  modified: []

key-decisions:
  - "Final RSK-NN entry count: 24 (within the 20-30 target band per CONTEXT.md Claude's Discretion Entry granularity). Plan 01-02 may not need to re-cluster."
  - "RSK-un-anchored-controller-knobs adopted as ONE umbrella entry naming four sub-points (window-length 32, multiplier-floor 4, multiplier-floor 16, lane-signal-source) rather than four sub-RSKs. Rationale: shares a single resolution path (Phase 4 literature search → anchor or disclose); grouping prevents the looks-anchored-on-3-of-4 partial-resolution failure mode. Both options were explicitly permitted by D-09; the planner's recommendation in the plan text is followed."
  - "RSK-substrate-scope adopted as ONE umbrella entry naming three sub-points (a) upstream f64 in non-pricing hot paths, (b) propagation-model fidelity, (c) utility-maximising actor model. Per CONTEXT.md Claude's Discretion 'Substrate-scope grouping': three sub-points share one mitigation path (none — they're inherited substrate); separate entries would imply separate verdicts could land, misleading."
  - "Provisional EXP-NN slugs use descriptive names (EXP-pool-number, EXP-sign-flip-variance, EXP-canonical-variance, EXP-run-length, EXP-window-length-anchor, EXP-multiplier-floor-4-anchor, EXP-multiplier-floor-16-anchor, EXP-lane-signal-source-anchor, EXP-multiplier-floor-16-companion-run) per CONTEXT.md Claude's Discretion EXP-NN ↔ TEST-NN alignment. Cross-references to TEST-NN are explicit (e.g. → TEST-03)."

patterns-established:
  - "Per-entry schema: id, title, category, description, evidence-for, evidence-against, scope-of-resolution, verdict, EXP/Resolution, disclosure-paragraph (nine fields; verdict + scope-of-resolution + EXP/Resolution + disclosure-paragraph are TBD plan 02 in the skeleton)"
  - "Verdict vocabulary disjoint from spike vocabulary: register uses LIVE / DORMANT / MITIGATED / DISCLOSED (D-06); spikes use VALIDATED / NEEDS-DISCLOSURE / RECOMMENDED / ADOPT (D-04). No verdict leakage between artefacts."
  - "Abbreviations expanded on first use throughout register prose per CLAUDE.md: CIP, IQR, PSE, BCa, SODA, CCS, AFT, MEV, eUTxO, SPO, CLM-NN, etc."

requirements-completed: [REG-01, REG-02, REG-04]

# Metrics
duration: ~30min
completed: 2026-05-15
---

# Phase 01 Plan 01: Inventory pass + de-duplication clustering + register skeleton with all RSK-NN entries

**Realism-risks register skeleton at `docs/phase-2/realism-risks-register.md` with 24 thematic RSK-NN entries clustering every risk-shaped statement from six source documents and seven spike READMEs; all four mandatory LIVE entries present with the canonical identifiers; descriptive fields populated in full and 93 `TBD plan 02` placeholders left for plan 01-02 to finalise.**

## Performance

- **Duration:** ~30 min (single executor pass)
- **Started:** 2026-05-15T12:13Z
- **Completed:** 2026-05-15T12:24Z
- **Tasks:** 2 (Task 1 was a context-only read-and-cluster pass; Task 2 produced the register file)
- **Files modified:** 1 created, 0 modified

## Accomplishments

- Created `docs/phase-2/realism-risks-register.md` — the single source of truth for unresolved realism risks affecting the CIP, with header, reading guide, index table, and 24 per-entry `RSK-NN` sections
- All four mandatory LIVE entries from CONTEXT.md D-09 present by canonical name: `RSK-pool-count`, `RSK-single-seed-precision`, `RSK-un-anchored-controller-knobs`, `RSK-substrate-scope`
- Locked `scope-of-resolution` text for `RSK-pool-count` per REG-05: "Δ% < seed-IQR (Inter-Quartile Range) of same job at 100 pools establishes MITIGATED"
- Substrate-scope adopted as ONE umbrella entry whose description names all three sub-points (`f64` in non-pricing paths, propagation fidelity, utility-maximising actor model) per CONTEXT.md Claude's Discretion
- Un-anchored controller knobs adopted as ONE umbrella entry whose description names all four sub-points (window-length 32, multiplier-floor 4, multiplier-floor 16, lane-signal-source) per the planner's recommendation in the plan text
- All 24 entries populate the nine required fields per D-08; the four judgement fields (`Verdict`, `Scope-of-resolution`, `EXP / Resolution`, `Disclosure-paragraph`) carry `TBD plan 02` placeholders so plan 01-02 can grep for them
- Abbreviations expanded on first use throughout register prose per CLAUDE.md convention: 8 expansions of "Cardano Improvement Proposal (CIP)", 2 of "Inter-Quartile Range (IQR)", 3 of "Paired Seed Evaluation (PSE)", plus BCa, SODA, CCS, AFT, MEV, eUTxO, SPO, IEEE, RTT, EMA, UTC, NFT, EIP-1559, CI

## Task Commits

Each task was committed atomically:

1. **Task 1: Inventory pass** — no commit (deliverable held in context per plan: "do NOT write a transient inventory file"; the register itself is the output)
2. **Task 2: Cluster into thematic RSK-NN entries and draft register file** — `659a042` (docs)

Notes:
- Task 1 produced no file artefact by design (plan explicit: "the deliverable is a working set of risk-shaped statements held in context")
- The register file itself encodes the Task 1 inventory's result (every risk-shaped statement is represented by at least one of the 24 entries)

## Files Created/Modified

- `docs/phase-2/realism-risks-register.md` — created. 24 RSK-NN entries clustering every risk-shaped statement from cardano-realism-audit.md, validity-threats.md, REVIEW.md, CONCERNS.md, mechanism-welfare-impact-2026-05-14.md, and the seven spike READMEs. All nine required fields populated per entry; judgement fields placeholdered TBD plan 02.

## Decisions Made

### Entry granularity: 24 clusters (within the 20-30 band)

The inventory sweep across six source docs and seven spike READMEs surfaced approximately 60 risk-shaped statements. Clustering on shared resolution path (rather than per-source-doc granularity) collapsed these into 24 thematic entries. Examples of how clusters formed:

- "Single-seed claims" appears in PITFALLS CRIT-1, validity-threats §welfare-precision, family-b-results-table seed-diversity column, mechanism-welfare-impact-2026-05-14.md TL;DR — all collapse into one `RSK-single-seed-precision` entry citing the four sign-flip cells (`d4_t50_w32`, `d8_t25_w32`, `x4_rb_quarter` under both `priority-only-rb-reserved` and `partitioned-both-dynamic` arms).
- "Standard-user fee drift exposure" appears in validity-threats §"phase-2-{moderate,realistic,sundaeswap}-both-dynamic" UNRESOLVED verdicts → one `RSK-standard-user-fee-drift-exposure` entry, with the broader UNRESOLVED-pending-output-read framing covered by the separate `RSK-unresolved-suite-claims` entry.

### Umbrella vs sub-RSK choice for un-anchored controller knobs

Adopted umbrella per the plan's planner-recommendation: one `RSK-un-anchored-controller-knobs` entry naming the four sub-points in the description, with `EXP / Resolution` field listing four placeholder EXP slugs (`EXP-window-length-anchor`, `EXP-multiplier-floor-4-anchor`, `EXP-multiplier-floor-16-anchor`, `EXP-lane-signal-source-anchor`). Rationale per CONTEXT.md Claude's Discretion: the four knobs share a single resolution path (Phase 4 two-hour literature search → anchor or disclose); grouping prevents the "looks anchored on 3 of 4" partial-resolution failure mode where three knobs find external anchors and one does not, but the umbrella reads as "anchored".

### Substrate-scope umbrella

Adopted exactly as CONTEXT.md prescribed: ONE entry whose description names three sub-points (upstream `f64` in non-pricing hot paths, propagation-model fidelity, utility-maximising actor model). Sub-points share one mitigation path (none — they are inherited substrate, out of scope per PROJECT.md); the disclosure paragraph in plan 02 will enumerate them under one umbrella. Each sub-point is individually citable from the CIP via the disclosure-paragraph anchor.

### EXP-NN naming alignment

Per CONTEXT.md Claude's Discretion: EXP-NN slugs use descriptive names, with explicit `(→ TEST-NN)` cross-references where the alignment is known. The mappings used:

- `EXP-pool-number` (→ `TEST-05`) for `RSK-pool-count`
- `EXP-sign-flip-variance` (→ `TEST-03`) for `RSK-single-seed-precision`
- `EXP-canonical-variance` (→ `TEST-04`) for `RSK-single-seed-precision` and `RSK-three-seed-statistical-power`
- `EXP-run-length` (→ `TEST-06`) for `RSK-steady-state-run-length`
- Four `EXP-*-anchor` slugs (TBD plan 02) for `RSK-un-anchored-controller-knobs`, with cross-reference to `DOC-03` (the Phase 4 anchoring/disclosure requirement)
- `EXP-multiplier-floor-16-companion-run` for `RSK-multiplier-floor-4-suite-coverage` (likely under `TEST-07`)

### Verdict-vocabulary disjointness from spikes

Per D-04 and D-06: spike verdicts (VALIDATED / NEEDS-DISCLOSURE / RECOMMENDED / ADOPT) are evidence-for citations only; the register's verdict vocabulary is the disjoint set LIVE / DORMANT / MITIGATED / DISCLOSED. The acceptance check `grep -E "^\*\*Verdict:\*\* (VALIDATED|RECOMMENDED|ADOPT)$"` returns 0 hits, confirming no leakage. (Plan 01-02 will replace `TBD plan 02` placeholders with the LIVE / DORMANT / MITIGATED / DISCLOSED values.)

## Thematic risks surfaced beyond the planner's projection

The Task-2 action listed a planner's projection of thematic clusters; the inventory pass surfaced a few additional thematic risks worth recording for plan 02:

- **`RSK-three-seed-statistical-power`** — distinct from `RSK-single-seed-precision`. The single-seed risk is about the 33-job sundaeswap smoke at seed=1 only; the three-seed-power risk is about the broader 19-suite × 3-seed default (also insufficient for tight 95% confidence intervals). Both surface in PITFALLS CRIT-1 and validity-threats §"Cross-cutting threats" but represent two different scales of the same underlying statistical concern.
- **`RSK-unresolved-suite-claims`** — surfaces from validity-threats §"Aggregate trust summary" UNRESOLVED row. Lowest-cost trust-upgrade in the matrix (one output-read pass flips each from UNRESOLVED to MEDIUM/LOW). Not in the planner's projection but matches the "register entries that have downstream-handoff resolution paths" theme.
- **`RSK-leios-spec-pre-deployment`** — distinct from `RSK-substrate-scope` and `RSK-calibration-stale-stake-snapshot`. The Leios-spec knobs (CIP-0164 Table 7) are pre-deployment, not stale — a structurally-different un-anchoredness from "we have an anchor but it's epoch-582". Surfaced from the audit's §"What lines up with mainnet" final bullet.

## Source-document risk-shaped statements considered and explicitly excluded

For plan 02 to honour the exclusion rationale, the following items were considered and excluded:

- **Internal-process items from REVIEW.md / CONCERNS.md** (per D-03): tech-debt items that are not CIP-facing — mixed serde casing across persisted artefacts, RB-reduced overlays as full replacements (not stacked), `Eip1559Pricing::step` saturating u128 ops (WR-4, applied), `MetricsCollector::is_representative` lazy fallback (WR-6, applied), legacy protocols in `sim-core/src/sim/` (informational). These are internal-process concerns; the register is CIP-facing per CONTEXT.md `<specifics>`. They remain in their original docs and can be referenced via `evidence-for` if needed.
- **Spike verdicts as register verdicts** (per D-04): the seven spike READMEs' verdicts (VALIDATED / NEEDS-DISCLOSURE / RECOMMENDED / ADOPT) are evidence-source citations only. They are cited in `evidence-for` fields but never lifted into register `Verdict` fields. (Acceptance check `grep -E "^\*\*Verdict:\*\* (VALIDATED|RECOMMENDED|ADOPT)$"` returns 0 hits.)
- **Per-claim trust ratings from validity-threats.md** (per D-01): per-suite HIGH/MEDIUM/LOW/UNRESOLVED ratings stay in validity-threats for Phase 2's coverage check (per-claim trust matrix) and Phase 4's pointer-doc refresh. Only per-threat *risk descriptions* are pulled into the register. The exception: `RSK-unresolved-suite-claims` references the UNRESOLVED-row meta-pattern (4 suites pending output read) — this is the *risk shape*, not the per-claim trust rating itself.
- **CIP-process items framed as Phase 2 concerns** (per the plan's `<action>`): MOD-7 (evidence-package menu-alignment) is a Phase 2 coverage-check design concern, not a register entry. Mentioned in `RSK-menu-collapse-to-advocacy`'s scope as cross-reference to REQ-COV-03, but the register entry itself is about the *risk* (welfare-only collapse) rather than the *mitigation infrastructure*.
- **Out-of-scope items from PROJECT.md** (per CONTEXT.md `<deferred>`): adversarial / strategic-bidder modelling, cross-architecture CI verification, upstream Leios simulator re-audit are folded into `RSK-substrate-scope` as inherited-substrate sub-points, not given separate entries. This preserves the umbrella's intent and avoids implying separate verdicts could land.
- **`RSK-cite-by-source-and-date`** (initially considered as a process-discipline entry under PITFALLS' cross-cutting principle 2): excluded as redundant — the same concern is captured concretely by `RSK-calibration-stale-stake-snapshot`, which carries the actionable freshness-policy resolution path.

## Deviations from Plan

None — plan executed exactly as written. The plan's `<action>` for Task 2 named a planner's projection of thematic clusters; the executor surfaced three additional thematic risks beyond that projection (per "Thematic risks surfaced beyond the planner's projection" above), which is explicitly permitted by the plan text ("the planner's list was a projection, not a contract").

## Issues Encountered

None.

The worktree shipped without `.planning/` or several `docs/phase-2/` files because the worktree branch was forked from an older base commit. The source documents were read directly from the main repository working tree via absolute paths (read-only); the register file was written into the worktree's `docs/phase-2/` directory (which existed and was committable). No worktree-branch divergence introduced or modifications to source documents made. SUMMARY.md is written into `.planning/phases/01-register-inventory/` (creating that path inside the worktree).

## Verification

All acceptance checks from the plan pass:

```
test -f docs/phase-2/realism-risks-register.md  → OK
grep -c "^## RSK-" → 24 (within 20-32 band)
grep "^## RSK-pool-count" → OK
grep "^## RSK-single-seed-precision" → OK
grep "^## RSK-substrate-scope" → OK
grep "^## RSK-un-anchored-controller-knobs" → OK
substrate-scope sub-points (f64|propagat|actor) → 10 matches (all 3 terms covered)
TBD plan 02 placeholders → 93 (≥ 3 × 24 = 72)
"Cardano Improvement Proposal (CIP)" expansions → 8
"Inter-Quartile Range (IQR)" expansions → 2
"Paired Seed Evaluation (PSE)" expansions → 3
spike verdict vocabulary (VALIDATED|RECOMMENDED|ADOPT) as Verdict → 0 hits
```

## Known Stubs

None. The register file contains `TBD plan 02` placeholders by design (per the plan), to be finalised by plan 01-02. These are not stubs in the "data not wired up" sense — they are explicit hand-off markers between the inventory pass (plan 01) and the judgement pass (plan 02).

## Next Phase Readiness

- Plan 01-02 (judgement-field finalisation) has the inventory it needs: grep `TBD plan 02` produces 93 sites across 24 entries × 4 judgement fields.
- Phase 2 (coverage check) can begin in parallel with plan 01-02 since the index table and `related-RSK-ids` referencing pattern is stable.
- Phase 3 cheap tests (TEST-01 through TEST-07) have provisional EXP-NN slugs and TEST-NN cross-references inside the register; the EXP definitions become test-design inputs once plan 01-02 finalises the `Verdict` and `Scope-of-resolution` fields per LIVE entry.

## Self-Check: PASSED

Created file exists:
- `docs/phase-2/realism-risks-register.md` — FOUND

Commit exists:
- `659a042` (docs(01-01): inventory realism-risks register skeleton with all RSK-NN entries) — FOUND in `git log`

All plan acceptance criteria pass (see Verification section).

---

*Phase: 01-register-inventory*
*Plan: 01*
*Completed: 2026-05-15*
