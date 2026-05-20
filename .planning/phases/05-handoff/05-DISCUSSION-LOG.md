# Phase 5: Handoff - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-18
**Phase:** 05-handoff
**Areas discussed:** cip-author-summary shape, 6 LIVE register entries, HAND-02 review scope & tooling

---

## Pre-discussion scope-selection

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — discuss 1, 2, 3; capture 4 as Claude's Discretion | HAND-03 tag mechanics and arXiv re-pass deferred with reasonable defaults | ✓ |
| Discuss all four | Active say over tag-message and arXiv pass | |
| Discuss only the LIVE entries (area 2) | Treat 1 and 3 as Claude's Discretion too | |

**User's choice:** Yes — discuss 1, 2, 3; capture 4 as Claude's Discretion (Recommended).
**Notes:** Confirmed before starting per-area sub-questions.

---

## Area 1: cip-author-summary.md shape (HAND-01)

### Q1 — Overall shape

| Option | Description | Selected |
|--------|-------------|----------|
| CIP-section-keyed | Top-level sections match CIP structure: Methodology, Limitations, Evidence | |
| Artefact-keyed | Walk through each of 5 docs in turn naming pasteable items | |
| Hybrid | Paste-target table + per-CIP-section recommendations + pinned references block | ✓ |

**User's choice:** Hybrid (Recommended).
**Notes:** Combines CIP-section discipline of (a) with artefact reading-guide of (b). Approximate authoring effort: 300-450 lines.

### Q2 — Inline vs reference treatment

| Option | Description | Selected |
|--------|-------------|----------|
| Reference-only | Summary names each pasteable item by ID + source-file + line range; CIP author jumps to source | |
| Inline all paste text | Self-contained paste-ready doc (~600-1000 lines); transcription-drift risk | |
| Tiered | Inline load-bearing items; reference-only for long tail; lands ~300-450 lines | ✓ |

**User's choice:** Tiered (Recommended).
**Notes:** Captures highest-value paste targets in one read; leaves long tail as references for author's discretion.

### Q3 — Headline CIP claim provenance

| Option | Description | Selected |
|--------|-------------|----------|
| Phase 5 derives them from Phase 3/4 evidence | Planner reads test results and writes claim list; maps to CLM rows | ✓ |
| User supplies the CIP outline now | User drops headline-claim list into discussion; planner uses verbatim | |
| Derived + flagged for user review | Planner derives; summary flags as "derived; reviewer may edit" | |

**User's choice:** Phase 5 derives them from Phase 3/4 evidence.
**Notes:** Strongest when no CIP outline exists yet. Planner reads Phase 3 evidence summaries + audit's "Recommended disclosure statements" to derive 4-8 headline claims.

---

## Area 2: 6 LIVE register entries

### Q1 — High-level disposition policy

| Option | Description | Selected |
|--------|-------------|----------|
| Resolve them all in Phase 5 | Apply Phase 4 SUMMARY recommendation: 5 prose-promotion flips + 1 policy flip; end state 24 DISCLOSED | ✓ |
| Resolve some; leave others LIVE | Per-entry judgement; CIP carries some LIVE as ongoing-limitation | |
| Leave all 6 LIVE; cip-author-summary calls them out | Phase 5 does no register edits; summary names the 6 LIVE as ongoing limitations | |

**User's choice:** Resolve them all in Phase 5 (Recommended).
**Notes:** End state 24 DISCLOSED + 0 LIVE + 0 MITIGATED + 0 DORMANT. No LIVE survives to CIP.

### Q2 — RSK-hash-diversity-policy strict vs soft

| Option | Description | Selected |
|--------|-------------|----------|
| Strict | Phase 2 D-19 locked rule; BACKED requires distinct sha256 count = seeds-cited | ✓ |
| Soft | Annotate collapsed rows but don't downgrade; weaker epistemic claim | |
| Hybrid (strict for primary, soft for supporting) | Two-tier coverage check; added complexity | |

**User's choice:** Strict (Recommended).
**Notes:** Phase 2 D-19 already locked strict; Phase 3 hash-diversity-gate results.md shows 17/17 BACKED-eligible cells passing under strict.

### Q3 — Verdict bucket for 5 non-policy LIVE entries (MITIGATED vs DISCLOSED)

| Option | Description | Selected |
|--------|-------------|----------|
| All 5 flip to DISCLOSED | Honest default; Phase 3 evidence moves hypothesis to "risk is bounded and disclosed" | ✓ |
| Sign-flip + canonical → MITIGATED; rest DISCLOSED | 2 MITIGATED + 22 DISCLOSED (RSK-single-seed-precision + RSK-three-seed-statistical-power flip) | |
| Per-entry planner judgement | Planner reads each Scope-of-resolution and licenses MITIGATED where evidence clearly warrants | |

**User's choice:** All 5 flip to DISCLOSED (Recommended).
**Notes:** MITIGATED requires evidence to move failure-mode hypothesis to "risk is not real"; the evidence at hand mostly moves it to "risk is bounded and disclosed".

---

## Area 3: HAND-02 review scope & tooling

