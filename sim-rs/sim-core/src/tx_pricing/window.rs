//! `CapacityWeightedWindow` — pure-function capacity-weighted-window
//! aggregator (chain-derived; spike 007).
//!
//! Per implementation-plan.md lines 30-44 and mechanism-design.md lines
//! 80-105, all controllers share one aggregation shape parameterised by
//! length. Capacity-varying signals (single-lane EIP-1559, both-dynamic
//! standard) use length 32; uniform-capacity priority signals
//! (RB-reserved priority controller) use length 1, which mathematically
//! reduces to the per-block fill rate.
//!
//! Under chain-derivation, the window is no longer a persistent
//! per-node ring buffer. Each block carries its own `WindowAggregate`
//! (`model.rs`) so descendants can step the controller in O(1):
//!   - Add the parent's samples to the parent's aggregate.
//!   - Subtract any samples that have just rolled off the tail
//!     (block at distance `window_length + 1` back).
//!
//! All state is `u128`/`u64` integers. f64 never enters this module.

use crate::model::WindowAggregate;

use super::{Lane, PricedBlockSample};

/// Walk an iterator of `PricedBlockSample`s and produce a
/// `WindowAggregate`. Used for cold-start computation and tests.
/// Bytes / capacity are split by `controller_lane`.
///
/// Note: `blocks_in_window` is incremented once per sample (not once
/// per block) — multi-sample blocks (e.g. both-dynamic EBs emit two
/// samples) increment the counter twice. This matches the legacy
/// `CapacityWeightedWindow::push` behaviour where each push counts
/// as one "ring slot". Per-controller window length is per-lane, so
/// this convention is invariant under lane filtering.
pub fn aggregate_from_chain<'a>(
    samples: impl IntoIterator<Item = &'a PricedBlockSample>,
) -> WindowAggregate {
    let mut agg = WindowAggregate::ZERO;
    for sample in samples {
        add_one(&mut agg, sample);
    }
    agg
}

/// Incrementally update an aggregate: add `add_samples`, subtract
/// `evict_samples`. The `window_length` is informational here (no
/// trimming happens — the caller is responsible for sourcing
/// `evict_samples` correctly to keep the per-lane sample count bounded
/// at `window_length`).
///
/// The caller (a `PricingBackend::compute_derived_quote` impl) is
/// responsible for sourcing `evict_samples` correctly: empty during the
/// warm-up regime, and the samples of the block at distance
/// `window_length + 1` once warm.
pub fn update_aggregate(
    parent: WindowAggregate,
    add_samples: &[PricedBlockSample],
    evict_samples: &[PricedBlockSample],
    _window_length: usize,
) -> WindowAggregate {
    let mut agg = parent;
    for sample in add_samples {
        add_one(&mut agg, sample);
    }
    for sample in evict_samples {
        sub_one(&mut agg, sample);
    }
    agg
}

fn add_one(agg: &mut WindowAggregate, sample: &PricedBlockSample) {
    match sample.controller_lane {
        Lane::Standard => {
            agg.standard_sum_bytes = agg
                .standard_sum_bytes
                .saturating_add(sample.relevant_bytes as u128);
            agg.standard_sum_capacity = agg
                .standard_sum_capacity
                .saturating_add(sample.relevant_capacity as u128);
        }
        Lane::Priority => {
            agg.priority_sum_bytes = agg
                .priority_sum_bytes
                .saturating_add(sample.relevant_bytes as u128);
            agg.priority_sum_capacity = agg
                .priority_sum_capacity
                .saturating_add(sample.relevant_capacity as u128);
        }
    }
    agg.blocks_in_window = agg.blocks_in_window.saturating_add(1);
}

fn sub_one(agg: &mut WindowAggregate, sample: &PricedBlockSample) {
    match sample.controller_lane {
        Lane::Standard => {
            agg.standard_sum_bytes = agg
                .standard_sum_bytes
                .saturating_sub(sample.relevant_bytes as u128);
            agg.standard_sum_capacity = agg
                .standard_sum_capacity
                .saturating_sub(sample.relevant_capacity as u128);
        }
        Lane::Priority => {
            agg.priority_sum_bytes = agg
                .priority_sum_bytes
                .saturating_sub(sample.relevant_bytes as u128);
            agg.priority_sum_capacity = agg
                .priority_sum_capacity
                .saturating_sub(sample.relevant_capacity as u128);
        }
    }
    agg.blocks_in_window = agg.blocks_in_window.saturating_sub(1);
}

