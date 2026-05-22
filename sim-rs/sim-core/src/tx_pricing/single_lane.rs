//! Single-lane pricing backends (chain-derived; spike 007).
//!
//! - `BaselinePricing`: flat fee, `c = 1`. Reference for tests, default
//!   when no pricing config is supplied.
//! - `Eip1559Pricing`: one EIP-1559 controller fed by a per-block
//!   capacity-weighted-window aggregate carried on the chain. Implements
//!   the spec's clamp formula and era-floor rule (`c ≥ 1`) via
//!   integer/rational arithmetic.
//!
//! Under chain-derivation, neither backend holds any controller state
//! — they are pure-function policies. The `compute_derived_quote`
//! method takes the parent block's quote + aggregate + samples and
//! returns the child block's quote + aggregate. All math is `u64` /
//! `u128`; f64 never appears.
//!
//! **Memoisation note** (spike 007 §"Edge cases" item 1): the spike
//! called for a per-`BlockId` cache to avoid recomputing
//! `derived_quote` on every revisit. Under our architecture the block's
//! `derived_quote` field IS the cache — block fields are O(1) lookup.
//! The simulator (`linear_leios.rs`) also maintains a per-block
//! `samples_in_block` cache, pruned at `2 × window_length` behind the
//! chain tip. No separate backend-level cache is needed.
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

use crate::model::{PerLaneQuote, WindowAggregate};

use super::{
    BlockKind, Lane, LaneSelectionOrder, LaneValidityRule, PricedBlockSample, PricingBackend,
};

/// Flat-fee backend. `c = 1`, so `quote_per_byte = minFeeA`.
#[derive(Debug, Clone)]
pub struct BaselinePricing {
    min_fee_a: u64,
}

impl BaselinePricing {
    pub fn new(min_fee_a: u64) -> Self {
        Self {
            min_fee_a: min_fee_a.max(1),
        }
    }

    pub fn min_fee_a(&self) -> u64 {
        self.min_fee_a
    }
}

impl PricingBackend for BaselinePricing {
    fn compute_derived_quote(
        &self,
        _parent_quote: PerLaneQuote,
        _parent_aggregate: WindowAggregate,
        _parent_samples: &[PricedBlockSample],
        _evicted_samples: &[PricedBlockSample],
    ) -> (PerLaneQuote, WindowAggregate) {
        // Flat-fee policy: ignore everything, return the pinned quote.
        (PerLaneQuote::flat(self.min_fee_a), WindowAggregate::ZERO)
    }

    fn effective_window_length(&self) -> usize {
        // Baseline has no window.
        usize::MAX
    }

    fn cold_start_quote(&self, _lane: Lane) -> u64 {
        self.min_fee_a
    }

    fn lane_validity_rule(&self, _block_kind: BlockKind) -> LaneValidityRule {
        LaneValidityRule::None
    }

    fn lane_selection_order(&self) -> LaneSelectionOrder {
        LaneSelectionOrder::Fifo
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
        // Validate that the controller-parameter intermediates fit in u128
        // for any per-sample bytes value up to a conservative cap. The
        // worst u128 intermediate in `compute_eip1559_step` is
        //   q × (move_den + move_num)   where
        //   move_den = util_den × target_num × max_change_denominator
        //   util_den ≤ window_length × MAX_BYTES_PER_SAMPLE
        //   move_num ≤ move_den
        //   q ≤ u64::MAX
        // We bound MAX_BYTES_PER_SAMPLE at 2^40 (≈ 1 TiB per block — well
        // above realistic block-body caps of ~12 MB) and require that
        //   u64::MAX × 2 × window_length × 2^40 × target_num × D ≤ u128::MAX
        // Equivalently, window_length × target_num × D ≤ 2^23.
        // This catches pathological controller settings (huge D, huge
        // target_num, huge window_length) at construction. Sample-time
        // bytes that exceed MAX_BYTES_PER_SAMPLE still saturate as a
        // belt-and-braces fallback (see `compute_eip1559_step`).
        const MAX_BYTES_PER_SAMPLE_LOG2: u32 = 40;
        const SAFETY_HEADROOM_LOG2: u32 = 1; // factor 2 for (move_den + move_num)
        let log2_budget: u32 = 128 - 64 - MAX_BYTES_PER_SAMPLE_LOG2 - SAFETY_HEADROOM_LOG2;
        let budget: u128 = 1u128 << log2_budget;
        let product = (self.window_length as u128)
            .checked_mul(self.target_num as u128)
            .and_then(|x| x.checked_mul(self.max_change_denominator as u128));
        match product {
            Some(p) if p <= budget => {}
            _ => anyhow::bail!(
                "Eip1559Settings: controller intermediates may overflow u128: \
                 window_length ({}) × target_num ({}) × max_change_denominator ({}) \
                 must be ≤ 2^{} = {}; got product {:?}",
                self.window_length,
                self.target_num,
                self.max_change_denominator,
                log2_budget,
                budget,
                product,
            ),
        }
        Ok(())
    }
}

