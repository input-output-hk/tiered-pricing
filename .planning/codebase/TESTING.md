# Testing Patterns

**Analysis Date:** 2026-05-13

## Test Framework

**Runner:** built-in Rust test harness via `cargo test`. No third-party runner (no nextest, no `cargo-test-runner` config).

**Workspace test invocation** (from `sim-rs/`):
```bash
cargo test --workspace          # all unit + integration tests, excludes #[ignore]'d
cargo test --release -- --ignored determinism   # M5 suite-level goldens (slow; ~1.5s release)
UPDATE_GOLDENS=1 cargo test --release -- --ignored determinism   # regenerate goldens
```

**Async tests:** `#[tokio::test]` from `tokio = { features = ["macros", "rt"] }`. Used only where the system under test is genuinely async (e.g. `ClockCoordinator` poll-based time advancement in `sim-rs/sim-core/src/clock/coordinator.rs`). Most tests are synchronous `#[test]` functions even when exercising async code — the M1-M3 drivers (`SmokeDriver`, `TwoLaneDriver`, `ActorDriver`) use `MockClockCoordinator` to step time deterministically without an async runtime.

**Hashing for goldens:** `sha2 = "0.10"` (`Sha256::new()` / `hasher.update()` / `hasher.finalize()`). Encoded with `hex = "0.4"` (`hex::encode`).

**Tempdirs for filesystem tests:** `tempfile = "3"` (dev-dependency in `sim-rs/sim-cli/Cargo.toml`). Used in `sim-rs/sim-cli/tests/determinism.rs` to redirect run output away from the tracked `sim-rs/output/` tree, and in `sim-rs/sim-cli/src/runner.rs::tests` to lay down fake suite fixtures.

## Test File Organization

Tests live in three places, each with a distinct role:

### 1. Inline `#[cfg(test)] mod tests { ... }` modules

Tight unit tests against a single module's surface. Sit at the bottom of the module file. 17 files carry inline test modules:

| File | Scope |
|---|---|
| `sim-rs/sim-core/src/tx_pricing/single_lane.rs` (lines 336+) | `BaselinePricing`, `Eip1559Pricing` controller arithmetic — target/clamp, era floor, ceil rounding, sustained-saturation drift |
| `sim-rs/sim-core/src/tx_pricing/two_lane.rs` (lines 377+) | `TwoLanePricing` — variant-specific sample emission, multiplier-floor invariant, RB-reserved window-length forcing |
| `sim-rs/sim-core/src/tx_pricing/window.rs` (lines 97+) | `CapacityWeightedWindow` — ring eviction, length-1 reduction, heterogeneous RB+EB aggregation |
| `sim-rs/sim-core/src/sim/mempool_gate.rs` (lines 293+) | `MempoolGate` — admission rejection paths, byte-cap, quote-drift revalidation, inclusion charging |
| `sim-rs/sim-core/src/tx_actors.rs` | `MaxFeePolicy`, lane choice math, latency estimator |
| `sim-rs/sim-core/src/sim/cpu.rs` (lines 92+) | CPU-task scheduling state machine |
| `sim-rs/sim-core/src/sim/linear_leios.rs` | EB validation, partition activation |
| `sim-rs/sim-core/src/clock/coordinator.rs` (lines 135+) | `ClockCoordinator` async barrier semantics (`#[tokio::test]`) |
| `sim-rs/sim-core/src/network/connection.rs` | Network layer (legacy) |
| `sim-rs/sim-cli/src/runner.rs` (lines 665+) | `verify_suite` malformed-hash bail paths |
| `sim-rs/sim-cli/src/metrics/collector.rs` | Metrics aggregation |
| `sim-rs/sim-cli/src/main.rs` | CLI argument plumbing |
| `sim-rs/sim-cli/src/bin/gen-test-data/strategy/{globe,organic,random_graph,simplified}.rs` | Topology-generation strategies (legacy) |
| `sim-rs/sim-core/src/sim.rs` | Simulation harness |

