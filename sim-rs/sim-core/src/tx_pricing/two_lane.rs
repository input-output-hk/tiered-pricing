//! Two-lane pricing backend — covers all four spec variants (chain-derived).
//! (mechanism-design.md §"RB-reserved priority-only premium",
//! §"Un-reserved priority-only premium", §"Both-dynamic").
//!
//! Architecture (chain-derived, spike 007):
//! - Two `Eip1559Settings`-driven controllers, one per lane.
//! - Window length is per partition × signal source: RB-reserved
//!   priority controller uses length 1 (per-block fill rate); capacity-
//!   varying signals (un-reserved priority, both-dynamic standard) use
//!   the configured length.
//! - Multiplier-floor invariant `c_priority ≥ multiplier_floor ×
//!   c_standard` enforced **inside** `compute_derived_quote`'s return:
//!   the function returns the post-floor `PerLaneQuote`. No persistent
//!   state to enforce on after construction.
//! - Sample emission rules per variant live in `samples_for_block`.
//!
//! All state is `u64`/`u128`. f64 is forbidden in this module.

use crate::model::{PerLaneQuote, WindowAggregate};

use super::{
    BlockKind, BlockLaneBreakdown, Eip1559Settings, Lane, LaneSelectionOrder, LaneValidityRule,
    Multiplier, PricedBlockSample, PricingBackend,
    single_lane::compute_eip1559_step,
    window::update_aggregate,
};

/// Which of the four spec variants this backend implements. The variant
/// determines (a) the RB lane-validity rule, (b) whether the standard
/// controller is dynamic, (c) which samples are emitted per priced
/// block, and (d) the priority controller's window length.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TwoLaneVariant {
    /// RB-reserved partition; priority dynamic; standard fixed at
    /// `c = 1`. Mechanism-design.md §"RB-reserved priority-only
    /// premium".
    RbReservedPriorityOnly,
    /// RB-reserved partition; both controllers dynamic.
    /// Mechanism-design.md §"Both-dynamic" (partitioned variant).
    RbReservedBothDynamic,
    /// No partition; priority dynamic; standard fixed at `c = 1`.
    /// Mechanism-design.md §"Un-reserved priority-only premium".
    UnreservedPriorityOnly,
    /// No partition; both controllers dynamic. Mechanism-design.md
    /// §"Both-dynamic" (un-partitioned variant).
    UnreservedBothDynamic,
}

impl TwoLaneVariant {
    /// True iff the standard controller responds to demand. Priority-
    /// only variants pin standard at `c = 1`.
    pub fn standard_dynamic(self) -> bool {
        matches!(
            self,
            Self::RbReservedBothDynamic | Self::UnreservedBothDynamic
        )
    }

    /// True iff RBs enforce the priority-only validity rule. RB-reserved
    /// partition variants do; un-reserved variants don't.
    pub fn rb_priority_only(self) -> bool {
        matches!(
            self,
            Self::RbReservedPriorityOnly | Self::RbReservedBothDynamic
        )
    }
}

/// Configuration for the two-lane backend.
#[derive(Debug, Clone)]
pub struct TwoLaneSettings {
    pub variant: TwoLaneVariant,
    /// Priority-controller settings. RB-reserved variants ignore
    /// `window_length` and substitute 1 (uniform per-block priority
    /// capacity reduces to per-block fill rate).
    pub priority: Eip1559Settings,
    /// Standard-controller settings. Used only by the both-dynamic
    /// variants; priority-only variants pin `c_standard = 1` and
    /// ignore this.
    pub standard: Eip1559Settings,
    /// Multiplier-floor invariant: `c_priority ≥ multiplier_floor ×
    /// c_standard`. Enforced inside `compute_derived_quote`.
    pub multiplier_floor: Multiplier,
    /// Block-build scan order (`PriorityFirst` or `Fifo`).
    pub lane_selection_order: LaneSelectionOrder,
    /// Spec invariant for RB-reserved variants:
    /// `priority_reservation_bytes = max_block_size`. Used by the
    /// simulator's EB binary fullness trigger and by the RB-reserved
    /// EB priority-controller sample's byte cap (plan line 73).
    pub priority_reservation_bytes: u64,
}

