# Spike 004 — Topology and actor model
Date: 2026-05-13
Verdict: NEEDS-DISCLOSURE

> **[Annotation added 2026-05-13]** This spike's audit assumed phase-2
> suites ran on `topology-single-producer.yaml`. As of 2026-05-13, the
> suites have been re-pointed to `topology-realistic-100.yaml`
> (multi-producer, 100 nodes, mass-stratified mainnet stake curve — see
> [`.planning/spikes/006-curve-design/README.md`](../006-curve-design/README.md)).
> The findings below (NEEDS-DISCLOSURE verdict, single-producer
> disclosure ranking item 1, honest-producer item 2) are preserved
> for historical context but are no longer the operational reality.
> The current topology choice is documented in `CLAUDE.md`
> §Calibration choices.

## Spike Question

- **Given** (a) Cardano mainnet's roughly ~3,000 SPOs operating
  ~3,200 stake pools with a non-uniform stake distribution and a
  global geographic spread, and (b) realistic user behaviour —
  multiple wallet apps, automated dApps, MEV-style searchers, mixed
  urgencies — none of which is currently dynamically-priced,
- **When** phase-2 uses `topology-single-producer.yaml` (N=1, single
  `stake=100000`, no propagation latencies between nodes because
  there's only one) with weighted multi-component actors
  (`paper_like_realistic.yaml`, `paper_like_congested.yaml`, etc.)
  where each component has fixed urgency, a per-tx-bytes distribution,
  a `LanePolicy::UtilityMaximising`, and a default
  `max_fee_policy = 4 × quote`,
- **Then** the simplifications are bounded and the simulation's
  conclusions remain valid under the implied limits — or, if not,
  the deviations must be acknowledged and bounded.

## Research

### Cardano mainnet topology

**Stake-pool count.** Cardano mainnet has ~3,200 registered stake
pools; PoolTool reports ~2,930 pools "with a chance to make a block"
(i.e. actively producing) as of 2026. Public-facing summaries cite
"~3,000 SPOs" as the round-number target. The protocol does not
distinguish "active" from "registered" in a single canonical number;
the operating-pool figure depends on whether one counts saturated +
zero-stake pools alike.

**Nakamoto coefficient.** Multiple community sources (AdaPulse,
Cardano Forum) place Cardano's Nakamoto coefficient (a.k.a. MAV —
Minimum Attack Vector) at approximately **25 as of epoch 559**, down
from 28 in epoch 323 (Feb 2022) and 20 around epoch 394. The
Decentralization Analysis of Pooling Behavior paper (DLT 2022)
characterises Cardano as having "relatively low Nakamoto coefficient
that is decreasing over time" despite the open validator set. Large
exchanges concentrate stake: Binance operates ~91 pools and Coinbase
~31, collectively ~12.59% of stake. The Saturday-Tuesday Parameter
Committee meeting notes (cited in spike 001) discuss pledge-leverage
proposals (CIP-50, CIP-75) aimed at this concentration; none in
flight for Conway. **Numeric implication: ~25 entities control 50% of
block production capacity** — substantially more decentralised than
Bitcoin's mining-pool figure (~3) but more concentrated than the
~3,000-pool surface number suggests.

**Stake distribution.** Cardano's k-parameter (current 500) caps the
"optimum" stake per pool at `total_stake / k`; pools saturate above
that and lose reward density, encouraging stake migration. The
phase-2 M6 plan calibrates the CIP-realistic topology to a
**Pareto(α=1.4)** stake distribution across 600 pools, summing to
~3 × 10^10 (mainnet ~22B ADA + headroom). The Pareto-1.4 shape
matches the heavy-tailed empirical distribution documented in the
DLT 2022 paper. **Gini coefficient: not directly published in
search results**; the academic literature characterises the
distribution qualitatively as "relatively high inequality" within
the open validator set.

