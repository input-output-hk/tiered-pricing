# CLAUDE.md

## Project Overview

This is a simulation testbed for evaluating **tiered transaction fee mechanisms** for Cardano's linear-Leios protocol. It extends an existing Leios network simulator (`sim-rs/`) with tiered pricing, actor-based transaction generation, and welfare metrics.

The core research question: can a tiered pricing mechanism (Kiayias et al., "Tiered Mechanisms for Blockchain Transaction Fees", 2304.06014) improve welfare and inclusion fairness compared to flat-fee or EIP-1559-style mechanisms during congestion?

## Repository Structure

```
sim-rs/
├── sim-core/               # Core simulation library (Rust)
│   └── src/
│       ├── model.rs            # Domain types: Transaction, Block, Endorsement, TierId, etc.
│       ├── config.rs           # YAML parameter parsing, SimConfiguration
│       ├── tx_pricing.rs       # Tiered pricing mechanism (4900+ lines)
│       │                       #   - EIP-1559-style per-tier price updates
│       │                       #   - Dynamic tier creation/removal
│       │                       #   - Delay enforcement, overflow retry
│       │                       #   - Block selection policies (shared, naive_rb_eb, continuous_rb_eb)
│       ├── tx_actors.rs        # Actor-based tx generation: distributions, phased arrival, urgency components
│       ├── sim/
│       │   ├── linear_leios.rs # Main simulation loop for linear-Leios variant (~2500 lines)
│       │   ├── tx.rs           # Transaction sampling, tier selection, value/urgency correlation
│       │   ├── driver.rs       # Simulation driver (orchestrates clock, network, sim)
│       │   └── slot.rs         # Slot lottery, block production
│       ├── clock.rs            # Simulation clock (coordinator + mock for tests)
│       ├── events.rs           # Event types emitted during simulation
│       └── network/            # Network topology, connection, message passing
├── sim-cli/                # CLI runner and metrics
│   └── src/
│       ├── main.rs             # CLI entry point, compare mode, parameter layering
│       ├── runner.rs           # Shared run execution API used by both CLIs
│       ├── suite.rs            # Config-driven experiment suite runner + manifest logic
│       └── events.rs           # Event monitoring, metrics collection, welfare tables (~3000 lines)
│                               #   - Per-actor and per-urgency-class welfare metrics
│                               #   - Retained value ratio, net utility, latency tracking
│                               #   - Time-series CSV output, pricing diagnostics
├── parameters/
│   ├── config.default.yaml     # Base simulation parameters (slot timing, block sizes, etc.)
│   ├── config.schema.json      # JSON schema for config validation
│   ├── topology.default.yaml   # Network topology
│   ├── actors/                 # Actor profile configs (TOML)
│   │   ├── paper_like_quick.toml           # Standard 5-component value-urgency mix
│   │   ├── paper_like_quick_low_value_skew.toml  # Skewed toward low-value txs
│   │   └── sundaeswap_congestion.toml      # DEX launch congestion scenario
│   ├── pricing/                # Pricing mechanism configs (TOML)
│   │   ├── baseline_quick.toml             # Fixed Cardano-style fees
│   │   ├── continuous_rb_eb_reject_overflow_aggregate_capped_tier_pressure_quick.toml
│   │   └── ...                             # Many mechanism variants
│   ├── suites/                 # Multi-job experiment suite configs (YAML)
│   │   ├── overnight-multi.yaml           # Example comparative suite
│   │   └── suite.schema.json              # YAML editor schema for suite configs
│   └── experiments/            # Experiment configs (YAML overlays on config.default.yaml)
│       ├── leios-sundaeswap-baseline.yaml
│       ├── leios-sundaeswap-aggregate-capped-tier-pressure.yaml
│       └── ...
├── scripts/
│   ├── run_sim_timestamped.sh  # Run experiments with timestamped output dirs
│   └── plot_tiers.py           # Visualization of tier dynamics
├── output/                     # Simulation output (gitignored)
└── docs/
    └── 2304.06014v1.txt        # Reference paper (text extraction)
```

## Key Concepts

### The Tiered Pricing Mechanism

Based on Kiayias et al. (2304.06014). Block space is divided into tiers with increasing delays and decreasing prices:
- **Tier 0**: Immediate inclusion (1 block delay), highest price
- **Tier 1**: 2-block delay, lower price
- **Tier k**: 2^k block delay, lowest price

Prices update per-tier using EIP-1559 rules. Delays and tier count adjust less frequently. Users self-select tiers based on their urgency and willingness-to-pay.

