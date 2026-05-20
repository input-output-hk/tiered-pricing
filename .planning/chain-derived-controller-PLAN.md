# PLAN — Chain-derived controller refactor (WR-1 fix)

Date: 2026-05-14
Goal: Replace the node-local mutable pricing accumulator with chain-derived `derived_quote` carried on every `LinearRankingBlock`, eliminating WR-1 orphan-block controller contamination by construction (EIP-1559 stateless pattern).
Spec: spike 007 (`.planning/spikes/007-chain-derived-controller/README.md`) — ADOPT verdict, 2026-05-14.

## Binding constraints (do not violate)

- **DO NOT modify `sim-rs/parameters/topology.default.yaml`** — upstream `main` consumes it; phase-2 work must not touch it.
- M2/M3 unit-test goldens and all 7 M5 suite-level goldens are expected to flip. Regenerating them via `UPDATE_GOLDENS=1` is in scope.
- Existing event-stream hash format and event types stay the same — `TXIncluded` and `TXEvictedQuoteDrift` remain the only hashed events; chain-derivation changes *where* `quote_per_byte` lives, not *what* gets hashed.
- The 12 fixes already applied by code review (WR-3, WR-4, WR-5, WR-6, IN-1..IN-8) must survive the refactor. Specifically: mempool↔gate byte-cap invariant assertion, `Eip1559Settings::validate` overflow bound, `eb_endorsement_valid` overflow warn-log, `is_representative` debug_assert, `Multiplier::new` denominator check, IN-2 div-by-D comment, IN-4 multiplier_floor ratio cap, IN-5 `is_zero_row` derived eq, IN-6 dedupe save, IN-7 BTreeMap test driver, IN-8 explicit `match cmp`.
- All simulation-affecting math stays integer/rational/u128. `f64` permitted only in already-cleared reporting paths (`retained_value`, `net_utility`) and the CR-1-blessed `endorsement_window_priced_blocks` (uses `libm::sqrt`/`libm::ceil` for cross-arch stability).

## Scope (files / behaviours changing)

- `sim-rs/sim-core/src/model.rs` — add `PerLaneQuote`, `WindowAggregate`, `CanonicalBlockSamples`; attach `derived_quote` + `window_aggregate` to `LinearRankingBlock`.
- `sim-rs/sim-core/src/tx_pricing/mod.rs` — refactor `PricingBackend` to pure-function shape; add `ChainView` trait; remove `current_quote`/`update_after_block`/`snapshot`/`worst_case_quote_at`.
- `sim-rs/sim-core/src/tx_pricing/window.rs` — strip persistent ring; provide `aggregate_from_chain` pure aggregator returning `WindowAggregate`; provide incremental update helper.
- `sim-rs/sim-core/src/tx_pricing/single_lane.rs` — `Eip1559Pricing` becomes stateless policy; expose `compute_step(parent_quote, aggregate, settings) -> u64` pure helper. `BaselinePricing` becomes a trivial constant returner.
- `sim-rs/sim-core/src/tx_pricing/two_lane.rs` — `TwoLanePricing` becomes stateless policy; multiplier-floor enforcement folded inside `compute_derived_quote`'s return. All four `TwoLaneVariant` arms preserved.
- `sim-rs/sim-core/src/sim/mempool_gate.rs` — `try_admit` and `revalidate` take chain-tip references for quote lookup.
- `sim-rs/sim-core/src/sim/linear_leios.rs` — remove `apply_priced_block` / `apply_eb_priced_block` / `feed_samples_and_revalidate`; compute `derived_quote` at block production; implement `ChainView` for `LinearLeiosNode`; route all `pricing.current_quote(lane)` reads through chain-tip lookup; staleness predictor walks forward via pure-function projection.
- `sim-rs/sim-core/src/sim/tests/{m1_smoke,m2_two_lane,m3_actors}.rs` — add `sibling_rbs_produce_identical_derived_quote` + `slot_battle_does_not_contaminate_canonical_quote` tests; regenerate inline golden constants for tests whose canonical-chain trajectory shifts.
- `sim-rs/sim-cli/tests/determinism.rs` — regenerate the 7 suite-level golden hashes under `parameters/phase-2-sweep/suites/.goldens/`.
- `CLAUDE.md` — update "Mechanism abstractions", "Determinism scope", "Calibration choices" wording for chain-derivation.
- `.planning/REVIEW.md` — WR-1 row moves to RESOLVED.
- `docs/phase-2/mechanism-design.md` — add "Chain-derived controller (implementation pattern)" section.
- `docs/phase-2/validity-threats.md` — WR-1 entry closed.
- `.planning/spikes/MANIFEST.md` — annotate spike 005 verdict with chain-derived resolution.

## Out of scope (defer)

- Implementing WR-2 (`AdmissionRejected { reason }` event-stream addition).
- Implementing WR-7 (`run_actors_for_slot` allocation refactor).
- Re-running phase-2 experimental suites and re-analysing welfare metrics (separate compute investment).
- Cross-architecture CI verification (already documented as not-yet-built in CLAUDE.md).
- Performance optimisation beyond the memoisation cache called out in Task 3.
- Changing the EIP-1559 step rule, multiplier-floor invariant, window-length default 32, or any "Calibration choices" knob — the rebuild is a pure implementation refactor, not a mechanism change.
- Adding "rollback events" to the event stream (not needed under chain-derived).
- Modifying `sim-rs/parameters/topology.default.yaml` (upstream-owned).
- Any `git commit`, `git tag`, or `git add` — leave all changes as working-tree modifications.

## Goal-backward verification (acceptance criteria)

The plan is done when ALL of:

1. **New unit test** `slot_battle_does_not_contaminate_canonical_quote` passes (Task 9). Two sibling RBs from the same parent produce identical `derived_quote`; the canonical winner's `derived_quote` is what subsequent gate/inclusion paths consult; the losing block's `derived_quote` is discarded with the block.
2. **New unit test** `sibling_rbs_produce_identical_derived_quote` passes — pure-function reasoning verified directly.
3. **Existing M2 unit-test goldens** in `sim-core/src/sim/tests/m2_two_lane.rs` pass after regeneration (inline `GOLDEN` constants at lines 970, 995 updated).
4. **Existing M3 unit-test goldens** in `sim-core/src/sim/tests/m3_actors.rs` pass after regeneration.
5. **M1 smoke test** in `sim-core/src/sim/tests/m1_smoke.rs` passes (no inline golden hash in M1; behavioural assertions only — these must still hold).
6. **All 7 M5 suite-level goldens** under `parameters/phase-2-sweep/suites/.goldens/*.sha256` regenerated and pass on a clean re-run via `cd sim-rs && cargo test --release -- --ignored determinism`.
7. **`cd sim-rs && cargo build --release`** completes with zero warnings beyond what `main` already emits.
8. **`cd sim-rs && cargo test --workspace`** passes (all 124+ sim-core unit tests, all sim-cli tests, all integration tests).
9. **WR-1 reclassifies to RESOLVED** in `.planning/REVIEW.md` with reference to spike 007 and this plan.
10. **No `pricing.current_quote(` / `update_after_block(` / `apply_priced_block(` / `apply_eb_priced_block(` references remain** in `sim-rs/sim-core/src/` (grep confirms zero matches, modulo deliberate comment references).
11. **No new `f64` reads** appear in simulation-affecting state; `quote_per_byte`, controller derivation, multiplier-floor enforcement remain integer/u128.
12. **Event-stream hash format preserved**: `TXIncluded` and `TXEvictedQuoteDrift` are still the only hashed events, fields unchanged, emission sites still fire (just from different code paths).
13. **WR-3..WR-6 / IN-1..IN-8 fixes intact** — spot-checked by re-reading the listed assertion / comment / structure changes after the refactor.

