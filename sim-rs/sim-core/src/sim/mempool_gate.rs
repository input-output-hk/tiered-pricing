//! Spec-faithful mempool fee-gate (implementation-plan.md §Spec-first
//! transaction lifecycle, lines 79-102).
//!
//! Responsibilities:
//! - **Admission**: at submission, prospective `posted_fee = minFeeB +
//!   current_quote(posted_lane) × bytes`. Reject if `posted_fee >
//!   max_fee_lovelace` or the byte cap would be exceeded.
//! - **Revalidation**: after every controller update, walk the resident
//!   set and emit eviction records for txs whose lane quote drifted above
//!   their `max_fee_lovelace`.
//! - **Lane-aware byte tracking**: per-lane resident byte counts. M1 only
//!   uses the `Standard` bucket; the type already supports both for M2+.
//! - **Inclusion charging**: `actual_fee_lovelace = minFeeB +
//!   quote_per_byte(served_lane) × bytes` and
//!   `refund_lovelace = max_fee_lovelace − actual_fee_lovelace`.
//!
//! All state is `u64`/`u128`; no f64. The fee rounding regime
//! (implementation-plan.md lines 92-95) is applied identically in
//! [`fee_at`] and the resulting `posted_fee` and `actual_fee_lovelace`
//! arithmetic.
//!
//! The gate intentionally does not own UTxO/conflict tracking — that
//! lives in the existing `Mempool` in `linear_leios.rs`. The two layers
//! cooperate: the gate is consulted on admission (fee + byte cap),
//! revalidation, and inclusion; the conflict-tracking mempool decides
//! UTxO conflicts and selection ordering.

use std::collections::BTreeMap;

use crate::{
    config::MempoolGateConfig,
    model::{Transaction, TransactionId},
    tx_pricing::Lane,
};

/// Reason the gate rejected an admission.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum AdmissionRejection {
    /// `posted_fee > max_fee_lovelace`: user authorised less than the
    /// current quote.
    InsufficientMaxFee {
        posted_fee: u64,
        max_fee_lovelace: u64,
    },
    /// Mempool byte cap would be exceeded if we admitted this tx.
    /// Reject-only; no eviction of valid txs to make room
    /// (implementation-plan.md line 89, 126).
    ByteCapExceeded {
        current_total_bytes: u64,
        tx_bytes: u64,
        cap: u64,
    },
    /// Fee arithmetic would overflow u64. Treated as a config/generation
    /// error from the user's perspective.
    FeeOverflow,
}

/// One eviction record produced during revalidation. The simulator uses
/// these to emit `TXEvictedQuoteDrift` events and remove the tx from its
/// other mempool data structures.
#[derive(Debug, Clone)]
pub struct EvictionRecord {
    pub tx_id: TransactionId,
    pub posted_lane: Lane,
    pub bytes: u64,
    /// Quote that exceeded the tx's max-fee budget when revalidation ran.
    pub current_quote_per_byte: u64,
    pub max_fee_lovelace: u64,
}

/// Inclusion charge produced when a tx is committed in a block.
#[derive(Debug, Clone, Copy)]
pub struct InclusionCharge {
    pub posted_lane: Lane,
    pub served_lane: Lane,
    pub bytes: u64,
    pub max_fee_lovelace: u64,
    pub actual_fee_lovelace: u64,
    pub refund_lovelace: u64,
}

#[derive(Debug, Clone, Copy)]
struct ResidentEntry {
    posted_lane: Lane,
    bytes: u64,
    max_fee_lovelace: u64,
}

/// Fee-validity + per-lane-bytes tracker.
///
/// The gate is the source of truth for "is this tx still admissible at
/// the current quote?". It does not track UTxO conflicts or selection
/// ordering — those remain in the existing `Mempool`.
#[derive(Debug, Clone)]
pub struct MempoolGate {
    config: MempoolGateConfig,
    resident: BTreeMap<TransactionId, ResidentEntry>,
    bytes_standard: u64,
    bytes_priority: u64,
}

impl MempoolGate {
    pub fn new(config: MempoolGateConfig) -> Self {
        Self {
            config,
            resident: BTreeMap::new(),
            bytes_standard: 0,
            bytes_priority: 0,
        }
    }

    pub fn config(&self) -> MempoolGateConfig {
        self.config
    }

    /// Total bytes resident across all lanes.
    pub fn total_bytes(&self) -> u64 {
        self.bytes_standard.saturating_add(self.bytes_priority)
    }

