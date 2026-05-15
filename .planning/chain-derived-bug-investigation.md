# Chain-derived controller regression — bug investigation

Date: 2026-05-14
Symptom: post-refactor smoke shows welfare sign-flip on a zero-slot-battle
job (`sundaeswap-singlelane / eip1559_d4_t50_w32`) that spike 007's
correctness contract required to converge byte-identically with the
accumulator implementation.

## TL;DR

**Best-guess root cause: one-step lag in `current_chain_tip_quote(lane)`
relative to the accumulator's `pricing.current_quote(lane)` at producer
production time.** The function reads the latest canonical RB's stored
`derived_quote`, but a canonical RB's stored `derived_quote` represents
*the quote at the moment that RB was produced* (i.e., it was stepped
from its parent's quote using **its parent's** samples). It does **not**
include the step that folds in *that block's own* samples. In the legacy
accumulator path, `pricing.current_quote()` after `publish_rb(B)` →
`apply_priced_block(B)` reflected exactly that missing step — it
included B's samples. Therefore, in chain-derived, every consumer that
reads "the current canonical-chain quote" (actor lane-choice,
`try_add_tx_to_mempool` admission, `eb_endorsement_valid`, EB-inclusion
charging) sees a quote that is **one controller step behind** what the
accumulator would have shown at the same logical moment.

Localised file/lines:
- `sim-rs/sim-core/src/sim/linear_leios.rs:2263-2269` — definition of
  `current_chain_tip_quote`. Reads `rb.derived_quote.get(lane)` directly
  from the chain tip; should instead fold in the chain-tip's *own*
  samples (i.e., return what a hypothetical child of the chain tip would
  have for `derived_quote`).
- `sim-rs/sim-core/src/sim/linear_leios.rs:1862` — admission call site
  consuming the lagged value.
- `sim-rs/sim-core/src/sim/linear_leios.rs:793` — EB-inclusion charge
  consuming the lagged value (mismatched with line 934's RB body charge
  which uses the new block's `derived_quote` = correct, one-step-ahead
  value).
- `sim-rs/sim-core/src/sim/linear_leios.rs:955-956` — EB endorsement
  validation consuming the lagged value.
- `sim-rs/sim-core/src/sim/linear_leios.rs:2543-2544` — actor lane
  choice in `run_actors_for_slot` consuming the lagged value.

Confidence: **HIGH** for the timing-lag mechanism explaining the d4
zero-slot-battle divergence. Confidence is HIGH because (1) the legacy
accumulator's `apply_priced_block` semantics map provably to "step using
this block's samples after the block is produced," (2) chain-derived's
`block.derived_quote` provably represents "step using this block's
*parent's* samples computed at production," and (3) these are two
different controller states separated by exactly one step. At d4 the
maximum per-step move is ±25%, which is large enough to explain the
observed 10× endpoint divergence over 2000 ticks of asymmetric
admission-vs-charging quote sources.

## Reference case

`sundaeswap-singlelane / eip1559_d4_t50_w32`, seed 1, zero slot battles
on both sides. From the smoke comparison report
(`.planning/smoke-comparison-chain-derived-vs-accumulator-2026-05-14.md`
table around lines 104–123 and the trajectory walk at lines 113–123):

- Final `c_priority`: 130 553 (accumulator) → 12 412 (chain-derived); ratio 0.10.
- Net utility: +2.034e+10 → −1.395e+10; sign flip.
- Trajectory at slot 100: 109 → 69, ratio 0.63 (already diverging).
- Trajectory at slot 1300: 2 425 → 2 968 217, ratio 1224 (peaks at
  different slots, valleys at different slots — not a simple offset).

The control case `baseline_flat_fee` matches byte-for-byte (no
controller state). This isolates the bug to chain-derived controller
computation.

## Hypothesis analysis

### H1: Quote-timing off-by-one in producer admission — SUPPORTED (this is the bug)

