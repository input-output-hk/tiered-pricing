# Testing Patterns

**Analysis Date:** 2026-05-15

This document codifies the test strategy for the phase-2 dynamic-pricing
simulator (`dynamic-experiment` branch). The simulator's determinism
contract is enforced by a three-layer test regime — unit-test goldens,
`experiment-suite verify`, and suite-level goldens — covering scopes
from a single backend's `compute_eip1559_step` call up to a 200-slot
suite baseline run.

## Test framework

**Runner:** Rust built-in `#[test]` via `cargo test`. No third-party
test runner (no `nextest`, no `criterion` in committed deps for
phase-2).

**Assertion macros:** `assert!`, `assert_eq!`, `assert_ne!` from std,
sometimes with formatted messages tying the failure back to the spec
or plan line number:

```rust
assert!(
    q.standard >= 875,
    "expected ≥ -12.5% clamp, got {}",
    q.standard
);
```

**Async test runtime:** `tokio` features `macros`, `rt` in
`sim-core/Cargo.toml` dev-dependencies. Tests build a per-test
`current_thread` runtime explicitly (the `Simulation` future isn't
`Send`):

```rust
// sim-rs/sim-cli/tests/determinism.rs
let runtime = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .expect("building tokio runtime");
let summary = runtime
    .block_on(async { runner::run_job(&suite, job_idx, seed).await })
    .unwrap_or_else(|e| panic!("running {suite_name}/{baseline_job} seed={seed}: {e:#}"));
```

**Hashing for goldens:** `sha2 = "0.10"` (`Sha256` + `Digest` trait),
`hex = "0.4"` for encoding. Both in `sim-core/Cargo.toml`
dev-dependencies and `sim-cli/Cargo.toml` deps.

**Fixtures via tempdir:** `tempfile = "3"` in
`sim-cli/Cargo.toml` dev-dependencies, used by both
`tests/determinism.rs` and `tests/parallel_runner.rs` to keep test
output out of the canonical `sim-rs/output/` tree.

## Run commands

All commands assume `pwd = sim-rs/`.

```bash
# Standard cycle: excludes the slow #[ignore]'d goldens.
cargo test --workspace

# Suite-level determinism goldens (~1.5s in --release). #[ignore]'d.
cargo test --release -- --ignored determinism

# Regenerate the suite goldens after an intentional simulator change.
UPDATE_GOLDENS=1 cargo test --release -- --ignored determinism

# Optional wall-clock smoke for the parallel runner.
cargo test --release -- --ignored parallel_wall_clock

# Determinism verify: re-run every Completed (job, seed) and assert
# the freshly-computed pricing_event_stream.sha256 matches the
# persisted on-disk value.
cargo run --release --bin experiment-suite -- verify \
    parameters/phase-2-sweep/suites/phase-2-eip1559-robustness.yaml
```

After regenerating goldens, commit the updated `.goldens/*.sha256`
files and tag the resulting commit:

```bash
git add parameters/phase-2-sweep/suites/.goldens
git commit -m "M5 goldens regenerated after <reason>"
git tag -a m5-goldens-<n> -m "..."
```

## Test file organisation

Tests are co-located inline in the crate they exercise (for unit tests)
or live in `tests/` directories (for integration tests). No separate
test crate.

**Unit tests** — at the bottom of the file they cover:
- `sim-rs/sim-core/src/tx_pricing/single_lane.rs` lines 403-575:
  `mod tests` for `BaselinePricing` and `Eip1559Pricing` backends.
- `sim-rs/sim-core/src/tx_pricing/two_lane.rs` lines 410-727:
  `mod tests` covering all four `TwoLaneVariant` arms plus the
  multiplier-floor invariant.
- `sim-rs/sim-core/src/tx_pricing/window.rs` lines 114-201:
  `mod tests` for `aggregate_from_chain` and `update_aggregate`.
- `sim-rs/sim-cli/src/runner.rs` lines 955-1050 (approx):
  `mod tests` for `verify_suite` corruption-detection paths.

**Scenario/integration unit tests** — in `sim-rs/sim-core/src/sim/tests/`:
- `mod.rs` (4 lines) — module declarations only.
- `m1_smoke.rs` (376 lines) — M1 single-lane smoke through full
  simulator.
