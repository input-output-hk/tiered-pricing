use std::{collections::BTreeMap, fmt::Display, sync::Arc};

use crate::{clock::Timestamp, config::NodeId, tx_pricing::BlockKind};
use serde::{Deserialize, Serialize};

macro_rules! id_wrapper {
    ($outer:ident, $inner:ty) => {
        #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $outer($inner);
        impl Display for $outer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
        impl Serialize for $outer {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(&self.0.to_string())
            }
        }
        impl $outer {
            #[allow(unused)]
            pub fn new(value: $inner) -> Self {
                Self(value)
            }
        }
    };
}

#[derive(Clone, Debug, Serialize)]
pub struct CpuTaskId<Node = NodeId> {
    pub node: Node,
    pub index: u64,
}

impl<Node: Display> Display for CpuTaskId<Node> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}-{}", self.node, self.index))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId<Node = NodeId> {
    pub slot: u64,
    pub producer: Node,
}

impl<Node: Display> Display for BlockId<Node> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}-{}", self.slot, self.producer))
    }
}

impl<Node: Display> Serialize for BlockId<Node> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct Block {
    pub id: BlockId,
    pub vrf: u64,
    pub parent: Option<BlockId>,
    pub header_bytes: u64,
    pub endorsement: Option<Endorsement>,
    pub transactions: Vec<Arc<Transaction>>,
}

