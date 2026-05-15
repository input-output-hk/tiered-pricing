<!-- refreshed: 2026-05-15 -->
# Architecture

**Analysis Date:** 2026-05-15

## System Overview

```text
┌──────────────────────────────────────────────────────────────────────┐
│ experiment-suite (sim-cli)                                            │
│   `sim-rs/sim-cli/src/bin/experiment-suite/main.rs`                   │
│   subcommands: run | status | verify                                  │
└──────────────────────────────────────────────────────────────────────┘
              │                                       │
              ▼                                       ▼
┌──────────────────────────────────┐   ┌────────────────────────────────┐
│ runner (suite orchestration)      │   │ metrics collector              │
│   `sim-cli/src/runner.rs`         │   │   `sim-cli/src/metrics/`       │
│   - Manifest (job × seed status)  │   │   - ingest(&Event)             │
│   - parallel job dispatch         │   │   - TimeSeriesRow per slot     │
│   - per-thread current_thread     │   │   - ComponentSummary           │
│     tokio runtime                 │   │   - SHA256 of pricing stream   │
└────────┬─────────────────────────┘   └──────────────▲─────────────────┘
         │ run_job(suite, idx, seed)                  │
         │ builds SimConfiguration                    │ Event channel
         ▼                                            │ (mpsc::unbounded)
┌──────────────────────────────────────────────────────┴───────────────┐
│ Simulation (sim-core)  `sim-core/src/sim.rs`                          │
│   - NetworkWrapper::LinearLeios (linear-Leios only for phase-2)       │
│   - NodeListWrapper drives N `LinearLeiosNode`s                       │
│   - TransactionProducer + SlotWitness + ClockCoordinator              │
└──────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────────┐
│ LinearLeiosNode  `sim-core/src/sim/linear_leios.rs`                   │
│   per-node state:                                                     │
│   - Mempool (UTxO conflict tracking)                                  │
│   - MempoolGate (`sim/mempool_gate.rs`) — fee + byte-cap authority    │
│   - praos.blocks: BTreeMap<BlockId, RankingBlockView>                 │
│   - block_samples: BTreeMap<BlockId, Vec<PricedBlockSample>>          │
│   - Box<dyn PricingBackend> (pure-function policy carrier)            │
│   - NodeActorState (per-component RNGs + LatencyEstimator)            │
│                                                                       │
│ implements `ChainView`: ancestor, samples_in_block,                   │
│ derived_quote, window_aggregate                                       │
└──────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Pricing kernel (sim-core/src/tx_pricing/)                             │
│                                                                       │
│   trait PricingBackend (`mod.rs`):                                    │
│     compute_derived_quote(parent_quote, parent_aggregate,             │
│                           parent_samples, evicted_samples)            │
│       -> (PerLaneQuote, WindowAggregate)            [pure function]   │
│     samples_for_block, lane_validity_rule,                            │
│     lane_selection_order, cold_start_quote                            │
│                                                                       │
│   trait ChainView (`mod.rs`):    [seam — backend reads chain]         │
│     ancestor(from, k), samples_in_block, derived_quote,               │
│     window_aggregate                                                  │
│                                                                       │
│   Concrete backends:                                                  │
│     `single_lane.rs`  BaselinePricing, Eip1559Pricing                 │
│     `two_lane.rs`     TwoLanePricing × 4 TwoLaneVariant arms          │
│     `window.rs`       aggregate_from_chain, update_aggregate          │
└──────────────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

| Component | Responsibility | File |
|-----------|----------------|------|
| `experiment-suite` binary | CLI entrypoint; parse args; dispatch to runner. | `sim-rs/sim-cli/src/bin/experiment-suite/main.rs` |
| `Suite` | YAML deserialisation of (name, output-dir, seeds, default-{slots,topology,protocol,demand}, jobs[]). | `sim-rs/sim-cli/src/suite.rs` |
| `Manifest` + `Job(Entry,Status)` | Per-(job, seed) status (Pending/Running/Completed/Failed); resume contract; serde kebab-case. | `sim-rs/sim-cli/src/runner.rs` |
| `run_suite_with_run_id` | Worker-pool dispatch (`std::thread` per worker, per-thread `current_thread` tokio runtime — `Simulation` is `!Send`). | `sim-rs/sim-cli/src/runner.rs:182` |
| `run_job` | Compose `RawParameters` via `figment` layered YAML (embedded config.default.yaml → protocol → demand → pricing); build `SimConfiguration`; spawn drain task that ingests `Event`s into `MetricsCollector`; `simulation.run(token)`. | `sim-rs/sim-cli/src/runner.rs:807` |
| `verify_suite_with_run_id` | Re-run every `Completed` (job, seed); assert freshly-computed `pricing_event_stream_sha256` matches persisted on-disk value. | `sim-rs/sim-cli/src/runner.rs:600` |
| `Simulation` | Owns `ClockCoordinator`, `NetworkWrapper`, `NodeListWrapper`, `TransactionProducer`, `SlotWitness`, `Vec<Box<dyn Actor>>`. Variant-dispatches to linear-Leios for phase-2. | `sim-rs/sim-core/src/sim.rs` |
| `LinearLeiosNode` | Per-node protocol state machine: TX/RB/EB/Vote propagation, block production, mempool, pricing backend, actor model. Implements `ChainView`. | `sim-rs/sim-core/src/sim/linear_leios.rs` |
| `MempoolGate` | Sole byte-cap authority. Admission (`min_fee_b + quote × bytes ≤ max_fee_lovelace` AND byte cap), revalidation (evict on quote drift), inclusion charging. | `sim-rs/sim-core/src/sim/mempool_gate.rs` |
| `PricingBackend` trait | Pure-function policy: maps (parent state, samples) to (child `PerLaneQuote`, `WindowAggregate`). No `&mut self`. | `sim-rs/sim-core/src/tx_pricing/mod.rs:171` |
| `ChainView` trait | Read-only seam exposed to backends. Backend never sees simulator types except `BlockId`. | `sim-rs/sim-core/src/tx_pricing/mod.rs:139` |
| `BaselinePricing` | Flat `c = 1`; returns `min_fee_a` regardless of input. | `sim-rs/sim-core/src/tx_pricing/single_lane.rs:43` |
| `Eip1559Pricing` | Single-lane EIP-1559 step (integer/rational clamp formula); driven by `WindowAggregate` standard-lane sums. | `sim-rs/sim-core/src/tx_pricing/single_lane.rs:176` |
| `TwoLanePricing` | Two controllers + 4 `TwoLaneVariant` arms (RB-reserved × {priority-only, both-dynamic}; un-reserved × …); multiplier-floor invariant enforced on output. | `sim-rs/sim-core/src/tx_pricing/two_lane.rs:152` |
| `aggregate_from_chain` / `update_aggregate` | Pure-function capacity-weighted-window aggregation in `u128`. | `sim-rs/sim-core/src/tx_pricing/window.rs` |
| `ActorComponent` / `ActorProfile` | Weighted multi-component demand-side sampling: per-arrival `(bytes, value, half-life)`; lane choice + `max_fee_lovelace`. | `sim-rs/sim-core/src/tx_actors.rs` |
| `MetricsCollector` | f64 reporting layer over the integer event stream. Per-slot `TimeSeriesRow`, per-component `ComponentSummary`, run-level `RunSummary` with SHA256 of pricing stream. | `sim-rs/sim-cli/src/metrics/collector.rs` |
| `EventTracker` / `Event` enum | Emission API + tagged enum carried over the simulator-to-collector mpsc channel. | `sim-rs/sim-core/src/events.rs` |

## Pattern Overview

**Overall:** Event-driven discrete-event simulation with a pure-function pricing kernel layered on top.

**Key Characteristics:**

- **Chain-derived controller (Family B / spike 007).** Every `LinearRankingBlock` carries its own `derived_quote: PerLaneQuote` and `window_aggregate: WindowAggregate` as header-equivalent fields (`sim-core/src/model.rs:180`). These are pure functions of the parent block's chain-derived state plus samples emitted by canonical predecessors within the smoothing window. Sibling RBs from a slot battle compute identical `derived_quote` given identical inputs (`tx_pricing/two_lane.rs::sibling_rbs_produce_identical_derived_quote` test). Orphan blocks carry their own `derived_quote` which is discarded with the block — pricing-state contamination from slot-battle reorgs is impossible by construction (closes WR-1).
- **Stateless backends.** `BaselinePricing`, `Eip1559Pricing`, and `TwoLanePricing` are settings carriers. `PricingBackend::compute_derived_quote` takes the parent's state explicitly; the trait has no `&mut self` methods. The controller "state" lives on the canonical chain.
- **ChainView seam.** The backend reads the chain via the `ChainView` trait (`tx_pricing/mod.rs:139`) — four methods: `ancestor`, `samples_in_block`, `derived_quote`, `window_aggregate`. `LinearLeiosNode` implements `ChainView` (`sim/linear_leios.rs:2850`). The backend never sees simulator types directly except `BlockId`.
- **Integer / rational arithmetic in hot paths.** Admission, eviction, fee charging, the EIP-1559 step, mempool tracking, `max_fee_lovelace`, the multiplier-floor invariant, and actor lane choice are all `u64` / `u128` / `i128` or pinned-libm math (`libm::pow` + `libm::round` into `i128` lovelace). f64 is restricted to the metrics reporting layer.
- **Per-node mempool + per-node controller view.** Every node runs its own `Mempool` + `MempoolGate` + chain view. Multi-producer topologies have N independent mempool pipelines stitched together by gossip propagation. The canonical "chain" view per node is reads from `praos.blocks` + `block_samples`.

## Layers

**Pricing kernel (`sim-core/src/tx_pricing/`):**
- Purpose: pure-function policy for transaction pricing.
- Location: `sim-rs/sim-core/src/tx_pricing/`
- Contains: `PricingBackend` trait, `ChainView` trait, 3 backend impls, `WindowAggregate` helpers, `Lane` enum, `PricedBlockSample`/`BlockLaneBreakdown` carriers.
- Depends on: `sim-core::model::{PerLaneQuote, WindowAggregate, BlockId}`.
- Used by: `sim-core::sim::linear_leios` (block production, mempool revalidation, EB endorsement validation).

**Protocol simulator (`sim-core/src/sim/linear_leios.rs`):**
- Purpose: simulate the linear-Leios protocol with phase-2 pricing wired in.
- Contains: TX/RB/EB/Vote propagation, slot lottery, block production, EB validation, voting, mempool, gate, chain-derived quote computation, actor sampling driver.
- Depends on: `sim-core::tx_pricing` (the backend trait), `sim-core::sim::mempool_gate`, `sim-core::tx_actors`, `sim-core::network`, `sim-core::clock`, `sim-core::events`.
- Used by: `sim-core::sim::Simulation` (variant-dispatched at construction).

**Actor model (`sim-core/src/tx_actors.rs`):**
- Purpose: demand-side surface — multi-component weighted profiles generate txs with `posted_lane`, `max_fee_lovelace`, `urgency`, `value_lovelace`.
- Contains: `ActorComponent`, `ActorProfile`, `MaxFeePolicy`, `LanePolicy`, `lane_choice::pick`, `welfare` module, `LatencyEstimator`.
- Depends on: `sim-core::tx_pricing::Lane`, `sim-core::probability::FloatDistribution`, `rand_chacha`, `libm`.
- Used by: `LinearLeiosNode::run_actors_for_slot` (`sim/linear_leios.rs`).

**Suite runner (`sim-cli/src/runner.rs`, `sim-cli/src/suite.rs`):**
- Purpose: orchestrate (job × seed) Cartesian product over a suite; resumable manifest; parallel dispatch.
- Contains: `Suite`, `Job`, `JobOverrides`, `Manifest`, `JobEntry`, `JobStatus`, `run_suite_with_run_id`, `verify_suite_with_run_id`, `run_job`.
- Depends on: `sim-core::{config, events, sim, clock}`, `figment` (layered YAML), `chrono`, `tokio`, `serde_json`.
- Used by: `experiment-suite` binary (`sim-cli/src/bin/experiment-suite/main.rs`).

**Metrics layer (`sim-cli/src/metrics/`):**
- Purpose: ingest the integer event stream; produce `time_series.csv`, `metrics_comparison.txt`, `diagnostics.log`, `run_summary.json`, `pricing_event_stream.sha256`.
- Contains: `MetricsCollector`, `TimeSeriesRow`, `ComponentSummary`, `RunSummary`, `comparison`, `diagnostics`, `time_series` writers.
- Depends on: `sim-core::{events::Event, tx_pricing::Lane, tx_actors::welfare, model::TransactionId}`, `sha2`, `serde`.
- Used by: `sim-cli::runner::run_job` (drain task on the event-channel receive end).

## Data Flow

### Block production → chain-derived quote → mempool revalidation

1. **Slot tick.** `LinearLeiosNode::handle_new_slot` (`sim/linear_leios.rs:578`) runs `emit_pricing_tick`, `run_actors_for_slot`, `try_generate_rb`.
2. **Actor sampling.** `run_actors_for_slot` walks the node's `NodeActorState.components`, samples arrivals via `ActorComponent::sample_arrivals` (`tx_actors.rs:584`), picks `posted_lane` via `lane_choice::pick` (`tx_actors.rs:305`), computes `max_fee_lovelace` via the component's `MaxFeePolicy`, then calls `generate_tx`.
3. **Admission.** `try_add_tx_to_mempool` (`sim/linear_leios.rs:1851`) reads `current_chain_tip_quote(posted_lane)` and asks the gate via `MempoolGate::try_admit` (`mempool_gate.rs:152`). On success the tx enters both the conflict-tracking `Mempool` and the gate's resident set.
4. **RB lottery + production.** `try_generate_rb` (`sim/linear_leios.rs:741`) wins the lottery, decides whether to endorse a parent EB (`eb_endorsement_valid` walks tx-by-tx against `current_chain_tip_quote`; refuses to endorse on staleness), packs the RB body (`sample_from_mempool_lane_aware` honours `LaneValidityRule::PriorityOnly` for RB-reserved variants), and packs an EB body (`select_eb_with_partition` decides the two-trigger partition activation and stores it on the EB).
5. **Chain-derived quote computation.** `compute_chain_derived_quote_for_child_of(parent)` (`sim/linear_leios.rs:2179`) reads `parent.derived_quote` and `parent.window_aggregate` via the local `ChainView` impl, fetches `parent_samples` (samples_in_block), and looks back `window_length` ancestors for `evicted_samples`. Calls `pricing.compute_derived_quote(...)`. Result is stored on the new `RankingBlock` as `derived_quote` + `window_aggregate`.
6. **Inclusion charging.** `charge_inclusions_at` (`sim/linear_leios.rs:2104`) computes `actual_fee_lovelace = min_fee_b + quote(served_lane) × bytes` and `refund_lovelace = max_fee − actual_fee` at the new block's `derived_quote`, emits `TXIncluded` events via the `EventTracker`.
7. **Publish + cache + prune.** `publish_rb` (`sim/linear_leios.rs:1043`) inserts the RB into `praos.blocks`, caches `samples_for_rb(&rb)` in `block_samples`, prunes entries older than `2 × window_length` slots behind the tip (`prune_block_samples`).
8. **Mempool revalidation.** `revalidate_against_new_tip` (`sim/linear_leios.rs:2353`) reads the new tip's lane quotes via `current_chain_tip_quote` and asks `gate.revalidate(|lane| q)`. Evicted txs trigger `TXEvictedQuoteDrift` events; conflict-tracking mempool's `remove_conflicting_txs` promotes any queue-waiting txs that no longer conflict.

### Suite run (orchestrator)

1. **Suite load.** `Suite::load` parses the YAML (`sim-cli/src/suite.rs:52`).
2. **Manifest init.** `Manifest::load_or_init` builds or loads `<output_dir>/manifest.json`; Running → Pending recovery for kill-mid-job (`sim-cli/src/runner.rs:77`).
3. **Pending snapshot.** `suite.job_seed_pairs()` produces a deterministic dispatch order; (job, seed) pairs already `Completed` are filtered out.
4. **Worker pool.** `parallelism` threads (default `min(available_parallelism(), 8)`) each own a `current_thread` tokio runtime; pull (job, seed) from an `std::sync::mpsc` channel; `runtime.block_on(run_job(...))`.
5. **run_job.** Composes `RawParameters` from layered YAML (config.default.yaml → protocol → demand → pricing) via `figment`; builds `SimConfiguration`; builds `MetricsCollector`; spawns a drain task that ingests events from a tokio `mpsc::unbounded_channel`; runs the `Simulation` to completion; `collector.finalise()` returns `(rows, summary)`; persists `run_summary.json`, `time_series.csv`, `diagnostics.log`, `pricing_event_stream.sha256`.
6. **Manifest transitions.** Pending → Running → Completed | Failed, each rewriting `manifest.json` under a `std::sync::Mutex` (also rewrites `metrics_comparison.txt` in the same critical section).
7. **Aggregate-and-continue.** A failed (job, seed) ends `Failed` in the manifest; siblings are not cancelled; final exit non-zero if any failed.

### Verify subcommand

1. Pre-flight: walk completed jobs, read each `pricing_event_stream.sha256`, reject malformed (length ≠ 64 or non-hex).
2. Re-run each (job, seed) on a worker; compare freshly-computed hash to the stored value; log mismatches; final non-zero exit if any mismatch.

**State Management:**
- Per-node mutable state stays inside `LinearLeiosNode` (mempool, gate, praos.blocks, block_samples, actor_state).
- Controller "state" lives **on the canonical chain** — every RB carries its own `derived_quote` + `window_aggregate`. No node-local accumulator.
- Suite state (manifest) lives on disk, mutex-protected for the parallel-run case.

## Key Abstractions

**`Lane`** (`sim-core/src/tx_pricing/mod.rs:36`):
- Two-variant enum (`Standard`, `Priority`) on every `Transaction` (`posted_lane`) and every inclusion event (`served_lane`). Single-lane mechanisms collapse both to `Standard`. No tier vocabulary anywhere.

**`PerLaneQuote`** (`sim-core/src/model.rs:101`):
- `{ standard: u64, priority: u64 }`. `PerLaneQuote::flat(q)` for single-lane mechanisms. `PerLaneQuote::get(lane)` for lane-keyed lookup.

**`WindowAggregate`** (`sim-core/src/model.rs:133`):
- `{ standard_sum_bytes, standard_sum_capacity, priority_sum_bytes, priority_sum_capacity: u128, blocks_in_window: u32 }`. `aggregate_util(lane) -> (u128, u128)` returns a rational; `(0, 1)` when no signal.

**`PricedBlockSample`** (`sim-core/src/tx_pricing/mod.rs:70`):
- `{ block_kind: BlockKind, controller_lane: Lane, relevant_bytes: u64, relevant_capacity: u64 }`. One sample per controller a block feeds. RB-reserved priority controller takes `min(priority_paying_bytes, max_block_size)` for its EB sample (cap-on-priority-bytes rule).

**`PricingBackend`** (`sim-core/src/tx_pricing/mod.rs:171`):
- Trait with one mandatory method: `compute_derived_quote(parent_quote, parent_aggregate, parent_samples, evicted_samples) -> (PerLaneQuote, WindowAggregate)`. Plus `effective_window_length`, `cold_start_quote`, `lane_validity_rule`, `lane_selection_order`, `min_priority_premium_multiplier`, `samples_for_block`.

**`ChainView`** (`sim-core/src/tx_pricing/mod.rs:139`):
- Trait exposed to backends: `ancestor`, `samples_in_block`, `derived_quote`, `window_aggregate`. Implemented by `LinearLeiosNode`. Backends never mutate the chain.

**`MempoolGate`** (`sim-core/src/sim/mempool_gate.rs:95`):
- Sole byte-cap authority. Methods: `try_admit`, `revalidate(quote_for_lane)`, `on_inclusion`, `remove_silent`, `fee_at`. Per-lane byte counts. Reject-only on full mempool — no eviction of valid txs.

**`TwoLaneVariant`** (`sim-core/src/tx_pricing/two_lane.rs:33`):
- Four arms: `RbReservedPriorityOnly`, `RbReservedBothDynamic`, `UnreservedPriorityOnly`, `UnreservedBothDynamic`. Encodes (RB validity rule, standard-controller dynamism) crossings.

**`ActorProfile` / `ActorComponent`** (`sim-core/src/tx_actors.rs:551`, `707`):
- Weighted multi-component demand model. Each component has its own RNG; sample (bytes, value, half-life) per arrival; lane choice via `lane_choice::pick` maximises `expected_utility(lane)` through `libm::pow` + `libm::round` into `i128`.

## Entry Points

**`experiment-suite` binary:**
- Location: `sim-rs/sim-cli/src/bin/experiment-suite/main.rs`
- Triggers: developer / CI invocation.
- Responsibilities: parse subcommand (Run/Status/Verify), resolve `--parallelism` (default `min(available_parallelism(), 8)`), call `run_suite_with_run_id` / `verify_suite_with_run_id` / `print_status`.

**`sim-cli` binary (legacy single-run driver):**
- Location: `sim-rs/sim-cli/src/main.rs`
- Triggers: legacy invocation for one-shot runs (predates the suite runner).
- Responsibilities: single-config simulator invocation; not the phase-2 driver.

**Workspace test suite:**
- `cargo test --workspace`: runs every non-`#[ignore]`'d test, including `tx_pricing` unit tests, `mempool_gate` unit tests, `linear_leios::mempool_tests`, M1/M2/M3 deterministic scenario tests under `sim-core/src/sim/tests/`, and `sim-cli/tests/parallel_runner.rs`.
- `cargo test --release -- --ignored determinism`: runs the M5 suite-level golden tests (`sim-rs/sim-cli/tests/determinism.rs`) — one canonical (job, seed=1) baseline per phase-2 suite, hash-asserted against `.goldens/<suite>.sha256`.

