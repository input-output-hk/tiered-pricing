# Phase 4: Refresh and Anchor - Context

**Gathered:** 2026-05-18
**Status:** Ready for planning

<domain>
## Phase Boundary

Refresh the authoritative phase-2 audit and validity-threats documents from their current annotated-historical state into Cardano Improvement Proposal (CIP)-pasteable voice, anchor or disclose the four un-anchored controller knobs, and produce a one-page Overview, Design concepts, Details (ODD) methodology index for the CIP author to cite by repository (repo) Uniform Resource Locator (URL). The phase consumes the Phase 1 register and Phase 3 cheap-test results as inputs and produces four deliverables that feed Phase 5's CIP-author handoff:

- `docs/phase-2/cardano-realism-audit.md` (DOC-01) — full rewrite in single authoritative voice; every calibration value reformatted as a `(value, source, date-retrieved)` triple; the two 2026-05-13 and 2026-05-14 annotation banners stripped and folded inline; dual-purpose ("Recommended disclosure statements" section regenerated against Phase 3 evidence)
- `docs/phase-2/validity-threats.md` (DOC-02) — per-suite trust matrix retained and refreshed; the 19 suite blocks acquire `RSK-NN` cross-references; the "Resolved 2026-05-13" / "Resolved 2026-05-14" banners are folded inline; verdicts reconciled with the register
- Anchor-or-disclose work for the four un-anchored controller knobs (DOC-03) — window-length 32, multiplier-floor 4, multiplier-floor 16, and lane-signal-source choices; motivating-citation bar; literature search runs until marginal anchor unlikely (no fixed budget); anchors land in both the audit (brief citation) and the register (full prose + verdict flip if applicable)
- `docs/phase-2/methodology-overview.md` (DOC-04) — ODD index table + brief prose per element + one worked example tracing a single (job, seed) end-to-end through the seven ODD elements; doubles as onboarding doc (~500+ lines)

Phase 4 reads the Phase 3 results (TEST-03, TEST-04, TEST-07a complete; TEST-05 pool-number sensitivity and TEST-06 run-length / steady-state being re-run by the user in flight and expected complete at Phase 4 commencement) and updates the realism-risks register's `disclosure-paragraph` fields and verdict flips in line with the final test verdicts.

Requirements covered: DOC-01, DOC-02, DOC-03, DOC-04.

</domain>

<decisions>
## Implementation Decisions

### TEST-05 / TEST-06 status

- **D-34:** The user is re-running TEST-05 (pool-number sensitivity, ~50 min wall-clock at `-P 8`) and TEST-06 (run-length / steady-state, ~56 min wall-clock at `-P 8`) themselves, in parallel with Phase 4 discuss. Both data sets are expected to be available at Phase 4 commencement (planning / execution start). Phase 4 reads the completed `.planning/realism-tests/pool-number-sensitivity/results.md` and `.planning/realism-tests/run-length-steady-state/results.md` as inputs to DOC-01 (refreshed audit), DOC-02 (refreshed validity-threats), and the register's `RSK-pool-count`, `RSK-calibration-stale-stake-snapshot`, and `RSK-steady-state-run-length` `disclosure-paragraph` fields. Verdict flips for these three Realism Risk (RSK) entries are conditional on the in-flight re-run results:
  - **MITIGATED** if TEST-05 Δ% falls inside seed-Inter-Quartile-Range (IQR) per the Phase 1 locked threshold (REG-05); **LIVE → DISCLOSED** otherwise.
  - **MITIGATED** for `RSK-steady-state-run-length` if TEST-06's per-(job, seed) `|median(deltas across seeds)| < seed-IQR` holds across all four canonical menu-option jobs at 2000 slots; suite defaults raised per menu option if 2000 slots fails for any option.

### Anchor-or-disclose bar (DOC-03)

