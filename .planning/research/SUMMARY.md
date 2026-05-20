# Project Research Summary — Phase-2 CIP Evidence Audit

**Project:** Phase-2 CIP Evidence Audit
**Domain:** CIP-grade evidence package for a menu-style mechanism proposal responding to [CPS-0023](../../docs/phase-2/CPS-0023/README.md) ("Urgency Signaling")
**Researched:** 2026-05-15
**Confidence:** MEDIUM-HIGH

---

## TL;DR

- The register must be built before the coverage check, and both before cheap tests: the `scope-of-resolution` field in each `RSK-*` entry disciplines test design and prevents the "looks mitigated, isn't" failure mode (PITFALLS MOD-5).
- The top publication-blocking risk is single-seed claims at precision-grade confidence (CRIT-1); multi-seed variance bands resolve it and should run immediately after the register draft.
- **ID convention resolved:** use Leios-style `RSK-*`/`EXP-*` for the register and a sibling `CLM-*` namespace for coverage claims — Cardano-native, grep-able, precedented in [`docs/ImpactAnalysis.md`](https://github.com/input-output-hk/ouroboros-leios/blob/main/docs/ImpactAnalysis.md). ARCHITECTURE.md's `RRR-NN`/`CLM-NN` proposal is functionally identical but `RSK-*` imports ecosystem recognition; drop `RRR-NN`.
- The one bounded code addition is `sim-cli/src/metrics/paired_bootstrap.rs` (~150 LoC, no new crate dependencies); everything else is Markdown artefacts and simulator re-runs.

---

## Executive Summary

This milestone produces the evidence base for a CIP responding to CPS-0023. The codebase is mature: M1–M5 complete, Family B committed 2026-05-14, seven goldens-pinned suites characterising the four two-lane variants. The audit's job is not to extend the simulator but to surface, structure, and stress-test the claims those suites support — closing the gap between "these suites run and pass" and "a CIP reviewer can independently audit each menu-item trade-off claim against a specific reproducible job."

The recommended approach follows a five-phase build order grounded in three established traditions: the Leios `RSK-*`/`EXP-*` register pattern (Cardano-native), the EIP-1559 empirical-analysis tradition (claims-stratified welfare analysis with named impossibility properties), and Wohlin's four-fold validity taxonomy (construct / internal / external / conclusion). Phase 1 builds the register first because its `scope-of-resolution` field is the discipline that keeps cheap tests honest. Phase 2 builds the coverage-check skeleton because it surfaces which claims need backing before compute is spent. Phases 3–5 execute tests, refresh existing documents, and produce the handoff artefacts the CIP author pastes directly.

The key risks are methodological, not implementation. Single-seed welfare claims that invert near zero (CRIT-1) could invalidate the RB-reserved and partitioned arm comparisons. Menu framing that reports only welfare metrics will collapse a four-option menu into de-facto advocacy for un-reserved-both-dynamic (CRIT-2). Four controller knobs — window-length 32, multiplier-floor 4, multiplier-floor 16, lane-signal-source choices — have no deployed-system anchor and must either find one or carry explicit CIP-disclosure paragraphs (CRIT-3). All three are resolvable within milestone scope with the right sequencing.

---

## Key Findings

### Recommended Stack

No new Rust dependencies are needed. `statrs 0.18` (already in `sim-cli/Cargo.toml`) covers distributions; `vergen-gitcl` (already integrated) provides build-time git-SHA provenance; the three-layer determinism regime is the reproducibility spine. The one bounded addition is `sim-cli/src/metrics/paired_bootstrap.rs` (~150 LoC): a paired-sample bootstrap BCa implementation for welfare-delta confidence intervals. This is the right implementation site — it keeps the determinism contract under repo control and avoids pulling in `stats-ci`'s transitive dependencies.

For statistical methodology, the simulator's intra-architectural determinism makes Paired Seed Evaluation (PSE) with paired-bootstrap BCa the correct choice over independent-seed t-tests. When two mechanism variants run on the same seed, all stochastic confounders are shared; the welfare delta isolates the mechanism effect (Glasserman & Yao classical CRN result, formalised for learning-based simulators by Sharma 2025). BCa over N=30 seeds is the primary target; N=20 is the minimum for at-nominal bootstrap coverage; claims at N<20 must be flagged as single-anecdote only (Pustejovsky simulations).

**Core methodology choices:**
- Risk-register format: Leios `RSK-*` / `EXP-*` IDs + verdict column (LIVE / DORMANT / MITIGATED / DISCLOSED) — Cardano-native, Wohlin four-fold categories
- Coverage-check format: RTM-style Markdown table, `CLM-*` IDs, columns: claim, menu option, backing suite, backing job, seeds, CI/method, golden SHA, verdict (BACKED / WEAK / UNBACKED / OUT-OF-SCOPE)
- Variance method: Paired Seed Evaluation, paired-bootstrap BCa 95% CI, N=30 seeds, B=10,000 replicates
- Disclosure format: per-RSK paragraph block, CIP-ready prose (not bullet caveats), matching the [`validity-threats.md`](../../docs/phase-2/validity-threats.md) per-suite trust-rating idiom
- Provenance: `vergen-gitcl` git SHA per artefact, exact `cargo run --release --bin experiment-suite -- verify <suite>.yaml` reproduction recipe per claim

### Expected Features

**Must have (table stakes — reviewer expects; absence blocks CIP progression):**
- `CLM-*` claim → backing job mapping table — not yet built; the central deliverable
- Variance bands per claim (BCa CI or at minimum median + IQR), not point estimates — all current evidence is seed=1 or 3-seed with no CI
- Calibration provenance per parameter: `(value, source, date-retrieved)` triples — mostly in [`cardano-realism-audit.md`](../../docs/phase-2/cardano-realism-audit.md); needs authoritative rewrite
- Consolidated disclosure section (one CIP-citeable section) — content exists scattered across `cardano-realism-audit.md`, `validity-threats.md`, `REVIEW.md`; not consolidated
- Reproducibility recipe exposing the existing 3-layer determinism to a first-time CIP reader — exists in `CLAUDE.md`; needs reader-facing distillation
- Realism-risks register (`RSK-*` IDs, verdict column) — the user's own framing; not yet built
- Comparator baseline per menu option (EIP-1559 control contrast table) — suites already include both; contrast table not assembled

**Should have (differentiators that materially raise trust above floor):**
- Counterfactual welfare vs flat-fee baseline — `baseline_flat_fee` data in every suite; contrast table not assembled (near-zero cost, strongly recommended)
- Mechanism-independence claim with explicit boundary — Family B vs accumulator data in [`mechanism-welfare-impact-2026-05-14.md`](../mechanism-welfare-impact-2026-05-14.md); not yet lifted into a CIP-citeable robustness table (strongly recommended)
- Pool-number sensitivity test (100 vs 150 pools × 5 demand profiles) — Active in PROJECT.md; preempts the most-likely reviewer objection at low compute cost
- Run-length / steady-state validation — diagnostic time-series check; Active in PROJECT.md; cheap

**Defer (out of scope per PROJECT.md):**
- Adversarial / strategic-bidder modelling — disclosed as future work; cite Chung & Shi SODA 2023 for the incentive/collusion frame
- Cross-architecture CI verification — intra-arch determinism sufficient; CR-1 caveat disclosed
- Deployed-EIP-1559 baseline comparison — pleasant but not load-bearing; flat-fee counterfactual is the right comparator

**Anti-features to avoid explicitly:**
- Summary statistics without uncertainty quantification
- Single-seed claims presented as conclusive
- Coverage check as narrative prose (hides gaps; forces re-derivation)
- Risk register as unstructured prose (cannot be audited or CIP-cited)
- Disclosure as hedge-language rather than named non-property under a named condition

### Architecture Approach

The evidence-base artefacts sit on top of the simulator as lightly cross-linked Markdown documents in `.planning/` and `docs/phase-2/`. The simulator codebase under `sim-rs/` is read-only substrate for this milestone (with the two additive exceptions: new suite YAML fragments for cheap tests, and the `paired_bootstrap.rs` module).

**Major artefact components:**
1. `docs/phase-2/realism-risks-register.md` (NEW) — single source of truth; `RSK-*` IDs; status field is the only mutable bit per entry; CIP-ready disclosure framing per entry
2. `docs/phase-2/coverage-check.md` (NEW) — `CLM-*` × (menu option, claim, suite, job, seeds, method, golden SHA, status, risks); flat table; gaps surface as empty cells
3. `.planning/realism-tests/<name>/` (NEW folder) — one subfolder per LIVE-risk targeted test; three files max: `README.md` (design + threshold), `results.md` (verdict), `jobs.yaml` (runnable suite fragment if needed)
4. `docs/phase-2/cardano-realism-audit.md` (EXISTING — refresh) — drop annotation banners; fold into authoritative voice; CIP-pasteable substrate-scope paragraph added
5. `docs/phase-2/validity-threats.md` (EXISTING — refresh) — fold per-suite trust verdicts under `RSK-*` scheme

**Key architectural constraint:** cheap-test artefacts must NOT live in `sim-rs/output/` (ephemeral, uncommitted). The durable evidence is `results.md` in the per-test subfolder; the full `output/<run-id>/` directory is regenerable ground truth.

### Critical Pitfalls

1. **CRIT-1: Single-seed welfare claims at publication precision** — Four flip cells (`d4_t50_w32`, `d8_t25_w32`, `x4_rb_quarter` under both rb-reserved and partitioned arms) sit close to zero in absolute welfare and could plausibly invert at different seeds. Prevention: PSE BCa CIs at N≥20 before any CIP claim cites these cells; treat any welfare claim with absolute delta under ~10× per-seed IQR as conditional.

2. **CRIT-2: Menu collapsing to advocacy** — un-reserved-both-dynamic dominates on every welfare metric in the published results table. A coverage check that reports only welfare metrics turns the four-option menu into a single-recommendation CIP in disguise. Prevention: the `CLM-*` table must include non-welfare property columns (anti-bribery: formal/informal/absent; standard-user-fee-drift-exposure; signal-source-anchoring; implementation complexity). Each cell cites spec section, simulator measurement, or "disclosed gap."

3. **CRIT-3: Un-anchored controller knobs** — window-length 32, multiplier-floor 4, multiplier-floor 16, and both lane-signal-source choices have no deployed-system anchor. Each needs either an external data citation or a CIP-disclosure paragraph of the form "we chose X; qualitative findings are conditional on X; alternative Y was not exercised."

4. **CRIT-4: Inherited-substrate scope not in CIP-readable form** — the Leios simulator's f64 in non-pricing hot paths, propagation-model fidelity, and utility-maximising actor model are documented in `.planning/` but not accessible to a CIP reader. Prevention: one "Substrate scope" paragraph (CIP-pasteable) in the register; cross-referenced from the CIP's Limitations section.

5. **CRIT-5: Calibration-stale parameters** — epoch-582 stake snapshot (2026-05-14), Q1 2026 demand profiles. Prevention: `(value, source, date-retrieved)` triples throughout the refreshed audit; pool-number sensitivity test provides a bounded argument the topology is not load-bearing in the 100–150 pool range.

---

## Implications for Roadmap

### Recommended Roadmap Shape

Five phases, each consuming specific PROJECT.md Active items:

| Phase | Name | Active Items | Key Pitfalls Resolved |
|-------|------|-------------|----------------------|
| 1 | Register Inventory | Item 1 | CRIT-5, MOD-3, MOD-5 |
| 2 | Coverage-Check Skeleton | Item 4 | CRIT-2, MOD-7, MIN-3 |
| 3 | Targeted Cheap Tests | Items 2, 3, 7, 8 | CRIT-1, MOD-1, MOD-2, MOD-4 |
| 4 | Refresh and Anchor | Items 5, 6 | CRIT-3, CRIT-4, MOD-6, MIN-1, MIN-2 |
| 5 | Handoff | (closes all prior) | MIN-4 |

---

### Phase 1: Register Inventory

**Rationale:** The register is upstream of everything else. The `scope-of-resolution` field on each `RSK-*` entry is what keeps cheap tests honest — without it, Phase 3 produces "looks mitigated, isn't" results (MOD-5). This is the founding discipline of the entire evidence package.

**Delivers:** `docs/phase-2/realism-risks-register.md` — complete inventory sweeping all existing artefacts (`cardano-realism-audit.md`, `validity-threats.md`, `CONCERNS.md`, `REVIEW.md`, `mechanism-welfare-impact-2026-05-14.md`, seven spike READMEs); de-duplicated into `RSK-*` entries; initial status (LIVE / DORMANT / MITIGATED / DISCLOSED) from existing evidence only; each LIVE entry flagged for Phase 3 with a hypothesis statement and explicit scope-of-resolution field.

**Addresses:** Active item 1; resolves CRIT-5, MOD-3, MOD-5

**Avoids:** Running cheap tests before knowing which claims they need to resolve

---

### Phase 2: Coverage-Check Skeleton

**Rationale:** The coverage check surfaces which claims have backing before compute is spent. Gaps are either Phase 3 work items or DISCLOSED `RSK-*` entries. Can begin at the Phase 1 tail once the first tranche of entries is stable.

**Delivers:** `docs/phase-2/coverage-check.md` — flat `CLM-*` table; each row = one menu-item trade-off claim (welfare AND non-welfare properties); backing suite + job + seeds + golden SHA; status (BACKED / WEAK / UNBACKED / OUT-OF-SCOPE); `RSK-*` risks affecting each claim. Gaps surface as empty backing cells.

**Addresses:** Active item 4; resolves CRIT-2 (non-welfare property columns force the menu to remain a menu), MOD-7, MIN-3

**Avoids:** Implicit menu-to-suite translation that forces reviewers to reconstruct mappings and risk misalignment

---

### Phase 3: Targeted Cheap Tests

**Rationale:** With register entries scoped and coverage gaps identified, targeted tests run against named hypotheses. Multi-seed variance bands are the highest-priority item in this phase because they resolve CRIT-1 and may flip coverage-check rows from WEAK to BACKED.

**Delivers (in priority order):**
- Multi-seed variance bands — four flip cells + canonical menu-item jobs at N=30 seeds; PSE BCa CIs; `sim-cli/src/metrics/paired_bootstrap.rs` (~150 LoC, no new deps); results at `.planning/realism-tests/multi-seed-variance/`
- Pool-number sensitivity test — 33-job smoke × {100, 150 pools} × {sundaeswap_moderate + 4 paper_like variants}; results at `.planning/realism-tests/pool-number-sensitivity/`
- Run-length / steady-state validation — one canonical job per arm at 2× and 4× run length; results at `.planning/realism-tests/run-length-steady-state/`
- Additional LIVE-risk tests surfaced by Phase 1 (3–5 anticipated; list emerges from register)

**Addresses:** Active items 2, 3, 7, 8; resolves CRIT-1, MOD-1, MOD-2, MOD-4, MOD-5 pattern validated

**Note on code addition:** `paired_bootstrap.rs` is the one bounded exception to the "simulator code is read-only" principle. It is additive — no mechanism code changes, no impact on existing golden hashes.

---

### Phase 4: Refresh and Anchor

**Rationale:** Refresh after tests have updated register entries so the authoritative documents are consistent with final verdicts. The four unanchored controller knobs are addressed here; some may acquire external anchors from a targeted literature search (Liu et al. CCS 2022, Reijsbergen AFT 2021 for Ethereum window-length and step-size data) — a 2-hour search at the start of Phase 4 is worth doing before defaulting to disclosure.

**Delivers:**
- Refreshed [`docs/phase-2/cardano-realism-audit.md`](../../docs/phase-2/cardano-realism-audit.md) — authoritative voice; `(value, source, date-retrieved)` triples; CIP-pasteable substrate-scope paragraph
- Refreshed [`docs/phase-2/validity-threats.md`](../../docs/phase-2/validity-threats.md) — cross-referenced to `RSK-*` scheme; consistent with register
- Anchoring or disclosure for 4 unanchored controller knobs — each gets an external data citation or an explicit "conditional on X" CIP-disclosure paragraph
- ODD-indexed methodology overview (`.planning/methodology-overview.md`) — one-page table mapping ODD protocol elements to in-repo artefacts; partial adoption only (index, do not rewrite)

**Addresses:** Active items 5, 6; resolves CRIT-3, CRIT-4, MOD-6, MIN-1, MIN-2

---

### Phase 5: Handoff

**Rationale:** The CIP author needs a clean paste guide. A final-pass consistency review prevents cross-document ID drift and ensures all three artefacts (`RSK-*` register, `CLM-*` coverage check, refreshed audit/validity-threats) point to the same identifiers.

**Delivers:**
- CIP-author summary: which paragraphs paste into Limitations; which rows cite in Evidence; which `RSK-*` / `CLM-*` IDs the CIP references
- Optional single-page evidence map diagram
- Final consistency review: no dead references; no renumbering; IDs append-only
- Pinned commit + topology snapshot reference (git tag publication commit; cite epoch-582 date)

**Addresses:** Closes all prior Active items in their CIP-citation form

---

### Phase Ordering Rationale

The ordering is a strict dependency chain at the top: register → coverage check → cheap tests. The register's `scope-of-resolution` fields determine what constitutes a valid cheap test. The coverage check's UNBACKED rows determine which tests deliver the most value. Running tests before either document exists is the canonical "looking under the streetlight" failure mode.

Phases 4 and 5 depend on Phase 3 verdicts being final. The ODD methodology overview (Phase 4) can be drafted during Phase 3 without waiting for test results — the one allowed overlap.

### Research Flags

**Phases needing closer attention during planning:**
- **Phase 3:** Wall-clock cost of N=30 at 8-parallelism should be estimated against one (job, seed) pair before committing to the full seed count. If 4 arms × 3 demand profiles × 30 seeds is too expensive in one session, prioritise the four flip cells and the two most-cited menu-item arms.
- **Phase 4:** Controller-knob anchor search — two hours of literature review at the start of Phase 4 determines anchor vs disclose for window-length 32. This is the one area where external research during planning could materially change the outcome.

**Phases with standard patterns (minimal research needed):**
- **Phase 1:** Source documents are all in-repo; `RSK-*` template fully specified; Wohlin four-fold categorisation is unambiguous.
- **Phase 2:** `CLM-*` table schema fully specified; menu items fixed; source suites are the 7 goldens-pinned suites.
- **Phase 5:** Consolidation only.

---

## Open Questions

Residual decisions the roadmapper or planner needs to resolve before or during phase execution. None block Phase 1, but all should be decided before Phase 3:

1. **N for multi-seed variance bands.** Measure wall-clock cost of one (job, seed) pair at the canonical menu-item jobs before committing to N=30. N=20 with explicit BCa-coverage disclosure is the acceptable fallback if cost is prohibitive.

2. **Whether to cite the 12 unpinned demand-regime suites.** They can appear as WEAK-verdict rows in `CLM-*` (exploratory, no golden-hash guarantee) or be excluded entirely from the coverage table. The policy choice affects how many CLM-* rows the coverage check has and their evidence quality. Decide before Phase 2 begins.

3. **Methodology appendix placement in the CIP.** ODD-indexed overview can live in `.planning/methodology-overview.md` (supplementary, CIP cites by URL) or be drafted as a CIP appendix. Lower-friction for the milestone to keep it supplementary; affects Phase 4 target output location.

4. **Exact Δ% threshold for pool-number sensitivity MITIGATED verdict.** A natural choice: Δ% < seed-IQR of the same job at 100 pools (within seed noise). Must be stated in the `RSK-pool-count` entry's `scope-of-resolution` field before the test runs (MOD-5 prevention). Decide during Phase 1 register drafting.

5. **Hash-diversity as a hard publication gate.** PITFALLS MOD-4 recommends that any cell with `hash-div < N_seeds` cannot be promoted to BACKED without re-running with different seed values. The strict policy is cleaner; the soft policy (mark WEAK with annotation) is cheaper. Decide before Phase 3 begins.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Methodology stack | HIGH | All dependencies in-tree; PSE/BCa methodology well-founded; `paired_bootstrap.rs` is trivial 150-LoC implementation |
| Feature catalogue | HIGH | Table-stakes derive from CIP-1 requirements and CIP-0164 precedent; differentiators from EIP-1559 empirical literature |
| Artefact architecture | HIGH | Register-plus-cheap-tests pattern validated on this project ([`validity-threats.md`](../../docs/phase-2/validity-threats.md) is the prototype); five-phase build order derives from non-negotiable dependency chain |
| Pitfalls | HIGH | CRIT-1 through CRIT-5 each have concrete in-repo evidence; moderate/minor pitfalls from empirical-SE literature with strong precedent |
| ID convention choice | HIGH | RSK-* wins on ecosystem recognition; decision is reversible by mass-rename if user prefers numeric IDs |

**Overall confidence:** HIGH for methodology and architecture; MEDIUM for N=30 specifically (depends on wall-clock cost not yet measured) and for controller-knob anchor availability (depends on literature search not yet done).

### Gaps to Address

- **Wall-clock cost of N=30 multi-seed runs** — measure before committing N; handled during Phase 3 task specification
- **External anchor availability for controller knobs** — 2-hour literature search at Phase 4 start; determines anchor vs disclose for window-length 32 especially
- **Exact Δ% threshold for pool-number sensitivity** — must be in `scope-of-resolution` before the test runs; handled during Phase 1 register drafting
- **12 unpinned demand-suite citation policy** — affects CLM-* row count and evidence quality; decide before Phase 2 begins
- **Hash-diversity gate strictness** — strict vs soft policy for seed-correlated cells; decide before Phase 3 begins

---

## Sources

**Cardano ecosystem (HIGH):** CIP-0001, CIP-0052, CIP-0164, Leios `ImpactAnalysis.md`, Leios technical-report-2

**EIP-1559 literature (HIGH):** Roughgarden 2020 ([arxiv:2012.00854](https://arxiv.org/abs/2012.00854)), Liu et al. CCS 2022 ([arxiv:2201.05574](https://arxiv.org/abs/2201.05574)), Chung & Shi SODA 2023 ([eprint 2021/1474](https://eprint.iacr.org/2021/1474)), Reijsbergen et al. AFT 2021

**Statistical methodology (HIGH):** Sharma 2025 ([arxiv:2512.24145](https://arxiv.org/abs/2512.24145)), Glasserman & Yao classical CRN, Pustejovsky BCa coverage simulations

**Empirical SE / validity (HIGH):** Wohlin four-fold taxonomy, Verdecchia et al. ESEM 2024, Sjøberg 2023

**Documentation / provenance (MEDIUM):** Grimm et al. 2020 (ODD protocol JASSS 23(2):7), SIMPROV 2024, Williams 2024 transparent-reporting checklist

**In-repo (authoritative context):** [`CLAUDE.md`](../../CLAUDE.md), [`.planning/PROJECT.md`](../PROJECT.md), [`docs/phase-2/validity-threats.md`](../../docs/phase-2/validity-threats.md), [`docs/phase-2/cardano-realism-audit.md`](../../docs/phase-2/cardano-realism-audit.md), [`.planning/REVIEW.md`](../REVIEW.md) Fix Status, [`.planning/family-b-decision-2026-05-14.md`](../family-b-decision-2026-05-14.md), [`.planning/mechanism-welfare-impact-2026-05-14.md`](../mechanism-welfare-impact-2026-05-14.md), [`.planning/family-b-results-table-2026-05-14.md`](../family-b-results-table-2026-05-14.md)