/// Single EIP-1559 controller — stateless policy under chain-derivation.
/// The struct is purely a settings carrier; all math is in pure
/// free functions (`compute_eip1559_step`,
/// `worst_case_eip1559_quote`) consuming `parent_quote` + the chain-
/// derived `WindowAggregate`.
#[derive(Debug, Clone)]
pub struct Eip1559Pricing {
    settings: Eip1559Settings,
}

impl Eip1559Pricing {
    pub fn new(settings: Eip1559Settings) -> anyhow::Result<Self> {
        settings.validate()?;
        Ok(Self { settings })
    }

    pub fn settings(&self) -> &Eip1559Settings {
        &self.settings
    }
}

/// Pure EIP-1559 step. Returns the post-step `quote_per_byte` from the
/// parent's quote and the new aggregate utilisation `(util_num, util_den)`.
/// Identical math to the legacy `Eip1559Pricing::step`, but with explicit
/// inputs — no `self.window` / `self.quote_per_byte` reads.
///
/// Preserves the WR-4 overflow bounds (validated by
/// `Eip1559Settings::validate`) and IN-2's "div-by-D is exact because
/// D | den by construction" proof.
pub fn compute_eip1559_step(
    parent_quote: u64,
    util: (u128, u128),
    settings: &Eip1559Settings,
) -> u64 {
    let (util_num, util_den) = util;
    // Empty window or zero capacity: no signal, no movement. Per the
    // legacy `Eip1559Pricing::step` semantics, the controller does not
    // step at all when no samples have flowed through (mempool would
    // be drifting toward zero otherwise). The `WindowAggregate`'s
    // `aggregate_util` returns `(0, 1)` in the empty case; callers who
    // pre-filter "no samples" should pass `(0, 0)` to trigger this
    // early return.
    if util_den == 0 {
        return parent_quote;
    }
    let target_num = settings.target_num as u128;
    let target_den = settings.target_den as u128;
    let d = settings.max_change_denominator as u128;

    // signal_num/signal_den = (aggregateUtil − target) / (target · D)
    //   = (util_num · target_den − target_num · util_den)
    //     / (util_den · target_num · D)
    //
    // `Eip1559Settings::validate` ensures
    //   window_length × target_num × max_change_denominator
    // is bounded so any per-sample bytes value up to 2^40 keeps these
    // u128 intermediates in range. The `saturating_mul`s below are
    // belt-and-braces only: a fatal misconfig would have failed
    // earlier in `validate`; a runtime sample exceeding the 2^40
    // bytes assumption would saturate here (a far less likely
    // failure mode, but harmless).
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
    // D | den by construction (den = util_den · target_num · D),
    // so `den / D = util_den · target_num` is exact.
    let max_step_num = den / d;
    let mut move_num = signal_abs_num;
    let move_den = den;
    // If move_num > max_step_num, clamp.
    if move_num > max_step_num {
        move_num = max_step_num;
    }

    // new_quote = quote · (move_den ± move_num) / move_den, with the
    // sign of `signal_positive`. Use `u128` to avoid overflow.
    let q = parent_quote as u128;
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
    let floor = settings.min_fee_a as u128;
    let clamped = new_quote.max(floor);
    // Saturate to u64 bounds defensively.
    u64::try_from(clamped).unwrap_or(u64::MAX)
}

/// Compute the worst-case EIP-1559 per-byte rate after `n` consecutive
/// max-up controller steps. Each step multiplies the quote by
/// `(D+1)/D` (the spec's `+1/D` clamp). Capped at u64::MAX on overflow
/// and at `MAX_PROJECTION_BLOCKS` so the exponentiation stays in u128.
///
/// Producers use this at EB-build time to skip txs whose
/// `max_fee_lovelace` won't survive the worst-case drift over the
/// endorsement window. Under chain-derivation the input
/// `current_quote` comes from the chain tip's `derived_quote`, not
/// from a per-node accumulator.
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

