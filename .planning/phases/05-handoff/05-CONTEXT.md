# Phase 5: Handoff - Context

**Gathered:** 2026-05-18
**Status:** Ready for planning

<domain>
## Phase Boundary

Consolidate the Phase 1–4 evidence package into a single Cardano Improvement Proposal (CIP)-author paste guide, perform a final cross-document consistency review on top of Plan 04-07's Phase 4 pass, and tag the milestone-close commit citably as the reference the CIP cites by Uniform Resource Locator (URL).

Phase 5 produces three deliverables:

- `docs/phase-2/cip-author-summary.md` (HAND-01) — the paste guide for the CIP author. Hybrid shape: paste-target table mapping (CIP section → source artefact → specific paragraphs/rows), per-CIP-section recommendations, pinned references block (tag + commit + epoch-582 stake snapshot reference). Tiered inline-vs-reference treatment: inline the load-bearing items (substrate-scope umbrella; top-3-5 paste-order Limitations paragraphs; the headline Evidence claim mappings); reference-only for the long tail. Phase 5 derives the headline CIP claim list from Phase 3 / Phase 4 evidence.
- Reproducible consistency-verification script at `.planning/phases/05-handoff/verify-consistency.sh` (HAND-02) — four checks: (i) no dead Realism Risk identifier (RSK)-NN / Claim identifier (CLM)-NN / Experiment identifier (EXP)-NN cross-references across the six in-scope documents, (ii) every coverage-check `backing-job` path resolves to a (suite, job) entry in `sim-rs/parameters/phase-2-sweep/suites/`, (iii) every `golden-sha256` value matches a hash present in `sim-rs/parameters/phase-2-sweep/suites/.goldens/<suite>.sha256`, (iv) no broken markdown links. In-scope documents: the five CIP-cited artefacts (`cardano-realism-audit.md`, `validity-threats.md`, `realism-risks-register.md`, `coverage-check.md`, `methodology-overview.md`) plus the new `cip-author-summary.md`. Script + report (`.planning/phases/05-handoff/05-CONSISTENCY-REPORT.md`) commit together.
- Citable git tag at the milestone-close commit (HAND-03) — name `phase-2-cip-evidence-v1` per ROADMAP suggestion; annotated tag; tag message lists the citable artefact list, the 24 DISCLOSED + 0 LIVE + 0 MITIGATED + 0 DORMANT register distribution, and the epoch-582 snapshot reference. **Tag is user-executed**, not Claude-executed, per the don't-auto-commit memory; the cip-author-summary references the tag name as the citable reference the CIP author quotes.

