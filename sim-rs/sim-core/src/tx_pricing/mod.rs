//! Transaction-pricing layer (chain-derived controller — spike 007).
//!
//! See `docs/phase-2/mechanism-design.md` for the spec these modules implement,
//! and `docs/phase-2/implementation-plan.md` lines 26-180 for the architecture.
//!
//! A `PricingBackend` is a **pure-function policy** under the chain-derived
//! pattern: it answers "given the parent block's `derived_quote` and
//! `window_aggregate`, plus the samples emitted by the parent (and any
//! samples evicted from the tail of the window), what is the child's
//! `(PerLaneQuote, WindowAggregate)`?" The backend holds no mutable
//! controller state — `derived_quote` is computed per block at production
//! and stored on `LinearRankingBlock` as a header field. This matches
//! EIP-1559's stateless pattern and closes WR-1 by construction: orphan
//! blocks from slot battles carry their own `derived_quote` which is
//! discarded with the block.
//!
//! All simulation-affecting state (controller coefficients, window contents,
//! quote-per-byte) is stored as `u64`/`u128` integers or as rationals. f64
//! never enters this module's hot paths. `compute_derived_quote` is pure
//! and integer/u128 throughout.

pub mod single_lane;
pub mod two_lane;
pub mod window;

use serde::{Deserialize, Serialize};

pub use single_lane::{BaselinePricing, Eip1559Pricing, Eip1559Settings};
pub use two_lane::{TwoLanePricing, TwoLaneSettings, TwoLaneVariant};
pub use window::{aggregate_from_chain, update_aggregate};

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

/// Lane breakdown of a priced block's transaction bytes.
///
/// The simulator collates this from the block's transactions and hands
/// it to the backend's [`PricingBackend::samples_for_block`] so the
/// backend can decide which controllers to feed and what numerator/
/// denominator each sample uses. The cap-on-priority-bytes rule from
/// implementation-plan.md line 73
/// (`relevant_bytes = min(priority_paying_bytes, max_block_size)` for
/// the RB-reserved priority controller's EB sample) is applied **inside
/// the variant's override**, not here — this struct carries raw bytes.
#[derive(Debug, Clone, Copy)]
pub struct BlockLaneBreakdown {
    /// Bytes of transactions whose `posted_lane = Priority`.
    pub priority_paying_bytes: u64,
    /// Bytes of transactions whose `posted_lane = Standard`.
    pub standard_paying_bytes: u64,
    /// The block's own capacity (RB body cap or EB tx-referenced cap).
    /// Two-lane samples use this directly or substitute one RB-worth
    /// per the spec (mechanism-design.md lines 168-180).
    pub block_capacity: u64,
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

/// Snapshot of pricing state for time-series logging. Derived from a
/// canonical block's `derived_quote` + `window_aggregate` (see
/// `snapshot_at`) — not from any node-local controller state.
#[derive(Debug, Clone)]
pub struct PricingSnapshot {
    pub standard_quote_per_byte: u64,
    pub priority_quote_per_byte: Option<u64>,
    pub standard_window_util_x_1e9: Option<u64>,
    pub priority_window_util_x_1e9: Option<u64>,
}

/// Read-only view of the canonical chain exposed to a `PricingBackend`'s
/// pure-function computation. The backend cannot mutate the chain — it
/// only walks ancestors and reads per-block fields. This is the seam
/// that keeps chain-derived computation a pure function of canonical
/// state (spike 007 §"Type-level shape").
pub trait ChainView {
    /// k-th canonical ancestor of `from`, walking back along canonical
    /// parents only. Returns `None` when the chain runs out (cold start)
    /// or `k` exceeds available depth.
    fn ancestor(&self, from: crate::model::BlockId, k: u32) -> Option<crate::model::BlockId>;

    /// Samples that the given canonical block emitted (RB body + endorsed
    /// EB body, per the variant's `samples_for_block` policy). Returns
    /// an empty slice when the block has no recorded samples.
    fn samples_in_block(&self, block_id: crate::model::BlockId) -> &[PricedBlockSample];

    /// `derived_quote` of a given canonical block (read of the field).
    /// `None` when the block is not on this node's canonical chain.
    fn derived_quote(&self, block_id: crate::model::BlockId)
        -> Option<crate::model::PerLaneQuote>;

