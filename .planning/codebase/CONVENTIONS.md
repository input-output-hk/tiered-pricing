# Coding Conventions

**Analysis Date:** 2026-05-13

## Numeric Representation Contract (HARD RULE)

**Simulation-affecting state must be integer / rational / `u128` / `i128`, never plain `f64`.** This is the strongest rule on the branch. Enforced by the determinism golden hashes — any accidental `f64` entry into a hot path flips a pinned hash.

**Storage rules:**
- `quote_per_byte` is stored as `u64` directly (never derived from an `f64` coefficient at query time). See `sim-rs/sim-core/src/tx_pricing/single_lane.rs` and `sim-rs/sim-core/src/tx_pricing/mod.rs`.
- The EIP-1559 update rule runs in `u128` rationals (`aggregateUtil = Σ num / Σ den`, `target = (num, den)`, `D` integer, clamped step on `quote_per_byte`). See `sim-rs/sim-core/src/tx_pricing/single_lane.rs` (`step`, `step_with_lane`).
- Multiplier-floor invariant (`c_priority ≥ multiplier_floor × c_standard`) is enforced on `quote_per_byte` with `u128` intermediates, never on `c` directly. See `sim-rs/sim-core/src/tx_pricing/two_lane.rs`.
- `CapacityWeightedWindow` is a `u128` ring buffer (`sum_bytes`, `sum_capacity`). See `sim-rs/sim-core/src/tx_pricing/window.rs`.
- `max_fee_lovelace`, `actual_fee_lovelace`, `refund_lovelace` are all `u64`. See `sim-rs/sim-core/src/sim/mempool_gate.rs`.
- Lane choice math uses `libm::pow` + `libm::round` into `i128` lovelace before any `>` comparison so it is bit-deterministic on the same architecture. See `sim-rs/sim-core/src/tx_actors.rs`.

**`f64` is allowed only in reporting outputs.** `retained_value`, `net_utility`, `retained_value_ratio` and friends in `sim-rs/sim-cli/src/metrics/collector.rs` are computed and stored as `f64`. These never feed back into simulation decisions.

**Carve-out:** `Transaction.urgency: f64` exists on the simulator's core type (`sim-rs/sim-core/src/model.rs`) but is read **only** by the actor lane-choice math which routes it through `libm::pow` + `libm::round` into `i128` lovelace. Never read it from any other simulation-affecting code path. Documented in `sim-rs/CLAUDE.md` "Conventions / gotchas".

**Pricing event-stream golden hashes** are over `Event::TXIncluded` and `Event::TXEvictedQuoteDrift` only — exactly the events that determine simulator outcomes. Hashing code in `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs` (`run_seeded_pricing_scenario`, `run_seeded_pricing_scenario_unreserved`) and `sim-rs/sim-cli/src/metrics/collector.rs`.

## No Prior-Art Content Rule

**No `pricing-sim-base` content.** That branch is observable as prior art only — no file, type, or function was moved across. Hard rule from `docs/phase-2/implementation-plan.md`.

## Naming Patterns

**Files:** snake_case `.rs` (e.g. `single_lane.rs`, `two_lane.rs`, `mempool_gate.rs`, `tx_actors.rs`, `tx_pricing/`, `m1_smoke.rs`, `m2_two_lane.rs`, `m3_actors.rs`).

**Types:** PascalCase (`PricingBackend`, `TwoLaneVariant`, `MempoolGate`, `EvictionRecord`, `InclusionCharge`, `LaneSelectionOrder`, `Eip1559Settings`).

**Functions / methods:** snake_case (`current_quote`, `update_after_block`, `lane_validity_rule`, `samples_for_block`, `try_admit`, `revalidate`, `on_inclusion`, `step_with_lane`).

**Constructors:** `new`, `new_*`, or domain-specific builders (`Eip1559Pricing::new`, `BaselinePricing::new`, `TwoLanePricing::new`, `MempoolGate::new`). Constructors that can fail return `anyhow::Result<Self>` (e.g. `Multiplier::new`, `Eip1559Pricing::new`).

