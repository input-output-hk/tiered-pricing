# Cardano-realism audit — phase-2 dynamic-pricing simulator

Date: 2026-05-18
Branch: dynamic-experiment
Scope: every calibration choice in `parameters/phase-2-sweep/` and every
modeling assumption in `sim-rs/sim-core/`.
Evidence: 4 spike READMEs under `.planning/spikes/`, cited inline; Phase 3
multi-seed evidence under `../test-results/`, cited inline.
Abbreviations on first use: Cardano Improvement Proposal (CIP), Ethereum
Improvement Proposal 1559 (EIP-1559), ranking block (RB), endorser block
(EB), Stake Pool Operator (SPO), Bias-corrected and accelerated (BCa)
bootstrap, confidence interval (CI), Inter-Quartile Range (IQR),
Conference on Computer and Communications Security (CCS), Advances in
Financial Technologies (AFT), Symposium on Discrete Algorithms (SODA),
Maximum Extractable Value (MEV), extended unspent-transaction-output
(eUTxO), additive-increase / multiplicative-decrease (AIMD), Coordinated
Universal Time (UTC), exponential-moving-average (EMA).

## TL;DR

Phase-2's pricing controller is chain-derived Family B (per
[`.planning/family-b-decision-2026-05-14.md`](../../.planning/family-b-decision-2026-05-14.md)):
every ranking block (RB) carries its own `derived_quote` as a pure
function of canonical predecessors, the controller advances exactly
once per canonical block, and reorg-safety holds by construction.
Protocol-cadence and fee-floor calibrations are literal re-uses of
current Cardano mainnet values, applied as `(value, source, date-
retrieved YYYY-MM-DD)` triples below: `rb-generation-probability =
0.05` (= `activeSlotsCoeff`), `min-fee-a = 44`, `min-fee-b = 155381`,
`maxTxSize = 16384`. The pricing controller inherits Ethereum
Improvement Proposal 1559 (EIP-1559)'s deployed knobs unchanged
(`D = 8`, `target = 0.5`, per-priced-block update cadence under
Family B). The operational topology is `topology-realistic-100.yaml`
(100-node mass-stratified mainnet stake curve from epoch 582;
retrieved 2026-05-14; downsampled from the 1,510 Cardano mainnet
pools with ≥ 1k ADA active stake), not the historical single-producer
overlay.

Three disclosure categories follow: (i) the `fee` field is
reinterpreted as a `max_fee_lovelace` envelope and the refund path
depends on a separate fee-change-return CIP; (ii) four pricing-
controller knobs (window length 32; multiplier-floor 4 in two of
seven suites; multiplier-floor 16 as the spec default; lane-signal-
source choices) addressed under the anchor-or-disclose discipline of
Plan 04-01 (one ANCHORED via Reijsbergen / Leonardos / Liu; three
DISCLOSED with sub-knob granularity in
`RSK-un-anchored-controller-knobs`); (iii) substrate-scope umbrella
for inherited upstream limitations (`f64` in non-pricing code paths,
propagation fidelity, utility-maximising actor model) per
`RSK-substrate-scope` in
[`realism-risks-register.md`](realism-risks-register.md).

Phase 3 multi-seed evidence (N = 20 seeds, sundaeswap_moderate
demand, multiplier_floor = 4) establishes the welfare ranking among
the four CIP menu options: un-reserved arms outperform single-lane
EIP-1559 (Δ `retained_value` ≈ +6.66e+09 to +7.95e+09; 95% BCa CI
excludes zero; sign-coherence 0.90); RB-reserved arms underperform
single-lane EIP-1559 (Δ ≈ −4.15e+09; 95% BCa CI excludes zero;
sign-coherence 0.65). At multiplier-floor 16 (TEST-07a) the rb-
scarcity finding inverts (priority captures everything; total
welfare collapses 93–98%) and the urgency-inversion finding weakly
reverses (correctly priced > mispriced by ~13%) — the floor-4
calibration is regime-dependent.

