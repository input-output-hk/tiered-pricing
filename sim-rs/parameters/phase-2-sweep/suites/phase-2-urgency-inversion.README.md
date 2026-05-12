# `phase-2-urgency-inversion` — urgency-inversion under mis-priced actors

## Experimental question

**With high-urgency actors paired with a low-multiplier
`MaxFeePolicy` (zero quote-drift headroom), does the priority lane
still deliver urgency separation? Or do mis-priced high-urgency
actors get evicted under sustained drift while correctly-priced
lower-urgency actors retain their service?**

The phase-2 mechanism's promise is that under congestion, urgent
users self-select into the priority lane and receive faster
inclusion via either the RB or the EB priority partition. That
promise is contingent on users posting a `max_fee_lovelace` that
covers not just the current quote but its drift under load. If a
user mis-prices their `MaxFeePolicy` — choosing a low multiplier
that gives zero drift headroom — the mempool gate will evict them
on the first controller update that nudges `c_priority` upward.

This suite asks: how robust is urgency separation to mis-pricing on
the high-urgency component?

## Relation to the previous (tiered-backend) framing

On `pricing-sim-base`, urgency inversion tested whether high-urgency
actors paying low fees lost service to low-urgency actors paying
high fees. That used the tiered backend's per-tier price discovery
where a tier's fee was set externally (not by the user's posted
fee).

The two-lane translation: instead of "low-urgency actors paying high
fees beat high-urgency actors paying low fees", the question becomes
"high-urgency actors with zero quote-drift headroom lose priority
service to lower-urgency actors with adequate headroom". The
mechanism is the mempool gate's quote-drift revalidation: as
`c_priority` ticks up under load, mis-priced actors fall out of the
mempool while correctly-priced actors hold their place.

Same conceptual question — "does the urgency signal survive
mis-pricing?" — re-expressed in two-lane vocabulary.

## Calibration

- **Pricing**: `two_lane_both_dynamic_partitioned_x4.yaml`
  (partitioned both-dynamic, `multiplier_floor = 4`). Both lanes
  are dynamic, so both `c_standard` and `c_priority` move under
  load. The lower-than-spec-default `multiplier_floor` (4 vs 16) is
  picked to drive both high-urgency and medium-urgency components
  onto the priority lane under utility-maximising lane choice — at
  16, only high-urgency picks priority and priority demand is too
  low to saturate the partition, so `c_priority` stays at the floor
  and no eviction shape emerges. With `multiplier_floor = 4`,
  high+medium urgency together generate roughly 120/240/80 KB per
  slot across the phased congested profile. This saturates priority
  service during the overload phase, the priority controller drifts
  upward, and mis-priced txs at `{1, 1}` zero-headroom are evicted on
  the next update.
- **Demand**: two profiles compared.
  - `paper_like_congested.yaml` — every component carries the
    default `ScaledOverLaneQuote{4, 1}` (4× headroom).
  - `paper_like_mispriced.yaml` — high-urgency component (0)
    carries `ScaledOverLaneQuote{1, 1}` (zero headroom); other
    components keep `{4, 1}`.
- **Slots**: 2000 per seed x 3 seeds x 2 jobs = 6 runs.

## Why `{1, 1}` is the mis-pricing knob (not `{0, 1}` or smaller)

`{1, 1}` means `max_fee_lovelace = min_fee_b + 1 × current_quote ×
bytes` — the actor's max-fee covers the lane fee *exactly* at
submission. Admission succeeds. Then on the next controller update,
if `c_priority` ticks upward, the lane's quote rises; the gate's
revalidation walks the mempool and evicts every tx whose
`max_fee_lovelace < min_fee_b + new_quote × bytes`. With zero
headroom, that's every priority tx posted just before the tick.

Smaller values like `{0, 1}` or `{1, 2}` would prevent admission
altogether (max_fee < min_fee_b + lane_fee even at submission). That
isn't "mis-pricing" — that's "doesn't even try". The interesting
shape is "tries, gets in, gets evicted under drift".

## Calibration history

An earlier revision of this README described mispricing's signal
as living entirely in `refund_lovelace = 0` because "every slot
produces a tx-bearing RB that absorbs that slot's priority
demand, so priority txs are admitted and included in the same
slot before any quote drift can hit them." That framing was an
artefact of a calibration bug: `rb-generation-probability: 1.0`
combined with the linear-Leios 13-slot endorsement window
prevented EBs from landing, kept RB cadence at 1 per slot, and
suppressed quote drift. See
[../../../../docs/phase-2/calibration-fix-postmortem.md](../../../../docs/phase-2/calibration-fix-postmortem.md)
for the full explanation. The findings below describe behaviour
under the corrected calibration (`rb-prob = 0.05`, expected RB
gap ~20 slots, controller now actually drifts under sustained
priority demand).

## How to read the output

The per-component breakdown in `metrics_comparison.txt` is the
primary signal. Under the corrected calibration the mispricing
shape shows up in **both** the refund envelope **and** the
eviction count:

- `refund_lovelace` for component 0:
  - `correctly_priced` — `max_fee = 4 × actual_fee` leaves a
    3 × actual_fee refund margin per included tx.
  - `mispriced_high_urgency` — `0`. `max_fee = actual_fee` at
    submission means no refund margin; the actor's budget is
    fully consumed.
- `evicted_quote_drift_count`: with the corrected RB cadence and the
  heavier phased demand, quote drift now bites. The mispriced excess
  should be component-0-specific: comp 0 mispriced txs are exactly
  the ones whose max_fee equals submission-time fee and so go stale
  on the first quote tick.
- `fees_paid_lovelace` for component 0 differs across the two
  jobs (the mispriced run pays less because evicted txs never
  pay). The economic differentiation is now visible in both
  what the actor *risks* (refund) and what the actor *loses*
  (eviction).
- `inclusion_rate` for component 0 drops below 1.0 under both
  jobs — the corrected RB cadence means priority demand
  sometimes outpaces priority capacity per slot, and not every
  comp-0 tx gets included even before mispricing kicks in.

The interpretation under the corrected calibration: mispriced
high-urgency actors face a double penalty — eviction risk
(quote drift past their `{1, 1}` budget) plus surrender of all
refund margin even when included. The `{4, 1}` default exists
precisely to give actors a safety margin against drift; the
`{1, 1}` mispricing shape shows what they're giving up by
tightening it.

`time_series.csv`'s `c_priority` column confirms the controller
is moving — `priority_over_standard_quote_ratio` reaches the
tens of × under saturation in both jobs (initial 4 × → much
larger as the controller drifts under sustained priority
demand).

## How to run

```sh
cd sim-rs
cargo run --release --bin experiment-suite -- run \
  parameters/phase-2-sweep/suites/phase-2-urgency-inversion.yaml
cargo run --release --bin experiment-suite -- verify \
  parameters/phase-2-sweep/suites/phase-2-urgency-inversion.yaml
```

Output lands under `output/phase-2/urgency-inversion/`.
