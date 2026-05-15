# Codebase Structure

**Analysis Date:** 2026-05-13

## Directory Layout

```
arc-tiered-pricing/
├── CLAUDE.md                              # Project-level context (read first)
├── README.md                              # Top-level pointer
├── docs/
│   ├── phase-1/                           # Upstream Leios docs (not phase-2)
│   └── phase-2/                           # The active design + handoffs
│       ├── mechanism-design.md            # The phase-2 spec
│       ├── implementation-plan.md         # The rebuild plan
│       ├── m{1..5}-handoff.md             # Per-milestone delta notes
│       ├── m6-implementation-plan.md      # M6 multi-node migration plan
│       ├── calibration-fix-postmortem.md  # rb-prob=1.0 bug post-mortem
│       └── CPS-0023/                      # CIP-0023 reference material
├── .planning/
│   └── codebase/                          # GSD codebase-map outputs (this dir)
└── sim-rs/                                # The Rust workspace (cargo root)
    ├── Cargo.toml                         # Workspace manifest (members: sim-core, sim-cli)
    ├── Cargo.lock
    ├── CHANGELOG.md
    ├── IMPLEMENTATION.md                  # Upstream simulator notes
    ├── README.md
    ├── convert.jq                         # Legacy event-log helper
    ├── txn_diffusion.sh                   # Legacy diffusion plotter
    ├── implementations/                   # Reference simulator impls (legacy)
    ├── output/                            # Per-suite artefact dirs (git-ignored)
    ├── parameters/                        # YAML configs (committed)
    │   ├── config.default.yaml
    │   ├── config.schema.json
    │   ├── linear.yaml                    # Base linear-Leios config
    │   ├── linear-with-tx-references.yaml
    │   ├── topology.default.yaml          # Multi-node default topology
    │   ├── 10x.yaml | 100x.yaml | 1000x.yaml
    │   ├── full.yaml | no-ibs.yaml | late-eb-attack.yaml | tx-references.yaml
    │   └── phase-2-sweep/                 # All phase-2 configs
    │       ├── protocol-base.yaml         # Phase-2 protocol baseline
    │       ├── protocol-rb-reduced-{half,third,quarter}.yaml  # RB-scarcity overlays
    │       ├── topology-single-producer.yaml
    │       ├── topology-cip-realistic.yaml
    │       ├── demand/                    # Actor profiles
    │       ├── pricing/                   # 13 controller-tuning YAMLs
    │       ├── experiments/               # CIP-topology generation tooling
    │       └── suites/                    # 21 suite YAMLs (M3 + M4 + smoke)
    │           ├── phase-2-*.yaml         # Phase-2 suites (M3/M4)
    │           ├── phase-2-{rb-scarcity,urgency-inversion}.README.md
    │           └── .goldens/              # Suite-level SHA256 baselines
    ├── scripts/                           # Shell wrappers for bulk runs
    │   ├── run-parallel-suites.sh         # Top-level batch runner
    │   ├── run-m6-{full-sweep-100,variance}.sh
    │   ├── run-smoke-*.sh                 # Smoke regressions
    │   ├── watch-suite-progress.sh
    │   └── generate-cip-topology.py
    ├── target/                            # Cargo build output (git-ignored)
    ├── test_data/                         # Legacy test fixtures
    ├── sim-core/                          # Crate: protocol + pricing kernel
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs                     # Re-exports the 7 modules
    │       ├── model.rs                   # Transaction, Block, EB, RB, ledger types
    │       ├── config.rs                  # RawParameters, SimConfiguration, all serde
    │       ├── events.rs                  # Event enum + EventTracker
    │       ├── probability.rs             # FloatDistribution wrapper
    │       ├── tx_actors.rs               # Demand-side actor model
    │       ├── clock.rs / clock/          # Virtual clock + coordinator
    │       │   ├── coordinator.rs
    │       │   ├── mock.rs                # Mock clock for tests
    │       │   └── timestamp.rs
    │       ├── network.rs / network/      # netsim-async wrapper
    │       │   ├── connection.rs
    │       │   └── coordinator.rs
    │       ├── tx_pricing/                # The phase-2 pricing kernel
    │       │   ├── mod.rs                 # PricingBackend trait, Lane, samples
    │       │   ├── window.rs              # CapacityWeightedWindow
    │       │   ├── single_lane.rs         # Baseline + Eip1559
    │       │   └── two_lane.rs            # TwoLanePricing + 4 variants
    │       └── sim.rs / sim/              # Simulation orchestration
    │           ├── driver.rs              # NodeDriver<N> event loop
    │           ├── cpu.rs                 # CPU task queue + subtasks
    │           ├── slot.rs                # SlotWitness
    │           ├── tx.rs                  # TransactionProducer
    │           ├── lottery.rs             # VRF stake probabilities
    │           ├── mempool_gate.rs        # Admission/eviction/charging
    │           ├── linear_leios.rs        # The phase-2 NodeImpl
    │           ├── linear_leios/
    │           │   └── attackers.rs       # EB-withholding attacker hook
    │           ├── leios.rs               # Legacy short-Leios NodeImpl
    │           ├── stracciatella.rs       # Legacy full-without-IBs NodeImpl
    │           └── tests/                 # M1-M3 deterministic scenarios
    │               ├── m1_smoke.rs
    │               ├── m2_two_lane.rs
    │               ├── m3_actors.rs
    │               ├── linear_leios.rs
    │               └── mod.rs
    └── sim-cli/                           # Crate: driver + metrics + binaries
        ├── Cargo.toml
        ├── build.rs                       # vergen git-SHA stamping
        ├── test_data/
        │   └── distribution.toml
        ├── src/
        │   ├── lib.rs                     # Re-exports metrics/runner/suite
        │   ├── main.rs                    # Legacy `sim-cli` single-run bin
        │   ├── runner.rs                  # Manifest, run_suite, run_job, verify
        │   ├── suite.rs                   # Suite YAML schema
        │   ├── events.rs                  # Legacy event monitor (single-run)
        │   ├── events/
        │   │   ├── aggregate.rs
        │   │   └── liveness.rs
        │   ├── metrics/                   # Phase-2 welfare layer
        │   │   ├── mod.rs
        │   │   ├── collector.rs           # MetricsCollector + RunSummary
        │   │   ├── time_series.rs         # CSV writer
        │   │   ├── diagnostics.rs         # Diagnostics log writer
        │   │   └── comparison.rs          # Per-suite metrics_comparison.txt
        │   └── bin/
        │       ├── experiment-suite/
        │       │   └── main.rs            # `experiment-suite run|status|verify`
        │       └── gen-test-data/
        │           ├── main.rs
        │           ├── strategy.rs
        │           └── strategy/
        │               ├── globe.rs
        │               ├── organic.rs
        │               ├── random_graph.rs
        │               ├── simplified.rs
        │               └── utils.rs
        └── tests/
            └── determinism.rs             # M5 suite-level goldens (`#[ignore]`'d)