impl TwoLaneSettings {
    pub fn validate(&self) -> anyhow::Result<()> {
        self.priority.validate()?;
        self.standard.validate()?;
        if self.priority.min_fee_a != self.standard.min_fee_a {
            anyhow::bail!(
                "TwoLaneSettings.priority.min_fee_a ({}) and standard.min_fee_a ({}) must match",
                self.priority.min_fee_a,
                self.standard.min_fee_a
            );
        }
        if self.multiplier_floor.denominator == 0 {
            anyhow::bail!("multiplier_floor.denominator must be non-zero");
        }
        if self.multiplier_floor.numerator < self.multiplier_floor.denominator {
            anyhow::bail!(
                "multiplier_floor must be ≥ 1 (got {}/{})",
                self.multiplier_floor.numerator,
                self.multiplier_floor.denominator
            );
        }
        if self.priority_reservation_bytes == 0 {
            anyhow::bail!("priority_reservation_bytes must be non-zero");
        }
        // Bound the multiplier-floor ratio so the u128 → u64 conversion
        // in the floor enforcement path cannot silently saturate for any
        // plausible `q_standard`. With a ratio cap of 2^32, we can fit
        //   floor = num × q_standard / den ≤ 2^32 × u64::MAX / 1
        // comfortably in u128; the residual saturation risk only kicks
        // in once `q_standard > u64::MAX / ratio`, which itself would
        // mean the controller has already saturated upstream and the
        // floor enforcement is moot. Realistic suites use ratios of 4,
        // 8, or 16 — orders of magnitude below the cap.
        const MULTIPLIER_FLOOR_RATIO_CAP_LOG2: u32 = 32;
        let ratio_cap = 1u128 << MULTIPLIER_FLOOR_RATIO_CAP_LOG2;
        let num = self.multiplier_floor.numerator as u128;
        let den = self.multiplier_floor.denominator as u128;
        // Ratio in fixed-point: num/den ≤ 2^32 ↔ num ≤ 2^32 × den.
        if num > ratio_cap.saturating_mul(den) {
            anyhow::bail!(
                "TwoLaneSettings.multiplier_floor ratio too large: \
                 {}/{} exceeds 2^{} cap; the u128 → u64 conversion in \
                 the multiplier-floor enforcement would saturate silently",
                self.multiplier_floor.numerator,
                self.multiplier_floor.denominator,
                MULTIPLIER_FLOOR_RATIO_CAP_LOG2,
            );
        }
        Ok(())
    }
}

/// Two-controller pricing backend (chain-derived; spike 007).
///
/// Holds only the settings — no controller state. `compute_derived_quote`
/// is a pure function over `(parent_quote, parent_aggregate, parent_samples,
/// evicted_samples)`. The multiplier-floor invariant is enforced on the
/// returned `PerLaneQuote` (not on persistent state, because there is none).
#[derive(Debug, Clone)]
pub struct TwoLanePricing {
    settings: TwoLaneSettings,
}

impl TwoLanePricing {
    pub fn new(mut settings: TwoLaneSettings) -> anyhow::Result<Self> {
        settings.validate()?;
        // RB-reserved priority controller: length 1 reduces to per-block
        // fill rate (mechanism-design.md line 176).
        if settings.variant.rb_priority_only() {
            settings.priority.window_length = 1;
        }
        // Priority-only variants: pin standard at min_fee_a (c = 1).
        // The construction-time floor enforcement is N/A under chain-
        // derivation — there is no persistent state to enforce on.
        // The floor is enforced exclusively on the *output* of
        // `compute_derived_quote`.
        if !settings.variant.standard_dynamic() {
            settings.standard.initial_quote_per_byte = settings.standard.min_fee_a;
        }
        Ok(Self { settings })
    }

    pub fn variant(&self) -> TwoLaneVariant {
        self.settings.variant
    }

    pub fn settings(&self) -> &TwoLaneSettings {
        &self.settings
    }