## Task DAG (sequencing)

```
T1 (model.rs types)
  → T2 (PricingBackend trait + ChainView)
    → T3 (Eip1559Pricing pure) ──┐
    → T4 (TwoLanePricing pure) ──┤
    → T5 (CapacityWeightedWindow aggregator) ──┘
      → T6 (MempoolGate signature)
        → T7 (linear_leios.rs: production, gate wiring, ChainView impl, staleness)
          → T8 (NEW slot-battle unit tests)
          → T9 (regen M1/M2/M3 inline goldens)
            → T10 (regen M5 suite goldens via UPDATE_GOLDENS=1)
              → T16 (final verification gate)

T11 (CLAUDE.md) ┐
T12 (REVIEW.md) │  all run after T7 in parallel; gated by T16
T13 (mechanism-design.md) │
T14 (validity-threats.md) │
T15 (MANIFEST.md) ┘
```

Critical path: T1 → T2 → {T3,T4,T5} parallel → T6 → T7 → T8 → T9 → T10 → T16. Doc tasks T11–T15 run in parallel after T7; T16 gates everything.

---

## Task breakdown

### Task 1: Extend block types with `derived_quote` and `WindowAggregate`

- **File:** `sim-rs/sim-core/src/model.rs`
- **Add types** (alongside existing block types; place after `LinearRankingBlockHeader` around line 94):
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
  pub struct PerLaneQuote { pub standard: u64, pub priority: u64 }

  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub struct WindowAggregate {
      pub standard_sum_bytes: u128,
      pub standard_sum_capacity: u128,
      pub priority_sum_bytes: u128,
      pub priority_sum_capacity: u128,
      pub blocks_in_window: u32,
  }
  ```
  `PerLaneQuote::get(lane) -> u64` helper for lane-keyed lookup.
- **Extend `LinearRankingBlock`** (currently lines 97–101) with two new fields:
  - `pub derived_quote: PerLaneQuote,`
  - `pub window_aggregate: WindowAggregate,`
  Both fields participate in `PartialEq`/`Eq` (the existing `derive` covers them — verify the manual `PartialEq` on `Transaction` is not affected).
- **Decision: EB does NOT carry `derived_quote`.** Document this with a code comment on `LinearEndorserBlock` (line 269): "EBs inherit `derived_quote` from their parent RB via chain-tip lookup. Adding a redundant field would risk drift between EB.parent_rb.derived_quote and EB.derived_quote on slot-battle paths." Spike 007 §"Edge cases" item 4 is the authoritative reference.
- **Constructor wiring:** every site that builds a `LinearRankingBlock` must populate the two new fields. Located in `linear_leios.rs` near line 853 (in `finish_generating_rb`). Tasks 7 onwards compute real values; in Task 1 use a sentinel `PerLaneQuote { standard: u64::MAX, priority: u64::MAX }` and `WindowAggregate::ZERO` so `cargo check` still passes — Task 7 replaces the sentinel with the real `compute_derived_quote(...)` call.
- **`CanonicalBlockSamples` record:** add a struct
  ```rust
  pub struct CanonicalBlockSamples {
      pub block_id: BlockId,
      pub samples: Vec<crate::tx_pricing::PricedBlockSample>,
  }
  ```
  Used by `ChainView` (Task 2) to expose per-block samples to the pure compute function.
- **Verification:** `cd sim-rs && cargo check` passes (compile-clean, may have warnings about unused sentinel fields — those go away in Task 7).
- **Done:** New types compile; `LinearRankingBlock` has both fields; every constructor still builds.

### Task 2: Refactor `PricingBackend` trait + introduce `ChainView`

- **File:** `sim-rs/sim-core/src/tx_pricing/mod.rs`
- **Add `ChainView` trait** (above the `PricingBackend` trait, around line 125):
  ```rust
  pub trait ChainView {
      /// k-th canonical ancestor of `from`, walking back along canonical
      /// parents only. Returns None when the chain runs out (cold start)
      /// or k exceeds available depth.
      fn ancestor(&self, from: crate::model::BlockId, k: u32) -> Option<crate::model::BlockId>;

      /// Samples that the given canonical block emitted (RB body + endorsed
      /// EB body, per the variant's samples_for_block policy).
      fn samples_in_block(&self, block_id: crate::model::BlockId) -> &[PricedBlockSample];

      /// derived_quote of a given canonical block (read of the field).
      fn derived_quote(&self, block_id: crate::model::BlockId) -> Option<crate::model::PerLaneQuote>;

      /// window_aggregate of a given canonical block (for incremental updates).
      fn window_aggregate(&self, block_id: crate::model::BlockId) -> Option<crate::model::WindowAggregate>;
  }
  ```
  No mutable methods; backend cannot mutate the simulator's chain.
- **Rewrite `PricingBackend` trait body** (replaces current lines 129–198):
  - **REMOVE** `current_quote(&self, lane) -> u64`. All callers will read `chain_tip.derived_quote.get(lane)` directly.
  - **REMOVE** `update_after_block(&mut self, samples: &[PricedBlockSample])`.
  - **REMOVE** `snapshot(&self) -> PricingSnapshot`. Replace with a free function `snapshot_at(block: &LinearRankingBlock, settings: &dyn PricingBackend) -> PricingSnapshot` so callers can render telemetry from any block (Task 7 wires this into `track_pricing_tick`).
  - **REMOVE** `worst_case_quote_at(&self, lane, blocks_ahead) -> u64`. Replaced by a free function `worst_case_quote_at(parent_quote, settings, lane, blocks_ahead) -> u64` (Task 3 provides the implementation, callable without backend state).
  - **ADD** `fn compute_derived_quote(&self, parent_quote: PerLaneQuote, parent_aggregate: WindowAggregate, parent_samples: &[PricedBlockSample], evicted_samples: &[PricedBlockSample]) -> (PerLaneQuote, WindowAggregate);` — the pure-function shape from spike 007 §"Type-level shape". `parent_samples` = samples emitted by the parent block; `evicted_samples` = samples falling off the window's tail this step (from the block at distance `window_length + 1` back, or empty during cold start).
  - **KEEP UNCHANGED** `lane_validity_rule`, `lane_selection_order`, `min_priority_premium_multiplier`, `samples_for_block` — already pure policy.
- **`Send + Sync` bound preserved** (the trait stays object-safe; `Box<dyn PricingBackend>` still works).
- **Verification:** `cd sim-rs && cargo check` will be RED until Tasks 3/4/5 update the impls; document this is expected. Task 2's exit criterion is "trait compiles" — downstream errors are tracked by Tasks 3+.
- **Done:** Trait file compiles in isolation (`cargo check -p sim-core --lib 2>&1 | grep "tx_pricing/mod.rs"` shows no errors local to that file); ChainView trait defined and object-safe.

### Task 3: Refactor `Eip1559Pricing` to stateless policy

- **File:** `sim-rs/sim-core/src/tx_pricing/single_lane.rs`
- **`BaselinePricing` changes** (lines 28–67):
  - Remove `quote_per_byte: u64` field if it can be derived from `settings.min_fee_a`; or keep as `Copy` configuration. Simplest: store `min_fee_a: u64` only.
  - Replace `current_quote`/`update_after_block`/`snapshot` impls with `compute_derived_quote` returning `PerLaneQuote { standard: min_fee_a, priority: min_fee_a }` and `WindowAggregate::ZERO` regardless of inputs (flat-fee policy carries no window).
- **`Eip1559Pricing` changes** (lines 150–300):
  - **REMOVE** fields `window: CapacityWeightedWindow` and `quote_per_byte: u64`. Keep `settings: Eip1559Settings` only — the struct becomes a configuration carrier.
  - **REMOVE** methods `step`, `step_with_lane`, `set_quote_for_floor`. The `step` math relocates to a free pure function:
    ```rust
    pub fn compute_eip1559_step(parent_quote: u64, util: (u128, u128), settings: &Eip1559Settings) -> u64
    ```
    Body: identical math to current `step` (lines 215–299) but takes `(util_num, util_den)` as a parameter rather than reading `self.window.aggregate_util()`, and returns the new quote rather than mutating `self.quote_per_byte`. Preserve WR-4's overflow-bound comment + the IN-2 div-by-D proof comment.
  - **ADD** `impl PricingBackend for Eip1559Pricing` with `compute_derived_quote` that:
    1. Updates the incoming `WindowAggregate` with `parent_samples` (add to sums + increment `blocks_in_window`, capped at `settings.window_length`).
    2. Subtracts `evicted_samples` from sums (only when `blocks_in_window` is at the cap).
    3. Calls `compute_eip1559_step(parent_quote.standard, (new_aggregate.standard_sum_bytes, new_aggregate.standard_sum_capacity), &self.settings)` for Standard lane.
    4. Returns `(PerLaneQuote { standard: <stepped>, priority: <stepped> }, new_aggregate)` — single-lane sets both lanes equal so callers reading either lane via `PerLaneQuote::get` get the right value.
- **`worst_case_eip1559_quote`** (lines 310–336): keep as-is, already pure. Add a `worst_case_quote_at(parent_quote, settings, lane, blocks_ahead)` free function for the staleness-predictor call site (Task 7).
- **Memoisation cache:** spike 007 §"Edge cases" item 1 calls for a per-`BlockId` cache to avoid recomputing `derived_quote` on every revisit. The chain-derived path computes per block at production once; the cache is for *consumers* (gate revalidation on tip change) that read the chain-tip's already-stored `derived_quote` field. **Decision: no separate cache.** The `derived_quote` field on the block IS the cache. The "memoisation cache" mentioned in spike 007 is degenerate under our architecture — block fields are O(1) lookup. Document this decision in a comment at the top of `single_lane.rs` and skip the cache infrastructure. If a future profile shows hot recomputation, revisit.
- **Tests:** existing `#[cfg(test)] mod tests` (lines 388–511) — rewrite each test to drive the pure function:
  - `baseline_pricing_returns_min_fee_a` → assert `BaselinePricing::new(44).compute_derived_quote(...).0.standard == 44`.
  - `eip1559_at_target_does_not_move` → seed parent_quote=1000, aggregate util=0.5, assert returned quote == 1000.
  - `eip1559_above_target_moves_up_within_step_clamp` → similar.
  - `eip1559_below_target_moves_down_within_step_clamp` → similar.
  - `eip1559_floor_at_min_fee_a` → loop 200 calls feeding parent.quote into next call; assert eventual floor.
  - `eip1559_uses_ceil_rounding_per_spec` → drive once with parent=44, util=1.0; assert returned quote == 50.
  - `eip1559_quote_drift_under_sustained_saturation` → loop 30 iterations, asserting monotone rise > 1500.