## Architectural Constraints

- **Threading model:** Each (job, seed) pair runs on a dedicated `std::thread` with its own `tokio::runtime::Builder::new_current_thread()` runtime. `Simulation` contains `Box<dyn Actor>` which is `!Send` (`sim-core/src/sim.rs:86`), so a multi-thread tokio runtime cannot drive it. Within a job, the simulation itself runs on the worker's single thread using tokio's `JoinSet` for cooperative scheduling.
- **Determinism scope:** **Intra-architecture.** The repo's pinned hashes (M2/M3 unit-test constants, M5 suite goldens) reproduce bit-identically on the same arch. Cross-arch CI verification is not yet built; documented as deferred infrastructure work.
- **Per-(job, seed) determinism under parallelism:** Suite-level goldens and `verify` both treat each (job, seed) as the determinism unit. Parallelism changes only wall-clock interleaving, not seeds/inputs/event streams. The manifest's `BTreeMap`-keyed-by-(job_name, seed_string) layout gives deterministic on-disk order regardless of completion order. Per-(job, seed) artefact paths `<output_dir>/<job_name>/<seed>/` are unique so no two parallel jobs touch the same file.
- **No node-local mutable controller state.** WR-1 (slot-battle pricing-state contamination) is resolved by construction: every RB carries its own `derived_quote` as a pure function of its own ancestors. Orphan blocks from slot battles carry their own quotes which are discarded with the block.
- **Mempool gate is the sole byte-cap authority.** `MempoolGate.config.max_total_size_bytes == Mempool.max_size_bytes` is invariant. `LinearLeiosNode::new` enforces this with a `debug_assert_eq!` (`sim/linear_leios.rs:479`). Any drift would silently reopen the queue-bypass path described in `Mempool::try_insert`.
- **`block_samples` cache pruning:** Bounded at `2 × window_length` slots behind the chain tip (`prune_block_samples`, `sim/linear_leios.rs:2325`). Under Cardano's k=2160 finality this is trivially within the chain-stability horizon.
- **Single producer per slot for RB lottery, but multiple producers per slot possible (slot battles).** Resolved via VRF tiebreaker in `finish_validating_rb_header` (lower VRF wins). Late slot battles where both bodies fully validate are observable via `RunSummary.slot_battles_count` / `orphaned_pricing_samples` — these are UPPER BOUNDS on pricing-state contamination at the representative node; under chain-derivation the bound is now 0 by construction (sibling RBs compute identical `derived_quote`).
- **Global state:** Module-level globals are absent in phase-2 hot paths. The only cross-job shared state during a suite is `manifest.json`, guarded by a single `std::sync::Mutex`.