```

## Directory Purposes

**`docs/phase-2/`:**
- Purpose: All phase-2 design + implementation notes. The mechanism spec, the rebuild plan, and per-milestone deltas live here.
- Contains: Markdown only — no code, no configs.
- Key files: `mechanism-design.md` (spec), `implementation-plan.md` (rebuild plan), `m{1..5}-handoff.md` (deltas), `calibration-fix-postmortem.md`.

**`sim-rs/`:**
- Purpose: Cargo workspace root. All Rust code, all configs, all scripts live under here. The "working directory" for all `cargo` / `experiment-suite` commands.
- Contains: Two crates (`sim-core`, `sim-cli`), parameters, scripts, output.
- Key files: `Cargo.toml` (workspace manifest), `parameters/phase-2-sweep/protocol-base.yaml`.

**`sim-rs/sim-core/`:**
- Purpose: The pure protocol-and-pricing kernel. No CLI, no I/O beyond what `tokio` and `netsim-async` need internally. Imported by `sim-cli`.
- Contains: `lib.rs` + 7 modules (`model`, `config`, `events`, `probability`, `tx_actors`, `tx_pricing`, `sim`, `clock`, `network`).
- Key files: `src/tx_pricing/mod.rs` (trait), `src/sim/linear_leios.rs` (protocol), `src/sim/mempool_gate.rs` (admission), `src/tx_actors.rs` (demand).

**`sim-rs/sim-core/src/tx_pricing/`:**
- Purpose: Policy-only fee controller(s). No simulator types reach this module.
- Contains: 4 files totalling ~1,440 lines.
- Key files: `mod.rs` (`PricingBackend` trait, `Lane`, `PricedBlockSample`, `BlockLaneBreakdown`, `Multiplier`), `window.rs` (`CapacityWeightedWindow`), `single_lane.rs` (`BaselinePricing` + `Eip1559Pricing`), `two_lane.rs` (`TwoLanePricing` + 4 `TwoLaneVariant` arms).

**`sim-rs/sim-core/src/sim/`:**
- Purpose: Simulation orchestration + per-variant `NodeImpl`s.
- Contains: One file per protocol variant + shared driver/cpu/slot/tx/lottery + `mempool_gate.rs` + `tests/`.
- Key files: `linear_leios.rs` (~2,750 lines — the only phase-2 protocol), `mempool_gate.rs` (admission/eviction/charging), `driver.rs` (per-node event loop), `tx.rs` (TransactionProducer + actor sampling), `lottery.rs` (VRF stake).

**`sim-rs/sim-cli/`:**
- Purpose: CLI binaries + welfare-metrics layer + suite runner. Where all f64 reporting lives.
- Contains: `lib.rs` (re-exports) + `runner.rs` + `suite.rs` + `metrics/` + two binaries (`experiment-suite`, `gen-test-data`) + legacy `main.rs`.
- Key files: `src/runner.rs` (resumable batch runner), `src/metrics/collector.rs` (event consumer), `src/bin/experiment-suite/main.rs` (CLI).

**`sim-rs/parameters/phase-2-sweep/`:**
- Purpose: All phase-2 YAML configs (protocols, topologies, demand profiles, pricing TOMLs, suite definitions).
- Contains: Top-level protocols + topologies, plus `demand/`, `pricing/`, `suites/` subdirs.
- Key files: `protocol-base.yaml` (baseline), `topology-single-producer.yaml` (one-node), `suites/phase-2-*.yaml` (7 phase-2 suites + smoke variants).

**`sim-rs/parameters/phase-2-sweep/suites/.goldens/`:**
- Purpose: One SHA-256 per phase-2 suite, asserted by `sim-cli/tests/determinism.rs` against re-runs of the canonical (job, seed=1) baseline.
- Contains: 7 `.sha256` files (one per phase-2 suite).
- Key files: `phase-2-{eip1559-robustness,eip1559-smoothing,priority-only-rb-reserved,priority-only-unreserved,two-lane-both-dynamic,rb-scarcity,urgency-inversion}.sha256`.

**`sim-rs/scripts/`:**
- Purpose: Shell wrappers for parallel suite execution, smoke runs, watch helpers.
- Contains: `run-parallel-suites.sh` (top-level), per-suite smoke scripts, `watch-suite-progress.sh`, `generate-cip-topology.py`.
- Key files: `run-parallel-suites.sh` (the canonical batch entry point; generates one `run_id` per invocation and passes it to all spawned suites).

**`sim-rs/output/`:**
- Purpose: Per-suite output dirs (git-ignored). Each suite run writes a tree of `<job>/<seed>/{run_summary.json, pricing_event_stream.sha256, time_series.csv, diagnostics.log}` plus the suite-level `manifest.json` and `metrics_comparison.txt`.
- Contains: Live run artefacts. Not committed.

**`sim-rs/sim-cli/tests/`:**
- Purpose: Cross-crate integration tests, including the slow `#[ignore]`'d determinism goldens.
- Contains: `determinism.rs`.

