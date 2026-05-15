# Coding Conventions

**Analysis Date:** 2026-05-15

This document codifies the conventions and idiomatic patterns used in
the phase-2 dynamic-pricing simulator (`dynamic-experiment` branch).
It is prescriptive: future Claude instances writing or modifying code
in this workspace should follow these rules. The numeric-representation
contract is load-bearing — violating it flips the determinism golden
hashes documented in [TESTING.md](TESTING.md).

## Numeric representation contract — the hard rule

**Simulation-affecting state is integer/rational/u128 or
bit-reproducible cross-platform math; never plain `f64`.** This is
the single most important convention on the branch. It is enforced by
the M2/M3 unit-test golden hashes and the M5 suite-level goldens. Any
accidental `f64` entry into a hot path flips them.

### Hot-path types (simulation-affecting)

Use these for everything that influences which transaction lands in
which block:

| Concern | Type | Example file |
|---|---|---|
| `quote_per_byte` | `u64` direct (never derived from f64 coefficient at query time) | `sim-rs/sim-core/src/model.rs` (`PerLaneQuote { standard: u64, priority: u64 }`) |
| Controller window sums | `u128` integer sums | `sim-rs/sim-core/src/model.rs` (`WindowAggregate`) |
| Controller arithmetic intermediates | `u128` rationals | `sim-rs/sim-core/src/tx_pricing/single_lane.rs` `compute_eip1559_step` |
| Fee charging / refunds / `max_fee_lovelace` | `u64` lovelace | `sim-rs/sim-core/src/sim/mempool_gate.rs` |
| Actor lane-choice expected utility | `i128` lovelace via `libm::round` before comparison | `sim-rs/sim-core/src/tx_actors.rs` `expected_utility_lovelace` |
| `posted_lane` / `served_lane` | `Lane` enum (no integer width) | `sim-rs/sim-core/src/tx_pricing/mod.rs` |

### Reporting-only f64 (cold paths)

These are computed from the deterministic event stream but **never feed
back into simulation decisions**:

- `retained_value`, `net_utility`, `retained_value_ratio` (see
  `welfare` submodule in `sim-rs/sim-core/src/tx_actors.rs`).
- `RunSummary` fields like `priority_retained_value_total`,
  `block_generation_probability`, shock metrics (see
  `sim-rs/sim-cli/src/metrics/collector.rs`).
- Time-series ratios in `sim-rs/sim-cli/src/metrics/time_series.rs`.

Comment block at top of `sim-rs/sim-core/src/tx_actors.rs` lines 22-23
makes this split explicit: "`welfare` — f64 reporting-only formulas".

### Bit-deterministic f64 escape hatch

When a calculation genuinely needs f64 (e.g. exponential decay for
retained-value), route it through `libm` for cross-arch bit-stability:

```rust
// sim-rs/sim-core/src/tx_actors.rs lane_choice::expected_utility_lovelace
let factor = libm::pow(urgency, -latency_blocks);
let retained_f64 = (value_lovelace as f64) * factor;
let retained_lov = libm::round(retained_f64) as i128;   // round FIRST, then cast
```

Rules:
- Use `libm::pow`, `libm::round`, `libm::exp`, `libm::ceil` — NOT
  `f64::powf`, `f64::round`, etc. The std library variants are not
  bit-stable across architectures.
- Always call `libm::round` (or `libm::ceil`) **before** the integer
  cast. `as i128` truncates toward zero, biasing positive values
  downward by up to one lovelace and silently changing the rule the
  hash is over.
- `urgency: f64` on `Transaction` is read **only** by the actor
  lane-choice math (it routes through `libm::pow` + `libm::round`).
  Never read it from any other simulation-affecting code path.
- The lane-choice module has a written caveat about `rand_distr`
  internals using `f64::ln`/`f64::exp` for sampling — this is the
  only known cross-arch drift source remaining on the branch. See
  `sim-rs/sim-core/src/tx_actors.rs` lines 28-35.

### Saturating arithmetic everywhere on hot paths

Hot-path u64/u128 arithmetic uses `saturating_add` / `saturating_mul`
/ `saturating_sub`. Roughly 30+ uses in `sim-rs/sim-core/src/tx_pricing/`
alone. Examples:

```rust
// sim-rs/sim-core/src/tx_pricing/window.rs
agg.standard_sum_bytes = agg
    .standard_sum_bytes
    .saturating_add(sample.relevant_bytes as u128);

// sim-rs/sim-core/src/tx_pricing/single_lane.rs compute_eip1559_step
let num_a = util_num.saturating_mul(target_den);
let den = util_den.saturating_mul(target_num).saturating_mul(d);
```

Belt-and-braces `debug_assert!(.checked_mul(..).is_some())` is paired
with saturating ops in places where validation has already ruled out
overflow — both protect against future regressions:

```rust
// sim-rs/sim-core/src/tx_pricing/single_lane.rs
debug_assert!(util_num.checked_mul(target_den).is_some());
// ... then ...
let num_a = util_num.saturating_mul(target_den);
```

### Ceiling division pattern

Used in fee rounding (mechanism spec line 175) and the multiplier-floor
invariant. The idiom:

```rust
// sim-rs/sim-core/src/tx_pricing/single_lane.rs
let new_quote = if new_quote_num == 0 {
    0
} else {
    (new_quote_num - 1) / move_den + 1
};

// sim-rs/sim-core/src/tx_pricing/two_lane.rs apply_floor
let floor = if scaled == 0 {
    0u128
} else {
    (scaled - 1) / den + 1
};
```

Never use plain `/` for fee rounding without an explicit decision —
floor vs ceil changes the golden hashes.

## Lane vocabulary — no "tier" anywhere

`Lane` is a two-variant enum at `sim-rs/sim-core/src/tx_pricing/mod.rs`:

```rust
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Lane {
    Standard,
    Priority,
}
```

Single-lane mechanisms collapse both to `Standard`. Two-lane mechanisms
(four variants in `TwoLaneVariant`) populate both. **There is no tier
vocabulary on the branch.** Do not introduce types or function names
that use "tier", "tier-1", "tier-2", "tier-N" — the historical
`pricing-sim-base` branch's naming is observable as prior art only.
The hard rule comes from `docs/phase-2/implementation-plan.md`.

Carried on:
- `Transaction.posted_lane` — the lane the user authorised at submission
- `Event::TXIncluded.posted_lane` and `.served_lane` — both recorded
  so refunds can be computed when the served lane differs (RB-reserved
  EB-below-capacity refunds posted-priority down to standard).

## Pure-function chain-derived computation

The `PricingBackend` trait at
`sim-rs/sim-core/src/tx_pricing/mod.rs` is the architectural seam:

```rust
pub trait PricingBackend: Send + Sync {
    fn compute_derived_quote(
        &self,
        parent_quote: crate::model::PerLaneQuote,
        parent_aggregate: crate::model::WindowAggregate,
        parent_samples: &[PricedBlockSample],
        evicted_samples: &[PricedBlockSample],
    ) -> (crate::model::PerLaneQuote, crate::model::WindowAggregate);
    // ...
}
```

Rules for any new pricing logic:
- **No `&mut self` anywhere on `PricingBackend`.** The backend holds
  no mutable controller state. The struct is a settings carrier; the
  trait method is a pure function.
- All four `compute_derived_quote` arguments are owned values or
  slices. Returns a fresh `(PerLaneQuote, WindowAggregate)`. No side
  effects.
- Sibling RBs with identical `(parent_quote, parent_aggregate,
  parent_samples, evicted_samples)` must produce identical outputs —
  asserted by `sibling_rbs_produce_identical_derived_quote_pure` in
  `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs`.
- The simulator owns block packing and the canonical chain. The
  backend never sees simulator types; the only seam is `ChainView`
  (read-only chain walk).
- Helper free-functions (`compute_eip1559_step`,
  `worst_case_eip1559_quote`, `aggregate_from_chain`,
  `update_aggregate`) take explicit inputs rather than reading
  `&self`. Tests assert purity directly via these free functions.

## Multiplier-floor invariant — u128 intermediates only

The invariant `c_priority ≥ multiplier_floor × c_standard` is enforced
on `quote_per_byte` (a `u64`), never on a fractional coefficient `c`.
The intermediates run in `u128`:

```rust
// sim-rs/sim-core/src/tx_pricing/two_lane.rs apply_floor
fn apply_floor(&self, q_standard: u64, q_priority: u64) -> u64 {
    let num = self.settings.multiplier_floor.numerator as u128;
    let den = self.settings.multiplier_floor.denominator as u128;
    let scaled = (q_standard as u128).saturating_mul(num);
    let floor = if scaled == 0 {
        0u128
    } else {
        (scaled - 1) / den + 1   // ceil division
    };
    debug_assert!(floor <= u64::MAX as u128, /* ... */);
    let floor_u64 = u64::try_from(floor).unwrap_or(u64::MAX);
    q_priority.max(floor_u64)
}
```

The floor is enforced on the **output** of `compute_derived_quote`,
not on any persistent state (because under chain-derivation there is
none). Constructor-time enforcement also raises the priority initial
quote up to the floor at cold start.

## Serde casing — mixed by historical accident, do not standardise

This is a documented gotcha (CLAUDE.md "Conventions / gotchas"):

| Type | Casing | File |
|---|---|---|
| YAML config types (`RawPricingConfig`, `RawTwoLaneConfig`, ...) | `#[serde(rename_all = "kebab-case")]` | `sim-rs/sim-core/src/config.rs` |
| Suite YAML schema (`Suite`, `Job`) | `kebab-case` | `sim-rs/sim-cli/src/suite.rs` |
| Manifest types (`JobStatus`, `JobEntry`, `Manifest`) | `kebab-case` | `sim-rs/sim-cli/src/runner.rs` |
| `RunSummary` (run_summary.json) | Rust `snake_case` (no `rename_all`) | `sim-rs/sim-cli/src/metrics/collector.rs` |

**Both shapes coexist on disk in persisted artefacts.** Standardising
would invalidate every persisted manifest under `sim-rs/output/` and
force re-runs of all 72 (job, seed) pairs. Not worth the churn.

**Rule for new schema additions:** match the surrounding type's
existing convention. Don't switch a type from one casing to the other.

## Defensive validation at construction

Settings structs validate at construction and bail with `anyhow`
descriptive errors:

```rust
// sim-rs/sim-core/src/tx_pricing/single_lane.rs Eip1559Settings::validate
pub fn validate(&self) -> anyhow::Result<()> {
    if self.min_fee_a == 0 {
        anyhow::bail!("Eip1559Settings.min_fee_a must be non-zero");
    }
    if self.target_num == 0 || self.target_num >= self.target_den {
        anyhow::bail!(
            "Eip1559Settings.target_num/target_den must be a fraction in (0, 1); got {}/{}",
            self.target_num,
            self.target_den
        );
    }
    // Bound controller-parameter intermediates so the u128 rationals
    // can't overflow at runtime for any plausible per-sample bytes value:
    const MAX_BYTES_PER_SAMPLE_LOG2: u32 = 40;
    // ...
    Ok(())
}
```

Pattern: validate aggressively at config-load, then rely on the
constraint inside the hot path. `TwoLaneSettings::validate` rejects
zero denominators, multiplier-floor ratios outside `[1, 2^32]`, and
mismatched `min_fee_a` between priority/standard.

The `Multiplier` constructor pattern is the most-compact form:

```rust
// sim-rs/sim-core/src/tx_pricing/mod.rs
impl Multiplier {
    pub fn new(numerator: u64, denominator: u64) -> anyhow::Result<Self> {
        if denominator == 0 {
            anyhow::bail!("Multiplier denominator must be non-zero");
        }
        Ok(Self { numerator, denominator })
    }
}
```

## Error handling — anyhow throughout

The crate uses `anyhow::Result<T>` for all fallible paths. `bail!` for
single-line error returns; `.with_context(|| format!("..."))` to
attach situational context as a chain.

Examples in `sim-rs/sim-cli/src/runner.rs`:

```rust
let text = std::fs::read_to_string(&summary_path).with_context(|| {
    format!("reading run_summary at {}", summary_path.display())
})?;

// Aggregated multi-error pattern (verify_suite_with_run_id):
let mut combined = anyhow::anyhow!("{} verify task(s) errored", errors.len());
for e in errors {
    combined = combined.context(format!("{e:#}"));
}
return Err(combined);
```