## Anti-Patterns

### Reading the parent RB's stored `derived_quote` for current charging / admission

**What happens:** Reading `latest_rb.derived_quote` directly when admitting a tx or charging RB-body inclusions returns the quote that was stepped from the *parent's* samples — one controller step behind the post-tip state.

**Why it's wrong:** Legacy accumulator semantics had `pricing.current_quote()` reflect the post-`apply_priced_block(tip)` state. Under chain-derivation, the stored `rb.derived_quote` is the quote computed FOR that block (i.e. from its parent), not the quote that should govern descendants. Reading it directly lags admission/charging by one step.

**Do this instead:** Use `current_chain_tip_quote(lane)` (`sim/linear_leios.rs:2278`) which computes `compute_chain_derived_quote_for_child_of(tip)` — the hypothetical child of the tip — and reads `next_quote.get(lane)`. RB-body inclusion charging in `try_generate_rb` uses the NEW block's own `rb.derived_quote` (the post-step value computed for that block at production), which by symmetry is the same value as `current_chain_tip_quote` against the parent.

### Putting f64 in a simulation-affecting code path

**What happens:** Using `f64` for `quote_per_byte`, `max_fee_lovelace`, the controller's `c` coefficient, fee arithmetic, or any value compared in admission/eviction/inclusion decisions.

