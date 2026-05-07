use std::{collections::BTreeMap, fmt::Display, sync::Arc};

use crate::{clock::Timestamp, config::NodeId, tx_pricing::Lane};
use serde::Serialize;

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

#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: TransactionId,
    pub shard: u64,
    pub bytes: u64,
    pub input_id: u64,
    pub overcollateralization_factor: u64,
    /// Total maximum lovelace this transaction authorises the ledger to
    /// charge (the spec's `maxFee` from mechanism-design.md line 39).
    /// Stored as integer lovelace; never f64.
    pub max_fee_lovelace: u64,
    /// Lane the user paid into. Single-lane mechanisms always set
    /// `Lane::Standard`; two-lane mechanisms (M2+) populate both.
    pub posted_lane: Lane,
    /// Sample value field used by welfare metrics (M3+). Default 0 for
    /// pre-actor-model paths.
    pub value_lovelace: u64,
    /// Urgency factor (real number > 1 per the paper). Stored as f64
    /// because at M1 nothing reads it; at M3 the actor model converts it
    /// into a lane-choice decision via fixed-point/pinned-libm math
    /// (implementation-plan.md lines 165-167).
    pub urgency: f64,
    /// Index of the actor component that produced this tx, for per-class
    /// welfare metrics (M3+). Default 0 for pre-actor paths.
    pub urgency_component_index: u32,
}

// Manual PartialEq/Eq: f64 is not Eq because of NaN, so we compare
// `urgency` by bit pattern (deterministic and reflexive on every value
// including NaN). All other fields are integer/enum and compared
// directly.
impl PartialEq for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.shard == other.shard
            && self.bytes == other.bytes
            && self.input_id == other.input_id
            && self.overcollateralization_factor == other.overcollateralization_factor
            && self.max_fee_lovelace == other.max_fee_lovelace
            && self.posted_lane == other.posted_lane
            && self.value_lovelace == other.value_lovelace
            && self.urgency.to_bits() == other.urgency.to_bits()
            && self.urgency_component_index == other.urgency_component_index
    }
}
impl Eq for Transaction {}

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

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Endorsement<Node: Display = NodeId> {
    pub eb: EndorserBlockId<Node>,
    pub size_bytes: u64,
    pub votes: BTreeMap<Node, usize>,
}