The hypothesis stated: "Under accumulator, slot N admits txs against
post-slot-(N−1) quote (= state after parent's `apply_priced_block`).
Under chain-derived, slot N's producer computes the new block's own
`derived_quote` (= function of parent + samples in parent), then admits
txs against this new value."

The actual implementation behavior is **not what H1 described**. Reading
the code:

- `sim-rs/sim-core/src/sim/linear_leios.rs:1862` (`try_add_tx_to_mempool`)
  uses `self.current_chain_tip_quote(tx.posted_lane)`, which returns
  the **parent** (chain-tip) RB's stored `derived_quote` — *not* the
  new block's. Same for actor lane choice (line 2543–2544), `eb_endorsement_valid`
  (lines 955–956), and the EB inclusion charge inside the producer's
  `try_generate_rb` (line 793 → `charge_inclusions` → reads chain-tip
  quote).

- Only the RB body inclusion charge at line 934 uses the new block's
  `rb.derived_quote` directly.

So admission, lane choice, EB-endorsement validation, and EB-inclusion
charging all consume `block_{tip}.derived_quote`. RB-body charging
consumes `block_{new}.derived_quote`. **These two values are separated
by exactly one controller step** — the step that folds in
`block_{tip}`'s own samples.

Now compare to legacy. Under accumulator:

- The legacy controller is mutated by `apply_priced_block` inside
  `publish_rb` (`git show 2f8ac5e:sim-rs/sim-core/src/sim/linear_leios.rs`,
  legacy lines 1044, 2060). At any moment between `publish_rb(B)` and
  `try_generate_rb` for B's child, the controller's
  `pricing.current_quote()` represents the state *after* stepping with
  B's samples.

- When the producer of B's child generates that child, legacy lines
  883 (`charge_inclusions` for the new RB body) and 793 (EB inclusion
  charge) **both** call `charge_inclusions` which reads
  `self.pricing.current_quote()` = state after publish_rb(B) = "stepped
  using B's samples".

- Legacy actor admission and lane choice also read
  `self.pricing.current_quote()` = same value.

So legacy uses one consistent quote for admission, lane choice, EB
endorsement, EB charging, and RB body charging: **"the post-step state
after the chain tip B's samples."**

Chain-derived's `block_{new}.derived_quote` (computed at line 892 of
`linear_leios.rs`) is:
```
step(parent_quote = block_tip.derived_quote,
     aggregate = block_tip.window_aggregate + block_tip's samples − evicted)
```
This **equals legacy's `current_quote` after `apply_priced_block(block_tip)`**
— the same "post-step state".

Chain-derived's `block_{tip}.derived_quote` is one step earlier: it
was computed as `step(block_{tip-1}.derived_quote, ...with block_{tip-1}'s
samples folded in...)`. So `block_{tip}.derived_quote` represents
legacy's `current_quote` at the moment block_{tip} *itself* was being
produced — i.e., one block earlier than the moment a child of block_tip
is about to be produced.

Conclusion: `current_chain_tip_quote()` returns a value **one step
behind** legacy semantics. All consumers of `current_chain_tip_quote`
see a stale (lagged) controller. The new RB's body inclusion charge,
which reads `rb.derived_quote` directly, sees the *correct* (post-step)
value. This asymmetry is the bug.

**Verdict: SUPPORTED.** This is the root cause.

### H2: Window-aggregate off-by-one — RULED OUT

The hypothesis: chain-derived's window aggregate at slot N might
include or exclude the parent's own samples incorrectly.

Reading `compute_chain_derived_quote_for_child_of`
(`linear_leios.rs:2179–2221`):

```rust
let parent_aggregate = parent.and_then(|id| self.window_aggregate(id))
    .unwrap_or(WindowAggregate::ZERO);
let parent_samples = parent.map(|id| self.samples_in_block(id).to_vec())
    .unwrap_or_default();
let evicted_samples = parent
    .filter(|_| window_length != usize::MAX)
    .and_then(|p| {
        let k = u32::try_from(window_length).ok()?;
        let ancestor_id = self.ancestor(p, k)?;
        Some(self.samples_in_block(ancestor_id).to_vec())
    })
    .unwrap_or_default();
```