Resolves all six remaining LIVE register entries to DISCLOSED before tag-time: `RSK-single-seed-precision`, `RSK-three-seed-statistical-power`, `RSK-unresolved-suite-claims`, `RSK-standard-user-fee-drift-exposure`, `RSK-menu-collapse-to-advocacy` (prose-promotion from fallback to load-bearing, integrating Phase 3 / Phase 4 evidence); `RSK-hash-diversity-policy` (cites the Phase 2 D-19 strict-gate locked rule as the gate semantics the CIP's BACKED rows meet). End-state register distribution at tag: 24 DISCLOSED + 0 LIVE + 0 MITIGATED + 0 DORMANT.

Requirements covered: HAND-01, HAND-02, HAND-03.

</domain>

<decisions>
## Implementation Decisions

### cip-author-summary.md shape (HAND-01)

- **D-44:** Shape is **hybrid**. Top section: a one-page paste-target table mapping (CIP section → source artefact → paste content). Middle section: per-CIP-section recommendations (Methodology, Limitations, Evidence, Calibration, Trust matrix) naming paste order and any caveats. Bottom section: pinned references block (tag name, commit Secure Hash Algorithm 256-bit (SHA-256), epoch-582 stake snapshot reference, retrieval date). Approximate authoring effort: 300–450 lines.

- **D-45:** Tiered inline-vs-reference treatment for paste content. **Inline verbatim**: the substrate-scope umbrella `disclosure-paragraph`, the top-3 to top-5 Limitations `disclosure-paragraph`s in their pre-decided paste order, the headline Evidence-section claim-to-CLM mappings. **Reference-only**: the long-tail Limitations paragraphs (each cited by RSK-NN identifier + source-file + line range), supporting CLM rows beyond the headline claims, calibration-triple references in `cardano-realism-audit.md`, validity-threats per-suite trust matrix. Rationale: gives the CIP author a single-pane-of-glass for the load-bearing paste targets while avoiding the transcription-drift risk a fully-inlined ~600-1000-line summary would carry.

- **D-46:** Phase 5 derives the **headline CIP claim list** from Phase 3 / Phase 4 evidence. The planner reads `.planning/phases/04-refresh-and-anchor/04-03-phase3-evidence-summary.md`, `.planning/realism-tests/multi-seed-variance/results.md`, `.planning/realism-tests/multiplier-floor-16-companion/results.md`, and `docs/phase-2/cardano-realism-audit.md` §"Recommended disclosure statements"; emits a list of 4–8 headline CIP claims (e.g. "un-reserved arms outperform single-lane Ethereum Improvement Proposal 1559 (EIP-1559) at N=20 Bias-corrected and accelerated (BCa) bootstrap 95% confidence interval"; "ranking-block-reserved (RB-reserved) arms underperform single-lane EIP-1559 under the same calibration"; "multiplier_floor=4 calibration is regime-dependent at floor=16"; "partitioned ≡ RB-reserved welfare at sundaeswap_moderate × multiplier_floor=4 replicates at N=20"); maps each headline claim to its backing CLM-NN row(s). The headline-claim list is committed as part of the summary; the CIP author may edit it before CIP submission.

### Six LIVE register entries → resolved to DISCLOSED at tag-time

- **D-47:** **All six remaining LIVE entries flip to DISCLOSED before HAND-03 tag.** No LIVE survives to the CIP. End-state register distribution: **24 DISCLOSED + 0 LIVE + 0 MITIGATED + 0 DORMANT**. Rationale per Phase 4 SUMMARY §"Open questions" item 5: the Phase 3 evidence at hand mostly moves the failure-mode hypothesis to "risk is bounded and disclosed" rather than to "risk is not real" (which would license MITIGATED). The five non-policy entries promote their existing draft fallback prose to load-bearing, rewritten to integrate Phase 3 / Phase 4 evidence. The one policy entry (RSK-hash-diversity-policy) cites the Phase 2 D-19 strict-gate rule.

- **D-48:** `RSK-hash-diversity-policy` flips LIVE → DISCLOSED citing the **strict gate** (Phase 2 D-19 + the Phase 3 `.planning/realism-tests/hash-diversity-gate/results.md` 17/17 BACKED-eligible cells passing). The `disclosure-paragraph` states: BACKED requires distinct `pricing_event_stream.sha256` count = `seeds-cited`; rows that collapse downgrade to WEAK with annotation, or are re-run with different seed values. This is the gate semantics every BACKED row in `coverage-check.md` meets.

- **D-49:** The other five LIVE entries (`RSK-single-seed-precision`, `RSK-three-seed-statistical-power`, `RSK-unresolved-suite-claims`, `RSK-standard-user-fee-drift-exposure`, `RSK-menu-collapse-to-advocacy`) flip LIVE → DISCLOSED via prose-promotion. The "(draft fallback; ...)" prefix is removed from each `disclosure-paragraph` field; the prose is light-touch refined to integrate Phase 3 / Phase 4 evidence (e.g. the Phase 3 TEST-03/TEST-04 N=20 BCa results for the two seed-precision entries; the Phase 2 output-read resolution of the four formerly-UNRESOLVED suites for `RSK-unresolved-suite-claims` and `RSK-standard-user-fee-drift-exposure`; the Phase 2 four-non-welfare-property-column structural mitigation for `RSK-menu-collapse-to-advocacy`).

### HAND-02 review scope and tooling

- **D-50:** HAND-02 consistency review **scope = the five Phase-4 CIP-cited documents + the new Phase-5 cip-author-summary.md** (six documents total). The supporting `.planning/` artefacts (spike READMEs, phase SUMMARYs, the family-b decision memos, the mechanism-welfare-impact memo) are out of HAND-02 scope. Rationale: those artefacts are append-only audit trail; the CIP does not paste from them, and Plan 04-07 already audited the five CIP-cited docs at Phase 4 close. Phase 5 re-runs the review after the six LIVE-entry flips and the new summary's cross-references.

- **D-51:** Tooling is a **reproducible verification script**, not ad-hoc grep. Four checks, each emitting structured output: (i) RSK-NN / CLM-NN / EXP-NN dead-reference scan across the six in-scope documents (every identifier referenced must resolve to a canonical definition site); (ii) `backing-job` path resolution against `sim-rs/parameters/phase-2-sweep/suites/<suite>.yaml`'s `jobs:` keys (every coverage-check `backing-job` cell must name a (suite, job) pair that exists); (iii) `golden-sha256` cross-check against `sim-rs/parameters/phase-2-sweep/suites/.goldens/<suite>.sha256` (every coverage-check truncated hash prefix must match a hash present in the corresponding .goldens file); (iv) markdown-link resolution (every `[label](path)` reference must resolve, internal links and external if locally cached). Rationale: a reproducible script lets CIP peer reviewers re-run the check independently, and gives future register / coverage-check edits continuous verification.

- **D-52:** Script location: **`.planning/phases/05-handoff/verify-consistency.sh`**. Phase-scoped to live with the Phase 5 artefacts; the cip-author-summary's pinned-references block points to it as the reproducer for the HAND-02 audit. Output: a structured `.planning/phases/05-handoff/05-CONSISTENCY-REPORT.md` (markdown table-per-check format matching Plan 04-07's consistency-report.md). Implementation language: shell (`bash`) with `grep`/`awk`/`yq` — no new repo-level dependency beyond what's already used by `sim-rs/scripts/`.

