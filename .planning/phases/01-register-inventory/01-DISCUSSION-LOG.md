# Phase 1: Register Inventory - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-15
**Phase:** 1-Register Inventory
**Areas discussed:** Relationship to existing audit-trail documents (only area selected from the initial multi-select)

---

## Gray-area menu — initial selection

| Option | Description | Selected |
|--------|-------------|----------|
| Entry granularity | One RSK-NN per source-doc statement (~40–60 entries) vs clustered into ~15–20 thematic risks | |
| Relationship to existing docs | Register supersedes / complements / hybrid of existing audit-trail documents | ✓ |
| EXP-NN ↔ TEST-NN alignment | EXP-NN matches TEST-NN REQ-IDs vs independent numbering | |
| Substrate-scope grouping | One super-RSK with sub-points / three separate RSKs / hybrid | |

**User's choice:** Relationship to existing docs (single area)
**Notes:** The three unselected areas were resolved under Claude's Discretion at wrap-up with named defaults — see CONTEXT.md `<decisions>` → "Claude's Discretion".

---

## Relationship to existing audit-trail documents

Discussed across four sub-questions, one per source document group.

### Sub-question 1: `validity-threats.md` split

| Option | Description | Selected |
|--------|-------------|----------|
| Split: per-claim trust → coverage check; per-threat risks → register; validity-threats becomes thin pointer doc | Cleanest split; no content lives in two places | ✓ |
| Register supersedes; validity-threats.md is deprecated | All content folds into the register | |
| Complement: validity-threats.md retained as-is, register cross-references it | Lower-effort but content duplication risk | |

**User's choice:** Split (per-claim trust → coverage check; per-threat risks → register; thin pointer doc in Phase 4)
**Notes:** Captured as CONTEXT.md D-01.

### Sub-question 2: `cardano-realism-audit.md` relationship

| Option | Description | Selected |
|--------|-------------|----------|
| Complement: audit stays calibration-provenance focused; risk-shaped sections fold into register | Each doc has one job — audit = parameter provenance, register = risk inventory | ✓ |
| Audit also becomes register-shaped (uniform structure) | Cleaner uniformity but duplicates work | |
| Audit deprecated, all content folds into register | Single artefact, no calibration-only doc | |

**User's choice:** Complement
**Notes:** Captured as CONTEXT.md D-02. NEEDS-DISCLOSURE sections (fee-as-maxFee, 4 un-anchored knobs) pull into the register; audit refreshes (Phase 4 / DOC-01) to (value, source, date-retrieved) triples per knob only.

### Sub-question 3: Internal process docs (`REVIEW.md`, `CONCERNS.md`)

| Option | Description | Selected |
|--------|-------------|----------|
| Register sources from them but doesn't supersede | They retain original working-artefact roles | ✓ |
| Register supersedes both | All risk-shaped items pull into register; process docs become non-CIP-citeable | |

**User's choice:** Register sources from them but doesn't supersede
**Notes:** Captured as CONTEXT.md D-03. Allows CONCERNS.md to remain regeneratable by `/gsd-map-codebase` without disturbing the register; REVIEW.md retains its code-review role with the Fix Status table.

### Sub-question 4: Evidence docs (mechanism-welfare-impact, 7 spike READMEs)

| Option | Description | Selected |
|--------|-------------|----------|
| Treat as evidence sources, not register inputs | Cleanest separation: register = inventory; evidence docs = audit trail | ✓ |
| Spike verdicts become register entries | Each NEEDS-DISCLOSURE verdict becomes its own RSK-NN | |

**User's choice:** Treat as evidence sources, not register inputs
**Notes:** Captured as CONTEXT.md D-04. Spike verdicts preserved in-place; register `evidence-for` / `evidence-against` fields point at them.

---

## Continuation gate

After the four sub-questions on the selected area, the user was asked whether to discuss any of the three remaining gray areas or wrap up.

| Option | Description | Selected |
|--------|-------------|----------|
| Wrap up — write CONTEXT.md | Use Claude's Discretion on the three remaining gray areas with named defaults | ✓ |
| Entry granularity | Discuss the ~20 vs ~60 cluster decision | |
| EXP-NN ↔ TEST-NN alignment | Discuss the naming-scheme alignment | |
| Substrate-scope grouping | Discuss the one-vs-three structure | |

**User's choice:** Wrap up

---

## Claude's Discretion

The user explicitly delegated these to Claude with named default resolutions, captured in CONTEXT.md `<decisions>` → "Claude's Discretion":

- **Entry granularity** — Cluster into ~20–30 thematic risks (not ~60 itemised). Per-source-doc-statement granularity reserved for genuinely-different risks.
- **EXP-NN ↔ TEST-NN alignment** — Descriptive EXP-NN names with TEST-NN cross-reference column for traceability. Example provisional mapping: `EXP-pool-number` → TEST-05; `EXP-sign-flip-variance` → TEST-03; `EXP-canonical-variance` → TEST-04; `EXP-run-length` → TEST-06.
- **Substrate-scope grouping** — One `RSK-substrate-scope` entry whose disclosure-paragraph enumerates the three sub-points (upstream f64, propagation-model fidelity, utility-maximising actors). Rationale: shared mitigation-path (none — out-of-scope per PROJECT.md).
- **Wohlin borderline cases** — Multi-category tagging allowed where a risk genuinely straddles categories.
- **Disclosure-paragraph voice** — Cardano CIP house style (CIP-0164 §"Trade-offs & Limitations" as the closest precedent); engineering-report register, not academic-paper register.
- **Abbreviation-on-first-use** — applied to register prose per the new CLAUDE.md rule.

## Deferred Ideas

None. The user did not raise scope-creep ideas during this discussion. No matching todos from `gsd-sdk query todo.match-phase 1` (empty matches set).
