# Validity threats — phase-2 dynamic-pricing simulator

Date: 2026-05-18
Branch: dynamic-experiment
Scope: per-claim trust assessment for all 19 phase-2 suite YAMLs in
`parameters/phase-2-sweep/suites/`.
Companion to: [cardano-realism-audit.md](cardano-realism-audit.md),
[realism-risks-register.md](realism-risks-register.md),
[coverage-check.md](coverage-check.md),
[REVIEW.md](../../.planning/REVIEW.md), and the audit spike READMEs.

Abbreviations on first use: Cardano Improvement Proposal (CIP),
Ethereum Improvement Proposal 1559 (EIP-1559), ranking block (RB),
endorser block (EB), Realism Risk (RSK) identifier in
[`realism-risks-register.md`](realism-risks-register.md),
claim identifier (CLM) in [`coverage-check.md`](coverage-check.md),
Inter-Quartile Range (IQR), Bias-corrected and accelerated (BCa)
bootstrap, Maximum Extractable Value (MEV), confidence interval (CI),
Paired Seed Evaluation (PSE).

## TL;DR

Of the 19 phase-2 suite YAMLs in `parameters/phase-2-sweep/suites/`,
this document gives a per-suite trust assessment so a CIP author or
reviewer can drill into a specific suite's evidence and know which
claims it can fairly license. The operational topology across every
suite is `topology-realistic-100.yaml` (100 nodes, mass-stratified
epoch-582 Cardano mainnet snapshot retrieved 2026-05-14, top-1 stake
share 1.97%, Nakamoto coefficient 35, Gini 0.253) per
[`.planning/spikes/006-curve-design/README.md`](../../.planning/spikes/006-curve-design/README.md);
all 7 goldens-pinned M3 / M4 suites verify bit-identically intra-arch.
The pricing controller is **Family B** (chain-derived, EIP-1559-
faithful, one step per canonical block) per
[`.planning/family-b-decision-2026-05-14.md`](../../.planning/family-b-decision-2026-05-14.md),
which closes WR-1 (controller contamination on slot-battle reorg) by
construction.

Phase 3's cheap tests
([`04-03-phase3-evidence-summary.md`](../../.planning/phases/04-refresh-and-anchor/04-03-phase3-evidence-summary.md))
produced multi-seed evidence at N=20 BCa CIs:

- **Un-reserved menu arms outperform single-lane EIP-1559** at
  `multiplier_floor = 4` under `sundaeswap_moderate` demand:
  priority-only un-reserved Δ retained_value = +6.66e+09 (95% BCa CI
  [+4.28e+09, +8.49e+09]); both-dynamic un-reserved Δ = +7.95e+09
  (CI [+5.65e+09, +1.09e+10]).
- **RB-reserved menu arms underperform single-lane EIP-1559** under
  the same calibration: priority-only RB-reserved Δ = −4.15e+09 (CI
  [−6.02e+09, −1.00e+09]); both-dynamic partitioned Δ = −4.15e+09
  (CI [−5.95e+09, −8.87e+08]). REFUTES the pre-Phase-3 single-seed
  framing that "two-lane mechanisms outperform single-lane EIP-1559"
  for the RB-reserved variants under this calibration.
- **`multiplier_floor = 4` is regime-dependent**: at floor=16 the
  `rb-scarcity` finding inverts (priority captures everything, total
  welfare drops 93–98%) and `urgency-inversion` weakly reverses
  (correctly-priced > mispriced by ~13%) per TEST-07a.
- **Hash-diversity gate**: 17 of 17 BACKED-eligible cells pass the
  COV-05 strict gate.

Aggregate count: **2 HIGH** (un-reserved menu arms Phase 3 confirms
at N=20), **13 MEDIUM** (defensible with the standard footer + 1–2
specific caveats; includes the four formerly-UNRESOLVED suites whose
verdicts are now derived from Phase 2's output-read per
[`coverage-check.md`](coverage-check.md)), **4 LOW** (conclusions
condition on `multiplier_floor = 4` being load-bearing or on Phase 3
having statistically refuted the pre-Phase-3 framing). No suite
carries UNRESOLVED today.

## Family B decision

The phase-2 publication mechanism is **Family B**: chain-derived,
EIP-1559-faithful, one controller step per canonical
`LinearRankingBlock`. Controller state lives on the canonical chain
itself (`derived_quote` as a pure function of parent + predecessor
samples) rather than in node-local mutable state. Family B was
committed for publication 2026-05-14 per
[`.planning/family-b-decision-2026-05-14.md`](../../.planning/family-b-decision-2026-05-14.md);
the M5 suite-level goldens were regenerated against the chain-derived
implementation that day and all 7 goldens-pinned suites verify
bit-identically intra-arch.

Welfare-impact across mechanisms (Phase 3 N=20 BCa CIs per
[`04-03-phase3-evidence-summary.md`](../../.planning/phases/04-refresh-and-anchor/04-03-phase3-evidence-summary.md),
superseding the earlier N=1 33-job characterisation in
[`.planning/mechanism-welfare-impact-2026-05-14.md`](../../.planning/mechanism-welfare-impact-2026-05-14.md)):

- **Un-reserved arms** outperform single-lane EIP-1559 with tight CIs
  at `sundaeswap_moderate × floor=4` (sign-coherence 0.90, N=20).
- **RB-reserved arms** underperform single-lane EIP-1559 by ~4e+09
  retained_value under the same calibration (sign-coherence 0.65,
  N=20).
- **Sign-flip cells** at the harshest combined-stress corner
  (`x4_rb_quarter` under RB-reserved and partitioned) produce
  real-but-noisy positive medians; CIs straddle zero. Both land WEAK
  per TEST-03.
- **Single-lane sign-flip cells** (`d4_t50_w32`, `d8_t25_w32`)
  resolve welfare-positive at N=20 vs the canonical (d8, t50, w32)
  baseline per TEST-03 BACKED.

The 4 cells whose sign flipped between accumulator and Family B
(`eip1559_d4_t50_w32`, `eip1559_d8_t25_w32`,
`rb_reserved_x4_rb_quarter`, `partitioned_x4_rb_quarter`) are
characterised at N=20: the two single-lane cells land BACKED
welfare-positive; the two RB-quarter cells WEAK with CI straddling
zero. Any Family-A-vs-Family-B discussion should report the N=20
verdict explicitly.

## Trust framework

Three validity layers, interpreted per-claim:

- **Internal validity (HIGH baseline).** REVIEW.md established that
  the pricing kernel, mempool gate, and event-stream hashing are
  tight. WR-1 is RESOLVED by Family B's chain-derived design. WR-2
  (admission-rejection diagnostics) and WR-7 (actor-component
  allocation amplification) remain disclosure-only on the register.
- **External validity (MEDIUM baseline).** The audit
  ([cardano-realism-audit.md](cardano-realism-audit.md)) identifies
  disclosure items across four categories (fee field, controller
  calibration, topology / actor model, demand). The standard footer
  applies universally and is not re-listed inline.
- **Conclusion-specific validity.** The unpinned demand-regime
  suites run at 3 seeds × 2000 slots; Phase 3 promoted 5 directly-
  tested canonical cells to N=20 BCa CIs. Determinism intra-arch
  only. *Shape* claims (sign, ordering) are well-supported at 3
  seeds for any suite passing the hash-diversity gate; *magnitude*
  claims demand the BCa CI evidence.

**4-level scale.**

- **HIGH** — Phase 3 N=20 BCa CI evidence; CI excludes zero;
  publication-ready with the standard footer only.
- **MEDIUM** — robust against most threats; 1–2 specific caveats.
- **LOW** — direction or shape materially sensitive to a disclosure
  item; recast as exploratory or pair with sensitivity sweep. Also
  applies to suites whose pre-Phase-3 framing was statistically
  refuted by Phase 3 (the RB-reserved underperform finding at
  TEST-04).
- **UNRESOLVED** — historically used for suites awaiting output
  read. **No suite carries UNRESOLVED in this refresh**: the four
  previously-UNRESOLVED non-pinned suites have refreshed verdicts
  derived from Phase 2's output-read pass (Plan 02-02; see
  [`coverage-check.md`](coverage-check.md) CLM-39 / CLM-40 / CLM-46
  / CLM-48 + adjacent rows promoting UNBACKED → WEAK).

**Common cross-suite facts** (true of all 19 unless noted):

- Seeds: `[1, 2, 3]` for unpinned demand-regime suites and the M3 /
  M4 goldens-pinned suites; the 5 canonical Phase 3 cells run at
  N=20 with BCa CIs.
- Slots: 2000 (~10 min simulated time at 0.5 s/slot).
- Topology:
  `parameters/phase-2-sweep/topology-realistic-100.yaml`.
- Protocol: `protocol-base.yaml` unless an RB-reduced overlay
  override is set per-job.

## Per-suite claims and trust ratings

### M3 suites — single-lane EIP-1559 mechanism characterisation

#### `phase-2-eip1559-robustness.yaml`

- **Demand:** `paper_like_congested` (phased 300/600/200 tx/slot).
- **Question:** Does single-lane EIP-1559 behave robustly across the
  `D × target` sweep ({4, 8, 16} × {0.25, 0.5, 0.75}, 5 jobs swept
  on the diagonal)?
- **Claim it would license:** "Single-lane EIP-1559 produces stable,
  load-tracking quotes across the deployed parameter range; no
  parameter combination collapses pathologically under sustained-
  overload paper-like demand."
- **Internal threats:** WR-4 (u128 saturation in
  `Eip1559Pricing::step`) is mitigated by the M3-applied bound
  (`window × target_num × D ≤ 2^23`).
- **External threats:** Audit external item #1 (window length 32
  unanchored — `RSK-un-anchored-controller-knobs`) applies.
- **Statistical:** 5 jobs × 3 seeds = 15 runs. TEST-03 re-ran the
  two sign-flip cells (`d4_t50_w32`, `d8_t25_w32`) at N=20 BCa CIs.
- **Related RSK:** `RSK-un-anchored-controller-knobs`,
  `RSK-three-seed-statistical-power`, `RSK-single-seed-precision`,
  `RSK-substrate-scope`, `RSK-leios-spec-pre-deployment`.
- **Related CLM:** CLM-05, CLM-10, CLM-11, CLM-18.
- **Phase 3 evidence:** `cell_eip1559_d4_t50_w32` **BACKED** (BCa
  CI [+3.38e+09, +1.35e+10], median +5.37e+09 vs the d8_t50_w32
  baseline, sign-coherence 0.75, hash-diversity 20/20);
  `cell_eip1559_d8_t25_w32` **BACKED** (CI [+4.68e+08, +5.66e+09],
  median +7.81e+07, sign-coherence 0.55). Both single-seed
  "welfare-negative collapse" claims were single-seed artefacts;
  both cells resolve welfare-positive at N=20 vs the canonical
  baseline.
- **Trust:** **MEDIUM** for the suite as a whole; the two specific
  cells Phase 3 directly tested carry **BACKED** rows.
- **Caveats:** (a) window length 32 sub-knob unanchored; (b) the 3
  un-tested jobs (`d8_t50_w32` baseline, `d8_t75_w32`,
  `d16_t50_w32`) carry 3-seed evidence only; (c) WR-1 RESOLVED via
  Family B.

#### `phase-2-eip1559-smoothing.yaml`

- **Demand:** `paper_like_congested`.
- **Question:** How sensitive is single-lane EIP-1559 to window
  length (16 / 32 / 64)?
- **Claim it would license:** "Window length is a smooth tuning
  parameter for single-lane EIP-1559 within {16, 32, 64} under
  paper-like-congested demand; the default of 32 is not a
  knife-edge."
- **Internal threats:** None specific beyond substrate-scope.
- **External threats:** This suite *is* the sensitivity sweep that
  partially answers audit external item #1 (window length 32
  unanchored); the sweep does not bracket `window = 1` (Ethereum
  unwindowed) or `window = 128` (over-smoothed).
- **Statistical:** 3 jobs × 3 seeds = 9 runs. Phase 3 did not re-run
  at N=20.
- **Related RSK:** `RSK-un-anchored-controller-knobs`,
  `RSK-three-seed-statistical-power`, `RSK-substrate-scope`.
- **Related CLM:** (no direct backing-suite row; informs CLM-05 /
  CLM-18 framing).
- **Phase 3 evidence:** Indirectly informed by TEST-03's resolution
  of the `d8_t25_w32` and `d4_t50_w32` cells (both BACKED at N=20).
- **Trust:** **MEDIUM**.
- **Caveats:** (a) endpoints `window = 1` and `window = 128`
  un-anchored; (b) congested demand only; (c) WR-1 RESOLVED via
  Family B.

### M4 suites — two-lane mechanism comparisons