### Claude's Discretion

The following items have planner / executor latitude with reasonable defaults named here:

- **HAND-03 tag specifics.**
  - **Name:** `phase-2-cip-evidence-v1` per ROADMAP.md Phase 5 success criterion #3.
  - **Type:** annotated tag (`git tag -a phase-2-cip-evidence-v1 -m '...'`); annotated is the convention for citable references and the existing repo precedent (`m6-goldens-v1` is the only prior tag and is annotated).
  - **Message content:** lists the five CIP-cited artefacts by repo-relative path, the six-document scope of HAND-02, the post-Phase-5 register verdict distribution (24 DISCLOSED + 0 LIVE + 0 MITIGATED + 0 DORMANT), and the epoch-582 stake snapshot reference (retrieved 2026-05-14). Cap ~15 lines.
  - **Execution:** **the user runs the tag command**, not Claude, per the don't-auto-commit memory. The cip-author-summary references the tag name as the citable reference; if the tag command has not yet executed at Phase 5 close, the summary reads "(tag pending: `phase-2-cip-evidence-v1` to be applied to commit `<SHA>`)" and the user applies the tag separately.

- **Optional 2024–2026 arXiv follow-up pass for Plan 04-01 anchor search.** **Deferred by default**: Plan 04-01 exited at "marginal anchor unlikely" with one ANCHORED + three DISCLOSED sub-knobs; a re-pass with WebFetch / WebSearch access is unlikely to surface new anchors strong enough to flip a sub-knob from DISCLOSED to ANCHORED. If the user explicitly requests the re-pass during Phase 5 execution, the planner runs it as a Wave 1 task and folds any new anchors into `RSK-un-anchored-controller-knobs`. Default: skip; defer to post-CIP-feedback if peer review surfaces a missed citation.

- **TEST-05 / TEST-06 verdict-flip patch.** Per Phase 4 SUMMARY §"Open questions" items 1–2: if the user re-runs TEST-05 (pool-number sensitivity) or TEST-06 (run-length / steady-state) between Phase 4 close (2026-05-18) and Phase 5 tag, Phase 5 incorporates the data as a verdict-flip patch on the affected RSK entries (`RSK-pool-count`, `RSK-calibration-stale-stake-snapshot`, `RSK-steady-state-run-length`). The locked threshold "Δ% < seed-Inter-Quartile Range (IQR) of same job at 100 pools establishes MITIGATED" is preserved per REG-05; a MITIGATED flip increments the register's MITIGATED count and decrements the DISCLOSED count. Default: assume no re-run lands; the three entries remain DISCLOSED per Phase 4's disclose-only fallback.

- **Limitations paste-order on disclosure-paragraphs.** Default: substrate-scope umbrella (`RSK-substrate-scope`) leads; then category-grouped (external risks → construct risks → conclusion risks → internal risks per the Wohlin four-fold); within each category, ordered by load-bearing-ness (anchor-or-disclose entries before pure disclosures). Planner may reorder if a different reading flow serves the CIP better.

