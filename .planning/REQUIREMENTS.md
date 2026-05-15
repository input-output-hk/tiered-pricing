# REQUIREMENTS.md — Phase-2 CIP Evidence Audit (v1)

The v1 scope for this milestone. Each requirement is testable: it's done when a specific artefact exists, lives at a specific path, and meets a specific acceptance criterion.

Decisions made during questioning are folded into the requirements (number of seeds N, hash-diversity policy, unpinned-suite handling, pool-number Δ% threshold, methodology-overview placement). Open items the planner will resolve are flagged in each requirement's notes.

REQ-ID prefixes:
- **REG-NN** — Realism-risks register (Phase 1 work)
- **COV-NN** — Coverage check (Phase 2 work)
- **TEST-NN** — Cheap targeted tests (Phase 3 work)
- **DOC-NN** — Refresh and anchor of existing audit/validity docs (Phase 4 work)
- **HAND-NN** — Handoff artefacts for the CIP (Cardano Improvement Proposal) author (Phase 5 work)

The convention `RSK-*` / `EXP-*` / `CLM-*` for in-artefact identifiers comes from the Leios `docs/ImpactAnalysis.md` precedent; this REQUIREMENTS.md uses GSD-flavoured `REG-NN` etc. for milestone-level traceability.

---

## v1 Requirements

### Realism-risks register

- [ ] **REG-01** — A single `docs/phase-2/realism-risks-register.md` exists, indexing every realism risk surfaced by `cardano-realism-audit.md`, `validity-threats.md`, `.planning/codebase/CONCERNS.md`, `.planning/REVIEW.md`, `mechanism-welfare-impact-2026-05-14.md`, and the seven spike READMEs. De-duplicated. Each entry has a stable `RSK-NN` identifier, never renumbered.

- [ ] **REG-02** — Every `RSK-NN` entry has the required fields populated: `id`, `title`, `category` (Wohlin four-fold: construct / internal / external / conclusion), `description`, `evidence-for`, `evidence-against`, `scope-of-resolution` (the explicit hypothesis the verdict must establish or refute), `verdict` (LIVE / DORMANT / MITIGATED / DISCLOSED), `disclosure-paragraph` (CIP-pasteable prose if verdict is DISCLOSED).

- [ ] **REG-03** — Each LIVE `RSK-NN` entry is paired with at least one `EXP-NN` identifier — a placeholder for the targeted cheap test that will move it toward MITIGATED or DISCLOSED. `EXP-NN` rows do not need to be runnable yet; just named, scoped, and linked to their `RSK-NN`.

- [ ] **REG-04** — The register includes the four already-named LIVE entries from research: pool-count sensitivity (CRIT-5 mitigation track), single-seed precision (CRIT-1 mitigation track), un-anchored controller knobs window=32 / multiplier-floor 4 / multiplier-floor 16 / lane-signal-source (CRIT-3 anchor-or-disclose track), substrate scope (CRIT-4 disclose-only track).

- [ ] **REG-05** — `RSK-pool-count` carries the locked `scope-of-resolution`: "Δ% < seed-IQR of same job at 100 pools establishes MITIGATED" — the threshold decision is in the register before the test runs, not after, per PITFALLS MOD-5.

### Coverage check

- [ ] **COV-01** — A single `docs/phase-2/coverage-check.md` exists, listing every menu-item trade-off claim the CIP will make as a stable `CLM-NN` identifier. One row per claim. Append-only identifier scheme.

- [ ] **COV-02** — Each `CLM-NN` row has columns: `claim`, `menu-option` (one of: priority-only-RB-reserved, priority-only-un-reserved, both-dynamic-partitioned, both-dynamic-un-partitioned, single-lane-EIP-1559-control), `backing-suite`, `backing-job`, `seeds-cited`, `confidence-method` (e.g. paired-bootstrap BCa N=20, sign-coherence N=5, etc.), `golden-sha256`, `status` (BACKED / WEAK / UNBACKED / OUT-OF-SCOPE), `related-RSK-ids`.

