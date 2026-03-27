mod attackers;
pub use attackers::register_actors;
use rand_distr::Distribution;
use tokio::sync::mpsc;

use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
    time::Duration,
};

use indexmap::IndexMap;

use rand::{Rng as _, seq::SliceRandom as _};
use rand_chacha::ChaChaRng;

use crate::{
    clock::{Clock, Timestamp},
    config::{
        CpuTimeConfig, EBPropagationCriteria, LeiosVariant, MempoolSamplingStrategy,
        NodeBehaviours, NodeConfiguration, NodeId, RelayStrategy, SimConfiguration, TierDelayUnit,
        TierSelectionPathLatencyConfig, TransactionConfig,
    },
    events::{EventTracker, TierInfo},
    model::{
        ActorId, BlockId, Endorsement, EndorserBlockId, LinearEndorserBlock as EndorserBlock,
        LinearRankingBlock as RankingBlock, LinearRankingBlockHeader as RankingBlockHeader,
        NoVoteReason, TierId, Transaction, TransactionId, TransactionRejectReason, VoteBundle,
        VoteBundleId,
    },
    sim::{
        MiniProtocol, NodeImpl, SimCpuTask, SimMessage,
        linear_leios::attackers::{EBWithholdingEvent, EBWithholdingSender},
        lottery::{LotteryConfig, LotteryKind, MockLotteryResults, vrf_probabilities},
    },
    tx_pricing::{
        BlockKind, OverflowRetryCurveMetric, OverflowRetryPolicy, OverflowRetrySource,
        PricingMechanism, Tier, TierBlockSelectionPolicy, TierCadenceUpdate,
        TierSelectionDelayModel, select_best_lane_tier_for_tx, select_tier_for_tx,
    },
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum PricingUpdateKey {
    RankingBlock(BlockId),
    EndorserBlock(EndorserBlockId),
    EndorserOpportunity(BlockId),
}

#[derive(Clone)]
pub(super) struct GlobalPricingCoordinator {
    state: Arc<Mutex<GlobalPricingState>>,
    tier_selection_delay_model: TierSelectionDelayModel,
}

struct GlobalPricingState {
    pricing: PricingMechanism,
    applied_updates: HashSet<PricingUpdateKey>,
}

struct TieredPricingUpdate {
    before: Vec<Tier>,
    after: Vec<Tier>,
    utilisations: Vec<f64>,
    cadence: TierCadenceUpdate,
}

impl GlobalPricingCoordinator {
    pub(super) fn new(
        pricing_config: &crate::tx_pricing::PricingMechanismConfig,
        seed: u64,
        tier_selection_path_latency: TierSelectionPathLatencyConfig,
    ) -> Self {
        let pricing = PricingMechanism::from_config(pricing_config, seed);
        let tier_selection_delay_model = match pricing_config {
            crate::tx_pricing::PricingMechanismConfig::TieredPricing { tiered_config }
                if tiered_config.block_selection_policy
                    == TierBlockSelectionPolicy::NaiveRbEbTwoTier =>
            {
                TierSelectionDelayModel::NaiveRbEbTwoTierPath {
                    rb_path_latency: tier_selection_path_latency.rb_delay_units,
                    eb_path_latency: tier_selection_path_latency.eb_delay_units,
                }
            }
            crate::tx_pricing::PricingMechanismConfig::TieredPricing { tiered_config }
                if tiered_config.block_selection_policy
                    == TierBlockSelectionPolicy::RbTier0Reserved
                    || tiered_config.block_selection_policy
                        == TierBlockSelectionPolicy::ContinuousRbEb =>
            {
                TierSelectionDelayModel::LanePathPlusTierDelay {
                    rb_path_latency: tier_selection_path_latency.rb_delay_units,
                    eb_path_latency: tier_selection_path_latency.eb_delay_units,
                }
            }
            _ => TierSelectionDelayModel::TierDelay,
        };
        Self {
            state: Arc::new(Mutex::new(GlobalPricingState {
                pricing,
                applied_updates: HashSet::new(),
            })),
            tier_selection_delay_model,
        }
    }

    fn tier_selection_delay_model(&self) -> TierSelectionDelayModel {
        self.tier_selection_delay_model
    }

    fn snapshot_for_block_kind(&self, block_kind: BlockKind) -> crate::tx_pricing::PricingSnapshot {
        let state = self
            .state
            .lock()
            .expect("global pricing coordinator mutex poisoned");
        state.pricing.snapshot_for_block_kind(block_kind)
    }

    fn has_separate_eb_pool(&self) -> bool {
        let state = self
            .state
            .lock()
            .expect("global pricing coordinator mutex poisoned");
        state.pricing.has_separate_eb_pool()
    }

    fn reject_on_pending_tier_overflow(&self) -> bool {
        let state = self
            .state
            .lock()
            .expect("global pricing coordinator mutex poisoned");
        state.pricing.reject_on_pending_tier_overflow()
    }

    fn overflow_retry_policy(&self) -> Option<OverflowRetryPolicy> {
        let state = self
            .state
            .lock()
            .expect("global pricing coordinator mutex poisoned");
        state.pricing.overflow_retry_policy()
    }

    fn include_overflow_aggregate_in_pricing_updates(&self) -> bool {
        let state = self
            .state
            .lock()
            .expect("global pricing coordinator mutex poisoned");
        state
            .pricing
            .include_overflow_aggregate_in_pricing_updates()
    }

    fn include_overflow_aggregate_in_tier_updates(&self) -> bool {
        let state = self
            .state
            .lock()
            .expect("global pricing coordinator mutex poisoned");
        state.pricing.include_overflow_aggregate_in_tier_updates()
    }

    fn is_tiered(&self) -> bool {
        let state = self
            .state
            .lock()
            .expect("global pricing coordinator mutex poisoned");
        state.pricing.is_tiered()
    }

    fn uses_lane_partitioned_tiers(&self) -> bool {
        let state = self
            .state
            .lock()
            .expect("global pricing coordinator mutex poisoned");
        state.pricing.uses_lane_partitioned_tiers()
    }

    fn uses_shared_single_pool_tiers(&self) -> bool {
        let state = self
            .state
            .lock()
            .expect("global pricing coordinator mutex poisoned");
        state.pricing.block_selection_policy() == Some(TierBlockSelectionPolicy::Shared)
            && !state.pricing.has_separate_eb_pool()
    }

    fn effective_tier_capacity_for_block_kind(
        &self,
        block_kind: BlockKind,
        tier_id: TierId,
        block_capacity: u64,
    ) -> Option<u64> {
        let state = self
            .state
            .lock()
            .expect("global pricing coordinator mutex poisoned");
        state
            .pricing
            .effective_tier_capacity_for_block_kind(block_kind, tier_id, block_capacity)
    }

    fn verify_preassigned_transaction(
        &self,
        tx: &Transaction,
    ) -> Result<(), TransactionRejectReason> {
        let state = self
            .state
            .lock()
            .expect("global pricing coordinator mutex poisoned");
        state.pricing.verify_preassigned_transaction(tx)
    }

    fn select_transactions_for_block(
        &self,
        txs: &[Arc<Transaction>],
        slot: u64,
        block_capacity: u64,
        block_kind: BlockKind,
    ) -> Vec<Arc<Transaction>> {
        let state = self
            .state
            .lock()
            .expect("global pricing coordinator mutex poisoned");
        state
            .pricing
            .select_transactions_for_block(txs, slot, block_capacity, block_kind)
    }

    fn update_after_block(
        &self,
        update_key: PricingUpdateKey,
        txs: &[Arc<Transaction>],
        tier_update_signal_txs: Option<&[Arc<Transaction>]>,
        overflow_pricing_signal_txs: Option<&[Arc<Transaction>]>,
        block_capacity: u64,
        block_kind: BlockKind,
        slot: u64,
    ) -> Option<TieredPricingUpdate> {
        let mut state = self
            .state
            .lock()
            .expect("global pricing coordinator mutex poisoned");
        if !state.applied_updates.insert(update_key) {
            return None;
        }

        let before = state.pricing.cloned_tiers_for_block_kind(block_kind);
        let cadence = state.pricing.update_after_block_with_signals(
            txs,
            tier_update_signal_txs,
            overflow_pricing_signal_txs,
            block_capacity,
            block_kind,
            slot,
        );
        let after = state.pricing.cloned_tiers_for_block_kind(block_kind);
        let utilisations = state.pricing.tier_utilisations_for_block_kind(block_kind);

        match (before, after) {
            (Some(before), Some(after)) => Some(TieredPricingUpdate {
                before,
                after,
                utilisations,
                cadence,
            }),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    // TX propagation
    AnnounceTx(TransactionId),
    RequestTx(TransactionId),
    Tx(Arc<Transaction>),

    // RB header propagation
    AnnounceRBHeader(BlockId),
    RequestRBHeader(BlockId),
    RBHeader(
        RankingBlockHeader,
        bool, /* has_body */
        bool, /* has_eb */
    ),

    // RB body propagation
    AnnounceRB(BlockId),
    RequestRB(BlockId),
    RB(Arc<RankingBlock>),

    // EB propagation
    AnnounceEB(EndorserBlockId),
    RequestEB(EndorserBlockId),
    EB(Arc<EndorserBlock>),

    // Vote propagation
    AnnounceVotes(VoteBundleId),
    RequestVotes(VoteBundleId),
    Votes(Arc<VoteBundle>),
}

impl SimMessage for Message {
    fn protocol(&self) -> MiniProtocol {
        match self {
            Self::AnnounceTx(_) => MiniProtocol::Tx,
            Self::RequestTx(_) => MiniProtocol::Tx,
            Self::Tx(_) => MiniProtocol::Tx,

            Self::AnnounceRBHeader(_) => MiniProtocol::Block,
            Self::RequestRBHeader(_) => MiniProtocol::Block,
            Self::RBHeader(_, _, _) => MiniProtocol::Block,

            Self::AnnounceRB(_) => MiniProtocol::Block,
            Self::RequestRB(_) => MiniProtocol::Block,
            Self::RB(_) => MiniProtocol::Block,

            Self::AnnounceEB(_) => MiniProtocol::EB,
            Self::RequestEB(_) => MiniProtocol::EB,
            Self::EB(_) => MiniProtocol::EB,

            Self::AnnounceVotes(_) => MiniProtocol::Vote,
            Self::RequestVotes(_) => MiniProtocol::Vote,
            Self::Votes(_) => MiniProtocol::Vote,
        }
    }

    fn bytes_size(&self) -> u64 {
        match self {
            Self::AnnounceTx(_) => 8,
            Self::RequestTx(_) => 8,
            Self::Tx(tx) => tx.bytes,

            Self::AnnounceRBHeader(_) => 8,
            Self::RequestRBHeader(_) => 8,
            Self::RBHeader(header, _, _) => header.bytes,

            Self::AnnounceRB(_) => 8,
            Self::RequestRB(_) => 8,
            Self::RB(rb) => rb.bytes(),

            Self::AnnounceEB(_) => 8,
            Self::RequestEB(_) => 8,
            Self::EB(eb) => eb.bytes,

            Self::AnnounceVotes(_) => 8,
            Self::RequestVotes(_) => 8,
            Self::Votes(v) => v.bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpuTask {
    /// A transaction has been received and validated, and is ready to propagate
    TransactionValidated(NodeId, Arc<Transaction>),
    /// A ranking block has been generated and is ready to propagate
    RBBlockGenerated(
        RankingBlock,
        Option<(EndorserBlock, Vec<Arc<Transaction>>)>,
        bool,
    ),
    /// An RB header has been received and validated, and ready to propagate
    RBHeaderValidated(NodeId, RankingBlockHeader, bool, bool),
    /// A ranking block has been received and validated, and is ready to propagate
    RBBlockValidated(Arc<RankingBlock>),
    /// An endorser block has been received, and its header has been validated. It is ready to propagate.
    EBHeaderValidated(NodeId, Arc<EndorserBlock>),
    /// An endorser block has been received and validated, and is ready to propagate
    EBBlockValidated(Arc<EndorserBlock>, Timestamp),
    /// A bundle of votes has been generated and is ready to propagate
    VTBundleGenerated(VoteBundle, Arc<EndorserBlock>),
    /// A bundle of votes has been received and validated, and is ready to propagate
    VTBundleValidated(NodeId, Arc<VoteBundle>),
}

impl SimCpuTask for CpuTask {
    fn name(&self) -> String {
        match self {
            Self::TransactionValidated(_, _) => "ValTX",
            Self::RBBlockGenerated(_, _, _) => "GenRB",
            Self::RBHeaderValidated(_, _, _, _) => "ValRH",
            Self::RBBlockValidated(_) => "ValRB",
            Self::EBHeaderValidated(_, _) => "ValEH",
            Self::EBBlockValidated(_, _) => "ValEB",
            Self::VTBundleGenerated(_, _) => "GenVote",
            Self::VTBundleValidated(_, _) => "ValVote",
        }
        .to_string()
    }

    fn extra(&self) -> String {
        match self {
            Self::TransactionValidated(_, _) => "".to_string(),
            Self::RBBlockGenerated(_, _, _) => "".to_string(),
            Self::RBHeaderValidated(_, _, _, _) => "".to_string(),
            Self::RBBlockValidated(_) => "".to_string(),
            Self::EBHeaderValidated(_, _) => "".to_string(),
            Self::EBBlockValidated(_, _) => "".to_string(),
            Self::VTBundleGenerated(_, _) => "".to_string(),
            Self::VTBundleValidated(_, _) => "".to_string(),
        }
    }

    fn times(&self, config: &CpuTimeConfig) -> Vec<Duration> {
        match self {
            Self::TransactionValidated(_, tx) => vec![
                config.tx_validation_constant + config.tx_validation_per_byte * tx.bytes as u32,
            ],
            Self::RBBlockGenerated(rb, eb, _) => {
                let mut rb_time = config.rb_generation + config.rb_body_validation_constant;
                let rb_bytes: u64 = rb.transactions.iter().map(|tx| tx.bytes).sum();
                rb_time += config.rb_validation_per_byte * (rb_bytes as u32);
                if let Some(endorsement) = &rb.endorsement {
                    let nodes = endorsement.votes.len();
                    rb_time += config.cert_generation_constant
                        + (config.cert_generation_per_node * nodes as u32);
                }
                let mut times = vec![rb_time];

                if let Some((eb, _)) = eb {
                    let mut eb_time = config.eb_generation + config.eb_body_validation_constant;
                    let eb_bytes: u64 = eb.txs.iter().map(|tx| tx.bytes).sum();
                    eb_time += config.eb_body_validation_per_byte * (eb_bytes as u32);
                    times.push(eb_time);
                }
                times
            }
            Self::RBHeaderValidated(_, _, _, _) => vec![config.rb_head_validation],
            Self::RBBlockValidated(rb) => {
                let mut time = config.rb_body_validation_constant;
                let bytes: u64 = rb.transactions.iter().map(|tx| tx.bytes).sum();
                time += config.rb_validation_per_byte * (bytes as u32);
                if let Some(endorsement) = &rb.endorsement {
                    let nodes = endorsement.votes.len();
                    time += config.cert_validation_constant
                        + (config.cert_validation_per_node * nodes as u32);
                }
                vec![time]
            }
            Self::EBHeaderValidated(_, _) => vec![config.eb_header_validation],
            Self::EBBlockValidated(eb, _) => {
                let mut time = config.eb_body_validation_constant;
                let bytes: u64 = eb.txs.iter().map(|tx| tx.bytes).sum();
                time += config.eb_body_validation_per_byte * (bytes as u32);
                vec![time]
            }
            Self::VTBundleGenerated(_, eb) => vec![
                config.vote_generation_constant
                    + (config.vote_generation_per_tx * eb.txs.len() as u32),
            ],
            Self::VTBundleValidated(_, _) => vec![config.vote_validation],
        }
    }
}

pub enum TimedEvent {
    TryVote(Arc<EndorserBlock>, Timestamp),
    RetryOverflowTx(Arc<Transaction>),
}

enum TransactionView {
    Pending,
    Received(Arc<Transaction>),
}

enum RankingBlockView {
    HeaderPending,
    Pending {
        header: RankingBlockHeader,
        header_seen: Timestamp,
    },
    Requested {
        header: RankingBlockHeader,
        header_seen: Timestamp,
    },
    Received {
        rb: Arc<RankingBlock>,
        header_seen: Timestamp,
        height: u64,
    },
}
impl RankingBlockView {
    fn header(&self) -> Option<&RankingBlockHeader> {
        match self {
            Self::HeaderPending => None,
            Self::Pending { header, .. } => Some(header),
            Self::Requested { header, .. } => Some(header),
            Self::Received { rb, .. } => Some(&rb.header),
        }
    }
    fn header_seen(&self) -> Option<Timestamp> {
        match self {
            Self::HeaderPending => None,
            Self::Pending { header_seen, .. } => Some(*header_seen),
            Self::Requested { header_seen, .. } => Some(*header_seen),
            Self::Received { header_seen, .. } => Some(*header_seen),
        }
    }

    fn height(&self) -> Option<u64> {
        match self {
            Self::Received { height, .. } => Some(*height),
            _ => None,
        }
    }
}

#[derive(Default)]
struct NodePraosState {
    peer_heads: BTreeMap<NodeId, u64>,
    blocks: BTreeMap<BlockId, RankingBlockView>,
    block_ids_by_slot: BTreeMap<u64, BlockId>,
}

#[derive(Debug)]
enum EndorserBlockView {
    Pending,
    Requested,
    Received {
        eb: Arc<EndorserBlock>,
        seen: Timestamp,
        all_txs_seen: bool,
        validated: bool,
    },
}

enum VoteBundleView {
    Requested,
    Received { votes: Arc<VoteBundle> },
}

#[derive(Default)]
struct NodeLeiosState {
    ebs: HashMap<EndorserBlockId, EndorserBlockView>,
    ebs_by_rb: HashMap<BlockId, EndorserBlockId>,
    eb_peer_announcements: HashMap<EndorserBlockId, Vec<NodeId>>,
    votes: HashMap<VoteBundleId, VoteBundleView>,
    votes_by_eb: HashMap<EndorserBlockId, BTreeMap<NodeId, usize>>,
    certified_ebs: HashSet<EndorserBlockId>,
    incomplete_onchain_ebs: HashSet<EndorserBlockId>,
    missing_txs: HashMap<TransactionId, Vec<EndorserBlockId>>,
}

#[derive(Clone, Default)]
struct LedgerState {
    spent_inputs: HashSet<u64>,
    seen_blocks: HashSet<BlockId>,
}

pub struct LinearLeiosNode {
    id: NodeId,
    sim_config: Arc<SimConfiguration>,
    queued: EventResult,
    tracker: EventTracker,
    rng: ChaChaRng,
    clock: Clock,
    lottery: LotteryConfig,
    consumers: Vec<NodeId>,
    txs: HashMap<TransactionId, TransactionView>,
    local_actor_submission_ids: HashSet<TransactionId>,
    overflow_retry_attempts: HashMap<TransactionId, u32>,
    overflow_rejected_bytes_rb: BTreeMap<TierId, u64>,
    overflow_rejected_bytes_eb: BTreeMap<TierId, u64>,
    overflow_rejected_ids_rb: BTreeMap<TierId, HashSet<TransactionId>>,
    overflow_rejected_ids_eb: BTreeMap<TierId, HashSet<TransactionId>>,
    next_overflow_aggregate_tx_id: u64,
    actor_overflow_retry_policy_overrides: HashMap<ActorId, OverflowRetryPolicy>,
    mempool: Mempool,
    pricing: Option<GlobalPricingCoordinator>,
    ledger_states: BTreeMap<BlockId, Arc<LedgerState>>,
    praos: NodePraosState,
    leios: NodeLeiosState,
    behaviours: NodeBehaviours,

    eb_withholding_sender: Option<EBWithholdingSender>,
    eb_withholding_event_source: Option<mpsc::UnboundedReceiver<EBWithholdingEvent>>,
}

type EventResult = super::EventResult<LinearLeiosNode>;

impl NodeImpl for LinearLeiosNode {
    type Message = Message;
    type Task = CpuTask;
    type TimedEvent = TimedEvent;
    type CustomEvent = EBWithholdingEvent;

    fn new(
        config: &NodeConfiguration,
        sim_config: Arc<SimConfiguration>,
        tracker: EventTracker,
        rng: ChaChaRng,
        clock: Clock,
    ) -> Self {
        let lottery = LotteryConfig::Random {
            stake: config.stake,
            total_stake: sim_config.total_stake,
        };
        let mempool_max_size_bytes = sim_config.mempool_size_bytes;
        let enforce_tier_delay = sim_config.enforce_tier_delay();
        let tier_delay_unit = sim_config.tier_delay_unit();
        let actor_overflow_retry_policy_overrides = sim_config
            .actors()
            .map(|actors| {
                actors
                    .iter()
                    .enumerate()
                    .filter_map(|(index, actor)| {
                        actor
                            .overflow_retry_policy_override
                            .clone()
                            .map(|policy| (ActorId::new(index as u64), policy))
                    })
                    .collect()
            })
            .unwrap_or_default();

        Self {
            id: config.id,
            sim_config,
            queued: EventResult::default(),
            tracker,
            rng,
            clock,
            lottery,
            consumers: config.consumers.clone(),
            txs: HashMap::new(),
            local_actor_submission_ids: HashSet::new(),
            overflow_retry_attempts: HashMap::new(),
            overflow_rejected_bytes_rb: BTreeMap::new(),
            overflow_rejected_bytes_eb: BTreeMap::new(),
            overflow_rejected_ids_rb: BTreeMap::new(),
            overflow_rejected_ids_eb: BTreeMap::new(),
            next_overflow_aggregate_tx_id: 0,
            actor_overflow_retry_policy_overrides,
            mempool: Mempool::with_delay_mode(
                mempool_max_size_bytes,
                enforce_tier_delay,
                tier_delay_unit,
            ),
            pricing: None,
            ledger_states: BTreeMap::new(),
            praos: NodePraosState::default(),
            leios: NodeLeiosState::default(),
            behaviours: config.behaviours.clone(),
            eb_withholding_sender: None,
            eb_withholding_event_source: None,
        }
    }

    fn custom_event_source(&mut self) -> Option<mpsc::UnboundedReceiver<Self::CustomEvent>> {
        self.eb_withholding_event_source.take()
    }

    fn handle_new_slot(&mut self, slot: u64) -> EventResult {
        self.try_generate_rb(slot);

        std::mem::take(&mut self.queued)
    }

    fn handle_new_tx(&mut self, tx: Arc<Transaction>) -> EventResult {
        self.generate_tx(tx);
        std::mem::take(&mut self.queued)
    }

    fn handle_message(&mut self, from: NodeId, msg: Self::Message) -> EventResult {
        match msg {
            // TX propagation
            Message::AnnounceTx(id) => self.receive_announce_tx(from, id),
            Message::RequestTx(id) => self.receive_request_tx(from, id),
            Message::Tx(tx) => self.receive_tx(from, tx),

            // RB header propagation
            Message::AnnounceRBHeader(id) => self.receive_announce_rb_header(from, id),
            Message::RequestRBHeader(id) => self.receive_request_rb_header(from, id),
            Message::RBHeader(header, has_body, has_eb) => {
                self.receive_rb_header(from, header, has_body, has_eb)
            }

            // RB body propagation
            Message::AnnounceRB(id) => self.receive_announce_rb(from, id),
            Message::RequestRB(id) => self.receive_request_rb(from, id),
            Message::RB(rb) => self.receive_rb(from, rb),

            // EB body propagation
            Message::AnnounceEB(id) => self.receive_announce_eb(from, id),
            Message::RequestEB(id) => self.receive_request_eb(from, id),
            Message::EB(rb) => self.receive_eb(from, rb),

            // Vote propagation
            Message::AnnounceVotes(id) => self.receive_announce_votes(from, id),
            Message::RequestVotes(id) => self.receive_request_votes(from, id),
            Message::Votes(votes) => self.receive_votes(from, votes),
        }
        std::mem::take(&mut self.queued)
    }

    fn handle_cpu_task(&mut self, task: Self::Task) -> EventResult {
        match task {
            CpuTask::TransactionValidated(from, tx) => self.propagate_tx(from, tx),
            CpuTask::RBBlockGenerated(rb, eb, eb_opportunity_available) => {
                self.finish_generating_rb(rb, eb, eb_opportunity_available)
            }
            CpuTask::RBHeaderValidated(from, header, has_body, has_eb) => {
                self.finish_validating_rb_header(from, header, has_body, has_eb)
            }
            CpuTask::RBBlockValidated(rb) => self.finish_validating_rb(rb),
            CpuTask::EBHeaderValidated(from, eb) => self.finish_validating_eb_header(from, eb),
            CpuTask::EBBlockValidated(eb, seen) => self.finish_validating_eb(eb, seen),
            CpuTask::VTBundleGenerated(votes, _) => self.finish_generating_vote_bundle(votes),
            CpuTask::VTBundleValidated(from, votes) => {
                self.finish_validating_vote_bundle(from, votes)
            }
        }
        std::mem::take(&mut self.queued)
    }

    fn handle_timed_event(&mut self, event: Self::TimedEvent) -> EventResult {
        match event {
            TimedEvent::TryVote(eb, seen) => self.vote_for_endorser_block(&eb, seen),
            TimedEvent::RetryOverflowTx(tx) => self.propagate_tx(self.id, tx),
        }
        std::mem::take(&mut self.queued)
    }

    fn handle_custom_event(&mut self, event: Self::CustomEvent) -> EventResult {
        match event {
            EBWithholdingEvent::NewEB(eb, withheld_txs) => {
                self.receive_withheld_eb(eb, withheld_txs)
            }
            EBWithholdingEvent::DisseminateEB(eb, withheld_txs) => {
                self.disseminate_withheld_eb(eb, withheld_txs)
            }
        }
        std::mem::take(&mut self.queued)
    }
}

// Transaction propagation
impl LinearLeiosNode {
    fn receive_announce_tx(&mut self, from: NodeId, id: TransactionId) {
        if self.txs.get(&id).is_none_or(|t| {
            self.sim_config.relay_strategy == RelayStrategy::RequestFromAll
                && matches!(t, TransactionView::Pending)
        }) {
            self.txs.insert(id, TransactionView::Pending);
            self.queued.send_to(from, Message::RequestTx(id));
        }
    }

    fn receive_request_tx(&mut self, from: NodeId, id: TransactionId) {
        if let Some(TransactionView::Received(tx)) = self.txs.get(&id) {
            self.tracker.track_transaction_sent(tx, self.id, from);
            self.queued.send_to(from, Message::Tx(tx.clone()));
        }
    }

    fn receive_tx(&mut self, from: NodeId, tx: Arc<Transaction>) {
        self.tracker
            .track_transaction_received(tx.id, from, self.id);
        self.queued
            .schedule_cpu_task(CpuTask::TransactionValidated(from, tx));
    }

    fn generate_tx(&mut self, tx: Arc<Transaction>) {
        self.local_actor_submission_ids.insert(tx.id);
        self.tracker.track_transaction_generated(&tx, self.id);
        self.propagate_tx(self.id, tx);
    }

    fn propagate_tx(&mut self, from: NodeId, tx: Arc<Transaction>) {
        let id = tx.id;
        if self
            .txs
            .get(&id)
            .is_some_and(|tx| matches!(tx, TransactionView::Received(_)))
        {
            return;
        }

        match self.try_add_tx_to_mempool(tx.clone()) {
            ActiveMempoolAdmission::Active(tx) => {
                self.overflow_retry_attempts.remove(&id);
                self.txs.insert(id, TransactionView::Received(tx.clone()));
                self.acknowledge_tx(&tx);
                for peer in &self.consumers {
                    if *peer == from {
                        continue;
                    }
                    self.queued.send_to(*peer, Message::AnnounceTx(id));
                }
            }
            ActiveMempoolAdmission::Queued(tx) | ActiveMempoolAdmission::Conflict(tx) => {
                self.overflow_retry_attempts.remove(&id);
                self.txs.insert(id, TransactionView::Received(tx.clone()));
                let referenced_by_eb = self.acknowledge_tx(&tx);
                if referenced_by_eb {
                    for peer in &self.consumers {
                        if *peer == from {
                            continue;
                        }
                        self.queued.send_to(*peer, Message::AnnounceTx(id));
                    }
                }
            }
            ActiveMempoolAdmission::Rejected {
                tx,
                reason,
                overflow_lane,
            } => {
                let scheduled_retry = reason == TransactionRejectReason::TierBacklogFull
                    && self.local_actor_submission_ids.contains(&id)
                    && overflow_lane.is_some_and(|lane| self.schedule_overflow_retry(&tx, lane));
                if reason == TransactionRejectReason::TierBacklogFull
                    && let Some(lane) = overflow_lane
                    && let Some((tier, pending_bytes, tier_capacity_bytes)) =
                        self.overflow_state_for_lane(&tx, lane)
                {
                    if self.pricing.as_ref().is_some_and(|pricing| {
                        pricing.include_overflow_aggregate_in_pricing_updates()
                    }) {
                        self.record_overflow_rejected_bytes(lane, tier, tx.id, tx.bytes);
                    }
                    self.tracker.track_transaction_overflow_rejected(
                        tx.id,
                        self.id,
                        lane,
                        tier,
                        pending_bytes,
                        tier_capacity_bytes,
                        scheduled_retry,
                    );
                }

                if scheduled_retry {
                    self.txs.remove(&id);
                } else {
                    self.txs.insert(id, TransactionView::Received(tx.clone()));
                    let referenced_by_eb = self.acknowledge_tx(&tx);
                    if referenced_by_eb {
                        for peer in &self.consumers {
                            if *peer == from {
                                continue;
                            }
                            self.queued.send_to(*peer, Message::AnnounceTx(id));
                        }
                    }
                    self.tracker
                        .track_transaction_rejected(tx.id, self.id, reason);
                    self.overflow_retry_attempts.remove(&id);
                }
            }
        }
    }

    fn has_tx(&self, tx_id: TransactionId) -> bool {
        matches!(self.txs.get(&tx_id), Some(TransactionView::Received(_)))
    }

    fn acknowledge_tx(&mut self, tx: &Transaction) -> bool {
        let Some(eb_ids) = self.leios.missing_txs.remove(&tx.id) else {
            return false;
        };
        for eb_id in eb_ids {
            self.try_validating_eb(eb_id);
        }
        true
    }

    fn current_slot(&self) -> u64 {
        (self.clock.now() - Timestamp::zero()).as_secs()
    }

    fn current_observed_rb_index(&self) -> u64 {
        self.latest_rb_height().unwrap_or(0)
    }

    fn rb_height_for_parent(&self, parent: Option<BlockId>) -> u64 {
        parent
            .and_then(|block_id| self.praos.blocks.get(&block_id))
            .and_then(RankingBlockView::height)
            .unwrap_or(0)
            .saturating_add(1)
    }

    fn set_mempool_entry_anchor(tx: &mut Transaction, slot: u64, rb_index: u64) {
        tx.mempool_entry_slot = Some(slot);
        tx.mempool_entry_rb_index = Some(rb_index);
    }

    fn has_any_mempool_entry_anchor(tx: &Transaction) -> bool {
        tx.mempool_entry_slot.is_some() || tx.mempool_entry_rb_index.is_some()
    }

    fn has_complete_mempool_entry_anchor(tx: &Transaction) -> bool {
        tx.mempool_entry_slot.is_some() && tx.mempool_entry_rb_index.is_some()
    }

    fn clear_tiered_assignment_and_maturity(tx: &Transaction) -> Arc<Transaction> {
        let mut cleared = tx.clone();
        cleared.posted_fee = None;
        cleared.tier_preference = None;
        cleared.tier_version_created_slot = None;
        cleared.tier_delay_slots = None;
        cleared.tier_price_per_byte_at_assignment = None;
        cleared.eb_tier_preference = None;
        cleared.eb_tier_version_created_slot = None;
        cleared.eb_posted_fee = None;
        cleared.eb_tier_delay_slots = None;
        cleared.eb_tier_price_per_byte_at_assignment = None;
        cleared.assigned_block_kind = None;
        cleared.mempool_entry_slot = None;
        cleared.mempool_entry_rb_index = None;
        Arc::new(cleared)
    }

    fn activate_tx_in_mempool(&mut self, tx: Arc<Transaction>) -> ActiveMempoolAdmission {
        let (tx, rejection_reason, overflow_lane) = self.apply_pricing_on_arrival(
            tx,
            self.current_slot(),
            self.current_observed_rb_index(),
        );
        if let Some(reason) = rejection_reason {
            return ActiveMempoolAdmission::Rejected {
                tx,
                reason,
                overflow_lane,
            };
        }
        ActiveMempoolAdmission::Active(tx)
    }

    fn apply_pricing_on_arrival(
        &mut self,
        tx: Arc<Transaction>,
        mempool_entry_slot: u64,
        mempool_entry_rb_index: u64,
    ) -> (
        Arc<Transaction>,
        Option<TransactionRejectReason>,
        Option<BlockKind>,
    ) {
        let Some(pricing) = self.pricing.clone() else {
            return (tx, None, None);
        };

        if pricing.is_tiered() && Self::has_any_tiered_assignment_payload(&tx) {
            if Self::has_any_mempool_entry_anchor(&tx)
                && !Self::has_complete_mempool_entry_anchor(&tx)
            {
                return (
                    tx,
                    Some(TransactionRejectReason::InvalidQuotedAssignment),
                    None,
                );
            }
            if self.sim_config.enforce_tier_delay() && !Self::has_complete_mempool_entry_anchor(&tx)
            {
                return (
                    tx,
                    Some(TransactionRejectReason::InvalidQuotedAssignment),
                    None,
                );
            }
            match pricing.verify_preassigned_transaction(&tx) {
                Ok(()) => {
                    if let Some(lane) = self.overfull_tier_lane(&pricing, &tx) {
                        return (
                            tx,
                            Some(TransactionRejectReason::TierBacklogFull),
                            Some(lane),
                        );
                    }
                    return (tx, None, None);
                }
                Err(reason) => {
                    return (tx, Some(reason), None);
                }
            }
        }

        if !pricing.is_tiered() && tx.posted_fee.is_some() && tx.tier_preference.is_some() {
            return (tx, None, None);
        }
        if pricing.is_tiered() && Self::has_any_mempool_entry_anchor(&tx) {
            return (
                tx,
                Some(TransactionRejectReason::InvalidQuotedAssignment),
                None,
            );
        }

        let delay_model = pricing.tier_selection_delay_model();
        let mut updated = (*tx).clone();

        // Assign RB tier preference.
        let rb_snapshot = pricing.snapshot_for_block_kind(BlockKind::RankingBlock);
        let rb_result = select_tier_for_tx(&updated, &rb_snapshot, delay_model);

        // Assign EB tier preference (separate pool if configured).
        let eb_result = if pricing.has_separate_eb_pool() {
            let eb_snapshot = pricing.snapshot_for_block_kind(BlockKind::EndorserBlock);
            select_tier_for_tx(&updated, &eb_snapshot, delay_model)
        } else {
            // When not using separate pools, EB uses the same tier as RB.
            rb_result
        };

        if pricing.uses_lane_partitioned_tiers() {
            let has_separate_eb_pool = pricing.has_separate_eb_pool();
            let eb_snapshot = pricing.snapshot_for_block_kind(BlockKind::EndorserBlock);
            let Some(selection) =
                select_best_lane_tier_for_tx(&updated, &rb_snapshot, &eb_snapshot, delay_model)
            else {
                return (tx, Some(TransactionRejectReason::TooExpensive), None);
            };

            updated.assigned_block_kind = Some(selection.block_kind);
            match selection.block_kind {
                BlockKind::RankingBlock => {
                    updated.tier_preference = Some(selection.tier_id);
                    updated.tier_version_created_slot = Some(selection.tier_version_created_slot);
                    updated.posted_fee = Some(selection.posted_fee);
                    updated.tier_delay_slots = Some(selection.tier_delay_slots);
                    updated.tier_price_per_byte_at_assignment =
                        Some(selection.tier_price_per_byte_at_assignment);
                    updated.eb_tier_preference = None;
                    updated.eb_tier_version_created_slot = None;
                    updated.eb_posted_fee = None;
                    updated.eb_tier_delay_slots = None;
                    updated.eb_tier_price_per_byte_at_assignment = None;
                    Self::set_mempool_entry_anchor(
                        &mut updated,
                        mempool_entry_slot,
                        mempool_entry_rb_index,
                    );
                    self.tracker.track_transaction_tier_assigned(
                        updated.id,
                        self.id,
                        BlockKind::RankingBlock,
                        selection.tier_id,
                        selection.tier_version_created_slot,
                        selection.posted_fee,
                        selection.tier_delay_slots,
                    );
                }
                BlockKind::EndorserBlock => {
                    if has_separate_eb_pool {
                        updated.tier_preference = None;
                        updated.tier_version_created_slot = None;
                        updated.posted_fee = None;
                        updated.tier_delay_slots = None;
                        updated.tier_price_per_byte_at_assignment = None;
                        updated.eb_tier_preference = Some(selection.tier_id);
                        updated.eb_tier_version_created_slot =
                            Some(selection.tier_version_created_slot);
                        updated.eb_posted_fee = Some(selection.posted_fee);
                        updated.eb_tier_delay_slots = Some(selection.tier_delay_slots);
                        updated.eb_tier_price_per_byte_at_assignment =
                            Some(selection.tier_price_per_byte_at_assignment);
                    } else {
                        updated.tier_preference = Some(selection.tier_id);
                        updated.tier_version_created_slot =
                            Some(selection.tier_version_created_slot);
                        updated.posted_fee = Some(selection.posted_fee);
                        updated.tier_delay_slots = Some(selection.tier_delay_slots);
                        updated.tier_price_per_byte_at_assignment =
                            Some(selection.tier_price_per_byte_at_assignment);
                        updated.eb_tier_preference = None;
                        updated.eb_tier_version_created_slot = None;
                        updated.eb_posted_fee = None;
                        updated.eb_tier_delay_slots = None;
                        updated.eb_tier_price_per_byte_at_assignment = None;
                    }
                    Self::set_mempool_entry_anchor(
                        &mut updated,
                        mempool_entry_slot,
                        mempool_entry_rb_index,
                    );
                    self.tracker.track_transaction_tier_assigned(
                        updated.id,
                        self.id,
                        BlockKind::EndorserBlock,
                        selection.tier_id,
                        selection.tier_version_created_slot,
                        selection.posted_fee,
                        selection.tier_delay_slots,
                    );
                }
            }

            if let Some(lane) = self.overfull_tier_lane(&pricing, &updated) {
                return (
                    Arc::new(updated),
                    Some(TransactionRejectReason::TierBacklogFull),
                    Some(lane),
                );
            }

            return (Arc::new(updated), None, None);
        }

        // A transaction is accepted if it can afford at least one block kind's tier.
        let accepted = rb_result.is_some() || eb_result.is_some();
        if !accepted {
            return (tx, Some(TransactionRejectReason::TooExpensive), None);
        }

        if let Some((
            tier_id,
            tier_version_created_slot,
            posted_fee,
            tier_delay_slots,
            tier_price_per_byte_at_assignment,
        )) = rb_result
        {
            updated.tier_preference = Some(tier_id);
            updated.tier_version_created_slot = Some(tier_version_created_slot);
            updated.posted_fee = Some(posted_fee);
            updated.tier_delay_slots = Some(tier_delay_slots);
            updated.tier_price_per_byte_at_assignment = Some(tier_price_per_byte_at_assignment);
            Self::set_mempool_entry_anchor(
                &mut updated,
                mempool_entry_slot,
                mempool_entry_rb_index,
            );
            self.tracker.track_transaction_tier_assigned(
                updated.id,
                self.id,
                BlockKind::RankingBlock,
                tier_id,
                tier_version_created_slot,
                posted_fee,
                tier_delay_slots,
            );
        }

        if pricing.has_separate_eb_pool() {
            if let Some((
                eb_tier_id,
                eb_tier_version_created_slot,
                eb_posted_fee,
                eb_tier_delay_slots,
                eb_tier_price_per_byte_at_assignment,
            )) = eb_result
            {
                updated.eb_tier_preference = Some(eb_tier_id);
                updated.eb_tier_version_created_slot = Some(eb_tier_version_created_slot);
                updated.eb_posted_fee = Some(eb_posted_fee);
                updated.eb_tier_delay_slots = Some(eb_tier_delay_slots);
                updated.eb_tier_price_per_byte_at_assignment =
                    Some(eb_tier_price_per_byte_at_assignment);
                Self::set_mempool_entry_anchor(
                    &mut updated,
                    mempool_entry_slot,
                    mempool_entry_rb_index,
                );
                self.tracker.track_transaction_tier_assigned(
                    updated.id,
                    self.id,
                    BlockKind::EndorserBlock,
                    eb_tier_id,
                    eb_tier_version_created_slot,
                    eb_posted_fee,
                    eb_tier_delay_slots,
                );
            }
        }

        if let Some(lane) = self.overfull_tier_lane(&pricing, &updated) {
            return (
                Arc::new(updated),
                Some(TransactionRejectReason::TierBacklogFull),
                Some(lane),
            );
        }

        (Arc::new(updated), None, None)
    }

    fn has_any_tiered_assignment_payload(tx: &Transaction) -> bool {
        tx.tier_preference.is_some()
            || tx.tier_version_created_slot.is_some()
            || tx.posted_fee.is_some()
            || tx.tier_delay_slots.is_some()
            || tx.tier_price_per_byte_at_assignment.is_some()
            || tx.eb_tier_preference.is_some()
            || tx.eb_tier_version_created_slot.is_some()
            || tx.eb_posted_fee.is_some()
            || tx.eb_tier_delay_slots.is_some()
            || tx.eb_tier_price_per_byte_at_assignment.is_some()
            || tx.assigned_block_kind.is_some()
    }

    fn overfull_tier_lane(
        &mut self,
        pricing: &GlobalPricingCoordinator,
        tx: &Transaction,
    ) -> Option<BlockKind> {
        if !pricing.is_tiered() || !pricing.reject_on_pending_tier_overflow() {
            return None;
        }

        let has_separate_eb_pool = pricing.has_separate_eb_pool();
        let uses_shared_single_pool = pricing.uses_shared_single_pool_tiers();

        if let Some(lane) = tx.assigned_block_kind {
            let tier = match lane {
                BlockKind::RankingBlock => tx.tier_preference,
                BlockKind::EndorserBlock => {
                    if has_separate_eb_pool {
                        tx.eb_tier_preference
                    } else {
                        tx.tier_preference
                    }
                }
            };
            let Some(tier) = tier else {
                return None;
            };
            let target_tick = self.tx_target_tick_for_block_kind(tx, lane);
            let pending = self.mempool.pending_bytes_for_tier_target(
                lane,
                tier,
                target_tick,
                has_separate_eb_pool,
            );
            let block_capacity = self.block_capacity_for_kind(lane);
            let Some(capacity) =
                pricing.effective_tier_capacity_for_block_kind(lane, tier, block_capacity)
            else {
                return None;
            };
            let overfull = pending.saturating_add(tx.bytes) > capacity;
            self.tracker.track_transaction_overflow_checked(
                tx.id, self.id, lane, tier, pending, capacity, overfull,
            );
            return overfull.then_some(lane);
        }

        let rb_assigned = tx.tier_preference.is_some();
        let eb_assigned = if has_separate_eb_pool {
            tx.eb_tier_preference.is_some()
        } else {
            uses_shared_single_pool && tx.tier_preference.is_some()
        };

        let over_rb_cap = tx
            .tier_preference
            .and_then(|tier| {
                let target_tick = self.tx_target_tick_for_block_kind(tx, BlockKind::RankingBlock);
                pricing
                    .effective_tier_capacity_for_block_kind(
                        BlockKind::RankingBlock,
                        tier,
                        self.block_capacity_for_kind(BlockKind::RankingBlock),
                    )
                    .map(|cap| {
                        let pending = self.mempool.pending_bytes_for_tier_target(
                            BlockKind::RankingBlock,
                            tier,
                            target_tick,
                            has_separate_eb_pool,
                        );
                        let over = pending.saturating_add(tx.bytes) > cap;
                        self.tracker.track_transaction_overflow_checked(
                            tx.id,
                            self.id,
                            BlockKind::RankingBlock,
                            tier,
                            pending,
                            cap,
                            over,
                        );
                        over
                    })
            })
            .unwrap_or(false);

        let eb_tier = if has_separate_eb_pool {
            tx.eb_tier_preference
        } else if uses_shared_single_pool {
            tx.tier_preference
        } else {
            None
        };
        let over_eb_cap = eb_tier
            .and_then(|tier| {
                let target_tick = self.tx_target_tick_for_block_kind(tx, BlockKind::EndorserBlock);
                pricing
                    .effective_tier_capacity_for_block_kind(
                        BlockKind::EndorserBlock,
                        tier,
                        self.block_capacity_for_kind(BlockKind::EndorserBlock),
                    )
                    .map(|cap| {
                        let pending = self.mempool.pending_bytes_for_tier_target(
                            BlockKind::EndorserBlock,
                            tier,
                            target_tick,
                            has_separate_eb_pool,
                        );
                        let over = pending.saturating_add(tx.bytes) > cap;
                        self.tracker.track_transaction_overflow_checked(
                            tx.id,
                            self.id,
                            BlockKind::EndorserBlock,
                            tier,
                            pending,
                            cap,
                            over,
                        );
                        over
                    })
            })
            .unwrap_or(false);
        let should_reject =
            Self::should_reject_on_overflow(rb_assigned, over_rb_cap, eb_assigned, over_eb_cap);
        if !should_reject {
            return None;
        }

        match (rb_assigned && over_rb_cap, eb_assigned && over_eb_cap) {
            (true, false) => Some(BlockKind::RankingBlock),
            (false, true) => Some(BlockKind::EndorserBlock),
            (true, true) => Some(self.select_retry_lane_when_both_overfull(tx)),
            (false, false) => None,
        }
    }

    fn overflow_state_for_lane(
        &self,
        tx: &Transaction,
        lane: BlockKind,
    ) -> Option<(TierId, u64, u64)> {
        let pricing = self.pricing.as_ref()?;
        let has_separate_eb_pool = pricing.has_separate_eb_pool();
        let tier = match lane {
            BlockKind::RankingBlock => tx.tier_preference?,
            BlockKind::EndorserBlock => {
                if has_separate_eb_pool {
                    tx.eb_tier_preference?
                } else {
                    tx.tier_preference?
                }
            }
        };
        let target_tick = self.tx_target_tick_for_block_kind(tx, lane);
        let pending = self.mempool.pending_bytes_for_tier_target(
            lane,
            tier,
            target_tick,
            has_separate_eb_pool,
        );
        let capacity = pricing.effective_tier_capacity_for_block_kind(
            lane,
            tier,
            self.block_capacity_for_kind(lane),
        )?;
        Some((tier, pending, capacity))
    }

    fn record_overflow_rejected_bytes(
        &mut self,
        lane: BlockKind,
        tier: TierId,
        tx_id: TransactionId,
        bytes: u64,
    ) {
        let seen = match lane {
            BlockKind::RankingBlock => self.overflow_rejected_ids_rb.entry(tier).or_default(),
            BlockKind::EndorserBlock => self.overflow_rejected_ids_eb.entry(tier).or_default(),
        };
        if !seen.insert(tx_id) {
            // Deduplicate repeated rejections of the same transaction (e.g. gossip/retries)
            // within one pricing-update window for this lane+tier.
            return;
        }
        let entry = match lane {
            BlockKind::RankingBlock => self.overflow_rejected_bytes_rb.entry(tier).or_default(),
            BlockKind::EndorserBlock => self.overflow_rejected_bytes_eb.entry(tier).or_default(),
        };
        *entry = entry.saturating_add(bytes);
    }

    fn aggregated_overflow_txs_for_block_kind(
        &mut self,
        block_kind: BlockKind,
        slot: u64,
    ) -> Vec<Arc<Transaction>> {
        let entries: Vec<(TierId, u64)> = match block_kind {
            BlockKind::RankingBlock => self
                .overflow_rejected_bytes_rb
                .iter()
                .map(|(tier, bytes)| (*tier, *bytes))
                .collect(),
            BlockKind::EndorserBlock => self
                .overflow_rejected_bytes_eb
                .iter()
                .map(|(tier, bytes)| (*tier, *bytes))
                .collect(),
        };
        entries
            .into_iter()
            .filter_map(|(tier, bytes)| {
                if bytes == 0 {
                    return None;
                }
                let tx_id = TransactionId::new(self.next_overflow_aggregate_tx_id);
                self.next_overflow_aggregate_tx_id =
                    self.next_overflow_aggregate_tx_id.saturating_add(1);

                let mut tx = Transaction {
                    id: tx_id,
                    actor_id: ActorId::new(0),
                    shard: 0,
                    bytes,
                    submission_slot: slot,
                    mempool_entry_slot: None,
                    mempool_entry_rb_index: None,
                    value: 0,
                    urgency: crate::model::UrgencyProfile::Indifferent,
                    posted_fee: None,
                    tier_preference: None,
                    tier_version_created_slot: None,
                    tier_delay_slots: None,
                    tier_price_per_byte_at_assignment: None,
                    eb_tier_preference: None,
                    eb_tier_version_created_slot: None,
                    eb_posted_fee: None,
                    eb_tier_delay_slots: None,
                    eb_tier_price_per_byte_at_assignment: None,
                    assigned_block_kind: Some(block_kind),
                    input_id: self.next_overflow_aggregate_tx_id,
                    overcollateralization_factor: 0,
                    urgency_component_index: None,
                };

                match block_kind {
                    BlockKind::RankingBlock => {
                        tx.tier_preference = Some(tier);
                        tx.tier_delay_slots = Some(1);
                        tx.tier_price_per_byte_at_assignment = Some(0);
                    }
                    BlockKind::EndorserBlock => {
                        tx.eb_tier_preference = Some(tier);
                        tx.eb_tier_delay_slots = Some(1);
                        tx.eb_tier_price_per_byte_at_assignment = Some(0);
                    }
                }
                Some(Arc::new(tx))
            })
            .collect()
    }

    fn clear_aggregated_overflow_for_block_kind(&mut self, block_kind: BlockKind) {
        match block_kind {
            BlockKind::RankingBlock => {
                self.overflow_rejected_bytes_rb.clear();
                self.overflow_rejected_ids_rb.clear();
            }
            BlockKind::EndorserBlock => {
                self.overflow_rejected_bytes_eb.clear();
                self.overflow_rejected_ids_eb.clear();
            }
        }
    }

    fn should_reject_on_overflow(
        rb_assigned: bool,
        rb_overfull: bool,
        eb_assigned: bool,
        eb_overfull: bool,
    ) -> bool {
        match (rb_assigned, eb_assigned) {
            (true, true) => rb_overfull && eb_overfull,
            (true, false) => rb_overfull,
            (false, true) => eb_overfull,
            (false, false) => false,
        }
    }

    fn select_retry_lane_when_both_overfull(&self, tx: &Transaction) -> BlockKind {
        let delay_model = self
            .pricing
            .as_ref()
            .map(|pricing| pricing.tier_selection_delay_model())
            .unwrap_or(TierSelectionDelayModel::TierDelay);
        let rb_retained_ratio = Self::retained_value_ratio_for_lane(
            tx,
            BlockKind::RankingBlock,
            OverflowRetryCurveMetric::RetainedValueRatio,
            delay_model,
        );
        let eb_retained_ratio = Self::retained_value_ratio_for_lane(
            tx,
            BlockKind::EndorserBlock,
            OverflowRetryCurveMetric::RetainedValueRatio,
            delay_model,
        );
        if eb_retained_ratio > rb_retained_ratio {
            BlockKind::EndorserBlock
        } else {
            BlockKind::RankingBlock
        }
    }

    fn schedule_overflow_retry(&mut self, tx: &Arc<Transaction>, lane: BlockKind) -> bool {
        let Some(pricing) = self.pricing.clone() else {
            return false;
        };
        let has_separate_eb_pool = pricing.has_separate_eb_pool();
        let retry_tier = match lane {
            BlockKind::RankingBlock => tx.tier_preference,
            BlockKind::EndorserBlock => {
                if has_separate_eb_pool {
                    tx.eb_tier_preference
                } else {
                    tx.tier_preference
                }
            }
        };
        let Some(retry_tier) = retry_tier else {
            return false;
        };
        let Some(global_policy) = pricing.overflow_retry_policy() else {
            return false;
        };
        let policy = self
            .actor_overflow_retry_policy_overrides
            .get(&tx.actor_id)
            .cloned()
            .unwrap_or(global_policy);
        if !policy.enabled || policy.source != OverflowRetrySource::LocalActorSubmissions {
            return false;
        }

        let retained_value_ratio = Self::retained_value_ratio_for_lane(
            tx,
            lane,
            policy.curve_metric,
            pricing.tier_selection_delay_model(),
        );
        let Some(band) = policy.band_for_retained_ratio(retained_value_ratio) else {
            return false;
        };

        let attempts_so_far = self
            .overflow_retry_attempts
            .get(&tx.id)
            .copied()
            .unwrap_or(0);
        if attempts_so_far >= band.max_attempts {
            return false;
        }
        let delay_slots = policy.retry_delay_slots(band, attempts_so_far);
        if delay_slots == 0 {
            return false;
        }
        let attempt = attempts_so_far.saturating_add(1);
        self.overflow_retry_attempts.insert(tx.id, attempt);
        let retry_tx = Self::clear_tiered_assignment_and_maturity(tx);
        self.queued.schedule_event(
            self.clock.now() + Duration::from_secs(delay_slots),
            TimedEvent::RetryOverflowTx(retry_tx),
        );
        self.tracker.track_transaction_retry_scheduled(
            tx.id,
            self.id,
            tx.actor_id,
            attempt,
            delay_slots,
            retained_value_ratio,
            lane,
            retry_tier,
        );
        true
    }

    fn retained_value_ratio_for_lane(
        tx: &Transaction,
        lane: BlockKind,
        metric: OverflowRetryCurveMetric,
        delay_model: TierSelectionDelayModel,
    ) -> f64 {
        match metric {
            OverflowRetryCurveMetric::RetainedValueRatio => {
                let (tier_delay_slots, tier_id) = match lane {
                    BlockKind::RankingBlock => {
                        (tx.tier_delay_slots.unwrap_or(1), tx.tier_preference)
                    }
                    BlockKind::EndorserBlock => (
                        tx.eb_tier_delay_slots.or(tx.tier_delay_slots).unwrap_or(1),
                        tx.eb_tier_preference.or(tx.tier_preference),
                    ),
                };
                let utility_delay = delay_model.utility_delay_units_for_lane(
                    lane,
                    tier_delay_slots.max(1),
                    tier_id,
                );
                if tx.value == 0 {
                    return 1.0;
                }
                let retained_value = tx.urgency.value_at_delay(tx.value, utility_delay);
                (retained_value as f64 / tx.value as f64).clamp(0.0, 1.0)
            }
        }
    }
}

// Ranking block propagation
impl LinearLeiosNode {
    fn try_generate_rb(&mut self, slot: u64) {
        let Some(vrf) = self.run_vrf(
            LotteryKind::GenerateRB,
            self.sim_config.block_generation_probability,
        ) else {
            return;
        };
        let candidate_rb_index = self.current_observed_rb_index().saturating_add(1);

        let parent = self.latest_rb_id();
        let endorsement = parent.and_then(|rb_id| {
            let earliest_endorse_time = Timestamp::from_secs(rb_id.slot)
                + (self.sim_config.header_diffusion_time * 3)
                + Duration::from_secs(self.sim_config.linear_vote_stage_length)
                + Duration::from_secs(self.sim_config.linear_diffuse_stage_length);

            if earliest_endorse_time > Timestamp::from_secs(slot) {
                // This RB was generated too quickly after another; hasn't been time to gather all the votes.
                // No endorsement.
                return None;
            }

            let eb_id = *self.leios.ebs_by_rb.get(&rb_id)?;
            let votes = self.leios.votes_by_eb.get(&eb_id)?;
            let total_votes = votes.values().copied().sum::<usize>();
            if (total_votes as u64) < self.sim_config.vote_threshold {
                // Not enough votes. No endorsement.
                return None;
            }
            let votes = votes.clone();

            if let Some(eb) = self.get_validated_eb(eb_id) {
                // If we're endorsing this EB, clear its TXs out of the mempool now
                // so that we don't include them in new blocks.
                self.remove_eb_txs_from_mempool(&eb);
            } else {
                // We haven't finished validating this EB, maybe even haven't received it and its contents.
                // That won't stop us from generating the endorsement, though it'll make us produce an empty block.
                self.leios.incomplete_onchain_ebs.insert(eb_id);
            }

            Some(Endorsement {
                eb: eb_id,
                size_bytes: self.sim_config.sizes.cert(votes.len()),
                votes: votes.clone(),
            })
        });

        // If we haven't validated any EBs from the current chain, we have no way to tell whether
        // including a TX would introduce conflicts. So, don't include ANY TXs, just to be safe.
        let produce_empty_block = !self.leios.incomplete_onchain_ebs.is_empty();
        let allow_rb_inline_txs =
            endorsement.is_none() || self.sim_config.rb_inline_txs_with_endorsement;

        let mut rb_transactions = vec![];
        if !produce_empty_block && self.sim_config.praos_fallback && allow_rb_inline_txs {
            if let TransactionConfig::Mock(config) = &self.sim_config.transactions {
                // Add one transaction, the right size for the extra RB payload
                let tx = config.mock_tx(config.rb_size);
                self.tracker.track_transaction_generated(&tx, self.id);
                rb_transactions.push(Arc::new(tx));
            } else {
                self.sample_from_mempool(
                    &mut rb_transactions,
                    self.sim_config.max_block_size,
                    true,
                    slot,
                    candidate_rb_index,
                    BlockKind::RankingBlock,
                );
            }
        }

        let mut eb_transactions = vec![];
        let mut withheld_txs = vec![];
        if !produce_empty_block {
            // If we are performing a "withheld TX" attack, we will include a bunch of brand-new TXs in this EB.
            // They will get disseminated through the network at the same time as the EB.
            withheld_txs = self.generate_withheld_txs(slot);
            eb_transactions.extend(withheld_txs.iter().cloned());

            if let TransactionConfig::Mock(config) = &self.sim_config.transactions {
                // Add one transaction, the right size for the extra RB payload
                let extra_size =
                    config.eb_size - withheld_txs.iter().map(|tx| tx.bytes).sum::<u64>();
                if extra_size > 0 {
                    let tx = config.mock_tx(extra_size);
                    self.tracker.track_transaction_generated(&tx, self.id);
                    eb_transactions.push(Arc::new(tx));
                }
            } else {
                self.sample_from_mempool(
                    &mut eb_transactions,
                    self.sim_config.max_eb_size,
                    false,
                    slot,
                    candidate_rb_index,
                    BlockKind::EndorserBlock,
                );
            }
        }
        let eb_opportunity_available = !produce_empty_block;
        let (eb_announcement, eb) = if eb_transactions.is_empty() {
            (None, None)
        } else {
            let eb_id = EndorserBlockId {
                slot,
                pipeline: 0,
                producer: self.id,
            };
            let eb = EndorserBlock {
                slot,
                producer: self.id,
                bytes: self.sim_config.sizes.linear_eb(&eb_transactions),
                txs: eb_transactions,
            };
            (Some(eb_id), Some((eb, withheld_txs)))
        };

        let rb = RankingBlock {
            header: RankingBlockHeader {
                id: BlockId {
                    slot,
                    producer: self.id,
                },
                vrf,
                parent,
                bytes: self.sim_config.sizes.block_header,
                eb_announcement,
            },
            transactions: rb_transactions,
            endorsement,
        };

        self.tracker.track_praos_block_lottery_won(rb.header.id);
        self.queued
            .schedule_cpu_task(CpuTask::RBBlockGenerated(rb, eb, eb_opportunity_available));
    }

    fn finish_generating_rb(
        &mut self,
        rb: RankingBlock,
        eb: Option<(EndorserBlock, Vec<Arc<Transaction>>)>,
        eb_opportunity_available: bool,
    ) {
        self.tracker.track_linear_rb_generated(&rb);
        if self.shared_single_pool_rb_only_pricing_mode() {
            self.maybe_update_shared_single_pool_pricing_for_rb(&rb);
        } else {
            self.update_pricing_after_block(
                PricingUpdateKey::RankingBlock(rb.header.id),
                &rb.transactions,
                self.sim_config.max_block_size,
                BlockKind::RankingBlock,
                rb.header.id.slot,
            );
        }
        if !self.shared_single_pool_rb_only_pricing_mode()
            && eb.is_none()
            && eb_opportunity_available
        {
            self.update_pricing_after_block(
                PricingUpdateKey::EndorserOpportunity(rb.header.id),
                &[],
                self.sim_config.max_eb_size,
                BlockKind::EndorserBlock,
                rb.header.id.slot,
            );
        }
        self.publish_rb(Arc::new(rb), false);
        if let Some((eb, withheld_txs)) = eb {
            self.tracker.track_linear_eb_generated(&eb);
            self.finish_generating_eb(eb, withheld_txs);
        }
    }

    fn publish_rb(&mut self, rb: Arc<RankingBlock>, already_sent_header: bool) {
        let header_seen = self
            .praos
            .blocks
            .get(&rb.header.id)
            .and_then(|rb| rb.header_seen())
            .unwrap_or(self.clock.now());
        if let Some(eb_id) = rb.header.eb_announcement {
            self.leios.ebs_by_rb.insert(rb.header.id, eb_id);
        }
        let height = self.rb_height_for_parent(rb.header.parent);
        self.praos.blocks.insert(
            rb.header.id,
            RankingBlockView::Received {
                rb: rb.clone(),
                header_seen,
                height,
            },
        );
        self.remove_rb_txs_from_mempool(&rb);
        for peer in &self.consumers {
            if self
                .praos
                .peer_heads
                .get(peer)
                .is_none_or(|&s| s < rb.header.id.slot)
            {
                let message = if already_sent_header {
                    Message::AnnounceRB(rb.header.id)
                } else {
                    Message::AnnounceRBHeader(rb.header.id)
                };
                self.queued.send_to(*peer, message);
                self.praos.peer_heads.insert(*peer, rb.header.id.slot);
            }
        }
    }

    fn receive_announce_rb_header(&mut self, from: NodeId, id: BlockId) {
        let should_request = match self.praos.blocks.get(&id) {
            None => true,
            Some(RankingBlockView::HeaderPending) => {
                self.sim_config.relay_strategy == RelayStrategy::RequestFromAll
            }
            _ => false,
        };
        if should_request {
            self.praos
                .blocks
                .insert(id, RankingBlockView::HeaderPending);
            self.queued.send_to(from, Message::RequestRBHeader(id));
        }
    }

    fn receive_request_rb_header(&mut self, from: NodeId, id: BlockId) {
        if let Some(rb) = self.praos.blocks.get(&id)
            && let Some(header) = rb.header()
        {
            // If we already have this RB's body,
            // let the requester know that it's ready to fetch.
            let have_body = matches!(rb, RankingBlockView::Received { .. });
            // If we already have the EB announced by this RB,
            // let the requester know that they can fetch it.
            // But if we are maliciously withholding the EB, do not let them know.
            let have_eb = header.eb_announcement.is_some_and(|eb_id| {
                matches!(
                    self.leios.ebs.get(&eb_id),
                    Some(EndorserBlockView::Received { .. })
                )
            }) && !self.should_withhold_ebs();
            self.queued
                .send_to(from, Message::RBHeader(header.clone(), have_body, have_eb));
        }
    }

    fn receive_rb_header(
        &mut self,
        from: NodeId,
        header: RankingBlockHeader,
        has_body: bool,
        has_eb: bool,
    ) {
        self.queued
            .schedule_cpu_task(CpuTask::RBHeaderValidated(from, header, has_body, has_eb));
    }

    fn finish_validating_rb_header(
        &mut self,
        from: NodeId,
        header: RankingBlockHeader,
        has_body: bool,
        has_eb: bool,
    ) {
        if let Some(old_block_id) = self.praos.block_ids_by_slot.get(&header.id.slot) {
            // SLOT BATTLE!!! lower VRF wins
            if let Some(old_header) = self.praos.blocks.get(old_block_id).and_then(|b| b.header()) {
                if old_header.vrf <= header.vrf {
                    // We like our block better than this new one.
                    return;
                }

                // Forget we ever saw that other block
                if let Some(RankingBlockView::Received { rb, .. }) =
                    self.praos.blocks.remove(old_block_id)
                    && let Some(endorsement) = &rb.endorsement
                {
                    self.leios.incomplete_onchain_ebs.remove(&endorsement.eb);
                }
            }
        }
        self.praos
            .block_ids_by_slot
            .insert(header.id.slot, header.id);
        self.praos.blocks.insert(
            header.id,
            RankingBlockView::Pending {
                header: header.clone(),
                header_seen: self.clock.now(),
            },
        );

        let head = self.praos.peer_heads.entry(from).or_default();
        if *head < header.id.slot {
            *head = header.id.slot
        }
        for peer in &self.consumers {
            if *peer == from {
                continue;
            }
            self.queued
                .send_to(*peer, Message::AnnounceRBHeader(header.id));
        }
        if has_body {
            self.queued.send_to(from, Message::RequestRB(header.id));
        }

        // Get ready to fetch the announced EB (if we don't have it already)
        let Some(eb_id) = header.eb_announcement else {
            return;
        };
        if matches!(
            self.leios.ebs.get(&eb_id),
            Some(EndorserBlockView::Received { .. })
        ) {
            return;
        }

        let eb_peer_announcements = self.leios.eb_peer_announcements.entry(eb_id).or_default();
        if has_eb {
            eb_peer_announcements.push(from);
        }

        // TODO: freshest first
        let peers_to_request_from = match self.sim_config.relay_strategy {
            RelayStrategy::RequestFromFirst => eb_peer_announcements
                .first()
                .iter()
                .copied()
                .copied()
                .collect::<Vec<_>>(),
            RelayStrategy::RequestFromAll => eb_peer_announcements.clone(),
        };

        if peers_to_request_from.is_empty() {
            // nobody we know has this EB yet, wait for someone to announce it
            self.leios.ebs.insert(eb_id, EndorserBlockView::Pending);
        } else {
            for peer in peers_to_request_from {
                self.queued.send_to(peer, Message::RequestEB(eb_id));
            }
            self.leios.ebs.insert(eb_id, EndorserBlockView::Requested);
        }
    }

    fn receive_announce_rb(&mut self, from: NodeId, id: BlockId) {
        let (header, header_seen) = match self.praos.blocks.get(&id) {
            Some(RankingBlockView::Pending {
                header,
                header_seen,
                ..
            }) => (header.clone(), *header_seen),
            Some(RankingBlockView::Requested {
                header,
                header_seen,
            }) => {
                if self.sim_config.relay_strategy == RelayStrategy::RequestFromAll {
                    (header.clone(), *header_seen)
                } else {
                    return;
                }
            }
            _ => return,
        };

        self.praos.blocks.insert(
            id,
            RankingBlockView::Requested {
                header,
                header_seen,
            },
        );
        self.queued.send_to(from, Message::RequestRB(id));
    }

    fn receive_request_rb(&mut self, from: NodeId, id: BlockId) {
        if let Some(RankingBlockView::Received { rb, .. }) = self.praos.blocks.get(&id) {
            self.tracker.track_linear_rb_sent(rb, self.id, from);
            self.queued.send_to(from, Message::RB(rb.clone()));
        }
    }

    fn receive_rb(&mut self, from: NodeId, rb: Arc<RankingBlock>) {
        self.tracker.track_linear_rb_received(&rb, from, self.id);
        self.queued.schedule_cpu_task(CpuTask::RBBlockValidated(rb));
    }

    fn finish_validating_rb(&mut self, rb: Arc<RankingBlock>) {
        let header_seen = self
            .praos
            .blocks
            .get(&rb.header.id)
            .and_then(|rb| rb.header_seen())
            .unwrap_or(self.clock.now());
        self.praos.blocks.insert(
            rb.header.id,
            RankingBlockView::Received {
                rb: rb.clone(),
                header_seen,
                height: self.rb_height_for_parent(rb.header.parent),
            },
        );
        if let Some(endorsement) = &rb.endorsement
            && !self.is_eb_validated(endorsement.eb)
        {
            self.leios.incomplete_onchain_ebs.insert(endorsement.eb);
        }

        if self.shared_single_pool_rb_only_pricing_mode() {
            self.maybe_update_shared_single_pool_pricing_for_rb(&rb);
        } else {
            self.update_pricing_after_block(
                PricingUpdateKey::RankingBlock(rb.header.id),
                &rb.transactions,
                self.sim_config.max_block_size,
                BlockKind::RankingBlock,
                rb.header.id.slot,
            );
        }
        self.publish_rb(rb, true);
    }

    fn latest_rb(&self) -> Option<(&Arc<RankingBlock>, Timestamp)> {
        self.praos.blocks.iter().rev().find_map(|(_, rb)| {
            if let RankingBlockView::Received {
                rb, header_seen, ..
            } = rb
            {
                Some((rb, *header_seen))
            } else {
                None
            }
        })
    }

    fn latest_rb_id(&self) -> Option<BlockId> {
        self.latest_rb().map(|(rb, _)| rb.header.id)
    }

    fn latest_rb_height(&self) -> Option<u64> {
        self.praos
            .blocks
            .iter()
            .rev()
            .find_map(|(_, rb)| rb.height())
    }
}

// EB operations
impl LinearLeiosNode {
    fn finish_generating_eb(&mut self, eb: EndorserBlock, withheld_txs: Vec<Arc<Transaction>>) {
        let eb_id = eb.id();
        let eb = Arc::new(eb);
        self.leios.ebs.insert(
            eb_id,
            EndorserBlockView::Received {
                eb: eb.clone(),
                seen: self.clock.now(),
                all_txs_seen: true,
                validated: true,
            },
        );
        if !self.shared_single_pool_rb_only_pricing_mode() {
            self.update_pricing_after_block(
                PricingUpdateKey::EndorserBlock(eb_id),
                &eb.txs,
                self.sim_config.max_eb_size,
                BlockKind::EndorserBlock,
                eb_id.slot,
            );
        }

        if self.should_withhold_ebs() {
            // We're an evil attacker, holding onto this EB until just long enough to collect votes.
            // Send it out-of-band to our evil buddies.
            self.share_new_withheld_eb(&eb, withheld_txs);
        } else {
            // We're a "well-behaved" node who will tell all our peers about this EB immediately.
            for peer in &self.consumers {
                self.queued.send_to(*peer, Message::AnnounceEB(eb_id));
                // If we were withholding some of the EB's transactions, start disseminating them now.
                for tx in &withheld_txs {
                    self.queued.send_to(*peer, Message::AnnounceTx(tx.id));
                }
            }
        }
        self.vote_for_endorser_block(&eb, self.clock.now());
    }

    fn receive_announce_eb(&mut self, from: NodeId, id: EndorserBlockId) {
        self.leios
            .eb_peer_announcements
            .entry(id)
            .or_default()
            .push(from);
        let should_request = match self.leios.ebs.get(&id) {
            Some(EndorserBlockView::Pending) => true,
            Some(EndorserBlockView::Requested) => {
                self.sim_config.relay_strategy == RelayStrategy::RequestFromAll
            }
            _ => false,
        };
        if should_request {
            // TODO: freshest first
            self.leios.ebs.insert(id, EndorserBlockView::Requested);
            self.queued.send_to(from, Message::RequestEB(id));
        }
    }

    fn receive_request_eb(&mut self, from: NodeId, id: EndorserBlockId) {
        if let Some(EndorserBlockView::Received { eb, .. }) = self.leios.ebs.get(&id) {
            self.tracker.track_linear_eb_sent(eb, self.id, from);
            self.queued.send_to(from, Message::EB(eb.clone()));
        }
    }

    fn receive_eb(&mut self, from: NodeId, eb: Arc<EndorserBlock>) {
        self.tracker.track_eb_received(eb.id(), from, self.id);
        self.queued
            .schedule_cpu_task(CpuTask::EBHeaderValidated(from, eb));
    }

    fn finish_validating_eb_header(&mut self, from: NodeId, eb: Arc<EndorserBlock>) {
        if let Some(EndorserBlockView::Received { .. }) = self.leios.ebs.get(&eb.id()) {
            // already received this EB
            return;
        }
        let seen = self.clock.now();
        let missing_txs = if matches!(self.sim_config.variant, LeiosVariant::Linear) {
            vec![]
        } else {
            eb.txs
                .iter()
                .map(|tx| tx.id)
                .filter(|id| !self.has_tx(*id))
                .collect()
        };
        self.leios.ebs.insert(
            eb.id(),
            EndorserBlockView::Received {
                eb: eb.clone(),
                seen,
                all_txs_seen: missing_txs.is_empty(),
                validated: false,
            },
        );

        let should_propagate_now = match self.sim_config.linear_eb_propagation_criteria {
            EBPropagationCriteria::EbReceived => true,
            EBPropagationCriteria::TxsReceived => missing_txs.is_empty(),
            EBPropagationCriteria::FullyValid => false,
        };
        if should_propagate_now {
            for peer in &self.consumers {
                if *peer == from {
                    continue;
                }
                self.queued.send_to(*peer, Message::AnnounceEB(eb.id()));
            }
        }

        if missing_txs.is_empty() {
            self.queued
                .schedule_cpu_task(CpuTask::EBBlockValidated(eb.clone(), seen));
        } else {
            for tx_id in missing_txs {
                self.leios
                    .missing_txs
                    .entry(tx_id)
                    .or_default()
                    .push(eb.id());
            }
        }

        if matches!(
            self.sim_config.variant,
            LeiosVariant::LinearWithTxReferences
        ) {
            // If the EB references any TXs which we already have, but are not in our mempool,
            // either we must have failed to add them to the mempool due to conflicts,
            // or they haven't reached the mempool _yet_.
            // Announce those TXs to our peers, since either way we didn't before.
            let mempool_ids = self.mempool.ids().collect::<HashSet<_>>();
            for tx in &eb.txs {
                if !self.has_tx(tx.id) || mempool_ids.contains(&tx.id) {
                    continue;
                }
                for peer in &self.consumers {
                    self.queued.send_to(*peer, Message::AnnounceTx(tx.id));
                }
            }
        }
    }

    fn try_validating_eb(&mut self, eb_id: EndorserBlockId) {
        let Some(EndorserBlockView::Received {
            eb,
            seen,
            all_txs_seen: false,
            validated: false,
        }) = self.leios.ebs.get(&eb_id)
        else {
            return;
        };
        let all_seen = eb.txs.iter().all(|tx| self.has_tx(tx.id));
        if all_seen {
            let eb = eb.clone();
            let seen = *seen;
            self.leios.ebs.insert(
                eb_id,
                EndorserBlockView::Received {
                    eb: eb.clone(),
                    seen,
                    all_txs_seen: true,
                    validated: false,
                },
            );
            if matches!(
                self.sim_config.linear_eb_propagation_criteria,
                EBPropagationCriteria::TxsReceived
            ) {
                // We have received all transactions, but haven't validated the entirety of the EB yet.
                // Propagate it now anyway.
                for peer in &self.consumers {
                    self.queued.send_to(*peer, Message::AnnounceEB(eb_id));
                }
            }
            self.queued
                .schedule_cpu_task(CpuTask::EBBlockValidated(eb, seen));
        }
    }

    fn finish_validating_eb(&mut self, eb: Arc<EndorserBlock>, seen: Timestamp) {
        if self.leios.incomplete_onchain_ebs.remove(&eb.id()) {
            self.remove_eb_txs_from_mempool(&eb);
        }
        let Some(EndorserBlockView::Received { validated, .. }) = self.leios.ebs.get_mut(&eb.id())
        else {
            panic!("how did we validate this EB without ever seeing it?");
        };
        *validated = true;
        if matches!(
            self.sim_config.linear_eb_propagation_criteria,
            EBPropagationCriteria::FullyValid
        ) {
            // We have received all transactions, but haven't validated the entirety of the EB yet.
            // Propagate it now anyway.
            for peer in &self.consumers {
                self.queued.send_to(*peer, Message::AnnounceEB(eb.id()));
            }
        }
        if self.shared_single_pool_rb_only_pricing_mode() {
            for rb in self.rbs_endorsing_eb(eb.id()) {
                self.maybe_update_shared_single_pool_pricing_for_rb(&rb);
            }
        } else {
            self.update_pricing_after_block(
                PricingUpdateKey::EndorserBlock(eb.id()),
                &eb.txs,
                self.sim_config.max_eb_size,
                BlockKind::EndorserBlock,
                eb.id().slot,
            );
        }
        self.vote_for_endorser_block(&eb, seen);
    }

    fn is_eb_validated(&self, eb_id: EndorserBlockId) -> bool {
        self.get_validated_eb(eb_id).is_some()
    }

    fn get_validated_eb(&self, eb_id: EndorserBlockId) -> Option<Arc<EndorserBlock>> {
        match self.leios.ebs.get(&eb_id) {
            Some(EndorserBlockView::Received {
                eb,
                validated: true,
                ..
            }) => Some(eb.clone()),
            _ => None,
        }
    }
}

// EB withholding:
// an attack on Linear Leios where one or more stake pools deliberately wait to
// propagate an EB until there is just barely enough time for honest nodes to vote on it.
// This increases the odds that an honest RB producer won't have the parent RB's EB yet,
// meaning they will need to publish a completely empty RB.
impl LinearLeiosNode {
    // This is called during simulation setup.
    // It tells this node that it should withhold EBs,
    // and sets up a side channel with all other nodes performing the same attack.
    pub fn register_as_eb_withholder(
        &mut self,
        sender: EBWithholdingSender,
    ) -> mpsc::UnboundedSender<EBWithholdingEvent> {
        self.eb_withholding_sender = Some(sender);
        let (sink, source) = mpsc::unbounded_channel();
        self.eb_withholding_event_source = Some(source);
        sink
    }

    fn should_withhold_ebs(&self) -> bool {
        self.eb_withholding_sender.is_some()
    }

    fn share_new_withheld_eb(
        &mut self,
        eb: &Arc<EndorserBlock>,
        withheld_txs: Vec<Arc<Transaction>>,
    ) {
        let sender = self
            .eb_withholding_sender
            .as_ref()
            .expect("eb_withholding_sender must be set via register_as_eb_withholder before calling share_new_withheld_eb");
        sender.send(eb.clone(), withheld_txs);
    }

    fn receive_withheld_eb(&mut self, eb: Arc<EndorserBlock>, withheld_txs: Vec<Arc<Transaction>>) {
        self.leios.ebs.insert(
            eb.id(),
            EndorserBlockView::Received {
                eb: eb.clone(),
                seen: self.clock.now(),
                all_txs_seen: true,
                validated: true,
            },
        );
        for tx in withheld_txs {
            // Add the peer's withheld TXs to the list we know of,
            // but not to our mempools
            self.txs.insert(tx.id, TransactionView::Received(tx));
        }
        // If an attacker receives an EB over a side channel,
        // it will skip validation and will not disseminate it to peers.
        // It will, however, try to vote for the EB immediately.
        self.vote_for_endorser_block(&eb, self.clock.now());
    }

    fn disseminate_withheld_eb(
        &mut self,
        eb_id: EndorserBlockId,
        withheld_txs: Vec<TransactionId>,
    ) {
        for peer in &self.consumers {
            self.queued.send_to(*peer, Message::AnnounceEB(eb_id));
            for tx_id in &withheld_txs {
                self.queued.send_to(*peer, Message::AnnounceTx(*tx_id));
            }
        }
    }
}

// TX withholding:
// an attack where a stake pool generates EBs with previously unknown
// transactions, so that they cannot propagate in advance.
// We implement this by generating the transactions at the same time as the EB itself.
impl LinearLeiosNode {
    fn generate_withheld_txs(&mut self, slot: u64) -> Vec<Arc<Transaction>> {
        if !self.behaviours.withhold_txs {
            return vec![];
        }
        let withhold_tx_config = self.sim_config.attacks.late_tx.as_ref().expect(
            "attacks.late_tx config must be present when withhold_txs behaviour is enabled",
        );
        let slot_ts = Timestamp::from_secs(slot);
        if withhold_tx_config.start_time.is_some_and(|s| slot_ts < s) {
            return vec![];
        }
        if withhold_tx_config.stop_time.is_some_and(|s| slot_ts > s) {
            return vec![];
        }
        if !self.rng.random_bool(withhold_tx_config.probability) {
            return vec![];
        }

        let txs_to_generate = withhold_tx_config.txs_to_generate.sample(&mut self.rng) as u64;
        let mut txs = vec![];
        for _ in 0..txs_to_generate {
            let mut tx = match &self.sim_config.transactions {
                TransactionConfig::Real(cfg) => cfg.new_tx(&mut self.rng, None),
                TransactionConfig::Mock(cfg) => cfg.mock_tx(cfg.eb_size / txs_to_generate),
            };
            tx.submission_slot = slot;
            self.tracker.track_transaction_generated(&tx, self.id);
            let (tx, rejection_reason, _) =
                self.apply_pricing_on_arrival(Arc::new(tx), slot, self.current_observed_rb_index());
            if let Some(reason) = rejection_reason {
                self.tracker
                    .track_transaction_rejected(tx.id, self.id, reason);
                continue;
            }
            self.txs
                .insert(tx.id, TransactionView::Received(tx.clone()));
            txs.push(tx);
        }
        txs
    }
}

// Voting
impl LinearLeiosNode {
    fn vote_for_endorser_block(&mut self, eb: &Arc<EndorserBlock>, seen: Timestamp) {
        let equivocation_cutoff_time =
            Timestamp::from_secs(eb.slot) + (self.sim_config.header_diffusion_time * 3);
        if eb.producer != self.id && self.clock.now() < equivocation_cutoff_time {
            // If we haven't waited long enough to detect equivocations,
            // schedule voting later.
            self.queued.schedule_event(
                equivocation_cutoff_time,
                TimedEvent::TryVote(eb.clone(), seen),
            );
            return;
        }
        if !self.try_vote_for_endorser_block(eb, seen) && self.sim_config.emit_conformance_events {
            self.tracker
                .track_linear_no_vote_generated(self.id, eb.id());
        }
    }

    fn try_vote_for_endorser_block(&mut self, eb: &Arc<EndorserBlock>, seen: Timestamp) -> bool {
        let vrf_wins = vrf_probabilities(self.sim_config.vote_probability)
            .filter_map(|f| self.run_vrf(LotteryKind::GenerateVote, f))
            .count();
        if vrf_wins == 0 {
            return false;
        }

        let id = VoteBundleId {
            slot: eb.slot,
            pipeline: 0,
            producer: self.id,
        };
        self.tracker.track_vote_lottery_won(id);

        if let Err(reason) = self.should_vote_for(eb, seen) {
            self.tracker
                .track_no_vote(eb.slot, 0, self.id, eb.id(), reason);
            return false;
        }

        let mut ebs = BTreeMap::new();
        ebs.insert(eb.id(), vrf_wins);
        let votes = VoteBundle {
            id,
            bytes: self.sim_config.sizes.vote_bundle(1),
            ebs,
        };
        self.queued
            .schedule_cpu_task(CpuTask::VTBundleGenerated(votes, eb.clone()));
        true
    }

    fn should_vote_for(&self, eb: &EndorserBlock, seen: Timestamp) -> Result<(), NoVoteReason> {
        let eb_must_be_received_by = Timestamp::from_secs(eb.slot)
            + (self.sim_config.header_diffusion_time * 3)
            + Duration::from_secs(self.sim_config.linear_vote_stage_length);
        if seen > eb_must_be_received_by {
            // An EB must be received within L_vote slots of its creation.
            return Err(NoVoteReason::LateEB);
        }
        let Some((rb, header_seen)) = self.latest_rb() else {
            // We only vote for whichever EB we was referenced by the head of the current chain.
            return Err(NoVoteReason::WrongEB);
        };
        if rb.header.eb_announcement != Some(eb.id()) {
            // We only vote for whichever EB we was referenced by the head of the current chain.
            return Err(NoVoteReason::WrongEB);
        }
        let rb_header_must_be_received_by =
            Timestamp::from_secs(eb.slot) + self.sim_config.header_diffusion_time;
        if header_seen >= rb_header_must_be_received_by {
            // The RB header must be received more quickly
            return Err(NoVoteReason::LateRBHeader);
        }

        if self.sim_config.variant == LeiosVariant::LinearWithTxReferences {
            for tx in &eb.txs {
                if !self.has_tx(tx.id) {
                    // We won't vote for an EB if we don't have all the TXs it references
                    // NB: this should be redundant; in this variant, we wait for TXs before validating
                    return Err(NoVoteReason::MissingTX);
                }
            }
        }
        Ok(())
    }

    fn finish_generating_vote_bundle(&mut self, votes: VoteBundle) {
        self.tracker.track_votes_generated(&votes);
        self.count_votes(&votes);
        let id = votes.id;
        let votes = Arc::new(votes);
        self.leios
            .votes
            .insert(votes.id, VoteBundleView::Received { votes });
        for peer in &self.consumers {
            self.queued.send_to(*peer, Message::AnnounceVotes(id));
        }
    }

    fn receive_announce_votes(&mut self, from: NodeId, id: VoteBundleId) {
        let should_request = match self.leios.votes.get(&id) {
            None => true,
            Some(VoteBundleView::Requested) => {
                self.sim_config.relay_strategy == RelayStrategy::RequestFromAll
            }
            _ => false,
        };
        if should_request {
            self.leios.votes.insert(id, VoteBundleView::Requested);
            self.queued.send_to(from, Message::RequestVotes(id));
        }
    }

    fn receive_request_votes(&mut self, from: NodeId, id: VoteBundleId) {
        if let Some(VoteBundleView::Received { votes }) = self.leios.votes.get(&id) {
            self.tracker.track_votes_sent(votes, self.id, from);
            self.queued.send_to(from, Message::Votes(votes.clone()));
        }
    }

    fn receive_votes(&mut self, from: NodeId, votes: Arc<VoteBundle>) {
        self.tracker.track_votes_received(&votes, from, self.id);
        self.queued
            .schedule_cpu_task(CpuTask::VTBundleValidated(from, votes));
    }

    fn finish_validating_vote_bundle(&mut self, from: NodeId, votes: Arc<VoteBundle>) {
        let id = votes.id;
        if self
            .leios
            .votes
            .insert(
                id,
                VoteBundleView::Received {
                    votes: votes.clone(),
                },
            )
            .is_some_and(|v| matches!(v, VoteBundleView::Received { .. }))
        {
            return;
        }
        self.count_votes(&votes);
        for peer in &self.consumers {
            if *peer == from {
                continue;
            }
            self.queued.send_to(*peer, Message::AnnounceVotes(id));
        }
    }

    fn count_votes(&mut self, votes: &VoteBundle) {
        let vote_threshold = self.sim_config.vote_threshold as usize;
        for (eb_id, count) in votes.ebs.iter() {
            let all_eb_votes = self.leios.votes_by_eb.entry(*eb_id).or_default();
            let total_votes_before = all_eb_votes.values().sum::<usize>();
            *all_eb_votes.entry(votes.id.producer).or_default() += count;

            let total_votes_after = total_votes_before + count;
            if total_votes_before < vote_threshold && total_votes_after >= vote_threshold {
                // this EB is officially certified
                self.leios.certified_ebs.insert(*eb_id);
            }
        }
    }
}

// Ledger/mempool operations
impl LinearLeiosNode {
    fn tx_delay_for_block_kind(&self, tx: &Transaction, block_kind: BlockKind) -> Option<u64> {
        let has_separate_eb_pool = self
            .pricing
            .as_ref()
            .is_some_and(|pricing| pricing.has_separate_eb_pool());
        match block_kind {
            BlockKind::RankingBlock => tx.tier_delay_slots,
            BlockKind::EndorserBlock => {
                if has_separate_eb_pool {
                    tx.eb_tier_delay_slots.or(tx.tier_delay_slots)
                } else {
                    tx.tier_delay_slots
                }
            }
        }
    }

    fn tx_target_tick_for_block_kind(&self, tx: &Transaction, block_kind: BlockKind) -> u64 {
        if !self.sim_config.enforce_tier_delay() {
            return 0;
        }
        let Some(delay) = self.tx_delay_for_block_kind(tx, block_kind) else {
            return 0;
        };
        match self.sim_config.tier_delay_unit() {
            TierDelayUnit::Slots => tx
                .mempool_entry_slot
                .map(|entry| entry.saturating_add(delay.max(1)))
                .unwrap_or(0),
            TierDelayUnit::Blocks => tx
                .mempool_entry_rb_index
                .map(|entry| entry.saturating_add(delay.max(1)))
                .unwrap_or(0),
        }
    }

    fn is_tx_mature_for_block(
        &self,
        tx: &Transaction,
        slot: u64,
        candidate_rb_index: u64,
        block_kind: BlockKind,
    ) -> bool {
        if !self.sim_config.enforce_tier_delay() {
            return true;
        }
        let Some(pricing) = &self.pricing else {
            return true;
        };
        if !pricing.is_tiered() {
            return true;
        }

        let Some(delay) = self.tx_delay_for_block_kind(tx, block_kind) else {
            return true;
        };
        if !Self::has_complete_mempool_entry_anchor(tx) {
            return false;
        }

        match self.sim_config.tier_delay_unit() {
            TierDelayUnit::Slots => tx
                .mempool_entry_slot
                .is_some_and(|entry| slot >= entry.saturating_add(delay.max(1))),
            TierDelayUnit::Blocks => tx
                .mempool_entry_rb_index
                .is_some_and(|entry| candidate_rb_index >= entry.saturating_add(delay.max(1))),
        }
    }

    fn replenish_mempool_from_queue(&mut self) {
        loop {
            match self.mempool.queued_front_activation_decision() {
                QueueFrontActivationDecision::BlockedByCapacity => return,
                QueueFrontActivationDecision::DropConflict => {
                    self.mempool.discard_queued_front();
                }
                QueueFrontActivationDecision::TryActivate => {
                    let Some(candidate) = self.mempool.queued_front().cloned() else {
                        return;
                    };
                    match self.activate_tx_in_mempool(candidate) {
                        ActiveMempoolAdmission::Active(tx) => {
                            self.txs
                                .insert(tx.id, TransactionView::Received(tx.clone()));
                            self.mempool.activate_queued_front(tx.clone());
                            for peer in &self.consumers {
                                self.queued.send_to(*peer, Message::AnnounceTx(tx.id));
                            }
                        }
                        ActiveMempoolAdmission::Queued(tx) => {
                            self.txs
                                .insert(tx.id, TransactionView::Received(tx.clone()));
                            return;
                        }
                        ActiveMempoolAdmission::Conflict(tx) => {
                            self.mempool.discard_queued_front();
                            self.txs.insert(tx.id, TransactionView::Received(tx));
                        }
                        ActiveMempoolAdmission::Rejected {
                            tx,
                            reason,
                            overflow_lane,
                        } => {
                            self.mempool.discard_queued_front();
                            let scheduled_retry = reason
                                == TransactionRejectReason::TierBacklogFull
                                && self.local_actor_submission_ids.contains(&tx.id)
                                && overflow_lane
                                    .is_some_and(|lane| self.schedule_overflow_retry(&tx, lane));
                            if reason == TransactionRejectReason::TierBacklogFull
                                && let Some(lane) = overflow_lane
                                && let Some((tier, pending_bytes, tier_capacity_bytes)) =
                                    self.overflow_state_for_lane(&tx, lane)
                            {
                                if self.pricing.as_ref().is_some_and(|pricing| {
                                    pricing.include_overflow_aggregate_in_pricing_updates()
                                }) {
                                    self.record_overflow_rejected_bytes(
                                        lane, tier, tx.id, tx.bytes,
                                    );
                                }
                                self.tracker.track_transaction_overflow_rejected(
                                    tx.id,
                                    self.id,
                                    lane,
                                    tier,
                                    pending_bytes,
                                    tier_capacity_bytes,
                                    scheduled_retry,
                                );
                            }

                            if scheduled_retry {
                                self.txs.remove(&tx.id);
                            } else {
                                self.txs
                                    .insert(tx.id, TransactionView::Received(tx.clone()));
                                self.tracker
                                    .track_transaction_rejected(tx.id, self.id, reason);
                                self.overflow_retry_attempts.remove(&tx.id);
                            }
                        }
                    }
                }
            }
        }
    }

    fn try_add_tx_to_mempool(&mut self, tx: Arc<Transaction>) -> ActiveMempoolAdmission {
        let ledger_state = self.resolve_ledger_state(self.latest_rb_id());
        if ledger_state.spent_inputs.contains(&tx.input_id) {
            // This TX conflicts with something already on-chain
            return ActiveMempoolAdmission::Conflict(tx);
        }

        match self.mempool.classify_new_arrival(&tx) {
            NewArrivalDisposition::Conflict => ActiveMempoolAdmission::Conflict(tx),
            NewArrivalDisposition::Queue => {
                self.mempool.insert_queued(tx.clone());
                ActiveMempoolAdmission::Queued(tx)
            }
            NewArrivalDisposition::TryActivate => {
                let admitted = self.activate_tx_in_mempool(tx);
                if let ActiveMempoolAdmission::Active(tx) = &admitted {
                    self.mempool.insert_new_active(tx.clone());
                }
                admitted
            }
        }
    }

    fn sample_from_mempool(
        &mut self,
        txs: &mut Vec<Arc<Transaction>>,
        max_size: u64,
        remove: bool,
        slot: u64,
        candidate_rb_index: u64,
        block_kind: BlockKind,
    ) {
        if let Some(pricing) = &self.pricing {
            let prefill_bytes = txs.iter().map(|tx| tx.bytes).sum::<u64>();
            let remaining_capacity = max_size.saturating_sub(prefill_bytes);
            if remaining_capacity == 0 {
                return;
            }
            let candidates: Vec<_> = self
                .mempool
                .transactions()
                .filter(|tx| self.is_tx_mature_for_block(tx, slot, candidate_rb_index, block_kind))
                .cloned()
                .collect();
            let selected = pricing.select_transactions_for_block(
                &candidates,
                slot,
                remaining_capacity,
                block_kind,
            );
            txs.extend(selected.iter().cloned());

            if remove {
                let removed_ids = selected.iter().map(|tx| tx.id);
                self.mempool.remove_txs(removed_ids);
                self.replenish_mempool_from_queue();
            }
            return;
        }

        let mut size = txs.iter().map(|tx| tx.bytes).sum::<u64>();
        let mut candidates: Vec<_> = self.mempool.ids().collect();
        if matches!(
            self.sim_config.mempool_strategy,
            MempoolSamplingStrategy::Random
        ) {
            candidates.shuffle(&mut self.rng);
        } else {
            candidates.reverse();
        }

        // Fill with as many pending transactions as can fit
        let mut removed_ids = vec![];
        while let Some(id) = candidates.pop() {
            let Some(TransactionView::Received(tx)) = self.txs.get(&id) else {
                panic!("missing a TX in our mempool");
            };
            if !self.is_tx_mature_for_block(tx, slot, candidate_rb_index, block_kind) {
                continue;
            }
            if size + tx.bytes > max_size {
                break;
            }
            size += tx.bytes;
            txs.push(tx.clone());
            if remove {
                removed_ids.push(tx.id);
            }
        }
        self.mempool.remove_txs(removed_ids);
        self.replenish_mempool_from_queue();
    }

    fn remove_rb_txs_from_mempool(&mut self, rb: &RankingBlock) {
        let mut txs = rb.transactions.clone();
        if let Some(endorsement) = &rb.endorsement
            && let Some(EndorserBlockView::Received { eb, .. }) =
                self.leios.ebs.get(&endorsement.eb)
        {
            txs.extend(eb.txs.iter().cloned());
        }
        self.remove_txs_from_mempool(&txs);
    }

    fn remove_eb_txs_from_mempool(&mut self, eb: &EndorserBlock) {
        self.remove_txs_from_mempool(&eb.txs);
    }

    fn remove_txs_from_mempool(&mut self, txs: &[Arc<Transaction>]) {
        let inputs = txs.iter().map(|tx| tx.input_id).collect::<HashSet<_>>();
        self.mempool.remove_conflicting_txs(&inputs);
        self.replenish_mempool_from_queue();
    }

    fn resolve_ledger_state(&mut self, rb_ref: Option<BlockId>) -> Arc<LedgerState> {
        let Some(block_id) = rb_ref else {
            return Arc::new(LedgerState::default());
        };
        if let Some(state) = self.ledger_states.get(&block_id) {
            return state.clone();
        };

        let mut state = self
            .ledger_states
            .last_key_value()
            .map(|(_, v)| v.as_ref().clone())
            .unwrap_or_default();

        let mut block_queue = vec![block_id];
        let mut complete = true;
        while let Some(block_id) = block_queue.pop() {
            if !state.seen_blocks.insert(block_id) {
                continue;
            }
            let Some(RankingBlockView::Received { rb, .. }) = self.praos.blocks.get(&block_id)
            else {
                continue;
            };
            if let Some(parent) = rb.header.parent {
                block_queue.push(parent);
            }
            for tx in &rb.transactions {
                state.spent_inputs.insert(tx.input_id);
            }

            if let Some(endorsement) = &rb.endorsement {
                match self.leios.ebs.get(&endorsement.eb) {
                    Some(EndorserBlockView::Received { eb, .. }) => {
                        for tx in &eb.txs {
                            if self.has_tx(tx.id) {
                                state.spent_inputs.insert(tx.input_id);
                            } else {
                                complete = false;
                            }
                        }
                    }
                    _ => {
                        // We haven't validated the EB yet, so we don't know the full ledger state
                        complete = false;
                    }
                }
            }
        }

        let state = Arc::new(state);
        if complete {
            self.ledger_states.insert(block_id, state.clone());
        }
        state
    }

    fn block_capacity_for_kind(&self, block_kind: BlockKind) -> u64 {
        match block_kind {
            BlockKind::RankingBlock => self.sim_config.max_block_size,
            BlockKind::EndorserBlock => self.sim_config.max_eb_size,
        }
    }

    fn shared_single_pool_rb_only_pricing_mode(&self) -> bool {
        self.pricing
            .as_ref()
            .is_some_and(|pricing| pricing.uses_shared_single_pool_tiers())
            && !self.sim_config.rb_inline_txs_with_endorsement
    }

    fn shared_single_pool_pricing_signal_for_rb(
        &self,
        rb: &RankingBlock,
    ) -> Option<(Vec<Arc<Transaction>>, u64)> {
        if !self.shared_single_pool_rb_only_pricing_mode() {
            return None;
        }
        if let Some(endorsement) = &rb.endorsement {
            let EndorserBlockView::Received { eb, .. } = self.leios.ebs.get(&endorsement.eb)?
            else {
                return None;
            };
            return Some((eb.txs.clone(), self.sim_config.max_eb_size));
        }
        Some((rb.transactions.clone(), self.sim_config.max_block_size))
    }

    fn maybe_update_shared_single_pool_pricing_for_rb(&mut self, rb: &RankingBlock) {
        let Some((pricing_txs, block_capacity)) = self.shared_single_pool_pricing_signal_for_rb(rb)
        else {
            return;
        };
        self.update_pricing_after_block(
            PricingUpdateKey::RankingBlock(rb.header.id),
            &pricing_txs,
            block_capacity,
            BlockKind::RankingBlock,
            rb.header.id.slot,
        );
    }

    fn rbs_endorsing_eb(&self, eb_id: EndorserBlockId) -> Vec<Arc<RankingBlock>> {
        self.praos
            .blocks
            .values()
            .filter_map(|view| match view {
                RankingBlockView::Received { rb, .. }
                    if rb
                        .endorsement
                        .as_ref()
                        .is_some_and(|endorsement| endorsement.eb == eb_id) =>
                {
                    Some(rb.clone())
                }
                _ => None,
            })
            .collect()
    }

    fn update_pricing_after_block(
        &mut self,
        update_key: PricingUpdateKey,
        txs: &[Arc<Transaction>],
        block_capacity: u64,
        block_kind: BlockKind,
        slot: u64,
    ) {
        let Some(pricing) = self.pricing.clone() else {
            return;
        };
        let include_overflow_in_pricing = pricing.include_overflow_aggregate_in_pricing_updates();
        let include_overflow_in_tier_updates = pricing.include_overflow_aggregate_in_tier_updates();
        let include_any_overflow_aggregate =
            include_overflow_in_pricing || include_overflow_in_tier_updates;
        let overflow_aggregate_txs = if include_any_overflow_aggregate {
            self.aggregated_overflow_txs_for_block_kind(block_kind, slot)
        } else {
            Vec::new()
        };

        let tier_update_signal_txs = if include_overflow_in_tier_updates {
            let mut signal_txs = Vec::with_capacity(txs.len() + overflow_aggregate_txs.len());
            signal_txs.extend(txs.iter().cloned());
            signal_txs.extend(overflow_aggregate_txs.iter().cloned());
            Some(signal_txs)
        } else {
            None
        };
        let overflow_pricing_signal_txs = if include_overflow_in_pricing {
            Some(overflow_aggregate_txs.as_slice())
        } else {
            None
        };

        let Some(update) = pricing.update_after_block(
            update_key,
            txs,
            tier_update_signal_txs.as_deref(),
            overflow_pricing_signal_txs,
            block_capacity,
            block_kind,
            slot,
        ) else {
            return;
        };
        if include_any_overflow_aggregate && !overflow_aggregate_txs.is_empty() {
            self.clear_aggregated_overflow_for_block_kind(block_kind);
        }

        let tiers = update
            .after
            .iter()
            .enumerate()
            .map(|(index, tier)| TierInfo {
                id: tier.id,
                capacity_bytes: tier.capacity,
                delay: tier.delay,
                price_per_byte: tier.price,
                utilisation: update.utilisations.get(index).copied().unwrap_or(0.0),
            })
            .collect::<Vec<_>>();
        self.tracker.track_tier_prices_updated(
            self.id,
            block_kind,
            slot,
            update.cadence.delay_update_triggered,
            update.cadence.tier_update_triggered,
            tiers.clone(),
        );

        let before_ids: std::collections::HashSet<_> =
            update.before.iter().map(|tier| tier.id).collect();
        let after_ids: std::collections::HashSet<_> =
            update.after.iter().map(|tier| tier.id).collect();

        for tier in update.after.iter() {
            if !before_ids.contains(&tier.id) {
                let info = TierInfo {
                    id: tier.id,
                    capacity_bytes: tier.capacity,
                    delay: tier.delay,
                    price_per_byte: tier.price,
                    utilisation: 0.0,
                };
                self.tracker.track_tier_created(self.id, info);
            }
        }

        for tier in update.before.iter() {
            if !after_ids.contains(&tier.id) {
                self.tracker.track_tier_removed(self.id, tier.id);
            }
        }
    }
}

// Common utilities
impl LinearLeiosNode {
    pub(super) fn set_global_pricing_coordinator(&mut self, coordinator: GlobalPricingCoordinator) {
        self.pricing = Some(coordinator);
    }

    #[allow(unused)]
    pub fn mock_lottery(&mut self, results: Arc<MockLotteryResults>) {
        self.lottery = LotteryConfig::Mock { results };
    }
    // Simulates the output of a VRF using this node's stake (if any).
    fn run_vrf(&mut self, kind: LotteryKind, success_rate: f64) -> Option<u64> {
        self.lottery.run(kind, success_rate, &mut self.rng)
    }
}

struct Mempool {
    active_size_bytes: u64,
    max_size_bytes: u64,
    enforce_tier_delay: bool,
    tier_delay_unit: TierDelayUnit,
    /// Active transactions visible to block builders, keyed by transaction ID.
    /// IndexMap preserves insertion order for deterministic iteration while giving O(1) lookups.
    active: IndexMap<TransactionId, Arc<Transaction>>,
    /// Waiting queue for transactions that arrived when the active pool was at byte capacity.
    /// These are invisible to block builders and have no tier assignment yet.
    waiting: VecDeque<Arc<Transaction>>,
    /// Input IDs of all active transactions, for conflict detection.
    input_ids: HashSet<u64>,
    rb_pending_bytes_by_tier_target: BTreeMap<(TierId, u64), u64>,
    eb_pending_bytes_by_tier_target: BTreeMap<(TierId, u64), u64>,
}

enum NewArrivalDisposition {
    TryActivate,
    Queue,
    Conflict,
}

enum QueueFrontActivationDecision {
    TryActivate,
    BlockedByCapacity,
    DropConflict,
}

enum ActiveMempoolAdmission {
    Active(Arc<Transaction>),
    Queued(Arc<Transaction>),
    Conflict(Arc<Transaction>),
    Rejected {
        tx: Arc<Transaction>,
        reason: TransactionRejectReason,
        overflow_lane: Option<BlockKind>,
    },
}

impl Mempool {
    #[cfg(test)]
    fn new(max_size_bytes: u64) -> Self {
        Self::with_delay_mode(max_size_bytes, false, TierDelayUnit::Slots)
    }

    fn with_delay_mode(
        max_size_bytes: u64,
        enforce_tier_delay: bool,
        tier_delay_unit: TierDelayUnit,
    ) -> Self {
        Self {
            active_size_bytes: 0,
            max_size_bytes,
            enforce_tier_delay,
            tier_delay_unit,
            active: IndexMap::new(),
            waiting: VecDeque::new(),
            input_ids: HashSet::new(),
            rb_pending_bytes_by_tier_target: BTreeMap::new(),
            eb_pending_bytes_by_tier_target: BTreeMap::new(),
        }
    }

    fn classify_new_arrival(&self, tx: &Transaction) -> NewArrivalDisposition {
        if self.input_ids.contains(&tx.input_id) {
            return NewArrivalDisposition::Conflict;
        }
        let new_bytes = self.active_size_bytes + tx.bytes;
        if !self.waiting.is_empty() || new_bytes > self.max_size_bytes {
            return NewArrivalDisposition::Queue;
        }
        NewArrivalDisposition::TryActivate
    }

    fn insert_new_active(&mut self, tx: Arc<Transaction>) {
        self.active_size_bytes = self.active_size_bytes.saturating_add(tx.bytes);
        self.input_ids.insert(tx.input_id);
        self.add_pending_tier_bytes(&tx);
        self.active.insert(tx.id, tx);
    }

    fn insert_queued(&mut self, tx: Arc<Transaction>) {
        self.waiting.push_back(tx);
    }

    fn queued_front(&self) -> Option<&Arc<Transaction>> {
        self.waiting.front()
    }

    fn queued_front_activation_decision(&self) -> QueueFrontActivationDecision {
        let Some(tx) = self.waiting.front() else {
            return QueueFrontActivationDecision::BlockedByCapacity;
        };
        if self.input_ids.contains(&tx.input_id) {
            return QueueFrontActivationDecision::DropConflict;
        }
        if self.active_size_bytes.saturating_add(tx.bytes) > self.max_size_bytes {
            return QueueFrontActivationDecision::BlockedByCapacity;
        }
        QueueFrontActivationDecision::TryActivate
    }

    fn discard_queued_front(&mut self) -> Option<Arc<Transaction>> {
        self.waiting.pop_front()
    }

    fn activate_queued_front(&mut self, tx: Arc<Transaction>) {
        let popped = self
            .waiting
            .pop_front()
            .expect("cannot activate queued front without a queued transaction");
        debug_assert_eq!(popped.id, tx.id);
        self.active_size_bytes = self.active_size_bytes.saturating_add(tx.bytes);
        self.input_ids.insert(tx.input_id);
        self.add_pending_tier_bytes(&tx);
        self.active.insert(tx.id, tx);
    }

    fn ids(&self) -> impl Iterator<Item = TransactionId> + '_ {
        self.active.keys().copied()
    }

    fn transactions(&self) -> impl Iterator<Item = &Arc<Transaction>> {
        self.active.values()
    }

    fn remove_txs(&mut self, ids: impl IntoIterator<Item = TransactionId>) {
        let id_set: HashSet<TransactionId> = ids.into_iter().collect();
        for id in &id_set {
            if let Some(tx) = self.active.shift_remove(id) {
                self.active_size_bytes = self.active_size_bytes.saturating_sub(tx.bytes);
                self.input_ids.remove(&tx.input_id);
                remove_pending_tier_bytes_from_maps(
                    &mut self.rb_pending_bytes_by_tier_target,
                    &mut self.eb_pending_bytes_by_tier_target,
                    &tx,
                    self.enforce_tier_delay,
                    self.tier_delay_unit,
                );
            }
        }
        self.waiting.retain(|tx| !id_set.contains(&tx.id));
    }

    fn remove_conflicting_txs(&mut self, conflicting_input_ids: &HashSet<u64>) {
        self.active.retain(|_, tx| {
            if !conflicting_input_ids.contains(&tx.input_id) {
                return true;
            }
            self.active_size_bytes = self.active_size_bytes.saturating_sub(tx.bytes);
            self.input_ids.remove(&tx.input_id);
            remove_pending_tier_bytes_from_maps(
                &mut self.rb_pending_bytes_by_tier_target,
                &mut self.eb_pending_bytes_by_tier_target,
                tx,
                self.enforce_tier_delay,
                self.tier_delay_unit,
            );
            false
        });
        self.waiting.retain(|tx| {
            !self.input_ids.contains(&tx.input_id) && !conflicting_input_ids.contains(&tx.input_id)
        });
    }

    fn pending_bytes_map(
        &self,
        block_kind: BlockKind,
        has_separate_eb_pool: bool,
    ) -> &BTreeMap<(TierId, u64), u64> {
        match block_kind {
            BlockKind::RankingBlock => &self.rb_pending_bytes_by_tier_target,
            BlockKind::EndorserBlock if has_separate_eb_pool => {
                &self.eb_pending_bytes_by_tier_target
            }
            BlockKind::EndorserBlock => &self.rb_pending_bytes_by_tier_target,
        }
    }

    #[cfg(test)]
    fn pending_bytes_for_tier(
        &self,
        block_kind: BlockKind,
        tier: TierId,
        has_separate_eb_pool: bool,
    ) -> u64 {
        self.pending_bytes_map(block_kind, has_separate_eb_pool)
            .iter()
            .filter_map(|((entry_tier, _), bytes)| (*entry_tier == tier).then_some(*bytes))
            .sum()
    }

    fn pending_bytes_for_tier_target(
        &self,
        block_kind: BlockKind,
        tier: TierId,
        target_tick: u64,
        has_separate_eb_pool: bool,
    ) -> u64 {
        self.pending_bytes_map(block_kind, has_separate_eb_pool)
            .get(&(tier, target_tick))
            .copied()
            .unwrap_or(0)
    }

    fn add_pending_tier_bytes(&mut self, tx: &Transaction) {
        add_pending_tier_bytes_to_maps(
            &mut self.rb_pending_bytes_by_tier_target,
            &mut self.eb_pending_bytes_by_tier_target,
            tx,
            self.enforce_tier_delay,
            self.tier_delay_unit,
        );
    }
}

fn add_pending_tier_bytes_to_maps(
    rb_by_tier_target: &mut BTreeMap<(TierId, u64), u64>,
    eb_by_tier_target: &mut BTreeMap<(TierId, u64), u64>,
    tx: &Transaction,
    enforce_tier_delay: bool,
    tier_delay_unit: TierDelayUnit,
) {
    if let Some(tier) = tx.tier_preference {
        let target_tick = tx_target_tick_for_pending_assignment(
            tx,
            BlockKind::RankingBlock,
            enforce_tier_delay,
            tier_delay_unit,
        );
        let entry = rb_by_tier_target.entry((tier, target_tick)).or_default();
        *entry = entry.saturating_add(tx.bytes);
    }
    if let Some(tier) = tx.eb_tier_preference {
        let target_tick = tx_target_tick_for_pending_assignment(
            tx,
            BlockKind::EndorserBlock,
            enforce_tier_delay,
            tier_delay_unit,
        );
        let entry = eb_by_tier_target.entry((tier, target_tick)).or_default();
        *entry = entry.saturating_add(tx.bytes);
    }
}

fn remove_pending_tier_bytes_from_maps(
    rb_by_tier_target: &mut BTreeMap<(TierId, u64), u64>,
    eb_by_tier_target: &mut BTreeMap<(TierId, u64), u64>,
    tx: &Transaction,
    enforce_tier_delay: bool,
    tier_delay_unit: TierDelayUnit,
) {
    if let Some(tier) = tx.tier_preference {
        let target_tick = tx_target_tick_for_pending_assignment(
            tx,
            BlockKind::RankingBlock,
            enforce_tier_delay,
            tier_delay_unit,
        );
        subtract_pending_tier_bytes(rb_by_tier_target, tier, target_tick, tx.bytes);
    }
    if let Some(tier) = tx.eb_tier_preference {
        let target_tick = tx_target_tick_for_pending_assignment(
            tx,
            BlockKind::EndorserBlock,
            enforce_tier_delay,
            tier_delay_unit,
        );
        subtract_pending_tier_bytes(eb_by_tier_target, tier, target_tick, tx.bytes);
    }
}

fn tx_target_tick_for_pending_assignment(
    tx: &Transaction,
    block_kind: BlockKind,
    enforce_tier_delay: bool,
    tier_delay_unit: TierDelayUnit,
) -> u64 {
    if !enforce_tier_delay {
        return 0;
    }
    let delay = match block_kind {
        BlockKind::RankingBlock => tx.tier_delay_slots,
        BlockKind::EndorserBlock => tx.eb_tier_delay_slots.or(tx.tier_delay_slots),
    };
    let Some(delay) = delay else {
        return 0;
    };
    match tier_delay_unit {
        TierDelayUnit::Slots => tx
            .mempool_entry_slot
            .map(|entry| entry.saturating_add(delay.max(1)))
            .unwrap_or(0),
        TierDelayUnit::Blocks => tx
            .mempool_entry_rb_index
            .map(|entry| entry.saturating_add(delay.max(1)))
            .unwrap_or(0),
    }
}

fn subtract_pending_tier_bytes(
    by_tier_target: &mut BTreeMap<(TierId, u64), u64>,
    tier: TierId,
    target_tick: u64,
    bytes: u64,
) {
    let Some(current) = by_tier_target.get_mut(&(tier, target_tick)) else {
        return;
    };
    *current = current.saturating_sub(bytes);
    if *current == 0 {
        by_tier_target.remove(&(tier, target_tick));
    }
}

#[cfg(test)]
mod mempool_tests {
    use std::{collections::BTreeMap, sync::Arc};

    use rand_chacha::{ChaChaRng, rand_core::SeedableRng};
    use tokio::sync::mpsc;

    use crate::{
        clock::{MockClockCoordinator, Timestamp},
        config::{
            DistributionConfig, NodeConfiguration, RawLinkInfo, RawNode, RawParameters,
            RawTopology, SimConfiguration, TierDelayUnit,
        },
        events::EventTracker,
        model::{
            ActorId, Endorsement, LinearEndorserBlock as EndorserBlock, TierId, Transaction,
            TransactionId, UrgencyProfile,
        },
        sim::NodeImpl,
        tx_pricing::{
            BlockKind, OverflowAggregatePricingMode, OverflowRetryCurveMetric, OverflowRetryPolicy,
            PricingMechanismConfig, TierAssignmentSemantics, TierBlockSelectionPolicy,
            TierSelectionDelayModel, TieredConfig,
        },
    };

    use super::{
        EndorserBlockView, GlobalPricingCoordinator, LinearLeiosNode, Mempool,
        NewArrivalDisposition, QueueFrontActivationDecision,
    };

    fn try_insert(mempool: &mut Mempool, tx: Arc<Transaction>) -> bool {
        match mempool.classify_new_arrival(&tx) {
            NewArrivalDisposition::TryActivate => {
                mempool.insert_new_active(tx);
                true
            }
            NewArrivalDisposition::Queue => {
                mempool.insert_queued(tx);
                false
            }
            NewArrivalDisposition::Conflict => false,
        }
    }

    fn promote_queued_transactions(mempool: &mut Mempool) -> Vec<TransactionId> {
        let mut promoted = Vec::new();
        loop {
            match mempool.queued_front_activation_decision() {
                QueueFrontActivationDecision::TryActivate => {
                    let Some(tx) = mempool.queued_front().cloned() else {
                        break;
                    };
                    promoted.push(tx.id);
                    mempool.activate_queued_front(tx);
                }
                QueueFrontActivationDecision::DropConflict => {
                    mempool.discard_queued_front();
                }
                QueueFrontActivationDecision::BlockedByCapacity => break,
            }
        }
        promoted
    }

    fn test_tiered_config() -> TieredConfig {
        TieredConfig {
            total_capacity: 100,
            max_tiers: 4,
            tier_size_fractions: vec![0.0, 0.2, 0.2, 0.2],
            base_fee_change_denominator: 8,
            target_utilisation: 0.5,
            delay_update_frequency: Some(1),
            delay_update_period_slots: None,
            delay_increase_threshold: 1.5,
            delay_increase_thresholds: vec![],
            delay_decrease_prob: 0.0,
            min_delay_ratio: 2.0,
            min_delay_ratios: vec![],
            tier_update_frequency: Some(10),
            tier_update_period_slots: None,
            add_tier_threshold: 5_000,
            remove_tier_threshold: 100,
            new_tier_price: 1_000,
            new_tier_delay_ratio: 2.0,
            block_selection_policy: TierBlockSelectionPolicy::Shared,
            rb_tier0_reservation_fraction: 1.0,
            separate_eb_pool: false,
            eb_total_capacity: None,
            assignment_semantics: TierAssignmentSemantics::NeverStale,
            reject_on_pending_tier_overflow: true,
            include_overflow_aggregate_in_pricing_updates: false,
            overflow_aggregate_pricing_mode: OverflowAggregatePricingMode::IncludeAsFillRate,
            overflow_linear_price_per_fill: 100,
            overflow_linear_fill_rate_cap: 1.0,
            dynamic_tier_sizing_enabled: false,
            dynamic_tier_sizing_alpha: 1.0,
            dynamic_tier_sizing_min_fraction: 0.02,
            enforce_boundary_price_caps: false,
            include_overflow_aggregate_in_tier_updates: false,
            add_tier_fill_rate_threshold: 1.0,
            remove_tier_fill_rate_threshold: 0.2,
            overflow_retry_policy: OverflowRetryPolicy::default(),
        }
    }

    fn test_node_with_tiered_pricing_config_with_block_sizes(
        tier_delay_unit: TierDelayUnit,
        tiered_config: TieredConfig,
        rb_body_max_size_bytes: u64,
        eb_referenced_txs_max_size_bytes: u64,
    ) -> LinearLeiosNode {
        let mut params: RawParameters =
            serde_yaml::from_slice(include_bytes!("../../../parameters/config.default.yaml"))
                .unwrap();
        params.leios_variant = crate::config::LeiosVariant::LinearWithTxReferences;
        params.tx_size_bytes_distribution = DistributionConfig::Constant { value: 10.0 };
        params.tx_max_size_bytes = 10;
        params.enforce_tier_delay = true;
        params.tier_delay_unit = tier_delay_unit;
        params.rb_body_max_size_bytes = rb_body_max_size_bytes;
        params.eb_referenced_txs_max_size_bytes = eb_referenced_txs_max_size_bytes;

        let topology = RawTopology {
            nodes: BTreeMap::from([(
                "node-1".to_string(),
                RawNode {
                    stake: Some(1_000),
                    location: crate::config::RawNodeLocation::Cluster {
                        cluster: "all".into(),
                    },
                    cpu_core_count: Some(1),
                    tx_conflict_fraction: None,
                    tx_generation_weight: None,
                    producers: BTreeMap::from([(
                        "node-1".to_string(),
                        RawLinkInfo {
                            latency_ms: 0.0,
                            bandwidth_bytes_per_second: None,
                        },
                    )]),
                    adversarial: None,
                    behaviours: vec![],
                },
            )]),
        };
        let sim_config = Arc::new(SimConfiguration::build(params, topology.into()).unwrap());
        let node_config: &NodeConfiguration = &sim_config.nodes[0];
        let clock = MockClockCoordinator::new();
        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let tracker = EventTracker::new(event_tx, clock.clock(), &sim_config.nodes);
        let rng = ChaChaRng::seed_from_u64(sim_config.seed);
        let mut node =
            LinearLeiosNode::new(node_config, sim_config.clone(), tracker, rng, clock.clock());
        let pricing = PricingMechanismConfig::TieredPricing { tiered_config };
        let coordinator = GlobalPricingCoordinator::new(
            &pricing,
            sim_config.seed,
            sim_config.tier_selection_path_latencies(),
        );
        node.set_global_pricing_coordinator(coordinator);
        node
    }

    fn test_node_with_tiered_pricing_config(
        tier_delay_unit: TierDelayUnit,
        tiered_config: TieredConfig,
    ) -> LinearLeiosNode {
        test_node_with_tiered_pricing_config_with_block_sizes(
            tier_delay_unit,
            tiered_config.clone(),
            tiered_config.total_capacity,
            tiered_config.total_capacity,
        )
    }

    fn test_node_with_tiered_pricing(tier_delay_unit: TierDelayUnit) -> LinearLeiosNode {
        test_node_with_tiered_pricing_config(tier_delay_unit, test_tiered_config())
    }

    struct TxFactory {
        next_id: u64,
    }
    impl TxFactory {
        fn new() -> Self {
            Self { next_id: 0 }
        }
        fn tx(&mut self, bytes: u64) -> Arc<Transaction> {
            self.tx_with_tiers(bytes, None, None)
        }
        fn tx_with_tiers(
            &mut self,
            bytes: u64,
            tier_preference: Option<TierId>,
            eb_tier_preference: Option<TierId>,
        ) -> Arc<Transaction> {
            let id = self.next_id;
            self.next_id += 1;
            Arc::new(Transaction {
                id: TransactionId::new(id),
                actor_id: ActorId::new(0),
                shard: 0,
                bytes,
                submission_slot: 0,
                value: 0,
                urgency: UrgencyProfile::Indifferent,
                posted_fee: None,
                tier_preference,
                tier_version_created_slot: None,
                tier_delay_slots: None,
                tier_price_per_byte_at_assignment: None,
                eb_tier_preference,
                eb_tier_version_created_slot: None,
                eb_posted_fee: None,
                eb_tier_delay_slots: None,
                eb_tier_price_per_byte_at_assignment: None,
                assigned_block_kind: None,
                mempool_entry_slot: None,
                mempool_entry_rb_index: None,
                input_id: id,
                overcollateralization_factor: 0,
                urgency_component_index: None,
            })
        }
        fn txs<const N: usize>(&mut self, bytes: [u64; N]) -> [Arc<Transaction>; N] {
            bytes.map(|b| self.tx(b))
        }

        fn anchored_rb_tx(
            &mut self,
            bytes: u64,
            tier: TierId,
            delay: u64,
            entry_slot: u64,
            entry_rb_index: u64,
        ) -> Arc<Transaction> {
            let id = self.next_id;
            self.next_id += 1;
            Arc::new(Transaction {
                id: TransactionId::new(id),
                actor_id: ActorId::new(0),
                shard: 0,
                bytes,
                submission_slot: entry_slot,
                value: 0,
                urgency: UrgencyProfile::Indifferent,
                posted_fee: Some(bytes.saturating_mul(10)),
                tier_preference: Some(tier),
                tier_version_created_slot: Some(entry_slot),
                tier_delay_slots: Some(delay),
                tier_price_per_byte_at_assignment: Some(10),
                eb_tier_preference: None,
                eb_tier_version_created_slot: None,
                eb_posted_fee: None,
                eb_tier_delay_slots: None,
                eb_tier_price_per_byte_at_assignment: None,
                assigned_block_kind: Some(BlockKind::RankingBlock),
                mempool_entry_slot: Some(entry_slot),
                mempool_entry_rb_index: Some(entry_rb_index),
                input_id: id,
                overcollateralization_factor: 0,
                urgency_component_index: None,
            })
        }
    }

    #[test]
    fn should_fill_as_space_is_available() {
        let mut txs = TxFactory::new();
        let [tx1, tx2, tx3] = txs.txs([5, 5, 5]);
        let mut mempool = Mempool::new(10);
        assert!(try_insert(&mut mempool, tx1.clone()));
        assert!(try_insert(&mut mempool, tx2.clone()));

        // new TX doesn't fit
        assert!(!try_insert(&mut mempool, tx3.clone()));
        assert_eq!(mempool.ids().collect::<Vec<_>>(), vec![tx1.id, tx2.id]);

        // until we remove a TX, and suddenly it does
        mempool.remove_txs([tx2.id]);
        let added = promote_queued_transactions(&mut mempool);
        assert_eq!(added, vec![tx3.id]);
        assert_eq!(mempool.ids().collect::<Vec<_>>(), vec![tx1.id, tx3.id]);
    }

    #[test]
    fn tracks_pending_bytes_by_lane_and_tier() {
        let mut txs = TxFactory::new();
        let tx1 = txs.tx_with_tiers(5, Some(TierId::new(0)), Some(TierId::new(1)));
        let tx2 = txs.tx_with_tiers(7, Some(TierId::new(1)), Some(TierId::new(2)));
        let mut mempool = Mempool::new(20);
        assert!(try_insert(&mut mempool, tx1.clone()));
        assert!(try_insert(&mut mempool, tx2.clone()));

        assert_eq!(
            mempool.pending_bytes_for_tier(BlockKind::RankingBlock, TierId::new(0), true),
            5
        );
        assert_eq!(
            mempool.pending_bytes_for_tier(BlockKind::RankingBlock, TierId::new(1), true),
            7
        );
        assert_eq!(
            mempool.pending_bytes_for_tier(BlockKind::EndorserBlock, TierId::new(1), true),
            5
        );
        assert_eq!(
            mempool.pending_bytes_for_tier(BlockKind::EndorserBlock, TierId::new(2), true),
            7
        );

        mempool.remove_txs([tx2.id]);

        assert_eq!(
            mempool.pending_bytes_for_tier(BlockKind::RankingBlock, TierId::new(1), true),
            0
        );
        assert_eq!(
            mempool.pending_bytes_for_tier(BlockKind::EndorserBlock, TierId::new(2), true),
            0
        );
    }

    #[test]
    fn queued_transactions_do_not_count_until_promoted() {
        let mut txs = TxFactory::new();
        let tx1 = txs.tx_with_tiers(6, Some(TierId::new(1)), None);
        let tx2 = txs.tx_with_tiers(6, Some(TierId::new(1)), None);
        let mut mempool = Mempool::new(10);
        assert!(try_insert(&mut mempool, tx1.clone()));
        assert!(!try_insert(&mut mempool, tx2.clone()));

        assert_eq!(
            mempool.pending_bytes_for_tier(BlockKind::RankingBlock, TierId::new(1), false),
            6
        );

        mempool.remove_txs([tx1.id]);
        let promoted = promote_queued_transactions(&mut mempool);
        assert_eq!(promoted, vec![tx2.id]);
        assert_eq!(
            mempool.pending_bytes_for_tier(BlockKind::RankingBlock, TierId::new(1), false),
            6
        );
    }

    #[test]
    fn pending_bytes_are_bucketed_by_maturity_target() {
        let mut txs = TxFactory::new();
        let tx_next = txs.anchored_rb_tx(6, TierId::new(1), 1, 10, 5);
        let tx_later = txs.anchored_rb_tx(7, TierId::new(1), 2, 10, 5);
        let mut mempool = Mempool::with_delay_mode(20, true, TierDelayUnit::Blocks);

        mempool.insert_new_active(tx_next);
        mempool.insert_new_active(tx_later);

        assert_eq!(
            mempool.pending_bytes_for_tier_target(
                BlockKind::RankingBlock,
                TierId::new(1),
                6,
                false,
            ),
            6
        );
        assert_eq!(
            mempool.pending_bytes_for_tier_target(
                BlockKind::RankingBlock,
                TierId::new(1),
                7,
                false,
            ),
            7
        );
    }

    #[test]
    fn overflow_rejection_requires_all_assigned_lanes_to_be_overfull() {
        assert!(!super::LinearLeiosNode::should_reject_on_overflow(
            true, true, true, false
        ));
        assert!(!super::LinearLeiosNode::should_reject_on_overflow(
            true, false, true, true
        ));
        assert!(super::LinearLeiosNode::should_reject_on_overflow(
            true, true, true, true
        ));

        assert!(super::LinearLeiosNode::should_reject_on_overflow(
            true, true, false, false
        ));
        assert!(!super::LinearLeiosNode::should_reject_on_overflow(
            true, false, false, false
        ));

        assert!(super::LinearLeiosNode::should_reject_on_overflow(
            false, false, true, true
        ));
        assert!(!super::LinearLeiosNode::should_reject_on_overflow(
            false, false, true, false
        ));
    }

    #[test]
    fn retained_value_ratio_uses_lane_delay_model_units() {
        let tx = Transaction {
            id: TransactionId::new(9),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 0,
            value: 100,
            urgency: UrgencyProfile::TimeBoxed { max_slots: 2 },
            posted_fee: None,
            tier_preference: Some(TierId::new(0)),
            tier_version_created_slot: None,
            tier_delay_slots: Some(1),
            tier_price_per_byte_at_assignment: None,
            eb_tier_preference: Some(TierId::new(1)),
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: Some(1),
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: None,
            mempool_entry_slot: None,
            mempool_entry_rb_index: None,
            input_id: 9,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        };
        let delay_model = TierSelectionDelayModel::LanePathPlusTierDelay {
            rb_path_latency: 1,
            eb_path_latency: 3,
        };

        let rb_ratio = super::LinearLeiosNode::retained_value_ratio_for_lane(
            &tx,
            BlockKind::RankingBlock,
            OverflowRetryCurveMetric::RetainedValueRatio,
            delay_model,
        );
        let eb_ratio = super::LinearLeiosNode::retained_value_ratio_for_lane(
            &tx,
            BlockKind::EndorserBlock,
            OverflowRetryCurveMetric::RetainedValueRatio,
            delay_model,
        );

        assert_eq!(rb_ratio, 1.0);
        assert_eq!(eb_ratio, 0.0);
    }

    #[test]
    fn slot_delay_requires_waiting_one_full_slot_before_inclusion() {
        let node = test_node_with_tiered_pricing(TierDelayUnit::Slots);
        let tx = Transaction {
            id: TransactionId::new(21),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 10,
            value: 100,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(100),
            tier_preference: Some(TierId::new(0)),
            tier_version_created_slot: Some(10),
            tier_delay_slots: Some(1),
            tier_price_per_byte_at_assignment: Some(10),
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: Some(BlockKind::RankingBlock),
            mempool_entry_slot: Some(10),
            mempool_entry_rb_index: Some(5),
            input_id: 21,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        };

        assert!(!node.is_tx_mature_for_block(&tx, 10, 5, BlockKind::RankingBlock));
        assert!(node.is_tx_mature_for_block(&tx, 11, 5, BlockKind::RankingBlock));
    }

    #[test]
    fn block_delay_requires_waiting_one_full_rb_before_inclusion() {
        let node = test_node_with_tiered_pricing(TierDelayUnit::Blocks);
        let tx = Transaction {
            id: TransactionId::new(22),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 10,
            value: 100,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(100),
            tier_preference: Some(TierId::new(0)),
            tier_version_created_slot: Some(10),
            tier_delay_slots: Some(1),
            tier_price_per_byte_at_assignment: Some(10),
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: Some(BlockKind::RankingBlock),
            mempool_entry_slot: Some(10),
            mempool_entry_rb_index: Some(5),
            input_id: 22,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        };

        assert!(!node.is_tx_mature_for_block(&tx, 10, 5, BlockKind::RankingBlock));
        assert!(node.is_tx_mature_for_block(&tx, 10, 6, BlockKind::RankingBlock));
    }

    #[test]
    fn overflow_check_ignores_future_maturity_bucket() {
        let mut node = test_node_with_tiered_pricing(TierDelayUnit::Blocks);
        let pricing = node.pricing.clone().expect("pricing coordinator");
        let tier = TierId::new(0);
        let capacity = pricing
            .effective_tier_capacity_for_block_kind(
                BlockKind::RankingBlock,
                tier,
                node.sim_config.max_block_size,
            )
            .expect("tier capacity");

        let future_tx = Transaction {
            id: TransactionId::new(24),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: capacity,
            submission_slot: 10,
            value: 100,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(capacity.saturating_mul(10)),
            tier_preference: Some(tier),
            tier_version_created_slot: Some(10),
            tier_delay_slots: Some(2),
            tier_price_per_byte_at_assignment: Some(10),
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: Some(BlockKind::RankingBlock),
            mempool_entry_slot: Some(10),
            mempool_entry_rb_index: Some(5),
            input_id: 24,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        };
        node.mempool.insert_new_active(Arc::new(future_tx));

        let next_block_tx = Transaction {
            id: TransactionId::new(25),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 1,
            submission_slot: 10,
            value: 100,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(10),
            tier_preference: Some(tier),
            tier_version_created_slot: Some(10),
            tier_delay_slots: Some(1),
            tier_price_per_byte_at_assignment: Some(10),
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: Some(BlockKind::RankingBlock),
            mempool_entry_slot: Some(10),
            mempool_entry_rb_index: Some(5),
            input_id: 25,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        };

        assert_eq!(
            node.mempool
                .pending_bytes_for_tier_target(BlockKind::RankingBlock, tier, 6, false),
            0
        );
        assert_eq!(
            node.mempool
                .pending_bytes_for_tier_target(BlockKind::RankingBlock, tier, 7, false),
            capacity
        );
        assert_eq!(node.overfull_tier_lane(&pricing, &next_block_tx), None);
    }

    #[test]
    fn clearing_assignment_for_retry_also_clears_maturity_anchor() {
        let tx = Arc::new(Transaction {
            id: TransactionId::new(23),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 10,
            submission_slot: 0,
            value: 100,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(100),
            tier_preference: Some(TierId::new(1)),
            tier_version_created_slot: Some(7),
            tier_delay_slots: Some(2),
            tier_price_per_byte_at_assignment: Some(10),
            eb_tier_preference: Some(TierId::new(2)),
            eb_tier_version_created_slot: Some(8),
            eb_posted_fee: Some(90),
            eb_tier_delay_slots: Some(3),
            eb_tier_price_per_byte_at_assignment: Some(9),
            assigned_block_kind: Some(BlockKind::RankingBlock),
            mempool_entry_slot: Some(12),
            mempool_entry_rb_index: Some(4),
            input_id: 23,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        });

        let cleared = LinearLeiosNode::clear_tiered_assignment_and_maturity(&tx);
        assert_eq!(cleared.posted_fee, None);
        assert_eq!(cleared.tier_preference, None);
        assert_eq!(cleared.tier_version_created_slot, None);
        assert_eq!(cleared.tier_delay_slots, None);
        assert_eq!(cleared.tier_price_per_byte_at_assignment, None);
        assert_eq!(cleared.eb_tier_preference, None);
        assert_eq!(cleared.eb_tier_version_created_slot, None);
        assert_eq!(cleared.eb_posted_fee, None);
        assert_eq!(cleared.eb_tier_delay_slots, None);
        assert_eq!(cleared.eb_tier_price_per_byte_at_assignment, None);
        assert_eq!(cleared.assigned_block_kind, None);
        assert_eq!(cleared.mempool_entry_slot, None);
        assert_eq!(cleared.mempool_entry_rb_index, None);
    }

    #[test]
    fn empty_eb_opportunity_reprices_naive_eb_lane() {
        let mut config = test_tiered_config();
        config.max_tiers = 2;
        config.tier_size_fractions = vec![0.0, 0.5];
        config.min_delay_ratio = 1.0;
        config.new_tier_delay_ratio = 1.0;
        config.new_tier_price = 90;
        config.block_selection_policy = TierBlockSelectionPolicy::NaiveRbEbTwoTier;

        let mut node = test_node_with_tiered_pricing_config(TierDelayUnit::Blocks, config);
        let before = {
            let pricing = node.pricing.as_ref().expect("pricing coordinator");
            let state = pricing
                .state
                .lock()
                .expect("global pricing coordinator mutex poisoned");
            state.pricing.tiers().expect("tiers")[1].price
        };
        assert_eq!(before, 90);

        let rb = crate::model::LinearRankingBlock {
            header: crate::model::LinearRankingBlockHeader {
                id: crate::model::BlockId {
                    slot: 1,
                    producer: node.id,
                },
                vrf: 0,
                parent: None,
                bytes: node.sim_config.sizes.block_header,
                eb_announcement: None,
            },
            transactions: vec![],
            endorsement: None,
        };

        node.finish_generating_rb(rb, None, true);

        let after = {
            let pricing = node.pricing.as_ref().expect("pricing coordinator");
            let state = pricing
                .state
                .lock()
                .expect("global pricing coordinator mutex poisoned");
            state.pricing.tiers().expect("tiers")[1].price
        };
        assert_eq!(after, 79);
    }

    #[test]
    fn no_eb_opportunity_does_not_reprice_naive_eb_lane() {
        let mut config = test_tiered_config();
        config.max_tiers = 2;
        config.tier_size_fractions = vec![0.0, 0.5];
        config.min_delay_ratio = 1.0;
        config.new_tier_delay_ratio = 1.0;
        config.new_tier_price = 90;
        config.block_selection_policy = TierBlockSelectionPolicy::NaiveRbEbTwoTier;

        let mut node = test_node_with_tiered_pricing_config(TierDelayUnit::Blocks, config);
        let rb = crate::model::LinearRankingBlock {
            header: crate::model::LinearRankingBlockHeader {
                id: crate::model::BlockId {
                    slot: 1,
                    producer: node.id,
                },
                vrf: 0,
                parent: None,
                bytes: node.sim_config.sizes.block_header,
                eb_announcement: None,
            },
            transactions: vec![],
            endorsement: None,
        };

        node.finish_generating_rb(rb, None, false);

        let eb_price = {
            let pricing = node.pricing.as_ref().expect("pricing coordinator");
            let state = pricing
                .state
                .lock()
                .expect("global pricing coordinator mutex poisoned");
            state.pricing.tiers().expect("tiers")[1].price
        };
        assert_eq!(eb_price, 90);
    }

    #[test]
    fn shared_single_pool_overflow_accepts_when_eb_has_headroom() {
        let mut config = test_tiered_config();
        config.max_tiers = 1;
        config.tier_size_fractions = vec![0.0];
        let mut node = test_node_with_tiered_pricing_config_with_block_sizes(
            TierDelayUnit::Blocks,
            config,
            100,
            1000,
        );
        let pricing = node.pricing.clone().expect("pricing coordinator");
        let tier = TierId::new(0);

        let pending_tx = Transaction {
            id: TransactionId::new(30),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 100,
            submission_slot: 10,
            value: 100,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(1_000),
            tier_preference: Some(tier),
            tier_version_created_slot: Some(10),
            tier_delay_slots: Some(1),
            tier_price_per_byte_at_assignment: Some(10),
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: Some(BlockKind::RankingBlock),
            mempool_entry_slot: Some(10),
            mempool_entry_rb_index: Some(5),
            input_id: 30,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        };
        node.mempool.insert_new_active(Arc::new(pending_tx));

        let candidate_tx = Transaction {
            id: TransactionId::new(31),
            actor_id: ActorId::new(0),
            shard: 0,
            bytes: 1,
            submission_slot: 10,
            value: 100,
            urgency: UrgencyProfile::Indifferent,
            posted_fee: Some(10),
            tier_preference: Some(tier),
            tier_version_created_slot: Some(10),
            tier_delay_slots: Some(1),
            tier_price_per_byte_at_assignment: Some(10),
            eb_tier_preference: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            eb_tier_price_per_byte_at_assignment: None,
            assigned_block_kind: None,
            mempool_entry_slot: Some(10),
            mempool_entry_rb_index: Some(5),
            input_id: 31,
            overcollateralization_factor: 0,
            urgency_component_index: None,
        };

        assert_eq!(
            pricing.effective_tier_capacity_for_block_kind(BlockKind::RankingBlock, tier, 100),
            Some(100)
        );
        assert_eq!(
            pricing.effective_tier_capacity_for_block_kind(BlockKind::EndorserBlock, tier, 1000),
            Some(1000)
        );
        assert_eq!(node.overfull_tier_lane(&pricing, &candidate_tx), None);
    }

    #[test]
    fn shared_single_pool_endorsed_eb_volume_reprices_only_on_rb_cadence() {
        let mut node = test_node_with_tiered_pricing(TierDelayUnit::Blocks);
        let mut txs = TxFactory::new();
        let eb_tx = txs.anchored_rb_tx(node.sim_config.max_eb_size, TierId::new(0), 1, 0, 0);
        let eb = EndorserBlock {
            slot: 1,
            producer: node.id,
            bytes: node
                .sim_config
                .sizes
                .linear_eb(std::slice::from_ref(&eb_tx)),
            txs: vec![eb_tx.clone()],
        };
        let eb_id = eb.id();

        node.finish_generating_eb(eb, vec![]);

        let price_after_eb = {
            let pricing = node.pricing.as_ref().expect("pricing coordinator");
            let state = pricing
                .state
                .lock()
                .expect("global pricing coordinator mutex poisoned");
            state.pricing.tiers().expect("tiers")[0].price
        };
        assert_eq!(price_after_eb, 1_000);

        let rb = Arc::new(crate::model::LinearRankingBlock {
            header: crate::model::LinearRankingBlockHeader {
                id: crate::model::BlockId {
                    slot: 2,
                    producer: node.id,
                },
                vrf: 0,
                parent: None,
                bytes: node.sim_config.sizes.block_header,
                eb_announcement: None,
            },
            transactions: vec![],
            endorsement: Some(Endorsement {
                eb: eb_id,
                size_bytes: 0,
                votes: BTreeMap::new(),
            }),
        });

        node.finish_validating_rb(rb);

        let price_after_rb = {
            let pricing = node.pricing.as_ref().expect("pricing coordinator");
            let state = pricing
                .state
                .lock()
                .expect("global pricing coordinator mutex poisoned");
            state.pricing.tiers().expect("tiers")[0].price
        };
        assert_eq!(price_after_rb, 1_125);
    }

    #[test]
    fn shared_single_pool_defers_endorsed_rb_pricing_until_eb_is_known() {
        let mut node = test_node_with_tiered_pricing(TierDelayUnit::Blocks);
        let mut txs = TxFactory::new();
        let eb_tx = txs.anchored_rb_tx(node.sim_config.max_eb_size, TierId::new(0), 1, 0, 0);
        let eb = Arc::new(EndorserBlock {
            slot: 1,
            producer: node.id,
            bytes: node
                .sim_config
                .sizes
                .linear_eb(std::slice::from_ref(&eb_tx)),
            txs: vec![eb_tx.clone()],
        });
        let rb = Arc::new(crate::model::LinearRankingBlock {
            header: crate::model::LinearRankingBlockHeader {
                id: crate::model::BlockId {
                    slot: 2,
                    producer: node.id,
                },
                vrf: 0,
                parent: None,
                bytes: node.sim_config.sizes.block_header,
                eb_announcement: None,
            },
            transactions: vec![],
            endorsement: Some(Endorsement {
                eb: eb.id(),
                size_bytes: 0,
                votes: BTreeMap::new(),
            }),
        });

        node.finish_validating_rb(rb);

        let price_before_eb = {
            let pricing = node.pricing.as_ref().expect("pricing coordinator");
            let state = pricing
                .state
                .lock()
                .expect("global pricing coordinator mutex poisoned");
            state.pricing.tiers().expect("tiers")[0].price
        };
        assert_eq!(price_before_eb, 1_000);

        node.leios.ebs.insert(
            eb.id(),
            EndorserBlockView::Received {
                eb: eb.clone(),
                seen: Timestamp::from_secs(0),
                all_txs_seen: true,
                validated: false,
            },
        );
        node.finish_validating_eb(eb, Timestamp::from_secs(0));

        let price_after_eb = {
            let pricing = node.pricing.as_ref().expect("pricing coordinator");
            let state = pricing
                .state
                .lock()
                .expect("global pricing coordinator mutex poisoned");
            state.pricing.tiers().expect("tiers")[0].price
        };
        assert_eq!(price_after_eb, 1_125);
    }
}
