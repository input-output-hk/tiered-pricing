# Technology Stack

**Analysis Date:** 2026-05-13

## Languages

**Primary:**
- Rust (edition 2024, MSRV 1.88) — entire simulator and CLI. Pinned in `sim-rs/sim-core/Cargo.toml` and `sim-rs/sim-cli/Cargo.toml` via `edition = "2024"` and `rust-version = "1.88"`.

**Secondary:**
- YAML — protocol, topology, demand, pricing, and suite configuration. All inputs live under `sim-rs/parameters/`.
- Bash — operator scripts under `sim-rs/scripts/` (parallel-suite runner, smoke-test runners, suite progress watcher).
- Python 3 — topology generator only (`sim-rs/scripts/generate-cip-topology.py`). Not part of the build, not invoked during simulation. Run manually to regenerate `parameters/phase-2-sweep/topology-cip-realistic.yaml`.
- JSON — manifest persistence (`output/<suite>/manifest.json`) and a `parameters/config.schema.json` JSON-Schema describing the config surface.

## Runtime

**Environment:**
- Tokio async runtime, multi-threaded scheduler. `sim-cli` enables the `"full"` Tokio feature set (`sim-rs/sim-cli/Cargo.toml:39`); `sim-core` enables only `"macros"` and adds `"rt"` for tests (`sim-rs/sim-core/Cargo.toml:20-28`).
- Single OS process per simulator run; the `experiment-suite` driver loops sequentially over `(job, seed)` pairs and shells out parallelism via `scripts/run-parallel-suites.sh` (one suite per child process).

**Package Manager:**
- Cargo (Rust 1.88 toolchain).
- Lockfile: `sim-rs/Cargo.lock` is committed (2,217 lines). A second `Cargo.lock` exists at the repo root for the upstream-main remnants but the working build runs from `sim-rs/`.

## Frameworks

**Core:**
- `tokio` 1.47 — async runtime for simulator tasks, channels (`mpsc`, `oneshot`), and `JoinSet`-based fan-out. Used in `sim-rs/sim-core/src/sim.rs` to drive node tasks and in `sim-rs/sim-cli/src/main.rs` for the event monitor.
- `tokio-util` 0.7 — `CancellationToken` for graceful shutdown on ctrl+c (`sim-rs/sim-cli/src/main.rs:117`).
- `futures` 0.3 + `async-stream` 0.3 — stream combinators inside `sim-core`.

**CLI / Config:**
- `clap` 4 (derive feature) — argparse for both `sim-cli` (`sim-rs/sim-cli/src/main.rs:39-57`) and `experiment-suite` (`sim-rs/sim-cli/src/bin/experiment-suite/main.rs:20-56`).
- `figment` 0.10 (yaml + toml features) — layered config merging. `sim-cli` merges the embedded `parameters/config.default.yaml` with caller-supplied overlays; `runner.rs` uses the same approach to stack protocol → demand → pricing YAMLs per job (`sim-rs/sim-cli/src/runner.rs:23-26`).
- `serde` 1 (derive) + `serde_yaml` 0.9 + `serde_json` 1 + `toml` 0.9 — YAML/JSON/TOML (de)serialisation throughout.
- `ctrlc` 3 — SIGINT trap so a running simulation flushes metrics before exit.

**Testing:**
- Built-in `cargo test` framework. No external runner (no nextest, no proptest). Test files live next to source under `sim-rs/sim-core/src/sim/tests/` and as one integration test at `sim-rs/sim-cli/tests/determinism.rs`.
- `tempfile` 3 (dev-dep, sim-cli) — temp dirs for runner tests.
- `sha2` 0.10 + `hex` 0.4 (dev-deps, sim-core; runtime, sim-cli) — pricing-event-stream SHA256 golden hashes. Used in `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs:21` and the suite-level goldens checker.

**Build / Dev:**
- `vergen-gitcl` 1 — emits the current git SHA at build time into the CLI version string (`sim-rs/sim-cli/build.rs`). Both binaries embed it via `concat!(env!("CARGO_PKG_VERSION"), "-", env!("VERGEN_GIT_SHA"))`.

## Key Dependencies

**Pricing kernel math (must be deterministic, no f64 in hot paths):**
- `libm` 0.2 — bit-reproducible `pow`/`round` for the actor utility-maximising lane choice (`sim-rs/sim-core/src/tx_actors.rs`). Picked specifically because Rust's stdlib `f64::powf`/`round` route through libm but with platform-dependent ABI; using the crate directly fixes the implementation.
- `num-traits` 0.2 — generic integer trait bounds inside the `CapacityWeightedWindow` and EIP-1559 update.
- `priority-queue` 2 — mempool ordering by fee.
- `dashmap` 6 — concurrent maps in the network coordinator.

**RNG (deterministic):**
- `rand` 0.9 — RNG trait surface.
- `rand_chacha` 0.9 — `ChaCha20Rng` seeded per (job, seed); the only RNG used in simulation-affecting paths.
- `rand_distr` 0.5 — `Exp`, `LogNormal`, `Normal`, `Poisson` distributions for arrival rates, tx sizes, and value sampling.