**Constants:** SCREAMING_SNAKE_CASE (`MIN_FEE_A`, `MIN_FEE_B`, `RB_BODY_BYTES`, `RB_BODY_MAX`, `EB_REF_MAX`, `MAX_VOLATILITY_AWARE_BLOCKS`, `GOLDEN`).

**Enums and variants:** PascalCase (`Lane::Standard`, `Lane::Priority`, `BlockKind::RankingBlock`, `BlockKind::EndorserBlock`, `LaneValidityRule::None`, `LaneValidityRule::PriorityOnly`, `JobStatus::Pending`/`Running`/`Completed`/`Failed`, `AdmissionRejection::InsufficientMaxFee`).

**Lane vocabulary:** strictly two variants — `Standard` and `Priority`. **No tier vocabulary anywhere on the branch.** Single-lane mechanisms collapse both to `Standard`. See `sim-rs/sim-core/src/tx_pricing/mod.rs` line 27.

## Module / Workspace Layout

**Workspace at `sim-rs/Cargo.toml`** with two members:
- `sim-rs/sim-core/` (library, edition 2024, rust-version 1.88)
- `sim-rs/sim-cli/` (library + two binaries: `sim-cli`, `experiment-suite`; edition 2024, rust-version 1.88)

**Module pattern:** prefer `mod.rs`-bearing directories for multi-file submodules (`sim-core/src/tx_pricing/{mod,single_lane,two_lane,window}.rs`, `sim-core/src/sim/{...}.rs` plus `sim-core/src/sim/tests/{mod,linear_leios,m1_smoke,m2_two_lane,m3_actors}.rs`).

**Top-of-file module docs** are mandatory for non-trivial modules — every file in `tx_pricing/`, `sim/mempool_gate.rs`, `tx_actors.rs`, `sim-cli/src/runner.rs`, `sim-cli/src/suite.rs`, `sim-cli/tests/determinism.rs` begins with a `//!` block that pins:
1. What the module does in one sentence
2. A pointer to `docs/phase-2/mechanism-design.md` or `implementation-plan.md` for spec/plan provenance
3. Any numeric-representation invariants the module guarantees

## Serde Conventions

**Casing is mixed by historical accident** — both shapes coexist on disk in persisted artefacts (manifest.json, run_summary.json). Standardising would invalidate every persisted manifest under `sim-rs/output/`, forcing re-runs of all (job, seed) pairs.

**kebab-case for YAML configs and on-disk manifest formats** — use `#[serde(rename_all = "kebab-case")]`:
- All `Raw*` config types in `sim-rs/sim-core/src/config.rs` (`RawParameters`, `RawTopology`, `RawNode`, `RawPricingConfig`, `RawTwoLaneConfig`, `RawActorProfile`, `RawActorComponent`, `RawLanePolicy`, `RawMaxFeePolicy`, `DistributionConfig`, `LeiosVariant`, etc.)
- `Lane`, `BlockKind`, `LaneSelectionOrder` enums in `sim-rs/sim-core/src/tx_pricing/mod.rs`
- `Suite`, `Job`, `JobOverrides` in `sim-rs/sim-cli/src/suite.rs`
- `Manifest`, `JobEntry`, `JobStatus` in `sim-rs/sim-cli/src/runner.rs`

**Rust snake_case for `RunSummary`** (no `rename_all`) — `sim-rs/sim-cli/src/metrics/collector.rs` (`pricing_event_stream_sha256`, `total_txs_included`, `multiplier_floor_breaches`, etc.). When adding schema fields, **match the surrounding type's existing convention** — do not standardise.

**Tagged enums use `#[serde(tag = "kind", rename_all = "kebab-case")]`** for variant discrimination in YAML:
- `MaxFeePolicy` in `sim-rs/sim-core/src/tx_actors.rs` line 61 (`tag = "kind"`)
- `RawLanePolicy`, `RawArrivalRate`, `RawPricingConfig` in `sim-rs/sim-core/src/config.rs`
- `DistributionConfig` uses `tag = "distribution"` (line 42)

