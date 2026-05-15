# External Integrations

**Analysis Date:** 2026-05-15

This is a **closed offline simulator**. There are no live external integrations at runtime: no HTTP clients, no database drivers, no message queues, no auth providers, no SDK calls to cloud services. The simulator ingests YAML from disk, runs an in-process discrete-event simulation, and writes flat files to disk. This document characterises that I/O surface and the determinism contract that pins it.

## APIs & External Services

**Live runtime:**
- None. The released binary makes no outbound network calls. There is no `reqwest`, `hyper`, `surf`, `ureq`, AWS/GCP/Azure SDK, or HTTP client in the dependency tree.

**Build-time only:**
- `vergen-gitcl` (build-dep of `sim-cli`, see `sim-rs/sim-cli/build.rs`) — shells out to local `git` at compile time to embed the commit SHA as `VERGEN_GIT_SHA`. No network access; consumed by `--version` output of `sim-cli` and `experiment-suite`.

**Out-of-band data refresh (not part of build/run):**
- `sim-rs/scripts/generate-realistic-100-topology.py` — one-shot Python script that fetches **Cardano mainnet pool stake data** from the public Koios API:
  - `https://api.koios.rest/api/v1/pool_list?active_stake=gt.0&order=active_stake.desc,pool_id_bech32.asc&limit=1000&offset={offset}` — two pages of 1,000 rows for the full active-stake list.
  - `https://api.koios.rest/api/v1/epoch_info?_include_next_epoch=false&limit=1` — current epoch metadata for the snapshot stamp.
  - No authentication; the API is public read-only.
  - Produces `sim-rs/parameters/phase-2-sweep/topology-realistic-100.yaml` (the canonical 100-node phase-2 topology since 2026-05-13). The committed YAML is the source of truth — the script is the reference recipe, not a build step. Last refresh: Cardano mainnet epoch 582, retrieved 2026-05-14 (see header comment in `sim-rs/parameters/phase-2-sweep/topology-realistic-100.yaml`).

## Data Storage

**Databases:**
- None. No SQL, no embedded KV store, no SQLite, no RocksDB.

**File Storage:**
- Local filesystem only. All outputs land under `sim-rs/output/`.

**Caching:**
- None. The simulator is deterministic and stateless across invocations; resumability is via the on-disk manifest.

## Authentication & Identity

- Not applicable. No user accounts, no API keys, no secrets management.

## Monitoring & Observability

**Logs:**
- `tracing` + `tracing-subscriber` `fmt` layer to stderr. `EnvFilter` reads `RUST_LOG`; default level `INFO`. Initialised in `sim-rs/sim-cli/src/main.rs:108-115` and `sim-rs/sim-cli/src/bin/experiment-suite/main.rs:89-97`.
- No external log shipper, no Sentry, no OpenTelemetry exporter.

**Metrics:**
- Per-run metrics (welfare, retained-value, latency, fees/refunds) are written to flat files (see "What Gets Written" below); they are **not** emitted to any monitoring backend.

## CI/CD & Deployment

**Hosting:** None — there is no deployed service.

**CI Pipeline:** None. No `.github/workflows/`, no `.gitlab-ci.yml`, no Drone/Buildkite/CircleCI config in this branch. The `dynamic-experiment` branch is currently a research-team-local development environment; the M5 handoff flags cross-arch CI verification as not-yet-built.

**Local orchestration:**
- `sim-rs/scripts/run-parallel-suites.sh` — drives multiple `experiment-suite run` invocations in parallel from a single batch identifier.
- `sim-rs/scripts/run-smoke-*.sh` — per-regime smoke wrappers.
- `sim-rs/scripts/run-m6-{full-sweep-100,variance}.sh` — full-sweep launchers.
- `sim-rs/scripts/watch-suite-progress.sh` — passive log tailer.

## Environment Configuration

**Required env vars:** None. The binaries run with no env vars set; `RUST_LOG` is optional.

**Optional env vars (see STACK.md "Configuration" for full list):**
- `RUST_LOG`, `UPDATE_GOLDENS`, `M6_RUN_ID`, `CARGO_PKG_VERSION` (compile-time), `VERGEN_GIT_SHA` (compile-time).

**Secrets location:** Not applicable. There are no secrets. No `.env` files anywhere in the repo.

## Webhooks & Callbacks

- None (incoming or outgoing).

---

# What Gets Read From Disk

Every input is YAML or text. There is no binary input format, no proprietary archive.

## 1. Suite YAML (entry point for `experiment-suite`)

**Path convention:** `sim-rs/parameters/phase-2-sweep/suites/<suite-name>.yaml`.

