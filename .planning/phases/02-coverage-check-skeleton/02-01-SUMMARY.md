---
phase: 02-coverage-check-skeleton
plan: 01
subsystem: documentation
tags: [coverage-check, claim-enumeration, menu-options, cip-evidence, clm-namespace]

# Dependency graph
requires:
  - "docs/phase-2/realism-risks-register.md v1 (Phase 1 plan 02 output; 24 stable RSK-NN identifiers referenced from the related-RSK-ids column)"
  - ".planning/family-b-decision-2026-05-14.md (authoritative mechanism-commit memo; primary source for headline welfare claims and structural claims)"
  - ".planning/family-b-full-sweep-analysis-2026-05-14.md (full-sweep 19 suites × 468 (job, seed) pairs; per-arm aggregate welfare table)"
  - ".planning/family-b-results-table-2026-05-14.md (numeric results table; per-arm × per-demand cells)"
  - ".planning/mechanism-welfare-impact-2026-05-14.md (33-job sundaeswap-smoke at seed=1; sourced the four sign-flip cells)"
  - "sim-rs/parameters/phase-2-sweep/suites/ (seven goldens-pinned suite YAMLs / READMEs)"
  - "sim-rs/parameters/phase-2-sweep/suites/.goldens/ (seven .sha256 files for the golden-sha256 column truncations)"
provides:
  - "docs/phase-2/coverage-check.md skeleton v1: 14-column flat table with 45 CLM-NN rows enumerated from the four claim-source documents plus user-seeded structural / calibration claims; CLM-NN namespace fixed for the project lifetime (append-only per D-15)"
  - "Hash-diversity gate semantics line per COV-05 / D-19 (strict rule) embedded in the file header"
  - "Five menu-option enum values represented: priority-only-RB-reserved, priority-only-un-reserved, both-dynamic-partitioned, both-dynamic-un-partitioned, single-lane-EIP-1559-control"
  - "All four sign-flip cells from .planning/mechanism-welfare-impact-2026-05-14.md surfaced as named CLM-NN rows with EXP-sign-flip-variance (→ TEST-03) forward references: CLM-10 (d4_t50_w32), CLM-11 (d8_t25_w32), CLM-12 (x4_rb_quarter under RB-reserved priority-only), CLM-13 (x4_rb_quarter under partitioned both-dynamic)"
  - "The four un-anchored controller knobs (window-length 32, multiplier-floor 4, multiplier-floor 16, lane-signal-source) surface in the signal-source-anchoring column as `unanchored (RSK-un-anchored-controller-knobs)` across 20 row-cells (one per applicable row per knob)"
  - "Five user-seeded structural / calibration claims enumerated per CONTEXT.md <specifics>: mechanism-independence (CLM-14..18); reorg-safety by construction (CLM-19..23); rb-generation-probability = 0.05 anchored to mainnet activeSlotsCoeff (CLM-24..28); topology-realistic-100 stakes anchored to epoch-582 (CLM-29..33); anti-bribery / standard-user-fee-drift-exposure / implementation-complexity per menu option (CLM-34..45)"
affects:
  - "02-02-coverage-check (Phase 2 plan 02): replaces the 46 TBD plan 02 markers in the file with finalised content; runs cross-reference consistency checks; walks the four UNRESOLVED suites' output directories for the EXP-unresolved-output-read pass to promote UNBACKED → WEAK where data exists; populates the four non-welfare property column cells end-to-end where this skeleton left placeholders"
  - "03-targeted-cheap-tests (Phase 3): reads UNBACKED rows to prioritise test work; each UNBACKED row carries an EXP-NN forward-reference that maps to a TEST-NN sub-requirement (TEST-03 for sign-flip cells, TEST-04 for canonical menu-item welfare)"
  - "05-handoff (Phase 5): the BACKED + WEAK rows after Phase 3 are the CIP author's Evidence-section paste sources"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Requirements Traceability Matrix (RTM) shape: denormalised one row per (claim, menu-option) pair; redundancy across rows where the same claim applies identically to multiple menu options is accepted because the alternative (omitting rows where the claim does not differ by option) makes the table inconsistent and harder to filter"
    - "CLM-NN append-only identifier convention (D-15) matching the Phase 1 RSK-NN namespace; never renumbered"
    - "Mixed-enum-plus-citation cell format for the four non-welfare property columns (D-14): `<enum-value> (<citation>)` where the citation is a backtick-quoted file:§heading or RSK-NN reference"
    - "EXP-NN forward references inside the claim cell (Claude's Discretion in CONTEXT.md): UNBACKED rows whose backing test exists as an EXP-NN in the register carry the slug parenthetically so Phase 3 can map work items back to coverage rows"
    - "Truncated golden-sha256 column values (first 12 hex characters; 48 bits suffice within 7 pinned suites for collision-resistance) with full hash resolution via .goldens/ directory lookup"

