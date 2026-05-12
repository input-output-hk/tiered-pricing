//! Phase-2 actor model. M3.
//!
//! Provides the demand-side surface used by phase-2 suites:
//! - `ActorComponent` — one weighted profile component sampling
//!   `(bytes, value, urgency)` per arrival, choosing `posted_lane`,
//!   and computing `max_fee_lovelace` via a configurable policy.
//! - `ActorProfile` — list of components plus the
//!   `block_generation_probability` and `min_fee_b` constants the
//!   lane-choice math needs.
//! - `MaxFeePolicy::ScaledOverLaneQuote { numerator, denominator }`
//!   — rational policy producing
//!   `max_fee_lovelace = min_fee_b + ⌈quote × bytes × num / den⌉`,
//!   computed in `u128` and rounded with the overflow-safe
//!   `ceil_div_u128` from plan lines 138-143. **Validation at config
//!   load**: `denominator > 0`.
//! - `lane_choice::pick` — utility-maximising lane choice. Uses
//!   `libm::pow` for `urgency^(-latency_blocks)` and `libm::round`
//!   (round-half-away-from-zero) to round into `i128` lovelace
//!   before the `>` comparison. Bit-deterministic given identical
//!   inputs.
//! - `welfare` — f64 reporting-only formulas pinned by plan lines
//!   148-152: `retained_value`, `net_utility`, `retained_value_ratio`.
//!   Negative `net_utility` is preserved through all aggregation.
//! - `LatencyEstimator` — per-lane rolling-average inclusion-delay
//!   estimator (blocks).
//!
//! **Cross-arch caveat (inherited from M2).** `libm::pow` is
//! bit-stable given identical inputs, but those inputs (`urgency`,
//! `value_lovelace`, `bytes`) are sampled via `rand_distr`, whose
//! internals use `f64::ln`/`f64::exp` (not in IEEE-754's bit-exact
//! mandate). Sampling drift can still cause cross-arch divergence in
//! `posted_lane` through different inputs. The simulator's pricing
//! event-stream golden hash is asserted intra-arch only; multi-arch
//! verification is an M5/CI infrastructure task.

use std::collections::VecDeque;

use anyhow::{Result, anyhow, bail};
use rand::Rng;
use rand_distr::{Distribution, Poisson};
use serde::{Deserialize, Serialize};

use crate::{probability::FloatDistribution, tx_pricing::Lane};

// ----------------------------------------------------------------------
// Max-fee policy
// ----------------------------------------------------------------------

/// Policy for computing `max_fee_lovelace` from the per-byte quote
/// at submission time.
///
/// - `ScaledOverLaneQuote` is the default rule-of-thumb: a fixed
///   multiplier (e.g. `{4, 1}` for 4× quote-drift headroom).
///   Doesn't adapt to expected wait or controller cadence.
/// - `VolatilityAware` adapts the headroom to the expected wait
///   under worst-case unidirectional controller drift, using the
///   `MaxFeeContext` provided at submission. Models a user who
///   actively avoids quote-drift eviction.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum MaxFeePolicy {
    /// `max_fee_lovelace = min_fee_b + ⌈quote × bytes × num / den⌉`.
    ScaledOverLaneQuote { numerator: u64, denominator: u64 },
    /// `max_fee_lovelace = min_fee_b + ⌈quote × bytes × ((D+1)/D)^N
    /// × safety_num / safety_den⌉` where `N = ⌈expected_wait_blocks⌉`
    /// is taken from the `MaxFeeContext` and capped at
    /// `MAX_VOLATILITY_AWARE_BLOCKS = 16` to keep the exponentiation
    /// tractable. The `(D+1)/D` factor is the controller's
    /// per-step max-up-move under EIP-1559 (one step is ±1/D); the
    /// exponent compounds that bound over the expected wait.
    /// `safety_factor` provides additional headroom over the
    /// worst-case bound.
    VolatilityAware {
        /// Conservative estimate of the priority controller's
        /// `max_change_denominator`. Real D may be larger (slower
        /// drift) — using a smaller assumed D over-estimates worst
        /// case, which is the safer direction.
        assumed_max_change_denominator: u64,
        /// Multiplicative safety factor over the worst-case drift
        /// bound. `{1, 1}` = no extra margin (just the worst case);
        /// `{2, 1}` = 2× extra; etc.
        safety_factor_num: u64,
        safety_factor_den: u64,
    },
}

/// Cap on the worst-case exponent for `VolatilityAware`. Keeps
/// `((D+1)/D)^N` tractable in u128 (`9^16 ≈ 1.8 × 10^15`, well
/// inside u128) and acknowledges that beyond ~16 blocks of expected
/// wait the worst-case bound is dominated by other failure modes
/// (mempool overflow, run-end truncation).
pub const MAX_VOLATILITY_AWARE_BLOCKS: u32 = 16;

/// Inputs that some `MaxFeePolicy` variants need beyond
/// `(quote, bytes, min_fee_b)`. `ScaledOverLaneQuote` ignores all
/// fields; `VolatilityAware` consumes `expected_wait_blocks`.
#[derive(Debug, Clone, Copy)]
pub struct MaxFeeContext {
    /// Expected wait until inclusion at the posted lane, in blocks
    /// (typically from `LatencyEstimator::expected`).
    pub expected_wait_blocks: f64,
}

impl MaxFeeContext {
    /// A context with no useful data. Use only for callers that
    /// invoke variants which ignore the context (`ScaledOverLaneQuote`).
    pub fn unused() -> Self {
        Self {
            expected_wait_blocks: 0.0,
        }
    }
}

impl MaxFeePolicy {
    /// Validate at construction. Required fields: any denominator
    /// or `assumed_max_change_denominator` must be non-zero.
    pub fn validate(&self) -> Result<()> {
        match *self {
            MaxFeePolicy::ScaledOverLaneQuote { denominator, .. } if denominator == 0 => {
                bail!("ScaledOverLaneQuote.denominator must be non-zero")
            }
            MaxFeePolicy::VolatilityAware {
                assumed_max_change_denominator: 0,
                ..
            } => {
                bail!("VolatilityAware.assumed_max_change_denominator must be non-zero")
            }
            MaxFeePolicy::VolatilityAware {
                safety_factor_den: 0,
                ..
            } => bail!("VolatilityAware.safety_factor_den must be non-zero"),
            MaxFeePolicy::VolatilityAware {
                safety_factor_num: 0,
                ..
            } => bail!("VolatilityAware.safety_factor_num must be non-zero"),
            _ => Ok(()),
        }
    }