**Schema:** `Suite` struct in `sim-rs/sim-cli/src/suite.rs:17`. Top-level keys (kebab-case): `suite-name`, `output-dir`, `seeds`, `default-slots`, `default-topology`, `default-protocol`, `default-demand`, `jobs`. Each `Job` carries `name`, `pricing`, and optional `overrides`.

**Loaded by:** `Suite::load(path)` → `serde_yaml::from_str` (`sim-rs/sim-cli/src/suite.rs:53`).

**Catalogue (21 suite YAMLs):**

| Suite | Scope |
|---|---|
| `phase-2-eip1559-robustness.yaml` | Single-lane EIP-1559 D × target sweep |
| `phase-2-eip1559-smoothing.yaml` | Single-lane EIP-1559 window-length sweep |
| `phase-2-priority-only-rb-reserved.yaml` | RB-reserved priority-only-static-standard (×4/×8/×16) |
| `phase-2-priority-only-unreserved.yaml` | Un-reserved priority-only premium (×4/×8/×16) |
| `phase-2-two-lane-both-dynamic.yaml` | Both-dynamic in partitioned / un-partitioned forms |
| `phase-2-rb-scarcity.yaml` | RB-capacity scarcity (M4) |
| `phase-2-urgency-inversion.yaml` | Mis-priced actors (M4) |
| `phase-2-{congested,moderate,realistic,sundaeswap}-*.yaml` | 12 demand-regime suites (not goldens-pinned) |

The seven goldens-pinned suites are listed in `CLAUDE.md` "Running the suites" (the M3/M4 mechanism-characterisation set).

## 2. Topology YAML

**Paths:**
- `sim-rs/parameters/phase-2-sweep/topology-realistic-100.yaml` — default for all phase-2 suites since 2026-05-13. 100 nodes, mass-stratified Cardano mainnet curve, rescaled to total stake = 3 × 10¹⁰ lovelace.
- `sim-rs/parameters/phase-2-sweep/topology-cip-realistic.yaml` — 600-pool CIP-0164 baseline (legacy M6).
- `sim-rs/parameters/phase-2-sweep/topology-single-producer.yaml` — single-node kernel-correctness fixture (used by `sim-rs/sim-cli/tests/determinism.rs:57`).
- `sim-rs/parameters/topology.default.yaml` — embedded fallback for the legacy `sim-cli` binary (`include_str!` in `sim-rs/sim-cli/src/main.rs:37`).

**Schema:** `RawTopology` (`sim-rs/sim-core/src/config.rs:466`). Each node has `cpu-core-count`, `location`, `producers: { <peer>: { bandwidth-bytes-per-second, latency-ms } }`, `stake`, optional `tx-generation-weight`.

**Loaded by:** `serde_yaml::from_str(&topology_text)` → `RawTopology::into(): Topology` → `topology.validate()` (`sim-rs/sim-cli/src/runner.rs:828-830`).

**Generation:** see `sim-rs/scripts/generate-realistic-100-topology.py` for the Koios-API recipe.

## 3. Layered config overlays

For each (job, seed) the runner composes four YAML layers via `figment` (`sim-rs/sim-cli/src/runner.rs:840-848`):

| Layer | Source | Role |
|---|---|---|
| Base | `parameters/config.default.yaml` (compiled-in via `include_str!`) | Every field with a sensible default |
| Protocol | `parameters/phase-2-sweep/protocol-base.yaml` (or one of the `protocol-rb-reduced-*.yaml` for the RB-scarcity suite) | Phase-2 spec defaults (min-fee, mempool cap, RB cadence, vote params, CIP-0164 stage lengths) |
| Demand | `parameters/phase-2-sweep/demand/<profile>.yaml` | `actors:` block (component arrivals, sizes, values, half-lives, max-fee policy) |
| Pricing | `parameters/phase-2-sweep/pricing/<config>.yaml` | Pricing backend kind + controller parameters |

File-extension dispatch in `merge_layer` (`sim-rs/sim-cli/src/runner.rs:798`) accepts `.toml` overlays too, but all phase-2 overlays are YAML.

**Demand profiles** (`sim-rs/parameters/phase-2-sweep/demand/`):
- `paper_like_{light,moderate,congested,realistic,mispriced}.yaml`
- `sundaeswap_moderate.yaml`

**Pricing configs** (`sim-rs/parameters/phase-2-sweep/pricing/`):
- `baseline_flat_fee.yaml`
- `eip1559_d{4,8,16}_target{0.25,0.5,0.75}_window{16,32,64}.yaml`
- `two_lane_priority_only_{static,unreserved}_x{4,8,16}.yaml`
- `two_lane_both_dynamic_{partitioned,unreserved}_x{4,16}.yaml`

