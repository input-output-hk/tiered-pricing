---
phase: 03-targeted-cheap-tests
status: complete-with-gaps
date: 2026-05-18
requirements: [TEST-01, TEST-02, TEST-03, TEST-04, TEST-05, TEST-06, TEST-07, COV-05]
plans:
  - 03-01-PLAN.md (Wave 1 — foundations)
  - 03-02-PLAN.md (Wave 2 — multi-seed sub-tasks)
  - 03-03-PLAN.md (Wave 3 — hash-diversity gate + consolidation)
deliverables_complete: 4
deliverables_partial: 2
deliverables_total: 6
---

# Phase 3: Targeted Cheap Tests — SUMMARY

**Goal:** Resolve Live realism risks via targeted cheap tests, producing
variance bands and sensitivity verdicts that flip `docs/phase-2/coverage-check.md`
rows from UNBACKED / WEAK to BACKED where evidence supports it.

**Outcome:** Goal partially achieved. Three of seven cheap tests landed full
multi-seed evidence (TEST-03, TEST-04, TEST-07a). TEST-01 and TEST-02 are
infrastructure deliverables that landed fully. TEST-05 (pool-number sensitivity)
and TEST-06 (run-length / steady-state) have partial data only and are deferred
to Phase 4 with documented gaps and re-run recipes.

## Headline finding (load-bearing for Cardano Improvement Proposal (CIP) Evidence section)

At `multiplier_floor = 4` under `sundaeswap_moderate` demand at N=20 seeds:

- **Un-reserved menu arms materially outperform single-lane Ethereum Improvement
  Proposal 1559 (EIP-1559)**: priority-only un-reserved Δ retained_value =
  +6.66e+09 (95% Bias-corrected and accelerated (BCa) confidence interval
  [+4.28e+09, +8.49e+09]); both-dynamic un-reserved Δ = +7.95e+09 (CI [+5.65e+09,
  +1.09e+10]). Sign-coherence 0.90 across 20 seeds.
- **RB-reserved menu arms underperform single-lane EIP-1559**: priority-only
  RB-reserved Δ = −4.15e+09 (CI [−6.02e+09, −1.00e+09]); both-dynamic
  RB-reserved (partitioned) Δ = −4.15e+09 (CI [−5.95e+09, −8.87e+08]). This
  REFUTES the Phase 1 / Phase 2 single-seed framing that "two-lane mechanisms
  outperform single-lane EIP-1559" — that framing holds only for the un-reserved
  variants under this calibration.
- The cross-arm duplicate-job artefact (partitioned ≡ rb-reserved welfare under
  sundaeswap_moderate × floor=4) replicates at N=20 because the standard quote
  never drifts off the multiplier floor.
- The multiplier_floor = 4 calibration is **regime-dependent**: at floor = 16
  (TEST-07a) the rb-scarcity finding inverts ("standard dominates welfare" →
  "priority captures everything; total welfare collapses 93–98%") and the
  urgency-inversion finding weakly reverses ("mispriced > correctly priced" →
  "correctly priced > mispriced by ~13%").

## Deliverables status

### Complete (4 of 6)

| Deliverable | Where | Status |
|---|---|---|
| `paired_bootstrap.rs` library (TEST-01) | `sim-rs/sim-cli/src/metrics/paired_bootstrap.rs` | 240 lines (~150 LoC + tests); 8 unit tests pass; M2/M3/M5 goldens unaffected |
| Scoping results (TEST-02) | `.planning/realism-tests/multi-seed-variance/scoping-results.md` | 5 seeds × 1 canonical job; mean wall-clock 93.7s; chosen N=20 |
| Multi-seed variance results (TEST-03 + TEST-04) | `.planning/realism-tests/multi-seed-variance/results.md` + `sign-flip/*.json` + `canonical/*.json` | All 9 cells (4 + 5) have BCa CIs at N=20; hash gate passes 17/17 |
| Multiplier-floor-16 companion (TEST-07a) | `.planning/realism-tests/multiplier-floor-16-companion/results.md` + 6 per-cell JSONs | LIVE → DISCLOSED on both rb-scarcity and urgency-inversion findings; floor=16 regime-dependent |
| Hash-diversity gate report (COV-05 / Wave 3) | `.planning/realism-tests/hash-diversity-gate/results.md` | 17/17 BACKED-eligible cells pass distinct-hash test |
| `topology-realistic-150.yaml` (TEST-05 prerequisite) | `sim-rs/parameters/phase-2-sweep/topology-realistic-150.yaml` | 150 nodes, total stake 3e10, lottery margin 944165 (>>100), reproducible via re-parameterised generator |