**Geographic distribution.** Cardano stake pools are documented as
operating across all populated continents. Cardano docs explicitly
guide large SPOs to "disperse their operations across multiple
regions" and "very large operators should aim for a global
presence." The phase-2 M6 topology generator chose **4 regions
(NA-East 30%, EU 30%, NA-West 20%, Asia 20%)** with intra-region
latency 25 ± 5 ms and inter-region latency 80–150 ms by region pair
— matching the CIP-0164 simulator's RIPE Atlas-derived inter-region
budget. IOG's own throughput design cites "300 ms TCP delay across
the globe" as the working bound, and Δp = 5s as the per-block
propagation budget. **Typical transatlantic ping is ~150 ms; CIP-0164
calibrates against this in its RIPE Atlas dataset.**

**CIP-0164 simulation topology.** The CIP-0164 / Linear Leios
simulator (`cardano-scaling/ouroboros-leios`) targets a mainnet-like
topology with 10,000 nodes (pseudo-mainnet) or 750 nodes
(mini-mainnet), each block producer connected to two dedicated
relays, using RIPE-Atlas-derived latencies and cloud-data-centre
lower-bound bandwidth. Phase-2's M6 distilled this down to **600
pools** matching the n=600 vote-probability calibration (`vote-
threshold: 450` = 75% quorum of n=600). The 600-pool topology is
already committed to the branch as
`sim-rs/parameters/phase-2-sweep/topology-cip-realistic.yaml`
(~40,000 lines).

### Honest-producer assumption

Ouroboros Praos (CRYPTO 2018; David, Gaži, Kiayias, Russell)
formalises Cardano's consensus as "adaptively secure as long as the
stakeholder distribution maintains an honest majority of stake."
This is the protocol invariant; **what fraction of stake is
"verifiably honest" is not directly measurable** — the academic
formulation is in terms of an honest stake majority, not an
identifiable subset of pools. In practice, Cardano's MAV ≈ 25
implies that an attacker needing 50% stake would have to coordinate
across ~25 distinct entities, which is the operational decentralisation
guarantee. **No published attack against deployed Praos** invalidates
the honest-majority assumption as of 2026; the security model has
held since Shelley (2020). Phase-2's mechanism-design spec inherits
this assumption silently — the partition-activation bit on
`LinearEndorserBlock` and the EB-validation-at-endorsement check
both presume an honest producer subset, and the simulator's
single-producer topology makes this assumption *operationally
trivial* (N=1, so the producer is by-construction the honest
majority).

### User behaviour on mainnet

**Transaction mix.** Daily transaction volume on Cardano averages
~2.6 million (2026); ~35% are smart-contract interactions, ~65%
simple transfers + native-token sends. ~310,000 native-token
transfer txs daily. ~17,000 Plutus contracts deployed by mid-2025,
~680 new contracts deployed monthly. **DeFi TVL ~$132 M (early
April 2026)**; Minswap dominates DEX TVL, WingRiders ~$5 M TVL.
The Q1 2026 dApp-tx volume surged 49% QoQ per Messari. **Implication
for phase-2: real demand is a mix of low-urgency P2P transfers,
mid-urgency DeFi swaps, NFT mints (bursty), governance votes, and a
small fraction of arb/MEV searchers. Phase-2's three-component
demand profiles approximate this with a hard-deadline/arb tail
(0.25–15 tx/slot, half-life 60 s), active DeFi (9–60 tx/slot,
half-life 5 min), and patient traffic (15.75–75 tx/slot, half-life
1 h).** Numerical shares are not bit-calibrated to observed
mainnet — the moderate profile lands at ~25 tx/slot total and the
realistic profile at ~150 tx/slot, both within the order-of-magnitude
of mainnet's ~30 tx/slot (≈ 2.6 M ÷ 86,400).

