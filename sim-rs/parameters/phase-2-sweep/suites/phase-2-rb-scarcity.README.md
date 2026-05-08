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
- **Demand**: `paper_like_congested.yaml` — sustained ~150 KB/slot.
- **RB capacities**: 90112 (baseline), 45056 (half), 30000 (third),
  22528 (quarter). Priority capacity per slot = 2 × these.

## How to read the output

`metrics_comparison.txt` carries the per-job per-component breakdown.
The **baseline-vs-reduced gradient** is the primary signal:

- `priority_lane_retained_value_ratio` (per job): how much of
  priority-served value is preserved through latency. Approaches
  1.0 when priority is uncontested; declines as RB scarcity bites.
  Observed: 1.0 (baseline) → 0.93 (half) → 0.12 (third) → 0.12
  (quarter).
- `included_count_priority` per slot (in `time_series.csv`) and
  total `txs_included` (in `metrics_comparison.txt`): the
  cross-job inclusion gradient is the primary scarcity signal.
  Observed: ~10500 → ~6000 → ~440 → ~400.
- Per-component `priority_included` and `inclusion_rate` (in
  `metrics_comparison.txt`): show which components retain service
  under scarcity. High-urgency stays close to 1.0 inclusion-rate
  through `rb_reduced_half`; medium-urgency drops sharply at
  reduced sizes; low-urgency falls to near-zero.
- `evicted_quote_drift_count` is **expected to stay at 0** with
  the default `{4, 1}` headroom. Under sustained scarcity the
  controller drifts `c_priority` upward but ~5× upward isn't
  enough to push txs above their max-fee budget. This isn't a
  bug; it's the calibration choice. Mis-priced calibrations would
  surface non-zero values here — see `phase-2-urgency-inversion`
  for a related shape.

## Known caveat (M3 §Known-limitations #9 persists; standard service stays empty)

The M3 protocol-base pins `rb-generation-probability: 1.0` and a
single producer (`tx-generation-weight: 1`). Under this calibration
*at baseline RB*, every slot produces a tx-bearing RB and every EB
saturates, so every priority-fee tx is served as Priority and
`priority_lane_retained_value_ratio = 1.0`,
`standard_lane_retained_value_ratio = 0.0` for the
`rb_baseline` job. This is documented as "informational only" in the
M3 handoff.

**The degeneracy persists across all four jobs in this suite.** A
fresh end-to-end run confirms `standard_lane_retained_value_ratio
= 0.000000` and `standard_included = 0` for every component on
every (job, seed) pair, baseline through quarter. The cross-RB
gradient does not lift standard service off zero — actor lane
choice keeps every component on priority across the gradient
(under utility-maximising lane choice with `multiplier_floor = 4`,
priority retains positive expected utility against standard for
all three components even at heavily-drifted priority quotes), and
mis-fit priority txs accumulate in the mempool rather than
overflowing into standard space.

The mechanism is: under reduced RB, priority demand exceeds the
per-slot RB priority capacity. With `{4, 1}` headroom, the
mempool gate's quote-drift revalidation does not evict the excess
priority txs; they simply sit. The EB priority partition activates
under saturation (so priority overflow lands in EB priority space,
not EB standard space), and what doesn't fit waits. By the end of
the 200-slot run, much of the mempool is priority-resident and
never makes it to a block. There is no priority-to-standard
refund because the partition activated; refund-to-standard is the
non-activation path, which doesn't fire here.

**The informative signal is therefore the cross-job priority
inclusion gradient**, not a within-job priority-vs-standard split.
Inclusion counts: baseline ≈ 10500 → reduced_half ≈ 6000 →
reduced_third ≈ 440 → reduced_quarter ≈ 400. `priority_lane_retained_value_ratio`
slides from 1.0 at baseline through 0.93 (half) to 0.12
(third/quarter). That's the answer: priority service degrades
sharply once total priority capacity (`2 × rb_body_max_size_bytes`)
falls below sustained demand. Standard service plays no role in
this experiment under the M3 calibration.

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
