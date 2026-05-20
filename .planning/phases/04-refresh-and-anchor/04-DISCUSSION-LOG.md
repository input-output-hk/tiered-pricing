# Phase 4: Refresh and Anchor - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-18
**Phase:** 04-refresh-and-anchor
**Areas discussed:** TEST-05/06 re-run decision (resolved out-of-band by user), DOC-03 anchor-or-disclose bar, DOC-01 / DOC-02 refresh strategy, DOC-04 ODD methodology overview

---

## Gray-area selection (multiSelect)

| Option | Description | Selected |
|--------|-------------|----------|
| TEST-05/06 re-run decision | Re-run partial pool-number and run-length tests, or accept LIVE→DISCLOSED via fallback disclosure paragraphs | ✓ (resolved by user mid-discussion) |
| DOC-03 anchor-or-disclose bar | What counts as anchored for the four un-anchored controller knobs; literature-search prescription | ✓ |
| DOC-01 / DOC-02 refresh strategy | Full rewrite vs targeted edits; thin pointer vs retained matrix for validity-threats | ✓ |
| DOC-04 ODD methodology overview | Path placement; shape (index table vs prose vs worked example) | ✓ |

**User's choice:** all four areas selected for discussion.
**Notes:** "I'm currently rerunning test-05 and 06 myself in the meantime. They'll be ready upon commencement." → captured as D-34 in CONTEXT.md; resolves the first gray area without further discussion.

---

## DOC-03 anchor-or-disclose bar — anchor strictness

| Option | Description | Selected |
|--------|-------------|----------|
| Numerical match required | Anchor = deployed-system or published numerical value matching within a tight band; most stringent; likely zero of the four knobs anchor under this bar | |
| Range bracket from literature | Anchor = citation motivating choice as inside a literature-defended range; window-length plausibly anchors, multiplier-floors likely don't, lane-signal-source stays disclosed; mid-strictness | |
| Motivating citation suffices | Anchor = published paper motivating the kind of choice without numerical match; most permissive; window-length and both multiplier-floors plausibly anchor; lane-signal-source likely stays disclosed | ✓ |

**User's choice:** Motivating citation suffices.
**Notes:** Implies asymmetric per-knob effort; planner exits literature search early on lane-signal-source (no obvious literature handle for the three-option choice in mechanism-design.md L207-211).

---

## DOC-03 anchor home — where the anchor lands in the docs

