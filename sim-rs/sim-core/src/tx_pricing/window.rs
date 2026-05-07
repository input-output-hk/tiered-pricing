//! `CapacityWeightedWindow` — the unified utilisation-signal abstraction.
//!
//! Per implementation-plan.md lines 30-44 and mechanism-design.md lines
//! 80-105, all controllers share one window type parameterised by length.
//! Capacity-varying signals (single-lane EIP-1559, both-dynamic standard)
//! use length 32; uniform-capacity priority signals (RB-reserved priority
//! controller) use length 1, which mathematically reduces to the per-block
//! fill rate.
//!
//! The aggregate is `sum(relevant_bytes) / sum(relevant_capacity)`. State
//! is `u64`/`u128` only; no f64 enters simulation-affecting math.

use std::collections::VecDeque;

use super::PricedBlockSample;

/// A bounded ring of `(relevant_bytes, relevant_capacity)` pairs that
/// produces a capacity-weighted aggregate utilisation as a rational.
#[derive(Debug, Clone)]
pub struct CapacityWeightedWindow {
    length: usize,
    samples: VecDeque<Sample>,
    sum_bytes: u128,
    sum_capacity: u128,
}

#[derive(Debug, Clone, Copy)]
struct Sample {
    bytes: u64,
    capacity: u64,
}

impl CapacityWeightedWindow {
    /// `length` is the maximum number of samples retained. `length == 0` is
    /// rejected — a zero-length window has no defined aggregate.
    pub fn new(length: usize) -> anyhow::Result<Self> {
        if length == 0 {
            anyhow::bail!("CapacityWeightedWindow length must be non-zero");
        }
        Ok(Self {
            length,
            samples: VecDeque::with_capacity(length),
            sum_bytes: 0,
            sum_capacity: 0,
        })
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn samples_len(&self) -> usize {
        self.samples.len()
    }

    /// Append a sample, evicting the oldest if full.
    pub fn push(&mut self, sample: PricedBlockSample) {
        let s = Sample {
            bytes: sample.relevant_bytes,
            capacity: sample.relevant_capacity,
        };
        if self.samples.len() == self.length {
            if let Some(old) = self.samples.pop_front() {
                self.sum_bytes -= old.bytes as u128;
                self.sum_capacity -= old.capacity as u128;
            }
        }
        self.samples.push_back(s);
        self.sum_bytes += s.bytes as u128;
        self.sum_capacity += s.capacity as u128;
    }

    /// Aggregate as a rational `(numerator, denominator)` where
    /// `numerator = sum(relevant_bytes)` and
    /// `denominator = sum(relevant_capacity)`.
    ///
    /// Returns `(0, 1)` when the window is empty (no signal yet).
    pub fn aggregate_util(&self) -> (u128, u128) {
        if self.samples.is_empty() || self.sum_capacity == 0 {
            (0, 1)
        } else {
            (self.sum_bytes, self.sum_capacity)
        }
    }

    /// Numerator, for tests and reporting.
    pub fn sum_bytes(&self) -> u128 {
        self.sum_bytes
    }

    /// Denominator, for tests and reporting.
    pub fn sum_capacity(&self) -> u128 {
        self.sum_capacity
    }
}

#[cfg(test)]
mod tests {
    use crate::tx_pricing::{BlockKind, Lane, PricedBlockSample};

    use super::CapacityWeightedWindow;

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
    fn rejects_zero_length() {
        assert!(CapacityWeightedWindow::new(0).is_err());
    }

    #[test]
    fn empty_window_returns_zero_aggregate() {
        let window = CapacityWeightedWindow::new(8).unwrap();
        assert_eq!(window.aggregate_util(), (0, 1));
    }

    #[test]
    fn heterogeneous_rb_and_eb_blocks_aggregate_correctly() {
        // Mechanism-design.md lines 88-94: capacity-weighting blends a
        // small RB and a large EB proportionally.
        // Full RB at 90KB plus half-full EB at 12MB:
        //   sum_bytes    = 90_000 + 6_000_000 = 6_090_000
        //   sum_capacity = 90_000 + 12_000_000 = 12_090_000
        let mut window = CapacityWeightedWindow::new(8).unwrap();
        window.push(rb(90_000, 90_000));
        window.push(eb(6_000_000, 12_000_000));
        let (num, den) = window.aggregate_util();
        assert_eq!((num, den), (6_090_000, 12_090_000));
    }

    #[test]
    fn length_one_reduces_to_per_block_fill_rate() {
        // Implementation-plan.md line 252: "Window length 1 reduces to
        // per-block fill rate (regression test for spec-priority-controller
        // equivalence)."
        let mut window = CapacityWeightedWindow::new(1).unwrap();
        window.push(rb(45_000, 90_000));
        let (num, den) = window.aggregate_util();
        assert_eq!((num, den), (45_000, 90_000));

        window.push(eb(3_000_000, 12_000_000));
        let (num, den) = window.aggregate_util();
        // Older RB sample evicted; only the EB remains, giving its
        // own per-block fill rate of 3M / 12M = 0.25.
        assert_eq!((num, den), (3_000_000, 12_000_000));
    }

    #[test]
    fn ring_evicts_oldest_when_full() {
        let mut window = CapacityWeightedWindow::new(3).unwrap();
        window.push(rb(10, 100));
        window.push(rb(20, 100));
        window.push(rb(30, 100));
        // sum: 60 / 300 = 0.2
        let (num, den) = window.aggregate_util();
        assert_eq!((num, den), (60, 300));

        window.push(rb(70, 100)); // evicts the 10/100 sample
        let (num, den) = window.aggregate_util();
        assert_eq!((num, den), (120, 300));
    }

    #[test]
    fn endorsement_only_rb_with_zero_bytes_drags_aggregate_down() {
        // Mechanism-design.md line 94: an endorsement-only RB contributes
        // 0 bytes to the numerator and its body capacity to the denominator.
        let mut window = CapacityWeightedWindow::new(4).unwrap();
        window.push(rb(90_000, 90_000)); // saturated RB
        window.push(rb(0, 90_000)); // endorsement-only RB
        let (num, den) = window.aggregate_util();
        assert_eq!((num, den), (90_000, 180_000));
    }
}
