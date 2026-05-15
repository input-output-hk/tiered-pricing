# Cardano-realism audit — phase-2 dynamic-pricing simulator

Date: 2026-05-13
Branch: dynamic-experiment
Scope: every calibration choice in `parameters/phase-2-sweep/` and every
modeling assumption in `sim-rs/sim-core/`.
Evidence: 4 spike READMEs under `.planning/spikes/`, cited inline.

> **[Annotation added 2026-05-13]** When this audit was written,
> phase-2 suites referenced `topology-single-producer.yaml` in
> CLAUDE.md and the spike trail. As of 2026-05-13, suites have been
> re-pointed to `topology-realistic-100.yaml` (100-node multi-producer
> with a mass-stratified mainnet stake curve). The "Topology and actor
> model" verdict and the single-producer disclosure paragraphs below
> are preserved for context but are no longer the operational
> reality; see CLAUDE.md §Calibration choices §"Topology choice" and
> [`.planning/spikes/006-curve-design/README.md`](../../.planning/spikes/006-curve-design/README.md).
> Per `validity-threats.md` the operational suites now expose
> slot-battle dynamics; WR-1 (no pricing-state rollback on slot-battle
> reorg) is correspondingly reclassified as LIVE rather than dormant —
> see `.planning/REVIEW.md`.

## TL;DR

Phase-2's protocol-cadence and fee-floor calibrations are literal re-uses of
current mainnet values (rb-prob = 0.05 = activeSlotsCoeff; minFeeA = 44;
minFeeB = 155381; maxTxSize = 16384; mempool-cap rule = 2× one bearer
block) — not approximations, but bit-equal lifts. The pricing controller
inherits EIP-1559's deployed knobs unchanged (`D = 8`, `target = 0.5`,
per-priced-block cadence). Three categories require disclosure but none
invalidate the conclusions: (i) the fee `field` is reinterpreted as a
maxFee envelope rather than the exact deterministic fee Cardano wallets
ship today, and the refund path depends on a separate CIP; (ii) four
controller knobs — window-length 32, multiplier-floor 4 (in 2 of 7
suites), multiplier-floor 16 (spec default), and the lane-signal-source
choices — are defensible but unanchored to deployed-system data; (iii)
the `topology-single-producer.yaml` collapses ~3,000 mainnet SPOs to
N=1, which removes slot-battle and fork dynamics by construction. No
hard limitations were surfaced — every deviation is bounded and
defensible with explicit disclosure.

## Verdict by category

| Category | Verdict | Disclosure level |
|---|---|---|
| RB cadence and capacity | VALIDATED | None — matches mainnet exactly |
| Fee structure and mempool sizing | NEEDS-DISCLOSURE | 3 reinterpretations to state |
| Pricing-controller calibration | NEEDS-DISCLOSURE | 4 spec-open choices to acknowledge |
| Topology and actor model | NEEDS-DISCLOSURE | Single-producer is the strongest abstraction |

## What lines up with mainnet

These are the "phase-2 is mainnet-grounded" anchors a reviewer can verify
against Cardano on-chain state / cardano-node directly:

- **RB cadence is bit-equal to mainnet Praos.** `rb-generation-probability =
  0.05` equals `activeSlotsCoeff = 0.05`; the expected 20-slot RB gap
  matches mainnet's observed ~20.1 s average (~0.5 % drift, attributable
  to pool downtime). See spike 001 §Comparison Table rows 1–3.
- **RB body cap is mainnet-current.** `rb-body-max-size-bytes = 90112`
  is the value set by the April-2022 protocol update and unchanged
  since. Spike 001 §Comparison Table row 4.
- **Fee floor matches mainnet to the lovelace.** `min-fee-a = 44`,
  `min-fee-b = 155381` are bit-equal to Conway-era mainnet; the
  EIP-1559 baseline initial quote of 44 reproduces today's
  `minFeeA × bytes` term at controller equilibrium. A 200-byte tx
  costs exactly 164,181 lovelace under both. Spike 002 §Findings.
- **`maxTxSize` matches mainnet exactly.** 16,384 bytes inherited from
  the upstream `config.default.yaml`. Spike 001 §Comparison Table.