impl Block {
    pub fn bytes(&self) -> u64 {
        self.header_bytes
            + self
                .endorsement
                .as_ref()
                .map(|e| e.size_bytes)
                .unwrap_or_default()
            + self.transactions.iter().map(|t| t.bytes).sum::<u64>()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinearRankingBlockHeader {
    pub id: BlockId,
    pub vrf: u64,
    pub parent: Option<BlockId>,
    pub bytes: u64,
    pub eb_announcement: Option<EndorserBlockId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinearRankingBlock {
    pub header: LinearRankingBlockHeader,
    pub transactions: Vec<Arc<Transaction>>,
    pub endorsement: Option<Endorsement>,
}

impl LinearRankingBlock {
    pub fn bytes(&self) -> u64 {
        self.header.bytes + self.transactions.iter().map(|t| t.bytes).sum::<u64>()
    }
}

id_wrapper!(TransactionId, u64);
id_wrapper!(ActorId, u64);
id_wrapper!(TierId, usize);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UrgencyProfile {
    #[serde(alias = "Indifferent")]
    Indifferent,
    #[serde(alias = "TimeBoxed")]
    TimeBoxed { max_slots: u64 },
    #[serde(alias = "Urgent")]
    Urgent,
    #[serde(alias = "ExponentialDecay")]
    ExponentialDecay {
        /// Portion of value retained each additional slot, in millionths.
        /// 1_000_000 means no decay, 500_000 means 50% retained per extra slot.
        retained_per_million: u32,
    },
    #[serde(alias = "LinearDecay")]
    LinearDecay {
        /// Absolute value lost for each additional slot beyond delay=1.
        value_drop_per_slot: u64,
    },
    #[serde(alias = "ValuationTable")]
    ValuationTable {
        /// Entry at index `i` is retained value (in millionths of `initial_value`)
        /// for delay `i + 1`.
        retained_per_million_by_delay: Vec<u32>,
        /// Optional retained value (in millionths) for delays beyond the table.
        /// If omitted, the last table value is reused.
        tail_retained_per_million: Option<u32>,
    },
}

impl UrgencyProfile {
    pub fn value_at_delay(&self, initial_value: u64, delay_slots: u64) -> u64 {
        fn retained_value(initial_value: u64, retained_per_million: u32) -> u64 {
            let retained = retained_per_million.min(1_000_000) as u128;
            ((initial_value as u128).saturating_mul(retained) / 1_000_000u128) as u64
        }

        let delay = delay_slots.max(1);
        match self {
            UrgencyProfile::Indifferent => initial_value,
            UrgencyProfile::TimeBoxed { max_slots } => {
                if delay <= *max_slots {
                    initial_value
                } else {
                    0
                }
            }
            // Urgent traffic keeps legacy behavior: it only values the minimal-delay lane.
            UrgencyProfile::Urgent => {
                if delay == 1 {
                    initial_value
                } else {
                    0
                }
            }
            UrgencyProfile::ExponentialDecay {
                retained_per_million,
            } => {
                if *retained_per_million >= 1_000_000 {
                    return initial_value;
                }
                let mut value = initial_value as u128;
                let ratio = *retained_per_million as u128;
                for _ in 1..delay {
                    value = value.saturating_mul(ratio) / 1_000_000u128;
                    if value == 0 {
                        break;
                    }
                }
                value.min(u64::MAX as u128) as u64
            }
            UrgencyProfile::LinearDecay {
                value_drop_per_slot,
            } => {
                let extra_slots = delay.saturating_sub(1);
                initial_value.saturating_sub(value_drop_per_slot.saturating_mul(extra_slots))
            }
            UrgencyProfile::ValuationTable {
                retained_per_million_by_delay,
                tail_retained_per_million,
            } => {
                if retained_per_million_by_delay.is_empty() {
                    return initial_value;
                }

                let index = delay.saturating_sub(1) as usize;
                let retained = retained_per_million_by_delay
                    .get(index)
                    .copied()
                    .or(*tail_retained_per_million)
                    .or_else(|| retained_per_million_by_delay.last().copied())
                    .unwrap_or(1_000_000);
                retained_value(initial_value, retained)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::UrgencyProfile;

    #[test]
    fn valuation_table_uses_delay_specific_entries() {
        let profile = UrgencyProfile::ValuationTable {
            retained_per_million_by_delay: vec![1_000_000, 800_000, 500_000],
            tail_retained_per_million: Some(100_000),
        };

        assert_eq!(profile.value_at_delay(1_000, 1), 1_000);
        assert_eq!(profile.value_at_delay(1_000, 2), 800);
        assert_eq!(profile.value_at_delay(1_000, 3), 500);
        assert_eq!(profile.value_at_delay(1_000, 7), 100);
    }

    #[test]
    fn valuation_table_reuses_last_entry_when_tail_is_missing() {
        let profile = UrgencyProfile::ValuationTable {
            retained_per_million_by_delay: vec![1_000_000, 750_000],
            tail_retained_per_million: None,
        };

        assert_eq!(profile.value_at_delay(2_000, 2), 1_500);
        assert_eq!(profile.value_at_delay(2_000, 9), 1_500);
    }

    #[test]
    fn valuation_table_clamps_retained_values_to_one() {
        let profile = UrgencyProfile::ValuationTable {
            retained_per_million_by_delay: vec![1_500_000],
            tail_retained_per_million: None,
        };

        assert_eq!(profile.value_at_delay(777, 1), 777);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub id: TransactionId,
    pub actor_id: ActorId,
    pub shard: u64,
    pub bytes: u64,
    pub submission_slot: u64,
    /// Slot when this transaction first became active in some node's mempool.
    pub mempool_entry_slot: Option<u64>,
    /// Observed RB height when this transaction first became active in some node's mempool.
    pub mempool_entry_rb_index: Option<u64>,
    pub value: u64,
    pub urgency: UrgencyProfile,
    pub posted_fee: Option<u64>,
    pub tier_preference: Option<TierId>,
    /// Slot when the assigned RB tier version became active for submissions.
    pub tier_version_created_slot: Option<u64>,
    /// Settlement delay assigned for RB inclusion (when pricing is tiered).
    pub tier_delay_slots: Option<u64>,
    /// Tier price-per-byte quote used when the RB assignment was admitted.
    pub tier_price_per_byte_at_assignment: Option<u64>,
    /// EB-specific tier preference (when pricing uses separate per-block-kind pools).
    pub eb_tier_preference: Option<TierId>,
    /// Slot when the assigned EB tier version became active for submissions.
    pub eb_tier_version_created_slot: Option<u64>,
    /// EB-specific posted fee (when pricing uses separate per-block-kind pools).
    pub eb_posted_fee: Option<u64>,
    /// EB-specific settlement delay (when pricing uses separate per-block-kind pools).
    pub eb_tier_delay_slots: Option<u64>,
    /// Tier price-per-byte quote used when the EB assignment was admitted.
    pub eb_tier_price_per_byte_at_assignment: Option<u64>,
    /// Assigned inclusion lane for tiered pricing admission.
    pub assigned_block_kind: Option<BlockKind>,
    pub input_id: u64,
    pub overcollateralization_factor: u64,
    /// Index of the value-urgency component that was sampled for this transaction.
    /// `None` when the actor has no value_urgency_components (legacy path).
    pub urgency_component_index: Option<u16>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InputBlockId<Node = NodeId> {
    pub slot: u64,
    pub pipeline: u64,
    pub producer: Node,
    /// Need this field to distinguish IBs from the same slot+producer.
    /// The real implementation can use the VRF proof for that.
    pub index: u64,
}

impl<Node: Display> Display for InputBlockId<Node> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}-{}-{}",
            self.slot, self.producer, self.index
        ))
    }
}

impl<Node: Display> Serialize for InputBlockId<Node> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct InputBlockHeader {
    pub id: InputBlockId,
    pub vrf: u64,
    pub shard: u64,
    pub timestamp: Timestamp,
    pub bytes: u64,
}

#[derive(Debug)]
pub struct InputBlock {
    pub header: InputBlockHeader,
    pub tx_payload_bytes: u64,
    pub transactions: Vec<Arc<Transaction>>,
    pub rb_ref: Option<BlockId>,
}
impl InputBlock {
    pub fn bytes(&self) -> u64 {
        self.header.bytes + self.tx_payload_bytes
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EndorserBlockId<Node = NodeId> {
    pub slot: u64,
    pub pipeline: u64,
    pub producer: Node,
}
impl<Node: Display> Display for EndorserBlockId<Node> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}-{}", self.slot, self.producer))
    }
}
impl<Node: Display> Serialize for EndorserBlockId<Node> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug)]
pub struct EndorserBlock {
    pub slot: u64,
    pub pipeline: u64,
    pub producer: NodeId,
    pub shard: u64,
    pub bytes: u64,
    pub ibs: Vec<InputBlockId>,
    pub ebs: Vec<EndorserBlockId>,
}
impl EndorserBlock {
    pub fn id(&self) -> EndorserBlockId {
        EndorserBlockId {
            slot: self.slot,
            pipeline: self.pipeline,
            producer: self.producer,
        }
    }
}

