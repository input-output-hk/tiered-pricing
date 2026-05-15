---
phase: 01-register-inventory
plan: 02
subsystem: documentation
tags: [realism-risks, register, cip, verdicts, disclosure-paragraphs, exp-cross-reference]

# Dependency graph
requires:
  - "docs/phase-2/realism-risks-register.md skeleton (produced by plan 01-01; provides 24 RSK-NN entries with TBD plan 02 markers in judgement fields)"
provides:
  - "docs/phase-2/realism-risks-register.md v1 — final state with verdicts, scope-of-resolution, EXP-NN cross-references, and CIP-pasteable disclosure-paragraphs for every RSK-NN entry"
  - "Final verdict distribution: 12 LIVE + 12 DISCLOSED across 24 entries (no DORMANT — no demand regime de-activates any risk; no MITIGATED — no Phase 3 test results yet exist to license that verdict)"
  - "EXP-NN ↔ TEST-NN cross-reference table realised (CONTEXT.md provisional mapping confirmed; three new EXP-NN slugs surfaced for LIVE entries beyond the original mapping)"
  - "RSK-pool-count's locked scope-of-resolution text 'Δ% < seed-IQR of same job at 100 pools establishes MITIGATED' present verbatim (REG-05)"
  - "Four mandatory LIVE entries from D-09 retain LIVE verdict with populated EXP-NN linkage and scope-of-resolution (REG-04)"
  - "CIP-pasteable disclosure-paragraphs for all 12 DISCLOSED entries (engineering-report voice; abbreviations expanded; no internal jargon)"
  - "RSK-substrate-scope load-bearing disclosure-paragraph naming all three sub-points (floating-point in non-pricing paths; propagation-model fidelity; utility-maximising actor model with Chung & Shi SODA 2023 frame)"
affects:
  - "02-coverage-check (Phase 2): related-RSK-ids column references stable RSK-NN identifiers from this register; the new EXP-unresolved-output-read and EXP-coverage-non-welfare-columns slugs hand off output-read and column-design work to Phase 2"
  - "03-targeted-cheap-tests (Phase 3): the 5 LIVE EXP-NN slugs aligned to TEST-03/04/05/06 hand the test hypotheses with explicit threshold-before-test discipline; EXP-multiplier-floor-16-companion-run maps to a TEST-07a sub-requirement"
  - "04-refresh-and-anchor (Phase 4): DOC-01 folds the substrate-scope disclosure-paragraph into the refreshed audit; DOC-03 resolves the four un-anchored controller knobs via anchor-or-disclose; the fallback disclosure-paragraphs for LIVE entries become Phase 4 reference material if test verdicts land as DISCLOSED"
  - "05-handoff (Phase 5): the load-bearing disclosure-paragraphs (substrate-scope, fee-as-maxFee-envelope, mempool-cap-magnitude, max-fee-policy-default, demand-mix-bit-calibration, demand-non-stationarity, target-inclusion-blocks-default, partition-activated-honest-producer, leios-spec-pre-deployment, cross-arch-determinism, admission-rejection-attribution, welfare-as-f64-reporting, sundaeswap-demand-staleness) paste into the CIP author's Limitations section"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Verdict-vocabulary disjointness preserved (LIVE/DORMANT/MITIGATED/DISCLOSED for register; VALIDATED/NEEDS-DISCLOSURE/RECOMMENDED/ADOPT for spike READMEs); zero leakage between artefacts"
    - "EXP-NN ↔ TEST-NN cross-reference format 'EXP-<slug> (→ TEST-NN)' for Phase 3 traceability; 'EXP-<slug> (Phase 4 / DOC-03 anchor-or-disclose)' for Phase 4 work"
    - "Disclosure-paragraph two-track convention: load-bearing CIP-pasteable prose for DISCLOSED entries; 'TBD — drafted in Phase 4 if test verdict lands as DISCLOSED' fallback for LIVE entries whose expected MITIGATED path makes prose pre-drafting premature"
    - "Threshold-before-test discipline (PITFALLS MOD-5): RSK-pool-count's locked scope-of-resolution text is the canonical example; every LIVE entry's scope-of-resolution names a metric, a threshold, and a range explicitly"

key-files:
  created: []
  modified:
    - "docs/phase-2/realism-risks-register.md"

