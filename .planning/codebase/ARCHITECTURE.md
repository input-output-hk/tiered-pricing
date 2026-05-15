<!-- refreshed: 2026-05-13 -->
# Architecture

**Analysis Date:** 2026-05-13

## System Overview

```text
┌──────────────────────────────────────────────────────────────────────┐
│                  experiment-suite (CLI / driver layer)               │
│  `sim-rs/sim-cli/src/bin/experiment-suite/main.rs`                   │
│  subcommands: run | status | verify                                  │
└────────────────────────────┬─────────────────────────────────────────┘
                             │ loads Suite YAML, expands (job × seed),
                             │ composes RawParameters from
                             │   protocol + topology + demand + pricing
                             ▼
┌──────────────────────────────────────────────────────────────────────┐
│                  Suite runner + manifest (resumable)                 │
│  `sim-rs/sim-cli/src/runner.rs`                                      │
│  `sim-rs/sim-cli/src/suite.rs`                                       │
│  Persists `manifest.json` + per-(job, seed) artefacts.               │
└─────────────┬────────────────────────────────────┬───────────────────┘
              │ run_job(suite, job_idx, seed)      │ MetricsCollector
              ▼                                    ▼
┌────────────────────────────────────┐  ┌─────────────────────────────┐
│  Simulation                        │  │ Welfare metrics layer       │
│  `sim-core/src/sim.rs`             │  │ `sim-cli/src/metrics/`      │
│  Builds clock, network, nodes,     │  │ collector / time_series /   │
│  TxProducer, SlotWitness; routes   │  │ diagnostics / comparison    │
│  the LeiosVariant to the right     │◀─┤ (consumes Event stream,     │
│  NodeImpl.                         │  │  f64 reporting only)        │
└─────────────┬──────────────────────┘  └─────────────────────────────┘
              │
              ▼
┌──────────────────────────────────────────────────────────────────────┐
│  LinearLeiosNode  (the only phase-2 protocol)                        │
│  `sim-core/src/sim/linear_leios.rs`                                  │
│  Owns: per-node Mempool + MempoolGate, PricingBackend instance,      │
│        RB/EB packing, vote/endorse, ledger, slot/lottery handling.   │
└──┬────────────────┬────────────────┬───────────────┬─────────────────┘
   │ admission /    │ block-build    │ post-block    │ event emission
   │ eviction /     │ uses           │ samples       │ via EventTracker
   │ inclusion      │ current_quote  │               │
   ▼                ▼                ▼               ▼
┌─────────────────┐ ┌────────────────────────┐ ┌──────────────────────┐
│ MempoolGate     │ │ PricingBackend         │ │ Event enum           │
│ `sim/mempool_   │ │ (trait object)         │ │ `sim-core/src/       │
│  gate.rs`       │ │ `tx_pricing/mod.rs`    │ │  events.rs`          │
│ byte-cap, fee   │ │ + Baseline /           │ │ → MetricsCollector   │
│ admission,      │ │   Eip1559 /            │ │   + EventMonitor     │
│ quote-drift     │ │   TwoLane variants     │ │                      │
│ revalidation,   │ │ + CapacityWeighted     │ │                      │
│ inclusion chrg  │ │   Window               │ │                      │
└─────────────────┘ └────────────────────────┘ └──────────────────────┘
   ▲
   │ submits tx (computed max_fee_lovelace, posted_lane)
┌──┴────────────────────────────────────────────────────────────────────┐
│  Actor model (demand-side)                                            │
│  `sim-core/src/tx_actors.rs`                                          │
│  ActorComponent → samples (bytes, value, half-life); MaxFeePolicy →   │
│  max_fee_lovelace; LanePolicy::UtilityMaximising → posted_lane via    │
│  libm::pow + libm::round + i128 lovelace; LatencyEstimator per (cmp,  │
│  lane) EMA.                                                           │
└───────────────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

| Component | Responsibility | File |
|-----------|----------------|------|
| `experiment-suite` binary | CLI entry: `run`/`status`/`verify` for a suite YAML | `sim-rs/sim-cli/src/bin/experiment-suite/main.rs` |
| `Suite` / `Job` / `JobOverrides` | Suite YAML schema (kebab-case serde) | `sim-rs/sim-cli/src/suite.rs` |
| `Manifest` + `run_suite` / `run_job` / `verify_suite` | Resumable per-(job, seed) execution; persists `manifest.json` + artefacts | `sim-rs/sim-cli/src/runner.rs` |
| `MetricsCollector` | Consumes `Event` stream → `RunSummary`, `time_series.csv`, hashes | `sim-rs/sim-cli/src/metrics/collector.rs` |
| `comparison::write_suite` | Per-suite `metrics_comparison.txt` aggregator | `sim-rs/sim-cli/src/metrics/comparison.rs` |
| `Simulation` | Builds network + nodes + tx producer; runs to cancellation token | `sim-rs/sim-core/src/sim.rs` |
| `NodeDriver<N>` | Per-node event loop: slot / message / CPU task / timed-event handling | `sim-rs/sim-core/src/sim/driver.rs` |
| `LinearLeiosNode` (`NodeImpl`) | Phase-2 protocol logic: RB/EB build, vote, endorse, mempool, pricing | `sim-rs/sim-core/src/sim/linear_leios.rs` |
| `Mempool` (linear-Leios internal) | UTxO/conflict tracking + selection ordering | `sim-rs/sim-core/src/sim/linear_leios.rs` (`struct Mempool`) |
| `MempoolGate` | Byte-cap + fee admission + quote-drift revalidation + inclusion charging | `sim-rs/sim-core/src/sim/mempool_gate.rs` |
| `PricingBackend` trait | Policy-only quote/update/sample/validity-rule API | `sim-rs/sim-core/src/tx_pricing/mod.rs` |
| `BaselinePricing` / `Eip1559Pricing` | Single-lane backends (`c=1`, dynamic) | `sim-rs/sim-core/src/tx_pricing/single_lane.rs` |
| `TwoLanePricing` + `TwoLaneVariant` | Two-lane backends; 4 spec variants | `sim-rs/sim-core/src/tx_pricing/two_lane.rs` |
| `CapacityWeightedWindow` | `Σ bytes / Σ capacity` rolling window (u128 ring) | `sim-rs/sim-core/src/tx_pricing/window.rs` |
| `ActorComponent` / `ActorProfile` | Demand-side weighted multi-component sampling | `sim-rs/sim-core/src/tx_actors.rs` |
| `MaxFeePolicy` / `LanePolicy` / `LatencyEstimator` / `welfare` | Per-tx max-fee, lane choice, latency EMA, reporting formulas | `sim-rs/sim-core/src/tx_actors.rs` |
| `Event` + `EventTracker` | Tagged event stream emitted by the simulator | `sim-rs/sim-core/src/events.rs` |
| `Network<MiniProtocol, Msg>` | netsim-async bandwidth/latency overlay | `sim-rs/sim-core/src/network/` |
| `Clock` + `ClockCoordinator` + `SlotWitness` | Virtual time, slot-tick fan-out | `sim-rs/sim-core/src/clock/` + `sim-rs/sim-core/src/sim/slot.rs` |
| `TransactionProducer` | Per-actor-profile arrival sampler (Poisson phases) | `sim-rs/sim-core/src/sim/tx.rs` |
| `lottery` | VRF-stake probabilities for RB/vote sortition | `sim-rs/sim-core/src/sim/lottery.rs` |

## Pattern Overview

**Overall:** Discrete-event protocol simulator with policy-trait pricing kernel and event-driven welfare reporting.

**Key Characteristics:**
- Async tokio runtime, one task per node via `NodeDriver`; `Simulation::run` joins them under a `CancellationToken`.
- Event sourcing: every simulator effect emits a typed `Event` via `EventTracker`; the metrics layer is a pure consumer.
- Trait-object pricing kernel: `Box<dyn PricingBackend>` lives inside each `LinearLeiosNode`; the simulator never depends on a concrete backend type.
- Selection / packing live in the simulator block builder, not in the backend. The backend answers `current_quote(lane)`, `lane_validity_rule`, `lane_selection_order`, and after-the-fact `samples_for_block` / `update_after_block`.
- Determinism is an architectural property, not a test concern: all simulation-affecting state is `u64`/`u128`/integer-rational; f64 is allowed only in metrics/reporting.
- Resumable batch runner: `Manifest` is persisted JSON keyed by `(job_name, seed)` and survives interruption; `Running` entries are demoted to `Pending` on reload.

## Layers

**CLI / suite-runner layer:**
- Purpose: Translate a YAML suite into N independent simulator runs; persist artefacts; expose `run`/`status`/`verify`.
- Location: `sim-rs/sim-cli/src/bin/experiment-suite/`, `sim-rs/sim-cli/src/runner.rs`, `sim-rs/sim-cli/src/suite.rs`
- Contains: clap argv parsing, `Suite` schema, `Manifest` state machine, `figment` config composition, per-job tokio runtime.
- Depends on: `sim-core` (for `SimConfiguration`, `Simulation`, `Event`), `sim-cli::metrics`.
- Used by: shell scripts under `sim-rs/scripts/` (`run-parallel-suites.sh`, smoke runners).

**Metrics layer (f64 reporting):**
- Purpose: Consume `Event` stream into per-slot time-series, welfare summary, comparison text, diagnostics log.
- Location: `sim-rs/sim-cli/src/metrics/`
- Contains: `MetricsCollector`, `TimeSeriesRow`, `ComponentSummary`, `RunSummary`, SHA-256 hashing over `TXIncluded` + `TXEvictedQuoteDrift`.
- Depends on: `sim-core` event/types only.
- Used by: `runner::run_job` (in-band) and re-loaded from disk for cross-suite comparison.

**Simulation orchestration layer:**
- Purpose: Compose nodes + network + producer + clock; pump events; route to per-variant `NodeImpl`.
- Location: `sim-rs/sim-core/src/sim.rs`, `sim-rs/sim-core/src/sim/driver.rs`, `sim-rs/sim-core/src/sim/slot.rs`, `sim-rs/sim-core/src/sim/cpu.rs`
- Contains: `Simulation`, `NetworkWrapper`/`NodeListWrapper` (variant dispatch), `NodeDriver<N>`, `SimCpuTask`, `EventResult`.
- Depends on: `network`, `clock`, `events`, all concrete `NodeImpl`s.
- Used by: `runner::run_job`, `sim-cli/src/main.rs` (legacy single-run).

**Protocol layer (`NodeImpl` per LeiosVariant):**
- Purpose: Encode the protocol's state machine. Phase-2 only uses `LinearLeiosNode`; legacy variants (`leios.rs`, `stracciatella.rs`) coexist but are not exercised by the phase-2 suites.
- Location: `sim-rs/sim-core/src/sim/linear_leios.rs` (phase-2), `sim-rs/sim-core/src/sim/leios.rs`, `sim-rs/sim-core/src/sim/stracciatella.rs` (legacy)
- Contains: `Message` enum, `CpuTask` enum, `TimedEvent` enum, per-node state (`NodePraosState`, `NodeLeiosState`, `NodeActorState`, `LedgerState`), RB/EB build & validate, vote sortition, ledger commit.
- Depends on: `tx_pricing`, `mempool_gate`, `tx_actors`, `lottery`, `model`.
- Used by: `Simulation::init` via variant dispatch.

**Pricing kernel (`tx_pricing/`):**
- Purpose: Policy-only fee controller(s) + utilisation window.
- Location: `sim-rs/sim-core/src/tx_pricing/`
- Contains: `PricingBackend` trait, `Lane`, `BlockKind`, `PricedBlockSample`, `BlockLaneBreakdown`, `Multiplier`, `BaselinePricing`, `Eip1559Pricing`, `TwoLanePricing` (+ 4 variants), `CapacityWeightedWindow`.
- Depends on: nothing simulator-side; pure data and arithmetic.
- Used by: `LinearLeiosNode` and the M1-M3 deterministic tests under `sim-core/src/sim/tests/`.

**Demand model (`tx_actors.rs`):**
- Purpose: Per-component arrival sampling + lane choice + max-fee policy.
- Location: `sim-rs/sim-core/src/tx_actors.rs`
- Contains: `ActorComponent`, `ActorProfile`, `MaxFeePolicy::{ScaledOverLaneQuote, VolatilityAware}`, `LanePolicy::UtilityMaximising`, `LatencyEstimator`, `welfare::{retained_value, net_utility, retained_value_ratio}`.
- Depends on: `tx_pricing::Lane` (read-only), `probability::FloatDistribution`.
- Used by: `TransactionProducer` (in `sim/tx.rs`) and `LinearLeiosNode` (lane choice at submission).

**Mempool admission layer (`mempool_gate.rs`):**
- Purpose: Sole byte-cap authority; admission, quote-drift revalidation, inclusion charging.
- Location: `sim-rs/sim-core/src/sim/mempool_gate.rs`
- Contains: `MempoolGate`, `AdmissionRejection`, `EvictionRecord`, `InclusionCharge`, per-lane byte counters.
- Depends on: `model::Transaction`, `tx_pricing::Lane`, `config::MempoolGateConfig`.
- Used by: `LinearLeiosNode` only (cooperates with the internal `Mempool` struct which handles UTxO conflicts).

**Network + clock primitives:**
- Purpose: Per-edge bandwidth/latency, virtual clock, slot fan-out.
- Location: `sim-rs/sim-core/src/network/`, `sim-rs/sim-core/src/clock/`, `sim-rs/sim-core/src/sim/slot.rs`
- Contains: `Network`/`NetworkSink`/`NetworkSource` over `netsim-async`, `ClockCoordinator`, `Timestamp`, `SlotWitness`.
- Depends on: `netsim-async`, `tokio`.
- Used by: `Simulation`, `NodeDriver`.

**Config layer (`config.rs`):**
- Purpose: Deserialise all YAML/TOML configs (kebab-case) into `Raw*` and validate into `SimConfiguration`.
- Location: `sim-rs/sim-core/src/config.rs`
- Contains: `RawParameters`, `RawTopology`, `Topology`, `SimConfiguration`, `NodeConfiguration`, `PricingConfig` (+ variants), `MempoolGateConfig`, `TransactionConfig`, `DistributionConfig`.
- Depends on: `tx_pricing` and `tx_actors` for inner types.
- Used by: `sim-cli/runner.rs`, `sim-cli/main.rs`.

## Data Flow

### Suite-run flow (top-level)

1. Operator invokes `experiment-suite run <suite.yaml>` (`sim-rs/sim-cli/src/bin/experiment-suite/main.rs:70`).
2. `runner::run_suite_with_run_id` (`sim-rs/sim-cli/src/runner.rs:172`) loads `Suite` YAML, applies optional `run_id` suffix, loads-or-initialises `manifest.json`.
3. For each `(job_idx, seed)` from `Suite::job_seed_pairs`: persist `Running` → invoke `run_job` on a fresh tokio current-thread runtime.
4. `run_job` composes a `RawParameters` via `figment` (`sim-rs/sim-cli/src/runner.rs`) layering `protocol` + `topology` + `demand` + per-job `pricing` + per-job `JobOverrides`.
5. Builds `SimConfiguration`, instantiates `Simulation::new`, attaches a `MetricsCollector` to the `EventTracker`'s mpsc receiver, runs to completion.
6. On success: write `run_summary.json`, `pricing_event_stream.sha256`, `time_series.csv`, `diagnostics.log`; update manifest `Completed`; rewrite suite-level `metrics_comparison.txt`.
7. On failure: mark `Failed` in manifest and bail with context.

### Per-tx lifecycle (simulator hot path)

1. `TransactionProducer::run` (`sim-rs/sim-core/src/sim/tx.rs`) wakes on arrival → samples one `ActorComponent` (`sim-core/src/tx_actors.rs:545`) → `ActorComponent::sample` produces `(bytes, value_lovelace, half_life_seconds)`, picks `posted_lane` via `lane_choice::pick`, computes `max_fee_lovelace` via `MaxFeePolicy::compute`. Emits `Event::TXGenerated` (carries `slot: u64`).
2. Tx is delivered to a node's `tx_source` channel; `NodeDriver` calls `LinearLeiosNode::handle_new_tx`.
3. Node consults its `PricingBackend::current_quote(posted_lane)`, calls `MempoolGate::try_admit` → either `Accept` (resident bytes/byte-cap updated) or `AdmissionRejection::{InsufficientMaxFee, ByteCapExceeded, FeeOverflow}`. The internal `Mempool` struct then handles UTxO/conflict bookkeeping.
4. On RB/EB build (lottery win): `select_eb_with_partition` (`sim-rs/sim-core/src/sim/linear_leios.rs:1861`) packs txs using `lane_selection_order`, decides `partition_activated`; `sample_from_mempool_lane_aware` (`linear_leios.rs:1766`) enforces `LaneValidityRule::PriorityOnly` on RB-reserved RBs.
5. Block sealed: `MempoolGate::charge_inclusion` (per-tx, in `mempool_gate.rs`) computes `actual_fee = minFeeB + quote(served_lane) × bytes` and `refund = max_fee − actual_fee`. Emits `Event::TXIncluded` per tx.
6. After every priced block: build `BlockLaneBreakdown`, call `backend.samples_for_block(...)`, feed to `backend.update_after_block(samples)`. Controller updates `quote_per_byte` via `CapacityWeightedWindow` aggregate; multiplier-floor invariant clamps priority post-update.
7. Quote drift may invalidate resident txs: `MempoolGate::revalidate` walks resident set, emits `EvictionRecord`s → `Event::TXEvictedQuoteDrift`.
8. At endorsement time: `eb_endorsement_valid` (`linear_leios.rs:886`) re-checks `posted_fee ≤ max_fee_lovelace` at producer's current quote; on any stale tx, drop endorsement entirely (RB ships unendorsed).

### Determinism-verify flow

1. `experiment-suite verify <suite.yaml>` → `verify_suite_with_run_id` (`runner.rs`).
2. For each `Completed` `(job, seed)`: re-run via `run_job`; recompute `pricing_event_stream_sha256` over `TXIncluded` + `TXEvictedQuoteDrift` only.
3. Assert recomputed hash equals on-disk `pricing_event_stream.sha256`; bail on mismatch.

**State Management:**
- Each `LinearLeiosNode` owns mutable per-node state (`NodePraosState`, `NodeLeiosState`, `NodeActorState`, `LedgerState`, `Mempool`, `MempoolGate`, `Box<dyn PricingBackend>`). No shared mutable state across nodes.
- `Simulation` owns the `Network`, `ClockCoordinator`, `TransactionProducer`, `SlotWitness`, and a `Vec<NodeDriver<N>>` (per-variant typed).
- `EventTracker` is `Clone`able and wraps an mpsc sender; every node clones it at construction; the collector owns the receiver.

## Key Abstractions

**`Lane` enum:**
- Purpose: Two-variant tag (`Standard`, `Priority`) carried on every `Transaction.posted_lane` and every inclusion event's `served_lane`. Single-lane mechanisms collapse to `Standard`.
- Examples: `sim-rs/sim-core/src/tx_pricing/mod.rs:29`, `sim-rs/sim-core/src/model.rs` (`Transaction.posted_lane`).
- Pattern: Sum type, `Copy`, kebab-case-serde for YAML.

**`PricingBackend` trait:**
- Purpose: Policy interface decoupling the controller(s) from the simulator. The simulator owns selection/packing; the backend answers quote queries and consumes after-the-fact samples.
- Examples: `sim-rs/sim-core/src/tx_pricing/mod.rs:129`. Implementors: `BaselinePricing`, `Eip1559Pricing`, `TwoLanePricing`.
- Pattern: Trait object with `Send + Sync`. Default methods cover single-lane behaviour (no validity rule, FIFO selection, no premium floor, single `Standard` sample per block).

**`CapacityWeightedWindow`:**
- Purpose: Unified utilisation signal: rolling `Σ relevant_bytes / Σ relevant_capacity` over a `VecDeque<Sample>` of fixed length. All controllers share it.
- Examples: `sim-rs/sim-core/src/tx_pricing/window.rs:20`.
- Pattern: u128 accumulators (no f64); `length = 1` collapses to per-block fill rate (used for RB-reserved priority).

**`MempoolGate`:**
- Purpose: Sole byte-cap authority. Owns per-lane resident bytes + per-tx (lane, bytes, max_fee_lovelace). Decisions: admit, evict-on-drift, charge-inclusion.
- Examples: `sim-rs/sim-core/src/sim/mempool_gate.rs:95`.
- Pattern: Plain struct (no async); called synchronously from inside the node's event handlers. Cooperates with `Mempool` (UTxO/conflict tracking) — neither owns the full lifecycle alone.

**`ActorComponent`:**
- Purpose: One weighted profile of a demand-side actor: arrival rate, byte distribution, value distribution, half-life, lane policy, max-fee policy.
- Examples: `sim-rs/sim-core/src/tx_actors.rs:545`.
- Pattern: `Clone` data + sampling methods that take `&mut Rng`. `LanePolicy::UtilityMaximising` routes the comparison through `libm::pow` + `libm::round` into `i128` lovelace for bit-determinism.

**`Simulation` + `NodeImpl`:**
- Purpose: Variant-polymorphic top-level driver. `LeiosVariant::Linear` is the only phase-2 path; legacy variants (`Full`, `FullWithoutIbs`, `Short`) coexist.
- Examples: `sim-rs/sim-core/src/sim.rs:101` (`Simulation`), `sim-rs/sim-core/src/sim.rs:330` (`NodeImpl`).
- Pattern: Per-variant enum wrapper (`NetworkWrapper`, `NodeListWrapper`) because `NodeImpl::Message` is an associated type that differs per variant; no dyn dispatch at this layer.

**`Manifest` / `Suite` / `Job` / `JobOverrides`:**
- Purpose: Resumable batch runner state machine + YAML schema.
- Examples: `sim-rs/sim-cli/src/runner.rs:67` (`Manifest`), `sim-rs/sim-cli/src/suite.rs` (schema).
- Pattern: Serde kebab-case structs persisted as JSON; `Running` entries demote to `Pending` on reload.

**`Event` enum:**
- Purpose: Single-tagged event stream for everything the simulator observes (slot ticks, CPU tasks, tx generation/inclusion/eviction, block production, votes, endorsements).
- Examples: `sim-rs/sim-core/src/events.rs:81`.
- Pattern: Tagged enum (`#[serde(tag = "type")]`); cloned through `mpsc` to the metrics consumer.