Trace with window_length = 32 for child C with parent P:
- Start with P.window_aggregate (which already contains samples from
  P's predecessors up to and including P.parent, by inductive
  construction).
- Add P's own samples → aggregate now contains samples from P.parent
  back N-1 ancestors *plus* P itself = window of the last N canonical
  blocks ending at P.
- Evict samples at `ancestor(P, window_length=32)` = the 32nd ancestor
  of P (= 33rd ancestor of C). This is the block whose samples are now
  exactly one too old to be in C's window.

This is correct. C's window holds samples from blocks at distances
1..32 from C (P, P-1, ..., P-31). The eviction logic walks
window_length back from parent, not window_length+1; this is correct
because parent's own samples were just added.

The unit test `ring_evicts_oldest_when_full` in `window.rs:167–182`
exercises exactly this pattern and asserts the right behavior.

Verdict: **RULED OUT.** Window aggregation matches the accumulator's
window semantics on the canonical chain.

### H3: EB-deferred sample timing — RULED OUT

The hypothesis: deviation #5 (`finish_validating_eb`'s deferred EB
sample update on the parent RB's `block_samples` cache) might
incorrectly shift the emission ordering for EB samples relative to the
next RB's controller computation.

Reading `finish_validating_eb` (`linear_leios.rs:1496–1541`):

When a deferred EB is validated *after* its certifying RB was already
published, the code finds the parent RB on the canonical chain and
overwrites `self.block_samples.insert(rb_id, samples)` with the
re-computed samples (now including the EB body). This means descendants
of that RB which had not yet been produced will see the full
RB+EB sample set when they call `samples_in_block(parent_rb_id)`.

But the RB's own `derived_quote` and `window_aggregate` (stored on the
block at production time, lines 908–909) are *not* updated by the
deferred EB validation. Only the cached samples for that RB are
updated.

This means: for any descendant produced *before* the deferred EB
validation, the descendant's `window_aggregate` is computed using the
*RB-only* samples (no EB). After the EB lands, the cache now has
*RB+EB* samples — but the descendant has already been built.

Conclusion: the deferred-EB path *does* introduce a path-dependence:
the chain-derived `derived_quote` depends on whether the EB validates
before or after the descendant block is produced. This is a divergence
from legacy semantics, where `apply_eb_priced_block` would step the
controller at the time of EB validation (mutating the live state, which
then affects future blocks).

However, **the d4_t50_w32 case has zero slot battles AND uses the
single-producer/sundaeswap-realistic topology where the producer also
endorses EBs synchronously at production time.** The deferred-EB path
fires only when an EB is endorsed in an RB but the EB body hasn't yet
been validated locally. In single-producer-with-endorsement scenarios,
producer-side EB endorsement skips the deferred path (line 775:
`if let Some(eb) = self.get_validated_eb(eb_id)`). So H3 cannot
explain this specific zero-SB job.

Verdict: **RULED OUT for the d4 reference case.** Could be a secondary
divergence source on multi-node topologies but doesn't apply here.

### H4: Multiplier-floor application point — RULED OUT

The hypothesis: multiplier-floor might be applied to the wrong quote
(old vs newly-computed) in `compute_derived_quote`.

Reading `two_lane.rs:230–289`:

```rust
let priority_quote_floored = self.apply_floor(standard_quote, priority_quote);

(PerLaneQuote {
    standard: standard_quote,
    priority: priority_quote_floored,
}, new_aggregate)
```

The floor is applied to the **newly-computed** `(standard_quote,
priority_quote)` pair, returned as the block's `derived_quote`. The
function returns the post-floor value. Multiplier-floor invariant is
preserved at the right point.