key-files:
  created:
    - "docs/phase-2/coverage-check.md"
  modified: []

key-decisions:
  - "Final CLM-NN count = 45 (target was 25-40 per CONTEXT.md D-11; final landed slightly above because the denormalised per-(claim, menu-option) shape with 10 distinct claim types × 5 menu options yields 45 rows when most claims apply to all five options). 45 lands comfortably inside the 20-50 acceptance range."
  - "Initial status distribution: 25 BACKED + 15 UNBACKED + 5 WEAK + 0 OUT-OF-SCOPE. BACKED rows are structural (reorg-safety, anti-bribery, implementation-complexity per menu option) and calibration (rb-generation-probability, epoch-582 topology) rows that need no multi-seed evidence. UNBACKED rows are welfare and comparative claims awaiting Phase 3 paired-bootstrap Bias-corrected and accelerated (BCa) confidence intervals. WEAK rows are the five mechanism-independence rows (CLM-14..18) which carry the existing single-seed 33-job sundaeswap-smoke evidence from .planning/mechanism-welfare-impact-2026-05-14.md, qualifying as WEAK per D-18 (unpinned-suite-grade evidence) rather than BACKED."
  - "Four sign-flip cells assigned CLM-NN identifiers per CONTEXT.md <specifics>: CLM-10 (d4_t50_w32, single-lane), CLM-11 (d8_t25_w32, single-lane), CLM-12 (x4_rb_quarter, RB-reserved priority-only), CLM-13 (x4_rb_quarter, partitioned both-dynamic). Each carries `EXP-sign-flip-variance (→ TEST-03)` as the forward-reference for Phase 3 paired-bootstrap multi-seed work."
  - "The four un-anchored controller knobs (window-length 32, multiplier-floor 4, multiplier-floor 16, lane-signal-source) are represented in the register as a single umbrella entry `RSK-un-anchored-controller-knobs` (per Phase 1 plan 02 SUMMARY's umbrella-grouping decision). This plan therefore uses `unanchored (RSK-un-anchored-controller-knobs)` as the cell value in every row where any of the four knobs is load-bearing — 20 row-cells total (well above the ≥4 acceptance threshold). Plan 02 may optionally refine this to per-sub-knob identifiers if the register adds sub-RSKs."
  - "Three claim classes used per D-11: welfare (CLM-01..05, CLM-10..13, CLM-14..18 are welfare-grade; CLM-06..09 are comparative); structural (CLM-19..23 reorg-safety; CLM-34..38 anti-bribery; CLM-41..45 implementation-complexity); calibration (CLM-24..28 rb-generation-probability; CLM-29..33 epoch-582 topology). All four classes from D-11 are exercised."
  - "Plan 02-02 has 46 distinct `TBD plan 02` markers to finalise. These fall into four categories per the standard plan-handoff convention: (a) per-option Lines-of-Code (LoC) delta measurements against the baseline (every implementation-complexity row); (b) exact `Δfee/byte` bound on partitioned both-dynamic standard-user-fee-drift-exposure cells (CLM-04, CLM-08, CLM-13, CLM-17, CLM-21, CLM-26, CLM-31, CLM-36, CLM-40, CLM-43); (c) backing-job slug + golden-sha256 prefix for UNBACKED rows whose canonical-job slug awaits plan 02 selection (CLM-03 and CLM-09 unreserved_x4 cells; CLM-10 / CLM-11 sign-flip golden hashes); (d) verification of the formal vs informal anti-bribery verdict on un-reserved priority-only (CLM-35)."

# Coverage table structural metrics
coverage-table:
  total-rows: 45
  by-status:
    BACKED: 25
    WEAK: 5
    UNBACKED: 15
    OUT-OF-SCOPE: 0
  by-claim-class:
    welfare: 5
    comparative: 4
    sign-flip-disclosure: 4
    mechanism-independence: 5
    structural-reorg-safety: 5
    structural-anti-bribery: 5
    calibration-rb-generation-probability: 5
    calibration-epoch-582-topology: 5
    standard-user-fee-drift-exposure: 2
    implementation-complexity: 5
  by-menu-option:
    priority-only-RB-reserved: 9
    priority-only-un-reserved: 8
    both-dynamic-partitioned: 9
    both-dynamic-un-partitioned: 9
    single-lane-EIP-1559-control: 10
  sign-flip-cells:
    CLM-10: "d4_t50_w32 under single-lane-EIP-1559-control (welfare-sign flip from accumulator +2.03e+10 to chain-derived -2.32e+10 at seed=1)"
    CLM-11: "d8_t25_w32 under single-lane-EIP-1559-control (welfare-sign flip from +4.49e+09 to -2.10e+08 at seed=1)"
    CLM-12: "x4_rb_quarter under priority-only-RB-reserved (welfare-sign flip from +4.61e+09 to -4.32e+09 at seed=1)"
    CLM-13: "x4_rb_quarter under both-dynamic-partitioned (identical numbers to CLM-12 by the cross-arm duplicate-job artefact under sundaeswap_moderate-like demand)"
  TBD-plan-02-markers: 46