- **Mempool sizing rule matches mainnet shape.** Both networks use
  `2 × one_bearer_block` with reject-on-full overflow. The absolute
  byte cap diverges (24 MB vs 180 KB) but only because Leios's 12 MB
  EB drives the bearer-block-size term; the rule itself is identical.
  Spike 002 §Comparison Table row "Mempool cap rule".
- **EIP-1559 controller parameters match Ethereum mainnet exactly.**
  `D = 8` (BASE_FEE_MAX_CHANGE_DENOMINATOR), `target = 0.5`
  (ELASTICITY_MULTIPLIER = 2), per-priced-block update cadence — all
  present in every baseline pricing YAML, with sweeps in
  `phase-2-eip1559-robustness.yaml` (D ∈ {4, 8, 16}, target ∈
  {0.25, 0.5, 0.75}) bracketing the deployed values for sensitivity.
  Spike 003 §Comparison Table rows 1–3.
- **Leios-specific knobs cite CIP-0164 Table 7 with in-YAML
  provenance.** `linear-vote-stage-length-slots = 4`,
  `linear-diffuse-stage-length-slots = 7`,
  `eb-referenced-txs-max-size-bytes = 12000000`,
  `eb-body-validation-cpu-time-ms-per-byte = 2.15e-5`, `n = 600`,
  `τ = 75 %` — none cross-checkable against deployed mainnet (Leios
  is pre-deployment), but each has an explicit "CIP-0164 Table 7"
  comment and the Leios FAQ ("RB ~20 s, EB ~5 s") corroborates the
  cadence shape. Spike 001 §Findings.

## What needs disclosure

### Fee structure and mempool sizing

1. **Fee-field semantic reinterpretation.** Mainnet `tx.fee` is the
   exact deterministic fee the wallet computed at sign-time; there is
   no maxFee envelope or refund path. Phase-2 reinterprets the same
   `fee` field as `max_fee_lovelace`, charges the (possibly-lower)
   current quote at inclusion, and refunds the gap via Polina's
   separate fee-change-return CIP. This is a deliberate mechanism-
   level change documented in `mechanism-design.md` L39-51 — not a
   calibration drift — but it is the single most user-visible
   deviation from the world Cardano users have today, and the refund
   path is an external dependency. **Defensible because** phase-2's
   welfare claims explicitly assume the refund mechanism exists and
   the spec is transparent about the reinterpretation. Spike 002
   §Findings + §Verdict item 1.

2. **Mempool absolute byte cap is 133× larger than mainnet** (24 MB
   vs 180 KB). The cap *rule* matches mainnet (`2 × bearer-block-
   bytes`), but Leios's 12 MB EB drives the bearer-block term to 24
   MB total. A reader who knows mainnet's ~180 KB mempool will be
   surprised. **Defensible because** the rule shape and overflow
   policy match exactly; the absolute number is a downstream
   consequence of CIP-0164's 12 MB EB target, not a different
   sizing philosophy. Spike 002 §Comparison Table row "mempool-
   max-total-size-bytes".

3. **Default `max_fee_policy = {4, 1}` (4× quote-drift headroom) is
   a forecast about post-deployment wallet behavior, not a
   calibration anchor.** Mainnet wallets today have no analogous
   knob — they ship at the exact deterministic min-fee via
   `cardano-serialization-lib`. Phase-2's 4× headroom is comparable
   to Ethereum's ~2× `maxFeePerGas` convention but is not measured
   against Cardano user behavior (which doesn't exist for this
   knob). **Defensible because** `paper_like_mispriced.yaml` uses
   `{1, 1}` (zero headroom) for the hard-deadline component to
   bound the worst case where users treat phase-2 like mainnet and
   ship at exact min-fee. Spike 002 §Findings + §Verdict item 3.

### Pricing-controller calibration