- `anyhow` is in both crates' `[dependencies]` (sim-core/Cargo.toml,
  sim-cli/Cargo.toml).
- `Result` aliases imported via `use anyhow::{Result, ...}`.
- Library code surfaces errors up; the runner aggregates and prints
  via `tracing`.

## Logging — tracing, info/warn/error

Logging uses the `tracing` crate (NOT `log`, NOT `println!`).

```rust
// sim-rs/sim-cli/src/runner.rs
tracing::info!("determinism verify ok: {checked} (job, seed) pairs match");
tracing::warn!(...);
tracing::error!(...);
```

Levels:
- `info!` for normal progress (job start/complete).
- `warn!` for recoverable oddities (manifest resume, missing fields).
- `error!` for run failures.
- `debug!` is reserved for one-off investigation; not in committed
  hot paths.

Tests and unit modules do not log — they use `assert!` macros and
`panic!` on construction errors. Initialiser is in `sim-cli`'s main
binaries (tracing-subscriber feature `env-filter`).

## Module documentation — top-of-file `//!` blocks

Every module starts with a `//!` doc-comment explaining its scope,
spec/plan references, and the determinism contract. Examples:

- `sim-rs/sim-core/src/tx_pricing/mod.rs` lines 1-21: module purpose,
  spike 007 reference, "All simulation-affecting state ... f64 never
  enters this module's hot paths".
- `sim-rs/sim-core/src/tx_pricing/single_lane.rs` lines 1-34: backend
  list, update rule in pseudocode, memoisation note.
- `sim-rs/sim-core/src/sim/mempool_gate.rs` lines 1-26: responsibility
  list, "All state is `u64`/`u128`; no f64."
- `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs` lines 1-12: M2
  scenario-test scope, single-producer rationale.

**When adding a new module, write a module-level doc:**
- One-sentence purpose.
- Forward-pointer to `docs/phase-2/mechanism-design.md` or
  `implementation-plan.md` (line numbers are common).
