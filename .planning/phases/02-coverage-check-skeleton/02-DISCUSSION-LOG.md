# Phase 2: Coverage Check Skeleton - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-15
**Phase:** 2-Coverage Check Skeleton
**Areas discussed:** All 4 gray areas selected from the initial multi-select.

---

## Gray-area menu — initial selection

| Option | Description | Selected |
|--------|-------------|----------|
| Row shape: per-(claim, option) vs per-claim | Denormalised pair rows (~30–40 rows) vs normalised claim rows with option columns (~15 rows) | ✓ |
| Claim taxonomy | Welfare-only / welfare+comparative / welfare+comparative+structural+calibration | ✓ |
| Non-welfare cell semantics | Enum-only / prose-only / mixed enum + citation | ✓ |
| Source of claims | Extract from existing artefacts / generate from matrix / user authorship | ✓ |

**User's choice:** All 4 areas.
**Notes:** Phase 2 has high information density; each decision shapes the rest of the table's structure.

---

## Sub-question 1: Claim taxonomy

| Option | Description | Selected |
|--------|-------------|----------|
| Welfare + comparative + structural + calibration | All four classes share the CLM namespace; confidence-method column distinguishes them | ✓ |
| Welfare only | Structural / calibration handled in register or CIP body prose | |
| Welfare + comparative only | Middle ground | |

**User's choice:** All four classes.
**Notes:** Captured as CONTEXT.md D-11. Expected ~25–40 CLM entries after pairing with menu options.

## Sub-question 2: Source of claims

| Option | Description | Selected |
|--------|-------------|----------|
| Extract from existing artefacts + user augmentation | Read family-b-* trio + suite READMEs; user seeds extras in `<specifics>` | ✓ |
| Generate from menu-item × property matrix | Systematic grid; risk: artificial BACKED/UNBACKED on non-claims | |
| User authorship | Most accurate; highest user effort | |

**User's choice:** Extract + augment.
**Notes:** Captured as CONTEXT.md D-12. Four source documents named in priority order; CONTEXT.md `<specifics>` seeds the structural/calibration claims that might be missed in welfare-heavy source documents.

## Sub-question 3: Row shape

| Option | Description | Selected |
|--------|-------------|----------|
| Per-(claim, menu-option) pair | Denormalised; ~30–40 rows; greppable; standard RTM | ✓ |
| Per-claim with 5 option columns | Normalised; ~10 rows; easier scan; harder filter | |
| Hybrid (header + sub-rows) | Two-level; markdown rendering complexity | |

**User's choice:** Per-(claim, menu-option) pair.
**Notes:** Captured as CONTEXT.md D-13. Matches Leios `ImpactAnalysis.md` precedent. 5 menu options confirmed: `priority-only-RB-reserved`, `priority-only-un-reserved`, `both-dynamic-partitioned`, `both-dynamic-un-partitioned`, `single-lane-EIP-1559-control` (control only, not a CIP menu item).

## Sub-question 4: Non-welfare cell content

| Option | Description | Selected |
|--------|-------------|----------|
| Mixed enum + citation | Controlled enum vocabulary + citation/quantitative bound per cell | ✓ |
| Pure enum | Easiest authoring; least defensible | |
| Pure prose | Most expressive; hardest to grep | |

**User's choice:** Mixed enum + citation.
**Notes:** Captured as CONTEXT.md D-14. Four controlled vocabularies defined per non-welfare column:
- `anti-bribery` ∈ `{formal, informal, absent}`
- `signal-source-anchoring` ∈ `{mainnet-data-cited, spec-default, unanchored}`
- `standard-user-fee-drift-exposure` ∈ `{none, bounded, exposed}`
- `implementation-complexity` ∈ `{low, medium, high}`

Cell format: `<enum-value> (<citation>)`.

---

## Carried forward from Phase 1 + initialization

| Item | Source | Mapping |
|---|---|---|
| `CLM-NN` ID convention | Phase 1 D-05 + REQ-COV-01 | D-15 |
| Coverage verdict vocabulary (BACKED / WEAK / UNBACKED / OUT-OF-SCOPE) | REQ-COV-02 | D-16 |
| 12 unpinned demand-regime suites → WEAK rows | REQ-COV-04 + init-questioning | D-18 |
| Strict hash-diversity gate | REQ-COV-05 + init-questioning | D-19 |
| Required column set | REQ-COV-02 | D-17 |
| Non-welfare property columns | REQ-COV-03 | D-14 + D-17 |
| Skeleton committable before Phase 3 (UNBACKED rows surface priorities) | REQ-COV-06 | D-20 |
| Abbreviation-on-first-use | CLAUDE.md + Phase 1 carry | D-21 |

---

## Claude's Discretion

Items resolved with named defaults rather than discussed:

- **Column ordering** — suggested left-to-right: `id | claim | menu-option | status | confidence-method | backing-suite | backing-job | seeds-cited | golden-sha256 | <non-welfare property columns> | related-RSK-ids`. Non-welfare columns to the right so casual readers hit status + backing first.
- **Status enum priority on conflicting evidence** — prefer goldens-pinned BACKED-eligible over unpinned-suite WEAK-eligible for the same `(claim, option)` row.
- **UNRESOLVED suites output-read scope** — walk `sim-rs/output/` for the 4 named UNRESOLVED suites; existing data → WEAK rows; missing data → UNBACKED. No re-runs in Phase 2.
- **Multi-RSK rows** — comma-separated list in `related-RSK-ids`; no per-RSK sub-rows.
- **EXP-NN forward references** — included in the `claim` cell parenthetical for UNBACKED rows so Phase 3 planner can map cheap tests to coverage rows.

## Deferred Ideas

- BACKED row population with paired-bootstrap BCa CIs → Phase 3.
- Hash-diversity gate application → Phase 3.
- Anchoring refresh of `signal-source-anchoring` cells → Phase 4 / DOC-03.
- Promotion of unpinned suites to goldens-pinned → out of scope.
- CIP Evidence-section row selection → Phase 5.

No matching todos from `gsd-sdk query todo.match-phase 2` (empty matches set).