**`#[serde(default)]`** on optional fields in `Manifest`/`JobEntry`/`Suite`/`JobOverrides` and on `RunSummary::pricing_event_stream_sha256` (defaults to empty string for backward compatibility with older runs).

## Error Handling

**`anyhow::Result` is the standard return type** for fallible operations:
- Constructor validation (`Eip1559Settings::validate`, `TwoLaneSettings::validate`, `MaxFeePolicy::validate`, `Multiplier::new`, `CapacityWeightedWindow::new`, `ActorProfile::validate`, `ActorComponent::validate`)
- I/O paths in `sim-rs/sim-cli/src/runner.rs` (`Manifest::load_or_init`, `Manifest::save`, `run_suite`, `run_job`, `verify_suite`, `persist_run_summary`, etc.)
- Suite loading: `Suite::load` in `sim-rs/sim-cli/src/suite.rs`

**Bail with `anyhow::bail!`** for validation failures with a descriptive message:
```rust
// sim-rs/sim-core/src/tx_pricing/single_lane.rs:91
anyhow::bail!("Eip1559Settings.min_fee_a must be non-zero");
```
Pattern: `<TypeName>.<field> must <constraint>`. Always print the offending value in the message.

**`.context(...)` for I/O wrapping** — `sim-rs/sim-cli/src/runner.rs` uses `use anyhow::{Context, Result}` and wraps file paths into error context (e.g. when loading suite YAML).

**Domain enums for non-Result rejection paths** — `AdmissionRejection` in `sim-rs/sim-core/src/sim/mempool_gate.rs` (lines 38-56) is a non-`anyhow` enum because callers (block builder, admission flow) want to pattern-match on the reject reason and convert into events. Pattern: where the caller cares about the reason, define a domain enum; where the caller just propagates, return `anyhow::Result`.

**`Option` for "absent" / "skipped"** — `MempoolGate::on_inclusion` returns `Option<InclusionCharge>` (None means tx not resident, not an error).

**Saturating / checked arithmetic** on the hot path:
- `u64::checked_mul`, `checked_add` in `MempoolGate::fee_at` (returns `None` on overflow → maps to `AdmissionRejection::FeeOverflow`)
- `u128::checked_pow`, `saturating_mul`, `saturating_add` in `worst_case_eip1559_quote` (`sim-rs/sim-core/src/tx_pricing/single_lane.rs` lines 258-285)

## Panic Policy

**Panics are reserved for invariant violations the type system cannot express.** Production code in the pricing kernel (`tx_pricing/`, `mempool_gate.rs`, `tx_actors.rs`) does **not** panic — it returns `anyhow::Result` or domain enums.

**Allowed panic sites** (grep `panic!` shows ~12 occurrences in sim-core, none in pricing kernel):
- `sim-rs/sim-core/src/sim/linear_leios.rs:1421` — "how did we validate this EB without ever seeing it?" (genuinely-unreachable mempool state)
- `sim-rs/sim-core/src/sim/linear_leios.rs:1812` — "missing a TX in our mempool" (invariant)
- `sim-rs/sim-core/src/clock/mock.rs` and `sim-rs/sim-core/src/clock/coordinator.rs` — test-harness invariants ("waiter waited twice", "advanced time too far")
- `sim-rs/sim-core/src/sim/leios.rs:653`, `stracciatella.rs:772` — legacy protocols, voting invariants
- `sim-rs/sim-core/src/sim/cpu.rs:142,162,165,174,177` — CPU-task-state-machine invariants

**`.expect("...")` for "validation has already happened upstream"** — exactly one site in the pricing kernel: `sim-rs/sim-core/src/tx_actors.rs:670` — `Poisson::new(rate).expect("arrival_rate_per_slot validated > 0")` after a finite-rate check on line 665.

**`.unwrap()` in production code is rare.** Common in tests (Result-returning constructors with statically-known good args). Producton callers either `?`-propagate or pattern-match.

## Logging

**Framework:** `tracing` (configured via `tracing-subscriber` with `EnvFilter` in `sim-rs/sim-cli/src/main.rs` and `sim-rs/sim-cli/src/bin/experiment-suite/main.rs`).