- `m2_two_lane.rs` (1,480 lines) — M2 four-variant scenarios + pinned
  pricing-event-stream goldens.
- `m3_actors.rs` (471 lines) — M3 actor-model integration + pricing
  goldens with actors driving demand.
- `linear_leios.rs` (589 lines) — protocol-level scenarios.

**Cross-crate integration tests** — in `sim-rs/sim-cli/tests/`:
- `determinism.rs` (229 lines) — M5 suite-level golden regime.
- `parallel_runner.rs` (380 lines) — concurrency + resume semantics
  for the experiment-suite runner.

## Test structure

### Inline `mod tests` pattern (unit tests)

```rust
// sim-rs/sim-core/src/tx_pricing/single_lane.rs lines 403-575
#[cfg(test)]
mod tests {
    use crate::model::{PerLaneQuote, WindowAggregate};
    use crate::tx_pricing::{BlockKind, Lane, PricedBlockSample, PricingBackend};

    use super::{BaselinePricing, Eip1559Pricing, Eip1559Settings, compute_eip1559_step};

    fn standard_rb(bytes: u64, capacity: u64) -> PricedBlockSample {
        PricedBlockSample {
            block_kind: BlockKind::RankingBlock,
            controller_lane: Lane::Standard,
            relevant_bytes: bytes,
            relevant_capacity: capacity,
        }
    }

    fn settings(initial: u64, d: u64) -> Eip1559Settings { /* ... */ }

    #[test]
    fn eip1559_at_target_does_not_move() {
        let pricing = Eip1559Pricing::new(settings(1000, 8)).unwrap();
        let (q, _) = pricing.compute_derived_quote(
            PerLaneQuote::flat(1000),
            WindowAggregate::ZERO,
            &[standard_rb(50, 100)],   // util = 0.5 = target
            &[],
        );
        assert_eq!(q.standard, 1000);
    }
}
```

Conventions:
- `#[cfg(test)]` at the module level; one or two `use super::*;` /
  explicit `use super::{...}` imports.
- Small helper builders (`standard_rb`, `settings`) at module top to
  keep test bodies focused.
- One assertion per test where practical; one test per asserted
  property.

### Driver-based scenario tests (sim/tests/m2_two_lane.rs, m3_actors.rs)

The M2 and M3 integration tests use a hand-rolled `*Driver` struct that
wraps the simulator's single-producer state. Pattern:

```rust
// sim-rs/sim-core/src/sim/tests/m2_two_lane.rs lines 111-245
struct TwoLaneDriver {
    config: Arc<SimConfiguration>,
    nodes: HashMap<NodeId, LinearLeiosNode>,
    lottery: HashMap<NodeId, Arc<MockLotteryResults>>,
    time: MockClockCoordinator,
    slot: u64,
    queued: HashMap<NodeId, EventResult<LinearLeiosNode>>,
    deferred: BTreeMap<Timestamp, Vec<(NodeId, TimedEvent)>>,
    events_rx: mpsc::UnboundedReceiver<(Event, Timestamp)>,
    next_tx_id: u64,
}

impl TwoLaneDriver {
    fn new(pricing: RawPricingConfig) -> Self { /* ... */ }
    fn make_tx(&mut self, bytes: u64, max_fee_lovelace: u64, posted_lane: Lane) -> Arc<Transaction>;
    fn submit_tx(&mut self, tx: Arc<Transaction>);
    fn win_lottery(&mut self, kind: LotteryKind, value: u64);
    fn next_slot(&mut self);
    fn drain_events(&mut self) -> Vec<Event>;
}
```

Each driver provides:
- A `new(...)` constructor that builds a one-node `SimConfiguration`
  from `parameters/config.default.yaml` and a hand-built
  `RawTopology`.
- `make_tx` / `submit_tx` for hand-crafted transactions.
- A `MockLotteryResults` interface (`win_lottery`) so tests can pin
  exactly which slots produce RBs.
- A `MockClockCoordinator` for deterministic time advancement.
- An `mpsc::UnboundedReceiver` consumed via `drain_events()` to read
  the per-test event stream.