## Entry Points

**`experiment-suite run|status|verify`:**
- Location: `sim-rs/sim-cli/src/bin/experiment-suite/main.rs`
- Triggers: Operator CLI invocation (or shell scripts under `sim-rs/scripts/`).
- Responsibilities: Argv parsing, subcommand dispatch into `runner::run_suite_with_run_id` / `print_status` / `verify_suite_with_run_id`.

**Legacy `sim-cli` single-run binary:**
- Location: `sim-rs/sim-cli/src/main.rs`
- Triggers: Direct invocation with a single `RawParameters` YAML; pre-phase-2 path.
- Responsibilities: Loads default topology, builds `SimConfiguration`, runs one simulation, emits the legacy `EventMonitor` event sink to JSONL.

**Test-suite entry points:**
- Unit tests: `sim-rs/sim-core/src/sim/tests/{m1_smoke, m2_two_lane, m3_actors, linear_leios}.rs` — invoked by `cargo test --workspace`.
- Suite-level determinism goldens: `sim-rs/sim-cli/tests/determinism.rs` — invoked by `cargo test --release -- --ignored determinism`.

**Per-node async actor:**
- Location: `sim-rs/sim-core/src/sim/driver.rs:46` (`NodeDriver::run`).
- Triggers: Spawned by `Simulation::run` via `JoinSet`; one task per node.
- Responsibilities: select-loop over (new slot, network message, CPU task completion, timed event, custom event); delegates each into `NodeImpl::handle_*`.