**Levels in use:** `info!`, `warn!`, `error!`, `trace!`. `debug!` is rare. Convention: prefer `tracing::info!` macro path over imported alias when only one or two call sites exist; import `use tracing::{info, warn};` when used heavily (e.g. `sim-rs/sim-core/src/config.rs`).

**Patterns:**
- Suite progress: `tracing::info!` in `sim-rs/sim-cli/src/runner.rs` lines 202, 211, 252, 480, 504 — one info line per (job, seed) start/skip/finish.
- Validation surprises: `tracing::warn!` in `sim-rs/sim-core/src/config.rs:1130` and `sim-rs/sim-cli/src/runner.rs:626,643`.
- Drop-on-error: `tracing::warn!` in `sim-rs/sim-core/src/events.rs:993` ("tried sending event after aggregator finished").

**Hot-path logging is forbidden.** No `info!`/`warn!`/`trace!` calls inside controller update paths, mempool admission, or any function that runs per-tx or per-block — they would flip determinism hashes only through wall-clock perturbation, but more importantly they violate the "no f64 in simulation-affecting state" rule because `tracing` events carry timestamps. Use `tracing::trace!` only in coarse-grained driver loops (`sim-rs/sim-core/src/sim/driver.rs:142`).

## Documentation Style

**Module-level docs** (`//!`) open every non-trivial file with a 5-50-line block. Required content:
1. One-sentence summary
2. Pointer to spec / plan (`docs/phase-2/mechanism-design.md`, `docs/phase-2/implementation-plan.md` with line range)
3. Numeric-representation invariants the module guarantees
4. Pointer to per-milestone handoff (`docs/phase-2/m{1,2,3,4,5}-handoff.md`) for any decision that was open in the plan

Example pattern: `sim-rs/sim-core/src/tx_pricing/single_lane.rs` lines 1-22, `sim-rs/sim-core/src/sim/mempool_gate.rs` lines 1-26.

**Function-level docs** (`///`) on every public item in `tx_pricing/`, `tx_actors.rs`, `mempool_gate.rs`. Include the formula or invariant when it isn't obvious from the signature.

**Anchor citations to the spec / plan with line numbers** wherever a decision is non-obvious — e.g. `// implementation-plan.md line 175: "Final quote_per_byte is integer-rounded once per update via ceil ..."` in `sim-rs/sim-core/src/tx_pricing/single_lane.rs:417-422`.

## Trait Pattern

**`PricingBackend` trait** (`sim-rs/sim-core/src/tx_pricing/mod.rs:129`) is the policy seam. Required bound: `Send + Sync`. Default implementations are provided for `lane_validity_rule`, `lane_selection_order`, `min_priority_premium_multiplier`, `samples_for_block`, `worst_case_quote_at` — single-lane backends inherit them; two-lane backends override.

**Trait-design rule:** the backend never sees simulator types (no `Transaction`, no `Mempool`, no `Block`). The simulator constructs `PricedBlockSample` / `BlockLaneBreakdown` and hands them to the backend. Selection lives in `LinearLeiosNode::select_eb_with_partition` / `sample_from_mempool_lane_aware` in `sim-rs/sim-core/src/sim/linear_leios.rs`.

## Imports

**Standard order:**
1. `std::` imports first (e.g. `use std::collections::{BTreeMap, HashMap};`)
2. Blank line
3. External crates (alphabetical): `anyhow`, `rand`, `rand_chacha`, `serde`, `sha2`, `tokio`, `tracing`, etc.
4. Blank line
5. Crate-internal `use crate::{...}` block

Example: `sim-rs/sim-core/src/sim/tests/m1_smoke.rs` lines 21-44, `sim-rs/sim-cli/src/runner.rs` lines 15-40.

**Re-exports:** crate facades use `pub use` to flatten the public surface. See `sim-rs/sim-core/src/tx_pricing/mod.rs:21-23` re-exporting `BaselinePricing`, `Eip1559Pricing`, `TwoLanePricing`, etc.

## Lints / Format

