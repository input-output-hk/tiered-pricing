---
phase: 02-coverage-check-skeleton
verified: 2026-05-15T00:00:00Z
status: passed
score: 5/5 must-haves verified
overrides_applied: 0
---

# Phase 2: Coverage Check Skeleton Verification Report

**Phase Goal:** A coverage check exists that maps every menu-item trade-off claim the Cardano Improvement Proposal (CIP) will make to a specific backing simulator job, including non-welfare property columns that keep the menu a menu, with gaps surfaced as `UNBACKED` rows that prioritise Phase 3 work.
**Verified:** 2026-05-15
**Status:** READY-TO-CLOSE
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths (from ROADMAP.md Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SC1 | `docs/phase-2/coverage-check.md` exists as a flat table with stable `CLM-NN` identifiers (append-only), one row per claim | VERIFIED | File exists; `grep -c "^| CLM-"` returns 55; IDs run CLM-01..CLM-55 with no gaps; no renumbering between plan 02-01 (CLM-01..45) and plan 02-02 (CLM-46..55 appended) |
| SC2 | Each `CLM-NN` row carries the full column set: `claim`, `menu-option`, `backing-suite`, `backing-job`, `seeds-cited`, `confidence-method`, `golden-sha256`, `status`, `related-RSK-ids` (14 columns total) | VERIFIED | Every row has exactly 16 pipe-delimited fields (14 data columns + 2 outer pipes); min/max both 16; verified via `awk -F'\|' '{ print NF }'` returning 16 for all 55 rows |
| SC3 | Non-welfare property columns are present alongside welfare claims — anti-bribery, standard-user-fee-drift exposure, signal-source anchoring, implementation complexity — with each cell citing a spec section, a simulator measurement, or "disclosed gap" | VERIFIED | All four non-welfare columns present in header and in every row; zero enum-format violations across all 55 × 4 = 220 cells; spot-checks on CLM-01, CLM-14, CLM-19, CLM-34, CLM-46 confirm `<enum> (<citation>)` format throughout |
| SC4 | The 12 unpinned demand-regime suites appear as `WEAK`-verdict rows where they cover claims not backed by the seven goldens-pinned suites; they are not promoted to goldens-pinned | VERIFIED | All 12 unpinned suites confirmed present as `backing-suite` cells; each carries `WEAK` status (no unpinned suite has `BACKED`); COV-04 enforcement confirmed; CLM-46..55 appended specifically to fill this requirement |
| SC5 | The skeleton is committable before Phase 3 begins: rows for claims awaiting cheap-test results carry `status: UNBACKED`, surfacing compute priorities for Phase 3 task ordering | VERIFIED | 13 UNBACKED rows present; all 13 carry EXP-NN forward references (`EXP-canonical-variance → TEST-04`, `EXP-sign-flip-variance → TEST-03`); COV-06 satisfied |

**Score:** 5/5 truths verified

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `docs/phase-2/coverage-check.md` | Flat table with CLM-NN rows, 14 columns, hash-diversity gate section | VERIFIED | 55 CLM-NN rows, 14 data columns per row, v1-finalised header and footer; `# Coverage Check` heading present |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `coverage-check.md` `related-RSK-ids` | `realism-risks-register.md` RSK-NN headings | `comm -23` cross-ref check | VERIFIED | 13 distinct RSK-NN identifiers cited; all 13 exist in the register; zero dangling references |
| `coverage-check.md` `backing-suite` | `sim-rs/parameters/phase-2-sweep/suites/*.yaml` | `test -f` path resolution | VERIFIED | All 17 distinct non-dash backing-suite paths resolve to existing suite YAML files |
| `coverage-check.md` `golden-sha256` | `sim-rs/parameters/phase-2-sweep/suites/.goldens/*.sha256` | Hash-prefix comparison | VERIFIED | Three prefix values in use: `92701c73944e` (eip1559-robustness `d8_target0.5_window32`), `af6adc822c3a` (rb-reserved/rb-scarcity/two-lane-both-dynamic `partitioned_x4`), `a04244d8374a` (priority-only-unreserved `multiplier_x4`); all match their `.sha256` file contents; non-canonical-job rows correctly carry `golden-sha256: —` |

---

## Detailed Verification Findings

### SC1 — Flat table with stable CLM-NN identifiers

`docs/phase-2/coverage-check.md` exists. The file opens with `# Coverage Check` (heading present). Row count = 55, within the 20–50 acceptance range for plan 02-01 (45 rows) and with the 10 appended in plan 02-02. IDs are sequential CLM-01 through CLM-55 with no gaps and no renumbering. The D-15 append-only invariant holds: plan 02-01 laid down CLM-01..45 and plan 02-02 extended to CLM-46..55. The status header reads "v1 — Phase 2 skeleton committable for Phase 3 entry (per COV-06)".

### SC2 — Full 14-column set on every row

Every CLM-NN row has exactly 16 pipe-delimited fields (14 data columns plus the two outer boundary pipes). The column order matches the D-17 specification and Claude's Discretion column-ordering: `id | claim | menu-option | status | confidence-method | backing-suite | backing-job | seeds-cited | golden-sha256 | anti-bribery | signal-source-anchoring | standard-user-fee-drift-exposure | implementation-complexity | related-RSK-ids`. No row is short or over-columned.

The five required menu-option enum values are all present: `both-dynamic-partitioned`, `both-dynamic-un-partitioned`, `priority-only-RB-reserved`, `priority-only-un-reserved`, `single-lane-EIP-1559-control`. The status column contains exactly {BACKED, UNBACKED, WEAK} — a strict subset of the four-value D-16 vocabulary (OUT-OF-SCOPE unused, which is correct: no disclaimed claims were enumerated).

### SC3 — Non-welfare property columns

All four non-welfare property columns populate correctly:

- **anti-bribery**: zero violations across 55 rows; values are `formal`, `informal`, or `absent`, each followed by ` (<citation>)`.
- **signal-source-anchoring**: zero violations; values are `mainnet-data-cited`, `spec-default`, or `unanchored`, each followed by ` (<citation-or-RSK>)`. The four un-anchored controller knobs (window-length 32, multiplier-floor 4, multiplier-floor 16, lane-signal-source) surface in 30 row-cells as `unanchored (RSK-un-anchored-controller-knobs)` — well above the ≥4 acceptance threshold.
- **standard-user-fee-drift-exposure**: zero violations; values are `none`, `bounded`, or `exposed`.
- **implementation-complexity**: zero violations; values are `low`, `medium`, or `high`.

All cells contain a citation, measurement reference, or `disclosed gap` reference as required by ROADMAP SC3 and COV-03. The `<enum-value> (<citation>)` D-14 format is honoured throughout.

### SC4 — 12 unpinned demand-regime suites as WEAK rows

All 12 unpinned demand-regime suites appear as `backing-suite` on at least one row, each with `WEAK` status:

- `phase-2-congested-both-dynamic` → CLM-51 (WEAK)
- `phase-2-congested-priority-only` → CLM-50 (WEAK)
- `phase-2-congested-singlelane` → CLM-52 (WEAK)
- `phase-2-moderate-both-dynamic` → CLM-40 (WEAK, promoted from UNBACKED after UNRESOLVED-suite output-read pass)
- `phase-2-moderate-priority-only` → CLM-47 (WEAK, new row appended)
- `phase-2-moderate-singlelane` → CLM-53 (WEAK)
- `phase-2-realistic-both-dynamic` → CLM-39 (WEAK, promoted from UNBACKED)
- `phase-2-realistic-priority-only` → CLM-48 (WEAK)
- `phase-2-realistic-singlelane` → CLM-54 (WEAK)
- `phase-2-sundaeswap-both-dynamic` → CLM-46 (WEAK)
- `phase-2-sundaeswap-priority-only` → CLM-49 (WEAK)
- `phase-2-sundaeswap-singlelane` → CLM-55 (WEAK)

No unpinned-suite row carries `BACKED`. The D-18 disclosure that these rows are "not under the 3-layer determinism regime per D-18" appears in every unpinned-suite row's `confidence-method` cell.

### SC5 — Committable skeleton with UNBACKED rows surfacing Phase 3 priorities

13 UNBACKED rows are present. All 13 carry EXP-NN forward references in their `confidence-method` cells:

- CLM-01..05 (welfare claims) → `EXP-canonical-variance → TEST-04`
- CLM-06..09 (comparative claims) → `EXP-canonical-variance → TEST-04`
- CLM-10..13 (sign-flip cells) → `EXP-sign-flip-variance → TEST-03`

The file header states "v1 — Phase 2 skeleton committable for Phase 3 entry (per COV-06)" and the footer confirms "Phase 3 will apply COV-05 hash-diversity gate and promote rows from UNBACKED to BACKED as multi-seed test results land." COV-06 requirement is satisfied.

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| COV-01 | 02-01-PLAN.md | Single `coverage-check.md` with stable CLM-NN identifiers | SATISFIED | File exists; 55 rows; sequential IDs; no renumbering |
| COV-02 | Both plans | Each row has the full column set (14 columns) | SATISFIED | All 55 rows have 16 pipe fields; all four non-welfare property columns populated end-to-end with zero enum violations |
| COV-03 | 02-02-PLAN.md | Non-welfare property columns with citations | SATISFIED | 220 non-welfare cells (55 × 4), each `<enum> (<citation>)` per D-14; zero violations |
| COV-04 | 02-02-PLAN.md | 12 unpinned demand-regime suites appear as WEAK rows | SATISFIED | All 12 confirmed present as WEAK; none carry BACKED |
| COV-05 | Phase 3 scope | Hash-diversity gate documented; applied in Phase 3 | DEFERRED | Gate semantics documented in file header per D-19; application is Phase 3 work; correctly deferred |
| COV-06 | Both plans | Skeleton committable before Phase 3; UNBACKED rows surface priorities | SATISFIED | 13 UNBACKED rows with EXP-NN references; v1-finalised header |

---

## Hash-Prefix Spot-Check

Three distinct hash prefixes are cited in the 17 WEAK rows that have non-dash golden-sha256 cells:

| Prefix | Suite | Job | Matches `.sha256` file |
|--------|-------|-----|------------------------|
| `92701c73944e` | `phase-2-eip1559-robustness.yaml` | `d8_target0.5_window32` | YES (`92701c73944ead391c...`) |
| `af6adc822c3a` | `phase-2-priority-only-rb-reserved.yaml`, `phase-2-rb-scarcity.yaml`, `phase-2-two-lane-both-dynamic.yaml`, `phase-2-urgency-inversion.yaml` | Various canonical jobs | YES (`af6adc822c3a9da20b...`) |
| `a04244d8374a` | `phase-2-priority-only-unreserved.yaml` | `multiplier_x4` | YES (`a04244d8374ad37c6e...`) |

Non-canonical-job rows (CLM-10, CLM-11, CLM-15) correctly carry `golden-sha256: —` per the plan 02-02's strict D-19 interpretation — their specific (job, seed) pairs are not under the 3-layer determinism regime.

All 25 BACKED rows have `backing-suite: —` (they are structural / calibration claims) and therefore `golden-sha256: —`, which is correct per plan 02-01 SC5 ("for structural claim rows, backing-suite `—` and golden-sha256 `—` are acceptable").

---

## Anti-Pattern Scan

This is a documentation-only phase. No executable code was created or modified. Scanned `docs/phase-2/coverage-check.md` for stub indicators:

- `TBD plan 02` markers: 0 (confirmed by `grep -q "TBD plan 02"` returning no match)
- Placeholder content: none found; every cell has finalised content
- Broken cross-references: none found (all 13 cited RSK-NN identifiers exist in the register; all 17 backing-suite paths resolve to existing files)

---

## CLM-NN Append-Only Invariant (D-15)

Plan 02-01 established CLM-01..CLM-45. Plan 02-02 appended CLM-46..CLM-55. No row was renumbered between plans. The CLM-NN namespace is stable and append-only.

---

## Anti-Finding Section (SUMMARY vs Actual Artefact)

The SUMMARY.md files' claims were verified against the actual artefact. The following differences between what the executor reports and what the file contains were investigated:

**COV-04 check false alarm:** The original check script used `grep -c "BACKED"` to detect BACKED status in unpinned-suite rows, which also matches "UNBACKED". Re-running with `grep -c "^BACKED$"` confirmed zero violations — CLM-04 appeared to be a hit only because its `confidence-method` cell mentions "moderate-both-dynamic" in passing; its `backing-suite` is the goldens-pinned `phase-2-two-lane-both-dynamic.yaml` and its status is `UNBACKED`, not BACKED.

**Plan 02-02 SUMMARY claim "25 BACKED + 17 WEAK + 13 UNBACKED":** Confirmed correct by direct row count.

**No material discrepancies found** between executor SUMMARY reports and the actual artefact state.

---

## Gaps Summary

No gaps. All five success criteria are met. All requirements COV-01, COV-02, COV-03, COV-04, COV-06 are satisfied. COV-05 is correctly deferred to Phase 3 per the REQUIREMENTS.md assignment decision (note at line 103: "COV-05 is mapped to Phase 3, where the gate is applied").

---

## Human Verification Required

None. This is a documentation-only phase. The artefact is a Markdown table; all properties are verifiable programmatically.

---

## Overall Verdict

**READY-TO-CLOSE**

The phase goal is achieved: `docs/phase-2/coverage-check.md` exists as a 55-row flat table mapping every menu-item trade-off claim across the five menu options to specific backing simulator jobs (or to by-construction citations / mainnet calibration triples). Non-welfare property columns are populated end-to-end. The 12 unpinned demand-regime suites appear as WEAK rows. 13 UNBACKED rows surface Phase 3 compute priorities with EXP-NN forward references. No blocking gaps exist.

Phase 3 (Targeted Cheap Tests) may begin. The UNBACKED rows provide the test-ordering backlog; the 13 UNBACKED rows map to `EXP-sign-flip-variance → TEST-03` (4 rows) and `EXP-canonical-variance → TEST-04` (9 rows).

---

_Verified: 2026-05-15_
_Verifier: Claude (gsd-verifier)_
