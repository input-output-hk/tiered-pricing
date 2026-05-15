# Chain-derived controller — bug #2 investigation

Date: 2026-05-14
Symptom: divergence from accumulator emerges at slot ~85-100 even after
bug #1 (`current_chain_tip_quote` one-step lag) is fixed. The reference
case is `sundaeswap-singlelane / eip1559_d4_t50_w32`, seed 1, on
`topology-realistic-100.yaml` (multi-node, zero slot battles).

## TL;DR

**Best-guess root cause: chain-derived under-counts controller steps
relative to the accumulator under deferred EB validation.** The
accumulator fires one controller step per `apply_*_block` invocation —
one for the certifying RB's `apply_priced_block`, and a *second* step
later when `apply_eb_priced_block` runs at deferred EB validation.
Chain-derived fires the controller exactly once per canonical block
(at production), so when a parent's EB validates after the parent's
child has already been produced, that EB's controller-step contribution
is lost. The asymmetry is **architectural**, not an off-by-one — it
reflects a true semantic difference between "step per apply event"
(accumulator) and "step per canonical block" (chain-derived).

**Localised file/lines:**

- `sim-rs/sim-core/src/sim/linear_leios.rs:2301-2320`
  (`samples_for_rb`): conditional inclusion of EB samples gated on
  `get_validated_eb`. When the EB hasn't been validated locally at the
  time the RB is published, the cached samples for that RB exclude the
  EB body. Descendants computed before the deferred EB validates
  permanently miss this signal.
- `sim-rs/sim-core/src/sim/linear_leios.rs:1496-1541`
  (`finish_validating_eb`): the deferred-EB validation handler
  overwrites `block_samples[rb_id]` with the now-complete RB+EB sample
  set. **The canonical RB's stored `window_aggregate` and
  `derived_quote` are not updated, and descendants already on chain
  retain stale `window_aggregate` values that never folded the EB
  sample in.** Crucially, this handler does *not* fire a controller
  step — unlike legacy's `apply_eb_priced_block`
  (`git show 2f8ac5e:.../linear_leios.rs:2084-2094`) which calls
  `feed_samples_and_revalidate` → `update_after_block` → one full
  EIP-1559 `step()`.
- `sim-rs/sim-core/src/sim/linear_leios.rs:2179-2221`
  (`compute_chain_derived_quote_for_child_of`): the only path that
  drives controller state forward. Called once per RB at production
  (line 892) and once per slot at the representative for
  trajectory/admission queries (lines 2278-2285). There is no
  corresponding hook on deferred EB validation.

**Confidence: HIGH** for the under-counting mechanism. The empirical
trajectory at slot 87 shows the accumulator stepping ~1.58×
(69 → 109; two clamped +25% steps applied within the same slot window)
while chain-derived steps exactly 1.26× (69 → 87; one clamped +25%
step). The accumulator diagnostics report `max_single_step = 1.946×`
(three clamped steps coalesced into one slot) vs chain-derived's
`1.261×` (one clamped step at any slot transition). The 1.946× value
is impossible under chain-derived's one-step-per-block invariant; it
*can only* come from `apply_eb_priced_block` firing additional steps
that chain-derived's design discards.

The slot ~85-100 onset timing is consistent with this mechanism:
deferred-EB validation requires (a) an EB whose body validation lags
its certifying RB's arrival, and (b) the certifying RB's child being
produced before validation completes. Both conditions are increasingly
likely as the network warms up; the first significant deferred validation
at the representative happens around slot 84-87 in this run, which is
exactly when the trajectories diverge.

## Reference trajectory

Recapped from `.planning/chain-derived-fix-revalidation-2026-05-14.md`
and verified against
`sim-rs/output/phase-2/smoke/sundaeswap-batch-20260514-134450/sundaeswap-singlelane/eip1559_d4_t50_w32/eip1559_d4_t50_w32/1/time_series.csv`
(chain-derived post-bug-1-fix) and
`sim-rs/output/phase-2/smoke/sundaeswap-singlelane/eip1559_d4_t50_w32/eip1559_d4_t50_w32/1/time_series.csv`
(accumulator):

| slot | acc c_priority | new c_priority | implied steps acc-vs-new |
|-----:|---------------:|---------------:|--------------------------|
| 0    |             44 |             44 | 0 = 0                    |
| 21   |             55 |             55 | 1 = 1 (44 → 55)          |
| 49   |             69 |             69 | 1 = 1 (55 → 69)          |
| 75   |             69 |             69 | 0 = 0                    |
| 87   |       **109**  |       **87**   | **2 ≠ 1**                |
| 99   |            109 |             87 | 0 = 0                    |
| 199  |            172 |            137 | 1 = 1 (each)             |
| 499  |         19,826 |          3,842 | many more on acc         |
| 999  |         12,720 |        457,423 | reversal (new runs hot)  |
| 1998 |        130,553 |         12,639 | acc 10× higher           |

The trajectory match through slot 75 confirms the bug-1 fix removed
the one-step lag. The divergence at slot 87 (chain-derived 1.26× step
vs accumulator 1.58× step) is the bug-2 signature: one extra
clamped controller step in the accumulator's history that chain-
derived missed. After this, the gap compounds — once two trajectories
diverge by one clamped step, every subsequent block's sample affects
both differently because the *quotes themselves* differ.