    /// Compute `max_fee_lovelace` for a tx of `bytes` bytes posted on
    /// a lane with `quote_per_byte`. Spec invariant: result ≥ `min_fee_b`.
    /// Overflow-safe via `u128` intermediates and `ceil_div_u128`.
    pub fn compute(
        &self,
        quote_per_byte: u64,
        bytes: u64,
        min_fee_b: u64,
        ctx: MaxFeeContext,
    ) -> Result<u64> {
        match *self {
            MaxFeePolicy::ScaledOverLaneQuote {
                numerator,
                denominator,
            } => {
                if denominator == 0 {
                    bail!("ScaledOverLaneQuote.denominator must be non-zero");
                }
                let product = (quote_per_byte as u128)
                    .checked_mul(bytes as u128)
                    .and_then(|v| v.checked_mul(numerator as u128))
                    .ok_or_else(|| {
                        anyhow!(
                            "max_fee_policy product overflow: \
                             quote={quote_per_byte} bytes={bytes} num={numerator}"
                        )
                    })?;
                let scaled = ceil_div_u128(product, denominator as u128);
                let scaled_u64: u64 = scaled.try_into().map_err(|_| {
                    anyhow!(
                        "max_fee_policy result exceeds u64: scaled={scaled} \
                         (quote={quote_per_byte} bytes={bytes} num={numerator} den={denominator})"
                    )
                })?;
                min_fee_b.checked_add(scaled_u64).ok_or_else(|| {
                    anyhow!(
                        "max_fee_lovelace overflow: min_fee_b={min_fee_b} + scaled={scaled_u64}"
                    )
                })
            }
            MaxFeePolicy::VolatilityAware {
                assumed_max_change_denominator: d,
                safety_factor_num,
                safety_factor_den,
            } => {
                if d == 0 {
                    bail!("VolatilityAware.assumed_max_change_denominator must be non-zero");
                }
                if safety_factor_den == 0 {
                    bail!("VolatilityAware.safety_factor_den must be non-zero");
                }
                if safety_factor_num == 0 {
                    bail!("VolatilityAware.safety_factor_num must be non-zero");
                }
                // N = ⌈expected_wait_blocks⌉, clamped to [1, MAX].
                // `libm::ceil` is bit-deterministic across arches;
                // the `as u32` cast saturates non-finite/out-of-range
                // values to 0/u32::MAX, which we then clamp.
                let n_raw = libm::ceil(ctx.expected_wait_blocks.max(1.0)) as u32;
                let n: u32 = n_raw.clamp(1, MAX_VOLATILITY_AWARE_BLOCKS);
                // Worst-case drift over N blocks: ((D+1)/D)^N.
                let drift_num: u128 = (d as u128 + 1)
                    .checked_pow(n)
                    .ok_or_else(|| anyhow!("VolatilityAware drift_num overflow: D={d} N={n}"))?;
                let drift_den: u128 = (d as u128)
                    .checked_pow(n)
                    .ok_or_else(|| anyhow!("VolatilityAware drift_den overflow: D={d} N={n}"))?;
                // max_fee_excess = ⌈quote × bytes × drift_num × safety_num
                //                  / (drift_den × safety_den)⌉.
                let product_num = (quote_per_byte as u128)
                    .checked_mul(bytes as u128)
                    .and_then(|v| v.checked_mul(drift_num))
                    .and_then(|v| v.checked_mul(safety_factor_num as u128))
                    .ok_or_else(|| {
                        anyhow!(
                            "VolatilityAware product overflow: \
                             quote={quote_per_byte} bytes={bytes} D={d} N={n} \
                             safety_num={safety_factor_num}"
                        )
                    })?;
                let den = drift_den
                    .checked_mul(safety_factor_den as u128)
                    .ok_or_else(|| {
                        anyhow!(
                            "VolatilityAware denominator overflow: D={d} N={n} \
                             safety_den={safety_factor_den}"
                        )
                    })?;
                let scaled = ceil_div_u128(product_num, den);
                let scaled_u64: u64 = scaled
                    .try_into()
                    .map_err(|_| anyhow!("VolatilityAware result exceeds u64: scaled={scaled}"))?;
                min_fee_b.checked_add(scaled_u64).ok_or_else(|| {
                    anyhow!(
                        "max_fee_lovelace overflow: min_fee_b={min_fee_b} + scaled={scaled_u64}"
                    )
                })
            }
        }
    }
}

/// Overflow-safe ceiling division. `ceil_div_u128(a, b) = ⌈a / b⌉`,
/// pinned per plan line 143. Never adds before dividing, so no
/// `(a + b − 1)` overflow.
pub fn ceil_div_u128(a: u128, b: u128) -> u128 {
    if a == 0 { 0 } else { 1 + (a - 1) / b }
}

// ----------------------------------------------------------------------
// Lane policy + lane-choice math
// ----------------------------------------------------------------------

/// Lane-choice policy. M3 ships utility-maximising; phase-2 default
/// per plan line 165.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LanePolicy {
    /// Pick the lane with the higher `expected_utility`. If both
    /// lanes are negative, `submit_when_underwater = true` submits
    /// anyway with `posted_lane = Standard` (per plan line 165 — the
    /// phase-2 default; actors don't game submission); `false`
    /// returns `None`.
    UtilityMaximising { submit_when_underwater: bool },
}

/// Per-lane inputs to `lane_choice::pick`.
#[derive(Debug, Clone, Copy)]
pub struct LaneInputs {
    pub current_quote_per_byte: u64,
    pub expected_latency_blocks: f64,
}

pub mod lane_choice {
    //! Lane-choice math. Cross-arch determinism scope is documented at
    //! the module top.

    use super::{Lane, LaneInputs, LanePolicy};