- [ ] **COV-03** — The coverage check includes non-welfare property columns alongside welfare claims: anti-bribery, standard-user-fee-drift exposure, signal-source anchoring, implementation complexity. Each cell either cites a spec section, a simulator measurement, or "disclosed gap." This prevents the menu collapsing into welfare-only advocacy (PITFALLS CRIT-2).

- [ ] **COV-04** — The 12 unpinned demand-regime suites (`paper_like_*`, `sundaeswap_*` beyond the 7 goldens-pinned set) appear as WEAK-verdict rows where they cover claims not backed by goldens-pinned suites. They are not promoted to goldens-pinned in this milestone; the WEAK verdict carries the disclosure.

- [ ] **COV-05** — Every `BACKED` row passes the hash-diversity gate: distinct `pricing_event_stream.sha256` count equals the seed count cited. Rows where seeds collapse to fewer distinct hashes are downgraded to WEAK with annotation, or the cell is re-run with different seed values.

- [ ] **COV-06** — Coverage check skeleton exists and is committable before Phase 3 begins: rows added with `status: UNBACKED` for claims awaiting cheap-test results; this surfaces compute priorities for Phase 3 task ordering.

### Cheap targeted tests

- [ ] **TEST-01** — A `sim-cli/src/metrics/paired_bootstrap.rs` module exists (~150 LoC, no new crate dependencies; uses the already-present `statrs` crate). Implements paired-sample Bias-corrected and accelerated (BCa) bootstrap confidence intervals on welfare deltas. Integration point: collector emits per-seed welfare scalars; the bootstrap is post-processing on the collected scalars, not on the simulation hot path. Acceptance: unit-tested against a known-distribution synthetic dataset (e.g. paired-Gaussian welfare-delta with known mean ± CI); does not perturb existing golden hashes.

- [ ] **TEST-02** — Multi-seed variance scoping run: one canonical menu-item job runs at the realistic-100 topology for N=5 seeds, wall-clock measured. Result determines exact N for the four sign-flip cells (target N=15–20, fall back to N=10 only if 20 is computationally prohibitive).

- [ ] **TEST-03** — Multi-seed variance bands on the four sign-flip cells: `d4_t50_w32`, `d8_t25_w32`, and `x4_rb_quarter` under both rb-reserved-priority and partitioned arms (4 cells total). N per the TEST-02 calibration. Each cell ends with: distinct-hash count, paired-bootstrap BCa 95% confidence interval on welfare delta, sign-coherence percentage, verdict (BACKED via TEST-01 gate / WEAK / re-run-needed). Results at `.planning/realism-tests/multi-seed-variance/`.

- [ ] **TEST-04** — Multi-seed variance bands on the canonical menu-item welfare claims (one job per menu option = 4 cells; single-lane-EIP-1559-control = 1 cell; 5 total). N=10 per cell unless TEST-02 indicates otherwise. Sign-coherence + median + Inter-Quartile Range (IQR), not necessarily bootstrap BCa.

- [ ] **TEST-05** — Pool-number sensitivity smoke: the existing 33-job sundaeswap smoke + analogous 33-job runs for each of the four `paper_like_*` variants (`paper_like_congested`, `paper_like_mispriced`, `paper_like_moderate`, `paper_like_realistic`), run at both 100 pools (current `topology-realistic-100.yaml`) and 150 pools (new `topology-realistic-150.yaml` to be generated by the existing `sim-rs/scripts/generate-realistic-100-topology.py` re-parameterised). Verdict: MITIGATED iff Δ% on welfare metrics < seed-IQR of same job at 100 pools (per REG-05). Results at `.planning/realism-tests/pool-number-sensitivity/`.

- [ ] **TEST-06** — Run-length / steady-state validation: one canonical job per menu option (4 jobs) run at default 2000 slots and at 4000 / 8000 slots. Steady-state criterion: the welfare-per-slot rolling mean over the second half of the run differs from the rolling mean over the first half by less than seed-IQR. If 2000 is insufficient for any menu option, the suite default is raised for affected jobs. Results at `.planning/realism-tests/run-length-steady-state/`.

