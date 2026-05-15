# Codebase Structure

**Analysis Date:** 2026-05-15

## Directory Layout

```
arc-tiered-pricing/
├── CLAUDE.md                              # Project instructions / handoff
├── README.md                              # Upstream Leios README
├── docs/
│   └── phase-2/                           # Spec + plan + per-milestone deltas
│       ├── mechanism-design.md
│       ├── implementation-plan.md
│       ├── m{1,2,3,4,5}-handoff.md
│       └── calibration-fix-postmortem.md
├── .planning/                             # Decision memos, spike notes, codebase docs
│   ├── family-b-decision-2026-05-14.md    # Authoritative Family B decision
│   ├── REVIEW.md                          # Review-finding dispositions
│   ├── codebase/                          # (this directory)
│   └── spikes/                            # spike artefacts (006-curve-design, etc.)
└── sim-rs/                                # The Rust workspace
    ├── Cargo.toml                         # 2-crate workspace
    ├── Cargo.lock
    ├── sim-core/                          # Protocol + pricing kernel
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs                     # `pub mod {clock, config, events, model, sim, tx_actors, tx_pricing, probability}`
    │       ├── model.rs                   # Transaction, Block, LinearRankingBlock, PerLaneQuote, WindowAggregate
    │       ├── config.rs                  # RawParameters/SimConfiguration, all Raw* deserialisation
    │       ├── events.rs                  # Event enum + EventTracker emitters
    │       ├── probability.rs             # FloatDistribution helpers
    │       ├── tx_actors.rs               # ActorComponent/Profile, MaxFeePolicy, lane_choice, welfare, LatencyEstimator
    │       ├── tx_pricing/                # The phase-2 pricing kernel (chain-derived)
    │       │   ├── mod.rs                 # PricingBackend trait, ChainView trait, Lane, samples
    │       │   ├── window.rs              # aggregate_from_chain + update_aggregate (pure)
    │       │   ├── single_lane.rs         # BaselinePricing + Eip1559Pricing
    │       │   └── two_lane.rs            # TwoLanePricing + 4 TwoLaneVariant arms
    │       ├── clock/                     # Discrete-event clock
    │       │   ├── coordinator.rs
    │       │   ├── mock.rs
    │       │   └── timestamp.rs
    │       ├── clock.rs                   # re-exports
    │       ├── network/                   # netsim-async wrapper
    │       │   ├── connection.rs
    │       │   └── coordinator.rs
    │       ├── network.rs                 # re-exports
    │       ├── sim.rs                     # Simulation + NodeImpl trait + NetworkWrapper
    │       └── sim/
    │           ├── linear_leios.rs        # THE phase-2 protocol (3,113 lines)
    │           ├── linear_leios/
    │           │   └── attackers.rs       # EB-withholding attacker hooks
    │           ├── mempool_gate.rs        # Admission/revalidation/inclusion charging
    │           ├── lottery.rs             # VRF lottery helpers
    │           ├── cpu.rs                 # CPU-task accounting
    │           ├── driver.rs              # NodeDriver — runs a NodeImpl in an actor loop
    │           ├── slot.rs                # SlotWitness
    │           ├── tx.rs                  # TransactionProducer (legacy non-actor path)
    │           ├── leios.rs               # Legacy full-Leios protocol (not phase-2-relevant)
    │           ├── stracciatella.rs       # Legacy variant (not phase-2-relevant)
    │           └── tests/                 # M1/M2/M3 deterministic scenario tests
    │               ├── mod.rs
    │               ├── m1_smoke.rs
    │               ├── m2_two_lane.rs
    │               ├── m3_actors.rs
    │               └── linear_leios.rs
    ├── sim-cli/                           # Phase-2 driver + metrics
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── lib.rs                     # re-exports (runner, suite, metrics, events)
    │   │   ├── main.rs                    # legacy single-run `sim-cli` binary
    │   │   ├── runner.rs                  # Manifest, run_suite, run_job, verify_suite
    │   │   ├── suite.rs                   # Suite YAML schema (Suite/Job/JobOverrides)
    │   │   ├── events.rs                  # legacy event sink
    │   │   ├── events/
    │   │   │   ├── aggregate.rs
    │   │   │   └── liveness.rs
    │   │   ├── metrics/                   # Welfare metrics layer
    │   │   │   ├── mod.rs
    │   │   │   ├── collector.rs           # MetricsCollector, RunSummary, ComponentSummary
    │   │   │   ├── comparison.rs          # metrics_comparison.txt writer
    │   │   │   ├── diagnostics.rs         # diagnostics.log writer
    │   │   │   └── time_series.rs         # time_series.csv writer
    │   │   └── bin/
    │   │       ├── experiment-suite/
    │   │       │   └── main.rs            # `experiment-suite {run|status|verify}` CLI
    │   │       └── gen-test-data/         # test-data generator (legacy)
    │   ├── parameters/                    # embedded defaults (e.g. config.default.yaml)
    │   └── tests/
    │       ├── determinism.rs             # M5 suite-level golden hashes
    │       └── parallel_runner.rs         # parallel-execution invariants
    ├── parameters/
    │   ├── phase-2-sweep/                 # ALL phase-2 configs
    │   │   ├── protocol-base.yaml
    │   │   ├── protocol-rb-reduced-half.yaml
    │   │   ├── protocol-rb-reduced-third.yaml
    │   │   ├── protocol-rb-reduced-quarter.yaml
    │   │   ├── topology-single-producer.yaml
    │   │   ├── topology-realistic-100.yaml          # mainnet-snapshot mass-stratified (DEFAULT)
    │   │   ├── topology-cip-realistic.yaml
    │   │   ├── demand/
    │   │   │   ├── paper_like_congested.yaml
    │   │   │   ├── paper_like_mispriced.yaml
    │   │   │   ├── paper_like_moderate.yaml
    │   │   │   ├── paper_like_realistic.yaml
    │   │   │   └── sundaeswap_moderate.yaml
    │   │   ├── pricing/                              # 19 controller-tuning YAMLs
    │   │   │   ├── baseline_flat_fee.yaml
    │   │   │   ├── eip1559_d{4,8,16}_target{0.25,0.5,0.75}_window{16,32,64}.yaml
    │   │   │   ├── two_lane_priority_only_static_x{4,8,16}.yaml
    │   │   │   ├── two_lane_priority_only_unreserved_x{4,8,16}.yaml
    │   │   │   ├── two_lane_both_dynamic_partitioned_x{4,16}.yaml
    │   │   │   └── two_lane_both_dynamic_unreserved_x{4,16}.yaml
    │   │   ├── suites/                              # 19 phase-2 suite YAMLs
    │   │   │   ├── phase-2-eip1559-robustness.yaml          ┐
    │   │   │   ├── phase-2-eip1559-smoothing.yaml            │
    │   │   │   ├── phase-2-priority-only-rb-reserved.yaml    │
    │   │   │   ├── phase-2-priority-only-unreserved.yaml     │ 7 M3/M4 suites
    │   │   │   ├── phase-2-two-lane-both-dynamic.yaml        │ pinned by M5
    │   │   │   ├── phase-2-rb-scarcity.yaml                  │ suite goldens
    │   │   │   ├── phase-2-urgency-inversion.yaml           ┘
    │   │   │   ├── phase-2-{congested,moderate,realistic}-{singlelane,priority-only,both-dynamic}.yaml
    │   │   │   ├── phase-2-sundaeswap-{singlelane,priority-only,both-dynamic}.yaml
    │   │   │   ├── phase-2-rb-scarcity.README.md
    │   │   │   ├── phase-2-urgency-inversion.README.md
    │   │   │   └── .goldens/
    │   │   │       ├── phase-2-eip1559-robustness.sha256
    │   │   │       ├── phase-2-eip1559-smoothing.sha256
    │   │   │       ├── phase-2-priority-only-rb-reserved.sha256
    │   │   │       ├── phase-2-priority-only-unreserved.sha256
    │   │   │       ├── phase-2-two-lane-both-dynamic.sha256
    │   │   │       ├── phase-2-rb-scarcity.sha256
    │   │   │       └── phase-2-urgency-inversion.sha256
    │   │   └── experiments/                          # (currently empty)
    │   ├── topology.default.yaml                     # upstream default topology
    │   └── … (other upstream parameter files)
    ├── scripts/                                       # helper scripts
    ├── output/                                        # per-run artefacts (gitignored)
    ├── target/                                        # cargo build output (gitignored)
    └── test_data/
```