#### `phase-2-priority-only-rb-reserved.yaml`

- **Demand:** `paper_like_congested`.
- **Question:** How does RB-reserved priority-only-static behave
  across `multiplier_floor ∈ {4, 8, 16}`?
- **Claim it would license:** "RB-reserved priority-only-static
  delivers price-discriminated service across the multiplier-floor
  sweep; floor magnitude controls the share of demand that
  self-selects into priority but does not break the partition rule."
- **Internal threats:** None specific beyond substrate-scope; the
  partition validity rule (`LaneValidityRule::PriorityOnly`)
  excludes standard-fee txs from RB by construction.
- **External threats:** Audit external item #3 (spec-default floor =
  16 unanchored) is *itself answered* by this suite's sweep.
  Anti-bribery property is load-bearing on the honest-producer
  assumption (`RSK-partition-activated-honest-producer`).
- **Statistical:** 3 jobs × 3 seeds = 9 runs. TEST-04
  (`menu_rb_reserved_priority_only_static_x4`) and TEST-03
  (`cell_rb_reserved_x4_rb_quarter`) re-ran at N=20 BCa CIs.
- **Related RSK:** `RSK-partition-activated-honest-producer`,
  `RSK-un-anchored-controller-knobs`, `RSK-single-seed-precision`,
  `RSK-multiplier-floor-4-suite-coverage`,
  `RSK-three-seed-statistical-power`, `RSK-substrate-scope`.
- **Related CLM:** CLM-02, CLM-06, CLM-12, CLM-16, CLM-19, CLM-24,
  CLM-29, CLM-34, CLM-41.
- **Phase 3 evidence:** `menu_rb_reserved_priority_only_static_x4`
  **BACKED** at N=20: BCa CI [−6.02e+09, −1.00e+09], median Δ =
  −4.15e+09 vs single-lane EIP-1559 control at `sundaeswap_moderate
  × multiplier_floor=4`. Sign-coherence 0.65. **REFUTES the
  pre-Phase-3 framing that RB-reserved priority-only outperforms
  single-lane EIP-1559** — the arm UNDERPERFORMS by ~4e+09 retained
  value (CLM-06). `cell_rb_reserved_x4_rb_quarter` **WEAK** (CI
  [−1.50e+09, +2.18e+09] crosses zero; CLM-12).
- **Trust:** **LOW** for the welfare-vs-single-lane claim shape
  (CLM-06 refuted at N=20 BCa); **MEDIUM** for the structural
  price-discrimination / partition-rule claim shape (CLM-19, CLM-24,
  CLM-29, CLM-34 BACKED structurally).
- **Caveats:** (a) "two-lane outperforms single-lane" REFUTED at
  `sundaeswap_moderate × floor=4`; rephrase as "delivers
  anti-bribery and price discrimination by construction but does
  not improve welfare over single-lane EIP-1559 at this
  calibration"; (b) anti-bribery formal only under honest-producer;
  (c) corner-stress `x4_rb_quarter` welfare-indeterminate; (d) WR-1
  RESOLVED via Family B.

#### `phase-2-priority-only-unreserved.yaml`

- **Demand:** `paper_like_congested`.
- **Question:** Same as RB-reserved variant but without the partition;
  priority delivery is producer-side `priority_first` block-build
  ordering only.
- **Claim it would license:** "Un-reserved priority-only-static
  produces price discrimination via fee economics alone; across
  multiplier-floor ∈ {4, 8, 16}, priority inclusion is preferential
  but not on-chain-validated."
- **Internal threats:** None specific beyond substrate-scope.
- **External threats:** Audit external item #4 (un-reserved priority
  signal source = option 1, `priority_paying_bytes /
  total_block_capacity`) is the load-bearing lane-signal-source
  sub-knob of `RSK-un-anchored-controller-knobs`.
- **Statistical:** 3 jobs × 3 seeds = 9 runs. TEST-04 canonical cell
  ran at N=20.
- **Related RSK:** `RSK-un-anchored-controller-knobs`,
  `RSK-single-seed-precision`, `RSK-three-seed-statistical-power`,
  `RSK-substrate-scope`.
- **Related CLM:** CLM-01, CLM-07, CLM-14, CLM-20, CLM-25, CLM-30,
  CLM-35, CLM-42.
- **Phase 3 evidence:** `menu_unreserved_priority_only_static_x4`
  **BACKED** at N=20: BCa CI [+4.28e+09, +8.49e+09], median Δ =
  +6.66e+09 vs single-lane control at `sundaeswap_moderate × floor=4`.
  Sign-coherence 0.90. The pre-Phase-3 "un-reserved priority-only
  outperforms single-lane EIP-1559" framing is **confirmed** at N=20
  (CLM-07).
- **Trust:** **HIGH** for the welfare-vs-single-lane claim shape at
  `sundaeswap_moderate × floor=4` (CLM-07 BACKED at N=20 with
  sign-coherence 0.90); **MEDIUM** for the broader floor sweep (the
  {x8, x16} jobs carry 3-seed evidence only).
- **Caveats:** (a) results condition on the option-1
  lane-signal-source; (b) anti-bribery is `informal` (fee economics
  + multiplier-floor, not partition rule); (c) N=20 confirmation is
  at `sundaeswap_moderate × floor=4`; other demand profiles
  documented with 3-seed evidence in the demand-regime suites; (d)
  WR-1 RESOLVED via Family B.

#### `phase-2-two-lane-both-dynamic.yaml`

- **Demand:** `paper_like_congested`.
- **Question:** Both-dynamic in partitioned and un-partitioned forms
  at multiplier-floor ∈ {4, 16}.
- **Claim it would license:** "Both lanes dynamic honours the
  multiplier-floor invariant without runaway divergence; partitioned
  and un-partitioned forms produce qualitatively distinct welfare
  under congested demand."
- **Internal threats:** IN-4 (multiplier-floor ratio bound) bounded
  at 2^32 in `TwoLaneSettings::validate`.
- **External threats:** Audit external item #4 (both-dynamic standard
  signal source = `standard_paying_bytes /
  eb_referenced_txs_max_size_bytes`, no standard sample on
  RB-reserved RBs) load-bearing.
  `RSK-standard-user-fee-drift-exposure` applies to un-partitioned
  variants.
- **Statistical:** 4 jobs × 3 seeds = 12 runs. TEST-04 canonical
  cells (partitioned + un-partitioned at floor=4) and TEST-03
  `cell_partitioned_x4_rb_quarter` re-ran at N=20.