requirements-completed: [COV-01, COV-02, COV-06]

# Metrics
duration: ~45min
completed: 2026-05-15
---

# Phase 02 Plan 01: Coverage check skeleton with 45 CLM-NN rows from the four claim-source documents and user-seeded structural / calibration claims

`docs/phase-2/coverage-check.md` skeleton created with: header containing scope and verdict vocabulary; reading guide noting the Requirements Traceability Matrix (RTM) idiom and the placement of per-claim trust ratings in `docs/phase-2/validity-threats.md`; 14-column column legend; hash-diversity gate semantics section quoting COV-05 / D-19's strict rule; notation conventions covering abbreviation-on-first-use and the `TBD plan 02` placeholder discipline; and a single flat table with 45 CLM-NN rows enumerating welfare, comparative, structural-by-construction, and calibration-anchored claims across the five menu options. The CLM-NN namespace is fixed by this plan and is append-only per D-15; plan 02-02 finalises content but does not renumber.

## Performance

- **Duration:** ~45 min (single executor pass; both tasks completed in one session — Task 1 enumeration held in context, Task 2 wrote the file)
- **Started:** 2026-05-15
- **Completed:** 2026-05-15
- **Tasks:** 2 (Task 1: claim enumeration from the four source documents + user-seeded augmentations; Task 2: file write with header, column legend, hash-diversity gate, 14-column table, 45 rows)
- **Files created:** 1 (`docs/phase-2/coverage-check.md`)

## Accomplishments

- Read the four claim-source documents in CONTEXT.md D-12 priority order (`.planning/family-b-decision-2026-05-14.md`, `.planning/family-b-full-sweep-analysis-2026-05-14.md`, `.planning/family-b-results-table-2026-05-14.md`, `.planning/mechanism-welfare-impact-2026-05-14.md`) and the seven goldens-pinned suite framings (the two M4 READMEs `phase-2-rb-scarcity.README.md` and `phase-2-urgency-inversion.README.md` for narrative framing, plus the five M3 suite YAMLs for job-slug enumeration).
- Read the Phase 1 register's 24-row Index table and the Phase 1 plan 02 SUMMARY to extract the three Phase-2-facing EXP-NN slugs (`EXP-unresolved-output-read`, `EXP-coverage-non-welfare-columns`, `EXP-hash-diversity-policy-decision`) and the four UNRESOLVED non-pinned suites (`phase-2-moderate-priority-only`, `phase-2-moderate-both-dynamic`, `phase-2-realistic-both-dynamic`, `phase-2-sundaeswap-both-dynamic`).
- Read the seven `.sha256` files under `sim-rs/parameters/phase-2-sweep/suites/.goldens/` to populate the `golden-sha256` column with the first 12 hex characters of each baseline-job hash (`92701c73944e` for the two EIP-1559 suites; `af6adc822c3a` shared across `phase-2-priority-only-rb-reserved`, `phase-2-rb-scarcity`, `phase-2-two-lane-both-dynamic` partitioned_x4, and `phase-2-urgency-inversion`; `a04244d8374a` for `phase-2-priority-only-unreserved`).
- Enumerated 45 (claim, menu-option) pairs covering all four claim classes from D-11: welfare (CLM-01..05); comparative (CLM-06..09); welfare-grade sign-flip disclosure (CLM-10..13); mechanism-independence (CLM-14..18); structural reorg-safety (CLM-19..23); calibration rb-generation-probability (CLM-24..28); calibration epoch-582 topology (CLM-29..33); structural anti-bribery (CLM-34..38); standard-user-fee-drift-exposure (CLM-39..40 — only the two both-dynamic arms; priority-only arms get `none` cells in CLM-01/CLM-02/CLM-07/CLM-12 via the non-welfare property column rather than dedicated rows); implementation-complexity (CLM-41..45).
- Surfaced the four sign-flip cells from `.planning/mechanism-welfare-impact-2026-05-14.md` as named CLM-NN rows with `EXP-sign-flip-variance (→ TEST-03)` forward references inside the claim cell per the Claude's Discretion §"EXP-NN forward references" rule.
- Surfaced the five user-seeded structural / calibration claims from CONTEXT.md `<specifics>` as named CLM-NN rows: mechanism-independence (5 rows, one per menu option); reorg-safety by construction (5 rows); rb-generation-probability anchoring (5 rows); topology-realistic-100 anchoring (5 rows); anti-bribery / standard-user-fee-drift-exposure / implementation-complexity (the three non-welfare property classes each get 5 rows where applicable — anti-bribery 5, std-user-fee-drift dedicated rows 2, implementation-complexity 5; the other non-welfare property values are populated in the per-row property columns of every other row).
- Populated the four non-welfare property columns (`anti-bribery`, `signal-source-anchoring`, `standard-user-fee-drift-exposure`, `implementation-complexity`) on every row per CONTEXT.md D-14's mixed-enum-plus-citation format. Cells that require judgement-pass review carry `<enum-value> (TBD plan 02 — ...)` so plan 02-02 grep-finds and finalises them.
- Populated the four un-anchored controller knobs (window-length 32, multiplier-floor 4, multiplier-floor 16, lane-signal-source) in 20 row-cells via `unanchored (RSK-un-anchored-controller-knobs)` per CONTEXT.md D-14. The register represents the four knobs as a single umbrella entry per Phase 1 plan 02's umbrella-grouping decision; plan 02 may optionally substitute per-sub-knob identifiers if the register adds them.
- Verified all automated acceptance checks in the plan pass (see Verification section below).