key-decisions:
  - "Verdict distribution in v1: 12 LIVE + 12 DISCLOSED (no DORMANT, no MITIGATED). Rationale: no Phase 3 test results yet exist to license MITIGATED for any entry; every entry that could go DORMANT (e.g. RSK-demand-non-stationarity) instead carries DISCLOSED because the underlying risk is real in principle and the resolution path is CIP disclosure rather than future-activation monitoring."
  - "RSK-fee-as-maxFee-envelope assigned DISCLOSED (not LIVE). Rationale: the refund-CIP dependency is a hard external dependency that no in-milestone cheap test resolves; the CIP-pasteable disclosure-paragraph naming the prerequisite is the resolution path. Same rationale for RSK-mempool-cap-magnitude, RSK-max-fee-policy-default, RSK-demand-mix-bit-calibration, RSK-target-inclusion-blocks-default, RSK-partition-activated-honest-producer, RSK-leios-spec-pre-deployment, RSK-cross-arch-determinism, RSK-admission-rejection-attribution, RSK-welfare-as-f64-reporting, RSK-sundaeswap-demand-staleness — each has the resolution path 'CIP disclosure of the limitation as scope boundary' rather than 'cheap test moves the verdict'."
  - "RSK-substrate-scope assigned LIVE (not DISCLOSED yet). Rationale per CONTEXT.md / D-09: in v1 this entry stays LIVE because Phase 4 / DOC-01 is what folds the disclosure-paragraph into the refreshed audit and flips the terminal verdict to DISCLOSED. The disclosure-paragraph is fully drafted in v1 (load-bearing for the CIP author) but the entry is LIVE-going-to-DISCLOSED rather than DISCLOSED-on-arrival."
  - "Three new EXP-NN slugs surfaced for LIVE entries beyond the CONTEXT.md provisional mapping: EXP-unresolved-output-read (Phase 2 coverage-check output-read pass; shared by RSK-unresolved-suite-claims and RSK-standard-user-fee-drift-exposure); EXP-coverage-non-welfare-columns (Phase 2 / REQ-COV-03 column-design requirement); EXP-hash-diversity-policy-decision (Phase 3 COV-05 strict-vs-soft policy gate). All three follow the append-only EXP-NN convention; none required a new TEST-NN sub-requirement (they map to Phase 2 / COV-05 work rather than Phase 3 cheap tests)."
  - "EXP-multiplier-floor-16-companion-run mapped to TEST-07a (a sub-requirement of the TEST-07 placeholder for 3-5 additional cheap tests). This is the only LIVE entry that produces a new Phase 3 test load beyond TEST-03/04/05/06; the test is a like-for-like companion run of phase-2-rb-scarcity and phase-2-urgency-inversion at multiplier_floor = 16 instead of 4."
  - "RSK-un-anchored-controller-knobs disclosure-paragraph drafts the per-knob 'conditional on X' frame for all four knobs (window-length 32; multiplier-floor 4; multiplier-floor 16; lane-signal-source). Per the load-bearing umbrella-grouping decision from plan 01, the umbrella verdict cannot land MITIGATED unless every sub-knob lands ANCHORED at Phase 4 / DOC-03; partial anchoring lands DISCLOSED with per-knob granularity in the per-sub-point disclosure prose."

patterns-established:
  - "Disclosure-paragraph voice: third-person engineering-report register; named non-property under a named condition; abbreviations expanded on first use within the paragraph itself (since the CIP author pastes paragraphs in isolation); no internal-process jargon like raw 'WR-1' without context (the underlying issue is described first, with the parenthetical identifier as a pointer)"
  - "Index table extended from 3 columns (RSK / Title / Category) to 5 columns (RSK / Title / Category / Verdict / EXP / Resolution) reflecting final values; no TBD markers anywhere in the table"
  - "Reading-guide note updated from 'inventory skeleton with TBD plan 02 placeholders' to 'v1 register with final verdicts'; the file's status header remains 'Draft (Phase 1)' until the CIP author pulls the cited paragraphs"
  - "File-footer changed from skeleton-marker ('Phase 1 inventory skeleton; Plan 01-02 finalises every TBD plan 02 placeholder') to v1-finalised marker"

requirements-completed: [REG-02, REG-03, REG-04, REG-05]

# Metrics
duration: ~30min
completed: 2026-05-15
---

# Phase 01 Plan 02: Verdict assignment, scope-of-resolution, EXP-NN cross-references, CIP-pasteable disclosure-paragraphs, and consistency verification

