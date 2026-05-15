# External Integrations

**Analysis Date:** 2026-05-13

## Summary

**This is an offline simulator. It has no external integrations at runtime.**

- No HTTP/RPC client code, no API SDKs (no Stripe / AWS / GCP / Azure / Supabase / Cardano-node clients).
- No outbound or inbound network calls during a simulation. All "network" traffic is in-process, simulated by the bundled `netsim-async` / `netsim-core` crates which model bandwidth and latency between nodes within a single OS process.
- No database, no cache, no message broker, no file storage backend other than the local filesystem under `sim-rs/output/`.
- No authentication, no identity provider, no secret management.
- No webhooks. No CI service integration committed in this branch.

The remainder of this document records the few thin external touchpoints that do exist (a build-time git query and a single git-pinned upstream crate) plus the explicit non-integrations, so future planning passes don't re-investigate.

## APIs & External Services

None — no service SDK or client is imported.

The closest thing to an "API" in the codebase is the in-process `netsim` simulated link layer:

- **`netsim-async`** 0.1 — IO-Hong-Kong's `ce-netsim` library, pinned to git rev `9d1e26c` of `https://github.com/input-output-hk/ce-netsim`. Used in `sim-rs/sim-core/` to model node-to-node bandwidth and latency entirely inside the process. Not a network egress dependency at runtime.
- **`netsim-core`** 0.1 — same git pin. Used only by the offline topology generator (`sim-rs/sim-cli/src/bin/gen-test-data/strategy/utils.rs:6`) for `geo::Location` + `latency_between_locations`.

Both `netsim` crates are vendored via Cargo's git-source mechanism. The git URL is contacted only at `cargo fetch` time, never at runtime.

## Data Storage

**Databases:**
- None. No SQL client, no NoSQL client, no embedded DB (no `sled`, `rocksdb`, `sqlite`, `redb`).

**File Storage:**
- Local filesystem only. The runner writes per-job artefacts under `sim-rs/output/phase-2/<suite>-<run-id>/<job>/<seed>/`:
  - `time_series.csv` — per-slot pricing/mempool snapshots.
  - `diagnostics.log` — resolved config + run-level validation.
  - `pricing_event_stream.events` / `.sha256` — the deterministic event stream and its golden hash.
  - `metrics_comparison.txt` — per-suite welfare comparison.
- The legacy event sink (`sim-rs/sim-cli/src/events.rs`) optionally writes a gzipped JSON or CBOR event log to a path passed via the CLI's positional `output` argument. Used only by the legacy `sim-cli` binary, not by `experiment-suite`.

**Caching:**
- None.

## Authentication & Identity

- None. The CLI runs unauthenticated and never speaks to a remote identity provider. The only "identity" in the simulator is `NodeId` (`sim-rs/sim-core/src/config.rs`), an internal `u64` used to disambiguate simulated nodes.

## Monitoring & Observability

**Error Tracking:**
- None. No Sentry/Bugsnag/Datadog/OpenTelemetry SDK.

**Logs:**
- Local stdout/stderr via `tracing` 0.1 + `tracing-subscriber` 0.3. Initialised in `sim-rs/sim-cli/src/main.rs:108-115` and `sim-rs/sim-cli/src/bin/experiment-suite/main.rs:59-66`. Default level `INFO`; override via `RUST_LOG`.
- No log shipper, no remote sink.

**Metrics:**
- Written to disk as CSV / plaintext (`time_series.csv`, `metrics_comparison.txt`). Not pushed anywhere.

## CI/CD & Deployment

**Hosting:**
- None. The simulator is a local CLI executable.

**CI Pipeline:**
- Not present on this branch — no `.github/workflows/`, no `.gitlab-ci.yml`, no Jenkinsfile, no `Dockerfile`, no `docker-compose.yml`.
- Operator scripts under `sim-rs/scripts/` (e.g. `run-parallel-suites.sh`, `run-m6-full-sweep-100.sh`) are local-bash batch drivers, not CI hooks.

**Build identity:**
- `sim-rs/sim-cli/build.rs` invokes `vergen-gitcl` 1, which shells out to `git rev-parse` at build time and bakes the SHA into `--version`. This is the only build-time external touchpoint and it stays local to the working tree.

## Environment Configuration

**Required env vars:** None for normal `cargo build` / `cargo test` / suite runs.

**Optional env vars:**
- `RUST_LOG` — `tracing-subscriber` directive (e.g. `RUST_LOG=info,sim_core=debug`). Defaults to `INFO`.
- `UPDATE_GOLDENS=1` — test-only; rewrites the M5 golden hashes in `sim-rs/parameters/phase-2-sweep/suites/.goldens/` instead of asserting (`sim-rs/sim-cli/tests/determinism.rs:127`).
- `M6_RUN_ID` — convention used by `sim-rs/scripts/run-parallel-suites.sh` to share a single batch identifier across concurrently spawned suite processes.

**Secrets location:**
- N/A — no secrets are used or stored. No `.env`, `.envrc`, or credentials file exists in the working tree.

## Webhooks & Callbacks

**Incoming:** None. The simulator does not bind a listening socket.

**Outgoing:** None. The simulator does not initiate network calls. SIGINT (ctrl+c) is the only external signal handled (`ctrlc` 3 in `sim-rs/sim-cli/src/main.rs:123-134`), used to flush metrics on graceful shutdown.

---

*Integration audit: 2026-05-13*