The drivers in M2 and M3 share almost identical shape but are
intentionally copy-pasted — they cover different test surfaces
(M2 = backend wiring, M3 = actor wiring).

### Asserting "no drift" vs "moves within bounds"

For controller behaviour, tests pin both directions (clamp at
upper/lower bound, and the "no movement" cases):

```rust
// At-target: no movement.
#[test]
fn eip1559_at_target_does_not_move() { /* ... util = 0.5 == target ... */ }

// Saturated: up move within +1/D clamp.
#[test]
fn eip1559_above_target_moves_up_within_step_clamp() {
    assert!(q.standard > 1000, "expected upward move, got {}", q.standard);
    assert!(q.standard <= 1125, "expected ≤ +12.5% clamp, got {}", q.standard);
}

// Empty block: down move within -1/D clamp.
#[test]
fn eip1559_below_target_moves_down_within_step_clamp() { /* mirror */ }

// 200 steps under empty demand: must floor at min_fee_a, never below.
#[test]
fn eip1559_floor_at_min_fee_a() { /* loop, then assert_eq!(q.standard, 44) */ }
```

## Mocking

**No third-party mocking framework.** The simulator exposes mock-
friendly seams directly:

- **`MockClockCoordinator`** at `sim-rs/sim-core/src/clock/mock.rs` —
  drop-in for the production `ClockCoordinator`. Tests call
  `time.advance_time(target)` to step time deterministically.
- **`MockLotteryResults`** at `sim-rs/sim-core/src/sim/lottery.rs` —
  `Arc<MockLotteryResults>` is installed via
  `node.mock_lottery(lr.clone())` and the test calls
  `lr.configure_win(LotteryKind::GenerateRB, 0)` to force the producer
  to win the RB lottery at the next slot.
- **`test_endorse_eb_dry_run`, `test_partition_trigger`,
  `test_eb_endorsement_valid`** on `LinearLeiosNode` — test-only entry
  points exposing private helpers without breaking encapsulation.
  Used by `m2_two_lane.rs::eb_partition_unit_test_four_cases` to
  exhaustively cover the four spec-mandated branches of the partition
  trigger.
- **`pricing_snapshot()`, `current_chain_tip_quote_for_test()`,
  `chain_tip_stored_derived_quote_for_test()`,
  `gate_contains_for_test()`** — read-only inspectors so tests can
  assert internal state without exposing it as a public API surface.

What is **never** mocked:
- The pricing backends (`BaselinePricing`, `Eip1559Pricing`,
  `TwoLanePricing`) — they're tested as the real implementations,
  with hand-built sample slices.
- The mempool gate (`MempoolGate`) — instantiated directly with a
  hand-built `MempoolGateConfig`.
- The `WindowAggregate` math — pure functions tested via
  `aggregate_from_chain` and `update_aggregate` directly.

## Fixtures and factories

### Inline test-config builders

The phase-2 tests build `RawTwoLaneConfig` / `RawEip1559Config` /
`RawPricingConfig` via small helpers near the top of each test file:

```rust
// sim-rs/sim-core/src/sim/tests/m2_two_lane.rs lines 50-76
fn two_lane_cfg(
    variant: RawTwoLaneVariant,
    selection_order: LaneSelectionOrder,
) -> RawTwoLaneConfig {
    RawTwoLaneConfig {
        variant,
        priority: RawEip1559Config {
            initial_quote_per_byte: MIN_FEE_A,
            target_num: 1,
            target_den: 2,
            max_change_denominator: 4,
            window_length: 4,
        },
        standard: /* ... same as priority ... */,
        multiplier_floor_num: 16,
        multiplier_floor_den: 1,
        lane_selection_order: selection_order,
    }
}
```

Magic constants live at the top of each test file:

```rust
const RB_BODY_MAX: u64 = 90_000;
const EB_REF_MAX: u64 = 1_000_000;
const TX_BYTES_DEFAULT: u64 = 30_000;
const MIN_FEE_B: u64 = 155_381;
const MIN_FEE_A: u64 = 44;
```

### Real YAML fixtures referenced from tests