#[cfg(test)]
mod tests {
    use crate::tx_pricing::{BlockKind, Lane, PricedBlockSample};

    use super::*;

    fn rb(bytes: u64, capacity: u64) -> PricedBlockSample {
        PricedBlockSample {
            block_kind: BlockKind::RankingBlock,
            controller_lane: Lane::Standard,
            relevant_bytes: bytes,
            relevant_capacity: capacity,
        }
    }

    fn eb(bytes: u64, capacity: u64) -> PricedBlockSample {
        PricedBlockSample {
            block_kind: BlockKind::EndorserBlock,
            controller_lane: Lane::Standard,
            relevant_bytes: bytes,
            relevant_capacity: capacity,
        }
    }

    #[test]
    fn empty_iterator_returns_zero_aggregate() {
        let agg = aggregate_from_chain(std::iter::empty::<&PricedBlockSample>());
        assert_eq!(agg, WindowAggregate::ZERO);
    }

    #[test]
    fn heterogeneous_rb_and_eb_blocks_aggregate_correctly() {
        // Mechanism-design.md lines 88-94: capacity-weighting blends a
        // small RB and a large EB proportionally.
        let samples = vec![rb(90_000, 90_000), eb(6_000_000, 12_000_000)];
        let agg = aggregate_from_chain(samples.iter());
        let (n, d) = agg.aggregate_util(Lane::Standard);
        assert_eq!((n, d), (6_090_000, 12_090_000));
    }

    #[test]
    fn length_one_reduces_to_per_block_fill_rate() {
        let first = vec![rb(45_000, 90_000)];
        let agg1 = update_aggregate(WindowAggregate::ZERO, &first, &[], 1);
        let (n, d) = agg1.aggregate_util(Lane::Standard);
        assert_eq!((n, d), (45_000, 90_000));

        let second = vec![eb(3_000_000, 12_000_000)];
        let agg2 = update_aggregate(agg1, &second, &first, 1);
        let (n, d) = agg2.aggregate_util(Lane::Standard);
        assert_eq!((n, d), (3_000_000, 12_000_000));
    }

    #[test]
    fn ring_evicts_oldest_when_full() {
        let s1 = vec![rb(10, 100)];
        let s2 = vec![rb(20, 100)];
        let s3 = vec![rb(30, 100)];
        let s4 = vec![rb(70, 100)];
        let agg = update_aggregate(WindowAggregate::ZERO, &s1, &[], 3);
        let agg = update_aggregate(agg, &s2, &[], 3);
        let agg = update_aggregate(agg, &s3, &[], 3);
        let (n, d) = agg.aggregate_util(Lane::Standard);
        assert_eq!((n, d), (60, 300));

        let agg = update_aggregate(agg, &s4, &s1, 3);
        let (n, d) = agg.aggregate_util(Lane::Standard);
        assert_eq!((n, d), (120, 300));
    }

    #[test]
    fn endorsement_only_rb_with_zero_bytes_drags_aggregate_down() {
        let s1 = vec![rb(90_000, 90_000)];
        let s2 = vec![rb(0, 90_000)];
        let agg = update_aggregate(WindowAggregate::ZERO, &s1, &[], 4);
        let agg = update_aggregate(agg, &s2, &[], 4);
        let (n, d) = agg.aggregate_util(Lane::Standard);
        assert_eq!((n, d), (90_000, 180_000));
    }

    #[test]
    fn aggregate_from_chain_is_deterministic() {
        let samples = vec![rb(90_000, 90_000), eb(6_000_000, 12_000_000), rb(0, 90_000)];
        let a = aggregate_from_chain(samples.iter());
        let b = aggregate_from_chain(samples.iter());
        assert_eq!(a, b);
    }
}
