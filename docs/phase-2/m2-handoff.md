# M2 → M3 handoff

Audience: the engineer picking up [implementation-plan.md §M3](implementation-plan.md#L266) (Actor model + metrics + suite runner + authored phase-2 suites). Read alongside [mechanism-design.md](mechanism-design.md) and [implementation-plan.md](implementation-plan.md) — those are authoritative; this note is just the M2 delta on top of [m1-handoff.md](m1-handoff.md).

## Branch state

`worktree-m2-two-lane`, on top of `dynamic-experiment` (which itself sits on `main`). M1 + M2 ship together as one delta on this branch.

- Build: `cd sim-rs && cargo build --release` clean.
- Tests: `cd sim-rs && cargo test --workspace` → 83 green, broken down as 4 (sim-cli) + 1 (sim-cli main) + 78 (sim-core lib) + 0 doc-tests. Specifically:
  - 14 new `tx_pricing::two_lane::tests` — controller-in-isolation coverage (multiplier-floor, sample-emission per variant, RB-reserved standard isolation at the controller level, settings validation).
  - 13 new `sim::tests::m2_two_lane::*` — end-to-end deterministic scenarios across all four variants, including the four-case EB binary fullness trigger, the two pinned refund-formula cases (a) and (b) at the gate API, both selection orders (`priority_first` and `Fifo`), partitioned + un-partitioned congestion sanity, the EB-validation-at-endorsement refusal *and* its through-simulator cascade-skip regression assertions (no priced sample, no `remove_conflicting_txs` cascade), and the cross-architecture determinism golden hash.

The hard rules (no `pricing-sim-base` content; no f64 in simulation-affecting state) held throughout. The new controller arithmetic (`u128` rationals), the multiplier-floor invariant, the EB binary trigger, the served-lane assignment, the per-variant sample emission, and the endorsement guard are all integer/enum.

## What M2 delivered

### New modules

- [sim-core/src/tx_pricing/two_lane.rs](../../sim-rs/sim-core/src/tx_pricing/two_lane.rs) — `TwoLanePricing`, `TwoLaneSettings`, `TwoLaneVariant` enum (`RbReservedPriorityOnly`, `RbReservedBothDynamic`, `UnreservedPriorityOnly`, `UnreservedBothDynamic`). Two `Eip1559Pricing` controllers per backend, multiplier-floor invariant enforced via `enforce_multiplier_floor` after both controllers run their independent step, variant-aware `samples_for_block` override.
- [sim-core/src/sim/tests/m2_two_lane.rs](../../sim-rs/sim-core/src/sim/tests/m2_two_lane.rs) — deterministic scenario tests, including the cross-architecture determinism golden hash.

### Modified

- [tx_pricing/mod.rs](../../sim-rs/sim-core/src/tx_pricing/mod.rs) — `BlockLaneBreakdown` struct, `samples_for_block` default-method on `PricingBackend` (single-lane backends inherit the trivial default emitting one `Standard` sample over total bytes; `TwoLanePricing` overrides per variant). Re-exports for `TwoLanePricing`, `TwoLaneSettings`, `TwoLaneVariant`.
- [tx_pricing/single_lane.rs](../../sim-rs/sim-core/src/tx_pricing/single_lane.rs) — added `Eip1559Pricing::step_with_lane(lane, samples)` and `Eip1559Pricing::set_quote_for_floor(quote)` so `TwoLanePricing` can drive two filtered controllers and then enforce the multiplier-floor invariant on top.
- [config.rs](../../sim-rs/sim-core/src/config.rs) — `RawPricingConfig::TwoLane(RawTwoLaneConfig)` arm; `RawTwoLaneVariant` mirrors `tx_pricing::TwoLaneVariant` for deserialisation; `PricingConfig::TwoLane(TwoLaneSettings)` mirror; build-time validation via `TwoLaneSettings::validate()` (rejects zero denominators and floors below 1).
- [linear_leios.rs](../../sim-rs/sim-core/src/sim/linear_leios.rs):
  - `LinearLeiosNode::new` constructs `TwoLanePricing` for the new config arm.
  - `sample_from_mempool_lane_aware(txs, max_size, remove, validity_rule, selection_order)` replaces the M1 lane-blind `sample_from_mempool`. Honours `LaneValidityRule::PriorityOnly` (skip standard-fee txs) and `LaneSelectionOrder::PriorityFirst` (priority-first scan order). The within-lane tiebreaker still defers to `MempoolSamplingStrategy`.
  - `select_eb_with_partition(...)` runs the spec's two-trigger fullness rule (saturation OR capacity-bound rejection). Returns `Vec<(Arc<Transaction>, Lane)>` plus an activated flag. **Test-only** (`#[cfg(test)]`); the producer's main flow uses `eb_served_lanes` (saturation rule) at endorsement time. Both paths agree on saturation; the capacity-bound trigger is exercised by the partition unit test only. M3's multi-producer flow may consolidate them.
  - `charge_inclusions(txs_with_served_lane)` replaces `charge_inclusions_for_rb_body`. Per-tx `served_lane` flows through; the actual fee is charged at the served-lane quote per [implementation-plan.md:96-100](implementation-plan.md#L96-L100).
  - `apply_priced_block` and `apply_eb_priced_block` (and `eb_samples`) compute a `BlockLaneBreakdown` from the block's transactions and call the backend's `samples_for_block` policy. The hard-coded single-lane sample emission is gone.
  - `try_generate_rb` reads `validity_rule` and `selection_order` from the backend; the RB body uses lane-aware sampling; the endorsement branch validates every candidate-EB tx against `posted_fee ≤ max_fee_lovelace` at the producer's current lane quote and **skips the endorsement entirely** if any tx is stale (refuse-to-endorse remedy per user direction at plan time).
  - Producer's RB-body inclusion uses `served_lane = Priority` for RB-reserved variants and `served_lane = posted_lane` for un-reserved/single-lane.
- [sim/tests/mod.rs](../../sim-rs/sim-core/src/sim/tests/mod.rs) — `mod m2_two_lane;`.
- [sim-core/Cargo.toml](../../sim-rs/sim-core/Cargo.toml) — `sha2` and `hex` added as dev-dependencies for the determinism golden-hash test.

## Decisions M2 made that M3 inherits

| Decision | Where | Why |
|---|---|---|
| **Sample-emission rules live on the backend** via the new `samples_for_block` default-method. | [tx_pricing/mod.rs](../../sim-rs/sim-core/src/tx_pricing/mod.rs), overridden in [tx_pricing/two_lane.rs](../../sim-rs/sim-core/src/tx_pricing/two_lane.rs) | Variant-aware emission rules are policy. Putting them on the backend keeps the per-variant numerator/denominator and the per-controller window cap (`min(priority_paying_bytes, max_block_size)`) next to the controller they feed. M3's actor-driven scenarios should not need to know which sample-shape goes to which controller — they hand the simulator the breakdown and the backend chooses. |
| **Multiplier-floor enforced on `quote_per_byte`, not on `c`.** Floor: `q_priority ≥ ceil(num × q_standard / den)`. | [tx_pricing/two_lane.rs `enforce_multiplier_floor`](../../sim-rs/sim-core/src/tx_pricing/two_lane.rs) | `quote = c × min_fee_a` ⟺ `min_fee_a` cancels in the cross-lane comparison. `u128` intermediates keep the floor exact. Window state is **not** touched by enforcement — only the final `quote_per_byte`. |
| **Multiplier-floor applied at construction.** | [TwoLanePricing::new](../../sim-rs/sim-core/src/tx_pricing/two_lane.rs) | If `priority.initial_quote_per_byte < floor × min_fee_a`, the floor wins. Tests assume this; M3 actor configs can rely on it. |
| **Priority-only variants pin standard at `c = 1`** (sample skipped, controller never stepped, `set_quote_for_floor` not called for it). | [TwoLanePricing::update_after_block](../../sim-rs/sim-core/src/tx_pricing/two_lane.rs) | Spec's "static standard" is just `c = 1` for the lifetime of the deployment. Implemented by ignoring Standard samples and not running the standard controller's `step_with_lane`. |
| **EB partition activation is recomputed at endorsement time using the saturation-only rule** (`body_bytes ≥ max_eb_size`). The two-trigger rule from plan lines 109-112 is honoured at EB-build time inside `select_eb_with_partition` (used by tests today). | [linear_leios.rs `eb_served_lanes`](../../sim-rs/sim-core/src/sim/linear_leios.rs) | The endorser does not have the EB-creator's mempool state, so the capacity-bound trigger can't be reproduced post-hoc. Saturation alone is on-chain observable from the EB body. For M2 single-producer scenarios this is consistent (saturation and capacity-bound rarely diverge); M3's multi-producer flows must revisit if endorser-side capacity-bound matters. |
| **Producer's RB body served-lane policy:** `served_lane = Priority` for RB-reserved variants; `served_lane = posted_lane` for un-reserved/single-lane (which collapses to `Standard` for the latter). | [linear_leios.rs `try_generate_rb`](../../sim-rs/sim-core/src/sim/linear_leios.rs) | RB-reserved RB validity rule already filtered out standard-fee txs; the survivors are by definition priority-fee, served as Priority. Un-reserved RBs serve at posted_lane (no partition). |
| **EB-validation-at-endorsement uses the refuse-to-endorse remedy.** | [linear_leios.rs `eb_endorsement_valid`](../../sim-rs/sim-core/src/sim/linear_leios.rs) | User direction at plan time. Filtering the EB body would mutate already-gossiped data and is not a clean fit for the spec; refusing the endorsement is. |
| **`samples_for_block` was added to the trait** (single-lane backends inherit a trivial default; two-lane override per variant). The M1 trait's signature otherwise stays as it was. | [tx_pricing/mod.rs](../../sim-rs/sim-core/src/tx_pricing/mod.rs) | Variant-aware sample rules don't fit cleanly into the simulator without coupling it to variant internals. The default keeps single-lane backwards-compatible; no external mocks of the trait existed before M2, so the breakage surface is internal-only. |
| **Multi-arch determinism is asserted intra-arch with a pinned golden hash.** | [m2_two_lane.rs `pricing_event_stream_deterministic_across_runs`](../../sim-rs/sim-core/src/sim/tests/m2_two_lane.rs) | A second-arch CI build is infrastructure work outside M2's scope. The golden hash is reproducible reference: a future arch (or a soft-float harness) verifies the same constant. The pinned value is `2c69ab58e4d76525d79df1dd68e6c539d8303fca95b44847243e0f062617ea79` (computed on x86_64 / glibc). |

## Where M3 picks up

Plan §M3 ([implementation-plan.md:266-275](implementation-plan.md#L266-L275)) lists four work items: actor model, metrics core, suite runner, and authored phase-2 suites.

The M2 simulator surface that M3 will lean on:

1. **Lane choice arrives in M3** ([implementation-plan.md:165-167](implementation-plan.md#L165-L167)). The actor model picks `posted_lane` per tx via utility maximisation. The plumbing on the simulator side is ready — `posted_lane` already flows through admission, mempool, selection, inclusion, refund, and pricing samples. Lane-choice math must use fixed-point or pinned-libm `pow`, never plain f64 (cross-platform determinism is asserted on the pricing event stream and that contract must continue to hold once actors are wired).

2. **`urgency: f64` on `Transaction` is unread by M2** for any simulation-affecting code path (handoff §"Gotchas" #1 from M1 still holds). M3's actor model is the first reader. If the lane-choice rounding pipeline doesn't go through `i128` lovelace before the comparison, the pricing-event-stream golden hash will flip.

3. **`served_lane` propagation: re-computed at endorsement, not propagated through the EB.** If M3 needs the producer's original served-lane decision (for instance, to attribute a refund event to the producer rather than the endorser), revisit `eb_served_lanes` and decide whether to bundle the served-lane vector with the EB. M2 single-producer tests don't need it.

4. **Backend trait already exposes `samples_for_block`.** Actor-emitted samples for M3's metrics layer can lean on the existing rules; if metrics need a per-block lane breakdown event, add it to `events.rs` (the breakdown is already computed by the simulator inside `apply_priced_block` / `apply_eb_priced_block`).

5. **Multi-architecture determinism CI**: documented gap. M2 ships an intra-arch determinism golden hash; M3 or later should add a second-architecture run against the same constant. The hash construction inside `run_seeded_pricing_scenario` is the canonical hash spec — same scenario, same fields, same little-endian byte ordering.

## Known limitations carried forward from M1 / introduced in M2

### 1. Pricing state has no rollback on fork/slot-battle (carried from M1)

Same caveat as M1 handoff §1. M2 single-producer scenarios don't exercise it. M5 (or earlier, if multi-producer suites surface fork resolution) must implement snapshot-and-replay. The code comment at the top of `apply_priced_block` is unchanged from M1.

### 2. EB-validation-at-endorsement uses refuse-to-endorse, not body filtering

By design (user direction). Drop-the-endorsement is wasteful when only one tx is stale (the whole EB is sacrificed) but the alternative — filtering the EB body — produces divergent block bodies across endorsers and is not what the spec contemplates. M3's multi-producer flows may revisit if endorsement throughput becomes a problem.

### 3. EB partition activation re-derived at endorsement uses saturation only

See *Decisions* table. The two-trigger rule (saturation OR capacity-bound rejection) lives at EB-build time inside `select_eb_with_partition`; the endorsement-time decision in `eb_served_lanes` uses saturation only. For M2 single-producer this is consistent because the same node sees the same mempool state. For M3 multi-producer flows, the endorser's view may differ from the producer's, and the served-lane decision will diverge — flagging here so M3 can make a consistent choice (either propagate the producer's decision via the EB, or recompute everywhere from saturation).

### 4. EB content validation is per-tx fee check only

`eb_endorsement_valid` checks `posted_fee ≤ max_fee_lovelace` at the producer's current quote for each tx in the candidate EB. It does not validate UTxO conflicts, double-spends, or other ledger-level invariants — those still go through the existing main pipeline. The handoff's §4 cascading-bug list (skewed pricing samples, polluted `spent_inputs`, mempool conflict cascades) was specifically about the stale-`maxFee` case; M2 closes it.

### 5. Multi-architecture determinism is intra-arch + golden-hash only

See *Decisions* table. Documented limitation; not a code defect.

### 6. `select_eb_with_partition` is `#[cfg(test)]` and the trigger has dual paths

The producer's main `try_generate_rb` flow uses `sample_from_mempool_lane_aware` for body packing and `eb_served_lanes` (saturation rule) at endorsement. The full two-trigger function `select_eb_with_partition` is gated `#[cfg(test)]` and exists solely to exercise both spec triggers in `eb_partition_unit_test_four_cases`. The two paths agree on the saturation trigger; only the capacity-bound trigger is unique to the test path.

If M3 brings the partition decision back to producer time (so the producer's view of "EB capacity-bound vs mempool-exhausted" matters on chain), `select_eb_with_partition` is the right starting point — but consolidate the two paths first so endorsement and EB-build use the same trigger logic.

## Architectural changes from M1 (flagged for M3 readers)

These are the structural shifts in M2 that change how M1 code reads. None of them are bug-prone but they're worth noting before a multi-producer rewrite:

1. **`charge_inclusions_for_rb_body` is gone** — replaced by `charge_inclusions(txs_with_served_lane)`. The served-lane vector is computed at selection time. M1's hard-coded `served_lane = Standard` is no longer reachable.

2. **`apply_priced_block` no longer hard-codes a single Standard sample** — it asks the backend `samples_for_block` for the lane-broken-down sample list. RB-reserved standard-isolation (line 313) is enforced inside the backend's variant override, not in the simulator.

3. **`sample_from_mempool` is gone** — replaced by `sample_from_mempool_lane_aware`. M1's `break`-on-too-big size rejection is preserved; the new validity-rule check (RB-reserved variants only) `continue`s past standard-fee txs since validity is independent of size. With `LaneValidityRule::None, LaneSelectionOrder::Fifo` the function reduces exactly to M1's loop.

4. **`try_generate_rb` reads pricing-backend policy** for both selection and validation. M1's flow was lane-blind; M2 reads `lane_validity_rule(BlockKind::RankingBlock)` and `lane_selection_order()` once, before generating.

5. **The EB endorsement closure now refuses to endorse stale EBs.** M1 passed staleness through to `charge_inclusions_for_rb_body` which emitted a `TXEvictedQuoteDrift` event but kept the EB on-chain. M2 returns `None` from the closure; the RB ships without an endorsement. Downstream (`incomplete_onchain_ebs`, `spent_inputs`, mempool conflicts) sees an unendorsed EB → no cascade.

## Gotchas

1. **Multiplier-floor at construction.** If `priority.initial_quote_per_byte < multiplier_floor × min_fee_a`, the constructor silently raises priority's quote to the floor. The tests rely on this. M3 actor configs that try to start `c_priority` near 1 in a both-dynamic configuration will see priority bumped up to ≥16× standard at startup.

2. **Window length is forced to 1 for RB-reserved priority controllers** even if the config sets a higher value. This is per [implementation-plan.md:47](implementation-plan.md#L47) — RB-reserved priority capacity is uniform per block, so length-1 reduces to per-block fill rate as the spec prescribes. The override happens in `TwoLanePricing::new` after settings are read.

3. **`urgency: f64` is still unread by simulation-affecting code.** M3 is the first reader. The cross-platform determinism golden hash will catch any accidental f64 entry into a hot path; if it flips after M3 lands, the lane-choice math is the first place to look.

4. **The pricing event-stream golden hash** is over `TXIncluded` and `TXEvictedQuoteDrift` only. M3 will add `TXSubmitted` (or similar admission events) and welfare-metric outputs; those should NOT be hashed against the same golden — they're not part of the simulation-affecting state. Keep the hash construction in `run_seeded_pricing_scenario` scoped exactly to those two event types.

5. **`samples_for_block` default vs override.** Single-lane backends inherit the default that emits one `Standard` sample over total bytes. If M3 needs a single-lane `Priority`-only flow for some experimental shape, it must override the default — relying on the default for non-Standard emission is a bug.

6. **The pricing event stream is tied to the simulator's RNG seed.** Any new f64-bearing code path that affects tx selection or inclusion order will flip the golden hash. The path forward is the integer/rational-only contract on simulation-affecting state — the golden test is the cheapest way to catch a regression.

## Test infra

- `m2_two_lane.rs::TwoLaneDriver` mirrors M1's `SmokeDriver` with a `posted_lane` argument on `make_tx`. For multi-producer scenarios in M3, extend the existing `TestDriver` in `tests/linear_leios.rs` rather than spinning up a new harness.
- `LinearLeiosNode::pricing_snapshot()`, `test_partition_trigger(...)`, `test_eb_endorsement_valid(...)` are `cfg(test)`-gated public accessors used by M2 scenario tests. They're convenient hooks for M3 to reuse if any test wants to inspect or exercise a single mechanism without going through the full slot machine.

## Recommended order of work for M3

1. **Actor model first**: `tx_actors.rs` per [implementation-plan.md:130-180](implementation-plan.md#L130-L180), with the pinned welfare formulas and the integer/rational `max_fee_policy`. Lane choice math (fixed-point or pinned `pow`) goes here. The pricing event-stream golden hash is your continuous check that lane choice stays integer-deterministic.
2. **Metrics core**: per-actor / per-component breakdowns, lane audit, time-series, comparison tables. This is read-only on the event stream — no simulation-affecting state changes.
3. **Suite runner**: resumable manifest, durable per-job state.
4. **Pricing TOMLs and overlays**: `protocol-base.yaml`, `paper_like_*` demand profiles, `eip1559_*.toml`, `two_lane_*.toml`, the five experiment overlays.
5. **Phase-2 suites 1-5** end-to-end runs.

## Hard rules — restated

These rules held throughout M2 and remain in force for M3:

1. **No code, configs, types, or schemas from `pricing-sim-base`.** Observe it as prior art only.
2. **No f64 in simulation-affecting state.** M3's new responsibility is actor lane-choice and welfare formulas — lane choice **is** simulation-affecting (it determines `posted_lane` which flows everywhere); welfare formulas in the metrics layer are not. Keep the boundary crisp; the cross-platform determinism golden hash is the regression alarm.
