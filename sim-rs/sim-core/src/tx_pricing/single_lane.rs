//! Single-lane pricing backends.
//!
//! - `BaselinePricing`: flat fee, `c = 1`. Reference for tests, default
//!   when no pricing config is supplied.
//! - `Eip1559Pricing`: one EIP-1559 controller driven by
//!   `CapacityWeightedWindow`. Implements the spec's clamp formula and
//!   era-floor rule (`c ≥ 1`) via integer/rational arithmetic.
//!
//! All controller state is `u64` (`quote_per_byte`) and `u128` rationals
//! (intermediate update math). f64 never appears.
//!
//! Update rule (mechanism-design.md lines 119-124, implementation-plan.md
//! lines 174-176):
//!
//! ```text
//! c ← c · (1 + clamp((aggregateUtil − target)/(target · D), ±1/D))
//! ```
//!
//! floored at `c ≥ 1`. The implementation never materialises `c` as f64;
//! it works directly on `quote_per_byte = max(minFeeA, ceil(c · minFeeA))`
//! and applies the same fractional move to that integer value.

use super::{
    BlockKind, CapacityWeightedWindow, Lane, LaneSelectionOrder, LaneValidityRule, PricedBlockSample,
    PricingBackend, PricingSnapshot,
};

/// Flat-fee backend. `c = 1`, so `quote_per_byte = minFeeA`.
#[derive(Debug, Clone)]
pub struct BaselinePricing {
    quote_per_byte: u64,
}

impl BaselinePricing {
    pub fn new(min_fee_a: u64) -> Self {
        Self {
            quote_per_byte: min_fee_a.max(1),
        }
    }
}

impl PricingBackend for BaselinePricing {
    fn current_quote(&self, _lane: Lane) -> u64 {
        self.quote_per_byte
    }

    fn update_after_block(&mut self, _samples: &[PricedBlockSample]) {
        // Flat fee: nothing to update.
    }

    fn lane_validity_rule(&self, _block_kind: BlockKind) -> LaneValidityRule {
        LaneValidityRule::None
    }

    fn lane_selection_order(&self) -> LaneSelectionOrder {
        LaneSelectionOrder::Fifo
    }

    fn snapshot(&self) -> PricingSnapshot {
        PricingSnapshot {
            standard_quote_per_byte: self.quote_per_byte,
            priority_quote_per_byte: None,
            standard_window_util_x_1e9: None,
            priority_window_util_x_1e9: None,
        }
    }
}

/// Settings for a single EIP-1559 controller.
///
/// `target` is a rational `(num, den)` (e.g. `(1, 2)` for 0.5). `D` is the
/// max-change denominator (`±1/D` per-step bound). All values are integer
/// or rational.
#[derive(Debug, Clone)]
pub struct Eip1559Settings {
    pub min_fee_a: u64,
    pub initial_quote_per_byte: u64,
    /// `target = target_num / target_den`. Must satisfy
    /// `0 < target_num < target_den` for a meaningful target in (0, 1).
    pub target_num: u64,
    pub target_den: u64,
    /// Per-step max change denominator: bounded at `±1/D`.
    pub max_change_denominator: u64,
    /// Capacity-weighted window length.
    pub window_length: usize,
}

impl Eip1559Settings {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.min_fee_a == 0 {
            anyhow::bail!("Eip1559Settings.min_fee_a must be non-zero");
        }
        if self.target_den == 0 {
            anyhow::bail!("Eip1559Settings.target_den must be non-zero");
        }
        if self.target_num == 0 || self.target_num >= self.target_den {
            anyhow::bail!(
                "Eip1559Settings.target_num/target_den must be a fraction in (0, 1); got {}/{}",
                self.target_num,
                self.target_den
            );
        }
        if self.max_change_denominator == 0 {
            anyhow::bail!("Eip1559Settings.max_change_denominator must be non-zero");
        }
        if self.window_length == 0 {
            anyhow::bail!("Eip1559Settings.window_length must be non-zero");
        }
        Ok(())
    }
}

/// Single EIP-1559 controller fed by a `CapacityWeightedWindow`.
#[derive(Debug, Clone)]
pub struct Eip1559Pricing {
    settings: Eip1559Settings,
    window: CapacityWeightedWindow,
    quote_per_byte: u64,
}

