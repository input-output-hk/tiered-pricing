---
phase: 01-register-inventory
verified: 2026-05-15T00:00:00Z
status: passed
score: 5/5 must-haves verified
overrides_applied: 0
---

# Phase 1: Register Inventory Verification Report

**Phase Goal:** A single realism-risks register exists that catalogues every realism risk surfaced by existing artefacts, with stable identifiers, Wohlin-categorised entries, and locked scope-of-resolution fields that discipline downstream cheap-test design.
**Verified:** 2026-05-15
**Status:** PASSED
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `docs/phase-2/realism-risks-register.md` exists with stable `RSK-NN` identifiers that de-duplicate risks across the 6 source documents and 7 spike READMEs | VERIFIED | File exists at 443 lines; 24 RSK-NN entries; evidence-for / evidence-against fields cite all required source documents (cardano-realism-audit.md, validity-threats.md, REVIEW.md, CONCERNS.md, mechanism-welfare-impact-2026-05-14.md, seven spike READMEs) |
| 2 | Every RSK-NN entry has all 9 required fields: id, title, category, description, evidence-for, evidence-against, scope-of-resolution, verdict, disclosure-paragraph | VERIFIED | All 24 entries carry every required field; verdict vocabulary restricted to LIVE / DORMANT / MITIGATED / DISCLOSED (12 LIVE + 12 DISCLOSED; no DORMANT or MITIGATED in v1); no `TBD plan 02` markers remain anywhere in the file |
| 3 | Each LIVE entry is paired with at least one named `EXP-NN` identifier scoped to move the verdict | VERIFIED | All 12 LIVE entries carry at least one EXP-NN slug; slugs cross-reference TEST-NN REQ-IDs where applicable; three new EXP-NN slugs (EXP-unresolved-output-read, EXP-coverage-non-welfare-columns, EXP-hash-diversity-policy-decision) appropriately map to Phase 2 / COV work; EXP-multiplier-floor-16-companion-run surfaces TEST-07a |
| 4 | The four already-named LIVE entries are present: pool-count sensitivity, single-seed precision, un-anchored controller knobs, substrate scope | VERIFIED | RSK-pool-count (LIVE), RSK-single-seed-precision (LIVE), RSK-un-anchored-controller-knobs (LIVE, umbrella with four sub-knobs), RSK-substrate-scope (LIVE) all present with correct identifiers and verdicts |
| 5 | `RSK-pool-count` carries the locked threshold "Δ% < seed-IQR of same job at 100 pools establishes MITIGATED" in its scope-of-resolution field | VERIFIED | Confirmed at line 59: "Δ% < seed-IQR (Inter-Quartile Range) of same job at 100 pools establishes MITIGATED" — verbatim locked text present with IQR expanded on first use per CLAUDE.md convention |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `docs/phase-2/realism-risks-register.md` | Single register file, 20-30 RSK-NN entries, finalised v1 | VERIFIED | 443 lines, 24 entries (within 20-30 target band), no TBD plan 02 markers, Index table consistent with per-entry sections |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| register evidence-for/against fields | `docs/phase-2/cardano-realism-audit.md` | Evidence citations in every DISCLOSED entry and most LIVE entries | WIRED | Multiple entries cite audit's NEEDS-DISCLOSURE sections (fee-as-maxFee, un-anchored knobs, topology/actor model); D-02 honoured |
| register evidence-for/against fields | `docs/phase-2/validity-threats.md` | Per-threat risk descriptions extracted; per-claim trust ratings deferred to Phase 2 | WIRED | D-01 honoured: per-threat content in register; trust-rating content not duplicated |
| register evidence-for/against fields | `.planning/REVIEW.md` | WR-2 cited in RSK-admission-rejection-attribution; WR-7 subsumed in substrate scope; Fix Status table cited | WIRED | D-03 honoured: sourced via evidence, not superseded |
| register EXP-NN fields | REQUIREMENTS.md TEST-NN | EXP-pool-number → TEST-05; EXP-sign-flip-variance → TEST-03; EXP-canonical-variance → TEST-04; EXP-run-length → TEST-06; EXP-multiplier-floor-16-companion-run → TEST-07a | WIRED | All Phase 3 test inputs have stable EXP-NN → TEST-NN cross-references |
| register EXP-NN fields | REQUIREMENTS.md COV-NN | EXP-unresolved-output-read → REQ-COV-06; EXP-coverage-non-welfare-columns → REQ-COV-03; EXP-hash-diversity-policy-decision → REQ-COV-05 | WIRED | Phase 2 inputs correctly mapped |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| REG-01 | 01-01-PLAN.md | Single register de-duplicating all 6 source docs + 7 spike READMEs | SATISFIED | 24 entries cover all sources; evidence-for fields cite all 6 source documents and relevant spike READMEs |
| REG-02 | 01-01-PLAN.md + 01-02-PLAN.md | All 9 required fields populated per entry | SATISFIED | All 24 entries have id, title, category, description, evidence-for, evidence-against, scope-of-resolution, verdict, disclosure-paragraph; no TBD plan 02 markers remain |
| REG-03 | 01-02-PLAN.md | Each LIVE entry paired with at least one EXP-NN | SATISFIED | 12 LIVE entries, each with at least one EXP-NN slug and a cross-reference to TEST-NN or Phase 2/4 work |
| REG-04 | 01-01-PLAN.md + 01-02-PLAN.md | Four named LIVE entries present and retaining LIVE | SATISFIED | RSK-pool-count (LIVE), RSK-single-seed-precision (LIVE), RSK-un-anchored-controller-knobs (LIVE), RSK-substrate-scope (LIVE) all verified |
| REG-05 | 01-02-PLAN.md | Locked scope-of-resolution text on RSK-pool-count | SATISFIED | "Δ% < seed-IQR (Inter-Quartile Range) of same job at 100 pools establishes MITIGATED" present verbatim at line 59 |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `docs/phase-2/realism-risks-register.md` | 286, 302, 316, 380, 394, 410 | "TBD — drafted in Phase 4 if test verdict lands as DISCLOSED" | INFO | These are the D-08-permitted forward-pointer placeholders for LIVE entries whose expected MITIGATED path makes prose pre-drafting premature. Distinct from forbidden "TBD plan 02" markers (zero of those remain). Not a stub — they name the expected MITIGATED path explicitly. |