    /// Pick the lane with the higher expected utility (rounded into
    /// `i128` lovelace via `libm::round`). Ties break to `Standard`.
    /// Returns `None` only when `submit_when_underwater = false` and
    /// both lanes' expected utility is negative.
    pub fn pick(
        value_lovelace: u64,
        urgency: f64,
        bytes: u64,
        priority: &LaneInputs,
        standard: &LaneInputs,
        min_fee_b: u64,
        lane_policy: LanePolicy,
    ) -> Option<Lane> {
        let exp_util_priority =
            expected_utility_lovelace(value_lovelace, urgency, bytes, priority, min_fee_b);
        let exp_util_standard =
            expected_utility_lovelace(value_lovelace, urgency, bytes, standard, min_fee_b);
        match lane_policy {
            LanePolicy::UtilityMaximising {
                submit_when_underwater,
            } => {
                if !submit_when_underwater && exp_util_priority < 0 && exp_util_standard < 0 {
                    return None;
                }
                Some(if exp_util_priority > exp_util_standard {
                    Lane::Priority
                } else {
                    Lane::Standard
                })
            }
        }
    }

    /// `expected_utility(lane) = retained_value − (min_fee_b + quote × bytes)`,
    /// rounded into `i128` lovelace via round-half-away-from-zero.
    pub fn expected_utility_lovelace(
        value_lovelace: u64,
        urgency: f64,
        bytes: u64,
        lane: &LaneInputs,
        min_fee_b: u64,
    ) -> i128 {
        // Spec uses urgency > 1; clamp pathological inputs to 1.0
        // (urgency ≤ 1 yields no decay, retained_value = value).
        let urgency = if urgency.is_finite() && urgency > 1.0 {
            urgency
        } else {
            1.0
        };
        // latency_blocks ≥ 0; clamp negatives to 0.
        let latency_blocks = if lane.expected_latency_blocks.is_finite() {
            lane.expected_latency_blocks.max(0.0)
        } else {
            0.0
        };
        // Bit-deterministic across architectures (per module note).
        let factor = libm::pow(urgency, -latency_blocks);
        let retained_f64 = (value_lovelace as f64) * factor;
        // Pinned rounding rule: round-half-away-from-zero via
        // `libm::round` *before* the integer cast. Rounding here is
        // what determines the rule — once `libm::round` returns,
        // the f64 holds an integer value and `as i128` is a pure
        // type conversion. Without the explicit `libm::round`,
        // `retained_f64 as i128` would truncate toward zero,
        // biasing positive expected_utility values downward by up
        // to one lovelace (and the integer event stream's hash
        // would depend on the chosen rule).
        let retained_lov = libm::round(retained_f64) as i128;
        let posted_fee_lov = (min_fee_b as i128)
            .saturating_add((lane.current_quote_per_byte as i128).saturating_mul(bytes as i128));
        retained_lov - posted_fee_lov
    }
}

// ----------------------------------------------------------------------
// Welfare formulas (reporting only; plain f64)
// ----------------------------------------------------------------------

pub mod welfare {
    //! Reporting-only welfare formulas (plan lines 148-152).
    //!
    //! These run on top of the integer event stream. Negative
    //! `net_utility` (regret events) is preserved through all
    //! aggregation steps; never clamped, floored, or filtered.

    pub fn retained_value(value_lovelace: u64, urgency: f64, latency_blocks: f64) -> f64 {
        let urgency = if urgency.is_finite() && urgency > 1.0 {
            urgency
        } else {
            1.0
        };
        let latency_blocks = if latency_blocks.is_finite() {
            latency_blocks.max(0.0)
        } else {
            0.0
        };
        (value_lovelace as f64) * libm::pow(urgency, -latency_blocks)
    }

    pub fn net_utility(retained_value: f64, actual_fee_lovelace: u64) -> f64 {
        retained_value - (actual_fee_lovelace as f64)
    }

    pub fn retained_value_ratio(retained_value: f64, value_lovelace: u64) -> f64 {
        if value_lovelace == 0 {
            0.0
        } else {
            retained_value / (value_lovelace as f64)
        }
    }
}

// ----------------------------------------------------------------------
// Latency estimator
// ----------------------------------------------------------------------

/// Per-lane rolling-average inclusion-delay estimator (blocks).
/// Initialised from configurable defaults; updated as inclusions
/// arrive. `expected(lane)` returns the current average for use by
/// `lane_choice::pick`.
#[derive(Debug, Clone)]
pub struct LatencyEstimator {
    window: usize,
    priority: VecDeque<f64>,
    standard: VecDeque<f64>,
    priority_default: f64,
    standard_default: f64,
}

impl LatencyEstimator {
    pub fn new(window: usize, priority_default: f64, standard_default: f64) -> Self {
        Self {
            window: window.max(1),
            priority: VecDeque::new(),
            standard: VecDeque::new(),
            priority_default,
            standard_default,
        }
    }

    pub fn observe(&mut self, lane: Lane, latency_blocks: f64) {
        let buf = match lane {
            Lane::Priority => &mut self.priority,
            Lane::Standard => &mut self.standard,
        };
        if buf.len() >= self.window {
            buf.pop_front();
        }
        buf.push_back(latency_blocks);
    }

    pub fn expected(&self, lane: Lane) -> f64 {
        let (buf, default) = match lane {
            Lane::Priority => (&self.priority, self.priority_default),
            Lane::Standard => (&self.standard, self.standard_default),
        };
        if buf.is_empty() {
            default
        } else {
            buf.iter().sum::<f64>() / (buf.len() as f64)
        }
    }
}

// ----------------------------------------------------------------------
// Arrival rate (time-varying via phases)
// ----------------------------------------------------------------------

/// One phase of a time-varying arrival rate. `[start_slot, end_slot)`
/// is half-open. Phases must be non-overlapping; outside the union of
/// phases the effective rate is 0.
#[derive(Debug, Clone)]
pub struct ArrivalPhase {
    pub start_slot: u64,
    pub end_slot: u64,
    pub rate: f64,
}

