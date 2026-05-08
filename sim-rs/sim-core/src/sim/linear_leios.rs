mod attackers;
pub use attackers::register_actors;
use rand_distr::Distribution;
use tokio::sync::mpsc;

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use rand::{Rng as _, RngCore as _, SeedableRng as _, seq::SliceRandom as _};
use rand_chacha::ChaChaRng;

use crate::{
    clock::{Clock, Timestamp},
    config::{
        CpuTimeConfig, EBPropagationCriteria, LeiosVariant, MempoolSamplingStrategy,
        NodeBehaviours, NodeConfiguration, NodeId, PricingConfig, RelayStrategy, SimConfiguration,
        TransactionConfig,
    },
    events::EventTracker,
    model::{
        BlockId, Endorsement, EndorserBlockId, LinearEndorserBlock as EndorserBlock,
        LinearRankingBlock as RankingBlock, LinearRankingBlockHeader as RankingBlockHeader,
        NoVoteReason, Transaction, TransactionId, VoteBundle, VoteBundleId,
    },
    sim::{
        MiniProtocol, NodeImpl, SimCpuTask, SimMessage,
        linear_leios::attackers::{EBWithholdingEvent, EBWithholdingSender},
        lottery::{LotteryConfig, LotteryKind, MockLotteryResults, vrf_probabilities},
        mempool_gate::MempoolGate,
    },
    tx_pricing::{
        BaselinePricing, BlockKind, BlockLaneBreakdown, Eip1559Pricing, Lane, LaneSelectionOrder,
        LaneValidityRule, PricedBlockSample, PricingBackend, TwoLanePricing,
    },
};

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
    RBBlockGenerated(RankingBlock, Option<(EndorserBlock, Vec<Arc<Transaction>>)>),
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
            Self::RBBlockGenerated(_, _) => "GenRB",
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
            Self::RBBlockGenerated(_, _) => "".to_string(),
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
            Self::RBBlockGenerated(rb, eb) => {
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
    mempool: Mempool,
    /// Phase-2 fee gate: tracks per-lane bytes, fee admission, and
    /// quote-drift revalidation.
    gate: MempoolGate,
    /// Phase-2 pricing backend (single-lane M1: `BaselinePricing` or
    /// `Eip1559Pricing`).
    pricing: Box<dyn PricingBackend>,
    ledger_states: BTreeMap<BlockId, Arc<LedgerState>>,
    praos: NodePraosState,
    leios: NodeLeiosState,
    behaviours: NodeBehaviours,
    /// Phase-2 actor state (M3+). Populated when the config supplies
    /// an `actors:` profile and this node has `tx_generation_weight > 0`;
    /// `None` otherwise. Each slot, `run_actors_for_slot` samples
    /// per-component arrivals and submits txs through `generate_tx`.
    actor_state: Option<NodeActorState>,

    eb_withholding_sender: Option<EBWithholdingSender>,
    eb_withholding_event_source: Option<mpsc::UnboundedReceiver<EBWithholdingEvent>>,
}