### 2. Cross-module integration tests under `sim-rs/sim-core/src/sim/tests/`

Wired via `sim-rs/sim-core/src/sim/tests/mod.rs`:

```rust
mod linear_leios;
mod m1_smoke;
mod m2_two_lane;
mod m3_actors;
```

| File | Lines | Scope |
|---|---|---|
| `sim-rs/sim-core/src/sim/tests/m1_smoke.rs` | 372 | M1 exit-criterion smoke test: one short single-producer run produces refunds and quote-drift evictions |
| `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs` | 1209 | M2 deterministic scenarios for all four `TwoLaneVariant` arms + two pinned cross-platform golden hashes |
| `sim-rs/sim-core/src/sim/tests/m3_actors.rs` | 471 | M3 actor-model integration: lane choice, max-fee policies, latency-estimator EMA, intra-arch golden |
| `sim-rs/sim-core/src/sim/tests/linear_leios.rs` | 589 | Legacy multi-node linear-Leios consensus / voting flows |

### 3. Workspace-level integration tests under `sim-rs/sim-cli/tests/`

| File | Lines | Scope |
|---|---|---|
| `sim-rs/sim-cli/tests/determinism.rs` | 219 | M5 suite-level golden-hash regime; 7 `#[test] #[ignore]` functions, one per phase-2 suite |

## Test Structure Pattern

**Unit-test pattern** (`#[cfg(test)] mod tests`):

```rust
// Helper builders at the top — encapsulate "valid args" so the test
// body focuses on the assertion under test.
fn standard_rb(bytes: u64, capacity: u64) -> PricedBlockSample { ... }
fn settings(initial: u64, d: u64) -> Eip1559Settings { ... }

#[test]
fn eip1559_at_target_does_not_move() {
    let mut pricing = Eip1559Pricing::new(settings(1000, 8)).unwrap();
    pricing.update_after_block(&[standard_rb(50, 100)]);
    assert_eq!(pricing.current_quote(Lane::Standard), 1000);
}
```

**Citation comments link each test to the plan / spec:**
```rust
// sim-rs/sim-core/src/sim/mempool_gate.rs:343-345
// Implementation-plan.md verification §M1, line 299:
//   "maxFee admission rejects when prospective posted_fee >
//    max_fee_lovelace."
```

**Driver pattern for integration tests** — each milestone has a single hand-rolled driver:

| Driver | File | Role |
|---|---|---|
| `SmokeDriver` | `sim-rs/sim-core/src/sim/tests/m1_smoke.rs:84` | M1 single-producer linear-Leios harness |
| `TwoLaneDriver` | `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs` (~line 200) | M2 extension with `posted_lane` arg and per-test `RawTwoLaneConfig` |
| `ActorDriver` | `sim-rs/sim-core/src/sim/tests/m3_actors.rs` (~line 200) | M3 extension that wires `RawActorProfile` into `LinearLeiosNode::handle_new_slot` |

Each driver owns:
- `Arc<SimConfiguration>` built from `parameters/config.default.yaml` via `include_bytes!`
- `HashMap<NodeId, LinearLeiosNode>` of simulated nodes
- `HashMap<NodeId, Arc<MockLotteryResults>>` for VRF mocking
- `MockClockCoordinator` for deterministic time advancement
- `mpsc::UnboundedReceiver<(Event, Timestamp)>` to drain the simulator's event stream
- Helpers: `make_tx`, `submit_tx`, `win_lottery`, `next_slot`, `advance_to`, `drain_events`

## Mocking

**Framework:** no `mockall` / `mockito`. Mocks are hand-rolled and lightweight.

### `MockClockCoordinator` / `MockLotteryResults`

| Mock | File | Purpose |
|---|---|---|
| `MockClockCoordinator` | `sim-rs/sim-core/src/clock/mock.rs` | Step time explicitly via `advance_time(t)`. Panics if a waiter waits twice or time advances past the next pending event |
| `MockLotteryResults` | `sim-rs/sim-core/src/sim/lottery.rs:24` | `DashMap<LotteryKind, VecDeque<u64>>` of pre-configured winning VRF values; `configure_win(kind, value)` queues a win, `LinearLeiosNode::mock_lottery(...)` installs |