Mid-run reversal (slot ~999, chain-derived runs *hotter* than
accumulator) is consistent with the accumulator having earlier hit
a strong-saturation regime and stepped up faster, then later
overshooting and returning toward the floor while chain-derived was
still climbing. Not a separate bug — a downstream consequence of the
step-count asymmetry.

The price-shock diagnostic crystallises this:

| Metric                                   | Accumulator |  Chain-derived |
|------------------------------------------|------------:|---------------:|
| `max_single_step` (max slot-to-slot ×)   |       1.946 |      **1.261** |
| `max_window` (max upward shock, 14-slot) |       3.03  |          1.94  |
| `p90_window`                             |       1.31  |          1.25  |

`max_single_step = 1.946×` ≈ (1.25)³ ≈ 1.953. Three clamped controller
steps coalesced into one slot. Under chain-derived this is
**structurally impossible** — at most one block lands per slot at the
representative (with zero slot battles confirmed), and each block
produces one `derived_quote`. The accumulator's 1.946× can only have
come from `apply_eb_priced_block` firing additional steps inside the
same slot window. Chain-derived has no equivalent path — its
`finish_validating_eb` handler only updates the sample cache and never
calls `compute_derived_quote`.

## Hypothesis analysis

### H1: `update_aggregate` vs `aggregate_from_chain` divergence — RULED OUT

The fear was that `update_aggregate` (incremental: parent's aggregate
+ new samples − evicted) and `aggregate_from_chain` (cold: walk N
blocks back and sum) could disagree, causing compounding drift.

Reading `compute_chain_derived_quote_for_child_of` at
`linear_leios.rs:2179-2221`:

```rust
let parent_aggregate = parent.and_then(|id| self.window_aggregate(id))
    .unwrap_or(WindowAggregate::ZERO);
let parent_samples: Vec<PricedBlockSample> = parent
    .map(|id| self.samples_in_block(id).to_vec())
    .unwrap_or_default();
let evicted_samples: Vec<PricedBlockSample> = parent
    .filter(|_| window_length != usize::MAX)
    .and_then(|p| {
        let k = u32::try_from(window_length).ok()?;
        let ancestor_id = self.ancestor(p, k)?;
        Some(self.samples_in_block(ancestor_id).to_vec())
    })
    .unwrap_or_default();
self.pricing.compute_derived_quote(parent_quote, parent_aggregate,
    &parent_samples, &evicted_samples)
```

The pattern is sound by induction: if parent.window_aggregate contains
samples from blocks {parent-N, parent-N+1, ..., parent-1}, then adding
samples_in_block(parent) and evicting samples_in_block(ancestor(parent, N))
gives an aggregate over {parent-N+1, ..., parent} — the correct window
for the new child.

The ancestor math is correct: with `ancestor(P, k)` walking k parent
links from P (verified at `linear_leios.rs:2851-2860`), `ancestor(P,
window_length)` is the block whose samples must roll off when adding
parent's samples. (The doc comment at `linear_leios.rs:2204-2209` says
"the (window_length-1)-ancestor" — the comment is *wrong*; the code
uses `window_length` directly and the math checks out.)

Crucially, at slot 100 the chain has ~5 canonical RBs (rb-prob 0.05 ×
100 slots), which is far short of `window_length = 32`. **Eviction
hasn't fired yet at the divergence onset.** Even if the eviction logic
were buggy, it cannot explain divergence at slot 87 because no sample
has yet rolled off the tail.

The `update_aggregate` unit test
(`tx_pricing/window.rs:167-182, ring_evicts_oldest_when_full`)
exercises eviction and asserts the correct result. Verdict: **RULED OUT**.

### H2: `block_samples` cache pruning timing — RULED OUT

`prune_block_samples` (`linear_leios.rs:2325-2346`) keeps blocks at
distance ≤ `2 × window_length = 64` behind the chain tip. The eviction
target for the controller is at distance `window_length = 32` behind
the chain tip — well inside the kept range. Furthermore, at slot 100
the chain has ~5 RBs, far short of the prune threshold.

Pruning cannot affect the controller computation. Verdict: **RULED OUT**.

### H3: Window-eviction off-by-one in `update_aggregate` — RULED OUT (different reason from H1)

This hypothesis specifically targeted the `add` vs `evict` ordering
inside `update_aggregate`. The code at
`tx_pricing/window.rs:54-68` adds all `add_samples` then subtracts all
`evict_samples`. Both operations use `saturating_add`/`saturating_sub`
on u128 fields. As long as the caller passes the correct sets, the
math is exact (u128 has plenty of headroom for these magnitudes).

At slot 100 with window not yet full, `evict_samples` is empty (the
ancestor at distance window_length doesn't exist). Eviction logic
hasn't fired, so any off-by-one in eviction cannot explain the
divergence onset.

Verdict: **RULED OUT**.

### H4: EB-deferred sample timing — SUPPORTED (this is the bug)

The hypothesis: deviation #5 (`finish_validating_eb`'s cache-overwrite
behavior) creates a real path-dependence that decouples
chain-derived's controller trajectory from the accumulator's.