The `parallel_runner.rs` tests reference real parameter YAMLs:

```rust
// sim-rs/sim-cli/tests/parallel_runner.rs lines 131-142
fn two_valid_jobs() -> Vec<(String, PathBuf)> {
    vec![
        (
            "baseline".to_string(),
            param("parameters/phase-2-sweep/pricing/baseline_flat_fee.yaml"),
        ),
        (
            "eip1559_window32".to_string(),
            param("parameters/phase-2-sweep/pricing/eip1559_d8_target0.5_window32.yaml"),
        ),
    ]
}
```

These are rebased onto `sim-rs/` via `sim_rs_root()` so the tests work
regardless of cargo's working directory.

### `include_bytes!` for `parameters/config.default.yaml`

Both M1 and M2 test files include the protocol baseline at build time
and mutate fields in-place:

```rust
// sim-rs/sim-core/src/sim/tests/m2_two_lane.rs lines 78-89
fn one_node_config(pricing: RawPricingConfig) -> Arc<SimConfiguration> {
    let mut params: RawParameters =
        serde_yaml::from_slice(include_bytes!("../../../../parameters/config.default.yaml"))
            .unwrap();
    params.leios_variant = LeiosVariant::Linear;
    params.tx_max_size_bytes = RB_BODY_MAX;
    params.rb_body_max_size_bytes = RB_BODY_MAX;
    params.eb_referenced_txs_max_size_bytes = EB_REF_MAX;
    params.vote_threshold = 1;
    params.pricing = Some(pricing);
    // ...
}
```

This couples test compilation to the YAML's serde shape — a renamed
field in `RawParameters` will surface at compile time, not at runtime.

## The three-layer determinism regime

This is the load-bearing test surface. Each layer asserts the same
property — bit-identical reproduction of the `TXIncluded` +
`TXEvictedQuoteDrift` event stream — at a different scope.

### Layer 1 — unit-test goldens in m2/m3

**Files:**
- `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs` lines 1216-1242:
  `pricing_event_stream_deterministic_across_runs` — RB-reserved
  both-dynamic variant.
- `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs` lines 1244-1267:
  `pricing_event_stream_deterministic_across_runs_unreserved` —
  un-reserved both-dynamic variant.
- `sim-rs/sim-core/src/sim/tests/m3_actors.rs` lines 374-441:
  `actor_event_stream_deterministic_across_runs` — actor-driven
  demand scenario.

Each pins a 64-character hex SHA256 constant in source:

```rust
// sim-rs/sim-core/src/sim/tests/m2_two_lane.rs lines 1216-1242
#[test]
fn pricing_event_stream_deterministic_across_runs() {
    let h1 = run_seeded_pricing_scenario();
    let h2 = run_seeded_pricing_scenario();
    assert_eq!(h1, h2, "pricing event stream must be deterministic");
    const GOLDEN: &str = "2c69ab58e4d76525d79df1dd68e6c539d8303fca95b44847243e0f062617ea79";
    assert_eq!(
        h1, GOLDEN,
        "pricing event-stream hash drifted from the pinned golden value. \
         If the simulation logic legitimately changed, update the constant \
         in this test and document the change in m2-handoff.md."
    );
}
```

These tests run on every `cargo test --workspace`. The pinned hashes
are intra-arch (development machine is x86_64 / glibc). The test
asserts both:
1. Same-process reproducibility (`h1 == h2`).
2. Pinned-value match (`h1 == GOLDEN`).

When the simulator's logic changes intentionally, update the constant
in the test source and document the change in the relevant milestone
handoff (`docs/phase-2/m{2,3}-handoff.md`).

### Layer 2 — `experiment-suite verify` re-running every Completed (job, seed)

**File:** `sim-rs/sim-cli/src/runner.rs` lines 591-794
(`verify_suite_with_run_id`).

After a suite has been run, `experiment-suite verify <suite.yaml>`:
1. Walks the manifest at `<output_dir>/manifest.json`.
2. For every (job, seed) with `status = Completed`, reads the stored
   `pricing_event_stream.sha256` from disk.
3. Re-runs `run_job(&suite, job_idx, seed)` from scratch.
4. Asserts `summary.pricing_event_stream_sha256 == stored_hash`.
5. Reports per-(job, seed) match/mismatch and bails on any drift.