**Why it's wrong:** Cross-arch determinism (when CI is built) requires bit-identical simulation outcomes. `f64` arithmetic is not bit-stable across architectures except for IEEE-754 §5.4.1 ops; hardware `f64::sqrt` is NOT mandated correctly-rounded. The pricing event-stream golden hashes (M2/M3 unit-test constants, M5 suite-level goldens) cover exactly `TXIncluded` and `TXEvictedQuoteDrift` events — any f64 entry into the hot path flips them.

**Do this instead:** Use `u64` / `u128` / `i128`. The EIP-1559 step runs in `u128` rationals (`tx_pricing/single_lane.rs::compute_eip1559_step`). The multiplier-floor invariant runs `u128 → u64` with ceiling-division. Actor lane-choice math uses `libm::pow` + `libm::round` into `i128` lovelace before any comparison. The only legitimate f64 is in the metrics reporting layer (`sim-cli/src/metrics/collector.rs`) and never feeds back into the simulator.

### Mutating a backend's controller state

**What happens:** Adding `&mut self` methods to `PricingBackend` or holding controller windows / coefficient state on the backend.

**Why it's wrong:** The chain-derived design intentionally has zero node-local mutable controller state. `PricingBackend::compute_derived_quote` is `(&self, ...) -> (PerLaneQuote, WindowAggregate)`. Mutating backend state on RB publish would re-introduce the WR-1 contamination class — orphan blocks would update controller state that survives reorg.

