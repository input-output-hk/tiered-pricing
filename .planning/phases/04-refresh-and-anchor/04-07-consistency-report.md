# Plan 04-07 — Consistency Report

**Date:** 2026-05-18
**Scope:** Final consistency review across the four refreshed Phase 4 documents (`docs/phase-2/cardano-realism-audit.md`, `docs/phase-2/validity-threats.md`, `docs/phase-2/realism-risks-register.md`, `docs/phase-2/coverage-check.md`) plus the new methodology-overview (`docs/phase-2/methodology-overview.md`).
**Reviewer:** Plan 04-07 Wave 3 (gsd-executor agent; no parallel siblings).
**Outcome:** PASS — all defects found are fixed in place; the post-Phase-4 register verdict distribution is **6 LIVE + 18 DISCLOSED + 0 MITIGATED + 0 DORMANT**.

This document is the Plan 04-07 audit log. The Phase 4 SUMMARY at `.planning/phases/04-refresh-and-anchor/04-SUMMARY.md` consumes this report's per-audit pass/fail verdicts. The §"Defects fixed in place" subsection below lists every defect found and the in-place fix applied.

## RSK-NN cross-reference integrity audit

**Method:** Extract every `RSK-NN` identifier from `realism-risks-register.md` (the canonical set) and verify every occurrence in the four other documents (`cardano-realism-audit.md`, `validity-threats.md`, `coverage-check.md`, `methodology-overview.md`) resolves to a canonical identifier.

**Canonical set:** 24 RSK-NN entries in the register (`RSK-admission-rejection-attribution`, `RSK-calibration-stale-stake-snapshot`, `RSK-cross-arch-determinism`, `RSK-demand-mix-bit-calibration`, `RSK-demand-non-stationarity`, `RSK-fee-as-maxFee-envelope`, `RSK-hash-diversity-policy`, `RSK-leios-spec-pre-deployment`, `RSK-max-fee-policy-default`, `RSK-mempool-cap-magnitude`, `RSK-menu-collapse-to-advocacy`, `RSK-multiplier-floor-4-suite-coverage`, `RSK-partition-activated-honest-producer`, `RSK-pool-count`, `RSK-single-seed-precision`, `RSK-standard-user-fee-drift-exposure`, `RSK-steady-state-run-length`, `RSK-substrate-scope`, `RSK-sundaeswap-demand-staleness`, `RSK-target-inclusion-blocks-default`, `RSK-three-seed-statistical-power`, `RSK-un-anchored-controller-knobs`, `RSK-unresolved-suite-claims`, `RSK-welfare-as-f64-reporting`). The metasyntax token `RSK-NN` appears in headers / column-legends / explainer prose and is intentionally not a real identifier.

**Results:**

| Document | Total unique RSK refs | Dead refs | Notes |
|---|---|---|---|
| `cardano-realism-audit.md` | 15 + `RSK-NN` | 0 | All resolve to register |
| `validity-threats.md` | 19 + `RSK-NN` | 0 (after one cosmetic fix) | One identifier was broken across a line break (`RSK-standard-user-fee-drift-\nexposure` at lines 575–576); identifier itself is canonical — fixed in place to keep identifier on a single line |
| `coverage-check.md` | 14 + `RSK-NN` + `RSK-ids` (column-name metasyntax) | 0 | All resolve to register |
| `methodology-overview.md` | 9 + `RSK-NN` (after one fix) | 0 (after fix) | **Defect found:** `RSK-mev-strategic-bidder` cited on line 141 does not exist in the register; **fixed in place** by replacing the citation with `RSK-substrate-scope` (the umbrella DISCLOSED entry that covers MEV / strategic-bidder risk per its disclosure-paragraph (c) and PROJECT.md Out-of-Scope items 2 + 3) |

**Verdict:** PASS — every RSK-NN reference in the four documents resolves to a canonical register entry after the in-place fixes.

## CLM-NN cross-reference integrity audit

**Method:** Extract every `CLM-NN` identifier from `coverage-check.md` (the canonical set) and verify every occurrence in `validity-threats.md` and `methodology-overview.md` resolves to a canonical row.

