//! M1 integration smoke test.
//!
//! Per implementation-plan.md §M1 line 253:
//!
//! > Integration smoke: one short run with the deterministic generator
//! > at congestion levels chosen to force quote drift and evictions.
//! > Confirm refunds and evictions appear and are correct.
//!
//! Exit criterion (line 255): "smoke run produces refunds and evictions
//! on the deterministic generator."
//!
//! Setup:
//! - One node, single-lane EIP-1559 pricing with low `D` (large
//!   per-step move) and target = 0.5.
//! - Hand-rolled tx tuples (`bytes`, `max_fee_lovelace`) submitted on
//!   each slot. Saturating-RB demand drives `quote_per_byte` upward;
//!   txs with low max-fee drift past their budget and are evicted.
//! - Bytes are sized so RBs fully saturate, generating the upward drift
//!   needed to force evictions on the marginal txs.

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use rand::{RngCore, SeedableRng};
use rand_chacha::ChaChaRng;
use tokio::sync::mpsc;

use crate::{
    clock::{Clock, MockClockCoordinator, Timestamp},
    config::{
        DistributionConfig, LeiosVariant, NodeId, RawEip1559Config, RawNode, RawNodeLocation,
        RawParameters, RawPricingConfig, RawTopology, SimConfiguration,
    },
    events::{Event, EventTracker},
    model::{Transaction, TransactionId},
    sim::{
        EventResult, NodeImpl,
        linear_leios::{LinearLeiosNode, TimedEvent},
        lottery::{LotteryKind, MockLotteryResults},
    },
    tx_pricing::Lane,
};

const RB_BODY_BYTES: u64 = 90_000;

fn one_node_config(eip1559: RawEip1559Config) -> Arc<SimConfiguration> {
    let mut params: RawParameters =
        serde_yaml::from_slice(include_bytes!("../../../../parameters/config.default.yaml"))
            .unwrap();
    params.leios_variant = LeiosVariant::Linear;
    // Saturating RB body — every tx fills the RB body so the controller's
    // utilisation signal pushes the price upward each slot.
    params.tx_size_bytes_distribution = DistributionConfig::Constant {
        value: RB_BODY_BYTES as f64,
    };
    params.tx_max_size_bytes = RB_BODY_BYTES;
    params.rb_body_max_size_bytes = RB_BODY_BYTES;
    params.vote_threshold = 1;
    // Engage the EIP-1559 controller with low D so quotes move ~12.5% per
    // saturated block and the smoke run produces drift fast.
    params.pricing = Some(RawPricingConfig::Eip1559(eip1559));
    let topology = RawTopology {
        nodes: BTreeMap::from([(
            "producer".to_string(),
            RawNode {
                stake: Some(1000),
                location: RawNodeLocation::Cluster {
                    cluster: "all".into(),
                },
                cpu_core_count: Some(4),
                tx_conflict_fraction: None,
                tx_generation_weight: None,
                producers: BTreeMap::new(),
                adversarial: None,
                behaviours: vec![],
            },
        )]),
    };
    Arc::new(SimConfiguration::build(params, topology.into()).unwrap())
}

struct SmokeDriver {
    config: Arc<SimConfiguration>,
    nodes: HashMap<NodeId, LinearLeiosNode>,
    lottery: HashMap<NodeId, Arc<MockLotteryResults>>,
    time: MockClockCoordinator,
    slot: u64,
    queued: HashMap<NodeId, EventResult<LinearLeiosNode>>,
    deferred: BTreeMap<Timestamp, Vec<(NodeId, TimedEvent)>>,
    events_rx: mpsc::UnboundedReceiver<(Event, Timestamp)>,
    next_tx_id: u64,
}

impl SmokeDriver {
    fn new(eip1559: RawEip1559Config) -> Self {
        let config = one_node_config(eip1559);
        let time = MockClockCoordinator::new();
        let (event_tx, events_rx) = mpsc::unbounded_channel();
        let (nodes, lottery) = build_nodes(config.clone(), event_tx, time.clock());
        Self {
            config,
            nodes,
            lottery,
            time,
            slot: 0,
            queued: HashMap::new(),
            deferred: BTreeMap::new(),
            events_rx,
            next_tx_id: 0,
        }
    }

    fn producer_id(&self) -> NodeId {
        self.config.nodes[0].id
    }

    fn make_tx(&mut self, bytes: u64, max_fee_lovelace: u64) -> Arc<Transaction> {
        let id = self.next_tx_id;
        self.next_tx_id += 1;
        Arc::new(Transaction {
            id: TransactionId::new(id),
            shard: 0,
            bytes,
            input_id: id,
            overcollateralization_factor: 0,
            max_fee_lovelace,
            posted_lane: Lane::Standard,
            value_lovelace: 0,
            urgency: 1.0,
            urgency_component_index: 0,
        })
    }