**MEV.** Cardano's eUTxO model **does not have a global mempool**;
each node validates locally and there is no equivalent to Ethereum's
pending-tx pool for bots to scan. Genius DEX and similar designs
explicitly market this as MEV-resistance. **Front-running in the
EVM sense (sandwich, sniper bots) is not observed on Cardano.**
Limited MEV-adjacent activity exists around DEX-order-book
arbitrage at slot-leader granularity (a producer can choose which
of their own seen txs to include), but the ~$3 B/year MEV figure
attributed to Ethereum + L2s + Solana in 2026 has no Cardano analogue
of meaningful size. **Implication for phase-2: the "arb tail"
component in the demand profiles is partially an aspirational model
of a hypothetical priority-lane user (DEX arb bot under a deployed
phase-2 mechanism), not a calibration from observed mainnet
behaviour. Spike 002 already flagged the upstream `max_fee` semantic
break; the same caveat applies here for the user-archetype side.**

**Wallet defaults.** Reuse from spike 002: mainnet wallets (Lace,
Eternl, Daedalus, Yoroi) all ship at the exact deterministic min-fee
via `cardano-serialization-lib`; there is no `maxFee` envelope and
no buffer policy. Phase-2's `max_fee_policy = {4, 1}` default (4×
quote-drift headroom) is a future-prediction assumption, not a
calibration from observed mainnet wallet behaviour. The mispriced
suite `paper_like_mispriced.yaml` `{1, 1}` deliberately models the
worst-case "wallet ships at exact-min-fee" archetype.

**Time-of-day patterns.** Cardano sees daily UTC peaks tracking
North-American + European working hours (DEX activity, governance
voting), with quieter Asian/overnight slots. Phase-2's `Phased`
arrival-rate machinery models this as 400-slot intervals (≈ 8000 s
≈ 2.2 h "phases" at 20s/RB) inside `paper_like_congested.yaml` and
`paper_like_mispriced.yaml`. **Phases are coarse and stylised**;
they capture demand non-stationarity at the order-of-2-hours scale
but not finer hourly diurnal patterns or one-off-event spikes
(governance deadlines, NFT drops). The actor model's arrival rate
is otherwise stationary-Poisson per phase.

### Phase-2 actor model surface

**Fixed urgency per component.** Each `ActorComponent` carries a
fixed half-life distribution (`half-life-seconds`), sampled per-tx
into an `urgency: f64` value. Within a component, urgency varies
across txs (log-normal half-life sampling); across components,
urgency families are fixed. **Real users adjust max-fee policy as
they observe the market and develop heterogeneous beliefs about
future congestion**; phase-2 models this through `LatencyEstimator`,
a per-(component, lane) rolling EMA seeded at `priority=1,
standard=4` blocks. The seed shapes early-run lane choice; observed
inclusion latencies overwrite it once events arrive.

**Lane choice.** `LanePolicy::UtilityMaximising { submit_when_
underwater }` picks `posted_lane` by maximising `expected_utility =
retained_value − (min_fee_b + quote × bytes)`, with `retained_value
= value × urgency^(-latency_blocks)`. The math runs through
`libm::pow` + `libm::round` into `i128` lovelace for cross-arch
determinism. M8 introduced `submit_when_underwater = false`
(rational refusal): if both lanes' expected utility is ≤ 0, the
actor doesn't submit. **Real users sometimes act strategically
(bid wars during congestion, hold-and-resubmit patterns); phase-2's
rational-actor model has no strategic component beyond
utility-maximisation.**

**Demand stationarity.** Each component samples arrivals from
`Poisson(λ_phase(slot))` where `λ_phase` is constant within a phase.
Across phases, λ steps discontinuously. **Mainnet's empirical demand
is non-stationary at finer scales** (governance deadlines, NFT
drops, exchange-listing events). Phase-2 captures coarse phase
shocks but not these.

## Comparison Table

