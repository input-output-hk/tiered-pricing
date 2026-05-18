# Cardano-realism audit — phase-2 dynamic-pricing simulator

Date: 2026-05-18
Branch: dynamic-experiment
Scope: every calibration choice in `parameters/phase-2-sweep/` and every
modeling assumption in `sim-rs/sim-core/`.
Evidence: 4 spike READMEs under `.planning/spikes/`, cited inline; Phase 3
multi-seed evidence under `.planning/realism-tests/`, cited inline.
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
`.planning/family-b-decision-2026-05-14.md`): every ranking block (RB)
carries its own `derived_quote` as a pure function of canonical
predecessors, the controller advances exactly once per canonical block,
and reorg-safety holds by construction (no node-local mutable controller
state). Protocol-cadence and fee-floor calibrations are literal re-uses
of current Cardano mainnet values, applied as `(value, source,
date-retrieved YYYY-MM-DD)` triples below: `rb-generation-probability =
0.05` (= `activeSlotsCoeff`), `min-fee-a = 44`, `min-fee-b = 155381`,
`maxTxSize = 16384`. The pricing controller inherits Ethereum Improvement
Proposal 1559 (EIP-1559)'s deployed knobs unchanged (`D = 8`, `target =
0.5`, per-priced-block update cadence under Family B). The operational
topology is `topology-realistic-100.yaml` (100-node mass-stratified
mainnet stake curve from epoch 582; retrieved 2026-05-14; downsampled
from the 1,510 Cardano mainnet pools with ≥ 1k ADA active stake), not
the historical single-producer overlay.

Three disclosure categories follow, none of which invalidate the
conclusions: (i) the existing Cardano transaction `fee` field is
reinterpreted as a `max_fee_lovelace` envelope rather than the exact
deterministic fee Cardano wallets ship today, and the refund path
depends on a separate fee-change-return CIP; (ii) four pricing-
controller knobs (window length 32; multiplier-floor 4 in two of seven
suites; multiplier-floor 16 as the spec default; lane-signal-source
choices) are addressed under the anchor-or-disclose discipline of Plan
04-01 (one ANCHORED via Reijsbergen / Leonardos / Liu; three DISCLOSED
with sub-knob granularity in `RSK-un-anchored-controller-knobs` of
[`docs/phase-2/realism-risks-register.md`](realism-risks-register.md));
(iii) substrate-scope umbrella for inherited upstream limitations
(`f64` in non-pricing code paths, propagation-model fidelity,
utility-maximising actor model) per `RSK-substrate-scope` in the
register.

Phase 3 multi-seed evidence (N=20 seeds, sundaeswap_moderate demand,
multiplier_floor = 4; results at
[`.planning/realism-tests/multi-seed-variance/results.md`](../../.planning/realism-tests/multi-seed-variance/results.md))
establishes the welfare ranking among the four CIP menu options: the
two **un-reserved menu arms materially outperform single-lane EIP-1559**
(Δ `retained_value` ≈ +6.66e+09 to +7.95e+09, 95% BCa CI excluding
zero, sign-coherence 0.90); the two **RB-reserved menu arms underperform
single-lane EIP-1559** (Δ ≈ −4.15e+09, 95% BCa CI excluding zero,
sign-coherence 0.65). The multiplier-floor 4 calibration itself is
regime-dependent: at multiplier-floor 16 (TEST-07a) the rb-scarcity
finding inverts (priority captures everything; total welfare collapses
93–98%) and the urgency-inversion finding weakly reverses (correctly
priced > mispriced by ~13%).

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

- **Ranking-block (RB) cadence is bit-equal to mainnet Praos.**
  `(rb-generation-probability = 0.05, source:
  docs/phase-2/calibration-fix-postmortem.md citing Cardano mainnet
  activeSlotsCoeff, date-retrieved: 2026-05-14)` equals
  `activeSlotsCoeff = 0.05`; the expected 20-slot RB gap matches
  mainnet's observed ~20.1-second average (~0.5% drift, attributable to
  pool downtime). See
  [Spike 001 §Comparison Table](../../.planning/spikes/001-rb-cadence-and-capacity/README.md)
  rows 1–3.
- **RB body cap is mainnet-current.**
  `(rb-body-max-size-bytes = 90112, source: Cardano mainnet protocol
  parameters since the April-2022 protocol update,
  date-retrieved: 2026-05-14)` is the value set by the April-2022
  protocol update and unchanged since. Spike 001 §Comparison Table row 4.
- **Fee floor matches mainnet to the lovelace.**
  `(min-fee-a = 44, source: Conway-era Cardano mainnet protocol
  parameters, date-retrieved: 2026-05-14)` and
  `(min-fee-b = 155381, source: Conway-era Cardano mainnet protocol
  parameters, date-retrieved: 2026-05-14)` are bit-equal to Conway-era
  mainnet. The Ethereum Improvement Proposal 1559 (EIP-1559) baseline
  initial quote of 44 reproduces today's `minFeeA × bytes` term at
  controller equilibrium; a 200-byte transaction costs exactly 164,181
  lovelace under both. See
  [Spike 002 §Findings](../../.planning/spikes/002-fee-structure-and-mempool-sizing/README.md).
- **`maxTxSize` matches mainnet exactly.**
  `(maxTxSize = 16384 bytes, source: upstream
  sim-rs/parameters/config.default.yaml inherited from
  cardano-node defaults, date-retrieved: 2026-05-14)`. Spike 001
  §Comparison Table.
- **Mempool sizing rule matches mainnet shape.**
  `(mempool-max-total-size-bytes = 2 × eb_referenced_txs_max_size_bytes
  = 24 megabytes (MB), source: derived per the CIP-0164 12 MB
  endorser-block (EB) target combined with the mainnet `2 ×
  one-bearer-block-bytes` rule, date-retrieved: 2026-05-14)`. Both
  networks use `2 × one-bearer-block-bytes` with reject-on-full
  overflow. The absolute byte cap diverges (24 MB vs ~180 KB on
  mainnet) but only because Leios's 12 MB endorser-block (EB) drives
  the bearer-block-size term; the rule itself is identical. Spike 002
  §Comparison Table row "Mempool cap rule".
- **EIP-1559 controller parameters match Ethereum mainnet exactly.**
  `(D = 8, source: Ethereum EIP-1559 specification field
  BASE_FEE_MAX_CHANGE_DENOMINATOR, date-retrieved: 2026-05-13)`;
  `(target = 0.5, source: Ethereum EIP-1559 specification field
  ELASTICITY_MULTIPLIER = 2, date-retrieved: 2026-05-13)`; and the
  per-priced-block update cadence — all present in every baseline
  pricing yet-another-markup-language (YAML) file. The
  `phase-2-eip1559-robustness.yaml` suite sweeps `D ∈ {4, 8, 16}` and
  `target ∈ {0.25, 0.5, 0.75}` bracketing the deployed values for
  sensitivity. Under Family B (chain-derived; committed 2026-05-14 per
  [`.planning/family-b-decision-2026-05-14.md`](../../.planning/family-b-decision-2026-05-14.md))
  the controller advances exactly once per canonical block — the
  EIP-1559-faithful cadence. See
  [Spike 003 §Comparison Table](../../.planning/spikes/003-pricing-controller-calibration/README.md)
  rows 1–3.
- **Leios-specific knobs cite CIP-0164 Table 7 with in-YAML
  provenance.** `(linear-vote-stage-length-slots = 4, source: CIP-0164
  Table 7, date-retrieved: 2026-05-13)`;
  `(linear-diffuse-stage-length-slots = 7, source: CIP-0164 Table 7,
  date-retrieved: 2026-05-13)`;
  `(eb-referenced-txs-max-size-bytes = 12000000, source: CIP-0164 Table
  7, date-retrieved: 2026-05-13)`;
  `(eb-body-validation-cpu-time-ms-per-byte = 2.15e-5, source: CIP-0164
  Table 7, date-retrieved: 2026-05-13)`; `(n = 600, source: CIP-0164
  Table 7 cohort size, date-retrieved: 2026-05-13)`; `(τ = 75%, source:
  CIP-0164 Table 7 quorum threshold, date-retrieved: 2026-05-13)`.
  None are cross-checkable against deployed mainnet (Leios is
  pre-deployment), but each has an explicit "CIP-0164 Table 7"
  comment in the YAML and the Leios Frequently Asked Questions (RB
  ~20 seconds, EB ~5 seconds) corroborates the cadence shape. Caveat
  preserved: these values are conditional on the Leios substrate
  reaching deployment with the specified parameters. Spike 001
  §Findings.
- **Operational topology is mainnet-curve-stratified.**
  `(topology = parameters/phase-2-sweep/topology-realistic-100.yaml,
  source: epoch-582 Cardano mainnet on-chain state snapshot via
  .planning/spikes/006-curve-design/README.md, date-retrieved:
  2026-05-14)`. 100 nodes; same locations / latencies / producers /
  bandwidth as the upstream `parameters/topology.default.yaml`; stake
  values are a mass-stratified downsample of the 1,510 Cardano mainnet
  pools with ≥ 1k ADA active stake, rescaled linearly to total = 3 ×
  10^10 lovelace. Top-1 stake share = 1.97%; Nakamoto coefficient =
  35; Gini = 0.253.

## What needs disclosure

### Fee structure and mempool sizing

1. **Fee-field semantic reinterpretation.** Mainnet `tx.fee` is the
   exact deterministic fee the wallet computed at sign-time; there is
   no `max_fee_lovelace` envelope or refund path. Phase-2 reinterprets
   the same `fee` field as a `max_fee_lovelace` envelope, charges the
   (possibly-lower) current quote at inclusion, and refunds the gap
   via Polina's separate fee-change-return Cardano Improvement Proposal
   (CIP). This is a deliberate mechanism-level change documented in
   [`docs/phase-2/mechanism-design.md`](mechanism-design.md) lines
   39–51 — not a calibration drift — but it is the single most
   user-visible deviation from the world Cardano users have today, and
   the refund path is an external dependency. **Defensible because**
   phase-2's welfare claims explicitly assume the refund mechanism
   exists and the spec is transparent about the reinterpretation. See
   `RSK-fee-as-maxFee-envelope` in
   [`docs/phase-2/realism-risks-register.md`](realism-risks-register.md)
   for the canonical CIP-pasteable disclosure paragraph.

2. **Mempool absolute byte cap is 133× larger than mainnet** (24 MB vs
   ~180 KB).
   `(mempool-max-total-size-bytes = 2 × eb_referenced_txs_max_size_bytes
   = 24 MB, source: derived per the CIP-0164 12 MB EB target,
   date-retrieved: 2026-05-14)` vs Cardano mainnet's current
   `(mempool-cap ≈ 180 KB, source: Cardano mainnet protocol
   parameters / cardano-node defaults, date-retrieved: 2026-05-14)`.
   The cap *rule* matches mainnet (`2 × one-bearer-block-bytes`), but
   Leios's 12 MB endorser-block (EB) drives the bearer-block term to
   24 MB total. A reader who knows mainnet's ~180 KB mempool will be
   surprised. **Defensible because** the rule shape and overflow
   policy match exactly; the absolute number is a downstream
   consequence of CIP-0164's 12 MB EB target, not a different sizing
   philosophy. See `RSK-mempool-cap-magnitude` in the register for the
   disclosure paragraph.

3. **Default `max_fee_policy = {4, 1}` (4× quote-drift headroom) is
   a forecast about post-deployment wallet behaviour, not a
   calibration anchor.** Mainnet wallets today have no analogous knob —
   they ship at the exact deterministic min-fee via
   `cardano-serialization-lib`. Phase-2's 4× headroom is comparable
   to Ethereum's approximate-2× `maxFeePerGas` convention but is not
   measured against Cardano user behaviour (which doesn't exist for
   this knob). **Defensible because** `paper_like_mispriced.yaml`
   uses `{1, 1}` (zero headroom) for the hard-deadline component to
   bound the worst case where users treat phase-2 like mainnet and
   ship at exact min-fee. See `RSK-max-fee-policy-default` in the
   register for the disclosure paragraph.

### Pricing-controller calibration

The pricing controller's core parameters match Ethereum mainnet
bit-exact (`D = 8`, `target = 0.5`, per-priced-block update cadence;
under Family B per
[`.planning/family-b-decision-2026-05-14.md`](../../.planning/family-b-decision-2026-05-14.md)
the controller advances exactly once per canonical block, reorg-safe
by construction). Four controller knobs are not anchored to deployed-
system data and were graded under the anchor-or-disclose discipline
of Plan 04-01 (literature search at the motivating-citation bar; see
[`.planning/phases/04-refresh-and-anchor/04-01-DOC-03-anchor-search.md`](../../.planning/phases/04-refresh-and-anchor/04-01-DOC-03-anchor-search.md)
for the per-sub-knob audit trail and rejected-citations list).

1. **Window length 32 for capacity-varying signals — ANCHORED.**
   `(window length = 32 priced blocks for capacity-varying signals;
   window length = 1 for the RB-reserved priority controller;
   motivating citation: Reijsbergen et al. AFT 2021 §"Short-term
   oscillation"; date-retrieved: 2026-05-13)`. The Reijsbergen et al.
   AFT 2021 finding of chaotic short-term oscillations under
   EIP-1559's per-block-no-smoothing controller motivates introducing
   a smoothing layer; Leonardos et al. AFT 2021 establishes the
   theoretical bounded-oscillation regime under variable demand; Liu
   et al. CCS 2022 reports the empirical counter-bound that this
   theoretical instability has not yet manifested as a mainnet
   usability problem. Phase-2 selects a capacity-weighted window
   over the literature's preferred AIMD response because the
   linear-Leios block-mix (RBs at ~90 KB versus EBs up to 12 MB;
   ratio ≈ 133×) requires capacity-weighting that AIMD does not
   provide. The specific length 32 is a round-number choice within
   the smoothing band; the `phase-2-eip1559-smoothing` suite sweeps
   {16, 32, 64} for sensitivity. See
   `RSK-un-anchored-controller-knobs` sub-knob (a) in
   [`docs/phase-2/realism-risks-register.md`](realism-risks-register.md)
   for the per-sub-knob CIP-pasteable disclosure paragraph.

2. **Multiplier-floor 4 in two suites — DISCLOSED; the calibration
   is regime-dependent.** Multiplier-floor 4 is a calibration
   accommodation chosen to surface controller drift at moderate
   priority demand. TEST-07a (Phase 3,
   [`.planning/realism-tests/multiplier-floor-16-companion/results.md`](../../.planning/realism-tests/multiplier-floor-16-companion/results.md))
   found that at multiplier-floor 16, the `phase-2-rb-scarcity`
   finding inverts ("standard dominates welfare" → "priority captures
   everything; total welfare collapses 93–98%") and the
   `phase-2-urgency-inversion` finding weakly reverses ("mispriced >
   correctly priced" → "correctly priced > mispriced by ~13%").
   Welfare findings from these two suites are conditional on the
   multiplier-floor = 4 calibration. `(multiplier-floor = 4 in
   phase-2-rb-scarcity and phase-2-urgency-inversion;
   multiplier-floor ∈ {4, 8, 16} swept in priority-only suites;
   multiplier-floor ∈ {4, 16} swept in both-dynamic suite; source: no
   external anchor — internal calibration accommodation per CLAUDE.md
   §"Calibration choices"; date-retrieved: —)`. CLAUDE.md states the
   rationale explicitly: at the spec default 16, only urgency≥5
   components find priority attractive and priority demand stays too
   low to surface controller drift in these two suites; at 4,
   urgency≥2 picks priority and the controller drifts. **Defensible
   because** 5 of 7 suites independently cover the spec default 16
   (priority-only at {4, 8, 16}; both-dynamic at {4, 16}), so the
   7-suite design as a whole is robust across the floor sweep, and
   the floor-16 regime-dependence is itself disclosed. See
   `RSK-un-anchored-controller-knobs` sub-knob (b) in the register
   for the per-sub-knob disclosure paragraph;
   `RSK-multiplier-floor-4-suite-coverage` carries the suite-coverage
   restatement of the same risk.

3. **Multiplier-floor 16 (spec default) — DISCLOSED.**
   `(multiplier-floor default = 16 in the spec; source: no external
   anchor — spec-internal "strong price-discrimination" rationale per
   docs/phase-2/mechanism-design.md line 155 and the Calibration-vs-
   Invariant table at line 290; date-retrieved: —)`. The EIP-1559
   academic-critique literature does not extend to second-lane
   controllers and Ethereum has no comparable multiplier floor. The
   phase-2 specification declares 16 as the default without citing
   calibration data; the only stated justification is that 16 gives
   a "strong price-discrimination guarantee". This is a spec-level
   disclosure rather than a simulator-level one — the simulator
   faithfully implements the spec's open-question framing. Welfare
   findings from suites operating at the spec default (the five
   goldens-pinned suites that include an `x16` variant) are reported
   under the spec-stated assumption that 16 gives strong
   discrimination, not under an externally validated calibration.
   Future work calibrating the floor against deployed-system
   telemetry from comparable second-lane mechanisms (none currently
   exist) is flagged as a follow-on item. See
   `RSK-un-anchored-controller-knobs` sub-knob (c) in the register.

4. **Lane-signal-source choices — DISCLOSED.**
   `(un-reserved priority signal source = priority_paying_bytes /
   total_block_capacity (option 1 of three open candidates in
   docs/phase-2/mechanism-design.md lines 207–211); both-dynamic
   standard signal source = standard_paying_bytes /
   eb_referenced_txs_max_size_bytes over endorser blocks (EBs), with
   no standard sample fired on RB-reserved RBs; source: no external
   anchor — internally consistent simplest-choice rationale per
   spike 003; date-retrieved: —)`. The EIP-1559 academic-critique
   literature (Liu et al. CCS 2022; Reijsbergen et al. AFT 2021;
   Leonardos et al. AFT 2021; Roughgarden EC 2021) analyses single-
   lane controllers only and cannot motivate a second-lane signal-
   source choice; no deployed dynamic-pricing system (Sui, Solana,
   NEAR) has a comparable second-lane signal-source choice. The
   phase-2 specification explicitly leaves the un-reserved priority
   signal source open (three candidates at `mechanism-design.md`
   lines 207–211; the specification states "The decision is left to
   a follow-up design pass") and likewise leaves the both-dynamic
   standard signal source open at line 238. The simulator's choice of
   option 1 for un-reserved priority is motivated by simplicity (no
   notional-share knob; no delay-EMA infrastructure); the both-
   dynamic standard side is motivated by the lane-isolation
   invariant (RB-reserved RBs cannot signal standard congestion
   without leaking the partition). Welfare findings from the un-
   reserved priority arm and the both-dynamic standard side are
   conditional on these specific signal-source definitions;
   alternative signal sources were not exercised. See
   `RSK-un-anchored-controller-knobs` sub-knob (d) in the register.

**Umbrella anchor verdict per Plan 04-01:** window-length 32 →
ANCHORED via Reijsbergen / Leonardos / Liu; multiplier-floor 4 →
DISCLOSED (calibration accommodation; regime-dependent at floor 16);
multiplier-floor 16 → DISCLOSED (spec-internal, no external anchor);
lane-signal-source → DISCLOSED (spec-open choice). The umbrella entry
verdict for `RSK-un-anchored-controller-knobs` flips from LIVE to
DISCLOSED rather than to MITIGATED because only one of four sub-knobs
anchors; see
[`docs/phase-2/realism-risks-register.md`](realism-risks-register.md)
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
   2026-05-14)`. Mainnet operates approximately 3,000 stake pools.
   Pool-count sensitivity within the 100-to-150 range is currently
   disclose-only via `RSK-pool-count` in
   [`docs/phase-2/realism-risks-register.md`](realism-risks-register.md)
   per the Phase 3 TEST-05 data-gap disposition (re-run not in
   Phase 4 scope per the phase context); behaviour at deployed-
   mainnet pool counts (~3,000) is DISCLOSED there. The snapshot's
   freshness over a six-month CIP review horizon is bounded by
   `RSK-calibration-stale-stake-snapshot` in the register.

2. **Honest-producer assumption under multi-producer.** The
   operational topology has 100 producer nodes; the
   `partition_activated` bit on `LinearEndorserBlock` is a producer
   claim, not a property derivable from the EB body. Under multi-
   producer with a byzantine producer, the bit could be mis-claimed
   to obtain priority service for standard-fee transactions in the
   same EB. The simulator does not exercise this attack — all
   producers are honest by construction. **Defensible because** the
   fix path (compute the bit from the priority-paying-bytes count in
   the EB body rather than carrying it as a producer claim) is
   straightforward but outside phase-2's scope. See
   `RSK-partition-activated-honest-producer` in the register for the
   honest-producer disclosure paragraph and the body-derivable
   follow-on pointer.

3. **Actor demand-mix is order-of-magnitude correct, not bit-
   calibrated to mainnet share-of-traffic.**
   `(demand-mix shares ≈ 35% smart-contract, 65% transfer; total
   ~30 transactions per second; source: Q1 2026 Cardano mainnet
   transaction-mix order-of-magnitude estimate per
   .planning/spikes/004-topology-and-actor-model/README.md,
   date-retrieved: 2026-05-13)`. The three-component profiles
   (hard-deadline-arb / DeFi / patient) qualitatively match the
   estimate; the log-normal byte-size distribution (median ~930 B)
   is consistent with mainnet's 200–2,000 B typical range; total
   transactions-per-slot rate (~25–150) is within an order of
   magnitude of mainnet's ~30 tx/s. But shares are not bit-
   calibrated. **Defensible because** the M4 / M5 sweeps probe
   demand-shape sensitivity via mispriced overlays and phased
   congestion variants; welfare claims should be reported "under
   this stylised demand mix." See `RSK-demand-mix-bit-calibration`
   in the register for the disclosure paragraph.

4. **`target_inclusion_blocks` defaults are mechanism-induced, not
   mainnet-anchored.**
   `(target_inclusion_blocks: priority = 1 block, standard = 4
   blocks; source: mechanism-induced default — no deployed-mainnet
   anchor because no priority lane exists on mainnet; date-retrieved:
   —)`. These seed the actor's `LatencyEstimator` per (component,
   lane). Standard = 4 models the expected wait when a standard
   transaction might sit behind several priority-only RBs — internal
   to the phase-2 mechanism, not measured on mainnet (where there is
   no priority lane). **Defensible because** observed inclusion
   latencies overwrite the seed once events arrive; the seed only
   shapes the first ~50 slots of actor lane choice. See
   `RSK-target-inclusion-blocks-default` in the register for the
   disclosure paragraph.

5. **Demand non-stationarity at finer than ~2-hour scale is not
   modelled, and MEV / strategic actors are absent.** Phase-2's
   `Phased` arrival-rate machinery captures order-of-2-hours stress
   regimes but not diurnal Coordinated Universal Time (UTC) working-
   hours peaks, non-fungible-token (NFT) drop spikes, or governance-
   deadline pile-ons. Cardano's eUTxO model is structurally MEV-
   resistant (no global mempool), so the absence of strategic-actor
   modelling is *mainnet-faithful in shape* — phase-2's "arb tail"
   component is partly aspirational (a model of what a DEX arb bot
   would look like under a deployed priority lane). The strategic-
   actor gap proper is the canonical formal frame of Chung and Shi
   SODA 2023's impossibility result for joint user-incentive-
   compatibility, miner-incentive-compatibility, and side-contract-
   proofness in transaction-fee mechanisms; phase-2 does not exercise
   strategic / adversarial regimes. **Defensible because** the
   controller-drift timescale is window-length × per-block-cadence
   ≈ 10 minutes, faster than diurnal demand shifts; the mispriced
   suite bounds the worst-case wallet-behaviour assumptions. See
   `RSK-demand-non-stationarity` and `RSK-substrate-scope` sub-point
   (c) in the register for the disclosure paragraphs.

**Substrate-scope umbrella disclosure (engineering-report voice; the
canonical CIP-pasteable prose lives at `RSK-substrate-scope` in
[`docs/phase-2/realism-risks-register.md`](realism-risks-register.md)).**
Phase-2's pricing kernel and mempool gate are integer / rational /
128-bit unsigned (`u128`) by construction (admission, eviction, fee
charging, controller coefficient, mempool tracking, multiplier-floor
invariant, actor lane choice; see CLAUDE.md §"Numeric representation
contract"), but the work inherits the upstream Leios simulator
substrate which carries three categories of unresolved-realism
limitation that the CIP must disclose: **(a) floating-point arithmetic
in non-pricing code paths** — slot lottery, propagation timing,
distribution sampling, and a residual `f64::sqrt` site in
`endorsement_window_priced_blocks` (review finding CR-1) retain `f64`;
intra-architecture determinism on x86_64 / glibc is pinned by golden
hashes, but cross-architecture continuous-integration verification is
disclosed as deferred future work; **(b) propagation-model fidelity**
— the simulator's round-trip-time-driven network model with real-
world-derived latencies stands in for the production Cardano mainnet
propagation reality (geographically distributed pools, varying round-
trip times, dynamic peer selection), reasonable but not validated
against packet-level mainnet propagation traces; **(c) utility-
maximising actor model** — no adversarial / strategic bidders;
Chung and Shi SODA 2023 is the canonical formal frame for the
strategic-bidder regime, and Roughgarden's foundational work on
transaction-fee mechanism design defines the strategic-bidder regime
phase-2 does not exercise. The three sub-points share a single
mitigation path (none — they are inherited substrate and out-of-scope
for re-audit per PROJECT.md Out-of-Scope items 2 and 3); the
disclosure paragraph in `RSK-substrate-scope` is the load-bearing
CIP-pasteable prose. This audit's summary above is the engineering-
report-voice equivalent for self-containment.

## What does NOT transfer cleanly (hard limitations)

No hard limitations identified; all deviations are bounded and
defensible with disclosure. The biggest single risk is the dependency
on Polina's separate fee-change-return CIP for the refund path — but
this is a known external coupling phase-2 has been transparent about
from the start, not a hidden assumption.

## Recommended disclosure statements

The following paragraphs are ready to paste into a "Limitations and
Modeling Assumptions" section of a phase-2 paper / CIP write-up. Tone
is rigorous, not defensive.

**On fee-field semantics.** This work reinterprets the existing
Cardano transaction `fee` field as a maxFee envelope (`max_fee_lovelace`)
rather than the exact deterministic fee a wallet computes at sign-time.
The actual charged amount is the (possibly drifted) current quote at
inclusion, with the gap refunded via the fee-change-return CIP. This
is a mechanism-level change, not a calibration drift; under the
deployed mainnet today, wallets ship exact fees via
`cardano-serialization-lib` and there is no refund path. Our welfare
claims assume this refund mechanism exists. The default actor max-fee
policy (`max_fee_policy = {4, 1}`, i.e. 4× quote-drift headroom) is a
forecast about post-deployment wallet conventions analogous to
Ethereum's ~2× `maxFeePerGas` default; we bound the worst case via
`paper_like_mispriced.yaml`, which uses `{1, 1}` (zero headroom)
modelling users who continue to ship at exact min-fee.

**On controller calibration.** The pricing controller's core parameters
(`D = 8` max-change-denominator, `target = 0.5` fraction-of-capacity,
per-priced-block update cadence) match Ethereum's deployed EIP-1559
exactly. Phase-2 introduces a capacity-weighted aggregation window of
length 32 over priced blocks, departing from EIP-1559's unwindowed
per-parent-block update; this is motivated by linear-Leios's
heterogeneous block sizes (RB ~90 KB vs EB up to 12 MB; 133× capacity
ratio) which the parent-block-only mechanism cannot smooth, and by
academic critique of EIP-1559's short-term oscillation (Reijsbergen et
al., 2021; Liu et al. CCS'22). Window length 32 is a round-number
choice; the `phase-2-eip1559-smoothing` suite sweeps {16, 32, 64} for
sensitivity. The two-lane multiplier-floor parameter (default 16 per
spec, swept at {4, 8, 16} in priority-only suites and {4, 16} in
both-dynamic) has no comparable deployed-system anchor — Ethereum has
no second-lane multiplier. Two suites (`phase-2-rb-scarcity`,
`phase-2-urgency-inversion`) use multiplier-floor 4 exclusively as a
calibration accommodation, and their conclusions should be read as
conditional on that choice.

**On topology.** This phase pins suite goldens to a single-producer
topology (N=1) to isolate the pricing-mechanism welfare question from
slot-battle and fork-resolution dynamics. Mainnet operates ~3,000
stake pools with Nakamoto coefficient ≈ 25, heavy-tailed (Pareto α≈1.4)
stake distribution, and 4-region global geographic spread. We have
generated a mainnet-faithful counterpart topology
(`topology-cip-realistic.yaml`, 600 pools matching CIP-0164's
calibration, with RIPE-Atlas-derived latencies) for any subsequent
cross-topology validation pass, but the phase-2 welfare conclusions
themselves are derived under N=1. The honest-producer assumption is
operationally trivial under N=1; under multi-producer threat models,
the `partition_activated` producer-claim attack surface (the bit is
not body-derivable) becomes relevant and is not exercised by the
current simulator.

**On demand modelling.** The actor model uses three weighted
components per profile (hard-deadline arb / active DeFi / patient
traffic) with fixed urgency families and Poisson arrivals per phase.
Demand shares are order-of-magnitude correct against the Q1 2026
mainnet transaction mix (~35 % smart-contract, ~65 % transfer; total
~30 tx/s) but are not bit-calibrated. Demand non-stationarity is
captured at the ~2-hour phase scale only; diurnal UTC working-hours
peaks, NFT-drop spikes, and governance-deadline pile-ons are not
modelled. The arb-tail component is partly aspirational, modelling a
hypothetical DEX arb bot under a deployed priority lane; Cardano's
eUTxO model is structurally MEV-resistant (no global mempool), so
this archetype does not have a meaningful mainnet calibration anchor.

**On mempool sizing.** Phase-2's default mempool cap of 24 MB is two
orders of magnitude larger than mainnet's ~180 KB, but the sizing
*rule* is identical (`2 × one-bearer-block-bytes` with operator
override). The divergence is driven entirely by Leios's 12 MB EB
target (CIP-0164 Table 7) replacing Praos's 90 KB RB as the bearer
block; under either system the mempool defaults to two bearer-blocks-
worth of capacity with reject-on-full overflow.

## Recommended next steps

- **M6 (already on this branch)** addresses topology and multi-producer
  disclosure items 1–2: `topology-cip-realistic.yaml` is a one-suite-
  config flip away from a multi-producer cross-check pass and could be
  added as an optional welfare-suite re-run in any external write-up.
- **Optional: calibration-sensitivity suite for window length.** The
  smoothing suite already sweeps {16, 32, 64}; extending to {1, 16,
  32, 64, 128} would anchor the choice of 32 across a wider range
  and surface where the controller transitions from
  "Ethereum-equivalent unwindowed" to "phase-2 default" to
  "over-smoothed and slow to react."
- **Optional: strategic-actor demand profile.** A
  `paper_like_strategic.yaml` adding a component with adaptive
  max-fee policy (raises buffer after observing eviction; lowers
  after stable inclusion) would characterise the gap between
  fixed-urgency actors and strategic actors, addressing the most
  significant residual modelling assumption in spike 004.
- **Optional: body-derivable `partition_activated`.** The fix path
  (compute the bit from the priority-paying-bytes count in the EB
  body rather than carrying it as a producer claim) is straightforward
  and would eliminate the highest-priority attack-surface disclosure
  from spike 004's verdict ranking. Outside phase-2's scope but worth
  flagging for a follow-on.
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

## Evidence

- [Spike 001 — RB cadence and capacity](../../.planning/spikes/001-rb-cadence-and-capacity/README.md) — VALIDATED
- [Spike 002 — Fee structure and mempool sizing](../../.planning/spikes/002-fee-structure-and-mempool-sizing/README.md) — NEEDS-DISCLOSURE
- [Spike 003 — Pricing-controller calibration](../../.planning/spikes/003-pricing-controller-calibration/README.md) — NEEDS-DISCLOSURE
- [Spike 004 — Topology and actor model](../../.planning/spikes/004-topology-and-actor-model/README.md) — NEEDS-DISCLOSURE