    fn submit_tx(&mut self, tx: Arc<Transaction>) {
        let producer = self.producer_id();
        let node = self.nodes.get_mut(&producer).unwrap();
        let events = node.handle_new_tx(tx);
        self.absorb(producer, events);
    }

    fn win_lottery(&mut self, kind: LotteryKind, value: u64) {
        let producer = self.producer_id();
        self.lottery
            .get(&producer)
            .unwrap()
            .configure_win(kind, value);
    }

    fn next_slot(&mut self) {
        self.advance_to(Timestamp::from_secs(self.slot + 1));
    }

    fn advance_to(&mut self, target: Timestamp) {
        let mut now = self.time.now();
        while now < target {
            let next_slot_time = Timestamp::from_secs(self.slot + 1);
            let mut next = target.min(next_slot_time);
            if let Some((t, _)) = self.deferred.first_key_value() {
                next = next.min(*t);
            }
            self.time.advance_time(next);
            now = next;

            // BTreeMap (not HashMap) so iteration order is deterministic.
            // Single-node smoke test doesn't strictly need this, but it
            // future-proofs the driver shape against multi-node reuse
            // and costs nothing here.
            let mut updates: BTreeMap<NodeId, EventResult<LinearLeiosNode>> = BTreeMap::new();
            if now == next_slot_time {
                self.slot += 1;
                for (id, node) in &mut self.nodes {
                    let evs = node.handle_new_slot(self.slot);
                    updates.entry(*id).or_default().merge(evs);
                }
            }
            if let Some(events) = self.deferred.remove(&now) {
                for (id, ev) in events {
                    let node = self.nodes.get_mut(&id).unwrap();
                    let evs = node.handle_timed_event(ev);
                    updates.entry(id).or_default().merge(evs);
                }
            }
            for (id, evs) in updates {
                self.absorb(id, evs);
            }
        }
    }

    fn absorb(&mut self, id: NodeId, mut events: EventResult<LinearLeiosNode>) {
        // Defer timed events; eagerly run scheduled CPU tasks. CPU
        // tasks may schedule more events, so recurse via `absorb` on
        // each task's result.
        for (t, ev) in events.timed_events.drain(..) {
            self.deferred.entry(t).or_default().push((id, ev));
        }
        let tasks: Vec<_> = events.tasks.drain(..).collect();
        for task in tasks {
            let evs = self.nodes.get_mut(&id).unwrap().handle_cpu_task(task);
            self.absorb(id, evs);
        }
        // Stash any remaining queued messages — single-node topology
        // means they are no-ops, but accumulate them for completeness.
        let pending = self.queued.entry(id).or_default();
        pending.messages.append(&mut events.messages);
    }

    fn drain_events(&mut self) -> Vec<Event> {
        let mut out = Vec::new();
        while let Ok((event, _)) = self.events_rx.try_recv() {
            out.push(event);
        }
        out
    }
}

fn build_nodes(
    sim_config: Arc<SimConfiguration>,
    event_tx: mpsc::UnboundedSender<(Event, Timestamp)>,
    clock: Clock,
) -> (
    HashMap<NodeId, LinearLeiosNode>,
    HashMap<NodeId, Arc<MockLotteryResults>>,
) {
    let tracker = EventTracker::new(event_tx, clock.clone(), &sim_config.nodes);
    let mut rng = ChaChaRng::seed_from_u64(sim_config.seed);
    let mut lottery = HashMap::new();
    let nodes = sim_config
        .nodes
        .iter()
        .map(|cfg| {
            let mut node = LinearLeiosNode::new(
                cfg,
                sim_config.clone(),
                tracker.clone(),
                ChaChaRng::seed_from_u64(rng.next_u64()),
                clock.clone(),
            );
            let lr = Arc::new(MockLotteryResults::default());
            node.mock_lottery(lr.clone());
            lottery.insert(cfg.id, lr);
            (cfg.id, node)
        })
        .collect();
    (nodes, lottery)
}