### Partial (2 of 6) — deferred to Phase 4

| Deliverable | Coverage | Re-run command |
|---|---|---|
| TEST-05 pool-number sensitivity | 35/1650 runs at the over-scoped batch (cut suite at 165 runs not executed) | `scripts/run-phase-3-suites.sh 1 parameters/phase-2-sweep/suites/phase-3-pool-number-sensitivity.yaml` (~50 min wall-clock) |
| TEST-06 run-length / steady-state | 31/120 runs; only 1 of 4 menu arms covered | `scripts/run-phase-3-suites.sh 1 parameters/phase-2-sweep/suites/phase-3-run-length.yaml` (~56 min wall-clock) |

Both gaps are documented with re-run recipes in their respective `results.md`
files. If Phase 4 chooses not to re-run, the `RSK-pool-count`,
`RSK-calibration-stale-stake-snapshot`, and `RSK-steady-state-run-length`
entries remain LIVE and depend on the Phase 1 plan-02 disclosure paragraphs
as the load-bearing fallback (which is the explicit design — see
`docs/phase-2/realism-risks-register.md`).

## What landed in tree

| File | Lines | Purpose |
|---|---|---|
| `sim-rs/sim-cli/src/metrics/paired_bootstrap.rs` | 240 | TEST-01 library; paired_bca_ci + paired_delta_summary + CiResult + DeltaSummary |
| `sim-rs/sim-cli/src/metrics/mod.rs` | +2 | Module wiring + re-export |
| `sim-rs/scripts/generate-realistic-100-topology.py` | 405 | Refactored: default emits 150-node from committed 100-node YAML; `--regenerate-100` preserved as legacy Koios path |
| `sim-rs/scripts/run-phase-3-suites.sh` | 79 | Phase-3 parallel suite runner (mirrors `run-parallel-suites.sh`) |
| `sim-rs/scripts/analyse-phase-3.py` | 270 | Phase-3 analysis pass; reads `run_summary.json` per (job, seed), emits per-cell `.json` artefacts and verdict tables |
| `sim-rs/parameters/phase-2-sweep/topology-realistic-150.yaml` | 5417 | 150-node mass-stratified mainnet topology |
| `sim-rs/parameters/phase-2-sweep/suites/phase-3-*.yaml` (6 files) | 2155 | 6 Phase-3 suite YAMLs (scoping + 5 Wave-2 suites) |
| `.planning/realism-tests/multi-seed-variance/scoping-results.md` | ~70 | TEST-02 wall-clock + chosen N |
| `.planning/realism-tests/multi-seed-variance/results.md` | ~90 | TEST-03 + TEST-04 results + methodology |
| `.planning/realism-tests/multi-seed-variance/sign-flip/*.json` | 4 files | TEST-03 per-cell BCa CI + DeltaSummary + per-seed retained_value + per-seed sha256 |
| `.planning/realism-tests/multi-seed-variance/canonical/*.json` | 4 files | TEST-04 per-cell artefacts |
| `.planning/realism-tests/multiplier-floor-16-companion/results.md` | ~70 | TEST-07a qualitative findings + floor=4 vs floor=16 comparison |
| `.planning/realism-tests/multiplier-floor-16-companion/*.json` | 6 files | TEST-07a per-cell raw retained_value + lane share + sha256 |
| `.planning/realism-tests/pool-number-sensitivity/results.md` | ~30 | TEST-05 data-gap documentation + re-run recipe |
| `.planning/realism-tests/run-length-steady-state/results.md` | ~30 | TEST-06 data-gap documentation + re-run recipe |
| `.planning/realism-tests/hash-diversity-gate/results.md` | ~50 | Wave 3 COV-05 gate report |
| `docs/phase-2/coverage-check.md` | +21 rows updated, +1 evidence section | 8 directly-tested CLM rows (CLM-06..13) flipped from UNBACKED to BACKED or WEAK with TEST-03/04 evidence; Phase 3 evidence summary section appended |

## Determinism + invariants