**Do this instead:** Compute the new state purely from inputs and return it. Store the result on the new `LinearRankingBlock` as a header field. Read via `ChainView`.

### Stacking RB-reduced protocol overlays

**What happens:** Trying to layer `protocol-base.yaml` + `protocol-rb-reduced-half.yaml` via the runner's `JobOverrides` mechanism, expecting partial override.

**Why it's wrong:** `JobOverrides::protocol` is a full replacement — `runner::run_job` picks `overrides.protocol` OR `suite.default_protocol`, never both. The four `protocol-rb-reduced-{half,third,quarter}.yaml` files duplicate everything from `protocol-base.yaml` and override only `rb-body-max-size-bytes`.

**Do this instead:** When adding a knob to `protocol-base.yaml`, propagate it to all three RB-reduced overlays manually. The "stacked overlay" extension (`protocol_overlay: Vec<PathBuf>`) is deferred work.

### Mixing serde casing conventions in new types

**What happens:** Adding a new type to `runner.rs` / `suite.rs` without `#[serde(rename_all = "kebab-case")]`, or adding `rename_all = "kebab-case"` to `RunSummary`.

**Why it's wrong:** `Suite` / `Manifest` / `JobEntry` use kebab-case; `RunSummary` uses snake_case (no `rename_all`). Both shapes coexist on disk in persisted artefacts (`manifest.json`, `run_summary.json`). Standardising would invalidate every persisted manifest under `sim-rs/output/`, forcing re-runs of all 72 (job, seed) pairs.

