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

/// Per-lane `quote_per_byte` carried as a header field on every RB
/// under the chain-derived controller pattern (spike 007). Single-lane
/// mechanisms set `standard == priority` so callers reading via
/// [`PerLaneQuote::get`] always see the right value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PerLaneQuote {
    pub standard: u64,
    pub priority: u64,
}

impl PerLaneQuote {
    /// Lane-keyed lookup.
    pub fn get(&self, lane: crate::tx_pricing::Lane) -> u64 {
        match lane {
            crate::tx_pricing::Lane::Standard => self.standard,
            crate::tx_pricing::Lane::Priority => self.priority,
        }
    }

    /// Both lanes equal to the same flat quote. Used by `BaselinePricing`
    /// and `Eip1559Pricing` (which is single-lane: priority and standard
    /// share one controller).
    pub fn flat(quote: u64) -> Self {
        Self {
            standard: quote,
            priority: quote,
        }
    }
}

/// Capacity-weighted window aggregate carried alongside `derived_quote`
/// on every RB. Stored as `u128` sums so the EIP-1559 controller step
/// can incrementally update without re-walking the canonical chain.
///
/// Per-lane (standard, priority) split mirrors the two-lane controller
/// inputs; single-lane mechanisms only populate the standard fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowAggregate {
    pub standard_sum_bytes: u128,
    pub standard_sum_capacity: u128,
    pub priority_sum_bytes: u128,
    pub priority_sum_capacity: u128,
    /// Number of canonical blocks contributing to this aggregate
    /// (capped at the configured `window_length`). Used to detect the
    /// cold-start regime (`blocks_in_window < window_length`) and to
    /// decide whether eviction must fire on the next step.
    pub blocks_in_window: u32,
}

impl WindowAggregate {
    pub const ZERO: Self = Self {
        standard_sum_bytes: 0,
        standard_sum_capacity: 0,
        priority_sum_bytes: 0,
        priority_sum_capacity: 0,
        blocks_in_window: 0,
    };

    /// Aggregate as a rational `(numerator, denominator)` for one lane,
    /// matching the legacy `CapacityWeightedWindow::aggregate_util`
    /// convention: `(0, 1)` when no signal exists.
    pub fn aggregate_util(&self, lane: crate::tx_pricing::Lane) -> (u128, u128) {
        let (n, d) = match lane {
            crate::tx_pricing::Lane::Standard => {
                (self.standard_sum_bytes, self.standard_sum_capacity)
            }
            crate::tx_pricing::Lane::Priority => {
                (self.priority_sum_bytes, self.priority_sum_capacity)
            }
        };
        if d == 0 { (0, 1) } else { (n, d) }
    }
}

/// Per-block sample payload, recorded at production for each canonical
/// block so descendants can fold them into their `WindowAggregate`. Held
/// in a node-local cache (pruned at `2 × window_length` behind the chain
/// tip) so `ChainView::samples_in_block` is O(1).
#[derive(Clone, Debug)]
pub struct CanonicalBlockSamples {
    pub block_id: BlockId,
    pub samples: Vec<crate::tx_pricing::PricedBlockSample>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinearRankingBlock {
    pub header: LinearRankingBlockHeader,
    pub transactions: Vec<Arc<Transaction>>,
    pub endorsement: Option<Endorsement>,
    /// Chain-derived per-lane quote, computed at block production as a
    /// pure function of the parent's `derived_quote` + samples emitted
    /// by canonical predecessors within the window. Carried on the
    /// block (header equivalent) so every node reads the same value
    /// without consulting node-local controller state. Closes WR-1 by
    /// construction (spike 007).
    pub derived_quote: PerLaneQuote,
    /// Incremental capacity-weighted window state used to compute
    /// `derived_quote`. Stored on the block so descendants can step
    /// the controller in O(1) without re-walking the chain. EBs do
    /// NOT carry this — they inherit from their parent RB via chain
    /// lookup (spike 007 §"Edge cases" item 4).
    pub window_aggregate: WindowAggregate,
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

/// EBs deliberately do **not** carry `derived_quote`. They inherit it
/// from their parent RB via chain-tip lookup. Adding a redundant field
/// would risk drift between `EB.parent_rb.derived_quote` and an
/// `EB.derived_quote` on slot-battle paths (the parent RB's quote is
/// always canonical; an EB-side mirror could lag). Spike 007 §"Edge
/// cases" item 4 is the authoritative reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearEndorserBlock {
    pub slot: u64,
    pub producer: NodeId,
    pub bytes: u64,
    pub txs: Vec<Arc<Transaction>>,
    /// Whether the producer activated the priority partition on this
    /// EB. M3 carries the producer's full two-trigger decision
    /// (saturation OR capacity-bound rejection per the spec) as a
    /// claim on the EB so the endorser's served-lane assignment
    /// matches by construction. The capacity-bound trigger is not
    /// re-derivable from the EB body alone (it needs the producer's
    /// mempool view), so the bit cannot be a derived property — it is
    /// an honest-producer claim. Future attacker models in M4/M5 may
    /// test producer dishonesty by setting this inconsistently with
    /// the EB's contents; see `docs/phase-2/m3-handoff.md`.
    pub partition_activated: bool,
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
    MempoolRejected,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Endorsement<Node: Display = NodeId> {
    pub eb: EndorserBlockId<Node>,
    pub size_bytes: u64,
    pub votes: BTreeMap<Node, usize>,
}