    /// Apply the multiplier-floor invariant to a `(standard, priority)`
    /// pair. `q_priority ≥ ceil(num × q_standard / den)`. Uses u128
    /// intermediates; saturates at u64::MAX (which `validate` caps).
    fn apply_floor(&self, q_standard: u64, q_priority: u64) -> u64 {
        let num = self.settings.multiplier_floor.numerator as u128;
        let den = self.settings.multiplier_floor.denominator as u128;
        let scaled = (q_standard as u128).saturating_mul(num);
        let floor = if scaled == 0 {
            0u128
        } else {
            (scaled - 1) / den + 1
        };
        debug_assert!(
            floor <= u64::MAX as u128,
            "multiplier-floor overflow: floor={floor} num={num} den={den} q_standard={q_standard}"
        );
        let floor_u64 = u64::try_from(floor).unwrap_or(u64::MAX);
        q_priority.max(floor_u64)
    }

    /// Per-lane worst-case quote projection. Used by the staleness
    /// predictor: read the chain-tip's `derived_quote.get(lane)`, then
    /// call this to project N max-up steps forward. Priority-only
    /// variants pin standard, so worst-case standard = current.
    pub fn worst_case_quote_for(
        &self,
        current_quote_for_lane: u64,
        lane: Lane,
        blocks_ahead: u32,
    ) -> u64 {
        match lane {
            Lane::Standard if !self.settings.variant.standard_dynamic() => current_quote_for_lane,
            Lane::Standard => super::single_lane::worst_case_eip1559_quote(
                current_quote_for_lane,
                self.settings.standard.max_change_denominator,
                blocks_ahead,
            ),
            Lane::Priority => super::single_lane::worst_case_eip1559_quote(
                current_quote_for_lane,
                self.settings.priority.max_change_denominator,
                blocks_ahead,
            ),
        }
    }
}

impl PricingBackend for TwoLanePricing {
    fn compute_derived_quote(
        &self,
        parent_quote: PerLaneQuote,
        parent_aggregate: WindowAggregate,
        parent_samples: &[PricedBlockSample],
        evicted_samples: &[PricedBlockSample],
    ) -> (PerLaneQuote, WindowAggregate) {
        // Step 1: fold parent_samples and evicted_samples into the
        // window aggregate. Each sample is lane-keyed (priority vs
        // standard) via its `controller_lane` field.
        // The per-lane window lengths can differ (RB-reserved priority
        // is forced to 1), but the aggregate carries both lanes in one
        // struct — the actual eviction policy is the caller's
        // responsibility (the simulator sources the evicted slice from
        // the block at `window_length + 1` back per controller).
        let new_aggregate = update_aggregate(
            parent_aggregate,
            parent_samples,
            evicted_samples,
            self.settings.priority.window_length.max(self.settings.standard.window_length),
        );

        // Step 2: priority controller step. Skip when the priority
        // lane has no samples in the window (matches legacy
        // `Eip1559Pricing::step` semantics — no signal, no movement).
        let priority_quote = if new_aggregate.priority_sum_capacity == 0 {
            parent_quote.priority
        } else {
            let (p_num, p_den) = new_aggregate.aggregate_util(Lane::Priority);
            compute_eip1559_step(parent_quote.priority, (p_num, p_den), &self.settings.priority)
        };

        // Step 3: standard controller step (or pin to min_fee_a for
        // priority-only variants). Skip-on-empty for the dynamic
        // variant too.
        let standard_quote = if self.settings.variant.standard_dynamic() {
            if new_aggregate.standard_sum_capacity == 0 {
                parent_quote.standard
            } else {
                let (s_num, s_den) = new_aggregate.aggregate_util(Lane::Standard);
                compute_eip1559_step(parent_quote.standard, (s_num, s_den), &self.settings.standard)
            }
        } else {
            // Pin c_standard = 1 (mechanism-design.md §"RB-reserved
            // priority-only premium"). The output is the spec's static
            // standard quote, regardless of input parent_quote.standard.
            self.settings.standard.min_fee_a
        };

        // Step 4: multiplier-floor invariant.
        let priority_quote_floored = self.apply_floor(standard_quote, priority_quote);

        (
            PerLaneQuote {
                standard: standard_quote,
                priority: priority_quote_floored,
            },
            new_aggregate,
        )
    }

    fn effective_window_length(&self) -> usize {
        self.settings
            .priority
            .window_length
            .max(self.settings.standard.window_length)
    }