## Directory Purposes

**`sim-rs/sim-core/`:**
- Purpose: protocol model + pricing kernel. No driver, no CLI, no metrics.
- Contains: types (`model.rs`), config deserialisation (`config.rs`), event tagged-enum (`events.rs`), the linear-Leios protocol (`sim/linear_leios.rs`), the pricing kernel (`tx_pricing/`), the actor model (`tx_actors.rs`), mempool gate (`sim/mempool_gate.rs`), clock + network plumbing.
- Key files: `tx_pricing/mod.rs` (PricingBackend trait), `sim/linear_leios.rs` (the only phase-2-relevant protocol), `sim/mempool_gate.rs` (admission authority), `model.rs` (`LinearRankingBlock`, `PerLaneQuote`, `WindowAggregate`).

**`sim-rs/sim-core/src/tx_pricing/`:**
- Purpose: the chain-derived pricing kernel.
- Contains: trait surface (`mod.rs`), window aggregation (`window.rs`), single-lane backends (`single_lane.rs`), two-lane backends (`two_lane.rs`).
- Key files: `mod.rs` (`PricingBackend`, `ChainView`, `Lane`, `BlockKind`, `PricedBlockSample`, `BlockLaneBreakdown`, `Multiplier`, `LaneValidityRule`, `LaneSelectionOrder`, `PricingSnapshot`, `snapshot_at`).

