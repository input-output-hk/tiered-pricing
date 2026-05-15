# Roadmap: Phase-2 Cardano Improvement Proposal (CIP) Evidence Audit

## Overview

This milestone produces the evidence base for a Cardano Improvement Proposal (CIP) responding to CPS-0023 ("Urgency Signaling"). The user authors the CIP themselves; this roadmap delivers the artefacts the CIP cites or pastes from: a realism-risks register, a coverage check mapping menu-item trade-off claims to specific simulator jobs, targeted cheap tests resolving Live risks, refreshed audit and validity documents, and a handoff package consolidating everything for the CIP author.

The build order is a strict dependency chain. The realism-risks register (Phase 1) comes first because the `scope-of-resolution` field on each `RSK-NN` entry disciplines the cheap tests in Phase 3 — without it, those tests produce "looks mitigated, isn't" results. The coverage check skeleton (Phase 2) surfaces which menu claims have backing before compute is spent, so Phase 3 test ordering is value-driven. Cheap tests (Phase 3) must precede the audit refresh (Phase 4) so authoritative documents are consistent with final test verdicts. The handoff (Phase 5) closes the loop with a clean paste guide for the CIP author and a citable git tag.

Granularity: standard. Five phases, 25 v1 requirements, 100% coverage.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3, 4, 5): planned milestone work
- Decimal phases (e.g. 2.1): urgent insertions (marked with INSERTED)

- [x] **Phase 1: Register Inventory** - Build the realism-risks register that disciplines every downstream test (completed 2026-05-15)
- [ ] **Phase 2: Coverage Check Skeleton** - Map menu-item trade-off claims to backing simulator jobs, surface gaps
- [ ] **Phase 3: Targeted Cheap Tests** - Resolve Live risks via paired-bootstrap variance bands, pool-number sensitivity, run-length validation
- [ ] **Phase 4: Refresh and Anchor** - Refresh authoritative audit documents and anchor or disclose the four un-anchored controller knobs
- [ ] **Phase 5: Handoff** - Consolidate the evidence package into a CIP-author summary and tag the citable commit

## Phase Details

### Phase 1: Register Inventory
**Goal**: A single realism-risks register exists that catalogues every realism risk surfaced by existing artefacts, with stable identifiers, Wohlin-categorised entries, and locked scope-of-resolution fields that discipline downstream cheap-test design.
**Depends on**: Nothing (first phase)
**Requirements**: REG-01, REG-02, REG-03, REG-04, REG-05
**Success Criteria** (what must be TRUE):
  1. `docs/phase-2/realism-risks-register.md` exists with stable `RSK-NN` identifiers (never renumbered) that de-duplicate risks across `cardano-realism-audit.md`, `validity-threats.md`, `.planning/codebase/CONCERNS.md`, `.planning/REVIEW.md`, `mechanism-welfare-impact-2026-05-14.md`, and the seven spike READMEs
  2. Every `RSK-NN` entry has all required fields populated: `id`, `title`, `category` (Wohlin construct / internal / external / conclusion), `description`, `evidence-for`, `evidence-against`, `scope-of-resolution`, `verdict` (LIVE / DORMANT / MITIGATED / DISCLOSED), `disclosure-paragraph`
  3. Each LIVE entry is paired with at least one named `EXP-NN` identifier scoped to move the verdict toward MITIGATED or DISCLOSED — the `EXP-NN` rows are the input to Phase 3 test ordering
  4. The four already-named LIVE entries from research are present: pool-count sensitivity, single-seed precision, un-anchored controller knobs, substrate scope
  5. `RSK-pool-count` carries the locked threshold `"Δ% < seed-IQR of same job at 100 pools establishes MITIGATED"` in its `scope-of-resolution` field before Phase 3 begins
**Plans:** 2 plans
Plans:
**Wave 1**
- [x] 01-01-PLAN.md — Inventory pass and register skeleton: cluster ~20–30 thematic RSK-NN entries from the six source documents and seven spike READMEs, populate descriptive fields, mark judgement fields TBD plan 02 (completed 2026-05-15)

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 01-02-PLAN.md — Finalise verdicts, scope-of-resolution, EXP-NN cross-references, and CIP-pasteable disclosure-paragraphs; verify register consistency (completed 2026-05-15)

