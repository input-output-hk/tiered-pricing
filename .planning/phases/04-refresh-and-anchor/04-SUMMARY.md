---
phase: 04-refresh-and-anchor
status: complete
date: 2026-05-18
requirements: [DOC-01, DOC-02, DOC-03, DOC-04]
plans: [04-01, 04-02, 04-03, 04-04, 04-05, 04-06, 04-07]
deliverables_complete: 4
deliverables_partial: 0
deliverables_total: 4
tags: [docs, audit-refresh, validity-threats-refresh, register-refresh, methodology-overview, anchor-or-disclose, phase-4-summary]
---

# Phase 04 Summary: Refresh and Anchor

Phase 4 refreshes the four authoritative phase-2 documents (audit, validity-threats, realism-risks-register, coverage-check) to consistent, Cardano Improvement Proposal (CIP)-pasteable voice and anchors-or-discloses the four un-anchored controller knobs identified in Phase 1. A new one-page Overview, Design concepts, Details (ODD) methodology index (`docs/phase-2/methodology-overview.md`) lands as the CIP's primary methodology citation target. Plan 04-07's Wave 3 consistency review reconciles all cross-document references, fixes eight in-place defects, and produces the post-Phase-4 register verdict distribution **6 LIVE + 18 DISCLOSED + 0 MITIGATED + 0 DORMANT** (down from Phase 1's 12 LIVE + 12 DISCLOSED).

## Goal

(per `.planning/ROADMAP.md` §"Phase 4: Refresh and Anchor")

> The authoritative audit and validity-threats documents are refreshed to consistent, CIP-pasteable voice, every calibration value carries a `(value, source, date-retrieved)` triple, the four un-anchored controller knobs are either anchored to deployed-system data or carry an explicit disclosure paragraph, and a one-page Overview, Design concepts, Details (ODD) methodology index exists for the CIP author to cite by repository (repo) Uniform Resource Locator (URL).

## Outcome

Phase 4 delivered all four success criteria (DOC-01 / DOC-02 / DOC-03 / DOC-04). The four refreshed documents and the new methodology-overview are CIP-pasteable and consistent across cross-references. The four un-anchored controller knobs identified in Phase 1 each have either an external anchor or a "conditional on X" disclosure paragraph per Plan 04-01's anchor-or-disclose work (one ANCHORED via Reijsbergen et al. Advances in Financial Technologies (AFT) 2021 + Leonardos et al. AFT 2021 + Liu et al. Conference on Computer and Communications Security (CCS) 2022; three DISCLOSED with sub-knob granularity). The umbrella entry `RSK-un-anchored-controller-knobs` flips LIVE → DISCLOSED. Five additional entries flip LIVE → DISCLOSED via Plan 04-06's register edits (`RSK-pool-count`, `RSK-calibration-stale-stake-snapshot`, `RSK-steady-state-run-length` via the TEST-05 / TEST-06 disclose-only fallback per CONTEXT.md `<deferred>`; `RSK-multiplier-floor-4-suite-coverage` via the TEST-07a regime-dependence reframe; `RSK-substrate-scope` via Plan 04-04's audit-side fold reconciled in Plan 04-07).

The Phase 3 TEST-05 (pool-number sensitivity) and TEST-06 (run-length / steady-state) cheap tests were deferred to disclose-only fallback in Phase 4 per `.planning/phases/04-refresh-and-anchor/04-CONTEXT.md` `<deferred>` — the partial coverage from Phase 3 (35 of 1650 TEST-05 runs ≈ 2.1%; 31 of 120 TEST-06 runs ≈ 26% covering only 1 of 4 menu arms) was insufficient to license MITIGATED verdicts. The disclose-only fallback is a deliberate Phase 4 decision rather than a coverage gap: the three affected RSK entries (`RSK-pool-count`, `RSK-calibration-stale-stake-snapshot`, `RSK-steady-state-run-length`) carry load-bearing disclosure-paragraphs identifying the 100-pool regime as the licensed pool-count regime and the partial steady-state coverage as a stylised run length. A future TEST-05 or TEST-06 re-run between Phase 4 close and CIP publication can be incorporated as a verdict-flip patch by a Phase 5 / HAND-02 reviewer.

## Deliverables status

| Deliverable | Where | Status |
|---|---|---|
| DOC-01 refreshed audit | `docs/phase-2/cardano-realism-audit.md` | **Complete** (Plan 04-04); 500 lines; 17 `(value, source, date-retrieved)` triples (14 with YYYY-MM-DD dates, 3 with em-dash dates for un-anchored values); 20 RSK-NN cross-references threaded through disclosure sections; substrate-scope umbrella paragraph included |
| DOC-02 refreshed validity-threats | `docs/phase-2/validity-threats.md` | **Complete** (Plan 04-05); 850 lines; 19 per-suite blocks acquired `Related RSK:` + `Related CLM:` cross-references; 7 per-suite blocks carry `Phase 3 evidence:` sub-fields citing TEST-03 / TEST-04 / TEST-07a N=20 Bias-corrected and accelerated (BCa) bootstrap Confidence Intervals (CIs); aggregate trust regenerated from 0 HIGH / 10 MEDIUM / 2 LOW / 4 UNRESOLVED to 2 HIGH / 13 MEDIUM / 4 LOW / 0 UNRESOLVED |
| DOC-03 anchor-or-disclose (audit + register) | `docs/phase-2/cardano-realism-audit.md` §"Pricing-controller calibration" + `docs/phase-2/realism-risks-register.md` `RSK-un-anchored-controller-knobs` | **Complete** (Plans 04-01 → 04-04 audit-side → 04-06 register-side); umbrella verdict DISCLOSED per Plan 04-01 anchor-or-disclose discipline; per-sub-knob disposition: window-length 32 ANCHORED via the Ethereum Improvement Proposal 1559 (EIP-1559) academic-critique tradition (Reijsbergen et al. AFT 2021 + Leonardos et al. AFT 2021 + Liu et al. CCS 2022); multiplier-floor 4 / multiplier-floor 16 / lane-signal-source DISCLOSED with per-sub-knob "conditional on X" disclosure-paragraphs in `RSK-un-anchored-controller-knobs` |
| DOC-04 methodology-overview | `docs/phase-2/methodology-overview.md` | **Complete** (Plan 04-02); 260 lines; 7 ODD-element H3 headings + 7 Worked-example H3 headings; worked example traces seed=1 of `menu_unreserved_priority_only_static_x4` end-to-end through the seven ODD elements |
| Register `disclosure-paragraph` edits + verdict flips | `docs/phase-2/realism-risks-register.md` | **Complete** (Plan 04-06 for 5 entries; Plan 04-07 for the sixth `RSK-substrate-scope` reconciliation); 6 register entries updated overall; post-Phase-4 verdict distribution **6 LIVE + 18 DISCLOSED + 0 MITIGATED + 0 DORMANT** |
| Coverage-check `signal-source-anchoring` updates | `docs/phase-2/coverage-check.md` | **Complete** (Plan 04-06); CLM-05 signal-source-anchoring parenthetical updated to cite Reijsbergen / Leonardos / Liu for the window-length 32 anchor; 14 other CLM rows reading `unanchored (RSK-un-anchored-controller-knobs)` preserved (their load-bearing sub-knobs remain DISCLOSED) |
| Consistency review | `.planning/phases/04-refresh-and-anchor/04-07-consistency-report.md` | **Complete** (Plan 04-07); 8 audit sections; 8 defects found and fixed in place; 0 open-for-user-review items |
| Phase 4 SUMMARY | `.planning/phases/04-refresh-and-anchor/04-SUMMARY.md` | **Complete** (this file; Plan 04-07) |

## What landed in tree

| File | Lines | Purpose |
|---|---|---|
| `docs/phase-2/cardano-realism-audit.md` | 500 | Refreshed; three historical banners (2026-05-13, 2026-05-14, 2026-05-13-corrected) stripped and folded; 17 `(value, source, date-retrieved)` triples; Phase 3 multi-seed evidence integrated into §"Recommended disclosure statements" "On the menu-item welfare distinction" paragraph; substrate-scope umbrella paragraph included |
| `docs/phase-2/validity-threats.md` | 850 | Refreshed in place; 19 per-suite `Related RSK:` + `Related CLM:` cross-references; 7 per-suite Phase 3 evidence sub-fields; aggregate trust 2 HIGH / 13 MEDIUM / 4 LOW / 0 UNRESOLVED |
| `docs/phase-2/realism-risks-register.md` | 451 | 6 entries' verdicts flipped LIVE → DISCLOSED (`RSK-un-anchored-controller-knobs`, `RSK-pool-count`, `RSK-calibration-stale-stake-snapshot`, `RSK-steady-state-run-length`, `RSK-multiplier-floor-4-suite-coverage` in Plan 04-06; `RSK-substrate-scope` in Plan 04-07); load-bearing disclosure-paragraphs landed for each; consolidated abbreviation-on-first-use header added by Plan 04-07 |
| `docs/phase-2/coverage-check.md` | 154 | CLM-05 signal-source-anchoring parenthetical updated per Plan 04-01's window-length-32 anchor outcome; consolidated abbreviation-on-first-use header added by Plan 04-07 |
| `docs/phase-2/methodology-overview.md` | 260 | New ODD methodology overview with 7-row index table + per-element prose + worked example traced through `menu_unreserved_priority_only_static_x4` at seed=1; Plan 04-07 fixed one dead RSK reference (`RSK-mev-strategic-bidder` → `RSK-substrate-scope`), one broken link (`sim-cli/parameters/config.default.yaml` → `parameters/config.default.yaml`), and one Status-line CIP-before-expansion violation |
| `.planning/phases/04-refresh-and-anchor/04-01-DOC-03-anchor-search.md` | 654 | Plan 04-01 per-sub-knob anchor decisions + rejected-citations list (consulted: Reijsbergen et al. AFT 2021, Leonardos et al. AFT 2021, Liu et al. CCS 2022, Roughgarden EC 2021, Azouvi DISC 2023, Chung and Shi Symposium on Discrete Algorithms (SODA) 2023; rejected non-anchors documented per sub-knob) |
| `.planning/phases/04-refresh-and-anchor/04-03-phase3-evidence-summary.md` | 461 | Plan 04-03 consolidated Phase 3 evidence for Wave 2 plans (TEST-03 + TEST-04 + TEST-07a numerical findings; TEST-05 / TEST-06 disclose-only fallback decision) |
| `.planning/phases/04-refresh-and-anchor/04-07-consistency-report.md` | 220 | Plan 04-07 consistency audit results (8 sections; 8 defects fixed in place); post-Phase-4 distribution 6 LIVE + 18 DISCLOSED |

## Determinism + invariants

**Phase 4 is documentation-only; no simulator code modified.** The M2 / M3 unit-test goldens are unchanged. The M5 suite-level goldens are unchanged. `cargo test --workspace` is unperturbed. No new `f64` paths added to simulation-affecting state. No changes to the chain-derived Family B controller or the integer / rational / 128-bit unsigned (u128) discipline in the pricing kernel.

The Phase 4 deliverables are CIP-cited artefacts and do not feed back into any simulation decision. The reporting `f64` boundary in `retained_value` and related welfare aggregates is documented in `RSK-welfare-as-f64-reporting` (DISCLOSED) — reported magnitudes should be interpreted to ≤ 3 significant figures.

## Phase 5 inputs

Phase 4 hands the following to Phase 5 (CIP-author handoff per HAND-01 / HAND-02 / HAND-03):

- **Four refreshed CIP-cited documents** (`docs/phase-2/cardano-realism-audit.md`, `docs/phase-2/validity-threats.md`, `docs/phase-2/realism-risks-register.md`, `docs/phase-2/coverage-check.md`) plus the new `docs/phase-2/methodology-overview.md`. These are the five primary CIP-cited artefacts.
- **Register load-bearing disclosure-paragraphs (18 DISCLOSED entries).** The disclosure-paragraph fields paste verbatim into the CIP's Limitations section per Phase 5 / HAND-01. The 6 remaining LIVE entries (`RSK-single-seed-precision`, `RSK-three-seed-statistical-power`, `RSK-unresolved-suite-claims`, `RSK-standard-user-fee-drift-exposure`, `RSK-menu-collapse-to-advocacy`, `RSK-hash-diversity-policy`) carry forward-pointer disclosure-paragraphs that Phase 5 must reconcile (most resolve via the Phase 2 / Phase 3 evidence already in the coverage-check; some require an explicit Phase 5 decision).
- **Plan 04-01 anchor-or-disclose audit trail.** `.planning/phases/04-refresh-and-anchor/04-01-DOC-03-anchor-search.md` records the per-sub-knob anchor decisions and the rejected-citations list. Phase 5 / HAND-02 can re-grade if a 2024–2026 follow-up arXiv pass surfaces new anchors.
- **Plan 04-03 Phase 3 evidence summary.** `.planning/phases/04-refresh-and-anchor/04-03-phase3-evidence-summary.md` consolidates the TEST-03 / TEST-04 / TEST-07a multi-seed BCa CI findings; the headline two-bullet finding ("un-reserved menu arms outperform single-lane EIP-1559; RB-reserved menu arms underperform under the same calibration") is the load-bearing welfare-distinction for the CIP's menu-option recommendation.
- **Plan 04-07 consistency report.** `.planning/phases/04-refresh-and-anchor/04-07-consistency-report.md` documents the cross-reference integrity of the five CIP-cited documents; Phase 5 / HAND-02 inherits a clean baseline (0 dead references; 0 broken markdown links; 0 abbreviation-on-first-use violations).

## Open questions for Phase 5

1. **TEST-05 re-run is user-managed and out of Phase 4 scope** per CONTEXT.md `<deferred>`. `RSK-pool-count` and `RSK-calibration-stale-stake-snapshot` both land at DISCLOSED via the existing fallback prose, citing the 100-pool licensed regime. If the user re-runs TEST-05 at 100 vs 150 pools between Phase 4 close and CIP publication, Phase 5 can incorporate the data as a verdict-flip patch (the locked threshold "Δ% < seed-Inter-Quartile Range (IQR) of same job at 100 pools establishes MITIGATED" is preserved in each entry's `Scope-of-resolution` field).
2. **TEST-06 re-run is similarly user-managed.** `RSK-steady-state-run-length` lands at DISCLOSED with the partial 1-of-4-menu-arms coverage explicitly named in the disclosure-paragraph. The re-run recipe (`scripts/run-phase-3-suites.sh 1 parameters/phase-2-sweep/suites/phase-3-run-length.yaml`, ~56 minutes wall-clock at parallelism 8) is preserved.
3. **The CIP author summary (HAND-01) is Phase 5 work.** HAND-01 will identify which `disclosure-paragraph` block from `realism-risks-register.md` pastes into which CIP section, and which `CLM-NN` row backs which CIP claim. The 18 DISCLOSED disclosure-paragraphs in the register are the primary paste-target.
4. **The optional 2024–2026 arXiv follow-up pass** for Plan 04-01's anchor search was scoped but not executed by Plan 04-01 (web-fetch tooling unavailable) and not executed by Plan 04-07 (same constraint). The current dispositions (1 sub-knob ANCHORED + 3 DISCLOSED) are robust to a future re-grade in the direction of *more* anchors only; a Phase 5 / HAND-02 reviewer with WebFetch / WebSearch access may at their option run the follow-up pass.
5. **The 6 remaining LIVE register entries** (RSK-single-seed-precision, RSK-three-seed-statistical-power, RSK-unresolved-suite-claims, RSK-standard-user-fee-drift-exposure, RSK-menu-collapse-to-advocacy, RSK-hash-diversity-policy) all carry either forward-pointer disclosure-paragraphs or draft fallback prose. Phase 5 / HAND-01 needs to decide for each whether: (a) the CIP cites the Phase 2 / Phase 3 evidence as sufficient to flip to MITIGATED, (b) the existing draft fallback prose is promoted to load-bearing and the entry flips to DISCLOSED, or (c) the entry stays LIVE with the load-bearing implication explicit in the CIP. Recommend: most can flip to DISCLOSED via the existing prose; only RSK-hash-diversity-policy requires an active policy decision (strict vs soft gate) before CIP publication.

## Verification

(per the four `.planning/ROADMAP.md` Phase 4 success criteria)

| Success criterion | Verdict | Evidence |
|---|---|---|
| **DOC-01:** `cardano-realism-audit.md` reads in authoritative voice (2026-05-13 banners removed and folded), every calibration value is a `(value, source, date-retrieved)` triple, substrate-scope paragraph included | **PASS** | 500 lines; 0 banner-residue in non-citation prose (all 5 `2026-05-13` occurrences inside legitimate `date-retrieved:` triples); 17 triples total (14 YYYY-MM-DD + 3 em-dash); substrate-scope umbrella paragraph at lines 335–349; Plan 04-04 SUMMARY confirms |
| **DOC-02:** `validity-threats.md` per-suite trust ratings cross-referenced to `RSK-NN`; verdicts consistent with register; menu-item-trade-off claims from coverage-check added | **PASS** | 850 lines; 19 per-suite `Related RSK:` fields (3-8 RSK-NN per block); 19 per-suite `Related CLM:` fields (CLM-01..CLM-55 mapped across); 7 per-suite Phase 3 evidence sub-fields; aggregate trust 2 HIGH / 13 MEDIUM / 4 LOW / 0 UNRESOLVED reconciled with register's 18 DISCLOSED + 6 LIVE post-Plan-04-07 distribution; Plan 04-07 consistency report §"Register ↔ validity-threats verdict reconciliation audit" verdict: PASS |
| **DOC-03:** Each of the four un-anchored controller knobs has either an external anchor or a "conditional on X" disclosure paragraph | **PASS** | Plan 04-01 disposed: sub-knob 1 (window-length 32) ANCHORED via Reijsbergen et al. AFT 2021 + Leonardos et al. AFT 2021 + Liu et al. CCS 2022; sub-knobs 2 / 3 / 4 (multiplier-floor 4; multiplier-floor 16; lane-signal-source) DISCLOSED with per-sub-knob "conditional on X" disclosure-paragraphs in `RSK-un-anchored-controller-knobs`. Umbrella verdict DISCLOSED. Plan 04-04 folded the per-sub-knob narration into `cardano-realism-audit.md` §"Pricing-controller calibration"; Plan 04-06 folded the per-sub-knob register prose into `RSK-un-anchored-controller-knobs`'s `Disclosure-paragraph` field |
| **DOC-04:** `methodology-overview.md` exists as ODD index + per-element prose + worked example | **PASS** | 260 lines; 7 ODD-element H3 headings (Purpose / State variables / Process overview / Design concepts / Initialisation / Input data / Submodels) at lines 38 / 42 / 46 / 50 / 54 / 58 / 62; 7 Worked-example H3 headings at lines 72 / 86 / 107 / 131 / 145 / 192 / 225; worked example traces `menu_unreserved_priority_only_static_x4 × seed=1` end-to-end through the seven elements per Plan 04-02 SUMMARY |

All four ROADMAP.md Phase 4 success criteria pass. Phase 4 is **verifiably complete** by `gsd-verify-phase`.

## Abbreviations on first use

(per `CLAUDE.md` §"Conventions / gotchas"): Cardano Improvement Proposal (CIP); Ethereum Improvement Proposal 1559 (EIP-1559); Ranking Block (RB); Endorser Block (EB); Realism Risk identifier (RSK); Claim identifier (CLM); Experiment identifier (EXP); Overview, Design concepts, Details (ODD) protocol; Bias-corrected and accelerated (BCa) bootstrap; Confidence Interval (CI); Inter-Quartile Range (IQR); Conference on Computer and Communications Security (CCS); Advances in Financial Technologies (AFT); Symposium on Discrete Algorithms (SODA); Maximum Extractable Value (MEV); 128-bit unsigned integer (u128); Uniform Resource Locator (URL); repository (repo); Secure Hash Algorithm 256-bit (SHA-256); Yet Another Markup Language (YAML).

## Self-Check: PASSED

**Files verified present on disk:**

- `/home/will/git/arc-tiered-pricing/docs/phase-2/cardano-realism-audit.md` — 500 lines
- `/home/will/git/arc-tiered-pricing/docs/phase-2/validity-threats.md` — 850 lines
- `/home/will/git/arc-tiered-pricing/docs/phase-2/realism-risks-register.md` — 451 lines
- `/home/will/git/arc-tiered-pricing/docs/phase-2/coverage-check.md` — 154 lines
- `/home/will/git/arc-tiered-pricing/docs/phase-2/methodology-overview.md` — 260 lines
- `/home/will/git/arc-tiered-pricing/.planning/phases/04-refresh-and-anchor/04-01-SUMMARY.md` — present
- `/home/will/git/arc-tiered-pricing/.planning/phases/04-refresh-and-anchor/04-02-SUMMARY.md` — present
- `/home/will/git/arc-tiered-pricing/.planning/phases/04-refresh-and-anchor/04-03-SUMMARY.md` — present
- `/home/will/git/arc-tiered-pricing/.planning/phases/04-refresh-and-anchor/04-04-SUMMARY.md` — present
- `/home/will/git/arc-tiered-pricing/.planning/phases/04-refresh-and-anchor/04-05-SUMMARY.md` — present
- `/home/will/git/arc-tiered-pricing/.planning/phases/04-refresh-and-anchor/04-06-SUMMARY.md` — present
- `/home/will/git/arc-tiered-pricing/.planning/phases/04-refresh-and-anchor/04-07-consistency-report.md` — created by Plan 04-07 Task 1; present

**Final state verification:**

- Register verdict distribution: 18 DISCLOSED + 6 LIVE (post-Plan-04-07; matches the post-Phase-4 narrative in Reading guide and footer)
- 0 dead RSK-NN / CLM-NN / EXP-NN references across the five CIP-cited documents
- 0 broken markdown links / backtick paths across the five documents
- 0 `TBD plan 02` markers in the register
- 0 `(draft fallback;` markers on the 5 Phase-4-touched RSK entries
- Audit lands at 500 lines (verification cap); validity-threats lands at 850 lines (verification cap); methodology-overview has 7 + 7 H3 headings as required

**ROADMAP.md Phase 4 success criteria:**

- DOC-01 PASS — refreshed audit in authoritative voice; 17 triples; substrate-scope paragraph included
- DOC-02 PASS — refreshed validity-threats with 19 per-suite Related-RSK + Related-CLM cross-references; verdicts reconciled
- DOC-03 PASS — 1 ANCHORED + 3 DISCLOSED per-sub-knob in `RSK-un-anchored-controller-knobs`
- DOC-04 PASS — methodology-overview ODD index + worked example present

Phase 4 is **verifiably complete** for `gsd-verify-phase` consumption. Phase 5 (Handoff) is the next phase.

---

*Phase 4 / 04-refresh-and-anchor / completed 2026-05-18. Plans executed: 04-01, 04-02, 04-03, 04-04, 04-05, 04-06, 04-07.*
