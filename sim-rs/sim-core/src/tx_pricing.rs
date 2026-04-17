use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use rand::Rng;
use rand_chacha::{ChaChaRng, rand_core::SeedableRng};
use serde::{Deserialize, Serialize};

use crate::model::{TierId, Transaction, TransactionRejectReason};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlockKind {
    RankingBlock,
    EndorserBlock,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TierBlockSelectionPolicy {
    #[default]
    Shared,
    NaiveRbEbTwoTier,
    RbTier0Reserved,
    ContinuousRbEb,
    ContinuousRbEbFallback,
}

impl TierBlockSelectionPolicy {
    fn allows_tier(self, tier_index: usize, block_kind: BlockKind) -> bool {
        match self {
            TierBlockSelectionPolicy::Shared => true,
            TierBlockSelectionPolicy::NaiveRbEbTwoTier => match block_kind {
                BlockKind::RankingBlock => tier_index == 0,
                BlockKind::EndorserBlock => tier_index == 1,
            },
            TierBlockSelectionPolicy::RbTier0Reserved => match block_kind {
                BlockKind::RankingBlock => tier_index == 0,
                BlockKind::EndorserBlock => tier_index > 0,
            },
            TierBlockSelectionPolicy::ContinuousRbEb
            | TierBlockSelectionPolicy::ContinuousRbEbFallback => true,
        }
    }

    fn is_lane_partitioned(self) -> bool {
        matches!(
            self,
            TierBlockSelectionPolicy::NaiveRbEbTwoTier
                | TierBlockSelectionPolicy::RbTier0Reserved
                | TierBlockSelectionPolicy::ContinuousRbEb
        )
    }

    /// Whether this policy uses per-lane pricing with separate tier states per block kind.
    /// True for both ContinuousRbEb (submitter picks lane) and ContinuousRbEbFallback
    /// (node decides lane, but pricing tracks each lane independently).
    pub fn uses_continuous_lane_pricing(self) -> bool {
        matches!(
            self,
            TierBlockSelectionPolicy::ContinuousRbEb
                | TierBlockSelectionPolicy::ContinuousRbEbFallback
        )
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TierLane {
    #[default]
    Ranking,
    Endorser,
}

fn tier_lane_for_block_kind(block_kind: BlockKind) -> TierLane {
    match block_kind {
        BlockKind::RankingBlock => TierLane::Ranking,
        BlockKind::EndorserBlock => TierLane::Endorser,
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TierAssignmentSemantics {
    /// Never-stale semantics: accepted assignments remain includable across repricing/tier churn.
    #[default]
    NeverStale,
    /// Legacy semantics: inclusion revalidates assigned tier against current price/tier set.
    LegacyRevalidateCurrentTier,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum OverflowRetryBackoffMode {
    #[default]
    Exponential,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum OverflowRetrySource {
    #[default]
    LocalActorSubmissions,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum OverflowRetryCurveMetric {
    #[default]
    RetainedValueRatio,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OverflowRetryBand {
    pub min_retained_ratio: f64,
    pub max_retained_ratio: f64,
    pub max_attempts: u32,
    pub base_delay_slots: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OverflowRetryPolicy {
    pub enabled: bool,
    #[serde(default)]
    pub source: OverflowRetrySource,
    #[serde(default)]
    pub curve_metric: OverflowRetryCurveMetric,
    #[serde(default)]
    pub backoff_mode: OverflowRetryBackoffMode,
    pub max_delay_slots: u64,
    pub bands: Vec<OverflowRetryBand>,
}

impl Default for OverflowRetryPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            source: OverflowRetrySource::LocalActorSubmissions,
            curve_metric: OverflowRetryCurveMetric::RetainedValueRatio,
            backoff_mode: OverflowRetryBackoffMode::Exponential,
            max_delay_slots: 64,
            bands: vec![
                OverflowRetryBand {
                    min_retained_ratio: 0.0,
                    max_retained_ratio: 0.4,
                    max_attempts: 0,
                    base_delay_slots: 0,
                },
                OverflowRetryBand {
                    min_retained_ratio: 0.4,
                    max_retained_ratio: 0.8,
                    max_attempts: 2,
                    base_delay_slots: 2,
                },
                OverflowRetryBand {
                    min_retained_ratio: 0.8,
                    max_retained_ratio: 1.0,
                    max_attempts: 6,
                    base_delay_slots: 2,
                },
            ],
        }
    }
}

impl OverflowRetryPolicy {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_delay_slots == 0 {
            return Err("overflow_retry_policy.max_delay_slots must be >= 1".to_string());
        }
        if self.bands.is_empty() {
            return Err("overflow_retry_policy.bands must not be empty".to_string());
        }

        const COVERAGE_EPS: f64 = 1e-9;
        let mut expected_min = 0.0_f64;
        for (index, band) in self.bands.iter().enumerate() {
            if !band.min_retained_ratio.is_finite() || !band.max_retained_ratio.is_finite() {
                return Err(format!(
                    "overflow_retry_policy.bands[{index}] ratios must be finite"
                ));
            }
            if !(0.0..=1.0).contains(&band.min_retained_ratio)
                || !(0.0..=1.0).contains(&band.max_retained_ratio)
            {
                return Err(format!(
                    "overflow_retry_policy.bands[{index}] ratios must be in [0, 1]"
                ));
            }
            if band.min_retained_ratio >= band.max_retained_ratio {
                return Err(format!(
                    "overflow_retry_policy.bands[{index}] must satisfy min_retained_ratio < max_retained_ratio"
                ));
            }
            if band.max_attempts > 0 && band.base_delay_slots == 0 {
                return Err(format!(
                    "overflow_retry_policy.bands[{index}] base_delay_slots must be >= 1 when max_attempts > 0"
                ));
            }
            if band.min_retained_ratio > expected_min + COVERAGE_EPS {
                return Err(format!(
                    "overflow_retry_policy.bands has a coverage gap before index {index}"
                ));
            }
            if band.min_retained_ratio + COVERAGE_EPS < expected_min {
                return Err(format!(
                    "overflow_retry_policy.bands must be sorted and non-overlapping (issue at index {index})"
                ));
            }
            expected_min = band.max_retained_ratio;
        }

        if (expected_min - 1.0).abs() > COVERAGE_EPS {
            return Err(
                "overflow_retry_policy.bands must fully cover retained ratio range [0, 1]"
                    .to_string(),
            );
        }
        if self
            .bands
            .last()
            .is_some_and(|band| (band.max_retained_ratio - 1.0).abs() > COVERAGE_EPS)
        {
            return Err(
                "overflow_retry_policy.bands must end at max_retained_ratio = 1.0".to_string(),
            );
        }

        Ok(())
    }

    pub fn band_for_retained_ratio(&self, retained_ratio: f64) -> Option<&OverflowRetryBand> {
        if !retained_ratio.is_finite() || self.bands.is_empty() {
            return None;
        }
        let bounded = retained_ratio.clamp(0.0, 1.0);
        let last_index = self.bands.len().saturating_sub(1);
        self.bands.iter().enumerate().find_map(|(index, band)| {
            let in_range = bounded >= band.min_retained_ratio
                && (bounded < band.max_retained_ratio
                    || (index == last_index && bounded <= band.max_retained_ratio));
            in_range.then_some(band)
        })
    }

    pub fn retry_delay_slots(&self, band: &OverflowRetryBand, attempt_index: u32) -> u64 {
        if band.max_attempts == 0 {
            return 0;
        }
        match self.backoff_mode {
            OverflowRetryBackoffMode::Exponential => {
                let shift = attempt_index.min(20);
                let factor = 1u64.checked_shl(shift).unwrap_or(u64::MAX);
                band.base_delay_slots
                    .saturating_mul(factor)
                    .min(self.max_delay_slots)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PricingMechanismConfig {
    #[serde(alias = "baseline")]
    Baseline { fee_per_byte: u64, base_fee: u64 },
    #[serde(alias = "eip1559")]
    Eip1559 {
        initial_base_fee: u64,
        max_change_denominator: u64,
        target_utilisation: f64,
        #[serde(default)]
        smoothing: Eip1559SmoothingConfig,
    },
    #[serde(alias = "eip1559_priority_lane")]
    Eip1559PriorityLane {
        initial_base_fee: u64,
        max_change_denominator: u64,
        target_utilisation: f64,
        priority_fee_multiplier: f64,
        #[serde(default = "default_priority_lane_capacity_fraction")]
        priority_capacity_fraction: f64,
        #[serde(default = "default_priority_lane_priority_delay")]
        priority_delay: u64,
        #[serde(default = "default_priority_lane_normal_delay")]
        normal_delay: u64,
    },
    #[serde(alias = "tiered")]
    TieredPricing { tiered_config: TieredConfig },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Eip1559SmoothingConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_eip1559_smoothing_alpha")]
    pub alpha: f64,
}

impl Default for Eip1559SmoothingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            alpha: default_eip1559_smoothing_alpha(),
        }
    }
}

impl Eip1559SmoothingConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled && (!self.alpha.is_finite() || self.alpha <= 0.0 || self.alpha > 1.0) {
            return Err("eip1559 smoothing alpha must be finite and in (0, 1]".to_string());
        }
        Ok(())
    }
}

fn default_eip1559_smoothing_alpha() -> f64 {
    0.2
}

fn default_priority_lane_capacity_fraction() -> f64 {
    1.0
}

fn default_priority_lane_priority_delay() -> u64 {
    1
}

fn default_priority_lane_normal_delay() -> u64 {
    2
}

pub fn validate_eip1559_priority_lane_config(
    max_change_denominator: u64,
    target_utilisation: f64,
    priority_fee_multiplier: f64,
    priority_capacity_fraction: f64,
    priority_delay: u64,
    normal_delay: u64,
) -> Result<(), String> {
    if max_change_denominator == 0 {
        return Err("max_change_denominator must be >= 1".to_string());
    }
    if !target_utilisation.is_finite() || target_utilisation <= 0.0 {
        return Err("target_utilisation must be finite and > 0".to_string());
    }
    if !priority_fee_multiplier.is_finite() || priority_fee_multiplier < 1.0 {
        return Err("priority_fee_multiplier must be finite and >= 1".to_string());
    }
    if !priority_capacity_fraction.is_finite()
        || priority_capacity_fraction <= 0.0
        || priority_capacity_fraction > 1.0
    {
        return Err("priority_capacity_fraction must be finite and in (0, 1]".to_string());
    }
    if priority_delay >= normal_delay {
        return Err("priority_delay must be lower than normal_delay".to_string());
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TieredConfig {
    /// Block capacity in bytes (B in the paper).
    pub total_capacity: u64,
    /// Maximum number of tiers that can exist simultaneously (k in the paper).
    pub max_tiers: usize,
    /// Pre-ordained capacity of each tier as a fraction of total_capacity (a_i in the paper).
    /// Entry i is the fraction for tier i. Tier 0 is typically 0.0 because it acts as the
    /// capacity reservoir — new tiers take their capacity from tier 0, and removed tiers
    /// return capacity to it.
    pub tier_size_fractions: Vec<f64>,
    /// Denominator for EIP-1559-style price updates per tier (the 1/8 in Mechanism 1).
    pub base_fee_change_denominator: u64,
    /// Target fill rate for each tier; prices increase above this, decrease below (targetLoad).
    pub target_utilisation: f64,
    /// How often (in blocks) tier delays are updated (dFreq in the paper).
    /// Legacy fallback used only when `delay_update_period_slots` is not set.
    #[serde(default)]
    pub delay_update_frequency: Option<u64>,
    /// Optional slot-based cadence for delay updates.
    /// When set, this overrides `delay_update_frequency`.
    #[serde(default)]
    pub delay_update_period_slots: Option<u64>,
    /// Default μ_j threshold: if p_{j+1} > threshold * p_j, increase d_{j+1}.
    pub delay_increase_threshold: f64,
    /// Per-boundary overrides for delay_increase_threshold (μ_j per boundary).
    #[serde(default)]
    pub delay_increase_thresholds: Vec<f64>,
    /// Probability of decreasing a tier's delay when constraints allow (pDecrease).
    pub delay_decrease_prob: f64,
    /// Default λ_j: minimum ratio between consecutive tier delays (d_{j+1} >= λ_j * d_j).
    pub min_delay_ratio: f64,
    /// Per-boundary overrides for min_delay_ratio (λ_j per boundary).
    #[serde(default)]
    pub min_delay_ratios: Vec<f64>,
    /// How often (in blocks) tier count is re-evaluated for add/remove (tFreq in the paper).
    /// Legacy fallback used only when `tier_update_period_slots` is not set.
    #[serde(default)]
    pub tier_update_frequency: Option<u64>,
    /// Optional slot-based cadence for tier add/remove checks.
    /// When set, this overrides `tier_update_frequency`.
    #[serde(default)]
    pub tier_update_period_slots: Option<u64>,
    /// Last tier's price above which a new tier is spawned (addTierPrice in the paper).
    pub add_tier_threshold: u64,
    /// Last tier's price below which it is removed (removeTierPrice in the paper).
    pub remove_tier_threshold: u64,
    /// Initial price assigned to a newly created tier (newTierPrice).
    pub new_tier_price: u64,
    /// Delay ratio applied when creating a new tier: new_delay = ratio * previous_tier_delay.
    pub new_tier_delay_ratio: f64,
    /// Initial delay for the first tier (tier 0 / fastest tier).
    /// Default: 1. For slot-based delays, set to ~20 to match block-based behavior.
    #[serde(default = "default_initial_tier_delay")]
    pub initial_tier_delay: u64,
    /// How tier delays are spaced when new tiers are added.
    /// "incremental" (default): each new tier = new_tier_delay_ratio * previous tier delay.
    /// "geometric_fixed_max": tiers geometrically spaced between 0 and max_tier_delay.
    #[serde(default)]
    pub tier_delay_spacing: TierDelaySpacing,
    /// Maximum delay for the slowest tier. Only used with geometric_fixed_max spacing.
    #[serde(default = "default_max_tier_delay")]
    pub max_tier_delay: u64,
    /// How tiers map to block types (RB/EB). Shared = paper-like single pool.
    #[serde(default)]
    pub block_selection_policy: TierBlockSelectionPolicy,
    /// Fraction of RB capacity reserved for tier 0 (only used with RbTier0Reserved policy).
    #[serde(default = "default_rb_tier0_reservation_fraction")]
    pub rb_tier0_reservation_fraction: f64,
    /// Optional lower target utilisation for the RB lane only.
    #[serde(default)]
    pub rb_target_utilisation: Option<f64>,
    /// Optional repricing denominator override for the RB lane only.
    #[serde(default)]
    pub rb_base_fee_change_denominator: Option<u64>,
    /// Optional stronger overflow repricing slope for the RB lane only.
    #[serde(default)]
    pub rb_overflow_linear_price_per_fill: Option<u64>,
    /// Soft reservation for RB tier 0 under continuous RB/EB pricing.
    /// Unused reserved bytes are allowed to spill back to slower RB tiers.
    #[serde(default = "default_rb_soft_reservation_fraction")]
    pub rb_soft_reservation_fraction: f64,
    /// Enable a separate EB tier pool.
    /// EB pool capacity is derived from top-level `eb-referenced-txs-max-size-bytes`.
    #[serde(default)]
    pub separate_eb_pool: bool,
    /// Legacy compatibility alias for enabling a separate EB tier pool.
    /// Any value provided here is ignored at runtime and replaced by top-level
    /// `eb-referenced-txs-max-size-bytes`.
    /// Kept for backward compatibility with older pricing files.
    #[serde(default)]
    #[serde(alias = "eb-total-capacity")]
    /// When set by older configs, indicates separate EB pool should be enabled.
    /// Runtime-derived value is used instead.
    ///
    /// Note: this field is intentionally retained for migration compatibility.
    /// Prefer `separate_eb_pool = true` in pricing files.
    ///
    /// `total_capacity` is used for RB tier pools and derived EB capacity for EB tier pools.
    /// Each pool evolves independently (prices, delays, tier count).
    pub eb_total_capacity: Option<u64>,
    /// Assignment validity semantics used at inclusion time.
    /// `never_stale` (default): preserve assignment validity across repricing/tier churn.
    /// `legacy_revalidate_current_tier`: require assigned tier to still exist and still be affordable.
    #[serde(default)]
    pub assignment_semantics: TierAssignmentSemantics,
    /// Reject new submissions when pending bytes in a tier already exceed that tier's capacity.
    /// This is a local admission control based purely on node mempool backlog.
    #[serde(default = "default_reject_on_pending_tier_overflow")]
    pub reject_on_pending_tier_overflow: bool,
    /// When true, bytes from locally-observed overflow rejections are aggregated by lane+tier and
    /// fed into tier fill-rate calculations on the next block for that lane.
    #[serde(default)]
    pub include_overflow_aggregate_in_pricing_updates: bool,
    /// How aggregated overflow demand affects repricing when
    /// `include_overflow_aggregate_in_pricing_updates` is enabled.
    #[serde(default)]
    pub overflow_aggregate_pricing_mode: OverflowAggregatePricingMode,
    /// Linear additive repricing slope (price-per-byte units per 1.0 overflow fill-rate)
    /// used only when `overflow_aggregate_pricing_mode = linear_additive`.
    #[serde(default = "default_overflow_linear_price_per_fill")]
    pub overflow_linear_price_per_fill: u64,
    /// Cap applied to overflow fill-rate before linear additive repricing
    /// when `overflow_aggregate_pricing_mode = linear_additive`.
    #[serde(default = "default_overflow_linear_fill_rate_cap")]
    pub overflow_linear_fill_rate_cap: f64,
    /// When true, periodically rebalance active tier capacities based on observed demand signal.
    /// This is applied during tier-update cadence and is lane-local.
    #[serde(default)]
    pub dynamic_tier_sizing_enabled: bool,
    /// Smoothing factor for dynamic sizing in [0,1].
    /// 0 keeps existing sizes, 1 applies full demand-weighted target each update.
    #[serde(default = "default_dynamic_tier_sizing_alpha")]
    pub dynamic_tier_sizing_alpha: f64,
    /// Minimum fraction of lane capacity guaranteed to each active tier during dynamic sizing.
    #[serde(default = "default_dynamic_tier_sizing_min_fraction")]
    pub dynamic_tier_sizing_min_fraction: f64,
    /// When true, enforce per-boundary price caps after repricing:
    /// p_{i+1} <= mu_i * p_i (using delay_increase_threshold(s) as mu_i).
    #[serde(default)]
    pub enforce_boundary_price_caps: bool,
    /// When true, tier add/remove decisions are driven by tier fill-rate pressure instead of
    /// last-tier price thresholds. Intended for using overflow demand as a structural signal.
    #[serde(default)]
    pub include_overflow_aggregate_in_tier_updates: bool,
    /// Last-tier fill-rate above which a new tier is spawned when
    /// `include_overflow_aggregate_in_tier_updates` is enabled.
    #[serde(default = "default_add_tier_fill_rate_threshold")]
    pub add_tier_fill_rate_threshold: f64,
    /// Last-tier fill-rate below which the last tier is removed when
    /// `include_overflow_aggregate_in_tier_updates` is enabled.
    #[serde(default = "default_remove_tier_fill_rate_threshold")]
    pub remove_tier_fill_rate_threshold: f64,
    /// Policy controlling resubmission behavior after tier-backlog overflow rejections.
    #[serde(default = "default_overflow_retry_policy")]
    pub overflow_retry_policy: OverflowRetryPolicy,
    /// Extra delay scale applied during RB-vs-EB lane selection based on observed lane backlog.
    /// 0 disables the penalty.
    #[serde(default = "default_lane_selection_backlog_delay_scale")]
    pub lane_selection_backlog_delay_scale: f64,
    /// Minimum fee per transaction, independent of size.
    /// Mirrors Cardano's minimum fee floor (~150k lovelace).
    /// Tier fee = max(price_per_byte * bytes + base_fee, min_fee).
    #[serde(default)]
    pub min_fee: u64,
    /// Blending factor for gradual tier capacity rebalancing in [0,1].
    /// When a tier is added or removed, each tier moves this fraction of the way
    /// toward its target capacity. 0 = no rebalancing, 1 = snap to target instantly.
    /// Default: 0.3 (converges over ~5-7 rebalancing events).
    #[serde(default = "default_tier_rebalance_alpha")]
    pub tier_rebalance_alpha: f64,
    /// Enable exponential moving average throughput for fill rate calculation.
    /// Only meaningful for shared single-pool (separate_eb_pool=false).
    /// Smooths the RB/EB fill rate oscillation by tracking throughput over time.
    #[serde(default)]
    pub throughput_ema_enabled: bool,
    /// Smoothing factor for throughput EMA in [0,1].
    /// Smaller = smoother (slower response). Default: 0.1.
    #[serde(default = "default_throughput_ema_alpha")]
    pub throughput_ema_alpha: f64,
    /// When true, block filling uses strict tier priority instead of per-tier capacity allocation.
    /// Tier 0 fills first (up to full block capacity), then tier 1 gets the remainder, etc.
    /// Delays are advisory only (used for self-selection but not enforced at inclusion time).
    /// Fill rates are computed against total block capacity, not per-tier capacity.
    #[serde(default)]
    pub priority_ordering: bool,
}

impl TieredConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_tiers == 0 {
            return Err("max_tiers must be at least 1".to_string());
        }
        let boundary_count = self.max_tiers.saturating_sub(1);
        if self.tier_size_fractions.len() < self.max_tiers {
            return Err("tier_size_fractions must have at least max_tiers entries".to_string());
        }
        let mut total = 0.0;
        for fraction in self.tier_size_fractions.iter().take(self.max_tiers) {
            if *fraction < 0.0 {
                return Err("tier_size_fractions entries must be non-negative".to_string());
            }
            total += fraction;
        }
        if total > 1.0 + f64::EPSILON {
            return Err("tier_size_fractions sum must be <= 1.0".to_string());
        }
        match (self.delay_update_frequency, self.delay_update_period_slots) {
            (Some(_), Some(_)) => {
                return Err(
                    "delay cadence duplicated: set only one of delay_update_frequency or delay_update_period_slots"
                        .to_string(),
                );
            }
            (None, None) => {
                return Err(
                    "delay cadence missing: set one of delay_update_frequency or delay_update_period_slots"
                        .to_string(),
                );
            }
            (Some(0), None) => {
                return Err("delay_update_frequency must be >= 1 when set".to_string());
            }
            (_, Some(0)) => {
                return Err("delay_update_period_slots must be >= 1 when set".to_string());
            }
            _ => {}
        }
        match (self.tier_update_frequency, self.tier_update_period_slots) {
            (Some(_), Some(_)) => {
                return Err(
                    "tier cadence duplicated: set only one of tier_update_frequency or tier_update_period_slots"
                        .to_string(),
                );
            }
            (None, None) => {
                return Err(
                    "tier cadence missing: set one of tier_update_frequency or tier_update_period_slots"
                        .to_string(),
                );
            }
            (Some(0), None) => {
                return Err("tier_update_frequency must be >= 1 when set".to_string());
            }
            (_, Some(0)) => {
                return Err("tier_update_period_slots must be >= 1 when set".to_string());
            }
            _ => {}
        }
        if self.min_delay_ratio < 1.0 {
            return Err("min_delay_ratio must be >= 1.0".to_string());
        }
        if !self.min_delay_ratios.is_empty() && self.min_delay_ratios.len() < boundary_count {
            return Err(
                "min_delay_ratios must have at least max_tiers - 1 entries when provided"
                    .to_string(),
            );
        }
        for ratio in self.min_delay_ratios.iter().take(boundary_count) {
            if *ratio < 1.0 {
                return Err("min_delay_ratios entries must be >= 1.0".to_string());
            }
        }
        if !self.delay_increase_thresholds.is_empty()
            && self.delay_increase_thresholds.len() < boundary_count
        {
            return Err(
                "delay_increase_thresholds must have at least max_tiers - 1 entries when provided"
                    .to_string(),
            );
        }
        for threshold in self.delay_increase_thresholds.iter().take(boundary_count) {
            if !threshold.is_finite() || *threshold <= 0.0 {
                return Err("delay_increase_thresholds entries must be finite and > 0".to_string());
            }
        }
        if self.delay_decrease_prob < 0.0 || self.delay_decrease_prob > 1.0 {
            return Err("delay_decrease_prob must be between 0 and 1".to_string());
        }
        if self.block_selection_policy == TierBlockSelectionPolicy::NaiveRbEbTwoTier {
            if self.max_tiers < 2 {
                return Err("naive_rb_eb_two_tier requires max_tiers >= 2".to_string());
            }
        }
        if self.block_selection_policy == TierBlockSelectionPolicy::RbTier0Reserved {
            if self.max_tiers < 2 {
                return Err("rb_tier0_reserved requires max_tiers >= 2".to_string());
            }
            if !self.rb_tier0_reservation_fraction.is_finite()
                || self.rb_tier0_reservation_fraction <= 0.0
                || self.rb_tier0_reservation_fraction > 1.0
            {
                return Err(
                    "rb_tier0_reservation_fraction must be finite and in (0, 1]".to_string()
                );
            }
        }
        if let Some(target) = self.rb_target_utilisation {
            if !target.is_finite() || !(0.0..=1.0).contains(&target) || target == 0.0 {
                return Err("rb_target_utilisation must be finite and in (0, 1]".to_string());
            }
        }
        if let Some(denominator) = self.rb_base_fee_change_denominator {
            if denominator == 0 {
                return Err("rb_base_fee_change_denominator must be >= 1".to_string());
            }
        }
        if !self.rb_soft_reservation_fraction.is_finite()
            || !(0.0..=1.0).contains(&self.rb_soft_reservation_fraction)
        {
            return Err("rb_soft_reservation_fraction must be finite and in [0, 1]".to_string());
        }
        if self.block_selection_policy.uses_continuous_lane_pricing()
            && self.eb_total_capacity.is_none()
            && !self.separate_eb_pool
        {
            return Err(
                "continuous_rb_eb requires separate_eb_pool=true (or legacy eb_total_capacity)"
                    .to_string(),
            );
        }
        if !self.add_tier_fill_rate_threshold.is_finite()
            || self.add_tier_fill_rate_threshold <= 0.0
        {
            return Err("add_tier_fill_rate_threshold must be finite and > 0".to_string());
        }
        if !self.remove_tier_fill_rate_threshold.is_finite()
            || self.remove_tier_fill_rate_threshold < 0.0
        {
            return Err("remove_tier_fill_rate_threshold must be finite and >= 0".to_string());
        }
        if self.remove_tier_fill_rate_threshold >= self.add_tier_fill_rate_threshold {
            return Err(
                "remove_tier_fill_rate_threshold must be < add_tier_fill_rate_threshold"
                    .to_string(),
            );
        }
        if !self.overflow_linear_fill_rate_cap.is_finite()
            || self.overflow_linear_fill_rate_cap < 0.0
        {
            return Err("overflow_linear_fill_rate_cap must be finite and >= 0".to_string());
        }
        if !self.dynamic_tier_sizing_alpha.is_finite()
            || !(0.0..=1.0).contains(&self.dynamic_tier_sizing_alpha)
        {
            return Err("dynamic_tier_sizing_alpha must be finite and in [0, 1]".to_string());
        }
        if !self.dynamic_tier_sizing_min_fraction.is_finite()
            || !(0.0..=1.0).contains(&self.dynamic_tier_sizing_min_fraction)
        {
            return Err(
                "dynamic_tier_sizing_min_fraction must be finite and in [0, 1]".to_string(),
            );
        }
        if !self.lane_selection_backlog_delay_scale.is_finite()
            || self.lane_selection_backlog_delay_scale < 0.0
        {
            return Err("lane_selection_backlog_delay_scale must be finite and >= 0".to_string());
        }
        self.overflow_retry_policy.validate()?;
        Ok(())
    }

    pub fn tier_capacity(&self, tier_index: usize) -> u64 {
        if tier_index >= self.tier_size_fractions.len() {
            return 0;
        }
        let fraction = self.tier_size_fractions[tier_index].max(0.0);
        (self.total_capacity as f64 * fraction).round() as u64
    }

    fn boundary_delay_increase_threshold(&self, boundary_index: usize) -> f64 {
        self.delay_increase_thresholds
            .get(boundary_index)
            .copied()
            .unwrap_or(self.delay_increase_threshold)
    }

    fn boundary_min_delay_ratio(&self, boundary_index: usize) -> f64 {
        self.min_delay_ratios
            .get(boundary_index)
            .copied()
            .unwrap_or(self.min_delay_ratio)
    }

    fn boundary_new_tier_delay_ratio(&self, boundary_index: usize) -> f64 {
        self.min_delay_ratios
            .get(boundary_index)
            .copied()
            .unwrap_or(self.new_tier_delay_ratio)
    }

    /// Compute the delay for a tier at `tier_index` (0-based, where 0 is fastest)
    /// under geometric_fixed_max spacing. Tier 0 = initial_tier_delay,
    /// remaining tiers geometrically spaced up to max_tier_delay.
    fn geometric_delay_for_tier(&self, tier_index: usize, total_tiers: usize) -> u64 {
        if tier_index == 0 {
            return self.initial_tier_delay;
        }
        if total_tiers <= 1 {
            return self.initial_tier_delay;
        }
        // Tiers 1..total_tiers-1 are geometrically spaced from min to max_tier_delay.
        // min is 1 (smallest nonzero delay). max is max_tier_delay.
        let n = (total_tiers - 1) as f64; // number of nonzero-delay tiers
        let i = tier_index as f64; // 1-based position in the nonzero range
        let max_d = self.max_tier_delay as f64;
        // Geometric: delay[i] = max_d ^ (i / (n))
        // This gives tier 1 = max_d^(1/n), tier n = max_d^1 = max_d
        let delay = max_d.powf(i / n);
        (delay.round() as u64).max(1)
    }

    fn effective_tier_update_cadence_slots(&self) -> u64 {
        self.tier_update_period_slots
            .or(self.tier_update_frequency)
            .unwrap_or(1)
    }

    fn rb_reserved_capacity(&self, block_capacity: u64) -> u64 {
        let reserved = (block_capacity as f64 * self.rb_tier0_reservation_fraction).round() as u64;
        reserved.min(block_capacity)
    }

    fn rb_soft_reserved_capacity(&self, block_capacity: u64) -> u64 {
        let reserved = (block_capacity as f64 * self.rb_soft_reservation_fraction).round() as u64;
        reserved.min(block_capacity)
    }

    fn target_utilisation_for_lane(&self, lane: TierLane) -> f64 {
        match lane {
            TierLane::Ranking => self
                .rb_target_utilisation
                .unwrap_or(self.target_utilisation),
            TierLane::Endorser => self.target_utilisation,
        }
    }

    fn base_fee_change_denominator_for_lane(&self, lane: TierLane) -> u64 {
        match lane {
            TierLane::Ranking => self
                .rb_base_fee_change_denominator
                .unwrap_or(self.base_fee_change_denominator),
            TierLane::Endorser => self.base_fee_change_denominator,
        }
    }

    fn overflow_linear_price_per_fill_for_lane(&self, lane: TierLane) -> u64 {
        match lane {
            TierLane::Ranking => self
                .rb_overflow_linear_price_per_fill
                .unwrap_or(self.overflow_linear_price_per_fill),
            TierLane::Endorser => self.overflow_linear_price_per_fill,
        }
    }

    /// Whether this config uses separate tier pools per block kind.
    pub fn has_separate_eb_pool(&self) -> bool {
        self.eb_total_capacity.is_some()
    }

    /// Return a copy of this config with total_capacity overridden (used for EB pool init).
    fn with_total_capacity(&self, capacity: u64) -> Self {
        let mut clone = self.clone();
        clone.total_capacity = capacity;
        clone
    }
}

fn default_rb_tier0_reservation_fraction() -> f64 {
    1.0
}

fn default_rb_soft_reservation_fraction() -> f64 {
    0.0
}

fn default_reject_on_pending_tier_overflow() -> bool {
    true
}

fn default_overflow_retry_policy() -> OverflowRetryPolicy {
    OverflowRetryPolicy::default()
}

fn default_add_tier_fill_rate_threshold() -> f64 {
    1.0
}

fn default_tier_rebalance_alpha() -> f64 {
    0.3
}

fn default_initial_tier_delay() -> u64 {
    1
}

fn default_max_tier_delay() -> u64 {
    200
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TierDelaySpacing {
    #[default]
    Incremental,
    GeometricFixedMax,
}

fn default_throughput_ema_alpha() -> f64 {
    0.1
}

fn default_remove_tier_fill_rate_threshold() -> f64 {
    0.2
}

fn default_overflow_linear_price_per_fill() -> u64 {
    100
}

fn default_overflow_linear_fill_rate_cap() -> f64 {
    1.0
}

fn default_dynamic_tier_sizing_alpha() -> f64 {
    1.0
}

fn default_dynamic_tier_sizing_min_fraction() -> f64 {
    0.02
}

fn default_lane_selection_backlog_delay_scale() -> f64 {
    0.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingFile {
    pub pricing_mechanism: PricingMechanismConfig,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum OverflowAggregatePricingMode {
    /// Existing behavior: aggregate overflow bytes into fill-rate before EIP-1559 repricing.
    #[default]
    IncludeAsFillRate,
    /// Alternative behavior: base repricing uses included fill-rate only; overflow contributes via
    /// bounded linear additive increment.
    LinearAdditive,
}

impl PricingFile {
    pub fn from_path(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read pricing config {}", path.display()))?;
        let file: PricingFile = toml::from_str(&contents)
            .with_context(|| format!("failed to parse pricing config {}", path.display()))?;
        Ok(file)
    }
}

#[derive(Debug, Clone)]
pub struct TierQuote {
    pub id: TierId,
    pub lane: TierLane,
    /// Slot when this tier version became active for new submissions.
    pub version_created_slot: u64,
    pub delay: u64,
    pub price_per_byte: u64,
    pub base_fee: u64,
}

impl TierQuote {
    pub fn required_fee(&self, size_bytes: u64) -> u64 {
        self.base_fee
            .saturating_add(self.price_per_byte.saturating_mul(size_bytes))
    }
}

#[derive(Debug, Clone)]
pub struct PricingSnapshot {
    pub tiers: Vec<TierQuote>,
}

impl PricingSnapshot {
    pub fn cheapest_tier(&self) -> Option<&TierQuote> {
        self.tiers.iter().min_by_key(|tier| tier.price_per_byte)
    }

    pub fn fastest_tier(&self) -> Option<&TierQuote> {
        self.tiers.iter().min_by_key(|tier| tier.delay)
    }

    pub fn cheapest_tier_with_delay_at_most(&self, max_delay: u64) -> Option<&TierQuote> {
        self.tiers
            .iter()
            .filter(|tier| tier.delay <= max_delay)
            .min_by_key(|tier| tier.price_per_byte)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum TierSelectionDelayModel {
    TierDelay,
    NaiveRbEbTwoTierPath {
        rb_path_latency: u64,
        eb_path_latency: u64,
    },
    LanePathPlusTierDelay {
        rb_path_latency: u64,
        eb_path_latency: u64,
    },
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub struct LaneDelayAdjustments {
    pub ranking_extra_delay: u64,
    pub endorser_extra_delay: u64,
}

impl TierSelectionDelayModel {
    fn utility_delay_units(self, tier: &TierQuote) -> u64 {
        self.utility_delay_units_for_lane(
            match tier.lane {
                TierLane::Ranking => BlockKind::RankingBlock,
                TierLane::Endorser => BlockKind::EndorserBlock,
            },
            tier.delay,
            Some(tier.id),
        )
    }

    pub(crate) fn utility_delay_units_for_lane_with_adjustments(
        self,
        lane: BlockKind,
        tier_delay_slots: u64,
        tier_id: Option<TierId>,
        delay_adjustments: LaneDelayAdjustments,
    ) -> u64 {
        self.utility_delay_units_for_lane(lane, tier_delay_slots, tier_id)
            .saturating_add(match lane {
                BlockKind::RankingBlock => delay_adjustments.ranking_extra_delay,
                BlockKind::EndorserBlock => delay_adjustments.endorser_extra_delay,
            })
    }

    pub(crate) fn utility_delay_units_for_lane(
        self,
        lane: BlockKind,
        tier_delay_slots: u64,
        tier_id: Option<TierId>,
    ) -> u64 {
        match self {
            TierSelectionDelayModel::TierDelay => tier_delay_slots,
            TierSelectionDelayModel::NaiveRbEbTwoTierPath {
                rb_path_latency,
                eb_path_latency,
            } => {
                // In naive RB/EB mode, tier 0 maps to the RB lane and tier 1 to the EB lane.
                // If additional tiers appear for any reason, treat non-zero tiers as EB-like.
                if tier_id == Some(TierId::new(0)) {
                    rb_path_latency.max(1)
                } else {
                    eb_path_latency.max(1)
                }
            }
            TierSelectionDelayModel::LanePathPlusTierDelay {
                rb_path_latency,
                eb_path_latency,
            } => {
                let lane_path = match lane {
                    BlockKind::RankingBlock => rb_path_latency.max(1),
                    BlockKind::EndorserBlock => eb_path_latency.max(1),
                };
                lane_path.saturating_add(tier_delay_slots.saturating_sub(1))
            }
        }
    }
}

pub fn select_tier_for_tx(
    tx: &Transaction,
    pricing: &PricingSnapshot,
    delay_model: TierSelectionDelayModel,
) -> Option<(TierId, u64, u64, u64, u64)> {
    let mut best: Option<(i128, u64, u64, TierId, u64, u64, u64, u64)> = None;
    for tier in &pricing.tiers {
        let required = tier.required_fee(tx.bytes);
        let utility_delay_units = delay_model.utility_delay_units(tier);
        let value_at_delay = tx.urgency.value_at_delay(tx.value, utility_delay_units);
        let utility = value_at_delay as i128 - required as i128;
        if utility < 0 {
            continue;
        }

        // Choose max utility, then lower delay, then lower fee, then stable tier id.
        let settlement_delay_slots = tier.delay;
        let candidate = (
            utility,
            utility_delay_units,
            required,
            tier.id,
            tier.version_created_slot,
            required,
            settlement_delay_slots,
            tier.price_per_byte,
        );
        let should_replace = match best {
            None => true,
            Some((best_utility, best_delay, best_required, best_tier_id, _, _, _, _)) => {
                utility > best_utility
                    || (utility == best_utility
                        && (utility_delay_units < best_delay
                            || (utility_delay_units == best_delay
                                && (required < best_required
                                    || (required == best_required && tier.id < best_tier_id)))))
            }
        };
        if should_replace {
            best = Some(candidate);
        }
    }
    best.map(
        |(
            _,
            _,
            _,
            tier_id,
            version_created_slot,
            required,
            settlement_delay_slots,
            price_per_byte,
        )| {
            (
                tier_id,
                version_created_slot,
                required,
                settlement_delay_slots,
                price_per_byte,
            )
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LaneTierSelection {
    pub block_kind: BlockKind,
    pub tier_id: TierId,
    pub tier_version_created_slot: u64,
    pub posted_fee: u64,
    pub tier_delay_slots: u64,
    pub tier_price_per_byte_at_assignment: u64,
}

pub fn select_best_lane_tier_for_tx(
    tx: &Transaction,
    rb_pricing: &PricingSnapshot,
    eb_pricing: &PricingSnapshot,
    delay_model: TierSelectionDelayModel,
) -> Option<LaneTierSelection> {
    select_best_lane_tier_for_tx_with_adjustments(
        tx,
        rb_pricing,
        eb_pricing,
        delay_model,
        LaneDelayAdjustments::default(),
    )
}

pub fn select_best_lane_tier_for_tx_with_adjustments(
    tx: &Transaction,
    rb_pricing: &PricingSnapshot,
    eb_pricing: &PricingSnapshot,
    delay_model: TierSelectionDelayModel,
    delay_adjustments: LaneDelayAdjustments,
) -> Option<LaneTierSelection> {
    let rb = select_tier_for_tx(tx, rb_pricing, delay_model);
    let eb = select_tier_for_tx(tx, eb_pricing, delay_model);

    fn candidate_utility(
        tx: &Transaction,
        lane: BlockKind,
        tier_id: Option<TierId>,
        required_fee: u64,
        tier_delay_slots: u64,
        delay_model: TierSelectionDelayModel,
        delay_adjustments: LaneDelayAdjustments,
    ) -> (i128, u64) {
        let utility_delay = delay_model.utility_delay_units_for_lane_with_adjustments(
            lane,
            tier_delay_slots,
            tier_id,
            delay_adjustments,
        );
        let value_at_delay = tx.urgency.value_at_delay(tx.value, utility_delay);
        (value_at_delay as i128 - required_fee as i128, utility_delay)
    }

    let mut best: Option<(i128, u64, u64, BlockKind, LaneTierSelection)> = None;

    if let Some((
        tier_id,
        tier_version_created_slot,
        posted_fee,
        tier_delay_slots,
        tier_price_per_byte_at_assignment,
    )) = rb
    {
        let (utility, utility_delay) = candidate_utility(
            tx,
            BlockKind::RankingBlock,
            Some(tier_id),
            posted_fee,
            tier_delay_slots,
            delay_model,
            delay_adjustments,
        );
        if utility >= 0 {
            let selection = LaneTierSelection {
                block_kind: BlockKind::RankingBlock,
                tier_id,
                tier_version_created_slot,
                posted_fee,
                tier_delay_slots,
                tier_price_per_byte_at_assignment,
            };
            best = Some((
                utility,
                utility_delay,
                posted_fee,
                BlockKind::RankingBlock,
                selection,
            ));
        }
    }

    if let Some((
        tier_id,
        tier_version_created_slot,
        posted_fee,
        tier_delay_slots,
        tier_price_per_byte_at_assignment,
    )) = eb
    {
        let (utility, utility_delay) = candidate_utility(
            tx,
            BlockKind::EndorserBlock,
            Some(tier_id),
            posted_fee,
            tier_delay_slots,
            delay_model,
            delay_adjustments,
        );
        if utility >= 0 {
            let selection = LaneTierSelection {
                block_kind: BlockKind::EndorserBlock,
                tier_id,
                tier_version_created_slot,
                posted_fee,
                tier_delay_slots,
                tier_price_per_byte_at_assignment,
            };
            let replace = match best {
                None => true,
                Some((best_utility, best_delay, best_fee, best_lane, _)) => {
                    utility > best_utility
                        || (utility == best_utility
                            && (utility_delay < best_delay
                                || (utility_delay == best_delay
                                    && (posted_fee < best_fee
                                        || (posted_fee == best_fee
                                            && best_lane == BlockKind::EndorserBlock)))))
                }
            };
            if replace {
                best = Some((
                    utility,
                    utility_delay,
                    posted_fee,
                    BlockKind::EndorserBlock,
                    selection,
                ));
            }
        }
    }

    best.map(|(_, _, _, _, selection)| selection)
}

#[derive(Debug, Clone)]
struct FeeEpoch {
    start_slot: u64,
    price_per_byte: u64,
    delay_slots: u64,
    valid_through_slot: Option<u64>,
}

#[derive(Debug, Clone, Default)]
struct SingleTierFeeHistory {
    by_tier: BTreeMap<TierId, Vec<FeeEpoch>>,
}

impl SingleTierFeeHistory {
    fn from_tiers(tiers: &[Tier], slot: u64) -> Self {
        let mut history = Self::default();
        for tier in tiers {
            history.replace_tier_with_epoch(tier.id, tier.price, tier.delay, slot);
        }
        history
    }

    fn replace_tier_with_epoch(
        &mut self,
        tier_id: TierId,
        price_per_byte: u64,
        delay_slots: u64,
        slot: u64,
    ) {
        self.by_tier.insert(
            tier_id,
            vec![FeeEpoch {
                start_slot: slot,
                price_per_byte,
                delay_slots: delay_slots,
                valid_through_slot: None,
            }],
        );
    }

    fn record_tier_state(
        &mut self,
        tier_id: TierId,
        price_per_byte: u64,
        delay_slots: u64,
        slot: u64,
    ) {
        let delay_slots = delay_slots;
        let epochs = self.by_tier.entry(tier_id).or_insert_with(|| {
            vec![FeeEpoch {
                start_slot: slot,
                price_per_byte,
                delay_slots,
                valid_through_slot: None,
            }]
        });
        let Some(last) = epochs.last_mut() else {
            return;
        };

        if last.price_per_byte == price_per_byte {
            // Keep the latest delay associated with this price regime.
            last.delay_slots = delay_slots;
            return;
        }

        let grace_end_slot = slot.saturating_add(last.delay_slots.saturating_sub(1));
        last.valid_through_slot = Some(grace_end_slot);
        epochs.push(FeeEpoch {
            start_slot: slot,
            price_per_byte,
            delay_slots,
            valid_through_slot: None,
        });
    }

    fn fee_satisfies(
        &self,
        tier_id: TierId,
        current_price_per_byte: u64,
        submission_slot: u64,
        inclusion_slot: u64,
        bytes: u64,
        posted_fee: u64,
        quote_grace_slots: u64,
    ) -> bool {
        let required_now = current_price_per_byte.saturating_mul(bytes);
        let Some(epochs) = self.by_tier.get(&tier_id) else {
            return posted_fee >= required_now;
        };
        let Some(current_epoch) = epochs.last() else {
            return posted_fee >= required_now;
        };
        if current_epoch.price_per_byte != current_price_per_byte {
            // Defensive fallback for stale/inconsistent history (e.g. tests mutating tiers directly).
            return posted_fee >= required_now;
        }

        let Some(epoch) = epochs
            .iter()
            .rev()
            .find(|epoch| epoch.start_slot <= submission_slot)
        else {
            return posted_fee >= required_now;
        };
        let required_epoch_fee = epoch.price_per_byte.saturating_mul(bytes);
        if posted_fee < required_epoch_fee {
            return false;
        }
        match epoch.valid_through_slot {
            Some(valid_through) => {
                inclusion_slot <= valid_through.saturating_add(quote_grace_slots)
            }
            None => true,
        }
    }
}

const ASSIGNMENT_HISTORY_WINDOW_MULTIPLIER: u64 = 16;

#[derive(Debug, Clone, Copy)]
struct QuotedAssignment {
    tier_id: TierId,
    version_created_slot: u64,
    posted_fee: u64,
    delay_slots: u64,
    price_per_byte_at_assignment: u64,
}

#[derive(Debug, Clone, Copy)]
struct InclusionAssignment {
    tier_id: TierId,
    delay_slots: u64,
    price_per_byte_at_assignment: u64,
}

#[derive(Debug, Clone, Copy)]
struct LegacyInclusionAssignment {
    tier_id: TierId,
    posted_fee: u64,
}

#[derive(Debug, Clone)]
struct TierAssignmentRecord {
    lane: TierLane,
    price_per_byte: u64,
    base_fee: u64,
    delay_slots: u64,
    created_slot: u64,
    retired_slot: Option<u64>,
}

#[derive(Debug, Clone, Default)]
struct TierAssignmentHistory {
    by_key: BTreeMap<(TierId, u64), TierAssignmentRecord>,
    history_window_slots: u64,
}

impl TierAssignmentHistory {
    fn from_tiers(
        tiers: &[Tier],
        tier_update_frequency: u64,
        lane: TierLane,
        min_fee: u64,
    ) -> Self {
        let mut history = Self {
            by_key: BTreeMap::new(),
            history_window_slots: tier_update_frequency
                .saturating_mul(ASSIGNMENT_HISTORY_WINDOW_MULTIPLIER),
        };
        history.sync_with_tiers(tiers, 0, lane, min_fee);
        history
    }

    fn sync_with_tiers(&mut self, tiers: &[Tier], slot: u64, lane: TierLane, min_fee: u64) {
        let mut active_keys = BTreeMap::new();
        for tier in tiers {
            let key = (tier.id, tier.version_created_slot);
            active_keys.insert(key, ());
            self.by_key
                .entry(key)
                .and_modify(|record| {
                    record.lane = lane;
                    record.price_per_byte = tier.price;
                    record.base_fee = min_fee;
                    record.delay_slots = tier.delay;
                    record.retired_slot = None;
                })
                .or_insert(TierAssignmentRecord {
                    lane,
                    price_per_byte: tier.price,
                    base_fee: min_fee,
                    delay_slots: tier.delay,
                    created_slot: tier.version_created_slot,
                    retired_slot: None,
                });
        }

        for (key, record) in &mut self.by_key {
            if active_keys.contains_key(key) {
                continue;
            }
            if record.retired_slot.is_none() {
                record.retired_slot = Some(slot);
            }
        }

        self.prune_retired(slot);
    }

    fn verify_assignment(
        &self,
        assignment: QuotedAssignment,
        tx_bytes: u64,
    ) -> std::result::Result<(), TransactionRejectReason> {
        let Some(record) = self
            .by_key
            .get(&(assignment.tier_id, assignment.version_created_slot))
        else {
            return Err(TransactionRejectReason::QuotedHistoryUnavailable);
        };

        let expected_fee = record
            .base_fee
            .saturating_add(record.price_per_byte.saturating_mul(tx_bytes));
        let quoted_expected_fee = record.base_fee.saturating_add(
            assignment
                .price_per_byte_at_assignment
                .saturating_mul(tx_bytes),
        );
        if record.created_slot != assignment.version_created_slot
            || record.delay_slots != assignment.delay_slots
            || record.price_per_byte != assignment.price_per_byte_at_assignment
            || assignment.posted_fee != expected_fee
            || assignment.posted_fee != quoted_expected_fee
        {
            return Err(TransactionRejectReason::InvalidQuotedAssignment);
        }

        Ok(())
    }

    fn prune_retired(&mut self, slot: u64) {
        self.by_key.retain(|_, record| match record.retired_slot {
            None => true,
            Some(retired_slot) => slot <= retired_slot.saturating_add(self.history_window_slots),
        });
    }
}

#[derive(Debug, Clone)]
pub enum PricingMechanism {
    Baseline(BaselinePricing),
    Eip1559(Eip1559Pricing),
    Eip1559PriorityLane(Eip1559PriorityLanePricing),
    Tiered(TieredPricing),
}

impl PricingMechanism {
    pub fn from_config(config: &PricingMechanismConfig, seed: u64) -> Self {
        match config {
            PricingMechanismConfig::Baseline {
                fee_per_byte,
                base_fee,
            } => PricingMechanism::Baseline(BaselinePricing::new(*fee_per_byte, *base_fee)),
            PricingMechanismConfig::Eip1559 {
                initial_base_fee,
                max_change_denominator,
                target_utilisation,
                smoothing,
            } => PricingMechanism::Eip1559(Eip1559Pricing::new(
                *initial_base_fee,
                *max_change_denominator,
                *target_utilisation,
                smoothing.clone(),
            )),
            PricingMechanismConfig::Eip1559PriorityLane {
                initial_base_fee,
                max_change_denominator,
                target_utilisation,
                priority_fee_multiplier,
                priority_capacity_fraction,
                priority_delay,
                normal_delay,
            } => PricingMechanism::Eip1559PriorityLane(Eip1559PriorityLanePricing::new(
                *initial_base_fee,
                *max_change_denominator,
                *target_utilisation,
                *priority_fee_multiplier,
                *priority_capacity_fraction,
                *priority_delay,
                *normal_delay,
            )),
            PricingMechanismConfig::TieredPricing { tiered_config } => {
                PricingMechanism::Tiered(TieredPricing::new(tiered_config.clone(), seed))
            }
        }
    }

    pub fn snapshot(&self) -> PricingSnapshot {
        match self {
            PricingMechanism::Baseline(pricing) => pricing.snapshot(),
            PricingMechanism::Eip1559(pricing) => pricing.snapshot(),
            PricingMechanism::Eip1559PriorityLane(pricing) => pricing.snapshot(),
            PricingMechanism::Tiered(pricing) => pricing.snapshot(),
        }
    }

    pub fn snapshot_for_block_kind(&self, block_kind: BlockKind) -> PricingSnapshot {
        match self {
            PricingMechanism::Tiered(pricing) => pricing.snapshot_for_block_kind(block_kind),
            _ => self.snapshot(),
        }
    }

    pub fn has_separate_eb_pool(&self) -> bool {
        match self {
            PricingMechanism::Tiered(pricing) => pricing.has_separate_eb_pool(),
            _ => false,
        }
    }

    pub fn lane_selection_backlog_delay_scale(&self) -> f64 {
        match self {
            PricingMechanism::Tiered(pricing) => pricing.lane_selection_backlog_delay_scale(),
            _ => 0.0,
        }
    }

    pub fn reject_on_pending_tier_overflow(&self) -> bool {
        match self {
            PricingMechanism::Tiered(pricing) => pricing.reject_on_pending_tier_overflow(),
            _ => false,
        }
    }

    pub fn block_selection_policy(&self) -> Option<TierBlockSelectionPolicy> {
        match self {
            PricingMechanism::Tiered(pricing) => Some(pricing.block_selection_policy()),
            _ => None,
        }
    }

    pub fn effective_tier_capacity_for_block_kind(
        &self,
        block_kind: BlockKind,
        tier_id: TierId,
        block_capacity: u64,
    ) -> Option<u64> {
        match self {
            PricingMechanism::Tiered(pricing) => {
                pricing.effective_tier_capacity_for_block_kind(block_kind, tier_id, block_capacity)
            }
            _ => None,
        }
    }

    pub fn uses_continuous_rb_eb(&self) -> bool {
        matches!(
            self.block_selection_policy(),
            Some(TierBlockSelectionPolicy::ContinuousRbEb)
                | Some(TierBlockSelectionPolicy::ContinuousRbEbFallback)
        )
    }

    pub fn uses_lane_partitioned_tiers(&self) -> bool {
        self.block_selection_policy()
            .map_or(false, |p| p.is_lane_partitioned())
    }

    pub fn overflow_retry_policy(&self) -> Option<OverflowRetryPolicy> {
        match self {
            PricingMechanism::Tiered(pricing) => Some(pricing.overflow_retry_policy()),
            _ => None,
        }
    }

    pub fn include_overflow_aggregate_in_pricing_updates(&self) -> bool {
        match self {
            PricingMechanism::Tiered(pricing) => {
                pricing.include_overflow_aggregate_in_pricing_updates()
            }
            _ => false,
        }
    }

    pub fn include_overflow_aggregate_in_tier_updates(&self) -> bool {
        match self {
            PricingMechanism::Tiered(pricing) => {
                pricing.include_overflow_aggregate_in_tier_updates()
            }
            _ => false,
        }
    }

    pub fn is_tiered(&self) -> bool {
        matches!(self, PricingMechanism::Tiered(_))
    }

    pub fn is_priority_ordering(&self) -> bool {
        match self {
            PricingMechanism::Tiered(pricing) => pricing.is_priority_ordering(),
            _ => false,
        }
    }

    pub fn verify_preassigned_transaction(
        &self,
        tx: &Transaction,
    ) -> std::result::Result<(), TransactionRejectReason> {
        match self {
            PricingMechanism::Tiered(pricing) => pricing.verify_preassigned_transaction(tx),
            _ => Ok(()),
        }
    }

    pub fn update_after_block(
        &mut self,
        txs: &[Arc<Transaction>],
        block_capacity: u64,
        block_kind: BlockKind,
        slot: u64,
    ) -> TierCadenceUpdate {
        self.update_after_block_with_signals(txs, None, None, block_capacity, block_kind, slot)
    }

    pub fn update_after_block_with_tier_signal(
        &mut self,
        txs: &[Arc<Transaction>],
        tier_update_signal_txs: Option<&[Arc<Transaction>]>,
        block_capacity: u64,
        block_kind: BlockKind,
        slot: u64,
    ) -> TierCadenceUpdate {
        self.update_after_block_with_signals(
            txs,
            tier_update_signal_txs,
            None,
            block_capacity,
            block_kind,
            slot,
        )
    }

    pub fn update_after_block_with_signals(
        &mut self,
        txs: &[Arc<Transaction>],
        tier_update_signal_txs: Option<&[Arc<Transaction>]>,
        overflow_pricing_signal_txs: Option<&[Arc<Transaction>]>,
        block_capacity: u64,
        block_kind: BlockKind,
        slot: u64,
    ) -> TierCadenceUpdate {
        match self {
            PricingMechanism::Baseline(pricing) => {
                pricing.update_after_block(txs, block_capacity);
                TierCadenceUpdate::default()
            }
            PricingMechanism::Eip1559(pricing) => {
                pricing.update_after_block(txs, block_capacity, slot);
                TierCadenceUpdate::default()
            }
            PricingMechanism::Eip1559PriorityLane(pricing) => {
                pricing.update_after_block(txs, block_capacity, slot);
                TierCadenceUpdate::default()
            }
            PricingMechanism::Tiered(pricing) => pricing.update_after_block_with_signals(
                txs,
                tier_update_signal_txs,
                overflow_pricing_signal_txs,
                block_capacity,
                block_kind,
                slot,
            ),
        }
    }

    pub fn select_transactions_for_block(
        &self,
        txs: &[Arc<Transaction>],
        slot: u64,
        block_capacity: u64,
        block_kind: BlockKind,
    ) -> Vec<Arc<Transaction>> {
        match self {
            PricingMechanism::Baseline(pricing) => {
                select_single_tier(txs, pricing.snapshot(), slot, block_capacity)
            }
            PricingMechanism::Eip1559(pricing) => {
                pricing.select_transactions(txs, slot, block_capacity)
            }
            PricingMechanism::Eip1559PriorityLane(pricing) => {
                pricing.select_transactions(txs, slot, block_capacity)
            }
            PricingMechanism::Tiered(pricing) => {
                pricing.select_transactions(txs, slot, block_capacity, block_kind)
            }
        }
    }

    pub fn tiers(&self) -> Option<&[Tier]> {
        match self {
            PricingMechanism::Tiered(pricing) => Some(&pricing.state.tiers),
            _ => None,
        }
    }

    pub fn cloned_tiers_for_block_kind(&self, block_kind: BlockKind) -> Option<Vec<Tier>> {
        match self {
            PricingMechanism::Eip1559PriorityLane(pricing) => {
                Some(pricing.cloned_tiers_for_block_kind())
            }
            PricingMechanism::Tiered(pricing) => {
                Some(pricing.cloned_tiers_for_block_kind(block_kind))
            }
            _ => None,
        }
    }

    pub fn tiers_for_block_kind(&self, block_kind: BlockKind) -> Option<&[Tier]> {
        match self {
            PricingMechanism::Eip1559PriorityLane(pricing) => Some(pricing.tiers_for_block_kind()),
            PricingMechanism::Tiered(pricing) => match block_kind {
                BlockKind::EndorserBlock => pricing
                    .eb_state
                    .as_ref()
                    .map(|s| s.tiers.as_slice())
                    .or(Some(&pricing.state.tiers)),
                BlockKind::RankingBlock => Some(&pricing.state.tiers),
            },
            _ => None,
        }
    }

    pub fn tier_capacity_for_block_kind(
        &self,
        block_kind: BlockKind,
        tier_id: TierId,
    ) -> Option<u64> {
        self.tiers_for_block_kind(block_kind)?
            .iter()
            .find(|tier| tier.id == tier_id)
            .map(|tier| tier.capacity)
    }

    pub fn tier_utilisations(&self) -> Vec<f64> {
        match self {
            PricingMechanism::Eip1559PriorityLane(pricing) => pricing.last_utilisations.clone(),
            PricingMechanism::Tiered(pricing) => pricing.state.last_utilisations.clone(),
            _ => Vec::new(),
        }
    }

    pub fn tier_utilisations_for_block_kind(&self, block_kind: BlockKind) -> Vec<f64> {
        match self {
            PricingMechanism::Eip1559PriorityLane(pricing) => pricing.last_utilisations.clone(),
            PricingMechanism::Tiered(pricing) => {
                pricing.tier_utilisations_for_block_kind(block_kind)
            }
            _ => Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BaselinePricing {
    fee_per_byte: u64,
    base_fee: u64,
}

impl BaselinePricing {
    pub fn new(fee_per_byte: u64, base_fee: u64) -> Self {
        Self {
            fee_per_byte,
            base_fee,
        }
    }

    pub fn snapshot(&self) -> PricingSnapshot {
        PricingSnapshot {
            tiers: vec![TierQuote {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                version_created_slot: 0,
                delay: 1,
                price_per_byte: self.fee_per_byte,
                base_fee: self.base_fee,
            }],
        }
    }

    pub fn update_after_block(&mut self, _txs: &[Arc<Transaction>], _block_capacity: u64) {}
}

#[derive(Debug, Clone)]
pub struct Eip1559Pricing {
    base_fee_per_byte: u64,
    max_change_denominator: u64,
    target_utilisation: f64,
    smoothing: Eip1559SmoothingConfig,
    ema_included_bytes: Option<f64>,
    ema_capacity_bytes: Option<f64>,
    fee_history: SingleTierFeeHistory,
}

impl Eip1559Pricing {
    pub fn new(
        initial_base_fee: u64,
        max_change_denominator: u64,
        target_utilisation: f64,
        smoothing: Eip1559SmoothingConfig,
    ) -> Self {
        let initial_tier = Tier {
            id: TierId::new(0),
            lane: TierLane::Ranking,
            capacity: 0,
            version_created_slot: 0,
            delay: 1,
            price: initial_base_fee.max(1),
            used_capacity: 0,
            tx_count: 0,
        };
        Self {
            base_fee_per_byte: initial_base_fee.max(1),
            max_change_denominator,
            target_utilisation,
            smoothing,
            ema_included_bytes: None,
            ema_capacity_bytes: None,
            fee_history: SingleTierFeeHistory::from_tiers(&[initial_tier], 0),
        }
    }

    pub fn snapshot(&self) -> PricingSnapshot {
        PricingSnapshot {
            tiers: vec![TierQuote {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                version_created_slot: 0,
                delay: 1,
                price_per_byte: self.base_fee_per_byte,
                base_fee: 0,
            }],
        }
    }

    pub fn update_after_block(&mut self, txs: &[Arc<Transaction>], block_capacity: u64, slot: u64) {
        if block_capacity == 0 {
            return;
        }
        let total_bytes = txs.iter().map(|tx| tx.bytes).sum::<u64>();
        let fill_rate = self.effective_fill_rate(total_bytes, block_capacity);
        self.base_fee_per_byte = update_eip1559_price(
            self.base_fee_per_byte,
            fill_rate,
            self.target_utilisation,
            self.max_change_denominator,
        );
        self.fee_history
            .record_tier_state(TierId::new(0), self.base_fee_per_byte, 1, slot);
    }

    pub fn select_transactions(
        &self,
        txs: &[Arc<Transaction>],
        slot: u64,
        block_capacity: u64,
    ) -> Vec<Arc<Transaction>> {
        select_single_tier_with_history(
            txs,
            self.snapshot(),
            slot,
            block_capacity,
            Some(&self.fee_history),
        )
    }

    fn effective_fill_rate(&mut self, total_bytes: u64, block_capacity: u64) -> f64 {
        if !self.smoothing.enabled {
            return total_bytes as f64 / block_capacity as f64;
        }

        let alpha = self.smoothing.alpha;
        let current_bytes = total_bytes as f64;
        let current_capacity = block_capacity as f64;
        let smoothed_bytes = match self.ema_included_bytes {
            Some(previous) => alpha * current_bytes + (1.0 - alpha) * previous,
            None => current_bytes,
        };
        let smoothed_capacity = match self.ema_capacity_bytes {
            Some(previous) => alpha * current_capacity + (1.0 - alpha) * previous,
            None => current_capacity,
        };

        self.ema_included_bytes = Some(smoothed_bytes);
        self.ema_capacity_bytes = Some(smoothed_capacity);

        if smoothed_capacity <= 0.0 {
            0.0
        } else {
            smoothed_bytes / smoothed_capacity
        }
    }
}

#[derive(Debug, Clone)]
pub struct Eip1559PriorityLanePricing {
    base_fee_per_byte: u64,
    max_change_denominator: u64,
    target_utilisation: f64,
    priority_fee_multiplier: f64,
    priority_capacity_fraction: f64,
    priority_delay: u64,
    normal_delay: u64,
    fee_history: SingleTierFeeHistory,
    last_tiers: Vec<Tier>,
    last_utilisations: Vec<f64>,
}

impl Eip1559PriorityLanePricing {
    pub fn new(
        initial_base_fee: u64,
        max_change_denominator: u64,
        target_utilisation: f64,
        priority_fee_multiplier: f64,
        priority_capacity_fraction: f64,
        priority_delay: u64,
        normal_delay: u64,
    ) -> Self {
        let base_fee_per_byte = initial_base_fee.max(1);
        let mut pricing = Self {
            base_fee_per_byte,
            max_change_denominator,
            target_utilisation,
            priority_fee_multiplier,
            priority_capacity_fraction,
            priority_delay,
            normal_delay,
            fee_history: SingleTierFeeHistory::default(),
            last_tiers: Vec::new(),
            last_utilisations: vec![0.0, 0.0],
        };
        pricing.last_tiers = pricing.synthetic_tiers(0);
        pricing.fee_history = SingleTierFeeHistory::from_tiers(&pricing.last_tiers, 0);
        pricing
    }

    pub fn snapshot(&self) -> PricingSnapshot {
        PricingSnapshot {
            tiers: vec![
                TierQuote {
                    id: Self::priority_tier_id(),
                    lane: TierLane::Ranking,
                    version_created_slot: 0,
                    delay: self.priority_delay,
                    price_per_byte: self.priority_price_per_byte(),
                    base_fee: 0,
                },
                TierQuote {
                    id: Self::normal_tier_id(),
                    lane: TierLane::Ranking,
                    version_created_slot: 0,
                    delay: self.normal_delay,
                    price_per_byte: self.base_fee_per_byte,
                    base_fee: 0,
                },
            ],
        }
    }

    pub fn update_after_block(&mut self, txs: &[Arc<Transaction>], block_capacity: u64, slot: u64) {
        if block_capacity == 0 {
            return;
        }

        let priority_bytes = txs
            .iter()
            .filter(|tx| tx.tier_preference == Some(Self::priority_tier_id()))
            .map(|tx| tx.bytes)
            .sum::<u64>();
        let normal_bytes = txs
            .iter()
            .filter(|tx| tx.tier_preference == Some(Self::normal_tier_id()))
            .map(|tx| tx.bytes)
            .sum::<u64>();
        let total_bytes = txs.iter().map(|tx| tx.bytes).sum::<u64>();
        let fill_rate = total_bytes as f64 / block_capacity as f64;

        self.base_fee_per_byte = update_eip1559_price(
            self.base_fee_per_byte,
            fill_rate,
            self.target_utilisation,
            self.max_change_denominator,
        );

        let priority_capacity = self.priority_capacity(block_capacity);
        self.last_utilisations = vec![
            utilisation(priority_bytes, priority_capacity),
            utilisation(normal_bytes, block_capacity),
        ];
        self.last_tiers = self.synthetic_tiers(block_capacity);
        for tier in &self.last_tiers {
            self.fee_history
                .record_tier_state(tier.id, tier.price, tier.delay, slot);
        }
    }

    pub fn select_transactions(
        &self,
        txs: &[Arc<Transaction>],
        slot: u64,
        block_capacity: u64,
    ) -> Vec<Arc<Transaction>> {
        let snapshot = self.snapshot();
        let Some(priority_quote) = snapshot
            .tiers
            .iter()
            .find(|tier| tier.id == Self::priority_tier_id())
        else {
            return Vec::new();
        };
        let Some(normal_quote) = snapshot
            .tiers
            .iter()
            .find(|tier| tier.id == Self::normal_tier_id())
        else {
            return Vec::new();
        };

        let mut priority_candidates =
            self.candidates_for_tier(txs, priority_quote, slot, self.priority_delay);
        let mut normal_candidates =
            self.candidates_for_tier(txs, normal_quote, slot, self.normal_delay);
        priority_candidates.sort_by(|a, b| compare_submission_order(a, b));
        normal_candidates.sort_by(|a, b| compare_submission_order(a, b));

        let mut included = Vec::new();
        let mut remaining_block = block_capacity;
        let mut remaining_priority = self.priority_capacity(block_capacity);
        for tx in priority_candidates {
            if tx.bytes <= remaining_priority && tx.bytes <= remaining_block {
                remaining_priority = remaining_priority.saturating_sub(tx.bytes);
                remaining_block = remaining_block.saturating_sub(tx.bytes);
                included.push(tx);
            }
        }
        for tx in normal_candidates {
            if tx.bytes <= remaining_block {
                remaining_block = remaining_block.saturating_sub(tx.bytes);
                included.push(tx);
            }
        }
        included
    }

    pub fn cloned_tiers_for_block_kind(&self) -> Vec<Tier> {
        self.last_tiers.clone()
    }

    pub fn tiers_for_block_kind(&self) -> &[Tier] {
        &self.last_tiers
    }

    fn candidates_for_tier(
        &self,
        txs: &[Arc<Transaction>],
        tier: &TierQuote,
        slot: u64,
        quote_grace_slots: u64,
    ) -> Vec<Arc<Transaction>> {
        txs.iter()
            .filter(|tx| tx.tier_preference == Some(tier.id))
            .filter(|tx| match tx.posted_fee {
                Some(posted_fee) => self.fee_history.fee_satisfies(
                    tier.id,
                    tier.price_per_byte,
                    tx.submission_slot,
                    slot,
                    tx.bytes,
                    posted_fee,
                    quote_grace_slots,
                ),
                None => false,
            })
            .cloned()
            .collect()
    }

    fn synthetic_tiers(&self, block_capacity: u64) -> Vec<Tier> {
        vec![
            Tier {
                id: Self::priority_tier_id(),
                lane: TierLane::Ranking,
                capacity: self.priority_capacity(block_capacity),
                version_created_slot: 0,
                delay: self.priority_delay,
                price: self.priority_price_per_byte(),
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: Self::normal_tier_id(),
                lane: TierLane::Ranking,
                capacity: block_capacity,
                version_created_slot: 0,
                delay: self.normal_delay,
                price: self.base_fee_per_byte,
                used_capacity: 0,
                tx_count: 0,
            },
        ]
    }

    fn priority_capacity(&self, block_capacity: u64) -> u64 {
        ((block_capacity as f64) * self.priority_capacity_fraction)
            .round()
            .clamp(0.0, block_capacity as f64) as u64
    }

    fn priority_price_per_byte(&self) -> u64 {
        let price = (self.base_fee_per_byte as f64 * self.priority_fee_multiplier).ceil();
        if !price.is_finite() || price >= u64::MAX as f64 {
            u64::MAX
        } else {
            (price as u64).max(1)
        }
    }

    fn priority_tier_id() -> TierId {
        TierId::new(0)
    }

    fn normal_tier_id() -> TierId {
        TierId::new(1)
    }
}

#[derive(Debug, Clone)]
pub struct TieredPricing {
    config: TieredConfig,
    state: TieredState,
    rng: ChaChaRng,
    assignment_history: TierAssignmentHistory,
    /// When `eb_total_capacity` is set, EB blocks use a separate tier pool.
    eb_config: Option<TieredConfig>,
    eb_state: Option<TieredState>,
    eb_rng: Option<ChaChaRng>,
    eb_assignment_history: Option<TierAssignmentHistory>,
}

impl TieredPricing {
    pub fn new(config: TieredConfig, seed: u64) -> Self {
        let state = TieredState::new(&config, TierLane::Ranking);
        let rng = ChaChaRng::seed_from_u64(seed ^ 0x54_49_45_52_45_44_5F_53);
        let assignment_history = TierAssignmentHistory::from_tiers(
            &state.tiers,
            config.effective_tier_update_cadence_slots(),
            TierLane::Ranking,
            config.min_fee,
        );

        let (eb_config, eb_state, eb_rng, eb_assignment_history) =
            if let Some(eb_capacity) = config.eb_total_capacity {
                let eb_cfg = config.with_total_capacity(eb_capacity);
                let eb_st = TieredState::new(&eb_cfg, TierLane::Endorser);
                let eb_r = ChaChaRng::seed_from_u64(seed ^ 0x45_42_5F_54_49_45_52_53);
                let eb_ah = TierAssignmentHistory::from_tiers(
                    &eb_st.tiers,
                    eb_cfg.effective_tier_update_cadence_slots(),
                    TierLane::Endorser,
                    eb_cfg.min_fee,
                );
                (Some(eb_cfg), Some(eb_st), Some(eb_r), Some(eb_ah))
            } else {
                (None, None, None, None)
            };

        Self {
            config,
            state,
            rng,
            assignment_history,
            eb_config,
            eb_state,
            eb_rng,
            eb_assignment_history,
        }
    }

    pub fn has_separate_eb_pool(&self) -> bool {
        self.eb_state.is_some()
    }

    pub fn reject_on_pending_tier_overflow(&self) -> bool {
        self.config.reject_on_pending_tier_overflow
    }

    pub fn block_selection_policy(&self) -> TierBlockSelectionPolicy {
        self.config.block_selection_policy
    }

    pub fn overflow_retry_policy(&self) -> OverflowRetryPolicy {
        self.config.overflow_retry_policy.clone()
    }

    pub fn include_overflow_aggregate_in_pricing_updates(&self) -> bool {
        self.config.include_overflow_aggregate_in_pricing_updates
    }

    pub fn include_overflow_aggregate_in_tier_updates(&self) -> bool {
        self.config.include_overflow_aggregate_in_tier_updates
    }

    pub fn is_priority_ordering(&self) -> bool {
        self.config.priority_ordering
    }

    pub fn lane_selection_backlog_delay_scale(&self) -> f64 {
        self.config.lane_selection_backlog_delay_scale
    }

    pub fn snapshot(&self) -> PricingSnapshot {
        self.snapshot_for_state(&self.state)
    }

    pub fn snapshot_for_block_kind(&self, block_kind: BlockKind) -> PricingSnapshot {
        match block_kind {
            BlockKind::EndorserBlock => {
                if let Some(eb_state) = &self.eb_state {
                    return self.snapshot_for_state(eb_state);
                }
            }
            BlockKind::RankingBlock => {}
        }
        // For lane-partitioned policies without a separate EB pool (e.g. NaiveRbEbTwoTier),
        // filter tiers to only those matching the requested block kind's lane.
        if self.config.block_selection_policy.is_lane_partitioned() {
            let target_lane = tier_lane_for_block_kind(block_kind);
            return PricingSnapshot {
                tiers: self
                    .state
                    .tiers
                    .iter()
                    .filter(|tier| tier.lane == target_lane)
                    .map(|tier| TierQuote {
                        id: tier.id,
                        lane: tier.lane,
                        version_created_slot: tier.version_created_slot,
                        delay: tier.delay,
                        price_per_byte: tier.price,
                        base_fee: self.config.min_fee,
                    })
                    .collect(),
            };
        }
        self.snapshot_for_state(&self.state)
    }

    fn cloned_tiers_for_block_kind(&self, block_kind: BlockKind) -> Vec<Tier> {
        match block_kind {
            BlockKind::EndorserBlock => {
                if let Some(eb_state) = &self.eb_state {
                    return eb_state.tiers.clone();
                }
            }
            BlockKind::RankingBlock => {}
        }
        if self.config.block_selection_policy.is_lane_partitioned() {
            let target_lane = tier_lane_for_block_kind(block_kind);
            return self
                .state
                .tiers
                .iter()
                .filter(|tier| tier.lane == target_lane)
                .cloned()
                .collect();
        }
        self.state.tiers.clone()
    }

    fn tier_utilisations_for_block_kind(&self, block_kind: BlockKind) -> Vec<f64> {
        match block_kind {
            BlockKind::EndorserBlock => {
                if let Some(eb_state) = &self.eb_state {
                    return eb_state.last_utilisations.clone();
                }
            }
            BlockKind::RankingBlock => {}
        }
        if self.config.block_selection_policy.is_lane_partitioned() {
            let target_lane = tier_lane_for_block_kind(block_kind);
            return self
                .state
                .tiers
                .iter()
                .zip(self.state.last_utilisations.iter().copied())
                .filter_map(|(tier, utilisation)| (tier.lane == target_lane).then_some(utilisation))
                .collect();
        }
        self.state.last_utilisations.clone()
    }

    fn effective_tier_capacity_for_block_kind(
        &self,
        block_kind: BlockKind,
        tier_id: TierId,
        block_capacity: u64,
    ) -> Option<u64> {
        let (tiers, config, has_separate_eb_pool) = match block_kind {
            BlockKind::EndorserBlock => {
                if let (Some(eb_state), Some(eb_config)) = (&self.eb_state, &self.eb_config) {
                    (&eb_state.tiers, eb_config, true)
                } else {
                    (&self.state.tiers, &self.config, false)
                }
            }
            BlockKind::RankingBlock => (&self.state.tiers, &self.config, self.eb_state.is_some()),
        };
        let (index, tier) = tiers
            .iter()
            .enumerate()
            .find(|(_, tier)| tier.id == tier_id)?;
        let shared_scale = if config.total_capacity == 0 {
            0.0
        } else {
            block_capacity as f64 / config.total_capacity as f64
        };
        if config.priority_ordering {
            // Priority ordering: each tier can use the full block capacity,
            // so overflow checks should use total capacity, not per-tier fractions.
            let capacity = if has_separate_eb_pool {
                config.total_capacity
            } else {
                (config.total_capacity as f64 * shared_scale).round() as u64
            };
            return Some(capacity);
        }
        let eb_capacity_sum = tiers
            .iter()
            .skip(1)
            .map(|candidate| candidate.capacity)
            .sum::<u64>();
        let effective_capacity = match config.block_selection_policy {
            TierBlockSelectionPolicy::Shared
            | TierBlockSelectionPolicy::ContinuousRbEb
            | TierBlockSelectionPolicy::ContinuousRbEbFallback => {
                if has_separate_eb_pool {
                    tier.capacity
                } else {
                    (tier.capacity as f64 * shared_scale).round() as u64
                }
            }
            TierBlockSelectionPolicy::NaiveRbEbTwoTier => {
                if config.block_selection_policy.allows_tier(index, block_kind) {
                    block_capacity
                } else {
                    0
                }
            }
            TierBlockSelectionPolicy::RbTier0Reserved => match block_kind {
                BlockKind::RankingBlock => config.rb_reserved_capacity(block_capacity),
                BlockKind::EndorserBlock => {
                    if eb_capacity_sum == 0 {
                        0
                    } else {
                        ((tier.capacity as f64 / eb_capacity_sum as f64) * block_capacity as f64)
                            .round() as u64
                    }
                }
            },
        };
        if matches!(
            config.block_selection_policy,
            TierBlockSelectionPolicy::ContinuousRbEb
                | TierBlockSelectionPolicy::ContinuousRbEbFallback
        ) && block_kind == BlockKind::RankingBlock
            && tier.lane == TierLane::Ranking
            && tier.id == TierId::new(0)
        {
            return Some(effective_capacity.max(config.rb_soft_reserved_capacity(block_capacity)));
        }
        Some(effective_capacity)
    }

    fn snapshot_for_state(&self, state: &TieredState) -> PricingSnapshot {
        PricingSnapshot {
            tiers: state
                .tiers
                .iter()
                .map(|tier| TierQuote {
                    id: tier.id,
                    lane: tier.lane,
                    version_created_slot: tier.version_created_slot,
                    delay: tier.delay,
                    price_per_byte: tier.price,
                    base_fee: self.config.min_fee,
                })
                .collect(),
        }
    }

    pub fn verify_preassigned_transaction(
        &self,
        tx: &Transaction,
    ) -> std::result::Result<(), TransactionRejectReason> {
        let rb_assignment = Self::extract_quoted_assignment(
            tx.tier_preference,
            tx.tier_version_created_slot,
            tx.posted_fee,
            tx.tier_delay_slots,
            tx.tier_price_per_byte_at_assignment,
        )?;
        let eb_assignment = if self.has_separate_eb_pool() {
            Self::extract_quoted_assignment(
                tx.eb_tier_preference,
                tx.eb_tier_version_created_slot,
                tx.eb_posted_fee,
                tx.eb_tier_delay_slots,
                tx.eb_tier_price_per_byte_at_assignment,
            )?
        } else {
            None
        };

        if rb_assignment.is_none() && eb_assignment.is_none() {
            return Err(TransactionRejectReason::InvalidQuotedAssignment);
        }
        if self.config.block_selection_policy == TierBlockSelectionPolicy::ContinuousRbEb
            && rb_assignment.is_some()
            && eb_assignment.is_some()
        {
            return Err(TransactionRejectReason::InvalidQuotedAssignment);
        }

        if let Some(assignment) = rb_assignment {
            self.assignment_history
                .verify_assignment(assignment, tx.bytes)?;
        }

        if let Some(assignment) = eb_assignment {
            let Some(history) = self.eb_assignment_history.as_ref() else {
                return Err(TransactionRejectReason::QuotedHistoryUnavailable);
            };
            history.verify_assignment(assignment, tx.bytes)?;
        }

        Ok(())
    }

    fn extract_quoted_assignment(
        tier_id: Option<TierId>,
        version_created_slot: Option<u64>,
        posted_fee: Option<u64>,
        delay_slots: Option<u64>,
        price_per_byte_at_assignment: Option<u64>,
    ) -> std::result::Result<Option<QuotedAssignment>, TransactionRejectReason> {
        let has_any = tier_id.is_some()
            || version_created_slot.is_some()
            || posted_fee.is_some()
            || delay_slots.is_some()
            || price_per_byte_at_assignment.is_some();
        if !has_any {
            return Ok(None);
        }

        let (
            Some(tier_id),
            Some(version_created_slot),
            Some(posted_fee),
            Some(delay_slots),
            Some(price_per_byte_at_assignment),
        ) = (
            tier_id,
            version_created_slot,
            posted_fee,
            delay_slots,
            price_per_byte_at_assignment,
        )
        else {
            return Err(TransactionRejectReason::InvalidQuotedAssignment);
        };

        Ok(Some(QuotedAssignment {
            tier_id,
            version_created_slot,
            posted_fee,
            delay_slots,
            price_per_byte_at_assignment,
        }))
    }

    fn lane_assignment_for_inclusion(
        tx: &Transaction,
        has_separate_eb_pool: bool,
        use_eb_preference: bool,
    ) -> Option<InclusionAssignment> {
        if use_eb_preference && has_separate_eb_pool {
            let tier_id = tx.eb_tier_preference?;
            let delay_slots = tx.eb_tier_delay_slots.unwrap_or(1);
            let price_per_byte_at_assignment = tx
                .eb_tier_price_per_byte_at_assignment
                .or_else(|| Self::derive_price_per_byte(tx.eb_posted_fee, tx.bytes))
                .unwrap_or(0);
            return Some(InclusionAssignment {
                tier_id,
                delay_slots,
                price_per_byte_at_assignment,
            });
        }

        let tier_id = tx.tier_preference?;
        let delay_slots = tx.tier_delay_slots.unwrap_or(1);
        let price_per_byte_at_assignment = tx
            .tier_price_per_byte_at_assignment
            .or_else(|| Self::derive_price_per_byte(tx.posted_fee, tx.bytes))
            .unwrap_or(0);
        Some(InclusionAssignment {
            tier_id,
            delay_slots,
            price_per_byte_at_assignment,
        })
    }

    fn lane_assignment_for_legacy_inclusion(
        tx: &Transaction,
        has_separate_eb_pool: bool,
        use_eb_preference: bool,
    ) -> Option<LegacyInclusionAssignment> {
        if use_eb_preference && has_separate_eb_pool {
            return Some(LegacyInclusionAssignment {
                tier_id: tx.eb_tier_preference?,
                posted_fee: tx.eb_posted_fee?,
            });
        }

        Some(LegacyInclusionAssignment {
            tier_id: tx.tier_preference?,
            posted_fee: tx.posted_fee?,
        })
    }

    fn derive_price_per_byte(posted_fee: Option<u64>, bytes: u64) -> Option<u64> {
        if bytes == 0 {
            return None;
        }
        posted_fee.map(|fee| fee / bytes)
    }

    fn resolve_inclusion_tier_index(
        state: &TieredState,
        index_by_id: &BTreeMap<TierId, usize>,
        assigned_tier_id: TierId,
        assigned_delay_slots: u64,
        assigned_price_per_byte: u64,
    ) -> Option<usize> {
        if let Some(index) = index_by_id.get(&assigned_tier_id).copied() {
            return Some(index);
        }
        if state.tiers.is_empty() {
            return None;
        }

        let mut best_equal_or_cheaper: Option<(u64, u64, TierId, usize)> = None;
        for (index, tier) in state.tiers.iter().enumerate() {
            if tier.price > assigned_price_per_byte {
                continue;
            }
            let delay_distance = tier.delay.abs_diff(assigned_delay_slots);
            let candidate = (delay_distance, tier.price, tier.id, index);
            if best_equal_or_cheaper.is_none_or(|current| candidate < current) {
                best_equal_or_cheaper = Some(candidate);
            }
        }
        if let Some((_, _, _, index)) = best_equal_or_cheaper {
            return Some(index);
        }

        // No equal-or-cheaper active tier survived. Route to deterministic lowest priority:
        // largest delay, then highest tier id.
        state
            .tiers
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.delay.cmp(&b.delay).then_with(|| a.id.cmp(&b.id)))
            .map(|(index, _)| index)
    }

    pub fn update_after_block(
        &mut self,
        txs: &[Arc<Transaction>],
        block_capacity: u64,
        block_kind: BlockKind,
        slot: u64,
    ) -> TierCadenceUpdate {
        self.update_after_block_with_signals(txs, None, None, block_capacity, block_kind, slot)
    }

    pub fn update_after_block_with_tier_signal(
        &mut self,
        txs: &[Arc<Transaction>],
        tier_update_signal_txs: Option<&[Arc<Transaction>]>,
        block_capacity: u64,
        block_kind: BlockKind,
        slot: u64,
    ) -> TierCadenceUpdate {
        self.update_after_block_with_signals(
            txs,
            tier_update_signal_txs,
            None,
            block_capacity,
            block_kind,
            slot,
        )
    }

    pub fn update_after_block_with_signals(
        &mut self,
        txs: &[Arc<Transaction>],
        tier_update_signal_txs: Option<&[Arc<Transaction>]>,
        overflow_pricing_signal_txs: Option<&[Arc<Transaction>]>,
        block_capacity: u64,
        block_kind: BlockKind,
        slot: u64,
    ) -> TierCadenceUpdate {
        if block_kind == BlockKind::EndorserBlock && self.eb_state.is_some() {
            // EB blocks update the separate EB tier pool.
            let eb_config = self.eb_config.as_ref().unwrap();
            let overflow_pricing_fill_rates = self.overflow_pricing_fill_rates_for_block(
                overflow_pricing_signal_txs,
                eb_config,
                block_capacity,
                block_kind,
                true,
                slot,
            );
            let tier_update_signal_fill_rates =
                if self.config.include_overflow_aggregate_in_tier_updates {
                    let signal_txs = tier_update_signal_txs.unwrap_or(txs);
                    let mut shadow_state = self.eb_state.as_ref().unwrap().clone();
                    Some(Self::record_block_usage_static(
                        signal_txs,
                        block_capacity,
                        block_kind,
                        &mut shadow_state,
                        eb_config,
                        true,
                        slot,
                    ))
                } else {
                    None
                };
            let included_fill_rates = Self::record_block_usage_static(
                txs,
                block_capacity,
                block_kind,
                self.eb_state.as_mut().unwrap(),
                eb_config,
                true,
                slot,
            );
            let fill_rates = Self::combined_pricing_fill_rates(
                eb_config,
                included_fill_rates,
                overflow_pricing_fill_rates.as_deref(),
            );
            let eb_config = self.eb_config.as_ref().unwrap();
            let eb_state = self.eb_state.as_mut().unwrap();
            let eb_rng = self.eb_rng.as_mut().unwrap();
            let cadence = eb_state.update_with_fill_rates(
                eb_config,
                eb_rng,
                fill_rates,
                tier_update_signal_fill_rates,
                block_kind,
                slot,
            );
            Self::apply_linear_overflow_adjustment(
                eb_config,
                eb_state,
                overflow_pricing_fill_rates.as_deref(),
                block_kind,
                slot,
            );
            Self::enforce_boundary_price_caps(eb_config, eb_state, block_kind, slot);
            let eb_history = self.eb_assignment_history.as_mut().unwrap();
            eb_history.sync_with_tiers(
                &self.eb_state.as_ref().unwrap().tiers,
                slot,
                TierLane::Endorser,
                self.eb_config.as_ref().unwrap().min_fee,
            );
            cadence
        } else {
            let overflow_pricing_fill_rates = self.overflow_pricing_fill_rates_for_block(
                overflow_pricing_signal_txs,
                &self.config,
                block_capacity,
                block_kind,
                false,
                slot,
            );
            let tier_update_signal_fill_rates =
                if self.config.include_overflow_aggregate_in_tier_updates {
                    let signal_txs = tier_update_signal_txs.unwrap_or(txs);
                    let mut shadow_state = self.state.clone();
                    Some(Self::record_block_usage_static(
                        signal_txs,
                        block_capacity,
                        block_kind,
                        &mut shadow_state,
                        &self.config,
                        false,
                        slot,
                    ))
                } else {
                    None
                };
            let included_fill_rates = Self::record_block_usage_static(
                txs,
                block_capacity,
                block_kind,
                &mut self.state,
                &self.config,
                false,
                slot,
            );
            let fill_rates = Self::combined_pricing_fill_rates(
                &self.config,
                included_fill_rates,
                overflow_pricing_fill_rates.as_deref(),
            );
            let cadence = self.state.update_with_fill_rates(
                &self.config,
                &mut self.rng,
                fill_rates,
                tier_update_signal_fill_rates,
                block_kind,
                slot,
            );
            Self::apply_linear_overflow_adjustment(
                &self.config,
                &mut self.state,
                overflow_pricing_fill_rates.as_deref(),
                block_kind,
                slot,
            );
            Self::enforce_boundary_price_caps(&self.config, &mut self.state, block_kind, slot);
            self.assignment_history.sync_with_tiers(
                &self.state.tiers,
                slot,
                TierLane::Ranking,
                self.config.min_fee,
            );
            cadence
        }
    }

    fn overflow_pricing_fill_rates_for_block(
        &self,
        overflow_pricing_signal_txs: Option<&[Arc<Transaction>]>,
        config: &TieredConfig,
        block_capacity: u64,
        block_kind: BlockKind,
        use_eb_preference: bool,
        slot: u64,
    ) -> Option<Vec<f64>> {
        if !config.include_overflow_aggregate_in_pricing_updates {
            return None;
        }
        let overflow_txs = overflow_pricing_signal_txs?;
        if overflow_txs.is_empty() {
            return None;
        }
        let mut shadow_state = if use_eb_preference {
            self.eb_state
                .as_ref()
                .expect("eb pool state missing")
                .clone()
        } else {
            self.state.clone()
        };
        Some(Self::record_block_usage_static(
            overflow_txs,
            block_capacity,
            block_kind,
            &mut shadow_state,
            config,
            use_eb_preference,
            slot,
        ))
    }

    fn combined_pricing_fill_rates(
        config: &TieredConfig,
        mut included_fill_rates: Vec<f64>,
        overflow_fill_rates: Option<&[f64]>,
    ) -> Vec<f64> {
        if !config.include_overflow_aggregate_in_pricing_updates
            || config.overflow_aggregate_pricing_mode
                != OverflowAggregatePricingMode::IncludeAsFillRate
        {
            return included_fill_rates;
        }
        let Some(overflow_fill_rates) = overflow_fill_rates else {
            return included_fill_rates;
        };
        for (fill_rate, overflow_fill_rate) in included_fill_rates
            .iter_mut()
            .zip(overflow_fill_rates.iter().copied())
        {
            *fill_rate += overflow_fill_rate;
        }
        included_fill_rates
    }

    fn apply_linear_overflow_adjustment(
        config: &TieredConfig,
        state: &mut TieredState,
        overflow_fill_rates: Option<&[f64]>,
        block_kind: BlockKind,
        slot: u64,
    ) {
        if !config.include_overflow_aggregate_in_pricing_updates
            || config.overflow_aggregate_pricing_mode
                != OverflowAggregatePricingMode::LinearAdditive
        {
            return;
        }
        let Some(overflow_fill_rates) = overflow_fill_rates else {
            return;
        };
        let cap = config.overflow_linear_fill_rate_cap;
        if cap <= 0.0 {
            return;
        }

        for (index, tier) in state.tiers.iter_mut().enumerate() {
            if config.block_selection_policy.uses_continuous_lane_pricing()
                && tier.lane != tier_lane_for_block_kind(block_kind)
            {
                continue;
            }
            if config.block_selection_policy.is_lane_partitioned()
                && !config.block_selection_policy.allows_tier(index, block_kind)
            {
                continue;
            }
            let overflow_fill_rate = overflow_fill_rates.get(index).copied().unwrap_or(0.0);
            if overflow_fill_rate <= 0.0 {
                continue;
            }
            let capped_fill_rate = overflow_fill_rate.min(cap);
            let scale = config.overflow_linear_price_per_fill_for_lane(tier.lane) as f64;
            if scale <= 0.0 {
                continue;
            }
            let linear_increment = (capped_fill_rate * scale).round() as u64;
            if linear_increment == 0 {
                continue;
            }
            let updated_price = tier.price.saturating_add(linear_increment);
            if updated_price != tier.price {
                tier.version_created_slot = slot;
                tier.price = updated_price;
            }
        }
    }

    fn enforce_boundary_price_caps(
        config: &TieredConfig,
        state: &mut TieredState,
        block_kind: BlockKind,
        slot: u64,
    ) {
        if !config.enforce_boundary_price_caps {
            return;
        }

        let active_indices: Vec<usize> = match config.block_selection_policy {
            TierBlockSelectionPolicy::Shared => (0..state.tiers.len()).collect(),
            TierBlockSelectionPolicy::ContinuousRbEb
            | TierBlockSelectionPolicy::ContinuousRbEbFallback => state
                .tiers
                .iter()
                .enumerate()
                .filter_map(|(index, tier)| {
                    (tier.lane == tier_lane_for_block_kind(block_kind)).then_some(index)
                })
                .collect(),
            TierBlockSelectionPolicy::NaiveRbEbTwoTier => Vec::new(),
            TierBlockSelectionPolicy::RbTier0Reserved => {
                if block_kind == BlockKind::EndorserBlock {
                    (1..state.tiers.len()).collect()
                } else {
                    Vec::new()
                }
            }
        };
        if active_indices.len() <= 1 {
            return;
        }

        for boundary_index in 0..active_indices.len().saturating_sub(1) {
            let faster_index = active_indices[boundary_index];
            let slower_index = active_indices[boundary_index + 1];
            let cap_ratio = config.boundary_delay_increase_threshold(boundary_index);
            let faster_price = state.tiers[faster_index].price;
            let cap_price = ((cap_ratio * faster_price as f64).floor() as u64).max(1);
            if state.tiers[slower_index].price > cap_price {
                state.tiers[slower_index].price = cap_price;
                state.tiers[slower_index].version_created_slot = slot;
            }
        }
    }

    pub fn select_transactions(
        &self,
        txs: &[Arc<Transaction>],
        _slot: u64,
        block_capacity: u64,
        block_kind: BlockKind,
    ) -> Vec<Arc<Transaction>> {
        if block_kind == BlockKind::EndorserBlock && self.eb_state.is_some() {
            return self.select_transactions_from_pool(
                txs,
                block_capacity,
                block_kind,
                self.eb_state.as_ref().unwrap(),
                &self.eb_config.as_ref().unwrap(),
                true,
                true,
            );
        }
        self.select_transactions_from_pool(
            txs,
            block_capacity,
            block_kind,
            &self.state,
            &self.config,
            false,
            false,
        )
    }

    fn select_transactions_from_pool(
        &self,
        txs: &[Arc<Transaction>],
        block_capacity: u64,
        block_kind: BlockKind,
        state: &TieredState,
        config: &TieredConfig,
        has_separate_eb_pool: bool,
        use_eb_preference: bool,
    ) -> Vec<Arc<Transaction>> {
        let tier_count = state.tiers.len();
        if tier_count == 0 {
            return Vec::new();
        }
        let shared_scale = if config.total_capacity == 0 {
            0.0
        } else {
            block_capacity as f64 / config.total_capacity as f64
        };

        let mut index_by_id = std::collections::BTreeMap::new();
        for (index, tier) in state.tiers.iter().enumerate() {
            index_by_id.insert(tier.id, index);
        }

        let mut buckets: Vec<Vec<Arc<Transaction>>> = vec![Vec::new(); tier_count];
        for tx in txs {
            match config.assignment_semantics {
                TierAssignmentSemantics::NeverStale => {
                    let Some(assignment) = Self::lane_assignment_for_inclusion(
                        tx,
                        has_separate_eb_pool,
                        use_eb_preference,
                    ) else {
                        continue;
                    };
                    let Some(index) = Self::resolve_inclusion_tier_index(
                        state,
                        &index_by_id,
                        assignment.tier_id,
                        assignment.delay_slots,
                        assignment.price_per_byte_at_assignment,
                    ) else {
                        continue;
                    };
                    buckets[index].push(tx.clone());
                }
                TierAssignmentSemantics::LegacyRevalidateCurrentTier => {
                    let Some(assignment) = Self::lane_assignment_for_legacy_inclusion(
                        tx,
                        has_separate_eb_pool,
                        use_eb_preference,
                    ) else {
                        continue;
                    };
                    let Some(index) = index_by_id.get(&assignment.tier_id).copied() else {
                        continue;
                    };
                    let current_required_fee = state.tiers[index].price.saturating_mul(tx.bytes);
                    if assignment.posted_fee < current_required_fee {
                        continue;
                    }
                    buckets[index].push(tx.clone());
                }
            }
        }

        let mut included = Vec::new();
        if config.priority_ordering {
            // Priority ordering: tier 0 gets first claim on full block capacity,
            // tier 1 gets remainder, etc. No per-tier capacity limits.
            let mut remaining_block = block_capacity;
            for (index, tier) in state.tiers.iter().enumerate() {
                if config.block_selection_policy.uses_continuous_lane_pricing()
                    && tier.lane != tier_lane_for_block_kind(block_kind)
                {
                    continue;
                }
                if !config.block_selection_policy.allows_tier(index, block_kind) {
                    continue;
                }
                let mut bucket = std::mem::take(&mut buckets[index]);
                bucket.sort_by(|a, b| compare_submission_order(a, b));
                for tx in bucket {
                    if tx.bytes <= remaining_block {
                        remaining_block = remaining_block.saturating_sub(tx.bytes);
                        included.push(tx);
                    }
                }
            }
        } else {
            let eb_capacity_sum = state
                .tiers
                .iter()
                .skip(1)
                .map(|candidate| candidate.capacity)
                .sum::<u64>();
            let base_effective_capacity = |_index: usize, tier: &Tier| -> u64 {
                match config.block_selection_policy {
                    TierBlockSelectionPolicy::Shared
                    | TierBlockSelectionPolicy::ContinuousRbEb
                    | TierBlockSelectionPolicy::ContinuousRbEbFallback => {
                        if self.eb_state.is_some() {
                            tier.capacity
                        } else {
                            (tier.capacity as f64 * shared_scale).round() as u64
                        }
                    }
                    TierBlockSelectionPolicy::NaiveRbEbTwoTier => block_capacity,
                    TierBlockSelectionPolicy::RbTier0Reserved => match block_kind {
                        BlockKind::RankingBlock => config.rb_reserved_capacity(block_capacity),
                        BlockKind::EndorserBlock => {
                            if eb_capacity_sum == 0 {
                                0
                            } else {
                                ((tier.capacity as f64 / eb_capacity_sum as f64)
                                    * block_capacity as f64)
                                    .round() as u64
                            }
                        }
                    },
                }
            };
            let soft_reserve = if matches!(
                config.block_selection_policy,
                TierBlockSelectionPolicy::ContinuousRbEb
                    | TierBlockSelectionPolicy::ContinuousRbEbFallback
            ) && block_kind == BlockKind::RankingBlock
            {
                config.rb_soft_reserved_capacity(block_capacity)
            } else {
                0
            };
            if soft_reserve > 0 {
                let mut remaining_block = block_capacity;
                let mut spill = 0u64;
                if let Some((tier0_index, tier0)) =
                    state.tiers.iter().enumerate().find(|(_, tier)| {
                        tier.lane == TierLane::Ranking && tier.id == TierId::new(0)
                    })
                {
                    if config
                        .block_selection_policy
                        .allows_tier(tier0_index, block_kind)
                    {
                        let mut bucket = std::mem::take(&mut buckets[tier0_index]);
                        bucket.sort_by(|a, b| compare_submission_order(a, b));
                        let mut remaining = base_effective_capacity(tier0_index, tier0)
                            .max(soft_reserve)
                            .min(remaining_block);
                        let mut used = 0u64;
                        for tx in bucket {
                            if tx.bytes <= remaining && tx.bytes <= remaining_block {
                                remaining = remaining.saturating_sub(tx.bytes);
                                remaining_block = remaining_block.saturating_sub(tx.bytes);
                                used = used.saturating_add(tx.bytes);
                                included.push(tx);
                            }
                        }
                        spill = soft_reserve.saturating_sub(used);
                    }
                }

                for (index, tier) in state.tiers.iter().enumerate() {
                    if remaining_block == 0 {
                        break;
                    }
                    if tier.lane == TierLane::Ranking && tier.id == TierId::new(0) {
                        continue;
                    }
                    if config.block_selection_policy.uses_continuous_lane_pricing()
                        && tier.lane != tier_lane_for_block_kind(block_kind)
                    {
                        continue;
                    }
                    if !config.block_selection_policy.allows_tier(index, block_kind) {
                        continue;
                    }
                    let mut bucket = std::mem::take(&mut buckets[index]);
                    bucket.sort_by(|a, b| compare_submission_order(a, b));
                    let mut remaining = base_effective_capacity(index, tier);
                    if tier.lane == TierLane::Ranking {
                        remaining = remaining.saturating_add(spill);
                    }
                    remaining = remaining.min(remaining_block);
                    for tx in bucket {
                        if tx.bytes <= remaining && tx.bytes <= remaining_block {
                            remaining = remaining.saturating_sub(tx.bytes);
                            remaining_block = remaining_block.saturating_sub(tx.bytes);
                            included.push(tx);
                        }
                    }
                }
                return included;
            }

            let mut remaining_block = block_capacity;
            for (index, tier) in state.tiers.iter().enumerate() {
                if remaining_block == 0 {
                    break;
                }
                if config.block_selection_policy.uses_continuous_lane_pricing()
                    && tier.lane != tier_lane_for_block_kind(block_kind)
                {
                    continue;
                }
                if !config.block_selection_policy.allows_tier(index, block_kind) {
                    continue;
                }
                let mut bucket = std::mem::take(&mut buckets[index]);
                bucket.sort_by(|a, b| compare_submission_order(a, b));
                let mut remaining = base_effective_capacity(index, tier).min(remaining_block);
                for tx in bucket {
                    if tx.bytes <= remaining && tx.bytes <= remaining_block {
                        remaining = remaining.saturating_sub(tx.bytes);
                        remaining_block = remaining_block.saturating_sub(tx.bytes);
                        included.push(tx);
                    }
                }
            }
        }
        included
    }

    fn record_block_usage_static(
        txs: &[Arc<Transaction>],
        block_capacity: u64,
        block_kind: BlockKind,
        state: &mut TieredState,
        config: &TieredConfig,
        use_eb_preference: bool,
        slot: u64,
    ) -> Vec<f64> {
        let tier_count = state.tiers.len();
        let mut used_bytes = vec![0u64; tier_count];
        let mut tx_counts = vec![0u64; tier_count];
        let mut index_by_id = std::collections::BTreeMap::new();
        for (index, tier) in state.tiers.iter().enumerate() {
            index_by_id.insert(tier.id, index);
        }

        for tx in txs {
            let Some(assignment) = Self::lane_assignment_for_inclusion(
                tx,
                config.eb_total_capacity.is_some(),
                use_eb_preference,
            ) else {
                continue;
            };
            let tier_id = assignment.tier_id;
            let Some(index) = index_by_id.get(&tier_id).copied() else {
                continue;
            };
            used_bytes[index] = used_bytes[index].saturating_add(tx.bytes);
            tx_counts[index] = tx_counts[index].saturating_add(1);
        }

        let shared_scale = if config.total_capacity == 0 {
            0.0
        } else {
            block_capacity as f64 / config.total_capacity as f64
        };
        let eb_capacity_sum = state
            .tiers
            .iter()
            .skip(1)
            .map(|candidate| candidate.capacity)
            .sum::<u64>();

        let has_separate_eb = config.eb_total_capacity.is_some();
        let mut fill_rates = Vec::with_capacity(tier_count);
        if config.priority_ordering {
            // Priority ordering: each tier's fill rate = bytes_in_tier / block_capacity.
            // This prevents price spirals from tier 0 exceeding its nominal capacity fraction.
            let denominator = if has_separate_eb {
                config.total_capacity as f64
            } else {
                (config.total_capacity as f64 * shared_scale).max(1.0)
            };
            for (index, tier) in state.tiers.iter_mut().enumerate() {
                tier.used_capacity = used_bytes[index];
                tier.tx_count = tx_counts[index];
                fill_rates.push(used_bytes[index] as f64 / denominator);
            }
        } else {
            for (index, tier) in state.tiers.iter_mut().enumerate() {
                tier.used_capacity = used_bytes[index];
                tier.tx_count = tx_counts[index];

                let effective_capacity = match config.block_selection_policy {
                    TierBlockSelectionPolicy::Shared => {
                        if has_separate_eb {
                            tier.capacity as f64
                        } else {
                            (tier.capacity as f64 * shared_scale).round()
                        }
                    }
                    TierBlockSelectionPolicy::NaiveRbEbTwoTier => {
                        if config.block_selection_policy.allows_tier(index, block_kind) {
                            block_capacity as f64
                        } else {
                            0.0
                        }
                    }
                    TierBlockSelectionPolicy::RbTier0Reserved => match block_kind {
                        BlockKind::RankingBlock => {
                            config.rb_reserved_capacity(block_capacity) as f64
                        }
                        BlockKind::EndorserBlock => {
                            if eb_capacity_sum == 0 {
                                0.0
                            } else {
                                tier.capacity as f64 / eb_capacity_sum as f64
                                    * block_capacity as f64
                            }
                        }
                    },
                    TierBlockSelectionPolicy::ContinuousRbEb
                    | TierBlockSelectionPolicy::ContinuousRbEbFallback => {
                        if tier.lane == tier_lane_for_block_kind(block_kind) {
                            if has_separate_eb {
                                tier.capacity as f64
                            } else {
                                (tier.capacity as f64 * shared_scale).round()
                            }
                        } else {
                            0.0
                        }
                    }
                };
                if effective_capacity <= 0.0 {
                    fill_rates.push(0.0);
                } else {
                    fill_rates.push(used_bytes[index] as f64 / effective_capacity);
                }
            }
        }

        // For shared single-pool with EMA enabled: smooth the fill rates using
        // a moving average of throughput per slot, rather than per-block fill rates.
        if config.throughput_ema_enabled && !has_separate_eb {
            let total_included: u64 = used_bytes.iter().sum();
            let slots_elapsed = slot.saturating_sub(state.ema_last_slot).max(1) as f64;
            let current_throughput = total_included as f64 / slots_elapsed;

            let alpha = config.throughput_ema_alpha.clamp(0.0, 1.0);
            state.ema_throughput_per_slot =
                alpha * current_throughput + (1.0 - alpha) * state.ema_throughput_per_slot;
            state.ema_last_slot = slot;

            // Target throughput: total_capacity spread over expected block interval.
            // For shared pool, total_capacity is the RB size, but EBs also contribute.
            // Use the fill rate as: ema_throughput / (total_capacity / shared_scale).
            // shared_scale = block_capacity / total_capacity, so:
            // target = total_capacity * shared_scale = block_capacity
            // But we want per-slot, and blocks arrive at varying intervals.
            // Simpler: use total_capacity as the target per block, scale by shared_scale.
            let target_per_slot = if shared_scale > 0.0 {
                (config.total_capacity as f64 * shared_scale) / slots_elapsed
            } else {
                config.total_capacity as f64
            };

            if target_per_slot > 0.0 {
                let smoothed_fill = state.ema_throughput_per_slot / target_per_slot;
                for rate in fill_rates.iter_mut() {
                    *rate = smoothed_fill;
                }
            }
        }

        state.last_utilisations = fill_rates.clone();
        fill_rates
    }
}

#[derive(Debug, Clone)]
pub struct TieredState {
    block_count: u64,
    eb_block_count: u64,
    last_delay_update_slot: Option<u64>,
    last_tier_update_slot: Option<u64>,
    tiers: Vec<Tier>,
    last_utilisations: Vec<f64>,
    /// Exponential moving average of throughput (bytes per slot) for shared single-pool.
    ema_throughput_per_slot: f64,
    /// Slot of the last EMA update.
    ema_last_slot: u64,
}

impl TieredState {
    pub fn new(config: &TieredConfig, default_lane: TierLane) -> Self {
        let initial_price = config.new_tier_price.max(1);
        let initial_delay = config.initial_tier_delay;
        let tiers = match config.block_selection_policy {
            TierBlockSelectionPolicy::NaiveRbEbTwoTier => {
                let tier1_capacity = config.tier_capacity(1).min(config.total_capacity);
                let tier0_capacity = config.total_capacity.saturating_sub(tier1_capacity);
                let tier0_delay = initial_delay;
                // In naive RB/EB mode, delay is represented by the block path (RB vs EB),
                // not by synthetic per-tier delay multipliers.
                let tier1_delay = tier0_delay;
                vec![
                    Tier {
                        id: TierId::new(0),
                        lane: TierLane::Ranking,
                        capacity: tier0_capacity,
                        version_created_slot: 0,
                        delay: tier0_delay,
                        price: initial_price,
                        used_capacity: 0,
                        tx_count: 0,
                    },
                    Tier {
                        id: TierId::new(1),
                        lane: TierLane::Endorser,
                        capacity: tier1_capacity,
                        version_created_slot: 0,
                        delay: tier1_delay.max(1),
                        price: initial_price,
                        used_capacity: 0,
                        tx_count: 0,
                    },
                ]
            }
            TierBlockSelectionPolicy::RbTier0Reserved => {
                let tier0_capacity = (config.total_capacity as f64
                    * config.rb_tier0_reservation_fraction)
                    .round() as u64;
                let tier0_capacity = tier0_capacity.min(config.total_capacity);
                // RB reservation targets tier 0 in RB blocks.
                // EB-side tier capacities are managed independently.
                let tier1_capacity = config.total_capacity;
                vec![
                    Tier {
                        id: TierId::new(0),
                        lane: TierLane::Ranking,
                        capacity: tier0_capacity,
                        version_created_slot: 0,
                        delay: initial_delay,
                        price: initial_price,
                        used_capacity: 0,
                        tx_count: 0,
                    },
                    Tier {
                        id: TierId::new(1),
                        lane: TierLane::Endorser,
                        capacity: tier1_capacity,
                        version_created_slot: 0,
                        delay: initial_delay,
                        price: initial_price,
                        used_capacity: 0,
                        tx_count: 0,
                    },
                ]
            }
            TierBlockSelectionPolicy::Shared => {
                vec![Tier {
                    id: TierId::new(0),
                    lane: default_lane,
                    capacity: config.total_capacity,
                    version_created_slot: 0,
                    delay: initial_delay,
                    price: initial_price,
                    used_capacity: 0,
                    tx_count: 0,
                }]
            }
            TierBlockSelectionPolicy::ContinuousRbEb
            | TierBlockSelectionPolicy::ContinuousRbEbFallback => {
                let first_tier_id = match default_lane {
                    TierLane::Ranking => 0,
                    TierLane::Endorser => 1,
                };
                vec![Tier {
                    id: TierId::new(first_tier_id),
                    lane: default_lane,
                    capacity: config.total_capacity,
                    version_created_slot: 0,
                    delay: initial_delay,
                    price: initial_price,
                    used_capacity: 0,
                    tx_count: 0,
                }]
            }
        };
        Self {
            block_count: 0,
            eb_block_count: 0,
            last_delay_update_slot: None,
            last_tier_update_slot: None,
            tiers,
            last_utilisations: Vec::new(),
            ema_throughput_per_slot: 0.0,
            ema_last_slot: 0,
        }
    }

    pub fn update_with_fill_rates(
        &mut self,
        config: &TieredConfig,
        rng: &mut ChaChaRng,
        fill_rates: Vec<f64>,
        tier_update_signal_fill_rates: Option<Vec<f64>>,
        block_kind: BlockKind,
        slot: u64,
    ) -> TierCadenceUpdate {
        for (index, (tier, fill_rate)) in self
            .tiers
            .iter_mut()
            .zip(fill_rates.iter().copied())
            .enumerate()
        {
            if config.block_selection_policy.uses_continuous_lane_pricing()
                && tier.lane != tier_lane_for_block_kind(block_kind)
            {
                continue;
            }
            if config.block_selection_policy.is_lane_partitioned()
                && !config.block_selection_policy.allows_tier(index, block_kind)
            {
                // Naive RB/EB policy has one dedicated lane per block kind.
                // Do not reprice the inactive lane from this block's zero fill.
                continue;
            }
            let updated_price = update_eip1559_price(
                tier.price,
                fill_rate,
                config.target_utilisation_for_lane(tier.lane),
                config.base_fee_change_denominator_for_lane(tier.lane),
            );
            if updated_price != tier.price {
                tier.version_created_slot = slot;
            }
            tier.price = updated_price;
        }

        self.block_count = self.block_count.saturating_add(1);
        if block_kind == BlockKind::EndorserBlock {
            self.eb_block_count = self.eb_block_count.saturating_add(1);
        }

        let should_update_delays = match config.block_selection_policy {
            TierBlockSelectionPolicy::Shared
            | TierBlockSelectionPolicy::ContinuousRbEb
            | TierBlockSelectionPolicy::ContinuousRbEbFallback => Self::is_cadence_due(
                slot,
                self.last_delay_update_slot,
                config.delay_update_period_slots,
                self.block_count,
                config.delay_update_frequency,
            ),
            TierBlockSelectionPolicy::NaiveRbEbTwoTier => false,
            TierBlockSelectionPolicy::RbTier0Reserved => {
                block_kind == BlockKind::EndorserBlock
                    && Self::is_cadence_due(
                        slot,
                        self.last_delay_update_slot,
                        config.delay_update_period_slots,
                        self.eb_block_count,
                        config.delay_update_frequency,
                    )
            }
        };
        if should_update_delays {
            match config.block_selection_policy {
                TierBlockSelectionPolicy::Shared
                | TierBlockSelectionPolicy::ContinuousRbEb
                | TierBlockSelectionPolicy::ContinuousRbEbFallback => {
                    update_delays(&mut self.tiers, config, rng);
                }
                TierBlockSelectionPolicy::NaiveRbEbTwoTier => {}
                TierBlockSelectionPolicy::RbTier0Reserved => {
                    update_delays_reserved(&mut self.tiers, config, rng);
                }
            }
            self.last_delay_update_slot = Some(slot);
        }

        let should_update_tiers = match config.block_selection_policy {
            TierBlockSelectionPolicy::Shared
            | TierBlockSelectionPolicy::ContinuousRbEb
            | TierBlockSelectionPolicy::ContinuousRbEbFallback => Self::is_cadence_due(
                slot,
                self.last_tier_update_slot,
                config.tier_update_period_slots,
                self.block_count,
                config.tier_update_frequency,
            ),
            TierBlockSelectionPolicy::NaiveRbEbTwoTier => false,
            TierBlockSelectionPolicy::RbTier0Reserved => {
                block_kind == BlockKind::EndorserBlock
                    && Self::is_cadence_due(
                        slot,
                        self.last_tier_update_slot,
                        config.tier_update_period_slots,
                        self.eb_block_count,
                        config.tier_update_frequency,
                    )
            }
        };
        if should_update_tiers {
            let last_tier_signal_fill_rate = tier_update_signal_fill_rates
                .as_ref()
                .and_then(|rates| rates.last().copied());
            match config.block_selection_policy {
                TierBlockSelectionPolicy::Shared
                | TierBlockSelectionPolicy::ContinuousRbEb
                | TierBlockSelectionPolicy::ContinuousRbEbFallback => {
                    update_tier_sizes(&mut self.tiers, config, slot, last_tier_signal_fill_rate);
                }
                TierBlockSelectionPolicy::NaiveRbEbTwoTier => {}
                TierBlockSelectionPolicy::RbTier0Reserved => {
                    update_tier_sizes_reserved(
                        &mut self.tiers,
                        config,
                        slot,
                        last_tier_signal_fill_rate,
                    );
                }
            }
            if config.dynamic_tier_sizing_enabled {
                let signal_fill_rates = tier_update_signal_fill_rates
                    .as_ref()
                    .unwrap_or(&fill_rates);
                match config.block_selection_policy {
                    TierBlockSelectionPolicy::Shared
                    | TierBlockSelectionPolicy::ContinuousRbEb
                    | TierBlockSelectionPolicy::ContinuousRbEbFallback => {
                        apply_dynamic_sizing(
                            &mut self.tiers,
                            signal_fill_rates,
                            config.dynamic_tier_sizing_min_fraction,
                            config.dynamic_tier_sizing_alpha,
                        );
                    }
                    TierBlockSelectionPolicy::NaiveRbEbTwoTier => {}
                    TierBlockSelectionPolicy::RbTier0Reserved => {
                        // Preserve tier-0 RB reservation; dynamically rebalance EB tiers only.
                        if self.tiers.len() > 1 {
                            apply_dynamic_sizing_subset(
                                &mut self.tiers,
                                signal_fill_rates,
                                1,
                                config.dynamic_tier_sizing_min_fraction,
                                config.dynamic_tier_sizing_alpha,
                            );
                        }
                    }
                }
            }
            self.last_tier_update_slot = Some(slot);
        }

        self.last_utilisations = fill_rates;

        for tier in &mut self.tiers {
            tier.used_capacity = 0;
            tier.tx_count = 0;
        }

        TierCadenceUpdate {
            delay_update_triggered: should_update_delays,
            tier_update_triggered: should_update_tiers,
        }
    }

    fn is_cadence_due(
        slot: u64,
        last_update_slot: Option<u64>,
        period_slots: Option<u64>,
        block_counter: u64,
        frequency: Option<u64>,
    ) -> bool {
        if let Some(period_slots) = period_slots {
            match last_update_slot {
                Some(previous) => slot.saturating_sub(previous) >= period_slots,
                None => slot >= period_slots,
            }
        } else if let Some(frequency) = frequency {
            block_counter % frequency == 0
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct Tier {
    pub id: TierId,
    pub lane: TierLane,
    pub capacity: u64,
    /// Slot when this tier version became active for submissions.
    pub version_created_slot: u64,
    pub delay: u64,
    pub price: u64,
    pub used_capacity: u64,
    pub tx_count: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TierCadenceUpdate {
    pub delay_update_triggered: bool,
    pub tier_update_triggered: bool,
}

pub fn update_eip1559_price(
    current_price: u64,
    fill_rate: f64,
    target: f64,
    denominator: u64,
) -> u64 {
    if current_price == 0 {
        return 1;
    }
    if target <= 0.0 || denominator == 0 {
        return current_price;
    }

    let delta_fraction = (fill_rate - target) / target / denominator as f64;
    let delta = (current_price as f64 * delta_fraction.abs()) as u64;

    if fill_rate > target {
        current_price.saturating_add(delta.max(1))
    } else if fill_rate < target {
        current_price.saturating_sub(delta).max(1)
    } else {
        current_price
    }
}

fn utilisation(bytes: u64, capacity: u64) -> f64 {
    if capacity == 0 {
        0.0
    } else {
        bytes as f64 / capacity as f64
    }
}

pub fn update_delays(tiers: &mut [Tier], config: &TieredConfig, rng: &mut ChaChaRng) {
    let count = tiers.len();
    if count <= 1 {
        return;
    }

    for i in 1..count {
        let boundary_index = i - 1;
        let prev_price = tiers[i - 1].price as f64;
        let current_price = tiers[i].price as f64;
        let threshold = config.boundary_delay_increase_threshold(boundary_index);

        if prev_price > 0.0 && current_price > threshold * prev_price {
            tiers[i].delay = tiers[i].delay.saturating_add(1);
        } else if should_decrease_delay(config.delay_decrease_prob, rng) {
            tiers[i].delay = tiers[i].delay.saturating_sub(1);
        }

        let min_ratio = config.boundary_min_delay_ratio(boundary_index);
        let min_delay = (min_ratio * tiers[i - 1].delay as f64).ceil() as u64;
        tiers[i].delay = tiers[i].delay.max(min_delay);
    }
}

/// Rebalance tier capacities so that active tiers share non-reserved capacity
/// proportionally according to `tier_size_fractions`. When there are fewer tiers
/// than max, earlier tiers get a larger share and gradually shrink as tiers are added.
///
/// `reservoir_index` is the tier whose capacity acts as the reservoir (typically 0).
/// `fixed_indices` are tiers whose capacity must not change (e.g. tier 0 in RbTier0Reserved).
fn rebalance_tier_capacities(
    tiers: &mut [Tier],
    config: &TieredConfig,
    reservoir_index: usize,
    fixed_indices: &[usize],
) {
    if tiers.len() <= 1 {
        return;
    }
    let active_indices: Vec<usize> = (0..tiers.len())
        .filter(|i| *i != reservoir_index && !fixed_indices.contains(i))
        .collect();
    if active_indices.is_empty() {
        return;
    }

    let sum_active_fractions: f64 = active_indices
        .iter()
        .map(|&i| {
            config
                .tier_size_fractions
                .get(i)
                .copied()
                .unwrap_or(0.0)
                .max(0.0)
        })
        .sum();
    if sum_active_fractions <= 0.0 {
        return;
    }

    let fixed_capacity: u64 = fixed_indices.iter().map(|&i| tiers[i].capacity).sum();
    let distributable = config.total_capacity.saturating_sub(fixed_capacity);

    let alpha = config.tier_rebalance_alpha.clamp(0.0, 1.0);
    let mut assigned = 0u64;
    for (pos, &i) in active_indices.iter().enumerate() {
        let fraction = config
            .tier_size_fractions
            .get(i)
            .copied()
            .unwrap_or(0.0)
            .max(0.0);
        let target = if pos == active_indices.len() - 1 {
            distributable.saturating_sub(assigned)
        } else {
            ((fraction / sum_active_fractions) * distributable as f64).round() as u64
        };
        let old = tiers[i].capacity;
        let blended = if alpha >= 1.0 {
            target
        } else {
            (old as f64 + alpha * (target as f64 - old as f64)).round() as u64
        };
        tiers[i].capacity = blended;
        assigned = assigned.saturating_add(blended);
    }

    tiers[reservoir_index].capacity = distributable.saturating_sub(assigned);
}

pub fn update_tier_sizes(
    tiers: &mut Vec<Tier>,
    config: &TieredConfig,
    slot: u64,
    last_tier_fill_rate: Option<f64>,
) {
    let count = tiers.len();
    if count == 0 {
        return;
    }
    let last_price = tiers[count - 1].price;
    let last_fill_rate = last_tier_fill_rate.unwrap_or(0.0);
    let should_remove = if config.include_overflow_aggregate_in_tier_updates {
        count > 1 && last_fill_rate < config.remove_tier_fill_rate_threshold
    } else {
        count > 1 && last_price < config.remove_tier_threshold
    };

    if should_remove {
        tiers.pop().expect("last tier exists");
        rebalance_tier_capacities(tiers, config, 0, &[]);
        return;
    }

    let should_add = if config.include_overflow_aggregate_in_tier_updates {
        count < config.max_tiers && last_fill_rate > config.add_tier_fill_rate_threshold
    } else {
        count < config.max_tiers && last_price > config.add_tier_threshold
    };

    if should_add {
        let new_tier_count = count + 1;
        let new_delay = match config.tier_delay_spacing {
            TierDelaySpacing::Incremental => {
                let previous_delay = tiers[count - 1].delay;
                let boundary_index = count - 1;
                let new_delay_ratio = config.boundary_new_tier_delay_ratio(boundary_index);
                (new_delay_ratio * previous_delay as f64).ceil() as u64
            }
            TierDelaySpacing::GeometricFixedMax => {
                config.geometric_delay_for_tier(count, new_tier_count)
            }
        };
        let new_id = next_tier_id(tiers);
        // Push with placeholder capacity; rebalance will set the real value.
        tiers.push(Tier {
            id: new_id,
            lane: tiers[count - 1].lane,
            capacity: 0,
            version_created_slot: slot,
            delay: new_delay.max(1),
            price: config.new_tier_price.max(1),
            used_capacity: 0,
            tx_count: 0,
        });

        // Under geometric_fixed_max, adding a tier changes the delay schedule
        // for ALL existing tiers. Recompute delays for earlier tiers.
        if config.tier_delay_spacing == TierDelaySpacing::GeometricFixedMax {
            for i in 0..tiers.len() {
                tiers[i].delay = config.geometric_delay_for_tier(i, new_tier_count);
            }
        }
    }

    // Gradually rebalance toward target capacities on every tier-update cadence,
    // not just when tiers are added/removed. This allows gradual convergence
    // when tier_rebalance_alpha < 1.0.
    rebalance_tier_capacities(tiers, config, 0, &[]);
}

fn update_delays_reserved(tiers: &mut [Tier], config: &TieredConfig, rng: &mut ChaChaRng) {
    let count = tiers.len();
    // tier 0 is the RB lane, tier 1 is the base EB lane; only EB tiers above 1 have relative delays.
    if count <= 2 {
        return;
    }

    for i in 2..count {
        let boundary_index = i - 2;
        let prev_price = tiers[i - 1].price as f64;
        let current_price = tiers[i].price as f64;
        let threshold = config.boundary_delay_increase_threshold(boundary_index);

        if prev_price > 0.0 && current_price > threshold * prev_price {
            tiers[i].delay = tiers[i].delay.saturating_add(1);
        } else if should_decrease_delay(config.delay_decrease_prob, rng) {
            tiers[i].delay = tiers[i].delay.saturating_sub(1);
        }

        let min_ratio = config.boundary_min_delay_ratio(boundary_index);
        let min_delay = (min_ratio * tiers[i - 1].delay as f64).ceil() as u64;
        tiers[i].delay = tiers[i].delay.max(min_delay);
    }
}

fn update_tier_sizes_reserved(
    tiers: &mut Vec<Tier>,
    config: &TieredConfig,
    slot: u64,
    last_tier_fill_rate: Option<f64>,
) {
    let count = tiers.len();
    if count <= 1 {
        return;
    }
    let eb_count = count - 1;
    let last_price = tiers[count - 1].price;
    let last_fill_rate = last_tier_fill_rate.unwrap_or(0.0);
    let should_remove = if config.include_overflow_aggregate_in_tier_updates {
        eb_count > 1 && last_fill_rate < config.remove_tier_fill_rate_threshold
    } else {
        eb_count > 1 && last_price < config.remove_tier_threshold
    };

    // Keep at least one EB tier (tier 1).
    // Tier 0 is the reserved RB lane; tier 1 is the EB reservoir.
    if should_remove {
        tiers.pop().expect("last tier exists");
        rebalance_tier_capacities(tiers, config, 1, &[0]);
        return;
    }

    let should_add = if config.include_overflow_aggregate_in_tier_updates {
        count < config.max_tiers && last_fill_rate > config.add_tier_fill_rate_threshold
    } else {
        count < config.max_tiers && last_price > config.add_tier_threshold
    };

    if should_add {
        let new_tier_count = count + 1;
        let new_delay = match config.tier_delay_spacing {
            TierDelaySpacing::Incremental => {
                let previous_delay = tiers[count - 1].delay;
                let boundary_index = eb_count.saturating_sub(1);
                let new_delay_ratio = config.boundary_new_tier_delay_ratio(boundary_index);
                (new_delay_ratio * previous_delay as f64).ceil() as u64
            }
            TierDelaySpacing::GeometricFixedMax => {
                config.geometric_delay_for_tier(count, new_tier_count)
            }
        };
        let new_id = next_tier_id(tiers);
        tiers.push(Tier {
            id: new_id,
            lane: tiers[count - 1].lane,
            capacity: 0,
            version_created_slot: slot,
            delay: new_delay.max(1),
            price: config.new_tier_price.max(1),
            used_capacity: 0,
            tx_count: 0,
        });

        if config.tier_delay_spacing == TierDelaySpacing::GeometricFixedMax {
            // Tier 0 is fixed RB reservation — skip it. Recompute EB tiers (1+).
            for i in 1..tiers.len() {
                tiers[i].delay = config.geometric_delay_for_tier(i - 1, new_tier_count - 1);
            }
        }
    }

    // Gradually rebalance toward target capacities on every tier-update cadence.
    rebalance_tier_capacities(tiers, config, 1, &[0]);
}

fn apply_dynamic_sizing(
    tiers: &mut [Tier],
    signal_fill_rates: &[f64],
    min_fraction: f64,
    alpha: f64,
) {
    apply_dynamic_sizing_subset(tiers, signal_fill_rates, 0, min_fraction, alpha);
}

fn apply_dynamic_sizing_subset(
    tiers: &mut [Tier],
    signal_fill_rates: &[f64],
    start_index: usize,
    min_fraction: f64,
    alpha: f64,
) {
    if !alpha.is_finite() || alpha <= 0.0 || start_index >= tiers.len() {
        return;
    }

    let active_count = tiers.len() - start_index;
    if active_count <= 1 {
        return;
    }

    let current_caps: Vec<u64> = tiers[start_index..]
        .iter()
        .map(|tier| tier.capacity)
        .collect();
    let total_capacity: u64 = current_caps.iter().copied().sum();
    if total_capacity == 0 {
        return;
    }

    let mut demand_weights = Vec::with_capacity(active_count);
    for (offset, cap) in current_caps.iter().copied().enumerate() {
        let fill = signal_fill_rates
            .get(start_index + offset)
            .copied()
            .unwrap_or(0.0)
            .max(0.0);
        demand_weights.push(fill * cap as f64);
    }
    let total_weight: f64 = demand_weights.iter().copied().sum();
    if !total_weight.is_finite() || total_weight <= 0.0 {
        return;
    }

    let per_tier_cap_limit = total_capacity / active_count as u64;
    let mut min_capacity = (total_capacity as f64 * min_fraction.max(0.0)).floor() as u64;
    min_capacity = min_capacity.min(per_tier_cap_limit);
    let residual_capacity = total_capacity.saturating_sub(min_capacity * active_count as u64);

    let alpha = alpha.clamp(0.0, 1.0);
    let mut blended_targets = Vec::with_capacity(active_count);
    let mut new_caps = Vec::with_capacity(active_count);
    for (current, weight) in current_caps
        .iter()
        .copied()
        .zip(demand_weights.iter().copied())
    {
        let raw_target = min_capacity as f64 + residual_capacity as f64 * (weight / total_weight);
        let blended = current as f64 * (1.0 - alpha) + raw_target * alpha;
        let rounded = blended.round().max(min_capacity as f64) as u64;
        blended_targets.push(blended);
        new_caps.push(rounded);
    }

    let mut assigned: u64 = new_caps.iter().copied().sum();
    while assigned < total_capacity {
        let mut best_index = 0usize;
        let mut best_gap = f64::NEG_INFINITY;
        for (index, cap) in new_caps.iter().copied().enumerate() {
            let gap = blended_targets[index] - cap as f64;
            if gap > best_gap {
                best_gap = gap;
                best_index = index;
            }
        }
        new_caps[best_index] = new_caps[best_index].saturating_add(1);
        assigned = assigned.saturating_add(1);
    }
    while assigned > total_capacity {
        let mut best_index = None;
        let mut best_excess = f64::NEG_INFINITY;
        for (index, cap) in new_caps.iter().copied().enumerate() {
            if cap <= min_capacity {
                continue;
            }
            let excess = cap as f64 - blended_targets[index];
            if excess > best_excess {
                best_excess = excess;
                best_index = Some(index);
            }
        }
        let Some(index) = best_index else {
            break;
        };
        new_caps[index] = new_caps[index].saturating_sub(1);
        assigned = assigned.saturating_sub(1);
    }

    for (offset, capacity) in new_caps.into_iter().enumerate() {
        tiers[start_index + offset].capacity = capacity;
    }
}

fn should_decrease_delay<R: Rng + ?Sized>(prob: f64, rng: &mut R) -> bool {
    if prob <= 0.0 {
        return false;
    }
    if prob >= 1.0 {
        return true;
    }
    rng.random::<f64>() < prob
}

fn tier_id_to_usize(tier_id: TierId) -> usize {
    tier_id.to_string().parse::<usize>().unwrap_or_default()
}

fn next_tier_id(tiers: &[Tier]) -> TierId {
    let next = tiers
        .iter()
        .map(|tier| tier_id_to_usize(tier.id))
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    TierId::new(next)
}

fn select_single_tier(
    txs: &[Arc<Transaction>],
    snapshot: PricingSnapshot,
    slot: u64,
    block_capacity: u64,
) -> Vec<Arc<Transaction>> {
    select_single_tier_with_history(txs, snapshot, slot, block_capacity, None)
}

fn select_single_tier_with_history(
    txs: &[Arc<Transaction>],
    snapshot: PricingSnapshot,
    slot: u64,
    block_capacity: u64,
    fee_history: Option<&SingleTierFeeHistory>,
) -> Vec<Arc<Transaction>> {
    let Some(tier) = snapshot.tiers.first() else {
        return Vec::new();
    };
    let mut candidates: Vec<Arc<Transaction>> = txs
        .iter()
        .filter(|tx| tx.tier_preference == Some(tier.id))
        .filter(|tx| match tx.posted_fee {
            Some(posted_fee) => match fee_history {
                Some(history) => history.fee_satisfies(
                    tier.id,
                    tier.price_per_byte,
                    tx.submission_slot,
                    slot,
                    tx.bytes,
                    posted_fee,
                    0,
                ),
                None => posted_fee >= tier.required_fee(tx.bytes),
            },
            None => false,
        })
        .cloned()
        .collect();
    candidates.sort_by(|a, b| compare_submission_order(a, b));

    let mut included = Vec::new();
    let mut remaining = block_capacity;
    for tx in candidates {
        if tx.bytes <= remaining {
            remaining = remaining.saturating_sub(tx.bytes);
            included.push(tx);
        }
    }
    included
}

fn compare_submission_order(a: &Transaction, b: &Transaction) -> Ordering {
    a.submission_slot
        .cmp(&b.submission_slot)
        .then_with(|| a.id.cmp(&b.id))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use rand_chacha::{ChaChaRng, rand_core::SeedableRng};

    use crate::model::{
        ActorId, Transaction, TransactionId, TransactionRejectReason, UrgencyProfile,
    };

    use super::{
        BlockKind, Eip1559Pricing, Eip1559PriorityLanePricing, Eip1559SmoothingConfig,
        OverflowAggregatePricingMode, OverflowRetryPolicy, PricingSnapshot, Tier,
        TierAssignmentHistory, TierAssignmentSemantics, TierBlockSelectionPolicy, TierDelaySpacing,
        TierId, TierLane, TierQuote, TierSelectionDelayModel, TieredConfig, TieredPricing,
        TieredState, rebalance_tier_capacities, select_best_lane_tier_for_tx, select_tier_for_tx,
        update_delays, update_tier_sizes,
    };

    fn test_config() -> TieredConfig {
        TieredConfig {
            total_capacity: 100,
            max_tiers: 4,
            tier_size_fractions: vec![0.0, 0.2, 0.2, 0.2],
            base_fee_change_denominator: 8,
            target_utilisation: 0.5,
            delay_update_frequency: Some(1),
            delay_update_period_slots: None,
            delay_increase_threshold: 1.5,
            delay_increase_thresholds: vec![],
            delay_decrease_prob: 0.0,
            min_delay_ratio: 2.0,
            min_delay_ratios: vec![],
            tier_update_frequency: Some(10),
            tier_update_period_slots: None,
            add_tier_threshold: 5000,
            remove_tier_threshold: 100,
            new_tier_price: 1000,
            new_tier_delay_ratio: 2.0,
            block_selection_policy: TierBlockSelectionPolicy::Shared,
            rb_tier0_reservation_fraction: 1.0,
            rb_target_utilisation: None,
            rb_base_fee_change_denominator: None,
            rb_overflow_linear_price_per_fill: None,
            rb_soft_reservation_fraction: 0.0,
            separate_eb_pool: false,
            eb_total_capacity: None,
            assignment_semantics: TierAssignmentSemantics::NeverStale,
            reject_on_pending_tier_overflow: true,
            include_overflow_aggregate_in_pricing_updates: false,
            overflow_aggregate_pricing_mode: OverflowAggregatePricingMode::IncludeAsFillRate,
            overflow_linear_price_per_fill: 100,
            overflow_linear_fill_rate_cap: 1.0,
            dynamic_tier_sizing_enabled: false,
            dynamic_tier_sizing_alpha: 1.0,
            dynamic_tier_sizing_min_fraction: 0.02,
            enforce_boundary_price_caps: false,
            include_overflow_aggregate_in_tier_updates: false,
            add_tier_fill_rate_threshold: 1.0,
            remove_tier_fill_rate_threshold: 0.2,
            overflow_retry_policy: OverflowRetryPolicy::default(),
            lane_selection_backlog_delay_scale: 0.0,
            min_fee: 0,
            tier_rebalance_alpha: 1.0,
            initial_tier_delay: 1,
            tier_delay_spacing: TierDelaySpacing::Incremental,
            max_tier_delay: 200,
            throughput_ema_enabled: false,
            throughput_ema_alpha: 0.1,
            priority_ordering: false,
        }
    }

    #[test]
    fn overflow_retry_policy_default_is_valid_and_full_coverage() {
        let policy = OverflowRetryPolicy::default();
        assert_eq!(policy.validate(), Ok(()));
        assert!(policy.band_for_retained_ratio(0.0).is_some());
        assert!(policy.band_for_retained_ratio(0.5).is_some());
        assert!(policy.band_for_retained_ratio(1.0).is_some());
    }

    #[test]
    fn overflow_retry_policy_validation_rejects_coverage_gaps() {
        let mut policy = OverflowRetryPolicy::default();
        policy.bands = vec![
            super::OverflowRetryBand {
                min_retained_ratio: 0.0,
                max_retained_ratio: 0.5,
                max_attempts: 1,
                base_delay_slots: 1,
            },
            super::OverflowRetryBand {
                min_retained_ratio: 0.6,
                max_retained_ratio: 1.0,
                max_attempts: 1,
                base_delay_slots: 1,
            },
        ];
        assert!(policy.validate().is_err());
    }

    #[test]
    fn overflow_retry_policy_exponential_backoff_caps_delay() {
        let policy = OverflowRetryPolicy {
            max_delay_slots: 8,
            bands: vec![super::OverflowRetryBand {
                min_retained_ratio: 0.0,
                max_retained_ratio: 1.0,
                max_attempts: 8,
                base_delay_slots: 3,
            }],
            ..OverflowRetryPolicy::default()
        };
        let band = policy.band_for_retained_ratio(0.5).unwrap();
        assert_eq!(policy.retry_delay_slots(band, 0), 3);
        assert_eq!(policy.retry_delay_slots(band, 1), 6);
        assert_eq!(policy.retry_delay_slots(band, 2), 8);
        assert_eq!(policy.retry_delay_slots(band, 5), 8);
    }

    #[test]
    fn tiered_config_rejects_cadence_duplicates_and_zero_slot_periods() {
        let mut config = test_config();
        config.delay_update_frequency = None;
        config.tier_update_frequency = None;
        config.delay_update_period_slots = Some(0);
        assert_eq!(
            config.validate(),
            Err("delay_update_period_slots must be >= 1 when set".to_string())
        );

        config.delay_update_period_slots = Some(1);
        config.tier_update_period_slots = Some(0);
        assert_eq!(
            config.validate(),
            Err("tier_update_period_slots must be >= 1 when set".to_string())
        );

        config.tier_update_period_slots = Some(1);
        assert_eq!(config.validate(), Ok(()));

        config.delay_update_frequency = Some(1);
        assert_eq!(
            config.validate(),
            Err(
                "delay cadence duplicated: set only one of delay_update_frequency or delay_update_period_slots"
                    .to_string()
            )
        );
    }

    #[test]
    fn slot_based_cadence_triggers_at_slot_intervals() {
        let mut config = test_config();
        config.delay_update_frequency = None;
        config.tier_update_frequency = None;
        config.delay_update_period_slots = Some(5);
        config.tier_update_period_slots = Some(7);
        config.delay_decrease_prob = 0.0;

        let mut state = TieredState::new(&config, TierLane::Ranking);
        let mut rng = ChaChaRng::seed_from_u64(1);

        let cadence = state.update_with_fill_rates(
            &config,
            &mut rng,
            vec![0.0],
            None,
            BlockKind::RankingBlock,
            1,
        );
        assert!(!cadence.delay_update_triggered);
        assert!(!cadence.tier_update_triggered);

        let cadence = state.update_with_fill_rates(
            &config,
            &mut rng,
            vec![0.0],
            None,
            BlockKind::RankingBlock,
            4,
        );
        assert!(!cadence.delay_update_triggered);
        assert!(!cadence.tier_update_triggered);

        let cadence = state.update_with_fill_rates(
            &config,
            &mut rng,
            vec![0.0],
            None,
            BlockKind::RankingBlock,
            5,
        );
        assert!(cadence.delay_update_triggered);
        assert!(!cadence.tier_update_triggered);

        let cadence = state.update_with_fill_rates(
            &config,
            &mut rng,
            vec![0.0],
            None,
            BlockKind::RankingBlock,
            7,
        );
        assert!(!cadence.delay_update_triggered);
        assert!(cadence.tier_update_triggered);

        let cadence = state.update_with_fill_rates(
            &config,
            &mut rng,
            vec![0.0],
            None,
            BlockKind::RankingBlock,
            10,
        );
        assert!(cadence.delay_update_triggered);
        assert!(!cadence.tier_update_triggered);
    }

    #[test]
    fn block_count_cadence_fallback_is_unchanged_when_slot_mode_unset() {
        let mut config = test_config();
        config.delay_update_frequency = Some(2);
        config.tier_update_frequency = Some(3);
        config.delay_update_period_slots = None;
        config.tier_update_period_slots = None;
        config.delay_decrease_prob = 0.0;

        let mut state = TieredState::new(&config, TierLane::Ranking);
        let mut rng = ChaChaRng::seed_from_u64(2);

        let first = state.update_with_fill_rates(
            &config,
            &mut rng,
            vec![0.0],
            None,
            BlockKind::RankingBlock,
            100,
        );
        assert!(!first.delay_update_triggered);
        assert!(!first.tier_update_triggered);

        let second = state.update_with_fill_rates(
            &config,
            &mut rng,
            vec![0.0],
            None,
            BlockKind::RankingBlock,
            200,
        );
        assert!(second.delay_update_triggered);
        assert!(!second.tier_update_triggered);

        let third = state.update_with_fill_rates(
            &config,
            &mut rng,
            vec![0.0],
            None,
            BlockKind::RankingBlock,
            300,
        );
        assert!(!third.delay_update_triggered);
        assert!(third.tier_update_triggered);
    }

    #[test]
    fn overflow_pricing_mode_include_as_fill_rate_keeps_existing_behavior() {
        let mut config = test_config();
        config.include_overflow_aggregate_in_pricing_updates = true;
        config.overflow_aggregate_pricing_mode = OverflowAggregatePricingMode::IncludeAsFillRate;
        config.delay_update_frequency = Some(1000);
        config.tier_update_frequency = Some(1000);

        let mut pricing = TieredPricing::new(config, 11);
        let overflow_tx = test_tx(1, TierId::new(0), 100, 0);

        pricing.update_after_block_with_signals(
            &[],
            None,
            Some(std::slice::from_ref(&overflow_tx)),
            100,
            BlockKind::RankingBlock,
            1,
        );

        // Initial price 1000, overflow fill-rate 1.0, target 0.5, denom 8 => +12.5%.
        assert_eq!(pricing.state.tiers[0].price, 1125);
    }

    #[test]
    fn overflow_pricing_mode_linear_additive_is_bounded_and_linear() {
        let mut config = test_config();
        config.include_overflow_aggregate_in_pricing_updates = true;
        config.overflow_aggregate_pricing_mode = OverflowAggregatePricingMode::LinearAdditive;
        config.overflow_linear_price_per_fill = 100;
        config.overflow_linear_fill_rate_cap = 1.0;
        config.delay_update_frequency = Some(1000);
        config.tier_update_frequency = Some(1000);

        let mut pricing = TieredPricing::new(config, 12);
        let overflow_tx = test_tx(1, TierId::new(0), 100, 0);

        pricing.update_after_block_with_signals(
            &[],
            None,
            Some(std::slice::from_ref(&overflow_tx)),
            100,
            BlockKind::RankingBlock,
            1,
        );

        // Base EIP update with included fill-rate 0.0 lowers 1000 -> 875.
        // Linear overflow term adds +100 for overflow fill-rate 1.0.
        assert_eq!(pricing.state.tiers[0].price, 975);
    }

    #[test]
    fn boundary_price_caps_clamp_slower_tier_after_repricing() {
        let mut config = test_config();
        config.enforce_boundary_price_caps = true;
        config.delay_increase_threshold = 0.9;

        let mut state = TieredState::new(&config, TierLane::Ranking);
        state.tiers.push(Tier {
            id: TierId::new(1),
            lane: TierLane::Ranking,
            capacity: 20,
            version_created_slot: 0,
            delay: 2,
            price: 3000,
            used_capacity: 0,
            tx_count: 0,
        });
        state.tiers[0].price = 1000;

        TieredPricing::enforce_boundary_price_caps(&config, &mut state, BlockKind::RankingBlock, 7);

        assert_eq!(state.tiers[1].price, 900);
        assert_eq!(state.tiers[1].version_created_slot, 7);
    }

    #[test]
    fn dynamic_tier_sizing_rebalances_capacity_from_signal() {
        let mut config = test_config();
        config.dynamic_tier_sizing_enabled = true;
        config.dynamic_tier_sizing_alpha = 1.0;
        config.dynamic_tier_sizing_min_fraction = 0.0;
        config.delay_update_frequency = Some(1000);
        config.tier_update_frequency = Some(1);
        config.add_tier_threshold = u64::MAX;
        config.remove_tier_threshold = 0;

        let mut state = TieredState::new(&config, TierLane::Ranking);
        state.tiers = vec![
            Tier {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                capacity: 60,
                version_created_slot: 0,
                delay: 1,
                price: 1000,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(1),
                lane: TierLane::Ranking,
                capacity: 20,
                version_created_slot: 0,
                delay: 2,
                price: 900,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(2),
                lane: TierLane::Ranking,
                capacity: 20,
                version_created_slot: 0,
                delay: 4,
                price: 800,
                used_capacity: 0,
                tx_count: 0,
            },
        ];

        let mut rng = ChaChaRng::seed_from_u64(123);
        let signal = vec![0.1, 1.0, 0.1];
        let cadence = state.update_with_fill_rates(
            &config,
            &mut rng,
            signal.clone(),
            Some(signal),
            BlockKind::RankingBlock,
            1,
        );

        assert!(cadence.tier_update_triggered);
        let total: u64 = state.tiers.iter().map(|tier| tier.capacity).sum();
        assert_eq!(total, 100);
        assert!(state.tiers[1].capacity > state.tiers[0].capacity);
        assert!(state.tiers[1].capacity > state.tiers[2].capacity);
    }

    #[test]
    fn separate_rb_eb_pools_track_slot_cadence_independently() {
        let mut config = test_config();
        config.eb_total_capacity = Some(100);
        config.delay_update_frequency = None;
        config.tier_update_frequency = None;
        config.delay_update_period_slots = Some(80);
        config.tier_update_period_slots = Some(160);

        let mut pricing = TieredPricing::new(config, 99);

        let rb_first = pricing.update_after_block(&[], 100, BlockKind::RankingBlock, 80);
        assert!(rb_first.delay_update_triggered);
        assert!(!rb_first.tier_update_triggered);

        let eb_first = pricing.update_after_block(&[], 100, BlockKind::EndorserBlock, 80);
        assert!(eb_first.delay_update_triggered);
        assert!(!eb_first.tier_update_triggered);

        let rb_second = pricing.update_after_block(&[], 100, BlockKind::RankingBlock, 160);
        assert!(rb_second.delay_update_triggered);
        assert!(rb_second.tier_update_triggered);

        let eb_second = pricing.update_after_block(&[], 100, BlockKind::EndorserBlock, 160);
        assert!(eb_second.delay_update_triggered);
        assert!(eb_second.tier_update_triggered);
    }

    #[test]
    fn update_delays_uses_per_boundary_parameters() {
        let mut config = test_config();
        config.delay_increase_thresholds = vec![2.5, 1.1, 1.1];
        config.min_delay_ratios = vec![3.0, 1.2, 1.2];

        let mut tiers = vec![
            Tier {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                capacity: 60,
                version_created_slot: 0,
                delay: 1,
                price: 100,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(1),
                lane: TierLane::Ranking,
                capacity: 20,
                version_created_slot: 0,
                delay: 2,
                price: 220,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(2),
                lane: TierLane::Ranking,
                capacity: 20,
                version_created_slot: 0,
                delay: 3,
                price: 260,
                used_capacity: 0,
                tx_count: 0,
            },
        ];
        let mut rng = ChaChaRng::seed_from_u64(1);
        update_delays(&mut tiers, &config, &mut rng);

        assert_eq!(tiers[0].delay, 1);
        assert_eq!(tiers[1].delay, 3);
        assert_eq!(tiers[2].delay, 4);
    }

    #[test]
    fn update_tier_sizes_uses_boundary_min_ratio_for_new_tier_delay() {
        let mut config = test_config();
        config.max_tiers = 3;
        config.tier_size_fractions = vec![0.0, 0.25, 0.25];
        config.min_delay_ratios = vec![3.0, 2.5];
        config.new_tier_delay_ratio = 1.5;

        let mut tiers = vec![Tier {
            id: TierId::new(0),
            lane: TierLane::Ranking,
            capacity: 100,
            version_created_slot: 0,
            delay: 2,
            price: 10_000,
            used_capacity: 0,
            tx_count: 0,
        }];

        update_tier_sizes(&mut tiers, &config, 0, None);

        assert_eq!(tiers.len(), 2);
        assert_eq!(tiers[1].delay, 6);
    }

    #[test]
    fn admission_accepts_valid_quoted_assignment() {
        let mut config = test_config();
        config.max_tiers = 1;
        config.tier_size_fractions = vec![1.0];

        let mut pricing = TieredPricing::new(config.clone(), 77);
        pricing.state.tiers = vec![Tier {
            id: TierId::new(0),
            lane: TierLane::Ranking,
            capacity: 100,
            version_created_slot: 7,
            delay: 3,
            price: 10,
            used_capacity: 0,
            tx_count: 0,
        }];
        pricing.assignment_history = TierAssignmentHistory::from_tiers(
            &pricing.state.tiers,
            config.effective_tier_update_cadence_slots(),
            TierLane::Ranking,
            0,
        );

        let tx = Transaction {
            id: TransactionId::new(1),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 4,
            value: 1_000_000,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(100),
            tier_preference: Some(TierId::new(0)),
            tier_version_created_slot: Some(7),
            tier_delay_slots: Some(3),
            tier_price_per_byte_at_assignment: Some(10),
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: None,
            mempool_entry_slot: None,
            mempool_entry_rb_index: None,
            input_id: 1,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        };

        assert_eq!(pricing.verify_preassigned_transaction(&tx), Ok(()));
    }

    #[test]
    fn admission_rejects_invalid_quoted_assignment() {
        let mut config = test_config();
        config.max_tiers = 1;
        config.tier_size_fractions = vec![1.0];

        let mut pricing = TieredPricing::new(config.clone(), 79);
        pricing.state.tiers = vec![Tier {
            id: TierId::new(0),
            lane: TierLane::Ranking,
            capacity: 100,
            version_created_slot: 9,
            delay: 2,
            price: 11,
            used_capacity: 0,
            tx_count: 0,
        }];
        pricing.assignment_history = TierAssignmentHistory::from_tiers(
            &pricing.state.tiers,
            config.effective_tier_update_cadence_slots(),
            TierLane::Ranking,
            0,
        );

        let tx = Transaction {
            id: TransactionId::new(2),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 4,
            value: 1_000_000,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(109),
            tier_preference: Some(TierId::new(0)),
            tier_version_created_slot: Some(9),
            tier_delay_slots: Some(2),
            tier_price_per_byte_at_assignment: Some(11),
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: None,
            mempool_entry_slot: None,
            mempool_entry_rb_index: None,
            input_id: 2,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        };

        assert_eq!(
            pricing.verify_preassigned_transaction(&tx),
            Err(TransactionRejectReason::InvalidQuotedAssignment)
        );
    }

    #[test]
    fn admission_rejects_when_quoted_history_is_unavailable() {
        let mut config = test_config();
        config.max_tiers = 1;
        config.tier_size_fractions = vec![1.0];

        let mut pricing = TieredPricing::new(config.clone(), 81);
        pricing.state.tiers = vec![Tier {
            id: TierId::new(0),
            lane: TierLane::Ranking,
            capacity: 100,
            version_created_slot: 9,
            delay: 2,
            price: 11,
            used_capacity: 0,
            tx_count: 0,
        }];
        pricing.assignment_history = TierAssignmentHistory::from_tiers(
            &pricing.state.tiers,
            config.effective_tier_update_cadence_slots(),
            TierLane::Ranking,
            0,
        );

        let tx = Transaction {
            id: TransactionId::new(3),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 4,
            value: 1_000_000,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(110),
            tier_preference: Some(TierId::new(0)),
            tier_version_created_slot: Some(5),
            tier_delay_slots: Some(2),
            tier_price_per_byte_at_assignment: Some(11),
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: None,
            mempool_entry_slot: None,
            mempool_entry_rb_index: None,
            input_id: 3,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        };

        assert_eq!(
            pricing.verify_preassigned_transaction(&tx),
            Err(TransactionRejectReason::QuotedHistoryUnavailable)
        );
    }

    #[test]
    fn tiered_selection_keeps_tx_includable_after_repricing() {
        let mut config = test_config();
        config.max_tiers = 1;
        config.tier_size_fractions = vec![1.0];

        let mut pricing = TieredPricing::new(config, 83);
        pricing.state.tiers = vec![Tier {
            id: TierId::new(0),
            lane: TierLane::Ranking,
            capacity: 100,
            version_created_slot: 10,
            delay: 1,
            price: 12,
            used_capacity: 0,
            tx_count: 0,
        }];

        let tx = Arc::new(Transaction {
            id: TransactionId::new(1),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 4,
            value: 1_000_000,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(100),
            tier_preference: Some(TierId::new(0)),
            tier_version_created_slot: Some(0),
            tier_delay_slots: Some(1),
            tier_price_per_byte_at_assignment: Some(10),
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: None,
            mempool_entry_slot: None,
            mempool_entry_rb_index: None,
            input_id: 1,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        });

        let included = pricing.select_transactions(
            std::slice::from_ref(&tx),
            20,
            100,
            BlockKind::RankingBlock,
        );
        assert_eq!(included.len(), 1);
    }

    #[test]
    fn legacy_revalidation_rejects_stale_assignment_after_repricing() {
        let mut config = test_config();
        config.max_tiers = 1;
        config.tier_size_fractions = vec![1.0];
        config.assignment_semantics = TierAssignmentSemantics::LegacyRevalidateCurrentTier;

        let mut pricing = TieredPricing::new(config, 84);
        pricing.state.tiers = vec![Tier {
            id: TierId::new(0),
            lane: TierLane::Ranking,
            capacity: 100,
            version_created_slot: 10,
            delay: 1,
            price: 12,
            used_capacity: 0,
            tx_count: 0,
        }];

        let tx = Arc::new(Transaction {
            id: TransactionId::new(11),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 4,
            value: 1_000_000,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(100),
            tier_preference: Some(TierId::new(0)),
            tier_version_created_slot: Some(0),
            tier_delay_slots: Some(1),
            tier_price_per_byte_at_assignment: Some(10),
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: None,
            mempool_entry_slot: None,
            mempool_entry_rb_index: None,
            input_id: 11,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        });

        let included = pricing.select_transactions(
            std::slice::from_ref(&tx),
            20,
            100,
            BlockKind::RankingBlock,
        );
        assert!(included.is_empty());
    }

    #[test]
    fn tiered_selection_keeps_tx_includable_after_tier_removal() {
        let mut config = test_config();
        config.max_tiers = 2;
        config.tier_size_fractions = vec![0.0, 0.5];

        let mut pricing = TieredPricing::new(config, 85);
        pricing.state.tiers = vec![Tier {
            id: TierId::new(0),
            lane: TierLane::Ranking,
            capacity: 100,
            version_created_slot: 11,
            delay: 1,
            price: 8,
            used_capacity: 0,
            tx_count: 0,
        }];

        let tx = Arc::new(Transaction {
            id: TransactionId::new(2),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 4,
            value: 1_000_000,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(100),
            tier_preference: Some(TierId::new(1)),
            tier_version_created_slot: Some(7),
            tier_delay_slots: Some(4),
            tier_price_per_byte_at_assignment: Some(10),
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: None,
            mempool_entry_slot: None,
            mempool_entry_rb_index: None,
            input_id: 2,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        });

        let included = pricing.select_transactions(
            std::slice::from_ref(&tx),
            20,
            100,
            BlockKind::RankingBlock,
        );
        assert_eq!(included.len(), 1);
    }

    #[test]
    fn legacy_revalidation_rejects_stale_assignment_after_tier_removal() {
        let mut config = test_config();
        config.max_tiers = 2;
        config.tier_size_fractions = vec![0.0, 0.5];
        config.assignment_semantics = TierAssignmentSemantics::LegacyRevalidateCurrentTier;

        let mut pricing = TieredPricing::new(config, 86);
        pricing.state.tiers = vec![Tier {
            id: TierId::new(0),
            lane: TierLane::Ranking,
            capacity: 100,
            version_created_slot: 11,
            delay: 1,
            price: 8,
            used_capacity: 0,
            tx_count: 0,
        }];

        let tx = Arc::new(Transaction {
            id: TransactionId::new(12),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 4,
            value: 1_000_000,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(100),
            tier_preference: Some(TierId::new(1)),
            tier_version_created_slot: Some(7),
            tier_delay_slots: Some(4),
            tier_price_per_byte_at_assignment: Some(10),
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: None,
            mempool_entry_slot: None,
            mempool_entry_rb_index: None,
            input_id: 12,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        });

        let included = pricing.select_transactions(
            std::slice::from_ref(&tx),
            20,
            100,
            BlockKind::RankingBlock,
        );
        assert!(included.is_empty());
    }

    #[test]
    fn removed_tier_fallback_prefers_equal_or_cheaper_nearest_delay() {
        let mut config = test_config();
        config.max_tiers = 3;
        config.tier_size_fractions = vec![0.0, 0.3, 0.3];

        let mut pricing = TieredPricing::new(config, 87);
        pricing.state.tiers = vec![
            Tier {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                capacity: 0,
                version_created_slot: 0,
                delay: 1,
                price: 30,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(1),
                lane: TierLane::Ranking,
                capacity: 0,
                version_created_slot: 0,
                delay: 4,
                price: 40,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(2),
                lane: TierLane::Ranking,
                capacity: 100,
                version_created_slot: 0,
                delay: 6,
                price: 35,
                used_capacity: 0,
                tx_count: 0,
            },
        ];

        let tx = Arc::new(Transaction {
            id: TransactionId::new(9),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 0,
            value: 1_000_000,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(400),
            tier_preference: Some(TierId::new(9)),
            tier_version_created_slot: Some(1),
            tier_delay_slots: Some(5),
            tier_price_per_byte_at_assignment: Some(40),
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: None,
            mempool_entry_slot: None,
            mempool_entry_rb_index: None,
            input_id: 9,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        });

        let included =
            pricing.select_transactions(std::slice::from_ref(&tx), 0, 100, BlockKind::RankingBlock);
        assert_eq!(included.len(), 1);
        assert_eq!(included[0].id, tx.id);
    }

    #[test]
    fn removed_tier_fallback_routes_to_lowest_priority_when_no_equal_or_cheaper() {
        let mut config = test_config();
        config.max_tiers = 3;
        config.tier_size_fractions = vec![0.0, 0.3, 0.3];

        let mut pricing = TieredPricing::new(config, 89);
        pricing.state.tiers = vec![
            Tier {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                capacity: 0,
                version_created_slot: 0,
                delay: 1,
                price: 50,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(1),
                lane: TierLane::Ranking,
                capacity: 0,
                version_created_slot: 0,
                delay: 3,
                price: 60,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(2),
                lane: TierLane::Ranking,
                capacity: 100,
                version_created_slot: 0,
                delay: 3,
                price: 70,
                used_capacity: 0,
                tx_count: 0,
            },
        ];

        let tx = Arc::new(Transaction {
            id: TransactionId::new(10),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 0,
            value: 1_000_000,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(400),
            tier_preference: Some(TierId::new(9)),
            tier_version_created_slot: Some(1),
            tier_delay_slots: Some(2),
            tier_price_per_byte_at_assignment: Some(40),
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: None,
            mempool_entry_slot: None,
            mempool_entry_rb_index: None,
            input_id: 10,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        });

        let included =
            pricing.select_transactions(std::slice::from_ref(&tx), 0, 100, BlockKind::RankingBlock);
        assert_eq!(included.len(), 1);
        assert_eq!(included[0].id, tx.id);
    }

    #[test]
    fn assignment_history_retains_retired_records_for_window_then_prunes() {
        let tiers = vec![Tier {
            id: TierId::new(0),
            lane: TierLane::Ranking,
            capacity: 100,
            version_created_slot: 0,
            delay: 1,
            price: 10,
            used_capacity: 0,
            tx_count: 0,
        }];
        let mut history = TierAssignmentHistory::from_tiers(&tiers, 5, TierLane::Ranking, 0);
        assert_eq!(
            history
                .by_key
                .get(&(TierId::new(0), 0))
                .map(|record| record.created_slot),
            Some(0)
        );

        history.sync_with_tiers(&[], 10, TierLane::Ranking, 0);
        assert_eq!(
            history
                .by_key
                .get(&(TierId::new(0), 0))
                .and_then(|record| record.retired_slot),
            Some(10)
        );

        history.sync_with_tiers(&[], 90, TierLane::Ranking, 0);
        assert!(history.by_key.contains_key(&(TierId::new(0), 0)));

        history.sync_with_tiers(&[], 91, TierLane::Ranking, 0);
        assert!(!history.by_key.contains_key(&(TierId::new(0), 0)));
    }

    fn test_tx(id: u64, tier: TierId, bytes: u64, fee: u64) -> Arc<Transaction> {
        Arc::new(Transaction {
            id: TransactionId::new(id),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes,
            submission_slot: id,
            value: 1_000_000,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(fee),
            tier_preference: Some(tier),
            tier_version_created_slot: Some(0),
            tier_delay_slots: Some(1),
            tier_price_per_byte_at_assignment: Some(fee / bytes.max(1)),
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: None,
            mempool_entry_slot: None,
            mempool_entry_rb_index: None,
            input_id: id,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        })
    }

    #[test]
    fn eip1559_smoothing_damps_large_capacity_empty_update() {
        let full_rb_txs = vec![test_tx(1, TierId::new(0), 100, 10_000)];

        let mut unsmoothed = Eip1559Pricing::new(100, 8, 0.5, Eip1559SmoothingConfig::default());
        unsmoothed.update_after_block(&full_rb_txs, 100, 1);
        let unsmoothed_after_full = unsmoothed.base_fee_per_byte;
        unsmoothed.update_after_block(&[], 10_000, 2);

        let mut smoothed = Eip1559Pricing::new(
            100,
            8,
            0.5,
            Eip1559SmoothingConfig {
                enabled: true,
                alpha: 0.2,
            },
        );
        smoothed.update_after_block(&full_rb_txs, 100, 1);
        assert_eq!(smoothed.base_fee_per_byte, unsmoothed_after_full);
        smoothed.update_after_block(&[], 10_000, 2);

        assert!(smoothed.base_fee_per_byte > unsmoothed.base_fee_per_byte);
    }

    #[test]
    fn eip1559_priority_lane_quotes_multiplier_and_delay_advantage() {
        let pricing = Eip1559PriorityLanePricing::new(90, 8, 0.5, 5.0, 1.0, 1, 2);
        let snapshot = pricing.snapshot();

        assert_eq!(snapshot.tiers.len(), 2);
        assert_eq!(snapshot.tiers[0].id, TierId::new(0));
        assert_eq!(snapshot.tiers[0].price_per_byte, 450);
        assert_eq!(snapshot.tiers[0].delay, 1);
        assert_eq!(snapshot.tiers[1].id, TierId::new(1));
        assert_eq!(snapshot.tiers[1].price_per_byte, 90);
        assert_eq!(snapshot.tiers[1].delay, 2);
    }

    #[test]
    fn eip1559_priority_lane_selects_priority_before_normal() {
        let pricing = Eip1559PriorityLanePricing::new(10, 8, 0.5, 5.0, 1.0, 1, 2);
        let priority = test_tx(1, TierId::new(0), 40, 2_000);
        let normal = test_tx(2, TierId::new(1), 40, 400);

        let included = pricing.select_transactions(&[normal.clone(), priority.clone()], 0, 80);

        assert_eq!(
            included.iter().map(|tx| tx.id).collect::<Vec<_>>(),
            vec![priority.id, normal.id]
        );
    }

    #[test]
    fn eip1559_priority_lane_capacity_cap_limits_priority_first_pass() {
        let pricing = Eip1559PriorityLanePricing::new(10, 8, 0.5, 5.0, 0.25, 1, 2);
        let priority = test_tx(1, TierId::new(0), 40, 2_000);
        let normal = test_tx(2, TierId::new(1), 40, 400);

        let included = pricing.select_transactions(&[priority.clone(), normal.clone()], 0, 100);

        assert_eq!(included.len(), 1);
        assert_eq!(included[0].id, normal.id);
    }

    #[test]
    fn naive_rb_eb_policy_routes_transactions_to_expected_block_kind() {
        let mut config = test_config();
        config.max_tiers = 2;
        config.tier_size_fractions = vec![0.0, 0.5];
        config.block_selection_policy = TierBlockSelectionPolicy::NaiveRbEbTwoTier;

        let mut pricing = TieredPricing::new(config, 7);
        pricing.state.tiers = vec![
            Tier {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                capacity: 50,
                version_created_slot: 0,
                delay: 1,
                price: 10,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(1),
                lane: TierLane::Ranking,
                capacity: 50,
                version_created_slot: 0,
                delay: 2,
                price: 10,
                used_capacity: 0,
                tx_count: 0,
            },
        ];

        let txs = vec![
            test_tx(1, TierId::new(0), 10, 200),
            test_tx(2, TierId::new(1), 10, 200),
        ];

        let rb = pricing.select_transactions(&txs, 0, 100, BlockKind::RankingBlock);
        assert_eq!(rb.len(), 1);
        assert_eq!(rb[0].tier_preference, Some(TierId::new(0)));

        let eb = pricing.select_transactions(&txs, 0, 100, BlockKind::EndorserBlock);
        assert_eq!(eb.len(), 1);
        assert_eq!(eb[0].tier_preference, Some(TierId::new(1)));
    }

    #[test]
    fn naive_rb_eb_policy_uses_full_block_capacity_per_block_kind() {
        let mut config = test_config();
        config.max_tiers = 2;
        config.tier_size_fractions = vec![0.0, 0.5];
        config.block_selection_policy = TierBlockSelectionPolicy::NaiveRbEbTwoTier;

        let mut pricing = TieredPricing::new(config, 11);
        pricing.state.tiers = vec![
            Tier {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                capacity: 50,
                version_created_slot: 0,
                delay: 1,
                price: 10,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(1),
                lane: TierLane::Ranking,
                capacity: 50,
                version_created_slot: 0,
                delay: 2,
                price: 10,
                used_capacity: 0,
                tx_count: 0,
            },
        ];

        // A 60-byte tx would be rejected by a hidden 50-byte per-tier cap, but
        // should fit in a 100-byte RB under the naive RB/EB mapping policy.
        let txs = vec![test_tx(1, TierId::new(0), 60, 1_000)];
        let rb = pricing.select_transactions(&txs, 0, 100, BlockKind::RankingBlock);
        assert_eq!(rb.len(), 1);
        assert_eq!(rb[0].id, TransactionId::new(1));
    }

    #[test]
    fn naive_rb_eb_policy_initialises_with_two_tiers() {
        let mut config = test_config();
        config.max_tiers = 2;
        config.tier_size_fractions = vec![0.0, 0.5];
        config.block_selection_policy = TierBlockSelectionPolicy::NaiveRbEbTwoTier;

        let pricing = TieredPricing::new(config, 13);
        assert_eq!(pricing.state.tiers.len(), 2);
        assert_eq!(pricing.state.tiers[0].id, TierId::new(0));
        assert_eq!(pricing.state.tiers[1].id, TierId::new(1));
        assert_eq!(pricing.state.tiers[0].delay, 1);
        assert_eq!(pricing.state.tiers[1].delay, 1);
    }

    #[test]
    fn naive_rb_eb_policy_keeps_two_tiers_after_updates() {
        let mut config = test_config();
        config.max_tiers = 4;
        config.tier_size_fractions = vec![0.0, 0.5, 0.25, 0.25];
        config.tier_update_frequency = Some(1);
        config.add_tier_threshold = 1;
        config.block_selection_policy = TierBlockSelectionPolicy::NaiveRbEbTwoTier;

        let mut pricing = TieredPricing::new(config, 17);
        assert_eq!(pricing.state.tiers.len(), 2);
        for _ in 0..5 {
            pricing.update_after_block(&[], 100, BlockKind::RankingBlock, 0);
            assert_eq!(pricing.state.tiers.len(), 2);
            assert_eq!(pricing.state.tiers[0].delay, 1);
            assert_eq!(pricing.state.tiers[1].delay, 1);
        }
    }

    #[test]
    fn naive_rb_eb_policy_only_reprices_active_lane() {
        let mut config = test_config();
        config.max_tiers = 2;
        config.tier_size_fractions = vec![0.0, 0.5];
        config.block_selection_policy = TierBlockSelectionPolicy::NaiveRbEbTwoTier;
        config.target_utilisation = 0.5;
        config.base_fee_change_denominator = 8;

        let mut pricing = TieredPricing::new(config, 23);
        pricing.state.tiers = vec![
            Tier {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                capacity: 50,
                version_created_slot: 0,
                delay: 1,
                price: 100,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(1),
                lane: TierLane::Ranking,
                capacity: 50,
                version_created_slot: 0,
                delay: 1,
                price: 80,
                used_capacity: 0,
                tx_count: 0,
            },
        ];

        // Full RB utilisation should increase tier 0. Tier 1 must remain unchanged.
        let rb_txs = vec![test_tx(1, TierId::new(0), 100, 10_000)];
        pricing.update_after_block(&rb_txs, 100, BlockKind::RankingBlock, 0);
        assert_eq!(pricing.state.tiers[0].price, 112);
        assert_eq!(pricing.state.tiers[1].price, 80);

        // Full EB utilisation should increase tier 1. Tier 0 must remain unchanged.
        let eb_txs = vec![test_tx(2, TierId::new(1), 100, 10_000)];
        pricing.update_after_block(&eb_txs, 100, BlockKind::EndorserBlock, 1);
        assert_eq!(pricing.state.tiers[0].price, 112);
        assert_eq!(pricing.state.tiers[1].price, 90);
    }

    #[test]
    fn rb_tier0_reserved_policy_limits_rb_capacity() {
        let mut config = test_config();
        config.max_tiers = 2;
        config.tier_size_fractions = vec![0.0, 0.5];
        config.block_selection_policy = TierBlockSelectionPolicy::RbTier0Reserved;
        config.rb_tier0_reservation_fraction = 0.3;

        let mut pricing = TieredPricing::new(config, 29);
        pricing.state.tiers = vec![
            Tier {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                capacity: 30,
                version_created_slot: 0,
                delay: 1,
                price: 10,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(1),
                lane: TierLane::Ranking,
                capacity: 70,
                version_created_slot: 0,
                delay: 1,
                price: 10,
                used_capacity: 0,
                tx_count: 0,
            },
        ];

        let txs = vec![
            test_tx(1, TierId::new(0), 20, 500),
            test_tx(2, TierId::new(0), 20, 500),
            test_tx(3, TierId::new(1), 60, 1_000),
        ];

        // 30% of 100-byte RB capacity should only admit one 20-byte premium tx.
        let rb = pricing.select_transactions(&txs, 0, 100, BlockKind::RankingBlock);
        assert_eq!(rb.len(), 1);
        assert_eq!(rb[0].tier_preference, Some(TierId::new(0)));

        // EB can use full capacity for non-premium tiers.
        let eb = pricing.select_transactions(&txs, 0, 100, BlockKind::EndorserBlock);
        assert_eq!(eb.len(), 1);
        assert_eq!(eb[0].tier_preference, Some(TierId::new(1)));
    }

    #[test]
    fn rb_tier0_reserved_policy_only_reprices_active_lane() {
        let mut config = test_config();
        config.max_tiers = 2;
        config.tier_size_fractions = vec![0.0, 0.5];
        config.block_selection_policy = TierBlockSelectionPolicy::RbTier0Reserved;
        config.rb_tier0_reservation_fraction = 0.3;
        config.target_utilisation = 0.5;
        config.base_fee_change_denominator = 8;

        let mut pricing = TieredPricing::new(config, 31);
        pricing.state.tiers = vec![
            Tier {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                capacity: 30,
                version_created_slot: 0,
                delay: 1,
                price: 100,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(1),
                lane: TierLane::Ranking,
                capacity: 70,
                version_created_slot: 0,
                delay: 1,
                price: 80,
                used_capacity: 0,
                tx_count: 0,
            },
        ];

        let rb_txs = vec![test_tx(1, TierId::new(0), 30, 10_000)];
        pricing.update_after_block(&rb_txs, 100, BlockKind::RankingBlock, 0);
        assert_eq!(pricing.state.tiers[0].price, 112);
        assert_eq!(pricing.state.tiers[1].price, 80);

        let eb_txs = vec![test_tx(2, TierId::new(1), 100, 10_000)];
        pricing.update_after_block(&eb_txs, 100, BlockKind::EndorserBlock, 1);
        assert_eq!(pricing.state.tiers[0].price, 112);
        assert_eq!(pricing.state.tiers[1].price, 90);
    }

    #[test]
    fn rb_tier0_reserved_policy_can_add_eb_tiers() {
        let mut config = test_config();
        config.max_tiers = 3;
        config.tier_size_fractions = vec![0.0, 0.2, 0.25];
        config.block_selection_policy = TierBlockSelectionPolicy::RbTier0Reserved;
        config.rb_tier0_reservation_fraction = 0.3;
        config.tier_update_frequency = Some(1);
        config.add_tier_threshold = 1;
        config.new_tier_price = 10;

        let mut pricing = TieredPricing::new(config, 41);
        pricing.state.tiers = vec![
            Tier {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                capacity: 30,
                version_created_slot: 0,
                delay: 1,
                price: 100,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(1),
                lane: TierLane::Ranking,
                capacity: 100,
                version_created_slot: 0,
                delay: 1,
                price: 1_000,
                used_capacity: 0,
                tx_count: 0,
            },
        ];

        pricing.update_after_block(&[], 100, BlockKind::EndorserBlock, 0);
        assert_eq!(pricing.state.tiers.len(), 3);
        // Tier 0 (RB reserved) is fixed at 30.
        assert_eq!(pricing.state.tiers[0].capacity, 30);
        // Tier 1 (EB reservoir) is empty — all EB capacity distributed to active tiers.
        assert_eq!(pricing.state.tiers[1].capacity, 0);
        // Tier 2 (sole active EB tier) gets all non-fixed capacity: 100 - 30 = 70.
        assert_eq!(pricing.state.tiers[2].capacity, 70);
    }

    #[test]
    fn rb_tier0_reserved_policy_does_not_add_tiers_on_rb_updates() {
        let mut config = test_config();
        config.max_tiers = 3;
        config.tier_size_fractions = vec![0.0, 0.2, 0.25];
        config.block_selection_policy = TierBlockSelectionPolicy::RbTier0Reserved;
        config.rb_tier0_reservation_fraction = 0.3;
        config.tier_update_frequency = Some(1);
        config.add_tier_threshold = 1;
        config.new_tier_price = 10;

        let mut pricing = TieredPricing::new(config, 43);
        pricing.state.tiers = vec![
            Tier {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                capacity: 30,
                version_created_slot: 0,
                delay: 1,
                price: 100,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(1),
                lane: TierLane::Ranking,
                capacity: 100,
                version_created_slot: 0,
                delay: 1,
                price: 1_000,
                used_capacity: 0,
                tx_count: 0,
            },
        ];

        pricing.update_after_block(&[], 100, BlockKind::RankingBlock, 0);
        assert_eq!(pricing.state.tiers.len(), 2);
    }

    #[test]
    fn rb_tier0_reserved_policy_adds_tier_after_nth_eb_update() {
        let mut config = test_config();
        config.max_tiers = 3;
        config.tier_size_fractions = vec![0.0, 0.2, 0.25];
        config.block_selection_policy = TierBlockSelectionPolicy::RbTier0Reserved;
        config.rb_tier0_reservation_fraction = 0.3;
        config.tier_update_frequency = Some(2);
        config.add_tier_threshold = 1;
        config.new_tier_price = 10;

        let mut pricing = TieredPricing::new(config, 47);
        pricing.state.tiers = vec![
            Tier {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                capacity: 30,
                version_created_slot: 0,
                delay: 1,
                price: 100,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(1),
                lane: TierLane::Ranking,
                capacity: 100,
                version_created_slot: 0,
                delay: 1,
                price: 1_000,
                used_capacity: 0,
                tx_count: 0,
            },
        ];

        // First EB update should not resize yet.
        pricing.update_after_block(&[], 100, BlockKind::EndorserBlock, 0);
        assert_eq!(pricing.state.tiers.len(), 2);
        // Interleaved RB updates should not affect EB resize cadence.
        pricing.update_after_block(&[], 100, BlockKind::RankingBlock, 1);
        assert_eq!(pricing.state.tiers.len(), 2);
        // Second EB update should now trigger resize.
        pricing.update_after_block(&[], 100, BlockKind::EndorserBlock, 2);
        assert_eq!(pricing.state.tiers.len(), 3);
    }

    #[test]
    fn naive_rb_eb_tier_selection_uses_path_latency_for_utility() {
        let tx = Transaction {
            id: TransactionId::new(1),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 0,
            value: 500,
            urgency: UrgencyProfile::TimeBoxed { max_slots: 25 },
            posted_fee: None,
            tier_preference: None,
            tier_version_created_slot: None,
            tier_delay_slots: None,
            tier_price_per_byte_at_assignment: None,
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: None,
            mempool_entry_slot: None,
            mempool_entry_rb_index: None,
            input_id: 1,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        };
        let snapshot = PricingSnapshot {
            tiers: vec![
                TierQuote {
                    id: TierId::new(0),
                    lane: TierLane::Ranking,
                    version_created_slot: 0,
                    delay: 1,
                    price_per_byte: 30,
                    base_fee: 0,
                },
                TierQuote {
                    id: TierId::new(1),
                    lane: TierLane::Ranking,
                    version_created_slot: 0,
                    delay: 1,
                    price_per_byte: 5,
                    base_fee: 0,
                },
            ],
        };

        // With plain tier delays both lanes look equally fast, so the cheaper EB lane wins.
        let baseline = select_tier_for_tx(&tx, &snapshot, TierSelectionDelayModel::TierDelay)
            .expect("tier selection should succeed");
        assert_eq!(baseline.0, TierId::new(1));

        // With CIP-style RB/EB path latency, the EB lane is too slow for this timeboxed tx.
        let naive = select_tier_for_tx(
            &tx,
            &snapshot,
            TierSelectionDelayModel::NaiveRbEbTwoTierPath {
                rb_path_latency: 20,
                eb_path_latency: 56,
            },
        )
        .expect("tier selection should succeed");
        assert_eq!(naive.0, TierId::new(0));
        // Settlement delay remains tier-local; path latency only affects utility selection.
        assert_eq!(naive.3, 1);
    }

    #[test]
    fn lane_path_plus_tier_delay_keeps_in_lane_delay_differentiation() {
        let tx = Transaction {
            id: TransactionId::new(2),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 0,
            value: 500,
            urgency: UrgencyProfile::TimeBoxed { max_slots: 25 },
            posted_fee: None,
            tier_preference: None,
            tier_version_created_slot: None,
            tier_delay_slots: None,
            tier_price_per_byte_at_assignment: None,
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: None,
            mempool_entry_slot: None,
            mempool_entry_rb_index: None,
            input_id: 2,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        };
        let snapshot = PricingSnapshot {
            tiers: vec![
                TierQuote {
                    id: TierId::new(0),
                    lane: TierLane::Ranking,
                    version_created_slot: 0,
                    delay: 1,
                    price_per_byte: 30,
                    base_fee: 0,
                },
                TierQuote {
                    id: TierId::new(2),
                    lane: TierLane::Ranking,
                    version_created_slot: 0,
                    delay: 10,
                    price_per_byte: 5,
                    base_fee: 0,
                },
            ],
        };

        let selected = select_tier_for_tx(
            &tx,
            &snapshot,
            TierSelectionDelayModel::LanePathPlusTierDelay {
                rb_path_latency: 20,
                eb_path_latency: 56,
            },
        )
        .expect("tier selection should succeed");

        // With lane-path + tier-delay utility, long-delay tier is penalized even within same lane.
        assert_eq!(selected.0, TierId::new(0));
    }

    #[test]
    fn lane_path_plus_tier_delay_supports_block_delay_units() {
        let tx = Transaction {
            id: TransactionId::new(3),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 0,
            value: 500,
            urgency: UrgencyProfile::TimeBoxed { max_slots: 2 },
            posted_fee: None,
            tier_preference: None,
            tier_version_created_slot: None,
            tier_delay_slots: None,
            tier_price_per_byte_at_assignment: None,
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: None,
            mempool_entry_slot: None,
            mempool_entry_rb_index: None,
            input_id: 3,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        };
        let snapshot = PricingSnapshot {
            tiers: vec![
                TierQuote {
                    id: TierId::new(0),
                    lane: TierLane::Ranking,
                    version_created_slot: 0,
                    delay: 1,
                    price_per_byte: 30,
                    base_fee: 0,
                },
                TierQuote {
                    id: TierId::new(1),
                    lane: TierLane::Endorser,
                    version_created_slot: 0,
                    delay: 1,
                    price_per_byte: 5,
                    base_fee: 0,
                },
            ],
        };

        let selected = select_tier_for_tx(
            &tx,
            &snapshot,
            TierSelectionDelayModel::LanePathPlusTierDelay {
                rb_path_latency: 1,
                eb_path_latency: 3,
            },
        )
        .expect("tier selection should succeed");

        assert_eq!(selected.0, TierId::new(0));
    }

    #[test]
    fn continuous_policy_requires_eb_total_capacity() {
        let mut config = test_config();
        config.block_selection_policy = TierBlockSelectionPolicy::ContinuousRbEb;
        config.eb_total_capacity = None;
        assert!(config.validate().is_err());

        config.separate_eb_pool = true;
        assert!(config.validate().is_ok());

        config.separate_eb_pool = false;
        config.eb_total_capacity = Some(200);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn continuous_policy_selects_single_best_lane_assignment() {
        let tx = Transaction {
            id: TransactionId::new(1),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 0,
            value: 500,
            urgency: UrgencyProfile::TimeBoxed { max_slots: 3 },
            posted_fee: None,
            tier_preference: None,
            tier_version_created_slot: None,
            tier_delay_slots: None,
            tier_price_per_byte_at_assignment: None,
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: None,
            mempool_entry_slot: None,
            mempool_entry_rb_index: None,
            input_id: 1,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        };
        let rb_snapshot = PricingSnapshot {
            tiers: vec![TierQuote {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                version_created_slot: 0,
                delay: 1,
                price_per_byte: 20,
                base_fee: 0,
            }],
        };
        let eb_snapshot = PricingSnapshot {
            tiers: vec![TierQuote {
                id: TierId::new(0),
                lane: TierLane::Endorser,
                version_created_slot: 0,
                delay: 8,
                price_per_byte: 1,
                base_fee: 0,
            }],
        };

        let selected = select_best_lane_tier_for_tx(
            &tx,
            &rb_snapshot,
            &eb_snapshot,
            TierSelectionDelayModel::TierDelay,
        )
        .expect("lane selection should succeed");

        assert_eq!(selected.block_kind, BlockKind::RankingBlock);
        assert_eq!(selected.tier_id, TierId::new(0));
    }

    #[test]
    fn continuous_policy_enforces_single_lane_inclusion() {
        let mut config = test_config();
        config.block_selection_policy = TierBlockSelectionPolicy::ContinuousRbEb;
        config.eb_total_capacity = Some(100);
        config.max_tiers = 2;
        config.tier_size_fractions = vec![1.0, 0.0];
        config.new_tier_price = 100;

        let pricing = TieredPricing::new(config, 7);

        let rb_tx = Arc::new(Transaction {
            id: TransactionId::new(1),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 0,
            value: 1_000,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(1_000),
            tier_preference: Some(TierId::new(0)),
            tier_version_created_slot: Some(0),
            tier_delay_slots: Some(1),
            tier_price_per_byte_at_assignment: Some(100),
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: Some(BlockKind::RankingBlock),
            mempool_entry_slot: None,
            mempool_entry_rb_index: None,
            input_id: 11,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        });
        let eb_tx = Arc::new(Transaction {
            id: TransactionId::new(2),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 0,
            value: 1_000,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: None,
            tier_preference: None,
            tier_version_created_slot: None,
            tier_delay_slots: None,
            tier_price_per_byte_at_assignment: None,
            eb_tier_preference: Some(TierId::new(0)),
            eb_tier_version_created_slot: Some(0),
            eb_posted_fee: Some(1_000),
            eb_tier_delay_slots: Some(1),
            eb_tier_price_per_byte_at_assignment: Some(100),
            assigned_block_kind: Some(BlockKind::EndorserBlock),
            mempool_entry_slot: None,
            mempool_entry_rb_index: None,
            input_id: 22,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        });

        let txs = vec![rb_tx.clone(), eb_tx.clone()];
        let rb_selected = pricing.select_transactions(&txs, 0, 100, BlockKind::RankingBlock);
        assert_eq!(rb_selected.len(), 1);
        assert_eq!(rb_selected[0].id, rb_tx.id);

        let eb_selected = pricing.select_transactions(&txs, 0, 100, BlockKind::EndorserBlock);
        assert_eq!(eb_selected.len(), 1);
        assert_eq!(eb_selected[0].id, eb_tx.id);
    }

    #[test]
    fn single_pool_endorser_assignment_uses_primary_fields() {
        let mut config = test_config();
        config.block_selection_policy = TierBlockSelectionPolicy::NaiveRbEbTwoTier;
        config.max_tiers = 2;
        config.tier_size_fractions = vec![0.0, 0.5];
        config.new_tier_price = 100;

        let pricing = TieredPricing::new(config, 9);
        let tx = Arc::new(Transaction {
            id: TransactionId::new(1),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 0,
            value: 1_000,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(1_000),
            tier_preference: Some(TierId::new(1)),
            tier_version_created_slot: Some(0),
            tier_delay_slots: Some(1),
            tier_price_per_byte_at_assignment: Some(100),
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: Some(BlockKind::EndorserBlock),
            mempool_entry_slot: None,
            mempool_entry_rb_index: None,
            input_id: 1,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        });

        assert_eq!(pricing.verify_preassigned_transaction(&tx), Ok(()));

        let rb_selected =
            pricing.select_transactions(&[tx.clone()], 0, 100, BlockKind::RankingBlock);
        assert!(rb_selected.is_empty());

        let eb_selected =
            pricing.select_transactions(&[tx.clone()], 0, 100, BlockKind::EndorserBlock);
        assert_eq!(eb_selected.len(), 1);
        assert_eq!(eb_selected[0].id, tx.id);
    }

    #[test]
    fn single_pool_lane_partitioned_reporting_filters_by_block_kind() {
        let mut config = test_config();
        config.block_selection_policy = TierBlockSelectionPolicy::NaiveRbEbTwoTier;
        config.max_tiers = 2;
        config.tier_size_fractions = vec![0.0, 0.5];
        config.new_tier_price = 100;

        let mut pricing = TieredPricing::new(config, 9);
        pricing.state.last_utilisations = vec![0.75, 0.25];

        let rb_tiers = pricing.cloned_tiers_for_block_kind(BlockKind::RankingBlock);
        assert_eq!(rb_tiers.len(), 1);
        assert_eq!(rb_tiers[0].id, TierId::new(0));
        assert_eq!(rb_tiers[0].lane, TierLane::Ranking);

        let eb_tiers = pricing.cloned_tiers_for_block_kind(BlockKind::EndorserBlock);
        assert_eq!(eb_tiers.len(), 1);
        assert_eq!(eb_tiers[0].id, TierId::new(1));
        assert_eq!(eb_tiers[0].lane, TierLane::Endorser);

        assert_eq!(
            pricing.tier_utilisations_for_block_kind(BlockKind::RankingBlock),
            vec![0.75]
        );
        assert_eq!(
            pricing.tier_utilisations_for_block_kind(BlockKind::EndorserBlock),
            vec![0.25]
        );
    }

    #[test]
    fn continuous_policy_rejects_dual_lane_preassignment() {
        let mut config = test_config();
        config.block_selection_policy = TierBlockSelectionPolicy::ContinuousRbEb;
        config.eb_total_capacity = Some(100);
        config.new_tier_price = 100;

        let pricing = TieredPricing::new(config, 9);
        let tx = Transaction {
            id: TransactionId::new(1),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 0,
            value: 1_000,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(1_000),
            tier_preference: Some(TierId::new(0)),
            tier_version_created_slot: Some(0),
            tier_delay_slots: Some(1),
            tier_price_per_byte_at_assignment: Some(100),
            eb_tier_preference: Some(TierId::new(0)),
            eb_tier_version_created_slot: Some(0),
            eb_posted_fee: Some(1_000),
            eb_tier_delay_slots: Some(1),
            eb_tier_price_per_byte_at_assignment: Some(100),
            assigned_block_kind: Some(BlockKind::RankingBlock),
            mempool_entry_slot: None,
            mempool_entry_rb_index: None,
            input_id: 1,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        };
        assert_eq!(
            pricing.verify_preassigned_transaction(&tx),
            Err(TransactionRejectReason::InvalidQuotedAssignment)
        );
    }

    #[test]
    fn rebalance_single_active_tier_gets_all_capacity() {
        let config = TieredConfig {
            total_capacity: 100,
            max_tiers: 4,
            tier_size_fractions: vec![0.0, 0.25, 0.25, 0.25],
            ..test_config()
        };
        let mut tiers = vec![
            Tier {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                capacity: 92,
                version_created_slot: 0,
                delay: 1,
                price: 100,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(1),
                lane: TierLane::Ranking,
                capacity: 8,
                version_created_slot: 0,
                delay: 2,
                price: 100,
                used_capacity: 0,
                tx_count: 0,
            },
        ];
        rebalance_tier_capacities(&mut tiers, &config, 0, &[]);
        assert_eq!(tiers[0].capacity, 0);
        assert_eq!(tiers[1].capacity, 100);
    }

    #[test]
    fn rebalance_two_equal_tiers_split_evenly() {
        let config = TieredConfig {
            total_capacity: 100,
            max_tiers: 4,
            tier_size_fractions: vec![0.0, 0.25, 0.25, 0.25],
            ..test_config()
        };
        let mut tiers = vec![
            Tier {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                capacity: 84,
                version_created_slot: 0,
                delay: 1,
                price: 100,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(1),
                lane: TierLane::Ranking,
                capacity: 8,
                version_created_slot: 0,
                delay: 2,
                price: 100,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(2),
                lane: TierLane::Ranking,
                capacity: 8,
                version_created_slot: 0,
                delay: 4,
                price: 50,
                used_capacity: 0,
                tx_count: 0,
            },
        ];
        rebalance_tier_capacities(&mut tiers, &config, 0, &[]);
        assert_eq!(tiers[0].capacity, 0);
        assert_eq!(tiers[1].capacity, 50);
        assert_eq!(tiers[2].capacity, 50);
    }

    #[test]
    fn rebalance_unequal_fractions_proportional() {
        let config = TieredConfig {
            total_capacity: 100,
            max_tiers: 3,
            tier_size_fractions: vec![0.0, 0.20, 0.10],
            ..test_config()
        };
        let mut tiers = vec![
            Tier {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                capacity: 70,
                version_created_slot: 0,
                delay: 1,
                price: 100,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(1),
                lane: TierLane::Ranking,
                capacity: 20,
                version_created_slot: 0,
                delay: 2,
                price: 100,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(2),
                lane: TierLane::Ranking,
                capacity: 10,
                version_created_slot: 0,
                delay: 4,
                price: 50,
                used_capacity: 0,
                tx_count: 0,
            },
        ];
        rebalance_tier_capacities(&mut tiers, &config, 0, &[]);
        assert_eq!(tiers[0].capacity, 0);
        // 0.20 / 0.30 * 100 = 67 (rounded)
        assert_eq!(tiers[1].capacity, 67);
        // remainder: 100 - 67 = 33
        assert_eq!(tiers[2].capacity, 33);
    }

    #[test]
    fn rebalance_with_fixed_tier_preserves_it() {
        let config = TieredConfig {
            total_capacity: 100,
            max_tiers: 3,
            tier_size_fractions: vec![0.0, 0.2, 0.25],
            ..test_config()
        };
        let mut tiers = vec![
            Tier {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                capacity: 30,
                version_created_slot: 0,
                delay: 1,
                price: 100,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(1),
                lane: TierLane::Ranking,
                capacity: 50,
                version_created_slot: 0,
                delay: 1,
                price: 100,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(2),
                lane: TierLane::Ranking,
                capacity: 20,
                version_created_slot: 0,
                delay: 2,
                price: 50,
                used_capacity: 0,
                tx_count: 0,
            },
        ];
        // Tier 0 fixed (RB reserved), tier 1 is reservoir, tier 2 is active.
        rebalance_tier_capacities(&mut tiers, &config, 1, &[0]);
        assert_eq!(tiers[0].capacity, 30); // unchanged
        assert_eq!(tiers[1].capacity, 0); // reservoir emptied
        assert_eq!(tiers[2].capacity, 70); // gets all non-fixed: 100 - 30
    }

    #[test]
    fn rebalance_after_removal_restores_capacity() {
        let config = TieredConfig {
            total_capacity: 100,
            max_tiers: 4,
            tier_size_fractions: vec![0.0, 0.25, 0.25, 0.25],
            ..test_config()
        };
        // Start with 3 tiers, each at 33.
        let mut tiers = vec![
            Tier {
                id: TierId::new(0),
                lane: TierLane::Ranking,
                capacity: 1,
                version_created_slot: 0,
                delay: 1,
                price: 100,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(1),
                lane: TierLane::Ranking,
                capacity: 33,
                version_created_slot: 0,
                delay: 2,
                price: 100,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(2),
                lane: TierLane::Ranking,
                capacity: 33,
                version_created_slot: 0,
                delay: 4,
                price: 50,
                used_capacity: 0,
                tx_count: 0,
            },
            Tier {
                id: TierId::new(3),
                lane: TierLane::Ranking,
                capacity: 33,
                version_created_slot: 0,
                delay: 8,
                price: 10,
                used_capacity: 0,
                tx_count: 0,
            },
        ];
        // Remove last tier.
        tiers.pop();
        rebalance_tier_capacities(&mut tiers, &config, 0, &[]);
        assert_eq!(tiers[0].capacity, 0);
        assert_eq!(tiers[1].capacity, 50);
        assert_eq!(tiers[2].capacity, 50);
    }
}