- **Headline Evidence-section CLM row count.** Default: 4–8 headline claims; ≤ 12 CLM rows cited inline as backing evidence. Planner expands if Phase 3 evidence licenses more headline claims or if the CIP outline (when authored) needs broader coverage.

- **CONSISTENCY-REPORT.md verbosity.** Default: structured tables per check (check name → result count → details); Plan 04-07's consistency-report.md is the template (220 lines, 8 audit sections). Phase 5's report covers four checks rather than eight, so should land ~120-180 lines.

- **HAND-01 forward-pointer disclosure if the 6 LIVE-entry flip surfaces unexpected blockers.** If a planner judges that a specific LIVE entry genuinely warrants staying LIVE (i.e., D-47's recommendation is wrong for that entry on closer reading), the planner surfaces this as an open-for-user-review item in the Phase 5 SUMMARY rather than silently flipping the verdict. Default: no surprises; D-47 holds for all six entries.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-level
- [`.planning/PROJECT.md`](../../PROJECT.md) — milestone context; the targeted-cheap-test pattern, the EIP-1559 demoted-to-control decision, the substrate-scope deferral, the Out-of-Scope list (no upstream re-audit, no new mechanism work, no cross-architecture CI)
- [`.planning/REQUIREMENTS.md`](../../REQUIREMENTS.md) — REQ-IDs covered by this phase: HAND-01, HAND-02, HAND-03
- [`.planning/ROADMAP.md`](../../ROADMAP.md) §"Phase 5: Handoff" — goal, dependencies (Phase 4), three success criteria; the suggested tag name `phase-2-cip-evidence-v1`
- [`CLAUDE.md`](../../../CLAUDE.md) §"Conventions / gotchas" — abbreviation-on-first-use rule (enforced across all Phase 5 documents); `.goldens/` directory layout

### Phase 4 outputs (the five CIP-cited documents the Phase 5 summary indexes)
- [`docs/phase-2/cardano-realism-audit.md`](../../../docs/phase-2/cardano-realism-audit.md) — 500 lines; 17 `(value, source, date-retrieved)` triples; §"Recommended disclosure statements" CIP-pasteable prose; substrate-scope umbrella at lines 335–349. Calibration-source-of-truth for the CIP's Calibration / Methodology mentions.
- [`docs/phase-2/validity-threats.md`](../../../docs/phase-2/validity-threats.md) — 850 lines; per-suite trust matrix across 19 suites; 19/19 `Related RSK:` + `Related CLM:` cross-references; aggregate trust 2 HIGH / 13 MEDIUM / 4 LOW / 0 UNRESOLVED. Trust-matrix-source-of-truth.
- [`docs/phase-2/realism-risks-register.md`](../../../docs/phase-2/realism-risks-register.md) — 452 lines; 24 RSK-NN entries; post-Phase-4 distribution 6 LIVE + 18 DISCLOSED, post-Phase-5 distribution 24 DISCLOSED per D-47. Disclosure-paragraph-source-of-truth (CIP Limitations section paste-targets).
- [`docs/phase-2/coverage-check.md`](../../../docs/phase-2/coverage-check.md) — 155 lines; ~25–40 CLM-NN rows (one per (claim, menu-option) pair); CLM-05 signal-source-anchoring cites Reijsbergen / Leonardos / Liu for window-length 32 anchor; the BACKED + WEAK rows are the CIP Evidence-section paste-targets.
- [`docs/phase-2/methodology-overview.md`](../../../docs/phase-2/methodology-overview.md) — 260 lines; Overview, Design concepts, Details (ODD) index + per-element prose + worked example at `menu_unreserved_priority_only_static_x4` seed=1. Methodology-source-of-truth (CIP Methodology section cites this by Uniform Resource Locator (URL)).

### Phase 4 supporting artefacts
- [`.planning/phases/04-refresh-and-anchor/04-SUMMARY.md`](../04-refresh-and-anchor/04-SUMMARY.md) — Phase 4 close-state; §"Phase 5 inputs" enumerates the four refreshed CIP-cited documents + the new methodology-overview; §"Open questions for Phase 5" raises the five questions Phase 5 disposes of (D-47 through D-52 cover items 3 and 5; items 1, 2, 4 are Claude's Discretion deferrals above)
- [`.planning/phases/04-refresh-and-anchor/04-VERIFICATION.md`](../04-refresh-and-anchor/04-VERIFICATION.md) — Phase 4 verifier output (PASS 4/4 success criteria); evidence baseline Phase 5 HAND-02 starts from
- [`.planning/phases/04-refresh-and-anchor/04-07-consistency-report.md`](../04-refresh-and-anchor/04-07-consistency-report.md) — Plan 04-07's 8-section consistency audit + 8-defects-fixed-in-place log; the template for Phase 5's `05-CONSISTENCY-REPORT.md`
- [`.planning/phases/04-refresh-and-anchor/04-03-phase3-evidence-summary.md`](../04-refresh-and-anchor/04-03-phase3-evidence-summary.md) — Phase 3 evidence consolidated for Phase 4 narrative; the headline two-bullet finding ("un-reserved menu arms outperform single-lane EIP-1559; RB-reserved menu arms underperform") sources the Phase 5 headline-CIP-claim list per D-46
- [`.planning/phases/04-refresh-and-anchor/04-01-DOC-03-anchor-search.md`](../04-refresh-and-anchor/04-01-DOC-03-anchor-search.md) — Plan 04-01's per-sub-knob anchor decisions + rejected-citations list; reference if the user opts into the deferred 2024–2026 arXiv re-pass

### Phase 3 test results (sourced by D-46 headline-claim derivation and by D-49 prose-promotion edits)
- [`.planning/realism-tests/multi-seed-variance/results.md`](../../realism-tests/multi-seed-variance/results.md) — TEST-03 + TEST-04 N=20 BCa intervals; the nine cell results that license the un-reserved-vs-RB-reserved welfare distinction
- [`.planning/realism-tests/multiplier-floor-16-companion/results.md`](../../realism-tests/multiplier-floor-16-companion/results.md) — TEST-07a regime-dependence finding (multiplier_floor=4 vs floor=16); the multiplier_floor regime-dependence headline claim
- [`.planning/realism-tests/hash-diversity-gate/results.md`](../../realism-tests/hash-diversity-gate/results.md) — 17/17 BACKED-eligible cells pass distinct-hash test; the source of D-48's strict-gate disclosure-paragraph for `RSK-hash-diversity-policy`
- [`.planning/realism-tests/pool-number-sensitivity/results.md`](../../realism-tests/pool-number-sensitivity/results.md) — TEST-05 partial coverage (35/1650 ≈ 2.1%); if user re-runs before tag, Phase 5 incorporates as verdict-flip patch (Claude's Discretion)
- [`.planning/realism-tests/run-length-steady-state/results.md`](../../realism-tests/run-length-steady-state/results.md) — TEST-06 partial coverage (31/120 ≈ 26%, 1 of 4 menu arms); same conditional-patch path

### Phase 1 / Phase 2 / Phase 3 / Phase 4 CONTEXT.md chain (decision provenance)
- [`.planning/phases/04-refresh-and-anchor/04-CONTEXT.md`](../04-refresh-and-anchor/04-CONTEXT.md) — D-38 (audit dual-purpose), D-39 (audit "Recommended disclosure statements" regen), D-40 (validity-threats per-suite refresh), D-41 (methodology-overview location), D-42 (ODD index + per-element prose + worked example shape)
- [`.planning/phases/03-targeted-cheap-tests/03-CONTEXT.md`](../03-targeted-cheap-tests/03-CONTEXT.md) — TEST-NN shape and N-determination
- [`.planning/phases/02-coverage-check-skeleton/02-CONTEXT.md`](../02-coverage-check-skeleton/02-CONTEXT.md) — D-13 (per-(claim, menu-option) row shape), D-14 (non-welfare property column enum vocabulary), D-16 (CLM verdict vocabulary), D-19 (strict hash-diversity gate semantics — the rule D-48 cites)
- [`.planning/phases/01-register-inventory/01-CONTEXT.md`](../01-register-inventory/01-CONTEXT.md) — D-05 / D-15 (RSK-NN append-only never-renumber), D-06 (LIVE/DORMANT/MITIGATED/DISCLOSED vocabulary), D-08 (RSK required fields schema)

### Suite + goldens (HAND-02 cross-check targets)
- [`sim-rs/parameters/phase-2-sweep/suites/`](../../../sim-rs/parameters/phase-2-sweep/suites/) — 25 suite Yet Another Markup Language (YAML) files; HAND-02 check (ii) resolves every coverage-check `backing-job` cell against the `jobs:` keys of the named suite YAML
- [`sim-rs/parameters/phase-2-sweep/suites/.goldens/`](../../../sim-rs/parameters/phase-2-sweep/suites/.goldens/) — seven `.sha256` files (one per goldens-pinned suite); HAND-02 check (iii) resolves every coverage-check `golden-sha256` truncated prefix against the contents

### CIP precedent and Cardano Problem Statement (CPS) reference
- [`docs/phase-2/CPS-0023/`](../../../docs/phase-2/CPS-0023/) — the Cardano Problem Statement the CIP responds to; the substantive framing the cip-author-summary's per-section recommendations anchor against
- CIP-0164 §"Trade-offs & Limitations" — closest in-Cardano disclosure-paragraph house-style precedent (referenced via [`docs/phase-2/cardano-realism-audit.md`](../../../docs/phase-2/cardano-realism-audit.md))

### Codebase maps
- [`.planning/codebase/CONVENTIONS.md`](../../codebase/CONVENTIONS.md) — repo conventions; abbreviation-on-first-use enforcement
- [`.planning/codebase/STRUCTURE.md`](../../codebase/STRUCTURE.md) — file-path landmarks the HAND-02 verification script walks
- [`.planning/codebase/CONCERNS.md`](../../codebase/CONCERNS.md) — known concerns; cross-references the substrate-scope and cross-architecture-CI disclosures already in the register

</canonical_refs>

<code_context>
## Existing Code Insights

This phase is **documentation-only** plus one shell script (the HAND-02 consistency verifier). No simulator code is created or modified.

### Reusable Assets

- **Phase 4 / Plan 04-07's consistency-report.md template** ([`.planning/phases/04-refresh-and-anchor/04-07-consistency-report.md`](../04-refresh-and-anchor/04-07-consistency-report.md)) — 220 lines, eight audit sections with per-section verdict tables. The Phase 5 `05-CONSISTENCY-REPORT.md` reuses this format reduced to four sections (the four checks in D-51).
- **Phase 4 register's disclosure-paragraph schema** in [`docs/phase-2/realism-risks-register.md`](../../../docs/phase-2/realism-risks-register.md) — engineering-report voice, CIP-pasteable, abbreviations expanded on first use, references named in-paragraph. The D-49 prose-promotion edits stay inside this schema; only the "(draft fallback; ...)" prefix is removed and prose lightly refined.
- **Existing `sim-rs/scripts/run-phase-3-suites.sh`-style shell idiom** (bash + grep/awk; standard POSIX tooling; no new dependencies). The HAND-02 verification script follows this idiom.
- **`.goldens/<suite>.sha256` file format** (one hash per line, plain text). HAND-02 check (iii) is a straightforward grep-and-compare against the truncated prefix in coverage-check.md's `golden-sha256` cells.
- **Existing tag annotation style** from `m6-goldens-v1` (the only prior tag; annotated; short message naming the milestone). HAND-03 tag annotation follows this precedent.

### Established Patterns

- **Stable, append-only identifiers** (RSK-NN, CLM-NN, EXP-NN) — Phase 5 never renumbers; HAND-02 check (i) enforces.
- **Abbreviation-on-first-use** per CLAUDE.md §"Conventions / gotchas" — applies to all Phase 5 documents including the cip-author-summary (which sees many fresh abbreviations: CIP, ODD, BCa, IQR, RSK, CLM, EXP, SHA-256, URL, AFT, CCS, SODA, EIP-1559).
- **Engineering-report voice** for CIP-pasteable prose — the existing register `disclosure-paragraph` fields and the audit's §"Recommended disclosure statements" set the voice the cip-author-summary inlines verbatim or by reference.
- **Phase-scoped script + report co-location** — the HAND-02 script + report co-locate at `.planning/phases/05-handoff/` for discoverability and for the cip-author-summary's pinned-references-block link to be stable.

### Integration Points

- **`docs/phase-2/realism-risks-register.md`** — six entries (D-47) get verdict flips LIVE → DISCLOSED + `disclosure-paragraph` edits. The `Index` table at the file head needs verdict-column updates for these six rows. The file's reading-guide preamble needs its post-Phase-5 distribution sentence updated from "6 LIVE + 18 DISCLOSED" to "0 LIVE + 24 DISCLOSED".
- **`docs/phase-2/cip-author-summary.md`** — new file; created by HAND-01. Lives alongside the other CIP-cited artefacts under `docs/phase-2/`.
- **`.planning/phases/05-handoff/verify-consistency.sh`** — new script (HAND-02 tooling); ~100–250 lines of bash. Future CIP reviewers re-run it against any subsequent register / coverage-check edits.
- **`.planning/phases/05-handoff/05-CONSISTENCY-REPORT.md`** — new file; output of the verify-consistency.sh run; committed alongside the script and the cip-author-summary.
- **Git tag `phase-2-cip-evidence-v1`** — applied by the user (per Claude's Discretion D-tag) to the milestone-close commit on the `dynamic-experiment` branch.

</code_context>

<specifics>
## Specific Ideas

- **Phase 4 SUMMARY's "Phase 5 inputs" enumeration** is the seed for the cip-author-summary's paste-target table: five CIP-cited documents + register's 18-now-24 DISCLOSED disclosure-paragraphs + Plan 04-01 anchor-or-disclose audit trail + Plan 04-03 Phase 3 evidence summary + Plan 04-07 consistency report.
- **Strict-gate citation prose for `RSK-hash-diversity-policy`** can quote the Phase 2 D-19 rule verbatim: "BACKED requires distinct `pricing_event_stream.sha256` count = `seeds-cited`; rows whose seed-set collapses to fewer distinct hashes are downgraded to WEAK with annotation, or are re-run with different seed values." The Phase 3 hash-diversity-gate results.md provides the load-bearing evidence (17/17 cells passed).
- **Headline-claim list (D-46) draft from Phase 3 evidence** (4 claims minimum, planner refines / extends):
  1. "Un-reserved two-lane mechanisms outperform single-lane Ethereum Improvement Proposal 1559 (EIP-1559) on welfare at N=20 seeds with Bias-corrected and accelerated (BCa) bootstrap 95% confidence intervals" → backed by CLM rows for the two un-reserved menu options (`priority-only-un-reserved`, `both-dynamic-un-partitioned`) plus the single-lane control
  2. "Ranking-block-reserved (RB-reserved) two-lane mechanisms underperform single-lane EIP-1559 on welfare under the same calibration" → backed by CLM rows for the two RB-reserved menu options
  3. "The multiplier_floor=4 calibration is regime-dependent: at multiplier_floor=16 the `phase-2-rb-scarcity` welfare finding inverts and the `phase-2-urgency-inversion` finding weakly reverses" → backed by TEST-07a CLM row(s)
  4. "Partitioned and RB-reserved mechanisms produce indistinguishable welfare at `sundaeswap_moderate × multiplier_floor=4`; the indistinguishability replicates at N=20" → backed by TEST-04 canonical-cells CLM row
- **Limitations paste-order draft** (Claude's Discretion default; planner reorders if needed):
  1. `RSK-substrate-scope` (umbrella; sets the substrate boundary)
  2. `RSK-cross-arch-determinism` (the substrate-scope's cross-architecture corollary)
  3. `RSK-leios-spec-pre-deployment` (the substrate-scope's specification-immaturity corollary)
  4. `RSK-pool-count` + `RSK-calibration-stale-stake-snapshot` (topology / calibration freshness)
  5. `RSK-steady-state-run-length` (simulation run-length boundary)
  6. `RSK-fee-as-maxFee-envelope` + `RSK-mempool-cap-magnitude` + `RSK-max-fee-policy-default` + `RSK-target-inclusion-blocks-default` (semantic-reinterpretation cluster)
  7. `RSK-demand-mix-bit-calibration` + `RSK-demand-non-stationarity` + `RSK-sundaeswap-demand-staleness` (demand-modelling cluster)
  8. `RSK-un-anchored-controller-knobs` with the four sub-knob paragraphs (controller-knob anchor-or-disclose)
  9. `RSK-multiplier-floor-4-suite-coverage` (suite-coverage corollary of un-anchored knobs)
  10. `RSK-partition-activated-honest-producer` (honest-producer-assumption boundary)
  11. `RSK-admission-rejection-attribution` + `RSK-welfare-as-f64-reporting` (reporting-precision cluster)
  12. The five new-to-DISCLOSED entries from D-49 (the seed-precision pair + the three Phase-2 output-read-resolved entries)
  13. `RSK-hash-diversity-policy` (gate-semantics; closes the Limitations section by reinforcing that the BACKED claims meet a known rule)
- **HAND-02 verification script output format** — markdown table per check, similar to Plan 04-07 §"Cross-reference integrity":

  ```
  | Check | Expected | Found | Status |
  |-------|----------|-------|--------|
  | RSK-NN dead refs | 0 | 0 | PASS |
  | CLM-NN dead refs | 0 | 0 | PASS |
  | backing-job paths | N | N | PASS |
  | golden-sha256 matches | M | M | PASS |
  | broken markdown links | 0 | 0 | PASS |
  ```

- **Tag-message draft** (~12 lines):

  ```
  phase-2-cip-evidence-v1 — milestone-close tag

  Citable reference for the Cardano Improvement Proposal (CIP)
  responding to CPS-0023 ("Urgency Signaling").

  CIP-cited artefacts:
  - docs/phase-2/cardano-realism-audit.md
  - docs/phase-2/validity-threats.md
  - docs/phase-2/realism-risks-register.md (24 DISCLOSED + 0 LIVE)
  - docs/phase-2/coverage-check.md
  - docs/phase-2/methodology-overview.md
  - docs/phase-2/cip-author-summary.md (paste guide)

  Topology snapshot: Cardano mainnet, epoch 582, retrieved 2026-05-14.
  Consistency audit: .planning/phases/05-handoff/05-CONSISTENCY-REPORT.md
  ```

</specifics>

<deferred>
## Deferred Ideas

- **Optional 2024–2026 arXiv follow-up pass for Plan 04-01 anchor search.** Captured as Claude's Discretion (default: skip; defer to post-CIP-feedback). Plan 04-01 already exited at "marginal anchor unlikely" with one ANCHORED + three DISCLOSED sub-knobs.
- **TEST-05 / TEST-06 verdict-flip patches.** Captured as Claude's Discretion (default: assume no re-run lands before tag; three RSK entries stay DISCLOSED). If user-managed re-runs complete before tag, fold opportunistically.
- **Adversarial / strategic-bidder modelling.** Out of scope per `.planning/PROJECT.md`; `RSK-substrate-scope` carries the disclosure paragraph already (sub-point on utility-maximising actor model).
- **600-pool / ~3,000-pool topology runs (CIP-0164 / mainnet pool-count regime).** Out of scope per PROJECT.md; superseded by TEST-05's pool-number sensitivity test (partial coverage at 100 vs 150 pools). `RSK-pool-count`'s disclosure-paragraph names the extrapolation gap.
- **Cross-architecture continuous integration (CI) verification.** Out of scope per PROJECT.md; `RSK-cross-arch-determinism` carries the disclosure paragraph already.
- **CIP draft itself.** Out of scope per PROJECT.md and REQUIREMENTS.md "Out of Scope". Phase 5 produces the paste guide; the user authors the CIP from the paste guide and the five CIP-cited artefacts.
- **Cross-ref index automation script generalised across phases.** The HAND-02 verification script in D-52 is phase-scoped; a more-general `.planning/scripts/` directory and a parametric script that works for any future milestone is a "if we do another CIP evidence audit" enhancement, not Phase 5 scope.
- **Promotion of any unpinned demand-regime suite to goldens-pinned.** Out of scope per REQ-COV-04 and PROJECT.md (carried forward from Phase 2).
- **Re-running the Phase 4 consistency review on the supporting `.planning/` artefacts (spike READMEs, family-b memos, mechanism-welfare-impact memo).** Out of HAND-02 scope per D-50. Those artefacts are append-only audit trail and are not pasted into the CIP.
- **Promotion of `docs/phase-2/m6-implementation-plan.md` (CIP-0164 600-pool migration plan) into Phase 5 outputs.** Out of scope per PROJECT.md; the file stays in tree as a contingency document the CIP author may reference if pool-number sensitivity is challenged post-publication.

### Reviewed Todos (not folded)

No reviewed-todo deferrals — the `cross_reference_todos` step returned an empty matches set (`todo_count: 0`).

</deferred>

---

*Phase: 5-Handoff*
*Context gathered: 2026-05-18*
