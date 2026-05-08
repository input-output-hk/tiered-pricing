# M3 → M4 handoff

> **Postscript (post-M5).** The calibration framing in §Known-limitation #9 below — the "M3 §9 degeneracy" — was not a calibration choice. It was a calibration *bug*: `rb-generation-probability: 1.0` combined with the linear-Leios endorsement window prevented EBs from ever landing on chain, which in turn produced the latency-zero / standard-zero outcomes documented as a degeneracy. The bug was fixed post-M5 by dropping `rb-generation-probability` to 0.05, increasing `stake` to 100000 (to preserve probability under VRF stake quantization), and bumping `default-slots` to 1000. Re-runs replaced the suite outputs and goldens. See [calibration-fix-postmortem.md](calibration-fix-postmortem.md) for the full explanation. The body of this handoff is preserved as historical context.

Audience: the engineer picking up [implementation-plan.md §M4](implementation-plan.md#L278) (Reframed experimental questions: RB scarcity, urgency inversion). Read alongside [mechanism-design.md](mechanism-design.md) and [implementation-plan.md](implementation-plan.md) — those are authoritative; this note is just the M3 delta on top of [m1-handoff.md](m1-handoff.md) and [m2-handoff.md](m2-handoff.md).

## Branch state

`dynamic-experiment` (no worktree, per project preference). M1 + M2 + M3 all ship as one accumulated delta on this branch.

- Build: `cd sim-rs && cargo build --release` clean.
- Tests: `cd sim-rs && cargo test --workspace` → 118 green:
  - 113 sim-core lib tests (M2's 79 + 28 new `tx_actors::tests::*` + 6 new `sim::tests::m3_actors::*`).
  - 4 sim-cli lib tests, 1 sim-cli main test.
- Five phase-2 suites run to completion and produce well-formed
  `time_series.csv`, `diagnostics.log`, `run_summary.json`,
  `pricing_event_stream.sha256`, and per-suite
  `metrics_comparison.txt` artefacts. Manifest-based resume preserves
  the full per-suite comparison: skipped-completed jobs reload their
  persisted `run_summary.json` from disk before the comparison is
  re-written. `experiment-suite verify <suite.yaml>` re-runs every
  Completed (job, seed) and asserts each freshly-computed pricing-
  event-stream SHA256 matches the persisted value (plan §M3
  verification line 321 inline determinism check).

The hard rules (no `pricing-sim-base` content; no f64 in simulation-affecting state) held throughout. The new actor lane-choice math, `MaxFeePolicy::ScaledOverLaneQuote`, and the latency estimator's f64 outputs all flow through `i128` lovelace before any `>` comparison; reporting-side welfare formulas (`retained_value`, `net_utility`, `retained_value_ratio`) are plain f64 and never feed back into simulation decisions.

## What M3 delivered

### New modules

- [sim-core/src/tx_actors.rs](../../sim-rs/sim-core/src/tx_actors.rs) — `ActorComponent`, `ActorProfile`, `MaxFeePolicy::ScaledOverLaneQuote { numerator, denominator }`, `LanePolicy::UtilityMaximising { submit_when_underwater }`, `LaneInputs`, `LatencyEstimator`, `lane_choice::pick`, `welfare::{retained_value, net_utility, retained_value_ratio}`. Lane-choice math uses `libm::pow` and `libm::round` (round-half-away-from-zero) so the `i128` lovelace comparison is bit-deterministic given identical inputs.
- [sim-core/src/sim/tests/m3_actors.rs](../../sim-rs/sim-core/src/sim/tests/m3_actors.rs) — single-node integration tests covering arrival rate, default `ScaledOverLaneQuote{4,1}` max-fee policy, high-urgency lane choice, underwater-skip, and pricing-event-stream determinism through the actor path.
- [sim-cli/src/lib.rs](../../sim-rs/sim-cli/src/lib.rs) + [sim-cli/src/metrics/](../../sim-rs/sim-cli/src/metrics/) — `MetricsCollector`, `time_series::write_csv`, `comparison::write_suite`, `diagnostics::write`. Subscribes to the same event stream as `EventMonitor`; produces phase-2 artefacts.
- [sim-cli/src/runner.rs](../../sim-rs/sim-cli/src/runner.rs) — `Manifest`, `JobStatus`, `JobEntry`, `run_suite`. Resumable-by-default: re-running a suite skips `Completed` entries and retries `Running|Pending|Failed`.
- [sim-cli/src/suite.rs](../../sim-rs/sim-cli/src/suite.rs) — `Suite` + `Job` + `JobOverrides` deserialisation surface.
- [sim-cli/src/bin/experiment-suite/main.rs](../../sim-rs/sim-cli/src/bin/experiment-suite/main.rs) — `experiment-suite run <suite.yaml>` and `experiment-suite status <suite.yaml>` CLI.

### New configs and suites

- [parameters/phase-2-sweep/protocol-base.yaml](../../sim-rs/parameters/phase-2-sweep/protocol-base.yaml) — phase-2 protocol overlay (`min_fee_a = 44`, `min_fee_b = 155381`, `simulate-transactions: true`, `rb-generation-probability: 1.0`, `vote-threshold: 1`).
- [parameters/phase-2-sweep/topology-single-producer.yaml](../../sim-rs/parameters/phase-2-sweep/topology-single-producer.yaml) — one-node topology (stake = 1, `tx-generation-weight: 1`) so the actor-bearing node also wins the RB lottery.
- `parameters/phase-2-sweep/demand/{paper_like_moderate,paper_like_congested}.yaml` — three-component weighted actor profiles (high/medium/low urgency).
- `parameters/phase-2-sweep/pricing/*.yaml` — 13 pricing configs covering the EIP-1559 D × target × window sweep, RB-reserved + un-reserved priority-only multiplier sweep, and partitioned + un-partitioned both-dynamic.
- `parameters/phase-2-sweep/suites/phase-2-{eip1559-robustness, eip1559-smoothing, priority-only-rb-reserved, priority-only-unreserved, two-lane-both-dynamic}.yaml` — five suite YAMLs, 18 jobs × 3 seeds = 54 runs total.

### Modified

- [sim-core/Cargo.toml](../../sim-rs/sim-core/Cargo.toml) — added `libm = "0.2"` (Sun's libm port; bit-deterministic across architectures).
- [sim-core/src/lib.rs](../../sim-rs/sim-core/src/lib.rs) — `pub mod tx_actors;`.
- [sim-core/src/config.rs](../../sim-rs/sim-core/src/config.rs):
  - `RawActorProfile`, `RawActorComponent`, `RawMaxFeePolicy`, `RawLanePolicy` deserialisation surface plus phase-2 default helpers (`default_max_fee_policy = ScaledOverLaneQuote{4,1}`, `default_target_inclusion_blocks_priority = 1.0`, `default_target_inclusion_blocks_standard = 4.0`).
  - `RawParameters.actors: Option<RawActorProfile>` + `SimConfiguration.actors: Option<Arc<ActorProfile>>` + public accessors (`actor_profile()`, `block_generation_probability()`).
  - Validation runs `ActorProfile::validate()` at config-build time.
- [sim-core/src/events.rs](../../sim-rs/sim-core/src/events.rs):
  - `Event::TXGenerated` extended with `urgency_component_index`, `value_lovelace`, `urgency`, `posted_lane`, `max_fee_lovelace` (defaults preserve legacy traces).
  - `Event::PricingTick` (per-slot per-node) so the metrics layer can populate `time_series.csv`.
  - `EventTracker::track_pricing_tick` helper.
- [sim-core/src/sim/tx.rs](../../sim-rs/sim-core/src/sim/tx.rs) — legacy `TransactionProducer` is silenced when `actor_profile().is_some()` (its `config` becomes `None` and `run` waits forever).
- [sim-core/src/model.rs](../../sim-rs/sim-core/src/model.rs) — `LinearEndorserBlock.partition_activated: bool`.
- [sim-core/src/sim/linear_leios.rs](../../sim-rs/sim-core/src/sim/linear_leios.rs):
  - `NodeActorState` (per-node actor sampling state); per-component `ChaChaRng` derived from the node's RNG; `LatencyEstimator` per (component, lane).
  - `run_actors_for_slot(slot)` runs at the start of `handle_new_slot`; reads the node's `pricing.snapshot()`, samples per-component arrivals via Poisson, picks `posted_lane` via `lane_choice::pick`, computes `max_fee_lovelace` via the component's `MaxFeePolicy`, builds a `Transaction` (encoding `(node_id, counter)` into `tx_id`/`input_id`), and submits via `propagate_tx`.
  - `emit_pricing_tick(slot)` runs at the start of `handle_new_slot`; emits `Event::PricingTick`.
  - `observe_actor_inclusion`/`forget_actor_pending` plumb `LatencyEstimator` updates and stale-pending cleanup through `charge_inclusions` and `feed_samples_and_revalidate`.
  - **Dual partition-trigger paths consolidated** (M2 handoff §6 forward-pointer + the user's M3 directive): `select_eb_with_partition` is no longer `cfg(test)` — it's the production path. `eb_served_lanes` is replaced by `assign_served_lanes`, which takes the producer's `partition_activated` bit (carried on the EB) and the variant's `rb_reserved` flag and returns per-tx served lanes deterministically.
- [sim-core/src/sim/tests/m2_two_lane.rs](../../sim-rs/sim-core/src/sim/tests/m2_two_lane.rs) — added `pricing_event_stream_deterministic_across_runs_unreserved`, a second cross-arch determinism golden hash exercising the un-reserved priority controller's sample path. Pinned constant: `7a976da3778c11887665769a6af32eccc41f6d735b2140ef035fee67d05eb91c` (computed on x86_64 / glibc). The original RB-reserved hash (`2c69ab58...`) is unchanged — partition consolidation produced the same served-lane assignments at saturation.
- [sim-cli/Cargo.toml](../../sim-rs/sim-cli/Cargo.toml) — declared `[lib]` and `[[bin]] experiment-suite` paths; added `chrono`; enabled the `toml` feature on `figment`.
- [sim-cli/src/events.rs](../../sim-rs/sim-cli/src/events.rs) — added `Event::PricingTick { .. } => {}` arm so the legacy event monitor remains exhaustive.

## Decisions M3 made that M4 inherits

| Decision | Where | Why |
|---|---|---|
| **Actor model lives per-node, not as a single producer task.** Each `LinearLeiosNode` owns its actor state and runs sampling at `handle_new_slot`. | [linear_leios.rs `run_actors_for_slot`](../../sim-rs/sim-core/src/sim/linear_leios.rs) | Lane choice needs the node's pricing snapshot. Per-node sampling avoids cross-task shared state and keeps determinism in a single chain of integer/rational state. The legacy `TransactionProducer` is silenced when actors are configured. |
| **Lane-choice math is `libm::pow` + `libm::round`, rounded into `i128` lovelace before the `>` comparison.** | [tx_actors::lane_choice](../../sim-rs/sim-core/src/tx_actors.rs) | Plan line 167. `f64::powf` is forbidden because IEEE-754 only mandates bit-exactness for `+ − × ÷ √`. `libm` is bit-stable across architectures given identical inputs. The `libm::round` choice (vs `as i128` truncation) is pinned because the integer event stream's determinism depends on it. |
| **Default actor `max_fee_policy = ScaledOverLaneQuote{4,1}`.** | [config.rs `default_max_fee_policy`](../../sim-rs/sim-core/src/config.rs) | Plan line 136. M3-default; M4/M5 may override per experiment. |
| **Default `target_inclusion_blocks_priority = 1.0`, `standard = 4.0`.** | [config.rs](../../sim-rs/sim-core/src/config.rs) | Plan line 163. The `LatencyEstimator` overwrites these from observed inclusions; defaults seed it. |
| **Window length 32 for capacity-varying signals; 1 for the RB-reserved priority controller** (already enforced by `TwoLanePricing::new` in M2). | [tx_pricing/two_lane.rs](../../sim-rs/sim-core/src/tx_pricing/two_lane.rs) | Plan lines 44, 47. Carries forward unchanged. |
| **`partition_activated: bool` is stored on `LinearEndorserBlock` as a producer claim.** | [model.rs](../../sim-rs/sim-core/src/model.rs), [linear_leios.rs `assign_served_lanes`](../../sim-rs/sim-core/src/sim/linear_leios.rs) | The capacity-bound trigger is not re-derivable from the EB body alone (it needs the producer's mempool view). Storing the bit makes producer/endorser served-lane assignment match by construction in honest-producer simulation. **Trust-model note for M4/M5**: a future attacker model could test producer dishonesty by setting the bit inconsistent with the EB's contents. The current simulator does not check this. Flag in the relevant attacker config when written. |
| **`Event::PricingTick` is per-node per-slot.** The metrics collector picks the lowest-name node as "representative" and uses its ticks for the time-series. Other nodes' ticks are dropped at the collector. | [linear_leios.rs `emit_pricing_tick`](../../sim-rs/sim-core/src/sim/linear_leios.rs), [metrics/collector.rs `is_representative`](../../sim-rs/sim-cli/src/metrics/collector.rs) | In single-producer suites all nodes converge to the same pricing state. For multi-producer M4 suites with diverging per-node state (e.g., slot-battle scenarios), the collector currently picks one and ignores the rest — see *Known limitations* §3. |
| **Suite runner uses `tokio::current_thread` to drive each (job, seed) sequentially.** | [runner.rs `run_suite`](../../sim-rs/sim-cli/src/runner.rs) | The `Simulation` future is `!Send` (because the per-node actor state contains f64-bearing distributions). Running on the current thread sidesteps Send bounds; sequential job execution is fine for M3's runtime budget. M4+ may parallelise *across* (job, seed) at the runner level if runtime becomes a problem. |
| **`Event::TXGenerated` carries actor-relevant fields directly.** Welfare metadata (`urgency_component_index`, `value_lovelace`, `urgency`, `posted_lane`, `max_fee_lovelace`) is on the event so the metrics collector can join welfare data on `TXIncluded` without a separate channel. | [events.rs](../../sim-rs/sim-core/src/events.rs) | Removes the need for an out-of-band runner→collector hook. Legacy traces deserialise unchanged because the new fields default to zero/`Lane::Standard`. |
| **`metrics_comparison.txt` preserves negative `net_utility`** through every aggregation step (per-component and per-suite). | [metrics/collector.rs](../../sim-rs/sim-cli/src/metrics/collector.rs), [metrics/comparison.rs](../../sim-rs/sim-cli/src/metrics/comparison.rs) | Plan line 152 — regret events (included tx whose `retained_value < actual_fee`) are part of the welfare picture and must not be clamped or filtered. |

## Where M4 picks up

Plan §M4 ([implementation-plan.md:278-284](implementation-plan.md#L278-L284)) lists two new suites and a documentation step:

1. **`phase-2-rb-scarcity.yaml`** — restate the RB-capacity scarcity question on the two-lane mechanism. Likely shape: a new protocol-overlay YAML reducing `rb_body_max_size_bytes` (and possibly `eb_referenced_txs_max_size_bytes`) plus the existing two-lane pricing configs. Run; confirm the experimental question (does priority-lane access hold up under RB scarcity?) is answerable from the output.
2. **`phase-2-urgency-inversion.yaml`** — restate the urgency-inversion question on a two-lane priority-dynamic mechanism. **New demand profile** with mis-priced actors: high-urgency components paired with a low-multiplier `MaxFeePolicy` (e.g., `ScaledOverLaneQuote { numerator: 1, denominator: 1 }`) so their max-fee budget can't cover the priority quote even though their value × decay justifies priority service. Compare against `paper_like_congested.yaml` actors who use the default `{4, 1}`.
3. **README per suite** documenting how each new framing relates to the previous tiered-backend formulation. If a question genuinely doesn't translate (plan line 282), drop it with a written rationale rather than reintroducing a tiered backend.

The M3 surface M4 will lean on:

1. **Actor-config plumbing is fully data-driven.** Adding a new demand profile is a YAML edit (`actors:` block); no code changes required. The `RawMaxFeePolicy::ScaledOverLaneQuote { numerator, denominator }` lets M4 author mis-priced actors at any rational multiplier.
2. **Per-component urgency-component-index** plumbs through to the metrics layer. The `metrics_comparison.txt`'s per-component breakdown lets M4 distinguish high-urgency-mis-priced from high-urgency-correctly-priced within the same suite.
3. **Suite YAML's `default_*` + per-job `overrides`** support per-job protocol overrides via `overrides.protocol`. The RB-scarcity suite can author one overlay reducing `rb_body_max_size_bytes` and bind it on a single job.
4. **Resumable manifest** makes long M4 suites (more seeds, more jobs) safe to interrupt and resume.
5. **The cross-arch determinism golden hashes** (RB-reserved + un-reserved) catch any accidental f64 entry into simulation-affecting state. M4's new code paths must keep them green.

## Known limitations carried forward + introduced in M3

### 1. Pricing state has no rollback on fork/slot-battle (carried from M1)

Same caveat as M1 handoff §1 and M2 handoff §1. M3's single-producer suites don't exercise it. Multi-producer M4 suites (if added) or M5 must implement snapshot-and-replay before slot-battle scenarios are admissible.

### 2. EB partition activation is a producer claim, not derivable from the EB body

By design (consolidation decision; see *Decisions* table). For honest-producer simulation, endorser and producer agree on served-lane assignment by construction. **Attacker models in M4/M5 may exploit this** by setting `partition_activated` inconsistent with the EB's contents; flag this in the attacker-config when one is written.

### 3. `PricingTick` is per-node; metrics use a single representative

In multi-producer flows where node A and node B reach divergent pricing state (e.g., slot-battle replacement), the metrics layer drops B's ticks and reports A's. For M3's single-producer suites this is moot. M4 multi-producer suites should either (a) ensure all producers converge on the same priced-block stream or (b) extend the collector to aggregate across nodes (mean? min? max?). Until then, the time-series under-reports cross-node disagreement.

### 4. Latency observation is per-node, not per-tx-actually-included

`LatencyEstimator` observes `(inclusion_slot − submit_slot) × block_generation_probability` only on the producer node that admitted the actor's tx into its own gate. If a tx is admitted on node A but included by node B's RB, A doesn't see the inclusion event for the latency observation. M3's single-producer suite path collapses A and B to the same node, so this is invisible. M4 multi-producer suites must revisit if the latency estimator drives lane choice at meaningful magnitudes.

### 5. The five M3 suites do not exercise quote-drift evictions at scale

With the default `MaxFeePolicy::ScaledOverLaneQuote{4, 1}`, `max_fee_lovelace = 4 × quote × bytes + min_fee_b`. Quote drift would have to multiply the per-byte price by 4× before any tx is evicted. Under congestion, the controller does drift the quote up (visible in c_priority/c_standard time-series), but not by enough in 200 slots to push large numbers of txs above their max-fee budget. The `evicted_quote_drift_count` column is populated but small. M4/M5 may want to author an additional suite (or a `--max-fee-policy ScaledOverLaneQuote{1,1}` overlay) to exercise quote drift at scale.

### 6. Multi-architecture determinism is intra-arch + golden-hash only (carried from M2)

`libm::pow` and `libm::round` are bit-stable across architectures given identical inputs, **but** the inputs themselves come through `rand_distr`'s f64 distributions which call `f64::ln`/`f64::exp` (not in IEEE-754's bit-exact mandate). Sampling drift can cause cross-arch divergence in `posted_lane` through different upstream values. The simulator's pricing event-stream golden hashes (M2's RB-reserved + M3's un-reserved) are asserted intra-arch only. Multi-arch CI verification is M5/CI infrastructure.

### 7. `simulate-transactions: true` is required for actor mode

The actor model only fires on the `Real` packing path inside `try_generate_rb`. Setting `simulate-transactions: false` switches to the Mock path which adds dummy txs to the RB/EB body alongside the actor's txs and pollutes the metrics. The phase-2 protocol-base pins `simulate-transactions: true`; M4/M5 configs must keep this if they layer on top.

### 8. RB-reserved rejection count is in `time_series.csv`, not as a counter

Plan line 320 asks for "diagnostics.log counts the rejections" of standard-fee txs in RBs under RB-reserved variants. The simulator skips standard-fee txs during RB-body packing in `sample_from_mempool_lane_aware` (validity-rule branch), not via an event — so the rejection count isn't directly observable from the event stream. The runner emits an Info note in `diagnostics.log` for RB-reserved variants pointing the reader at the equivalent CSV evidence: `included_count_standard` per slot is 0 on RB-only inclusions; non-zero values reflect EB-side standard inclusions, not RB-rule violations. Threading a dedicated rejection event is an M4/M5 enhancement if the count itself becomes load-bearing.

### 9. `priority_lane_retained_value_ratio = 1.0`, `standard = 0.0` is degenerate under the M3 calibration

The M3 protocol-base pins `rb-generation-probability: 1.0` (so every slot produces an RB) and uses a single producer with `tx-generation-weight: 1`. Combined with a high-urgency demand profile and the spec's 16× multiplier-floor, the actor's lane-choice math always picks `Posted::Priority` and every included tx is served at Priority. The `standard_lane_retained_value_ratio` reads 0.0 because no txs land at `served_lane = Standard` — *not* because standard service performed badly, but because no demand for standard service exists. M4 should either (a) author a demand profile with mis-priced actors that *can't* afford priority (the urgency-inversion suite already plans this), or (b) lower `rb-generation-probability` so EB partition non-activation refunds priority txs to standard. Until either is in place, this metric pair is informational only.

### 10. Pricing-config sweep is on the lighter side of the plan's "20-30" target

M3 ships 13 pricing configs (`parameters/phase-2-sweep/pricing/*.yaml`). The plan recommends 20-30 (line 218); M3's exit criterion checks pass at 13 because the qualitative results are already legible. M4 may want to expand the sweep — particularly for `phase-2-eip1559-smoothing` and `phase-2-eip1559-robustness` — if a finer-grained smoothing-vs-robustness comparison is needed for the down-select argument.

## Architectural changes from M2 (flagged for M4 readers)

These are the structural shifts in M3 that change how M2 code reads. None are bug-prone, but worth noting before M4 work:

1. **`select_eb_with_partition` is the production path.** M2's `eb_served_lanes` (saturation-only) is gone; `assign_served_lanes` takes the EB's stored `partition_activated` bit + body bytes and produces served lanes. Existing M2 tests use the new path transparently.
2. **`LinearEndorserBlock` has a new field**: `partition_activated: bool`. Legacy code that constructs an EB literal must set it explicitly. Test helpers default it to `false` (no partition); production sets it from `select_eb_with_partition`.
3. **`Event::TXGenerated` carries five new fields** (`urgency_component_index`, `value_lovelace`, `urgency`, `posted_lane`, `max_fee_lovelace`). Legacy traces that don't have these fields deserialise with serde defaults (zero/`Lane::Standard`).
4. **`Event::PricingTick` is a new variant.** Code that exhaustively matches `Event` variants must add an arm for it (the legacy `EventMonitor` already does).
5. **`SimConfiguration` has `actors: Option<Arc<ActorProfile>>`** — a new field. Public accessor `actor_profile()` returns it.
6. **`LinearLeiosNode::handle_new_slot` calls `emit_pricing_tick` and `run_actors_for_slot`** before `try_generate_rb`. Order matters: the pricing snapshot the actors consult is captured at slot-start, before any block production this slot.
7. **The legacy `TransactionProducer` is silenced** when actors are configured (it returns `None` for `config` and waits forever). Existing non-actor configs are unaffected.

## Gotchas

1. **`libm::round` (round-half-away-from-zero)** is the pinned f64 → i128 rounding rule for lane choice. Rust's default `as i128` truncates toward zero, biasing positive values downward by up to one lovelace. Don't switch to `as i128` for "performance" — the integer event stream's determinism depends on the chosen rule.
2. **The actor's `urgency` is clamped to `[1.0, ∞)`** at sample time. A demand profile that authors an `urgency: { distribution: constant, value: 0.5 }` will see all txs treated as if `urgency = 1.0` (no decay). The `tx_actors::tests::sampled_inputs_are_clamped_to_safe_ranges` test pins this.
3. **The actor encodes `(node_id, counter)` into `tx_id` and `input_id`** so multi-node setups don't collide. Format: `((node_id as u64) << 48) | (counter & 0xFFFF_FFFF_FFFF)`. Legacy tx generators (Real and Mock) use a separate counter space starting from 0, so collisions are possible if both legacy and actor modes run simultaneously — but the legacy producer is silenced in actor mode, so this isn't reachable in practice.
4. **`tx_generation_weight: 1` is required on at least one stake-bearing node** for actor mode to fire on the RB-producing node. The default topology has every node carrying stake, which (without `tx_generation_weight`) yields weight 0 and silences actors. The phase-2 single-producer topology pins `stake: 1, tx-generation-weight: 1` on the producer.
5. **`rb-generation-probability` must be high enough that included blocks ≫ submitted txs**. With the default `0.05`, most submitted txs miss inclusion in 200-slot runs. The phase-2 protocol-base pins `1.0` for single-producer suites; multi-producer M4 suites should reason about this explicitly.
6. **The metrics collector's "representative node"** is whichever node sends the first `PricingTick`. In single-producer setups this is deterministic. In multi-producer setups, the first node to tick wins — which depends on tokio task scheduling order, **not** the simulator's RNG. This is fine for honest single-producer runs but the collector should be revisited in M4 multi-producer flows.

## Test infra

- `sim::tests::m3_actors::ActorDriver` mirrors M2's `TwoLaneDriver` with an `actors:` profile pre-populated. Use it for any M4 single-node test that needs actor-driven demand.
- `sim::tests::m2_two_lane`'s test helpers (`test_partition_trigger`, `test_eb_endorsement_valid`) carry forward unchanged.
- `experiment-suite run <suite.yaml> --resume` is the integration-test surface for end-to-end runs; combined with `experiment-suite status` it's the loop M4 should use to verify suite re-entry.

## Recommended order of work for M4

1. **`phase-2-rb-scarcity.yaml`** — author a protocol-overlay YAML reducing `rb_body_max_size_bytes` (and possibly `eb_referenced_txs_max_size_bytes`); reuse the existing two-lane pricing configs. Compare metrics across normal vs reduced RB capacity. Document in the suite README how this restates the previous tiered-backend RB-scarcity question.
2. **`phase-2-urgency-inversion.yaml`** — author a new `mis_priced_actors.yaml` demand profile pairing high-urgency components with `MaxFeePolicy::ScaledOverLaneQuote{1, 1}` (or another low multiplier). Run against the same priority-dynamic pricing as the moderate suite; compare welfare per-component. Document the framing.
3. **Write one suite README per new suite** explaining the previous-vs-new framing relationship.
4. **Run + verify** that exit criteria 7 are met for all 7 suites total. If a question doesn't translate cleanly, drop it with rationale.

## Hard rules — restated

These rules held throughout M3 and remain in force for M4:

1. **No code, configs, types, or schemas from `pricing-sim-base`.** Observe it as prior art only.
2. **No f64 in simulation-affecting state.** M3's lane-choice and `MaxFeePolicy` flow f64 through bit-deterministic `libm::pow` + `libm::round` into `i128` lovelace before any `>` comparison. M4's new attacker/mis-priced-actor configs must preserve this discipline.
3. **Suite reframing risk** (plan line 327): if a phase-2 experimental question can't be expressed cleanly with the new schema, drop it with a written rationale rather than reintroducing tiered-backend baggage.