- **D-35:** The "anchored" bar is **motivating citation suffices**. A knob is considered anchored if a published paper or deployed-system reference motivates the *kind* of choice the simulator makes, even without a numerical match. Under this bar:
  - **Window length 32**: plausibly anchors via Liu et al. Conference on Computer and Communications Security (CCS) 2022 and Reijsbergen et al. Advances in Financial Technologies (AFT) 2021 (EIP-1559 short-term oscillation motivating any window > 1).
  - **Multiplier-floor 4 and multiplier-floor 16**: plausibly anchor via the spec's own rationale (`docs/phase-2/mechanism-design.md` §"Open calibration choices" and the CLAUDE.md calibration-choice narrative).
  - **Lane-signal-source**: likely stays disclosed (genuinely spec-open per `mechanism-design.md` lines 207-211 enumerating three options; option 1 chosen without a motivating external reference).

- **D-36:** The anchor lands in **both** the refreshed audit and the register. The refreshed audit (`cardano-realism-audit.md`) carries the brief citation + `(value, source, date-retrieved)` triple under §"Pricing-controller calibration". The register's `RSK-un-anchored-controller-knobs` entry's `disclosure-paragraph` field carries the full motivating-citation prose; if all four knobs anchor, the entry verdict flips from LIVE to MITIGATED; otherwise the entry retains a refined disclosure-paragraph naming which sub-knobs anchored and which did not. CIP-author paste-targets: register for the CIP Limitations section, audit for any CIP Methodology / Parameters table mention.

- **D-37:** Literature search budget is **open-ended**. The REQUIREMENTS.md "two-hour" figure was an estimate, not a hard cap. The planner cuts the search when the marginal anchor is judged unlikely; per-knob effort is asymmetric (window-length anchor candidates are well-known in the EIP-1559 literature; lane-signal-source has no obvious literature handle so the search exits early). The cut-off decision and the consulted-but-rejected citations are documented in the DOC-03 plan output for reviewer traceability.

### DOC-01 / DOC-02 refresh strategy

