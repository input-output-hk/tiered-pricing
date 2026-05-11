//! Two-lane pricing backend — covers all four spec variants
//! (mechanism-design.md §"RB-reserved priority-only premium",
//! §"Un-reserved priority-only premium", §"Both-dynamic").
//!
//! Architecture (implementation-plan.md lines 46-50):
//! - Two `Eip1559Pricing`-style controllers, one per lane.
//! - Window length is per partition × signal source: RB-reserved
//!   priority controller uses length 1 (per-block fill rate, since
//!   priority capacity is uniform per block); capacity-varying signals
//!   (un-reserved priority, both-dynamic standard) use the configured
//!   length.
//! - Multiplier-floor invariant `c_priority ≥ multiplier_floor ×
//!   c_standard` enforced **after** both controllers' independent
//!   updates each block. State is the integer `quote_per_byte` so the
//!   floor reduces to `q_priority ≥ ceil(num × q_standard / den)`.
//! - Sample emission rules per variant live in `samples_for_block`.
//!
//! All state is `u64`/`u128`. f64 is forbidden in this module.

use super::{
    BlockKind, BlockLaneBreakdown, Eip1559Pricing, Eip1559Settings, Lane, LaneSelectionOrder,
    LaneValidityRule, Multiplier, PricedBlockSample, PricingBackend, PricingSnapshot,
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
    /// c_standard`. Enforced post-update.
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
        Ok(())
    }
}

/// Two-controller pricing backend.
///
/// Holds two `Eip1559Pricing` instances (priority, standard). For
/// priority-only variants the standard controller is constructed but
/// never stepped — its `quote_per_byte` stays pinned at `min_fee_a`
/// (so `c_standard = 1` per spec). The priority controller's window
/// length is forced to 1 for RB-reserved variants per plan line 47.
#[derive(Debug, Clone)]
pub struct TwoLanePricing {
    settings: TwoLaneSettings,
    priority: Eip1559Pricing,
    standard: Eip1559Pricing,
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
        if !settings.variant.standard_dynamic() {
            settings.standard.initial_quote_per_byte = settings.standard.min_fee_a;
        }
        let priority = Eip1559Pricing::new(settings.priority.clone())?;
        let standard = Eip1559Pricing::new(settings.standard.clone())?;
        let mut me = Self {
            settings,
            priority,
            standard,
        };
        // Apply the floor at construction so the initial state already
        // satisfies the invariant. (If `initial_quote_per_byte` for
        // priority is below the floor, the floor wins.)
        me.enforce_multiplier_floor();
        Ok(me)
    }

    pub fn variant(&self) -> TwoLaneVariant {
        self.settings.variant
    }

    pub fn settings(&self) -> &TwoLaneSettings {
        &self.settings
    }

    /// Test/inspection accessor.
    pub fn priority_controller(&self) -> &Eip1559Pricing {
        &self.priority
    }

    /// Test/inspection accessor.
    pub fn standard_controller(&self) -> &Eip1559Pricing {
        &self.standard
    }

    /// Apply the multiplier-floor invariant: `q_priority ≥ ceil(num ×
    /// q_standard / den)`. Equivalent to `c_priority ≥ multiplier_floor
    /// × c_standard` since `q = c × min_fee_a` and `min_fee_a` cancels.
    /// Done with `u128` to keep the multiplication safe.
    fn enforce_multiplier_floor(&mut self) {
        let q_standard = self.standard.current_quote(Lane::Standard) as u128;
        let num = self.settings.multiplier_floor.numerator as u128;
        let den = self.settings.multiplier_floor.denominator as u128;
        // ceil(num × q_standard / den)
        let scaled = q_standard.saturating_mul(num);
        let floor = if scaled == 0 {
            0
        } else {
            (scaled - 1) / den + 1
        };
        let q_priority = self.priority.current_quote(Lane::Priority) as u128;
        if q_priority < floor {
            // Bypass the controller's own window: we're enforcing an
            // invariant, not running an EIP-1559 step. Reach in via the
            // newly-added setter on `Eip1559Pricing`. The floor fits in
            // u64 for any sane configuration (q_standard ≤ u64::MAX,
            // multiplier_floor ratio ≪ 2^64); a misconfiguration that
            // overflows is a bug we want to surface in dev rather than
            // silently saturate.
            debug_assert!(
                floor <= u64::MAX as u128,
                "multiplier-floor overflow: floor={floor} num={num} den={den} q_standard={q_standard}"
            );
            let new_q = u64::try_from(floor).unwrap_or(u64::MAX);
            self.priority.set_quote_for_floor(new_q);
        }
    }
}

impl PricingBackend for TwoLanePricing {
    fn current_quote(&self, lane: Lane) -> u64 {
        match lane {
            Lane::Standard => self.standard.current_quote(Lane::Standard),
            Lane::Priority => self.priority.current_quote(Lane::Priority),
        }
    }

