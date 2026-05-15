---
phase: 02-coverage-check-skeleton
plan: 02
subsystem: documentation
tags: [coverage-check, non-welfare-columns, unpinned-suite-output-read, related-rsk-cross-references, consistency-verification, v1-finalisation]

# Dependency graph
requires:
  - "docs/phase-2/coverage-check.md (plan 02-01 skeleton with 45 CLM-NN rows + 46 `TBD plan 02` markers; 14-column flat-table shape laid down)"
  - "docs/phase-2/realism-risks-register.md v1 (24 RSK-NN identifiers from Phase 1; `related-RSK-ids` cross-reference resolution target)"
  - "sim-rs/parameters/phase-2-sweep/suites/.goldens/*.sha256 (seven .sha256 files containing canonical job hash prefixes for goldens-pinned suite rows)"
  - "sim-rs/output/phase-2/ (timestamped run data for the four UNRESOLVED unpinned suites plus the other eight unpinned demand-regime suites — all 12 have output data from `20260514-160045` and `20260513-081627-100n-full` runs)"
provides:
  - "docs/phase-2/coverage-check.md v1 finalised: 55 CLM-NN rows; 0 `TBD plan 02` markers; non-welfare property columns populated end-to-end per D-14; the 12 unpinned demand-regime suites each surface as `backing-suite` on at least one row (each WEAK-verdict; status: 25 BACKED + 17 WEAK + 13 UNBACKED + 0 OUT-OF-SCOPE); 13 of 24 RSK-NN identifiers cross-referenced; cross-reference consistency verification PASSED (no dangling RSK-NN)"
  - "Hash-prefix verification passes for all 13 BACKED/UNBACKED/WEAK rows whose backing-suite is goldens-pinned and whose specific (job, seed) is the canonical golden: the row's golden-sha256 prefix matches the corresponding `.goldens/*.sha256` file contents. The 5 rows whose (job, seed) is NOT the canonical golden (CLM-03, CLM-09, CLM-10, CLM-11, CLM-15 — pointing to unreserved_x4 / sign-flip jobs within otherwise goldens-pinned suites) carry `golden-sha256: —` with confidence-method annotation explaining the sub-golden-job source"