## Architectural Constraints

- **Threading:** Tokio current-thread runtime (one OS thread). Each node is a tokio task; concurrency is cooperative, not preemptive. `Simulation::run` uses a `JoinSet` + `select!` over the network, tx producer, slot witness, and clock coordinator. No `Mutex` is needed for per-node state because each node owns its state exclusively.
- **Global state:** No module-level singletons in simulation-affecting paths. `SimConfiguration` is wrapped in `Arc` and shared read-only. `EventTracker` is `Clone`able and wraps an `mpsc::UnboundedSender`. Atomics: `AtomicU64` in `config.rs` is used only for next-id generation in test/data construction.
- **f64 prohibition in hot paths:** Simulation-affecting state (admission, eviction, fee charging, controller coefficient, multiplier-floor invariant, actor lane choice, `quote_per_byte`, `max_fee_lovelace`) must use `u64`/`u128`/integer-rational. `libm::pow` + `libm::round` are the only blessed float ops, and only in `lane_choice::pick` where they round into `i128` lovelace before comparison. f64 is allowed *only* in `sim-cli/src/metrics/` and `tx_actors::welfare`.
- **Determinism is intra-architecture:** All goldens are pinned on x86_64 / glibc. Cross-arch CI is documented as not-yet-built; non-pricing code paths inherited from upstream `main` (slot lottery, propagation, distribution sampling) have not been hardened.
- **Selection lives in the simulator, not the backend:** The `PricingBackend` never sees `Transaction`, `Mempool`, or simulator types. All packing/selection logic is in `linear_leios.rs::select_eb_with_partition` / `sample_from_mempool_lane_aware`.
- **Variant routing is non-generic at the top:** `NetworkWrapper`/`NodeListWrapper` (`sim-rs/sim-core/src/sim.rs:36-84`) enumerate the three protocol variants because each `NodeImpl::Message` is a distinct type; adding a new variant requires a new enum arm here.
- **Two cooperating mempool layers per node:** The internal `Mempool` struct (in `linear_leios.rs`, around line 2553) tracks UTxO conflicts and selection ordering; `MempoolGate` tracks fee admissibility and byte cap. Neither owns the full lifecycle alone — they must be kept in sync by the calling node.
- **RB-reduced overlays are full replacements, not stacked:** `JobOverrides::protocol` either replaces `default_protocol` whole-cloth or doesn't — there is no overlay composition. Changes to `protocol-base.yaml` must be replicated into all three `protocol-rb-reduced-{half,third,quarter}.yaml` files manually.
- **Serde casing is heterogeneous by accident:** `Suite`/`Job`/`JobOverrides`/`Manifest`/`JobEntry` use `#[serde(rename_all = "kebab-case")]`; `RunSummary` uses Rust snake_case (no `rename_all`). Both shapes coexist on disk. New fields should match the surrounding type's existing convention.