    fn worst_case_quote_at(&self, lane: Lane, blocks_ahead: u32) -> u64 {
        match lane {
            // Priority-only-static variants pin c_standard at 1 — the
            // standard controller never moves. Worst case is the current
            // quote.
            Lane::Standard if !self.settings.variant.standard_dynamic() => {
                self.standard.current_quote(Lane::Standard)
            }
            Lane::Standard => self.standard.worst_case_quote_at(Lane::Standard, blocks_ahead),
            Lane::Priority => self.priority.worst_case_quote_at(Lane::Priority, blocks_ahead),
        }
    }

    fn update_after_block(&mut self, samples: &[PricedBlockSample]) {
        // Each sample carries the lane it feeds. The single-lane
        // backend filtered by `controller_lane == Standard`; here we
        // route Priority samples to the priority controller and (if
        // the standard side is dynamic) Standard samples to the
        // standard controller. Priority-only variants ignore Standard
        // samples even if the simulator emitted them — the spec keeps
        // c_standard = 1.
        //
        // Implementation: feed each controller its own filtered slice
        // through `update_after_block`, which already honours its
        // `controller_lane` filter (priority controller only consumes
        // Priority samples; standard controller's existing impl filters
        // Standard).
        //
        // Two issues with reusing the single-lane impl directly:
        //  1. `Eip1559Pricing::update_after_block` filters on
        //     `controller_lane == Lane::Standard` — we need a Priority
        //     filter for the priority controller.
        //  2. Priority-only variants must skip the standard controller
        //     entirely.
        //
        // We pass the slice and let each controller's `step_with_lane`
        // helper do its own filtering.
        self.priority
            .step_with_lane(Lane::Priority, samples);
        if self.settings.variant.standard_dynamic() {
            self.standard
                .step_with_lane(Lane::Standard, samples);
        }
        // Multiplier-floor enforcement after both controllers move.
        self.enforce_multiplier_floor();
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
            // RB-reserved RB: priority-only by validity rule. Only the
            // priority controller sees an RB sample (line 68); standard
            // controller is isolated even when it's dynamic.
            //
            // The selection-time validity filter guarantees every tx in
            // this RB has `posted_lane = Priority`. Assert that
            // invariant rather than silently summing both lanes — a
            // standard byte leaking through the filter is a regression
            // we want to fail loudly in dev, not absorb.
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
            // RB-reserved EB: priority controller's `relevant_bytes` is
            // capped at one RB-worth (plan line 73,
            // mechanism-design.md lines 168-180). Standard controller
            // (when dynamic) sees the EB's `standard_paying_bytes`.
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
            // Un-reserved (RB and EB share the same emission shape):
            // priority controller sees priority-paying-bytes against
            // block_capacity (option 1 signal); standard controller
            // (when dynamic) sees standard-paying-bytes against the
            // same. No partition, no per-block byte cap.
            (
                TwoLaneVariant::UnreservedPriorityOnly | TwoLaneVariant::UnreservedBothDynamic,
                _,
            ) => {
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

    fn snapshot(&self) -> PricingSnapshot {
        let p_snap = self.priority.snapshot();
        let s_snap = self.standard.snapshot();
        PricingSnapshot {
            standard_quote_per_byte: s_snap.standard_quote_per_byte,
            priority_quote_per_byte: Some(p_snap.standard_quote_per_byte),
            standard_window_util_x_1e9: s_snap.standard_window_util_x_1e9,
            priority_window_util_x_1e9: p_snap.standard_window_util_x_1e9,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Plan line 47: RB-reserved priority controller window length
        // 1 reduces to per-block fill rate.
        let pricing = TwoLanePricing::new(settings(TwoLaneVariant::RbReservedPriorityOnly)).unwrap();
        assert_eq!(pricing.priority.window().length(), 1);
    }

    #[test]
    fn unreserved_keeps_priority_window_length_from_settings() {
        let pricing = TwoLanePricing::new(settings(TwoLaneVariant::UnreservedPriorityOnly)).unwrap();
        assert_eq!(pricing.priority.window().length(), 4);
    }

    #[test]
    fn priority_only_variant_pins_standard_at_min_fee_a() {
        // Standard side is c = 1 (mechanism-design.md
        // §"RB-reserved priority-only premium").
        let mut pricing =
            TwoLanePricing::new(settings(TwoLaneVariant::RbReservedPriorityOnly)).unwrap();
        let q0 = pricing.current_quote(Lane::Standard);
        // Feed a saturated standard sample directly — variant must
        // ignore it and keep c_standard = 1 (q = min_fee_a).
        pricing.update_after_block(&[PricedBlockSample {
            block_kind: BlockKind::EndorserBlock,
            controller_lane: Lane::Standard,
            relevant_bytes: 100,
            relevant_capacity: 100,
        }]);
        let q1 = pricing.current_quote(Lane::Standard);
        assert_eq!(q0, 44);
        assert_eq!(q1, 44, "priority-only variant must not move c_standard");
    }

    #[test]
    fn multiplier_floor_holds_at_construction() {
        // Default settings put multiplier_floor at 16; initial priority
        // quote is min_fee_a = 44 = standard quote, so the floor
        // forces priority up to 16 × 44 = 704.
        let pricing = TwoLanePricing::new(settings(TwoLaneVariant::RbReservedBothDynamic)).unwrap();
        let q_p = pricing.current_quote(Lane::Priority);
        let q_s = pricing.current_quote(Lane::Standard);
        assert_eq!(q_s, 44);
        assert_eq!(q_p, 16 * 44);
    }

    #[test]
    fn multiplier_floor_holds_after_standard_moves_up() {
        // Plan line 312: multiplier-floor invariant after every
        // controller update, including when c_standard moves.
        let mut pricing =
            TwoLanePricing::new(settings(TwoLaneVariant::RbReservedBothDynamic)).unwrap();
        // Drive c_standard up: feed a saturated standard EB sample.
        // RB-reserved both-dynamic emits a standard EB sample, so we
        // can pass it through update_after_block.
        let sample = PricedBlockSample {
            block_kind: BlockKind::EndorserBlock,
            controller_lane: Lane::Standard,
            relevant_bytes: 100,
            relevant_capacity: 100,
        };
        for _ in 0..20 {
            pricing.update_after_block(&[sample]);
            let q_s = pricing.current_quote(Lane::Standard);
            let q_p = pricing.current_quote(Lane::Priority);
            // Floor: q_p ≥ ceil(16 × q_s / 1) = 16 × q_s.
            let floor = (16u128 * q_s as u128) as u64;
            assert!(
                q_p >= floor,
                "multiplier-floor violation: q_p={q_p} q_s={q_s} floor={floor}"
            );
        }
    }

    #[test]
    fn rb_reserved_only_emits_priority_sample_for_rb() {
        // Plan line 68 + verification line 313: standard controller
        // does not see RB samples even when both-dynamic.
        let pricing = TwoLanePricing::new(settings(TwoLaneVariant::RbReservedBothDynamic)).unwrap();
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
        // Plan line 73: RB-reserved priority controller's EB sample uses
        // `relevant_bytes = min(priority_paying_bytes, max_block_size)`.
        // Saturating priority demand cannot push the signal above 1.0.
        let pricing = TwoLanePricing::new(settings(TwoLaneVariant::RbReservedPriorityOnly)).unwrap();
        let breakdown = BlockLaneBreakdown {
            priority_paying_bytes: 1_000_000, // way over one RB-worth
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
        let pricing = TwoLanePricing::new(settings(TwoLaneVariant::RbReservedBothDynamic)).unwrap();
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
        // Plan line 313: a saturated priority-only RB updates
        // c_priority but does **not** change c_standard or its window.
        let mut pricing =
            TwoLanePricing::new(settings(TwoLaneVariant::RbReservedBothDynamic)).unwrap();
        let q_s_before = pricing.current_quote(Lane::Standard);
        let standard_window_bytes_before = pricing.standard.window().sum_bytes();
        let breakdown = BlockLaneBreakdown {
            priority_paying_bytes: 90_000,
            standard_paying_bytes: 0,
            block_capacity: 90_000,
        };
        let samples = pricing.samples_for_block(BlockKind::RankingBlock, &breakdown);
        // Confirm sample emission shape first.
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].controller_lane, Lane::Priority);
        // Now apply.
        pricing.update_after_block(&samples);
        let q_s_after = pricing.current_quote(Lane::Standard);
        let standard_window_bytes_after = pricing.standard.window().sum_bytes();
        assert_eq!(
            q_s_after, q_s_before,
            "saturated priority-only RB must not move c_standard"
        );
        assert_eq!(
            standard_window_bytes_after, standard_window_bytes_before,
            "saturated priority-only RB must not feed the standard window"
        );
    }

    #[test]
    fn rejects_zero_denominator_floor() {
        let mut s = settings(TwoLaneVariant::RbReservedBothDynamic);
        // Multiplier::new rejects denominator==0; bypass via direct
        // construction to test TwoLaneSettings::validate.
        s.multiplier_floor = Multiplier {
            numerator: 16,
            denominator: 0,
        };
        assert!(TwoLanePricing::new(s).is_err());
    }

    #[test]
    fn rejects_floor_below_one() {
        let mut s = settings(TwoLaneVariant::RbReservedBothDynamic);
        s.multiplier_floor = Multiplier::new(1, 2).unwrap(); // 0.5 < 1
        assert!(TwoLanePricing::new(s).is_err());
    }
}