    /// `window_aggregate` of a given canonical block (for incremental
    /// updates). `None` when the block is not on this node's canonical
    /// chain.
    fn window_aggregate(
        &self,
        block_id: crate::model::BlockId,
    ) -> Option<crate::model::WindowAggregate>;
}

/// Pure-function transaction-pricing policy (chain-derived; spike 007).
///
/// The simulator owns block packing and the canonical chain. The
/// backend is a configuration carrier + pure-function compute step:
/// given the parent block's `derived_quote` + `window_aggregate` +
/// the parent's samples + any evicted samples, return the child's
/// `(PerLaneQuote, WindowAggregate)`. No `&mut self` anywhere.
pub trait PricingBackend: Send + Sync {
    /// Compute the child block's `derived_quote` and `window_aggregate`
    /// as a pure function of the parent's chain-derived state.
    ///
    /// - `parent_quote` — parent block's `derived_quote` (or the
    ///   cold-start initial quote when the parent has none).
    /// - `parent_aggregate` — parent block's `window_aggregate` (or
    ///   `WindowAggregate::ZERO` for cold start).
    /// - `parent_samples` — samples emitted by the parent block, to
    ///   fold into the aggregate.
    /// - `evicted_samples` — samples falling off the tail of the window
    ///   this step (from the block at distance `window_length + 1` back,
    ///   or empty during the warm-up regime).
    fn compute_derived_quote(
        &self,
        parent_quote: crate::model::PerLaneQuote,
        parent_aggregate: crate::model::WindowAggregate,
        parent_samples: &[PricedBlockSample],
        evicted_samples: &[PricedBlockSample],
    ) -> (crate::model::PerLaneQuote, crate::model::WindowAggregate);

    /// Effective window length used by this backend. For
    /// `BaselinePricing` returns `usize::MAX` (no window). For
    /// `Eip1559Pricing` returns its configured length. For
    /// `TwoLanePricing` returns the maximum of both lanes' lengths
    /// (the simulator uses this to size the per-block-samples cache).
    fn effective_window_length(&self) -> usize;

    /// Cold-start initial quote for `lane`. Used by the simulator at
    /// genesis (no parent RB exists) so the first block's
    /// `derived_quote` has a defined starting value.
    fn cold_start_quote(&self, lane: Lane) -> u64;

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

    /// Decide which `PricedBlockSample`s this priced block emits to
    /// which controller(s). Default emits one `Standard` sample over
    /// total bytes — correct for every single-lane backend
    /// (priority bytes are always 0 in single-lane, so this collapses
    /// to the M1 emission rule). Two-lane backends override per
    /// variant (implementation-plan.md lines 65-77).
    fn samples_for_block(
        &self,
        block_kind: BlockKind,
        breakdown: &BlockLaneBreakdown,
    ) -> Vec<PricedBlockSample> {
        let total_bytes = breakdown
            .priority_paying_bytes
            .saturating_add(breakdown.standard_paying_bytes);
        vec![PricedBlockSample {
            block_kind,
            controller_lane: Lane::Standard,
            relevant_bytes: total_bytes,
            relevant_capacity: breakdown.block_capacity,
        }]
    }
}

/// Render a `PricingSnapshot` from a canonical block's chain-derived
/// state. Used for time-series logging and `PricingTick` events.
/// Two-lane variants render both quotes; single-lane variants leave
/// `priority_quote_per_byte` as `None` (priority and standard share the
/// same controller — the convention matches the legacy renderer).
pub fn snapshot_at(
    derived_quote: crate::model::PerLaneQuote,
    aggregate: crate::model::WindowAggregate,
    is_two_lane: bool,
) -> PricingSnapshot {
    fn util_x_1e9(num: u128, den: u128) -> Option<u64> {
        if den == 0 {
            None
        } else {
            let scaled = num.saturating_mul(1_000_000_000) / den;
            Some(u64::try_from(scaled).unwrap_or(u64::MAX))
        }
    }
    let standard_util = util_x_1e9(aggregate.standard_sum_bytes, aggregate.standard_sum_capacity);
    if is_two_lane {
        let priority_util =
            util_x_1e9(aggregate.priority_sum_bytes, aggregate.priority_sum_capacity);
        PricingSnapshot {
            standard_quote_per_byte: derived_quote.standard,
            priority_quote_per_byte: Some(derived_quote.priority),
            standard_window_util_x_1e9: standard_util,
            priority_window_util_x_1e9: priority_util,
        }
    } else {
        PricingSnapshot {
            standard_quote_per_byte: derived_quote.standard,
            priority_quote_per_byte: None,
            standard_window_util_x_1e9: standard_util,
            priority_window_util_x_1e9: None,
        }
    }
}