| Knob / concept | Phase-2 value | Mainnet value | Δ | Impact on phase-2 conclusions |
|---|---|---|---|---|
| Producer count N | 1 (`topology-single-producer.yaml` — kernel-correctness default for M5 suite goldens) | ~3,200 registered stake pools / ~2,930 actively producing / ~600 in CIP-0164 simulation topology | **−3 orders of magnitude** | Producer-side **slot battles and fork-resolution dynamics are absent by construction** under N=1. The WR-1 pricing-rollback concern (CONCERNS.md) is dormant. Multi-producer behaviour was *not* the phase-2 scope; M6 introduced `topology-cip-realistic.yaml` (600 pools) as the multi-producer counterpart but the suite goldens still pin single-producer. **NEEDS-DISCLOSURE**: phase-2's welfare conclusions are derived under a degenerate-topology assumption. |
| Stake distribution | single node, `stake=100000` (chosen to clear lottery's `target_vrf_stake = stake × probability` truncation at rb-prob=0.05) | Heavy-tailed (Pareto(α≈1.4) per CIP-0164 simulator); Nakamoto coefficient ≈ 25; top-25 entities control 50% of block production | Counter-factual by design | Stake distribution is unmeasurable under N=1 (there's nothing to distribute over). The CIP-realistic topology's Pareto(α=1.4) over 600 pools is the M6 counterpart. **Phase-2's pricing-mechanism conclusions don't rest on stake distribution** (the mechanism prices bytes, not stake), but cross-producer controller-state divergence (each node's local view of the window-aggregate) is invisible under N=1. NEEDS-DISCLOSURE in any paper. |
| Geographic distribution / propagation latencies | No network because N=1; intra-cluster only (`location.cluster: phase-2`) | 4-region global (NA-East/EU/NA-West/Asia); intra-region 25 ms, inter-region 80–150 ms; IOG cites 300 ms TCP-globe bound and Δp=5 s | **No latency model in this topology** | Header diffusion (`header-diffusion-time: 1 s`) has no inter-node effect; gossip propagation in `linear_leios.rs` runs but routes back to the same node. **Pricing-controller convergence (per-block update cadence, 32-block windows) is the dominant timescale; both are slot-scoped, so latency-free is the conservative assumption for the pricing question.** The CIP-realistic 600-pool topology has the full RIPE-Atlas-shaped latency matrix and is available for any future multi-node suite. |
| Honest-producer assumption | Explicit, by construction — the sole producer is the honest majority | Implicit, via Praos honest-stake-majority invariant; no published attack against deployed Praos through 2026 | Sim makes the assumption operationally trivial | The `partition_activated` bit on `LinearEndorserBlock` is a **producer claim, not body-derivable** (CONCERNS.md security note). Under N=1 honest producer, this is sound; under multi-producer with byzantine producers the claim could be lied about. **Any attacker-model write-up must disclose this explicitly.** NEEDS-DISCLOSURE. |
| Actor heterogeneity | 3–4 fixed weighted components per profile (hard-deadline arb / DeFi / patient + optional mispriced overlay), each with log-normal `(size, value, half-life)` distributions | Continuous distribution of user types: P2P transfers, DEX swaps (Minswap, WingRiders, etc.), NFT mints, staking ops, governance votes, native-token transfers, oracle updates | Coarse-grained but order-of-magnitude correct | The three-component shape (arb / DeFi / patient) maps qualitatively to the mainnet mix (smart-contract ~35% / token-transfer ~12% / simple-transfer ~65% by Q1 2026 Messari + Cardano statistics). Bytes-per-tx distribution (log-normal μ=6.833 ⇒ median ~930 B) is consistent with mainnet's typical 200–2,000 B txs. **The phase-2 profiles capture demand-mix order of magnitude but are not bit-calibrated to mainnet shares. NEEDS-DISCLOSURE in welfare claims** ("welfare comparison assumes this stylised demand mix"). |
| Demand stationarity | Constant rate per phase; phases are coarse (~400-slot ≈ 2.2 h intervals in congested/mispriced; constant elsewhere). Poisson within phase. | Non-stationary at multiple scales: diurnal (UTC working-hours peaks), weekly (Monday governance opens), one-off (NFT drops, exchange listings, governance deadlines). | Coarse | Phase-2's phased rates capture the order-of-2-hours stress regime that motivates the controller-drift question; they do **not** model NFT-drop spikes or governance-deadline pile-ons. **NEEDS-DISCLOSURE** for any specific dApp launch / event modelling claim. |
| `max_fee_policy` | Default `ScaledOverLaneQuote { 4, 1 }` (4× quote-drift headroom); `paper_like_mispriced.yaml` uses `{1, 1}` (zero headroom) for the hard-deadline component | No `maxFee` concept on mainnet; wallets compute exact fee via `cardano-serialization-lib` and ship without buffer | Phase-2-only mechanism | Carried over from spike 002 finding: phase-2's `{4, 1}` is a future-prediction assumption about wallet behaviour under a deployed dynamic mechanism. The `{1, 1}` mispriced overlay deliberately bounds the worst case. **NEEDS-DISCLOSURE** in any external write-up: the welfare numbers depend on assumed wallet-policy distribution, not on observed user behaviour. |
| `target_inclusion_blocks` defaults | Priority=1, Standard=4 (seeds the per-(component, lane) `LatencyEstimator`; overwritten by observed latencies on first inclusion) | Under normal mainnet load, inclusion is typically ~1 block; under congestion, multi-block waits do occur but standard inclusion is not segmented from priority because there is no priority lane | Standard=4 is a phase-2 mechanism-induced default, not a mainnet-anchored value | The 4-block standard default models **expected wait when a priority lane exists and a standard tx might wait through several priority-only RBs**. It's mechanism-internal, not user-observed. **The seed shapes early-run lane choice before observations arrive**; M5 calibration showed the seed materially affects which urgency components self-select priority in the first ~50 slots. **NEEDS-DISCLOSURE** as a modelling choice — the seed is not a mainnet measurement, it's a sensible bootstrap for the actor's LatencyEstimator under the phase-2 mechanism. |
| MEV / strategic behaviour | None modelled (actors are utility-maximising under fixed urgency; no front-running, sandwich-attacks, or strategic bid wars) | eUTxO model has **no global mempool**, so EVM-style front-running is structurally absent on mainnet; small slot-leader-discretion MEV exists but no $-billion-scale flow as on Ethereum | Sim's lack of strategic behaviour is mainnet-faithful in shape | The "arb tail" component partially models a hypothetical priority-lane user (DEX arb bot) under a deployed phase-2 mechanism. **No mainnet MEV calibration data exists for this archetype because the priority lane doesn't exist today.** Defensible at this scope; flagged for completeness. |

