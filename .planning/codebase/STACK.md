# Technology Stack

**Analysis Date:** 2026-05-15

## Languages

**Primary:**
- Rust (edition 2024) — the entire simulator. MSRV pinned to `rust-version = "1.88"` in both crate manifests (`sim-rs/sim-core/Cargo.toml`, `sim-rs/sim-cli/Cargo.toml`). Edition 2024 enables `let … else` chains and other 2024-only features used throughout (see e.g. `if let Some(n) = opt && n >= 1` in `sim-rs/sim-cli/src/bin/experiment-suite/main.rs`).

**Secondary:**
- Python 3 — one-shot topology generators under `sim-rs/scripts/` (`generate-realistic-100-topology.py`, `generate-cip-topology.py`). Not part of the build; produces the committed `parameters/phase-2-sweep/topology-*.yaml` artefacts.
- Bash — orchestration shell scripts under `sim-rs/scripts/` (`run-parallel-suites.sh`, `run-m6-full-sweep-100.sh`, smoke wrappers).

## Runtime

**Environment:**
- Native binary, no managed runtime. Built locally via `cargo build --release` from `sim-rs/`. Async work is driven by `tokio` with per-thread `current_thread` runtimes (the `Simulation` future contains `Box<dyn Actor>` which is `!Send`; see `sim-rs/sim-cli/src/runner.rs:240`).

**Package Manager:**
- Cargo (Rust toolchain). Lockfile: `sim-rs/Cargo.lock` (version 4, committed).

## Frameworks

**Workspace layout:**

The workspace is declared at `sim-rs/Cargo.toml` with two members:

| Member | Path | Role |
|---|---|---|
| `sim-core` | `sim-rs/sim-core/` | Protocol + pricing kernel library |
| `sim-cli`  | `sim-rs/sim-cli/`  | Driver library + binaries (`sim-cli`, `experiment-suite`) |

Both crates are versioned `1.4.0` in lockstep.

**Binaries (declared in `sim-rs/sim-cli/Cargo.toml`):**

| Binary | Path | Purpose |
|---|---|---|
| `sim-cli` | `sim-rs/sim-cli/src/main.rs` | Legacy single-run driver (loads `parameters/topology.default.yaml` + a YAML overlay list, runs one simulation, writes a JSON/CBOR/gzip event trace). |
| `experiment-suite` | `sim-rs/sim-cli/src/bin/experiment-suite/main.rs` | Phase-2 suite runner with `run | status | verify` subcommands. The primary phase-2 tool. |
| `gen-test-data` | `sim-rs/sim-cli/src/bin/gen-test-data/main.rs` | Synthetic topology generator (legacy, pre-phase-2). |

**Testing:**
- Built-in `cargo test` harness. No third-party test framework.
- Determinism goldens at three layers:
  - Unit-test goldens: `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs`, `m3_actors.rs` — pinned SHA-256 constants in source.
  - Verify subcommand: `experiment-suite verify <suite.yaml>` re-runs every `Completed` (job, seed) and asserts the freshly-computed `pricing_event_stream.sha256` equals the persisted on-disk value.
  - Suite-level goldens: `sim-rs/sim-cli/tests/determinism.rs`, `#[ignore]`'d by default; goldens committed at `sim-rs/parameters/phase-2-sweep/suites/.goldens/<suite>.sha256`.

**Build/Dev:**
- `vergen-gitcl` (build-dependency) — at compile time embeds the git SHA via `sim-rs/sim-cli/build.rs`, surfaced as `env!("VERGEN_GIT_SHA")` in `--version` output of `sim-cli` and `experiment-suite`.

## Key Dependencies

**Async runtime / concurrency:**
- `tokio = "1"` (features `macros` in `sim-core`, `full` in `sim-cli`) — async runtime. Used for the simulator event loop, channels (`mpsc::unbounded_channel`), and the per-job per-thread `current_thread` runtimes in the suite runner.
- `tokio-util = "0.7"` — `CancellationToken` for cooperative shutdown on SIGINT.
- `futures = "0.3"` — `BoxFuture` plumbing for the `Actor` trait (`sim-rs/sim-core/src/sim.rs:4`).
- `async-stream = "0.3"` (in `sim-core`) — stream construction helpers.