    fn cold_start_quote(&self, lane: Lane) -> u64 {
        match lane {
            Lane::Standard => self
                .settings
                .standard
                .initial_quote_per_byte
                .max(self.settings.standard.min_fee_a),
            Lane::Priority => {
                // Raise priority's initial quote to the multiplier-floor
                // if needed (mirrors the legacy constructor's `enforce_
                // multiplier_floor` invariant at construction time).
                let initial = self
                    .settings
                    .priority
                    .initial_quote_per_byte
                    .max(self.settings.priority.min_fee_a);
                let standard = self
                    .settings
                    .standard
                    .initial_quote_per_byte
                    .max(self.settings.standard.min_fee_a);
                let standard_for_floor = if self.settings.variant.standard_dynamic() {
                    standard
                } else {
                    self.settings.standard.min_fee_a
                };
                self.apply_floor(standard_for_floor, initial)
            }
        }
    }

    fn lane_validity_rule(&self, block_kind: BlockKind) -> LaneValidityRule {
        match (block_kind, self.settings.variant.rb_priority_only()) {
            (BlockKind::RankingBlock, true) => LaneValidityRule::PriorityOnly,
            _ => LaneValidityRule::None,
        }
    }

    fn lane_selection_order(&self) -> LaneSelectionOrder {
        self.settings.lane_selection_order
    }

    fn min_priority_premium_multiplier(&self) -> Option<Multiplier> {
        Some(self.settings.multiplier_floor)
    }