**Do this instead:** Match the surrounding type's existing convention. Future schema additions to `Manifest` get kebab-case; future additions to `RunSummary` get snake_case.

## Error Handling

**Strategy:** `anyhow::Result` at the suite-runner and config-build boundaries; typed `Err` variants for fine-grained admission rejection (`AdmissionRejection` in `mempool_gate.rs:38`) where the caller branches on cause.

**Patterns:**

- **`anyhow::Result` at API boundaries.** `run_suite_with_run_id`, `verify_suite_with_run_id`, `run_job`, `Suite::load`, `MempoolGate::*` admissions, `Eip1559Settings::validate`, `TwoLaneSettings::validate` all return `anyhow::Result`.
- **Aggregate-and-continue at suite scope.** A failed (job, seed) lands `Failed` in the manifest; siblings are not cancelled; final exit aggregates all failures into one `anyhow::anyhow!` chain (`runner.rs:438`).
- **Refuse-to-act, not silently degrade.** `eb_endorsement_valid` (`sim/linear_leios.rs:950`) refuses to endorse an EB with a stale tx rather than mutating the EB body. `MempoolGate::try_admit` returns `Err(AdmissionRejection::*)` rather than silently truncating fees. `Manifest::load_or_init` resets Running → Pending on reload rather than assuming the previous worker completed.
- **Saturating arithmetic for u128 → u64 fallbacks.** Controller intermediates saturate at `u64::MAX` rather than panicking. `Eip1559Settings::validate` enforces a `window_length × target_num × max_change_denominator ≤ 2^23` bound at construction so realistic settings never reach the saturation path.
- **Defensive verify pre-flight.** A corrupt or hand-edited `pricing_event_stream.sha256` (empty string, non-hex, wrong length) makes `verify_suite_with_run_id` bail before any work runs (`runner.rs:660`) — otherwise an empty stored hash would silently match an empty freshly-computed hash.