### Q1 — Review scope

| Option | Description | Selected |
|--------|-------------|----------|
| Five CIP-cited docs only | Same five docs Plan 04-07 audited; Phase 5 re-runs after Phase 5 edits | ✓ |
| Five CIP-cited + cip-author-summary itself | Add the new HAND-01 deliverable; folds in naturally | (implicit; recommended) |
| Expand to all `.planning/` artefacts the CIP might cite | Include spike READMEs, phase SUMMARYs, etc.; roughly doubles review surface | |

**User's choice:** Five CIP-cited docs only (Recommended).
**Notes:** Phase 5's CONTEXT.md folds in the cip-author-summary.md as implicit sixth in-scope document (it's a Phase 5 deliverable that needs its own references resolved).

### Q2 — Tooling: ad-hoc vs reproducible script

| Option | Description | Selected |
|--------|-------------|----------|
| Reproducible script | Small shell script doing the 4 checks; CIP peer reviewers re-run independently | ✓ |
| Ad-hoc grep | Phase 5 plan inlines grep commands; no persistent script | |
| Hybrid | Ad-hoc for iteration; script committed at end of phase | |

**User's choice:** Reproducible script (Recommended).
**Notes:** Strongest long-lived asset; lets future register / coverage-check edits get continuous verification.

### Q3 — Script location

| Option | Description | Selected |
|--------|-------------|----------|
| `.planning/phases/05-handoff/verify-consistency.sh` | Phase-scoped; lives with rest of Phase 5 artefacts | ✓ |
| `sim-rs/scripts/verify-cip-evidence.sh` | Matches existing convention; out of place alongside simulator scripts | |
| New top-level `scripts/verify-cip-evidence.sh` | Repo-root scripts/; over-engineered for one file | |

**User's choice:** `.planning/phases/05-handoff/verify-consistency.sh` (Recommended).
**Notes:** Discoverable via the cip-author-summary's pinned-references block.

---

## Wrap-up

| Option | Description | Selected |
|--------|-------------|----------|
| Proceed to write CONTEXT.md | Decisions captured: hybrid summary; tiered; Phase 5 derives claims; resolve all 6 LIVE; review script | ✓ |
| Add a Limitations paste-order preference | If user has specific paste-order in mind for 18 disclosure-paragraphs | |
| Add a CIP outline pointer | If draft CIP outline exists to reference in canonical_refs | |

**User's choice:** Proceed to write CONTEXT.md (Recommended).
**Notes:** Limitations paste-order is captured as Claude's Discretion with a draft 13-bullet order in CONTEXT.md `<specifics>`; CIP outline is the headline-claim list Phase 5 derives per D-46.

---

## Claude's Discretion (deferred sub-decisions with named defaults)

The following items were named as Claude's Discretion with reasonable defaults pinned in CONTEXT.md:

- **HAND-03 tag specifics** — name `phase-2-cip-evidence-v1`; annotated; ~12-line message; user-executed not Claude-executed (per don't-auto-commit memory).
- **Optional 2024-2026 arXiv re-pass** — deferred by default; Plan 04-01 already exited at "marginal anchor unlikely". User may explicitly request during Phase 5 execution to fold in.
- **TEST-05 / TEST-06 verdict-flip patch** — assume no re-run lands before tag; three RSK entries stay DISCLOSED. If user re-runs land opportunistically, fold in.
- **Limitations paste-order on 18 disclosure-paragraphs** — substrate-scope umbrella leads; category-grouped (external → construct → conclusion → internal); load-bearing within each category. Draft 13-bullet order in CONTEXT.md `<specifics>`.
- **Headline Evidence-section CLM row count** — 4-8 headline claims; ≤ 12 CLM rows inline.
- **CONSISTENCY-REPORT.md verbosity** — ~120-180 lines (4 checks vs Plan 04-07's 8); same table-per-check format.
- **HAND-01 forward-pointer disclosure** — if a LIVE entry genuinely warrants staying LIVE on closer reading, planner surfaces as open-for-user-review in Phase 5 SUMMARY rather than silent flip.

## Deferred Ideas

(See CONTEXT.md `<deferred>` for the full list.)

- Optional 2024-2026 arXiv follow-up pass for Plan 04-01 anchor search (default: skip)
- TEST-05 / TEST-06 verdict-flip patches (default: assume no re-run before tag)
- Adversarial / strategic-bidder modelling (out of scope per PROJECT.md)
- 600-pool / ~3,000-pool topology runs (out of scope; CIP-0164 superseded by TEST-05)
- Cross-architecture continuous integration (CI) verification (out of scope per PROJECT.md)
- CIP draft itself (out of scope; user-authored)
- Cross-ref index automation generalised across phases (post-Phase-5 enhancement)
- Promotion of unpinned demand-regime suites to goldens-pinned (out of scope per COV-04)
- Re-running consistency review on supporting `.planning/` artefacts (out of HAND-02 scope per D-50)
- Promotion of `docs/phase-2/m6-implementation-plan.md` into Phase 5 outputs (contingency only)