**Realism-risks register at `docs/phase-2/realism-risks-register.md` finalised to v1: 24 RSK-NN entries with final verdicts (12 LIVE + 12 DISCLOSED), populated scope-of-resolution hypotheses, EXP-NN slugs cross-referenced to TEST-NN REQ-IDs or Phase 4 / DOC-03 anchor-or-disclose work, and CIP-pasteable disclosure-paragraphs in engineering-report voice. No TBD plan 02 markers remain. The four mandatory LIVE entries from D-09 retain LIVE verdict; the locked threshold text on RSK-pool-count is preserved verbatim; RSK-substrate-scope's load-bearing disclosure-paragraph names all three sub-points (floating-point arithmetic in non-pricing paths; propagation-model fidelity; utility-maximising actor model with Chung & Shi SODA 2023 frame). Index ↔ entry sections consistent (no orphans either direction); all required abbreviations (CIP, IQR, PSE, BCa, SODA, CCS, AFT, MEV, eUTxO, MB, KB, EMA, NFT, UTC, ARM, IEEE, RTT, CI, EIP-1559, EB, RB) expanded on first use.

## Performance

- **Duration:** ~30 min (single executor pass; both tasks completed in one session)
- **Started:** 2026-05-15
- **Completed:** 2026-05-15
- **Tasks:** 2 (Task 1: verdict + scope + EXP-NN per entry; Task 2: disclosure-paragraphs + consistency verification)
- **Files modified:** 1 (docs/phase-2/realism-risks-register.md)

## Accomplishments

- Replaced every `TBD plan 02` marker in `docs/phase-2/realism-risks-register.md` with finalised content. The file went from 93 placeholders to 0.
- Assigned final verdicts to all 24 RSK-NN entries: 12 LIVE, 12 DISCLOSED, 0 DORMANT, 0 MITIGATED. Verdict-vocabulary disjointness from spike vocabulary preserved (no `VALIDATED` / `NEEDS-DISCLOSURE` / `RECOMMENDED` / `ADOPT` leakage).
- The four mandatory LIVE entries from D-09 (`RSK-pool-count`, `RSK-single-seed-precision`, `RSK-un-anchored-controller-knobs`, `RSK-substrate-scope`) retain LIVE verdict with populated scope-of-resolution and EXP-NN linkage.
- `RSK-pool-count` carries the locked threshold text verbatim: "Δ% < seed-IQR (Inter-Quartile Range) of same job at 100 pools establishes MITIGATED" (REG-05).
- `RSK-substrate-scope` load-bearing disclosure-paragraph drafted in CIP house style (~750 words, three labeled sub-points: (a) Floating-point arithmetic in non-pricing code paths; (b) Propagation-model fidelity; (c) Utility-maximising actor model). Pastes verbatim into the CIP's Limitations section once Phase 4 / DOC-01 lands.
- `RSK-un-anchored-controller-knobs` per-knob "conditional on X" disclosure-paragraph drafted for all four sub-knobs (window-length 32, multiplier-floor 4, multiplier-floor 16, lane-signal-source).
- Index table expanded from 3 columns to 5 columns (added Verdict and EXP / Resolution), reflecting final values across all 24 rows.
- Consistency verification pass: Index ↔ entry sections one-to-one (24 sections = 24 index rows); every Verdict value matches between table and per-entry field; every EXP-NN slug follows lowercase-hyphen-separated pattern; mandatory four LIVE entries retain LIVE; locked threshold text present; no TBD plan 02 markers anywhere.

## Task Commits

Each task's changes are staged but not committed per the user's no-auto-commit memory rule. The user will commit themselves. Both tasks' content is in the working-tree state of `docs/phase-2/realism-risks-register.md` (also `.planning/phases/01-register-inventory/01-02-SUMMARY.md`, `.planning/STATE.md`, `.planning/ROADMAP.md`, `.planning/REQUIREMENTS.md` per the standard execute-plan.md post-task updates).

1. **Task 1: Verdict + scope-of-resolution + EXP-NN per entry** — content present in working tree (not committed); recommended commit message: `docs(01-02): assign verdicts, scope-of-resolution, EXP-NN for every RSK-NN entry`
2. **Task 2: CIP-pasteable disclosure-paragraphs + consistency verification** — content present in working tree (not committed); recommended commit message: `docs(01-02): finalise CIP-pasteable disclosure-paragraphs and verify register consistency`