## Anti-Patterns

### Reading `urgency: f64` outside the lane-choice math

**What happens:** A future change reads `Transaction.urgency` (an `f64`) from a non-`lane_choice` code path.
**Why it's wrong:** `urgency` is f64; any simulation-affecting decision derived from it bypasses the integer/rational discipline. Cross-arch determinism is preserved only because `lane_choice::pick` routes the comparison through `libm::pow` + `libm::round` into `i128` lovelace before any compare.
**Do this instead:** Treat `urgency` as a black-box parameter consumed only inside `tx_actors::lane_choice` (`sim-rs/sim-core/src/tx_actors.rs`). Reporting code may inspect it for time-series output.

### Re-introducing delta-slot inference of `submit_slot`

**What happens:** Code in `MetricsCollector` infers a tx's `submit_slot` from the ordering of events rather than reading `slot: u64` from `Event::TXGenerated`.
**Why it's wrong:** M4 added an explicit `slot` field on `TXGenerated` specifically to remove the fragile delta-tracking invariant. Inference re-introduces a hidden ordering dependency that breaks on event reordering.
**Do this instead:** Read `submit_slot` directly from `Event::TXGenerated.slot` (`sim-rs/sim-core/src/events.rs`).

### Treating "one tx source" as "one mempool"

**What happens:** Code or analysis assumes that because `topology-single-producer.yaml` has a single `tx-generation-weight`, there is also a single mempool.
**Why it's wrong:** Every `LinearLeiosNode` owns its own `Mempool` + `MempoolGate`. With single-producer, N=1, so source/producer/mempool happen to coincide; in any multi-node topology the per-mempool counts diverge.
**Do this instead:** Always reason per-node. `MetricsCollector` pins a single representative node via the runner (`runner.rs` pre-sets the lexicographically smallest name); production runs are deterministic, tests use a "first tick wins" fallback.