**Networking simulation:**
- `netsim-async` (git, `input-output-hk/ce-netsim`, rev `9d1e26c`) — IOG's network simulator, used inside `sim-core` for message-passing and `ClockCoordinator` (`sim-rs/sim-core/src/clock.rs:10`, `sim-rs/sim-core/src/network.rs:4`). Both `sim-core` and `sim-cli` pin the same revision.
- `netsim-core` (git, same repo and revision, in `sim-cli`) — `geo::Location` and `latency_between_locations` for the test-data generator's geographic latency model (`sim-rs/sim-cli/src/bin/gen-test-data/strategy/utils.rs:6`).

**RNG (determinism-critical):**
- `rand = "0.9"` — RNG traits.
- `rand_chacha = "0.9"` — `ChaCha20Rng` / `ChaChaRng` is the only RNG used for simulation state. All per-task RNGs are seeded from `ChaChaRng::seed_from_u64(config.seed)` in `sim-rs/sim-core/src/sim.rs:178`. Plain `rand::ThreadRng` is never used in simulation-affecting code.
- `rand_distr = "0.5"` — distribution sampling (used by `sim-core/src/probability.rs`, `tx_actors.rs`, etc).

**Bit-deterministic math:**
- `libm = "0.2"` — software-implemented `pow`, `round`, `exp`, `ceil`, `sqrt`. Substituted for the hardware-dependent `f64` methods anywhere a value crosses into simulation-affecting state (e.g. the actor lane-choice math, `sim-rs/sim-core/src/tx_actors.rs:378–389`, where `libm::pow` and `libm::round` route an `f64` into `i128` lovelace before comparison). The cross-arch determinism contract relies on this.
- `num-traits = "0.2"` — integer trait helpers.

**Serialization:**
- `serde = "1"` (features `derive`) — derive-based serialization across both crates.
- `serde_yaml = "0.9"` — YAML parsing for every config file (`Suite::load`, `RawTopology` parse, `RawParameters` extraction).
- `serde_json = "1"` (in `sim-cli`) — `manifest.json`, `run_summary.json`, JSONL event traces.
- `figment = "0.10"` (features `yaml`, `toml`) — config layering. `Figment::new().merge(Yaml::string(...)).merge(Yaml::file_exact(...))` composes the suite's config overlay stack in `sim-rs/sim-cli/src/runner.rs:840`. File-extension dispatch in `merge_layer` (`runner.rs:798`) accepts `.toml` and `.yaml`/`.yml` overlays.
- `toml = "0.9"` (in `sim-cli`) — TOML support for figment.
- `minicbor-serde = "0.6"` (features `alloc`) — CBOR encoding for trace events when `--output *.cbor` (`sim-rs/sim-cli/src/events.rs:770`).
- `async-compression = "0.4"` (features `tokio`, `gzip`) — `GzipEncoder` for `.jsonl.gz` / `.cbor.gz` trace outputs (`sim-rs/sim-cli/src/events.rs:5`).

**Hashing (determinism contract):**
- `sha2 = "0.10"` — `Sha256` digest of the pricing event stream. Incremental hashing of `TXIncluded` + `TXEvictedQuoteDrift` events in `sim-rs/sim-cli/src/metrics/collector.rs:272`; persisted as `pricing_event_stream.sha256` per (job, seed). Same digest used in M2/M3 unit-test goldens (`sim-rs/sim-core/src/sim/tests/m2_two_lane.rs:21`).
- `hex = "0.4"` — hex-encoding for digest output (`hex::encode(hasher.finalize())`).

**Data structures / utilities:**
- `dashmap = "6"` — concurrent hashmap (used internally by `sim-core` for per-node state).
- `priority-queue = "2"` — used by the simulator scheduler.
- `anyhow = "1"` — error type for application code (both crates).
- `chrono = "0.4"` (features `serde`) — UTC timestamps in `manifest.json` (`started_at_utc`, `completed_at_utc`).
- `itertools = "0.14"` — iterator combinators in `sim-cli`.
- `ctrlc = "3"` — SIGINT handler for the legacy `sim-cli` binary.
- `pretty-bytes-rust = "0.3"` — human-readable byte formatting in legacy event monitor.

**CLI:**
- `clap = "4"` (features `derive`) — `Args` derives in `sim-rs/sim-cli/src/main.rs:39` and `sim-rs/sim-cli/src/bin/experiment-suite/main.rs:20`.