### Value-Urgency Model

Transactions have value `v` and urgency parameter `u > 1`. Retained value after `d` blocks of delay:

```
retained_value = v * u^(-d)
```

High-urgency users (large `u`) lose value quickly with delay and prefer expensive fast tiers. Low-urgency users (small `u`) tolerate delay for cheaper tiers. This enables **price discrimination by urgency**.

### Actor System

Each actor config defines weighted value-urgency components. A transaction samples one component, getting correlated (value, urgency) from that component's distributions. The `urgency_component_index` field tracks which component was sampled, enabling per-urgency-class welfare metrics.

### Block Structure in Linear Leios

- **Ranking Blocks (RB)**: Produced with probability 0.05/slot (~1 per 20 slots). Max body 90,112 bytes.
- **Endorser Blocks (EB)**: Reference transactions, endorsed by votes.
- With `continuous_rb_eb` policy, tiers span both RB and EB capacity.
- `rb-generation-probability: 0.05` means **1 block ≈ 20 slots** — this is critical for interpreting delay units.

### Welfare Metrics

- **Retained value ratio**: `value_at_delay / initial_value` — primary optimization target
- **Net utility**: `retained_value - fee_paid`
- **Inclusion rate**: Fraction of generated txs that get included
- **Latency**: Slots from generation to on-chain inclusion
- Per-actor and per-urgency-class breakdowns reveal whether tiers achieve price discrimination

## Building and Running

```bash
cd sim-rs
cargo build --release

# Run a single experiment
scripts/run_sim_timestamped.sh \
  --experiment parameters/experiments/leios-sundaeswap-aggregate-capped-tier-pressure.yaml \
  --label my-run

# Compare two mechanisms (A/B test)
scripts/run_sim_timestamped.sh \
  --experiment parameters/experiments/leios-sundaeswap-aggregate-capped-tier-pressure.yaml \
  --compare-experiment parameters/experiments/leios-sundaeswap-baseline.yaml \
  --label sundaeswap-tiered-vs-baseline

# Run a multi-job experiment suite from config
cargo run -q -p sim-cli --bin experiment-suite -- \
  run parameters/suites/overnight-multi.yaml

# Resume an interrupted or failed suite
cargo run -q -p sim-cli --bin experiment-suite -- \
  resume output/experiment-suites/<timestamp>-<label>
```

Output goes to `output/eb-compare/<timestamp>-<label>/`. Key outputs:
- `metrics_comparison.txt` — welfare tables (per-actor, per-urgency-class)
- `time_series.csv` — slot-by-slot tier prices, fill rates, tx counts
- `diagnostics.log` — pricing mechanism state transitions

Suite runs go to `output/experiment-suites/<timestamp>-<label>/`. Key suite outputs:
- `suite.yaml` — copied source suite config
- `manifest.json` — durable suite/job/attempt state used for resume
- `jobs/<job-id>/attempt-###/` — single-run artifacts
- `jobs/<job-id>/seed-<n>/attempt-###/` — seeded sweep artifacts

## Running Tests

```bash
cd sim-rs
cargo test
```

Tests are in `sim-core/src/sim/tests/` and inline `#[cfg(test)]` modules. They use deterministic seeded RNG for reproducibility.

## Development Guidelines

1. **Build after every change**: `cargo build` before moving on. Keep the feedback loop tight. If a build fails and you can't immediately see why, revert to the last working state, make a smaller change, and build again.
2. **Readability over cleverness**: This is a research codebase. Explicit, verbose code beats clever one-liners. Prefer explicit over implicit, verbose over terse, simple over clever.
3. **Determinism matters**: Always use seeded RNG (`StdRng::seed_from_u64`). Pass `&mut rng` explicitly rather than using thread-local randomness. Tests assert specific tx orderings.
4. **The mempool uses `IndexMap`**: O(1) lookups with insertion-order-preserving iteration. Use `shift_remove` (not `swap_remove`) to preserve order.
5. **Transaction has `urgency_component_index: Option<u16>`**: Must be set when constructing transactions (use `None` for legacy/test paths).
6. **Delay units are blocks, not slots**: `tier-delay-unit: blocks` in config. With `rb-generation-probability: 0.05`, 1 block ≈ 20 slots. A tier-0 delay of 1 block means ~20 slots minimum latency.

## Coding Standards

### Naming