### Composing protocol overlays as if they stack

**What happens:** A new YAML defines partial protocol overrides expecting the runner to merge them on top of `protocol-base.yaml`.
**Why it's wrong:** `JobOverrides::protocol` is a whole-file replacement. There is no `protocol_overlay: Vec<PathBuf>` field; figment is fed exactly one protocol file (either `overrides.protocol` or `default_protocol`).
**Do this instead:** Duplicate `protocol-base.yaml` into a new full file and override only the knobs you intend to change (this is what `protocol-rb-reduced-{half,third,quarter}.yaml` do).

### Mutating an already-gossiped EB body to drop stale txs

**What happens:** At endorsement, instead of refusing to endorse a stale EB, code edits the EB to remove the offending tx.
**Why it's wrong:** The EB has already gossiped to other nodes; mutating its body diverges the network view. M2 settled this: the producer refuses to endorse and the RB ships unendorsed.
**Do this instead:** `eb_endorsement_valid` (`sim-rs/sim-core/src/sim/linear_leios.rs:886`) returns false on any stale tx; the endorsement is dropped wholesale.

## Error Handling

**Strategy:** `anyhow::Result` end-to-end from CLI through `runner::run_job` / `Simulation::new`. Pricing-kernel constructors return `anyhow::Result` and validate at config-load time. Admission failure is *not* an error — `AdmissionRejection` is a normal enum return value consumed by the node.