- Determinism contract for this module ("All state is ...; f64 never
  enters ...").

## Inline comments — explain the WHY, especially for hash-load-bearing decisions

The codebase is heavily commented in places where a future reader
might reasonably try to "simplify" a calculation that's actually
load-bearing for cross-arch determinism. Examples:

```rust
// sim-rs/sim-core/src/tx_pricing/single_lane.rs
// Ceiling division per the spec rounding regime
// (implementation-plan.md line 175: "Final `quote_per_byte` is
// integer-rounded once per update via `ceil`"). Floor would let
// small above-target moves stick at the old quote, e.g.
// `44 × 1.125 = 49.5 → floor 49` (no movement past 50) where
// the spec specifies 50.

// sim-rs/sim-core/src/tx_actors.rs lane_choice
// Pinned rounding rule: round-half-away-from-zero via
// `libm::round` *before* the integer cast. Rounding here is
// what determines the rule — once `libm::round` returns,
// the f64 holds an integer value and `as i128` is a pure
// type conversion. Without the explicit `libm::round`,
// `retained_f64 as i128` would truncate toward zero,
// biasing positive expected_utility values downward by up
// to one lovelace (and the integer event stream's hash
// would depend on the chosen rule).
```

Pattern: when you make a numerical decision that the hash depends on,
say so in a comment with the concrete worked example.

## Naming patterns

**Files:** `snake_case.rs`. Tests in `sim-core/src/sim/tests/` follow
the `m{1,2,3}_topic.rs` milestone-tagged convention.

**Types:** `UpperCamelCase` (`PricingBackend`, `WindowAggregate`,
`PerLaneQuote`, `TwoLaneVariant`). `Raw*` prefix for serde-deserialise
types that mirror YAML schema (`RawPricingConfig`, `RawEip1559Config`,
`RawTwoLaneConfig`).

**Functions:** `snake_case`. Pure free functions in `tx_pricing` get
verb-first names: `compute_eip1559_step`, `worst_case_eip1559_quote`,
`aggregate_from_chain`, `update_aggregate`, `snapshot_at`,
`apply_floor`.

**Constants:** `SCREAMING_SNAKE_CASE`. Examples:
`MAX_BYTES_PER_SAMPLE_LOG2`, `MAX_PROJECTION_BLOCKS`,
`MULTIPLIER_FLOOR_RATIO_CAP_LOG2`, `RB_BODY_MAX` (in tests).

**Test names:** describe the asserted property, not the steps.
`baseline_pricing_does_not_drift`, `eip1559_above_target_moves_up_within_step_clamp`,
`sibling_rbs_produce_identical_derived_quote_pure`,
`pricing_event_stream_deterministic_across_runs`.

## Import organisation

Standard order observed across the codebase:

```rust
// 1. std imports
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

// 2. external crate imports
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

// 3. local crate imports (sim_core::, sim_cli::, crate::)
use sim_core::{config::SimConfiguration, sim::Simulation};

// 4. relative super:: / self:: imports
use super::{Lane, PricedBlockSample, PricingBackend};
```

Grouped imports use `{...}` braces. Path aliases are rare — the only
notable one is `crate::tx_pricing::Lane` re-exported through
`tx_pricing::mod.rs` `pub use` statements.

## Deterministic container choice

Always use `BTreeMap<K, V>` and `BTreeSet<T>` for collections that
influence simulation order, persisted artefacts, or test output.
`HashMap`/`HashSet` are reserved for collections whose iteration order
is not observable (intermediate scratch, per-tx metadata in the
collector that gets joined back via key lookup).

Examples:
- `sim-rs/sim-cli/src/runner.rs` line 73: `Manifest.jobs:
  BTreeMap<String, BTreeMap<String, JobEntry>>` — keyed by (job_name,
  seed_string) for stable on-disk order regardless of completion
  order.
- `sim-rs/sim-core/src/sim/tests/m2_two_lane.rs` line 204: `BTreeMap`
  for `updates` because "iteration order is deterministic. Single-node
  tests today don't strictly need this, but the driver shape will be
  reused for multi-node M6+ tests".
- `sim-rs/sim-cli/src/metrics/collector.rs` lines 256-277:
  `HashMap<TransactionId, TxMeta>` and `HashMap<u32, ComponentSummary>`
  are OK because they're joined back via direct key lookup; the
  reporting order is fixed via separate output writers.

## Function design

- Public function signatures take explicit types — no `impl Trait` in
  parameters except where iterator-style flexibility is needed
  (`aggregate_from_chain` takes `impl IntoIterator<Item = &'a
  PricedBlockSample>`).
- Long u128 expressions are broken across lines with intermediate
  let-bindings whose names document the math. See
  `compute_eip1559_step` for an extended example.
- Helper functions are private (`fn add_one`, `fn sub_one` in
  `window.rs`) and tested via their public callers.

## Module design — `pub use` re-exports

`mod.rs` files re-export the most-used types and functions so callers
can `use crate::tx_pricing::{Eip1559Pricing, TwoLaneVariant,
update_aggregate}` instead of reaching into submodules:

```rust
// sim-rs/sim-core/src/tx_pricing/mod.rs
pub mod single_lane;
pub mod two_lane;
pub mod window;

pub use single_lane::{BaselinePricing, Eip1559Pricing, Eip1559Settings};
pub use two_lane::{TwoLanePricing, TwoLaneSettings, TwoLaneVariant};
pub use window::{aggregate_from_chain, update_aggregate};
```

`sim-rs/sim-cli/src/metrics/mod.rs` follows the same pattern for
`MetricsCollector`, `RunSummary`, `comparison`, `diagnostics`,
`time_series`.

## Avoid

- **No `f64` in hot paths** (covered above).
- **No `pricing-sim-base` content.** That branch is observable as
  prior art only; no file, type, or function moved across.
- **No `println!` / `eprintln!` outside tests and CLI binaries.**
  Use `tracing::*` macros.
- **No mutable controller state in `PricingBackend` impls.** Chain-
  derived is the architecture; per-node accumulators are gone.
- **No re-introducing a delta-slot read pattern** for `submit_slot` —
  `Event::TXGenerated.slot: u64` is the source of truth (M4).
- **No reading `urgency: f64` outside the actor lane-choice math.**

---

*Convention analysis: 2026-05-15*