impl Eip1559Pricing {
    pub fn new(settings: Eip1559Settings) -> anyhow::Result<Self> {
        settings.validate()?;
        let window = CapacityWeightedWindow::new(settings.window_length)?;
        let quote_per_byte = settings.initial_quote_per_byte.max(settings.min_fee_a);
        Ok(Self {
            settings,
            window,
            quote_per_byte,
        })
    }

    pub fn settings(&self) -> &Eip1559Settings {
        &self.settings
    }

    pub fn window(&self) -> &CapacityWeightedWindow {
        &self.window
    }

    /// Run an EIP-1559 step using only samples whose `controller_lane`
    /// matches `lane`. Used by `TwoLanePricing` to feed two
    /// independent controllers from the same priced-block sample
    /// vector (each variant emits at most one sample per controller
    /// per block; the filter is what routes them).
    pub fn step_with_lane(&mut self, lane: Lane, samples: &[PricedBlockSample]) {
        for sample in samples.iter().filter(|s| s.controller_lane == lane) {
            self.window.push(*sample);
        }
        self.step();
    }

    /// Overwrite `quote_per_byte` to enforce the multiplier-floor
    /// invariant. The controller's window is **not** touched —
    /// invariant enforcement is policy layered on top of the
    /// controller's independent step (mechanism-design.md
    /// §"RB-reserved priority-only premium" closing paragraph;
    /// implementation-plan.md line 38).
    ///
    /// Used only from `TwoLanePricing::enforce_multiplier_floor`.
    pub fn set_quote_for_floor(&mut self, quote_per_byte: u64) {
        self.quote_per_byte = quote_per_byte.max(self.settings.min_fee_a);
    }

    /// Apply the EIP-1559 step. Operates on the integer `quote_per_byte`
    /// directly: the same fractional move would apply to either `c` or
    /// `quote = c · minFeeA`, so we keep the `u64` and round once.
    fn step(&mut self) {
        if self.window.samples_len() == 0 {
            return;
        }
        let (util_num, util_den) = self.window.aggregate_util();
        if util_den == 0 {
            return;
        }
        let target_num = self.settings.target_num as u128;
        let target_den = self.settings.target_den as u128;
        let d = self.settings.max_change_denominator as u128;

        // signal_num/signal_den = (aggregateUtil − target) / (target · D)
        //   = (util_num · target_den − target_num · util_den)
        //     / (util_den · target_num · D)
        //
        // u128 holds these for any realistic config (window length 32 ×
        // u64 bytes ≈ 2^69; target/D parameters single-digit). Saturate
        // in release as a defensive backstop, but flag pathological
        // inputs in dev so we don't silently mask a real bug behind a
        // saturated max value.
        debug_assert!(util_num.checked_mul(target_den).is_some());
        debug_assert!(target_num.checked_mul(util_den).is_some());
        debug_assert!(
            util_den
                .checked_mul(target_num)
                .and_then(|x| x.checked_mul(d))
                .is_some()
        );
        let num_a = util_num.saturating_mul(target_den);
        let num_b = target_num.saturating_mul(util_den);
        let den = util_den.saturating_mul(target_num).saturating_mul(d);
        let signal_positive = num_a >= num_b;
        let signal_abs_num = if signal_positive {
            num_a - num_b
        } else {
            num_b - num_a
        };

        // Clamp at ±1/D: |signal| ≤ 1/D
        // i.e. signal_abs_num · D ≤ den (since 1/D = den/(D·den))
        // We compute the post-clamp move as a rational
        // (move_num/move_den) where move_den = den.
        // 1/D as rational over `den`: numerator = den / D = den / D.
        // For safety we clamp by comparing signal_abs_num against den/D,
        // i.e. signal_abs_num · D against den.
        let max_step_num = den / d; // den / D, integer division (safe: D|den_val? not always)
        let mut move_num = signal_abs_num;
        let move_den = den;
        // If move_num > max_step_num, clamp.
        if move_num > max_step_num {
            move_num = max_step_num;
        }

        // new_quote = quote · (move_den ± move_num) / move_den, with the
        // sign of `signal_positive`. Use `u128` to avoid overflow.
        let q = self.quote_per_byte as u128;
        let new_quote_num = if signal_positive {
            q.saturating_mul(move_den.saturating_add(move_num))
        } else {
            q.saturating_mul(move_den.saturating_sub(move_num))
        };
        // Ceiling division per the spec rounding regime
        // (implementation-plan.md line 175: "Final `quote_per_byte` is
        // integer-rounded once per update via `ceil`"). Floor would let
        // small above-target moves stick at the old quote, e.g.
        // `44 × 1.125 = 49.5 → floor 49` (no movement past 50) where
        // the spec specifies 50.
        let new_quote = if new_quote_num == 0 {
            0
        } else {
            (new_quote_num - 1) / move_den + 1
        };

        // Era floor: c ≥ 1, i.e. quote ≥ min_fee_a.
        let floor = self.settings.min_fee_a as u128;
        let clamped = new_quote.max(floor);
        // Saturate to u64 bounds defensively.
        self.quote_per_byte = u64::try_from(clamped).unwrap_or(u64::MAX);
    }
}