Crucially, d4_t50_w32 is the **single-lane** sundaeswap-singlelane
suite, which uses `Eip1559Pricing` (not `TwoLanePricing`) and has no
multiplier-floor invariant at all. H4 cannot apply to the d4 reference
case.

Verdict: **RULED OUT** for the d4 reference case (single-lane has no
floor) and **PRESERVED-CORRECTLY** for two-lane suites in general.

### Other hypotheses

- **Block sample cache pruning** (`prune_block_samples`, line 2309).
  The cache is pruned at `2 × window_length` behind chain tip. For
  d4_t50_w32 with window=32, the keep range is the last 64 blocks. The
  producer reads `samples_in_block(ancestor(parent, 32))` which is well
  inside the kept range. Pruning is not affecting controller
  computation.

- **Cold-start initial quote.** For Eip1559 single-lane the
  cold_start_quote returns `initial_quote_per_byte.max(min_fee_a)`
  (line 388–392 of `single_lane.rs`), and the first block has
  `blocks_in_window == 0` which short-circuits to `parent_quote`
  (single_lane.rs:372–374). Cold-start trajectory should match legacy
  for the first ~32 blocks until the window fills. The smoke
  comparison shows trajectories diverging at slot ~100 (after window
  is full), which is consistent with the H1 mechanism kicking in once
  the window has signal but inconsistent with cold-start being the
  source.

## Localised diagnosis

**File:** `sim-rs/sim-core/src/sim/linear_leios.rs`
**Lines:** 2263–2269

```rust
fn current_chain_tip_quote(&self, lane: Lane) -> u64 {
    self.latest_rb_id()
        .and_then(|id| self.praos.blocks.get(&id))
        .and_then(|view| view.received_rb())
        .map(|rb| rb.derived_quote.get(lane))
        .unwrap_or_else(|| self.pricing.cold_start_quote(lane))
}
```

This returns `rb.derived_quote.get(lane)` where `rb` is the latest
canonical RB. By the chain-derived design, `rb.derived_quote` was
computed at the moment `rb` was produced, via
`compute_derived_quote(parent_quote = rb.parent.derived_quote,
parent_aggregate = rb.parent.window_aggregate, parent_samples = rb.parent.samples, …)`.

That is, `rb.derived_quote` is the result of stepping the controller
using `rb.parent`'s samples — **not `rb`'s own samples**. Under the
accumulator semantics that legacy preserved, the quote at this moment
should reflect "the controller after stepping with `rb`'s samples"
(because `apply_priced_block(rb)` ran during `publish_rb(rb)`).

The corrected function should compute what a hypothetical child of `rb`
would have as its `derived_quote`:

```rust
fn current_chain_tip_quote(&self, lane: Lane) -> u64 {
    let tip = self.latest_rb_id();
    let (next_quote, _agg) =
        self.compute_chain_derived_quote_for_child_of(tip);
    next_quote.get(lane)
}
```

This invokes the same pure-function machinery that block production
uses, applied to the chain tip's stored state. The result is equivalent
to "legacy's `pricing.current_quote()` after `apply_priced_block(tip)`."

Equivalent inline implementation that avoids materializing the
WindowAggregate just to throw it away:

```rust
fn current_chain_tip_quote(&self, lane: Lane) -> u64 {
    match self.latest_rb_id() {
        None => self.pricing.cold_start_quote(lane),
        Some(tip_id) => {
            let (next, _) = self.compute_chain_derived_quote_for_child_of(Some(tip_id));
            next.get(lane)
        }
    }
}
```