    fn samples_for_block(
        &self,
        block_kind: BlockKind,
        breakdown: &BlockLaneBreakdown,
    ) -> Vec<PricedBlockSample> {
        // Sample-emission rules per implementation-plan.md lines 65-77,
        // branching on (variant, block_kind).
        match (self.settings.variant, block_kind) {
            (
                TwoLaneVariant::RbReservedPriorityOnly | TwoLaneVariant::RbReservedBothDynamic,
                BlockKind::RankingBlock,
            ) => {
                debug_assert_eq!(
                    breakdown.standard_paying_bytes, 0,
                    "RB-reserved RB must contain only priority-fee txs (validity rule)"
                );
                vec![PricedBlockSample {
                    block_kind,
                    controller_lane: Lane::Priority,
                    relevant_bytes: breakdown.priority_paying_bytes,
                    relevant_capacity: breakdown.block_capacity,
                }]
            }
            (
                TwoLaneVariant::RbReservedPriorityOnly | TwoLaneVariant::RbReservedBothDynamic,
                BlockKind::EndorserBlock,
            ) => {
                let cap = self.settings.priority_reservation_bytes;
                let priority_bytes = breakdown.priority_paying_bytes.min(cap);
                let mut out = vec![PricedBlockSample {
                    block_kind,
                    controller_lane: Lane::Priority,
                    relevant_bytes: priority_bytes,
                    relevant_capacity: cap,
                }];
                if matches!(self.settings.variant, TwoLaneVariant::RbReservedBothDynamic) {
                    out.push(PricedBlockSample {
                        block_kind,
                        controller_lane: Lane::Standard,
                        relevant_bytes: breakdown.standard_paying_bytes,
                        relevant_capacity: breakdown.block_capacity,
                    });
                }
                out
            }
            (TwoLaneVariant::UnreservedPriorityOnly | TwoLaneVariant::UnreservedBothDynamic, _) => {
                let mut out = vec![PricedBlockSample {
                    block_kind,
                    controller_lane: Lane::Priority,
                    relevant_bytes: breakdown.priority_paying_bytes,
                    relevant_capacity: breakdown.block_capacity,
                }];
                if matches!(self.settings.variant, TwoLaneVariant::UnreservedBothDynamic) {
                    out.push(PricedBlockSample {
                        block_kind,
                        controller_lane: Lane::Standard,
                        relevant_bytes: breakdown.standard_paying_bytes,
                        relevant_capacity: breakdown.block_capacity,
                    });
                }
                out
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PerLaneQuote, WindowAggregate};

    fn settings(variant: TwoLaneVariant) -> TwoLaneSettings {
        TwoLaneSettings {
            variant,
            priority: Eip1559Settings {
                min_fee_a: 44,
                initial_quote_per_byte: 44,
                target_num: 1,
                target_den: 2,
                max_change_denominator: 8,
                window_length: 4,
            },
            standard: Eip1559Settings {
                min_fee_a: 44,
                initial_quote_per_byte: 44,
                target_num: 1,
                target_den: 2,
                max_change_denominator: 8,
                window_length: 4,
            },
            multiplier_floor: Multiplier::new(16, 1).unwrap(),
            lane_selection_order: LaneSelectionOrder::PriorityFirst,
            priority_reservation_bytes: 90_000,
        }
    }

    #[test]
    fn rb_reserved_forces_priority_window_length_one() {
        let pricing =
            TwoLanePricing::new(settings(TwoLaneVariant::RbReservedPriorityOnly)).unwrap();
        assert_eq!(pricing.settings().priority.window_length, 1);
    }

    #[test]
    fn unreserved_keeps_priority_window_length_from_settings() {
        let pricing =
            TwoLanePricing::new(settings(TwoLaneVariant::UnreservedPriorityOnly)).unwrap();
        assert_eq!(pricing.settings().priority.window_length, 4);
    }

    #[test]
    fn priority_only_variant_pins_standard_at_min_fee_a() {
        let pricing =
            TwoLanePricing::new(settings(TwoLaneVariant::RbReservedPriorityOnly)).unwrap();
        // Initial quote at cold start: priority and standard.
        let q_standard = pricing.cold_start_quote(Lane::Standard);
        assert_eq!(q_standard, 44);
        // Feed a saturated standard sample — must not move c_standard.
        let parent = PerLaneQuote {
            standard: 44,
            priority: pricing.cold_start_quote(Lane::Priority),
        };
        let (q, _) = pricing.compute_derived_quote(
            parent,
            WindowAggregate::ZERO,
            &[PricedBlockSample {
                block_kind: BlockKind::EndorserBlock,
                controller_lane: Lane::Standard,
                relevant_bytes: 100,
                relevant_capacity: 100,
            }],
            &[],
        );
        assert_eq!(q.standard, 44, "priority-only variant must not move c_standard");
    }

    #[test]
    fn multiplier_floor_holds_at_construction() {
        let pricing =
            TwoLanePricing::new(settings(TwoLaneVariant::RbReservedBothDynamic)).unwrap();
        let q_p = pricing.cold_start_quote(Lane::Priority);
        let q_s = pricing.cold_start_quote(Lane::Standard);
        assert_eq!(q_s, 44);
        assert_eq!(q_p, 16 * 44);
    }

    #[test]
    fn multiplier_floor_holds_after_standard_moves_up() {
        let pricing =
            TwoLanePricing::new(settings(TwoLaneVariant::RbReservedBothDynamic)).unwrap();
        let mut q = PerLaneQuote {
            standard: pricing.cold_start_quote(Lane::Standard),
            priority: pricing.cold_start_quote(Lane::Priority),
        };
        let mut agg = WindowAggregate::ZERO;
        let sample = PricedBlockSample {
            block_kind: BlockKind::EndorserBlock,
            controller_lane: Lane::Standard,
            relevant_bytes: 100,
            relevant_capacity: 100,
        };
        for _ in 0..20 {
            let (nq, na) = pricing.compute_derived_quote(q, agg, &[sample], &[]);
            let floor = (16u128 * nq.standard as u128) as u64;
            assert!(
                nq.priority >= floor,
                "multiplier-floor violation: q_p={} q_s={} floor={}",
                nq.priority,
                nq.standard,
                floor
            );
            q = nq;
            agg = na;
        }
    }

    #[test]
    fn rb_reserved_only_emits_priority_sample_for_rb() {
        let pricing =
            TwoLanePricing::new(settings(TwoLaneVariant::RbReservedBothDynamic)).unwrap();
        let breakdown = BlockLaneBreakdown {
            priority_paying_bytes: 90_000,
            standard_paying_bytes: 0,
            block_capacity: 90_000,
        };
        let samples = pricing.samples_for_block(BlockKind::RankingBlock, &breakdown);
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].controller_lane, Lane::Priority);
        assert_eq!(samples[0].relevant_bytes, 90_000);
        assert_eq!(samples[0].relevant_capacity, 90_000);
    }