## 4. Test fixtures

- `sim-rs/test_data/{simple,small,medium,thousand,realistic,simplified,organic}.yaml` — legacy test topologies (predate phase-2).
- `sim-rs/sim-cli/test_data/distribution.toml` — input for the `gen-test-data` binary's TOML distribution loader (`sim-rs/sim-cli/src/bin/gen-test-data/strategy/globe.rs:88`).
- M2/M3 tests load `parameters/config.default.yaml` via `include_bytes!` (e.g. `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs:80`, `m3_actors.rs:100`).

---

# What Gets Written To Disk

All output lands under `sim-rs/output/`. Per-suite layout under `<suite.output-dir>/`:

```
<output_dir>/
├── suite.yaml                       # snapshot of the suite YAML at run start
├── manifest.json                    # resumable per-(job, seed) status (JSON)
├── manifest.lock                    # cross-process file lock
├── metrics_comparison.txt           # cross-job welfare comparison (rebuilt after each completed job)
└── <job_name>/<seed>/               # per-(job, seed) artefacts
    ├── run_summary.json
    ├── pricing_event_stream.sha256
    ├── time_series.csv
    └── diagnostics.log
```

## Per-suite artefacts

**`suite.yaml`** — verbatim copy of the input suite YAML, snapshot at run start. Lets a later `verify` or post-hoc analysis reconstruct what was actually run even if the source YAML drifts.

**`manifest.json`** — resumable state. Schema: `Manifest` struct (`sim-rs/sim-cli/src/runner.rs:69`). Keyed `BTreeMap<job_name, BTreeMap<seed_string, JobEntry>>` — deterministic on-disk order regardless of parallel completion order. Each `JobEntry` carries `status ∈ {pending, running, completed, failed}`, `started_at_utc` (chrono `DateTime<Utc>`), `completed_at_utc`, optional `output_path`, optional `error`. Atomic write via `std::fs::write`. On resume, `Running` entries are auto-reset to `Pending` (the previous run was killed mid-job).

**`manifest.lock`** — sentinel file used for cross-process locking when multiple `experiment-suite` invocations might race.

**`metrics_comparison.txt`** — per-suite welfare summary across all `Completed` (job, seed) pairs. Plain-text format produced by `sim-rs/sim-cli/src/metrics/comparison.rs`. Rebuilt after each completed job so partial output is always inspectable during a long-running suite.

## Per-(job, seed) artefacts

**`run_summary.json`** — serialised `RunSummary` (`sim-rs/sim-cli/src/metrics/collector.rs:111`). Carries:
- Per-`ComponentSummary` welfare aggregates (`retained_value_total`, `net_utility_total`, `inclusion_rate`, `eviction_rate`, latency-blocks observations).
- Cross-component totals (`total_txs_submitted`, `total_txs_included`, fees, refunds).
- Per-lane retained-value totals (priority / standard).
- `multiplier_floor_breaches` (must always be 0; non-zero is a simulator bug).
- `min/max_priority_over_standard_ratio` observed.
- `pricing_event_stream_sha256` — the SHA-256 over the deterministic pricing event stream, also written separately as `pricing_event_stream.sha256`.
- `block_generation_probability`, `pricing_ticks`.
- Multi-node noise metrics (M6).

**`pricing_event_stream.sha256`** — bare-hex SHA-256 (64 hex chars, no newline padding). The **determinism contract** anchor. Computed incrementally over `TXIncluded` + `TXEvictedQuoteDrift` events in observation order (`sim-rs/sim-cli/src/metrics/collector.rs:272`), using the same encoding as the M2/M3 unit-test golden tests. Persisted by `persist_run_artefacts` (`sim-rs/sim-cli/src/runner.rs:471`).

**`time_series.csv`** — per-slot snapshots. Header pinned at `sim-rs/sim-cli/src/metrics/time_series.rs:16`:

```
slot,c_priority,c_standard,util_priority_window_x_1e9,util_standard_window_x_1e9,
mempool_bytes_total,mempool_bytes_priority,mempool_bytes_standard,
included_bytes_priority,included_bytes_standard,
included_count_priority,included_count_standard,
evicted_quote_drift_count,fees_paid_lovelace,refund_lovelace
```

All columns are integers. The reporting-f64 welfare metrics live in `run_summary.json` and `metrics_comparison.txt`, **not** in the CSV.

**`diagnostics.log`** — resolved config dump, controller settings, multiplier-floor breach count (must be 0), observed quote-ratio min/max, and free-form `DiagnosticNote`s (`sim-rs/sim-cli/src/metrics/diagnostics.rs:17`).

## Legacy `sim-cli` binary outputs