**Usage:**
```rust
// sim-rs/sim-core/src/sim/tests/m1_smoke.rs:290
sim.win_lottery(LotteryKind::GenerateRB, 0);
sim.next_slot();
```

### Embedded config fixtures

Tests embed `parameters/config.default.yaml` at compile time via `serde_yaml::from_slice(include_bytes!(...))` rather than reading from disk:

```rust
// sim-rs/sim-core/src/sim/tests/m1_smoke.rs:49-50
let mut params: RawParameters =
    serde_yaml::from_slice(include_bytes!("../../../../parameters/config.default.yaml"))
        .unwrap();
```

Same pattern in `m2_two_lane.rs:80`, `m3_actors.rs:100`, `linear_leios.rs:25`. After loading, tests override specific fields (e.g. `params.leios_variant`, `params.tx_size_bytes_distribution`, `params.pricing`).

### Filesystem fixtures via `tempfile`

`sim-rs/sim-cli/src/runner.rs::tests::lay_down_verify_suite_fixture` (line 674) builds a complete `tempfile::TempDir` skeleton — `suite.yaml` + `manifest.json` + `pricing_event_stream.sha256` + `run_summary.json` — to drive `verify_suite` deep enough to reach the malformed-hash bail. Uses `serde_json::json!` (not raw string literals) so future renames of `Manifest`/`RunSummary` fields surface as serialise errors rather than silent string drift.

## Golden-Hash Regime (3 Layers)

Determinism is asserted **intra-architecture** with pinned golden hashes. Layers grow in scope:

### Layer 1 — Inline unit-test goldens

| File | Constant | Scenario |
|---|---|---|
| `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs:964` | `GOLDEN = "2c69ab58e4d76525d79df1dd68e6c539d8303fca95b44847243e0f062617ea79"` | RB-reserved both-dynamic; 5 slots × 3 priority txs |
| `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs:989` | `GOLDEN = "7a976da3778c11887665769a6af32eccc41f6d735b2140ef035fee67d05eb91c"` | Un-reserved both-dynamic; mixed priority + standard |
| `sim-rs/sim-core/src/sim/tests/m3_actors.rs:374` (unpinned, only intra-run equality) | — | Actor-driven scenario — proves two runs match; the suite-level goldens pin the value |

Hashed events: `Event::TXIncluded` and `Event::TXEvictedQuoteDrift` only. Encoding is byte-stable (`u64::to_le_bytes`, `lane` mapped to 0/1, `TransactionId` rendered via `Display`).

### Layer 2 — `experiment-suite verify <suite.yaml>`

`sim-rs/sim-cli/src/runner.rs::verify_suite` (line 422) re-runs every `Completed` `(job, seed)` entry recorded in `manifest.json` and asserts the freshly-computed `pricing_event_stream.sha256` equals the value persisted under the run's `output_path`. Bails on:
- malformed stored hash (empty / non-hex / wrong length) — covered by `verify_suite_bails_on_empty_stored_hash`, `verify_suite_bails_on_non_hex_stored_hash`
- drift between fresh and stored hash

### Layer 3 — Suite-level goldens

Pinned in `sim-rs/parameters/phase-2-sweep/suites/.goldens/<suite>.sha256` (line format: `<job_name> <seed> <hash>`). One canonical baseline `(job, seed=1)` per suite, asserted by `sim-rs/sim-cli/tests/determinism.rs` via 7 `#[test] #[ignore]` functions:

```rust
#[test]
#[ignore]
fn determinism_phase_2_eip1559_robustness() {
    run_baseline_and_check_golden("phase-2-eip1559-robustness", "d8_target0.5_window32", 1);
}
```