/// Compute the worst-case EIP-1559 per-byte rate after `n` consecutive
/// max-up controller steps. Each step multiplies the quote by
/// `(D+1)/D` (the spec's `+1/D` clamp). Capped at u64::MAX on overflow
/// and at `MAX_PROJECTION_BLOCKS` so the exponentiation stays in u128.
///
/// Producers use this at EB-build time to skip txs whose
/// `max_fee_lovelace` won't survive the worst-case drift over the
/// endorsement window.
pub fn worst_case_eip1559_quote(current_quote: u64, d: u64, blocks_ahead: u32) -> u64 {
    if d == 0 || blocks_ahead == 0 {
        return current_quote;
    }
    // Cap N to keep ((D+1)/D)^N tractable in u128. For typical D ≥ 4
    // and N ≤ 32, (D+1)^32 fits comfortably in u128. Beyond that the
    // worst-case bound is dominated by other failure modes anyway
    // (mempool overflow, run-end truncation).
    const MAX_PROJECTION_BLOCKS: u32 = 32;
    let n = blocks_ahead.min(MAX_PROJECTION_BLOCKS);
    let num = match (d as u128 + 1).checked_pow(n) {
        Some(v) => v,
        None => return u64::MAX,
    };
    let den = match (d as u128).checked_pow(n) {
        Some(v) => v,
        None => return u64::MAX,
    };
    // ⌈current_quote × num / den⌉, saturating to u64::MAX.
    let numerator = (current_quote as u128).saturating_mul(num);
    let projected = if numerator == 0 {
        0u128
    } else {
        1 + (numerator - 1) / den
    };
    u64::try_from(projected).unwrap_or(u64::MAX)
}

impl PricingBackend for Eip1559Pricing {
    fn current_quote(&self, _lane: Lane) -> u64 {
        self.quote_per_byte
    }

    fn worst_case_quote_at(&self, _lane: Lane, blocks_ahead: u32) -> u64 {
        worst_case_eip1559_quote(self.quote_per_byte, self.settings.max_change_denominator, blocks_ahead)
    }

    fn update_after_block(&mut self, samples: &[PricedBlockSample]) {
        // Single-lane: aggregate every Standard sample into the window.
        for sample in samples
            .iter()
            .filter(|s| s.controller_lane == Lane::Standard)
        {
            self.window.push(*sample);
        }
        self.step();
    }

    fn lane_validity_rule(&self, _block_kind: BlockKind) -> LaneValidityRule {
        LaneValidityRule::None
    }

    fn lane_selection_order(&self) -> LaneSelectionOrder {
        LaneSelectionOrder::Fifo
    }

