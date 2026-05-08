//! `metrics_comparison.txt` writer. M3.
//!
//! Per-actor / per-component breakdown of welfare metrics, plus
//! cross-lane retained-value audit. **Negative `net_utility` is
//! preserved through every aggregation step**: regret events
//! contribute their negative value to the per-component and
//! per-suite totals (plan line 152). Never clamp, floor, or filter
//! these.

use std::{io::Write, path::Path};

use anyhow::Result;

use super::collector::RunSummary;

/// Write per-suite metrics_comparison.txt aggregating across multiple
/// (job, seed) `RunSummary`s.
pub fn write_suite(
    path: &Path,
    suite_name: &str,
    runs: &[(String, u64, RunSummary)],
) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::File::create(path)?;
    writeln!(f, "Phase-2 metrics comparison — suite: {suite_name}")?;
    writeln!(f)?;
    for (job, seed, summary) in runs {
        writeln!(f, "## job={job} seed={seed}")?;
        write_run(&mut f, summary)?;
        writeln!(f)?;
    }
    Ok(())
}

/// Write per-run summary to a single file (used for diagnostics in
/// per-job tests).
pub fn write_run_only(path: &Path, summary: &RunSummary) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::File::create(path)?;
    write_run(&mut f, summary)?;
    Ok(())
}

fn write_run(f: &mut std::fs::File, summary: &RunSummary) -> Result<()> {
    writeln!(
        f,
        "- total_txs_submitted: {}",
        summary.total_txs_submitted
    )?;
    writeln!(f, "- total_txs_included: {}", summary.total_txs_included)?;
    writeln!(
        f,
        "- total_txs_evicted_quote_drift: {}",
        summary.total_txs_evicted_quote_drift
    )?;
    writeln!(
        f,
        "- total_fees_paid_lovelace: {}",
        summary.total_fees_paid_lovelace
    )?;
    writeln!(
        f,
        "- total_refund_lovelace: {}",
        summary.total_refund_lovelace
    )?;
    let pri_ratio = retained_value_ratio_aggregate(
        summary.priority_retained_value_total,
        summary.priority_included_value_total,
    );
    let std_ratio = retained_value_ratio_aggregate(
        summary.standard_retained_value_total,
        summary.standard_included_value_total,
    );
    writeln!(f, "- priority_lane_retained_value_ratio: {pri_ratio:.6}")?;
    writeln!(f, "- standard_lane_retained_value_ratio: {std_ratio:.6}")?;
    writeln!(
        f,
        "- multiplier_floor_breaches: {}",
        summary.multiplier_floor_breaches
    )?;
    writeln!(
        f,
        "- priority_over_standard_quote_ratio: min={:.4} max={:.4}",
        summary.min_priority_over_standard_ratio, summary.max_priority_over_standard_ratio
    )?;
    writeln!(f, "- pricing_ticks_observed: {}", summary.pricing_ticks)?;

    writeln!(f, "- per-component:")?;
    for c in &summary.components {
        writeln!(
            f,
            "  - component_index={} txs_submitted={} txs_included={} \
             txs_evicted={} bytes_included={} fees_paid={} refund={} \
             retained_value_ratio={:.6} net_utility_total={:.2} \
             latency_blocks_mean={:.4} inclusion_rate={:.4} eviction_rate={:.4} \
             priority_included={} standard_included={}",
            c.component_index,
            c.txs_submitted,
            c.txs_included,
            c.txs_evicted_quote_drift,
            c.bytes_included,
            c.fees_paid_lovelace,
            c.refund_lovelace,
            c.retained_value_ratio_aggregate(),
            c.net_utility_total(),
            c.latency_blocks_mean(),
            c.inclusion_rate(),
            c.eviction_rate(),
            c.priority_included,
            c.standard_included,
        )?;
    }
    Ok(())
}

fn retained_value_ratio_aggregate(retained_total: f64, included_value_total: u128) -> f64 {
    if included_value_total == 0 {
        0.0
    } else {
        retained_total / (included_value_total as f64)
    }
}