## Findings

- **Single-producer topology is the largest abstraction in phase-2,
  and it is intentional**. The branch's M5 suite goldens are pinned
  against `topology-single-producer.yaml` to remove slot-battle and
  fork-resolution dynamics from the pricing-mechanism question.
  This is sound *for the pricing-mechanism welfare question*: the
  controller's per-block update cadence and capacity-weighted
  windowing are slot-scoped, and a multi-producer topology would
  add noise (per-node controller divergence, slot-battle siblings
  with different pricing samples) without changing the underlying
  controller behaviour. **Bounded; affects neither calibration
  validity nor pricing-mechanism welfare comparison. NEEDS-DISCLOSURE
  level: high** — any reader assuming "mainnet-like" topology will
  be surprised. The CIP-realistic 600-pool topology already exists
  on this branch (M6) and is available for any future cross-topology
  validation pass.

- **No latency model affects the pricing experiments.** Pricing is
  slot-scoped: the controller updates per priced block; the
  capacity-weighted window aggregates over the last 32 priced
  blocks; the actor's `LatencyEstimator` tracks inclusion delay in
  blocks (not seconds). Inter-node latency would shift per-node
  controller state convergence and slot-battle frequency but would
  not change the controller's per-block reaction. **Bounded;
  unaffected by phase-2's conclusions. Disclosure level: low**
  (already implied by single-producer).

- **The honest-producer assumption is operationally trivial under
  N=1 but is implicit and load-bearing**. The `partition_activated`
  bit on `LinearEndorserBlock` is a producer claim, not derivable
  from the EB body (the CONCERNS.md security note). Under N=1
  honest producer, the claim is by-construction sound. **Under a
  multi-producer threat model, a byzantine producer could mis-claim
  `partition_activated = true` to obtain priority service for
  standard-fee txs in the same EB**. The simulator does not exercise
  this attack. **NEEDS-DISCLOSURE level: high** — any
  attacker-model write-up must surface this. The fix path is
  straightforward (make `partition_activated` body-derivable from
  the priority-paying-bytes count in the EB), but it's outside
  phase-2's scope and was deferred to a follow-on.