**Logging / observability:**
- `tracing = "0.1"` — structured logging across both crates.
- `tracing-subscriber = "0.3"` (features `env-filter`) — initialised in both binaries' `main` (`fmt_layer().compact().without_time()` + `EnvFilter` from `RUST_LOG`).

**Statistics:**
- `statrs = "0.18"` — `Beta` and `ContinuousCDF` for the test-data generator's latency-distribution model (`sim-rs/sim-cli/src/bin/gen-test-data/strategy/utils.rs:9`).
- `average = "0.16"` — `Variance` for live message-rate aggregation in the legacy event monitor (`sim-rs/sim-cli/src/events.rs:6`).

**Dev-only:**
- `tempfile = "3"` (sim-cli dev-dep) — per-test scratch dirs in `sim-rs/sim-cli/tests/determinism.rs` and `parallel_runner.rs`.
- `hex`, `serde_yaml`, `sha2`, `tokio = { rt }` are dev-deps of `sim-core` for the M2/M3 test fixtures.

**No `criterion`, `proptest`, `quickcheck`, or external HTTP/database clients.** The simulator is a pure pre-compute: it ingests YAML, runs an in-process discrete-event simulation, and writes flat files. No live network I/O, no async HTTP client, no DB driver.

## Configuration

**Environment:**
- `RUST_LOG` — tracing-subscriber `EnvFilter` directive; default level `INFO`.
- `UPDATE_GOLDENS=1` — flips `sim-rs/sim-cli/tests/determinism.rs` from "assert against golden" to "write fresh hash to disk" mode (used after intentional simulator changes; see `CLAUDE.md` "Running the suites").
- `M6_RUN_ID` — read by `sim-rs/scripts/run-parallel-suites.sh` to share a batch identifier across concurrent suites (default: UTC `YYYYMMDD-HHMMSS`).
- `CARGO_PKG_VERSION`, `VERGEN_GIT_SHA` — compile-time env vars consumed by the `--version` strings; populated by cargo and `sim-rs/sim-cli/build.rs` respectively.

No `.env` file; no runtime secrets; no API keys anywhere in the codebase.

**Build:**
- `sim-rs/Cargo.toml` — workspace root; `resolver = "2"`; `[profile.release]` keeps `debug = true` so flamegraphs / `perf` symbolicate against release binaries.
- `sim-rs/sim-cli/build.rs` — emits `cargo:rustc-env=VERGEN_GIT_SHA=…` via `vergen-gitcl`.

## Platform Requirements

**Development:**
- Rust toolchain ≥ 1.88 (matches the `rust-version` MSRV).
- x86_64 / glibc Linux is the reference architecture for the pinned golden hashes (see `CLAUDE.md` "Determinism is intra-arch"). The math layer (libm + u128 rationals + integer arithmetic) is bit-stable across architectures by construction, but cross-arch CI verification is not yet built.
- Memory: peak RSS scales linearly in `--parallelism`; the default cap of 8 stays under 32 GB with the 100-node topology.
- Python 3 with `pyyaml` (and Koios API reachability) is needed only to regenerate `topology-realistic-100.yaml` — not part of the build path; the YAML is a checked-in artefact.

**Production:**
- Not applicable. This is an offline simulator; outputs are flat files for analysis, not a deployed service.

## Build & Test Commands

All commands assume `pwd = sim-rs/`.

```sh
# Build the release binaries.
cargo build --release

# Standard test cycle (excludes the slow #[ignore]'d determinism goldens).
cargo test --workspace

# Slow suite-level determinism goldens (~1.5s in --release, 7 suites).
cargo test --release -- --ignored determinism

# Regenerate the suite goldens after an intentional simulator change.
UPDATE_GOLDENS=1 cargo test --release -- --ignored determinism

# Run a phase-2 suite end-to-end (resumable).
cargo run --release --bin experiment-suite -- run \
    parameters/phase-2-sweep/suites/phase-2-eip1559-robustness.yaml

# Verify determinism on a previously-run suite.
cargo run --release --bin experiment-suite -- verify \
    parameters/phase-2-sweep/suites/phase-2-eip1559-robustness.yaml

# Inspect manifest status.
cargo run --release --bin experiment-suite -- status \
    parameters/phase-2-sweep/suites/phase-2-eip1559-robustness.yaml
```

## Parameters / YAML Configuration System