- **D-38:** `docs/phase-2/cardano-realism-audit.md` is a **full rewrite in single authoritative voice**. The two existing annotation banners (2026-05-13 topology correction stating that `topology-realistic-100.yaml` is the operational topology; 2026-05-14 chain-derived controller commitment) are stripped; the sections they invalidated (the original "Single-producer topology (N=1)" disclosure paragraphs, the historical accumulator-cadence framing) are removed entirely rather than annotated. Every calibration value is presented as a `(value, source, date-retrieved)` triple. The CIP-pasteable substrate-scope paragraph (required by DOC-01 success criterion #1) is included. Approximate authoring effort: 410 → 300–400 lines.

- **D-39:** The refreshed audit is **dual-purpose**: it retains a "Recommended disclosure statements" section with CIP-pasteable disclosure prose regenerated against Phase 3 evidence (un-reserved arms outperform single-lane Ethereum Improvement Proposal 1559 (EIP-1559); ranking-block-reserved (RB-reserved) arms underperform single-lane EIP-1559; the multiplier_floor=4 calibration is regime-dependent at floor=16; the cross-arm duplicate-job artefact at `sundaeswap_moderate × floor=4` replicates at N=20). Each paragraph in the audit may duplicate or cross-reference the register's `disclosure-paragraph` for the same risk; the duplication is accepted as the cost of the audit reading as a self-contained CIP-pasteable document. **This is a deliberate deviation from Phase 1 D-02's "audit = calibration-provenance only" prescription**, taken because the existing audit's "Recommended disclosure statements" section is load-bearing for the CIP-author paste path.

- **D-40:** `docs/phase-2/validity-threats.md` retains its **per-suite trust matrix** and is refreshed in place. The 19 suite blocks acquire `RSK-NN` cross-references in each per-suite "Trust" sub-section; the "Resolved 2026-05-13" / "Resolved 2026-05-14" historical banners are folded into the per-suite prose; verdicts are reconciled with the register's verdicts; menu-item-trade-off claims that the Phase 2 coverage check identifies (claim identifier (CLM)-N cross-references) are added where applicable. Approximate output: 713 → 600–800 lines, refreshed but not restructured. **This is a deliberate deviation from Phase 1 D-01's "validity-threats becomes a thin pointer" prescription**, taken to preserve per-suite reviewability as an accountability artefact; three sources (register, coverage check, validity-threats) coexist with some duplicated content.

### DOC-04 ODD methodology overview

- **D-41:** Lives at **`docs/phase-2/methodology-overview.md`** (not `.planning/methodology-overview.md`). Rationale: REQUIREMENTS.md mandates that the CIP cite this document by repo URL; `docs/phase-2/` is the natural home alongside the other CIP-cited artefacts (`cardano-realism-audit.md`, `validity-threats.md`, `realism-risks-register.md`, `coverage-check.md`).

- **D-42:** Shape is **index table + brief prose per ODD element + one worked example**:
  - **Index table at the top.** Seven rows (Purpose, State variables, Process overview, Design concepts, Initialisation, Input data, Submodels) × 3 columns (ODD element, in-repo location, one-line description).
  - **Brief prose per ODD element.** A ~4-6-sentence paragraph per element summarising what that element looks like in phase-2, with inline file-path links to canonical content in CLAUDE.md, `mechanism-design.md`, suite YAMLs, and source files. Prose is *summary-only* — content lives in the linked files; the paragraph orients the reader without duplicating the linked file's prose.
  - **One worked example.** Traces a single canonical (job, seed) pair end-to-end through the seven ODD elements (e.g. seed=1 of one canonical menu-item job from `phase-2-priority-only-unreserved`) — what `Purpose` was for that run, what the `State variables` were at slot zero, what the `Process overview` looked like across one block-production cycle, etc. Doubles as onboarding documentation for new contributors. Expected total: ~500+ lines.

### Plan-wave decomposition (sketch)

- **D-43:** The plan-wave decomposition is delegated to the gsd-planner agent, but a reasonable default sketch:
  - **Wave 1 (parallel):** DOC-03 literature search; DOC-04 methodology-overview draft (no upstream dependency); read TEST-05 / TEST-06 results.
  - **Wave 2 (gated on Wave 1; parallel within wave):** DOC-01 full rewrite (audit); DOC-02 in-place refresh (validity-threats); register `disclosure-paragraph` updates for the three RSK entries gated on TEST-05 / TEST-06 verdicts and the four un-anchored knob entries.
  - **Wave 3 (sequential):** Consistency review across the four refreshed documents — RSK-NN / CLM-NN cross-reference integrity, verdict reconciliation between register and validity-threats, abbreviation-on-first-use audit. Plan 04-SUMMARY for verify-phase consumption.

### Claude's Discretion

The following items have planner / executor latitude with reasonable defaults named here:

- **Worked-example job choice (DOC-04).** A canonical menu-item job from `phase-2-priority-only-unreserved.yaml` or `phase-2-two-lane-both-dynamic.yaml` (the two suites whose TEST-04 cells show the un-reserved-arms-outperform-single-lane-EIP-1559 headline). Seed = 1 for tractability; ~2000 slots. The choice should match the Phase 3 TEST-04 canonical cell so the worked example walks through a CIP-cited result.

- **Audit narrative ordering (DOC-01).** The refreshed audit's "What lines up with mainnet" / "What needs disclosure" / "What does NOT transfer cleanly" / "Recommended disclosure statements" sections may be reorganised at planner discretion if the Phase 3 un-reserved vs RB-reserved welfare distinction warrants a new top-level section. Default: keep the existing four-section ordering; integrate Phase 3 findings into "What needs disclosure" item by item and into the disclosure prose at the bottom.

- **Multiplier-floor regime-dependence narration (DOC-01 and register).** TEST-07a found that at floor=16, the `phase-2-rb-scarcity` finding inverts and the `phase-2-urgency-inversion` finding weakly reverses. Where this lands in DOC-01: the §"Pricing-controller calibration" disclosure item #2 (multiplier_floor=4 as calibration accommodation) is rewritten to lead with the regime-dependent finding; the §"Recommended disclosure statements" "On controller calibration" paragraph is extended to name the regime dependence explicitly. The register's `RSK-multiplier-floor-4-suite-coverage` `disclosure-paragraph` is updated in parallel.

- **Per-suite trust-matrix refresh scope (DOC-02).** The 19 suite blocks include the 7 goldens-pinned M3/M4 suites and 12 unpinned demand-regime suites. The planner may keep all 19 in the refreshed matrix or prune the 4 UNRESOLVED suites named in Phase 1 plan-02 SUMMARY (now resolved via Phase 2 coverage-check walk) to the current set of resolved suites. Default: keep all 19; the four formerly-UNRESOLVED suites get refreshed verdicts derived from Phase 2's output-read.

- **DOC-03 literature-search scope.** The planner reads at minimum: Liu et al. CCS 2022 (Ethereum mainnet EIP-1559 controller stability), Reijsbergen et al. AFT 2021 (EIP-1559 short-term oscillation), Leonardos et al. AFT 2021 (EIP-1559 analysis). Optional further reading: any 2024-2026 Ethereum gas-fee-market literature surfacing in a basic Google Scholar / arXiv pass. The search exits when the marginal new citation adds nothing to the current anchor / disclose set; rejected-citations are listed in DOC-03's output for traceability.

- **Verdict-flip authority.** The register's RSK entry verdicts are updated by Phase 4 only where Phase 3 / Phase 4 evidence licenses the flip (e.g. TEST-05 verdict → `RSK-pool-count`, anchor citation → `RSK-un-anchored-controller-knobs`). Existing DISCLOSED entries remain DISCLOSED unless the Phase 4 work surfaces new evidence; existing MITIGATED entries (none in v1 register; Phase 3 added some via TEST-03 / TEST-04) remain MITIGATED.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-level
- [`.planning/PROJECT.md`](../../PROJECT.md) — Project context, core value, Active requirements, Out of Scope, Key Decisions (especially the targeted-cheap-test pattern, the EIP-1559 demoted-to-control decision, the substrate-scope deferral)
- [`.planning/REQUIREMENTS.md`](../../REQUIREMENTS.md) — REQ-IDs covered by this phase: DOC-01, DOC-02, DOC-03, DOC-04
- [`.planning/ROADMAP.md`](../../ROADMAP.md) §"Phase 4: Refresh and Anchor" — goal, dependencies (Phase 3), 4 success criteria
- [`CLAUDE.md`](../../../CLAUDE.md) — Abbreviation-on-first-use rule (§"Conventions / gotchas"), calibration choices, mechanism abstractions, calibration-fix-postmortem cross-reference

### Phase 1 outputs (the register Phase 4 updates in place)
- [`docs/phase-2/realism-risks-register.md`](../../../docs/phase-2/realism-risks-register.md) — the 24 RSK-NN entries; Phase 4 updates `disclosure-paragraph` fields and verdict flips for: `RSK-pool-count`, `RSK-calibration-stale-stake-snapshot`, `RSK-steady-state-run-length` (gated on TEST-05 / TEST-06 results), `RSK-un-anchored-controller-knobs` (gated on DOC-03 anchor outcomes), and `RSK-multiplier-floor-4-suite-coverage` (gated on TEST-07a regime-dependence finding)
- [`.planning/phases/01-register-inventory/01-CONTEXT.md`](../01-register-inventory/01-CONTEXT.md) — D-01 (validity-threats refresh fate; **Phase 4 overrides to retain matrix per D-40**), D-02 (audit refresh fate; **Phase 4 partially overrides to dual-purpose per D-39**), D-06 (LIVE/DORMANT/MITIGATED/DISCLOSED vocabulary), D-08 (RSK required fields)
- [`.planning/phases/01-register-inventory/01-02-SUMMARY.md`](../01-register-inventory/01-02-SUMMARY.md) — verdict distribution at register-v1 close (12 LIVE + 12 DISCLOSED); the four UNRESOLVED non-pinned suites' status post Phase 2 coverage-check walk

### Phase 2 outputs (the coverage-check rows Phase 4 cross-references)
- [`docs/phase-2/coverage-check.md`](../../../docs/phase-2/coverage-check.md) — the CLM-NN rows (current count ~25-40; updated incrementally through Phase 3); DOC-02 adds CLM cross-references inside each of the 19 per-suite Trust sub-sections in validity-threats.md
- [`.planning/phases/02-coverage-check-skeleton/02-CONTEXT.md`](../02-coverage-check-skeleton/02-CONTEXT.md) — D-13 (per-(claim, menu-option) row shape), D-14 (non-welfare property column enum vocabulary; signal-source-anchoring is the column DOC-03 may update from `unanchored` to `mainnet-data-cited` if any knob anchors), D-16 (CLM verdict vocabulary), D-19 (strict hash-diversity gate semantics)

### Phase 3 outputs (the test verdicts Phase 4 narrates into the docs)
- [`.planning/phases/03-targeted-cheap-tests/03-SUMMARY.md`](../03-targeted-cheap-tests/03-SUMMARY.md) — headline finding (un-reserved arms outperform single-lane EIP-1559 at N=20 seeds; RB-reserved arms underperform single-lane EIP-1559); the four Phase 3 disclosures Phase 4 must integrate; open questions for Phase 4 (the multiplier_floor regime-dependence disclosure, the cross-arm duplicate-job artefact)
- [`.planning/realism-tests/multi-seed-variance/results.md`](../../realism-tests/multi-seed-variance/results.md) — TEST-03 + TEST-04 full results; 9 cells with paired Bias-corrected and accelerated (BCa) 95% confidence intervals; hash-diversity gate 17/17
- [`.planning/realism-tests/multiplier-floor-16-companion/results.md`](../../realism-tests/multiplier-floor-16-companion/results.md) — TEST-07a: floor=4 vs floor=16 qualitative findings; the rb-scarcity inversion and urgency-inversion weak reversal
- [`.planning/realism-tests/pool-number-sensitivity/results.md`](../../realism-tests/pool-number-sensitivity/results.md) — TEST-05 (user-managed re-run in flight; expected complete at Phase 4 commencement)
- [`.planning/realism-tests/run-length-steady-state/results.md`](../../realism-tests/run-length-steady-state/results.md) — TEST-06 (user-managed re-run in flight; expected complete at Phase 4 commencement)
- [`.planning/realism-tests/hash-diversity-gate/results.md`](../../realism-tests/hash-diversity-gate/results.md) — Wave 3 coverage check 5 (COV-05) report; all 17 BACKED-eligible cells pass distinct-hash test

### Documents being refreshed by this phase
- [`docs/phase-2/cardano-realism-audit.md`](../../../docs/phase-2/cardano-realism-audit.md) — DOC-01 target; current 410 lines with two annotation banners (2026-05-13 topology, 2026-05-14 controller calibration); §"Verdict by category" table, §"What lines up with mainnet", §"What needs disclosure" (Fee structure, Pricing-controller calibration, Topology and actor model), §"Recommended disclosure statements" (CIP-pasteable prose)
- [`docs/phase-2/validity-threats.md`](../../../docs/phase-2/validity-threats.md) — DOC-02 target; current 713 lines with "Resolved 2026-05-13" and "Resolved 2026-05-14" sub-sections; per-suite trust matrix across the 19 phase-2 suites; trust framework

### Mechanism and calibration sources
- [`docs/phase-2/mechanism-design.md`](../../../docs/phase-2/mechanism-design.md) — the spec; §"Open calibration choices" enumerates the four spec-open choices DOC-03 anchors or discloses; lines 207-211 are the three lane-signal-source options that the simulator picks option 1 from
- [`.planning/family-b-decision-2026-05-14.md`](../../family-b-decision-2026-05-14.md) — Family B publication-commit memo; §"What changed" supplies the by-construction reorg-safety claim that the refreshed audit cites; §"Empirical welfare-impact characterisation" sources Phase 4's narrative for the un-reserved-arms vs RB-reserved-arms distinction
- [`.planning/mechanism-welfare-impact-2026-05-14.md`](../../mechanism-welfare-impact-2026-05-14.md) — 33-job sundaeswap-smoke characterisation; sources the four sign-flip cells DOC-01 references in the multiplier_floor regime-dependence narrative
- [`.planning/family-b-results-table-2026-05-14.md`](../../family-b-results-table-2026-05-14.md) — per-(job, seed) result table; cross-referenced from DOC-02's per-suite trust matrix
- [`docs/phase-2/calibration-fix-postmortem.md`](../../../docs/phase-2/calibration-fix-postmortem.md) — the rb-prob = 1.0 → rb-prob = 0.05 fix; supplies the `(value, source, date-retrieved)` triple for `rb-generation-probability` in the refreshed audit

### DOC-03 literature-search candidates
- Liu, Yulin et al. (2022) "Empirical Analysis of EIP-1559: Transaction Fees, Waiting Times, and Consensus Security" — Conference on Computer and Communications Security (CCS) 2022; candidate anchor for window-length 32 and per-priced-block update cadence
- Reijsbergen, Daniël et al. (2021) "Transaction Fees on a Honeymoon: Ethereum's EIP-1559 One Month Later" / (2025) follow-up — Advances in Financial Technologies (AFT) 2021; candidate anchor for window-length 32 and EIP-1559 short-term oscillation
- Leonardos, Stefanos et al. (2021) "Dynamical Analysis of the EIP-1559 Ethereum Fee Market" — AFT 2021; supplementary EIP-1559 controller analysis
- Optional 2024-2026 Ethereum gas-fee-market arXiv literature — open scope; planner cuts when marginal anchor unlikely

### ODD methodology reference (DOC-04)
- Grimm, Volker et al. (2006, 2010) "A standard protocol for describing individual-based and agent-based models" — Ecological Modelling; the foundational ODD framework; seven elements (Purpose, State variables, Process overview, Design concepts, Initialisation, Input data, Submodels)
- Grimm, Volker et al. (2020) "The ODD Protocol for Describing Agent-Based and Other Simulation Models: A Second Update to Improve Clarity, Replication, and Structural Realism" — JASSS; the current canonical ODD reference

### CIP precedent (paste-target shape)
- [CIP-0164 §"Trade-offs & Limitations"](https://cips.cardano.org/cip/CIP-0164) — closest in-Cardano disclosure-paragraph house-style precedent; DOC-01's "Recommended disclosure statements" and the register's `disclosure-paragraph` fields target this voice
- [`docs/phase-2/CPS-0023/README.md`](../../../docs/phase-2/CPS-0023/README.md) — Cardano Problem Statement 23, "Urgency Signaling"; the Cardano Problem Statement (CPS) the CIP responds to

### Codebase maps
- [`.planning/codebase/CONVENTIONS.md`](../../codebase/CONVENTIONS.md) — repo conventions; abbreviation-on-first-use is enforced by Phase 4 docs
- [`.planning/codebase/STRUCTURE.md`](../../codebase/STRUCTURE.md) — file-path landmarks DOC-04's worked example walks through

</canonical_refs>

<code_context>
## Existing Code Insights

This phase is documentation-only. No simulator code is created or modified. The "assets" are existing artefacts and the Phase 3 test outputs.

### Reusable Assets

- **Phase 1 register's RSK-NN entry shape** in [`docs/phase-2/realism-risks-register.md`](../../../docs/phase-2/realism-risks-register.md) — the structured fields (`id`, `title`, `category`, `description`, `evidence-for`, `evidence-against`, `scope-of-resolution`, `verdict`, `disclosure-paragraph`) are the canonical schema Phase 4 updates in place. Per Phase 1 D-05 / D-15 the RSK-NN identifier never renumbers.
- **Phase 2 coverage-check `signal-source-anchoring` enum** (`{mainnet-data-cited, spec-default, unanchored}`) in [`docs/phase-2/coverage-check.md`](../../../docs/phase-2/coverage-check.md) — DOC-03 flips `unanchored (RSK-…)` cells to `mainnet-data-cited (citation)` for any knob that acquires an anchor.
- **Existing audit annotation-banner pattern** in [`docs/phase-2/cardano-realism-audit.md`](../../../docs/phase-2/cardano-realism-audit.md) — the two banners (2026-05-13, 2026-05-14) demonstrate the audit's history; Phase 4 strips them rather than preserves them, but the pattern shows where the prior corrections landed and what content needs folding inline.
- **Existing per-suite trust block pattern** in [`docs/phase-2/validity-threats.md`](../../../docs/phase-2/validity-threats.md) — the 19 blocks share a per-suite "Trust" sub-section structure; DOC-02 adds the RSK cross-reference field to each block without restructuring.
- **CIP-pasteable disclosure paragraph pattern** in [`docs/phase-2/cardano-realism-audit.md`](../../../docs/phase-2/cardano-realism-audit.md) §"Recommended disclosure statements" — five paragraphs (fee-field semantics, controller calibration, topology, demand modelling, mempool sizing) in engineering-report voice; DOC-01 regenerates these against Phase 3 evidence.

### Established Patterns

- **Stable, append-only identifiers** for cross-document traceability (`RSK-NN`, `CLM-NN`, `EXP-NN`); Phase 4 never renumbers.
- **`(value, source, date-retrieved)` triple format** for calibration claims — the audit's existing values inherit this format in the rewrite; Phase 4's job is to apply the format uniformly.
- **Abbreviation-on-first-use** per CLAUDE.md §"Conventions / gotchas" — applies to all Phase 4 documents including DOC-04 (ODD = "Overview, Design concepts, Details", BCa = "Bias-corrected and accelerated", AFT = "Advances in Financial Technologies", CCS = "Conference on Computer and Communications Security", etc.).
- **Engineering-report voice** for CIP-pasteable prose — not academic-paper voice, not internal-process voice. Closest in-repo precedent is the existing audit's §"Recommended disclosure statements".

### Integration Points

- **Phase 1's register `disclosure-paragraph` fields** — Phase 4 writes verdict flips and refined disclosure-paragraph prose; the register file is updated in place. New verdicts: `RSK-pool-count` (LIVE → MITIGATED or LIVE → DISCLOSED gated on TEST-05), `RSK-steady-state-run-length` (similar, gated on TEST-06), `RSK-un-anchored-controller-knobs` (LIVE → MITIGATED if all four anchor; or LIVE → MITIGATED-on-some-DISCLOSED-on-others if partial).
- **Phase 2's coverage-check `signal-source-anchoring` column** — DOC-03 flips `unanchored` cells to `mainnet-data-cited` per anchor outcome. Existing CLM rows that referenced `unanchored (RSK-window-length-32)` etc. get their cell content updated; no new CLM rows are added.
- **Phase 3's test results** — read once at Phase 4 plan-open (Wave 1); narrated into DOC-01 disclosure prose and into the register's gated `disclosure-paragraph` fields in Wave 2.
- **Phase 5 inputs** — Phase 4 produces the four refreshed documents that Phase 5's `cip-author-summary.md` paste-guide cites by repo URL. The git tag (HAND-03) is applied at Phase 5 close; Phase 4 output must be in final form before tagging.

</code_context>

<specifics>
## Specific Ideas

- **DOC-04 worked example.** Trace seed=1 of a canonical menu-item job from `phase-2-priority-only-unreserved.yaml` or `phase-2-two-lane-both-dynamic.yaml` end-to-end through the seven ODD elements. The choice matches a Phase 3 TEST-04 canonical cell so the worked example walks through a CIP-cited result.
- **DOC-01 narrative integration of Phase 3 headline finding.** The un-reserved arms outperform single-lane EIP-1559; RB-reserved arms underperform. The refreshed §"Recommended disclosure statements" "On controller calibration" paragraph is rewritten to lead with this distinction. The pre-Phase-3 single-seed framing that "two-lane mechanisms outperform single-lane EIP-1559" is replaced with the more accurate "un-reserved two-lane mechanisms outperform single-lane EIP-1559 at N=20 seeds; RB-reserved two-lane mechanisms underperform under the same calibration".
- **DOC-01 cross-arm duplicate-job artefact disclosure.** The Phase 3 finding that partitioned ≡ RB-reserved welfare at `sundaeswap_moderate × multiplier_floor=4` replicates at N=20 is itself a CIP-disclose item. Lands in DOC-01's "Recommended disclosure statements" as a paragraph on calibration-conditional menu indistinguishability under specific demand profiles.
- **DOC-03 anchor-priority order.** Window-length 32 is the most-likely-to-anchor knob (Liu, Reijsbergen). Multiplier-floor 4 and multiplier-floor 16 plausibly anchor via spec rationale. Lane-signal-source likely stays disclosed (no external literature for the three-option choice). The planner exits the search per knob asymmetrically.
- **DOC-02 per-suite trust verdict reconciliation.** Where validity-threats currently has a per-suite trust verdict (HIGH / MEDIUM / LOW) that disagrees with the register's RSK-derived verdict, the matrix is refreshed against the register and the per-suite rationale updated. The four formerly-UNRESOLVED suites get refreshed verdicts derived from Phase 2's output-read.
- **Worked example seed choice.** Seed=1 is canonical across phase-2 (Phase 3 suite seeds use sequential `[1..N]`); the worked example tracks the same seed=1 the family-B results table and the Phase 3 BACKED rows already reference.

</specifics>

<deferred>
## Deferred Ideas

- **Re-running TEST-05 / TEST-06 inside Phase 4.** Already in flight via user-managed re-run; data available at Phase 4 commencement. If the re-run results are not in hand at plan-execute time, the planner falls back to disclose-only treatment per the existing register disclosure paragraphs.
- **600-pool / mainnet ~3,000-pool topology runs.** Out of scope per PROJECT.md; superseded by TEST-05 (100 vs 150). Disclosure paragraph in `RSK-pool-count` covers the extrapolation gap.
- **Cross-architecture continuous integration (CI) verification.** Out of scope per PROJECT.md; `RSK-cross-arch-determinism` carries the disclosure paragraph already.
- **Adversarial / strategic-bidder modelling.** Out of scope per PROJECT.md; `RSK-substrate-scope` carries the disclosure paragraph already (sub-point on utility-maximising actor model).
- **DOC-04 worked example for every menu option.** The default is one worked example for one (job, seed) pair. If the planner judges that the reader needs more than one example to understand the seven ODD elements, additional examples may be added; default scope is one.
- **Replacing the existing CIP-pasteable disclosure paragraphs in the audit with register `disclosure-paragraph` content verbatim.** Dual-purpose audit per D-39 keeps the audit's own prose; full deduplication against the register is not in Phase 4's scope.
- **`docs/phase-2/m6-implementation-plan.md` (CIP-0164 600-pool migration plan).** Out of scope per PROJECT.md; contingency only if TEST-05 surfaces a real gap.
- **CIP author summary.** Phase 5 / HAND-01 work; Phase 4 produces the inputs but does not author the summary.
- **Git tag at milestone close.** Phase 5 / HAND-03 work.

### Reviewed Todos (not folded)
No reviewed-todo deferrals — the `cross_reference_todos` step returned an empty matches set.

</deferred>

---

*Phase: 4-Refresh and Anchor*
*Context gathered: 2026-05-18*