### Phase-Scope Discipline

The git status at verification time shows only `CLAUDE.md` modified and new files under `.planning/` — no modifications to `docs/phase-2/cardano-realism-audit.md`, `docs/phase-2/validity-threats.md`, `.planning/REVIEW.md`, `.planning/codebase/CONCERNS.md`, or any spike README. Phase 1 was correctly bounded to creating one new file.

### Context Decisions Honoured

| Decision | Status | Evidence |
|----------|--------|----------|
| D-01: validity-threats per-threat content → register; per-claim trust ratings deferred to Phase 2 | HONOURED | Register contains per-threat RSK-NN entries; no per-suite HIGH/MEDIUM/LOW trust ratings duplicated |
| D-02: cardano-realism-audit NEEDS-DISCLOSURE sections → register; audit doc not modified | HONOURED | Multiple entries cite audit NEEDS-DISCLOSURE sections; audit file not modified |
| D-03: REVIEW.md and CONCERNS.md cited via evidence-for/against; not modified | HONOURED | WR-2 cited in RSK-admission-rejection-attribution evidence-for; CONCERNS.md cited in RSK-substrate-scope, RSK-cross-arch-determinism, RSK-admission-rejection-attribution |
| D-04: Spike verdicts cited as evidence, not lifted into register verdicts | HONOURED | All seven spike READMEs cited in evidence-for fields; no VALIDATED / RECOMMENDED / ADOPT / NEEDS-DISCLOSURE leaks into Verdict fields |
| D-05: Leios RSK-NN / EXP-NN convention; append-only | HONOURED | Identifiers follow RSK-{descriptive-slug} / EXP-{descriptive-slug} pattern throughout |
| D-06: Four-value verdict vocabulary only | HONOURED | 12 LIVE + 12 DISCLOSED; zero DORMANT or MITIGATED in v1 |
| D-07: Wohlin four-fold; multi-tagging allowed | HONOURED | Entries carry category fields from {construct, internal, external, conclusion}; multi-tagged entries present (e.g. RSK-single-seed-precision: conclusion, construct) |
| D-08: All 9 required fields populated | HONOURED | Verified across all 24 entries; disclosure-paragraph mandatory for DISCLOSED entries (12 load-bearing paragraphs present), optional placeholder for LIVE entries |
| D-09: Four mandatory LIVE entries retain LIVE | HONOURED | All four retain LIVE verdict |
| D-10: Locked scope-of-resolution text on RSK-pool-count | HONOURED | Verbatim text confirmed at line 59 |

### Claude's Discretion Defaults Applied

| Default | Status | Evidence |
|---------|--------|----------|
| Entry granularity ~20-30 (not ~60) | MET | 24 entries — within the 20-30 target band |
| Substrate-scope is ONE entry with three sub-points | MET | RSK-substrate-scope is a single entry; description names all three sub-points (a) f64 in non-pricing paths, (b) propagation fidelity, (c) utility-maximising actor model |
| Abbreviations expanded on first use | MET | CIP, IQR, PSE, BCa, SODA, CCS, AFT, MEV, eUTxO, SPO, IEEE, RTT, EMA, UTC, NFT, EIP-1559 all expanded on first use in register prose |
| EXP-NN slugs with TEST-NN cross-reference format | MET | Format "EXP-pool-number (→ TEST-05)" used consistently throughout |

### Downstream Usability Assessment

As a Phase 2 executor needing to build the coverage check skeleton:

- The `related-RSK-ids` column is fully poputable: all 24 stable RSK-NN identifiers are established and append-only. RSK-substrate-scope, RSK-fee-as-maxFee-envelope, RSK-un-anchored-controller-knobs are the primary CIP disclosure anchors.
- The substrate-scope disclosure-paragraph at RSK-substrate-scope is load-bearing and CIP-pasteable immediately — it names all three sub-points with (a)/(b)/(c) labels making each individually citable.
- The EXP-NN → TEST-NN cross-reference table gives Phase 3 the test hypotheses with explicit threshold-before-test discipline (RSK-pool-count's locked scope-of-resolution is the canonical example).
- Three Phase 2 work items now have explicit EXP-NN linkage: EXP-unresolved-output-read (four UNRESOLVED suites output-read), EXP-coverage-non-welfare-columns (non-welfare property columns per COV-03), EXP-hash-diversity-policy-decision (COV-05 policy gate).

### Human Verification Required

None. This is a documentation-only phase. All quality checks are programmatically verifiable via the register file content (grepping for required text patterns, field population, abbreviation expansion). No UI, real-time behaviour, external service integration, or code execution is involved.

---

_Verified: 2026-05-15_
_Verifier: Claude (gsd-verifier)_
