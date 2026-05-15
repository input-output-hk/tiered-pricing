# Spike 007 — Chain-derived controller (WR-1 fix via design redesign)
Date: 2026-05-14
Verdict: **ADOPT — chain-derived (Option D), supersedes accumulator design.**
The redesign eliminates WR-1 (orphan-block contamination of the pricing
controller) by construction, aligns phase-2 with the only deployed
dynamic-fee mechanism (EIP-1559), and reduces the implementation
surface compared to the alternative (snapshot/restore of mutable
backend state on every fork resolution).

## Spike Question

**Given** the phase-2 simulator currently implements its pricing
controller as a node-local accumulator (mutable
`Eip1559Pricing.quote_per_byte`, mutable `CapacityWeightedWindow`
rolling buffer) — which creates WR-1: orphan blocks from slot battles
contaminate the controller state, empirically observed at ~1.2
contaminating battles per (job, seed) under the realistic stake
topology — **and given** Ethereum's EIP-1559 solves the same class of
problem by making `base_fee` a pure function of the parent block,

**When** we adopt the chain-derived pattern: store the controller's
`quote_per_byte` (and aggregate window state) **on each block**,
computed as a pure function of the parent block + samples in the
parent (or last N canonical blocks),

**Then** WR-1 disappears by construction (no node-local state to roll
back; orphan blocks carry their own `derived_quote` that is never
consulted by canonical-chain consumers) and the simulator matches
what a real Cardano deployment would adopt.

## The problem: WR-1 in one paragraph