- Types: `PascalCase` — `Transaction`, `TieredState`, `MetricsCollector`
- Functions: `snake_case` — `build_block`, `update_tier_prices`, `select_tier_for_transaction`
- Constants: `SCREAMING_SNAKE_CASE` — `DEFAULT_SLOT_DURATION_MS`, `MAX_TIERS`
- Booleans: prefix with `is_`, `has_`, `should_` — `is_mature`, `has_capacity`, `should_include`

### Error Handling

Use `Result` for operations that can fail, `Option` for values that may be absent. Avoid `.unwrap()` except in tests or when the invariant is obvious and documented:
```rust
// OK: invariant is clear
let first_tier = tiers.first().expect("tiers must be non-empty");

// Better: handle the case
let Some(first_tier) = tiers.first() else {
    return Err(SimulationError::NoTiersConfigured);
};
```

Use `saturating_add` / `saturating_sub` for counters and prices to prevent overflow.

### Numeric Pitfalls

Don't compare floats with `==`. Use approximate comparison where needed. Be explicit about whether slot/block ranges are inclusive or exclusive — document the convention in comments.

### Imports

Group in this order, separated by blank lines: (1) standard library, (2) external crates, (3) `crate::` internal, (4) `super`/`self`.

### When Stuck

If you encounter a design decision not covered here: choose the simpler option, document your choice with a comment, and make it easy to change later. For domain questions (what should happen when X meets Y, is this the right interpretation of the paper, which feature matters more) — stop and ask rather than guessing.

## Creating New Experiments

An experiment needs three things:

1. **Actor profile** (`parameters/actors/*.toml`): Defines actor groups with arrival rates, tx sizes, value-urgency components.
2. **Pricing config** (`parameters/pricing/*.toml`): Configures the fee mechanism (baseline, tiered, etc.).
3. **Experiment overlay** (`parameters/experiments/*.yaml`): References the above, sets `enforce-tier-delay`, `seed`, and protocol parameters.

Experiment YAML files layer on top of `config.default.yaml` via `-p` flags. Fields in the experiment override defaults.

## Creating Experiment Suites

A suite YAML groups multiple runs into one resumable campaign. It supports:
- `defaults`: shared topology / parameter / tracing / slot settings
- `jobs`: ordered run definitions with `parameters`, optional `compare-parameters`, and optional `seeds`

Each `jobs[].parameters` list becomes the common `-p` set for that job. If `compare-parameters` is present, sim-cli runs a baseline case plus one case per compare overlay. If `seeds` is present, the suite expands the job into one resumable unit per seed, with the seed applied as a generated trailing override.

## Parameter Layering

The CLI applies parameters in order: `config.default.yaml` → `linear.yaml` → `protocol-base.yaml` → experiment YAML. Later files override earlier ones. The `pricing.config-path` and `actors.config-path` fields point to TOML files loaded separately.

Phase-2 experiments use thin overlays that specify only `pricing.config-path`, `actors.config-path`, and any per-experiment overrides. Shared protocol parameters live in `parameters/phase-2-sweep/protocol-base.yaml`.

## Pending / Experimental Design Choices

Decisions made during development that may need revisiting:

- **Tier rebalancing on over-capacity (2026-04-01)**: When a new tier spawns and existing tiers are resized, tiers may temporarily exceed their new (smaller) capacity. We allow this — the overflow rejection machinery handles admission for new txs naturally, and the tier drains on the next block. Alternative approaches (rebalance only at block boundaries, or never shrink below pending bytes) were considered but deferred as unnecessary complexity. If price oscillation is observed around tier-spawn events, revisit this choice.

- **Tier 0 delay should be zero (2026-04-07)**: The paper specifies tier 0 as immediate inclusion (zero artificial delay). All experiments to date have used `initial_tier_delay = 1` (block or slot), imposing a minimum 1-block (~20 slot) delay even on the fastest tier. This affects absolute latency numbers but not relative mechanism comparisons (all variants share the same tier-0 delay). A follow-up experiment with `initial_tier_delay = 0` for tier 0 specifically is needed to validate the paper's intended behavior.

- **EB effective capacity (2026-04-01)**: The EB pool's `total_capacity` is set to `eb-referenced-txs-max-size-bytes` (12MB per CIP-0164). However, effective EB throughput is much lower because ~50% of EBs fail certification due to RB timing (structural property of linear Leios). The pricing mechanism sees low EB fill rates and keeps EB prices artificially low. A potential improvement: set `eb_total_capacity` to reflect effective certified throughput (~4-6MB) rather than raw EB size. Not yet implemented.