| Option | Description | Selected |
|--------|-------------|----------|
| Audit only | (value, source, date-retrieved) triple lands in refreshed audit; register disclosure-paragraph rewritten as 'See audit §...' or verdict flips LIVE→MITIGATED if all four anchor; single source of truth = the audit | |
| Register only | Motivating citation goes into RSK entry's evidence-for field; audit only takes (value, source, date-retrieved) for values with numerical mainnet match | |
| Both, audit citing register | Audit names value + brief citation; register holds full motivating-citation prose; CIP author picks paste target per CIP section | ✓ (Claude's recommendation) |

**User's choice:** "Whatever you recommend" → Claude's recommendation locked in as D-36: Both, audit citing register.
**Notes:** Recommendation rationale: audit is parameter-provenance (brief citation suits its current shape); register's disclosure-paragraph is CIP-pasteable prose (rewritten "Anchored by [...]" prose belongs there since CIP author paste-targets the register for the Limitations section); verdict flips happen on the register.

---

## DOC-03 search budget — how the literature-search budget is enforced

| Option | Description | Selected |
|--------|-------------|----------|
| Two hours aggregate, hard cap | Single two-hour search across all four knobs; whatever surfaces is the anchor set; matches REQUIREMENTS.md framing literally | |
| Two hours per knob (8h total) | Each knob gets its own two-hour search; maximises anchor chance; risks Phase 4 ballooning into a literature-review sub-phase | |
| No budget; search until satisfied | Open-ended; cut when marginal anchor judged unlikely; REQUIREMENTS.md two-hour figure was estimate, not hard cap | ✓ |

**User's choice:** No budget; search until satisfied.
**Notes:** Per D-37, the cut-off decision and consulted-but-rejected citations are documented in the DOC-03 output for reviewer traceability.

---

## DOC-02 shape — thin pointer vs retained matrix

| Option | Description | Selected |
|--------|-------------|----------|
| Thin pointer per D-01 | validity-threats.md becomes ~50-100 lines; framework + redirect; per-suite trust matrix migrates into RSK entries / CLM rows or drops; single source of truth | |
| Retained matrix, refreshed | 19 suite blocks get RSK cross-references added; historical 'Resolved' banners folded inline; verdicts reconciled with register; ~600-800 lines refreshed but not restructured | ✓ |
| Hybrid: pointer + per-suite annex | Pointer at top + compact per-suite annex table at bottom; per-suite Trust prose dies | |

**User's choice:** Retained matrix, refreshed.
**Notes:** Captured as D-40 with explicit note that this **overrides Phase 1 D-01** (which prescribed thin-pointer). Some duplication between register / coverage-check / validity-threats accepted as cost of per-suite reviewability.

---

## DOC-01 shape — full rewrite vs targeted edits

| Option | Description | Selected |
|--------|-------------|----------|
| Full rewrite, single voice | Re-author as single authoritative document; two banners folded into narrative; invalidated sections removed entirely; every calibration value reformatted as (value, source, date-retrieved) triple; cleanest CIP-pasteable artefact; highest authoring effort | ✓ |
| Targeted edits, history preserved | Strip banners, fold corrections inline; rewrite only invalidated sections; lower effort; risks disjointed voice | |
| Hybrid: rewrite invalidated, keep valid | Unaffected sections stay as-is with (value, source, date-retrieved) reformatting; invalidated sections full rewrite; middle-ground effort | |

**User's choice:** Full rewrite, single voice.
**Notes:** Captured as D-38; 410 lines → 300-400 lines authoritative voice.

---

## DOC-01 prose — single source of truth vs dual-purpose

| Option | Description | Selected |
|--------|-------------|----------|
| Single source of truth = register | Audit = calibration-provenance only per Phase 1 D-02; triples + substrate-scope paragraph; all other CIP-pasteable prose lives in register | |
| Dual-purpose audit | Audit retains 'Recommended disclosure statements' section regenerated against Phase 3 evidence; each paragraph duplicates or cross-references a register RSK entry; audit reads as self-contained CIP-pasteable document; two paste targets | ✓ |
| Audit cites register, no prose | Audit retains 'Disclosure summary' section as table (one row per RSK-NN with verdict + one-line summary + link); no CIP-pasteable prose duplicated | |

**User's choice:** Dual-purpose audit.
**Notes:** Captured as D-39 with explicit note that this **partially overrides Phase 1 D-02** (which prescribed calibration-provenance only).

---

## DOC-04 shape — index vs prose vs worked example

| Option | Description | Selected |
|--------|-------------|----------|
| Pure index table | One markdown table: 7 rows × 2-3 columns; ODD element / location / one-line description; maximises 'one-page' adherence; skeleton-only | |
| Index + brief prose per element | Table + 4-6-sentence paragraph per ODD element with inline file-path links; ~200-400 lines | |
| Index + brief prose + worked example | Table + paragraph per element + worked example tracing single (job, seed) through 7 elements end-to-end; doubles as onboarding; ~500+ lines; scope-expands beyond 'one-page' framing | ✓ |

**User's choice:** Index + brief prose + worked example.
**Notes:** Captured as D-42; deliberate scope expansion accepted because the resulting document doubles as onboarding doc for new contributors.

---

## DOC-04 path — docs/ vs .planning/

| Option | Description | Selected |
|--------|-------------|----------|
| docs/phase-2/methodology-overview.md | CIP cites by repo URL → docs/phase-2/ is natural home alongside other CIP-cited artefacts; visible to external readers | ✓ |
| .planning/methodology-overview.md | Internal-process planning directory; CIP can still cite by full repo URL but docs/ vs .planning/ distinction signals internal-only | |
| Both, with .planning/ canonical and docs/ pointer | Canonical in .planning/; docs/ a one-line redirect; most flexible; adds maintenance burden | |

**User's choice:** docs/phase-2/methodology-overview.md.
**Notes:** Captured as D-41.

---

## Claude's Discretion

- **D-36 anchor home** delegated by user with "Whatever you recommend" → Both, audit citing register, locked in with rationale.
- **D-43 plan-wave decomposition** sketched (Wave 1 = literature search + DOC-04 draft + read TEST-05/06 results; Wave 2 = DOC-01 + DOC-02 + register updates; Wave 3 = consistency review) but final wave structure delegated to gsd-planner.
- **DOC-04 worked-example job choice** delegated to planner; default = canonical menu-item job from `phase-2-priority-only-unreserved.yaml` or `phase-2-two-lane-both-dynamic.yaml`, seed=1, matching a Phase 3 TEST-04 cell.
- **Audit narrative ordering (DOC-01)** delegated to planner; default = keep the existing four-section ordering with Phase 3 findings integrated into "What needs disclosure" and "Recommended disclosure statements" item by item.
- **DOC-03 literature-search scope** delegated to planner with minimum reading list (Liu 2022, Reijsbergen 2021/2025, Leonardos 2021) and open-ended exit criterion.
- **Verdict-flip authority** delegated to planner — Phase 4 updates register verdicts only where Phase 3 / Phase 4 evidence licenses the flip; existing DISCLOSED entries stay DISCLOSED unless new evidence surfaces.

## Deferred Ideas

- Re-running TEST-05 / TEST-06 inside Phase 4 (already in flight via user-managed re-run; data available at Phase 4 commencement).
- 600-pool / mainnet ~3,000-pool topology runs (out of scope per PROJECT.md; covered by RSK-pool-count disclosure).
- Cross-architecture CI verification (out of scope per PROJECT.md; covered by RSK-cross-arch-determinism disclosure).
- Adversarial / strategic-bidder modelling (out of scope per PROJECT.md; covered by RSK-substrate-scope disclosure).
- DOC-04 worked example for every menu option (default scope is one).
- Replacing existing CIP-pasteable disclosure paragraphs in the audit with register disclosure-paragraph content verbatim (full deduplication not in Phase 4 scope).
- m6-implementation-plan.md (CIP-0164 600-pool migration; out of scope per PROJECT.md; contingency only).
- CIP author summary (Phase 5 / HAND-01).
- Git tag at milestone close (Phase 5 / HAND-03).

### Reviewed Todos (not folded)
No reviewed-todo deferrals — the `cross_reference_todos` step returned an empty matches set.