- **Related RSK:** `RSK-un-anchored-controller-knobs`,
  `RSK-standard-user-fee-drift-exposure`,
  `RSK-partition-activated-honest-producer` (partitioned variants),
  `RSK-single-seed-precision`, `RSK-three-seed-statistical-power`,
  `RSK-multiplier-floor-4-suite-coverage`, `RSK-substrate-scope`.
- **Related CLM:** CLM-03, CLM-04, CLM-08, CLM-09, CLM-13, CLM-15,
  CLM-17, CLM-21, CLM-22, CLM-26, CLM-27, CLM-31, CLM-32, CLM-36,
  CLM-37, CLM-43, CLM-44.
- **Phase 3 evidence:** `menu_unreserved_both_dynamic_x4`
  **BACKED**: BCa CI [+5.65e+09, +1.09e+10], median Δ = +7.95e+09 vs
  single-lane control (sign-coherence 0.90; CLM-09 confirmed).
  `menu_rb_reserved_both_dynamic_x4` (partitioned) **BACKED**: CI
  [−5.95e+09, −8.87e+08], median Δ = −4.15e+09 — direction
  REVERSED (CLM-08 REFUTED). The cross-arm duplicate-job artefact
  (partitioned ≡ RB-reserved priority-only welfare at
  `sundaeswap_moderate × floor=4`) replicates at N=20 because the
  standard quote never drifts off the multiplier floor.
  `cell_partitioned_x4_rb_quarter` **WEAK** (CI [−1.61e+09,
  +2.14e+09]; CLM-13).
- **Trust:** **HIGH** for the un-partitioned welfare-vs-single-lane
  claim at `sundaeswap_moderate × floor=4` (CLM-09 BACKED at N=20);
  **LOW** for the partitioned welfare-vs-single-lane claim under the
  same calibration (CLM-08 REFUTED); **MEDIUM** for the structural
  multiplier-floor-invariant / `partition_activated` honour claim
  (CLM-21, CLM-22, CLM-36, CLM-37 BACKED structurally).
- **Caveats:** (a) partitioned welfare claim REFUTED at
  `sundaeswap_moderate × floor=4`; same magnitude as RB-reserved
  priority-only per cross-arm duplicate-job artefact; (b)
  un-partitioned exposes standard users to controller drift
  (RSK-standard-user-fee-drift-exposure); drift magnitude bound is a
  disclosed gap; (c) {4, 16} floor sweep narrower than priority-only
  suites' {4, 8, 16}; (d) WR-1 RESOLVED via Family B.

#### `phase-2-rb-scarcity.yaml`

- **Demand:** `paper_like_congested`.
- **Question:** How does priority-lane access degrade as RB body
  capacity shrinks (baseline / half / third / quarter)?
- **Claim it would license:** "Under sustained overload demand and
  `multiplier_floor = 4`, priority retained_value scales smoothly
  with RB body capacity — but this finding is regime-dependent on
  `multiplier_floor = 4`."