- [ ] **TEST-07** — Additional 3–5 targeted cheap tests, scoped from REG-01's LIVE entries. Concrete list emerges from the register at Phase 1 close; placeholder REQ-IDs `TEST-07a`, `TEST-07b`, etc. created when the register surfaces them. Each follows the TEST-03 template (named hypothesis → scoped run → verdict in `.planning/realism-tests/<name>/results.md`).

### Refresh and anchor

- [ ] **DOC-01** — `docs/phase-2/cardano-realism-audit.md` is refreshed: the 2026-05-13 annotation banners are removed and folded into authoritative voice. Every calibration value is presented as a `(value, source, date-retrieved)` triple. Includes the CIP-pasteable substrate-scope paragraph for upstream-Leios-simulator limitations.

- [ ] **DOC-02** — `docs/phase-2/validity-threats.md` is refreshed: per-suite trust ratings are cross-referenced to the new `RSK-NN` identifiers. Verdicts are consistent with the register's verdicts. Adds the menu-item-trade-off claims that the coverage check identifies.

- [ ] **DOC-03** — Anchoring or disclosure paragraphs exist for each of the four un-anchored controller knobs: window-length 32, multiplier-floor 4, multiplier-floor 16, lane-signal-source. Two-hour literature search at Phase 4 open: cite Liu et al. CCS 2022 (Conference on Computer and Communications Security) and Reijsbergen et al. AFT 2021 (Advances in Financial Technologies) for Ethereum window-length / step-size data if anchorable. If not anchorable, a "conditional on X" CIP-disclosure paragraph in the `RSK-NN` entry's `disclosure-paragraph` field.

- [ ] **DOC-04** — `docs/phase-2/methodology-overview.md` (or `.planning/methodology-overview.md`) exists as a one-page ODD (Overview, Design concepts, Details) index: a table mapping the ODD protocol's seven elements (Purpose, State variables, Process overview, Design concepts, Initialisation, Input data, Submodels) to in-repo locations (CLAUDE.md sections, mechanism-design.md sections, suite YAMLs, source files). The CIP cites this document by repo URL rather than embedding.

### Handoff

- [ ] **HAND-01** — A `docs/phase-2/cip-author-summary.md` exists listing: which `disclosure-paragraph` blocks paste into the CIP's Limitations section; which `CLM-NN` rows cite into the CIP's Evidence section; the pinned git commit + tag that all artefacts reference; the epoch-582 stake snapshot reference for the topology.

- [ ] **HAND-02** — A final consistency review confirms: no `RSK-NN` or `CLM-NN` references in any artefact point to non-existent identifiers; no identifiers were renumbered; all backing-job paths in the coverage check resolve to suite + job entries that still exist in `parameters/phase-2-sweep/suites/`; all `golden-sha256` values in the coverage check match the current `.goldens/` directory contents.

- [ ] **HAND-03** — The `dynamic-experiment` branch is git-tagged at the milestone-close commit (e.g. `phase-2-cip-evidence-v1`). The tag is the citable reference for the CIP.

### Traceability

Each v1 requirement is mapped to exactly one phase in ROADMAP.md. Coverage: 25/25 (no orphans, no duplicates).

| REQ-ID | Phase | Status |
|--------|-------|--------|
| REG-01 | Phase 1: Register Inventory | Pending |
| REG-02 | Phase 1: Register Inventory | Pending |
| REG-03 | Phase 1: Register Inventory | Pending |
| REG-04 | Phase 1: Register Inventory | Pending |
| REG-05 | Phase 1: Register Inventory | Pending |
| COV-01 | Phase 2: Coverage Check Skeleton | Pending |
| COV-02 | Phase 2: Coverage Check Skeleton | Pending |
| COV-03 | Phase 2: Coverage Check Skeleton | Pending |
| COV-04 | Phase 2: Coverage Check Skeleton | Pending |
| COV-06 | Phase 2: Coverage Check Skeleton | Pending |
| TEST-01 | Phase 3: Targeted Cheap Tests | Pending |
| TEST-02 | Phase 3: Targeted Cheap Tests | Pending |
| TEST-03 | Phase 3: Targeted Cheap Tests | Pending |
| TEST-04 | Phase 3: Targeted Cheap Tests | Pending |
| TEST-05 | Phase 3: Targeted Cheap Tests | Pending |
| TEST-06 | Phase 3: Targeted Cheap Tests | Pending |
| TEST-07 | Phase 3: Targeted Cheap Tests | Pending |
| COV-05 | Phase 3: Targeted Cheap Tests | Pending |
| DOC-01 | Phase 4: Refresh and Anchor | Pending |
| DOC-02 | Phase 4: Refresh and Anchor | Pending |
| DOC-03 | Phase 4: Refresh and Anchor | Pending |
| DOC-04 | Phase 4: Refresh and Anchor | Pending |
| HAND-01 | Phase 5: Handoff | Pending |
| HAND-02 | Phase 5: Handoff | Pending |
| HAND-03 | Phase 5: Handoff | Pending |