The pre-phase-2 `sim-cli` driver (`sim-rs/sim-cli/src/main.rs`) writes a different artefact set via `EventMonitor` (`sim-rs/sim-cli/src/events.rs`):

- `events.jsonl` — line-delimited JSON `OutputEvent { time_s, message }` (`sim-rs/sim-cli/src/events.rs:35`).
- `events.cbor` — same payload encoded with `minicbor-serde` when the output path ends in `.cbor` (`events.rs:770`).
- `events.{jsonl,cbor}.gz` — `async-compression` `GzipEncoder` wraps either format when the path ends in `.gz` (`events.rs:127-138`).

The legacy binary is retained for the upstream Leios protocol research workflow; phase-2 simulations always use `experiment-suite` and the welfare-metrics output schema above.

---

# Goldens Regime — Determinism Contract

The simulator's determinism is asserted at three layers, each more comprehensive than the last. All three pin SHA-256 digests; cross-architecture verification is not yet built (see `CLAUDE.md` "Determinism is intra-arch").

## Layer 1 — Unit-test goldens

**Locations:**
- `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs:1289` (two-lane scenarios)
- `sim-rs/sim-core/src/sim/tests/m3_actors.rs:391` (actor lane-choice scenarios)

**Format:** SHA-256 hex constants pinned in source. Scoped scenarios — a handful of (transactions, blocks, controller steps) per test.

**Run:** `cargo test --workspace` (default).

## Layer 2 — In-suite verify

**Subcommand:** `experiment-suite verify <suite.yaml>`. Defined in `sim-rs/sim-cli/src/runner.rs:600` (`verify_suite_with_run_id`).

**Behaviour:** Walks the suite's manifest, for each `Completed` (job, seed):
1. Reads the persisted `pricing_event_stream.sha256` from disk.
2. Re-runs the simulation from scratch (same seed, same composed config, same topology).
3. Asserts the freshly-computed `RunSummary.pricing_event_stream_sha256` matches the persisted value bit-for-bit.

**Defensive guard:** Malformed stored hashes (not exactly 64 hex chars) are rejected pre-flight (`runner.rs:660`) — silent pass-by-default against an empty freshly-computed hash would defeat the contract.

**Parallelism:** identical to `run` — `min(available_parallelism(), 8)`, per-thread `current_thread` tokio runtimes.

## Layer 3 — Suite-level goldens

**Test file:** `sim-rs/sim-cli/tests/determinism.rs`.

**Goldens directory:** `sim-rs/parameters/phase-2-sweep/suites/.goldens/<suite>.sha256` — 7 files, one per goldens-pinned suite:

```
phase-2-eip1559-robustness.sha256
phase-2-eip1559-smoothing.sha256
phase-2-priority-only-rb-reserved.sha256
phase-2-priority-only-unreserved.sha256
phase-2-rb-scarcity.sha256
phase-2-two-lane-both-dynamic.sha256
phase-2-urgency-inversion.sha256
```

**Format per file:** one line `<job_name> <seed> <64-hex-sha256>`, e.g.:
```
d8_target0.5_window32 1 92701c73944ead391c490ffd1819bae9338e3742848fd51fa002ca197c1ea1b7
```

**Behaviour:** Each test rebases the suite's relative paths onto `CARGO_MANIFEST_DIR/..` (`tests/determinism.rs:62`), redirects `output_dir` to a `tempfile::TempDir`, clamps `slots` to `DETERMINISM_SLOTS = 200`, swaps the topology for `topology-single-producer.yaml`, and runs the suite's canonical baseline (job, seed=1). Asserts the freshly-computed hash equals the committed golden.

**Run:** `#[ignore]`'d by default; invoke via `cargo test --release -- --ignored determinism`. ~1.5s total in release mode.

**Regenerate:** `UPDATE_GOLDENS=1 cargo test --release -- --ignored determinism` writes fresh hashes to disk instead of asserting. After intentional simulator changes, commit the regenerated goldens and tag the branch (see `CLAUDE.md` "Running the suites").

## Hashing semantics

The SHA-256 is computed over **only** `TXIncluded` and `TXEvictedQuoteDrift` events, in observation order. These are the events that determine simulator outcomes — any accidental f64 entry into the simulation-affecting hot path flips them. Reporting f64 metrics (`retained_value`, `net_utility`, etc.) are computed from the deterministic event stream but never feed back into simulation decisions, and are therefore **not** part of the hash (see `CLAUDE.md` "Numeric representation contract").

The same digest algorithm and event encoding is used at all three layers (M2/M3 unit goldens, runner verify, suite goldens) — they cross-check each other.

---

*Integration audit: 2026-05-15*