**`sim-rs/sim-core/src/sim/`:**
- Purpose: protocol-simulation implementations.
- Contains: linear-Leios (`linear_leios.rs`, the only phase-2-relevant impl), legacy variants (`leios.rs`, `stracciatella.rs`), the mempool gate, the lottery, the slot witness, the legacy tx producer, CPU accounting, the node-driver loop.
- Key files: `linear_leios.rs`, `mempool_gate.rs`. The `tests/` subdirectory holds M1/M2/M3 deterministic scenario tests.

**`sim-rs/sim-cli/`:**
- Purpose: the driver + metrics + binaries. Builds on top of `sim-core`.
- Contains: `Suite` schema (`suite.rs`), the parallel runner (`runner.rs`), the metrics collector + writers (`metrics/`), legacy event sinks (`events.rs`, `events/`), the `experiment-suite` binary (`bin/experiment-suite/main.rs`), the legacy single-run `sim-cli` binary (`main.rs`).
- Key files: `runner.rs`, `suite.rs`, `metrics/collector.rs`, `bin/experiment-suite/main.rs`.

**`sim-rs/sim-cli/src/metrics/`:**
- Purpose: f64 reporting layer over the integer event stream.
- Contains: `MetricsCollector` (`collector.rs`), `time_series.csv` writer (`time_series.rs`), `metrics_comparison.txt` writer (`comparison.rs`), `diagnostics.log` writer (`diagnostics.rs`).
- Key files: `collector.rs` — the entire ingestion pipeline and run-summary types.

**`sim-rs/parameters/phase-2-sweep/`:**
- Purpose: all phase-2 configuration. Layered overlays for the YAML-figment composition in `run_job`.
- Contains: protocol overlays (`protocol-base.yaml` + 3 RB-reduced variants), topology files, demand-profile YAMLs, pricing-tuning YAMLs, suite YAMLs, suite READMEs, and the goldens.
- Key files: `protocol-base.yaml` (the phase-2 protocol baseline), `topology-realistic-100.yaml` (the suite default since 2026-05-13), `suites/.goldens/<suite>.sha256` (M5 suite-level goldens).