- **Verification:** `cd sim-rs && cargo test -p sim-core --lib tx_pricing::single_lane::` passes. `compute_eip1559_step` called twice with same args returns identical u64 (purity assertion — add a dedicated test).
- **Done:** `Eip1559Pricing` has zero mutable state beyond config; `compute_derived_quote` is pure; existing test invariants preserved with rewritten drivers.

### Task 4: Refactor `TwoLanePricing` to stateless policy

- **File:** `sim-rs/sim-core/src/tx_pricing/two_lane.rs`
- **`TwoLanePricing` field changes** (lines 149–155):
  - **REMOVE** fields `priority: Eip1559Pricing` and `standard: Eip1559Pricing` (they were the mutable controllers).
  - **KEEP** `settings: TwoLaneSettings`.
- **REMOVE** methods `enforce_multiplier_floor` (the stateful version at line 204), `priority_controller`/`standard_controller` accessors.
- **`compute_derived_quote` implementation:**
  1. Compute new aggregate from `parent_aggregate + parent_samples − evicted_samples`. Split bytes/capacity into `standard_*` and `priority_*` sums by sample's `controller_lane`.
  2. Compute `priority_quote = compute_eip1559_step(parent_quote.priority, (new_aggregate.priority_sum_bytes, new_aggregate.priority_sum_capacity), &self.settings.priority)`.
  3. If `self.settings.variant.standard_dynamic()`: `standard_quote = compute_eip1559_step(...)` against the standard aggregate. Else: `standard_quote = self.settings.standard.min_fee_a` (pinned at c=1).
  4. **Apply multiplier-floor invariant inside the return:** compute `floor = ceil(num × standard_quote / den)` in u128 (IN-4's ratio cap intact); if `priority_quote < floor`, raise priority_quote to floor. Preserve the IN-4 debug_assert that floor fits in u64 under the realistic-config ratio cap.
  5. Return `(PerLaneQuote { standard: standard_quote, priority: priority_quote }, new_aggregate)`.
- **Variant-specific aggregate handling:**
  - **RB-reserved variants:** priority controller's `window_length` is 1 (line 161). The aggregator must reflect this — when `variant.rb_priority_only()` is true, the priority lane's `blocks_in_window` caps at 1, evicting the prior block's priority samples on every step. Implement by routing through a variant-aware "effective window length" helper.
  - **Capacity-varying signals** (un-reserved priority, both-dynamic standard) use the configured `window_length` (default 32 per CLAUDE.md "Calibration choices").
- **`samples_for_block`** (lines 309–390): keep unchanged — already pure policy.
- **`min_priority_premium_multiplier`** (line 305): keep unchanged.
- **`lane_validity_rule`** (line 294): keep unchanged.
- **Construction invariant:** the constructor `TwoLanePricing::new` (line 157) still validates settings and forces `priority.window_length = 1` for RB-reserved variants. The multiplier-floor "applied at construction" path (line 178 `me.enforce_multiplier_floor()`) is now N/A — since there is no persistent state, the floor is enforced exclusively on the *output* of `compute_derived_quote`. Document this in a comment on `TwoLanePricing::new`.
- **Tests:** rewrite the `#[cfg(test)] mod tests` block (lines 404–689) to drive `compute_derived_quote` directly:
  - `multiplier_floor_holds_at_construction` → the constructor no longer enforces a floor on construction (no state to enforce on). Instead, assert that the FIRST `compute_derived_quote` call (with parent_quote = `PerLaneQuote { standard: 44, priority: 44 }` matching initial settings, empty aggregate, empty samples) returns a quote satisfying the floor.
  - `multiplier_floor_holds_after_standard_moves_up` → loop 20 iterations passing each call's output as next call's input; assert the floor at every step.
  - `priority_only_variant_pins_standard_at_min_fee_a` → assert standard_quote in output == 44 regardless of standard-sample saturation.
  - `rb_reserved_standard_isolation_does_not_move_c_standard_on_priority_rb` → call `compute_derived_quote` with a priority-only RB sample; assert returned `standard` == input parent.standard.
  - `rb_reserved_forces_priority_window_length_one` / `unreserved_keeps_priority_window_length_from_settings` → keep as constructor-introspection tests against `settings.priority.window_length`.
  - `rejects_zero_denominator_floor` / `rejects_floor_below_one` → unchanged (constructor validation).
  - `*_emits_*_sample_*` tests → unchanged (sample_for_block is unchanged).
- **Verification:** `cd sim-rs && cargo test -p sim-core --lib tx_pricing::two_lane::` passes. All four `TwoLaneVariant` arms covered with at least one test each.
- **Done:** `TwoLanePricing` has zero mutable state beyond settings; multiplier-floor invariant enforced inside `compute_derived_quote`; all four variants behave correctly.

### Task 5: Refactor `CapacityWeightedWindow` to pure aggregator

- **File:** `sim-rs/sim-core/src/tx_pricing/window.rs`
- **REMOVE** persistent `VecDeque<Sample>` ring + `sum_bytes`/`sum_capacity` state (lines 20–25, 57–71).
- **REPLACE** with:
  ```rust
  pub fn aggregate_from_chain<'a>(
      samples: impl IntoIterator<Item = &'a PricedBlockSample>,
  ) -> WindowAggregate;
  ```
  Walks an iterator of `PricedBlockSample`s and produces a `WindowAggregate` with bytes/capacity split by `controller_lane`. Used for cold-start computation (no parent aggregate available) and for tests.
- **ADD** an incremental update helper:
  ```rust
  pub fn update_aggregate(
      parent: WindowAggregate,
      add_samples: &[PricedBlockSample],
      evict_samples: &[PricedBlockSample],
      window_length: usize,
  ) -> WindowAggregate;
  ```
  Adds `add_samples` to the aggregate, subtracts `evict_samples`, caps `blocks_in_window` at `window_length`. Used by `Eip1559Pricing::compute_derived_quote` and `TwoLanePricing::compute_derived_quote`.
- **`WindowAggregate::ZERO`** constant — the cold-start state, all sums zero.
- **`WindowAggregate::aggregate_util(lane) -> (u128, u128)`** method — returns `(bytes_sum, capacity_sum)` for the requested lane, replacing the old `CapacityWeightedWindow::aggregate_util`. Returns `(0, 1)` when capacity sum is zero (existing convention from line 79).
- **Tests:** rewrite (lines 97–188) — `rejects_zero_length` becomes "rejects_zero_window_length_in_settings" (settings validation already covers this in `Eip1559Settings::validate`, so this test moves to single_lane.rs or is removed if redundant). Other tests pivot to `aggregate_from_chain` and `update_aggregate`:
  - `empty_iterator_returns_zero_aggregate` → `aggregate_from_chain([]) == WindowAggregate::ZERO`.
  - `heterogeneous_rb_and_eb_blocks_aggregate_correctly` → feed RB + EB samples, assert sums.
  - `length_one_reduces_to_per_block_fill_rate` → `update_aggregate` with `window_length=1`; second push evicts first.
  - `ring_evicts_oldest_when_full` → exercise `update_aggregate` with explicit `evict_samples`.
  - `endorsement_only_rb_with_zero_bytes_drags_aggregate_down` → unchanged semantics.
- **Verification:** `cd sim-rs && cargo test -p sim-core --lib tx_pricing::window::` passes; `aggregate_from_chain` called twice with same iterator content returns identical `WindowAggregate` (purity test).
- **Done:** `window.rs` exposes only pure functions; no persistent state; existing aggregate semantics preserved.

### Task 6: Update `MempoolGate` signatures for chain-tip lookup

- **File:** `sim-rs/sim-core/src/sim/mempool_gate.rs`
- **`try_admit` signature change** (line 152): keep the existing form
  ```rust
  pub fn try_admit(&mut self, tx: &Transaction, quote_per_byte_for_posted_lane: u64)
      -> Result<(), AdmissionRejection>
  ```
  **DO NOT** change the gate's signature to take a `&dyn ChainView`. The gate is correctly decoupled from the chain via the `quote_per_byte` parameter pattern. Task 7 reads `chain_tip.derived_quote.get(tx.posted_lane)` at the call site (in `try_add_tx_to_mempool`) and passes it as the existing `u64` parameter. This preserves WR-3's gate-is-sole-byte-cap-authority invariant and avoids reaching the simulator into the gate's body.
- **`revalidate` signature** (line 199): unchanged — already takes a `quote_for_lane` closure. Task 7's call site swaps from `lane -> self.pricing.current_quote(lane)` to `lane -> chain_tip.derived_quote.get(lane)`.
- **`on_inclusion` signature** (line 242): unchanged — already takes `served_lane` and `quote_per_byte_at_served_lane`. Task 7's call site sources the quote from the block being produced (whose `derived_quote` was just computed).
- **Internal logic:** unchanged. `fee_at`, byte-cap checks, eviction record building, refund computation — all preserved verbatim. WR-3's mempool-gate byte-cap invariant assertion stays at the constructor site in `linear_leios.rs` (Task 7 preserves it).
- **Tests:** `#[cfg(test)] mod tests` (lines 293+) — unchanged. The gate's unit tests already pass `quote_per_byte` as a literal `u64`; no API drift.
- **Verification:** `cd sim-rs && cargo test -p sim-core --lib sim::mempool_gate::` passes unchanged.
- **Done:** Gate's public API stable. The "chain-derived" change happens at the caller layer (Task 7), not inside the gate. This is the right seam — keeps the gate's tests stable and avoids re-validating the WR-3 invariant.

### Task 7: Refactor `linear_leios.rs` — block production, gate wiring, `ChainView` impl

- **File:** `sim-rs/sim-core/src/sim/linear_leios.rs`
- **Implement `ChainView` for `LinearLeiosNode`** (add `impl ChainView for LinearLeiosNode` block; place near the existing `NodeImpl` impl around line 423):
  - `ancestor(from, k)`: walk `self.praos.blocks.get(from).header.parent` repeatedly k times. Returns `None` on chain underrun.
  - `samples_in_block(block_id)`: lookup the cached samples for this RB (see "Per-block sample cache" below).
  - `derived_quote(block_id)` / `window_aggregate(block_id)`: read directly from `LinearRankingBlock`'s new fields.
- **Per-block sample cache:** `LinearLeiosNode` needs to remember the `PricedBlockSample` vec each canonical block emitted, so `ChainView::samples_in_block` can return them. Add a field `block_samples: BTreeMap<BlockId, Vec<PricedBlockSample>>` to the node. Populate in `publish_rb` (computed at production time anyway). Prune entries older than `2 × window_length` blocks behind the chain tip (spike 007 §"Edge cases" item 1). The pruning is the cache-bounding hook — anchor the prune to `publish_rb`'s tail.
- **Block production path** in `finish_generating_rb` (around line 853) and `publish_rb` (line 988):
  - Before constructing the new `LinearRankingBlock`, compute:
    1. `parent_id = parent` (already available from line 718).
    2. `parent_quote = self.derived_quote(parent_id).unwrap_or(initial_quote)` — initial quote for cold start derives from `BaselinePricing::new(min_fee_a)` or `Eip1559Settings.initial_quote_per_byte` per backend.
    3. `parent_aggregate = self.window_aggregate(parent_id).unwrap_or(WindowAggregate::ZERO)`.
    4. `parent_samples = self.samples_in_block(parent_id).cloned()` — samples the parent emitted (built when the parent was produced).
    5. `evicted_samples = self.samples_in_block(self.ancestor(parent_id, window_length))` — samples falling off the tail (empty during cold start).
    6. `(new_quote, new_aggregate) = self.pricing.compute_derived_quote(parent_quote, parent_aggregate, &parent_samples, &evicted_samples);`
    7. Attach `new_quote` and `new_aggregate` to the new RB's fields.
  - Compute `samples_this_block = self.pricing.samples_for_block(BlockKind::RankingBlock, &breakdown_for(&rb.transactions, ...))` for the new RB's body, plus EB samples if endorsed. Insert `(rb.header.id, samples_this_block)` into `self.block_samples` so future descendants can read them.
- **Remove `apply_priced_block`** (line 2060) and **`apply_eb_priced_block`** (line 2084) entirely. The work moves into the production code path above.
- **Remove `feed_samples_and_revalidate`** (line 2096). Replace with a `revalidate_against_new_tip(&mut self, new_tip_id: BlockId)` helper called from `publish_rb` after the new RB is inserted into `self.praos.blocks`:
  - Reads `new_tip.derived_quote.{standard,priority}`.
  - Calls `self.gate.revalidate(|lane| new_tip.derived_quote.get(lane))`.
  - Emits `TXEvictedQuoteDrift` events identically to the current code path (lines 2114–2151) — fan-out, mempool sync, peer announce all preserved.
- **Slot-battle resolution** (`finish_validating_rb_header`, line 1098): the existing logic is already correct under chain-derivation — when `old_block` is dropped (line 1115), no controller state needs unwinding because there is no controller state. **Add an assertion** (debug only) that documents the new invariant: "sibling blocks at the same slot, when both produced from the same parent, MUST have identical `derived_quote` — verified at runtime in debug builds, by construction in release builds." If the sibling has a different parent (which can happen on deeper reorgs), this assertion does not apply and is skipped.
- **Update all `self.pricing.current_quote(lane)` call sites** to read from the chain tip's `derived_quote`. Helper:
  ```rust
  fn current_chain_tip_quote(&self, lane: Lane) -> u64 {
      self.latest_rb_id()
          .and_then(|id| self.praos.blocks.get(&id))
          .and_then(|view| view.received_rb())
          .map(|rb| rb.derived_quote.get(lane))
          .unwrap_or_else(|| self.cold_start_quote(lane))
  }
  ```
  Replace at these sites:
  - Line 900–901 (`eb_endorsement_valid`)
  - Line 1768 (`try_add_tx_to_mempool` — pass to `gate.try_admit`)
  - Lines 1987–1988 (`charge_inclusions` — pass per served-lane to `gate.on_inclusion`)
  - Lines 2102–2103 (formerly inside `feed_samples_and_revalidate`; now inside `revalidate_against_new_tip`)
  - Lines 2308–2311 (`run_actors_for_slot` — feeds lane-choice math)
- **Staleness predictor** (line 373–388 + caller at line 1903): replace `pricing.worst_case_quote_at(lane, n)` (line 1906, 1909) with a free-function call:
  ```rust
  let priority_at_endorsement = worst_case_quote_at_chain(
      self.current_chain_tip_quote(Lane::Priority),
      self.pricing_settings_for(Lane::Priority),  // e.g. Eip1559Settings or none
      Lane::Priority,
      endorsement_window_blocks,
  );
  ```
  The math (lines 310–336 in `single_lane.rs`) is unchanged; only the input source moves from `self.pricing` to the chain-tip's stored quote. CR-1's `endorsement_window_priced_blocks` (`libm::sqrt`-based) is untouched.
- **`PricingTick` event emission** (around line 2280): the snapshot path currently calls `self.pricing.snapshot()`. Replace with `snapshot_at(chain_tip_rb, &*self.pricing)` returning a `PricingSnapshot` built from the chain tip's `derived_quote` + `window_aggregate`. Existing event field shape preserved (`standard_quote_per_byte`, `priority_quote_per_byte`, `standard_window_util_x_1e9`, `priority_window_util_x_1e9`).
- **`LinearLeiosNode::new`** (line 429): the backend construction (lines 461–473) is unchanged — `Box<dyn PricingBackend>` still selected by `PricingConfig`. The backend's pre-existing role as "settings carrier" is now its only role. WR-3's `debug_assert_eq!(mempool_max_size_bytes, gate.config().max_total_size_bytes)` (line 456) stays.
- **Cold start (genesis)**: the first RB has no parent. Its `derived_quote` defaults to:
  - `BaselinePricing`: `PerLaneQuote { standard: min_fee_a, priority: min_fee_a }`.
  - `Eip1559Pricing`: `PerLaneQuote { standard: settings.initial_quote_per_byte, priority: settings.initial_quote_per_byte }`.
  - `TwoLanePricing`: `PerLaneQuote { standard: settings.standard.initial_quote_per_byte, priority: max(settings.priority.initial_quote_per_byte, multiplier_floor × standard) }`.
  Implement this in a method `cold_start_quote(&self, lane: Lane) -> u64` on `LinearLeiosNode` (or as a free function `initial_quote(&dyn PricingBackend, Lane) -> u64` on the trait — pick whichever localises the variant matching cleanly). The constructor invariant ensuring the priority initial quote ≥ floor (current line 178 `me.enforce_multiplier_floor()`) folds into this helper.
- **WR-1 metric removal:** `track_linear_pricing_sample_applied` (line 1039) and the `slot_battles_count` / `orphaned_pricing_samples` metric counters can stay (they're observability infrastructure on existing event types) but the comment block at lines 1015–1038 explaining the M1 limitation MUST be rewritten to describe chain-derivation. The counter becomes a "sibling-pair-fully-validated-at-this-node" event for orphan-rate observability, no longer a contamination-bound signal.
- **Verification:**
  - `cd sim-rs && cargo build --release` clean.
  - `grep -n "self.pricing.current_quote\|self.pricing.update_after_block\|apply_priced_block\|apply_eb_priced_block\|feed_samples_and_revalidate" sim-rs/sim-core/src/sim/linear_leios.rs` returns zero matches (modulo comment references).
- **Done:** `linear_leios.rs` builds; backend is purely a settings carrier + pure-function policy; `derived_quote` is computed at production and read from the chain tip elsewhere.

### Task 8: New unit test — `slot_battle_does_not_contaminate_canonical_quote`

- **File:** new test module in `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs` (or new file `m_chain_derived.rs` reachable from the parent `tests/mod.rs`).
- **Test 1: `sibling_rbs_produce_identical_derived_quote`** — direct purity assertion:
  - Construct a parent `LinearRankingBlock` with a known `derived_quote`, `window_aggregate`, and pinned `samples_in_block`.
  - Twice call `backend.compute_derived_quote(parent_quote, parent_aggregate, &parent_samples, &evicted)` with identical inputs.
  - Assert both calls return identical `PerLaneQuote` and `WindowAggregate`.
- **Test 2: `slot_battle_does_not_contaminate_canonical_quote`** — end-to-end:
  - Stand up a 2-node single-suite scenario where slot battles can fire (use the `topology-single-producer.yaml` won't help — battles require ≥2 producers). Use the existing M2 multi-producer test harness if present; if not, construct a minimal harness with two nodes both running the lottery and producing competing RBs at the same slot (seed the RNG to force a slot battle).
  - Run the scenario for N slots.
  - At the canonical chain tip, walk back and compute the expected `derived_quote` trajectory via the pure function applied only to canonical blocks.
  - Assert: the on-chain `derived_quote` at each block matches the pure-function trajectory exactly.
  - **Negative control:** verify the test would have failed under the pre-refactor accumulator design — assert the chain has at least one slot battle (`tracker` exposes `track_linear_pricing_sample_applied` counts ≥ 2 at some slot). If zero battles, the test setup is wrong and must be re-seeded.
- **Test 3: `derived_quote_field_propagates_through_publish_rb`** — sanity:
  - Produce a block via the node's production path.
  - Assert `block.derived_quote` is non-sentinel (i.e. `compute_derived_quote` was actually called and Task 7 wired it through).
- **Verification:** `cd sim-rs && cargo test -p sim-core --lib sim::tests::` runs the new tests; all pass.
- **Done:** Three new tests cover purity, slot-battle invariance, and field propagation. WR-1's threat surface is now empirically closed.

### Task 9: Regenerate M1/M2/M3 inline unit-test goldens

- **Files:**
  - `sim-rs/sim-core/src/sim/tests/m1_smoke.rs` — no inline `GOLDEN` constants (behavioural assertions only). Confirm the smoke test still passes its existing invariants; no golden regen needed unless an assertion flips.
  - `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs` — two inline goldens at lines 970 and 995. Format: `const GOLDEN: &str = "<64-hex>";`.
  - `sim-rs/sim-core/src/sim/tests/m3_actors.rs` — inline goldens at the equivalent location (per `grep -n GOLDEN m3_actors.rs`).
- **Process per golden:**
  1. Run the test (`cargo test --release -p sim-core --lib <test_name>`).
  2. The assertion will fail with a message showing both the expected and actual hash (the existing assertion message body says "pricing event-stream hash drifted from the pinned golden value").
  3. Inspect the new hash; reason about whether the trajectory shift is expected under chain-derivation (the test's scenario should produce the SAME canonical-chain controller trajectory in steady state — divergence beyond pure timing/ordering shifts means there's a math bug in Task 3/4).
  4. If expected: update the inline constant to the new hex value.
  5. Re-run; confirm green.
- **DO NOT lower assertion strength.** If a test gets harder to express under chain-derivation (e.g. an assertion about mid-update mutation timing that doesn't exist anymore), rewrite the assertion to verify the chain-derived equivalent — do not weaken to `assert!(true)` or skip.
- **DO NOT update goldens by running once and copy-pasting blindly.** Each updated golden must be reasoned-about, with a one-line comment in the commit-ready diff (left as working-tree comment in the test file) explaining "Updated 2026-05-14 for chain-derived refactor; trajectory diverges from accumulator at slots N–M because <reason>".
- **Verification:** `cd sim-rs && cargo test --workspace` passes (all 124+ tests). No golden assertions fire.
- **Done:** All M2/M3 inline goldens reflect the chain-derived canonical trajectory. Each updated value documented inline.

### Task 10: Regenerate M5 suite-level goldens

- **Files:** `sim-rs/parameters/phase-2-sweep/suites/.goldens/*.sha256` (7 files — already shown as modified in `git status`).
- **Process:**
  1. `cd sim-rs && UPDATE_GOLDENS=1 cargo test --release -- --ignored determinism` (per CLAUDE.md "Running the suites" block).
  2. Verify all 7 `.sha256` files are now in working tree (`git status` should show them modified — they already are from prior work, this rewrites them).
  3. Re-run without `UPDATE_GOLDENS`: `cd sim-rs && cargo test --release -- --ignored determinism`. All 7 must pass.
- **Determinism verification:** for each suite, also run `cargo run --release --bin experiment-suite -- verify parameters/phase-2-sweep/suites/<suite>.yaml` to confirm the per-(job, seed) `pricing_event_stream.sha256` values persisted on disk under `output/` are consistent with a freshly-computed re-run. This is a stronger guarantee than the suite-level golden alone.
- **Verification:** zero golden assertion failures across all 7 suites in two consecutive `--release -- --ignored determinism` runs.
- **Done:** All M5 goldens regenerated and stable.

### Task 11: Update CLAUDE.md

- **File:** `CLAUDE.md`
- **Sections to edit:**
  - **"Mechanism abstractions"** — the `PricingBackend` bullet (around the start of the section): rewrite from "policy-only. Exposes `current_quote(lane)`, `update_after_block(samples)`..." to:
    > `PricingBackend` trait: pure-function policy. Exposes `compute_derived_quote(parent_quote, parent_aggregate, parent_samples, evicted_samples) -> (PerLaneQuote, WindowAggregate)`, `lane_validity_rule`, `lane_selection_order`, `min_priority_premium_multiplier`, `samples_for_block`. The backend holds no mutable controller state — `derived_quote` is computed per block at production and stored on the `LinearRankingBlock` as a header field. This matches Ethereum's EIP-1559 stateless pattern: orphan blocks from slot battles carry their own `derived_quote` which is discarded with the block, so controller contamination from short forks is impossible by construction (closes WR-1, per spike 007).
  - **"Calibration choices"** — "Update cadence: per priced block" bullet: reframe as "Derived-quote cadence: one quote per priced block, materialised on the canonical chain as `LinearRankingBlock.derived_quote`. The block is the unit of state; each block's `derived_quote` is a pure function of `parent.derived_quote`, `parent.window_aggregate`, and the samples carried by the parent and any endorsed EB."
  - **"Determinism scope"** — add a sentence to the closing paragraph: "Chain-derivation is reorg-safe by construction: deep reorgs replace the canonical chain entirely, and every block on the new chain carries its own `derived_quote` (computed as a pure function of its own ancestors), so no rollback step is needed and no contamination from orphan blocks is possible."
  - **"Numeric representation contract"** — add a sentence to the closing paragraph: "All chain-derived computation is integer/u128 throughout: `compute_derived_quote` is a pure function returning `PerLaneQuote` and `WindowAggregate`, both of which are `u64`/`u128` only. Block fields `derived_quote` and `window_aggregate` are bit-stable across architectures."
  - **NEW bullet under "Mechanism abstractions"** (insert near the `PricingBackend` bullet rewrite):
    > **`derived_quote` on `LinearRankingBlock`**: every RB carries a `PerLaneQuote { standard: u64, priority: u64 }` plus a `WindowAggregate` (the controller window's incremental state). These are pure functions of the parent RB plus samples in canonical predecessors. EBs do not carry `derived_quote` — they inherit it from their parent RB via chain lookup. The simulator's local block cache (`block_samples: BTreeMap<BlockId, Vec<PricedBlockSample>>`) is pruned at `2 × window_length` behind the chain tip to bound memory; under Cardano's k=2160 finality, this is trivially well within the chain-stability horizon.
- **DO NOT add WR-1 commentary to CLAUDE.md.** The "don't surface dormant threats" memory rule applies: WR-1 is resolved, not active, so it does not appear in operational docs (it appears in REVIEW.md as a closed historical entry — that is the right venue).
- **Verification:** the four bullet edits applied; CLAUDE.md still renders cleanly (no broken markdown).
- **Done:** CLAUDE.md describes the chain-derived pattern as the live mechanism, not as a future plan.

### Task 12: Update REVIEW.md — WR-1 to RESOLVED

- **File:** `.planning/REVIEW.md`
- **Edit:** in the "Fix Status" table (line ~17, the WR-1 row), change:
  - **Status column:** `LIVE / disclosure-required` → `RESOLVED via chain-derived refactor (2026-05-14, spike 007)`
  - **Notes column:** prepend `Closed 2026-05-14: chain-derived (EIP-1559-style) controller refactor adopted per spike 007. The accumulator's mutable node-local state is replaced by a `derived_quote: PerLaneQuote` field on each `LinearRankingBlock`, computed as a pure function of `parent.derived_quote` + `parent.window_aggregate` + samples in the parent. Slot-battle orphan blocks carry their own `derived_quote` that is discarded with the block — no controller contamination is possible. See `.planning/chain-derived-controller-PLAN.md` for the implementation deltas.`
  - Keep the historical line (`Pricing-state rollback on slot-battle reorg is no longer dormant...`) below the new closure note as historical context.
- **Edit deferred-items paragraph (line ~32):** remove WR-1 from the "Deferred items (WR-1, WR-2, WR-7)" list. Updated reads: "Deferred items (WR-2, WR-7) are surfaced to the user for explicit decision."
- **Verification:** `grep -c "RESOLVED" .planning/REVIEW.md` ≥ 1.
- **Done:** WR-1 marked closed; deferred-items list reflects current state.

### Task 13: Update mechanism-design.md

- **File:** `docs/phase-2/mechanism-design.md`
- **Add a new section** (after the existing "Open questions" section, before any appendices — the executor should grep for the closing section to find the right anchor):
  > ## Chain-derived controller (implementation pattern)
  >
  > The mechanism specifies a controller signal (capacity-weighted window over the last N canonical blocks) and an update rule (EIP-1559 step). The implementation realises this as a **chain-derived** quote: every `LinearRankingBlock` carries a `derived_quote: PerLaneQuote` field, computed at block production as a pure function of `parent.derived_quote`, `parent.window_aggregate`, and the samples emitted by canonical predecessors within the window.
  >
  > This matches Ethereum's EIP-1559 deployed pattern: the controller is *stateless at the node level*. There is no node-local mutable accumulator; the canonical chain itself carries the controller state. Three implications:
  >
  > 1. **Reorg-safe by construction.** When a slot battle resolves and the losing RB is dropped, its `derived_quote` is discarded along with the block — no rollback is needed because there is no per-node state to roll back. Sibling blocks produced from the same parent are guaranteed (by pure-function reasoning) to have identical `derived_quote`, so the canonical chain's pricing trajectory is invariant under slot-battle resolution.
  > 2. **Trivially auditable.** Any third party can re-derive the canonical chain's `derived_quote` sequence from the canonical blocks alone, given the published controller settings and the samples emitted by each block. There is no "controller state" hidden in a node's local memory.
  > 3. **No spec change.** The mechanism's math (EIP-1559 step, multiplier-floor invariant, capacity-weighted window aggregate, sample-emission rules) is unchanged. Chain-derivation changes *where* the state lives, not *what* the math computes.
  >
  > See `.planning/spikes/007-chain-derived-controller/README.md` for the design rationale; `.planning/chain-derived-controller-PLAN.md` for the implementation deltas.
- **Update any place where the spec implicitly assumes an accumulator:** grep for "accumulator", "controller state", "rollback" in `mechanism-design.md`; for each hit, either remove the accumulator framing or reframe as "the controller signal is the canonical chain's `derived_quote` sequence; there is no separate accumulator state to manage".
- **"Methodology: simulator approximations" table** (if present per spike 007's mention): add a row noting "Chain-derived implementation pattern closes WR-1 (orphan-block contamination)."
- **Verification:** new section present; no remaining references to "accumulator" or "rollback" in a forward-looking sense (historical mentions OK if clearly framed).
- **Done:** Spec describes chain-derivation as the implementation pattern.

### Task 14: Update validity-threats.md

- **File:** `docs/phase-2/validity-threats.md`
- **Edits:**
  - In the "Resolved" section, add an entry below the existing "Topology gap resolved 2026-05-13" block:
    > **WR-1 (controller contamination) resolved 2026-05-14.** Spike 007 adopted the chain-derived (EIP-1559-style) pattern as the WR-1 fix. The pricing controller's `derived_quote` is now stored on each `LinearRankingBlock` as a pure function of the parent's `derived_quote` + samples in canonical predecessors. Slot-battle orphan blocks cannot contaminate the canonical chain's controller trajectory by construction. See `.planning/chain-derived-controller-PLAN.md`.
  - Find the table row (if present) where each suite was rated MEDIUM-with-WR-1-disclosure or similar; remove the WR-1 disclosure caveat. If a suite was MEDIUM exclusively because of WR-1, upgrade to HIGH. If a suite was MEDIUM for multiple reasons, downgrade the WR-1 component from the rationale.
  - Update the TL;DR: if it currently says "MEDIUM count" includes WR-1-dependent suites, recount without WR-1 and update the numbers (executor must read the current TL;DR table and recompute).
- **Verification:** no remaining live-WR-1 references; trust ratings recomputed.
- **Done:** validity-threats.md reflects WR-1 closure.

### Task 15: Update `.planning/spikes/MANIFEST.md` for spike 005

- **File:** `.planning/spikes/MANIFEST.md`
- **Edit:** in the spike-005 row (the `SURFACED-DISCREPANCY` row), append to the "Notes" column (or the verdict if the row uses verdict-style): "Discrepancy resolved 2026-05-14 via spike 007 (chain-derived controller) + `.planning/chain-derived-controller-PLAN.md`."
- **Verification:** `grep "005" .planning/spikes/MANIFEST.md` shows the updated annotation.
- **Done:** spike 005's surfaced discrepancy is cross-referenced to its resolution.

### Task 16: Final verification gate

Run all of the following from `pwd = /home/will/git/arc-tiered-pricing`. All must pass before the plan is considered complete.

- `cd sim-rs && cargo build --release` → clean (zero new warnings).
- `cd sim-rs && cargo test --workspace` → all tests pass (including the new chain-derived tests from Task 8).
- `cd sim-rs && cargo test --release -- --ignored determinism` → all 7 M5 suite goldens pass (post-Task 10 regeneration).
- `grep -rn "self.pricing.current_quote\|self.pricing.update_after_block\|apply_priced_block\|apply_eb_priced_block\|feed_samples_and_revalidate" sim-rs/sim-core/src/` → zero non-comment matches (use `grep -v '^[[:space:]]*//'`).
- `grep -c "RESOLVED" .planning/REVIEW.md` → ≥ 1.
- `grep "chain-derived" CLAUDE.md` → matches present in the rewritten sections.
- `grep "chain-derived\|derived_quote" docs/phase-2/mechanism-design.md` → matches present.
- `git status --short` → shows ~25–30 modified/new files including: the seven `.sha256` files, `sim-rs/sim-core/src/{model.rs, tx_pricing/{mod,single_lane,two_lane,window}.rs, sim/{mempool_gate.rs, linear_leios.rs, tests/{m1_smoke,m2_two_lane,m3_actors}.rs}}`, `sim-rs/sim-cli/tests/determinism.rs` (only if Task 8 added entries there — likely not), `CLAUDE.md`, `.planning/REVIEW.md`, `.planning/spikes/MANIFEST.md`, `docs/phase-2/{mechanism-design,validity-threats}.md`, and the new `.planning/chain-derived-controller-PLAN.md` itself.
- **NOTHING committed and NOTHING tagged.** `git log -1 --format=%H` shows the same SHA as before the executor started.

If any of the above fails, the executor MUST stop and surface the failure for human review rather than work around it.

## Risks & mitigations

- **Risk: `ChainView` leaks too much of the simulator into the backend.** The trait should only expose ancestor walks, per-block samples, `derived_quote`, and `window_aggregate` — strictly read-only. Mitigation: keep the trait surface as defined in Task 2; resist adding methods that expose `praos`, `leios`, or mempool state.
- **Risk: memoisation cache memory growth.** Spike 007 calls for a per-`BlockId` cache; Task 3 records the decision to skip it (the block field IS the cache). The `block_samples` map in `LinearLeiosNode` (Task 7) does grow with chain depth — mitigation: prune at `2 × window_length` behind the chain tip in `publish_rb`.
- **Risk: M2/M3 unit-test scenarios depend on mutation-timing behaviour that doesn't survive chain-derivation.** Each test that flips its golden must be reasoned about (Task 9). If a test was implicitly verifying mutation-time semantics, redesign it to verify the chain-derived equivalent. If the test breaks for a legitimate semantic shift (the canonical-chain trajectory genuinely differs because the accumulator was contaminated), accept the new golden — that IS the fix landing.
- **Risk: performance regression from per-call walk-back.** Mitigation: the block field IS the cached value, so hot-path lookup is O(1). Walking back happens only at production time, bounded to `window_length` blocks. Cold paths are bounded by chain depth from a peer's first-seen point. Profile only if a determinism golden run takes meaningfully longer than the pre-refactor baseline (CLAUDE.md says ~1.5s; tolerate up to 3s, investigate beyond).
- **Risk: circular dependency between `LinearLeiosNode` and the pricing backend.** Mitigation: `ChainView` is implemented BY the node and consumed BY the backend via `&dyn ChainView` parameter. The backend never holds a reference to the node — only borrows one for the duration of a `compute_derived_quote` call. No `Rc<RefCell>` or similar required.
- **Risk: CR-1's `endorsement_window_priced_blocks` (uses `libm::sqrt`) interacts with chain-derived because the staleness predictor consumes the chain-derived quote.** Mitigation: Task 7 explicitly preserves CR-1's `libm::sqrt`/`libm::ceil` path; only the *input quote* to `StalenessPredictor` changes source (chain-tip lookup instead of `pricing.worst_case_quote_at`). The math is byte-identical.
- **Risk: WR-3 (mempool↔gate byte-cap invariant) accidentally undone.** Mitigation: Task 7 explicitly preserves `debug_assert_eq!(mempool_max_size_bytes, gate.config().max_total_size_bytes)` at line 456. Task 16's verification step greps for the assertion.
- **Risk: WR-4 (`Eip1559Settings::validate` overflow bound) lost in refactor.** Mitigation: Task 3 explicitly preserves the validation block (current lines 89–146); the validation runs at settings construction, which the new architecture still does.
- **Risk: WR-5 (`eb_endorsement_valid` overflow warn-log) lost.** Mitigation: Task 7 preserves the `eb_endorsement_valid` function (only the quote-source changes from `pricing.current_quote` to chain-tip lookup); the `tracing::warn!` at line 921 stays.
- **Risk: WR-6 (`is_representative` debug_assert) lost.** Mitigation: WR-6 lives in the metrics collector, not in the pricing kernel; refactor doesn't touch it. Verify intact via `grep "is_representative" sim-rs/sim-cli/src/metrics/` after Task 7.
- **Risk: sentinel `derived_quote` from Task 1 leaks into a real block.** Mitigation: Task 8 includes `derived_quote_field_propagates_through_publish_rb` which fails if the sentinel value `u64::MAX` ever appears on a real block.
- **Risk: WR-2 (gate-reject info loss) and WR-7 (actor allocation) regress under the refactor.** Mitigation: explicitly out of scope; verify their existing behaviour is unchanged by re-reading the relevant sites post-refactor. If chain-derivation makes either cleaner to address, surface as opportunistic follow-up — do NOT silently re-scope them in.
- **Risk: executor accidentally commits.** Mitigation: binding constraint at the top of this plan; Task 16 verifies `git log -1 --format=%H` is unchanged.