The bug #1 investigation ruled out H4 for the d4 reference case on the
basis that "the d4_t50_w32 case has zero slot battles AND uses the
single-producer/sundaeswap-realistic topology where the producer also
endorses EBs synchronously at production time."

**This rationale was incorrect.** Two errors:

1. **"Single-producer topology" was wrong.** The
   `phase-2-sundaeswap-singlelane.yaml` suite uses
   `default-topology: parameters/phase-2-sweep/topology-realistic-100.yaml`,
   which is a 100-node mainnet-faithful topology. Only `node-0` has
   `tx-generation-weight: 1`, but all 100 nodes participate in the RB
   lottery and produce blocks proportional to their stake.

2. **"Zero slot battles" was conflated with "single producer".** Slot
   battles are specifically 2+ RBs at the *same slot*. The smoke is
   zero-SB because at rb-prob = 0.05 across 100 nodes, the per-slot
   collision rate is ~0.001. But the canonical chain still alternates
   between many producers across its length, each with its own local
   EB-validation state.

The mechanism in detail:

Legacy `apply_priced_block(rb)` at
`git show 2f8ac5e:.../linear_leios.rs:2060-2082`:
- Push RB body sample (if non-empty).
- Push EB body sample *iff* `get_validated_eb(endorsement.eb)`
  succeeds locally now.
- Call `update_after_block(samples)` → push samples to window, run
  `step()` **once**.

Legacy `apply_eb_priced_block(eb)` at
`git show 2f8ac5e:.../linear_leios.rs:2084-2094`, invoked from
`finish_validating_eb` (legacy line 1447):
- Push EB body sample.
- Call `update_after_block(samples)` → run `step()` **once**.

So in legacy, an RB endorsing an EB whose validation is deferred
produces **two controller steps over time**: one at RB publish (with
RB-only sample, since EB not yet validated), and one later at EB
validation (with EB-only sample, separate `step()`).

Chain-derived's `publish_rb` at `linear_leios.rs:1043-1103`:
- Builds the RB at line 892, computing `derived_quote` and
  `window_aggregate` **once** via
  `compute_chain_derived_quote_for_child_of(parent)`.
- Caches `samples_for_rb(rb)` at line 1093. If parent's EB isn't
  validated locally yet, the cache has RB-body-only samples for parent.
- The new RB's `derived_quote` is fixed.

Chain-derived's `finish_validating_eb` at `linear_leios.rs:1496-1541`:
- Overwrites `block_samples[parent_rb_id]` with the now-complete
  RB+EB samples.