/// Per-component arrival rate. `Constant` is the legacy behaviour;
/// `Phased` supports demand shocks (Sundaeswap-style DEX launches,
/// daily peak hours, etc.). The lookup is linear over phases — fine
/// for small phase counts; revisit if a profile carries 100+ phases.
#[derive(Debug, Clone)]
pub enum ArrivalRate {
    Constant(f64),
    Phased(Vec<ArrivalPhase>),
}

impl ArrivalRate {
    /// Effective rate at `slot`. `Constant` ignores the slot. `Phased`
    /// returns the rate of the first phase whose interval contains
    /// `slot`, or 0 outside all phases.
    pub fn rate_at_slot(&self, slot: u64) -> f64 {
        match self {
            ArrivalRate::Constant(r) => *r,
            ArrivalRate::Phased(phases) => phases
                .iter()
                .find(|p| slot >= p.start_slot && slot < p.end_slot)
                .map_or(0.0, |p| p.rate),
        }
    }
}

// ----------------------------------------------------------------------
// ActorComponent / ActorProfile
// ----------------------------------------------------------------------

/// One weighted component of an actor profile. Each arrival samples
/// `(bytes, value, urgency)` from f64 distributions, computes a
/// `posted_lane` via `LanePolicy`, and computes `max_fee_lovelace`
/// via `MaxFeePolicy`.
#[derive(Debug, Clone)]
pub struct ActorComponent {
    /// Index of this component in the profile. Recorded on every tx
    /// (`Transaction.urgency_component_index`) so welfare metrics can
    /// bucket per-class.
    pub index: u32,
    /// Mean transaction arrivals per slot for this component.
    /// `Constant` for a flat rate; `Phased` for demand shocks.
    /// Sampled per slot via Poisson at the rate active for the
    /// current slot. Components fire independently; the per-slot
    /// total is the sum across components.
    pub arrival_rate_per_slot: ArrivalRate,
    /// Transaction byte-size distribution.
    pub size_bytes: FloatDistribution,
    /// Value distribution (lovelace; sampled to f64, rounded to u64).
    pub value_lovelace: FloatDistribution,
    /// Urgency distribution. Spec calls for u > 1; the actor clamps
    /// to `[1.0, ∞)` at sample time so the lane-choice math never
    /// inverts the decay direction.
    pub urgency: FloatDistribution,
    /// Lane-choice policy.
    pub lane_policy: LanePolicy,
    /// Max-fee policy.
    pub max_fee_policy: MaxFeePolicy,
    /// Initial expected priority-lane inclusion latency (blocks).
    /// Used by `LatencyEstimator` until enough observations accrue.
    pub target_inclusion_blocks_priority: f64,
    /// Initial expected standard-lane inclusion latency (blocks).
    pub target_inclusion_blocks_standard: f64,
}

impl ActorComponent {
    pub fn validate(&self) -> Result<()> {
        match &self.arrival_rate_per_slot {
            ArrivalRate::Constant(r) => {
                if !r.is_finite() || *r < 0.0 {
                    bail!(
                        "arrival_rate_per_slot for component {} must be finite and ≥ 0 (got {})",
                        self.index,
                        r
                    );
                }
            }
            ArrivalRate::Phased(phases) => {
                if phases.is_empty() {
                    bail!(
                        "arrival_rate_per_slot for component {} is Phased with no phases",
                        self.index
                    );
                }
                for p in phases {
                    if !p.rate.is_finite() || p.rate < 0.0 {
                        bail!(
                            "arrival_rate_per_slot phase rate for component {} must be finite and ≥ 0 (got {})",
                            self.index,
                            p.rate
                        );
                    }
                    if p.start_slot >= p.end_slot {
                        bail!(
                            "arrival_rate_per_slot phase for component {} must have start_slot < end_slot (got [{}, {}))",
                            self.index,
                            p.start_slot,
                            p.end_slot
                        );
                    }
                }
                // Detect overlap by checking each phase against the others.
                for (i, a) in phases.iter().enumerate() {
                    for b in phases.iter().skip(i + 1) {
                        if a.start_slot < b.end_slot && b.start_slot < a.end_slot {
                            bail!(
                                "arrival_rate_per_slot phases for component {} overlap: [{}, {}) and [{}, {})",
                                self.index,
                                a.start_slot,
                                a.end_slot,
                                b.start_slot,
                                b.end_slot
                            );
                        }
                    }
                }
            }
        }
        self.max_fee_policy.validate()?;
        for (label, blocks) in [
            (
                "target_inclusion_blocks_priority",
                self.target_inclusion_blocks_priority,
            ),
            (
                "target_inclusion_blocks_standard",
                self.target_inclusion_blocks_standard,
            ),
        ] {
            if !blocks.is_finite() || blocks < 0.0 {
                bail!(
                    "{label} for component {} must be finite and ≥ 0 (got {blocks})",
                    self.index,
                );
            }
        }
        Ok(())
    }

    /// Sample the per-slot arrival count from `Poisson(λ)` where
    /// `λ` is the arrival rate active at `slot`. Returns 0 when
    /// `λ ≤ 0` (e.g. outside the union of phase intervals for a
    /// `Phased` rate).
    pub fn sample_arrival_count<R: Rng>(&self, rng: &mut R, slot: u64) -> u64 {
        let rate = self.arrival_rate_per_slot.rate_at_slot(slot);
        if !rate.is_finite() || rate <= 0.0 {
            return 0;
        }
        // Poisson::new returns Err only on non-positive λ; we've
        // checked.
        let dist = Poisson::new(rate).expect("arrival_rate_per_slot validated > 0");
        // `Poisson<f64>::sample` returns f64 ≥ 0; round-half-away-
        // from-zero is fine for u64.
        libm::round(dist.sample(rng)).max(0.0) as u64
    }

    /// Sample one transaction's `(bytes, value_lovelace, urgency)`
    /// triple. Negative samples are clamped: `bytes ≥ 1`,
    /// `value_lovelace ≥ 0`, `urgency ≥ 1.0`.
    pub fn sample_tx_inputs<R: Rng>(&self, rng: &mut R) -> SampledTxInputs {
        let bytes_f = self.size_bytes.sample(rng);
        let value_f = self.value_lovelace.sample(rng);
        let urgency_f = self.urgency.sample(rng);
        SampledTxInputs {
            bytes: bytes_f.max(1.0) as u64,
            value_lovelace: value_f.max(0.0) as u64,
            urgency: if urgency_f.is_finite() && urgency_f > 1.0 {
                urgency_f
            } else {
                1.0
            },
        }
    }
}