## Cross-Cutting Concerns

**Logging:** `tracing` crate. Per-job INFO-level lines at `[N/total] run: <job> seed=<s>`, `... done: ...`, `... skip (completed): ...`. Diagnostics layer (`sim-cli/src/metrics/diagnostics.rs`) emits `DiagnosticNote { level, message }` to `diagnostics.log`.

**Validation:** Config validation at YAML-load time via `Eip1559Settings::validate` / `TwoLaneSettings::validate` (overflow bounds, non-zero denominators, multiplier-floor ≥ 1, ratio caps). `SimConfiguration::build` calls validation as part of parameter composition.

**Authentication:** Not applicable (offline research simulator).

**Determinism:** Three-layer regime (CLAUDE.md "Determinism scope"):
1. Unit-test goldens in `sim-core/src/sim/tests/m2_two_lane.rs` and `m3_actors.rs` with pinned source constants.
2. `experiment-suite verify <suite.yaml>` — re-runs every Completed (job, seed) and asserts fresh hash matches persisted hash.
3. Suite-level goldens in `sim-rs/parameters/phase-2-sweep/suites/.goldens/<suite>.sha256` (one per suite), asserted by `sim-rs/sim-cli/tests/determinism.rs` (slow-by-default, `#[ignore]`'d, run via `cargo test --release -- --ignored determinism`).

---

*Architecture analysis: 2026-05-15*