**`docs/phase-2/`:**
- Purpose: the spec + the implementation plan + per-milestone handoff notes.
- Contains: `mechanism-design.md` (the spec), `implementation-plan.md` (the rebuild plan), `m{1,2,3,4,5}-handoff.md`, `calibration-fix-postmortem.md`.

**`.planning/`:**
- Purpose: in-flight decision memos, spike artefacts, review dispositions, codebase docs.
- Contains: `family-b-decision-2026-05-14.md` (authoritative chain-derived adoption memo), `REVIEW.md` (review-finding fix-status table), `spikes/` (spike notes), `codebase/` (this directory), various `*-PLAN.md` and `*-investigation.md` working files.

**`sim-rs/output/`** (gitignored):
- Per-run artefacts: `<output_dir>/<job_name>/<seed>/{time_series.csv, diagnostics.log, run_summary.json, pricing_event_stream.sha256}` plus `<output_dir>/{manifest.json, metrics_comparison.txt}`.

## Key File Locations

**Entry Points:**
- `sim-rs/sim-cli/src/bin/experiment-suite/main.rs`: the phase-2 driver — `experiment-suite {run|status|verify} <suite.yaml> [--run-id ID] [--parallelism N]`.
- `sim-rs/sim-cli/src/main.rs`: legacy single-run `sim-cli` binary (predates the suite runner).

**Configuration:**
- `sim-rs/sim-cli/parameters/config.default.yaml`: embedded base config layered first by `run_job`.
- `sim-rs/parameters/phase-2-sweep/protocol-base.yaml`: phase-2 protocol baseline (mempool cap, max block size, lottery params, cpu times, sizes, EB validation thresholds).
- `sim-rs/parameters/phase-2-sweep/protocol-rb-reduced-{half,third,quarter}.yaml`: full replacements that override only `rb-body-max-size-bytes`.
- `sim-rs/parameters/phase-2-sweep/topology-realistic-100.yaml`: 100-node mass-stratified mainnet curve (suite default since 2026-05-13).
- `sim-rs/parameters/phase-2-sweep/topology-single-producer.yaml`: 1-node testing topology used by `sim-cli/tests/determinism.rs`.
- `sim-rs/parameters/phase-2-sweep/demand/*.yaml`: actor profiles (`ActorProfile` YAML).
- `sim-rs/parameters/phase-2-sweep/pricing/*.yaml`: per-controller tunings (single-lane EIP-1559 settings or two-lane `{variant, priority, standard, multiplier_floor}`).

**Core Logic:**
- `sim-rs/sim-core/src/tx_pricing/mod.rs`: `PricingBackend` and `ChainView` traits — the only seam between protocol simulator and pricing kernel.
- `sim-rs/sim-core/src/tx_pricing/single_lane.rs`: `BaselinePricing`, `Eip1559Pricing`, `compute_eip1559_step`, `worst_case_eip1559_quote`.
- `sim-rs/sim-core/src/tx_pricing/two_lane.rs`: `TwoLanePricing`, `TwoLaneVariant`, `TwoLaneSettings`, `apply_floor`.
- `sim-rs/sim-core/src/tx_pricing/window.rs`: `aggregate_from_chain`, `update_aggregate`.
- `sim-rs/sim-core/src/sim/linear_leios.rs`: protocol state machine; chain-derived production path (`try_generate_rb`, `compute_chain_derived_quote_for_child_of`, `publish_rb`, `samples_for_rb`, `prune_block_samples`, `revalidate_against_new_tip`); EB endorsement validation (`eb_endorsement_valid`); EB partition decision (`select_eb_with_partition`); served-lane assignment (`assign_served_lanes`); `impl ChainView for LinearLeiosNode`.
- `sim-rs/sim-core/src/sim/mempool_gate.rs`: `MempoolGate::{try_admit, revalidate, on_inclusion, remove_silent, fee_at}`.
- `sim-rs/sim-core/src/tx_actors.rs`: `ActorComponent`, `ActorProfile`, `MaxFeePolicy`, `LanePolicy`, `lane_choice::pick`, `welfare`, `LatencyEstimator`.
- `sim-rs/sim-core/src/model.rs`: `Transaction`, `LinearRankingBlock` (carries `derived_quote: PerLaneQuote` + `window_aggregate: WindowAggregate`), `LinearEndorserBlock` (carries `partition_activated: bool`).
- `sim-rs/sim-core/src/events.rs`: `Event` enum (`TXIncluded`, `TXEvictedQuoteDrift`, `PricingTick`, `TXGenerated`, `LinearPricingSampleApplied`, …) + `EventTracker`.
- `sim-rs/sim-cli/src/runner.rs`: `Suite` loading, `Manifest` + `JobEntry` lifecycle, parallel worker pool, `run_job`, `verify_suite_with_run_id`.
- `sim-rs/sim-cli/src/metrics/collector.rs`: `MetricsCollector::ingest`, `RunSummary`, `ComponentSummary`, `TimeSeriesRow`, SHA256 over pricing events.

