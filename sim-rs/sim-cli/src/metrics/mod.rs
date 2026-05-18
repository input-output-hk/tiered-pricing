//! Phase-2 welfare-metrics layer. M3.
//!
//! Consumes `sim_core::events::Event`s from a running simulation and
//! produces three artefacts per (job, seed):
//!
//! - `time_series.csv` — per-slot snapshots of pricing state, mempool
//!   bytes, and per-slot deltas of inclusions/evictions/fees/refunds.
//! - `metrics_comparison.txt` — per-actor and per-component breakdown
//!   of welfare metrics (retained_value_ratio, net_utility,
//!   latency_blocks, inclusion_rate, eviction_rate, refund_total,
//!   per-lane retained-value audit).
//! - `diagnostics.log` — resolved config, controller settings,
//!   multiplier-floor breach counts, partition-activation counts,
//!   run-level validation notes.
//!
//! Plain f64 arithmetic. The integer event stream is what's
//! deterministic; metrics are derived from it but never feed back
//! into simulation decisions.

pub mod collector;
pub mod comparison;
pub mod diagnostics;
pub mod paired_bootstrap;
pub mod time_series;

pub use collector::{ComponentSummary, MetricsCollector, RunSummary};
pub use paired_bootstrap::{CiResult, DeltaSummary, paired_bca_ci, paired_delta_summary};