The simulator is configured by a layered stack of YAML overlays composed at runtime via `figment` (`sim-rs/sim-cli/src/runner.rs:840`). For each (job, seed) pair the runner merges, in order:

1. **Embedded base** — `parameters/config.default.yaml`, compiled into the binary via `include_str!` (`sim-rs/sim-cli/src/runner.rs:840-842`). Provides every field with a sensible default; shared with the upstream Haskell simulator.
2. **Protocol overlay** — `parameters/phase-2-sweep/protocol-base.yaml` (or one of `protocol-rb-reduced-{half,third,quarter}.yaml` for the M4 RB-scarcity suite). Sets phase-2-specific protocol knobs (min-fee-a/b, mempool cap, RB-generation probability, vote-generation parameters, CIP-0164 Table 7 stage lengths).
3. **Demand overlay** — `parameters/phase-2-sweep/demand/<profile>.yaml`. Declares the actor `components:` block: per-component arrival rate (phased), size/value/half-life log-normal distributions, max-fee policy.
4. **Pricing overlay** — `parameters/phase-2-sweep/pricing/<config>.yaml`. Picks the `pricing.kind` (`baseline` | `eip1559` | `two-lane`) and its controller parameters (initial quote, target ratio, max-change-denominator, window length, multiplier floor, lane selection order).

The suite YAML itself (`Suite` struct, `sim-rs/sim-cli/src/suite.rs:17`) declares:

- `suite-name`, `output-dir`
- `seeds: [u64]`
- `default-slots`, `default-topology`, `default-protocol`, `default-demand`
- `jobs: [{ name, pricing, overrides? }]`
- Optional per-job `overrides` for `slots`, `seeds`, `demand`, `topology`, `protocol` (replacement, not stacked — see `CLAUDE.md` "RB-reduced overlays are full replacements").

**Topology** is a separate YAML (`RawTopology`, `sim-rs/sim-core/src/config.rs:466`), loaded directly via `serde_yaml::from_str` (`sim-rs/sim-cli/src/runner.rs:828`) — it's not part of the figment stack because `Topology` validation happens before merging.

Format detection in `merge_layer` (`sim-rs/sim-cli/src/runner.rs:798`) accepts `.yaml`/`.yml` (default) and `.toml`; all phase-2 overlays are YAML.

Serde rename casing is mixed by historical accident: YAML configs and `Manifest`/`JobEntry` use `kebab-case` via `#[serde(rename_all = "kebab-case")]`; `RunSummary` uses Rust `snake_case`. See `CLAUDE.md` "Conventions / gotchas" for the migration rationale (standardising would invalidate every persisted on-disk artefact).

The parameters directory tree:

```
sim-rs/parameters/
├── config.default.yaml             # embedded base (compile-time include_str!)
├── config.schema.json              # JSON-schema for editor autocomplete
├── topology.default.yaml           # legacy multi-region topology (still used by sim-cli's default)
├── linear.yaml, full.yaml, ...     # legacy linear-leios / full-leios overlays
└── phase-2-sweep/                  # all phase-2 configs
    ├── protocol-base.yaml
    ├── protocol-rb-reduced-{half,third,quarter}.yaml
    ├── topology-single-producer.yaml
    ├── topology-realistic-100.yaml         # default since 2026-05-13
    ├── topology-cip-realistic.yaml         # 600-pool CIP-0164 baseline
    ├── demand/
    │   ├── paper_like_{light,moderate,congested,realistic,mispriced}.yaml
    │   └── sundaeswap_moderate.yaml
    ├── pricing/                            # 19 controller-tuning YAMLs
    │   ├── baseline_flat_fee.yaml
    │   ├── eip1559_d{4,8,16}_target{0.25,0.5,0.75}_window{16,32,64}.yaml
    │   └── two_lane_*_{x4,x8,x16}.yaml
    ├── suites/                             # 21 suite YAMLs
    │   ├── phase-2-{eip1559-*,priority-only-*,two-lane-*,rb-scarcity,urgency-inversion}.yaml
    │   ├── phase-2-{congested,moderate,realistic,sundaeswap}-*.yaml
    │   ├── *.README.md
    │   └── .goldens/<suite>.sha256         # the M5 pinned hashes
    └── experiments/                        # legacy single-run experiment configs
```

---

*Stack analysis: 2026-05-15*
