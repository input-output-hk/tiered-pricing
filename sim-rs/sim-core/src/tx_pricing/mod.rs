use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::model::{TierId, Transaction, TransactionRejectReason};

mod tiered;
pub use tiered::*;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlockKind {
    RankingBlock,
    EndorserBlock,
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
pub struct PricingFile {
    pub pricing_mechanism: PricingMechanismConfig,
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