#[test]
fn smoke_run_produces_refunds_and_evictions() {
    // Implementation-plan.md M1 exit criterion (line 255):
    // "smoke run produces refunds and evictions on the deterministic
    // generator".
    //
    // Eip1559 settings: low D for fast drift, target=0.5, generous
    // initial quote so the first txs fit easily. Window length 4 so
    // sustained saturation moves the aggregate near 1.0 quickly.
    let eip1559 = RawEip1559Config {
        initial_quote_per_byte: 50,
        target_num: 1,
        target_den: 2,
        max_change_denominator: 4,
        window_length: 4,
    };

    // minFeeB from the default config; mirrors the sim default
    // (config.rs DEFAULT_MIN_FEE_B). We assert against this constant
    // directly because the smoke run's RB-saturation arithmetic depends
    // on it.
    const MIN_FEE_B: u64 = 155_381;

    let mut sim = SmokeDriver::new(eip1559.clone());

    // Submit a generous tx (high max-fee) for the first slot — this
    // gets included with a refund > 0.
    let big_max_fee = MIN_FEE_B + 1_000 * 90_000; // covers up to quote=1000.
    let generous = sim.make_tx(RB_BODY_BYTES, big_max_fee);
    sim.submit_tx(generous);

    // Then a tx with a marginal max-fee — covers the initial quote of
    // 50 with a small refund, but well below the saturated trajectory.
    // Once two saturated RBs land, the controller pushes quote past
    // marginal_max_fee's break-even point and revalidation evicts it.
    // The break-even quote_per_byte is `(marginal_max_fee - MIN_FEE_B)
    // / 90_000`. We pick marginal_max_fee for break-even ≈ 56.
    let marginal_break_even_quote = 56;
    let marginal_max_fee = MIN_FEE_B + marginal_break_even_quote * RB_BODY_BYTES;
    let marginal = sim.make_tx(RB_BODY_BYTES, marginal_max_fee);
    sim.submit_tx(marginal);

    // Slot 1: producer wins lottery and saturates RB body with the
    // generous tx. (Mempool sampling may pick either tx; the test
    // doesn't depend on the order.)
    sim.win_lottery(LotteryKind::GenerateRB, 0);
    sim.next_slot();

    // Slot 2: feed another generous tx, win lottery again, saturate.
    let next_generous = sim.make_tx(RB_BODY_BYTES, big_max_fee);
    sim.submit_tx(next_generous);
    sim.win_lottery(LotteryKind::GenerateRB, 0);
    sim.next_slot();

    // Slots 3-6: keep saturating with generous txs. The controller
    // pushes quote past the marginal tx's budget by slot 4 or 5; the
    // gate evicts it on the next revalidation.
    for _ in 3..=8 {
        let tx = sim.make_tx(RB_BODY_BYTES, big_max_fee);
        sim.submit_tx(tx);
        sim.win_lottery(LotteryKind::GenerateRB, 0);
        sim.next_slot();
    }

    let events = sim.drain_events();

    // Refunds: at least one TXIncluded with refund_lovelace > 0
    // (the generous tx pays current quote, refund = max_fee - actual).
    let refunds: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            Event::TXIncluded {
                refund_lovelace,
                actual_fee_lovelace,
                served_lane,
                posted_lane,
                ..
            } if *refund_lovelace > 0 => Some((
                *refund_lovelace,
                *actual_fee_lovelace,
                *served_lane,
                *posted_lane,
            )),
            _ => None,
        })
        .collect();
    assert!(
        !refunds.is_empty(),
        "expected at least one TXIncluded with refund>0; got events: {events:#?}"
    );
    for (refund, actual, served, posted) in &refunds {
        // Single-lane: posted and served are both Standard.
        assert_eq!(*posted, Lane::Standard);
        assert_eq!(*served, Lane::Standard);
        // Refund formula: max - actual, with non-negative.
        assert!(*refund > 0);
        assert!(*actual >= MIN_FEE_B);
    }

    // Evictions: the marginal tx's lane quote drifted above its
    // max-fee budget; revalidation emitted a TXEvictedQuoteDrift event.
    let evictions: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            Event::TXEvictedQuoteDrift {
                bytes,
                current_quote_per_byte,
                max_fee_lovelace,
                ..
            } => Some((*bytes, *current_quote_per_byte, *max_fee_lovelace)),
            _ => None,
        })
        .collect();
    assert!(
        !evictions.is_empty(),
        "expected at least one TXEvictedQuoteDrift event; got events: {events:#?}"
    );
    for (bytes, q, max_fee) in &evictions {
        // The tx was evicted because minFeeB + q × bytes > max_fee.
        let posted_fee = MIN_FEE_B
            .checked_add(q.checked_mul(*bytes).unwrap_or(u64::MAX))
            .unwrap_or(u64::MAX);
        assert!(
            posted_fee > *max_fee,
            "eviction record violates spec: minFeeB + q×bytes = {posted_fee} should exceed max_fee = {max_fee}"
        );
    }
}