    /// Bytes resident in `lane`.
    pub fn bytes_in_lane(&self, lane: Lane) -> u64 {
        match lane {
            Lane::Standard => self.bytes_standard,
            Lane::Priority => self.bytes_priority,
        }
    }

    pub fn len(&self) -> usize {
        self.resident.len()
    }

    pub fn is_empty(&self) -> bool {
        self.resident.is_empty()
    }

    pub fn contains(&self, tx_id: &TransactionId) -> bool {
        self.resident.contains_key(tx_id)
    }

    /// Compute `posted_fee = minFeeB + quote_per_byte × bytes` with
    /// checked arithmetic. `Err(_)` on u64 overflow.
    pub fn fee_at(&self, quote_per_byte: u64, bytes: u64) -> Option<u64> {
        quote_per_byte
            .checked_mul(bytes)
            .and_then(|q| q.checked_add(self.config.min_fee_b))
    }

    /// Admit a tx. Validates fee (against `quote_per_byte` for
    /// `tx.posted_lane`) and the byte cap. On success, records the tx as
    /// resident and updates per-lane byte counts.
    pub fn try_admit(
        &mut self,
        tx: &Transaction,
        quote_per_byte_for_posted_lane: u64,
    ) -> Result<(), AdmissionRejection> {
        // Fee check. Overflow is possible if a malicious config sets
        // bytes × quote past u64::MAX; we treat that as rejection rather
        // than a panic per implementation-plan.md line 94.
        let Some(posted_fee) = self.fee_at(quote_per_byte_for_posted_lane, tx.bytes) else {
            return Err(AdmissionRejection::FeeOverflow);
        };
        if posted_fee > tx.max_fee_lovelace {
            return Err(AdmissionRejection::InsufficientMaxFee {
                posted_fee,
                max_fee_lovelace: tx.max_fee_lovelace,
            });
        }

        // Byte-cap check. Reject-new-when-full, no margin-eviction.
        let current_total = self.total_bytes();
        let new_total = current_total.saturating_add(tx.bytes);
        if new_total > self.config.max_total_size_bytes {
            return Err(AdmissionRejection::ByteCapExceeded {
                current_total_bytes: current_total,
                tx_bytes: tx.bytes,
                cap: self.config.max_total_size_bytes,
            });
        }

        // Commit.
        let entry = ResidentEntry {
            posted_lane: tx.posted_lane,
            bytes: tx.bytes,
            max_fee_lovelace: tx.max_fee_lovelace,
        };
        if self.resident.insert(tx.id, entry).is_none() {
            self.add_lane_bytes(tx.posted_lane, tx.bytes);
        }
        Ok(())
    }

    /// Walk the resident set and return eviction records for any tx
    /// whose lane quote has risen above its `max_fee_lovelace`. Removes
    /// each evicted tx from the resident set and updates byte counts.
    ///
    /// `quote_for_lane(lane) -> quote_per_byte` is provided by the
    /// caller so the gate stays decoupled from the pricing backend.
    pub fn revalidate<F>(&mut self, mut quote_for_lane: F) -> Vec<EvictionRecord>
    where
        F: FnMut(Lane) -> u64,
    {
        // Cache lane quotes once to avoid per-tx callback churn.
        let q_standard = quote_for_lane(Lane::Standard);
        let q_priority = quote_for_lane(Lane::Priority);
        let mut to_evict = Vec::new();
        for (tx_id, entry) in &self.resident {
            let q = match entry.posted_lane {
                Lane::Standard => q_standard,
                Lane::Priority => q_priority,
            };
            // posted_fee at the current quote, with overflow → eviction.
            let posted_fee = match q
                .checked_mul(entry.bytes)
                .and_then(|x| x.checked_add(self.config.min_fee_b))
            {
                Some(v) => v,
                None => u64::MAX,
            };
            if posted_fee > entry.max_fee_lovelace {
                to_evict.push(EvictionRecord {
                    tx_id: *tx_id,
                    posted_lane: entry.posted_lane,
                    bytes: entry.bytes,
                    current_quote_per_byte: q,
                    max_fee_lovelace: entry.max_fee_lovelace,
                });
            }
        }
        for record in &to_evict {
            self.resident.remove(&record.tx_id);
            self.sub_lane_bytes(record.posted_lane, record.bytes);
        }
        to_evict
    }