    fn snapshot(&self) -> PricingSnapshot {
        let (num, den) = self.window.aggregate_util();
        let util_x_1e9 = if den == 0 {
            None
        } else {
            // util as fixed-point ×1e9, capped at u64::MAX
            let scaled = num.saturating_mul(1_000_000_000) / den;
            Some(u64::try_from(scaled).unwrap_or(u64::MAX))
        };
        PricingSnapshot {
            standard_quote_per_byte: self.quote_per_byte,
            priority_quote_per_byte: None,
            standard_window_util_x_1e9: util_x_1e9,
            priority_window_util_x_1e9: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tx_pricing::{BlockKind, Lane, PricedBlockSample, PricingBackend};

    use super::{BaselinePricing, Eip1559Pricing, Eip1559Settings};

    fn standard_rb(bytes: u64, capacity: u64) -> PricedBlockSample {
        PricedBlockSample {
            block_kind: BlockKind::RankingBlock,
            controller_lane: Lane::Standard,
            relevant_bytes: bytes,
            relevant_capacity: capacity,
        }
    }

    #[test]
    fn baseline_pricing_returns_min_fee_a() {
        let pricing = BaselinePricing::new(44);
        assert_eq!(pricing.current_quote(Lane::Standard), 44);
    }

    #[test]
    fn baseline_pricing_does_not_drift() {
        let mut pricing = BaselinePricing::new(44);
        pricing.update_after_block(&[standard_rb(90_000, 90_000)]);
        assert_eq!(pricing.current_quote(Lane::Standard), 44);
    }

    fn settings(initial: u64, d: u64) -> Eip1559Settings {
        Eip1559Settings {
            min_fee_a: 44,
            initial_quote_per_byte: initial,
            target_num: 1,
            target_den: 2,
            max_change_denominator: d,
            window_length: 4,
        }
    }

    #[test]
    fn eip1559_at_target_does_not_move() {
        let mut pricing = Eip1559Pricing::new(settings(1000, 8)).unwrap();
        // util = 0.5 = target.
        pricing.update_after_block(&[standard_rb(50, 100)]);
        assert_eq!(pricing.current_quote(Lane::Standard), 1000);
    }

    #[test]
    fn eip1559_above_target_moves_up_within_step_clamp() {
        let mut pricing = Eip1559Pricing::new(settings(1000, 8)).unwrap();
        // Saturated block: util = 1.0 = target + 0.5 = 100% above target.
        // Per-step move clamped at +1/D = +12.5%, so quote ≤ 1125.
        pricing.update_after_block(&[standard_rb(100, 100)]);
        let q = pricing.current_quote(Lane::Standard);
        assert!(q > 1000, "expected upward move, got {q}");
        assert!(q <= 1125, "expected ≤ +12.5% clamp, got {q}");
    }

    #[test]
    fn eip1559_below_target_moves_down_within_step_clamp() {
        let mut pricing = Eip1559Pricing::new(settings(1000, 8)).unwrap();
        // Empty block: util = 0 = target − 0.5 = 100% below target.
        // Per-step move clamped at -1/D = -12.5%, so quote ≥ 875.
        pricing.update_after_block(&[standard_rb(0, 100)]);
        let q = pricing.current_quote(Lane::Standard);
        assert!(q < 1000, "expected downward move, got {q}");
        assert!(q >= 875, "expected ≥ -12.5% clamp, got {q}");
    }

    #[test]
    fn eip1559_floor_at_min_fee_a() {
        // Run many empty-block updates and confirm we floor at min_fee_a.
        let mut pricing = Eip1559Pricing::new(settings(100, 4)).unwrap();
        for _ in 0..200 {
            pricing.update_after_block(&[standard_rb(0, 100)]);
        }
        assert_eq!(pricing.current_quote(Lane::Standard), 44);
    }

    #[test]
    fn eip1559_uses_ceil_rounding_per_spec() {
        // implementation-plan.md line 175: "Final `quote_per_byte` is
        // integer-rounded once per update via `ceil` against `minFeeA`
        // for the era floor."
        //
        // Concrete: minFeeA = 44, target 0.5, D = 8, saturated block.
        // 44 × 1.125 = 49.5; spec ceil → 50, not floor 49.
        let mut pricing = Eip1559Pricing::new(Eip1559Settings {
            min_fee_a: 44,
            initial_quote_per_byte: 44,
            target_num: 1,
            target_den: 2,
            max_change_denominator: 8,
            window_length: 1,
        })
        .unwrap();
        pricing.update_after_block(&[standard_rb(100, 100)]);
        assert_eq!(
            pricing.current_quote(Lane::Standard),
            50,
            "spec rounding regime is ceil; floor would give 49 and \
             smaller above-target moves would stick at the old quote"
        );
    }

    #[test]
    fn eip1559_quote_drift_under_sustained_saturation() {
        // Smoke-style: under sustained saturated demand, quote rises
        // monotonically and well above the initial value. This is the
        // mechanic that lets the M1 smoke test produce evictions.
        let mut pricing = Eip1559Pricing::new(settings(1000, 8)).unwrap();
        let mut last = 1000u64;
        for _ in 0..30 {
            pricing.update_after_block(&[standard_rb(100, 100)]);
            let q = pricing.current_quote(Lane::Standard);
            assert!(q >= last, "quote regressed: {last} -> {q}");
            last = q;
        }
        assert!(
            last > 1500,
            "expected sustained drift to push quote well above 1500, got {last}"
        );
    }
}