For consistency, `current_chain_tip_aggregate()` (lines 2273–2279)
should also reflect the *post-step* aggregate (= the aggregate that
the new block's `derived_quote` was computed from). It currently
returns the chain-tip's stored aggregate, which is the pre-step one.
This matters for `PricingTick` event emission and metrics utilization
reporting (lines 2480–2514) but not for simulation-affecting math —
the per-block `derived_quote` is the load-bearing simulation input.

**Why this propagates to the d4_t50_w32 symptom:**

1. At slot N, when an actor submits a tx, `try_add_tx_to_mempool` reads
   the lagged tip quote. The actor's max-fee comparison
   (`quote × bytes + min_fee_b ≤ max_fee_lovelace`) uses this lagged
   value. For climbing demand (saturated chain), the lagged quote is
   *lower* than the true quote, so more txs admit than legacy would
   have allowed.

2. When the producer for slot N+1 generates a block, it charges the RB
   body at `rb.derived_quote` = the *correct* post-step quote (one step
   ahead of admission). Some admitted txs now fail the
   `actual_fee > max_fee_lovelace` check at line 2134 and get evicted
   via `TXEvictedQuoteDrift`. The remaining included txs pay a higher
   fee than the actor expected → lower refund → lower net utility.

3. Under d4, each step can move the quote by ±25%. The compounding
   effect of "admit at q_{N-1, post step with N-2's samples}, charge
   at q_{N, post step with N-1's samples}" over 2000 slots is a
   systematic mismatch that produces:
   - Different sample patterns (different sets of txs get included)
   - Different feedback into the controller's signal (because
     `samples_for_block` depends on which txs got into the RB body,
     which depends on which txs admitted)
   - Compounding divergence of c_priority over the run

4. Welfare flips sign because the chain-derived run admits more
   borderline txs (at lower lagged quote), then charges them at a
   higher actual quote where many lose their refund or even get
   evicted. The net utility = Σ refunds − Σ value-of-lost-txs flips
   negative when evictions dominate.

**Why baseline_flat_fee matches:** `BaselinePricing::compute_derived_quote`
returns `PerLaneQuote::flat(min_fee_a)` regardless of inputs
(`single_lane.rs:60–69`). Every block's `derived_quote` is exactly
`min_fee_a`, so the "one step behind" lag is invisible — the lagged
and post-step quotes are identical (44 = 44).

## Predicted-vs-observed cross-check

If the bug is "1-step lag in admission/EB-charge quote", predictions
across the 33 jobs:

1. **High-D (D=4) suites should diverge more than low-D (D=8 / D=16).**
   D=4 → 25%/step; D=8 → 12.5%/step; D=16 → 6.25%/step.

   Observed (singlelane): d4_t50_w32 final c_priority drift −90 %;
   d8 (multiple) range −16 % to +91 183 %; d16_t50_w32 drift −78 %.

   Partially consistent — d4 has dramatic divergence and so does d8.
   But d16 (less reactive) also shows a large drift. The mechanism
   predicts a smaller per-step gap at D=16, but the
   cumulative effect over 2000 ticks can still be substantial if the
   chain is consistently saturated or consistently under target. (For
   reactive sundaeswap demand the actual D-dependence is weakened by
   actor lane choice and saturation effects.) Direction is consistent;
   magnitude tracking requires per-trajectory analysis.

2. **Larger window (w=64) should accumulate the lag effect more
   slowly per step but over longer horizons.** Observed
   eip1559_d8_t50_w64: c_priority 155 → 141 489. This is a w=64 with
   very-low-quote accumulator endpoint (155, suggesting the
   accumulator hit the era floor) vs chain-derived running hot. The
   directional flip (running hot vs hitting floor) is consistent with
   the lag flipping the controller from "consistently under target"
   to "consistently over target" once the asymmetry takes hold.

3. **Two-tier mechanisms with priority lane should show worse
   divergence on priority controllers** because both lane choice and
   priority-only EB charging are affected. Observed: priority-only
   `rb_reserved_x4_rb_quarter` welfare drops 89.9%; partitioned
   `partitioned_x4_rb_quarter` also drops 89.9%. Consistent.

4. **`baseline_flat_fee` must match byte-for-byte.** Observed: matches.
   Strongest single confirmation.

5. **`unreserved_x16` (both-dynamic) sees c_priority drop from
   294 272 → 15 936 while c_standard drops from 18 392 → 996.**
   Both controllers feel the lag; in both-dynamic, the lag in standard
   admission can propagate to priority via the multiplier floor. The
   relative ratios changing dramatically (factor 16 vs new ~16, but
   absolute floor much smaller) is consistent with both controllers
   trapped at lower regions due to admission undercharging during
   high-demand periods, leading to more "underwater" actor choices.

6. **`eip1559_d8_t75_w32` c_priority went 5382 → 0.** Hitting the era
   floor (c=1, quote = min_fee_a = 44) on chain-derived but not
   accumulator. With t=0.75 target (chain wants 75% utilization), the
   chain-derived's admission lag undercounts the saturation signal,
   leading the controller to drift down toward the floor when it
   "shouldn't have." Consistent with the lag systematically
   under-reporting saturation.

Overall: 5/6 predictions consistent with H1's mechanism; 1 partially
consistent (the D-dependence is direction-correct but magnitude
varies). The smoke comparison's "completely uncorrelated trajectories
by mid-run" framing is consistent with a 1-step admission lag
compounded over 2000 ticks at a reactive D=4 setting.

## Recommended surgical fix (for the user to apply)

**Do NOT apply patches per the investigation constraints.**

The minimal patch is to `current_chain_tip_quote` in
`sim-rs/sim-core/src/sim/linear_leios.rs:2263-2269`.

Before:
```rust
fn current_chain_tip_quote(&self, lane: Lane) -> u64 {
    self.latest_rb_id()
        .and_then(|id| self.praos.blocks.get(&id))
        .and_then(|view| view.received_rb())
        .map(|rb| rb.derived_quote.get(lane))
        .unwrap_or_else(|| self.pricing.cold_start_quote(lane))
}
```

After:
```rust
fn current_chain_tip_quote(&self, lane: Lane) -> u64 {
    // Return the quote that a hypothetical child of the current
    // chain tip would have as its `derived_quote`. This matches
    // legacy `pricing.current_quote()` semantics, which reflected the
    // controller state *after* the chain tip's apply_priced_block
    // had folded in the tip's own samples. The previously-stored
    // `rb.derived_quote` is the chain tip's *own* derived quote
    // (stepped using its parent's samples), which is one controller
    // step earlier than the legacy `current_quote` reading.
    let tip = self.latest_rb_id();
    if tip.is_none() {
        return self.pricing.cold_start_quote(lane);
    }
    let (next, _agg) = self.compute_chain_derived_quote_for_child_of(tip);
    next.get(lane)
}
```

For symmetry, `current_chain_tip_aggregate` (lines 2273–2279) should
also use the post-step aggregate. The metrics impact is cosmetic
(window utilization reporting will reflect "what the next block sees"
instead of "what the chain tip computed against"), but the consistency
helps downstream readers.

Performance note: `compute_chain_derived_quote_for_child_of` is O(1)
given the cached `samples_in_block` and `derived_quote`/`window_aggregate`
on blocks. Calling it once per `current_chain_tip_quote` invocation
is cheap.

The fix should leave RB body inclusion charging (line 934) **unchanged**:
`rb.derived_quote` is already the correct post-step value. After this
fix, admission, lane choice, EB endorsement validation, EB inclusion
charging, and RB body inclusion charging will all consume the same
controller state — restoring the accumulator's "one quote per slot
production" invariant on the canonical chain.

After applying the fix:
1. Re-run the sundaeswap-singlelane smoke and confirm
   `eip1559_d4_t50_w32` produces near-identical `c_priority`
   trajectory and welfare versus accumulator on the zero-SB job. Some
   sub-percent divergence is expected on positive-SB jobs (the WR-1
   contamination removal is real).
2. Re-run all 33 jobs and confirm `c_priority` deltas drop to the
   sub-percent range on zero-SB jobs and that positive-SB jobs show
   small directional deltas consistent with WR-1 contamination
   removal (not 200–25 000 %).

## Recommended regression-prevention test

Add to `sim-rs/sim-core/src/sim/tests/m_chain_derived.rs` (or
equivalent — the test file location already exists per spike 007 plan):

```rust
#[test]
fn chain_derived_matches_accumulator_on_zero_slot_battle_run() {
    // Set up a single-producer linear-Leios scenario where:
    //   - topology = single producer (no slot battles possible)
    //   - pricing = eip1559 (d=4, t=1/2, w=32) — the most-reactive
    //     setting from sundaeswap-singlelane
    //   - 200+ slots
    //
    // Assert that the canonical chain's `derived_quote` sequence
    // exactly equals an oracle computation:
    //   - q[0] = cold_start_quote
    //   - q[n+1] = compute_eip1559_step(q[n], aggregate_for_block_n)
    //
    // ... and that the consumer-visible quote (admission /
    // inclusion-charge) matches q[n+1] when block n+1 is being
    // produced, not q[n].
    //
    // This test would fail under the current (lagged) implementation
    // and pass after the `current_chain_tip_quote` fix.
}
```

A property test asserting "for any zero-slot-battle scenario,
chain-derived pricing event stream hash equals accumulator's pricing
event stream hash" is the strongest possible regression guard, but it
requires retaining the accumulator implementation behind a feature
flag. As a lighter alternative, capture a single pinned golden hash
for one zero-SB scenario under the fixed chain-derived implementation
and add a sibling assertion that the same scenario re-run any number
of times (different RNG paths, different mempool sampling) but with
the same input topology, demand, and pricing all produce the same
canonical-chain `derived_quote` sequence.

## Confidence in diagnosis

**HIGH.** Justification:

1. The legacy accumulator's `apply_priced_block` runs inside
   `publish_rb` (legacy line 1044), so the controller is in
   post-step-with-this-block's-samples state immediately after a
   block's `publish_rb` returns. This is the state that legacy
   `current_quote()` reads for the next block's admission, lane
   choice, EB endorsement, and inclusion charging.

2. Chain-derived `block.derived_quote` is computed via
   `compute_derived_quote(parent_quote, parent_aggregate, parent_samples, …)`
   — i.e., it steps using the *parent's* samples, *not* the block's
   own. So `block.derived_quote == legacy current_quote at production
   of block` (consistent across implementations on the new RB body
   charge). It is **not** `legacy current_quote at production of
   block's child` (which would be one step further along).

3. `current_chain_tip_quote()` returns `tip.derived_quote` directly.
   This equals "legacy current_quote at production of tip", which is
   **one step behind** "legacy current_quote at production of tip's
   child". The latter is what admission/lane-choice/EB-charge need.

4. The only consumer that reads the *correct* post-step value is the
   RB-body inclusion charge at line 934, which directly reads
   `rb.derived_quote` (the new RB's own field). All other consumers
   read `current_chain_tip_quote` and get the lagged value.

5. The d4_t50_w32 outlier is the lowest-D (most reactive) single-lane
   suite, and zero slot battles eliminate WR-1 contamination as a
   confounding factor. The observed mid-run trajectory divergence
   (slot ~100 onward, ratio 0.63 → 0.16 → 19.7 → 0.10) is consistent
   with a 1-step lag compounding through a high-reactivity controller.

What would shake confidence:
- Discovery of a *second* off-by-one elsewhere that produces the
  observed magnitude on its own (e.g., a window-aggregate edge case I
  missed).
- A direct trace showing chain-derived block N's `derived_quote` not
  matching the legacy `current_quote` post-publish-rb(block N) (would
  indicate the math in `compute_eip1559_step` differs even with
  matching inputs).

The math in `compute_eip1559_step` (single_lane.rs:199–287) is
character-for-character identical to legacy `Eip1559Pricing::step`
(verified by `git show 2f8ac5e:sim-rs/sim-core/src/tx_pricing/single_lane.rs`
lines 200–303), so the math-divergence concern is ruled out. The only
remaining variable is the input source — which is exactly H1's
mechanism.