**Patterns:**
- Construct-then-validate: `MaxFeePolicy::validate()`, `Eip1559Settings::validate()`, `TwoLaneSettings::validate()`, `Multiplier::new` (rejects denominator=0), `CapacityWeightedWindow::new` (rejects length=0).
- Failed job → manifest `Failed` entry with `error: Option<String>`; `run_suite` bails on first failure with `with_context`.
- Determinism mismatch in `verify_suite` is a hard error; on `UPDATE_GOLDENS=1` in `tests/determinism.rs` the test overwrites instead of asserting.
- `tracing::warn!` for non-fatal misconfigurations (e.g. missing pricing config defaults to `BaselinePricing`).

## Cross-Cutting Concerns

**Logging:** `tracing` crate; `tracing_subscriber` initialised in both `sim-cli/main.rs` and `experiment-suite/main.rs` with compact, no-time format. `RUST_LOG`/`EnvFilter` controls verbosity; default `INFO`. Per-job progress lines are at `INFO`; controller-internal diagnostics emit to the per-job `diagnostics.log` via `metrics::diagnostics`.

**Validation:** All config knobs validated at load (`RawParameters` → `SimConfiguration::new`). Multiplier-floor, window length, max-fee policy denominators, controller `D`/`target` parameters all checked before any simulation runs.

**Authentication:** Not applicable (offline simulator).

**Time:** Virtual clock via `clock::Clock` + `ClockCoordinator` + `ClockBarrier`. `Timestamp` is integer-microseconds. Wall-clock time is read only for manifest timestamps and progress-write debounce (`PROGRESS_WRITE_WALL_INTERVAL`).

**Determinism seeding:** Every `Simulation::init` seeds a `ChaChaRng` from `SimConfiguration.seed`; per-node RNGs are derived deterministically via `rng.next_u64()` for each `NodeImpl::new`. The `TransactionProducer` gets its own derived seed.

---

*Architecture analysis: 2026-05-13*