- **Actor heterogeneity is order-of-magnitude correct but not
  bit-calibrated to mainnet share-of-traffic**. The three-component
  profiles (arb / DeFi / patient) qualitatively match mainnet's
  Q1 2026 mix (~35% smart contracts, ~65% transfers, with arb a
  small fraction of smart contracts). The byte-size distribution
  (log-normal median ~930 B) is consistent with mainnet's typical
  200–2,000 B txs. **NEEDS-DISCLOSURE level: medium** — for any
  specific welfare claim ("priority lane delivers X% net-utility
  improvement for arb users"), the demand-mix calibration is
  approximate and the X% has demand-shape sensitivity. The branch's
  M4 / M5 sweeps probe this via mispriced overlays and phased
  congestion variants but do not exhaustively sweep the
  arb/DeFi/patient share.

- **Demand non-stationarity at finer than ~2-hour scale is not
  modelled**. Phase-2's `Phased` arrival-rate machinery captures
  the order-of-2-hours stress regime that motivates the
  controller-drift question. Diurnal (UTC-working-hours) and
  one-off-event (NFT drops, governance deadlines) demand shocks are
  outside the model. **NEEDS-DISCLOSURE level: low** for the
  controller-drift question (the relevant timescale is window-length
  × per-block-cadence ≈ 32 × 20 s ≈ 10 min, faster than diurnal);
  **medium** for any external-event-modelling claim.

- **MEV is structurally absent from mainnet (eUTxO, no global
  mempool); phase-2's arb-component is a future-mechanism
  assumption, not a mainnet calibration**. Cardano's local
  validation prevents EVM-style front-running; no $-billion-scale
  MEV flow exists. Phase-2's hard-deadline-arb-tail component is
  partially aspirational (a model of what a DEX arb bot *would*
  look like under a deployed priority lane). **Bounded by the
  mispriced suite's `{1, 1}` worst-case**. NEEDS-DISCLOSURE level:
  low — surfaces in any external write-up alongside the spike 002
  fee-semantics disclosure.

- **`max_fee_policy = {4, 1}` default and `target_inclusion_blocks`
  defaults (priority=1, standard=4) are modelling choices, not
  mainnet calibrations**. Carried over from spike 002. The
  `LatencyEstimator` seed shapes early-run lane choice; once
  observed inclusions accrue the seed is overwritten. **NEEDS-
  DISCLOSURE level: medium** — these are reasonable but unmeasured
  defaults; an external paper should cite them as "modelling
  assumptions" rather than "calibrated values."

- **The simulator's mainnet-equivalent topology already exists on
  this branch** (`topology-cip-realistic.yaml`, 600 pools,
  CIP-0164-aligned, Pareto(α=1.4) stakes, 4 regions, RIPE-Atlas
  latencies). Re-running the welfare suites against this topology
  is a one-suite-config flip away. The fact that **M5 goldens are
  still pinned to single-producer** is a deliberate scope choice
  documented in `topology-single-producer.yaml`'s preamble, not an
  oversight.

## Investigation Trail

- Stake-pool count cross-referenced across CoinLaw (~3,200),
  PoolTool (~2,930), and SQ Magazine (~3,000). The variance is
  whether one counts registered, actively-producing, or
  non-zero-stake pools — none of these definitions is canonical;
  the order-of-magnitude figure is the well-defined one.

- Nakamoto coefficient (MAV ≈ 25) cross-referenced against AdaPulse
  (epoch 559 value, but no live date stamp in the search), the
  Cardano Forum's "What is the Nakamoto coefficient of Cardano?"
  thread, and the DLT 2022 academic paper. **No 2026-fresh single
  authoritative source** was found via search; the MAV figure is
  consensus across multiple community resources but lacks a
  bit-fresh primary source. Recommended: a phase-2 external write-up
  should compute MAV from a fresh mainnet stake-distribution snapshot
  if the figure is load-bearing.

- The CIP-0164 simulation topology details (10,000 / 750 nodes,
  RIPE Atlas latencies, cloud-data-centre bandwidth) come from the
  CIP-0164 Impact Analysis doc on
  `github.com/input-output-hk/ouroboros-leios`. Phase-2's distilled
  600-pool version is documented in `m6-implementation-plan.md` and
  generated by `sim-rs/scripts/generate-cip-topology.py` with a
  pinned seed.

- Cardano's lack of a global mempool (and consequent MEV-resistance)
  is documented in Genius DEX's marketing material and is a
  well-known qualitative property of eUTxO; the search confirmed
  this against multiple sources but no quantitative "MEV on Cardano
  = $X" figure was retrievable (because the figure is structurally
  near zero). The "Cardano vs Ethereum MEV loophole" article (Q4
  2025) characterises the difference qualitatively.