- Calls `revalidate_against_new_tip` (lines 2353-2380), which
  re-queries the gate against `current_chain_tip_quote`. **It does
  not run a controller step.** The chain tip's `derived_quote` is
  unchanged (it's a block field, frozen).

So when a deferred EB validates in chain-derived, no controller step
fires. The EB sample is recorded in the cache for descendants to pick
up — *but only if those descendants are direct children of the
certifying RB AND those descendants are produced after the validation*.
Descendants further down the chain don't fold the EB sample in either:
their parent's stored `window_aggregate` is frozen and doesn't
include the EB sample, so the EB sample's contribution is permanently
absent from the canonical chain's controller state.

**Step-count accounting on the canonical chain, sundaeswap d4_t50_w32
seed=1, slot 87 transition:**

- 1 RB lands; producer's view of parent's samples = RB-body-only
  (some prior EB was still in flight). New RB's `derived_quote` =
  step(69, parent_aggregate with RB-only sample) = 87 (one clamped
  +25%).
- Meanwhile/shortly after, a deferred EB validates somewhere on the
  chain. In legacy, this fires an extra `step()` against the window
  including the EB body sample → quote moves from 69 to ~86 via the
  "RB" step, then from ~86 to 109 via the "EB" step (two clamped
  +25% steps). End: 109.
- In chain-derived, the deferred-EB validation only updates the
  sample cache; no step. End: 87.

Numerically: 69 × 1.25 = 86.25 → ceil → 87. So chain-derived's 87 is
exactly *one* clamped step. 69 × 1.25² ≈ 107.81 → ceil → 109. So
accumulator's 109 is exactly *two* clamped steps. The arithmetic
matches the hypothesis exactly.

Cross-check via the `price_shock` diagnostic: accumulator records
`max_single_step = 1.946×` ≈ 1.25³, indicating that somewhere in the
run, a single slot saw *three* clamped controller steps land — which
must mean three `apply_*_block` events at that slot (e.g. 1 RB + 2
deferred EBs validating). Chain-derived records `max_single_step =
1.261×` ≈ 1.25 (with rounding), the exact one-step ceiling. The
chain-derived bound is structurally hard-capped at 1.26× under
window_length × `(D + 1) / D` for D = 4.

Verdict: **SUPPORTED. This is the bug.**

### H5: `window_aggregate` field — stored vs re-derived asymmetry — PARTIAL (cosmetic only)

`current_chain_tip_aggregate()` (`linear_leios.rs:2289-2295`) returns
the chain tip's *stored* `window_aggregate` (pre-step). This drives
`util_priority_window_x_1e9` and `util_standard_window_x_1e9` in
`PricingTick` → `time_series.csv`.

`current_chain_tip_quote()` (post-bug-1 fix, lines 2278-2285) returns
the *post-step* quote (re-derived via
`compute_chain_derived_quote_for_child_of`).

The asymmetry: at any slot S, the trajectory shows the quote *for the
next block* but the utilization *for the current tip block*. This is
a one-step phase shift in the reporting layer only.

Empirical evidence at slot 21 in the post-fix run:
`c_priority = 55` but `util_standard_window_x_1e9 = 0`. The tip at
slot 21 is RB-1 (the first non-warm-up RB), whose `window_aggregate`
is ZERO (= aggregate over zero ancestor blocks). The quote `55`
correctly reflects "RB-1 had a saturated body; step from 44 → 55";
the displayed util `0` reflects RB-1's *pre-step* aggregate.

This is the same asymmetry called out in the bug-1 investigation's
fix recommendation (`bug-investigation.md` lines 489-493). It is
**cosmetic** — it affects reporting only and does not feed back into
controller state. The `pricing_event_stream.sha256` and downstream
welfare metrics are unaffected.

Verdict: **PARTIAL. Real but not the trajectory-divergence root cause.**

### H6 (new): "Predicted-child" semantics at the representative node — RULED OUT, but worth noting

When the representative observes `current_chain_tip_quote` for
`emit_pricing_tick` / admission / lane choice, it re-derives the
*hypothetical child's* `derived_quote` from the rep's local cache. If
the rep's local cache differs from the actual next-block producer's
cache (e.g. different EB-validation timing), the rep's prediction
won't match the next block's actual `derived_quote`.

But this affects only the per-slot trajectory observation, not the
on-chain `derived_quote` sequence. Once the actual next RB lands at
the rep, the rep's view of `latest_rb_id()` updates to the new tip,
and subsequent observations re-derive against the new tip — which has
its own stored `derived_quote` (computed by its producer).

This is a divergence in *what the rep thinks the next block's quote
is during the inter-block interval*, not in the canonical chain's
state. For the trajectory, the rep's prediction influences the
sample-rate path (admissibility decisions during the inter-block
interval) but does not retroactively alter chain state.

Verdict: **RULED OUT for the trajectory divergence**, but flagged as
a sub-architectural concern in the "could there be a bug #3" section.

## Localized diagnosis

**File:** `sim-rs/sim-core/src/sim/linear_leios.rs`
**Lines:** 1496-1541 (`finish_validating_eb`) and 2301-2320
(`samples_for_rb`), in combination.

Code path in chain-derived (current):

```rust
// linear_leios.rs:2301 — samples_for_rb conditional on EB validation
fn samples_for_rb(&self, rb: &RankingBlock) -> Vec<PricedBlockSample> {
    let mut samples = Vec::new();
    if !rb.transactions.is_empty() { /* RB body sample */ }
    if let Some(endorsement) = &rb.endorsement
        && let Some(eb) = self.get_validated_eb(endorsement.eb)  // ← gate
    {
        /* EB body sample only if validated */
    }
    samples
}
```

```rust
// linear_leios.rs:1496 — finish_validating_eb only updates cache, no step
fn finish_validating_eb(&mut self, eb: Arc<EndorserBlock>, _seen: Timestamp) {
    if self.leios.incomplete_onchain_ebs.remove(&eb.id()) {
        // ... locate parent RB ...
        if let Some(rb_id) = parent_rb_id {
            let samples = self.samples_for_rb(&rb_arc);
            self.block_samples.insert(rb_id, samples);  // ← overwrite only
        }
        self.revalidate_against_new_tip(slot);  // ← gate sync, no controller step
    }
    // ...
}
```

Compare to legacy accumulator (verified via
`git show 2f8ac5e:sim-rs/sim-core/src/sim/linear_leios.rs`):

```rust
// legacy linear_leios.rs:1441 — fires apply_eb_priced_block on deferred validation
fn finish_validating_eb(&mut self, eb: Arc<EndorserBlock>, seen: Timestamp) {
    if self.leios.incomplete_onchain_ebs.remove(&eb.id()) {
        self.remove_eb_txs_from_mempool(&eb);
        self.apply_eb_priced_block(&eb);  // ← ONE FULL STEP
    }
    // ...
}

// legacy linear_leios.rs:2084
fn apply_eb_priced_block(&mut self, eb: &EndorserBlock) {
    let slot = ...;
    let samples = self.eb_samples(eb);
    self.feed_samples_and_revalidate(slot, &samples);  // push + step
}
```

The accumulator's `feed_samples_and_revalidate` (legacy lines
2096-2160) calls `pricing.update_after_block(samples)`, which pushes
samples to the window then calls `step()` — a full clamped EIP-1559
update. This is **the missing controller step** in chain-derived.