    /// Mark a tx as included in a block at `served_lane` with the given
    /// `quote_per_byte_at_served_lane`. Removes the tx from the resident
    /// set and returns the inclusion charge (actual fee, refund).
    ///
    /// `None` if the tx wasn't resident (e.g. removed elsewhere first).
    pub fn on_inclusion(
        &mut self,
        tx_id: TransactionId,
        served_lane: Lane,
        quote_per_byte_at_served_lane: u64,
    ) -> Option<InclusionCharge> {
        let entry = self.resident.remove(&tx_id)?;
        self.sub_lane_bytes(entry.posted_lane, entry.bytes);
        // actual_fee_lovelace = minFeeB + quote_per_byte(served) × bytes
        let actual_fee = quote_per_byte_at_served_lane
            .checked_mul(entry.bytes)
            .and_then(|q| q.checked_add(self.config.min_fee_b))
            .unwrap_or(u64::MAX);
        // refund = max_fee − actual_fee, saturating at zero (the spec's
        // refund is non-negative; included txs by construction satisfy
        // their max-fee budget at the served lane).
        let refund = entry.max_fee_lovelace.saturating_sub(actual_fee);
        Some(InclusionCharge {
            posted_lane: entry.posted_lane,
            served_lane,
            bytes: entry.bytes,
            max_fee_lovelace: entry.max_fee_lovelace,
            actual_fee_lovelace: actual_fee,
            refund_lovelace: refund,
        })
    }

    /// Remove a resident tx for a non-quote-drift reason (e.g. UTxO
    /// conflict from the cooperating mempool). No event/charge is
    /// produced; this just keeps the gate in sync.
    pub fn remove_silent(&mut self, tx_id: TransactionId) {
        if let Some(entry) = self.resident.remove(&tx_id) {
            self.sub_lane_bytes(entry.posted_lane, entry.bytes);
        }
    }

    fn add_lane_bytes(&mut self, lane: Lane, bytes: u64) {
        match lane {
            Lane::Standard => self.bytes_standard = self.bytes_standard.saturating_add(bytes),
            Lane::Priority => self.bytes_priority = self.bytes_priority.saturating_add(bytes),
        }
    }

