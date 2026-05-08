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
  high+medium urgency together generate ~60 KB/slot priority
  demand against the 90 KB RB partition, the priority controller
  drifts upward under saturation, and mis-priced txs at `{1, 1}`
  zero-headroom are evicted on the next update.
- **Demand**: two profiles compared.
  - `paper_like_congested.yaml` — every component carries the
    default `ScaledOverLaneQuote{4, 1}` (4× headroom).
  - `paper_like_mispriced.yaml` — high-urgency component (0)
    carries `ScaledOverLaneQuote{1, 1}` (zero headroom); other
    components keep `{4, 1}`.
- **Slots**: 200 per seed × 3 seeds × 2 jobs = 6 runs.

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

## How to read the output

The per-component breakdown in `metrics_comparison.txt` is the
primary signal. Under the M3 single-producer + `rb-generation-probability:
1.0` + 90 KB RB calibration, *every* slot produces a tx-bearing RB
that is large enough to absorb that slot's priority demand
(high+medium ≈ 60 KB, RB cap 90 KB), so priority txs are admitted
and included **in the same slot** before any quote drift can hit
them in revalidation. Quote-drift evictions therefore stay at 0
across both jobs even at `multiplier_floor = 4` — the result of
M3 §Known-limitations #5 which already flagged that the M3
calibration doesn't surface drift-evictions in 200 slots.

The mispricing shape shows up instead in the **refund envelope**:

- `refund_lovelace` for component 0 (per `metrics_comparison.txt`):
  - `correctly_priced` — `~13 B` per seed (max_fee = 4 × actual_fee
    leaves a 3 × actual_fee refund margin).
  - `mispriced_high_urgency` — exactly `0` (max_fee = actual_fee,
    zero refund room). The actor's max-fee budget is fully consumed
    by the inclusion charge.
- `fees_paid_lovelace` for component 0 is *identical* across the two
  jobs at each seed (same submitted txs, same served quote at
  inclusion). The economic difference is in what the actor *risked*,
  not what they *paid*.
- `inclusion_rate` for component 0 stays at 1.0 under both jobs —
  confirming that under this calibration, mis-pricing does not
  evict priority service in-slot.

The interpretation: under priority capacity that absorbs each
slot's demand, mispricing is operationally equivalent to
correct pricing for service quality, but costs the actor every
lovelace of their max_fee budget. In a slightly more aggressive
drift regime — smaller RB capacity, multi-producer slot battles, or
demand spikes that linger across slots — the mispriced actors would
be evicted on the first revalidation tick because they have zero
headroom. The {4, 1} default exists to give actors a safety margin
against drift; the {1, 1} mispricing shape shows what they're giving
up by tightening it.

`time_series.csv`'s `c_priority` column confirms the controller is
moving — `priority_over_standard_quote_ratio` reaches ~50 × under
saturation in both jobs (initial 4 × → ~200 × min_fee_a) — but the
within-slot inclusion timing means lingering txs aren't where the
drift bites.

For a calibration that *does* trigger evictions, see
`phase-2-rb-scarcity` with `rb_reduced_third` or `rb_reduced_quarter`
overlays — when RB capacity is too small to absorb a slot's priority
demand, txs linger and become eviction-eligible.

## How to run

```sh
cd sim-rs
cargo run --release --bin experiment-suite -- run \
  parameters/phase-2-sweep/suites/phase-2-urgency-inversion.yaml
cargo run --release --bin experiment-suite -- verify \
  parameters/phase-2-sweep/suites/phase-2-urgency-inversion.yaml
```

Output lands under `output/phase-2/urgency-inversion/`.