Today's `Eip1559Pricing.quote_per_byte: u64` and the underlying
`CapacityWeightedWindow` ring buffer are **node-local mutable
accumulators**: `apply_priced_block` mutates them in place whenever a
node validates an RB or its certified EB
(`sim-core/src/sim/linear_leios.rs:2060` and `:2084`). When two
sibling RBs win the slot lottery for the same slot and both bodies are
fully validated at a representative node before the VRF tiebreak
resolves, the losing block's mutations remain stuck in the controller
and the gate (`finish_validating_rb_header` at
`sim-core/src/sim/linear_leios.rs:1098` removes the losing RB from
`praos.blocks` but does not roll back the controller, the gate, or
the `TXIncluded` events). The
[2026-05-14 smoke comparison](../../smoke-comparison-2026-05-14.md)
quantified this empirically: **41 slot battles across 33 (job,
seed=1) pairs** on `topology-realistic-100.yaml` — ~1.2 battles per
job, with 29/33 jobs showing at least one. This is real, persistent
controller contamination, no longer a dormant assumption; the
[REVIEW.md WR-1 row](../../REVIEW.md#wr-1) is currently classified
**LIVE / disclosure-required**.

## Why mainnet Cardano doesn't have this problem

Cardano's mainnet fee formula is static (`fee = minFeeB + minFeeA ×
bytes` with both constants protocol invariants). It has **no
controller state at all** — orphan blocks lose the txs they carried
but there is nothing to roll back: the per-byte rate is a global
constant. So mainnet offers no precedent that phase-2 can lift; the
novel question phase-2's dynamic-pricing pivot raises is

> *how do you do dynamic fees in a system that has slot battles and
> short forks?*

Phase-2's answer to that question is a deliberate choice. The
**accumulator option** (current design) defers the choice to "M6
implements rollback or accepts the documented WR-1 contamination". The
**chain-derived option** answers it the way Ethereum did when it
shipped EIP-1559: by making the controller stateless at the node
level.

## How EIP-1559 solves this

Ethereum's EIP-1559 specification (eips.ethereum.org/EIPS/eip-1559,
"Specification" section: `base_fee_per_gas` calculation) defines the
next block's base fee as a **pure function of the parent block**:

```
parent_gas_used > parent_gas_target →
    base_fee_delta = max(1,
        parent.base_fee_per_gas * (parent.gas_used - parent.gas_target)
        / parent.gas_target / BASE_FEE_MAX_CHANGE_DENOMINATOR)
    new.base_fee_per_gas = parent.base_fee_per_gas + base_fee_delta

parent_gas_used < parent_gas_target →  (symmetric, subtract)

parent_gas_used == parent_gas_target →
    new.base_fee_per_gas = parent.base_fee_per_gas
```

Three properties make this reorg-safe:

1. **Every block carries its own `base_fee_per_gas` field.** This is
   a header field, validated by every node, replicated in the block.
   It is not derived from a per-node accumulator; it is a function of
   the immediate parent.
2. **The function is pure.** Given a parent block, every honest node
   computes the same `base_fee_per_gas` for any child. There is no
   "history" beyond the parent — Ethereum reads exactly the
   `gas_used`, `gas_target`, and `base_fee_per_gas` from the parent
   header.
3. **The canonical chain's base-fee sequence is deterministically
   derivable.** A reorg replaces an entire subchain. The new
   canonical chain has its own `base_fee_per_gas` sequence (one per
   block), trivially correct because each entry is the pure function
   of its predecessor. **Orphaned blocks' base fees are simply never
   consulted** by anyone — they're discarded along with the orphaned
   block. There is no rollback step; there is nothing to roll back.

The academic literature treats this property explicitly. Roughgarden
(*"Transaction Fee Mechanism Design for the Ethereum Blockchain: An
Economic Analysis of EIP-1559"*, 2020/2021 — cited in
[spike 003](../003-pricing-controller-calibration/README.md)) refers
to EIP-1559 as a **stateless mechanism**: the fee-determination logic
depends only on the parent block, not on a persistent controller
state held by nodes. This is precisely the design property phase-2
loses today and would regain under chain-derivation.

Reijsbergen et al.'s *"Honeymoon"* paper (arXiv:2110.04753, 2021) and
follow-on dynamics analysis confirm empirically that EIP-1559's
short-fork behaviour matches the design intent: short reorgs on
Ethereum mainnet do not produce systemic fee misestimation because
the new canonical chain re-derives its base-fee sequence from its own
parents.

## Phase-2's current design and why it has WR-1

`Eip1559Pricing.quote_per_byte: u64` is mutable node-local state
(`sim-core/src/tx_pricing/single_lane.rs:155`). The
`CapacityWeightedWindow` (`sim-core/src/tx_pricing/window.rs:20-25`)
is a mutable `VecDeque` plus running `sum_bytes`/`sum_capacity`
counters, mutated via `push` (`window.rs:57`). When a block is
validated, `apply_priced_block`/`apply_eb_priced_block`
(`sim-core/src/sim/linear_leios.rs:2060-2094`) calls
`samples_for_block` → `update_after_block` → `window.push` and an
EIP-1559 `step` (`single_lane.rs:215-299`) — all of which mutate
state on `self.pricing` and `self.gate` (the mempool gate is
revalidated, with evictions emitted to the event stream).

The offending mutation points:
- `sim-core/src/tx_pricing/single_lane.rs:215-299`
  (`Eip1559Pricing::step`): writes `self.quote_per_byte` and
  `self.window`.
- `sim-core/src/tx_pricing/window.rs:57-71`
  (`CapacityWeightedWindow::push`): writes `self.samples`,
  `self.sum_bytes`, `self.sum_capacity`.
- `sim-core/src/tx_pricing/two_lane.rs:262-292`
  (`TwoLanePricing::update_after_block`): drives both controllers'
  mutable state and then `enforce_multiplier_floor` mutates
  `self.priority.quote_per_byte`.

When `finish_validating_rb_header` at
`sim-core/src/sim/linear_leios.rs:1098-1184` resolves a slot battle
by removing the higher-VRF block, none of these mutations are undone.
Empirically (smoke comparison, 2026-05-14) the contamination is small
in magnitude (~10⁻³ of pricing samples) but non-zero, and is what
prevents the phase-2 evidence from being clean.

## The chain-derived redesign

### Core idea

`derived_quote` becomes a **field on every block** (one per lane for
two-lane variants). Its value is a pure function of:
- the parent block's `derived_quote`, and
- the samples carried by the last `N` canonical blocks (the
  capacity-weighted window aggregate).

The `PricingBackend` is no longer stateful at the node level. It is
a **policy object** that computes
`compute_quote(parent_block, samples_in_window, lane) -> u64` and
returns the result. The "controller state at chain tip T" is by
definition `T.derived_quote(lane)`, computed once at block
production and stored on the block. Consumers
(`MempoolGate::try_admit`, `lane_choice::pick`,
`StalenessPredictor`) read the chain tip's `derived_quote(lane)`
instead of consulting a node-local mutable backend.

### Type-level shape

The concrete Rust-level changes:

```rust
// model.rs additions
pub struct PerLaneQuote {
    pub standard: u64,
    pub priority: u64,
}

pub struct LinearRankingBlock {
    // existing fields...
    /// Derived per-lane quote computed at block production from
    /// parent + window of canonical predecessors. Carried on the
    /// block (header is the natural location; on-disk impact
    /// minimal: 16 bytes).
    pub derived_quote: PerLaneQuote,
    /// Snapshot of the controller window aggregate used to compute
    /// `derived_quote`, for memoization and audit.
    pub window_aggregate: WindowAggregate,
}

pub struct LinearEndorserBlock {
    // existing fields...
    /// EBs inherit their derived_quote from the RB they reference
    /// (or the chain tip at EB-construction time). EBs do not
    /// themselves drive a separate derivation; they contribute
    /// samples to the parent RB's successor's window.
    pub derived_quote: PerLaneQuote,
}

pub struct WindowAggregate {
    pub standard_sum_bytes: u128,
    pub standard_sum_capacity: u128,
    pub priority_sum_bytes: u128,
    pub priority_sum_capacity: u128,
    /// Number of canonical blocks contributing to this aggregate
    /// (≤ window_length). Tracks warm-up state.
    pub blocks_in_window: u32,
}
```

The `PricingBackend` trait shifts to a pure-function shape:

```rust
pub trait PricingBackend: Send + Sync {
    /// Compute the per-lane derived quote for a child of `parent`.
    /// Returns the (quote, aggregate) pair: the quote goes onto the
    /// block; the aggregate is stored alongside so descendants can
    /// recompute incrementally.
    fn compute_derived_quote(
        &self,
        parent: &ParentSnapshot,
        samples_in_window: &[CanonicalBlockSamples],
    ) -> (PerLaneQuote, WindowAggregate);

    /// Read the chain tip's quote (lookup, not derivation).
    fn quote_at(&self, block: &BlockSnapshot, lane: Lane) -> u64 {
        block.derived_quote.get(lane)
    }

    /// Validity rule for blocks of the given kind.
    fn lane_validity_rule(&self, block_kind: BlockKind) -> LaneValidityRule;

    /// Lane-selection order (PriorityFirst or Fifo).
    fn lane_selection_order(&self) -> LaneSelectionOrder;

    /// Multiplier-floor (two-lane only).
    fn min_priority_premium_multiplier(&self) -> Option<Multiplier>;

    /// Block-kind-aware sample emission (unchanged signature).
    fn samples_for_block(
        &self,
        block_kind: BlockKind,
        breakdown: &BlockLaneBreakdown,
    ) -> Vec<PricedBlockSample>;
}
```

Key shape differences from the current trait:
- **No `&mut self` anywhere.** The backend is a configuration object,
  not state.
- **No `current_quote(lane)`.** Replaced by `quote_at(block, lane)` —
  callers must specify *which block*.
- **No `update_after_block`.** Replaced by `compute_derived_quote`,
  which is a pure function applied at block production.
- **No `snapshot()`.** The PricingSnapshot for time-series logging is
  derived from whatever block the metrics collector observes (chain
  tip at the relevant slot).

`CapacityWeightedWindow` becomes a per-call helper rather than a
persistent ring buffer:

```rust
impl CapacityWeightedWindow {
    /// Pure aggregator: walks N canonical predecessor blocks back
    /// from `parent`, sums samples by lane, returns the WindowAggregate.
    pub fn aggregate_from_chain(
        chain_walker: impl Iterator<Item = &'_ CanonicalBlockSamples>,
        length: usize,
    ) -> WindowAggregate;
}
```

The aggregator can be incrementally computed: given parent's
`WindowAggregate` (cached on parent), adding the parent's own samples
and evicting the oldest sample at distance N produces the child's
aggregate in O(1) per block. The `CanonicalBlockSamples` records
needed for eviction must be retained for `length` blocks past the
current tip — bounded memory.

### Block production

When a producer assembles a new block at chain tip T:

1. **Look up the parent block.** Today this is implicit (the node's
   chain head); under chain-derivation it becomes a structural input.
2. **Construct the window-aggregate input.**
   - Take `parent.window_aggregate`.
   - Add the samples that `parent` produced (RB body samples and any
     EB samples for the EB endorsed in parent), via
     `backend.samples_for_block(...)`.
   - Evict any samples from the block at distance `N+1` from the
     producing block (looked up via the local block store).
   - Result: the child's `WindowAggregate`.
3. **Apply the EIP-1559 step rule against the parent's
   `derived_quote`.** This is the same integer-rational math used
   today in `Eip1559Pricing::step` (the math itself is reusable; the
   only change is its inputs are explicit rather than read off
   `self`).
4. **Enforce the multiplier-floor invariant** on the resulting
   `PerLaneQuote` (two-lane variants).
5. **Attach** `derived_quote` and `window_aggregate` to the new
   block.

The producer then proceeds with normal block-build: scan the mempool
at `derived_quote` for fee admissibility, pack txs by lane, etc.
Crucially, **the values used for tx admissibility are the new block's
own `derived_quote`** — the controller's "future" is fixed at the
moment of production.

### Block consumption (gate, actor)

`MempoolGate::try_admit` changes signature from
`try_admit(tx, quote_per_byte)` to
`try_admit(tx, parent_block_id) -> ...`. The gate looks up the
parent block's `derived_quote(lane)` and uses it for fee
admissibility. No mutation of any pricing-side state occurs —
`try_admit` does not feed back into a controller.

```rust
impl MempoolGate {
    pub fn try_admit(
        &mut self,
        tx: &Transaction,
        chain_tip: &BlockSnapshot,
    ) -> Result<(), AdmissionRejection> {
        let quote = chain_tip.derived_quote.get(tx.posted_lane);
        // ... existing fee-admissibility logic against `quote` ...
    }
}
```

Revalidation works the same way: when the chain tip advances (a new
canonical block lands), the gate walks its resident set, looks up
the new tip's `derived_quote(lane)`, and evicts txs whose `quote ×
bytes + minFeeB > max_fee_lovelace`. Inclusion charging
(`on_inclusion`) reads the served-lane `derived_quote` from the block
the tx was included in (which is the block currently being produced,
i.e. the block whose `derived_quote` was just computed in the
production step above).

Actor lane-choice (`lane_choice::pick`, `tx_actors.rs:339`) already
takes the quotes as parameters; it just receives them from the
chain-tip lookup rather than from `backend.current_quote(lane)`.

### Slot-battle resolution under chain-derived

Let B₁ and B₂ be sibling RBs at slot S, both produced from parent A.
Under chain-derivation:

- Both B₁ and B₂ are produced with `derived_quote` computed as
  `f(A.derived_quote, A.window_aggregate ⊕ A.samples)`. **The
  inputs are identical** for both siblings — they share the parent
  A and therefore share A's samples and A's window. Hence
  **B₁.derived_quote == B₂.derived_quote** by pure-function
  reasoning.
- The VRF tiebreak picks one canonical winner (say B₁). The other
  (B₂) is discarded.
- A node that fully validated B₂ before learning about B₁ has not
  mutated any controller state. The block B₂ carries its own
  `derived_quote` in its block body; nothing else was touched.
  When B₂ is dropped, the `derived_quote` value is discarded along
  with it. The canonical chain past slot S consults only
  B₁.derived_quote.
- The mempool gate, which was queried with `chain_tip = B₂` for any
  admissions that occurred during B₂'s brief canonical-claim
  period, is now re-queried against `chain_tip = B₁`. Since
  B₁.derived_quote == B₂.derived_quote (same parent, same window),
  no tx that was admitted under B₂ is now invalid under B₁ — they
  are the same quote. Inclusion records on `TXIncluded` events
  emitted by the producer at slot S+1 reference B₁'s quote
  (canonical) by construction.

**WR-1 is impossible by construction.** Sibling blocks produce
identical controller state. There is no contamination to detect, no
metric to bound, no disclosure to issue. The canonical chain's
pricing trajectory is purely a function of its own block sequence.

The narrow remaining concern — what if B₁ has different samples than
B₂ because the producers picked different mempool subsets? — has no
effect on the *child's* `derived_quote`, only on the **child of S+1
producing on top of S**: that child's window will see the canonical
winner B₁'s samples, not B₂'s. Which is exactly correct behaviour:
the canonical chain defines the canonical samples, full stop.

### Edge cases to address in implementation

1. **Memoization cache pruning.** The local per-`BlockId` cache of
   `derived_quote` and `window_aggregate` (used to skip recomputation
   when a parent is revisited) can grow unboundedly without pruning.
   The natural cap is the chain-stability window: anything more than
   `2 × window_length` (default 64 blocks) behind the chain tip can
   be evicted; under Cardano's k=2160 finality, **chain-stability >>
   pricing window** so the cache trivially fits in bounded memory.
2. **Deep reorgs (past 32 blocks).** If a reorg goes back further
   than the 32-block window, the chain-derived computation is still
   well-defined (the new canonical chain has its own derived_quote
   on every block), but the cache invalidates. Practically, k=2160
   means deep reorgs are rare and bounded; this is engineering noise
   compared to today's correctness gap.
3. **First 32 blocks (cold-start).** The window has fewer than 32
   samples available. The mathematically correct behaviour is to
   aggregate over what exists (the `blocks_in_window` field in
   `WindowAggregate` tracks this). The first block has no parent and
   uses the genesis-configured `initial_quote_per_byte` (matching
   the current constructor behaviour). Document the warm-up
   explicitly in the spec; tests cover.
4. **RB vs EB derivation.** Praos's chain-tip identifier is the RB,
   not the EB. Recommendation: **`derived_quote` attaches to RBs;
   EBs inherit the quote from their parent RB.** EB samples feed
   the *next* RB's window (i.e. the RB endorsing the EB or the
   subsequent RB if the EB is endorsed in a later RB). This
   preserves linear-Leios's RB-is-tip invariant; the EB
   `derived_quote` field on `LinearEndorserBlock` is convenient for
   actors/gate using the EB as a service-time signal but is
   structurally redundant (derivable from EB.parent_rb).
   Implementation may choose to omit the EB field and resolve via
   the parent RB lookup; flag for the planning stage.
5. **EB endorsement window staleness check.** The current
   `StalenessPredictor` (used in `endorsement_window_priced_blocks`,
   line 407 of `linear_leios.rs`) projects `worst_case_quote_at(lane,
   blocks_ahead)`. Under chain-derived, this is no longer a
   projection of mutable state — it walks forward from the current
   chain tip, computing N successive `derived_quote` values under
   worst-case sample assumptions. Same math, different call shape.
6. **Migration: golden hash impact.** Every M2/M3 unit-test golden
   in `sim-core/src/sim/tests/` and every M5 suite golden under
   `parameters/phase-2-sweep/suites/.goldens/` will flip. **This is
   expected and tractable**: the math is the same in steady state
   (the integer-rational EIP-1559 step rule is reused verbatim), so
   the canonical chain's controller trajectory should be *very*
   close to today's — the differences are exactly the orphan-block
   removals plus any cross-pollination effects we are explicitly
   eliminating. Regenerate with `UPDATE_GOLDENS=1` per the existing
   workflow in CLAUDE.md.
7. **Determinism contract preservation.** The chain-derived
   computation must remain integer/rational/u128 throughout.
   `compute_derived_quote` is pure but must still avoid f64. The
   existing `Eip1559Pricing::step` math is directly reusable.
8. **MempoolGate revalidation timing.** Today the gate revalidates
   on `update_after_block`. Under chain-derived, revalidation
   triggers when the chain tip advances to a new block whose
   `derived_quote` differs from the previous tip's. The hook moves
   from "after the backend updates" to "after the local chain tip
   changes" — natural place is `publish_rb` (currently calls
   `apply_priced_block`).
9. **Two-lane multiplier-floor invariant.** Today
   `enforce_multiplier_floor` is called after both controllers'
   updates and mutates `priority.quote_per_byte`. Under
   chain-derived, the invariant is applied **inside**
   `compute_derived_quote`: the function returns the post-floor
   `PerLaneQuote`. No persistent state to enforce on after
   construction.

### What stays the same

- **The EIP-1559 step rule.** Clamp formula, era floor, u128
  rationals — unchanged. The math now takes parent.derived_quote and
  window_aggregate as inputs rather than reading them off
  `self.window` and `self.quote_per_byte`, but the formula is
  identical.
- **The multiplier-floor invariant** for two-lane: same rule
  (`q_priority ≥ ceil(num × q_standard / den)`), same enforcement on
  `quote_per_byte`, same u128 intermediates. Just applied inside
  `compute_derived_quote` rather than on persistent state.
- **The capacity-weighted window aggregate logic** — same
  aggregation (Σ relevant_bytes / Σ relevant_capacity), same
  per-variant `samples_for_block` rules, same RB-reserved-priority
  length=1 reduction.
- **The actor model**, lane-choice math, max-fee policy
  (`ScaledOverLaneQuote`) — all unchanged; they just receive the
  chain-tip quote rather than the backend's mutable quote.
- **The mempool gate semantics** — admission, revalidation,
  inclusion charging — unchanged in shape. Same per-lane bytes
  tracking, same `AdmissionRejection` enum, same `InclusionCharge`
  record. The only signature change is `try_admit` taking
  `&BlockSnapshot` for quote lookup rather than a `u64` quote
  parameter.
- **The producer's lane-selection order, partition activation,
  EB-validation-at-endorsement** — all unchanged. They consult the
  chain tip's derived quote and execute the same logic.
- **Event stream shape.** `TXIncluded`, `TXEvictedQuoteDrift`,
  `PricingTick` — same fields, same emission sites. The quote
  values they carry come from the chain-derived computation rather
  than from `backend.current_quote(lane)`.
- **Calibration choices.** Window length 32, RB-reserved priority
  length 1, multiplier_floor defaults, update cadence per priced
  block, signal sources — every choice from CLAUDE.md "Calibration
  choices" survives unchanged.

## Comparison table

| Property | Current (accumulator) | Chain-derived (EIP-1559 pattern) |
|---|---|---|
| State location | Node-local mutable `Eip1559Pricing.quote_per_byte` + `CapacityWeightedWindow` ring | Derived from canonical chain; stored on each block as `derived_quote` |
| WR-1 contamination | **LIVE** (~1.2 battles/job empirically, 29/33 jobs affected on realistic topology) | **Impossible by construction** |
| Per-block cost | One mutation (window push + EIP-1559 step) | One walk-back + memoized cache lookup; incremental aggregate in O(1) once cached |
| Rollback complexity | Requires snapshot+restore on every `publish_rb` OR defer-to-finality scheme | None — no persistent state to roll back |
| Reorg behavior | Needs explicit handling; deep reorgs cascade through cached samples | Automatic — canonical chain defines state |
| Match to deployed precedent (EIP-1559) | **No** — accumulator is a design choice unique to phase-2 | **Yes** — directly matches Ethereum's deployed pattern |
| Code complexity | Mutable state, in-place updates, defensive ordering | Pure functions, explicit dependencies, memoized lookups |
| Block-storage overhead | 0 bytes/block | 16 bytes/block for `PerLaneQuote` + ~40 bytes for `WindowAggregate` |
| Cache memory | Window contents bounded at `window_length` samples | Memoization cache bounded at `≤ 2 × window_length` blocks past chain tip |
| Goldens impact | Status quo | All M2/M3 unit-test goldens flip; all M5 suite goldens flip (one regenerate cycle) |
| Publication framing | *"We implemented rollback for slot battles"* / *"contamination bounded at 10⁻³"* | *"We adopted EIP-1559's chain-derived pattern; orphan blocks cannot contaminate the controller"* |
| Implementation cost | M6 deliverable: snapshot/restore mempool + pricing state on every `publish_rb` (substantial; mutates hot path) | Refactor: net +200–400 LoC across tx_pricing/, model.rs, linear_leios.rs; eliminates `update_after_block` / `current_quote` API |

## Implementation roadmap (high-level — full plan in the next pipeline stage)

This roadmap names files and points at breakpoints; the
gsd-plan-phase invocation that follows will produce file-by-file
deltas.

1. **Extend block types.**
   - `sim-core/src/model.rs`: add `PerLaneQuote` and
     `WindowAggregate` types. Add `derived_quote: PerLaneQuote` and
     `window_aggregate: WindowAggregate` fields to
     `LinearRankingBlock` (and consider whether to add a derived
     `derived_quote` to `LinearEndorserBlock` or compute lazily from
     parent RB lookup).

2. **Refactor `PricingBackend` trait.**
   - `sim-core/src/tx_pricing/mod.rs`: replace `current_quote`,
     `update_after_block`, `worst_case_quote_at`, `snapshot` with
     `compute_derived_quote(parent, samples_in_window) ->
     (PerLaneQuote, WindowAggregate)`. Keep `lane_validity_rule`,
     `lane_selection_order`, `min_priority_premium_multiplier`, and
     `samples_for_block` unchanged (they are already pure-policy).

3. **Refactor `BaselinePricing`, `Eip1559Pricing`, `TwoLanePricing`.**
   - `sim-core/src/tx_pricing/single_lane.rs`: rip out
     `self.quote_per_byte` and `self.window`. The EIP-1559 `step`
     function becomes a pure helper `compute_step(parent_quote,
     aggregate, settings) -> u64`. `BaselinePricing` returns
     `min_fee_a` unconditionally (trivial change).
   - `sim-core/src/tx_pricing/two_lane.rs`: rip out the two
     persistent `Eip1559Pricing` instances; compute both lanes inline
     in `compute_derived_quote` and apply the multiplier-floor
     invariant before returning.
   - `sim-core/src/tx_pricing/window.rs`: `CapacityWeightedWindow`
     becomes a pure aggregator over a slice/iterator of
     `CanonicalBlockSamples`. The persistent ring is gone; replaced
     by a `WindowAggregate` struct that can be incrementally updated.

4. **Refactor `MempoolGate`.**
   - `sim-core/src/sim/mempool_gate.rs`: `try_admit` takes
     `&BlockSnapshot` for quote lookup; `revalidate` takes the new
     chain tip. The `quote_for_lane` callback parameter stays as the
     decoupling seam (caller passes a closure that reads from the
     chain tip).

5. **Refactor `linear_leios.rs` block production.**
   - `sim-core/src/sim/linear_leios.rs`: at `publish_rb` (line 988)
     and EB construction sites, compute `derived_quote` from parent
     + window. Remove `apply_priced_block` (line 2060) and
     `apply_eb_priced_block` (line 2084) — their work moves into the
     block-production code path. The `feed_samples_and_revalidate`
     helper (line 2096) becomes `revalidate_against_new_tip`.
   - Replace all `self.pricing.current_quote(lane)` call sites
     (lines 900-901, 1768, 1987-1988, 2102-2103, 2308-2311) with
     chain-tip lookups (the chain tip's `derived_quote` field).
   - `LinearLeiosNode::new` (line 447): the backend is now a
     configuration object, not a state container.
   - Event emission unchanged in shape: `TXIncluded`,
     `TXEvictedQuoteDrift`, `PricingTick` all still fire from the
     same places, just reading quote values from the chain tip.

6. **Update tests.**
   - `sim-core/src/sim/tests/m1_smoke.rs`,
     `sim-core/src/sim/tests/m2_two_lane.rs`,
     `sim-core/src/sim/tests/m3_actors.rs`: golden hashes will flip
     in steady state; the *math* of the controller is unchanged, so
     trajectories should be near-identical except where WR-1
     contamination was present. Regenerate with `UPDATE_GOLDENS=1`.
   - Add explicit tests:
     - `sibling_rbs_produce_identical_derived_quote`: construct two
       sibling RBs from the same parent and assert
       `B1.derived_quote == B2.derived_quote`.
     - `slot_battle_does_not_contaminate_canonical_quote`: walk a
       slot battle, fully validate both bodies at a representative,
       then assert the canonical chain's `derived_quote` sequence is
       identical to the no-slot-battle baseline.
     - `cache_pruning_keeps_memory_bounded`: stress test with deep
       chain growth and verify the local memoization cache is
       bounded.
   - `sim-cli/tests/determinism.rs`: regenerate the M5 suite
     goldens (`UPDATE_GOLDENS=1 cargo test --release -- --ignored
     determinism`); commit and tag.

7. **Update docs.**
   - `CLAUDE.md`: update the "Mechanism abstractions" section to
     describe the chain-derived pattern. Strike the "PricingBackend
     is policy-only" claim's accumulator framing; replace with
     "PricingBackend is pure-function; derived_quote is stored on
     each block." Update the "Determinism scope" and "Calibration
     choices" sections (update cadence is now "computed per block at
     production" rather than "per priced block by accumulator").
   - `docs/phase-2/mechanism-design.md`: reframe the "Controller
     signal" subsection to describe `derived_quote` as a block-level
     field; the math is unchanged but the spec language must reflect
     statelessness explicitly. The
     [§"Methodology: simulator approximations" table](../../../docs/phase-2/mechanism-design.md)
     gets a new row noting that the chain-derived implementation
     closes WR-1 (it was not part of the original spec, but is now
     the production model).
   - `.planning/REVIEW.md`: WR-1 row moves from "LIVE /
     disclosure-required" to "RESOLVED by chain-derived
     redesign"; reference this spike and the resulting plan.
   - `docs/phase-2/validity-threats.md`: WR-1 row updated to
     "Closed" with cross-reference.
   - `docs/phase-2/cardano-realism-audit.md`: spike 003's
     `NEEDS-DISCLOSURE` framing around windowing-vs-EIP-1559 is
     unchanged; this spike adds a new alignment point — the
     stateless/chain-derived pattern *also* aligns with EIP-1559.

8. **Regenerate M5 suite-level goldens.** Per the workflow in
   CLAUDE.md "Running the suites".

## Mapping to phase-2's open calibration questions

The chain-derived redesign affects only *where* state lives, not
*when* the controller updates or *what* it samples. So most
calibration choices from CLAUDE.md "Calibration choices" survive
unchanged:

- **Window length 32 (capacity-varying signals).** Same — the
  window is now a per-block aggregate over the last 32 canonical
  blocks rather than a node-local ring of the last 32 samples seen.
  Mathematically equivalent on the canonical chain.
- **Window length 1 (RB-reserved priority).** Same — per-block fill
  rate. Trivially chain-derived.
- **Update cadence: per priced block.** Same conceptually — each
  block computes its own `derived_quote` from parent + window. The
  word "update" is slightly misleading under chain-derivation
  because there is no mutable state being "updated" — but the
  cadence at which the quote changes is identical.
- **Un-reserved priority signal source (option 1).** Same — sample
  emission is unchanged in shape.
- **Both-dynamic standard signal source.** Same — `samples_for_block`
  policy is preserved verbatim.
- **Default `max_fee_policy = ScaledOverLaneQuote{4,1}`.** Unchanged;
  actor behaviour is unchanged.
- **`multiplier_floor` defaults (4, 8, 16).** Unchanged; enforced
  identically inside `compute_derived_quote`.
- **`rb-generation-probability = 0.05`, `default-slots = 1000`.**
  Unchanged.
- **`target_inclusion_blocks`.** Unchanged.
- **Mempool cap default.** Unchanged.

The one calibration concept that *strengthens* under chain-derivation
is the rollback story: under the accumulator design,
"contamination ~10⁻³" was a calibration-dependent number (it scales
with slot-battle rate, which depends on topology, stake
distribution, and rb-generation-probability). Under chain-derivation
it is identically zero across all calibrations.

## Sources

- **EIP-1559 specification**, "Specification" section: `base_fee_per_gas`
  calculation as a pure function of `parent.base_fee_per_gas`,
  `parent.gas_used`, `parent.gas_target`, and
  `BASE_FEE_MAX_CHANGE_DENOMINATOR`.
  <https://eips.ethereum.org/EIPS/eip-1559> — retrieved 2026-05-14.
- **Tim Roughgarden**, *"Transaction Fee Mechanism Design for the
  Ethereum Blockchain: An Economic Analysis of EIP-1559"*, 2020
  (revised 2021). Formal characterisation of EIP-1559 as a
  **stateless mechanism**: fee determination depends only on the
  parent block, not on persistent controller state held by nodes.
  Cited via [spike
  003](../003-pricing-controller-calibration/README.md) "Sources"
  section; arXiv:2012.00854.
- **Reijsbergen, Sridhar, Monnot, Leonardos et al.**, *"Transaction
  Fees on a Honeymoon: Ethereum's EIP-1559 One Month Later"*,
  arXiv:2110.04753, 2021. Empirical confirmation that EIP-1559's
  short-fork behaviour matches design intent; reorgs do not produce
  systemic fee misestimation on mainnet because the canonical chain
  re-derives its own base-fee sequence. Follow-on: *"Dynamics of
  Ethereum's EIP-1559…"* (DLT'R&P 2025). Cited in spike 003 for the
  windowing-rationale framing; cited here for the stateless-pattern
  empirical validation.
- **Internal phase-2 artefacts:**
  - [`.planning/REVIEW.md`](../../REVIEW.md) — WR-1 LIVE
    classification (2026-05-13).
  - [`.planning/smoke-comparison-2026-05-14.md`](../../smoke-comparison-2026-05-14.md)
    — empirical evidence of WR-1 contamination at ~1.2 battles/job
    on `topology-realistic-100.yaml`.
  - [`docs/phase-2/mechanism-design.md`](../../../docs/phase-2/mechanism-design.md)
    — the spec under which chain-derivation is being adopted.
  - [`CLAUDE.md`](../../../CLAUDE.md) — current accumulator-pattern
    description (pre-spike).
  - Spike 003,
    [`.planning/spikes/003-pricing-controller-calibration/README.md`](../003-pricing-controller-calibration/README.md)
    — EIP-1559 deployed parameters cross-reference and academic
    critique survey.
