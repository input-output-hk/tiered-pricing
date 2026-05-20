# M1 → M2 handoff

Audience: the engineer picking up [implementation-plan.md §M2](implementation-plan.md#L257) (Two-lane and full mechanism set). Read alongside [mechanism-design.md](mechanism-design.md) and [implementation-plan.md](implementation-plan.md) — those are authoritative; this note is just the M1 delta.

## Branch state

`dynamic-experiment`, on top of `main`. M1 added the spec kernel; no M2 work has started.

- Build: `cd sim-rs && cargo build --release` clean.
- Tests: `cd sim-rs && cargo test --workspace` → 55 green (8 mempool gate, 11 single_lane, 6 window, 1 m1 smoke, plus existing main tests).

The branch carries no code, configs, types, or schemas from `pricing-sim-base`. **Do not import from that branch.** Treat it as observable prior art only — the rule is in [implementation-plan.md:7](implementation-plan.md#L7) and [implementation-plan.md:324](implementation-plan.md#L324).

## What M1 delivered

### New modules (single-lane only; two-lane stubs absent by design)

- [sim-core/src/tx_pricing/mod.rs](../../sim-rs/sim-core/src/tx_pricing/mod.rs) — `PricingBackend` trait + `Lane`/`BlockKind`/`PricedBlockSample`/`Multiplier`/`LaneSelectionOrder`/`LaneValidityRule`/`PricingSnapshot`.
- [sim-core/src/tx_pricing/window.rs](../../sim-rs/sim-core/src/tx_pricing/window.rs) — `CapacityWeightedWindow` (u128 rationals, length-1 reduces to per-block fill rate).
- [sim-core/src/tx_pricing/single_lane.rs](../../sim-rs/sim-core/src/tx_pricing/single_lane.rs) — `BaselinePricing` (flat `c=1`) and `Eip1559Pricing` with the spec's clamp formula in u128 rationals; era floor at `min_fee_a`. Re-uses `CapacityWeightedWindow`.
- [sim-core/src/sim/mempool_gate.rs](../../sim-rs/sim-core/src/sim/mempool_gate.rs) — admission, revalidation, lane-aware byte tracking (Standard + Priority buckets already wired), inclusion charging. Pure u64.

### Modified

- [model.rs](../../sim-rs/sim-core/src/model.rs) — `Transaction` extended with `max_fee_lovelace`, `posted_lane`, `value_lovelace`, `urgency`, `urgency_component_index`. `urgency: f64` per the plan's text — see *Gotchas* below.
- [config.rs](../../sim-rs/sim-core/src/config.rs) — `RawPricingConfig`, `PricingConfig`, `MempoolGateConfig`, plus optional `min_fee_a` / `min_fee_b` / `mempool_max_total_size_bytes` / `pricing` `RawParameters` fields. Defaults per spec (44 / 155_381 / 2× `eb_referenced_txs_max_size_bytes`).
- [events.rs](../../sim-rs/sim-core/src/events.rs) — `Event::TXIncluded` (carries `posted_lane`, `served_lane`, `actual_fee_lovelace`, `refund_lovelace`) and `Event::TXEvictedQuoteDrift`. `EventTracker::track_tx_included` and `track_tx_evicted_quote_drift`.
- [linear_leios.rs](../../sim-rs/sim-core/src/sim/linear_leios.rs) — `pricing` and `gate` fields on `LinearLeiosNode`; admission gating in `try_add_tx_to_mempool`; `try_generate_rb` charges inclusions before mempool clears; `publish_rb` and `finish_validating_eb` feed samples and revalidate. New helpers: `charge_inclusions_for_rb_body`, `charge_inclusions_for_eb_txs`, `apply_priced_block`, `apply_eb_priced_block`, `eb_sample`, `feed_samples_and_revalidate`.

## Decisions M1 made that M2 inherits

| Decision | Where | Why |
|---|---|---|
| Pricing controller is **per-node**, fed at the node's `publish_rb` (and deferred-EB at `finish_validating_eb`) | [linear_leios.rs `apply_priced_block` / `apply_eb_priced_block`](../../sim-rs/sim-core/src/sim/linear_leios.rs) | Spec says "c is held in ledger state"; per-node is the simplest faithful approximation. Each node converges to the same state once it has seen the same priced blocks. M2 can keep this. |
| **Producer emits inclusion events**, every other node uses `gate.remove_silent` via `remove_txs_from_mempool` | `try_generate_rb` charges inclusions; `remove_txs_from_mempool` does silent removes | One inclusion event per included tx, attributable to the canonical producer. Don't move this hook to validators. |
| **`served_lane = Standard`** is hard-coded in `charge_inclusions_for_rb_body` | [linear_leios.rs](../../sim-rs/sim-core/src/sim/linear_leios.rs) | M1 is single-lane. **M2 must replace this with the EB binary-fullness logic + RB priority-only validity rule.** |
| `gate.remove_silent` is called from `remove_txs_from_mempool` | Keeps gate state consistent on every block-driven removal | Side effect: producer's inclusion charge runs first, then `publish_rb → remove_rb_txs_from_mempool` runs and the silent-remove is a no-op. Don't reorder. |
| Endorsement-only RB emits **no** RB sample; only the EB sample fires when the EB is validated | `apply_priced_block` checks `rb.transactions.is_empty()`; `finish_validating_eb` defers via `incomplete_onchain_ebs` | Per [implementation-plan.md:70](implementation-plan.md#L70). |
| **Gate is the sole byte-cap authority**: `LinearLeiosNode::new` passes `gate.config().max_total_size_bytes` to `Mempool::new`, so the legacy `leios_mempool_size_bytes` knob no longer governs the mempool cap in the wired path | [linear_leios.rs `LinearLeiosNode::new`](../../sim-rs/sim-core/src/sim/linear_leios.rs) | The gate runs admission first; with caps harmonised, the underlying mempool's queue-on-byte-cap-failure path stays dead. Required for fee-admission and quote-drift revalidation to cover every tx that enters the mempool. See §3 of *Known limitations*. |
| `Multiplier` rejects `denominator == 0` at construction | [tx_pricing/mod.rs](../../sim-rs/sim-core/src/tx_pricing/mod.rs) | Per [implementation-plan.md:38](implementation-plan.md#L38), [:178](implementation-plan.md#L178). |

## Where M2 picks up

Plan §M2 ([implementation-plan.md:257-265](implementation-plan.md#L257-L265)) lists four work items:

1. **`tx_pricing/two_lane.rs`** — new file. Two `Eip1559Pricing`-like controllers, `min_priority_premium_multiplier` invariant enforced post-update, configurable `LaneSelectionOrder`, per-axis × signal-source window-length wiring (length-1 for RB-reserved priority controllers, length-32 default for capacity-varying).
2. **Lane-aware block selection** — `try_generate_rb` and `sample_from_mempool` need to honour the backend's `lane_validity_rule(BlockKind)` and `lane_selection_order()`. This is where the **EB binary fullness trigger** ([implementation-plan.md:104-120](implementation-plan.md#L104-L120)) lives — currently absent.
3. **Deterministic two-lane scenario tests** — hand-rolled mempool arrivals exercising multiplier-floor enforcement, EB partition activation under saturation, EB partition non-activation under empty mempool, RB validity rejection of standard-fee txs, lane-mismatch refund accounting (the *general* refund formula — see [implementation-plan.md:311](implementation-plan.md#L311)), `priority_first` vs `fifo` selection order.
4. **Sanity check** — priority lane retains more value than standard under congestion in both partitioned and un-partitioned setups.

Plus M2 verification ([implementation-plan.md:307-314](implementation-plan.md#L307-L314)):
- RB partition validity under RB-reserved variants
- No RB validity rule under un-reserved + single-lane (already true; the trait default is `LaneValidityRule::None`)
- EB binary fullness trigger (three cases: not activated when room remains, not activated when mempool empty, activated when no remaining tx fits)
- Lane-mismatch refund formula must be the **general** `max_fee − actual_fee`, not a hardcoded "priority − standard" shortcut. Test cases (a) and (b) at [:311](implementation-plan.md#L311) are pinned.
- **RB-reserved standard isolation**: in partitioned both-dynamic, a saturated priority-only RB updates `c_priority` but does **not** touch `c_standard` or its window. Test by snapshot-before/after. Easy to get wrong if you naively emit a Standard sample for every block.
- **Cross-platform determinism**: same seed on x86_64 vs aarch64 → bit-identical event-stream SHA256. **Scope this to the pricing event stream** (`TXIncluded`, `TXEvictedQuoteDrift`, controller updates) — the rest of the simulator carries f64 from main (slot lottery, propagation, distribution sampling) which has not been hardened for cross-arch determinism, and that's not an M1/M2 task.

## Known limitations from M1 review

Surfaced during the M1 → M2 handoff review. §1 and §4 are blocking correctness issues for M2/M5; §2 and §3 are fixed in M1 but carry maintenance contracts that M2 must preserve.

### 1. Pricing state has no rollback on fork/slot-battle — blocking for M2/M5

`apply_priced_block` mutates `pricing` and `gate` immediately at `publish_rb`. `finish_validating_rb_header` does slot-battle replacement (lower VRF wins) by removing the losing block from `praos.blocks`, but it does **not** undo:

- controller updates the losing block triggered (`pricing.update_after_block`),
- gate `on_inclusion` removals,
- `TXIncluded` events already emitted for the losing block,
- `TXEvictedQuoteDrift` events already emitted by post-block revalidation.

The mechanism spec at [mechanism-design.md:115](mechanism-design.md#L115) treats `c` as ledger state, so canonical-chain reasoning requires rollback-and-replay. M1 ships without this because the smoke test is single-producer; M2's deterministic scenario tests should either (a) avoid slot-battle configurations or (b) add snapshot-and-replay rollback before the cross-platform determinism work in M5. There is a code comment at the top of `apply_priced_block` flagging this.

### 2. Inclusion accounting for txs not admitted via the local gate — fixed for normal flow

`charge_inclusions_for_rb_body` originally returned early when `gate.on_inclusion` returned `None` (tx not resident), which silently dropped inclusion events for any tx that bypassed local admission. M1 review fixes compute fee/refund from the `Transaction` directly; the gate is consulted only for state cleanup. Three scenarios this used to miss, now handled:

- Mock-mode tx generation (`TransactionConfig::Mock`) where `mock_tx(...)` constructs txs in `try_generate_rb` without going through admission. These have `max_fee_lovelace = u64::MAX` (default), so refund = `u64::MAX − actual_fee`, mathematically correct given the (default) max-fee but cosmetically large. Welfare metrics in M3 should filter on this sentinel if they don't want mock-mode skew.
- Withheld-tx attack scenarios (`generate_withheld_txs`, `receive_withheld_eb`) where attacker txs land in `self.txs` directly without admission. Same `max_fee_lovelace = u64::MAX` caveat as mock-mode.
- `LeiosVariant::Linear` multi-producer flow where an EB body carries txs that aren't separately propagated as Tx messages. A non-producing endorser's gate doesn't have those txs; the previous code skipped their inclusion event. With the fix, the endorser emits `TXIncluded` derived from the `Transaction` carried in the EB body. M1's single-node smoke test doesn't exercise this; M2 multi-producer tests should.

### 3. Gate is sole byte-cap authority — fixed; M2 maintenance contract

The original M1 wiring kept two byte caps active: legacy `Mempool::max_size_bytes` (from `leios_mempool_size_bytes`) and new `MempoolGate::max_total_size_bytes` (defaulting to 2× `eb_referenced_txs_max_size_bytes`). When the legacy cap was smaller, mempool's queue absorbed byte-cap-rejected txs and later promoted them via `remove_txs`/`remove_conflicting_txs` without consulting the gate, bypassing fee admission and quote-drift revalidation. M1 now sets the underlying mempool's cap to the gate's cap in `LinearLeiosNode::new`, so the gate is the sole byte-cap authority and the legacy queue path is dead.

**Maintenance contract for M2**: gate residency and mempool residency must move in lockstep. After tracing every path:

- `try_add_tx_to_mempool` admits gate-then-mempool with rollback on either failure (UTxO conflict in mempool → `gate.remove_silent`).
- `sample_from_mempool` (RB body, `remove=true`) decrements mempool; the subsequent `charge_inclusions_for_rb_body` decrements gate via `on_inclusion`. Both decrement for the same set within `try_generate_rb`; intermediate states aren't read.
- `remove_*_from_mempool` (block-driven) calls `remove_txs_from_mempool` which decrements both gate (via `remove_silent`) and mempool (via `remove_conflicting_txs`).
- `feed_samples_and_revalidate` decrements gate via `revalidate` and mempool via `remove_conflicting_txs` for the same evicted set.

If M2 introduces a new path that decrements one without the other (e.g. an "admit-without-mempool" or "remove-without-gate" route), parity breaks and re-introduces the queued-promotion bypass under different conditions. `debug_assert` invariants on byte counts at known synchronisation points are cheap insurance if you find yourself reasoning about this surface.

### 4. EB-validation-at-endorsement-time — blocking for M2 correctness

M1's [`charge_inclusions_for_rb_body`](../../sim-rs/sim-core/src/sim/linear_leios.rs) detects `actual_fee > tx.max_fee_lovelace` and emits `TXEvictedQuoteDrift` instead of a misleading `refund = 0` `TXIncluded`. **This is an event-accounting patch, not a validity fix.** Per [mechanism-design.md:43](mechanism-design.md#L43), such a tx is invalid for inclusion and the protocol layer would reject the EB at endorsement validation. M1 doesn't model that, so a stale-max-fee EB tx remains physically in the chain data path with several real consequences:

- **Pricing samples are skewed**: `apply_priced_block` sums `eb.txs[*].bytes` for the EB sample's `relevant_bytes`. Invalid txs still contribute, driving the controller upward as if they were legitimately served bytes.
- **Ledger spent_inputs polluted**: `resolve_ledger_state` walks endorsed EBs and inserts each tx's `input_id` into `spent_inputs`. Future legitimate txs sharing that input are then rejected at admission as "conflicting with on-chain state" even though the conflicting on-chain tx was never validly included.
- **Mempool conflict cascades**: `remove_rb_txs_from_mempool` calls `remove_conflicting_txs` over the EB's input_ids, which evicts other in-mempool txs that share an input — including legitimate ones.

M2 must add EB-content validation at the endorser's `try_generate_rb` endorsement branch: walk the candidate EB's txs, check `posted_fee ≤ max_fee_lovelace` at the producer's current quote for each tx, and either drop the endorsement entirely or filter the offending txs out of the EB before endorsing. Bundle this with the RB-reserved partition rule wiring — both checks run at the same point in the producer's flow.

## Architectural gaps M1 left for M2

### 1. `sample_from_mempool` is lane-blind

Current behaviour ([linear_leios.rs](../../sim-rs/sim-core/src/sim/linear_leios.rs) `sample_from_mempool`): walks `mempool.ids()` in either `OrderedById` or `Random` order; no notion of `posted_lane`. For M2:
- Read `pricing.lane_selection_order()`. If `PriorityFirst`, scan priority-lane txs first.
- Read `pricing.lane_validity_rule(BlockKind::RankingBlock)`. If `PriorityOnly`, skip standard-fee txs in the RB.
- For EBs, run the binary-fullness trigger to decide partition activation, then assign `served_lane` per [implementation-plan.md:114-118](implementation-plan.md#L114-L118).

`Mempool::ids()` is currently a flat iterator. You'll likely need a per-lane index or a filter step. The `MempoolGate` already tracks per-lane bytes (`bytes_in_lane`), so the resident set there can be your source of truth — but be aware the existing `linear_leios::Mempool` is the source of truth for **selection ordering** today.

### 2. `served_lane` assignment

`charge_inclusions_for_rb_body` hardcodes `Lane::Standard`. Replace with a per-tx `served_lane` decided at selection time:
- Single-lane: always `Standard` (current behaviour).
- RB-reserved: RB body txs → `Priority`; EB priority partition → `Priority`; EB overflow + standard space → `Standard` (refund kicks in for posted-priority txs).
- Un-reserved: `served_lane = posted_lane`.

The cleanest pattern is to make selection produce `Vec<(Arc<Transaction>, Lane)>` instead of `Vec<Arc<Transaction>>`, and pass the served-lane decision into the inclusion charge.

### 3. Sample emission must become variant-aware

`apply_priced_block` currently emits one `Standard` sample per tx-bearing RB and one per EB. For two-lane variants ([implementation-plan.md:65-77](implementation-plan.md#L65-L77)):
- **RB-reserved priority-only / partitioned both-dynamic**: RB emits **only** a `Priority` sample (not `Standard`). Standard controller never sees RB samples.
- **Un-reserved priority-only / un-partitioned both-dynamic**: RB emits one sample per active controller, each with that lane's posted-fee bytes against `max_block_size`.
- EB samples branch similarly per [implementation-plan.md:71-75](implementation-plan.md#L71-L75).

The signature of `apply_priced_block` is fine (a `Vec<PricedBlockSample>` slice) — the work is in deciding which samples to emit.

### 4. `min_priority_premium_multiplier` invariant enforcement

[implementation-plan.md:38](implementation-plan.md#L38) says the invariant lives **inside the controller update path**, not in a metrics layer. After both controllers run their EIP-1559 step, clamp `c_priority ← max(c_priority, multiplier_floor × c_standard)`. Use rationals; no f64.

## Gotchas

1. **`urgency: f64` on `Transaction`**. M1 doesn't read it from any simulation-affecting code path. The plan permits this because actor lane-choice (which uses urgency) only arrives in M3 with fixed-point/pinned-libm math ([implementation-plan.md:165-167](implementation-plan.md#L165-L167)). M2 should not start reading `urgency` for anything simulation-affecting; if you need to, use the same pattern that M3 will (fixed-point or pinned `pow`).

2. **Manual `PartialEq` on `Transaction`** uses `urgency.to_bits()` for reflexivity over NaN. If M2 adds another `f64` field to `Transaction`, follow the same pattern. Don't `#[derive(PartialEq)]` over an f64.

3. **`incomplete_onchain_ebs` coordination** for deferred EB samples: the existing `HashSet<EndorserBlockId>` is used both for (a) "we endorsed this EB but don't have the body validated" (existing main behaviour) and (b) "I, as a node, owe the priced-block sample for this EB once it validates" (M1 addition). Keep these aligned — `apply_priced_block` only emits the EB sample if `get_validated_eb` returns `Some`; otherwise `finish_validating_eb` emits it later. If M2 changes the timing of `incomplete_onchain_ebs` insertion/removal, audit both paths.

4. **`gate.revalidate` returns evicted records**; the wired `feed_samples_and_revalidate` translates each into an event AND a `mempool.remove_conflicting_txs(input_ids)` call. This last bit is needed because the existing `linear_leios::Mempool` doesn't have a "remove by tx_id without checking input conflicts" path — `remove_conflicting_txs` is the closest. If M2 adds richer tx-id-based removal to `Mempool`, this can simplify.

5. **The `Mempool` byte cap in [linear_leios.rs:1541-1582](../../sim-rs/sim-core/src/sim/linear_leios.rs)** is still the *original* main implementation, including a "queue" concept that absorbs byte-cap-rejected txs for later promotion. M1 sets the underlying `Mempool::max_size_bytes` to the gate's cap so the queue path stays dead in the wired flow. If M2 reduces the mempool's cap below the gate's (or admits txs to mempool without first running them through the gate), queued promotions can re-enter the active mempool without admission and bypass quote-drift revalidation — re-introducing the bug §3 of *Known limitations* describes.

6. **f64 in `RawParameters`** (e.g. `tx_size_bytes_distribution`, latency_ms) drives non-pricing simulation. Cross-arch determinism for those is not in scope per the plan; the M2 determinism test should hash only the pricing event stream.

7. **`get_validated_eb` borrow check**: I duplicated the EB into a local `Arc` inside `apply_priced_block` because the borrow checker doesn't let you hold an `&EndorserBlockView::Received` while calling other `&mut self` methods. M2 will hit the same pattern when scaling out — the simplest fix is `clone()` on the `Arc<EndorserBlock>` and operate on that.

## Test infra

[m1_smoke.rs](../../sim-rs/sim-core/src/sim/tests/m1_smoke.rs) ships a single-node `SmokeDriver` with manual lottery + slot tick + tx submission + event drain. M2 deterministic tests can either extend this or use the existing `TestDriver` in [linear_leios.rs](../../sim-rs/sim-core/src/sim/tests/linear_leios.rs). For two-lane scenarios you'll likely want a 2-3 node topology (existing `TestDriver` supports this; just feed an `Eip1559Pricing` or two-lane config via a similar `RawPricingConfig` override).

The deterministic generator is just hand-rolled `Transaction` literals via `make_tx(bytes, max_fee_lovelace)`. For two-lane scenarios, accept `posted_lane` as an additional parameter.

## Recommended order of work for M2

1. **`tx_pricing/two_lane.rs`** with the multiplier-floor invariant + unit tests on the controller in isolation. Mirror the M1 unit-test structure for `Eip1559Pricing`.
2. **Backend trait surface**: implement `lane_validity_rule`, `lane_selection_order`, `min_priority_premium_multiplier` for the four two-lane variants. Snapshot type stays as-is unless you need richer fields.
3. **Lane-aware selection** in `linear_leios.rs`. This is the riskiest step ([implementation-plan.md:325](implementation-plan.md#L325)) — slow down here. Get the EB binary fullness trigger right with isolated tests before integrating.
4. **`served_lane` plumbing** through `charge_inclusions_for_rb_body` (rename to `charge_inclusions(txs, served_lane_per_tx)` or pass a closure).
5. **Variant-aware sample emission** in `apply_priced_block` and `apply_eb_priced_block`.
6. **Deterministic scenario tests** matching plan lines 261-262 and verification 307-314.
7. **Cross-platform determinism test** at the end. Scope it to the pricing event stream.

## Hard rules — restated

These are the rules I held throughout M1; they remain in force.

1. **No code, configs, types, or schemas from `pricing-sim-base`.** Observe it as prior art only.
2. **No f64 in simulation-affecting state.** Admission, eviction, fee charging, controller state, mempool tracking, maxFee computation, and (M2's new responsibility) lane choice and the multiplier-floor invariant. Use u64/u128 integers and rationals. f64 is permitted only in reporting outputs, which arrive in M3.

If a critical-review pass surfaces a violation of either rule, the violation is the bug.