/// Per-node actor sampling state. Each component carries its own
/// `ChaChaRng` (seeded from the node's RNG + component index) so
/// adding/removing components doesn't disturb other components'
/// sampling streams.
struct NodeActorState {
    profile: Arc<crate::tx_actors::ActorProfile>,
    component_rngs: Vec<ChaChaRng>,
    next_tx_id: u64,
    /// Per-(component, lane) rolling-average inclusion-delay
    /// estimator. Initialised from the component's
    /// `target_inclusion_blocks_*` defaults; updated via
    /// `LinearLeiosNode::observe_actor_inclusion` when the actor's
    /// txs are charged.
    latency: Vec<crate::tx_actors::LatencyEstimator>,
    /// Map `tx_id → (submit_slot, component_index)` for actor-submitted
    /// txs awaiting inclusion. Used to compute `latency_blocks` when
    /// the tx lands.
    pending: HashMap<TransactionId, (u64, u32)>,
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
        mut rng: ChaChaRng,
        clock: Clock,
    ) -> Self {
        let lottery = LotteryConfig::Random {
            stake: config.stake,
            total_stake: sim_config.total_stake,
        };
        let gate = MempoolGate::new(sim_config.mempool_gate_config());
        // The fee gate is the sole byte-cap authority in the wired
        // path: it admits before the underlying mempool. Set the
        // mempool's own cap to the gate's cap so byte-cap rejections
        // happen at the gate, never at the mempool. If the mempool's
        // cap were smaller, byte-cap-full mempool inserts would queue
        // the tx for later promotion, and the gate (which we'd already
        // rolled back) would never see it on promotion — the tx would
        // re-enter the active mempool with no fee admission, no
        // quote-drift revalidation, and no inclusion charging.
        let mempool_max_size_bytes = gate.config().max_total_size_bytes;
        let pricing: Box<dyn PricingBackend> = match sim_config.pricing_config() {
            PricingConfig::Baseline => {
                Box::new(BaselinePricing::new(sim_config.mempool_gate_config().min_fee_a))
            }
            PricingConfig::Eip1559(settings) => Box::new(
                Eip1559Pricing::new(settings.clone())
                    .expect("Eip1559Settings validated at config build time"),
            ),
            PricingConfig::TwoLane(settings) => Box::new(
                TwoLanePricing::new(settings.clone())
                    .expect("TwoLaneSettings validated at config build time"),
            ),
        };

        // M3: only nodes with `tx_generation_weight > 0` run actors.
        // Default weight is 0 if `stake > 0`, 1 otherwise (matches the
        // legacy `TransactionProducer` weighting). Each component
        // gets its own ChaChaRng derived from the node RNG, so
        // adding/removing components doesn't disturb other components'
        // sample streams.
        let actor_state = sim_config
            .actor_profile()
            .filter(|_| {
                config
                    .tx_generation_weight
                    .unwrap_or(if config.stake > 0 { 0 } else { 1 })
                    > 0
            })
            .map(|profile| {
                let component_rngs = profile
                    .components
                    .iter()
                    .map(|_| ChaChaRng::seed_from_u64(rng.next_u64()))
                    .collect();
                let latency = profile
                    .components
                    .iter()
                    .map(|c| {
                        crate::tx_actors::LatencyEstimator::new(
                            32,
                            c.target_inclusion_blocks_priority,
                            c.target_inclusion_blocks_standard,
                        )
                    })
                    .collect();
                NodeActorState {
                    profile: Arc::clone(profile),
                    component_rngs,
                    next_tx_id: 0,
                    latency,
                    pending: HashMap::new(),
                }
            });

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
            mempool: Mempool::new(mempool_max_size_bytes),
            gate,
            pricing,
            ledger_states: BTreeMap::new(),
            praos: NodePraosState::default(),
            leios: NodeLeiosState::default(),
            behaviours: config.behaviours.clone(),
            actor_state,
            eb_withholding_sender: None,
            eb_withholding_event_source: None,
        }
    }

    fn custom_event_source(&mut self) -> Option<mpsc::UnboundedReceiver<Self::CustomEvent>> {
        self.eb_withholding_event_source.take()
    }

    fn handle_new_slot(&mut self, slot: u64) -> EventResult {
        // Ordering invariant (relied on by sim-cli's MetricsCollector
        // to derive `submit_slot` for actor-generated txs):
        //   1. `emit_pricing_tick(slot)` — advances the metrics
        //      collector's slot pointer to `slot`.
        //   2. `run_actors_for_slot(slot)` — emits `TXGenerated`,
        //      which the collector tags with the just-advanced slot.
        //   3. `try_generate_rb(slot)` — may emit `TXIncluded`/
        //      `TXEvictedQuoteDrift`, also at `slot`.
        // Reordering steps 1 and 2 would tag actor txs with the
        // *previous* slot's number and skew per-component latency.
        self.emit_pricing_tick(slot);
        self.run_actors_for_slot(slot);
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
            CpuTask::RBBlockGenerated(rb, eb) => self.finish_generating_rb(rb, eb),
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
        self.tracker.track_transaction_generated(&tx, self.id);
        self.propagate_tx(self.id, tx);
    }

    fn propagate_tx(&mut self, from: NodeId, tx: Arc<Transaction>) {
        let id = tx.id;
        if self
            .txs
            .insert(id, TransactionView::Received(tx.clone()))
            .is_some_and(|tx| matches!(tx, TransactionView::Received(_)))
        {
            return;
        }

        let referenced_by_eb = self.acknowledge_tx(&tx);
        let added_to_mempool = self.try_add_tx_to_mempool(&tx);

        // If we added the TX to our mempool, we want to propagate it so our peers can as well.
        // If it was referenced by an EB, we want to propagate it so our peers have the full EB.
        // TODO: should send to producers instead (make configurable)
        if referenced_by_eb || added_to_mempool {
            for peer in &self.consumers {
                if *peer == from {
                    continue;
                }
                self.queued.send_to(*peer, Message::AnnounceTx(id));
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

        let validity_rule = self.pricing.lane_validity_rule(BlockKind::RankingBlock);
        let selection_order = self.pricing.lane_selection_order();
        let rb_reserved = matches!(validity_rule, LaneValidityRule::PriorityOnly);

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
                // M2: EB-validation-at-endorsement-time (handoff §4 /
                // approved scope). Walk the candidate EB's txs and
                // refuse to endorse if any has gone stale at the
                // producer's current per-lane quote — staleness in the
                // EB would otherwise pollute the priced-block sample,
                // `spent_inputs`, and downstream conflict cascades.
                if !self.eb_endorsement_valid(&eb) {
                    return None;
                }
                // EB content is endorseable. Decide per-tx served_lane
                // and charge inclusions. Sample-from-mempool already
                // removed eb.txs at EB-creation time; this clears them
                // from the fee gate and emits one inclusion event
                // each.
                let served = self.assign_served_lanes(&eb, rb_reserved);
                let pairs: Vec<(Arc<Transaction>, Lane)> = eb
                    .txs
                    .iter()
                    .cloned()
                    .zip(served)
                    .collect();
                self.charge_inclusions(&pairs);
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

        let mut rb_transactions = vec![];
        if !produce_empty_block && self.sim_config.praos_fallback && endorsement.is_none() {
            if let TransactionConfig::Mock(config) = &self.sim_config.transactions {
                // Add one transaction, the right size for the extra RB payload
                let tx = config.mock_tx(config.rb_size);
                self.tracker.track_transaction_generated(&tx, self.id);
                rb_transactions.push(Arc::new(tx));
            } else {
                self.sample_from_mempool_lane_aware(
                    &mut rb_transactions,
                    self.sim_config.max_block_size,
                    true,
                    validity_rule,
                    selection_order,
                );
            }
        }

        let mut eb_transactions = vec![];
        let mut withheld_txs = vec![];
        // M3: producer's full two-trigger partition decision is
        // computed at EB-build time and stored on the EB. Mock-mode
        // and withheld-tx-attack paths do not exercise the partition
        // (no real priority demand to gate); they default to false.
        let mut eb_partition_activated = false;
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
                // M3: pack the EB body and record the producer's
                // two-trigger partition decision in one step. The
                // endorser reuses `eb.partition_activated` via
                // `assign_served_lanes`; producer and endorser agree
                // by construction.
                let (packed, activated) = self
                    .select_eb_with_partition(self.sim_config.max_eb_size, selection_order);
                eb_transactions.extend(packed);
                eb_partition_activated = activated;
            }
        }
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
                partition_activated: eb_partition_activated,
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

        // Producer charges its own RB body's transactions for inclusion.
        // RB body served-lane policy:
        // - RB-reserved variants (priority-only RB): served_lane = Priority.
        // - Un-reserved (single-lane and un-reserved two-lane): served_lane = posted_lane.
        let rb_pairs: Vec<(Arc<Transaction>, Lane)> = rb
            .transactions
            .iter()
            .cloned()
            .map(|tx| {
                let served = if rb_reserved {
                    Lane::Priority
                } else {
                    tx.posted_lane
                };
                (tx, served)
            })
            .collect();
        self.charge_inclusions(&rb_pairs);

        self.tracker.track_praos_block_lottery_won(rb.header.id);
        self.queued
            .schedule_cpu_task(CpuTask::RBBlockGenerated(rb, eb));
    }

    /// Validate every tx in a candidate EB against this producer's
    /// current per-lane quote. Returns false if any tx's `posted_fee`
    /// exceeds its `max_fee_lovelace` — meaning the EB has at least one
    /// stale tx and must not be endorsed
    /// (mechanism-design.md line 43; M1 handoff §"Known limitations" §4).
    ///
    /// **Refuse-to-endorse remedy** (per user direction at plan time):
    /// the endorser cannot rewrite the EB body, so the only spec-faithful
    /// response to a stale-tx EB is to skip the endorsement entirely.
    fn eb_endorsement_valid(&self, eb: &EndorserBlock) -> bool {
        let q_standard = self.pricing.current_quote(Lane::Standard);
        let q_priority = self.pricing.current_quote(Lane::Priority);
        let min_fee_b = self.gate.config().min_fee_b;
        for tx in &eb.txs {
            let q = match tx.posted_lane {
                Lane::Standard => q_standard,
                Lane::Priority => q_priority,
            };
            let posted_fee = q
                .checked_mul(tx.bytes)
                .and_then(|x| x.checked_add(min_fee_b));
            match posted_fee {
                Some(fee) if fee <= tx.max_fee_lovelace => continue,
                _ => return false,
            }
        }
        true
    }

    /// Per-tx `served_lane` for a candidate EB, given the producer's
    /// stored partition decision (`eb.partition_activated`) and the
    /// variant's `rb_reserved` flag. Endorser and producer agree by
    /// construction because the activation bit is carried on the EB.
    ///
    /// - Un-reserved variants (`rb_reserved = false`): no partition;
    ///   `served_lane = posted_lane`.
    /// - RB-reserved + activated: priority-fee txs whose cumulative
    ///   bytes ≤ `priority_reservation_bytes` get `Priority`; further
    ///   priority txs and all standard txs get `Standard`.
    /// - RB-reserved + NOT activated: all priority-fee txs get
    ///   `Standard` (refunded down to standard fee per spec).
    fn assign_served_lanes(&self, eb: &EndorserBlock, rb_reserved: bool) -> Vec<Lane> {
        if !rb_reserved {
            return eb.txs.iter().map(|t| t.posted_lane).collect();
        }
        if !eb.partition_activated {
            return vec![Lane::Standard; eb.txs.len()];
        }
        let priority_reservation_bytes = self.sim_config.max_block_size;
        let mut out = Vec::with_capacity(eb.txs.len());
        let mut priority_used: u64 = 0;
        for tx in &eb.txs {
            let lane = match tx.posted_lane {
                Lane::Standard => Lane::Standard,
                Lane::Priority => {
                    if priority_used.saturating_add(tx.bytes) <= priority_reservation_bytes {
                        priority_used = priority_used.saturating_add(tx.bytes);
                        Lane::Priority
                    } else {
                        Lane::Standard
                    }
                }
            };
            out.push(lane);
        }
        out
    }

    fn finish_generating_rb(
        &mut self,
        rb: RankingBlock,
        eb: Option<(EndorserBlock, Vec<Arc<Transaction>>)>,
    ) {
        self.tracker.track_linear_rb_generated(&rb);
        self.publish_rb(Arc::new(rb), false);
        if let Some((eb, withheld_txs)) = eb {
            self.tracker.track_linear_eb_generated(&eb);
            self.finish_generating_eb(eb, withheld_txs);
        }
    }

    fn publish_rb(&mut self, rb: Arc<RankingBlock>, already_sent_header: bool) {
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
        let header_seen = self
            .praos
            .blocks
            .get(&rb.header.id)
            .and_then(|rb| rb.header_seen())
            .unwrap_or(self.clock.now());
        if let Some(eb_id) = rb.header.eb_announcement {
            self.leios.ebs_by_rb.insert(rb.header.id, eb_id);
        }
        // Phase-2 controller hook: feed any priced-block samples this RB
        // produces (RB body if non-empty; endorsed EB if locally
        // validated), update pricing, and revalidate the gate.
        self.apply_priced_block(&rb);
        self.praos
            .blocks
            .insert(rb.header.id, RankingBlockView::Received { rb, header_seen });
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
            },
        );
        if let Some(endorsement) = &rb.endorsement
            && !self.is_eb_validated(endorsement.eb)
        {
            self.leios.incomplete_onchain_ebs.insert(endorsement.eb);
        }

        self.publish_rb(rb, true);
    }

    fn latest_rb(&self) -> Option<(&Arc<RankingBlock>, Timestamp)> {
        self.praos.blocks.iter().rev().find_map(|(_, rb)| {
            if let RankingBlockView::Received { rb, header_seen } = rb {
                Some((rb, *header_seen))
            } else {
                None
            }
        })
    }

    fn latest_rb_id(&self) -> Option<BlockId> {
        self.latest_rb().map(|(rb, _)| rb.header.id)
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
            // The cert RB landed on-chain earlier; emit the deferred EB
            // sample now and run the consequent pricing update +
            // revalidation. Single-lane: one Standard sample.
            self.apply_eb_priced_block(&eb);
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
        let sender = self.eb_withholding_sender.as_ref().unwrap();
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
        let withhold_tx_config = self.sim_config.attacks.late_tx.as_ref().unwrap();
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
            let tx = match &self.sim_config.transactions {
                TransactionConfig::Real(cfg) => cfg.new_tx(&mut self.rng, None),
                TransactionConfig::Mock(cfg) => cfg.mock_tx(cfg.eb_size / txs_to_generate),
            };
            self.tracker.track_transaction_generated(&tx, self.id);
            let tx = Arc::new(tx);
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
    fn try_add_tx_to_mempool(&mut self, tx: &Arc<Transaction>) -> bool {
        let ledger_state = self.resolve_ledger_state(self.latest_rb_id());
        if ledger_state.spent_inputs.contains(&tx.input_id) {
            // This TX conflicts with something already on-chain
            return false;
        }

        // Phase-2 fee admission. Reject if posted_fee at the lane's
        // current quote exceeds the tx's max-fee budget, or if the
        // mempool byte cap would be exceeded.
        let quote = self.pricing.current_quote(tx.posted_lane);
        if self.gate.try_admit(tx, quote).is_err() {
            return false;
        }

        // UTxO/conflict + byte-cap mempool. If this rejects (e.g.
        // input-id conflict in mempool), revert the gate to keep state
        // consistent.
        if !self.mempool.try_insert(tx.clone()) {
            self.gate.remove_silent(tx.id);
            return false;
        }
        true
    }

    /// Lane-aware mempool sampling (implementation-plan.md
    /// §"Lane-aware block selection", lines 91, 105-118; M1 handoff
    /// §"Architectural gaps M1 left for M2" item 1).
    ///
    /// Two filters layered on the M1 base:
    /// - `validity_rule == PriorityOnly`: skip standard-fee txs (they
    ///   cannot be in this block's body — RB-reserved RB rule).
    /// - `selection_order == PriorityFirst`: priority-fee txs are
    ///   considered before standard-fee txs (canonical block-build
    ///   order for the live two-lane mechanisms).
    ///
    /// The `MempoolSamplingStrategy` (random vs ordered-by-id) acts as
    /// the within-lane tiebreaker — the lane order takes precedence.
    fn sample_from_mempool_lane_aware(
        &mut self,
        txs: &mut Vec<Arc<Transaction>>,
        max_size: u64,
        remove: bool,
        validity_rule: LaneValidityRule,
        selection_order: LaneSelectionOrder,
    ) {
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

        // Lane-priority ordering is applied AFTER the strategy-based
        // ordering. We `pop` from the back, so priority txs need to
        // sort to higher keys (back of vec) under PriorityFirst.
        // `sort_by_key` is stable, so within-lane order is preserved.
        if matches!(selection_order, LaneSelectionOrder::PriorityFirst) {
            let txs_view = &self.txs;
            candidates.sort_by_key(|id| match txs_view.get(id) {
                Some(TransactionView::Received(tx)) => match tx.posted_lane {
                    Lane::Priority => 1u8,
                    Lane::Standard => 0u8,
                },
                _ => 0u8,
            });
        }

        // Fill with as many pending transactions as can fit. Validity
        // rejections `continue` (the next candidate may still fit);
        // size rejections `break` (matches M1 single-lane behaviour
        // and is what the EB binary fullness trigger's "valid
        // unselected mempool tx remains and none fits in residual"
        // case relies on — see implementation-plan.md lines 109-112).
        let mut removed_ids = vec![];
        while let Some(id) = candidates.pop() {
            let Some(TransactionView::Received(tx)) = self.txs.get(&id) else {
                panic!("missing a TX in our mempool");
            };
            if matches!(validity_rule, LaneValidityRule::PriorityOnly)
                && tx.posted_lane == Lane::Standard
            {
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
        for newly_queued_tx in self.mempool.remove_txs(removed_ids) {
            for peer in &self.consumers {
                self.queued
                    .send_to(*peer, Message::AnnounceTx(newly_queued_tx));
            }
        }
    }

    /// EB selection with the spec's binary fullness trigger
    /// (implementation-plan.md lines 104-120). Returns the EB body
    /// Pack an EB body and decide whether to activate the priority
    /// partition. Returns the packed body and the activation flag.
    ///
    /// Activation rules (OR'd):
    /// 1. **Saturation** — `selected_bytes >= eb_capacity`. A full EB
    ///    is a saturation event regardless of mempool depletion.
    /// 2. **Capacity-bound rejection** — at least one valid unselected
    ///    tx remains and none of them fits in residual bytes.
    ///
    /// If neither holds (mempool ran dry before the EB filled, or
    /// some unselected tx still fits), partition is **not** activated.
    ///
    /// **Production path** (M3): the producer calls this at EB-build
    /// time and stores the activation bit on the EB. The endorser
    /// re-uses the stored bit via `assign_served_lanes`. Endorser and
    /// producer agree on served-lane assignment by construction; see
    /// `LinearEndorserBlock::partition_activated` and
    /// `docs/phase-2/m3-handoff.md`.
    fn select_eb_with_partition(
        &mut self,
        eb_capacity: u64,
        selection_order: LaneSelectionOrder,
    ) -> (Vec<Arc<Transaction>>, bool) {
        // Pack the EB greedily under the configured selection order.
        let mut packed: Vec<Arc<Transaction>> = Vec::new();
        self.sample_from_mempool_lane_aware(
            &mut packed,
            eb_capacity,
            false, // don't drain the mempool — selection happens elsewhere
            LaneValidityRule::None,
            selection_order,
        );
        let selected_bytes: u64 = packed.iter().map(|t| t.bytes).sum();
        let residual = eb_capacity.saturating_sub(selected_bytes);

        // Two-trigger activation rule.
        let activated = if selected_bytes >= eb_capacity {
            true
        } else {
            let packed_ids: HashSet<_> = packed.iter().map(|t| t.id).collect();
            let mut any_unselected = false;
            let mut any_fits = false;
            for id in self.mempool.ids() {
                if packed_ids.contains(&id) {
                    continue;
                }
                let Some(TransactionView::Received(tx)) = self.txs.get(&id) else {
                    continue;
                };
                any_unselected = true;
                if tx.bytes <= residual {
                    any_fits = true;
                    break;
                }
            }
            any_unselected && !any_fits
        };
        (packed, activated)
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

    /// Charge inclusions for transactions in a block (RB body or EB).
    /// Fee and refund are computed from the `Transaction` itself, not
    /// from gate residency: a tx may be in a producer's RB/EB without
    /// being resident in that producer's fee gate (e.g. in
    /// `LeiosVariant::Linear`, EB-borne txs aren't separately
    /// propagated as Tx messages, so an endorsing producer who didn't
    /// admit them won't have them in its gate). The gate's
    /// `on_inclusion` is still called for cleanup but is not the gate
    /// for event emission.
    ///
    /// **Per-tx `served_lane`** (M2): the caller decides served-lane
    /// per tx during selection. For RB-reserved RB bodies the served
    /// lane is `Priority`; for un-reserved variants it's `posted_lane`;
    /// for EB bodies the partition trigger decides. The actual fee is
    /// charged at the served-lane quote per
    /// implementation-plan.md lines 96-100.
    fn charge_inclusions(
        &mut self,
        txs_with_served_lane: &[(Arc<Transaction>, Lane)],
    ) {
        if txs_with_served_lane.is_empty() {
            return;
        }
        let slot = (self.clock.now() - Timestamp::zero()).as_secs();
        let q_standard = self.pricing.current_quote(Lane::Standard);
        let q_priority = self.pricing.current_quote(Lane::Priority);
        let min_fee_b = self.gate.config().min_fee_b;
        for (tx, served_lane) in txs_with_served_lane {
            let quote = match served_lane {
                Lane::Standard => q_standard,
                Lane::Priority => q_priority,
            };
            // Same rounding regime as admission/revalidation
            // (implementation-plan.md lines 92-95): minFeeB +
            // quote_per_byte × bytes.
            let actual_fee = quote
                .checked_mul(tx.bytes)
                .and_then(|q| q.checked_add(min_fee_b))
                .unwrap_or(u64::MAX);
            // Spec max-fee invariant (mechanism-design.md §EIP-1559
            // maximum-fee semantics, line 43). M2 closes the loophole
            // for endorsed EBs by validating the EB at endorsement
            // time (no stale tx is permitted into a certified EB).
            // The fall-through here remains as a defensive backstop
            // for txs that arrived via paths not covered by gate
            // revalidation or endorsement validation.
            if actual_fee > tx.max_fee_lovelace {
                self.tracker.track_tx_evicted_quote_drift(
                    tx.id,
                    self.id,
                    slot,
                    tx.bytes,
                    tx.posted_lane,
                    quote,
                    tx.max_fee_lovelace,
                );
                self.gate.remove_silent(tx.id);
                self.forget_actor_pending(tx.id);
                continue;
            }
            let refund = tx.max_fee_lovelace - actual_fee;
            self.gate.remove_silent(tx.id);
            self.tracker.track_tx_included(
                tx.id,
                self.id,
                slot,
                tx.bytes,
                tx.posted_lane,
                *served_lane,
                tx.max_fee_lovelace,
                actual_fee,
                refund,
            );
            self.observe_actor_inclusion(tx.id, *served_lane, slot);
        }
    }

    /// Build priced-block samples from this RB and (if its EB is
    /// locally validated) the endorsed EB. Apply them to the pricing
    /// backend, then revalidate the gate; any quote-drift evictions
    /// emit `TXEvictedQuoteDrift` events and are also removed from the
    /// linear-leios mempool.
    ///
    /// **Known limitation (M1)**: pricing state mutates here with no
    /// rollback path. Slot-battle replacement at
    /// `finish_validating_rb_header` removes the losing block from
    /// `praos.blocks` but does not undo controller updates, gate
    /// `on_inclusion` removals, or `TXIncluded` events that the losing
    /// block already triggered. The mechanism spec treats `c` as
    /// ledger state, so a fork resolution conceptually requires
    /// rolling back the controller and re-applying samples for the
    /// canonical chain. M1's exit criterion is the single-producer
    /// smoke test where slot battles cannot occur; M2's deterministic
    /// scenario tests should either avoid slot battles or implement
    /// snapshot-and-replay rollback before exercising them.
    fn apply_priced_block(&mut self, rb: &RankingBlock) {
        let slot = rb.header.id.slot;
        let mut samples: Vec<PricedBlockSample> = Vec::new();
        // Tx-bearing RB → variant-aware sample(s) via the backend's
        // `samples_for_block` policy. Endorsement-only RB → no RB
        // sample (implementation-plan.md line 70).
        if !rb.transactions.is_empty() {
            let breakdown = breakdown_for(&rb.transactions, self.sim_config.max_block_size);
            samples.extend(
                self.pricing
                    .samples_for_block(BlockKind::RankingBlock, &breakdown),
            );
        }
        // Endorsed EB applied alongside this RB iff it's locally
        // validated. Otherwise the EB sample is deferred until
        // `finish_validating_eb`.
        if let Some(endorsement) = &rb.endorsement
            && let Some(eb) = self.get_validated_eb(endorsement.eb)
        {
            samples.extend(self.eb_samples(&eb));
        }
        self.feed_samples_and_revalidate(slot, &samples);
    }

    fn apply_eb_priced_block(&mut self, eb: &EndorserBlock) {
        let slot = (self.clock.now() - Timestamp::zero()).as_secs();
        let samples = self.eb_samples(eb);
        self.feed_samples_and_revalidate(slot, &samples);
    }

    fn eb_samples(&self, eb: &EndorserBlock) -> Vec<PricedBlockSample> {
        let breakdown = breakdown_for(&eb.txs, self.sim_config.max_eb_size);
        self.pricing
            .samples_for_block(BlockKind::EndorserBlock, &breakdown)
    }

    fn feed_samples_and_revalidate(&mut self, slot: u64, samples: &[PricedBlockSample]) {
        if !samples.is_empty() {
            self.pricing.update_after_block(samples);
        }
        // Walk the gate; evict any txs whose lane quote drifted above
        // their max-fee budget. Returns the eviction records.
        let q_standard = self.pricing.current_quote(Lane::Standard);
        let q_priority = self.pricing.current_quote(Lane::Priority);
        let evicted = self.gate.revalidate(|lane| match lane {
            Lane::Standard => q_standard,
            Lane::Priority => q_priority,
        });
        if evicted.is_empty() {
            return;
        }
        // Emit the eviction events and clear the same txs from the
        // linear-leios mempool. `remove_conflicting_txs` is the right
        // existing API: each evicted tx maps to its `input_id`.
        let mut input_ids: HashSet<u64> = HashSet::with_capacity(evicted.len());
        for record in &evicted {
            self.tracker.track_tx_evicted_quote_drift(
                record.tx_id,
                self.id,
                slot,
                record.bytes,
                record.posted_lane,
                record.current_quote_per_byte,
                record.max_fee_lovelace,
            );
            self.forget_actor_pending(record.tx_id);
            // Look up the underlying tx to find its `input_id`. Evicted
            // txs are still in `self.txs` (TransactionView::Received)
            // because we never drop them from the propagation cache.
            if let Some(TransactionView::Received(tx)) = self.txs.get(&record.tx_id) {
                input_ids.insert(tx.input_id);
            }
        }
        if !input_ids.is_empty() {
            // Drop these from the conflict-aware mempool. Any newly
            // queued slack txs get re-announced to peers.
            //
            // Note for M2: under the gate-is-sole-byte-cap-authority
            // invariant (handoff §3), `mempool.queue` is empty in the
            // wired flow, so `remove_conflicting_txs` returns zero
            // promotions and this fan-out is dead in practice. Kept
            // as a defensive matched mirror of `sample_from_mempool`'s
            // re-announce. If multi-node M2 tests reintroduce queue
            // semantics, double-check that the per-evicted-tx × peers
            // fan-out cost stays bounded.
            for newly_queued_tx in self.mempool.remove_conflicting_txs(&input_ids) {
                for peer in &self.consumers {
                    self.queued
                        .send_to(*peer, Message::AnnounceTx(newly_queued_tx));
                }
            }
        }
    }

    fn remove_txs_from_mempool(&mut self, txs: &[Arc<Transaction>]) {
        // Keep the fee-gate's resident set in sync with the mempool —
        // these silent removes do not emit inclusion events. Producers
        // emit inclusion events at block-generation time via
        // `charge_inclusions`. Quote-drift evictions emit eviction
        // events from `apply_priced_block`.
        for tx in txs {
            self.gate.remove_silent(tx.id);
        }
        let inputs = txs.iter().map(|tx| tx.input_id).collect::<HashSet<_>>();
        for newly_queued_tx in self.mempool.remove_conflicting_txs(&inputs) {
            for peer in &self.consumers {
                self.queued
                    .send_to(*peer, Message::AnnounceTx(newly_queued_tx));
            }
        }
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
}

// Common utilities
impl LinearLeiosNode {
    #[allow(unused)]
    pub fn mock_lottery(&mut self, results: Arc<MockLotteryResults>) {
        self.lottery = LotteryConfig::Mock { results };
    }
    // Simulates the output of a VRF using this node's stake (if any).
    fn run_vrf(&mut self, kind: LotteryKind, success_rate: f64) -> Option<u64> {
        self.lottery.run(kind, success_rate, &mut self.rng)
    }

    /// Test-only inspection: a fresh `PricingSnapshot` from the
    /// node's pricing backend. Used by M2 deterministic scenario
    /// tests (line 313 standard-isolation assertion etc.).
    #[cfg(test)]
    pub(crate) fn pricing_snapshot(&self) -> crate::tx_pricing::PricingSnapshot {
        self.pricing.snapshot()
    }

    /// Test-only inspection: whether the node's mempool gate still
    /// has `tx_id` resident. Used by M2 regression tests to verify
    /// no cascade fires when an EB endorsement is refused.
    #[cfg(test)]
    pub(crate) fn gate_contains_for_test(&self, tx_id: &TransactionId) -> bool {
        self.gate.contains(tx_id)
    }

    /// Emit a per-slot `PricingTick` event so the metrics layer can
    /// populate `time_series.csv` rows with controller state and
    /// per-lane mempool bytes. M3+.
    fn emit_pricing_tick(&self, slot: u64) {
        let snapshot = self.pricing.snapshot();
        self.tracker.track_pricing_tick(
            self.id,
            slot,
            snapshot
                .priority_quote_per_byte
                .unwrap_or(snapshot.standard_quote_per_byte),
            snapshot.standard_quote_per_byte,
            snapshot.priority_window_util_x_1e9,
            snapshot.standard_window_util_x_1e9,
            self.gate.total_bytes(),
            self.gate.bytes_in_lane(Lane::Priority),
            self.gate.bytes_in_lane(Lane::Standard),
        );
    }

    /// M3 actor hook: sample arrivals from each component, build txs,
    /// and submit them through `generate_tx`. No-op when actor mode
    /// is not configured for this node.
    fn run_actors_for_slot(&mut self, slot: u64) {
        if self.actor_state.is_none() {
            return;
        }
        // Read pricing snapshot + per-lane quotes and latency
        // estimates while we have only an immutable borrow on
        // `self.actor_state`. Then move into a mutable section to
        // sample, build, and submit.
        let q_priority = self.pricing.current_quote(crate::tx_pricing::Lane::Priority);
        let q_standard = self.pricing.current_quote(crate::tx_pricing::Lane::Standard);
        let min_fee_b = self.gate.config().min_fee_b;
        // Snapshot per-component sampling inputs while holding only
        // immutable borrows on actor_state (so we can re-borrow
        // mutably below to step the RNGs).
        struct ComponentInputs {
            priority_latency: f64,
            standard_latency: f64,
            arrival_rate: f64,
            size_bytes: crate::probability::FloatDistribution,
            value_lovelace: crate::probability::FloatDistribution,
            urgency: crate::probability::FloatDistribution,
            lane_policy: crate::tx_actors::LanePolicy,
            max_fee_policy: crate::tx_actors::MaxFeePolicy,
            index: u32,
        }
        let component_inputs: Vec<ComponentInputs> = {
            let state = self.actor_state.as_ref().expect("checked above");
            state
                .profile
                .components
                .iter()
                .enumerate()
                .map(|(i, c)| ComponentInputs {
                    priority_latency: state.latency[i].expected(crate::tx_pricing::Lane::Priority),
                    standard_latency: state.latency[i].expected(crate::tx_pricing::Lane::Standard),
                    arrival_rate: c.arrival_rate_per_slot,
                    size_bytes: c.size_bytes,
                    value_lovelace: c.value_lovelace,
                    urgency: c.urgency,
                    lane_policy: c.lane_policy,
                    max_fee_policy: c.max_fee_policy,
                    index: c.index,
                })
                .collect()
        };
        // Sample arrival counts and tx inputs per component, build
        // txs, and submit. We collect the txs first (RNG sampling +
        // map insert) and submit afterwards so `generate_tx` can take
        // a mutable borrow on `self`.
        let mut to_submit: Vec<Arc<Transaction>> = Vec::new();
        for (i, ci) in component_inputs.iter().enumerate() {
            // Build a temporary `ActorComponent` view for the sampling
            // helpers — keeps the f64 → integer rounding/clamping
            // logic in one place (`tx_actors::ActorComponent`).
            let comp = crate::tx_actors::ActorComponent {
                index: ci.index,
                arrival_rate_per_slot: ci.arrival_rate,
                size_bytes: ci.size_bytes,
                value_lovelace: ci.value_lovelace,
                urgency: ci.urgency,
                lane_policy: ci.lane_policy,
                max_fee_policy: ci.max_fee_policy,
                target_inclusion_blocks_priority: ci.priority_latency,
                target_inclusion_blocks_standard: ci.standard_latency,
            };
            let count = {
                let state = self.actor_state.as_mut().expect("checked above");
                comp.sample_arrival_count(&mut state.component_rngs[i])
            };
            for _ in 0..count {
                let inputs = {
                    let state = self.actor_state.as_mut().expect("checked above");
                    comp.sample_tx_inputs(&mut state.component_rngs[i])
                };
                let priority_inputs = crate::tx_actors::LaneInputs {
                    current_quote_per_byte: q_priority,
                    expected_latency_blocks: ci.priority_latency,
                };
                let standard_inputs = crate::tx_actors::LaneInputs {
                    current_quote_per_byte: q_standard,
                    expected_latency_blocks: ci.standard_latency,
                };
                let Some(posted_lane) = crate::tx_actors::lane_choice::pick(
                    inputs.value_lovelace,
                    inputs.urgency,
                    inputs.bytes,
                    &priority_inputs,
                    &standard_inputs,
                    min_fee_b,
                    ci.lane_policy,
                ) else {
                    // submit_when_underwater = false and both lanes
                    // negative → skip this arrival.
                    continue;
                };
                let lane_quote = match posted_lane {
                    crate::tx_pricing::Lane::Priority => q_priority,
                    crate::tx_pricing::Lane::Standard => q_standard,
                };
                let Ok(max_fee_lovelace) =
                    ci.max_fee_policy
                        .compute(lane_quote, inputs.bytes, min_fee_b)
                else {
                    // Overflow in max_fee computation: skip this
                    // arrival. The tx_actors test suite already
                    // covers the overflow surface.
                    continue;
                };
                // Mint a unique tx_id by encoding (node_id, counter)
                // into the high/low halves of a u64. With u64 we
                // have 2^16 nodes × 2^48 txs/node — enough for any
                // M3 sim.
                let (tx_id, input_id) = {
                    let state = self.actor_state.as_mut().expect("checked above");
                    let counter = state.next_tx_id;
                    state.next_tx_id += 1;
                    let combined = ((self.id.to_inner() as u64) << 48) | (counter & 0xFFFF_FFFF_FFFF);
                    (TransactionId::new(combined), combined)
                };
                let tx = Transaction {
                    id: tx_id,
                    shard: 0,
                    bytes: inputs.bytes,
                    input_id,
                    overcollateralization_factor: 0,
                    max_fee_lovelace,
                    posted_lane,
                    value_lovelace: inputs.value_lovelace,
                    urgency: inputs.urgency,
                    urgency_component_index: ci.index,
                };
                self.tracker.track_transaction_generated(&tx, self.id);
                let arc = Arc::new(tx);
                {
                    let state = self.actor_state.as_mut().expect("checked above");
                    state.pending.insert(tx_id, (slot, ci.index));
                }
                to_submit.push(arc);
            }
        }
        // Submit each actor-built tx. We bypass `generate_tx`'s
        // tracker call (which would emit a duplicate `TXGenerated`)
        // and call `propagate_tx` directly, since we already emitted
        // `TXGenerated` above.
        for tx in to_submit {
            self.propagate_tx(self.id, tx);
        }
    }

    /// Update the LatencyEstimator for an actor-submitted tx that
    /// landed on chain. Called from `charge_inclusions` for every
    /// successful inclusion. No-op if the tx wasn't actor-generated
    /// or actor mode is off.
    fn observe_actor_inclusion(
        &mut self,
        tx_id: TransactionId,
        served_lane: Lane,
        inclusion_slot: u64,
    ) {
        let Some(state) = self.actor_state.as_mut() else {
            return;
        };
        let Some((submit_slot, comp_idx)) = state.pending.remove(&tx_id) else {
            return;
        };
        let lat_slots = inclusion_slot.saturating_sub(submit_slot) as f64;
        let lat_blocks = lat_slots * state.profile.block_generation_probability;
        if let Some(est) = state.latency.get_mut(comp_idx as usize) {
            est.observe(served_lane, lat_blocks);
        }
    }

    /// Drop a stale `pending` entry for an actor-submitted tx that
    /// failed to land (admission failed, quote-drift evicted, etc.).
    /// Keeps the pending map bounded.
    fn forget_actor_pending(&mut self, tx_id: TransactionId) {
        if let Some(state) = self.actor_state.as_mut() {
            state.pending.remove(&tx_id);
        }
    }

    /// Test-only entry point exercising `select_eb_with_partition`'s
    /// activation decision in isolation (M2 verification line 310,
    /// four cases). Returns whether the partition activated.
    /// `priority_reservation_bytes` and `rb_reserved` are kept on the
    /// signature for test-call-site stability but are unused by the
    /// trigger itself (they only matter for served-lane assignment,
    /// which lives in `assign_served_lanes`).
    #[cfg(test)]
    pub(crate) fn test_partition_trigger(
        &mut self,
        eb_capacity: u64,
        _priority_reservation_bytes: u64,
        _rb_reserved: bool,
    ) -> bool {
        let selection_order = self.pricing.lane_selection_order();
        let (_selected, activated) =
            self.select_eb_with_partition(eb_capacity, selection_order);
        activated
    }

    /// Test-only entry point for `eb_endorsement_valid` — builds a
    /// throwaway EB from the supplied txs and runs the validation
    /// guard (M2 verification line 313 / handoff §4 refuse-to-endorse).
    #[cfg(test)]
    pub(crate) fn test_eb_endorsement_valid(&self, txs: &[Arc<Transaction>]) -> bool {
        let eb = EndorserBlock {
            slot: 0,
            producer: self.id,
            bytes: self.sim_config.sizes.linear_eb(txs),
            txs: txs.to_vec(),
            partition_activated: false,
        };
        self.eb_endorsement_valid(&eb)
    }

    /// Test-only mirror of `try_generate_rb`'s endorsement-and-apply
    /// closure: validate, and (if valid) charge inclusions, remove
    /// from mempool, and feed the EB priced sample. Returns `true`
    /// iff the EB was endorseable. Used by M2 regression tests that
    /// pin the *cascade-skip* on refusal — that no charge_inclusions,
    /// no mempool removal, and no priced sample fire when an EB
    /// contains a stale tx (M1 handoff §"Known limitations" §4).
    #[cfg(test)]
    pub(crate) fn test_endorse_eb_dry_run(
        &mut self,
        eb_txs: Vec<Arc<Transaction>>,
        rb_reserved: bool,
    ) -> bool {
        let eb = EndorserBlock {
            slot: 0,
            producer: self.id,
            bytes: self.sim_config.sizes.linear_eb(&eb_txs),
            txs: eb_txs,
            // Existing M2 callers exercise refuse-to-endorse; they
            // don't depend on a particular partition decision. Default
            // to false (no partition); served-lane reduces to
            // `posted_lane` for un-reserved variants and to all-Standard
            // for RB-reserved + not-activated.
            partition_activated: false,
        };
        if !self.eb_endorsement_valid(&eb) {
            return false;
        }
        let served = self.assign_served_lanes(&eb, rb_reserved);
        let pairs: Vec<(Arc<Transaction>, Lane)> =
            eb.txs.iter().cloned().zip(served).collect();
        self.charge_inclusions(&pairs);
        self.remove_eb_txs_from_mempool(&eb);
        self.apply_eb_priced_block(&eb);
        true
    }
}

/// Build a `BlockLaneBreakdown` from a block's transactions.
/// Sums per-lane bytes and pairs them with the block's capacity for the
/// backend's `samples_for_block` policy.
fn breakdown_for(txs: &[Arc<Transaction>], block_capacity: u64) -> BlockLaneBreakdown {
    let mut priority = 0u64;
    let mut standard = 0u64;
    for tx in txs {
        match tx.posted_lane {
            Lane::Priority => priority = priority.saturating_add(tx.bytes),
            Lane::Standard => standard = standard.saturating_add(tx.bytes),
        }
    }
    BlockLaneBreakdown {
        priority_paying_bytes: priority,
        standard_paying_bytes: standard,
        block_capacity,
    }
}

struct Mempool {
    next_id: u64,
    mempool_count: usize,
    mempool_size_bytes: u64,
    max_size_bytes: u64,
    queue: BTreeMap<u64, Arc<Transaction>>,
    input_ids: HashSet<u64>,
}
impl Mempool {
    fn new(max_size_bytes: u64) -> Self {
        Self {
            next_id: 0,
            mempool_count: 0,
            mempool_size_bytes: 0,
            max_size_bytes,
            queue: BTreeMap::new(),
            input_ids: HashSet::new(),
        }
    }
    fn try_insert(&mut self, tx: Arc<Transaction>) -> bool {
        let new_bytes = self.mempool_size_bytes + tx.bytes;
        if self.mempool_count < self.queue.len() || new_bytes > self.max_size_bytes {
            // mempool is or would be full, just put this at the end and Be Done
            let id = self.new_id();
            self.queue.insert(id, tx);
            return false;
        }
        if self.input_ids.contains(&tx.input_id) {
            // conflicts with something already in the mempool
            return false;
        }

        self.mempool_count += 1;
        self.mempool_size_bytes = new_bytes;
        self.input_ids.insert(tx.input_id);
        let id = self.new_id();
        self.queue.insert(id, tx);
        true
    }

    fn ids(&self) -> impl Iterator<Item = TransactionId> {
        self.queue.values().take(self.mempool_count).map(|tx| tx.id)
    }

    // Removes a set of TXs from the mempool.
    // Returns any previously-queued TXs now added to the mempool.
    fn remove_txs(&mut self, ids: impl IntoIterator<Item = TransactionId>) -> Vec<TransactionId> {
        let id_set: HashSet<TransactionId> = ids.into_iter().collect();
        if id_set.is_empty() {
            return vec![];
        }
        let mut new_mempool_count = self.mempool_count;
        let mut full = false;
        let mut newly_added = vec![];
        let mut seen_so_far = 0;
        self.queue.retain(|_, tx| {
            let seen = seen_so_far;
            seen_so_far += 1;
            if seen < self.mempool_count {
                // we're iterating through the mempool
                if !id_set.contains(&tx.id) {
                    return true;
                }
                // this is a transaction in the mempool which we want to remove
                new_mempool_count -= 1;
                self.mempool_size_bytes -= tx.bytes;
                self.input_ids.remove(&tx.input_id);
                false
            } else {
                // we're iterating through the queued TXs which aren't yet in the mempool
                if self.input_ids.contains(&tx.input_id) {
                    // conflicts with the mempool, remove it at once
                    return false;
                }
                // add TXs until we're full
                if !full {
                    let new_size = self.mempool_size_bytes + tx.bytes;
                    if new_size > self.max_size_bytes {
                        full = true;
                    } else {
                        new_mempool_count += 1;
                        self.mempool_size_bytes = new_size;
                        self.input_ids.insert(tx.input_id);
                        newly_added.push(tx.id);
                    }
                }
                true
            }
        });
        self.mempool_count = new_mempool_count;
        newly_added
    }

    fn remove_conflicting_txs(&mut self, input_ids: &HashSet<u64>) -> Vec<TransactionId> {
        let mut new_mempool_count = self.mempool_count;
        let mut full = false;
        let mut newly_added = vec![];
        let mut seen_so_far = 0;
        self.queue.retain(|_, tx| {
            let seen = seen_so_far;
            seen_so_far += 1;
            if seen < self.mempool_count {
                // we're iterating through the mempool
                if !input_ids.contains(&tx.input_id) {
                    return true;
                }
                // this is a transaction in the mempool which we want to remove
                new_mempool_count -= 1;
                self.mempool_size_bytes -= tx.bytes;
                self.input_ids.remove(&tx.input_id);
                false
            } else {
                // we're iterating through the queued TXs which aren't yet in the mempool
                if self.input_ids.contains(&tx.input_id) || input_ids.contains(&tx.input_id) {
                    // conflicts with the ledger or the new mempool, remove it at once
                    return false;
                }
                // add TXs until we're full
                if !full {
                    let new_size = self.mempool_size_bytes + tx.bytes;
                    if new_size > self.max_size_bytes {
                        full = true;
                    } else {
                        new_mempool_count += 1;
                        self.mempool_size_bytes = new_size;
                        self.input_ids.insert(tx.input_id);
                        newly_added.push(tx.id);
                    }
                }
                true
            }
        });
        self.mempool_count = new_mempool_count;
        newly_added
    }

    fn new_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

#[cfg(test)]
mod mempool_tests {
    use std::sync::Arc;

    use crate::model::{Transaction, TransactionId};

    use super::Mempool;

    struct TxFactory {
        next_id: u64,
    }
    impl TxFactory {
        fn new() -> Self {
            Self { next_id: 0 }
        }
        fn tx(&mut self, bytes: u64) -> Arc<Transaction> {
            let id = self.next_id;
            self.next_id += 1;
            Arc::new(Transaction {
                id: TransactionId::new(id),
                shard: 0,
                bytes,
                input_id: id,
                overcollateralization_factor: 0,
                max_fee_lovelace: u64::MAX,
                posted_lane: crate::tx_pricing::Lane::Standard,
                value_lovelace: 0,
                urgency: 1.0,
                urgency_component_index: 0,
            })
        }
        fn txs<const N: usize>(&mut self, bytes: [u64; N]) -> [Arc<Transaction>; N] {
            bytes.map(|b| self.tx(b))
        }
    }

    #[test]
    fn should_fill_as_space_is_available() {
        let mut txs = TxFactory::new();
        let [tx1, tx2, tx3] = txs.txs([5, 5, 5]);
        let mut mempool = Mempool::new(10);
        assert!(mempool.try_insert(tx1.clone()));
        assert!(mempool.try_insert(tx2.clone()));

        // new TX doesn't fit
        assert!(!mempool.try_insert(tx3.clone()));
        assert_eq!(mempool.ids().collect::<Vec<_>>(), vec![tx1.id, tx2.id]);

        // until we remove a TX, and suddenly it does
        let added = mempool.remove_txs([tx2.id]);
        assert_eq!(added, vec![tx3.id]);
        assert_eq!(mempool.ids().collect::<Vec<_>>(), vec![tx1.id, tx3.id]);
    }
}