**No project-level `rustfmt.toml`, `clippy.toml`, `deny.toml`, or `.cargo/config.toml`** in the repo. Default `rustfmt` and `clippy` apply.

**No `#![deny(...)]`, `#![warn(...)]`, or `#![forbid(...)]` crate-level attributes** on `sim-core` or `sim-cli` `lib.rs` files (grep finds zero outside of vendored deps in `target/`).

**Inline `#[allow(unused)]`** is used sparingly for test/mock entry points (e.g. `MockLotteryResults::configure_win` in `sim-rs/sim-core/src/sim/lottery.rs:31`, `LotteryConfig::Mock` line 42, `id_wrapper!` macro in `sim-rs/sim-core/src/model.rs:24`).

## Function / Module Design

**Validation at construction, not per-call.** `Eip1559Settings::validate`, `TwoLaneSettings::validate`, `MaxFeePolicy::validate`, `ActorComponent::validate`, `ActorProfile::validate` are called from `*::new` constructors so a successfully-constructed value is unconditionally usable.

**Multiplier-floor invariant enforcement** lives **inside the controller update path** (`TwoLanePricing::update_after_block` in `sim-rs/sim-core/src/tx_pricing/two_lane.rs`) and is enforced on `quote_per_byte`, not on `c`. Constructor-time enforcement also raises priority's initial quote up to the floor if needed.

**`MempoolGate` is the sole byte-cap authority** (`sim-rs/sim-core/src/sim/mempool_gate.rs`). It owns admission (`try_admit`), revalidation on quote change (`revalidate`), and inclusion charging (`on_inclusion`). Reject-only on full mempool — no eviction of valid txs to make room. Other layers must consult the gate; they must not duplicate fee/byte logic.

**RB-reduced overlays are full replacements**, not stacked overlays. The runner's `JobOverrides` picks `overrides.protocol` OR `default_protocol`, never both — so the three `parameters/phase-2-sweep/protocol-rb-reduced-{half,third,quarter}.yaml` files duplicate everything from `protocol-base.yaml` and override only the `rb-body-max-size-bytes` knob. **Future additions to `protocol-base.yaml` must be propagated manually to all three RB-reduced overlays.**

## Determinism Invariants

- **Use `ChaChaRng` seeded with `ChaChaRng::seed_from_u64(...)`** wherever simulation RNG is needed. Never `thread_rng()` in simulation-affecting paths.
- **Use `BTreeMap` over `HashMap`** for any structure whose iteration order can affect simulation output. `sim-rs/sim-core/src/sim/mempool_gate.rs` uses `BTreeMap<TransactionId, ResidentEntry>` for the resident set so eviction iteration order is deterministic.
- **Use `libm::pow` + `libm::round`** for cross-platform-stable floating-point intermediates that feed integer comparisons (`sim-rs/sim-core/src/tx_actors.rs` `lane_choice::pick`).
- **The metrics collector's representative node** is pre-set by `runner::run_job` to the lexicographically smallest node name. The lazy "first-tick wins" fallback in `MetricsCollector::is_representative` (`sim-rs/sim-cli/src/metrics/collector.rs`) is for tests / standalone callers only.
- **`Event::TXGenerated` carries `slot: u64`** so the metrics collector reads `submit_slot` from the event field, not from a delta-tracking ordering invariant. Do not re-introduce a delta-slot read pattern.

## Test-Only Conventions

- `#[cfg(test)] mod tests { ... }` inline at the bottom of each module file for tight unit tests against the module's own surface.
- Cross-module / integration tests live in `sim-rs/sim-core/src/sim/tests/` registered via `sim-rs/sim-core/src/sim/tests/mod.rs`.
- Suite-level slow tests live in `sim-rs/sim-cli/tests/determinism.rs` and are `#[ignore]`'d by default.
- Test function names are full English sentences in snake_case (`smoke_run_produces_refunds_and_evictions`, `rejects_when_max_fee_below_quote`, `eip1559_at_target_does_not_move`, `multiplier_floor_holds_after_standard_moves_up`, `high_urgency_actor_picks_priority_lane_under_two_lane`).

---

*Convention analysis: 2026-05-13*