### Phase 2: Coverage Check Skeleton
**Goal**: A coverage check exists that maps every menu-item trade-off claim the Cardano Improvement Proposal (CIP) will make to a specific backing simulator job, including non-welfare property columns that keep the menu a menu, with gaps surfaced as `UNBACKED` rows that prioritise Phase 3 work.
**Depends on**: Phase 1 (the `RSK-NN` identifiers from REG-01 are cross-referenced from coverage rows via the `related-RSK-ids` column)
**Requirements**: COV-01, COV-02, COV-03, COV-04, COV-06
**Success Criteria** (what must be TRUE):
  1. `docs/phase-2/coverage-check.md` exists as a flat table with stable `CLM-NN` identifiers (append-only), one row per claim
  2. Each `CLM-NN` row carries the full column set: `claim`, `menu-option`, `backing-suite`, `backing-job`, `seeds-cited`, `confidence-method`, `golden-sha256`, `status` (BACKED / WEAK / UNBACKED / OUT-OF-SCOPE), `related-RSK-ids`
  3. Non-welfare property columns are present alongside welfare claims — anti-bribery, standard-user-fee-drift exposure, signal-source anchoring, implementation complexity — with each cell citing a spec section, a simulator measurement, or "disclosed gap"
  4. The 12 unpinned demand-regime suites appear as `WEAK`-verdict rows where they cover claims not backed by the seven goldens-pinned suites; they are not promoted to goldens-pinned in this milestone
  5. The skeleton is committable before Phase 3 begins: rows for claims awaiting cheap-test results carry `status: UNBACKED`, surfacing compute priorities for Phase 3 task ordering
**Plans**: TBD

### Phase 3: Targeted Cheap Tests
**Goal**: Live risks identified in the register are resolved (or explicitly downgraded to disclosure) via targeted cheap tests, producing variance bands and sensitivity verdicts that flip coverage-check rows from `UNBACKED` / `WEAK` to `BACKED` where the evidence supports it.
**Depends on**: Phase 1 (test hypotheses come from `RSK-NN` `scope-of-resolution` fields), Phase 2 (test priority comes from `UNBACKED` rows)
**Requirements**: TEST-01, TEST-02, TEST-03, TEST-04, TEST-05, TEST-06, TEST-07, COV-05
**Success Criteria** (what must be TRUE):
  1. `sim-cli/src/metrics/paired_bootstrap.rs` exists (~150 lines of code (LoC), no new crate dependencies, uses the in-tree `statrs` crate), implements paired-sample Bias-corrected and accelerated (BCa) bootstrap confidence intervals on welfare deltas, is unit-tested against a known-distribution synthetic dataset, and does not perturb existing golden hashes
  2. Wall-clock scoping run (N=5 seeds on one canonical menu-item job at the realistic-100 topology) is completed and its result determines the exact N used by TEST-03 (target N=15–20, fallback N=10)
  3. The four sign-flip cells (`d4_t50_w32`, `d8_t25_w32`, and `x4_rb_quarter` under both rb-reserved-priority and partitioned arms) each have results in `.planning/realism-tests/multi-seed-variance/` reporting distinct-hash count, paired-bootstrap BCa 95% confidence interval, sign-coherence percentage, and a verdict (BACKED / WEAK / re-run-needed)
  4. The five canonical menu-item welfare cells (one per menu option plus the single-lane EIP-1559 control) have results with sign-coherence, median, and Inter-Quartile Range (IQR) at N=10 (or as TEST-02 indicates)
  5. Pool-number sensitivity results exist at `.planning/realism-tests/pool-number-sensitivity/` for the 33-job smoke × {100, 150 pools} × {sundaeswap_moderate + 4 paper_like variants} cross-product, with MITIGATED verdict iff Δ% on welfare metrics is within the seed-IQR threshold locked by REG-05
  6. Run-length / steady-state validation results exist at `.planning/realism-tests/run-length-steady-state/` for one canonical job per menu option (4 jobs) at 2000 / 4000 / 8000 slots, with the suite default raised for any menu option that fails the steady-state criterion
  7. The hash-diversity gate (COV-05) has been applied: every `BACKED` coverage-check row has a distinct `pricing_event_stream.sha256` count equal to its seed count, and rows that collapse are downgraded to `WEAK` with annotation or re-run with different seed values