    #[test]
    fn rb_reserved_caps_priority_eb_bytes_at_one_rb() {
        let pricing =
            TwoLanePricing::new(settings(TwoLaneVariant::RbReservedPriorityOnly)).unwrap();
        let breakdown = BlockLaneBreakdown {
            priority_paying_bytes: 1_000_000,
            standard_paying_bytes: 5_000_000,
            block_capacity: 12_000_000,
        };
        let samples = pricing.samples_for_block(BlockKind::EndorserBlock, &breakdown);
        let priority_sample = samples
            .iter()
            .find(|s| s.controller_lane == Lane::Priority)
            .unwrap();
        assert_eq!(priority_sample.relevant_bytes, 90_000);
        assert_eq!(priority_sample.relevant_capacity, 90_000);
        assert!(
            priority_sample.relevant_bytes <= priority_sample.relevant_capacity,
            "saturating priority must keep signal in [0, 1]"
        );
    }

    #[test]
    fn rb_reserved_both_dynamic_eb_emits_two_samples() {
        let pricing =
            TwoLanePricing::new(settings(TwoLaneVariant::RbReservedBothDynamic)).unwrap();
        let breakdown = BlockLaneBreakdown {
            priority_paying_bytes: 50_000,
            standard_paying_bytes: 5_000_000,
            block_capacity: 12_000_000,
        };
        let samples = pricing.samples_for_block(BlockKind::EndorserBlock, &breakdown);
        assert_eq!(samples.len(), 2);
        let priority = samples
            .iter()
            .find(|s| s.controller_lane == Lane::Priority)
            .unwrap();
        let standard = samples
            .iter()
            .find(|s| s.controller_lane == Lane::Standard)
            .unwrap();
        assert_eq!(priority.relevant_bytes, 50_000);
        assert_eq!(priority.relevant_capacity, 90_000);
        assert_eq!(standard.relevant_bytes, 5_000_000);
        assert_eq!(standard.relevant_capacity, 12_000_000);
    }