**Network simulation (transitive Cardano infra):**
- `netsim-async` and `netsim-core` 0.1 — IO-Hong-Kong's `ce-netsim` library, pinned to git rev `9d1e26c` of `https://github.com/input-output-hk/ce-netsim`. Provides the in-process network coordinator (bandwidth, latency, geo-aware link simulation) that sits beneath the protocol simulator. `netsim-core` also exposes `geo::Location` + `latency_between_locations` used by the `gen-test-data` topology generator (`sim-rs/sim-cli/src/bin/gen-test-data/strategy/utils.rs:6`).

**Metrics / serialisation (reporting-only, plain f64 OK):**
- `chrono` 0.4 (serde feature) — UTC timestamps in the manifest (`sim-rs/sim-cli/src/runner.rs:22`).
- `statrs` 0.18 — Beta/CDF distributions used by the topology generator only.
- `average` 0.16 — `Variance` accumulator in `sim-rs/sim-cli/src/events.rs:7`.
- `async-compression` 0.4 (tokio + gzip) — gzip-stream the legacy event sink for sized output.
- `minicbor-serde` 0.6 (alloc) — CBOR stream output for the legacy event sink (alternative to JSON).
- `itertools` 0.14, `pretty-bytes-rust` 0.3, `hex` 0.4 — utility crates in metrics formatting.
- `anyhow` 1 — error propagation across the workspace.

**Logging:**
- `tracing` 0.1 — structured logging spans/events throughout.
- `tracing-subscriber` 0.3 (env-filter) — `RUST_LOG`-compatible env filter, default `INFO`. Wired in both `sim-rs/sim-cli/src/main.rs:108-115` and `sim-rs/sim-cli/src/bin/experiment-suite/main.rs:59-66`.

## Configuration

**Build-time env vars (read by build.rs):**
- `VERGEN_GIT_SHA` — emitted by `vergen-gitcl`, baked into `--version` output. Not user-set.

**Runtime env vars (read by the CLI):**
- `RUST_LOG` — `tracing-subscriber` env filter directive. Defaults to `INFO` if unset.
- `UPDATE_GOLDENS=1` — test-only switch that rewrites `sim-rs/parameters/phase-2-sweep/suites/.goldens/*` instead of asserting equality (`sim-rs/sim-cli/tests/determinism.rs:127`).
- `M6_RUN_ID` — operator script convention (`sim-rs/scripts/run-parallel-suites.sh`) for sharing a batch identifier across parallel suite invocations. Passed through to the binary as `--run-id`.

**Config files (YAML, layered):**
- `sim-rs/parameters/config.default.yaml` — protocol baseline (slot, RB, EB, IB, vote, certificate settings). Embedded with `include_str!` for binary-self-sufficiency.
- `sim-rs/parameters/topology.default.yaml` — fallback topology embedded via `include_str!` (`sim-rs/sim-cli/src/main.rs:37`).
- `sim-rs/parameters/phase-2-sweep/protocol-base.yaml` — phase-2 overlay with `min_fee_a`/`min_fee_b`, RB cadence, vote calibration. Three RB-reduced variants (`protocol-rb-reduced-{half,third,quarter}.yaml`) are full replacements, not stacked overlays — see CLAUDE.md "Conventions / gotchas".
- `sim-rs/parameters/phase-2-sweep/demand/*.yaml` — actor profiles (5 of them: `paper_like_{realistic,moderate,congested,mispriced}.yaml`, `sundaeswap_moderate.yaml`).
- `sim-rs/parameters/phase-2-sweep/pricing/*.yaml` — 19 pricing-mechanism configs (1 baseline flat-fee, 7 EIP-1559 controller variants, 11 two-lane variants).
- `sim-rs/parameters/phase-2-sweep/suites/*.yaml` — 19 suite manifests (M4-M6 phase-2 sweep) listing `(default protocol, default demand, jobs[*].pricing)` tuples plus seeds.
- `sim-rs/parameters/config.schema.json` — JSON Schema describing the YAML config surface (linked via `yaml-language-server` header comment in `config.default.yaml`).

**Build:**
- `sim-rs/Cargo.toml` — workspace manifest. Two members (`sim-cli`, `sim-core`), resolver 2, `profile.release.debug = true`.
- `sim-rs/sim-cli/build.rs` — emits the git-SHA build-info.
- No `rust-toolchain.toml` / `rust-toolchain` file. Toolchain is pinned only via `rust-version = "1.88"` in each crate's `Cargo.toml` (rejected at compile time, not via rustup-auto-install).

## Platform Requirements

**Development:**
- Rust 1.88+ with edition 2024 support.
- `cargo` reachable in `PATH`.
- A `git` checkout (the `vergen-gitcl` build script shells out to `git rev-parse`); building outside a git working tree fails the build script.
- For topology generation only: Python 3 (the `generate-cip-topology.py` script).

**Production:**
- Linux x86_64 / glibc is the development and golden-hash reference. CLAUDE.md "Determinism scope" notes determinism is asserted **intra-architecture**; cross-arch CI is not built.
- No deployment target other than the local CLI binaries. The simulator writes artefacts under `output/` and exits — no daemon, no server.

**Disk:**
- Each suite run writes `time_series.csv`, `diagnostics.log`, `pricing_event_stream.{events,sha256}`, and the per-suite `metrics_comparison.txt` under `output/phase-2/<suite>-<run-id>/`. The `.gitignore` excludes `output/`, `target/`, `flamegraph.svg`, `perf.data*`, `.docker/`, and `*.tar`.

---

*Stack analysis: 2026-05-13*