**Canonical set:** 55 CLM-NN rows in `coverage-check.md`: CLM-01 through CLM-55.

**Results:**

| Document | Total CLM refs | Dead refs |
|---|---|---|
| `validity-threats.md` | 50 unique CLM identifiers in the range CLM-01..CLM-55 | 0 |
| `methodology-overview.md` | 0 CLM identifiers (the document refers to `coverage-check.md` by document name only, no inline CLM-NN citations) | 0 |

**Verdict:** PASS — every `CLM-NN` reference resolves to a canonical row.

## EXP-NN cross-reference integrity audit

**Method:** Extract every `EXP-NN` identifier from the register (the canonical source per the append-only convention) and verify every occurrence in the other documents resolves.

**Canonical set:** 12 EXP-NN identifiers in the register: `EXP-canonical-variance`, `EXP-coverage-non-welfare-columns`, `EXP-hash-diversity-policy-decision`, `EXP-lane-signal-source-anchor`, `EXP-multiplier-floor-16-anchor`, `EXP-multiplier-floor-16-companion-run`, `EXP-multiplier-floor-4-anchor`, `EXP-pool-number`, `EXP-run-length`, `EXP-sign-flip-variance`, `EXP-unresolved-output-read`, `EXP-window-length-anchor`. Plus metasyntax token `EXP-NN`.

**Results:**

| Document | EXP refs | Dead refs |
|---|---|---|
| `cardano-realism-audit.md` | 0 EXP-NN identifiers (the audit refers to TEST-NN identifiers from `.planning/realism-tests/*` instead) | 0 |
| `validity-threats.md` | 0 EXP-NN identifiers (same pattern as audit) | 0 |
| `coverage-check.md` | 3 EXP-NN identifiers (`EXP-canonical-variance`, `EXP-sign-flip-variance`, `EXP-unresolved-output-read`) + metasyntax `EXP-NN` and `EXP-slug` | 0 |
| `methodology-overview.md` | 0 EXP-NN identifiers | 0 |

**Verdict:** PASS — every `EXP-NN` reference resolves.

## Register verdict ↔ Index table consistency audit

**Method:** For every entry under `## RSK-NN: ...` headings in `realism-risks-register.md`, extract the `**Verdict:**` line. For every row in the §"Index" table, extract the Verdict cell. Verify the two maps agree for all 24 entries.

**Result (post-Plan-04-07 fix):**

- Per-entry `**Verdict:**` line: 18 DISCLOSED + 6 LIVE
- §"Index" table verdict cell: 18 DISCLOSED + 6 LIVE
- Per-identifier reconciliation: all 24 entries' Index-table verdict matches per-entry Verdict line.