/// One sampled tx's pre-pricing inputs.
#[derive(Debug, Clone, Copy)]
pub struct SampledTxInputs {
    pub bytes: u64,
    pub value_lovelace: u64,
    pub urgency: f64,
}

#[derive(Debug, Clone)]
pub struct ActorProfile {
    pub components: Vec<ActorComponent>,
    /// `block_generation_probability` from the protocol config; used
    /// to convert observed inclusion latency from slots to blocks
    /// (`latency_blocks = latency_slots × p`, plan line 149).
    pub block_generation_probability: f64,
    pub min_fee_b: u64,
}

impl ActorProfile {
    pub fn validate(&self) -> Result<()> {
        if self.components.is_empty() {
            bail!("ActorProfile must have at least one component");
        }
        if !self.block_generation_probability.is_finite()
            || self.block_generation_probability <= 0.0
            || self.block_generation_probability > 1.0
        {
            bail!(
                "block_generation_probability must lie in (0, 1] (got {})",
                self.block_generation_probability
            );
        }
        for c in &self.components {
            c.validate()?;
        }
        Ok(())
    }

    /// Convert a slot-count latency to a block-count latency using
    /// `block_generation_probability`. Convenience for callers
    /// observing latency in slots.
    pub fn latency_slots_to_blocks(&self, slots: f64) -> f64 {
        slots * self.block_generation_probability
    }
}