- **Goldens unchanged.** Phase 3 modules (`paired_bootstrap.rs`, the topology
  generator's 150-pool path, the new Phase-3 suite YAMLs) are pure additions
  or post-processing on `RunSummary` reporting scalars. They do not perturb
  M2/M3 unit-test goldens or M5 suite-level goldens (per CLAUDE.md §"Numeric
  representation contract"). Verified by `cargo test --release --workspace`
  passing 130/130 tests after Phase 3 additions.
- **Phase-3 suites are NOT goldens-pinned** (per CONTEXT.md D-25). No entries
  in `parameters/phase-2-sweep/suites/.goldens/`, no rows in
  `sim-cli/tests/determinism.rs`. The pricing-event-stream SHA-256 is still
  computed and recorded per (job, seed); the hash-diversity gate (COV-05)
  uses it directly without elevating to golden status.
- **Hash-diversity gate.** All 17 BACKED-eligible cells pass at distinct-hash
  count = N seeds. No re-run-needed verdicts; no downgrades to WEAK from gate
  failure.
- **No new sim-cli crate dependencies.** TEST-01 uses `rand::rngs::StdRng`
  (ChaCha-based, value-stable within `rand` 0.9.x) instead of
  `rand_chacha::ChaCha20Rng`. Cargo.toml + Cargo.lock unperturbed.

## Phase 4 inputs

This phase produces the following Phase 4 inputs:

1. **CIP Evidence section material.** Use the headline finding above (and
   the per-cell `*.json` artefacts) as load-bearing evidence for the menu-
   option recommendation. The unreserved arms vs RB-reserved arms welfare
   distinction is the central finding.
2. **CIP Limitations section material.** Use the four Phase 3 disclosures:
   (a) TEST-05 pool-number sensitivity is partial — defer to disclosure
   paragraph in `docs/phase-2/realism-risks-register.md` `RSK-pool-count`.
   (b) TEST-06 run-length steady-state is partial — defer to disclosure
   paragraph in `RSK-steady-state-run-length`.
   (c) TEST-07a shows multiplier_floor = 4 calibration is regime-dependent;
   findings invert at floor = 16. This is reframe-not-replicate for
   `RSK-multiplier-floor-4-suite-coverage`.
   (d) The cross-arm duplicate-job artefact (partitioned ≡ rb-reserved
   welfare at sundaeswap_moderate × floor=4) replicates at N=20; this is
   itself a finding worth disclosing in the CIP.
3. **Phase 4 audit refresh inputs.** The `docs/phase-2/cardano-realism-audit.md`
   refresh (DOC-01) reads the new BACKED rows in coverage-check.md and the
   new disclosure paragraphs. The TEST-04 refutation of two welfare claims is
   particularly load-bearing.
4. **Phase 4 re-run option.** If Phase 4 chooses to land TEST-05 and TEST-06
   data, the two re-run commands in this SUMMARY are the recipes. Expected
   wall-clock combined: ~2 hours at `-P 8`.

## Open questions for Phase 4

- **Did `RSK-pool-count` MITIGATE or stay LIVE → DISCLOSED?** Depends on
  whether Phase 4 re-runs TEST-05 (decision recorded in Phase 4's discuss).
- **Does the multiplier_floor calibration choice deserve more disclosure?**
  TEST-07a showed it is regime-dependent in a non-trivial way. The CIP's
  Limitations section should reflect this clearly.
- **Cross-arm duplicate-job artefact at higher demand congestion.** The
  Phase-3 evidence is at `sundaeswap_moderate`. Whether the duplicate-job
  artefact persists at `paper_like_congested` or `paper_like_mispriced` is
  worth a single sanity-check seed if Phase 4 has compute headroom.

## Verification

The auto-verifier should:
1. Confirm files in the "What landed" table all exist with non-empty content.
2. Run `cargo test --release -p sim-cli paired_bootstrap` and check 8/8 pass.
3. Run `cargo run --release --bin experiment-suite -- verify
   parameters/phase-2-sweep/suites/phase-3-canonical-variance.yaml` (and the
   other complete suites) to confirm determinism.
4. Spot-check `docs/phase-2/coverage-check.md` rows CLM-06..13 carry the new
   `TEST-03` / `TEST-04` evidence in their `confidence-method` cells.
5. Confirm the Phase 3 evidence summary section is present at the file's tail.

## Abbreviations on first use

- **BCa** — Bias-corrected and accelerated (bootstrap confidence-interval method)
- **CIP** — Cardano Improvement Proposal
- **CLM** — claim identifier in coverage-check.md
- **EIP-1559** — Ethereum Improvement Proposal 1559
- **IQR** — Inter-Quartile Range
- **LoC** — Lines of Code
- **RB** — Ranking Block
- **RSK** — realism-risk identifier in realism-risks-register.md