> **Update 2026-05-14:** The disclosure items below reflected the
> audited spec (per-priced-block update cadence matching EIP-1559).
> Empirical investigation revealed the pre-refactor accumulator
> implementation diverged from the spec by double-stepping per RB-EB
> pair (separate `apply_priced_block` and `apply_eb_priced_block`
> calls; see
> [`.planning/chain-derived-bug2-investigation.md`](../../.planning/chain-derived-bug2-investigation.md)).
> The chain-derived refactor (spike 007, Family B committed
> 2026-05-14) brings the implementation in line with the spec.
> Disclosure items 1-4 below are now historical and apply to the
> spec-matched implementation. The empirical welfare-impact
> characterization is at
> [`.planning/mechanism-welfare-impact-2026-05-14.md`](../../.planning/mechanism-welfare-impact-2026-05-14.md);
> the decision memo is at
> [`.planning/family-b-decision-2026-05-14.md`](../../.planning/family-b-decision-2026-05-14.md).

1. **Window length 32 for capacity-varying signals is a defensible
   response to a real-but-empirically-mild problem.** EIP-1559's
   per-block-no-smoothing controller exhibits chaotic short-term
   oscillations under uniform-bidder models (Reijsbergen et al.
   2021/2025; Leonardos et al. AFT'21), though Liu et al. (CCS'22)
   show this has not been a usability disaster on Ethereum mainnet.
   Phase-2's capacity-weighted window picks a *different*
   architectural answer from the literature's preferred AIMD fix,
   addressing the same problem plus the linear-Leios-specific
   problem of blending 90 KB RBs with 12 MB EBs (133× capacity
   ratio). **Defensible because** the smoothing suite
   (`phase-2-eip1559-smoothing.yaml`) sweeps window ∈ {16, 32, 64}
   for sensitivity; the specific length 32 is a round-number choice,
   not an empirical anchor. Spike 003 §Comparison Table row "Window
   length" + §Verdict item 1.

2. **`multiplier_floor = 4` in `phase-2-rb-scarcity` and
   `phase-2-urgency-inversion` is a calibration accommodation, not
   an economic claim.** CLAUDE.md states the rationale explicitly:
   at the spec default 16, only urgency≥5 components find priority
   attractive and priority demand stays too low to surface
   controller drift; at 4, urgency≥2 picks priority and the
   controller drifts. Any external summary of these two suites'
   conclusions must lead with "under multiplier_floor = 4."
   **Defensible because** 5 of 7 suites independently cover the spec
   default 16 (priority-only at {4, 8, 16}; both-dynamic at {4, 16}),
   so the 7-suite design as a whole is robust across the floor sweep.
   Spike 003 §Comparison Table row "Multiplier-floor" + §Verdict
   item 2.

3. **The spec default 16 itself has no empirical anchor.**
   `mechanism-design.md` L155, L290 list the default without citing
   calibration data; the only justification is "strong price-
   discrimination guarantee." Ethereum has no comparable second-lane
   multiplier. The multiplier-floor magnitude is therefore the
   single weakest-anchored calibration in phase-2, but this is a
   spec-level rather than simulator-level issue — the simulator
   faithfully implements the spec's open-question framing.
   **Defensible because** the priority-only suites sweep {4, 8, 16}
   and the both-dynamic suite sweeps {4, 16}, so phase-2's findings
   are reported across the floor sweep rather than at a single point.
   Spike 003 §Findings + §Verdict item 3.

4. **Both-dynamic standard signal source and un-reserved priority
   option 1 are spec-open choices.** The simulator picks
   (a) `standard_paying_bytes / eb_referenced_txs_max_size_bytes`
   for EBs with no standard sample on RB-reserved RBs (lane-
   isolation invariant), and (b) `priority_paying_bytes /
   total_block_capacity` for un-reserved priority (the simplest of
   three open options in `mechanism-design.md` L207-211). Neither
   has an empirical anchor; both have internally-consistent
   motivations. **Defensible because** option 1's known weakness
   (priority demand fitting below partition-worth reads as low
   utilization) is partially probed by the multiplier-floor sweep,
   and the both-dynamic lane-isolation argument is forced by the
   partition rule's logic. Spike 003 §Findings + §Verdict item 4.

### Topology and actor model