**Testing:**
- `sim-rs/sim-core/src/sim/tests/m1_smoke.rs`: M1 smoke (single-lane baseline).
- `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs`: M2 two-lane scenario tests + intra-arch event-stream goldens (constants pinned in source).
- `sim-rs/sim-core/src/sim/tests/m3_actors.rs`: M3 actor-model scenario tests + intra-arch event-stream goldens.
- `sim-rs/sim-core/src/sim/tests/linear_leios.rs`: legacy linear-Leios protocol tests.
- `sim-rs/sim-cli/tests/determinism.rs`: M5 suite-level golden hashes (`#[ignore]`'d by default; run via `cargo test --release -- --ignored determinism`).
- `sim-rs/sim-cli/tests/parallel_runner.rs`: parallel-execution invariants for the suite runner.

## Naming Conventions

**Files:**
- Rust modules: snake_case (`linear_leios.rs`, `mempool_gate.rs`, `tx_pricing/mod.rs`, `tx_actors.rs`).
- Test files: `m<N>_<name>.rs` or `<protocol>.rs` under `src/sim/tests/`.
- YAML configs: kebab-case for protocol/topology/suite files (`protocol-base.yaml`, `topology-realistic-100.yaml`, `phase-2-eip1559-robustness.yaml`); snake_case for pricing/demand tunings (`eip1559_d8_target0.5_window32.yaml`, `paper_like_congested.yaml`).
- Suite READMEs: `<suite>.README.md` colocated with the YAML.
- Golden files: `<suite>.sha256` under `suites/.goldens/`.

**Directories:**
- All-lowercase, kebab-case for parameter directories (`phase-2-sweep/`, `parameters/`).
- Crate names: kebab-case (`sim-core`, `sim-cli`).

**Functions:**
- Rust functions: snake_case (`compute_derived_quote`, `try_generate_rb`, `aggregate_from_chain`).
- Test functions: snake_case describing the assertion (`rb_reserved_skips_standard_fee_tx_in_rb_body`, `sibling_rbs_produce_identical_derived_quote`).

**Types:**
- PascalCase (`PricingBackend`, `LinearRankingBlock`, `WindowAggregate`, `TwoLaneVariant`, `Lane`, `MempoolGate`).
- Type-level wrappers: PascalCase (`BlockId`, `TransactionId`, `EndorserBlockId`).

**Serde conventions:**
- `Suite` / `Manifest` / `JobEntry` / `Job` / `JobOverrides`: kebab-case via `#[serde(rename_all = "kebab-case")]`.
- `RunSummary` / `ComponentSummary`: snake_case (no `rename_all`, idiomatic Rust field names).
- `Event` enum variants: PascalCase tag with `#[serde(tag = "type")]` (`events.rs:80`).
- `Lane`, `BlockKind`, `LaneSelectionOrder`, `JobStatus`: kebab-case (e.g. `"standard"`, `"priority"`, `"ranking-block"`).

## Where to Add New Code

**New pricing backend variant or controller arm:**
- Implementation: `sim-rs/sim-core/src/tx_pricing/<single_lane.rs|two_lane.rs|new_module.rs>`.
- Re-export in: `sim-rs/sim-core/src/tx_pricing/mod.rs:22-30` (`pub use ...`).
- Config plumbing: `sim-rs/sim-core/src/config.rs` (add a `PricingConfig::*` variant + `RawPricingConfig` arm).
- Wire-up in protocol: `sim-rs/sim-core/src/sim/linear_leios.rs:484` (the `match sim_config.pricing_config()` block at `LinearLeiosNode::new`).
- Unit tests: in the new backend module (follow `tx_pricing/single_lane.rs:403`'s `#[cfg(test)] mod tests`).
- Integration test: `sim-rs/sim-core/src/sim/tests/m<N>_<name>.rs`.
- Pricing YAML: `sim-rs/parameters/phase-2-sweep/pricing/<descriptor>.yaml`.
- Suite YAML (if a new mechanism category): `sim-rs/parameters/phase-2-sweep/suites/<phase-2-…>.yaml` (+ `<…>.README.md` if framing departs from the spec) (+ `<…>.sha256` golden via `UPDATE_GOLDENS=1`).

**New `Event` variant:**
- Definition: `sim-rs/sim-core/src/events.rs:80` (add to the `Event` enum).
- Emitter: `EventTracker::track_*` method (`sim-rs/sim-core/src/events.rs:411+`).
- Call sites: protocol code in `sim-rs/sim-core/src/sim/linear_leios.rs`.
- Collector: `sim-rs/sim-cli/src/metrics/collector.rs::ingest` (`collector.rs:343`).
- If it affects pricing semantics: also update the SHA256 stream in `collector.rs::ingest_pricing_event` (search for `pricing_event_stream_sha256` updates).
- Hash regeneration: `UPDATE_GOLDENS=1 cargo test --release -- --ignored determinism` + commit + tag.

**New mempool / gate behaviour:**
- Implementation: `sim-rs/sim-core/src/sim/mempool_gate.rs` for fee-validity / byte-cap concerns; the UTxO-conflict `Mempool` lives in `sim-rs/sim-core/src/sim/linear_leios.rs:2905` and is not phase-2's primary surface.
- Unit tests: `mempool_gate.rs` has a comprehensive `#[cfg(test)] mod tests` block (`mempool_gate.rs:293`) — add alongside.
- Invariant: `MempoolGate.config.max_total_size_bytes == Mempool.max_size_bytes` must hold (enforced by `debug_assert_eq!` in `linear_leios.rs:479`).

**New actor profile or `MaxFeePolicy` arm:**
- Implementation: `sim-rs/sim-core/src/tx_actors.rs` (extend `MaxFeePolicy` or `LanePolicy` enum).
- Config: `sim-rs/sim-core/src/config.rs::RawActorProfile`.
- Demand YAML: `sim-rs/parameters/phase-2-sweep/demand/<descriptor>.yaml`.
- Unit tests: `sim-rs/sim-core/src/sim/tests/m3_actors.rs`.

**New suite:**
- Suite YAML: `sim-rs/parameters/phase-2-sweep/suites/<phase-2-…>.yaml` (keys: `suite-name`, `output-dir`, `seeds`, `default-{slots,topology,protocol,demand}`, `jobs: [{name, pricing, overrides?}]`).
- Optional README: `sim-rs/parameters/phase-2-sweep/suites/<phase-2-…>.README.md` (colocated).
- M5 golden: `sim-rs/parameters/phase-2-sweep/suites/.goldens/<phase-2-…>.sha256` + matching `#[test] #[ignore] fn determinism_<…>()` in `sim-rs/sim-cli/tests/determinism.rs:189-228`.

**New scenario test:**
- Determinism-pinned scenario: `sim-rs/sim-core/src/sim/tests/m<N>_<name>.rs` (follow `m2_two_lane.rs` / `m3_actors.rs` shape: build a `SimConfiguration` with `topology-single-producer.yaml`-equivalent, drive the node manually, drain events, hash, assert against a `const EXPECTED_SHA: &str = "..."`).
- Pricing unit test: `#[cfg(test)] mod tests` inside the backend module.
- Suite-level integration: a new `phase-2-<…>.yaml` + golden (above).

**New runner / metric / report:**
- Metric: `sim-rs/sim-cli/src/metrics/collector.rs` (add field to `RunSummary`/`ComponentSummary` + ingestion in `ingest`); writer: `sim-rs/sim-cli/src/metrics/{comparison.rs,diagnostics.rs,time_series.rs}` as appropriate.
- Runner subcommand: `sim-rs/sim-cli/src/bin/experiment-suite/main.rs::Command` + a new function in `sim-rs/sim-cli/src/runner.rs`.

## Special Directories

**`.planning/`:**
- Purpose: working memos, spike artefacts, decision records, review dispositions.
- Generated: No (hand-authored).
- Committed: Yes (memos and decisions are first-class artefacts).
- Subdirectory: `.planning/codebase/` (this directory) — auto-generated codebase maps; `.planning/spikes/` — per-spike notes.

**`sim-rs/output/`:**
- Purpose: per-run artefacts written by `experiment-suite`.
- Generated: Yes (by the runner).
- Committed: No (gitignored).
- Contents per (job, seed): `time_series.csv`, `diagnostics.log`, `run_summary.json`, `pricing_event_stream.sha256`. Per suite: `manifest.json`, `metrics_comparison.txt`.

**`sim-rs/target/`:**
- Purpose: cargo build output.
- Generated: Yes.
- Committed: No (gitignored).

**`sim-rs/parameters/phase-2-sweep/suites/.goldens/`:**
- Purpose: pinned suite-level pricing-event-stream SHA256 hashes (one per M3/M4 suite).
- Generated: Yes (via `UPDATE_GOLDENS=1 cargo test --release -- --ignored determinism`).
- Committed: Yes — these are the determinism contract.
- Regeneration ritual: change simulator → run `UPDATE_GOLDENS=1 ...` → `git add parameters/phase-2-sweep/suites/.goldens && git commit -m "M5 goldens regenerated after <reason>" && git tag -a m5-goldens-<n>`.

**`sim-rs/sim-cli/parameters/`:**
- Purpose: holds the embedded `config.default.yaml` consumed at runtime via `include_str!` (`sim-cli/src/runner.rs:841`).
- Generated: No.
- Committed: Yes.

**`sim-rs/test_data/`:**
- Purpose: legacy test fixtures.
- Committed: Yes.

## The 7 Phase-2 Mechanism Suites (M5-Pinned)

These seven suites have suite-level goldens under `sim-rs/parameters/phase-2-sweep/suites/.goldens/`:

| Suite YAML | Question | Goldens |
|---|---|---|
| `phase-2-eip1559-robustness.yaml` | Single-lane EIP-1559 across `D` (4, 8, 16) and `target` (0.25, 0.5, 0.75) | `phase-2-eip1559-robustness.sha256` |
| `phase-2-eip1559-smoothing.yaml` | Single-lane EIP-1559 window-length sweep (16, 32, 64) | `phase-2-eip1559-smoothing.sha256` |
| `phase-2-priority-only-rb-reserved.yaml` | RB-reserved priority-only-static-standard × ×4 / ×8 / ×16 multiplier floor | `phase-2-priority-only-rb-reserved.sha256` |
| `phase-2-priority-only-unreserved.yaml` | Un-reserved priority-only premium × same multiplier sweep | `phase-2-priority-only-unreserved.sha256` |
| `phase-2-two-lane-both-dynamic.yaml` | Both-dynamic in partitioned and un-partitioned forms | `phase-2-two-lane-both-dynamic.sha256` |
| `phase-2-rb-scarcity.yaml` | RB-capacity scarcity restated as a two-lane experiment | `phase-2-rb-scarcity.sha256` |
| `phase-2-urgency-inversion.yaml` | Urgency inversion under mis-priced actors | `phase-2-urgency-inversion.sha256` |

The remaining 12 suites under `sim-rs/parameters/phase-2-sweep/suites/` (`phase-2-{congested,moderate,realistic,sundaeswap}-{singlelane,priority-only,both-dynamic}.yaml`) are demand-regime sweeps and are not goldens-pinned.

---

*Structure analysis: 2026-05-15*