## Verdict by category

| Category | Verdict | Disclosure level |
|---|---|---|
| RB cadence and capacity | VALIDATED | None — matches mainnet exactly |
| Fee structure and mempool sizing | NEEDS-DISCLOSURE | 3 reinterpretations to state |
| Pricing-controller calibration | NEEDS-DISCLOSURE | Anchor-or-disclose at sub-knob granularity per Plan 04-01: 1 ANCHORED + 3 DISCLOSED (umbrella verdict DISCLOSED) |
| Topology and actor model | NEEDS-DISCLOSURE | Substrate-scope umbrella (`RSK-substrate-scope`): 100-node topology vs mainnet ~3,000 SPOs; honest-producer assumption; demand-mix not bit-calibrated |

## What lines up with mainnet

These are the "phase-2 is mainnet-grounded" anchors a reviewer can verify
against Cardano on-chain state / `cardano-node` directly. Each calibration
value is presented as a `(value, source, date-retrieved YYYY-MM-DD)`
triple.

- **RB cadence is bit-equal to mainnet Praos.**
  `(rb-generation-probability = 0.05, source: Cardano mainnet
  activeSlotsCoeff per docs/phase-2/calibration-fix-postmortem.md,
  date-retrieved: 2026-05-14)`. Expected 20-slot RB gap matches
  mainnet's observed ~20.1-second average (~0.5% drift). Spike 001
  §Comparison Table rows 1–3.
- **RB body cap is mainnet-current.**
  `(rb-body-max-size-bytes = 90112, source: Cardano mainnet protocol
  parameters since the April-2022 update, date-retrieved: 2026-05-14)`.
  Spike 001 §Comparison Table row 4.
- **Fee floor matches mainnet to the lovelace.**
  `(min-fee-a = 44, source: Conway-era Cardano mainnet protocol
  parameters, date-retrieved: 2026-05-14)` and
  `(min-fee-b = 155381, source: Conway-era Cardano mainnet protocol
  parameters, date-retrieved: 2026-05-14)`. The EIP-1559 baseline
  initial quote of 44 reproduces today's `minFeeA × bytes` term at
  controller equilibrium; a 200-byte transaction costs exactly
  164,181 lovelace under both. Spike 002 §Findings.
- **`maxTxSize` matches mainnet exactly.**
  `(maxTxSize = 16384 bytes, source: upstream
  sim-rs/parameters/config.default.yaml inherited from cardano-node
  defaults, date-retrieved: 2026-05-14)`. Spike 001 §Comparison Table.
- **Mempool sizing rule matches mainnet shape.**
  `(mempool-max-total-size-bytes = 2 × eb_referenced_txs_max_size_bytes
  = 24 MB, source: derived per the CIP-0164 12 MB EB target combined
  with the mainnet `2 × one-bearer-block-bytes` rule, date-retrieved:
  2026-05-14)`. The cap rule is identical to mainnet; the absolute
  byte cap diverges (24 MB vs ~180 KB) because Leios's 12 MB EB
  replaces Praos's 90 KB RB as the bearer block. Spike 002 row
  "Mempool cap rule".
- **EIP-1559 controller parameters match Ethereum mainnet exactly.**
  `(D = 8, source: Ethereum EIP-1559 specification
  BASE_FEE_MAX_CHANGE_DENOMINATOR, date-retrieved: 2026-05-13)`;
  `(target = 0.5, source: Ethereum EIP-1559 specification
  ELASTICITY_MULTIPLIER = 2, date-retrieved: 2026-05-13)`. The
  `phase-2-eip1559-robustness.yaml` suite sweeps `D ∈ {4, 8, 16}` and
  `target ∈ {0.25, 0.5, 0.75}`. Under Family B per
  [`.planning/family-b-decision-2026-05-14.md`](../../.planning/family-b-decision-2026-05-14.md)
  the controller advances exactly once per canonical block. Spike 003
  rows 1–3.