| Suite | Baseline job | Goldens file |
|---|---|---|
| `phase-2-eip1559-robustness` | `d8_target0.5_window32` | `parameters/phase-2-sweep/suites/.goldens/phase-2-eip1559-robustness.sha256` |
| `phase-2-eip1559-smoothing` | `window32` | `parameters/phase-2-sweep/suites/.goldens/phase-2-eip1559-smoothing.sha256` |
| `phase-2-priority-only-rb-reserved` | `multiplier_x4` | `parameters/phase-2-sweep/suites/.goldens/phase-2-priority-only-rb-reserved.sha256` |
| `phase-2-priority-only-unreserved` | `multiplier_x4` | `parameters/phase-2-sweep/suites/.goldens/phase-2-priority-only-unreserved.sha256` |
| `phase-2-two-lane-both-dynamic` | `partitioned_x4` | `parameters/phase-2-sweep/suites/.goldens/phase-2-two-lane-both-dynamic.sha256` |
| `phase-2-rb-scarcity` | `rb_baseline` | `parameters/phase-2-sweep/suites/.goldens/phase-2-rb-scarcity.sha256` |
| `phase-2-urgency-inversion` | `correctly_priced` | `parameters/phase-2-sweep/suites/.goldens/phase-2-urgency-inversion.sha256` |

**Regeneration:** `UPDATE_GOLDENS=1 cargo test --release -- --ignored determinism` (writes file rather than asserting). After regenerating, commit and tag with `m5-goldens-<n>`.

**Path-rebasing helper:** `sim-rs/sim-cli/tests/determinism.rs::rebase_suite_paths` rewrites every relative `PathBuf` in a freshly-loaded `Suite` onto `CARGO_MANIFEST_DIR/..` so the test passes regardless of cargo's working directory.

**Output redirection:** each test redirects `suite.output_dir` to a per-test `tempfile::tempdir()` so the test never writes to `sim-rs/output/`. The committed goldens file is the only repo-tracked artefact the test cares about.

## Reproducibility Patterns

**Seeded RNG everywhere.** `ChaChaRng::seed_from_u64(<seed>)` is the only RNG construction pattern in simulation paths and tests. `thread_rng()` does not appear in `sim-core/src/sim/` or `sim-core/src/tx_*`.

**RNG seed chaining:** drivers seed a top-level `ChaChaRng` from `SimConfiguration.seed`, then call `rng.next_u64()` to derive a per-node seed. See `sim-rs/sim-core/src/sim/tests/m1_smoke.rs:223-235`, `m2_two_lane.rs:250`, `m3_actors.rs:242`.

**Deterministic clock:** `MockClockCoordinator::advance_time(Timestamp)` steps the clock to an exact target. Driver loops compute the next event time (slot boundary or deferred event) and `min`-clamp to advance.

**Ordered collections:** `BTreeMap<TransactionId, ResidentEntry>` in `MempoolGate` so eviction iteration order is stable across runs. `HashMap` is allowed in driver scaffolding (test-only side state) but not in the simulated mempool / pricing kernel.

## Test Naming

Test function names are full snake_case English sentences:

- `smoke_run_produces_refunds_and_evictions`
- `rejects_when_max_fee_below_quote`
- `eip1559_at_target_does_not_move`
- `eip1559_above_target_moves_up_within_step_clamp`
- `eip1559_uses_ceil_rounding_per_spec`
- `multiplier_floor_holds_after_standard_moves_up`
- `rb_reserved_only_emits_priority_sample_for_rb`
- `high_urgency_actor_picks_priority_lane_under_two_lane`
- `underwater_actor_with_skip_submits_no_txs`
- `pricing_event_stream_deterministic_across_runs`
- `verify_suite_bails_on_empty_stored_hash`

Names embed the property under test. The test body's assertion failure message then quotes the spec/plan line being defended.

## Assertion Patterns

**Standard `assert!` / `assert_eq!`** with descriptive messages embedding the offending value:

```rust
// sim-rs/sim-core/src/sim/tests/m1_smoke.rs:367-370
assert!(
    posted_fee > *max_fee,
    "eviction record violates spec: minFeeB + q×bytes = {posted_fee} should exceed max_fee = {max_fee}"
);
```