**Plans**: TBD

### Phase 4: Refresh and Anchor
**Goal**: The authoritative audit and validity-threats documents are refreshed to consistent, CIP-pasteable voice, every calibration value carries a `(value, source, date-retrieved)` triple, the four un-anchored controller knobs are either anchored to deployed-system data or carry an explicit disclosure paragraph, and a one-page Overview, Design concepts, Details (ODD) methodology index exists for the CIP author to cite by repo URL.
**Depends on**: Phase 3 (refresh verdicts must reflect final test results)
**Requirements**: DOC-01, DOC-02, DOC-03, DOC-04
**Success Criteria** (what must be TRUE):
  1. `docs/phase-2/cardano-realism-audit.md` reads in authoritative voice (2026-05-13 annotation banners removed and folded in), every calibration value is presented as a `(value, source, date-retrieved)` triple, and the CIP-pasteable substrate-scope paragraph for upstream-Leios-simulator limitations is included
  2. `docs/phase-2/validity-threats.md` is refreshed so per-suite trust ratings are cross-referenced to `RSK-NN` identifiers, verdicts are consistent with the register, and any menu-item-trade-off claims identified by the coverage check are added
  3. Each of the four un-anchored controller knobs (window-length 32, multiplier-floor 4, multiplier-floor 16, lane-signal-source) carries either an external anchor (e.g. Liu et al. Conference on Computer and Communications Security (CCS) 2022, Reijsbergen et al. Advances in Financial Technologies (AFT) 2021 for Ethereum window-length / step-size data) or a "conditional on X" disclosure paragraph in the `RSK-NN` entry's `disclosure-paragraph` field, after a two-hour literature search at phase open
  4. `docs/phase-2/methodology-overview.md` (or `.planning/methodology-overview.md`) exists as a one-page ODD index mapping the seven ODD elements (Purpose, State variables, Process overview, Design concepts, Initialisation, Input data, Submodels) to in-repo locations, so the CIP cites this document by repo URL rather than embedding methodology prose
**Plans**: TBD

### Phase 5: Handoff
**Goal**: The Cardano Improvement Proposal (CIP) author has a single consolidated summary identifying which artefacts paste into which CIP sections, a final consistency review confirms no dead identifier references and no renumbering across the evidence package, and the `dynamic-experiment` branch is git-tagged at a citable milestone-close commit.
**Depends on**: Phase 4 (all refreshed documents and disclosure paragraphs must be final before consolidation)
**Requirements**: HAND-01, HAND-02, HAND-03
**Success Criteria** (what must be TRUE):
  1. `docs/phase-2/cip-author-summary.md` exists listing: which `disclosure-paragraph` blocks paste into the CIP's Limitations section, which `CLM-NN` rows cite into the CIP's Evidence section, the pinned git commit and tag that all artefacts reference, and the epoch-582 stake snapshot reference for the topology
  2. A final consistency review has been performed and recorded: no `RSK-NN` or `CLM-NN` references in any artefact point to non-existent identifiers, no identifiers were renumbered, all `backing-job` paths in the coverage check resolve to suite + job entries that still exist in `parameters/phase-2-sweep/suites/`, and all `golden-sha256` values in the coverage check match the current `.goldens/` directory contents
  3. The `dynamic-experiment` branch carries a git tag at the milestone-close commit (suggested name: `phase-2-cip-evidence-v1`), and that tag is the citable reference recorded in `cip-author-summary.md`
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Register Inventory | 2/2 | Complete | 2026-05-15 |
| 2. Coverage Check Skeleton | 0/TBD | Not started | - |
| 3. Targeted Cheap Tests | 0/TBD | Not started | - |
| 4. Refresh and Anchor | 0/TBD | Not started | - |
| 5. Handoff | 0/TBD | Not started | - |