#[derive(Debug)]
pub struct StracciatellaEndorserBlock {
    pub slot: u64,
    pub pipeline: u64,
    pub producer: NodeId,
    pub shard: u64,
    pub bytes: u64,
    pub txs: Vec<Arc<Transaction>>,
    pub ebs: Vec<EndorserBlockId>,
}
impl StracciatellaEndorserBlock {
    pub fn id(&self) -> EndorserBlockId {
        EndorserBlockId {
            slot: self.slot,
            pipeline: self.pipeline,
            producer: self.producer,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearEndorserBlock {
    pub slot: u64,
    pub producer: NodeId,
    pub bytes: u64,
    pub txs: Vec<Arc<Transaction>>,
}
impl LinearEndorserBlock {
    pub fn id(&self) -> EndorserBlockId {
        EndorserBlockId {
            slot: self.slot,
            pipeline: 0,
            producer: self.producer,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VoteBundleId<Node = NodeId> {
    pub slot: u64,
    pub pipeline: u64,
    pub producer: Node,
}
impl<Node: Display> Display for VoteBundleId<Node> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}-{}", self.slot, self.producer))
    }
}
impl<Node: Display> Serialize for VoteBundleId<Node> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VoteBundle {
    pub id: VoteBundleId,
    pub bytes: u64,
    pub ebs: BTreeMap<EndorserBlockId, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub enum NoVoteReason {
    InvalidSlot,
    ExtraIB,
    MissingIB,
    MissingEB,
    LateIBHeader,
    EquivocatedIB,
    ExtraTX,
    MissingTX,
    UncertifiedEBReference,
    LateRBHeader,
    LateEB,
    WrongEB,
}

#[derive(Debug, Clone, Serialize)]
pub enum TransactionLostReason {
    IBExpired,
    EBExpired,
}

#[derive(Debug, Copy, Clone, Serialize, PartialEq, Eq)]
pub enum TransactionRejectReason {
    TooExpensive,
    TierBacklogFull,
    InvalidQuotedAssignment,
    QuotedHistoryUnavailable,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Endorsement<Node: Display = NodeId> {
    pub eb: EndorserBlockId<Node>,
    pub size_bytes: u64,
    pub votes: BTreeMap<Node, usize>,
}