- **Internal threats:** None specific beyond substrate-scope.
- **External threats:** Audit external item #2 (`multiplier_floor =
  4` as calibration accommodation —
  `RSK-multiplier-floor-4-suite-coverage`) is load-bearing.
- **Statistical:** 4 jobs × 3 seeds = 12 runs. TEST-07a re-ran at
  floor=16 with N=3 (qualitative replication; no BCa CI gate).
- **Related RSK:** `RSK-multiplier-floor-4-suite-coverage`,
  `RSK-partition-activated-honest-producer`,
  `RSK-un-anchored-controller-knobs`,
  `RSK-three-seed-statistical-power`, `RSK-substrate-scope`.
- **Related CLM:** (no direct backing-suite citation; informs the
  regime-dependence framing for CLM-06, CLM-08, CLM-12, CLM-13).
- **Phase 3 evidence:** TEST-07a found the rb-scarcity finding is
  regime-dependent: floor=4 → "standard dominates welfare, RB
  scarcity mostly invisible"; floor=16 → "priority captures
  everything; total welfare drops 93–98%; RB scarcity is the binding
  constraint". Verdict: **LIVE → DISCLOSED** (the floor=4 finding
  does not generalise to floor=16).
- **Trust:** **LOW**. Conclusion conditions on
  `multiplier_floor = 4` being load-bearing; the floor=16 companion
  shows the finding inverts at the spec default.
- **Caveats:** (a) entire conclusion conditions on
  `multiplier_floor = 4`; (b) published framing must lead with the
  regime-dependence; (c) WR-1 RESOLVED via Family B.

#### `phase-2-urgency-inversion.yaml`

- **Demand:** `paper_like_congested` vs `paper_like_mispriced` (2
  jobs only).
- **Question:** Do mis-priced (`MaxFeePolicy = {1, 1}`) high-urgency
  actors lose priority service to correctly-priced lower-urgency
  actors under sustained drift?
- **Claim it would license:** "Mis-pricing the hard-deadline
  component's max-fee policy at `{1, 1}` (zero quote-drift headroom)
  inflates the mispriced actor's measured retained_value at
  `multiplier_floor = 4` — but only because the priority quote barely
  rises above the floor and over-payments don't get charged extra.
  Finding weakly reverses at `multiplier_floor = 16`."
- **Internal threats:** WR-2 (no admission-rejection reason
  distinction — `RSK-admission-rejection-attribution`) is acutely
  relevant: the cause of component-0 eviction (gate-reject vs
  revalidation-evict) is exactly the diagnostic the suite needs to
  attribute the effect cleanly.
- **External threats:** Audit external item #2 (`multiplier_floor =
  4` — `RSK-multiplier-floor-4-suite-coverage`) load-bearing. Audit
  demand item #1 (default `max_fee_policy = {4, 1}` —
  `RSK-max-fee-policy-default`) is *exactly the assumption this
  suite probes*.
- **Statistical:** 2 jobs × 3 seeds = 6 runs. TEST-07a re-ran at
  floor=16 with N=3 seeds.
- **Related RSK:** `RSK-multiplier-floor-4-suite-coverage`,
  `RSK-max-fee-policy-default`,
  `RSK-admission-rejection-attribution`,
  `RSK-un-anchored-controller-knobs`,
  `RSK-three-seed-statistical-power`, `RSK-substrate-scope`.
- **Related CLM:** (no direct backing-suite citation; informs the
  regime-dependence narrative).
- **Phase 3 evidence:** TEST-07a found the finding **weakly reverses**
  at floor=16: at floor=4, mispriced (`{1,1}`) > correctly-priced
  (`{4,1}`) because priority quote barely rises above the floor; at
  floor=16, high-urgency over-spending is more expensive, so
  correctly-priced > mispriced by ~13%. Verdict: **LIVE → DISCLOSED
  with reframe**.
- **Trust:** **LOW**. Same floor-conditionality as `rb-scarcity`
  plus the WR-2 attribution gap.
- **Caveats:** (a) conditions on `multiplier_floor = 4`; floor=16
  weakly reverses; (b) WR-2 attribution gap; (c) framing must lead
  with "mispriced max-fee policies win only when the floor is low
  enough that priority stays near floor and over-payments aren't
  charged extra"; (d) WR-1 RESOLVED via Family B.

### Demand-regime suites — four demand profiles × three mechanism arms

The 12 demand-regime suites cross four demand profiles —
`paper_like_congested` (300/600/200 phased overload),
`paper_like_moderate` (~25 tx/slot, no congestion),
`paper_like_realistic` (~150 tx/slot, DeFi-heavy),
`sundaeswap_moderate` (SundaeSwap January 2022 launch reference) —
against three mechanism arms (`singlelane`, `priority-only`,
`both-dynamic`). None of the 12 are goldens-pinned; they ran at 3
seeds during the Phase 2 output-read pass (Plan 02-02), which
promoted UNBACKED rows to WEAK across the coverage table per
[`coverage-check.md`](coverage-check.md). The four suites previously
rated UNRESOLVED (`moderate-priority-only`, `moderate-both-dynamic`,
`realistic-both-dynamic`, `sundaeswap-both-dynamic`) carry refreshed
MEDIUM verdicts derived from those CLM-row promotions.

Job structure per arm: `singlelane` 7 jobs (flat-fee + 6 EIP-1559
settings); `priority-only` 16 jobs (rb-reserved × {4,8,16} ×
{default, half, third, quarter} = 12, plus unreserved × {4,8,16} =
3, plus flat-fee); `both-dynamic` 10 jobs (partitioned × {4,16} ×
{default, half, third, quarter} = 8, plus unreserved × {4,16}).

#### `phase-2-congested-singlelane.yaml`

- **Demand:** `paper_like_congested`. **Jobs × seeds:** 7 × 3.
- **Claim:** "Under sustained-overload congested demand, single-lane
  EIP-1559 outperforms the flat-fee baseline on retained-value-ratio
  across the deployed-EIP-1559 parameter range."
- **Threats:** Audit external item #1 (window 32) partially answered
  by the included window sweep.
- **Related RSK:** `RSK-un-anchored-controller-knobs`,
  `RSK-three-seed-statistical-power`, `RSK-substrate-scope`,
  `RSK-leios-spec-pre-deployment`.
- **Related CLM:** CLM-52.
- **Trust:** **MEDIUM**. Flat-fee-vs-dynamic comparison is the
  most-anchored claim shape (both arms in the same suite).
- **Caveats:** standard footer + window length disclosure; WR-1
  RESOLVED via Family B.

#### `phase-2-congested-priority-only.yaml`

- **Demand:** `paper_like_congested`. **Jobs × seeds:** 16 × 3.
- **Claim:** "Under sustained-overload demand, RB-reserved priority-
  only-static delivers urgency separation across `multiplier_floor
  ∈ {4, 8, 16}`, and degrades [smoothly / sharply] as RB body
  capacity shrinks; un-reserved variants produce similar priority-
  lane behaviour without the partition's anti-bribery property."
- **Threats:** Audit external item #3 (spec default 16) answered by
  the floor sweep. RSK-partition-activated-honest-producer applies
  to RB-reserved jobs. External item #4 (un-reserved option-1
  signal) applies to un-reserved jobs.
- **Related RSK:** `RSK-un-anchored-controller-knobs`,
  `RSK-partition-activated-honest-producer`,
  `RSK-three-seed-statistical-power`,
  `RSK-multiplier-floor-4-suite-coverage` (for the x4 jobs),
  `RSK-substrate-scope`.
- **Related CLM:** CLM-50.
- **Trust:** **MEDIUM**.
- **Caveats:** anti-bribery conditional on honest producers;
  un-reserved jobs condition on option-1 signal source; standard
  footer; WR-1 RESOLVED.

#### `phase-2-congested-both-dynamic.yaml`

- **Demand:** `paper_like_congested`. **Jobs × seeds:** 10 × 3.
- **Claim:** "Under congested demand, both-dynamic mechanisms
  (partitioned and un-partitioned) honour the multiplier-floor
  invariant and respond to load on both lanes; RB-capacity
  reductions degrade priority service in partitioned variants while
  un-reserved variants are unaffected by RB cap directly."
- **Threats:** External item #4 (standard signal source) applies to
  all jobs. External item #2 (`multiplier_floor = 4`) paired with
  x16 throughout for sensitivity.
- **Related RSK:** `RSK-un-anchored-controller-knobs`,
  `RSK-standard-user-fee-drift-exposure`,
  `RSK-partition-activated-honest-producer` (partitioned jobs),
  `RSK-multiplier-floor-4-suite-coverage` (for the x4 jobs),
  `RSK-three-seed-statistical-power`, `RSK-substrate-scope`.
- **Related CLM:** CLM-51.
- **Trust:** **MEDIUM**.
- **Caveats:** standard signal source spec-open; floor sweep {4, 16}
  only (no x8 mid-point); WR-1 RESOLVED.

#### `phase-2-moderate-singlelane.yaml`

- **Demand:** `paper_like_moderate` (~25 tx/slot, no congestion).
  **Jobs × seeds:** 7 × 3.
- **Claim:** "Under non-congested demand, single-lane EIP-1559
  produces welfare comparable to flat-fee — the controller stays
  near the era floor and the mechanism imposes no meaningful cost."
- **Threats:** Universal substrate-scope + window-length disclosure;
  the regime is mostly quiescent so the controller barely moves.
- **Related RSK:** `RSK-un-anchored-controller-knobs`,
  `RSK-three-seed-statistical-power`, `RSK-substrate-scope`,
  `RSK-leios-spec-pre-deployment`.
- **Related CLM:** CLM-53.
- **Trust:** **MEDIUM**.
- **Caveats:** null-result claim should be reported as a confidence
  band around zero welfare delta, not a point estimate; with 3 seeds
  the band is wide; WR-1 RESOLVED.

#### `phase-2-moderate-priority-only.yaml`

- **Demand:** `paper_like_moderate`. **Jobs × seeds:** 16 × 3.
- **Claim:** "Under non-congested demand, priority-only-static
  mechanisms see significant priority demand at
  `multiplier_floor = 4` (consistent with the unreserved-priority
  outperforming finding generalising to non-congested regimes); the
  RB-reserved jobs produce welfare-positive single-seed point
  estimates per the Phase 2 output-read."
- **Threats:** External item #3 partially exercised by the floor
  sweep. RSK-partition-activated-honest-producer applies to
  RB-reserved jobs.
- **Related RSK:** `RSK-multiplier-floor-4-suite-coverage`,
  `RSK-un-anchored-controller-knobs`,
  `RSK-partition-activated-honest-producer`,
  `RSK-unresolved-suite-claims`, `RSK-menu-collapse-to-advocacy`,
  `RSK-three-seed-statistical-power`, `RSK-substrate-scope`.
- **Related CLM:** CLM-47.
- **Trust:** **MEDIUM** (refreshed from UNRESOLVED via Phase 2
  output-read; CLM-47 WEAK; further upgrade requires N≥20 BCa
  evidence Phase 3 did not produce).
- **Caveats:** rows are WEAK in `coverage-check.md` pending
  multi-seed evidence; WR-1 RESOLVED.

#### `phase-2-moderate-both-dynamic.yaml`

- **Demand:** `paper_like_moderate`. **Jobs × seeds:** 10 × 3.
- **Claim:** "Under non-congested demand, the both-dynamic
  standard-lane controller does not drift away from the era floor;
  the price experience is indistinguishable from flat-fee for
  standard users. Partitioned both-dynamic produces bounded
  standard-quote drift per CLM-40 / RSK-standard-user-fee-drift-
  exposure."
- **Threats:** External item #4 (standard signal source).
  RSK-standard-user-fee-drift-exposure applies; the suite's whole
  point is the standard-user-drift diagnostic.
- **Related RSK:** `RSK-standard-user-fee-drift-exposure`,
  `RSK-un-anchored-controller-knobs`,
  `RSK-unresolved-suite-claims`,
  `RSK-partition-activated-honest-producer` (partitioned jobs),
  `RSK-three-seed-statistical-power`, `RSK-substrate-scope`.
- **Related CLM:** CLM-40.
- **Trust:** **MEDIUM** (refreshed from UNRESOLVED via Phase 2
  output-read; CLM-40 WEAK; further upgrade requires multi-seed
  evidence Phase 3 did not produce).
- **Caveats:** standard-user drift bound is a disclosed gap; WR-1
  RESOLVED.

#### `phase-2-realistic-singlelane.yaml`

- **Demand:** `paper_like_realistic` (~150 tx/slot, DeFi-heavy).
  **Jobs × seeds:** 7 × 3.
- **Claim:** "Under DeFi-heavy stress-day demand (Q1 2026 mainnet-
  proxy mix), single-lane EIP-1559 tracks load smoothly above the
  flat-fee baseline; controller dynamics are not destabilised by
  the higher-share short-half-life component."
- **Threats:** Audit demand item #1 (mix order-of-magnitude correct
  but not bit-calibrated — `RSK-demand-mix-bit-calibration`).
- **Related RSK:** `RSK-un-anchored-controller-knobs`,
  `RSK-demand-mix-bit-calibration`,
  `RSK-three-seed-statistical-power`, `RSK-substrate-scope`,
  `RSK-leios-spec-pre-deployment`.
- **Related CLM:** CLM-54.
- **Trust:** **MEDIUM**.
- **Caveats:** demand mix is order-of-magnitude correct vs mainnet
  Q1 2026, not bit-calibrated; WR-1 RESOLVED.

#### `phase-2-realistic-priority-only.yaml`

- **Demand:** `paper_like_realistic`. **Jobs × seeds:** 16 × 3.
- **Claim:** "Under DeFi-heavy realistic demand, priority-only-static
  mechanisms produce price-discriminated service across the floor
  sweep; the ~10 % hard-deadline / arb component self-selects into
  priority and is served preferentially."
- **Threats:** External item #3 answered by floor sweep. Demand item
  #3 (arb-tail aspirational under Cardano's eUTxO MEV-resistance —
  the simulator's mainnet-proxy mix forecasts a deployed priority-
  lane population shape that mainnet does not yet exhibit).
- **Related RSK:** `RSK-un-anchored-controller-knobs`,
  `RSK-partition-activated-honest-producer` (RB-reserved jobs),
  `RSK-demand-mix-bit-calibration`,
  `RSK-multiplier-floor-4-suite-coverage` (for the x4 jobs),
  `RSK-three-seed-statistical-power`, `RSK-substrate-scope`.
- **Related CLM:** CLM-48.
- **Trust:** **MEDIUM**.
- **Caveats:** arb-tail component is aspirational; frame as "under a
  hypothetical deployed priority-lane population matching this
  profile"; WR-1 RESOLVED.

#### `phase-2-realistic-both-dynamic.yaml`

- **Demand:** `paper_like_realistic`. **Jobs × seeds:** 10 × 3.
- **Claim:** "Under DeFi-heavy realistic demand, both-dynamic
  mechanisms preserve the multiplier-floor invariant while exposing
  standard users to controller drift; the standard-lane drift
  magnitude under realistic load is exposed per
  RSK-standard-user-fee-drift-exposure but the quantitative bound
  is a disclosed gap."
- **Threats:** External item #4 (standard signal source).
  RSK-standard-user-fee-drift-exposure load-bearing.
- **Related RSK:** `RSK-standard-user-fee-drift-exposure`,
  `RSK-un-anchored-controller-knobs`,
  `RSK-unresolved-suite-claims`,
  `RSK-demand-mix-bit-calibration`,
  `RSK-three-seed-statistical-power`, `RSK-substrate-scope`.
- **Related CLM:** CLM-39.
- **Trust:** **MEDIUM** (refreshed from UNRESOLVED via Phase 2
  output-read; CLM-39 WEAK).
- **Caveats:** standard-lane drift bound is a disclosed gap; WR-1
  RESOLVED.

#### `phase-2-sundaeswap-singlelane.yaml`

- **Demand:** `sundaeswap_moderate` (phased January 2022 launch
  profile). **Jobs × seeds:** 7 × 3.
- **Claim:** "Replaying the SundaeSwap January 2022 congestion event
  under single-lane EIP-1559 produces a controlled fee-rise during
  the spike phase and clean recovery during cooldown; retained-value
  ratio is materially above flat-fee during the spike."
- **Threats:** This profile is the **single most empirically-
  anchored demand source** in phase-2; spike 004's "community-
  recognisable" reference. `RSK-sundaeswap-demand-staleness` applies
  (4-year-old retail spike, not steady-state).
- **Related RSK:** `RSK-sundaeswap-demand-staleness`,
  `RSK-un-anchored-controller-knobs`,
  `RSK-three-seed-statistical-power`,
  `RSK-substrate-scope`, `RSK-leios-spec-pre-deployment`.
- **Related CLM:** CLM-55.
- **Trust:** **MEDIUM** (close to HIGH on demand grounds; pulled
  down by window-length and 3-seed caveats).
- **Caveats:** results are for the SundaeSwap-launch demand shape
  specifically; window length 32 unanchored; WR-1 RESOLVED.

#### `phase-2-sundaeswap-priority-only.yaml`

- **Demand:** `sundaeswap_moderate`. **Jobs × seeds:** 16 × 3.
- **Claim:** "Under the SundaeSwap January 2022 congestion event,
  priority-only-static delivers urgency separation across the floor
  sweep; the spike phase exercises the RB partition rule and the
  cooldown phase exercises EB-below-capacity refund."
- **Threats:** External item #3 answered.
  RSK-partition-activated-honest-producer applies to RB-reserved
  jobs. RSK-sundaeswap-demand-staleness applies.
- **Related RSK:** `RSK-un-anchored-controller-knobs`,
  `RSK-sundaeswap-demand-staleness`,
  `RSK-partition-activated-honest-producer`,
  `RSK-multiplier-floor-4-suite-coverage` (for the x4 jobs),
  `RSK-three-seed-statistical-power`, `RSK-substrate-scope`.
- **Related CLM:** CLM-49.
- **Trust:** **MEDIUM**.
- **Caveats:** SundaeSwap retail spike is congestion-driven, not
  arb-driven, so hard-deadline-arb-tail behaviour is less exercised
  than in `realistic`; WR-1 RESOLVED.

#### `phase-2-sundaeswap-both-dynamic.yaml`

- **Demand:** `sundaeswap_moderate`. **Jobs × seeds:** 10 × 3.
- **Claim:** "Under the SundaeSwap congestion event, both-dynamic
  standard-lane behaviour during the spike phase is bounded by the
  `partition_activated` gate (partitioned variant) or by the
  multiplier-floor invariant (un-partitioned variant); the
  community concern about standard users experiencing fee surges
  during congestion events maps to this suite's standard-quote time
  series. Quantitative drift magnitudes are a disclosed gap pending
  multi-seed evidence."
- **Threats:** External item #4 (standard signal source).
  RSK-sundaeswap-demand-staleness applies.
- **Related RSK:** `RSK-standard-user-fee-drift-exposure`,
  `RSK-un-anchored-controller-knobs`,
  `RSK-sundaeswap-demand-staleness`,
  `RSK-unresolved-suite-claims`,
  `RSK-partition-activated-honest-producer` (partitioned jobs),
  `RSK-multiplier-floor-4-suite-coverage` (for the x4 jobs),
  `RSK-three-seed-statistical-power`, `RSK-substrate-scope`.
- **Related CLM:** CLM-46.
- **Trust:** **MEDIUM** (refreshed from UNRESOLVED via Phase 2
  output-read; CLM-46 WEAK).
- **Caveats:** standard-lane drift bound is a disclosed gap; WR-1
  RESOLVED.

## Aggregate trust summary

| Trust level | Suite count | Suites |
|---|---|---|
| HIGH | 2 | `phase-2-priority-only-unreserved` (for the CLM-07 claim shape at `sundaeswap_moderate × floor=4`), `phase-2-two-lane-both-dynamic` (for the CLM-09 un-partitioned claim shape at `sundaeswap_moderate × floor=4`) — Phase 3 TEST-04 BACKED at N=20 BCa CI with sign-coherence 0.90 |
| MEDIUM | 13 | M3: `phase-2-eip1559-robustness`, `phase-2-eip1559-smoothing`. M4 (structural / partial): `phase-2-priority-only-unreserved` (broader floor sweep; HIGH only for the canonical cell), `phase-2-two-lane-both-dynamic` (structural multiplier-floor-invariant claim; HIGH only for the un-partitioned canonical cell). Demand-regime: `phase-2-congested-singlelane`, `phase-2-congested-priority-only`, `phase-2-congested-both-dynamic`, `phase-2-moderate-singlelane`, `phase-2-realistic-singlelane`, `phase-2-realistic-priority-only`, `phase-2-sundaeswap-singlelane`, `phase-2-sundaeswap-priority-only`. Formerly-UNRESOLVED, refreshed via Phase 2 output-read: `phase-2-moderate-priority-only`, `phase-2-moderate-both-dynamic`, `phase-2-realistic-both-dynamic`, `phase-2-sundaeswap-both-dynamic`. |
| LOW | 4 | `phase-2-priority-only-rb-reserved` (CLM-06 REFUTED at N=20 BCa: the arm underperforms single-lane EIP-1559 at `sundaeswap_moderate × floor=4`), `phase-2-two-lane-both-dynamic` (CLM-08 REFUTED at N=20 BCa for the partitioned variant under the same calibration), `phase-2-rb-scarcity` (regime-dependent on `multiplier_floor = 4`; TEST-07a inversion at floor=16), `phase-2-urgency-inversion` (regime-dependent on `multiplier_floor = 4`; TEST-07a weak reversal at floor=16). |
| UNRESOLVED | 0 | None — the previously-UNRESOLVED suites (`moderate-priority-only`, `moderate-both-dynamic`, `realistic-both-dynamic`, `sundaeswap-both-dynamic`) carry refreshed MEDIUM verdicts derived from Phase 2's output-read. |

Notes: two suites appear in both HIGH and MEDIUM (and
`two-lane-both-dynamic` also in LOW) because a single suite can
license multiple claim shapes at different trust levels. The MEDIUM
row deduplicates per-suite entries at the suite's predominant trust
level.

## Cross-cutting threats

- **WR-1 RESOLVED via Family B.** Controller state lives on the
  canonical chain (`LinearRankingBlock.derived_quote` as a pure
  function of predecessors); orphan blocks carry their own
  `derived_quote` and are discarded with the block. See
  [`.planning/family-b-decision-2026-05-14.md`](../../.planning/family-b-decision-2026-05-14.md).
- **`multiplier_floor = 4` is regime-dependent.** The two
  exclusively-floor=4 suites (`rb-scarcity`, `urgency-inversion`)
  plus floor=4 jobs across demand-regime suites license findings
  that may not generalise to the spec default 16. TEST-07a confirms
  rb-scarcity inverts and urgency-inversion weakly reverses at
  floor=16. `RSK-multiplier-floor-4-suite-coverage` LIVE →
  DISCLOSED with reframe per Phase 4 plan 04-06.
- **Three-seed statistical power is the dominant conclusion-validity
  limit for 14 of 19 suites.** Phase 3 promoted 9 cell-runs to N=20
  BCa CIs; the rest carry 3-seed evidence only. Shape claims are
  well-supported for any suite passing the hash-diversity gate
  (17/17 BACKED-eligible pass); magnitude claims are
  publication-grade only for the 9 cells Phase 3 directly tested.
- **CR-1 (`f64::sqrt`)** asterisks cross-arch reproducibility;
  `RSK-cross-arch-determinism` DISCLOSED.
- **WR-2 (no `AdmissionRejected` event —
  `RSK-admission-rejection-attribution`)** is acutely relevant to
  `phase-2-urgency-inversion`: component-0 eviction attribution is
  indirect without WR-2 fixed.
- **Refund-CIP dependency** (`RSK-fee-as-maxFee-envelope` DISCLOSED)
  is a hard external dependency on every welfare claim.
- **TEST-04 headline distinction** ("un-reserved arms outperform /
  RB-reserved arms underperform single-lane EIP-1559" at
  `sundaeswap_moderate × floor=4`) must propagate into every
  CIP-pasteable disclosure paragraph touching mechanism welfare.

## Recommendations to raise trust

In rough order of leverage:

1. **Run TEST-04 canonical-variance across the other 3 demand
   profiles at N=20** — TEST-04 BCa CIs are pinned only at
   `sundaeswap_moderate × floor=4`; generalisation unknown.
2. **Re-run TEST-05 (pool-number sensitivity)** at 165 fresh
   150-pool runs at `sundaeswap_moderate`. Would flip
   `RSK-pool-count` + `RSK-calibration-stale-stake-snapshot` to
   MITIGATED or harden the DISCLOSED paragraphs.
3. **Re-run TEST-06 (run-length / steady-state) for the 3 remaining
   menu arms** at 2000 / 4000 / 8000 slots; may flip
   `RSK-steady-state-run-length` to MITIGATED.
4. **Resolve CR-1 (`f64::sqrt` → `libm::sqrt`).** Closes cross-arch.
5. **Resolve WR-2 (`AdmissionRejected { reason }`)** before any
   re-run of `urgency-inversion`. Closes eviction-attribution gap.
6. **Run a `multiplier_floor = 8` companion** for `rb-scarcity` and
   `urgency-inversion` to bracket the regime-dependence midpoint.

## Recommended publication framing per claim category

**For HIGH claims (2).** CIP framing: "Un-reserved priority-only
delivers median +6.66e+09 retained_value (95% BCa CI [+4.28e+09,
+8.49e+09], sign-coherence 0.90, N=20) vs single-lane EIP-1559 at
`sundaeswap_moderate × multiplier_floor = 4`; un-reserved
both-dynamic delivers +7.95e+09 (CI [+5.65e+09, +1.09e+10],
sign-coherence 0.90). Both pass COV-05 hash-diversity 20/20.
Published-grade at this calibration; generalisation to other demand
profiles pending multi-seed re-run. *Standard footer applies.*"

**For MEDIUM claims (13).** Lead with per-suite caveats + standard
footer.

**For LOW claims (4).** Recast as conditionally-refuted (the two
N=20-REFUTED claim shapes) or as exploratory regime-dependent (the
two TEST-07a suites). Example for
`phase-2-priority-only-rb-reserved`: "At `sundaeswap_moderate ×
multiplier_floor = 4` and N=20 paired bootstrap, the RB-reserved
priority-only-static arm UNDERPERFORMS single-lane EIP-1559 by
median −4.15e+09 retained_value (95% BCa CI [−6.02e+09, −1.00e+09],
sign-coherence 0.65). The pre-Phase-3 single-seed framing that
'two-lane mechanisms outperform single-lane EIP-1559' is
statistically refuted for the RB-reserved variant under this
calibration; structural anti-bribery
(`LaneValidityRule::PriorityOnly`) and the welfare-trade-off must be
cited together. *Standard footer applies.*" For `phase-2-rb-scarcity`
and `phase-2-urgency-inversion`: lead with the floor=4-vs-floor=16
regime-dependence per TEST-07a.

**For UNRESOLVED claims (0).** None — refreshed via Phase 2
output-read (`coverage-check.md` CLM-39 / CLM-40 / CLM-46 / CLM-47 /
CLM-48 license the four MEDIUM verdicts).

**Standard footer** (paste into every claim):

> *Results are produced by the `dynamic-experiment` branch of the
> Leios `sim-rs/` simulator on `topology-realistic-100.yaml` (100
> nodes; mass-stratified epoch-582 Cardano mainnet snapshot retrieved
> 2026-05-14), at 2000 slots × N seeds per job (N=3 for unpinned
> demand-regime suites; N=20 with BCa 95% paired-bootstrap CIs for
> the 5 canonical Phase 3 cells). The pricing kernel and mempool
> gate are integer / rational / u128 disciplined; reporting outputs
> are f64. Intra-arch determinism pinned by golden hashes against
> the Family B chain-derived controller; cross-arch reproducibility
> not yet proven (`RSK-cross-arch-determinism`) and contingent on a
> small residual `f64::sqrt` site
> (`endorsement_window_priced_blocks`). The transaction fee field is
> reinterpreted as a maxFee envelope; the refund path depends on the
> separate fee-change-return CIP being adopted
> (`RSK-fee-as-maxFee-envelope`). Demand mixes are
> order-of-magnitude correct against Q1 2026 mainnet
> (`RSK-demand-mix-bit-calibration`); the SundaeSwap January 2022
> profile is a 4-year-old retail-spike reference, not steady-state
> (`RSK-sundaeswap-demand-staleness`). See
> `cardano-realism-audit.md`, `realism-risks-register.md`,
> `coverage-check.md`, and `.planning/REVIEW.md` for the full
> disclosure / risk-register / per-claim coverage / internal-validity
> material.*