- The `max_fee_policy` and `target_inclusion_blocks` defaults
  carry over from spike 002 (fee-semantics) and CLAUDE.md
  ("Calibration choices"). No new evidence in this spike's
  research shifts the disclosure level on these knobs.

- The honest-producer assumption surface was traced via
  `mechanism-design.md` (which doesn't discuss attacker models
  explicitly — it assumes honest producers throughout) and the
  CONCERNS.md security note about `partition_activated` being a
  producer claim. The Ouroboros Praos paper supplies the
  protocol-level honest-stake-majority assumption; phase-2 inherits
  it transparently.

- The CIP-realistic topology file was inspected directly
  (`topology-cip-realistic.yaml`, 40,220 lines, 600 pools, with
  per-peer latencies in the 16–80 ms range for the inspected
  pool-000). The M6 implementation plan documents the calibration
  rationale.

## Verdict

**NEEDS-DISCLOSURE.** No phase-2 calibration is *wrong*, but the
single-producer topology is the **single biggest abstraction in the
phase-2 simulator** and must be disclosed at the strongest level in
any external write-up. The honest-producer assumption is operationally
trivial under N=1 but is implicit and load-bearing (the
`partition_activated` producer-claim attack surface is not exercised
by the simulator); an attacker-model write-up must surface this. The
actor model's three-component shape is order-of-magnitude correct
against mainnet's transaction mix but is not bit-calibrated, and its
`{4, 1}` max-fee policy and `{1, 4}` target-inclusion-blocks defaults
are modelling assumptions, not calibrations. The CIP-realistic
600-pool topology already exists on this branch (M6) as the
mainnet-faithful counterpart and is a one-suite-config flip away from
producing multi-producer cross-checks if the phase-2 paper / CIP
write-up wants them.

Disclosure ranking (most → least urgent):
1. Single-producer topology (N=1 vs mainnet ~3,000 SPOs) — high
2. Honest-producer assumption + `partition_activated` producer-claim
   attack surface — high
3. Actor demand-mix calibration (order-of-magnitude, not bit-exact) — medium
4. `max_fee_policy = {4, 1}` and `target_inclusion_blocks`
   defaults — medium
5. Demand non-stationarity (no diurnal / event-driven shocks) — low
6. No inter-node latency model — low (implied by N=1)
7. MEV / strategic-actor absence — low (mainnet-faithful in shape;
   eUTxO is structurally MEV-resistant)

## Sources