**Structural note:** the chain-derived spike intentionally moved
controller updates out of the `apply_*_block` event flow and into
block production (per spike 007 design: "the values used for tx
admissibility are the new block's own `derived_quote` — the
controller's future is fixed at the moment of production"). This is
the right invariant for sibling-block-orphan robustness (which fixes
WR-1), but it has the secondary consequence of binding
controller-step count to *block count* rather than to *apply-event
count*. In multi-node topologies with EB-validation propagation
delays, the gap between apply-event count and block count is exactly
the bug.

## Window-rotation timing analysis

Sanity-check: at what slot does the rolling window first fill, and
does that align with the divergence onset?

The reference case uses `rb-generation-probability = 0.05` and
`default-slots = 2000`, so the chain produces ~100 canonical RBs over
2000 slots (~1 RB per 20 slots on average). `window_length = 32`.

| Slot      | Expected chain depth | Window state                          |
|-----------|---------------------:|---------------------------------------|
| 0-20      |        0 (cold start) | Empty; quote at 44                    |
| 20-40     |                  ~1-2 | First samples enter window            |
| 80-100    |                  ~4-5 | Window holds ~4-5 samples (not full)  |
| 600-650   |                  ~32  | **Window first becomes full**         |
| 1280-1320 |                  ~64  | First full rotation completes         |

The empirical divergence at slot 87 is **far inside the warm-up
regime**. The window holds only ~4-5 samples, eviction logic hasn't
fired, and no sample has rotated off the tail. This empirically rules
out H1, H2, and H3 (all of which depend on eviction or rotation) as
the divergence-onset mechanism.

It does *not* rule out H4: deferred-EB validation is unrelated to
window-rotation count. It depends only on (a) an RB endorsing an EB
landing on chain and (b) the EB body's validation completing *after*
the RB endorsing it has been published. Both conditions are
satisfiable from the first endorsed-EB RB onward — i.e. starting at
the very first canonical RB that endorses an EB.

The first RB landed around slot 17-20 (per the `34820`-byte
`mempool_bytes_priority` jump at slot 21 in both runs). The first
divergence at slot 87 reflects the first occurrence where the
representative observes a *deferred*-EB validation completing inside
its run window. In a 100-node topology with ~50-200 ms latencies and
12 MB EB bodies, validation propagation can take 1-5 seconds (= 5-25
slots in the 1-slot-per-second clock); the slot-87 onset is consistent
with the second or third RB landing's EB taking that long to validate
at the representative.

## Predicted-vs-observed cross-check

If H4 is correct, predictions:

1. **Single-producer topology should NOT see bug #2.** With one
   producer, that producer has every EB locally validated synchronously
   at the moment of RB build, so `samples_for_rb` always returns
   complete RB+EB samples. No deferred path. Chain-derived =
   accumulator on single-producer.

   This is testable but not yet directly observed. The "smoke" used
   `topology-realistic-100.yaml`. A `topology-single-producer.yaml`
   variant with the same pricing/demand should match accumulator
   byte-for-byte. (Existing unit test
   `chain_derived_matches_accumulator_on_zero_slot_battle_run` per
   bug-1 recommendation list would catch this exact scenario.)

2. **Larger D (slower controllers) should have smaller divergence.**
   Each missing step is smaller at D = 8 or D = 16, so the
   cumulative gap accrues more slowly. Observed (revalidation note):
   `eip1559_d4_t50_w32` final ratio = 0.10× (chain-derived 10× lower);
   `eip1559_d16_t50_w32` final ratio = ~0.22× (smaller divergence).
   Consistent with the +1/D step-size dependence.

3. **Larger window_length (more smoothing) should *partially* mask
   the divergence.** With a longer window, each missing EB sample
   contributes a smaller fraction of the aggregate. Observed:
   `eip1559_d8_t50_w64` final ratio is dramatic (155 vs 141 489), so
   in saturated regimes the controller saturates regardless of window
   length. Direction not cleanly testable from the existing data.

4. **`baseline_flat_fee` must remain byte-identical.** Confirmed
   (revalidation note: still matches byte-for-byte). Flat-fee
   controller has no state to lose; the bug doesn't apply.

5. **Two-lane suites should show priority controller diverging more
   than standard.** Priority signals come from EB content for some
   variants (un-reserved priority); RB-reserved priority only emits
   from RB body. The deferred-EB path affects EB samples only, so:
   - Un-reserved priority controllers feel the bug strongly.
   - Both-dynamic standard controllers feel the bug (their signal
     also samples EB content).
   - RB-reserved priority controllers feel the bug *less* (their
     signal is per-block fill rate of RB-reserved priority bytes,
     length 1; EB contribution is absent).

   Observed: `rb_reserved_x4 / partitioned_x4` zero-SB job shows ~90%
   delta; `unreserved` variants show wider deltas. Roughly consistent
   but mixed by other confounds (different multiplier floors, different
   variants).

Five of six predictions consistent with H4 (the sixth is untested but
testable). Strong empirical alignment.

## Recommended surgical fix

**Do NOT apply patches per the investigation constraints.** The fix
below is described, not implemented.

There are two structurally different families of fix, with materially
different downstream consequences. The user must pick.

### Family A: "Fire a step on deferred EB validation" (accumulator-equivalent)

Inside `finish_validating_eb` (`linear_leios.rs:1496-1541`), after
overwriting `block_samples[rb_id]` with the updated samples, also
update the parent RB's stored `window_aggregate` and `derived_quote`
to reflect the additional samples, AND propagate that update forward
to all descendants of `rb_id` already on the canonical chain.

This is **intrusive**:

- `LinearRankingBlock.derived_quote` and `window_aggregate` are
  currently treated as immutable post-production. Family A requires
  making them mutable, breaking the "pure function" invariant from
  spike 007.
- The forward-propagation walk is O(blocks-since-rb_id), which is
  bounded by the prune horizon `2 × window_length` but is still a
  non-trivial mutation per deferred-EB validation.
- All canonical descendants' stored `derived_quote` change values,
  so consumers cached on those values (e.g. the gate's earlier
  admission decisions) need re-validation.

A simpler variant: instead of propagating mutations, compute the
descendant's `derived_quote` lazily on each query, walking back
N blocks and aggregating from the *current* `samples_in_block` cache
(which already reflects deferred-EB updates). This is the
"cold-start-only" fix:

- Replace `compute_chain_derived_quote_for_child_of` with a version
  that always uses `aggregate_from_chain` over the last N canonical
  blocks (rather than the incremental
  `parent_aggregate + parent_samples - evicted` path).
- Each block's stored `window_aggregate` field becomes a stale
  optimisation/cache only; consumers ignore it for canonical-chain
  queries.

This restores **some** of the missing EB samples (those that validate
before the next consumer query) but does **not** match accumulator
step-count semantics: each canonical block still produces one
`derived_quote`, not one-per-apply-event. A deferred EB validation
mid-run still skips its "own" controller step; the EB sample just gets
folded into the next-produced block's step instead.

Pre/post sketch (cold-start-only variant):

```rust
// BEFORE: linear_leios.rs:2179-2221 (incremental aggregate)
fn compute_chain_derived_quote_for_child_of(&self, parent: Option<BlockId>)
    -> (PerLaneQuote, WindowAggregate)
{
    let parent_aggregate = parent.and_then(|id| self.window_aggregate(id))
        .unwrap_or(WindowAggregate::ZERO);
    let parent_samples = parent.map(|id| self.samples_in_block(id).to_vec())...;
    let evicted_samples = ...;
    self.pricing.compute_derived_quote(parent_quote, parent_aggregate,
        &parent_samples, &evicted_samples)
}

// AFTER: always re-aggregate from the canonical chain
fn compute_chain_derived_quote_for_child_of(&self, parent: Option<BlockId>)
    -> (PerLaneQuote, WindowAggregate)
{
    let window_length = self.pricing.effective_window_length();
    let parent_quote = parent.and_then(|id| self.derived_quote(id))
        .unwrap_or_else(|| /* cold-start */);
    // Walk back up to `window_length` canonical blocks from `parent`,
    // concatenating their current samples_in_block.
    let mut all_samples = Vec::new();
    let mut cur = parent;
    for _ in 0..window_length.min(u32::MAX as usize) {
        match cur {
            None => break,
            Some(id) => {
                all_samples.extend_from_slice(self.samples_in_block(id));
                cur = self.ancestor(id, 1);
            }
        }
    }
    // Hand to backend with empty incremental inputs.
    self.pricing.compute_derived_quote(
        parent_quote,
        crate::tx_pricing::window::aggregate_from_chain(all_samples.iter()),
        &[], &[],
    )
}
```

This is **the smaller change**. It preserves the chain-derived
"one-step-per-block" invariant but fixes the path-dependence by
reading `samples_in_block` fresh each query. After a deferred EB
validates and updates the cache, the *next* `compute_derived_quote`
call (whether at production time for a new RB, or for the
representative's `emit_pricing_tick`) will pick up the EB sample
via the walk-back.

Note: the cold-start fix does NOT fully restore accumulator behaviour.
It only ensures the EB sample isn't permanently lost. The
"missing extra step" asymmetry — legacy fires N steps per N apply
events, chain-derived fires 1 step per block — remains.

### Family B: "Accept the architectural difference"

The chain-derived design intentionally collapses RB+EB into a single
controller event per canonical block. This matches Cardano's
production stance (block-level rather than per-action mutation), and
EIP-1559's per-block stepping. The accumulator's "step per
apply_*_block" was an implementation artifact, not a spec
requirement.

Under Family B, the fix is to:
- Adopt the cold-start-only path (as in Family A's smaller variant)
  to ensure deferred EB samples aren't permanently lost.
- **Accept that the chain-derived canonical-chain trajectory differs
  from the accumulator's**, with chain-derived's trajectory matching
  what a real EIP-1559-style implementation would produce.
- Regenerate the M5 suite goldens. WR-1 is closed by construction.

The "smoke comparison must converge byte-identically on zero-SB jobs"
expectation set by spike 007 (sibling-block sections) is then
**incorrect as stated**: chain-derived is not equivalent to the
accumulator even in the absence of slot battles, because the
underlying mechanism is different.

### Recommendation

Family B is more defensible if the user's goal is a Cardano-aligned
mechanism. The accumulator's per-apply-event semantics has no obvious
spec justification — `mechanism-design.md` describes the update rule
in terms of *blocks*, not in terms of *apply events*. The per-EB
extra step is an implementation choice that the chain-derived design
implicitly corrects.

Family A is required if the user's goal is byte-identical
accumulator-replacement on zero-SB scenarios for WR-1 disclosure.
This requires the intrusive mutation/propagation work and breaks
the "pure function" invariant from spike 007.

A reasonable middle path is to:
1. Apply the cold-start fix from Family A's smaller variant (smaller
   change, restores deferred-EB sample availability for descendants).
2. Document that chain-derived and accumulator diverge on
   deferred-EB scenarios by design, with the chain-derived trajectory
   reflecting "per-block controller updates" (Cardano-aligned).
3. Regenerate goldens. Update WR-1 disclosure to reflect that
   chain-derived eliminates orphan-block contamination but is NOT
   trajectory-equivalent to the accumulator under multi-node
   topologies.

This corresponds to Family B with the cold-start fix applied.

## Recommended regression test

Add to `sim-core/src/sim/tests/m_chain_derived.rs` (or wherever the
chain-derived test suite lives):

```rust
#[test]
fn deferred_eb_validation_folds_into_next_block_derived_quote() {
    // Construct a minimal 2-node scenario:
    //   - Node A produces RB-1 endorsing EB-1, with EB-1 not yet
    //     validated at Node B.
    //   - Node B receives RB-1 and publishes it (samples_for_rb
    //     returns RB-only samples at B because EB-1 isn't validated).
    //   - EB-1 then validates at B: block_samples[RB-1.id] is
    //     overwritten to include EB-1 samples.
    //   - Node B produces RB-2 with parent = RB-1. Under the
    //     cold-start fix, RB-2.derived_quote should now reflect the
    //     EB-1 sample.
    //
    // Under the BUGGY incremental path, RB-2.derived_quote would
    // miss EB-1 (the EB sample is in samples_in_block[RB-1] but
    // RB-1.window_aggregate is frozen, and the incremental path
    // adds samples_in_block[RB-1] to RB-1.window_aggregate, which
    // double-counts the RB body sample and adds the EB sample
    // exactly once — actually wait, this IS correct under the
    // incremental path because samples_in_block[RB-1] reflects
    // the post-deferred-validation state. The bug is in the
    // intermediate step where descendants produced BEFORE the
    // deferred validation have their derived_quote frozen
    // missing the EB sample).
    //
    // Better assertion: produce RB-2 BEFORE EB-1 validates at B;
    // observe RB-2.derived_quote = step(RB-1, RB-only samples).
    // Then produce RB-3 AFTER EB-1 validates; under the cold-start
    // fix, RB-3.derived_quote should fold in EB-1's sample (via
    // walk-back over RB-1's now-updated cache). Under the buggy
    // incremental path, RB-3 uses RB-2.window_aggregate (which
    // doesn't have EB-1) + samples_in_block[RB-2] (no EB-1) → EB-1
    // permanently absent.
}

#[test]
fn synchronous_eb_validation_matches_accumulator_one_step() {
    // Sanity check: when EB validates synchronously (immediately at
    // production), chain-derived and accumulator should both step
    // exactly once with both samples (RB + EB) in the window.
    // No path-dependence; no off-by-one.
}
```

A property test asserting "chain-derived's pricing event stream equals
accumulator's pricing event stream on single-producer scenarios with
no slot battles" is the strongest regression guard. The accumulator
implementation would need to be retained behind a feature flag for
this to work.

## Confidence

**HIGH** for the H4 mechanism and its localisation.

Justifications:

1. The trajectory arithmetic at slot 87 *exactly* matches "one
   clamped +25% step in chain-derived vs two clamped +25% steps in
   accumulator". The reading is `chain-derived: 69 × 1.25 = 86.25
   → ceil → 87`; `accumulator: 69 × 1.25² ≈ 107.81 → ceil → 109`. No
   slack for alternative explanations.

2. The `price_shock` diagnostic shows `max_single_step = 1.946×` on
   accumulator (= 1.25³ = three clamped steps coalesced) but only
   1.261× on chain-derived (= 1.25 = exactly one clamped step). This
   is a hard structural bound — chain-derived cannot exceed one
   clamped step per block at the representative.

3. The slot ~85-100 onset timing is consistent with EB-validation
   propagation latency in a 100-node topology. The first canonical
   RB lands around slot 17; by slot ~85 several RBs and EBs are in
   flight, and the first deferred-EB validation completing at the
   representative is in this window.

4. The bug-1 investigation's H4 rule-out was based on a faulty
   premise ("single-producer topology"). The reference case uses the
   100-node realistic topology, where deferred-EB validation is the
   norm rather than the exception.

5. The code path is concrete and observable: `samples_for_rb`'s
   conditional EB inclusion gate (line 2310-2317),
   `finish_validating_eb`'s cache-overwrite without a controller
   step (lines 1521-1533), and the absence of any
   `compute_derived_quote` call on the EB-validation path in
   chain-derived.

What would shake confidence:

- An empirical scenario where deferred-EB validation is *zero* yet
  the divergence still occurs (single-producer mode would test this).
- Discovery that `finish_validating_eb` is **never** entered for the
  d4 reference run — but the price_shock = 1.946× metric on the
  accumulator side would be unexplained.
- The mid-run reversal (slot ~999 chain-derived runs hot, slot ~1500
  it cools) — this is consistent with H4 once divergence has set in
  (the two trajectories take different paths through the
  saturation-clamp regime), but it's hard to directly attribute to
  H4 without instrumenting `finish_validating_eb` calls per slot.

The first two concerns would directly falsify the mechanism. The
third is consistent with H4 but not uniquely so.

## Could there be a bug #3?

**Yes, structurally possible. Cautiously: probably not for the d4
sundaeswap reference case, but quite likely in other regimes.**

Reasons to suspect more bugs:

1. **Multi-node "predicted-child" semantics.**
   `current_chain_tip_quote` returns the representative's local
   prediction of the *next* block's `derived_quote`, computed from
   the rep's local cache. Different nodes have different
   `samples_in_block` caches (due to different EB validation
   timing). So at any moment in time, different nodes have different
   answers to "what is the canonical chain's current controller
   state". Admission decisions, lane choice, and EB endorsement
   validation all read this varying value. There may be effects
   here that aren't yet diagnosed; the bug-2 investigation noted
   this as H6 (RULED OUT for the trajectory divergence specifically,
   but raised as a sub-architectural concern).

2. **Sub-architectural mismatches in the deferred-EB
   sample/aggregate accounting.** The cache overwrite at
   `finish_validating_eb:1532` updates `block_samples[rb_id]` but
   does not update `rb.window_aggregate`. Direct children of `rb`
   produced after the validation will (under the cold-start fix)
   correctly fold in the EB sample. But descendants produced
   *before* the validation already have their `window_aggregate`
   stored, and their descendants compute against that frozen
   aggregate. The EB sample is missing from the deeper canonical
   chain unless the cold-start fix walks back far enough to fold it
   in directly.

3. **EB-validation race vs producer-side `try_generate_rb`.** When
   a producer is about to build an RB, it might endorse an EB it
   hasn't fully validated locally yet (line 775's
   `get_validated_eb` check). In this case the producer falls into
   `incomplete_onchain_ebs` and produces an empty block (line 810
   `produce_empty_block`). The chain-derived state evolution in
   this case is consistent with legacy (both produce empty blocks),
   but the `derived_quote` of the empty block is still computed
   from `samples_in_block(parent)` which may itself be partial.
   Likely not a separate bug, but worth tracing.

4. **Two-lane multiplier-floor + deferred EB.** When a deferred EB
   validates after a two-lane block has computed its
   `derived_quote` with the multiplier-floor applied, the EB sample
   might break the floor invariant retrospectively (because the
   floor was computed against the wrong aggregate). The block's
   stored `derived_quote` is fine, but the descendant chain might
   accumulate floor-violating intermediate quotes if the cold-start
   fix is applied. Worth a targeted test on two-lane suites.

Reasons to expect bug #2 is the last one for the d4 reference case:

- The d4_t50_w32 sundaeswap-singlelane setup is single-lane
  (no multiplier floor), single-controller (no cross-lane
  interaction), zero-slot-battle (no orphan effects). The only
  remaining axis of divergence is the deferred-EB-step-count
  asymmetry, which H4 fully accounts for at the slot 87 transition.
- The price-shock diagnostic `max_single_step = 1.946× → 1.261×`
  delta is fully explained by H4. There is no residual gap that
  another mechanism needs to fill.
- The arithmetic checks out (69 → 87 = one step; 69 → 109 = two
  steps). No fitting noise required.

But for the broader sweep — especially two-lane suites with both
controllers, and any scenario with intentionally high EB
propagation latency — additional bugs are plausible.

**Recommendation before re-validating:** invest in a
property-based test that compares the accumulator and chain-derived
implementations on a randomised single-producer scenario space
(where the deferred-EB path is structurally absent and the two
implementations should match byte-for-byte). If they match on
single-producer across a broad randomised sweep, then the *only*
remaining divergence source is multi-node EB-deferred semantics
(= bug #2 as scoped above), and Family B is defensible. If they
don't match even on single-producer, then bug #3 (and possibly more)
is still hiding.

A complementary smoke: add a deferred-EB validation count metric to
`run_summary.json` and re-run the existing smoke. The
divergence-to-deferred-EB-count correlation across the 33 jobs would
directly validate or refute H4 as the *primary* mechanism, and would
quantify how much residual divergence (if any) remains after
accounting for deferred-EB step-count loss.

## Sources

- Code references: `sim-rs/sim-core/src/sim/linear_leios.rs`
  (current state, post-bug-1 fix). All line numbers refer to current
  HEAD.
- Legacy code references: `git show 2f8ac5e:...` commit (the
  pre-chain-derived accumulator implementation).
- Trajectory data:
  `sim-rs/output/phase-2/smoke/sundaeswap-batch-20260514-134450/sundaeswap-singlelane/eip1559_d4_t50_w32/eip1559_d4_t50_w32/1/`
  (post-fix) and
  `sim-rs/output/phase-2/smoke/sundaeswap-singlelane/eip1559_d4_t50_w32/eip1559_d4_t50_w32/1/`
  (accumulator), specifically `time_series.csv` and `diagnostics.log`.
- Prior investigation:
  `.planning/chain-derived-bug-investigation.md` (bug #1).
- Revalidation note:
  `.planning/chain-derived-fix-revalidation-2026-05-14.md`.
- Spike design:
  `.planning/spikes/007-chain-derived-controller/README.md`.