> **[Corrected 2026-05-13]** The disclosure below described the
> topology as `topology-single-producer.yaml`, which was incorrect for
> the operational suites at audit-time. The suites now use
> `topology-realistic-100.yaml` (100-node, mass-stratified mainnet
> curve). The N=1 single-producer disclosure no longer applies;
> instead, multi-producer disclosures apply: per-node controller
> divergence, slot-battle siblings with different pricing samples,
> and the WR-1 rollback gap are all live concerns. The CIP-realistic
> 600-pool topology remains available for any larger multi-node
> cross-check.

1. **Single-producer topology (N=1) vs mainnet ~3,000 SPOs is the
   strongest abstraction in the simulator** and an intentional one.
   M5 suite goldens are pinned to `topology-single-producer.yaml` to
   remove slot-battle and fork-resolution dynamics from the pricing-
   mechanism welfare question. Per-node controller divergence and
   multi-producer pricing-sample races are not exercised.
   **Defensible because** (a) the pricing-mechanism welfare
   question is slot-scoped and would only be noised by multi-
   producer dynamics, (b) M6 already produced
   `topology-cip-realistic.yaml` (600 pools, Pareto(α=1.4) stakes,
   4 regions, RIPE-Atlas latencies) as the mainnet-faithful
   counterpart, and (c) `topology-single-producer.yaml`'s preamble
   documents the choice explicitly. Spike 004 §Findings item 1 +
   §Verdict ranking item 1.

2. **The honest-producer assumption is operationally trivial under
   N=1 but is implicit and load-bearing.** The `partition_activated`
   bit on `LinearEndorserBlock` is a producer claim, not derivable
   from the EB body (CONCERNS.md security note). Under multi-
   producer with a byzantine producer, this could be mis-claimed to
   obtain priority service for standard-fee txs in the same EB. The
   simulator does not exercise this attack. **Defensible because**
   N=1 makes the producer by-construction the honest majority, and
   the fix path (make `partition_activated` body-derivable from the
   priority-paying-bytes count) is straightforward but outside
   phase-2's scope. Spike 004 §Findings item 3 + §Verdict ranking
   item 2.

3. **Actor demand-mix is order-of-magnitude correct, not bit-
   calibrated to mainnet share-of-traffic.** The three-component
   profiles (hard-deadline-arb / DeFi / patient) qualitatively match
   Q1 2026 mainnet's ~35 % smart-contract / ~65 % transfer mix; the
   log-normal byte-size distribution (median ~930 B) is consistent
   with mainnet's 200–2,000 B typical range; total tx/slot rate
   (~25–150) is within an order of magnitude of mainnet's ~30 tx/s.
   But shares are not bit-calibrated. **Defensible because** the M4
   / M5 sweeps probe demand-shape sensitivity via mispriced overlays
   and phased congestion variants; welfare claims should be reported
   "under this stylised demand mix." Spike 004 §Findings item 4 +
   §Verdict ranking item 3.

4. **`target_inclusion_blocks` defaults (priority=1, standard=4)
   are mechanism-induced, not mainnet-anchored.** These seed the
   actor's `LatencyEstimator` per (component, lane). Standard=4
   models the expected wait when a standard tx might sit behind
   several priority-only RBs — internal to the phase-2 mechanism,
   not measured on mainnet (where there is no priority lane).
   **Defensible because** observed inclusion latencies overwrite
   the seed once events arrive; the seed only shapes the first
   ~50 slots of actor lane choice. Spike 004 §Comparison Table row
   "`target_inclusion_blocks` defaults" + §Verdict ranking item 4.

5. **Demand non-stationarity at finer than ~2-hour scale is not
   modelled, and MEV / strategic actors are absent.** Phase-2's
   `Phased` arrival-rate machinery captures order-of-2-hours stress
   regimes but not diurnal patterns, NFT drops, or governance
   deadlines. Cardano's eUTxO model is structurally MEV-resistant
   (no global mempool), so the lack of strategic-actor modelling is
   *mainnet-faithful in shape* — phase-2's "arb tail" component is
   partly aspirational (a model of what a DEX arb bot would look
   like under a deployed priority lane). **Defensible because** the
   controller-drift timescale is window-length × per-block-cadence
   ≈ 10 min, faster than diurnal demand shifts; the mispriced suite
   bounds the worst-case wallet-behavior assumptions. Spike 004
   §Findings items 5–6 + §Verdict ranking items 5–7.

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