- [Cardano ADA Statistics 2026 (CoinLaw)](https://coinlaw.io/cardano-statistics/) — retrieved 2026-05-13 (~3,200 pools, daily-tx ~2.6 M, smart-contract share ~35%)
- [Cardano PoolTool](https://pooltool.io/) — retrieved 2026-05-13 (~2,930 actively producing)
- [SQ Magazine — Cardano ADA Statistics 2026](https://sqmagazine.co.uk/cardano-ada-statistics/) — retrieved 2026-05-13 (decentralisation summary, ~3,000 SPOs)
- [AdaPulse — MAV: The Safety Metric in Block Production Decentralization](https://adapulse.io/mav-the-safety-metric-in-block-production-decentralization/) — retrieved 2026-05-13 (MAV/Nakamoto coefficient ≈ 25 epoch 559; historical comparison)
- [Cardano Forum — What is the Nakamoto coefficient of Cardano?](https://forum.cardano.org/t/what-is-the-nakamoto-coefficient-of-cardano/109348) — retrieved 2026-05-13
- [Decentralization Analysis of Pooling Behavior in Cardano Proof of Stake (ACM DLT 2022)](https://dl.acm.org/doi/fullHtml/10.1145/3533271.3561787) — retrieved 2026-05-13 (Pareto-shaped stake distribution, qualitative inequality)
- [Cardano Docs — Guidelines for operating large stake pools](https://docs.cardano.org/stake-pool-operators/guidelines-for-large-spos) — retrieved 2026-05-13 (multi-region operational guidance)
- [Cardano Forum — Geographical location and stakepools](https://forum.cardano.org/t/geographical-location-and-stakepools/36726) — retrieved 2026-05-13
- [CIP-0164 — Ouroboros Linear Leios (cips.cardano.org)](https://cips.cardano.org/cip/CIP-0164) — retrieved 2026-05-13
- [input-output-hk/ouroboros-leios](https://github.com/input-output-hk/ouroboros-leios) — retrieved 2026-05-13 (Impact Analysis: 10,000-node pseudo-mainnet / 750-node mini-mainnet topology, RIPE Atlas latencies, cloud-data-centre bandwidth)
- [Ouroboros Praos paper (David, Gaži, Kiayias, Russell — CRYPTO 2018)](https://eprint.iacr.org/2017/573.pdf) — retrieved 2026-05-13 (honest-stake-majority assumption formalisation)
- [IOG — Increasing the transaction throughput of Cardano](https://iohk.io/en/blog/posts/2022/03/21/increasing-the-transaction-throughput-of-cardano/) — retrieved 2026-05-13 ("300 ms TCP delay across the globe", Δp = 5 s propagation budget)
- [DappRadar — DeFi Surging on Cardano: 3 Dapps to Keep Watch](https://dappradar.com/blog/defi-surging-on-cardano-3-dapps-to-keep-watch) — retrieved 2026-05-13
- [DefiLlama — Cardano](https://defillama.com/chain/cardano) — retrieved 2026-05-13 (DeFi TVL context)
- [ZyCrypto — Cardano DApp Transactions Surge Over 45%](https://zycrypto.com/cardano-ada-dapp-transactions-surge-over-45-amid-network-activity-boom/) — retrieved 2026-05-13 (Messari Q1 2026 dApp-tx growth)
- [Genius Yield docs — Introduction (MEV resistance via eUTxO local validation)](https://docs.geniusyield.co/genius-dex/1.-introduction) — retrieved 2026-05-13
- [bitcoinethereumnews.com — Ethereum vs Cardano: MEV Loophole in Spotlight](https://bitcoinethereumnews.com/ethereum/ethereum-versus-cardano-big-truth-on-mev-loophole-in-spotlight/) — retrieved 2026-05-13
- In-repo provenance: `sim-rs/parameters/phase-2-sweep/topology-single-producer.yaml`, `sim-rs/parameters/phase-2-sweep/topology-cip-realistic.yaml`, `sim-rs/parameters/phase-2-sweep/demand/paper_like_*.yaml`, `sim-rs/sim-core/src/tx_actors.rs`, `docs/phase-2/m6-implementation-plan.md`, `docs/phase-2/calibration-fix-postmortem.md`
- Prior spikes: `.planning/spikes/001-rb-cadence-and-capacity/README.md` (cadence, CIP-0164 Table 7 partial), `.planning/spikes/002-fee-structure-and-mempool-sizing/README.md` (fee semantics, max-fee policy), `.planning/spikes/003-pricing-controller-calibration/README.md` (controller calibration anchors EIP-1559)