**`sim-rs/sim-core/src/sim/tests/`:**
- Purpose: Deterministic scenario tests pinned by per-milestone golden constants. M1 smoke, M2 two-lane, M3 actors, plus linear-Leios-specific tests.
- Contains: `m1_smoke.rs`, `m2_two_lane.rs`, `m3_actors.rs`, `linear_leios.rs`, `mod.rs`.

## Key File Locations

**Entry Points:**
- `sim-rs/sim-cli/src/bin/experiment-suite/main.rs`: Phase-2 suite-runner CLI (`run`/`status`/`verify`).
- `sim-rs/sim-cli/src/main.rs`: Legacy single-run `sim-cli` binary (pre-phase-2 default-run).
- `sim-rs/sim-cli/src/bin/gen-test-data/main.rs`: Topology-generation utility.
- `sim-rs/sim-core/src/sim.rs` (`Simulation::new` / `run`): Library entry point used by the runner.
- `sim-rs/sim-core/src/sim/driver.rs` (`NodeDriver::run`): Per-node async task entry.

**Configuration:**
- `sim-rs/Cargo.toml`: Workspace manifest.
- `sim-rs/sim-core/Cargo.toml`, `sim-rs/sim-cli/Cargo.toml`: Crate manifests.
- `sim-rs/parameters/phase-2-sweep/protocol-base.yaml`: Phase-2 protocol baseline (`min-fee-a`, `min-fee-b`, `rb-generation-probability`, `vote-*`, `linear-*-stage-length-slots`, etc.).
- `sim-rs/parameters/phase-2-sweep/protocol-rb-reduced-{half,third,quarter}.yaml`: RB-scarcity full replacements.
- `sim-rs/parameters/phase-2-sweep/topology-single-producer.yaml`, `topology-cip-realistic.yaml`: Topology overlays.
- `sim-rs/parameters/topology.default.yaml`: Default multi-node topology.
- `sim-rs/parameters/phase-2-sweep/demand/*.yaml`: Actor profiles (paper_like_*, sundaeswap_moderate).
- `sim-rs/parameters/phase-2-sweep/pricing/*.yaml`: Controller settings (eip1559_*, two_lane_*).
- `sim-rs/parameters/phase-2-sweep/suites/*.yaml`: Suite definitions (M3/M4 + smoke).