- **Leios-specific knobs cite CIP-0164 Table 7.**
  `(linear-vote-stage-length-slots = 4; linear-diffuse-stage-length-slots
  = 7; eb-referenced-txs-max-size-bytes = 12000000;
  eb-body-validation-cpu-time-ms-per-byte = 2.15e-5; n = 600; τ = 75%;
  source: CIP-0164 Table 7, date-retrieved: 2026-05-13)`. None are
  cross-checkable against deployed mainnet (Leios is pre-deployment);
  each has an explicit "CIP-0164 Table 7" comment in the YAML and the
  Leios Frequently Asked Questions (RB ~20 seconds, EB ~5 seconds)
  corroborates the cadence shape. Values are conditional on the Leios
  substrate reaching deployment with the specified parameters; see
  `RSK-leios-spec-pre-deployment` in the register. Spike 001 §Findings.
- **Operational topology is mainnet-curve-stratified.**
  `(topology = parameters/phase-2-sweep/topology-realistic-100.yaml;
  100-node multi-producer; mass-stratified downsample of the 1,510
  Cardano mainnet pools with ≥ 1k ADA active stake at epoch 582;
  top-1 stake share = 1.97%, Nakamoto coefficient = 35, Gini = 0.253;
  source: epoch-582 mainnet snapshot per
  .planning/spikes/006-curve-design/README.md, date-retrieved:
  2026-05-14)`. Stake values rescaled linearly to total = 3 × 10^10
  lovelace; locations / latencies / producers / bandwidth inherited
  from upstream `parameters/topology.default.yaml`.

## What needs disclosure

### Fee structure and mempool sizing

1. **Fee-field semantic reinterpretation.** Mainnet `tx.fee` is the
   exact deterministic fee the wallet computed at sign-time; there is
   no `max_fee_lovelace` envelope or refund path. Phase-2 reinterprets
   the `fee` field as a `max_fee_lovelace` envelope, charges the
   (possibly-lower) current quote at inclusion, and refunds the gap
   via Polina's separate fee-change-return CIP. This is a deliberate
   mechanism-level change documented in
   [`../../docs/phase-2/mechanism-design.md`](../../docs/phase-2/mechanism-design.md) lines
   39–51 — not a calibration drift — but the refund path is an
   external dependency. **Defensible because** phase-2's welfare
   claims explicitly assume the refund mechanism exists and the spec
   is transparent about the reinterpretation. See
   `RSK-fee-as-maxFee-envelope` in
   [`realism-risks-register.md`](realism-risks-register.md)
   for the canonical CIP-pasteable disclosure paragraph.

2. **Mempool absolute byte cap is 133× larger than mainnet.**
   `(mempool-max-total-size-bytes = 24 MB = 2 ×
   eb_referenced_txs_max_size_bytes, source: derived per the CIP-0164
   12 MB EB target, date-retrieved: 2026-05-14)` vs Cardano mainnet's
   `(mempool-cap ≈ 180 KB, source: Cardano mainnet protocol
   parameters / cardano-node defaults, date-retrieved: 2026-05-14)`.
   The cap *rule* matches mainnet (`2 × one-bearer-block-bytes`); the
   absolute number is a downstream consequence of CIP-0164's 12 MB EB
   target, not a different sizing philosophy. See
   `RSK-mempool-cap-magnitude` in the register.

3. **Default `max_fee_policy = {4, 1}` is a forecast, not a
   calibration anchor.** 4× quote-drift headroom; mainnet wallets today
   have no analogous knob (they ship at the exact deterministic
   min-fee via `cardano-serialization-lib`). Phase-2's 4× is
   comparable to Ethereum's approximate-2× `maxFeePerGas` convention.
   **Defensible because** `paper_like_mispriced.yaml` uses `{1, 1}`
   (zero headroom) for the hard-deadline component to bound the worst
   case. See `RSK-max-fee-policy-default` in the register.