Notes on cross-phase assignment:

- **COV-05** (hash-diversity gate) is structurally a property of coverage-check rows but its acceptance is gated by the multi-seed test results that arrive during Phase 3. Per the prompt's REQ-to-phase mapping ("COV-05 hash-diversity gate applies as test results arrive"), COV-05 is mapped to Phase 3, where the gate is applied; the gate's effect (row downgrades) updates the Phase 2 coverage-check document in place.
- **TEST-07** is a placeholder for 3–5 sub-requirements (`TEST-07a`, `TEST-07b`, …) that materialise at Phase 1 close when the register surfaces additional LIVE entries beyond the four already named. Each sub-requirement inherits TEST-07's Phase 3 assignment.

---

## v2 / Deferred

None for v1. Phase-2-milestone-internal deferrals (per PROJECT.md Out of Scope) are below; v2 represents a future milestone after CIP-publication, e.g. post-CIP-feedback revisions, adversarial-actor regime addition, cross-architecture CI build-out.

---

## Out of Scope

- **Writing the CIP itself** — User authors the CIP. Milestone delivers evidence base only.
- **Adversarial / strategic-bidder modelling** — Current actors are utility-maximising; adversarial regime is named as future work in `disclosure-paragraph` per CRIT-3 / Chung & Shi SODA 2023 (Symposium on Discrete Algorithms) precedent. Adding this would double milestone length.
- **Re-auditing upstream simulator code paths** — `sim/lottery.rs`, `sim/driver.rs`, propagation, vote diffusion, and the inherited f64 in non-pricing hot paths are out-of-scope substrate. CRIT-4 covers them with a disclosure paragraph in the register.
- **Cross-architecture continuous integration (CI) verification** — Intra-architectural determinism is sufficient for CIP evidence; deferred per `.planning/codebase/CONCERNS.md`. CRIT-4 substrate-scope paragraph covers the disclosure.
- **600-pool CIP-0164 topology migration** ([`docs/phase-2/m6-implementation-plan.md`](../docs/phase-2/m6-implementation-plan.md) as drafted) — Superseded by TEST-05. The drafted M6 plan stays in tree as a contingency if pool-number sensitivity surfaces a real gap.
- **Cutting the EIP-1559 (Ethereum Improvement Proposal 1559) suites** — Retained as control evidence; CIP cites as baseline. PROJECT.md decision.
- **Promoting unpinned demand-regime suites to goldens-pinned** — Per COV-04, the 12 unpinned suites remain unpinned; goldens regeneration is out-of-scope churn.
- **N=30 uniform across all jobs and seed-sets** — Tiered N (TEST-02 calibration) is the decision; reviewers will see "N=15–20 on flip cells, N=10 on canonical, N=3–5 on broader sweeps." Not "N=30 everywhere."
- **Deployed-EIP-1559 baseline comparison from mainnet data** — The flat-fee counterfactual already in suites is the load-bearing comparator; pulling in external Ethereum data is a differentiator, not table stakes (FEATURES.md D2).
- **Cross-ref index automation** — A small script that grep's `RSK-NN` / `CLM-NN` across artefacts is flagged in ARCHITECTURE.md as "at 200 entries"; the milestone expects 30–50 entries, manageable manually.

---

*Last updated: 2026-05-15 after roadmap creation (traceability table populated)*