/// Free-function projection wrapper that's lane-aware so callers in
/// `linear_leios.rs`' staleness predictor can pass a `Lane` without
/// caring about backend introspection. For single-lane backends both
/// lanes resolve to the same controller — for two-lane backends the
/// caller should route per-lane via `TwoLanePricing::worst_case_quote_for`.
pub fn worst_case_quote_at(
    current_quote: u64,
    settings: &Eip1559Settings,
    _lane: Lane,
    blocks_ahead: u32,
) -> u64 {
    worst_case_eip1559_quote(current_quote, settings.max_change_denominator, blocks_ahead)
}

impl PricingBackend for Eip1559Pricing {
    fn compute_derived_quote(
        &self,
        parent_quote: PerLaneQuote,
        parent_aggregate: WindowAggregate,
        parent_samples: &[PricedBlockSample],
        evicted_samples: &[PricedBlockSample],
    ) -> (PerLaneQuote, WindowAggregate) {
        // Single-lane: only Standard-lane samples matter.
        let filter_standard = |s: &&PricedBlockSample| s.controller_lane == Lane::Standard;
        let add: Vec<PricedBlockSample> = parent_samples
            .iter()
            .filter(filter_standard)
            .copied()
            .collect();
        let evict: Vec<PricedBlockSample> = evicted_samples
            .iter()
            .filter(filter_standard)
            .copied()
            .collect();
        let new_aggregate = super::window::update_aggregate(
            parent_aggregate,
            &add,
            &evict,
            self.settings.window_length,
        );
        // If the controller has seen no samples yet (cold-start
        // genesis, or a stretch of endorsement-only RBs), the step
        // semantics from the legacy `Eip1559Pricing::step` skip the
        // update entirely. Mirror that here by short-circuiting to
        // parent_quote when blocks_in_window is 0.
        let new_quote = if new_aggregate.blocks_in_window == 0 {
            parent_quote.standard
        } else {
            let (util_num, util_den) = new_aggregate.aggregate_util(Lane::Standard);
            compute_eip1559_step(parent_quote.standard, (util_num, util_den), &self.settings)
        };
        // Single-lane: both lanes share the controller, so callers
        // reading either lane via `PerLaneQuote::get` see the right
        // value.
        (PerLaneQuote::flat(new_quote), new_aggregate)
    }

    fn effective_window_length(&self) -> usize {
        self.settings.window_length
    }

    fn cold_start_quote(&self, _lane: Lane) -> u64 {
        self.settings
            .initial_quote_per_byte
            .max(self.settings.min_fee_a)
    }

    fn lane_validity_rule(&self, _block_kind: BlockKind) -> LaneValidityRule {
        LaneValidityRule::None
    }

    fn lane_selection_order(&self) -> LaneSelectionOrder {
        LaneSelectionOrder::Fifo
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{PerLaneQuote, WindowAggregate};
    use crate::tx_pricing::{BlockKind, Lane, PricedBlockSample, PricingBackend};

    use super::{BaselinePricing, Eip1559Pricing, Eip1559Settings, compute_eip1559_step};

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
        let (q, _) = pricing.compute_derived_quote(
            PerLaneQuote::flat(44),
            WindowAggregate::ZERO,
            &[standard_rb(90_000, 90_000)],
            &[],
        );
        assert_eq!(q.standard, 44);
        assert_eq!(q.priority, 44);
    }

    #[test]
    fn baseline_pricing_does_not_drift() {
        let pricing = BaselinePricing::new(44);
        let (q1, agg1) = pricing.compute_derived_quote(
            PerLaneQuote::flat(44),
            WindowAggregate::ZERO,
            &[standard_rb(90_000, 90_000)],
            &[],
        );
        let (q2, _) = pricing.compute_derived_quote(q1, agg1, &[standard_rb(100, 100)], &[]);
        assert_eq!(q2.standard, 44);
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
        let pricing = Eip1559Pricing::new(settings(1000, 8)).unwrap();
        // util = 0.5 = target.
        let (q, _) = pricing.compute_derived_quote(
            PerLaneQuote::flat(1000),
            WindowAggregate::ZERO,
            &[standard_rb(50, 100)],
            &[],
        );
        assert_eq!(q.standard, 1000);
    }