    #[test]
    fn unreserved_priority_only_emits_only_priority_sample() {
        let pricing =
            TwoLanePricing::new(settings(TwoLaneVariant::UnreservedPriorityOnly)).unwrap();
        let breakdown = BlockLaneBreakdown {
            priority_paying_bytes: 1_000_000,
            standard_paying_bytes: 5_000_000,
            block_capacity: 12_000_000,
        };
        let samples = pricing.samples_for_block(BlockKind::EndorserBlock, &breakdown);
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].controller_lane, Lane::Priority);
        assert_eq!(samples[0].relevant_bytes, 1_000_000);
        assert_eq!(samples[0].relevant_capacity, 12_000_000);
    }

    #[test]
    fn unreserved_both_dynamic_emits_two_samples_for_each_block_kind() {
        let pricing =
            TwoLanePricing::new(settings(TwoLaneVariant::UnreservedBothDynamic)).unwrap();
        let breakdown = BlockLaneBreakdown {
            priority_paying_bytes: 30_000,
            standard_paying_bytes: 60_000,
            block_capacity: 90_000,
        };
        let rb_samples = pricing.samples_for_block(BlockKind::RankingBlock, &breakdown);
        let eb_samples = pricing.samples_for_block(BlockKind::EndorserBlock, &breakdown);
        assert_eq!(rb_samples.len(), 2);
        assert_eq!(eb_samples.len(), 2);
        for samples in [&rb_samples, &eb_samples] {
            assert!(samples.iter().any(|s| s.controller_lane == Lane::Priority));
            assert!(samples.iter().any(|s| s.controller_lane == Lane::Standard));
        }
    }

    #[test]
    fn lane_validity_rule_priority_only_for_rb_reserved_rb() {
        for variant in [
            TwoLaneVariant::RbReservedPriorityOnly,
            TwoLaneVariant::RbReservedBothDynamic,
        ] {
            let pricing = TwoLanePricing::new(settings(variant)).unwrap();
            assert_eq!(
                pricing.lane_validity_rule(BlockKind::RankingBlock),
                LaneValidityRule::PriorityOnly,
                "{variant:?}"
            );
            assert_eq!(
                pricing.lane_validity_rule(BlockKind::EndorserBlock),
                LaneValidityRule::None,
                "{variant:?}"
            );
        }
        for variant in [
            TwoLaneVariant::UnreservedPriorityOnly,
            TwoLaneVariant::UnreservedBothDynamic,
        ] {
            let pricing = TwoLanePricing::new(settings(variant)).unwrap();
            assert_eq!(
                pricing.lane_validity_rule(BlockKind::RankingBlock),
                LaneValidityRule::None,
                "{variant:?}"
            );
        }
    }

    #[test]
    fn rb_reserved_standard_isolation_does_not_move_c_standard_on_priority_rb() {
        // Plan line 313: a saturated priority-only RB updates c_priority
        // but does **not** change c_standard or its window samples.
        let pricing =
            TwoLanePricing::new(settings(TwoLaneVariant::RbReservedBothDynamic)).unwrap();
        let parent_q = PerLaneQuote {
            standard: pricing.cold_start_quote(Lane::Standard),
            priority: pricing.cold_start_quote(Lane::Priority),
        };
        let breakdown = BlockLaneBreakdown {
            priority_paying_bytes: 90_000,
            standard_paying_bytes: 0,
            block_capacity: 90_000,
        };
        let samples = pricing.samples_for_block(BlockKind::RankingBlock, &breakdown);
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].controller_lane, Lane::Priority);
        let (q, new_agg) =
            pricing.compute_derived_quote(parent_q, WindowAggregate::ZERO, &samples, &[]);
        // Standard quote does not move: derived from min_fee_a context + an
        // empty standard aggregate. After one step at target=0.5, util=0/0
        // means no signal → returns input parent_q.standard ... but the
        // controller takes its input through the aggregate. With sum_cap=0
        // for standard, the step yields parent_q.standard unchanged.
        assert_eq!(
            q.standard, parent_q.standard,
            "saturated priority-only RB must not move c_standard"
        );
        assert_eq!(
            new_agg.standard_sum_bytes, 0,
            "saturated priority-only RB must not feed the standard window"
        );
        assert_eq!(new_agg.standard_sum_capacity, 0);
    }

    #[test]
    fn rejects_zero_denominator_floor() {
        let mut s = settings(TwoLaneVariant::RbReservedBothDynamic);
        s.multiplier_floor = Multiplier {
            numerator: 16,
            denominator: 0,
        };
        assert!(TwoLanePricing::new(s).is_err());
    }

    #[test]
    fn rejects_floor_below_one() {
        let mut s = settings(TwoLaneVariant::RbReservedBothDynamic);
        s.multiplier_floor = Multiplier::new(1, 2).unwrap();
        assert!(TwoLanePricing::new(s).is_err());
    }

    #[test]
    fn sibling_rbs_produce_identical_derived_quote() {
        // Spike 007 §"Slot-battle resolution under chain-derived":
        // two children of the same parent with identical compute
        // inputs must produce identical (PerLaneQuote, WindowAggregate).
        let pricing =
            TwoLanePricing::new(settings(TwoLaneVariant::RbReservedBothDynamic)).unwrap();
        let parent_q = PerLaneQuote {
            standard: pricing.cold_start_quote(Lane::Standard),
            priority: pricing.cold_start_quote(Lane::Priority),
        };
        let parent_agg = WindowAggregate::ZERO;
        let breakdown = BlockLaneBreakdown {
            priority_paying_bytes: 50_000,
            standard_paying_bytes: 5_000_000,
            block_capacity: 12_000_000,
        };
        let samples = pricing.samples_for_block(BlockKind::EndorserBlock, &breakdown);
        let (a_q, a_agg) =
            pricing.compute_derived_quote(parent_q, parent_agg, &samples, &[]);
        let (b_q, b_agg) =
            pricing.compute_derived_quote(parent_q, parent_agg, &samples, &[]);
        assert_eq!(a_q, b_q, "sibling derived_quote must be identical");
        assert_eq!(a_agg, b_agg, "sibling window_aggregate must be identical");
    }
}