Final-plan commit (SUMMARY.md + STATE.md + ROADMAP.md + REQUIREMENTS.md) is also left for the user to commit.

## Files Created/Modified

- `docs/phase-2/realism-risks-register.md` — modified. Every TBD plan 02 marker replaced with finalised content. Index table extended to 5 columns. All 24 entries carry final verdict, scope-of-resolution, EXP-NN cross-reference, and disclosure-paragraph (load-bearing or fallback). Reading-guide note and file-footer marker updated to reflect v1 state.

## Decisions Made

### Verdict distribution: 12 LIVE + 12 DISCLOSED (no DORMANT, no MITIGATED)

Per D-06: verdicts are chosen from the four-value vocabulary LIVE / DORMANT / MITIGATED / DISCLOSED. Concrete assignments:

- **MITIGATED** is not used in v1 because no Phase 3 test results yet exist to license that verdict. The closest case (`RSK` for WR-1 contamination) is not a register entry on its own — WR-1 was folded into `RSK-substrate-scope`'s `evidence-for` and `evidence-against` rather than re-expressed as a register verdict, because the underlying chain-derived adoption is documented in `.planning/family-b-decision-2026-05-14.md` and `.planning/REVIEW.md` Fix Status table directly.
- **DORMANT** is not used in v1 because every risk surveyed is either (a) actively exercised by at least one demand profile (which makes it LIVE pending a test), or (b) acknowledged as a real principled limitation whose resolution path is CIP disclosure (which makes it DISCLOSED). No entry has the "real-in-principle but no demand regime activates it" shape that DORMANT prescribes.
- **LIVE** (12 entries) covers everything with a Phase 3 cheap-test path or a Phase 2 output-read path that could license MITIGATED at a later phase.
- **DISCLOSED** (12 entries) covers everything whose resolution is CIP-pasteable disclosure rather than a test. Includes structurally-disclose-only items (substrate-scope sub-points; cross-architecture determinism; admission-rejection-attribution; `f64`-reporting precision; sundaeswap historical-spike framing) and mechanism-level CIP dependencies (fee-as-maxFee envelope; max-fee-policy default; mempool-cap magnitude; demand-mix bit-calibration; demand-non-stationarity; target-inclusion-blocks default; partition-activated honest-producer assumption; Leios spec pre-deployment).

### Substrate-scope verdict in v1: LIVE-going-to-DISCLOSED

Per D-09 and CONTEXT.md `<specifics>`: `RSK-substrate-scope` retains `LIVE` in the v1 register because Phase 4 / DOC-01 is what folds the disclosure-paragraph into the refreshed `cardano-realism-audit.md` and flips the terminal verdict to DISCLOSED. The disclosure-paragraph itself is fully drafted in v1 (load-bearing for the CIP author; pastes verbatim into the Limitations section). The two-step LIVE → DISCLOSED transition is intentional and matches the original plan-01 framing.

### Three new EXP-NN slugs surfaced beyond the CONTEXT.md provisional mapping

Plan 01-02's verdict-assignment pass surfaced three LIVE entries whose resolution path was not covered by the CONTEXT.md provisional EXP-NN list:

- `EXP-unresolved-output-read` — Phase 2 coverage-check output-read pass on the four UNRESOLVED suites (`phase-2-moderate-priority-only`, `phase-2-moderate-both-dynamic`, `phase-2-realistic-both-dynamic`, `phase-2-sundaeswap-both-dynamic`). Shared by `RSK-unresolved-suite-claims` and `RSK-standard-user-fee-drift-exposure` (the standard-lane drift entry's resolution overlaps the output-read pass). Maps to Phase 2 / REQ-COV-06, not Phase 3.
- `EXP-coverage-non-welfare-columns` — Phase 2 coverage-check column-design requirement enforcing four non-welfare property columns (anti-bribery, signal-source anchoring, standard-user-fee-drift exposure, implementation complexity). Maps to Phase 2 / REQ-COV-03.
- `EXP-hash-diversity-policy-decision` — Phase 3 COV-05 strict-vs-soft policy gate. Maps to Phase 3 / REQ-COV-05 (existing requirement; this EXP names the policy decision that gates COV-05's application).

All three follow the lowercase-hyphen-separated EXP-`<descriptive-slug>` convention. None require a new TEST-NN sub-requirement.

### EXP-multiplier-floor-16-companion-run → TEST-07a

`RSK-multiplier-floor-4-suite-coverage` requires a Phase 3 cheap test (`multiplier_floor = 16` companion job per the two LOW-trust suites that condition on 4) that does not align to TEST-03/04/05/06. It is mapped to TEST-07a — the first sub-requirement of the TEST-07 placeholder, which REQUIREMENTS.md describes as "3-5 additional cheap tests, scoped from REG-01's LIVE entries". This is the only LIVE entry surfaced by plan 01-02 that adds Phase 3 compute load beyond the originally-named tests.

### Disclosure-paragraph two-track convention

Per D-08, disclosure-paragraph is mandatory for DISCLOSED entries and optional for LIVE entries that may later flip to DISCLOSED. The plan distinguishes two tracks:

1. **Load-bearing disclosure-paragraphs** (DISCLOSED entries + RSK-substrate-scope + RSK-pool-count + RSK-single-seed-precision + RSK-un-anchored-controller-knobs + RSK-standard-user-fee-drift-exposure): fully drafted CIP-pasteable prose in engineering-report voice. The CIP author pastes these verbatim (or with light editing) into the CIP's Limitations / Trade-offs sections.
2. **Fallback / forward-pointer disclosure-paragraphs** (the other LIVE entries: RSK-multiplier-floor-4-suite-coverage, RSK-three-seed-statistical-power, RSK-unresolved-suite-claims, RSK-menu-collapse-to-advocacy, RSK-steady-state-run-length, RSK-hash-diversity-policy): explicit "TBD — drafted in Phase 4 if test verdict lands as DISCLOSED" placeholder explaining why the expected MITIGATED path makes prose pre-drafting premature. These are not `TBD plan 02` markers (which are forbidden) — they are the D-08-permitted "optional placeholder for LIVE entries" form.

## EXP-NN ↔ TEST-NN cross-reference table (realised)

| EXP-NN | LIVE entries citing | TEST-NN / Other | Deviation from CONTEXT.md provisional mapping |
|--------|---------------------|-----------------|-----------------------------------------------|
| EXP-pool-number | RSK-pool-count; RSK-calibration-stale-stake-snapshot (overlap) | TEST-05 | Matches CONTEXT.md; calibration-stale entry surfaces the overlap |
| EXP-sign-flip-variance | RSK-single-seed-precision; RSK-three-seed-statistical-power (overlap) | TEST-03 | Matches CONTEXT.md |
| EXP-canonical-variance | RSK-single-seed-precision; RSK-three-seed-statistical-power | TEST-04 | Matches CONTEXT.md |
| EXP-run-length | RSK-steady-state-run-length | TEST-06 | Matches CONTEXT.md |
| EXP-window-length-anchor | RSK-un-anchored-controller-knobs | Phase 4 / DOC-03 | Matches CONTEXT.md |
| EXP-multiplier-floor-4-anchor | RSK-un-anchored-controller-knobs | Phase 4 / DOC-03 | Matches CONTEXT.md |
| EXP-multiplier-floor-16-anchor | RSK-un-anchored-controller-knobs | Phase 4 / DOC-03 | Matches CONTEXT.md |
| EXP-lane-signal-source-anchor | RSK-un-anchored-controller-knobs | Phase 4 / DOC-03 | Matches CONTEXT.md |
| EXP-multiplier-floor-16-companion-run | RSK-multiplier-floor-4-suite-coverage | TEST-07a (new sub-requirement) | Matches the plan-01 SUMMARY recommendation; surfaces a TEST-07 sub-requirement |
| EXP-unresolved-output-read | RSK-unresolved-suite-claims; RSK-standard-user-fee-drift-exposure | Phase 2 / REQ-COV-06 | New (not in CONTEXT.md); shared across two LIVE entries |
| EXP-coverage-non-welfare-columns | RSK-menu-collapse-to-advocacy | Phase 2 / REQ-COV-03 | New (not in CONTEXT.md); Phase 2 column-design path |
| EXP-hash-diversity-policy-decision | RSK-hash-diversity-policy | Phase 3 / REQ-COV-05 | New (not in CONTEXT.md); policy gate, not a test |

## DISCLOSED entries: load-bearing vs forward-pointer

All 12 DISCLOSED entries received **load-bearing** CIP-pasteable disclosure-paragraphs (paste-into-Limitations-section ready, abbreviations expanded, engineering-report voice):

1. `RSK-fee-as-maxFee-envelope` — refund-CIP dependency framing
2. `RSK-mempool-cap-magnitude` — 133× rule-conserving magnitude framing
3. `RSK-max-fee-policy-default` — wallet-headroom forecast framing
4. `RSK-demand-mix-bit-calibration` — stylised-demand-mix framing
5. `RSK-demand-non-stationarity` — stationary-Poisson-with-phased-shifts framing
6. `RSK-target-inclusion-blocks-default` — early-run-only seed framing
7. `RSK-partition-activated-honest-producer` — honest-producer assumption + body-derivable follow-on
8. `RSK-leios-spec-pre-deployment` — Leios-substrate maturation as underlying anchor
9. `RSK-cross-arch-determinism` — intra-architectural qualification on all reproducibility claims
10. `RSK-admission-rejection-attribution` — aggregated-rejection framing pending WR-2 follow-on
11. `RSK-welfare-as-f64-reporting` — three-significant-figures interpretation caveat
12. `RSK-sundaeswap-demand-staleness` — historical-retail-spike-robustness framing

Four LIVE entries received **draft fallback** disclosure-paragraphs (load-bearing only if the expected MITIGATED path fails):

- `RSK-pool-count` — fallback for the case TEST-05 lands at LIVE or DISCLOSED
- `RSK-single-seed-precision` — fallback for cells whose multi-seed variance bands do not resolve via Paired Seed Evaluation
- `RSK-un-anchored-controller-knobs` — per-knob fallback for sub-knobs that find no external anchor at Phase 4 / DOC-03 open
- `RSK-calibration-stale-stake-snapshot` — fallback only if TEST-05 surfaces sensitivity within 100-150 pool range
- `RSK-standard-user-fee-drift-exposure` — fallback only if Phase 2 output-read surfaces unbounded or spike-amplified drift

`RSK-substrate-scope` is a special case: LIVE in v1, load-bearing disclosure-paragraph fully drafted, expected to flip to DISCLOSED at Phase 4 / DOC-01.

Six LIVE entries received **forward-pointer / TBD-Phase-4** disclosure-paragraphs (placeholder explaining the expected MITIGATED path):

- `RSK-multiplier-floor-4-suite-coverage` — expects MITIGATED via TEST-07a
- `RSK-three-seed-statistical-power` — expects MITIGATED via TEST-04
- `RSK-unresolved-suite-claims` — expects MITIGATED via Phase 2 output-read
- `RSK-menu-collapse-to-advocacy` — expects MITIGATED via Phase 2 / REQ-COV-03 column-design
- `RSK-steady-state-run-length` — expects MITIGATED via TEST-06
- `RSK-hash-diversity-policy` — expects MITIGATED via Phase 3 COV-05 policy decision

## Additional TEST-NN sub-requirements surfaced

Per the plan's `<output>` requirement: which RSK-NN entries surfaced needs for additional TEST-NN sub-requirements beyond TEST-07 (Phase 3's placeholder)?

- **TEST-07a** (new): `EXP-multiplier-floor-16-companion-run` — companion `multiplier_floor = 16` jobs for `phase-2-rb-scarcity` and `phase-2-urgency-inversion` (two jobs total). The only LIVE entry whose resolution adds Phase 3 compute load beyond TEST-03/04/05/06.

No other TEST-07x sub-requirements surfaced. The three new EXP-NN slugs (`EXP-unresolved-output-read`, `EXP-coverage-non-welfare-columns`, `EXP-hash-diversity-policy-decision`) map to Phase 2 work (COV-03 / COV-06 / COV-05) rather than Phase 3 cheap tests.

## Deviations from Plan

None — plan executed as written. The plan permitted plan 01-02 to surface additional EXP-NN slugs for LIVE entries beyond the CONTEXT.md provisional mapping, and three were surfaced (`EXP-unresolved-output-read`, `EXP-coverage-non-welfare-columns`, `EXP-hash-diversity-policy-decision`); these map to Phase 2 / COV-05 work rather than introducing new Phase 3 sub-requirements, which keeps the Phase 3 task ordering unchanged. The plan also permitted TEST-07x sub-requirements to be surfaced; one was (`TEST-07a` = `EXP-multiplier-floor-16-companion-run`), which is recorded above for the Phase 3 planner.

## Issues Encountered

None.

The user's project memory prohibits auto-committing; this executor honoured that rule and left all changes staged-and-unstaged in the working tree for the user to commit themselves. STATE.md, ROADMAP.md, REQUIREMENTS.md are updated per the standard execute-plan.md protocol but not committed; the user may commit them along with the register's changes in a single commit (or split between Task 1 / Task 2 / metadata as the plan recommends).

## Verification

All plan acceptance checks pass:

```
grep -c "TBD plan 02" docs/phase-2/realism-risks-register.md → 0
grep -E "^\*\*Verdict:\*\*" docs/phase-2/realism-risks-register.md | sort | uniq -c → 12 DISCLOSED + 12 LIVE
grep "Δ% < seed-IQR" docs/phase-2/realism-risks-register.md → present (REG-05 locked text)
grep -q "Cardano Improvement Proposal (CIP)" → OK
grep -q "Inter-Quartile Range (IQR)" → OK
grep -q "Paired Seed Evaluation (PSE)" → OK
grep -q "Bias-corrected and accelerated (BCa) bootstrap" → OK
grep -q "Symposium on Discrete Algorithms (SODA)" → OK
grep -q "Conference on Computer and Communications Security (CCS)" → OK
grep -q "Advances in Financial Technologies (AFT)" → OK
Index ↔ entry sections: 24 unique RSK-NN identifiers in each, identical sets → consistent
Four mandatory LIVE entries: all carry Verdict: LIVE → OK
RSK-substrate-scope disclosure-paragraph mentions all three sub-points (f64/floating-point; propagation; actor/utility-maximising/strategic) → OK
```

## Known Stubs

None. The register file's `Disclosure-paragraph` placeholders for the six LIVE entries that defer disclosure-drafting to Phase 4 are NOT stubs — they are the D-08-permitted "optional placeholder for LIVE entries" form, written as explicit "TBD — drafted in Phase 4 if test verdict lands as DISCLOSED" with the expected MITIGATED path named. These are intentional hand-off markers between Phase 1 (this milestone) and Phase 4 (refresh-and-anchor), not data-not-wired-up stubs.

## Next Phase Readiness

- **Phase 2 (coverage check) is unblocked**: the register's 24 RSK-NN identifiers are stable and the `related-RSK-ids` column on `CLM-NN` rows can reference them directly. Three Phase 2 work items now have explicit EXP-NN linkage from the register: (a) the output-read pass on the four UNRESOLVED suites (`EXP-unresolved-output-read`); (b) the non-welfare property columns design (`EXP-coverage-non-welfare-columns`); (c) the COV-05 hash-diversity policy decision (`EXP-hash-diversity-policy-decision`).
- **Phase 3 (cheap tests) task ordering is value-driven**: the 12 LIVE entries' scope-of-resolution fields name the test hypotheses with explicit threshold-before-test discipline. TEST-03/04/05/06 each have a named hypothesis; TEST-07a (`EXP-multiplier-floor-16-companion-run`) is a new sub-requirement; the Phase 4 / DOC-03 anchor-or-disclose work covers the four un-anchored controller knobs.
- **Phase 4 (refresh and anchor) inputs**: DOC-01 folds the load-bearing substrate-scope disclosure-paragraph into the refreshed `cardano-realism-audit.md`; DOC-03 resolves the four un-anchored knobs via anchor-or-disclose; the LIVE-entry fallback disclosure-paragraphs become reference material if test verdicts land as DISCLOSED.
- **Phase 5 (handoff) inputs**: the 12 load-bearing disclosure-paragraphs (DISCLOSED entries + substrate-scope) are the CIP author's paste-into-Limitations material; the 5 fallback paragraphs are the CIP author's paste-if-needed reference.

## Self-Check: PASSED

Modified file exists and contains all required content:

- `docs/phase-2/realism-risks-register.md` — FOUND (443 lines; 24 RSK-NN sections; Index ↔ entries consistent)
- Locked threshold text `Δ% < seed-IQR` — FOUND in RSK-pool-count
- All four mandatory LIVE entries retain LIVE verdict — VERIFIED
- All 12 DISCLOSED entries have populated load-bearing disclosure-paragraphs — VERIFIED
- All required abbreviations expanded on first use — VERIFIED (CIP, IQR, PSE, BCa, SODA, CCS, AFT, and others)
- Zero `TBD plan 02` markers — VERIFIED

Commits not yet made per the user's no-auto-commit memory rule. The user will commit the working-tree changes themselves.

---

*Phase: 01-register-inventory*
*Plan: 02*
*Completed: 2026-05-15*