    #[test]
    fn eip1559_above_target_moves_up_within_step_clamp() {
        let pricing = Eip1559Pricing::new(settings(1000, 8)).unwrap();
        // Saturated block: util = 1.0 = target + 0.5 = 100% above target.
        // Per-step move clamped at +1/D = +12.5%, so quote ≤ 1125.
        let (q, _) = pricing.compute_derived_quote(
            PerLaneQuote::flat(1000),
            WindowAggregate::ZERO,
            &[standard_rb(100, 100)],
            &[],
        );
        assert!(
            q.standard > 1000,
            "expected upward move, got {}",
            q.standard
        );
        assert!(
            q.standard <= 1125,
            "expected ≤ +12.5% clamp, got {}",
            q.standard
        );
    }

    #[test]
    fn eip1559_below_target_moves_down_within_step_clamp() {
        let pricing = Eip1559Pricing::new(settings(1000, 8)).unwrap();
        // Empty block: util = 0 = target − 0.5 = 100% below target.
        // Per-step move clamped at -1/D = -12.5%, so quote ≥ 875.
        let (q, _) = pricing.compute_derived_quote(
            PerLaneQuote::flat(1000),
            WindowAggregate::ZERO,
            &[standard_rb(0, 100)],
            &[],
        );
        assert!(
            q.standard < 1000,
            "expected downward move, got {}",
            q.standard
        );
        assert!(
            q.standard >= 875,
            "expected ≥ -12.5% clamp, got {}",
            q.standard
        );
    }

    #[test]
    fn eip1559_floor_at_min_fee_a() {
        // Run many empty-block updates and confirm we floor at min_fee_a.
        let pricing = Eip1559Pricing::new(settings(100, 4)).unwrap();
        let mut q = PerLaneQuote::flat(100);
        let mut agg = WindowAggregate::ZERO;
        for _ in 0..200 {
            let (nq, na) = pricing.compute_derived_quote(q, agg, &[standard_rb(0, 100)], &[]);
            q = nq;
            agg = na;
        }
        assert_eq!(q.standard, 44);
    }

    #[test]
    fn eip1559_uses_ceil_rounding_per_spec() {
        // implementation-plan.md line 175: "Final `quote_per_byte` is
        // integer-rounded once per update via `ceil` against `minFeeA`
        // for the era floor."
        //
        // Concrete: minFeeA = 44, target 0.5, D = 8, saturated block.
        // 44 × 1.125 = 49.5; spec ceil → 50, not floor 49.
        let pricing = Eip1559Pricing::new(Eip1559Settings {
            min_fee_a: 44,
            initial_quote_per_byte: 44,
            target_num: 1,
            target_den: 2,
            max_change_denominator: 8,
            window_length: 1,
        })
        .unwrap();
        let (q, _) = pricing.compute_derived_quote(
            PerLaneQuote::flat(44),
            WindowAggregate::ZERO,
            &[standard_rb(100, 100)],
            &[],
        );
        assert_eq!(
            q.standard, 50,
            "spec rounding regime is ceil; floor would give 49 and \
             smaller above-target moves would stick at the old quote"
        );
    }

    #[test]
    fn eip1559_quote_drift_under_sustained_saturation() {
        // Smoke-style: under sustained saturated demand, quote rises
        // monotonically and well above the initial value.
        let pricing = Eip1559Pricing::new(settings(1000, 8)).unwrap();
        let mut q = PerLaneQuote::flat(1000);
        let mut agg = WindowAggregate::ZERO;
        let mut last = 1000u64;
        for _ in 0..30 {
            let (nq, na) = pricing.compute_derived_quote(q, agg, &[standard_rb(100, 100)], &[]);
            assert!(
                nq.standard >= last,
                "quote regressed: {last} -> {}",
                nq.standard
            );
            last = nq.standard;
            q = nq;
            agg = na;
        }
        assert!(
            last > 1500,
            "expected sustained drift to push quote well above 1500, got {last}"
        );
    }

    #[test]
    fn compute_eip1559_step_is_pure() {
        let s = settings(1000, 8);
        let a = compute_eip1559_step(1000, (50, 100), &s);
        let b = compute_eip1559_step(1000, (50, 100), &s);
        assert_eq!(a, b);
    }
}