// ----------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaChaRng;

    const MIN_FEE_B: u64 = 155_381;

    fn comp_default() -> ActorComponent {
        ActorComponent {
            index: 0,
            arrival_rate_per_slot: ArrivalRate::Constant(1.0),
            size_bytes: FloatDistribution::constant(1024.0),
            value_lovelace: FloatDistribution::constant(1_000_000.0),
            urgency: FloatDistribution::constant(1.05),
            lane_policy: LanePolicy::UtilityMaximising {
                submit_when_underwater: true,
            },
            max_fee_policy: MaxFeePolicy::ScaledOverLaneQuote {
                numerator: 4,
                denominator: 1,
            },
            target_inclusion_blocks_priority: 1.0,
            target_inclusion_blocks_standard: 4.0,
        }
    }

    // --- MaxFeePolicy ---

    #[test]
    fn scaled_over_lane_quote_rejects_zero_denominator() {
        let policy = MaxFeePolicy::ScaledOverLaneQuote {
            numerator: 4,
            denominator: 0,
        };
        assert!(policy.validate().is_err());
        assert!(
            policy
                .compute(44, 1024, MIN_FEE_B, MaxFeeContext::unused())
                .is_err()
        );
    }

    #[test]
    fn scaled_over_lane_quote_computes_min_fee_b_plus_scaled() {
        let policy = MaxFeePolicy::ScaledOverLaneQuote {
            numerator: 4,
            denominator: 1,
        };
        let bytes = 1024u64;
        let quote = 44u64;
        let max_fee = policy
            .compute(quote, bytes, MIN_FEE_B, MaxFeeContext::unused())
            .unwrap();
        // 4 × 44 × 1024 = 180_224
        assert_eq!(max_fee, MIN_FEE_B + 180_224);
    }

    #[test]
    fn scaled_over_lane_quote_uses_ceil_division() {
        let policy = MaxFeePolicy::ScaledOverLaneQuote {
            numerator: 1,
            denominator: 3,
        };
        // 1 × 100 / 3 = 33.33; ceil = 34
        let max_fee = policy
            .compute(100, 1, MIN_FEE_B, MaxFeeContext::unused())
            .unwrap();
        assert_eq!(max_fee, MIN_FEE_B + 34);
    }

    #[test]
    fn scaled_over_lane_quote_zero_inputs_yield_min_fee_b() {
        let policy = MaxFeePolicy::ScaledOverLaneQuote {
            numerator: 4,
            denominator: 1,
        };
        // bytes = 0 → max_fee = min_fee_b.
        assert_eq!(
            policy
                .compute(44, 0, MIN_FEE_B, MaxFeeContext::unused())
                .unwrap(),
            MIN_FEE_B
        );
        // quote = 0 → max_fee = min_fee_b.
        assert_eq!(
            policy
                .compute(0, 1024, MIN_FEE_B, MaxFeeContext::unused())
                .unwrap(),
            MIN_FEE_B
        );
    }

    #[test]
    fn scaled_over_lane_quote_overflow_is_an_error() {
        let policy = MaxFeePolicy::ScaledOverLaneQuote {
            numerator: u64::MAX,
            denominator: 1,
        };
        // u64::MAX × u64::MAX × u64::MAX overflows u128.
        let err = policy.compute(u64::MAX, u64::MAX, MIN_FEE_B, MaxFeeContext::unused());
        assert!(err.is_err(), "expected overflow error, got {err:?}");
    }

    // --- VolatilityAware ---

    #[test]
    fn volatility_aware_rejects_zero_fields() {
        let zero_d = MaxFeePolicy::VolatilityAware {
            assumed_max_change_denominator: 0,
            safety_factor_num: 1,
            safety_factor_den: 1,
        };
        assert!(zero_d.validate().is_err());
        let zero_safety_den = MaxFeePolicy::VolatilityAware {
            assumed_max_change_denominator: 8,
            safety_factor_num: 1,
            safety_factor_den: 0,
        };
        assert!(zero_safety_den.validate().is_err());
        let zero_safety_num = MaxFeePolicy::VolatilityAware {
            assumed_max_change_denominator: 8,
            safety_factor_num: 0,
            safety_factor_den: 1,
        };
        assert!(zero_safety_num.validate().is_err());
    }

    #[test]
    fn volatility_aware_n1_d8_no_safety_matches_one_step_drift() {
        // ((D+1)/D)^N = (9/8)^1 = 9/8.
        // max_fee = min_fee_b + ⌈quote × bytes × 9 / 8⌉
        let policy = MaxFeePolicy::VolatilityAware {
            assumed_max_change_denominator: 8,
            safety_factor_num: 1,
            safety_factor_den: 1,
        };
        let ctx = MaxFeeContext {
            expected_wait_blocks: 1.0,
        };
        let bytes = 1024u64;
        let quote = 44u64;
        let max_fee = policy.compute(quote, bytes, MIN_FEE_B, ctx).unwrap();
        // 44 × 1024 × 9 / 8 = 50_688
        assert_eq!(max_fee, MIN_FEE_B + 50_688);
    }

    #[test]
    fn volatility_aware_clamps_n_to_at_least_1() {
        // expected_wait_blocks = 0 → N clamped to 1.
        let policy = MaxFeePolicy::VolatilityAware {
            assumed_max_change_denominator: 8,
            safety_factor_num: 1,
            safety_factor_den: 1,
        };
        let max_fee_zero_wait = policy
            .compute(
                44,
                1024,
                MIN_FEE_B,
                MaxFeeContext {
                    expected_wait_blocks: 0.0,
                },
            )
            .unwrap();
        let max_fee_unit_wait = policy
            .compute(
                44,
                1024,
                MIN_FEE_B,
                MaxFeeContext {
                    expected_wait_blocks: 1.0,
                },
            )
            .unwrap();
        assert_eq!(max_fee_zero_wait, max_fee_unit_wait);
    }

    #[test]
    fn volatility_aware_caps_n_at_max_blocks() {
        // expected_wait_blocks = 1000 should produce the same result
        // as expected_wait_blocks = MAX_VOLATILITY_AWARE_BLOCKS.
        let policy = MaxFeePolicy::VolatilityAware {
            assumed_max_change_denominator: 8,
            safety_factor_num: 1,
            safety_factor_den: 1,
        };
        let big = policy
            .compute(
                44,
                1024,
                MIN_FEE_B,
                MaxFeeContext {
                    expected_wait_blocks: 1000.0,
                },
            )
            .unwrap();
        let cap = policy
            .compute(
                44,
                1024,
                MIN_FEE_B,
                MaxFeeContext {
                    expected_wait_blocks: MAX_VOLATILITY_AWARE_BLOCKS as f64,
                },
            )
            .unwrap();
        assert_eq!(big, cap);
    }

    #[test]
    fn volatility_aware_grows_with_expected_wait() {
        // Strictly monotonic in N until the cap.
        let policy = MaxFeePolicy::VolatilityAware {
            assumed_max_change_denominator: 8,
            safety_factor_num: 1,
            safety_factor_den: 1,
        };
        let mut prev = 0u64;
        for n in 1..=(MAX_VOLATILITY_AWARE_BLOCKS as u64) {
            let max_fee = policy
                .compute(
                    44,
                    1024,
                    MIN_FEE_B,
                    MaxFeeContext {
                        expected_wait_blocks: n as f64,
                    },
                )
                .unwrap();
            assert!(
                max_fee > prev,
                "expected strict growth at N={n}: prev={prev} curr={max_fee}"
            );
            prev = max_fee;
        }
    }

    #[test]
    fn volatility_aware_with_safety_factor_is_proportional() {
        // safety_factor = {2, 1} should give ~2× the excess of {1, 1}.
        let no_safety = MaxFeePolicy::VolatilityAware {
            assumed_max_change_denominator: 8,
            safety_factor_num: 1,
            safety_factor_den: 1,
        };
        let double = MaxFeePolicy::VolatilityAware {
            assumed_max_change_denominator: 8,
            safety_factor_num: 2,
            safety_factor_den: 1,
        };
        let ctx = MaxFeeContext {
            expected_wait_blocks: 4.0,
        };
        let no_safety_excess = no_safety.compute(44, 1024, MIN_FEE_B, ctx).unwrap() - MIN_FEE_B;
        let double_excess = double.compute(44, 1024, MIN_FEE_B, ctx).unwrap() - MIN_FEE_B;
        // With ceil rounding the doubled result may be 1 lovelace
        // larger than 2× the single result; allow ±1.
        let twice = no_safety_excess * 2;
        assert!(
            (double_excess as i64 - twice as i64).abs() <= 1,
            "expected ~2× excess; got no_safety={no_safety_excess} double={double_excess} twice={twice}"
        );
    }

    // --- ceil_div_u128 ---

    #[test]
    fn ceil_div_u128_handles_zero() {
        assert_eq!(ceil_div_u128(0, 1), 0);
        assert_eq!(ceil_div_u128(0, 17), 0);
    }

    #[test]
    fn ceil_div_u128_rounds_up() {
        assert_eq!(ceil_div_u128(1, 3), 1);
        assert_eq!(ceil_div_u128(4, 3), 2);
        assert_eq!(ceil_div_u128(6, 3), 2);
        assert_eq!(ceil_div_u128(7, 3), 3);
    }

    #[test]
    fn ceil_div_u128_does_not_overflow_at_u128_max() {
        // (u128::MAX - 1) / 1 + 1 = u128::MAX; the naive
        // `(a + b - 1) / b` form would overflow. Confirm we don't.
        assert_eq!(ceil_div_u128(u128::MAX, 1), u128::MAX);
    }

    // --- lane_choice ---

    #[test]
    fn lane_choice_is_deterministic_across_runs() {
        let priority = LaneInputs {
            current_quote_per_byte: 100,
            expected_latency_blocks: 1.0,
        };
        let standard = LaneInputs {
            current_quote_per_byte: 50,
            expected_latency_blocks: 4.0,
        };
        let policy = LanePolicy::UtilityMaximising {
            submit_when_underwater: true,
        };
        let pick1 = lane_choice::pick(
            10_000_000, 1.05, 1024, &priority, &standard, MIN_FEE_B, policy,
        );
        let pick2 = lane_choice::pick(
            10_000_000, 1.05, 1024, &priority, &standard, MIN_FEE_B, policy,
        );
        assert_eq!(pick1, pick2);
        assert!(pick1.is_some());
    }

    #[test]
    fn lane_choice_with_urgency_one_picks_lower_quote() {
        // urgency = 1.0 ⇒ retained_value = value (no decay) ⇒
        // expected_utility = value − fee. The lane with the lower
        // quote produces the higher utility. Since the actor's
        // urgency clamp pins urgency ≥ 1.0, and 1.0 ⇒ no decay, this
        // collapses to "pick the lane with the lowest fee". Standard
        // here has a lower quote, so picks Standard.
        let priority = LaneInputs {
            current_quote_per_byte: 1000,
            expected_latency_blocks: 1.0,
        };
        let standard = LaneInputs {
            current_quote_per_byte: 50,
            expected_latency_blocks: 4.0,
        };
        let policy = LanePolicy::UtilityMaximising {
            submit_when_underwater: true,
        };
        let lane = lane_choice::pick(
            1_000_000, 1.0, 1024, &priority, &standard, MIN_FEE_B, policy,
        )
        .unwrap();
        assert_eq!(lane, Lane::Standard);
    }

    #[test]
    fn lane_choice_high_urgency_prefers_priority() {
        // High urgency + high latency-gap on standard → Priority
        // wins despite paying more per byte.
        let priority = LaneInputs {
            current_quote_per_byte: 200,
            expected_latency_blocks: 1.0,
        };
        let standard = LaneInputs {
            current_quote_per_byte: 50,
            expected_latency_blocks: 10.0,
        };
        let policy = LanePolicy::UtilityMaximising {
            submit_when_underwater: true,
        };
        let lane = lane_choice::pick(
            100_000_000,
            10.0,
            1024,
            &priority,
            &standard,
            MIN_FEE_B,
            policy,
        )
        .unwrap();
        assert_eq!(lane, Lane::Priority);
    }

    #[test]
    fn lane_choice_underwater_skip_returns_none() {
        // Both lanes' fee massively exceeds the value's retained
        // value ⇒ both expected_utility < 0. With submit_when_underwater
        // = false, return None.
        let priority = LaneInputs {
            current_quote_per_byte: 10_000_000,
            expected_latency_blocks: 1.0,
        };
        let standard = LaneInputs {
            current_quote_per_byte: 5_000_000,
            expected_latency_blocks: 4.0,
        };
        let policy = LanePolicy::UtilityMaximising {
            submit_when_underwater: false,
        };
        let lane = lane_choice::pick(1, 1.05, 1024, &priority, &standard, MIN_FEE_B, policy);
        assert!(lane.is_none());
    }

    #[test]
    fn lane_choice_underwater_default_submits_as_standard() {
        // submit_when_underwater = true (default) → submit anyway,
        // pick the less-bad lane. Standard typically less bad here.
        let priority = LaneInputs {
            current_quote_per_byte: 10_000_000,
            expected_latency_blocks: 1.0,
        };
        let standard = LaneInputs {
            current_quote_per_byte: 5_000_000,
            expected_latency_blocks: 4.0,
        };
        let policy = LanePolicy::UtilityMaximising {
            submit_when_underwater: true,
        };
        let lane =
            lane_choice::pick(1, 1.05, 1024, &priority, &standard, MIN_FEE_B, policy).unwrap();
        assert_eq!(lane, Lane::Standard);
    }

    #[test]
    fn lane_choice_ties_break_to_standard() {
        // Identical lane inputs ⇒ identical expected_utility ⇒ tie.
        // Tie-break is `>` so Standard wins.
        let lane_inputs = LaneInputs {
            current_quote_per_byte: 100,
            expected_latency_blocks: 2.0,
        };
        let policy = LanePolicy::UtilityMaximising {
            submit_when_underwater: true,
        };
        let lane = lane_choice::pick(
            1_000_000,
            1.05,
            1024,
            &lane_inputs,
            &lane_inputs,
            MIN_FEE_B,
            policy,
        )
        .unwrap();
        assert_eq!(lane, Lane::Standard);
    }

    // --- Welfare formulas ---

    #[test]
    fn retained_value_no_decay_returns_value() {
        // urgency = 1.0 ⇒ no decay regardless of latency.
        assert_eq!(welfare::retained_value(1_000, 1.0, 5.0), 1000.0);
    }

    #[test]
    fn retained_value_decays_with_latency() {
        // urgency = 2.0, latency_blocks = 1 ⇒ retained = value × 0.5.
        let r = welfare::retained_value(1000, 2.0, 1.0);
        assert!((r - 500.0).abs() < 1e-9);
    }

    #[test]
    fn net_utility_can_be_negative_regret_event() {
        // retained < actual_fee ⇒ negative net_utility.
        let nu = welfare::net_utility(100.0, 500);
        assert!(nu < 0.0);
        assert!((nu - (-400.0)).abs() < 1e-9);
    }

    #[test]
    fn retained_value_ratio_zero_value_returns_zero() {
        assert_eq!(welfare::retained_value_ratio(0.0, 0), 0.0);
    }

    // --- LatencyEstimator ---

    #[test]
    fn latency_estimator_returns_default_when_empty() {
        let est = LatencyEstimator::new(8, 1.0, 4.0);
        assert_eq!(est.expected(Lane::Priority), 1.0);
        assert_eq!(est.expected(Lane::Standard), 4.0);
    }

    #[test]
    fn latency_estimator_averages_observations() {
        let mut est = LatencyEstimator::new(8, 1.0, 4.0);
        est.observe(Lane::Priority, 2.0);
        est.observe(Lane::Priority, 4.0);
        est.observe(Lane::Priority, 6.0);
        assert!((est.expected(Lane::Priority) - 4.0).abs() < 1e-9);
    }

    #[test]
    fn latency_estimator_evicts_oldest_when_full() {
        let mut est = LatencyEstimator::new(2, 1.0, 4.0);
        est.observe(Lane::Standard, 10.0);
        est.observe(Lane::Standard, 20.0);
        est.observe(Lane::Standard, 30.0); // evicts the 10.0
        assert!((est.expected(Lane::Standard) - 25.0).abs() < 1e-9);
    }

    // --- ActorProfile/Component validation ---

    #[test]
    fn empty_profile_rejects() {
        let profile = ActorProfile {
            components: vec![],
            block_generation_probability: 0.05,
            min_fee_b: MIN_FEE_B,
        };
        assert!(profile.validate().is_err());
    }

    #[test]
    fn invalid_block_generation_probability_rejects() {
        let profile = ActorProfile {
            components: vec![comp_default()],
            block_generation_probability: 0.0,
            min_fee_b: MIN_FEE_B,
        };
        assert!(profile.validate().is_err());

        let profile = ActorProfile {
            components: vec![comp_default()],
            block_generation_probability: 1.5,
            min_fee_b: MIN_FEE_B,
        };
        assert!(profile.validate().is_err());
    }

    #[test]
    fn negative_arrival_rate_rejects() {
        let mut comp = comp_default();
        comp.arrival_rate_per_slot = ArrivalRate::Constant(-1.0);
        let profile = ActorProfile {
            components: vec![comp],
            block_generation_probability: 0.05,
            min_fee_b: MIN_FEE_B,
        };
        assert!(profile.validate().is_err());
    }

    #[test]
    fn zero_denominator_max_fee_policy_rejects() {
        let mut comp = comp_default();
        comp.max_fee_policy = MaxFeePolicy::ScaledOverLaneQuote {
            numerator: 4,
            denominator: 0,
        };
        let profile = ActorProfile {
            components: vec![comp],
            block_generation_probability: 0.05,
            min_fee_b: MIN_FEE_B,
        };
        assert!(profile.validate().is_err());
    }

    #[test]
    fn arrival_count_is_zero_when_lambda_is_zero() {
        let mut comp = comp_default();
        comp.arrival_rate_per_slot = ArrivalRate::Constant(0.0);
        let mut rng = ChaChaRng::seed_from_u64(0);
        assert_eq!(comp.sample_arrival_count(&mut rng, 0), 0);
    }

    #[test]
    fn arrival_count_sampling_is_deterministic_under_seed() {
        let comp = ActorComponent {
            arrival_rate_per_slot: ArrivalRate::Constant(5.0),
            ..comp_default()
        };
        let mut rng_a = ChaChaRng::seed_from_u64(42);
        let mut rng_b = ChaChaRng::seed_from_u64(42);
        // Same seed → same arrival counts across calls.
        for slot in 0..20 {
            assert_eq!(
                comp.sample_arrival_count(&mut rng_a, slot),
                comp.sample_arrival_count(&mut rng_b, slot)
            );
        }
    }

    #[test]
    fn phased_arrival_rate_selects_active_phase() {
        let rate = ArrivalRate::Phased(vec![
            ArrivalPhase {
                start_slot: 100,
                end_slot: 200,
                rate: 5.0,
            },
            ArrivalPhase {
                start_slot: 300,
                end_slot: 400,
                rate: 50.0,
            },
        ]);
        assert_eq!(rate.rate_at_slot(0), 0.0); // before any phase
        assert_eq!(rate.rate_at_slot(99), 0.0); // just before phase 1
        assert_eq!(rate.rate_at_slot(100), 5.0); // start of phase 1 (inclusive)
        assert_eq!(rate.rate_at_slot(150), 5.0); // mid phase 1
        assert_eq!(rate.rate_at_slot(199), 5.0); // end of phase 1 (inclusive)
        assert_eq!(rate.rate_at_slot(200), 0.0); // end_slot is exclusive
        assert_eq!(rate.rate_at_slot(250), 0.0); // gap between phases
        assert_eq!(rate.rate_at_slot(300), 50.0); // start of phase 2
        assert_eq!(rate.rate_at_slot(399), 50.0);
        assert_eq!(rate.rate_at_slot(400), 0.0); // after all phases
    }

    #[test]
    fn phased_arrival_rate_validates_overlap() {
        let comp = ActorComponent {
            arrival_rate_per_slot: ArrivalRate::Phased(vec![
                ArrivalPhase {
                    start_slot: 0,
                    end_slot: 200,
                    rate: 5.0,
                },
                ArrivalPhase {
                    start_slot: 100,
                    end_slot: 300,
                    rate: 10.0,
                },
            ]),
            ..comp_default()
        };
        assert!(comp.validate().is_err());
    }

    #[test]
    fn phased_arrival_rate_validates_inverted_phase() {
        let comp = ActorComponent {
            arrival_rate_per_slot: ArrivalRate::Phased(vec![ArrivalPhase {
                start_slot: 200,
                end_slot: 100,
                rate: 5.0,
            }]),
            ..comp_default()
        };
        assert!(comp.validate().is_err());
    }

    #[test]
    fn phased_arrival_rate_sampling_respects_phase() {
        let comp = ActorComponent {
            arrival_rate_per_slot: ArrivalRate::Phased(vec![ArrivalPhase {
                start_slot: 100,
                end_slot: 200,
                rate: 50.0,
            }]),
            ..comp_default()
        };
        let mut rng = ChaChaRng::seed_from_u64(0);
        // Outside the phase: zero arrivals.
        for slot in [0u64, 50, 99, 200, 500] {
            assert_eq!(comp.sample_arrival_count(&mut rng, slot), 0);
        }
        // Inside the phase: Poisson(50) samples — almost certainly > 0.
        let total: u64 = (100..200u64)
            .map(|s| comp.sample_arrival_count(&mut rng, s))
            .sum();
        assert!(
            total > 0,
            "expected non-zero arrivals across 100 slots at λ=50"
        );
    }

    #[test]
    fn sampled_inputs_are_clamped_to_safe_ranges() {
        let comp = ActorComponent {
            // value can be negative under a normal distribution; it
            // must be clamped to ≥ 0.
            value_lovelace: FloatDistribution::normal(-1_000_000.0, 1.0),
            // urgency below 1 must be clamped to 1.0.
            urgency: FloatDistribution::constant(0.5),
            // size below 1 byte must be clamped to 1.
            size_bytes: FloatDistribution::constant(0.0),
            ..comp_default()
        };
        let mut rng = ChaChaRng::seed_from_u64(7);
        let inputs = comp.sample_tx_inputs(&mut rng);
        assert_eq!(inputs.bytes, 1);
        assert_eq!(inputs.value_lovelace, 0);
        assert_eq!(inputs.urgency, 1.0);
    }
}
