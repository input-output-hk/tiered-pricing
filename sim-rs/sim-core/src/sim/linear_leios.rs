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
        NoVoteReason, PerLaneQuote, Transaction, TransactionId, TransactionLostReason, VoteBundle,
        VoteBundleId, WindowAggregate,
    },
    sim::{
        MiniProtocol, NodeImpl, SimCpuTask, SimMessage,
        linear_leios::attackers::{EBWithholdingEvent, EBWithholdingSender},
        lottery::{LotteryConfig, LotteryKind, MockLotteryResults, vrf_probabilities},
        mempool_gate::MempoolGate,
    },
    tx_pricing::{
        BaselinePricing, BlockKind, BlockLaneBreakdown, ChainView, Eip1559Pricing, Lane,
        LaneSelectionOrder, LaneValidityRule, PricedBlockSample, PricingBackend, TwoLanePricing,
        snapshot_at,
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    // TX propagation
    AnnounceTx(TransactionId),
    RequestTx(TransactionId),
    Tx(Arc<Transaction>),

    // RB header propagation
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
            Self::Pending { header, .. } => Some(header),
            Self::Requested { header, .. } => Some(header),
            Self::Received { rb, .. } => Some(&rb.header),
        }
    }
    fn header_seen(&self) -> Option<Timestamp> {
        match self {
            Self::Pending { header_seen, .. } => Some(*header_seen),
            Self::Requested { header_seen, .. } => Some(*header_seen),
            Self::Received { header_seen, .. } => Some(*header_seen),
        }
    }
    /// Test/chain-derived helper: the fully-validated RB if this view
    /// is in the `Received` state.
    fn received_rb(&self) -> Option<&Arc<RankingBlock>> {
        match self {
            Self::Received { rb, .. } => Some(rb),
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
    mempool: Mempool,
    /// Phase-2 fee gate: tracks per-lane bytes, fee admission, and
    /// quote-drift revalidation.
    gate: MempoolGate,
    /// Phase-2 pricing backend (chain-derived, spike 007): a pure-
    /// function policy object with no mutable controller state. The
    /// chain-derived `derived_quote` lives on each `LinearRankingBlock`
    /// in `praos.blocks`.
    pricing: Box<dyn PricingBackend>,
    /// Whether the backend is two-lane (renders both quotes in the
    /// `PricingTick` snapshot); cached once at construction so
    /// `emit_pricing_tick` doesn't dispatch dynamically per tick.
    pricing_is_two_lane: bool,
    /// Per-block samples cache for chain-derived computation. Each
    /// canonical RB's `samples_in_block` (RB body + endorsed EB) is
    /// stored here so the `ChainView` impl can serve them when a
    /// descendant computes its own `derived_quote`. Pruned at
    /// `2 × window_length` behind the chain tip in `publish_rb` to
    /// keep memory bounded (spike 007 §"Edge cases" item 1).
    block_samples: BTreeMap<BlockId, Vec<PricedBlockSample>>,
    ledger_states: BTreeMap<BlockId, Arc<LedgerState>>,
    praos: NodePraosState,
    leios: NodeLeiosState,
    behaviours: NodeBehaviours,
    /// Phase-2 actor state (M3+). Populated when the config supplies
    /// an `actors:` profile and this node has `tx_generation_weight > 0`;
    /// `None` otherwise. Each slot, `run_actors_for_slot` samples
    /// per-component arrivals and submits txs through `generate_tx`.
    actor_state: Option<NodeActorState>,
    /// Most-recent slot seen by `handle_new_slot`. Read by
    /// `generate_tx` (the lane-blind `handle_new_tx` driver path) so
    /// the `TXGenerated` event carries a real submit slot. The actor
    /// path passes its own `slot` argument directly and does not rely
    /// on this field.
    current_slot: u64,

    /// Deferred per-tx cache eviction queue, keyed by the slot at
    /// which each tx was declared terminal (included, evicted, or
    /// rejected) locally. Drained by `prune_terminal_tx_cache` once
    /// entries are older than `max_eb_age`.
    ///
    /// Populated only under `LeiosVariant::LinearWithTxReferences`:
    /// that variant must keep tx bodies cached past local
    /// terminality because incoming EBs reference txs by 32-byte
    /// hash and validators need the body to validate the EB body.
    /// Other variants either prune immediately (`Linear`, where the
    /// EB itself carries the body) or do not require the cache.
    pending_tx_evictions: BTreeMap<u64, Vec<TransactionId>>,

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

/// Producer-side staleness filter for EB packing. Predicts which
/// candidate txs would have `posted_fee > max_fee_lovelace` against
/// the worst-case lane quote at expected endorsement time, and lets
/// the producer skip them at packing time. Without this, a single
/// stale priority-fee tx in the EB causes `eb_endorsement_valid` to
/// refuse the entire EB, throwing co-resident standard-fee
/// passengers under the bus.
struct StalenessPredictor {
    priority_quote_at_endorsement: u64,
    standard_quote_at_endorsement: u64,
    min_fee_b: u64,
}

impl StalenessPredictor {
    fn is_predicted_stale(&self, tx: &Transaction) -> bool {
        let q = match tx.posted_lane {
            Lane::Priority => self.priority_quote_at_endorsement,
            Lane::Standard => self.standard_quote_at_endorsement,
        };
        // posted_fee at the projected worst-case quote.
        let predicted_fee = self.min_fee_b.saturating_add(q.saturating_mul(tx.bytes));
        predicted_fee > tx.max_fee_lovelace
    }
}

/// Expected upper bound on the number of *priced blocks* that fire
/// during the endorsement window. Producers use this as the
/// projection horizon for staleness prediction.
///
/// The endorsement window in *slots* is fixed by the protocol's
/// stage delays (header_diffusion × 3 + vote_stage + diffuse_stage,
/// ~13 slots under defaults). But the price controller only steps
/// when a priced block lands, which happens with probability
/// `block_generation_probability` per slot. So expected priced
/// blocks in the window is `μ = window_slots × rb_prob`. We use a
/// 2-sigma upper bound on Poisson(μ): `μ + 2·√μ`, ceil'd to an
/// integer step count, with a floor of 1 (always assume at least
/// one drift step is possible over a non-trivial window).
///
/// Cross-arch determinism: f64 +, − and × are correctly-rounded per
/// IEEE-754 §5.4.1; `libm::sqrt` and `libm::ceil` are software
/// implementations and therefore bit-stable across architectures.
/// (Hardware `f64::sqrt` is NOT mandated correctly-rounded by IEEE-754
/// — using `libm::sqrt` closes that gap.)
fn endorsement_window_priced_blocks(cfg: &SimConfiguration) -> u32 {
    let window_slots = cfg
        .header_diffusion_time
        .as_secs()
        .saturating_mul(3)
        .saturating_add(cfg.linear_vote_stage_length)
        .saturating_add(cfg.linear_diffuse_stage_length);
    let mu = (window_slots as f64) * cfg.block_generation_probability;
    let bound = mu + 2.0 * libm::sqrt(mu);
    let n = libm::ceil(bound) as u32;
    n.max(1)
}

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
        // Invariant: mempool.max_size_bytes == gate.max_total_size_bytes.
        // The construction above enforces this; the explicit assert
        // surfaces any future drift (e.g. a "soft cap" knob added to
        // one but not the other) that would silently reopen the
        // queue-bypass path described in `Mempool::try_insert`.
        debug_assert_eq!(
            mempool_max_size_bytes,
            gate.config().max_total_size_bytes,
            "mempool byte cap must equal gate byte cap to keep the gate the sole byte-cap authority"
        );
        let (pricing, pricing_is_two_lane): (Box<dyn PricingBackend>, bool) =
            match sim_config.pricing_config() {
                PricingConfig::Baseline => (
                    Box::new(BaselinePricing::new(
                        sim_config.mempool_gate_config().min_fee_a,
                    )),
                    false,
                ),
                PricingConfig::Eip1559(settings) => (
                    Box::new(
                        Eip1559Pricing::new(settings.clone())
                            .expect("Eip1559Settings validated at config build time"),
                    ),
                    false,
                ),
                PricingConfig::TwoLane(settings) => (
                    Box::new(
                        TwoLanePricing::new(settings.clone())
                            .expect("TwoLaneSettings validated at config build time"),
                    ),
                    true,
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
            pricing_is_two_lane,
            block_samples: BTreeMap::new(),
            ledger_states: BTreeMap::new(),
            praos: NodePraosState::default(),
            leios: NodeLeiosState::default(),
            behaviours: config.behaviours.clone(),
            actor_state,
            current_slot: 0,
            pending_tx_evictions: BTreeMap::new(),
            eb_withholding_sender: None,
            eb_withholding_event_source: None,
        }
    }

    fn custom_event_source(&mut self) -> Option<mpsc::UnboundedReceiver<Self::CustomEvent>> {
        self.eb_withholding_event_source.take()
    }

    fn handle_new_slot(&mut self, slot: u64) -> EventResult {
        self.current_slot = slot;
        self.prune_old_leios_state();
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
        self.tracker
            .track_transaction_generated(&tx, self.id, self.current_slot);
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
        if !referenced_by_eb && !added_to_mempool {
            if from == self.id {
                self.tracker
                    .track_transaction_lost(id, TransactionLostReason::MempoolRejected);
            }
            self.forget_actor_pending(id);
            self.forget_terminal_tx(id);
        }

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
                let pairs: Vec<(Arc<Transaction>, Lane)> =
                    eb.txs.iter().cloned().zip(served).collect();
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
                self.tracker.track_transaction_generated(&tx, self.id, slot);
                rb_transactions.push(Arc::new(tx));
            } else {
                // RB body: txs are charged inclusions immediately
                // (no staleness risk — no time between packing and
                // inclusion), so no staleness filter.
                self.sample_from_mempool_lane_aware(
                    &mut rb_transactions,
                    self.sim_config.max_block_size,
                    None,
                    true,
                    validity_rule,
                    selection_order,
                    None,
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
                    self.tracker.track_transaction_generated(&tx, self.id, slot);
                    eb_transactions.push(Arc::new(tx));
                }
            } else {
                // M3: pack the EB body and record the producer's
                // two-trigger partition decision in one step. The
                // endorser reuses `eb.partition_activated` via
                // `assign_served_lanes`; producer and endorser agree
                // by construction.
                let (packed, activated) =
                    self.select_eb_with_partition(self.sim_config.max_eb_size, selection_order);
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

        // Chain-derived controller (spike 007): compute the new RB's
        // `derived_quote` and `window_aggregate` as a pure function of
        // the parent's chain-derived state + samples emitted by the
        // parent. This replaces the legacy node-local accumulator.
        // The block_samples cache is populated below from the new RB
        // (and its endorsed EB, if any).
        let (derived_quote, window_aggregate) =
            self.compute_chain_derived_quote_for_child_of(parent);

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
            derived_quote,
            window_aggregate,
        };

        // Producer charges its own RB body's transactions for inclusion.
        // RB body served-lane policy:
        // - RB-reserved variants (priority-only RB): served_lane = Priority.
        // - Un-reserved (single-lane and un-reserved two-lane): served_lane = posted_lane.
        //
        // Chain-derived: charge at the NEW block's `derived_quote`, not
        // the parent's — the new block is the canonical reference for
        // its own body. This matches spike 007's design (the controller's
        // "future" is fixed at production).
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
        self.charge_inclusions_at(
            &rb_pairs,
            rb.derived_quote.standard,
            rb.derived_quote.priority,
        );

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
        // Chain-derived: read quotes from the canonical chain tip's
        // `derived_quote`, not from a node-local accumulator. The
        // staleness predictor logic upstream uses the same chain-tip
        // source so producer and endorser agree by construction.
        let q_standard = self.current_chain_tip_quote(Lane::Standard);
        let q_priority = self.current_chain_tip_quote(Lane::Priority);
        let min_fee_b = self.gate.config().min_fee_b;
        let eb_inclusions_pay_standard = self.pricing.eb_inclusions_pay_standard();
        for tx in &eb.txs {
            let q = if eb_inclusions_pay_standard {
                q_standard
            } else {
                match tx.posted_lane {
                    Lane::Standard => q_standard,
                    Lane::Priority => q_priority,
                }
            };
            let posted_fee = q
                .checked_mul(tx.bytes)
                .and_then(|x| x.checked_add(min_fee_b));
            match posted_fee {
                Some(fee) if fee <= tx.max_fee_lovelace => continue,
                Some(_) => return false,
                None => {
                    // Genuine arithmetic overflow — distinct from
                    // "tx's max_fee_lovelace was exceeded". Refusing the
                    // endorsement is still correct (the EB cannot ship),
                    // but the overflow is a pathological-config signal
                    // we want diagnosable in the log stream rather than
                    // silently conflated with staleness.
                    tracing::warn!(
                        "EB endorsement skipped due to fee arithmetic overflow: \
                         tx={:?} q={} bytes={} min_fee_b={}",
                        tx.id,
                        q,
                        tx.bytes,
                        min_fee_b
                    );
                    return false;
                }
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
    /// - Giorgos RB-reserved both-dynamic: every EB tx is charged at
    ///   the standard lane while priority-posted EB bytes still feed
    ///   the priority controller sample.
    /// - RB-reserved + activated: priority-fee txs whose cumulative
    ///   bytes ≤ `priority_reservation_bytes` get `Priority`; further
    ///   priority txs and all standard txs get `Standard`.
    /// - RB-reserved + NOT activated: all priority-fee txs get
    ///   `Standard` (refunded down to standard fee per spec).
    fn assign_served_lanes(&self, eb: &EndorserBlock, rb_reserved: bool) -> Vec<Lane> {
        if self.pricing.eb_inclusions_pay_standard() {
            return vec![Lane::Standard; eb.txs.len()];
        }
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
                    // RB headers are the latency-critical object for
                    // Linear Leios voting. Push the 1 KB header
                    // directly instead of spending an extra
                    // announce/request round trip on every hop.
                    Message::RBHeader(
                        rb.header.clone(),
                        true,
                        rb.header.eb_announcement.is_some() && !self.should_withhold_ebs(),
                    )
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
        // Chain-derived controller (spike 007): the RB carries its own
        // `derived_quote` and `window_aggregate` (computed at production
        // and stored on the block). Insert into the chain, cache the
        // block's emitted samples for descendants, prune the cache
        // tail, and revalidate the mempool gate against the new tip.
        //
        // `track_linear_pricing_sample_applied` becomes a "sibling-
        // pair-fully-validated-at-this-node" event for orphan-rate
        // observability. Under chain-derivation the counter is no
        // longer a contamination-bound signal — sibling blocks
        // produce identical `derived_quote` by pure-function
        // reasoning, so even fully-validated orphans cannot contaminate
        // the canonical chain's controller trajectory.
        self.tracker.track_linear_pricing_sample_applied(
            self.id,
            rb.header.id.slot,
            rb.header.id.producer,
        );
        // Cache the samples this block emitted so descendants can read
        // them via `ChainView::samples_in_block` when computing their
        // own `derived_quote`.
        let block_samples = self.samples_for_rb(&rb);
        let rb_id = rb.header.id;
        self.block_samples.insert(rb_id, block_samples);
        let slot = rb_id.slot;
        self.praos
            .blocks
            .insert(rb_id, RankingBlockView::Received { rb, header_seen });
        // Bound memory: prune samples older than 2 × window_length
        // behind the chain tip. The terminal-tx cache (populated by
        // `forget_terminal_tx` under `LinearWithTxReferences`) is
        // pruned separately by EB max-age — see
        // `prune_terminal_tx_cache`.
        self.prune_block_samples();
        self.prune_terminal_tx_cache();
        // Revalidate the gate against the new tip's `derived_quote`.
        self.revalidate_against_new_tip(slot);
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
            // Forward the header itself. The relay may not yet have
            // the RB body or announced EB, so receivers learn the
            // chain head promptly and fetch the heavier objects from a
            // peer that advertises them.
            self.queued
                .send_to(*peer, Message::RBHeader(header.clone(), false, false));
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
            // Chain-derived: the certifying RB's `derived_quote` is
            // already canonical (computed at production and stored on
            // the block). What the deferred EB validation gives us is
            // the chance to populate the `block_samples` cache entry
            // for the parent RB with the full sample set (RB body +
            // EB body), so any descendants we produce on top of this
            // RB read the correct samples. Find the RB carrying this
            // EB on our canonical chain and update its cached samples.
            let eb_id = eb.id();
            // Locate the RB that endorsed this EB on our chain.
            let mut parent_rb_id: Option<BlockId> = None;
            for (block_id, view) in self.praos.blocks.iter() {
                if let Some(rb) = view.received_rb()
                    && rb.endorsement.as_ref().is_some_and(|e| e.eb == eb_id)
                {
                    parent_rb_id = Some(*block_id);
                    break;
                }
            }
            if let Some(rb_id) = parent_rb_id
                && let Some(rb_arc) = self
                    .praos
                    .blocks
                    .get(&rb_id)
                    .and_then(|v| v.received_rb())
                    .cloned()
            {
                // Recompute samples_for_rb now that the EB is
                // validated — it will now include the EB body.
                let samples = self.samples_for_rb(&rb_arc);
                self.block_samples.insert(rb_id, samples);
            }
            // Gate revalidation: the chain tip's `derived_quote` may
            // not have changed (it was set at production), but a
            // newly-canonical EB-included tx might already have been
            // included on the producer side and the gate needs to be
            // resynced.
            let slot = (self.clock.now() - Timestamp::zero()).as_secs();
            self.revalidate_against_new_tip(slot);
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
            self.tracker.track_transaction_generated(&tx, self.id, slot);
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
        // mempool byte cap would be exceeded. Chain-derived: the
        // quote comes from the canonical chain tip's `derived_quote`.
        let quote = self.current_chain_tip_quote(tx.posted_lane);
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
    /// `max_count`, when present, caps the number of selected tx
    /// references independently of transaction bytes. This models the
    /// linear-with-tx-references split between `S_EB` (EB wire object)
    /// and `S_EB-tx` (referenced transaction bytes).
    fn sample_from_mempool_lane_aware(
        &mut self,
        txs: &mut Vec<Arc<Transaction>>,
        max_size: u64,
        max_count: Option<usize>,
        remove: bool,
        validity_rule: LaneValidityRule,
        selection_order: LaneSelectionOrder,
        staleness_filter: Option<&StalenessPredictor>,
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
        // Staleness rejections `continue` (the next candidate may not
        // be stale).
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
            if let Some(predictor) = staleness_filter
                && predictor.is_predicted_stale(tx)
            {
                continue;
            }
            if max_count.is_some_and(|limit| txs.len() >= limit) {
                break;
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
        // Producer-side staleness filter: skip txs that won't survive
        // worst-case controller drift between EB build time and
        // expected endorsement time. Without this, a single mis-priced
        // priority-fee tx in the EB causes `eb_endorsement_valid` to
        // refuse the entire EB, taking all co-resident standard-fee
        // passengers down with it.
        let endorsement_window_blocks = endorsement_window_priced_blocks(&self.sim_config);
        // Chain-derived staleness projection: read the chain tip's
        // `derived_quote` per lane, then project N max-up steps
        // forward. CR-1's `endorsement_window_priced_blocks` math is
        // unchanged (`libm::sqrt` / `libm::ceil`, bit-stable across
        // architectures).
        let priority_now = self.current_chain_tip_quote(Lane::Priority);
        let standard_now = self.current_chain_tip_quote(Lane::Standard);
        let predictor = StalenessPredictor {
            priority_quote_at_endorsement: self.worst_case_quote_for_staleness(
                Lane::Priority,
                priority_now,
                endorsement_window_blocks,
            ),
            standard_quote_at_endorsement: self.worst_case_quote_for_staleness(
                Lane::Standard,
                standard_now,
                endorsement_window_blocks,
            ),
            min_fee_b: self.gate.config().min_fee_b,
        };

        // Pack the EB greedily under the configured selection order.
        let mut packed: Vec<Arc<Transaction>> = Vec::new();
        let max_reference_count = self
            .sim_config
            .sizes
            .linear_eb_reference_count_limit(self.sim_config.max_eb_wire_size);
        self.sample_from_mempool_lane_aware(
            &mut packed,
            eb_capacity,
            max_reference_count,
            false, // don't drain the mempool — selection happens elsewhere
            LaneValidityRule::None,
            selection_order,
            Some(&predictor),
        );
        let selected_bytes: u64 = packed.iter().map(|t| t.bytes).sum();
        let residual = eb_capacity.saturating_sub(selected_bytes);
        let selected_refs = packed.len();
        let refs_saturated = max_reference_count.is_some_and(|limit| selected_refs >= limit);

        // Two-trigger activation rule.
        let activated = if selected_bytes >= eb_capacity || refs_saturated {
            true
        } else {
            let packed_ids: HashSet<_> = packed.iter().map(|t| t.id).collect();
            let mut any_unselected = false;
            let mut any_fits = false;
            let residual_refs = max_reference_count
                .map(|limit| limit.saturating_sub(selected_refs))
                .unwrap_or(usize::MAX);
            for id in self.mempool.ids() {
                if packed_ids.contains(&id) {
                    continue;
                }
                let Some(TransactionView::Received(tx)) = self.txs.get(&id) else {
                    continue;
                };
                any_unselected = true;
                if residual_refs > 0 && tx.bytes <= residual {
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
    fn charge_inclusions(&mut self, txs_with_served_lane: &[(Arc<Transaction>, Lane)]) {
        // Chain-derived: source served-lane quotes from the canonical
        // chain tip's `derived_quote`. Used by EB inclusion paths
        // (`try_generate_rb`'s endorsement branch and
        // `test_endorse_eb_dry_run`) where the consuming block is the
        // chain tip itself, not a newly-produced RB.
        let q_standard = self.current_chain_tip_quote(Lane::Standard);
        let q_priority = self.current_chain_tip_quote(Lane::Priority);
        self.charge_inclusions_at(txs_with_served_lane, q_standard, q_priority);
    }

    /// Like `charge_inclusions`, but with the served-lane quotes
    /// supplied explicitly. Used by the RB-body inclusion path inside
    /// `try_generate_rb`, where the producer charges its own block's
    /// txs at the NEW block's `derived_quote` (spike 007: "the values
    /// used for tx admissibility are the new block's own `derived_quote`
    /// — the controller's future is fixed at the moment of production").
    fn charge_inclusions_at(
        &mut self,
        txs_with_served_lane: &[(Arc<Transaction>, Lane)],
        q_standard: u64,
        q_priority: u64,
    ) {
        if txs_with_served_lane.is_empty() {
            return;
        }
        let slot = (self.clock.now() - Timestamp::zero()).as_secs();
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
                self.forget_terminal_tx(tx.id);
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
            self.forget_terminal_tx(tx.id);
        }
    }

    /// Chain-derived controller (spike 007): compute the `derived_quote`
    /// and `window_aggregate` for a new block whose parent is `parent`.
    ///
    /// Pure function of (parent's chain-derived state, samples emitted
    /// by the parent, samples falling off the tail of the window). No
    /// per-node mutable accumulator is involved — orphan blocks from
    /// slot battles cannot contaminate the canonical chain's controller
    /// trajectory because their `derived_quote` is discarded along with
    /// the block.
    ///
    /// Cold start (no parent): use the backend's `cold_start_quote` for
    /// each lane and start from `WindowAggregate::ZERO`.
    fn compute_chain_derived_quote_for_child_of(
        &self,
        parent: Option<BlockId>,
    ) -> (PerLaneQuote, WindowAggregate) {
        let parent_quote = parent
            .and_then(|id| self.derived_quote(id))
            .unwrap_or_else(|| PerLaneQuote {
                standard: self.pricing.cold_start_quote(Lane::Standard),
                priority: self.pricing.cold_start_quote(Lane::Priority),
            });
        let parent_aggregate = parent
            .and_then(|id| self.window_aggregate(id))
            .unwrap_or(WindowAggregate::ZERO);
        // Samples emitted by the parent (RB body + endorsed EB, per
        // the variant's samples_for_block policy). Empty during cold
        // start.
        let parent_samples: Vec<PricedBlockSample> = parent
            .map(|id| self.samples_in_block(id).to_vec())
            .unwrap_or_default();
        // Samples falling off the tail: the block at distance
        // `window_length + 1` back. None during the warm-up regime.
        let window_length = self.pricing.effective_window_length();
        let evicted_samples: Vec<PricedBlockSample> = parent
            .filter(|_| window_length != usize::MAX)
            .and_then(|p| {
                // Need `window_length` ancestors back from `parent`,
                // i.e. the block whose samples roll off when we add
                // the parent's. That is the (window_length-1)-ancestor
                // of `parent` — its samples were in the window for
                // `window_length` consecutive blocks ending at parent,
                // and the new child's window no longer includes them.
                let k = u32::try_from(window_length).ok()?;
                let ancestor_id = self.ancestor(p, k)?;
                Some(self.samples_in_block(ancestor_id).to_vec())
            })
            .unwrap_or_default();
        self.pricing.compute_derived_quote(
            parent_quote,
            parent_aggregate,
            &parent_samples,
            &evicted_samples,
        )
    }

    /// Project the worst-case quote for `lane` after `blocks_ahead`
    /// max-up controller steps from `current_quote`. Reads the
    /// backend's static configuration (D, etc.) via the
    /// `PricingConfig` enum on `sim_config`; the chain-tip quote
    /// itself is supplied by the caller via `current_quote`. Pure
    /// math; no node-local state.
    fn worst_case_quote_for_staleness(
        &self,
        lane: Lane,
        current_quote: u64,
        blocks_ahead: u32,
    ) -> u64 {
        match self.sim_config.pricing_config() {
            PricingConfig::Baseline => current_quote, // flat fee — no drift
            PricingConfig::Eip1559(settings) => {
                crate::tx_pricing::single_lane::worst_case_eip1559_quote(
                    current_quote,
                    settings.max_change_denominator,
                    blocks_ahead,
                )
            }
            PricingConfig::TwoLane(settings) => {
                let d = match lane {
                    Lane::Priority => settings.priority.max_change_denominator,
                    Lane::Standard if !settings.variant.standard_dynamic() => {
                        // c_standard pinned; no drift.
                        return current_quote;
                    }
                    Lane::Standard => settings.standard.max_change_denominator,
                };
                crate::tx_pricing::single_lane::worst_case_eip1559_quote(
                    current_quote,
                    d,
                    blocks_ahead,
                )
            }
        }
    }

    /// Read the quote that consumers (admission, lane choice, EB
    /// endorsement validation, EB inclusion charging) should use against
    /// the current canonical chain tip.
    ///
    /// Returns `tip.derived_quote.get(lane)` for the canonical chain tip
    /// — the same value every node sees once the tip's RB header is on
    /// chain. Falls back to the backend cold-start initial quote when
    /// there is no canonical RB yet (genesis path).
    ///
    /// Protocol-soundness rationale: the quote a user signs against
    /// (`max_fee_lovelace`) must equal the quote the network uses to
    /// evaluate that transaction. Reading the canonical
    /// `rb.derived_quote` directly gives a quote that is a pure
    /// function of the canonical chain — every node agrees by
    /// construction. The previous hypothetical-child-of-tip path
    /// (`compute_chain_derived_quote_for_child_of`) read the node-local
    /// mutable `block_samples` cache, which mutates when deferred
    /// Endorser Blocks (EBs) finally validate; that produced per-node
    /// divergence at the same canonical chain tip and violated
    /// EIP-1559 protocol fidelity (cf. spike 007 chain-derived design).
    ///
    /// RB body inclusion charging is unaffected: the producer charges
    /// the RB body against the new RB's own `rb.derived_quote` (the
    /// post-step value computed for that block at production, see
    /// `produce_rb`). Producer-side admission against the previous
    /// canonical tip's quote and consumer-side validation of the new
    /// block both agree on the canonical-tip quote everywhere.
    fn current_chain_tip_quote(&self, lane: Lane) -> u64 {
        self.latest_rb_id()
            .and_then(|id| self.praos.blocks.get(&id))
            .and_then(|view| view.received_rb())
            .map(|rb| rb.derived_quote.get(lane))
            .unwrap_or_else(|| self.pricing.cold_start_quote(lane))
    }

    /// Read the chain tip's `window_aggregate`. Empty `ZERO` aggregate
    /// at cold start.
    fn current_chain_tip_aggregate(&self) -> WindowAggregate {
        self.latest_rb_id()
            .and_then(|id| self.praos.blocks.get(&id))
            .and_then(|view| view.received_rb())
            .map(|rb| rb.window_aggregate)
            .unwrap_or(WindowAggregate::ZERO)
    }

    /// Build priced-block samples for this RB (RB body + endorsed EB
    /// when locally validated). The returned slice is what canonical
    /// descendants will fold into their own `compute_derived_quote`
    /// inputs via the `ChainView::samples_in_block` lookup.
    fn samples_for_rb(&self, rb: &RankingBlock) -> Vec<PricedBlockSample> {
        let mut samples: Vec<PricedBlockSample> = Vec::new();
        if !rb.transactions.is_empty() {
            let breakdown = breakdown_for(&rb.transactions, self.sim_config.max_block_size);
            samples.extend(
                self.pricing
                    .samples_for_block(BlockKind::RankingBlock, &breakdown),
            );
        }
        if let Some(endorsement) = &rb.endorsement
            && let Some(eb) = self.get_validated_eb(endorsement.eb)
        {
            let breakdown = breakdown_for(&eb.txs, self.sim_config.max_eb_size);
            samples.extend(
                self.pricing
                    .samples_for_block(BlockKind::EndorserBlock, &breakdown),
            );
        }
        samples
    }

    /// Prune the `block_samples` cache to bound memory at
    /// `2 × window_length` behind the chain tip. Called from
    /// `publish_rb` after the new RB lands.
    fn prune_block_samples(&mut self) {
        let window_length = self.pricing.effective_window_length();
        if window_length == usize::MAX {
            // Baseline has no window — nothing to retain.
            self.block_samples.clear();
            return;
        }
        let Some(tip_id) = self.latest_rb_id() else {
            return;
        };
        // Walk back 2 × window_length from the tip; anything older can
        // be evicted (it cannot contribute to any future descendant's
        // `compute_derived_quote` since the eviction window is
        // window_length back).
        let cap = u32::try_from(window_length.saturating_mul(2)).unwrap_or(u32::MAX);
        let keep_from_slot = match self.ancestor(tip_id, cap) {
            Some(boundary) => boundary.slot,
            None => return, // chain shorter than 2×window — nothing to prune
        };
        self.block_samples.retain(|id, _| id.slot >= keep_from_slot);
    }

    /// Revalidate the mempool gate against the new canonical chain
    /// tip's `derived_quote`. Emits `TXEvictedQuoteDrift` events for
    /// txs whose lane quote has drifted above their `max_fee_lovelace`.
    /// Called from `publish_rb` after the new RB is inserted into
    /// `self.praos.blocks` so the chain tip read is consistent.
    fn revalidate_against_new_tip(&mut self, slot: u64) {
        let q_standard = self.current_chain_tip_quote(Lane::Standard);
        let q_priority = self.current_chain_tip_quote(Lane::Priority);
        let evicted = self.gate.revalidate(|lane| match lane {
            Lane::Standard => q_standard,
            Lane::Priority => q_priority,
        });
        if evicted.is_empty() {
            return;
        }
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
            if let Some(TransactionView::Received(tx)) = self.txs.get(&record.tx_id) {
                input_ids.insert(tx.input_id);
            }
            self.forget_terminal_tx(record.tx_id);
        }
        if !input_ids.is_empty() {
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
        // events from `revalidate_against_new_tip` (chain-derived).
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
        for tx in txs {
            self.forget_terminal_tx(tx.id);
        }
    }

    /// Bound the per-node transaction propagation cache once a tx is
    /// terminal for this node.
    ///
    /// Under `LeiosVariant::Linear` the full-body EBs/RBs carry the tx
    /// payloads in block structures, so old standalone tx cache
    /// entries are not needed after admission rejection, inclusion, or
    /// eviction — drop them immediately.
    ///
    /// Under `LeiosVariant::LinearWithTxReferences` EB validation and
    /// voting depend on the standalone tx cache (EBs reference txs by
    /// 32-byte hash and validators need the body), so an immediate
    /// drop is unsafe. Defer eviction until the tx is older than
    /// `eb_max_age_slots`; after that no future EB can reference it,
    /// so the body is safe to drop. `prune_terminal_tx_cache` drains
    /// the queue.
    fn forget_terminal_tx(&mut self, tx_id: TransactionId) {
        match self.sim_config.variant {
            LeiosVariant::Linear => {
                self.txs.remove(&tx_id);
            }
            LeiosVariant::LinearWithTxReferences => {
                self.pending_tx_evictions
                    .entry(self.current_slot)
                    .or_default()
                    .push(tx_id);
            }
            _ => {}
        }
    }

    /// Drain the deferred-eviction queue populated by
    /// `forget_terminal_tx` under `LeiosVariant::LinearWithTxReferences`,
    /// removing tx-cache entries for txs that became terminal more
    /// than `eb_max_age_slots` slots behind the current slot. Called
    /// from `publish_rb` after the new RB lands, alongside
    /// `prune_block_samples`. No-op for variants that don't populate
    /// the queue.
    ///
    /// The horizon is `eb_max_age_slots` (config default 100 slots)
    /// rather than the `2 × window_length` controller-window horizon
    /// used by `prune_block_samples`. Per CIP-0164, an EB cannot be
    /// referenced past `eb_max_age_slots` after its production, and a
    /// tx terminal at slot S can only be referenced by EBs produced
    /// at slots ≤ S + eb_max_age_slots. So a tx terminal more than
    /// `eb_max_age_slots` ago at the current node cannot be
    /// referenced by any future EB the node might receive, and its
    /// body can be dropped from the standalone cache. Using the
    /// controller-window horizon instead would keep ~13× more cache
    /// entries than necessary under the default config — an OOM
    /// risk under sustained over-capacity demand.
    fn prune_terminal_tx_cache(&mut self) {
        if self.pending_tx_evictions.is_empty() {
            return;
        }
        let horizon = self.sim_config.max_eb_age;
        let keep_from_slot = self.current_slot.saturating_sub(horizon);
        // `BTreeMap::split_off` returns entries with keys ≥ split key;
        // the older entries remain in `self.pending_tx_evictions` to
        // be drained.
        let to_keep = self.pending_tx_evictions.split_off(&keep_from_slot);
        let to_prune = std::mem::replace(&mut self.pending_tx_evictions, to_keep);
        for (_slot, ids) in to_prune {
            for id in ids {
                self.txs.remove(&id);
            }
        }
    }

    /// Bound EB/vote-side bookkeeping by the protocol's EB age
    /// horizon. In `LinearWithTxReferences`, an in-memory EB carries
    /// the full referenced tx list even though the wire object carries
    /// references, so retaining old EBs dominates overcapacity memory.
    ///
    /// Keep three classes of older EBs:
    /// - the recent EB-age/endorsement window, because they can still
    ///   gather votes or be endorsed;
    /// - EBs referenced by an on-chain RB whose body is still
    ///   incomplete locally, because deferred validation may still
    ///   need to update samples and mempool state;
    /// - EBs whose full body validation task has already been
    ///   scheduled but not marked validated yet, because the CPU task
    ///   will call back into `finish_validating_eb`.
    fn prune_old_leios_state(&mut self) {
        let horizon = self
            .sim_config
            .max_eb_age
            .saturating_add(self.sim_config.endorsement_window_slots());
        let keep_from_slot = self.current_slot.saturating_sub(horizon);
        let incomplete_onchain_ebs = self.leios.incomplete_onchain_ebs.clone();

        self.leios.ebs.retain(|id, view| {
            id.slot >= keep_from_slot
                || incomplete_onchain_ebs.contains(id)
                || matches!(
                    view,
                    EndorserBlockView::Received {
                        all_txs_seen: true,
                        validated: false,
                        ..
                    }
                )
        });
        self.leios
            .ebs_by_rb
            .retain(|rb_id, _| rb_id.slot >= keep_from_slot);
        self.leios
            .eb_peer_announcements
            .retain(|id, _| id.slot >= keep_from_slot || incomplete_onchain_ebs.contains(id));
        self.leios.votes.retain(|id, _| id.slot >= keep_from_slot);
        self.leios
            .votes_by_eb
            .retain(|id, _| id.slot >= keep_from_slot || incomplete_onchain_ebs.contains(id));
        self.leios
            .certified_ebs
            .retain(|id| id.slot >= keep_from_slot || incomplete_onchain_ebs.contains(id));
        self.leios.missing_txs.retain(|_, eb_ids| {
            eb_ids.retain(|id| id.slot >= keep_from_slot || incomplete_onchain_ebs.contains(id));
            !eb_ids.is_empty()
        });
    }

    fn resolve_ledger_state(&mut self, rb_ref: Option<BlockId>) -> Arc<LedgerState> {
        let Some(block_id) = rb_ref else {
            return Arc::new(LedgerState::default());
        };
        if let Some(state) = self.ledger_states.get(&block_id) {
            return state.clone();
        };

        let parent = self
            .praos
            .blocks
            .get(&block_id)
            .and_then(|view| view.received_rb())
            .and_then(|rb| rb.header.parent);
        let mut state = parent
            .and_then(|parent| self.ledger_states.get(&parent))
            .map(|state| state.as_ref().clone())
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
                            if matches!(self.sim_config.variant, LeiosVariant::Linear)
                                || self.has_tx(tx.id)
                            {
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
            // `LedgerState` contains the cumulative spent-input set.
            // Keeping one full snapshot per RB makes memory grow with
            // chain length times ledger size under overload. The only
            // production caller asks for the current chain tip, and
            // the cached state's `seen_blocks` lets the next tip be
            // computed incrementally from this snapshot, so retain just
            // the newest complete snapshot.
            self.ledger_states.clear();
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

    /// Test-only inspection: a fresh `PricingSnapshot` derived from
    /// the canonical chain tip's `derived_quote` + `window_aggregate`.
    /// Used by M2 deterministic scenario tests (line 313 standard-
    /// isolation assertion etc.).
    #[cfg(test)]
    pub(crate) fn pricing_snapshot(&self) -> crate::tx_pricing::PricingSnapshot {
        let derived_quote = PerLaneQuote {
            standard: self.current_chain_tip_quote(Lane::Standard),
            priority: self.current_chain_tip_quote(Lane::Priority),
        };
        let aggregate = self.current_chain_tip_aggregate();
        snapshot_at(derived_quote, aggregate, self.pricing_is_two_lane)
    }

    /// Test-only inspection: the consumer-visible chain-tip quote for
    /// `lane`. Mirrors what admission, lane choice, EB endorsement
    /// validation, and EB inclusion charging actually read.
    #[cfg(test)]
    pub(crate) fn current_chain_tip_quote_for_test(&self, lane: Lane) -> u64 {
        self.current_chain_tip_quote(lane)
    }

    /// Test-only inspection: the chain tip's stored `derived_quote`
    /// for `lane` — i.e., the value computed at the tip's production
    /// and carried on chain. Used by the regression test to
    /// demonstrate that `current_chain_tip_quote_for_test` reads the
    /// canonical block value rather than recomputing a hypothetical
    /// child quote from node-local cached samples.
    #[cfg(test)]
    pub(crate) fn chain_tip_stored_derived_quote_for_test(&self, lane: Lane) -> Option<u64> {
        let tip = self.latest_rb_id()?;
        self.praos
            .blocks
            .get(&tip)
            .and_then(|view| view.received_rb())
            .map(|rb| rb.derived_quote.get(lane))
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
        // Chain-derived: render the snapshot from the canonical chain
        // tip's `derived_quote` + `window_aggregate`, not from a per-
        // node accumulator. Cold start uses the backend's
        // `cold_start_quote` values for both lanes.
        let derived_quote = PerLaneQuote {
            standard: self.current_chain_tip_quote(Lane::Standard),
            priority: self.current_chain_tip_quote(Lane::Priority),
        };
        let aggregate = self.current_chain_tip_aggregate();
        let snapshot = snapshot_at(derived_quote, aggregate, self.pricing_is_two_lane);
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
        // sample, build, and submit. Chain-derived: quotes come from
        // the canonical chain tip's `derived_quote`, not from a
        // per-node accumulator.
        let q_priority = self.current_chain_tip_quote(crate::tx_pricing::Lane::Priority);
        let q_standard = self.current_chain_tip_quote(crate::tx_pricing::Lane::Standard);
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
            half_life_seconds: crate::probability::FloatDistribution,
            seconds_per_block: f64,
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
                    arrival_rate: c.arrival_rate_per_slot.rate_at_slot(slot),
                    size_bytes: c.size_bytes,
                    value_lovelace: c.value_lovelace,
                    half_life_seconds: c.half_life_seconds,
                    seconds_per_block: c.seconds_per_block,
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
                arrival_rate_per_slot: crate::tx_actors::ArrivalRate::Constant(ci.arrival_rate),
                size_bytes: ci.size_bytes,
                value_lovelace: ci.value_lovelace,
                half_life_seconds: ci.half_life_seconds,
                seconds_per_block: ci.seconds_per_block,
                lane_policy: ci.lane_policy,
                max_fee_policy: ci.max_fee_policy,
                target_inclusion_blocks_priority: ci.priority_latency,
                target_inclusion_blocks_standard: ci.standard_latency,
            };
            let count = {
                let state = self.actor_state.as_mut().expect("checked above");
                comp.sample_arrival_count(&mut state.component_rngs[i], slot)
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
                let max_fee_ctx = crate::tx_actors::MaxFeeContext {
                    expected_wait_blocks: match posted_lane {
                        crate::tx_pricing::Lane::Priority => ci.priority_latency,
                        crate::tx_pricing::Lane::Standard => ci.standard_latency,
                    },
                };
                let Ok(max_fee_lovelace) =
                    ci.max_fee_policy
                        .compute(lane_quote, inputs.bytes, min_fee_b, max_fee_ctx)
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
                    let combined =
                        ((self.id.to_inner() as u64) << 48) | (counter & 0xFFFF_FFFF_FFFF);
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
                self.tracker.track_transaction_generated(&tx, self.id, slot);
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
        let (_selected, activated) = self.select_eb_with_partition(eb_capacity, selection_order);
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

    /// Test-only entry point for the EB served-lane policy.
    #[cfg(test)]
    pub(crate) fn test_eb_served_lanes(
        &self,
        txs: &[Arc<Transaction>],
        rb_reserved: bool,
        partition_activated: bool,
    ) -> Vec<Lane> {
        let eb = EndorserBlock {
            slot: 0,
            producer: self.id,
            bytes: self.sim_config.sizes.linear_eb(txs),
            txs: txs.to_vec(),
            partition_activated,
        };
        self.assign_served_lanes(&eb, rb_reserved)
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
        let pairs: Vec<(Arc<Transaction>, Lane)> = eb.txs.iter().cloned().zip(served).collect();
        self.charge_inclusions(&pairs);
        self.remove_eb_txs_from_mempool(&eb);
        // Chain-derived: an EB body does not mutate a per-node
        // controller. The next RB produced on top of this chain tip
        // will fold the parent RB's samples (including this EB's
        // samples, if the parent endorsed it) into its
        // `derived_quote`. The test's pricing-snapshot assertions
        // therefore check the chain-tip-derived snapshot, which is
        // stable across EB-only operations.
        true
    }
}

/// ChainView impl — read-only view of the canonical chain exposed to
/// pure-function compute steps. The backend never gets a mutable
/// reference to the simulator; this trait is the only seam.
impl ChainView for LinearLeiosNode {
    fn ancestor(&self, from: BlockId, k: u32) -> Option<BlockId> {
        let mut current = Some(from);
        for _ in 0..k {
            let id = current?;
            let view = self.praos.blocks.get(&id)?;
            let header = view.header()?;
            current = header.parent;
        }
        current
    }

    fn samples_in_block(&self, block_id: BlockId) -> &[PricedBlockSample] {
        self.block_samples
            .get(&block_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn derived_quote(&self, block_id: BlockId) -> Option<PerLaneQuote> {
        self.praos
            .blocks
            .get(&block_id)
            .and_then(|v| v.received_rb())
            .map(|rb| rb.derived_quote)
    }

    fn window_aggregate(&self, block_id: BlockId) -> Option<WindowAggregate> {
        self.praos
            .blocks
            .get(&block_id)
            .and_then(|v| v.received_rb())
            .map(|rb| rb.window_aggregate)
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
        // Invariant: `mempool_count <= queue.len()`. The "queue" holds
        // both the active mempool prefix (first `mempool_count` items)
        // and any overflow items waiting for slack. Anything that
        // breaks this ordering would let an overflow item slip into
        // the active mempool without going through the gate (see the
        // dead-code comment on the gate ↔ mempool invariant).
        debug_assert!(
            self.mempool_count <= self.queue.len(),
            "mempool_count ({}) must not exceed queue.len() ({})",
            self.mempool_count,
            self.queue.len()
        );
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