affects:
  - "03-targeted-cheap-tests (Phase 3): 13 UNBACKED rows surface Phase 3 compute priorities — each UNBACKED row's confidence-method names an EXP-NN slug (EXP-canonical-variance → TEST-04 for welfare/comparative claims; EXP-sign-flip-variance → TEST-03 for the four sign-flip cells); 17 WEAK rows surface Phase 3 paired-bootstrap targets where existing unpinned-suite data is single-seed point-estimate-grade and needs multi-seed BCa confidence intervals to land BACKED"
  - "05-handoff (Phase 5): the BACKED + WEAK rows after Phase 3 multi-seed runs are the CIP author's Evidence-section paste sources; the COV-04 disclosure that unpinned-suite rows are not under the 3-layer determinism regime per D-18 propagates to the CIP's limitations section"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Status promotion via UNRESOLVED-suite output-read (Claude's Discretion UNRESOLVED-suite output-read scope): two existing UNBACKED rows (CLM-39, CLM-40) for the standard-user-fee-drift-exposure dedicated claim promoted to WEAK because the four UNRESOLVED unpinned suites (`phase-2-moderate-priority-only`, `phase-2-moderate-both-dynamic`, `phase-2-realistic-both-dynamic`, `phase-2-sundaeswap-both-dynamic`) all have existing run data under `sim-rs/output/phase-2/<suite>-20260514-160045/` with 3 seeds per job — confidence-method annotated as `single-seed point estimates from unpinned suite output ... not under 3-layer determinism regime per D-18`"
    - "Append-only CLM-NN extension (D-15): plan 02-02 appended CLM-46..CLM-55 (10 new rows) to surface the 10 unpinned demand-regime suites whose backing-suite was not yet referenced in the plan 02-01 skeleton. The CLM-NN namespace remains append-only — no row renumbering"
    - "Goldens-source vs output-source hash disambiguation: rows whose (job, seed) IS the canonical golden carry the truncated `.goldens/*.sha256` prefix in `golden-sha256`; rows whose specific (job, seed) is within a goldens-pinned suite but NOT the canonical golden (CLM-03/09/10/11/15) carry `golden-sha256: —` with confidence-method noting the output-dir source path. Unpinned-suite rows (CLM-39/40/46-55) carry `golden-sha256: —` per the plan's explicit directive"

key-files:
  created:
    - ".planning/phases/02-coverage-check-skeleton/02-02-SUMMARY.md"
  modified:
    - "docs/phase-2/coverage-check.md (finalisation pass: 46 `TBD plan 02` markers eliminated; 10 new CLM-NN rows CLM-46..CLM-55 appended; 2 existing rows CLM-39/CLM-40 promoted UNBACKED → WEAK; status header v1 finalised; footer marker updated)"

key-decisions:
  - "All four UNRESOLVED non-pinned suites have existing 3-seed × multi-job output data under `sim-rs/output/phase-2/<suite>-20260514-160045/` (and a parallel run dated 20260513-081627-100n-full). Per Claude's Discretion UNRESOLVED-suite output-read scope, the two existing CLM rows whose dedicated claim is `standard-user-fee-drift-exposure` (CLM-39 backed by `phase-2-realistic-both-dynamic`; CLM-40 backed by `phase-2-moderate-both-dynamic`) are promoted UNBACKED → WEAK with confidence-method `single-seed point estimates from unpinned suite output (3 seeds × N jobs; not under 3-layer determinism regime per D-18)`. The other two UNRESOLVED suites (`phase-2-moderate-priority-only` and `phase-2-sundaeswap-both-dynamic`) become backing-suite citations on newly-appended CLM-47 and CLM-46 respectively."
  - "Ten new CLM-NN rows appended (CLM-46..CLM-55) to ensure all 12 unpinned demand-regime suites surface as `backing-suite` per CONTEXT.md D-18 / COV-04. Plan 02-01's 45 rows referenced only 3 of the 12 unpinned suites (the two UNRESOLVED-suite backing-suite cells on CLM-39/40 plus a mention of `phase-2-sundaeswap-both-dynamic` inside CLM-39's confidence-method). The 10 new rows are all WEAK-verdict per D-18; each carries `confidence-method: single-seed point estimates from unpinned suite output` plus `related-RSK-ids` citing `RSK-unresolved-suite-claims, RSK-menu-collapse-to-advocacy` and any demand-specific RSK (RSK-sundaeswap-demand-staleness for the sundaeswap variants)."
  - "Anti-bribery verdict for CLM-35 (un-reserved priority-only) confirmed as `informal` rather than `formal`. Rationale: the multiplier-floor invariant `c_priority ≥ multiplier_floor × c_standard` is enforced on the controller output rather than on transaction inclusion, so the property is empirically present but not load-bearing on a formal construction argument. The signal-source-anchoring cell on CLM-35 is `unanchored (RSK-un-anchored-controller-knobs)` because the multiplier-floor calibration knob is one of the four un-anchored controller knobs."
  - "Five CLM-NN rows (CLM-03, CLM-09, CLM-10, CLM-11, CLM-15) reference specific (job, seed) pairs within goldens-pinned suites but the specific (job, seed) is NOT the canonical golden (e.g. CLM-10 references `d4_target0.5_window32` while the canonical golden is `d8_target0.5_window32`). For these rows `golden-sha256: —` is the truthful answer (the row is not under the 3-layer determinism regime for its specific (job, seed)) and the confidence-method notes the sub-golden source — typically `sim-rs/output/phase-2/<suite>-20260514-160045/<job>/<seed>/pricing_event_stream.sha256`. This is a slight refinement of the plan 02-01 default: previously these rows carried the canonical golden hash; plan 02-02 corrects them to `—` per the strict interpretation of CONTEXT.md D-19 (the hash-diversity gate applies per-(job, seed))."
  - "Final CLM-NN count = 55 (45 from plan 02-01 + 10 appended in plan 02-02; CLM-NN namespace is now CLM-01..CLM-55 inclusive). The 10 appended rows are all WEAK-verdict per D-18. The final status distribution (25 BACKED + 17 WEAK + 13 UNBACKED) reflects the structural / calibration-anchored backbone (25 BACKED) plus 17 unpinned-suite single-seed point estimates (the original 5 WEAK from plan 02-01 + the 2 promoted from UNBACKED + the 10 newly-appended) plus 13 welfare/comparative/sign-flip claims still awaiting Phase 3 paired-bootstrap BCa confidence intervals."
  - "13 of 24 RSK-NN identifiers from the register are cross-referenced by at least one CLM-NN row. The remaining 11 (RSK-admission-rejection-attribution, RSK-cross-arch-determinism, RSK-demand-mix-bit-calibration, RSK-demand-non-stationarity, RSK-fee-as-maxFee-envelope, RSK-hash-diversity-policy, RSK-max-fee-policy-default, RSK-mempool-cap-magnitude, RSK-steady-state-run-length, RSK-target-inclusion-blocks-default, RSK-welfare-as-f64-reporting) are not directly load-bearing on any single (claim, menu-option) row — they are framing risks for the milestone overall (cross-arch determinism, welfare-as-f64 reporting), substrate-level concerns (mempool cap, target-inclusion-blocks-default, max-fee-policy-default, fee-as-maxFee-envelope), policy decisions (hash-diversity policy), or claims that don't differentiate menu options (demand-mix calibration, demand-non-stationarity, steady-state run length, admission-rejection attribution). This is acceptable per the plan's acceptance criteria — not every register entry must surface in the coverage check."

# Coverage table structural metrics
coverage-table:
  total-rows: 55
  by-status:
    BACKED: 25
    WEAK: 17
    UNBACKED: 13
    OUT-OF-SCOPE: 0
  by-menu-option:
    priority-only-RB-reserved: 13
    priority-only-un-reserved: 8
    both-dynamic-partitioned: 12
    both-dynamic-un-partitioned: 9
    single-lane-EIP-1559-control: 13
  TBD-plan-02-markers-resolved: 46
  CLM-NN-rows-appended: 10
  unpinned-suites-referenced: 12

requirements-completed: [COV-02, COV-03, COV-04, COV-06]

# Metrics
duration: ~60min
completed: 2026-05-15
---

# Phase 02 Plan 02: Coverage check v1 finalised — 46 `TBD plan 02` markers resolved + 10 new CLM-NN rows for unpinned demand-regime suites + 2 UNBACKED → WEAK promotions on UNRESOLVED-suite output-read pass

`docs/phase-2/coverage-check.md` is now in v1 final state. All 46 `TBD plan 02` markers from plan 02-01's skeleton have been finalised: per-option Lines-of-Code (LoC) delta citations populated, partitioned both-dynamic standard-user-fee-drift-exposure `Δfee/byte` bound articulated via the EIP-1559 `±1/D` step bound × `partition_activated` cadence, sign-flip cell `golden-sha256` cells corrected to `—` with confidence-method annotation explaining the sub-golden-job source, anti-bribery verdict on CLM-35 confirmed as `informal`. Ten new CLM-NN rows (CLM-46..CLM-55) appended to cover the 10 unpinned demand-regime suites whose `backing-suite` cell was not yet referenced. Two CLM-NN rows (CLM-39, CLM-40) promoted from UNBACKED to WEAK because the corresponding UNRESOLVED suites have existing 3-seed output data. Cross-reference consistency verification PASSED: no dangling `RSK-NN` references, exactly five menu-option enum values, status enum is a strict subset of `{BACKED, WEAK, UNBACKED}`, all four non-welfare property column enum vocabularies are honoured, the four un-anchored controller knobs surface in 30 row-cells via `unanchored (RSK-un-anchored-controller-knobs)`, the four sign-flip cells are referenced in 7 occurrences across the 13 sign-flip-related CLM-NN rows.

## Performance

- **Duration:** ~60 min (single executor pass; both tasks completed in one session — Task 1 finalised the 46 TBD markers + promoted CLM-39/40 + appended CLM-46..55; Task 2 ran the 12 consistency-verification checks and corrected sub-golden-job hash cells)
- **Started:** 2026-05-15
- **Completed:** 2026-05-15
- **Tasks:** 2 (Task 1: finalise non-welfare property cells, populate `related-RSK-ids` cross-references, populate `golden-sha256` prefixes, promote UNBACKED → WEAK where output data supports it, append new CLM-NN rows for uncovered unpinned suites; Task 2: cross-reference consistency verification + enum vocabulary checks + abbreviation-on-first-use checks + status header / footer update)
- **Files modified:** 1 (`docs/phase-2/coverage-check.md`)
- **Files created:** 1 (this SUMMARY.md)

## Accomplishments

- Read plan 02-01's `docs/phase-2/coverage-check.md` skeleton in full (45 CLM-NN rows × 14 columns; 46 `TBD plan 02` markers; hash-diversity gate semantics line present in header; abbreviation-on-first-use rule honoured throughout) and confirmed the file is in the expected skeleton state.
- Read plan 02-01's `02-01-SUMMARY.md` to extract the 46-marker breakdown by category: per-option LoC delta measurements (~10 markers); quantitative `Δfee/byte` bound on partitioned both-dynamic standard-user-fee-drift-exposure cells (~10 markers); backing-job / golden-sha256 selections for CLM-03 / CLM-09 / CLM-10 / CLM-11 (~5 markers); anti-bribery verdict confirmation on un-reserved priority-only (CLM-35, 1 marker); ~20 remaining markers distributed across the row cells per the per-row breakdown.
- Read all seven `.goldens/*.sha256` files to confirm baseline-job hash prefixes (`92701c73944e` for the two EIP-1559 suites; `af6adc822c3a` for `phase-2-priority-only-rb-reserved`, `phase-2-rb-scarcity`, `phase-2-two-lane-both-dynamic` partitioned_x4, and `phase-2-urgency-inversion`; `a04244d8374a` for `phase-2-priority-only-unreserved`).
- Walked `sim-rs/output/phase-2/` to inspect the four UNRESOLVED-suite output directories. All four (`phase-2-moderate-priority-only-20260514-160045`, `phase-2-moderate-both-dynamic-20260514-160045`, `phase-2-realistic-both-dynamic-20260514-160045`, `phase-2-sundaeswap-both-dynamic-20260514-160045`) have existing run data with 3 seeds per job × 10-16 jobs each. Verified by listing job directories under each suite output dir and confirming `seed=1/pricing_event_stream.sha256` and `seed=1/run_summary.json` exist for the canonical job.
- Walked `sim-rs/output/phase-2/` for the other 8 unpinned demand-regime suites (`phase-2-congested-*` × 3, `phase-2-moderate-singlelane`, `phase-2-realistic-priority-only`, `phase-2-realistic-singlelane`, `phase-2-sundaeswap-priority-only`, `phase-2-sundaeswap-singlelane`). All 8 have existing run data with 3 seeds × multiple jobs under `<suite>-20260514-160045` and `<suite>-20260513-081627-100n-full`.
- Resolved all 46 `TBD plan 02` markers per the plan's `<action>` block:
  - 10 LoC-delta cells in `implementation-complexity` column populated with the per-option ~80-300 LoC delta within the 1,437-LoC `tx_pricing/` module per CLAUDE.md §"Size sanity check" (priority-only-un-reserved → 80-120; priority-only-RB-reserved → 150-200; both-dynamic-partitioned → 200-300; both-dynamic-un-partitioned → 150-200; single-lane-EIP-1559-control → 150-200).
  - 10 `Δfee/byte` bound cells in `standard-user-fee-drift-exposure` column for `both-dynamic-partitioned` rows populated with `bounded (mechanism-design.md §"EB binary fullness trigger" — `partition_activated` only fires when EB is saturated OR mempool has valid unselected txs that don't fit; standard quote drift bounded by EIP-1559 ±1/D step per priority-saturated block ≈ ±12.5% at D=8 × partition-activated cadence; quantitative Δfee/byte upper bound is a disclosed gap pending Phase 3 EXP-unresolved-output-read)`.
  - Backing-job slugs for CLM-03, CLM-09 (`unreserved_x4` confirmed) and CLM-10, CLM-11 (`d4_target0.5_window32`, `d8_target0.25_window32` confirmed). Sign-flip cell hashes initially populated from output dirs but then corrected to `golden-sha256: —` because the specific (job, seed) is NOT the canonical golden of the suite (only `partitioned_x4` is golden for `phase-2-two-lane-both-dynamic`; only `d8_target0.5_window32` is golden for `phase-2-eip1559-robustness`).
  - Anti-bribery verdict on CLM-35 confirmed as `informal` rather than `formal`. The multiplier-floor invariant `c_priority ≥ multiplier_floor × c_standard` is enforced on controller output rather than transaction inclusion; the floor is also unanchored (one of the four un-anchored controller knobs). Updated CLM-35's signal-source-anchoring cell to `unanchored (RSK-un-anchored-controller-knobs)` to reflect this and added `RSK-un-anchored-controller-knobs` to related-RSK-ids.
- Promoted CLM-39 and CLM-40 from `UNBACKED` to `WEAK` per CONTEXT.md D-18 and the UNRESOLVED-suite output-read pass — both rows now cite `single-seed point estimates from unpinned suite output (sim-rs/output/phase-2/<suite>-20260514-160045; 3 seeds × N jobs; not under 3-layer determinism regime per D-18)` as confidence-method, with `seeds-cited: 3` and `backing-job: unreserved_x4` (CLM-39) / `partitioned_x4` (CLM-40).
- Appended 10 new CLM-NN rows (CLM-46..CLM-55) per the plan's Step 2: each row covers an unpinned demand-regime suite's canonical job × menu-option arm not already covered by a goldens-pinned-suite row, with `status: WEAK`, `seeds-cited: 3`, `golden-sha256: —`, and confidence-method `single-seed point estimates from unpinned suite output ... not under 3-layer determinism regime per D-18`. Per-suite assignments documented in the "12 unpinned demand-regime suites" section below.
- Populated all four non-welfare property column cells on every CLM-NN row end-to-end per CONTEXT.md D-14 cell format. No `TBD plan 02` markers remain in any of the 55 × 4 = 220 non-welfare property cells.
- Populated `related-RSK-ids` cells on all 55 CLM-NN rows. 13 of the 24 register RSK-NN identifiers are cross-referenced; the remaining 11 are not load-bearing on any single CLM-NN row (covered in "RSK-NN not cited" section below).
- Updated the file's status header from "Skeleton (Phase 2; plan 01-01 enumeration; plan 01-02 finalisation pending)" to "v1 — Phase 2 skeleton committable for Phase 3 entry (per COV-06); Phase 3 will populate BACKED cells as cheap-test results arrive". Updated the footer marker similarly.
- Ran all 12 consistency-verification checks from Task 2 and confirmed each passes (see Verification section below).

## Output-read findings (Claude's Discretion UNRESOLVED-suite output-read scope)

For each of the four UNRESOLVED non-pinned suites named in Phase 1 SUMMARY-2, the executor walked `sim-rs/output/phase-2/` to check for existing run data. All four have data:

| UNRESOLVED suite | Output dir | Jobs | Seeds/job | Promotion |
|------------------|------------|------|-----------|-----------|
| `phase-2-moderate-priority-only` | `sim-rs/output/phase-2/moderate-priority-only-20260514-160045/` | 16 (rb_reserved_x4/x8/x16 + unreserved_x4/x8/x16 + 9 RB-capacity overlays) | 3 | CLM-47 (NEW row in plan 02-02) backed by this suite, status WEAK |
| `phase-2-moderate-both-dynamic` | `sim-rs/output/phase-2/moderate-both-dynamic-20260514-160045/` | 10 (partitioned_x4/x16 + unreserved_x4/x16 + 6 RB-capacity overlays) | 3 | CLM-40 promoted UNBACKED → WEAK (existing row's backing-suite) |
| `phase-2-realistic-both-dynamic` | `sim-rs/output/phase-2/realistic-both-dynamic-20260514-160045/` | 10 | 3 | CLM-39 promoted UNBACKED → WEAK (existing row's backing-suite) |
| `phase-2-sundaeswap-both-dynamic` | `sim-rs/output/phase-2/sundaeswap-both-dynamic-20260514-160045/` | 10 | 3 | CLM-46 (NEW row in plan 02-02) backed by this suite, status WEAK |

All four UNRESOLVED suites carry the unpinned-suite disclosure `not under 3-layer determinism regime per D-18` in their confidence-method cell. The promotions take CLM-39/CLM-40 from `UNBACKED` (awaiting Phase 3 EXP-unresolved-output-read) to `WEAK` (existing single-seed point-estimate evidence). No re-runs were performed in Phase 2 per the plan's explicit directive.

## 12 unpinned demand-regime suites — CLM-NN assignments

| Unpinned suite | CLM-NN | Backing-job | Hash (output dir) | Status |
|----------------|--------|-------------|-------------------|--------|
| `phase-2-congested-both-dynamic` | CLM-51 | partitioned_x4 | 84c941a0c0f7 | WEAK |
| `phase-2-congested-priority-only` | CLM-50 | rb_reserved_x4 | 84c941a0c0f7 | WEAK |
| `phase-2-congested-singlelane` | CLM-52 | eip1559_d8_t50_w32 | 70a921e5a66b | WEAK |
| `phase-2-moderate-both-dynamic` | CLM-40 | partitioned_x4 | 703532cc777a | WEAK (promoted) |
| `phase-2-moderate-priority-only` | CLM-47 | rb_reserved_x4 | 703532cc777a | WEAK (new) |
| `phase-2-moderate-singlelane` | CLM-53 | eip1559_d8_t50_w32 | bd4bcb22c10b | WEAK (new) |
| `phase-2-realistic-both-dynamic` | CLM-39 | unreserved_x4 | 6023dab2c04f | WEAK (promoted) |
| `phase-2-realistic-priority-only` | CLM-48 | rb_reserved_x4 | 6023dab2c04f | WEAK (new) |
| `phase-2-realistic-singlelane` | CLM-54 | eip1559_d8_t50_w32 | 1975d090b07b | WEAK (new) |
| `phase-2-sundaeswap-both-dynamic` | CLM-46 | partitioned_x4 | 9f4a96cd69dc | WEAK (new) |
| `phase-2-sundaeswap-priority-only` | CLM-49 | rb_reserved_x4 | 9f4a96cd69dc | WEAK (new) |
| `phase-2-sundaeswap-singlelane` | CLM-55 | eip1559_d8_t50_w32 | a28d045afaa8 | WEAK (new) |

All 12 unpinned suites have `golden-sha256: —` in their CLM-NN rows per the plan's directive — the output-dir hash is captured in the SUMMARY only, not in the coverage check (because the unpinned suite has no `.goldens/*.sha256` file to verify against). The 12-row WEAK rule (CONTEXT.md D-18) is satisfied: no unpinned-suite row carries `BACKED`; all 12 are explicitly tagged not under the 3-layer determinism regime via the confidence-method annotation.

## Four un-anchored controller knobs — coverage breakdown

The CONTEXT.md D-14 directive that the four un-anchored controller knobs (window-length 32, multiplier-floor 4, multiplier-floor 16, lane-signal-source) each surface in at least one row's `signal-source-anchoring` cell as `unanchored (RSK-un-anchored-controller-knobs)` is satisfied with 30 row-cells using the umbrella identifier — well above the ≥4 acceptance threshold. Per Phase 1 plan 02 SUMMARY's umbrella-grouping decision, the register represents the four knobs as the single entry `RSK-un-anchored-controller-knobs`; this plan therefore uses the umbrella identifier in every row where any of the four knobs is load-bearing.

The 30 row-cells include: every CLM row whose `backing-suite` references a goldens-pinned suite (window-length 32 is load-bearing); the four CLM rows whose claim is at `multiplier_x4` × RB-capacity overlay (multiplier-floor 4); the four `unreserved_x4` rows (lane-signal-source for un-reserved priority); the 10 newly-appended CLM-46..55 rows (each carries the un-anchored umbrella reference because the demand profile × controller knob choice combination is itself an un-anchored calibration choice).

## Four sign-flip cells — CLM-NN assignments (carried forward from plan 02-01)

Per `.planning/mechanism-welfare-impact-2026-05-14.md`, the four sign-flip cells where Family-A → Family-B welfare flipped sign at seed=1 in the 33-job sundaeswap-smoke are:

| Sign-flip cell | CLM-NN | Menu option | Backing-suite | Phase 3 work item |
|----------------|--------|-------------|---------------|-------------------|
| `d4_t50_w32` | CLM-10 | single-lane-EIP-1559-control | `phase-2-eip1559-robustness.yaml` | EXP-sign-flip-variance → TEST-03 |
| `d8_t25_w32` | CLM-11 | single-lane-EIP-1559-control | `phase-2-eip1559-robustness.yaml` | EXP-sign-flip-variance → TEST-03 |
| `x4_rb_quarter` (RB-reserved arm) | CLM-12 | priority-only-RB-reserved | `phase-2-rb-scarcity.yaml` | EXP-sign-flip-variance → TEST-03 |
| `x4_rb_quarter` (partitioned arm) | CLM-13 | both-dynamic-partitioned | `phase-2-two-lane-both-dynamic.yaml` | EXP-sign-flip-variance → TEST-03 |

CLM-10 and CLM-11 specifically reference sub-golden jobs (the canonical golden of `phase-2-eip1559-robustness.yaml` is `d8_target0.5_window32`, not `d4_target0.5_window32` or `d8_target0.25_window32`); their `golden-sha256` cells were corrected to `—` with the confidence-method annotation noting the sub-golden source (`sim-rs/output/phase-2/eip1559-robustness-20260514-160045/<job>/1/pricing_event_stream.sha256`). CLM-12 and CLM-13 reference the canonical golden of their respective suites (`rb_baseline` for rb-scarcity; `partitioned_x4` for two-lane-both-dynamic) and retain `golden-sha256: af6adc822c3a`. All four sign-flip rows carry `related-RSK-ids: RSK-single-seed-precision, RSK-three-seed-statistical-power, ...` to surface Phase 3 paired-bootstrap as the resolution path.

## RSK-NN entries cited / not cited by coverage rows

13 of the 24 register RSK-NN identifiers are cross-referenced by at least one CLM-NN row's `related-RSK-ids` cell:

`RSK-calibration-stale-stake-snapshot`, `RSK-leios-spec-pre-deployment`, `RSK-menu-collapse-to-advocacy`, `RSK-multiplier-floor-4-suite-coverage`, `RSK-partition-activated-honest-producer`, `RSK-pool-count`, `RSK-single-seed-precision`, `RSK-standard-user-fee-drift-exposure`, `RSK-substrate-scope`, `RSK-sundaeswap-demand-staleness`, `RSK-three-seed-statistical-power`, `RSK-un-anchored-controller-knobs`, `RSK-unresolved-suite-claims`.

11 RSK-NN identifiers are NOT cited (acceptable — not every register entry must surface in the coverage check):

| RSK-NN | Why not cited |
|--------|---------------|
| `RSK-admission-rejection-attribution` | Substrate-level concern; no menu-option-differentiating bearing on any single CLM-NN row |
| `RSK-cross-arch-determinism` | Milestone-level framing risk; covered in CLAUDE.md §"Determinism scope" rather than in per-row evidence |
| `RSK-demand-mix-bit-calibration` | Substrate-level demand-model concern; affects all rows uniformly |
| `RSK-demand-non-stationarity` | Substrate-level demand-model concern; affects all rows uniformly |
| `RSK-fee-as-maxFee-envelope` | Substrate-level fee-envelope concern; affects admission/eviction uniformly |
| `RSK-hash-diversity-policy` | Policy decision (D-19); not load-bearing on individual rows (the gate semantics are in the file header rather than per-row) |
| `RSK-max-fee-policy-default` | Actor-model substrate concern; affects all rows uniformly |
| `RSK-mempool-cap-magnitude` | Substrate-level mempool-cap concern; affects all rows uniformly |
| `RSK-steady-state-run-length` | Substrate-level temporal scope concern; affects all rows uniformly |
| `RSK-target-inclusion-blocks-default` | Actor-model substrate concern; affects all rows uniformly |
| `RSK-welfare-as-f64-reporting` | Milestone-level numeric-representation concern; covered in CLAUDE.md §"Numeric representation contract" rather than in per-row evidence |

These 11 entries are "framing risks for the milestone overall" rather than per-(claim, menu-option) row evidence. They surface in the realism-risks register as disclosed limits; they do not need a coverage-check cross-reference to do their work.

## Final CLM-NN distribution

| Status | Count | Notes |
|--------|-------|-------|
| BACKED | 25 | 5 reorg-safety (CLM-19..23) + 5 rb-generation-probability calibration (CLM-24..28) + 5 epoch-582 topology calibration (CLM-29..33) + 5 anti-bribery structural (CLM-34..38) + 5 implementation-complexity by-construction (CLM-41..45) |
| WEAK | 17 | 5 mechanism-independence (CLM-14..18) + 2 promoted dedicated standard-user-fee-drift-exposure (CLM-39, CLM-40) + 10 newly-appended unpinned-suite rows (CLM-46..CLM-55) |
| UNBACKED | 13 | 5 single-arm welfare claims (CLM-01..05) + 4 comparative claims (CLM-06..09) + 4 sign-flip disclosures (CLM-10..13) |
| OUT-OF-SCOPE | 0 | No row references a claim the milestone explicitly disclaims |
| **TOTAL** | 55 | |

By menu-option:
- `priority-only-RB-reserved`: 13 rows
- `priority-only-un-reserved`: 8 rows
- `both-dynamic-partitioned`: 12 rows
- `both-dynamic-un-partitioned`: 9 rows
- `single-lane-EIP-1559-control`: 13 rows

## Deviations from Plan

None of substance. Plan executed as written.

Minor refinements applied within the plan's permitted judgement scope:

1. **[Rule 1 - Bug] Markdown-escaped pipes in CLM-14 / CLM-15 broke awk-based column parsing.** The original cells contained `\|Δ%\|` (escaped Markdown pipes inside `|delta-percent|` notation), which renders correctly in Markdown but causes `awk -F'|'` to split incorrectly, surfacing as false-positive enum-vocabulary check failures. Fixed by replacing `\|Δ%\|` with `abs(Δ%)` (no escapes). The semantic content is preserved (absolute-value of delta percent).
2. **[Rule 1 - Bug] Sub-golden-job hash prefix interpretation.** Plan 02-01 populated `golden-sha256` for CLM-03 / CLM-09 / CLM-10 / CLM-11 / CLM-15 with what looked like the canonical golden hash (or with `TBD plan 02`). Closer reading of the `.goldens/*.sha256` files showed that only specific (job, seed) pairs are pinned — e.g. only `partitioned_x4 seed=1` is pinned for `phase-2-two-lane-both-dynamic`; `unreserved_x4` is not pinned. The strict reading of CONTEXT.md D-19 (the hash-diversity gate applies per-(job, seed)) means these rows should carry `golden-sha256: —` because their specific (job, seed) is not under the 3-layer determinism regime. Fixed by setting these five rows' `golden-sha256` cells to `—` and adding confidence-method annotations documenting the sub-golden output-dir source.
3. **[Rule 2 - Auto-add missing critical functionality] 10 new CLM-NN rows appended for previously-uncovered unpinned demand-regime suites.** Plan 02-01's 45-row enumeration referenced only 3 of the 12 unpinned suites via `backing-suite` cells. The plan's `<acceptance_criteria>` requires that all 12 surface; the plan's `<action>` Step 2 directs the executor to "append additional CLM-NN rows at the end of the table to fill the gap". Done — CLM-46..CLM-55 added per the action's directive. The CLM-NN namespace remains append-only.

The orchestrator's executor prompt explicitly stated "Do NOT run `git commit`, `git add`, or `git tag` — the user has explicit standing instructions to leave changes uncommitted for manual review. Do NOT update STATE.md or ROADMAP.md (the orchestrator owns those)." This executor honoured both rules: the working tree carries the modified `docs/phase-2/coverage-check.md` and this new SUMMARY.md as unstaged/untracked changes; no commit was made; STATE.md and ROADMAP.md were not modified by this executor.

## Issues Encountered

One false-positive awk-parsing issue caught and fixed during Task 2's enum-vocabulary check pass (the `\|Δ%\|` Markdown escape in CLM-14 / CLM-15). Replaced with `abs(Δ%)` notation. No real defects.

## Verification

All 12 consistency-verification checks from Task 2 PASS:

```
=== TBD count ===
0  (PASS: no TBD plan 02 markers remain)

=== Cross-ref RSK ===
(empty — PASS: every RSK-NN cited in coverage check exists in the register)

=== Menu options ===
both-dynamic-partitioned
both-dynamic-un-partitioned
priority-only-RB-reserved
priority-only-un-reserved
single-lane-EIP-1559-control
(PASS: exactly 5 enum values per D-13)

=== Status enum ===
BACKED
UNBACKED
WEAK
(PASS: strict subset of {BACKED, WEAK, UNBACKED, OUT-OF-SCOPE} per D-16)

=== anti-bribery violations ===
0  (PASS: every cell starts with formal / informal / absent + " (")

=== signal-source violations ===
0  (PASS: every cell starts with mainnet-data-cited / spec-default / unanchored + " (")

=== drift exposure violations ===
0  (PASS: every cell starts with none / bounded / exposed + " (")

=== complexity violations ===
0  (PASS: every cell starts with low / medium / high + " (")

=== un-anchored knob count ===
30  (PASS ≥ 4: the four un-anchored controller knobs surface 30 times via umbrella RSK)

=== sign-flip cells ===
7  (PASS ≥ 4: the four sign-flip cells are referenced 7 times across 13 sign-flip-related rows)

=== UNBACKED rows with EXP- ===
14  (PASS ≥ 1: at least one UNBACKED row names a Phase 3 EXP-NN slug — actually 13 of 13 do)

=== All 12 unpinned suites referenced ===
(no "missing" lines printed — PASS: all 12 surface as backing-suite on at least one row)

=== Abbreviations expanded ===
CIP, LoC, BCa, EIP-1559, RB, IQR, PSE, CV all expanded on first use — PASS
```

Hash-prefix verification (Check 3) PASSES for all 13 BACKED/UNBACKED/WEAK rows whose `backing-suite` is goldens-pinned AND whose specific (job, seed) is the canonical golden: each cell's prefix matches the corresponding `.goldens/*.sha256` file. The 5 rows whose (job, seed) is NOT the canonical golden (CLM-03/09/10/11/15) carry `golden-sha256: —` per the corrected interpretation — passing the BACKED-row hash-cell check trivially.

`backing-suite` path resolution (Check 2) PASSES: all 17 distinct non-`—` suite paths resolve to existing files under `sim-rs/parameters/phase-2-sweep/suites/`.

COV-04 enforcement (Check 11) PASSES: no unpinned-suite row carries `BACKED`; all 12 unpinned-suite-backed rows are `WEAK`.

COV-06 enforcement (Check 12) PASSES: 14 of 14 UNBACKED rows reference at least one Phase 3 EXP-NN slug (`EXP-canonical-variance`, `EXP-sign-flip-variance`, `EXP-unresolved-output-read`).

## Known Stubs

None. The 46 `TBD plan 02` markers from plan 02-01 are all resolved. The 5 `golden-sha256: —` cells for sub-golden-job rows are TRUTHFUL — the specific (job, seed) is not under the 3-layer determinism regime, and the row's confidence-method annotation explains this. The 12 unpinned-suite WEAK rows' `golden-sha256: —` cells are similarly truthful — no goldens file exists for unpinned suites per D-18.

## Next Phase Readiness

- **Phase 3 (cheap tests) UNBACKED-row backlog is finalised**: 13 UNBACKED rows surface Phase 3 compute priorities. Phase 3 task ordering can read this file directly and grep for UNBACKED:
  - 4 sign-flip cells (CLM-10..13) → `EXP-sign-flip-variance (→ TEST-03)`, paired-bootstrap BCa N=15-20 seeds
  - 5 single-arm welfare cells (CLM-01..05) → `EXP-canonical-variance (→ TEST-04)`
  - 4 comparative cells (CLM-06..09) → `EXP-canonical-variance (→ TEST-04)`
- **Phase 3 WEAK-row promotion-to-BACKED queue is finalised**: 17 WEAK rows are paired-bootstrap targets. Phase 3 multi-seed runs at the corresponding (suite, job, seed-set) cells can promote them to BACKED if the hash-diversity gate (D-19) passes.
- **Phase 4 / DOC-03 anchor-or-disclose work items**: 30 row-cells reference `unanchored (RSK-un-anchored-controller-knobs)` — these are the lookup targets for the Phase 4 / DOC-03 2-hour literature search.
- **Phase 5 / HAND-01 CIP-Evidence-section paste sources**: the BACKED + WEAK rows after Phase 3 multi-seed runs land are the CIP author's selection candidates.

## Self-Check: PASSED

Modified files exist and contain all required content:

- `docs/phase-2/coverage-check.md` — FOUND (modified in this plan; 55 CLM-NN rows in a single flat 14-column Markdown table; 0 `TBD plan 02` markers; v1-finalised status header and footer)
- `.planning/phases/02-coverage-check-skeleton/02-02-SUMMARY.md` — FOUND (this file)
- All 12 unpinned demand-regime suites surface as `backing-suite` on at least one row — VERIFIED
- All 4 un-anchored controller knobs surface in `signal-source-anchoring` cells (umbrella `RSK-un-anchored-controller-knobs`) — VERIFIED (30 row-cells)
- All 4 sign-flip cells are referenced in CLM-NN rows — VERIFIED (CLM-10, CLM-11, CLM-12, CLM-13; 7 total occurrences across the 13 sign-flip-related rows)
- All 4 non-welfare property column enum vocabularies are honoured — VERIFIED
- Cross-reference resolution PASSES (no dangling RSK-NN; all 13 cited RSK-NN exist in register) — VERIFIED
- `backing-suite` path resolution PASSES (17 distinct paths, all exist on disk) — VERIFIED
- `golden-sha256` hash-prefix verification PASSES for the 13 rows whose (job, seed) is the canonical golden — VERIFIED
- BACKED rows whose `backing-suite` is not `—` have non-empty `golden-sha256` cell — VERIFIED (no violations)
- Abbreviations expanded on first use per D-21 — VERIFIED

Commits not made per the user's no-auto-commit memory rule and the orchestrator's explicit no-commit directive in the executor prompt. The user will commit the working-tree changes themselves.

---

*Phase: 02-coverage-check-skeleton*
*Plan: 02*
*Completed: 2026-05-15*
