//! Transaction-pricing layer.
//!
//! See `docs/phase-2/mechanism-design.md` for the spec these modules implement,
//! and `docs/phase-2/implementation-plan.md` lines 26-180 for the architecture.
//!
//! A `PricingBackend` is policy-only: it answers "what is the per-byte rate
//! for lane `L`?" and "given the priced blocks I just saw, how should the
//! coefficient(s) move?". Block packing, partition activation, and selection
//! live in the simulator and consult the backend through these queries.
//!
//! All simulation-affecting state (controller coefficients, window contents,
//! quote-per-byte) is stored as `u64`/`u128` integers or as rationals. f64
//! never enters this module's hot paths.

pub mod single_lane;
pub mod window;

use serde::{Deserialize, Serialize};

pub use single_lane::{BaselinePricing, Eip1559Pricing, Eip1559Settings};
pub use window::CapacityWeightedWindow;

/// One of two transaction lanes. Single-lane mechanisms always set
/// `Lane::Standard`; two-lane mechanisms (M2+) populate both variants.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Lane {
    Standard,
    Priority,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BlockKind {
    RankingBlock,
    EndorserBlock,
}

/// Lane-validity rule for a block being assembled.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum LaneValidityRule {
    None,
    PriorityOnly,
}

/// Order in which the simulator scans the mempool when packing a block.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LaneSelectionOrder {
    PriorityFirst,
    Fifo,
}

/// One sample fed to a controller for a single priced block.
///
/// A block emits zero or more samples (per implementation-plan.md lines
/// 65-77): single-lane RBs/EBs emit one `Standard` sample; both-dynamic
/// mechanisms typically emit two (one per controller). M1 only ever emits
/// one `Standard` sample per priced block.
#[derive(Debug, Clone, Copy)]
pub struct PricedBlockSample {
    pub block_kind: BlockKind,
    pub controller_lane: Lane,
    pub relevant_bytes: u64,
    pub relevant_capacity: u64,
}

/// Rational multiplier `numerator / denominator`. Used for the
/// multiplier-floor invariant in two-lane controllers (M2+) and for
/// rational scaling factors elsewhere — never f64, so cross-platform
/// arithmetic stays bit-identical.
///
/// Use [`Multiplier::new`] to construct: it rejects `denominator == 0`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Multiplier {
    pub numerator: u64,
    pub denominator: u64,
}

impl Multiplier {
    pub fn new(numerator: u64, denominator: u64) -> anyhow::Result<Self> {
        if denominator == 0 {
            anyhow::bail!("Multiplier denominator must be non-zero");
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }
}

/// Snapshot of pricing state for time-series logging.
#[derive(Debug, Clone)]
pub struct PricingSnapshot {
    pub standard_quote_per_byte: u64,
    pub priority_quote_per_byte: Option<u64>,
    pub standard_window_util_x_1e9: Option<u64>,
    pub priority_window_util_x_1e9: Option<u64>,
}

/// Policy-only transaction pricing backend.
///
/// The simulator owns block packing; the backend answers pricing queries
/// and accepts post-block samples to update its controller(s).
pub trait PricingBackend: Send + Sync {
    /// Per-byte rate (lovelace/byte) for `lane` after the spec's clamp/floor
    /// and integer rounding. Reads `quote_per_byte: u64` directly — never
    /// derived from an f64 coefficient.
    fn current_quote(&self, lane: Lane) -> u64;

    /// Apply zero or more priced-block samples produced for the most recent
    /// block. Single-lane pricing receives at most one Standard sample per
    /// block; two-lane mechanisms (M2+) typically receive two.
    fn update_after_block(&mut self, samples: &[PricedBlockSample]);

    /// Lane-validity rule for blocks of the given kind.
    /// `LaneValidityRule::None` for single-lane and un-reserved variants;
    /// `LaneValidityRule::PriorityOnly` for RB-reserved RBs (M2+).
    fn lane_validity_rule(&self, _block_kind: BlockKind) -> LaneValidityRule {
        LaneValidityRule::None
    }

    /// Lane-selection order for block packing.
    fn lane_selection_order(&self) -> LaneSelectionOrder {
        LaneSelectionOrder::Fifo
    }

    /// Rational multiplier-floor for two-lane backends. `None` for
    /// single-lane.
    fn min_priority_premium_multiplier(&self) -> Option<Multiplier> {
        None
    }

    /// Snapshot for time-series logging.
    fn snapshot(&self) -> PricingSnapshot;
}