Defensive checks:
- A stored hash that is empty or non-hex (corruption) is rejected
  before any work is spawned (`sim-rs/sim-cli/src/runner.rs` lines
  660-667).
- Verification runs in parallel just like the original suite run; the
  JoinSet pattern uses per-thread `current_thread` tokio runtimes
  because `Simulation` is `!Send`.

This layer catches:
- Non-determinism between two runs at *different times* with possibly
  different process state.
- Simulator regressions where a refactor changes the event stream but
  leaves the layer-1 unit-test scenarios untouched.

### Layer 3 — suite-level goldens (slow, `#[ignore]`'d)

**File:** `sim-rs/sim-cli/tests/determinism.rs` (229 lines, 7
`#[test]` + `#[ignore]`'d functions).

**Goldens directory:**
`sim-rs/parameters/phase-2-sweep/suites/.goldens/` contains seven
`.sha256` files, one per pinned suite:

```text
phase-2-eip1559-robustness.sha256
phase-2-eip1559-smoothing.sha256
phase-2-priority-only-rb-reserved.sha256
phase-2-priority-only-unreserved.sha256
phase-2-rb-scarcity.sha256
phase-2-two-lane-both-dynamic.sha256
phase-2-urgency-inversion.sha256
```

**Format:** one line per file — `<baseline_job> <seed> <sha256>`:

```text
d8_target0.5_window32 1 92701c73944ead391c490ffd1819bae9338e3742848fd51fa002ca197c1ea1b7
```

**Test mechanics:**
- Each test calls `run_baseline_and_check_golden(suite_name,
  baseline_job, seed)`.
- The harness loads `parameters/phase-2-sweep/suites/<suite>.yaml`,
  rebases all relative paths via `rebase_suite_paths` (so tests work
  regardless of cargo's working dir), redirects `output_dir` to a
  fresh `tempfile::TempDir`, pins `slots = 200` and `topology =
  parameters/phase-2-sweep/topology-single-producer.yaml`.
- Runs the (job, seed) end-to-end through `runner::run_job(...)`.
- Asserts `summary.pricing_event_stream_sha256 == stored_hash` or, if
  `UPDATE_GOLDENS=1`, writes the new hash to disk.

**Why `#[ignore]`'d:** each baseline run is a 200-slot single-producer
simulation; full file run is ~1.5s in `--release` mode. Excluded from
the default `cargo test --workspace` cycle so regular test runs stay
fast. Run explicitly via:

```bash
cargo test --release -- --ignored determinism
```

**`UPDATE_GOLDENS=1` regeneration workflow:**
1. Make the intentional simulator change.
2. Run: `cd sim-rs && UPDATE_GOLDENS=1 cargo test --release -- --ignored determinism`
3. The test writes the freshly-computed hash to the goldens file
   instead of asserting against it (lines 137-144 of
   `tests/determinism.rs`).
4. `git add parameters/phase-2-sweep/suites/.goldens && git commit -m "M5 goldens regenerated after <reason>"`
5. Optionally: `git tag -a m5-goldens-<n> -m "..."` for a named pin.

## What the pricing event-stream hash is over

**Hot-path events only:** `Event::TXIncluded` and
`Event::TXEvictedQuoteDrift`. Nothing else.

Encoding (from
`sim-rs/sim-core/src/sim/tests/m2_two_lane.rs` lines 1289-1344 and
mirrored in `sim-rs/sim-cli/src/metrics/collector.rs` lines 393-415):

```rust
match ev {
    Event::TXIncluded {
        id, slot, bytes, posted_lane, served_lane,
        max_fee_lovelace, actual_fee_lovelace, refund_lovelace, ..
    } => {
        hasher.update(b"INCL");
        hasher.update(id.to_string().as_bytes());
        hasher.update(slot.to_le_bytes());
        hasher.update(bytes.to_le_bytes());
        hasher.update([
            match posted_lane { Lane::Standard => 0, Lane::Priority => 1 },
            match served_lane { Lane::Standard => 0, Lane::Priority => 1 },
        ]);
        hasher.update(max_fee_lovelace.to_le_bytes());
        hasher.update(actual_fee_lovelace.to_le_bytes());
        hasher.update(refund_lovelace.to_le_bytes());
    }
    Event::TXEvictedQuoteDrift {
        id, slot, bytes, posted_lane, current_quote_per_byte, max_fee_lovelace, ..
    } => {
        hasher.update(b"EVCT");
        hasher.update(id.to_string().as_bytes());
        hasher.update(slot.to_le_bytes());
        hasher.update(bytes.to_le_bytes());
        hasher.update([match posted_lane { Lane::Standard => 0, Lane::Priority => 1 }]);
        hasher.update(current_quote_per_byte.to_le_bytes());
        hasher.update(max_fee_lovelace.to_le_bytes());
    }
    _ => {}
}
```

The events fed to the hasher are exactly the integer-valued events
that determine simulator outcomes. Reporting-only `f64` outputs
(`PricingTick.standard_window_util_x_1e9`, retained-value, net-utility)
are intentionally excluded — they don't feed back into simulation
decisions, so they don't enter the determinism contract.

This means: **any accidental `f64` entry into a hot path (admission,
eviction, fee charging, controller coefficient) immediately flips
every layer of the regime.** Conversely, changes to reporting-only
metrics do NOT flip these hashes.

## Intra-arch determinism scope

**The pinned hashes reproduce bit-identically on the same arch only.**
The development machine is `x86_64 / glibc`. The underlying math (
`libm::pow`, `libm::round`, `u128` rationals, integer arithmetic)
is bit-stable across architectures *given identical inputs*. But the
simulator inherits `f64` from `main` in non-pricing code paths (slot
lottery, propagation, distribution sampling via `rand_distr`) that
have not been hardened for cross-arch determinism.

Notable caveat documented at `sim-rs/sim-core/src/tx_actors.rs` lines
28-35: `rand_distr` internals use `f64::ln` / `f64::exp` (not in
IEEE-754's bit-exact mandate), so the inputs to `libm::pow` can drift
across arches.

**Cross-architecture CI verification is not yet built.** A second-arch
build pipeline is infrastructure work outside phase-2's code scope —
flagged in `docs/phase-2/m5-handoff.md` for the CIP / external
write-up. Until then:
- The M2/M3 unit-test goldens carry a comment ("expected to match on
  aarch64 because every simulation-affecting path is integer/rational")
  but this is a theoretical claim, not a tested assertion.
- The M5 suite-level goldens are pinned only on x86_64 / glibc.

## Concurrency tests (parallel_runner.rs)

`sim-rs/sim-cli/tests/parallel_runner.rs` (380 lines) exercises the
experiment-suite runner's `JoinSet`-pattern dispatcher. Unlike
`determinism.rs`, these tests are **NOT `#[ignore]`'d** — they run on
every `cargo test --workspace`.

Tests cover:

| Test | Property asserted |
|---|---|
| `parallel_run_matches_sequential` | `--parallelism 4` produces bit-identical `pricing_event_stream.sha256` to `--parallelism 1` for every (job, seed). |
| `partial_failure_leaves_recoverable_manifest` | A configured-to-fail job ends `Failed`; siblings complete cleanly; no `Running` entries left. |
| `resume_under_parallelism_skips_completed` | Re-running with same `run_id` doesn't rewrite `run_summary.json` (mtime stable). |
| `parallel_wall_clock_smoke` (`#[ignore]`'d) | Parallel run isn't dramatically slower than sequential — catches the regression where parallelism somehow serialises. |

Fixture shape:
- A `TinySuiteBuilder` writes a 2-job × 2-seed × 100-slot suite YAML
  to a `tempfile::TempDir` referencing real parameter YAMLs under
  `parameters/phase-2-sweep/`.
- Wall time per fixture build is under 5 seconds in `--release`; whole
  file runs in under 30 seconds.

## Coverage

**No coverage requirement enforced.** No `cargo tarpaulin` /
`cargo llvm-cov` in committed CI. Coverage in this codebase is judged
by the determinism regime (event-stream hashes catch behavioural
regressions) plus per-property unit tests, not by line-coverage
percentages.

## Test types — summary

| Type | Scope | Location | Run cycle |
|---|---|---|---|
| Pure-function unit tests | `compute_eip1559_step`, `update_aggregate`, `apply_floor`, etc. | `tx_pricing/*.rs` inline `mod tests` | every `cargo test --workspace` |
| Scenario unit tests | Single-producer simulator runs with hand-crafted txs | `sim/tests/m{1,2,3}_*.rs` | every `cargo test --workspace` |
| Pricing event-stream goldens (unit-test layer) | RB-reserved + un-reserved + actor variants, pinned hashes | `sim/tests/m{2,3}_*.rs` | every `cargo test --workspace` |
| Runner concurrency tests | `JoinSet` dispatcher, resume, partial failure | `sim-cli/tests/parallel_runner.rs` | every `cargo test --workspace` |
| Verify subcommand | Re-run all Completed (job, seed); compare to stored hashes | `experiment-suite verify <suite.yaml>` | on demand |
| Suite-level goldens (M5) | 7 baseline (job, seed=1) pairs at 200 slots | `sim-cli/tests/determinism.rs` | `cargo test --release -- --ignored determinism` |

## Common patterns

### Async tests via blocking `block_on`

The simulator's `Simulation` future isn't `Send`, so tests build a
per-test `current_thread` runtime and call `block_on`:

```rust
let runtime = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .expect("building tokio runtime");
let summary = runtime
    .block_on(async { runner::run_job(&suite, job_idx, seed).await })
    .unwrap();
```

`#[tokio::test]` is NOT used in the phase-2 integration tests for
this reason — it would build a multi-thread runtime by default and
the simulator can't run on one.

### Error-path tests (constructor rejection)

```rust
// sim-rs/sim-core/src/tx_pricing/two_lane.rs
#[test]
fn rejects_zero_denominator_floor() {
    let mut s = settings(TwoLaneVariant::RbReservedBothDynamic);
    s.multiplier_floor = Multiplier { numerator: 16, denominator: 0 };
    assert!(TwoLanePricing::new(s).is_err());
}

#[test]
fn rejects_floor_below_one() {
    let mut s = settings(TwoLaneVariant::RbReservedBothDynamic);
    s.multiplier_floor = Multiplier::new(1, 2).unwrap();
    assert!(TwoLanePricing::new(s).is_err());
}
```

Pattern: build a settings struct that violates the invariant, call
`new()`, assert `is_err()`. No `unwrap_err()` chaining unless the
exact error message is the property under test.

### Purity tests for pure-function backends

```rust
// sim-rs/sim-core/src/tx_pricing/single_lane.rs
#[test]
fn compute_eip1559_step_is_pure() {
    let s = settings(1000, 8);
    let a = compute_eip1559_step(1000, (50, 100), &s);
    let b = compute_eip1559_step(1000, (50, 100), &s);
    assert_eq!(a, b);
}

// sim-rs/sim-core/src/tx_pricing/two_lane.rs
#[test]
fn sibling_rbs_produce_identical_derived_quote() {
    // Two children of the same parent with identical compute
    // inputs must produce identical (PerLaneQuote, WindowAggregate).
    let (a_q, a_agg) = pricing.compute_derived_quote(parent_q, parent_agg, &samples, &[]);
    let (b_q, b_agg) = pricing.compute_derived_quote(parent_q, parent_agg, &samples, &[]);
    assert_eq!(a_q, b_q);
    assert_eq!(a_agg, b_agg);
}
```

Pattern: call the function twice with identical inputs; assert
identical outputs. Critical for chain-derived backends where slot-
battle siblings must produce bit-identical `derived_quote` values.

### Hash-comparison assertions with diagnostic messages

```rust
assert_eq!(
    fresh, stored_hash,
    "{suite_name}/{baseline_job} seed={seed} hash drifted: \
     fresh={fresh} stored={stored_hash}. \
     Re-run with UPDATE_GOLDENS=1 if the change is intentional."
);
```

Pattern: include both values plus the regeneration command in the
failure message so debugging a CI failure doesn't require reading
the test source.

---

*Testing analysis: 2026-05-15*
