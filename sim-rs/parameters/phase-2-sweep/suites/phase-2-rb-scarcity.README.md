# `phase-2-rb-scarcity` — RB-capacity scarcity, restated on the two-lane mechanism

## Experimental question

**How does priority-lane access (one RB-worth of guaranteed service)
hold up when RB capacity is reduced?**

Per spec ([mechanism-design.md §RB-reserved priority-only premium](../../../docs/phase-2/mechanism-design.md#L146-L196)):
the priority partition's per-slot capacity is one RB-worth, delivered
either via the RB itself (definitionally priority-only in the
RB-reserved variant) or via the EB priority partition (logical, of
size `priority_reservation_bytes = max_block_size`). Both quantities
track `rb_body_max_size_bytes` in the simulator's resolved config
(see `sim-core/src/config.rs:1029-1036`), so reducing
`rb-body-max-size-bytes` shrinks both in lockstep. Total per-slot
priority capacity = `2 × rb_body_max_size_bytes`.

This suite sweeps the RB body cap across baseline, half, third, and
quarter, holding pricing and demand fixed. The cross-job comparison
in `metrics_comparison.txt` answers whether priority service degrades
gracefully or sharply as the RB partition shrinks.

## Relation to the previous (tiered-backend) framing

On `pricing-sim-base`, RB scarcity tested how many *tiers* survived
when RB capacity shrank — an artefact of the tiered backend's per-tier
queue model, where a tier could be effectively starved out by RB
contention. The two-lane mechanism collapses that question to a
single dimension: can the priority lane retain its service guarantee
under reduced RB capacity?

The previous framing's "tier survival" generalises to "priority
retained-value ratio under load" — same question, different vocabulary.
Both frames ask whether the mechanism's price-discrimination promise
holds when the protected partition is constrained.

## Calibration

- **Pricing**: `two_lane_priority_only_static_x4.yaml` (RB-reserved,
  static-standard, `multiplier_floor = 4`). The lower-than-default
  multiplier-floor (×4 instead of ×16) puts more components onto the
  priority lane, raising priority demand into the constrained
  capacity. Under utility-maximising lane choice with the default
  `MaxFeePolicy::ScaledOverLaneQuote{4, 1}`:
  - High-urgency (×5) prefers priority (4× cost vs ~125× retained-value
    benefit).
  - Medium-urgency (×2) prefers priority (4× cost vs ~8× benefit).
  - Low-urgency (×1.05) submits to priority too in practice. The
    absolute fee gap (priority ~335 K lovelace vs standard ~200 K
    at era-floor quotes) is small relative to log-normal sampled
    values, so high-V txs find priority's 1.16× retained-value
    benefit worth the extra ~135 K fee. Component 2's
    `priority_included = 89, standard_included = 0` at baseline
    confirms this — every served low-urgency tx was priority-served.

  Net effect: priority is the only served lane in this suite. See
  the *Known caveat* section below for what this implies for the
  cross-job interpretation.
- **Demand**: `paper_like_congested.yaml` — phased 300/600/200 tx/slot,
  peaking at roughly 600 KB/slot.
- **RB capacities**: 90112 (baseline), 45056 (half), 30000 (third),
  22528 (quarter). Priority capacity per slot = 2 × these.

## How to read the output

`metrics_comparison.txt` carries the per-job per-component breakdown.
The **baseline-vs-reduced gradient** is the primary signal:

- `included_count_priority` and total `txs_included`: the
  cross-job inclusion gradient is the primary scarcity signal.
  Under the corrected calibration the gradient is gentle:
  total inclusions roughly halve from baseline to half-RB and
  approximately halve again to quarter-RB.
- Per-component `priority_included` and `inclusion_rate` (in
  `metrics_comparison.txt`): show which components retain service
  under scarcity. High-urgency (component 0) is most exposed
  to RB shrinkage by share of priority demand, but absolute
  comp-0 inclusion drops smoothly across the four jobs rather
  than falling off a cliff.
- `latency_blocks_mean` per component is roughly stable across
  the gradient (~6-9 blocks). At the corrected RB cadence
  (~1 RB per 20 slots) the bottleneck is RB *cadence* more than
  RB *body size* — shrinking the body reduces throughput
  proportionally without producing a sharp regime change.
- `evicted_quote_drift_count` shows non-zero values across the
  gradient as the controller drifts under sustained priority
  demand. Under the corrected calibration this is the regime
  where mis-priced actor calibrations (`{1, 1}` budget — see
  `phase-2-urgency-inversion`) face real eviction risk.

## Calibration history

An earlier revision of this README described an "M3 §9 degeneracy"
where `standard_lane_retained_value_ratio = 0.0` across all jobs.
That outcome was a *calibration bug*, not a calibration consequence:
`rb-generation-probability: 1.0` combined with the linear-Leios
13-slot endorsement window prevented EBs from ever landing on
chain, so EB-borne service (where standard-fee txs go in
RB-reserved variants) was structurally invisible. The bug was
fixed post-M5 by dropping rb-prob to 0.05 and bumping
`default-slots` to 1000. The current M6 suites run 2000 slots for
parity with `pricing-sim-base`; see
[../../../../docs/phase-2/calibration-fix-postmortem.md](../../../../docs/phase-2/calibration-fix-postmortem.md)
for the full explanation. The findings below describe behaviour
under the corrected calibration.

## What the suite shows

**The informative signal is the cross-job priority inclusion
gradient.** As the RB body shrinks, priority capacity (`2 ×
rb_body_max_size_bytes` per spec) shrinks proportionally. At
baseline the priority lane absorbs the high+medium-urgency
demand; below half-RB it runs out, and additional priority
demand stalls in the mempool rather than flooding into standard
(the EB priority partition activates only when the EB body
reaches its 16 MB capacity, which the actor demand profile does
not produce at this rate). Standard service is now non-zero
across the suite — EB endorsement fires under the corrected
calibration, so standard-fee txs from comp 2 (and overflow
priority refunded to standard when the partition is *not*
activated) appear as `standard_included` per component.

The headline numbers (each averaged across 3 seeds) under the
corrected calibration are in the suite output's
[metrics_comparison.txt](../../../output/phase-2/rb-scarcity/metrics_comparison.txt).
Cross-job priority inclusion gradient stays the qualitative
answer: priority service degrades sharply once total priority
capacity falls below sustained demand. Standard service is
present but congested under the same demand profile.

## How to run

```sh
cd sim-rs
cargo run --release --bin experiment-suite -- run \
  parameters/phase-2-sweep/suites/phase-2-rb-scarcity.yaml
cargo run --release --bin experiment-suite -- verify \
  parameters/phase-2-sweep/suites/phase-2-rb-scarcity.yaml
```

Output lands under `output/phase-2/rb-scarcity/` (resumable manifest;
delete the directory or use `experiment-suite status` to inspect).