    fn sub_lane_bytes(&mut self, lane: Lane, bytes: u64) {
        match lane {
            Lane::Standard => self.bytes_standard = self.bytes_standard.saturating_sub(bytes),
            Lane::Priority => self.bytes_priority = self.bytes_priority.saturating_sub(bytes),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        config::MempoolGateConfig,
        model::{Transaction, TransactionId},
        tx_pricing::Lane,
    };

    use super::{AdmissionRejection, MempoolGate};

    const MIN_FEE_A: u64 = 44;
    const MIN_FEE_B: u64 = 155_381;

    fn cfg(cap: u64) -> MempoolGateConfig {
        MempoolGateConfig {
            max_total_size_bytes: cap,
            min_fee_a: MIN_FEE_A,
            min_fee_b: MIN_FEE_B,
        }
    }

    fn tx(id: u64, bytes: u64, max_fee: u64) -> Arc<Transaction> {
        Arc::new(Transaction {
            id: TransactionId::new(id),
            shard: 0,
            bytes,
            input_id: id,
            overcollateralization_factor: 0,
            max_fee_lovelace: max_fee,
            posted_lane: Lane::Standard,
            value_lovelace: 0,
            urgency: 1.0,
            urgency_component_index: 0,
        })
    }

    #[test]
    fn admits_when_fee_fits_under_max_fee() {
        let mut gate = MempoolGate::new(cfg(1_000_000));
        // posted_fee = 155_381 + 44 × 1000 = 199_381
        let t = tx(1, 1000, 200_000);
        gate.try_admit(&t, MIN_FEE_A).unwrap();
        assert!(gate.contains(&t.id));
        assert_eq!(gate.bytes_in_lane(Lane::Standard), 1000);
    }

    #[test]
    fn rejects_when_max_fee_below_quote() {
        // Implementation-plan.md verification §M1, line 299:
        //   "maxFee admission rejects when prospective posted_fee >
        //    max_fee_lovelace."
        let mut gate = MempoolGate::new(cfg(1_000_000));
        // posted_fee = 155_381 + 44 × 1000 = 199_381; max_fee = 199_380.
        let t = tx(1, 1000, 199_380);
        let err = gate.try_admit(&t, MIN_FEE_A).unwrap_err();
        match err {
            AdmissionRejection::InsufficientMaxFee {
                posted_fee,
                max_fee_lovelace,
            } => {
                assert_eq!(posted_fee, 199_381);
                assert_eq!(max_fee_lovelace, 199_380);
            }
            other => panic!("expected InsufficientMaxFee, got {other:?}"),
        }
        assert!(!gate.contains(&t.id));
        assert_eq!(gate.bytes_in_lane(Lane::Standard), 0);
    }

    #[test]
    fn rejects_when_byte_cap_exceeded() {
        // Plan §M1 line 305: "Mempool cap rejects new arrivals at
        // capacity; no eviction of valid txs."
        let mut gate = MempoolGate::new(cfg(2000));
        let t1 = tx(1, 1500, u64::MAX);
        let t2 = tx(2, 700, u64::MAX);
        gate.try_admit(&t1, MIN_FEE_A).unwrap();
        let err = gate.try_admit(&t2, MIN_FEE_A).unwrap_err();
        assert!(matches!(err, AdmissionRejection::ByteCapExceeded { .. }));
        // The first tx is unaffected.
        assert!(gate.contains(&t1.id));
        assert!(!gate.contains(&t2.id));
        assert_eq!(gate.bytes_in_lane(Lane::Standard), 1500);
    }

    #[test]
    fn revalidation_evicts_on_quote_drift() {
        // Plan §M1 line 300: "Quote drift after admission triggers
        // eviction on next controller update."
        let mut gate = MempoolGate::new(cfg(1_000_000));
        // posted_fee at quote=44: 199_381. We pick max_fee just above to
        // force eviction once quote rises.
        let t = tx(1, 1000, 200_000);
        gate.try_admit(&t, MIN_FEE_A).unwrap();
        // Now quote rises to 50: posted_fee = 155_381 + 50 × 1000 = 205_381.
        let evicted = gate.revalidate(|_| 50);
        assert_eq!(evicted.len(), 1);
        assert_eq!(evicted[0].tx_id, t.id);
        assert_eq!(evicted[0].current_quote_per_byte, 50);
        assert!(!gate.contains(&t.id));
        assert_eq!(gate.bytes_in_lane(Lane::Standard), 0);
    }

    #[test]
    fn revalidation_keeps_txs_whose_max_fee_still_covers_quote() {
        let mut gate = MempoolGate::new(cfg(1_000_000));
        // generous max-fee, even at quote=100 fee = 255_381.
        let t = tx(1, 1000, 1_000_000);
        gate.try_admit(&t, MIN_FEE_A).unwrap();
        let evicted = gate.revalidate(|_| 100);
        assert_eq!(evicted.len(), 0);
        assert!(gate.contains(&t.id));
    }

    #[test]
    fn inclusion_charges_at_served_lane_quote_with_refund() {
        // Plan §M1 lines 301-302:
        //   "Inclusion fee at served_lane matches minFeeB + current_quote × bytes."
        //   "Refund equals max_fee_lovelace − actual_fee_lovelace."
        let mut gate = MempoolGate::new(cfg(1_000_000));
        let t = tx(1, 1000, 1_000_000);
        gate.try_admit(&t, MIN_FEE_A).unwrap();
        // quote at served lane is now 60 (quote drifted up since admission).
        let charge = gate.on_inclusion(t.id, Lane::Standard, 60).unwrap();
        let expected_actual = MIN_FEE_B + 60 * 1000;
        assert_eq!(charge.actual_fee_lovelace, expected_actual);
        assert_eq!(charge.refund_lovelace, t.max_fee_lovelace - expected_actual);
        assert_eq!(charge.posted_lane, Lane::Standard);
        assert_eq!(charge.served_lane, Lane::Standard);
        // resident set is now empty.
        assert!(!gate.contains(&t.id));
    }

    #[test]
    fn inclusion_returns_none_when_not_resident() {
        let mut gate = MempoolGate::new(cfg(1_000_000));
        assert!(
            gate.on_inclusion(TransactionId::new(99), Lane::Standard, 44)
                .is_none()
        );
    }

    #[test]
    fn fee_at_uses_pinned_rounding_regime() {
        let gate = MempoolGate::new(cfg(1_000_000));
        // Identical formula in admission, revalidation, inclusion
        // (plan lines 92-95).
        assert_eq!(gate.fee_at(44, 1000), Some(MIN_FEE_B + 44 * 1000));
        // Overflow returns None.
        assert_eq!(gate.fee_at(u64::MAX, 2), None);
    }
}
