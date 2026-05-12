//! `diagnostics.log` writer. M3.
//!
//! Per-run diagnostics. Captures resolved config, controller
//! settings, multiplier-floor breach count (must always be 0), the
//! observed priority/standard quote ratio min/max, and a free-form
//! notes section for run-level validation messages.

use std::{io::Write, path::Path};

use anyhow::Result;
use sim_core::config::{PricingConfig, SimConfiguration};

use super::collector::RunSummary;

/// One line of run-level validation note.
#[derive(Debug, Clone)]
pub struct DiagnosticNote {
    pub level: NoteLevel,
    pub message: String,
}

#[derive(Debug, Clone, Copy)]
pub enum NoteLevel {
    Info,
    Warn,
    Error,
}

pub fn write(
    path: &Path,
    sim_config: &SimConfiguration,
    summary: &RunSummary,
    notes: &[DiagnosticNote],
) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::File::create(path)?;
    writeln!(f, "## Resolved config")?;
    writeln!(
        f,
        "- block_generation_probability: {}",
        summary.block_generation_probability
    )?;
    writeln!(
        f,
        "- mempool_max_total_size_bytes: {}",
        sim_config.mempool_gate_config().max_total_size_bytes
    )?;
    writeln!(
        f,
        "- min_fee_a: {}",
        sim_config.mempool_gate_config().min_fee_a
    )?;
    writeln!(
        f,
        "- min_fee_b: {}",
        sim_config.mempool_gate_config().min_fee_b
    )?;
    writeln!(f, "- pricing-config:")?;
    match sim_config.pricing_config() {
        PricingConfig::Baseline => writeln!(f, "  - kind: baseline")?,
        PricingConfig::Eip1559(s) => writeln!(
            f,
            "  - kind: eip1559 initial_quote={} target={}/{} D={} window_length={}",
            s.initial_quote_per_byte,
            s.target_num,
            s.target_den,
            s.max_change_denominator,
            s.window_length
        )?,
        PricingConfig::TwoLane(s) => {
            writeln!(
                f,
                "  - kind: two-lane variant={:?} multiplier_floor={}/{} order={:?}",
                s.variant,
                s.multiplier_floor.numerator,
                s.multiplier_floor.denominator,
                s.lane_selection_order
            )?;
            writeln!(
                f,
                "  - priority: initial_quote={} target={}/{} D={} window_length={}",
                s.priority.initial_quote_per_byte,
                s.priority.target_num,
                s.priority.target_den,
                s.priority.max_change_denominator,
                s.priority.window_length
            )?;
            writeln!(
                f,
                "  - standard: initial_quote={} target={}/{} D={} window_length={}",
                s.standard.initial_quote_per_byte,
                s.standard.target_num,
                s.standard.target_den,
                s.standard.max_change_denominator,
                s.standard.window_length
            )?;
        }
    }
    writeln!(f)?;
    writeln!(f, "## Run summary")?;
    writeln!(f, "- total_txs_submitted: {}", summary.total_txs_submitted)?;
    writeln!(f, "- total_txs_included: {}", summary.total_txs_included)?;
    writeln!(
        f,
        "- total_txs_evicted_quote_drift: {}",
        summary.total_txs_evicted_quote_drift
    )?;
    writeln!(
        f,
        "- multiplier_floor_breaches: {} (must be 0)",
        summary.multiplier_floor_breaches
    )?;
    writeln!(
        f,
        "- priority_over_standard_quote_ratio: min={:.4} max={:.4}",
        summary.min_priority_over_standard_ratio, summary.max_priority_over_standard_ratio
    )?;
    writeln!(f, "- pricing_ticks_observed: {}", summary.pricing_ticks)?;
    writeln!(
        f,
        "- slot_battles_count: {} (slots where >=2 sibling RB bodies were fully validated at the representative)",
        summary.slot_battles_count
    )?;
    writeln!(
        f,
        "- orphaned_pricing_samples: {} (upper bound on representative-node pricing samples applied to RBs the canonical chain would later orphan)",
        summary.orphaned_pricing_samples
    )?;
    writeln!(
        f,
        "- price_shock (window={} slots): max_single_step={:.3}x  max_window={:.2}x  p90_window={:.2}x  eviction_risk_at_4x={:.4}",
        summary.shock_window_slots,
        summary.max_single_step_priority_shock,
        summary.max_priority_shock_over_window,
        summary.p90_priority_shock_over_window,
        summary.eviction_risk_rate_at_4x,
    )?;
    writeln!(f)?;
    writeln!(f, "## Notes")?;
    for n in notes {
        let prefix = match n.level {
            NoteLevel::Info => "[info]",
            NoteLevel::Warn => "[warn]",
            NoteLevel::Error => "[err]",
        };
        writeln!(f, "{prefix} {}", n.message)?;
    }
    Ok(())
}