**Core Logic:**
- `sim-rs/sim-core/src/tx_pricing/mod.rs`: `PricingBackend` trait, `Lane`, `PricedBlockSample`, `BlockLaneBreakdown`, `Multiplier`, `LaneValidityRule`, `LaneSelectionOrder`.
- `sim-rs/sim-core/src/tx_pricing/window.rs`: `CapacityWeightedWindow` (u128 ring, Σbytes/Σcapacity).
- `sim-rs/sim-core/src/tx_pricing/single_lane.rs`: `BaselinePricing`, `Eip1559Pricing`, `Eip1559Settings`.
- `sim-rs/sim-core/src/tx_pricing/two_lane.rs`: `TwoLanePricing`, `TwoLaneSettings`, `TwoLaneVariant` (4 arms).
- `sim-rs/sim-core/src/sim/mempool_gate.rs`: `MempoolGate`, `AdmissionRejection`, `EvictionRecord`, `InclusionCharge`.
- `sim-rs/sim-core/src/sim/linear_leios.rs`: `LinearLeiosNode`, `Mempool`, `select_eb_with_partition`, `eb_endorsement_valid`, `sample_from_mempool_lane_aware`, `breakdown_for`.
- `sim-rs/sim-core/src/tx_actors.rs`: `ActorComponent`, `ActorProfile`, `MaxFeePolicy` (`ScaledOverLaneQuote`, `VolatilityAware`), `LanePolicy`, `LatencyEstimator`, `welfare` module.
- `sim-rs/sim-core/src/model.rs`: `Transaction`, `Block`, `LinearRankingBlock`, `LinearEndorserBlock`, id wrappers.
- `sim-rs/sim-core/src/events.rs`: `Event` enum, `EventTracker`, `Node` wrapper.
- `sim-rs/sim-core/src/config.rs`: `RawParameters`, `RawTopology`, `Topology`, `SimConfiguration`, `NodeConfiguration`, all serde structs (kebab-case).
- `sim-rs/sim-cli/src/runner.rs`: `Manifest`, `JobEntry`, `run_suite`, `run_job`, `verify_suite`, figment config composition.
- `sim-rs/sim-cli/src/suite.rs`: `Suite`, `Job`, `JobOverrides`.
- `sim-rs/sim-cli/src/metrics/collector.rs`: `MetricsCollector`, `ComponentSummary`, `RunSummary`, SHA-256 hashing.