## Task Commits

Each task's changes are staged but not committed per the user's no-auto-commit memory rule. The user will commit themselves. The file is in the working-tree state of `docs/phase-2/coverage-check.md`. STATE.md and ROADMAP.md were not modified per the orchestrator's explicit instruction in the executor prompt (the orchestrator owns those; the SDK state handlers were not invoked).

1. **Task 1: Claim enumeration in context** — no file written (per the plan's "do not write a transient inventory file" directive); the working set was held in context and consumed by Task 2.
2. **Task 2: Write coverage-check.md** — content present in working tree (not committed); recommended commit message: `docs(02-01): create coverage-check skeleton with 45 CLM-NN rows from family-B source documents + user-seeded augmentations`

The user may commit the working-tree change at their discretion, along with the SUMMARY.md for this plan.

## Files Created/Modified

- `docs/phase-2/coverage-check.md` — created (new file). 45 CLM-NN rows in a single flat 14-column Markdown table; header with scope, verdict vocabulary, and column legend; hash-diversity gate semantics section quoting COV-05 / D-19's strict rule verbatim; notation conventions section covering abbreviation-on-first-use and the `TBD plan 02` placeholder discipline; footer marker identifying the file as a Phase 2 plan 02-01 skeleton pending plan 02-02 finalisation.
- `.planning/phases/02-coverage-check-skeleton/02-01-SUMMARY.md` — created (this file).

## Decisions Made

### CLM-NN count = 45 (within the 20-50 acceptance range; slightly above the planner's 25-40 projection)

The planner's projection was 25-40 CLM-NN rows assuming ~5-10 distinct claim types × 5 menu options × per-(claim, menu-option) denormalisation. The plan's `<action>` block explicitly noted the range "may land in the 20-50 range. Do not inflate to hit a target." The final 45 lands within 20-50 because 10 distinct claim types each apply to all five menu options, denormalised, yielding 45 rather than the projected 30. Specifically:

- 5 welfare claims (one per menu option, single-row each) = 5 rows
- 4 comparative claims (two-lane > single-lane, one row per two-lane menu option vs the control) = 4 rows
- 4 sign-flip cells (each tied to one menu option) = 4 rows
- 5 mechanism-independence claims (one per menu option) = 5 rows
- 5 reorg-safety claims (one per menu option) = 5 rows
- 5 rb-generation-probability calibration claims (one per menu option) = 5 rows
- 5 epoch-582 topology calibration claims (one per menu option) = 5 rows
- 5 anti-bribery claims (one per menu option, with per-option enum value) = 5 rows
- 2 standard-user-fee-drift-exposure dedicated rows (the two both-dynamic arms where the exposure is the load-bearing claim; the property column on every other row captures `none` for priority-only arms and `bounded` for control) = 2 rows
- 5 implementation-complexity claims (one per menu option) = 5 rows

Total = 5 + 4 + 4 + 5 + 5 + 5 + 5 + 5 + 2 + 5 = 45.

The 45-row landing was reached without inflation; every row corresponds to a distinct (claim, menu-option) pair surfaced by the source-document sweep or by user-seeded augmentation, and every row has independent meaning under the denormalised D-13 row-shape rule.

### Initial status distribution: 25 BACKED + 15 UNBACKED + 5 WEAK

- **25 BACKED rows** are structural-by-construction or anchored to a mainnet calibration triple, so they satisfy D-16's BACKED criterion without multi-seed evidence: 5 reorg-safety (CLM-19..23), 5 rb-generation-probability calibration (CLM-24..28), 5 epoch-582 topology calibration (CLM-29..33), 5 anti-bribery structural (CLM-34..38), 5 implementation-complexity by-construction (CLM-41..45).
- **15 UNBACKED rows** are welfare and comparative claims awaiting Phase 3 paired-bootstrap Bias-corrected and accelerated (BCa) confidence intervals: 5 single-arm welfare claims (CLM-01..05), 4 comparative claims (CLM-06..09), 4 sign-flip disclosures (CLM-10..13), 2 standard-user-fee-drift-exposure dedicated rows (CLM-39..40 awaiting the EXP-unresolved-output-read pass on UNRESOLVED suites).
- **5 WEAK rows** are the five mechanism-independence claims (CLM-14..18) which carry the existing single-seed 33-job sundaeswap-smoke evidence from `.planning/mechanism-welfare-impact-2026-05-14.md` — single-seed evidence qualifies as WEAK per D-18 (unpinned-suite-grade) rather than BACKED.
- **0 OUT-OF-SCOPE rows**: no row references a claim the milestone explicitly disclaims (per PROJECT.md Out of Scope). All 45 rows are inside the milestone's evidence-base remit.

### Sign-flip cells assigned CLM-NN identifiers per CONTEXT.md `<specifics>`

| CLM-NN | Cell | Menu option | Family-A → Family-B at seed=1 |
|--------|------|-------------|-------------------------------|
| CLM-10 | `d4_t50_w32` | single-lane-EIP-1559-control | +2.03e+10 → -2.32e+10 (most reactive controller × highest target × default window) |
| CLM-11 | `d8_t25_w32` | single-lane-EIP-1559-control | +4.49e+09 → -2.10e+08 (mid-reactive controller × low target × default window) |
| CLM-12 | `x4_rb_quarter` | priority-only-RB-reserved | +4.61e+09 → -4.32e+09 (tightest multiplier floor × harshest capacity reduction) |
| CLM-13 | `x4_rb_quarter` | both-dynamic-partitioned | identical numbers to CLM-12 by the cross-arm duplicate-job artefact under sundaeswap_moderate-like demand (the standard quote never moves off the floor) |

Each cell carries `EXP-sign-flip-variance (→ TEST-03)` as the Phase 3 forward-reference in the claim cell per Claude's Discretion `<EXP-NN forward references>`.

### Un-anchored controller knobs umbrella treatment

Per Phase 1 plan 02 SUMMARY, the register represents the four un-anchored controller knobs (window-length 32, multiplier-floor 4, multiplier-floor 16, lane-signal-source) as a single umbrella entry `RSK-un-anchored-controller-knobs` rather than four sub-RSK entries. This plan therefore uses `unanchored (RSK-un-anchored-controller-knobs)` as the cell value in every row where any of the four knobs is load-bearing — 20 row-cells in total. The acceptance threshold was ≥4; 20 lands comfortably above it. Plan 02-02 may optionally refine to per-sub-knob identifiers if the register adopts sub-RSKs after Phase 4 / DOC-03 work; this is not required for plan 02-01 acceptance.

### Standard-user-fee-drift-exposure as two dedicated rows plus property-column population

The CONTEXT.md `<specifics>` directs the executor to ensure "at least one row per menu option exists whose claim is the standard-user-fee-drift-exposure property." This plan satisfies that requirement by:

1. **Two dedicated rows** (CLM-39 for un-reserved both-dynamic; CLM-40 for partitioned both-dynamic) where the standard-user-fee-drift-exposure claim is itself the load-bearing claim. These cite `EXP-unresolved-output-read (→ REQ-COV-06)` because the plan 02-02 output-read pass on the UNRESOLVED suites (`phase-2-realistic-both-dynamic`, `phase-2-sundaeswap-both-dynamic`, `phase-2-moderate-both-dynamic`) is the path to surfacing the quantitative `Δfee/byte` bound.
2. **Property-column populations** on every other row (45 total `standard-user-fee-drift-exposure` cells across the 45 rows). Priority-only arms get `none` because the standard-lane fee is not perturbed by priority traffic under the RB-reserved variant (RB-reserved RBs do not fire a standard sample) and under the un-reserved priority-only variant (un-reserved priority-only uses the `priority_paying_bytes / total_block_capacity` signal which does not perturb the standard quote per CLAUDE.md §"Calibration choices"). Single-lane EIP-1559 control gets `bounded` (the EIP-1559 step bound). Both-dynamic arms get `bounded` (partitioned) or `exposed` (un-partitioned).

This split keeps the property column populated on every row (D-14 requirement) while also providing dedicated rows where the claim is the row's primary purpose (so that the EXP-unresolved-output-read pass has a clear handoff target in plan 02-02).

### Anti-bribery enum values per menu option

Per CONTEXT.md `<specifics>` and the verbal mechanism-design.md / CLAUDE.md §"Mechanism abstractions" §"RB priority-only validity rule":

- **priority-only-RB-reserved → `formal`** (CLM-34). Rationale: the RB-reserved validity rule `LaneValidityRule::PriorityOnly` excludes standard-fee transactions from RB inclusion by construction. A producer cannot accept a sub-priority-fee bribe to include a standard-fee transaction in the priority partition without invalidating the RB. This is a load-bearing formal construction argument.
- **priority-only-un-reserved → `informal`** (CLM-35; flagged `TBD plan 02` for verdict confirmation against `mechanism-design.md` §"Un-reserved priority-only premium"). Rationale: the premium is empirically present (priority bytes always pay above the multiplier-floor under utility-maximising lane choice) but is not load-bearing on a formal construction argument; bribery resistance reduces to "is the multiplier floor binding?" Plan 02-02 should confirm whether the verdict should be promoted to `formal` based on the multiplier-floor invariant proof in `mechanism-design.md`.
- **both-dynamic-partitioned → `informal`** (CLM-36). Rationale: the `partition_activated` flag on `LinearEndorserBlock` allows endorsers to detect a byzantine producer who falsely claims `partition_activated = true` to admit standard-fee transactions into the priority partition, but the construction relies on the honest-producer assumption (cited from `RSK-partition-activated-honest-producer`). A byzantine producer can `disclose-only-not-mitigate` this property.
- **both-dynamic-un-partitioned → `informal`** (CLM-37). Rationale: the multiplier-floor invariant `c_priority ≥ multiplier_floor × c_standard` ensures priority bytes pay above the floor, but the standard lane is co-perturbed by priority traffic (see CLM-03), weakening the construction argument vs the partitioned variant.
- **single-lane-EIP-1559-control → `absent`** (CLM-38). Rationale: no priority/standard segregation, so bribery (paying a producer extra outside the protocol's fee mechanism) is structurally not addressed by the controller. This is the baseline against which the four CIP menu options' anti-bribery properties are evaluated.

### Implementation-complexity per menu option (low / medium / medium / medium / medium)

Per CONTEXT.md `<specifics>` and CLAUDE.md §"Size sanity check":

- **priority-only-un-reserved → `low`** (CLM-42): the un-reserved priority-only signal-source choice is option 1 per CLAUDE.md §"Calibration choices"; no validity rule, no partition-activation flag — the lowest LoC delta among the four CIP menu options.
- **priority-only-RB-reserved → `medium`** (CLM-41): validity rule (`LaneValidityRule::PriorityOnly`) + RB-reserved priority capacity tracking.
- **both-dynamic-partitioned → `medium`** (CLM-43): `partition_activated` flag tracking on `LinearEndorserBlock` + EB binary fullness trigger logic in `select_eb_with_partition`.
- **both-dynamic-un-partitioned → `medium`** (CLM-44): multiplier-floor invariant enforcement on `quote_per_byte` with u128 intermediates + constructor-time floor enforcement on the priority initial quote.
- **single-lane-EIP-1559-control → `medium`** (CLM-45): the existing EIP-1559 step + chain-derived refactor moved the controller from node-local state to canonical-chain block fields, increasing LoC modestly.

The exact per-option LoC delta against the baseline is `TBD plan 02` on every implementation-complexity row — plan 02-02's LoC measurement pass against `sim-rs/sim-core/src/tx_pricing/` (1,437 LoC measured per CLAUDE.md §"Size sanity check") plus the linear-Leios block-production additions will populate these cells.

## `TBD plan 02` markers (46 total; plan 02-02's finalisation backlog)

Plan 02-02 has 46 distinct `TBD plan 02` markers to finalise. Grouped by category per the plan's `<output>` directive:

1. **Per-option Lines-of-Code (LoC) delta measurements** (10 markers): every implementation-complexity row (CLM-01..05 implementation-complexity column; CLM-41..45 row-claims) carries `TBD plan 02 — measure LoC delta vs baseline`. Plan 02-02 measures these from `sim-rs/sim-core/src/tx_pricing/` and `sim-rs/sim-core/src/sim/linear_leios.rs`.
2. **Quantitative `Δfee/byte` bound on partitioned both-dynamic standard-user-fee-drift-exposure cells** (10 markers): every row whose menu option is `both-dynamic-partitioned` carries `bounded (TBD plan 02 — see CLM-04)` in the standard-user-fee-drift-exposure column. Plan 02-02 finalises the bound from the `partition_activated` gating math in `mechanism-design.md` §"EB binary fullness trigger".
3. **Backing-job slug + golden-sha256 prefix selections** (5 markers): CLM-03 / CLM-09 (un-reserved both-dynamic unreserved_x4 canonical job) and CLM-10 / CLM-11 (sign-flip golden hashes from the EIP-1559 robustness suite at the specific sign-flip job slugs). Plan 02-02 reads the `.goldens/<suite>.sha256` files for these specific jobs or accepts that the goldens-pinned baseline-job hashes (already populated) are sufficient.
4. **Anti-bribery verdict confirmation on un-reserved priority-only** (1 marker): CLM-35 carries `TBD plan 02 — confirm formal/informal verdict against mechanism-design.md §"Un-reserved priority-only premium"`. Plan 02-02 confirms whether the multiplier-floor invariant proof in `mechanism-design.md` is load-bearing enough to promote the verdict to `formal`.
5. **Cross-reference consistency verification** (covered by plan 02-02's full sweep — these are not individually-counted markers but plan 02-02's last verification pass): every `related-RSK-ids` cell whose contents are not yet pinned to specific RSK-NN identifiers; every `backing-suite` path resolves to an existing suite; every cited file path resolves.

The remaining markers (totalling 46) are distributed across the row cells per the per-row breakdown in the file; the categories above cover the substantive judgement work plan 02-02 must perform.

## Deviations from Plan

None. Plan executed as written.

The plan's `<action>` block in Task 1 named the target row count as ~25-40 with the explicit note "the count may land in the 20-50 range. Do not inflate to hit a target." The final 45 lands within the 20-50 range without inflation, with every row corresponding to a distinct (claim, menu-option) pair surfaced by the source-document sweep or by user-seeded augmentation. The plan's `<acceptance_criteria>` row-count check `[ if ($1 >= 20 && $1 <= 50) ]` is satisfied at 45.

The plan also permitted `TBD plan 02` markers in cells that require judgement-pass review; this skeleton uses them in 46 places (LoC measurements, `Δfee/byte` bounds, golden-sha256 prefixes for specific sign-flip jobs, one anti-bribery verdict confirmation). Per the plan's `<output>` directive these are counted by category in the section above.

The orchestrator's executor prompt explicitly stated "Do NOT run `git commit`, `git add`, or `git tag` — the user has explicit standing instructions to leave changes uncommitted for manual review. Do NOT update STATE.md or ROADMAP.md (the orchestrator owns those)." This executor honoured both rules: the working tree carries the new file `docs/phase-2/coverage-check.md` and this SUMMARY.md as unstaged/untracked changes; no commit was made; STATE.md and ROADMAP.md were not modified by this executor.

## Issues Encountered

None.

## Verification

All plan acceptance checks pass:

```
test -f docs/phase-2/coverage-check.md → OK (file exists)
grep -q "^# Coverage Check" docs/phase-2/coverage-check.md → OK (heading present)
grep -c "^| CLM-" docs/phase-2/coverage-check.md → 45 (within 20-50 range)
grep -q "Hash-diversity gate" docs/phase-2/coverage-check.md → OK
grep -q "anti-bribery" docs/phase-2/coverage-check.md → OK
grep -q "signal-source-anchoring" docs/phase-2/coverage-check.md → OK
grep -q "standard-user-fee-drift-exposure" docs/phase-2/coverage-check.md → OK
grep -q "implementation-complexity" docs/phase-2/coverage-check.md → OK

Menu-option enum: extracted column 3 of CLM- rows, sort -u → exactly 5 distinct values:
  both-dynamic-partitioned, both-dynamic-un-partitioned, priority-only-RB-reserved,
  priority-only-un-reserved, single-lane-EIP-1559-control → OK

Status enum: extracted column 4 of CLM- rows, sort -u → subset of {BACKED, UNBACKED, WEAK}
  (OUT-OF-SCOPE not used in skeleton; this is a strict subset of the four-value vocabulary) → OK

Sign-flip cells: grep -c "d4_t50_w32\|d8_t25_w32\|x4_rb_quarter" → 7 (≥ 4) → OK

Four un-anchored controller knobs: grep -c "unanchored (RSK-un-anchored-controller-knobs)" → 20 (≥ 4) → OK

User-seeded structural / calibration claims:
  reorg-safe | reorg safety | Family B closes WR-1 | chain-derived → 8 hits → OK
  rb-generation-probability | activeSlotsCoeff → 5 hits → OK
  epoch-582 | epoch 582 | mass-stratified → 5 hits → OK

Abbreviations expanded on first use:
  Cardano Improvement Proposal (CIP) → present → OK
  Lines-of-Code (LoC) → present → OK
  Bias-corrected and accelerated (BCa) bootstrap → present → OK
  Ethereum Improvement Proposal 1559 (EIP-1559) → present → OK
```

## Known Stubs

None. The 46 `TBD plan 02` markers are NOT stubs in the project-skill sense — they are the D-08-permitted "plan handoff" placeholders for fields that require plan 02-02's judgement pass (LoC measurements, `Δfee/byte` bounds, sign-flip golden hash prefixes, one anti-bribery verdict confirmation). The plan's `<action>` block in Task 2 explicitly allows them: "Cells that require plan 02 judgement carry `TBD plan 02` markers — plan 02 grep-finds and finalises them." Plan 02-02 will `grep "TBD plan 02"` and finalise each one.

## Next Phase Readiness

- **Phase 2 plan 02-02 is unblocked**: the CLM-NN namespace is fixed; the 14-column table shape is laid down; the four non-welfare property columns are present in the header with mixed-enum-plus-citation cell content on every row; the hash-diversity gate semantics line is embedded in the file header. Plan 02-02's work is finalisation: grep-find each `TBD plan 02` marker, replace with the finalised content (LoC delta measurement, `Δfee/byte` bound, golden-sha256 prefix, anti-bribery verdict), walk the four UNRESOLVED suites' `sim-rs/output/` directories for the `EXP-unresolved-output-read` pass to promote UNBACKED → WEAK where data exists, run the cross-reference consistency check against the register's 24 RSK-NN identifiers.
- **Phase 3 (cheap tests) UNBACKED-row backlog is now visible**: 15 UNBACKED rows surface the Phase 3 compute priorities. The four sign-flip cells (CLM-10..13) map to `EXP-sign-flip-variance (→ TEST-03)`; the five welfare cells (CLM-01..05) plus the four comparative cells (CLM-06..09) plus the two standard-user-fee-drift-exposure cells (CLM-39..40) map variously to `EXP-canonical-variance (→ TEST-04)` and `EXP-unresolved-output-read (→ REQ-COV-06)`. Phase 3 task ordering can read this file directly and grep for UNBACKED.

## Self-Check: PASSED

Modified / created files exist and contain all required content:

- `docs/phase-2/coverage-check.md` — FOUND (created in this plan; 45 CLM-NN rows in a single flat 14-column Markdown table; header, reading guide, column legend, hash-diversity gate semantics section, notation conventions, footer marker)
- All 5 menu-option enum values present in the menu-option column — VERIFIED
- All 4 status enum values from the locked vocabulary used or available; the skeleton uses 3 (BACKED, WEAK, UNBACKED); OUT-OF-SCOPE not used (no row references a disclaimed claim) — VERIFIED
- All 4 non-welfare property columns present in the header — VERIFIED
- Hash-diversity gate semantics line present — VERIFIED
- 4 sign-flip cells surfaced as named CLM-NN rows — VERIFIED (CLM-10, CLM-11, CLM-12, CLM-13)
- 4 un-anchored controller knobs surface in signal-source-anchoring cells — VERIFIED (20 row-cells under the umbrella `RSK-un-anchored-controller-knobs` identifier)
- 5 user-seeded structural / calibration claims represented — VERIFIED
- Abbreviations expanded on first use — VERIFIED (CIP, LoC, BCa, EIP-1559, RB, EB, RTM, PSE, IQR, CV, SODA, CCS, AFT all present in their first-use expanded form)

Commits not yet made per the user's no-auto-commit memory rule and the orchestrator's explicit no-commit directive in the executor prompt. The user will commit the working-tree changes themselves.

---

*Phase: 02-coverage-check-skeleton*
*Plan: 01*
*Completed: 2026-05-15*