**Golden-hash mismatch messages quote the recovery path:**
```rust
// sim-rs/sim-core/src/sim/tests/m2_two_lane.rs:967-969
"pricing event-stream hash drifted from the pinned golden value. \
 If the simulation logic legitimately changed, update the constant \
 in this test and document the change in m2-handoff.md."
```

**Pattern-matching on domain enums:**
```rust
// sim-rs/sim-core/src/sim/mempool_gate.rs:350-358
match err {
    AdmissionRejection::InsufficientMaxFee { posted_fee, max_fee_lovelace } => {
        assert_eq!(posted_fee, 199_381);
        assert_eq!(max_fee_lovelace, 199_380);
    }
    other => panic!("expected InsufficientMaxFee, got {other:?}"),
}
```

## What Gets Mocked vs Not

**Mocked:**
- Lottery (VRF): `MockLotteryResults` with `configure_win`
- Time advancement: `MockClockCoordinator`
- Filesystem (in runner tests): `tempfile::tempdir`

**Not mocked:**
- The pricing kernel (`Eip1559Pricing`, `TwoLanePricing`, `BaselinePricing`, `CapacityWeightedWindow`) — tested directly because it is the unit under test
- `MempoolGate` — tested directly with real `Transaction` values
- The event stream — captured from a real `mpsc::UnboundedReceiver`, not synthesised
- Config parsing — real `serde_yaml::from_slice` against `parameters/config.default.yaml`

## Coverage Gaps

No coverage tooling is configured (no `tarpaulin`, no `llvm-cov`, no codecov badge). Coverage is not enforced numerically; the milestone exit criteria in `docs/phase-2/implementation-plan.md` act as the de-facto checklist.

**Known untested areas:**

- **Cross-architecture determinism.** Pinned hashes reproduce bit-identically only on the development machine (x86_64 / glibc). The math (`libm::pow`/`libm::round`, `u128` rationals, integer arithmetic) is bit-stable cross-arch given identical inputs, but the simulator inherits `f64` from `main` in non-pricing paths (slot lottery, propagation, `rand_distr` sampling internals) which has not been hardened. Documented in `sim-rs/CLAUDE.md` "Determinism scope" and `docs/phase-2/m5-handoff.md`. A second-arch CI pipeline is flagged as deferred infrastructure work.

- **`sim-cli/src/metrics/` writers** (`time_series.rs`, `diagnostics.rs`, `comparison.rs`) have minimal direct test coverage — they are exercised end-to-end by the suite runner during the slow `--ignored` determinism tests but not asserted on field-by-field.

- **`sim-cli/src/runner.rs::run_job` / `run_suite` happy paths** are exercised only by the slow `#[ignore]`'d determinism tests. The fast `cargo test --workspace` path covers only the malformed-hash bails (`verify_suite_bails_on_empty_stored_hash`, `verify_suite_bails_on_non_hex_stored_hash`).

- **Multi-node topologies under `TwoLaneVariant` un-reserved.** The M2/M3 tests are single-producer (`topology-single-producer.yaml`). Multi-mempool slot-battle pricing-state-rollback was an open question deferred in the M2 handoff.

- **Legacy protocols** (`leios.rs`, `stracciatella.rs`) are not phase-2 scope; their tests pre-date this branch and are not maintained as actively as the linear-Leios + pricing surface.

## Running Specific Tests

```bash
# All workspace tests (excludes #[ignore]'d)
cd sim-rs && cargo test --workspace

# Only the pricing kernel inline tests
cd sim-rs && cargo test --package sim-core --lib tx_pricing

# Only the M2 cross-platform golden
cd sim-rs && cargo test --package sim-core pricing_event_stream_deterministic_across_runs

# Slow suite-level goldens (release-mode, all 7)
cd sim-rs && cargo test --release -- --ignored determinism

# A single suite's golden
cd sim-rs && cargo test --release determinism_phase_2_rb_scarcity -- --ignored
```

---

*Testing analysis: 2026-05-13*