**Testing:**
- `sim-rs/sim-core/src/sim/tests/m1_smoke.rs`: M1 baseline smoke.
- `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs`: M2 two-lane goldens (pinned constants).
- `sim-rs/sim-core/src/sim/tests/m3_actors.rs`: M3 actor-model goldens.
- `sim-rs/sim-core/src/sim/tests/linear_leios.rs`: Protocol-level tests.
- `sim-rs/sim-cli/tests/determinism.rs`: M5 suite-level SHA-256 goldens (`#[ignore]`'d).

**Documentation:**
- `CLAUDE.md`: Project-level context (read first by any agent).
- `docs/phase-2/mechanism-design.md`: The spec.
- `docs/phase-2/implementation-plan.md`: The rebuild plan.
- `docs/phase-2/m{1..5}-handoff.md`: Per-milestone delta notes.
- `docs/phase-2/calibration-fix-postmortem.md`: rb-prob=1.0 bug post-mortem.
- `sim-rs/parameters/phase-2-sweep/suites/phase-2-rb-scarcity.README.md`, `phase-2-urgency-inversion.README.md`: M4 suite framing.

## Naming Conventions

**Files (Rust):**
- snake_case (`mempool_gate.rs`, `linear_leios.rs`, `tx_pricing/mod.rs`, `time_series.rs`).
- Per-milestone test files prefixed `m<N>_` (`m1_smoke.rs`, `m2_two_lane.rs`, `m3_actors.rs`).
- Binary entry: `main.rs` under `src/bin/<binary-name>/`.

**Files (configs):**
- Top-level configs: kebab-case (`protocol-base.yaml`, `topology-single-producer.yaml`, `topology-cip-realistic.yaml`).
- Suite YAMLs: `phase-2-<topic>.yaml` (e.g. `phase-2-eip1559-robustness.yaml`).
- Suite READMEs: `<suite-name>.README.md` next to the YAML.
- Pricing tuning files: `<mechanism>_<knobs>.yaml` with underscores (`eip1559_d8_target0.5_window32.yaml`, `two_lane_priority_only_static_x16.yaml`).
- Demand profiles: `paper_like_<regime>.yaml`, `sundaeswap_<regime>.yaml`.
- Smoke shell scripts: `run-smoke-<scope>.sh`.
- Golden artefacts: `<suite-name>.sha256` under `.goldens/`.

**Directories:**
- kebab-case under `sim-rs/parameters/` (`phase-2-sweep/`, `phase-2-sweep/suites/`, `phase-2-sweep/pricing/`).
- snake_case under `sim-rs/sim-core/src/` and `sim-rs/sim-cli/src/` (`tx_pricing/`, `bin/experiment-suite/`).
- Cargo binaries live in `src/bin/<bin-name>/` (note the trailing directory, with `main.rs` inside) when they pull in helper modules; single-file binaries use `src/bin/<bin-name>.rs`.

**Rust types/functions:**
- `PascalCase` for types/traits/enums (`PricingBackend`, `MempoolGate`, `TwoLaneVariant`).
- `snake_case` for functions/modules/fields (`current_quote`, `update_after_block`, `sample_from_mempool_lane_aware`).
- `SCREAMING_SNAKE_CASE` for consts (`RUN_SUMMARY_FILE`, `MAX_VOLATILITY_AWARE_BLOCKS`).

**Serde casing:**
- Most YAML-facing types: `#[serde(rename_all = "kebab-case")]` (`RawParameters`, `Suite`, `Job`, `JobOverrides`, `Manifest`, `JobEntry`, `Lane`, `BlockKind`, `LaneSelectionOrder`).
- `RunSummary` (`sim-cli/src/metrics/collector.rs`): no `rename_all`; Rust snake_case appears on disk. Heterogeneous by historical accident — match the surrounding type's convention when adding fields.
- Tagged enum variants use `tag = "kind"` or `tag = "type"` + kebab-case for variant names (`Event`, `MaxFeePolicy`, `DistributionConfig`).

## Where to Add New Code

**New pricing controller (single- or multi-lane):**
- Implementation: new file under `sim-rs/sim-core/src/tx_pricing/` (e.g. `three_lane.rs`). Re-export from `sim-rs/sim-core/src/tx_pricing/mod.rs`.
- Wire into `PricingBackend`: implement the trait; ensure all state is `u64`/`u128`/integer-rational (no f64 in hot paths).
- Config plumbing: extend `PricingConfig` in `sim-rs/sim-core/src/config.rs` with a new variant; add validation; add a pricing YAML under `sim-rs/parameters/phase-2-sweep/pricing/`.
- Tests: add a `m<N>_<topic>.rs` scenario under `sim-rs/sim-core/src/sim/tests/` and register it in `mod.rs`.

**New protocol variant (alongside `linear`):**
- Implementation: new file under `sim-rs/sim-core/src/sim/` (e.g. `<variant>.rs`) implementing `NodeImpl`.
- Wire into `Simulation::new` (`sim-rs/sim-core/src/sim.rs`): add a `LeiosVariant::<New>` arm and corresponding `NetworkWrapper` / `NodeListWrapper` enum members.
- Config: extend `LeiosVariant` in `sim-rs/sim-core/src/config.rs`.
- Tests: scenario file under `sim-rs/sim-core/src/sim/tests/`.

**New metrics column / welfare term:**
- Implementation: extend `ComponentSummary` or `TimeSeriesRow` in `sim-rs/sim-cli/src/metrics/collector.rs`. f64 is OK here — this is the reporting layer.
- Writer: extend `time_series::write_row` (`sim-rs/sim-cli/src/metrics/time_series.rs`) and/or `comparison::write_suite` (`sim-rs/sim-cli/src/metrics/comparison.rs`).
- Critically: do NOT let the new field feed back into simulation decisions; the f64 prohibition is enforced by the golden hashes.

**New actor `MaxFeePolicy` or `LanePolicy` variant:**
- Implementation: add variant in `MaxFeePolicy` / `LanePolicy` (`sim-rs/sim-core/src/tx_actors.rs`); extend `MaxFeePolicy::validate` and `MaxFeePolicy::compute`.
- Config: serde-tagged variant uses `#[serde(tag = "kind", rename_all = "kebab-case")]` — the variant name on disk is kebab-case.
- Tests: extend `m3_actors.rs`.

**New suite (sweep over existing mechanism):**
- Implementation: new YAML under `sim-rs/parameters/phase-2-sweep/suites/phase-2-<topic>.yaml`.
- Schema: `suite-name`, `output-dir`, `seeds`, `default-slots`, `default-topology`, `default-protocol`, `default-demand`, `jobs[]` (each with `name` + `pricing` + optional `overrides`).
- Golden: after the first canonical run, capture the (job 0, seed 1) hash and write it to `.goldens/<suite-name>.sha256`. Register the new suite in `sim-rs/sim-cli/tests/determinism.rs` if you want it gate-tested.
- README: if the suite's framing isn't self-evident, add `<suite-name>.README.md` next to the YAML.

**New event variant:**
- Implementation: add to `Event` enum in `sim-rs/sim-core/src/events.rs`. Use `#[serde(tag = "type")]` + kebab-case for the variant tag.
- Emit from the producing node in `sim-rs/sim-core/src/sim/linear_leios.rs` via `EventTracker::send`.
- Consume in `sim-rs/sim-cli/src/metrics/collector.rs` if the event affects metrics.
- Hashing: only `TXIncluded` and `TXEvictedQuoteDrift` feed the pricing-event-stream SHA-256 (see `MetricsCollector::ingest`). Adding a new event there flips every golden.

**New CLI subcommand for `experiment-suite`:**
- Implementation: extend the `Command` enum in `sim-rs/sim-cli/src/bin/experiment-suite/main.rs` and dispatch in `main`.
- Library glue: add a `pub fn <subcommand>_suite_with_run_id` in `sim-rs/sim-cli/src/runner.rs`.

**Shared helpers / utilities:**
- Pricing math helpers: `sim-rs/sim-core/src/tx_pricing/mod.rs` or a sibling file under `tx_pricing/`.
- Actor math helpers: `sim-rs/sim-core/src/tx_actors.rs` (note `ceil_div_u128` already lives there).
- Probability/distribution: `sim-rs/sim-core/src/probability.rs`.
- Metrics aggregation: `sim-rs/sim-cli/src/metrics/`.

## Special Directories

**`sim-rs/target/`:**
- Purpose: Cargo build output (debug + release).
- Generated: Yes (`cargo build`).
- Committed: No (git-ignored).

**`sim-rs/output/`:**
- Purpose: Per-suite run artefacts. Tree shape: `output/phase-2/<suite-name>/<job-name>/<seed>/{run_summary.json, pricing_event_stream.sha256, time_series.csv, diagnostics.log}` plus `<suite-name>/{manifest.json, metrics_comparison.txt}`.
- Generated: Yes (by `experiment-suite run`).
- Committed: No (git-ignored).

**`sim-rs/parameters/phase-2-sweep/suites/.goldens/`:**
- Purpose: Committed SHA-256 baselines, one per phase-2 suite. Asserted by `sim-cli/tests/determinism.rs` against the canonical (job 0, seed 1) baseline.
- Generated: Yes (regenerable via `UPDATE_GOLDENS=1 cargo test --release -- --ignored determinism`).
- Committed: **Yes** — flipping a golden is an intentional act tracked by git (and typically tagged).

**`sim-rs/test_data/`, `sim-rs/sim-cli/test_data/`:**
- Purpose: Legacy/static fixtures (e.g. `distribution.toml`).
- Generated: No.
- Committed: Yes.

**`sim-rs/implementations/`:**
- Purpose: Reference simulator implementations carried over from upstream — not phase-2 code.
- Generated: No.
- Committed: Yes (legacy).

**`sim-rs/sim-rs/`:**
- Purpose: Leftover sub-tree (contains `output/`, `parameters/`, `sim-cli/`). Vestigial — the canonical workspace root is `sim-rs/` one level up.
- Generated: No.
- Committed: Yes (legacy; don't add new files here).

**`.planning/codebase/`:**
- Purpose: GSD codebase-map outputs (this document + ARCHITECTURE.md and any others).
- Generated: Yes (by the codebase-mapper agent).
- Committed: At the operator's discretion.

---

*Structure analysis: 2026-05-13*