**Defect found and fixed:** Pre-Plan-04-07, the register read **7 LIVE + 17 DISCLOSED** (per Plan 04-06's reported distribution). The Plan 04-06 SUMMARY flagged that `RSK-substrate-scope` was expected to flip from LIVE to DISCLOSED via Plan 04-04's audit-side substrate-scope umbrella fold but that Plan 04-04 had not updated the register. Verified: Plan 04-04 wrote the audit-side fold (the substrate-scope umbrella paragraph at lines 335–349 of `cardano-realism-audit.md`) but did not update either the register's Index table or the per-entry Verdict line for `RSK-substrate-scope`.

**Fix applied by Plan 04-07:** Flipped `RSK-substrate-scope` from LIVE to DISCLOSED in both:

- Index table row (line 24 of `realism-risks-register.md`): Verdict cell `LIVE` → `DISCLOSED`; EXP / Resolution cell updated to read "Disclosed (no EXP — substrate out-of-scope per PROJECT.md); disclosure-paragraph load-bearing as folded into `docs/phase-2/cardano-realism-audit.md` §"Topology and actor model" substrate-scope umbrella per Plan 04-04"
- Per-entry `**Verdict:**` line (line 117): `LIVE` → `DISCLOSED`
- Per-entry Scope-of-resolution prose: removed the "v1 verdict is LIVE because Phase 4 / DOC-01 is what folds the disclosure-paragraph..." conditional and replaced with declarative "Phase 4 / DOC-01 (Plan 04-04) folded the substrate-scope umbrella paragraph into `docs/phase-2/cardano-realism-audit.md` §"Topology and actor model"; Plan 04-07 flipped the register-side verdict from LIVE to DISCLOSED to reconcile audit-side and register-side state"
- Per-entry EXP / Resolution prose: removed "drafted below for Phase 4 / DOC-01 flip to DISCLOSED" and replaced with "load-bearing as folded into `docs/phase-2/cardano-realism-audit.md` §"Topology and actor model" substrate-scope umbrella per Plan 04-04; register-side verdict flipped to DISCLOSED by Plan 04-07's consistency review"
- Per-entry Disclosure-paragraph: removed the italic `*(load-bearing; pastes verbatim into the Cardano Improvement Proposal (CIP)'s Limitations section once Phase 4 / DOC-01 lands)*` conditional marker since the fold landed in Plan 04-04
- §"Reading guide" prose: replaced the "Post-Plan-04-06 verdict distribution: 7 LIVE + 17 DISCLOSED ... `RSK-substrate-scope` is flagged in the v1 entry as LIVE → DISCLOSED at Phase 4 / DOC-01" narrative with the post-Plan-04-07 distribution narrative
- Footer prose: updated the "Register through Phase 4 Wave 2 (Plan 04-06)" framing to "Register through Phase 4 Wave 3 (Plan 04-07)" with the six-flip evolution narrative ending at 6 LIVE + 18 DISCLOSED

**Verdict:** PASS — register's Index table and per-entry Verdict fields agree for all 24 entries at the post-Plan-04-07 distribution **6 LIVE + 18 DISCLOSED + 0 MITIGATED + 0 DORMANT**.

## Register ↔ validity-threats verdict reconciliation audit

**Method:** For each per-suite block in `validity-threats.md` with a `Related RSK:` field, verify the suite's Trust verdict (HIGH / MEDIUM / LOW / UNRESOLVED) is qualitatively consistent with the cited RSK entries' register verdicts.

**Results:** 19 per-suite blocks examined. No inconsistency surfaces:

- The 4 LOW-rated suites (`phase-2-priority-only-rb-reserved` CLM-06 claim shape; `phase-2-two-lane-both-dynamic` CLM-08 claim shape; `phase-2-rb-scarcity`; `phase-2-urgency-inversion`) all cite `RSK-multiplier-floor-4-suite-coverage` (DISCLOSED) and `RSK-un-anchored-controller-knobs` (DISCLOSED). Trust LOW is correct: the LOW verdict is conditioned on the Phase 3 N=20 BCa CI REFUTATION of pre-Phase-3 framing (CLM-06 / CLM-08) or on the TEST-07a regime-dependence finding (`rb-scarcity` / `urgency-inversion`), not on a register entry being LIVE-and-unmitigated.
- The 2 HIGH-rated suites (`phase-2-priority-only-unreserved` CLM-07 claim shape; `phase-2-two-lane-both-dynamic` CLM-09 un-partitioned claim shape) cite `RSK-un-anchored-controller-knobs` (DISCLOSED) as their load-bearing RSK; Trust HIGH is licensed by the Phase 3 N=20 BCa CI BACKED finding at `sundaeswap_moderate × floor=4`. No conflict with a register LIVE entry.
- The 13 MEDIUM-rated suites mostly cite `RSK-substrate-scope` (DISCLOSED post-Plan-04-07), `RSK-three-seed-statistical-power` (LIVE), `RSK-un-anchored-controller-knobs` (DISCLOSED), and various demand-specific RSKs (`RSK-demand-mix-bit-calibration`, `RSK-sundaeswap-demand-staleness`, `RSK-leios-spec-pre-deployment`, all DISCLOSED). The LIVE `RSK-three-seed-statistical-power` citation is consistent with MEDIUM Trust because the validity-threats audit's MEDIUM definition explicitly accommodates 3-seed evidence for shape-claims; magnitude-claims are the load-bearing distinction Phase 3 promoted at N=20.

**Verdict:** PASS — no per-suite Trust verdict carries an unexplained LIVE-RSK dependency. No Trust-verdict edits made by Plan 04-07 (Trust-verdict revision authority belongs to Plan 04-05 per CONTEXT.md).

## Audit ↔ register TEST-07a regime-dependence narration consistency

**Method:** Compare the audit's §"Pricing-controller calibration" disclosure-item 2 narration of TEST-07a regime-dependence against the register's `RSK-multiplier-floor-4-suite-coverage` disclosure-paragraph.

**Results:**

| Claim | Audit (line 212–230, 388–408) | Register `RSK-multiplier-floor-4-suite-coverage` (Scope-of-resolution + Disclosure-paragraph) |
|---|---|---|
| At floor=16, rb-scarcity inverts | "the rb-scarcity finding inverts ('standard dominates welfare' → 'priority captures everything; total welfare collapses 93–98%')" | "the rb-scarcity finding inverts ('standard dominates welfare; ranking-block (RB) scarcity mostly invisible' at floor = 4 → 'priority captures everything; total welfare collapses 93-98%' at floor = 16)" |
| At floor=16, urgency-inversion weakly reverses | "the urgency-inversion finding weakly reverses ('mispriced > correctly priced' → 'correctly priced > mispriced by ~13%')" | "the urgency-inversion finding weakly reverses ('mispriced > correctly priced' at floor = 4 → 'correctly priced > mispriced by ~13%' at floor = 16)" |
| Five-of-seven suites cover spec default 16 | "5 of 7 suites independently cover the spec default 16, and the floor-16 regime-dependence is itself disclosed" (line 228) | "The remaining five goldens-pinned suites cover the spec default 16 (priority-only suites sweep {4, 8, 16}; the both-dynamic suite sweeps {4, 16}), so the phase-2 design as a whole brackets the spec default" |
| Conditional-on-X framing | "Welfare findings from these two suites are conditional on the multiplier-floor = 4 calibration" (line 220) | "Welfare findings from `phase-2-rb-scarcity` and `phase-2-urgency-inversion` are conditional on the `multiplier_floor = 4` calibration; the menu-item welfare distinction these suites establish is regime-specific, not universal" |

**Consistency:** Both documents tell the same story with the same numerical anchors (93–98% collapse; ~13% reversal). The register's disclosure-paragraph is longer because it additionally narrates the TEST-07a cross-cell SHA-256 hash identity finding (`rb_scarcity_x16_baseline` and `urgency_inversion_x16_correctly_priced` produce identical pricing event stream Secure Hash Algorithm 256-bit (SHA-256) hashes at seeds 1 and 2); the audit elides this finer detail because it is one mechanism level removed from the welfare narrative. This is the dual-purpose document pattern from CONTEXT.md D-39 working as designed — audit prose is engineering-report-voice summary; register prose is canonical CIP-paste source.

**Verdict:** PASS — no narration divergence between audit and register. No edits required.

## Abbreviation-on-first-use audit

**Method:** Per CLAUDE.md §"Conventions / gotchas", every abbreviation introduced via parentheses must be the first use of that abbreviation in the document. Three forms are acceptable: (a) consolidated header abbreviation expansion block; (b) inline expansion at first use; (c) per-section reintroduction in §"Worked example" — a deliberate choice in `methodology-overview.md` per CONTEXT.md D-42 to make the worked example readable as a standalone learning artefact for new contributors.

**Results:**

| Document | Header expansion block? | Abbreviations introduced | Defects found | Defects fixed |
|---|---|---|---|---|
| `cardano-realism-audit.md` | Yes (16 abbreviations on lines 9–17) plus inline NFT and DeFi expansions in body | 18 | 0 | — |
| `validity-threats.md` | Yes (lines 12–19; 10 abbreviations) plus inline expansions in body | 10+ | 0 | — |
| `realism-risks-register.md` | **Missing** (pre-Plan-04-07): the Index table used RB / MB / AFT / CCS / EMA / NFT before body-section expansions | 24 | 1 (Index-table-before-expansion violation for RB / CCS / AFT / EMA / NFT / MB) | **Added** consolidated header expansion block immediately after the Verdict-vocabulary line listing 24 abbreviations (CIP, RB, EB, SPO, CCS, AFT, SODA, BCa bootstrap, IQR, PSE, EMA, NFT, UTC, MB, KB, DeFi, MEV, IEEE, ARM, YAML, eUTxO, RTT, SHA-256, u128, EIP-1559) |
| `coverage-check.md` | **Partial** (pre-Plan-04-07): a §"Notation conventions" expansion block lived on line 45 but the column-legend on lines 18–34 used CIP / EIP-1559 / RB / EB / LoC before line 45 | 12 | 1 (column-legend-before-expansion violation) | **Added** consolidated header `**Abbreviations on first use**` bullet immediately after the Verdict-vocabulary line listing 16 abbreviations (CIP, EIP-1559, RB, EB, LoC, BCa bootstrap, PSE, IQR, CV, SODA, CCS, AFT, RSK, CLM, EXP, SHA-256); the §"Notation conventions" bullet on line 45 is retained as informational redundancy |
| `methodology-overview.md` | Yes (line 8; 16 abbreviations) plus inline reintroductions in §"Worked example" per CONTEXT.md D-42 | 16+ | 1 (Status line 3 used "CIP" before line 4's "Cardano Improvement Proposal (CIP)" expansion) | **Fixed** in place by expanding CIP on line 3 ("Phase-2 Cardano Improvement Proposal (CIP) evidence base methodology index") and shortening line 4 to use the now-introduced abbreviation |

**Verdict:** PASS — after the three fixes (register header expansion block; coverage-check header expansion bullet; methodology-overview Status-line CIP expansion), every abbreviation introduced via parentheses is the first standalone use of that abbreviation in its document (modulo the methodology-overview §"Worked example" deliberate re-expansion per CONTEXT.md D-42).

## (value, source, date-retrieved YYYY-MM-DD) triple-format conformance audit

**Method:** Audit `docs/phase-2/cardano-realism-audit.md` for every calibration-value expression. The canonical format per ROADMAP.md success criterion #1 and CONTEXT.md D-38 is `(<value>, source: <citation>, date-retrieved: YYYY-MM-DD)`.

**Audit pattern:** every parenthetical containing `source:` must also carry `date-retrieved:` followed by either a `YYYY-MM-DD` ISO-8601 date or `—` (em-dash, used for un-anchored values per the Plan 04-01 disclose-frame).

**Results:**

| Metric | Count |
|---|---|
| Total parentheticals matching `source:[\s\S]*?date-retrieved:` | 17 |
| Triples with YYYY-MM-DD date-retrieved | 14 |
| Triples with em-dash date-retrieved (un-anchored / no external citation) | 3 |
| Parentheticals with `source:` but missing `date-retrieved:` | 0 |
| Malformed triples (missing format compliance) | 0 |

**Date distribution among the 14 YYYY-MM-DD triples:** 2026-05-14 (mainnet calibration retrieval date) × 11; 2026-05-13 (literature retrieval date for EIP-1559 academic-critique tradition + CIP-0164 Table 7 + demand-mix retrieval) × 3.

**Verdict:** PASS — all 17 triples conform; the audit exceeds the Plan 04-04 SUMMARY's reported 12 triples (the additional 5 are em-dash-dated triples in §"Pricing-controller calibration" for the three DISCLOSED sub-knobs + two un-anchored target_inclusion_blocks / topology items, all valid per Plan 04-01's disclose-frame convention).

## Markdown link resolution audit

**Method:** Extract every markdown link of the form `[text](path)` from each of the five documents and verify each `path` resolves on disk (after stripping any `#anchor` suffix). Also audit backtick-wrapped path references (`` `path/file.ext` ``) in `realism-risks-register.md` and `coverage-check.md` (these documents use backtick paths in the body and the Index / coverage tables).

**Results:**

| Document | Markdown links (relative) | Broken | Backtick paths | Broken |
|---|---|---|---|---|
| `cardano-realism-audit.md` | 13 (10 unique) | 0 | n/a | n/a |
| `validity-threats.md` | 8 (5 unique) | 0 | n/a | n/a |
| `realism-risks-register.md` | 0 (the register uses backtick paths exclusively) | 0 | 20 unique | 0 |
| `coverage-check.md` | 0 (backtick paths only) | 0 | 11 unique | 0 |
| `methodology-overview.md` | 91 (38 unique) | 1 → 0 | n/a | n/a |

**Defect found:** `methodology-overview.md` cited `../../sim-rs/sim-cli/parameters/config.default.yaml` as the base configuration file path; the file actually lives at `sim-rs/parameters/config.default.yaml` (the `sim-cli/parameters/` directory does not exist).

**Fix applied:** Updated the link target from `../../sim-rs/sim-cli/parameters/config.default.yaml` to `../../sim-rs/parameters/config.default.yaml`.

**Verdict:** PASS — every relative markdown link and every backtick-wrapped path reference resolves on disk after the in-place fix.

## Audit document constraints

**Method:** Per CONTEXT.md and Plan 04-07 PLAN.md `<action>` §"Audit document constraints", verify:

1. `cardano-realism-audit.md` does NOT contain `2026-05-13` outside the §References / footer area (banners stripped per D-38).
2. `cardano-realism-audit.md` lands between 300 and 500 lines.
3. `validity-threats.md` lands between 500 and 850 lines.
4. `methodology-overview.md` has the seven `^### ` ODD headings matching the regex `^### (Purpose|State variables|Process overview|Design concepts|Initialisation|Input data|Submodels)$` plus the seven `### Worked example: ...` headings.
5. `realism-risks-register.md` has no `TBD plan 02` markers, no `(draft fallback;` markers on the 5 Phase-4-touched entries.

**Results:**

| Constraint | Result | Notes |
|---|---|---|
| 1. No `2026-05-13` banner-residue in audit prose | PASS | All 5 occurrences (lines 113, 115, 125, 198, 306) are inside `date-retrieved:` triples — legitimate citations, not banner-residue |
| 2. Audit lands 300–500 lines | PASS | Exactly 500 lines; at the verification cap. Plan 04-04 SUMMARY explicitly noted this trade-off; Plan 04-07 confirms further compaction would require dropping must-have content |
| 3. Validity-threats lands 500–850 lines | PASS | Exactly 850 lines; at the verification cap per Plan 04-05 SUMMARY's noted trade-off (preserving per-suite reviewability with new Related-RSK / Related-CLM / Phase-3-evidence fields necessarily expands each block) |
| 4. Methodology-overview has 7+7 H3 headings | PASS | 7 ODD-element H3 headings at lines 38 / 42 / 46 / 50 / 54 / 58 / 62; 7 Worked-example H3 headings at lines 72 / 86 / 107 / 131 / 145 / 192 / 225 |
| 5. Register has no `TBD plan 02` markers | PASS | 0 matches across the file |
| 5b. Register has no `(draft fallback;` markers on the 5 Phase-4-touched entries (RSK-pool-count, RSK-un-anchored-controller-knobs, RSK-calibration-stale-stake-snapshot, RSK-steady-state-run-length, RSK-multiplier-floor-4-suite-coverage) | PASS | 0 matches across the touched entries. Remaining `(draft fallback;` markers (on RSK-single-seed-precision and RSK-standard-user-fee-drift-exposure) are intentional per Plan 04-06 SUMMARY: both entries are LIVE, both carry forward-pointer fallback disclosure-paragraphs that may become load-bearing if their later-phase test results land DISCLOSED |

**Verdict:** PASS — all five audit-document constraints hold.

## Plan 04-01 optional 2024–2026 arXiv catch-up pass

**Status: NOT EXECUTED in Plan 04-07.**

Plan 04-01's executor recorded that the optional 2024–2026 arXiv follow-up pass was scoped but not executed because web-fetch tooling was unavailable in the executor environment for that run. Plan 04-01's §"2024–2026 follow-up arXiv pass" noted that Plan 04-07 may at its option run the follow-up pass and re-grade.

Plan 04-07's executor environment does not provide WebFetch or WebSearch tools either. Per Plan 04-01's recorded marginal-new-citation expectation:

- Sub-knob 1 (window length 32) is already ANCHORED at the D-35 bar via Reijsbergen et al. AFT 2021 + Leonardos et al. AFT 2021 + Liu et al. CCS 2022; a 2024–2026 pass could only strengthen the existing anchor.
- Sub-knobs 2 / 3 / 4 (multiplier-floor 4; multiplier-floor 16; lane-signal-source choices) are structurally not the kind of choices the Ethereum Improvement Proposal 1559 (EIP-1559) academic-critique literature can anchor: Ethereum has no second-lane controller, no multiplier floor, and no lane-signal-source choice. Per Plan 04-01 §"Search methodology and cut-off" the marginal-new-citation expectation for these three sub-knobs is zero.

**Cut-off recorded:** Plan 04-07 does not execute the 2024–2026 follow-up pass. Plan 04-01's recorded verdicts (1 ANCHORED + 3 DISCLOSED) stand. The disposition is robust to a future re-grade in the direction of *more* anchors only (no anchor can flip back to disclose; only disclose can flip to anchor if a 2024–2026 paper surfaces analysing a comparable second-lane mechanism).

A future Phase 5 / HAND-02 reviewer with WebFetch / WebSearch access may at their option run the follow-up pass; the current dispositions are stable.

## Defects fixed in place — summary

| # | Defect | Document | Fix |
|---|--------|----------|-----|
| 1 | `RSK-substrate-scope` carried Verdict `LIVE` in Index table and per-entry block despite Plan 04-04's audit-side substrate-scope umbrella fold having landed | `realism-risks-register.md` | Flipped Verdict from `LIVE` to `DISCLOSED` in both Index table and per-entry block; updated Scope-of-resolution + EXP/Resolution + Disclosure-paragraph italic-conditional + Reading-guide + footer prose to declarative post-Plan-04-07 form |
| 2 | Register Reading-guide and footer reported pre-Plan-04-07 distribution `7 LIVE + 17 DISCLOSED` | `realism-risks-register.md` | Updated to post-Plan-04-07 distribution `6 LIVE + 18 DISCLOSED + 0 MITIGATED + 0 DORMANT`; framed the flip as Plan 04-07's reconciliation of Plan 04-04's audit-side substrate-scope umbrella fold |
| 3 | `RSK-mev-strategic-bidder` cited in `methodology-overview.md` §"Worked example: Design concepts" but no such RSK identifier exists in the register | `methodology-overview.md` | Replaced citation with `RSK-substrate-scope` (the umbrella DISCLOSED entry whose disclosure-paragraph sub-point (c) covers MEV / strategic-bidder risk per `.planning/PROJECT.md` Out-of-Scope items 2 + 3) |
| 4 | `methodology-overview.md` linked `../../sim-rs/sim-cli/parameters/config.default.yaml` but that path does not exist (the file lives at `sim-rs/parameters/config.default.yaml`) | `methodology-overview.md` | Corrected the link target to `../../sim-rs/parameters/config.default.yaml` |
| 5 | `RSK-standard-user-fee-drift-exposure` identifier broken across a line in `validity-threats.md` §"phase-2-moderate-both-dynamic.yaml" Claim text (lines 575–576) | `validity-threats.md` | Reflowed the identifier onto a single line; backtick-wrapped the identifier consistent with the surrounding usage |
| 6 | `coverage-check.md` column-legend (lines 18–34) used CIP / EIP-1559 / RB / EB / LoC abbreviations before the §"Notation conventions" expansion block on line 45 | `coverage-check.md` | Added a consolidated `**Abbreviations on first use**` bullet immediately after the Verdict-vocabulary line in the header (line 7), listing 16 abbreviations including CIP, EIP-1559, RB, EB, LoC, BCa bootstrap, PSE, IQR, CV, SODA, CCS, AFT, RSK, CLM, EXP, SHA-256. The §"Notation conventions" expansion block on line 45 is retained as informational redundancy |
| 7 | `realism-risks-register.md` Index table (lines 19–44) used RB / MB / AFT / CCS / EMA / NFT abbreviations before body-section first-use expansions | `realism-risks-register.md` | Added a consolidated `**Abbreviations on first use**` line immediately after the Verdict-vocabulary line in the header (line 7), listing 24 abbreviations covering all standalone uses in the Index table and body |
| 8 | `methodology-overview.md` Status line 3 used "CIP" before line 4's "Cardano Improvement Proposal (CIP)" expansion | `methodology-overview.md` | Expanded CIP on line 3 ("Phase-2 Cardano Improvement Proposal (CIP) evidence base methodology index"); shortened line 4 to use the now-introduced abbreviation |

**Total defects:** 8 (all fixed in place; no defect escalated to user judgement).

## Post-fix verification

**Verdict distribution (re-verified post-fix):**

```text
$ grep -E '^\*\*Verdict:\*\*' docs/phase-2/realism-risks-register.md | sort | uniq -c
     18 **Verdict:** DISCLOSED
      6 **Verdict:** LIVE

$ awk '/^## Index/,/^## RSK-pool/' docs/phase-2/realism-risks-register.md | grep -oE '\| (LIVE|DISCLOSED|MITIGATED|DORMANT) \|' | sort | uniq -c
     18 | DISCLOSED |
      6 | LIVE |
```

Index table verdicts match per-entry Verdict fields. Both report 18 DISCLOSED + 6 LIVE.

**Cross-reference integrity (re-verified post-fix):**

- 0 dead `RSK-NN` references across the four documents + methodology-overview (RSK-mev-strategic-bidder defect fixed).
- 0 dead `CLM-NN` references.
- 0 dead `EXP-NN` references.

**Markdown link resolution (re-verified post-fix):**

- `cardano-realism-audit.md`: 0 broken links (10 unique relative links checked).
- `validity-threats.md`: 0 broken links (5 unique relative links checked).
- `realism-risks-register.md`: 0 broken backtick paths (20 unique paths checked).
- `coverage-check.md`: 0 broken backtick paths (11 unique paths checked).
- `methodology-overview.md`: 0 broken links (38 unique relative links checked; broken `sim-cli/parameters/config.default.yaml` defect fixed).

**Abbreviation-on-first-use (re-verified post-fix):**

- All four refreshed documents + methodology-overview now carry a consolidated header abbreviation expansion block.
- No standalone use precedes the first parenthetical expansion in any document (except the methodology-overview §"Worked example" deliberate re-expansions per CONTEXT.md D-42).

**Audit-document constraints (re-verified post-fix):**

- Audit: 500 lines, no banner-residue, all triples conform.
- Validity-threats: 850 lines, refreshed per Plan 04-05.
- Methodology-overview: 7 + 7 H3 headings present.
- Register: no `TBD plan 02` markers; no `(draft fallback;` markers on the 5 Phase-4-touched entries.

## Open for user review

**None.** All defects found by Plan 04-07's consistency review were within Plan 04-07's deviation-rule scope (cross-reference resolution, identifier hygiene, abbreviation-on-first-use, line breaks) and were fixed in place. No anomaly required user judgement.

The post-Phase-4 register distribution **6 LIVE + 18 DISCLOSED + 0 MITIGATED + 0 DORMANT** is the canonical post-Phase-4 state. Phase 5 / HAND-02 (the milestone-close consistency review) inherits a clean baseline.

---

*Plan 04-07 consistency review complete. See `.planning/phases/04-refresh-and-anchor/04-SUMMARY.md` for the Phase 4 SUMMARY consumed by `gsd-verify-phase`.*