### Pricing-controller calibration

Core parameters match Ethereum mainnet bit-exact (`D = 8`,
`target = 0.5`, per-priced-block update cadence; under Family B the
controller advances exactly once per canonical block, reorg-safe by
construction). Four controller knobs are not anchored to deployed-
system data and were graded under Plan 04-01's anchor-or-disclose
discipline (see
[`.planning/phases/04-refresh-and-anchor/04-01-DOC-03-anchor-search.md`](../../.planning/phases/04-refresh-and-anchor/04-01-DOC-03-anchor-search.md)
for the per-sub-knob audit trail and rejected-citations list).

1. **Window length 32 for capacity-varying signals — ANCHORED.**
   `(window length = 32 priced blocks for capacity-varying signals;
   window length = 1 for the RB-reserved priority controller;
   motivating citation: Reijsbergen et al. AFT 2021 §"Short-term
   oscillation"; date-retrieved: 2026-05-13)`. Reijsbergen et al.
   AFT 2021 (chaotic-oscillation finding) + Leonardos et al. AFT
   2021 (bounded-oscillation theoretical) + Liu et al. CCS 2022
   (empirical counter-bound) motivate the *kind* of choice (a
   smoothing layer beyond the unwindowed Ethereum baseline). Phase-2
   picks a capacity-weighted window over the literature's preferred
   AIMD response because the linear-Leios block-mix (RBs ~90 KB
   versus EBs up to 12 MB; ratio ≈ 133×) requires capacity-weighting.
   Length 32 is a round-number choice; the
   `phase-2-eip1559-smoothing` suite sweeps {16, 32, 64} for
   sensitivity. See `RSK-un-anchored-controller-knobs` sub-knob (a)
   in
   [`realism-risks-register.md`](realism-risks-register.md).

2. **Multiplier-floor 4 in two suites — DISCLOSED; regime-dependent
   at floor 16.** TEST-07a (Phase 3,
   [`../test-results/multiplier-floor-16-companion/results.md`](../test-results/multiplier-floor-16-companion/results.md))
   found that at multiplier-floor 16, the `phase-2-rb-scarcity`
   finding inverts ("standard dominates welfare" → "priority captures
   everything; total welfare collapses 93–98%") and the
   `phase-2-urgency-inversion` finding weakly reverses ("mispriced >
   correctly priced" → "correctly priced > mispriced by ~13%").
   Welfare findings from these two suites are conditional on the
   multiplier-floor = 4 calibration. `(multiplier-floor = 4 in
   phase-2-rb-scarcity and phase-2-urgency-inversion;
   multiplier-floor ∈ {4, 8, 16} swept in priority-only suites;
   multiplier-floor ∈ {4, 16} in both-dynamic suite; source: no
   external anchor — internal calibration accommodation per CLAUDE.md
   §"Calibration choices"; date-retrieved: —)`. **Defensible because**
   5 of 7 suites independently cover the spec default 16, and the
   floor-16 regime-dependence is itself disclosed. See
   `RSK-un-anchored-controller-knobs` sub-knob (b) and
   `RSK-multiplier-floor-4-suite-coverage` in the register.

3. **Multiplier-floor 16 (spec default) — DISCLOSED.**
   `(multiplier-floor default = 16 in the spec; source: no external
   anchor — spec-internal "strong price-discrimination" rationale per
   docs/phase-2/mechanism-design.md line 155 and the Calibration-vs-
   Invariant table at line 290; date-retrieved: —)`. The EIP-1559
   academic-critique literature does not extend to second-lane
   controllers and Ethereum has no comparable multiplier floor; the
   spec declares 16 as the default without citing calibration data.
   This is a spec-level disclosure — the simulator faithfully
   implements the open-question framing. See
   `RSK-un-anchored-controller-knobs` sub-knob (c).

4. **Lane-signal-source choices — DISCLOSED.**
   `(un-reserved priority signal source = priority_paying_bytes /
   total_block_capacity (option 1 of three open candidates in
   docs/phase-2/mechanism-design.md lines 207–211); both-dynamic
   standard signal source = standard_paying_bytes /
   eb_referenced_txs_max_size_bytes over endorser blocks (EBs), with
   no standard sample fired on RB-reserved RBs; source: no external
   anchor — simplest-choice rationale per spike 003; date-retrieved:
   —)`. The EIP-1559 academic-critique literature (Liu CCS 2022;
   Reijsbergen AFT 2021; Leonardos AFT 2021; Roughgarden EC 2021)
   analyses single-lane controllers only; no deployed system (Sui,
   Solana, NEAR) has a comparable second-lane signal-source choice.
   The spec leaves both choices open (lines 207–211 and 238). The
   simulator's option-1 choice is motivated by simplicity; the
   both-dynamic standard side is forced by the lane-isolation
   invariant. Welfare findings are conditional on these specific
   signal-source definitions. See `RSK-un-anchored-controller-knobs`
   sub-knob (d).

**Umbrella anchor verdict per Plan 04-01:** 1 ANCHORED (window
length 32) + 3 DISCLOSED (multiplier-floor 4; multiplier-floor 16;
lane-signal-source). The umbrella entry verdict for
`RSK-un-anchored-controller-knobs` flips from LIVE to DISCLOSED
rather than to MITIGATED because only one of four sub-knobs anchors;
see
[`realism-risks-register.md`](realism-risks-register.md)
`RSK-un-anchored-controller-knobs` for the per-sub-knob disclosure
paragraph.

### Topology and actor model

1. **100-node topology versus mainnet ~3,000 stake-pool operators
   (SPOs).**
   `(topology = parameters/phase-2-sweep/topology-realistic-100.yaml;
   100-node multi-producer; mass-stratified downsample of the 1,510
   Cardano mainnet pools with ≥ 1k ADA active stake at epoch 582;
   summary statistics: top-1 stake share = 1.97%, Nakamoto
   coefficient = 35, Gini = 0.253; source: epoch-582 mainnet snapshot
   per .planning/spikes/006-curve-design/README.md, date-retrieved:
   2026-05-14)`. Pool-count sensitivity within the 100-to-150 range
   is currently disclose-only via `RSK-pool-count` per the Phase 3
   TEST-05 data-gap disposition (re-run not in Phase 4 scope);
   behaviour at deployed-mainnet pool counts (~3,000) is DISCLOSED
   there. Snapshot freshness over a six-month CIP review horizon is
   bounded by `RSK-calibration-stale-stake-snapshot`.

2. **Honest-producer assumption under multi-producer.** The
   `partition_activated` bit on `LinearEndorserBlock` is a producer
   claim, not derivable from the EB body. Under multi-producer with
   a byzantine producer, the bit could be mis-claimed to obtain
   priority service for standard-fee transactions in the same EB.
   The simulator does not exercise this attack — all producers are
   honest by construction. **Defensible because** the fix path
   (compute the bit from the priority-paying-bytes count in the EB
   body) is straightforward but outside phase-2's scope. See
   `RSK-partition-activated-honest-producer` in the register.

3. **Actor demand-mix is order-of-magnitude correct, not bit-
   calibrated.** `(demand-mix shares ≈ 35% smart-contract, 65%
   transfer; total ~30 transactions per second; source: Q1 2026
   Cardano mainnet transaction-mix order-of-magnitude estimate per
   .planning/spikes/004-topology-and-actor-model/README.md,
   date-retrieved: 2026-05-13)`. The three-component profiles
   (hard-deadline-arbitrage / decentralised finance (DeFi) / patient)
   qualitatively match; shares are not bit-calibrated. The M4 / M5
   sweeps probe demand-shape sensitivity via mispriced overlays and
   phased congestion variants. See `RSK-demand-mix-bit-calibration`
   in the register.

4. **`target_inclusion_blocks` defaults are mechanism-induced.**
   `(target_inclusion_blocks: priority = 1 block, standard = 4
   blocks; source: mechanism-induced default — no deployed-mainnet
   anchor because no priority lane exists on mainnet; date-retrieved:
   —)`. These seed the actor's `LatencyEstimator`; observed-latency
   EMA overwrites the seed within ~50 slots. See
   `RSK-target-inclusion-blocks-default` in the register.

5. **Demand non-stationarity at finer than ~2-hour scale not
   modelled; MEV / strategic actors absent.** `Phased` arrival
   captures order-of-2-hours regimes but not diurnal UTC working-
   hours peaks, non-fungible-token (NFT)-drop spikes, or
   governance-deadline pile-ons.
   Cardano's eUTxO model is structurally MEV-resistant (no global
   mempool), so the absence of strategic-actor modelling is mainnet-
   faithful in shape. Chung and Shi's *Foundations of Transaction
   Fee Mechanism Design* (SODA 2023) is relevant only as the formal
   frame for the unmodelled strategic-bidder regime: joint
   user-incentive-compatibility, miner/proposer-incentive-
   compatibility, and side-contract-proofness. See
   `RSK-demand-non-stationarity` and `RSK-substrate-scope` sub-point
   (c).

**Substrate-scope umbrella disclosure.** Phase-2's pricing kernel
and mempool gate are integer / rational / 128-bit unsigned (`u128`)
throughout (per CLAUDE.md §"Numeric representation contract"), but
the work inherits the upstream Leios simulator substrate which
carries three categories of unresolved-realism limitation: (a)
floating-point arithmetic in non-pricing code paths (slot lottery,
propagation timing, distribution sampling, plus a residual
`f64::sqrt` site in `endorsement_window_priced_blocks`); intra-
architecture determinism on x86_64 / glibc is pinned by golden
hashes; cross-architecture continuous-integration verification is
deferred future work; (b) propagation-model fidelity — round-trip-
time-driven network model not validated against packet-level mainnet
traces; (c) utility-maximising actor model — no adversarial /
strategic bidders, no MEV strategies. The load-bearing CIP-pasteable
prose lives at `RSK-substrate-scope` in the register.

## What does NOT transfer cleanly (hard limitations)

No hard limitations identified; all deviations are bounded and
defensible with disclosure. The biggest single risk is the dependency
on Polina's separate fee-change-return CIP for the refund path — but
this is a known external coupling phase-2 has been transparent about
from the start, not a hidden assumption.

## Recommended disclosure statements

The following paragraphs are ready to paste into a "Limitations and
Modeling Assumptions" section of a phase-2 paper / CIP write-up. Tone
is rigorous, not defensive. These paragraphs are dual-purpose with
the per-`RSK-NN` disclosure paragraphs in
[`realism-risks-register.md`](realism-risks-register.md);
the audit's paragraphs are engineering-report-voice summaries; the
register's paragraphs are the per-risk canonical CIP-paste source.

**On fee-field semantics.** This work reinterprets the Cardano
transaction `fee` field as a `max_fee_lovelace` envelope rather than
the exact deterministic fee a wallet computes at sign-time. The
actual charged amount is the (possibly drifted) current quote at
inclusion, with the gap refunded via Polina's separate fee-change-
return CIP. Welfare claims assume this refund mechanism exists; on
deployed mainnet today there is no refund path. The default actor
max-fee policy (`{4, 1}`, i.e. 4× quote-drift headroom) is a forecast
analogous to Ethereum's approximate-2× `maxFeePerGas`; the worst case
is bounded via `paper_like_mispriced.yaml` (`{1, 1}` zero headroom).
See `RSK-fee-as-maxFee-envelope` and `RSK-max-fee-policy-default` in
the register.

**On controller calibration.** The pricing controller's core
parameters (`D = 8` max-change-denominator, `target = 0.5` fraction-
of-capacity, per-priced-block update cadence under Family B per
[`.planning/family-b-decision-2026-05-14.md`](../../.planning/family-b-decision-2026-05-14.md))
match Ethereum's deployed EIP-1559 exactly; reorg-safety holds by
construction because every RB carries its own derived quote as a
pure function of canonical predecessors. Phase 3 multi-seed evidence
at multiplier-floor 16 (TEST-07a;
[`../test-results/multiplier-floor-16-companion/results.md`](../test-results/multiplier-floor-16-companion/results.md))
establishes that the multiplier-floor 4 calibration in
`phase-2-rb-scarcity` and `phase-2-urgency-inversion` is regime-
dependent: at floor 16 the rb-scarcity finding inverts ("standard
dominates welfare" → "priority captures everything; total retained
value collapses 93–98%") and the urgency-inversion finding weakly
reverses ("mispriced > correctly priced" → "correctly priced >
mispriced by ~13%"). Four controller knobs were graded under Plan
04-01's anchor-or-disclose discipline: window length 32 ANCHORED via
Reijsbergen et al. AFT 2021 + Leonardos et al. AFT 2021 + Liu et al.
CCS 2022; multiplier-floor 4 DISCLOSED (calibration accommodation;
regime-dependent at floor 16); multiplier-floor 16 DISCLOSED (spec-
internal "strong price-discrimination" rationale; no deployed system
has a comparable second-lane multiplier); lane-signal-source
DISCLOSED (specification leaves the choice open at
`mechanism-design.md` lines 207–211 and 238). See
`RSK-un-anchored-controller-knobs` and
`RSK-multiplier-floor-4-suite-coverage` in the register for the
canonical per-sub-knob disclosure paragraphs.

**On topology.** This phase uses a 100-node mass-stratified mainnet
stake-curve topology (`topology-realistic-100.yaml`), a downsample of
the Cardano mainnet snapshot at epoch 582 (retrieved 2026-05-14 per
[`../../docs/phase-2/calibration-fix-postmortem.md`](../../docs/phase-2/calibration-fix-postmortem.md)).
Mainnet operates approximately 3,000 stake-pool operators (SPOs);
pool-count sensitivity within 100-to-150 is bounded by
`RSK-pool-count`; behaviour at ~3,000 pools is DISCLOSED there;
snapshot freshness over a six-month CIP review horizon is bounded by
`RSK-calibration-stale-stake-snapshot`. The honest-producer
assumption operates by construction; the `partition_activated`
producer-claim attack surface is not exercised — see
`RSK-partition-activated-honest-producer` in the register.

**On demand modelling.** The actor model uses three weighted demand
components per profile (hard-deadline arbitrage / active DeFi /
patient traffic) with fixed urgency families and Poisson arrivals per
phase. Demand shares are order-of-magnitude correct against the Q1
2026 mainnet transaction mix (~35% smart-contract, ~65% transfer;
total ~30 transactions per second) but are not bit-calibrated.
Demand non-stationarity is captured at the approximately-2-hour
phase scale only; diurnal UTC working-hours peaks, NFT-drop spikes,
and governance-deadline pile-ons are not modelled. The arbitrage-
tail component is partly aspirational under Cardano's eUTxO model
(structurally MEV-resistant; no global mempool). See
`RSK-demand-mix-bit-calibration` and `RSK-demand-non-stationarity`
in the register.

**On mempool sizing.** The default mempool cap of 24 MB is two
orders of magnitude larger than mainnet's ~180 KB, but the sizing
*rule* is identical (`2 × one-bearer-block-bytes` with reject-on-
full overflow). The divergence is driven entirely by Leios's 12 MB
EB target (CIP-0164 Table 7) replacing Praos's 90 KB RB as the
bearer block. See `RSK-mempool-cap-magnitude` in the register.

**On the menu-item welfare distinction.** Phase 3 multi-seed evidence
(TEST-04 at N=20 seeds, sundaeswap_moderate demand, multiplier_floor =
4; results at
[`../test-results/multi-seed-variance/results.md`](../test-results/multi-seed-variance/results.md))
establishes the welfare ranking among the four CIP menu options:

- **Un-reserved menu arms materially outperform single-lane
  EIP-1559**: priority-only un-reserved Δ retained_value = +6.66e+09
  (95% Bias-corrected and accelerated (BCa) bootstrap confidence
  interval (CI) [+4.28e+09, +8.49e+09]); both-dynamic un-reserved Δ
  = +7.95e+09 (CI [+5.65e+09, +1.09e+10]). Sign-coherence 0.90 across
  20 seeds.
- **RB-reserved menu arms underperform single-lane EIP-1559 under
  the same calibration**: priority-only RB-reserved Δ = −4.15e+09
  (CI [−6.02e+09, −1.00e+09]); both-dynamic RB-reserved (partitioned)
  Δ = −4.15e+09 (CI [−5.95e+09, −8.87e+08]). The pre-Phase-3 single-
  seed framing "two-lane mechanisms outperform single-lane EIP-1559"
  held only for the un-reserved variants under this calibration.
- **The cross-arm duplicate-job artefact** (partitioned ≡
  RB-reserved welfare at sundaeswap_moderate × multiplier_floor = 4)
  replicates at N = 20 because the standard-lane controller never
  drifts off the multiplier floor under this demand profile —
  calibration-conditional menu indistinguishability worth disclosing.
- Phase 3 hash-diversity gate: 17 of 17 BACKED-eligible cells pass
  at distinct-hash count = N. See
  [`coverage-check.md`](coverage-check.md) and
  `RSK-single-seed-precision` in the register.

## Recommended next steps

- **Hard dependency to flag in any publication.** The fee-field
  reinterpretation depends on Polina's separate fee-change-return
  CIP being adopted; phase-2's welfare conclusions assume the refund
  mechanism exists. This is a hard dependency, not a soft one, and
  should be cited as such in any phase-2 paper or CIP write-up.
- **Documentation residual.** CIP-0164's Table 7 was difficult to
  retrieve cleanly during the audit; an embedded numerical cross-
  reference table in the phase-2 publication would aid future
  auditors who hit the same upstream-rendering problem (spike 001
  §Findings).
- **Pool-count sensitivity above 100 pools is currently disclose-
  only per `RSK-pool-count`.** A future TEST-05 re-run at 100 versus
  150 pools (recipe in
  [`../test-results/pool-number-sensitivity/results.md`](../test-results/pool-number-sensitivity/results.md))
  would replace the disclosure with a MITIGATED verdict if Δ% on
  welfare metrics is within seed-Inter-Quartile Range (IQR).
- **Run-length / steady-state validation is similarly disclose-only
  per `RSK-steady-state-run-length`.** A future TEST-06 re-run recipe
  is at
  [`../test-results/run-length-steady-state/results.md`](../test-results/run-length-steady-state/results.md).

## Evidence

- [Spike 001 — RB cadence and capacity](../../.planning/spikes/001-rb-cadence-and-capacity/README.md) — VALIDATED
- [Spike 002 — Fee structure and mempool sizing](../../.planning/spikes/002-fee-structure-and-mempool-sizing/README.md) — NEEDS-DISCLOSURE
- [Spike 003 — Pricing-controller calibration](../../.planning/spikes/003-pricing-controller-calibration/README.md) — NEEDS-DISCLOSURE
- [Spike 004 — Topology and actor model](../../.planning/spikes/004-topology-and-actor-model/README.md) — NEEDS-DISCLOSURE
