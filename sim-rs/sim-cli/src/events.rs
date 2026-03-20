use std::{
    collections::BTreeMap,
    path::PathBuf,
    pin::Pin,
    time::{Duration, Instant},
};

use aggregate::TraceAggregator;
use anyhow::Result;
use async_compression::tokio::write::GzipEncoder;
use average::Variance;
use itertools::Itertools as _;
use liveness::LivenessMonitor;
use pretty_bytes_rust::{PrettyBytesOptions, pretty_bytes};
use serde::Serialize;
use serde_json::json;
use sim_core::{
    clock::Timestamp,
    config::{LeiosVariant, NodeId, SimConfiguration, TierDelayUnit},
    events::{BlockRef, Event, Node, RetryLane},
    model::{ActorId, BlockId, TierId, TransactionId, TransactionRejectReason, UrgencyProfile},
    tx_pricing::BlockKind,
};
use tokio::{
    fs::{self, File},
    io::{AsyncWrite, AsyncWriteExt as _, BufWriter},
    sync::mpsc,
    time::{self, MissedTickBehavior},
};
use tokio_util::sync::CancellationToken;
use tracing::{info, info_span};

mod aggregate;
mod liveness;

type InputBlockId = sim_core::model::InputBlockId<Node>;
type EndorserBlockId = sim_core::model::EndorserBlockId<Node>;
type VoteBundleId = sim_core::model::VoteBundleId<Node>;

type TraceSink = Pin<Box<dyn AsyncWrite + Send + Sync + 'static>>;

#[derive(Clone, Serialize)]
struct OutputEvent {
    time_s: Timestamp,
    message: Event,
}

#[derive(Clone, Copy)]
enum OutputFormat {
    JsonStream,
    CborStream,
}

pub struct EventMonitor {
    variant: LeiosVariant,
    node_ids: Vec<NodeId>,
    pool_ids: Vec<NodeId>,
    maximum_ib_age: u64,
    maximum_eb_age: u64,
    events_source: LivenessMonitor,
    output_path: Option<PathBuf>,
    write_trace: bool,
    aggregate: bool,
    seed: u64,
    tier_delay_unit: TierDelayUnit,
    shutdown: CancellationToken,
    actor_names: BTreeMap<ActorId, String>,
    /// Maps (actor_id, component_index) to a display name for urgency-class welfare reporting.
    urgency_class_names: BTreeMap<(ActorId, u16), String>,
    pricing_metrics: PricingMetrics,
    time_series: Vec<TimeSeriesPoint>,
    time_series_slot_index: BTreeMap<u64, usize>,
}

#[derive(Clone, Debug)]
pub struct RunSummary {
    pub submissions: u64,
    pub unique_generated: u64,
    pub rejected: u64,
    pub included: u64,
    pub inclusion_rate: f64,
    pub unique_inclusion_rate: f64,
    pub tier_delay_unit: TierDelayUnit,
    pub latency_mean_slots: f64,
    pub latency_p95_slots: f64,
    pub latency_p99_slots: f64,
    pub fees_total: u128,
    pub fee_per_byte: f64,
    pub fee_per_tx: f64,
    pub retained_value_total: u128,
    pub retained_value_ratio_generated: f64,
    pub retained_value_ratio_settled: f64,
    pub net_utility_total: i128,
    pub net_utility_per_generated_tx: f64,
    pub rb_generated: u64,
    pub eb_generated: u64,
    pub max_tier_count: usize,
}

impl EventMonitor {
    pub fn new(
        config: &SimConfiguration,
        events_source: mpsc::UnboundedReceiver<(Event, Timestamp)>,
        output_path: Option<PathBuf>,
        write_trace: bool,
        shutdown: CancellationToken,
    ) -> Self {
        let node_ids = config.nodes.iter().map(|p| p.id).collect();
        let pool_ids = config
            .nodes
            .iter()
            .filter_map(|p| if p.stake > 0 { Some(p.id) } else { None })
            .collect();
        let stage_length = config.stage_length;
        let maximum_ib_age = stage_length * 3;
        let actor_names = config
            .actors()
            .map(|actors| {
                actors
                    .iter()
                    .enumerate()
                    .map(|(index, actor)| (ActorId::new(index as u64), actor.name.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let urgency_class_names: BTreeMap<(ActorId, u16), String> = config
            .actors()
            .map(|actors| {
                actors
                    .iter()
                    .enumerate()
                    .flat_map(|(actor_idx, actor)| {
                        let actor_id = ActorId::new(actor_idx as u64);
                        actor
                            .value_urgency_components
                            .iter()
                            .enumerate()
                            .map(move |(comp_idx, comp)| {
                                let name = comp.name.clone().unwrap_or_else(|| {
                                    format!("{}:component_{}", actor.name, comp_idx)
                                });
                                ((actor_id, comp_idx as u16), name)
                            })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            variant: config.variant,
            node_ids,
            pool_ids,
            maximum_ib_age,
            maximum_eb_age: config.max_eb_age,
            events_source: LivenessMonitor::new(config, events_source),
            output_path,
            write_trace,
            aggregate: config.aggregate_events,
            seed: config.seed,
            tier_delay_unit: config.tier_delay_unit(),
            shutdown,
            actor_names,
            urgency_class_names,
            pricing_metrics: PricingMetrics::default(),
            time_series: Vec::new(),
            time_series_slot_index: BTreeMap::new(),
        }
    }

    // Monitor and report any events emitted by the simulation,
    // including any aggregated stats at the end.
    pub async fn run(mut self) -> Result<RunSummary> {
        let mut blocks_published: BTreeMap<NodeId, u64> = BTreeMap::new();
        let mut blocks_rejected: BTreeMap<NodeId, u64> = BTreeMap::new();
        let mut blocks: BTreeMap<u64, (NodeId, u64)> = BTreeMap::new();
        let mut txs: BTreeMap<TransactionId, Transaction> = BTreeMap::new();
        let mut ibs: BTreeMap<InputBlockId, InputBlock> = BTreeMap::new();
        let mut ebs: BTreeMap<EndorserBlockId, EndorserBlock> = BTreeMap::new();
        let mut seen_ibs: BTreeMap<NodeId, f64> = BTreeMap::new();
        let mut ibs_containing_tx: BTreeMap<TransactionId, f64> = BTreeMap::new();
        let mut ebs_containing_ib: BTreeMap<InputBlockId, f64> = BTreeMap::new();
        let mut votes_per_bundle: BTreeMap<VoteBundleId, f64> = BTreeMap::new();
        let mut votes_per_pool: BTreeMap<NodeId, f64> =
            self.pool_ids.iter().copied().map(|id| (id, 0.0)).collect();
        let mut eb_votes: BTreeMap<EndorserBlockId, f64> = BTreeMap::new();

        let mut last_timestamp = Timestamp::zero();
        let mut total_slots = 0u64;
        let mut total_votes = 0u64;
        let mut leios_blocks_with_endorsements = 0u64;
        let mut total_leios_txs = 0u64;
        let mut total_leios_bytes = 0u64;
        let mut tx_messages = MessageStats::default();
        let mut ib_messages = MessageStats::default();
        let mut eb_messages = MessageStats::default();
        let mut vote_messages = MessageStats::default();
        let mut rb_generated = 0u64;
        let mut eb_generated = 0u64;
        let mut event_count = 0u64;
        let mut last_slot = 0u64;
        let mut last_rb_tier_count = 0usize;
        let mut last_rb_tier_prices: Vec<u64> = Vec::new();
        let mut last_rb_tier_prices_by_id: BTreeMap<TierId, u64> = BTreeMap::new();
        let mut last_rb_tier_delays: Vec<u64> = Vec::new();
        let mut last_rb_tier_capacities: Vec<u64> = Vec::new();
        let mut last_rb_tier_utilisations: Vec<f64> = Vec::new();
        let mut last_eb_tier_count = 0usize;
        let mut last_eb_tier_prices: Vec<u64> = Vec::new();
        let mut last_eb_tier_prices_by_id: BTreeMap<TierId, u64> = BTreeMap::new();
        let mut last_eb_tier_delays: Vec<u64> = Vec::new();
        let mut last_eb_tier_capacities: Vec<u64> = Vec::new();
        let mut last_eb_tier_utilisations: Vec<f64> = Vec::new();
        let mut last_tasks_in_flight = 0u64;
        let mut last_actors_running = 0u64;
        let mut last_actors_total = 0u64;
        let mut last_running_actor_ids: Vec<u64> = Vec::new();
        let mut last_task_started_by: Option<u64> = None;
        let mut last_task_finished_by: Option<u64> = None;
        let mut last_wait_actor: Option<u64> = None;
        let mut last_wait_until_nanos: Option<u64> = None;
        let mut last_woken_actor: Option<u64> = None;
        let mut last_advance_to_nanos: Option<u64> = None;
        let mut last_wait_queue_len: u64 = 0;
        let mut cumulative_rb_inclusions = 0u64;
        let mut cumulative_eb_inclusions = 0u64;
        let mut last_tx_producer_slot: Option<u64> = None;
        let mut last_tx_producer_phase: Option<&'static str> = None;
        let mut last_node_handler_started: Option<Node> = None;
        let mut last_node_handler_started_kind: Option<&'static str> = None;
        let mut last_node_handler_finished: Option<Node> = None;
        let mut last_node_handler_finished_kind: Option<&'static str> = None;
        let mut actor_registry: BTreeMap<u64, String> = BTreeMap::new();

        // Pretty print options for bytes
        let pbo = Some(PrettyBytesOptions {
            use_1024_instead_of_1000: Some(false),
            number_of_decimal: Some(2),
            remove_zero_decimal: Some(true),
        });

        let output_dir = self.pricing_output_dir();
        let mut diagnostics =
            DiagnosticsLogger::new(&self.output_path, &output_dir, Instant::now()).await?;

        let mut output = if self.write_trace {
            if let Some(path) = &self.output_path
                && let Some(parent) = path.parent()
            {
                fs::create_dir_all(parent).await?;
            }

            match self.output_path.as_mut() {
                Some(path) => {
                    let file = File::create(&path).await?;

                    let mut gzipped = false;
                    if path
                        .extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|ext| ext == "gz")
                    {
                        path.set_extension("");
                        gzipped = true;
                    }

                    let file: TraceSink = if gzipped {
                        let encoder = GzipEncoder::new(file);
                        Box::pin(BufWriter::new(encoder))
                    } else {
                        Box::pin(BufWriter::new(file))
                    };

                    let format = if path
                        .extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|ext| ext == "cbor")
                    {
                        OutputFormat::CborStream
                    } else {
                        OutputFormat::JsonStream
                    };
                    if self.aggregate {
                        OutputTarget::AggregatedEventStream {
                            aggregation: TraceAggregator::new(),
                            format,
                            file,
                        }
                    } else {
                        OutputTarget::EventStream { format, file }
                    }
                }
                None => OutputTarget::None,
            }
        } else {
            OutputTarget::None
        };
        let mut heartbeat = time::interval(Duration::from_secs(10));
        heartbeat.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            let maybe_event = tokio::select! {
                _ = self.shutdown.cancelled() => {
                    break;
                }
                _ = heartbeat.tick() => {
                    let running_actor_names =
                        format_actor_names(&last_running_actor_ids, &actor_registry);
                    let last_task_started_name =
                        format_actor_name(last_task_started_by, &actor_registry);
                    let last_task_finished_name =
                        format_actor_name(last_task_finished_by, &actor_registry);
                    let last_wait_actor_id = last_wait_actor
                        .map(|id| id.to_string())
                        .unwrap_or_default();
                    let last_woken_actor_id = last_woken_actor
                        .map(|id| id.to_string())
                        .unwrap_or_default();
                    let last_wait_until_s = last_wait_until_nanos
                        .map(|n| format!("{:.6}", n as f64 / 1_000_000_000.0))
                        .unwrap_or_default();
                    let last_advance_to_s = last_advance_to_nanos
                        .map(|n| format!("{:.6}", n as f64 / 1_000_000_000.0))
                        .unwrap_or_default();
                    let last_node_handler_started_id = last_node_handler_started
                        .as_ref()
                        .map(|node| node.id.to_string())
                        .unwrap_or_default();
                    let last_node_handler_started_name = last_node_handler_started
                        .as_ref()
                        .map(|node| node.name.as_str())
                        .unwrap_or("");
                    let last_node_handler_finished_id = last_node_handler_finished
                        .as_ref()
                        .map(|node| node.id.to_string())
                        .unwrap_or_default();
                    let last_node_handler_finished_name = last_node_handler_finished
                        .as_ref()
                        .map(|node| node.name.as_str())
                        .unwrap_or("");
                    diagnostics
                        .log_heartbeat(
                            last_slot,
                            last_timestamp,
                            event_count,
                            last_tasks_in_flight,
                            last_actors_running,
                            last_actors_total,
                            &last_running_actor_ids,
                            &running_actor_names,
                            last_task_started_by,
                            &last_task_started_name,
                            last_task_finished_by,
                            &last_task_finished_name,
                            &last_wait_actor_id,
                            &last_wait_until_s,
                            &last_woken_actor_id,
                            &last_advance_to_s,
                            last_wait_queue_len,
                            &last_node_handler_started_id,
                            last_node_handler_started_name,
                            last_node_handler_started_kind.unwrap_or(""),
                            &last_node_handler_finished_id,
                            last_node_handler_finished_name,
                            last_node_handler_finished_kind.unwrap_or(""),
                            last_tx_producer_slot,
                            last_tx_producer_phase.unwrap_or(""),
                            &txs,
                            &self.pricing_metrics,
                            rb_generated,
                            eb_generated,
                            last_rb_tier_count,
                            &last_rb_tier_prices,
                            last_eb_tier_count,
                            &last_eb_tier_prices,
                        )
                        .await?;
                    self.write_pricing_outputs(&output_dir).await?;
                    continue;
                }
                event = self.events_source.recv() => event,
            };

            let Some((event, time)) = maybe_event else {
                break;
            };
            event_count = event_count.saturating_add(1);
            last_timestamp = time;
            let output_event = OutputEvent {
                time_s: time,
                message: event.clone(),
            };
            output.write(output_event).await?;
            match event {
                Event::GlobalSlot { slot: number } => {
                    info!("Slot {number} has begun.");
                    total_slots = number + 1;
                    last_slot = number;
                    self.upsert_time_series(TimeSeriesPoint {
                        slot: number,
                        rb_tier_count: last_rb_tier_count,
                        rb_tier_prices: last_rb_tier_prices.clone(),
                        rb_tier_delays: last_rb_tier_delays.clone(),
                        rb_tier_capacities: last_rb_tier_capacities.clone(),
                        rb_tier_utilisations: last_rb_tier_utilisations.clone(),
                        eb_tier_count: last_eb_tier_count,
                        eb_tier_prices: last_eb_tier_prices.clone(),
                        eb_tier_delays: last_eb_tier_delays.clone(),
                        eb_tier_capacities: last_eb_tier_capacities.clone(),
                        eb_tier_utilisations: last_eb_tier_utilisations.clone(),
                        cumulative_inclusions: self.pricing_metrics.included,
                        cumulative_rb_inclusions,
                        cumulative_eb_inclusions,
                        cumulative_block_inclusions_total: self
                            .pricing_metrics
                            .block_included_total,
                        cumulative_block_inclusions_with_delay: self
                            .pricing_metrics
                            .block_included_with_delay,
                        cumulative_submitted_bytes: self.pricing_metrics.total_submitted_bytes,
                        cumulative_included_bytes: self.pricing_metrics.total_included_bytes,
                        cumulative_fees: self.pricing_metrics.total_fees,
                        cumulative_rb_tier_assignments_total: self
                            .pricing_metrics
                            .rb_tier_assignments_total,
                        cumulative_rb_tier_assignments_max_priced: self
                            .pricing_metrics
                            .rb_tier_assignments_to_max_priced_tier,
                        cumulative_rb_tier_assignments_by_tier: self
                            .pricing_metrics
                            .rb_tier_assignments_by_tier
                            .clone(),
                        cumulative_eb_tier_assignments_total: self
                            .pricing_metrics
                            .eb_tier_assignments_total,
                        cumulative_eb_tier_assignments_max_priced: self
                            .pricing_metrics
                            .eb_tier_assignments_to_max_priced_tier,
                        cumulative_eb_tier_assignments_by_tier: self
                            .pricing_metrics
                            .eb_tier_assignments_by_tier
                            .clone(),
                    });
                    let running_actor_names =
                        format_actor_names(&last_running_actor_ids, &actor_registry);
                    let last_task_started_name =
                        format_actor_name(last_task_started_by, &actor_registry);
                    let last_task_finished_name =
                        format_actor_name(last_task_finished_by, &actor_registry);
                    let last_wait_actor_id =
                        last_wait_actor.map(|id| id.to_string()).unwrap_or_default();
                    let last_woken_actor_id = last_woken_actor
                        .map(|id| id.to_string())
                        .unwrap_or_default();
                    let last_wait_until_s = last_wait_until_nanos
                        .map(|n| format!("{:.6}", n as f64 / 1_000_000_000.0))
                        .unwrap_or_default();
                    let last_advance_to_s = last_advance_to_nanos
                        .map(|n| format!("{:.6}", n as f64 / 1_000_000_000.0))
                        .unwrap_or_default();
                    let last_node_handler_started_id = last_node_handler_started
                        .as_ref()
                        .map(|node| node.id.to_string())
                        .unwrap_or_default();
                    let last_node_handler_started_name = last_node_handler_started
                        .as_ref()
                        .map(|node| node.name.as_str())
                        .unwrap_or("");
                    let last_node_handler_finished_id = last_node_handler_finished
                        .as_ref()
                        .map(|node| node.id.to_string())
                        .unwrap_or_default();
                    let last_node_handler_finished_name = last_node_handler_finished
                        .as_ref()
                        .map(|node| node.name.as_str())
                        .unwrap_or("");
                    diagnostics
                        .log_slot(
                            number,
                            time,
                            event_count,
                            last_tasks_in_flight,
                            last_actors_running,
                            last_actors_total,
                            &last_running_actor_ids,
                            &running_actor_names,
                            last_task_started_by,
                            &last_task_started_name,
                            last_task_finished_by,
                            &last_task_finished_name,
                            &last_wait_actor_id,
                            &last_wait_until_s,
                            &last_woken_actor_id,
                            &last_advance_to_s,
                            last_wait_queue_len,
                            &last_node_handler_started_id,
                            last_node_handler_started_name,
                            last_node_handler_started_kind.unwrap_or(""),
                            &last_node_handler_finished_id,
                            last_node_handler_finished_name,
                            last_node_handler_finished_kind.unwrap_or(""),
                            last_tx_producer_slot,
                            last_tx_producer_phase.unwrap_or(""),
                            &txs,
                            &self.pricing_metrics,
                            rb_generated,
                            eb_generated,
                            last_rb_tier_count,
                            &last_rb_tier_prices,
                            last_eb_tier_count,
                            &last_eb_tier_prices,
                        )
                        .await?;
                }
                Event::ActorRegistered { actor_id, name } => {
                    actor_registry.insert(actor_id, name);
                }
                Event::ClockDiagnostics {
                    tasks_in_flight,
                    actors_running,
                    actors_total,
                    running_actor_ids,
                    last_task_started_by: started_by,
                    last_task_finished_by: finished_by,
                    last_wait_actor: wait_actor,
                    last_wait_until_nanos: wait_until_nanos,
                    last_woken_actor: woken_actor,
                    last_advance_to_nanos: advance_to_nanos,
                    wait_queue_len,
                    ..
                } => {
                    last_tasks_in_flight = tasks_in_flight;
                    last_actors_running = actors_running;
                    last_actors_total = actors_total;
                    last_running_actor_ids = running_actor_ids;
                    last_task_started_by = started_by;
                    last_task_finished_by = finished_by;
                    last_wait_actor = wait_actor;
                    last_wait_until_nanos = wait_until_nanos;
                    last_woken_actor = woken_actor;
                    last_advance_to_nanos = advance_to_nanos;
                    last_wait_queue_len = wait_queue_len;
                }
                Event::TxProducerDiagnostics { slot, phase } => {
                    last_tx_producer_slot = Some(slot);
                    last_tx_producer_phase = Some(phase.as_str());
                }
                Event::NodeHandlerDiagnostics { node, kind, phase } => match phase.as_str() {
                    "start" => {
                        last_node_handler_started = Some(node);
                        last_node_handler_started_kind = Some(kind.as_str());
                    }
                    "finish" => {
                        last_node_handler_finished = Some(node);
                        last_node_handler_finished_kind = Some(kind.as_str());
                    }
                    _ => {}
                },
                Event::Slot { .. } => {}
                Event::CpuTaskScheduled { .. } => {}
                Event::CpuTaskFinished { .. } => {}
                Event::Cpu { .. } => {}
                Event::TXGenerated {
                    id,
                    size_bytes,
                    actor_id,
                    submission_slot,
                    urgency,
                    value,
                    urgency_component_index,
                    ..
                } => {
                    txs.insert(
                        id,
                        Transaction::new(
                            size_bytes,
                            time,
                            submission_slot,
                            actor_id,
                            urgency,
                            value,
                            urgency_component_index,
                        ),
                    );
                    self.pricing_metrics
                        .record_generated(actor_id, value, urgency_component_index);
                }
                Event::TXSent { .. } => {
                    tx_messages.sent += 1;
                }
                Event::TXReceived { .. } => {
                    tx_messages.received += 1;
                }
                Event::TXRejected { id, reason, .. } => {
                    if let Some(tx) = txs.get_mut(&id) {
                        if !tx.rejected {
                            tx.rejected = true;
                            self.pricing_metrics.record_rejection(
                                tx.actor_id,
                                reason,
                                tx.submitted,
                            );
                        }
                    }
                }
                Event::TXTierAssigned {
                    id,
                    block_kind,
                    tier,
                    tier_version_created_slot,
                    posted_fee,
                    tier_delay_slots,
                    ..
                } => {
                    let lane_prices_by_id = match block_kind {
                        BlockKind::RankingBlock => &last_rb_tier_prices_by_id,
                        BlockKind::EndorserBlock => &last_eb_tier_prices_by_id,
                    };
                    self.pricing_metrics.record_tier_assignment(
                        block_kind,
                        tier,
                        lane_prices_by_id,
                    );
                    if let Some(tx) = txs.get_mut(&id) {
                        if !tx.submitted {
                            tx.submitted = true;
                            self.pricing_metrics
                                .record_submission(tx.actor_id, tx.bytes);
                        }
                        match block_kind {
                            BlockKind::RankingBlock => {
                                tx.tier = Some(tier);
                                tx.tier_version_created_slot = Some(tier_version_created_slot);
                                tx.posted_fee = Some(posted_fee);
                                tx.tier_delay_slots = Some(tier_delay_slots);
                            }
                            BlockKind::EndorserBlock => {
                                tx.eb_tier_version_created_slot = Some(tier_version_created_slot);
                                tx.eb_posted_fee = Some(posted_fee);
                                tx.eb_tier_delay_slots = Some(tier_delay_slots);
                            }
                        }
                    }
                }
                Event::TXRetryScheduled {
                    actor_id,
                    lane,
                    tier,
                    ..
                } => {
                    self.pricing_metrics
                        .record_retry_scheduled(actor_id, lane, tier);
                }
                Event::TXOverflowChecked {
                    block_kind,
                    tier,
                    pending_bytes,
                    tier_capacity_bytes,
                    overfull,
                    ..
                } => {
                    self.pricing_metrics.record_overflow_checked(
                        block_kind,
                        tier,
                        pending_bytes,
                        tier_capacity_bytes,
                        overfull,
                    );
                }
                Event::TXOverflowRejected {
                    block_kind,
                    tier,
                    retry_scheduled,
                    ..
                } => {
                    self.pricing_metrics.record_overflow_rejected(
                        block_kind,
                        tier,
                        retry_scheduled,
                    );
                }
                Event::TXLost { .. } => {}
                Event::RBLotteryWon { .. } => {}
                Event::RBGenerated {
                    id: BlockId { slot, producer },
                    vrf,
                    endorsement,
                    transactions,
                    ..
                } => {
                    rb_generated = rb_generated.saturating_add(1);
                    info!(
                        "Pool {} produced a praos block in slot {slot} with {} tx(s).",
                        producer,
                        transactions.len()
                    );
                    if let Some(endorsement) = endorsement {
                        total_leios_bytes += endorsement.size_bytes;
                        leios_blocks_with_endorsements += 1;

                        let mut block_leios_txs = vec![];
                        let mut eb_queue = vec![endorsement.eb.id.clone()];
                        while let Some(eb_id) = eb_queue.pop() {
                            let eb = ebs.get_mut(&eb_id).unwrap();
                            if eb.included_in_block.is_some() {
                                continue;
                            }
                            eb.included_in_block = Some(time);

                            eb_queue.extend(eb.ebs.iter().cloned());

                            for ib_id in &eb.ibs {
                                let ib = ibs.get_mut(ib_id).unwrap();
                                if ib.included_in_block.is_none() {
                                    ib.included_in_block = Some(time);
                                }
                                for tx_id in &ib.txs {
                                    block_leios_txs.push(*tx_id);
                                    if txs
                                        .get(tx_id)
                                        .is_some_and(|tx| tx.included_in_block.is_none())
                                    {
                                        record_inclusion(
                                            &mut self.pricing_metrics,
                                            &mut txs,
                                            *tx_id,
                                            time,
                                            BlockKind::EndorserBlock,
                                        );
                                        cumulative_eb_inclusions =
                                            cumulative_eb_inclusions.saturating_add(1);
                                        if let Some(tx) = txs.get_mut(tx_id) {
                                            tx.tx_type = Some(TransactionType::Leios);
                                        }
                                    }
                                }
                            }
                            for tx_id in &eb.txs {
                                block_leios_txs.push(*tx_id);
                                if txs
                                    .get(tx_id)
                                    .is_some_and(|tx| tx.included_in_block.is_none())
                                {
                                    record_inclusion(
                                        &mut self.pricing_metrics,
                                        &mut txs,
                                        *tx_id,
                                        time,
                                        BlockKind::EndorserBlock,
                                    );
                                    cumulative_eb_inclusions =
                                        cumulative_eb_inclusions.saturating_add(1);
                                    if let Some(tx) = txs.get_mut(tx_id) {
                                        tx.tx_type = Some(TransactionType::Leios);
                                    }
                                }
                            }
                        }

                        total_leios_txs += block_leios_txs.len() as u64;
                        if matches!(
                            self.variant,
                            LeiosVariant::FullWithTxReferences | LeiosVariant::FullWithoutIbs
                        ) {
                            // In variants where transactions are referenced by Leios blocks but not embedded in IBs,
                            // referenced TXs need to be persisted separately. So count those referenced TX sizes
                            // against Leios's "space efficiency".
                            total_leios_bytes += block_leios_txs
                                .iter()
                                .map(|tx_id| txs.get(tx_id).unwrap().bytes)
                                .sum::<u64>();
                        }
                        let unique_block_leios_txs =
                            block_leios_txs.iter().copied().sorted().dedup().count();
                        info!(
                            "This block had an additional {} leios tx(s) ({} unique).",
                            block_leios_txs.len(),
                            unique_block_leios_txs,
                        );
                    }
                    for tx_id in &transactions {
                        if txs
                            .get(tx_id)
                            .is_some_and(|tx| tx.included_in_block.is_none())
                        {
                            record_inclusion(
                                &mut self.pricing_metrics,
                                &mut txs,
                                *tx_id,
                                time,
                                BlockKind::RankingBlock,
                            );
                            cumulative_rb_inclusions = cumulative_rb_inclusions.saturating_add(1);
                            if let Some(tx) = txs.get_mut(tx_id) {
                                tx.tx_type = Some(TransactionType::Praos);
                            }
                        }
                    }
                    if let Some((old_producer, old_vrf)) = blocks.get(&slot) {
                        if *old_vrf > vrf {
                            *blocks_published.entry(producer.id).or_default() += 1;
                            *blocks_published.entry(*old_producer).or_default() -= 1;
                            *blocks_rejected.entry(*old_producer).or_default() += 1;
                            blocks.insert(slot, (producer.id, vrf));
                        } else {
                            *blocks_rejected.entry(producer.id).or_default() += 1;
                        }
                    } else {
                        *blocks_published.entry(producer.id).or_default() += 1;
                        blocks.insert(slot, (producer.id, vrf));
                    }
                }
                Event::RBSent { .. } => {}
                Event::RBReceived { .. } => {}
                Event::IBLotteryWon { .. } => {}
                Event::IBGenerated {
                    id,
                    header_bytes,
                    size_bytes,
                    transactions,
                    shard,
                    ..
                } => {
                    ibs.insert(
                        id.clone(),
                        InputBlock::new(size_bytes, time, transactions.clone()),
                    );
                    total_leios_bytes += size_bytes;
                    let mut tx_bytes = header_bytes;
                    for tx_id in &transactions {
                        *ibs_containing_tx.entry(*tx_id).or_default() += 1.;
                        let tx = txs.get_mut(tx_id).unwrap();
                        tx_bytes += tx.bytes;
                        if tx.included_in_ib.is_none() {
                            tx.included_in_ib = Some(time);
                        }
                    }
                    *seen_ibs.entry(id.producer.id).or_default() += 1.;
                    info!(
                        "Pool {} generated an IB in shard {} with {} transaction(s) in slot {} ({}).",
                        id.producer,
                        shard,
                        transactions.len(),
                        id.slot,
                        pretty_bytes(tx_bytes, pbo.clone()),
                    )
                }
                Event::NoIBGenerated { .. } => {}
                Event::IBSent { .. } => {
                    ib_messages.sent += 1;
                }
                Event::IBReceived { recipient, .. } => {
                    ib_messages.received += 1;
                    *seen_ibs.entry(recipient.id).or_default() += 1.;
                }
                Event::EBLotteryWon { .. } => {}
                Event::EBGenerated {
                    id,
                    transactions,
                    input_blocks,
                    endorser_blocks,
                    size_bytes,
                    ..
                } => {
                    eb_generated = eb_generated.saturating_add(1);
                    ebs.insert(
                        id.clone(),
                        EndorserBlock::new(
                            time,
                            transactions.iter().map(|tx| tx.id).collect(),
                            input_blocks.iter().map(|ib| ib.id.clone()).collect(),
                            endorser_blocks.iter().map(|eb| eb.id.clone()).collect(),
                        ),
                    );
                    total_leios_bytes += size_bytes;
                    for BlockRef { id: tx_id } in &transactions {
                        let tx = txs.get_mut(tx_id).unwrap();
                        if tx.included_in_eb.is_none() {
                            tx.included_in_eb = Some(time);
                        }
                    }
                    for BlockRef { id: ib_id } in &input_blocks {
                        let ib = ibs.get_mut(ib_id).unwrap();
                        if ib.included_in_eb.is_none() {
                            ib.included_in_eb = Some(time);
                        }
                        *ebs_containing_ib.entry(ib_id.clone()).or_default() += 1.0;
                        for tx_id in &ib.txs {
                            let tx = txs.get_mut(tx_id).unwrap();
                            if tx.included_in_eb.is_none() {
                                tx.included_in_eb = Some(time);
                            }
                        }
                    }
                    info!(
                        "Pool {} generated an EB with {} IB(s) and {} TX(s) in slot {}.",
                        id.producer,
                        input_blocks.len(),
                        transactions.len(),
                        id.slot,
                    )
                }
                Event::NoEBGenerated { .. } => {}
                Event::EBSent { .. } => {
                    eb_messages.sent += 1;
                }
                Event::EBReceived { .. } => {
                    eb_messages.received += 1;
                }
                Event::VTLotteryWon { .. } => {}
                Event::VTBundleGenerated { id, votes, .. } => {
                    for (eb, count) in votes.0 {
                        total_votes += count as u64;
                        *votes_per_bundle.entry(id.clone()).or_default() += count as f64;
                        *eb_votes.entry(eb).or_default() += count as f64;
                        *votes_per_pool.entry(id.producer.id).or_default() += count as f64;
                    }
                }
                Event::NoVTBundleGenerated { .. } => {}
                Event::VTBundleNotGenerated { .. } => {}
                Event::VTBundleSent { .. } => {
                    vote_messages.sent += 1;
                }
                Event::VTBundleReceived { .. } => {
                    vote_messages.received += 1;
                }
                Event::TierPricesUpdated {
                    node: _,
                    block_kind,
                    slot,
                    delay_update_triggered,
                    tier_update_triggered,
                    tiers,
                } => {
                    self.pricing_metrics.record_cadence_update(
                        block_kind,
                        slot,
                        delay_update_triggered,
                        tier_update_triggered,
                    );
                    let count = tiers.len();
                    let prices: Vec<u64> = tiers.iter().map(|tier| tier.price_per_byte).collect();
                    let prices_by_id: BTreeMap<TierId, u64> = tiers
                        .iter()
                        .map(|tier| (tier.id, tier.price_per_byte))
                        .collect();
                    let delays: Vec<u64> = tiers.iter().map(|tier| tier.delay).collect();
                    let capacities: Vec<u64> =
                        tiers.iter().map(|tier| tier.capacity_bytes).collect();
                    let utilisations: Vec<f64> =
                        tiers.iter().map(|tier| tier.utilisation).collect();
                    match block_kind {
                        BlockKind::RankingBlock => {
                            last_rb_tier_count = count;
                            last_rb_tier_prices = prices;
                            last_rb_tier_prices_by_id = prices_by_id;
                            last_rb_tier_delays = delays;
                            last_rb_tier_capacities = capacities;
                            last_rb_tier_utilisations = utilisations;
                        }
                        BlockKind::EndorserBlock => {
                            last_eb_tier_count = count;
                            last_eb_tier_prices = prices;
                            last_eb_tier_prices_by_id = prices_by_id;
                            last_eb_tier_delays = delays;
                            last_eb_tier_capacities = capacities;
                            last_eb_tier_utilisations = utilisations;
                        }
                    }
                    self.upsert_time_series(TimeSeriesPoint {
                        slot,
                        rb_tier_count: last_rb_tier_count,
                        rb_tier_prices: last_rb_tier_prices.clone(),
                        rb_tier_delays: last_rb_tier_delays.clone(),
                        rb_tier_capacities: last_rb_tier_capacities.clone(),
                        rb_tier_utilisations: last_rb_tier_utilisations.clone(),
                        eb_tier_count: last_eb_tier_count,
                        eb_tier_prices: last_eb_tier_prices.clone(),
                        eb_tier_delays: last_eb_tier_delays.clone(),
                        eb_tier_capacities: last_eb_tier_capacities.clone(),
                        eb_tier_utilisations: last_eb_tier_utilisations.clone(),
                        cumulative_inclusions: self.pricing_metrics.included,
                        cumulative_rb_inclusions,
                        cumulative_eb_inclusions,
                        cumulative_block_inclusions_total: self
                            .pricing_metrics
                            .block_included_total,
                        cumulative_block_inclusions_with_delay: self
                            .pricing_metrics
                            .block_included_with_delay,
                        cumulative_submitted_bytes: self.pricing_metrics.total_submitted_bytes,
                        cumulative_included_bytes: self.pricing_metrics.total_included_bytes,
                        cumulative_fees: self.pricing_metrics.total_fees,
                        cumulative_rb_tier_assignments_total: self
                            .pricing_metrics
                            .rb_tier_assignments_total,
                        cumulative_rb_tier_assignments_max_priced: self
                            .pricing_metrics
                            .rb_tier_assignments_to_max_priced_tier,
                        cumulative_rb_tier_assignments_by_tier: self
                            .pricing_metrics
                            .rb_tier_assignments_by_tier
                            .clone(),
                        cumulative_eb_tier_assignments_total: self
                            .pricing_metrics
                            .eb_tier_assignments_total,
                        cumulative_eb_tier_assignments_max_priced: self
                            .pricing_metrics
                            .eb_tier_assignments_to_max_priced_tier,
                        cumulative_eb_tier_assignments_by_tier: self
                            .pricing_metrics
                            .eb_tier_assignments_by_tier
                            .clone(),
                    });
                }
                Event::TierCreated { .. } => {}
                Event::TierRemoved { .. } => {}
            }
        }

        output.flush().await?;
        diagnostics.flush().await?;

        let mut finalized_txs = 0;
        let mut finalized_tx_bytes = 0;
        let mut pending_txs = 0;
        let mut pending_tx_bytes = 0;
        let mut praos_txs = 0;
        let mut praos_tx_bytes = 0;
        let mut leios_txs = 0;
        let mut leios_tx_bytes = 0;
        for tx in txs.values() {
            if let Some(tx_type) = tx.tx_type {
                finalized_txs += 1;
                finalized_tx_bytes += tx.bytes;
                match tx_type {
                    TransactionType::Praos => {
                        praos_txs += 1;
                        praos_tx_bytes += tx.bytes;
                    }
                    TransactionType::Leios => {
                        leios_txs += 1;
                        leios_tx_bytes += tx.bytes;
                    }
                }
            } else {
                pending_txs += 1;
                pending_tx_bytes += tx.bytes;
            }
        }

        info_span!("praos").in_scope(|| {
            info!("{} transactions(s) were generated in total.", txs.len());
            info!("{} naive praos block(s) were published.", blocks.len());
            info!(
                "{} slot(s) had no naive praos blocks.",
                total_slots - blocks.len() as u64
            );
            info!("{} transaction(s) ({}) finalized in a naive praos block.", finalized_txs, pretty_bytes(finalized_tx_bytes, pbo.clone()));
            info!(
                "{} transaction(s) ({}) did not reach a naive praos block.",
                pending_txs,
                pretty_bytes(
                    pending_tx_bytes,
                    pbo.clone(),
                ),
            );

            for id in &self.node_ids {
                if let Some(published) = blocks_published.get(id) {
                    info!("Pool {id} published {published} naive praos block(s)");
                }
                if let Some(rejected) = blocks_rejected.get(id) {
                    info!("Pool {id} failed to publish {rejected} naive praos block(s) due to slot battles.");
                }
            }
        });

        info_span!("leios").in_scope(|| {
            let times_to_reach_ib: Vec<_> = txs
                .values()
                .filter_map(|tx| {
                    let ib_time = tx.included_in_ib?;
                    Some(ib_time - tx.generated)
                })
                .collect();
            let times_to_reach_eb: Vec<_> = txs
                .values()
                .filter_map(|tx| {
                    let eb_time = tx.included_in_eb?;
                    Some(eb_time - tx.generated)
                })
                .collect();
            let times_to_reach_block: Vec<_> = txs
                .values()
                .filter_map(|tx| {
                    let block_time = tx.included_in_block?;
                    Some(block_time - tx.generated)
                })
                .collect();
            let ib_expiration_cutoff = last_timestamp.checked_sub_duration(Duration::from_secs(self.maximum_ib_age)).unwrap_or_default();
            let expired_ibs = ibs.values().filter(|ib| ib.included_in_eb.is_none() && ib.generated < ib_expiration_cutoff).count();
            let eb_expiration_cutoff = last_timestamp.checked_sub_duration(Duration::from_secs(self.maximum_eb_age)).unwrap_or_default();
            let expired_ebs = ebs.values().filter(|eb| eb.included_in_eb.is_none() && eb.included_in_block.is_none() && eb.generated < eb_expiration_cutoff).count();
            let empty_ebs = ebs.values().filter(|eb| eb.is_empty()).count();
            let bundle_count = votes_per_bundle.len();
            let txs_per_ib = compute_stats(ibs.values().map(|ib| ib.txs.len() as f64));
            let bytes_per_ib = compute_stats(ibs.values().map(|ib| ib.bytes as f64));
            let ibs_per_tx = compute_stats(ibs_containing_tx.into_values());
            let txs_per_eb = compute_stats(ebs.values().map(|eb| eb.txs.len() as f64));
            let ibs_per_eb = compute_stats(ebs.values().map(|eb| eb.ibs.len() as f64));
            let ebs_per_ib = compute_stats(ebs_containing_ib.into_values());
            let ib_time_stats = compute_stats(times_to_reach_ib.iter().map(|t| t.as_secs_f64()));
            let eb_time_stats = compute_stats(times_to_reach_eb.iter().map(|t| t.as_secs_f64()));
            let block_time_stats = compute_stats(times_to_reach_block.iter().map(|t| t.as_secs_f64()));
            let ibs_received = compute_stats(
                self.node_ids
                    .iter()
                    .map(|id| seen_ibs.get(id).copied().unwrap_or_default()),
            );
            let votes_per_pool = compute_stats(votes_per_pool.into_values());
            let votes_per_eb = compute_stats(eb_votes.into_values());
            let votes_per_bundle = compute_stats(votes_per_bundle.into_values());

            info!(
                "{} IB(s) were generated, on average {:.3} IB(s) per slot.",
                ibs.len(),
                ibs.len() as f64 / total_slots as f64
            );
            info!(
                "{} out of {} transaction(s) were included in at least one IB.",
                times_to_reach_ib.len(),
                txs.len(),
            );
            let avg_age = txs.values().filter_map(|tx| {
                if tx.tx_type.is_none() {
                    Some((last_timestamp - tx.generated).as_secs_f64())
                } else {
                    None
                }
            });
            let avg_age_stats = compute_stats(avg_age);
            info!(
                "The average age of the pending transactions is {:.3}s (stddev {:.3}).",
                avg_age_stats.mean, avg_age_stats.std_dev,
            );
            info!(
                "Each transaction was included in an average of {:.3} IB(s) (stddev {:.3}).",
                ibs_per_tx.mean, ibs_per_tx.std_dev,
            );
            info!(
                "Each IB contained an average of {:.3} transaction(s) (stddev {:.3}) and an average of {} (stddev {:.3}). {} IB(s) were empty.",
                txs_per_ib.mean, txs_per_ib.std_dev,
                pretty_bytes(bytes_per_ib.mean.trunc() as u64, pbo.clone()), pretty_bytes(bytes_per_ib.std_dev.trunc() as u64, pbo.clone()),
                ibs.values().filter(|ib| ib.is_empty()).count(),
            );
            info!(
                "Each node received an average of {:.3} IB(s) (stddev {:.3}).",
                ibs_received.mean, ibs_received.std_dev,
            );
            info!(
                "{} EB(s) were generated; on average there were {:.3} EB(s) per slot.",
                ebs.len(),
                ebs.len() as f64 / total_slots as f64
            );
            info!(
                "Each EB contained an average of {:.3} transaction(s) (stddev {:.3}). {} EB(s) were empty.",
                txs_per_eb.mean, txs_per_eb.std_dev, empty_ebs
            );
            info!(
                "Each EB contained an average of {:.3} IB(s) (stddev {:.3}). {} EB(s) were empty.",
                ibs_per_eb.mean, ibs_per_eb.std_dev, empty_ebs
            );
            info!(
                "Each IB was included in an average of {:.3} EB(s) (stddev {:.3}).",
                ebs_per_ib.mean, ebs_per_ib.std_dev,
            );
            info!(
                "{} out of {} IBs were included in at least one EB.",
                ibs.values().filter(|ib| ib.included_in_eb.is_some()).count(), ibs.len(),
            );
            info!(
                "{} out of {} IBs expired before they reached an EB.",
                expired_ibs, ibs.len(),
            );
            info!(
                "{} out of {} EBs expired before an EB from their stage reached an RB.",
                expired_ebs, ebs.len(),
            );
            info!(
                "{} out of {} transaction(s) were included in at least one EB.",
                times_to_reach_eb.len(), txs.len(),
            );
            info!("{} total votes were generated.", total_votes);
            info!("Each stake pool produced an average of {:.3} vote(s) (stddev {:.3}).",
                votes_per_pool.mean, votes_per_pool.std_dev);
            info!("Each EB received an average of {:.3} vote(s) (stddev {:.3}).",
                votes_per_eb.mean, votes_per_eb.std_dev);
            info!("There were {bundle_count} bundle(s) of votes. Each bundle contained {:.3} vote(s) (stddev {:.3}).",
                votes_per_bundle.mean, votes_per_bundle.std_dev);
            info!("{} L1 block(s) had a Leios endorsement.", leios_blocks_with_endorsements);
            info!("{} tx(s) ({}) were referenced by a Leios endorsement.", leios_txs, pretty_bytes(leios_tx_bytes, pbo.clone()));
            info!("{} tx(s) ({}) were included directly in a Praos block.", praos_txs, pretty_bytes(praos_tx_bytes, pbo.clone()));
            info!("Spatial efficiency: {}/{} ({:.3}%) of Leios bytes were unique transactions.", pretty_bytes(leios_tx_bytes, pbo.clone()), pretty_bytes(total_leios_bytes, pbo.clone()),
                  (leios_tx_bytes as f64 / total_leios_bytes as f64) * 100.);
            info!("{} tx(s) ({:.3}%) referenced by a Leios endorsement were redundant.", total_leios_txs - leios_txs, (total_leios_txs - leios_txs) as f64 / total_leios_txs as f64 * 100.);
            info!(
                "Each transaction took an average of {:.3}s (stddev {:.3}) to be included in an IB.",
                ib_time_stats.mean, ib_time_stats.std_dev,
            );
            info!(
                "Each transaction took an average of {:.3}s (stddev {:.3}) to be included in an EB.",
                eb_time_stats.mean, eb_time_stats.std_dev,
            );
            info!(
                "Each transaction took an average of {:.3}s (stddev {:.3}) to be included in a block.",
                block_time_stats.mean, block_time_stats.std_dev,
            );
        });

        info_span!("network").in_scope(|| {
            tx_messages.display("TX");
            ib_messages.display("IB");
            eb_messages.display("EB");
            vote_messages.display("Vote");
        });

        self.write_pricing_outputs(&output_dir).await?;

        let latency = compute_latency_stats(&self.pricing_metrics.latency_samples_slots);
        let inclusion_rate = if self.pricing_metrics.submissions == 0 {
            0.0
        } else {
            self.pricing_metrics.included as f64 / self.pricing_metrics.submissions as f64
        };
        let fee_per_byte = if self.pricing_metrics.total_included_bytes == 0 {
            0.0
        } else {
            self.pricing_metrics.total_fees as f64
                / self.pricing_metrics.total_included_bytes as f64
        };
        let fee_per_tx = if self.pricing_metrics.included == 0 {
            0.0
        } else {
            self.pricing_metrics.total_fees as f64 / self.pricing_metrics.included as f64
        };
        let unique_inclusion_rate = if self.pricing_metrics.unique_generated == 0 {
            0.0
        } else {
            self.pricing_metrics.included as f64 / self.pricing_metrics.unique_generated as f64
        };
        let retained_value_ratio_generated = if self.pricing_metrics.generated_value_total == 0 {
            0.0
        } else {
            self.pricing_metrics.retained_value_total as f64
                / self.pricing_metrics.generated_value_total as f64
        };
        let retained_value_ratio_settled = if self.pricing_metrics.settled_initial_value_total == 0
        {
            0.0
        } else {
            self.pricing_metrics.retained_value_total as f64
                / self.pricing_metrics.settled_initial_value_total as f64
        };
        let net_utility_per_generated_tx = if self.pricing_metrics.unique_generated == 0 {
            0.0
        } else {
            self.pricing_metrics.net_utility_total as f64
                / self.pricing_metrics.unique_generated as f64
        };
        let max_tier_count = self
            .time_series
            .iter()
            .map(|point| point.rb_tier_count.max(point.eb_tier_count))
            .max()
            .unwrap_or(0);

        Ok(RunSummary {
            submissions: self.pricing_metrics.submissions,
            unique_generated: self.pricing_metrics.unique_generated,
            rejected: self.pricing_metrics.rejected,
            included: self.pricing_metrics.included,
            inclusion_rate,
            unique_inclusion_rate,
            tier_delay_unit: self.tier_delay_unit,
            latency_mean_slots: latency.mean,
            latency_p95_slots: latency.p95,
            latency_p99_slots: latency.p99,
            fees_total: self.pricing_metrics.total_fees,
            fee_per_byte,
            fee_per_tx,
            retained_value_total: self.pricing_metrics.retained_value_total,
            retained_value_ratio_generated,
            retained_value_ratio_settled,
            net_utility_total: self.pricing_metrics.net_utility_total,
            net_utility_per_generated_tx,
            rb_generated,
            eb_generated,
            max_tier_count,
        })
    }

    async fn write_pricing_outputs(&self, output_dir: &PathBuf) -> Result<()> {
        let mut points = self.time_series.clone();
        points.sort_unstable_by_key(|point| point.slot);

        if self.pricing_metrics.submissions == 0
            && self.pricing_metrics.rejected == 0
            && points.is_empty()
        {
            return Ok(());
        }

        if !output_dir.as_path().exists() {
            fs::create_dir_all(output_dir).await?;
        }

        let metrics_text = format_metrics_table(
            &self.pricing_metrics,
            &self.actor_names,
            &self.urgency_class_names,
            self.seed,
            self.tier_delay_unit,
        );
        let metrics_path = output_dir.join("metrics_comparison.txt");
        fs::write(metrics_path, metrics_text).await?;

        let time_series_text = format_time_series_csv(&points);
        let time_series_path = output_dir.join("time_series.csv");
        fs::write(time_series_path, time_series_text).await?;

        let time_series_html = format_time_series_html(&points, self.tier_delay_unit);
        let time_series_html_path = output_dir.join("tiered_plot.html");
        fs::write(time_series_html_path, time_series_html).await?;

        Ok(())
    }

    fn upsert_time_series(&mut self, point: TimeSeriesPoint) {
        if let Some(&index) = self.time_series_slot_index.get(&point.slot) {
            let existing = &mut self.time_series[index];
            existing.rb_tier_count = point.rb_tier_count;
            existing.rb_tier_prices = point.rb_tier_prices;
            existing.rb_tier_delays = point.rb_tier_delays;
            existing.rb_tier_capacities = point.rb_tier_capacities;
            existing.rb_tier_utilisations = point.rb_tier_utilisations;
            existing.eb_tier_count = point.eb_tier_count;
            existing.eb_tier_prices = point.eb_tier_prices;
            existing.eb_tier_delays = point.eb_tier_delays;
            existing.eb_tier_capacities = point.eb_tier_capacities;
            existing.eb_tier_utilisations = point.eb_tier_utilisations;
            existing.cumulative_inclusions = existing
                .cumulative_inclusions
                .max(point.cumulative_inclusions);
            existing.cumulative_rb_inclusions = existing
                .cumulative_rb_inclusions
                .max(point.cumulative_rb_inclusions);
            existing.cumulative_eb_inclusions = existing
                .cumulative_eb_inclusions
                .max(point.cumulative_eb_inclusions);
            existing.cumulative_block_inclusions_total = existing
                .cumulative_block_inclusions_total
                .max(point.cumulative_block_inclusions_total);
            existing.cumulative_block_inclusions_with_delay = existing
                .cumulative_block_inclusions_with_delay
                .max(point.cumulative_block_inclusions_with_delay);
            existing.cumulative_submitted_bytes = existing
                .cumulative_submitted_bytes
                .max(point.cumulative_submitted_bytes);
            existing.cumulative_included_bytes = existing
                .cumulative_included_bytes
                .max(point.cumulative_included_bytes);
            existing.cumulative_fees = existing.cumulative_fees.max(point.cumulative_fees);
            existing.cumulative_rb_tier_assignments_total = existing
                .cumulative_rb_tier_assignments_total
                .max(point.cumulative_rb_tier_assignments_total);
            existing.cumulative_rb_tier_assignments_max_priced = existing
                .cumulative_rb_tier_assignments_max_priced
                .max(point.cumulative_rb_tier_assignments_max_priced);
            merge_cumulative_counts(
                &mut existing.cumulative_rb_tier_assignments_by_tier,
                &point.cumulative_rb_tier_assignments_by_tier,
            );
            existing.cumulative_eb_tier_assignments_total = existing
                .cumulative_eb_tier_assignments_total
                .max(point.cumulative_eb_tier_assignments_total);
            existing.cumulative_eb_tier_assignments_max_priced = existing
                .cumulative_eb_tier_assignments_max_priced
                .max(point.cumulative_eb_tier_assignments_max_priced);
            merge_cumulative_counts(
                &mut existing.cumulative_eb_tier_assignments_by_tier,
                &point.cumulative_eb_tier_assignments_by_tier,
            );
            return;
        }

        let slot = point.slot;
        self.time_series.push(point);
        self.time_series_slot_index
            .insert(slot, self.time_series.len() - 1);
    }

    fn pricing_output_dir(&self) -> PathBuf {
        let Some(path) = &self.output_path else {
            return PathBuf::from(".");
        };

        if path.is_dir() {
            return path.clone();
        }

        if path.extension().is_none() {
            return path.clone();
        }

        path.parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

#[derive(Clone)]
struct Transaction {
    bytes: u64,
    generated: Timestamp,
    submission_slot: u64,
    actor_id: ActorId,
    urgency: UrgencyProfile,
    value: u64,
    urgency_component_index: Option<u16>,
    posted_fee: Option<u64>,
    tier: Option<TierId>,
    tier_version_created_slot: Option<u64>,
    tier_delay_slots: Option<u64>,
    eb_tier_version_created_slot: Option<u64>,
    eb_posted_fee: Option<u64>,
    eb_tier_delay_slots: Option<u64>,
    submitted: bool,
    rejected: bool,
    included_in_ib: Option<Timestamp>,
    included_in_eb: Option<Timestamp>,
    included_in_block: Option<Timestamp>,
    tx_type: Option<TransactionType>,
}
impl Transaction {
    fn new(
        bytes: u64,
        generated: Timestamp,
        submission_slot: u64,
        actor_id: ActorId,
        urgency: UrgencyProfile,
        value: u64,
        urgency_component_index: Option<u16>,
    ) -> Self {
        Self {
            bytes,
            generated,
            submission_slot,
            actor_id,
            urgency,
            value,
            urgency_component_index,
            posted_fee: None,
            tier: None,
            tier_version_created_slot: None,
            tier_delay_slots: None,
            eb_tier_version_created_slot: None,
            eb_posted_fee: None,
            eb_tier_delay_slots: None,
            submitted: false,
            rejected: false,
            included_in_ib: None,
            included_in_eb: None,
            included_in_block: None,
            tx_type: None,
        }
    }
}

#[derive(Clone)]
#[derive(Default)]
struct PricingMetrics {
    unique_generated: u64,
    generated_value_total: u128,
    submissions: u64,
    rejected: u64,
    rejected_too_expensive: u64,
    rejected_tier_backlog_full: u64,
    rejected_invalid_quoted_assignment: u64,
    rejected_quoted_history_unavailable: u64,
    rejected_after_assignment_non_overflow: u64,
    rejected_after_assignment_overflow: u64,
    retries_scheduled_total: u64,
    retries_scheduled_rb: u64,
    retries_scheduled_eb: u64,
    retries_scheduled_by_lane_tier: BTreeMap<String, u64>,
    overflow_checks_total: u64,
    overflow_checks_overfull: u64,
    overflow_rejects_total: u64,
    overflow_rejects_retry_scheduled: u64,
    overflow_pending_ratio_sum: f64,
    overflow_pending_ratio_samples: u64,
    overflow_checks_by_lane_tier: BTreeMap<String, u64>,
    overflow_rejects_by_lane_tier: BTreeMap<String, u64>,
    delay_updates_rb: u64,
    delay_updates_eb: u64,
    tier_updates_rb: u64,
    tier_updates_eb: u64,
    delay_update_slot_interval_sum_rb: u64,
    delay_update_slot_interval_sum_eb: u64,
    delay_update_slot_interval_samples_rb: u64,
    delay_update_slot_interval_samples_eb: u64,
    tier_update_slot_interval_sum_rb: u64,
    tier_update_slot_interval_sum_eb: u64,
    tier_update_slot_interval_samples_rb: u64,
    tier_update_slot_interval_samples_eb: u64,
    last_delay_update_slot_rb: Option<u64>,
    last_delay_update_slot_eb: Option<u64>,
    last_tier_update_slot_rb: Option<u64>,
    last_tier_update_slot_eb: Option<u64>,
    block_included_total: u64,
    block_included_with_delay: u64,
    included: u64,
    settled_with_delay: u64,
    total_fees: u128,
    total_submitted_bytes: u64,
    total_included_bytes: u64,
    settled_initial_value_total: u128,
    retained_value_total: u128,
    net_utility_total: i128,
    settled_value_retention_ratio_sum: f64,
    settled_value_retention_ratio_samples: u64,
    rb_tier_assignments_total: u64,
    rb_tier_assignments_to_max_priced_tier: u64,
    rb_tier_assignments_by_tier: Vec<u64>,
    eb_tier_assignments_total: u64,
    eb_tier_assignments_to_max_priced_tier: u64,
    eb_tier_assignments_by_tier: Vec<u64>,
    latency_samples_slots: Vec<u64>,
    per_actor: BTreeMap<ActorId, ActorMetrics>,
    /// Per urgency-class metrics, keyed by (actor_id, component_index).
    per_urgency_class: BTreeMap<(ActorId, u16), ActorMetrics>,
}

#[derive(Clone, Default)]
struct ActorMetrics {
    unique_generated: u64,
    submissions: u64,
    rejected: u64,
    retries_scheduled: u64,
    included: u64,
    fees_paid: u128,
    latency_samples_slots: Vec<u64>,
    generated_value_total: u128,
    settled_initial_value_total: u128,
    retained_value_total: u128,
    net_utility_total: i128,
    settled_value_retention_ratio_sum: f64,
    settled_value_retention_ratio_samples: u64,
}

impl PricingMetrics {
    fn lane_tier_key(block_kind: BlockKind, tier: TierId) -> String {
        match block_kind {
            BlockKind::RankingBlock => format!("rb:{}", tier),
            BlockKind::EndorserBlock => format!("eb:{}", tier),
        }
    }

    fn lane_tier_key_for_retry(lane: RetryLane, tier: TierId) -> String {
        match lane {
            RetryLane::Ranking => format!("rb:{}", tier),
            RetryLane::Endorser => format!("eb:{}", tier),
        }
    }

    fn record_generated(
        &mut self,
        actor_id: ActorId,
        initial_value: u64,
        urgency_component_index: Option<u16>,
    ) {
        self.unique_generated = self.unique_generated.saturating_add(1);
        self.generated_value_total = self
            .generated_value_total
            .saturating_add(initial_value as u128);
        let entry = self.per_actor.entry(actor_id).or_default();
        entry.unique_generated = entry.unique_generated.saturating_add(1);
        entry.generated_value_total = entry
            .generated_value_total
            .saturating_add(initial_value as u128);
        if let Some(idx) = urgency_component_index {
            let uc = self.per_urgency_class.entry((actor_id, idx)).or_default();
            uc.unique_generated = uc.unique_generated.saturating_add(1);
            uc.generated_value_total = uc
                .generated_value_total
                .saturating_add(initial_value as u128);
        }
    }

    fn record_submission(&mut self, actor_id: ActorId, bytes: u64) {
        self.submissions += 1;
        self.total_submitted_bytes = self.total_submitted_bytes.saturating_add(bytes);
        self.per_actor.entry(actor_id).or_default().submissions += 1;
    }

    fn record_tier_assignment(
        &mut self,
        block_kind: BlockKind,
        tier: TierId,
        lane_prices_by_id: &BTreeMap<TierId, u64>,
    ) {
        let Some(tier_index) = tier_id_to_index(tier) else {
            return;
        };
        let (total, to_max_priced, by_tier) = match block_kind {
            BlockKind::RankingBlock => (
                &mut self.rb_tier_assignments_total,
                &mut self.rb_tier_assignments_to_max_priced_tier,
                &mut self.rb_tier_assignments_by_tier,
            ),
            BlockKind::EndorserBlock => (
                &mut self.eb_tier_assignments_total,
                &mut self.eb_tier_assignments_to_max_priced_tier,
                &mut self.eb_tier_assignments_by_tier,
            ),
        };

        *total = total.saturating_add(1);
        ensure_count_capacity(by_tier, tier_index);
        by_tier[tier_index] = by_tier[tier_index].saturating_add(1);

        let Some(assigned_price) = lane_prices_by_id.get(&tier).copied() else {
            return;
        };
        let max_price = lane_prices_by_id.values().copied().max().unwrap_or(0);
        if assigned_price == max_price {
            *to_max_priced = to_max_priced.saturating_add(1);
        }
    }

    fn record_rejection(
        &mut self,
        actor_id: ActorId,
        reason: TransactionRejectReason,
        after_assignment: bool,
    ) {
        self.rejected += 1;
        match reason {
            TransactionRejectReason::TooExpensive => {
                self.rejected_too_expensive += 1;
            }
            TransactionRejectReason::TierBacklogFull => {
                self.rejected_tier_backlog_full += 1;
            }
            TransactionRejectReason::InvalidQuotedAssignment => {
                self.rejected_invalid_quoted_assignment += 1;
            }
            TransactionRejectReason::QuotedHistoryUnavailable => {
                self.rejected_quoted_history_unavailable += 1;
            }
        }
        if after_assignment {
            match reason {
                TransactionRejectReason::TierBacklogFull => {
                    self.rejected_after_assignment_overflow += 1;
                }
                _ => {
                    self.rejected_after_assignment_non_overflow += 1;
                }
            }
        }
        self.per_actor.entry(actor_id).or_default().rejected += 1;
    }

    fn record_retry_scheduled(&mut self, actor_id: ActorId, lane: RetryLane, tier: TierId) {
        self.retries_scheduled_total = self.retries_scheduled_total.saturating_add(1);
        match lane {
            RetryLane::Ranking => {
                self.retries_scheduled_rb = self.retries_scheduled_rb.saturating_add(1)
            }
            RetryLane::Endorser => {
                self.retries_scheduled_eb = self.retries_scheduled_eb.saturating_add(1)
            }
        }
        let key = Self::lane_tier_key_for_retry(lane, tier);
        let entry = self.retries_scheduled_by_lane_tier.entry(key).or_default();
        *entry = entry.saturating_add(1);
        let entry = self.per_actor.entry(actor_id).or_default();
        entry.retries_scheduled = entry.retries_scheduled.saturating_add(1);
    }

    fn record_overflow_checked(
        &mut self,
        block_kind: BlockKind,
        tier: TierId,
        pending_bytes: u64,
        tier_capacity_bytes: u64,
        overfull: bool,
    ) {
        self.overflow_checks_total = self.overflow_checks_total.saturating_add(1);
        if overfull {
            self.overflow_checks_overfull = self.overflow_checks_overfull.saturating_add(1);
        }
        if tier_capacity_bytes > 0 {
            self.overflow_pending_ratio_sum += pending_bytes as f64 / tier_capacity_bytes as f64;
            self.overflow_pending_ratio_samples =
                self.overflow_pending_ratio_samples.saturating_add(1);
        }
        let key = Self::lane_tier_key(block_kind, tier);
        let entry = self.overflow_checks_by_lane_tier.entry(key).or_default();
        *entry = entry.saturating_add(1);
    }

    fn record_overflow_rejected(
        &mut self,
        block_kind: BlockKind,
        tier: TierId,
        retry_scheduled: bool,
    ) {
        self.overflow_rejects_total = self.overflow_rejects_total.saturating_add(1);
        if retry_scheduled {
            self.overflow_rejects_retry_scheduled =
                self.overflow_rejects_retry_scheduled.saturating_add(1);
        }
        let key = Self::lane_tier_key(block_kind, tier);
        let entry = self.overflow_rejects_by_lane_tier.entry(key).or_default();
        *entry = entry.saturating_add(1);
    }

    fn record_cadence_update(
        &mut self,
        block_kind: BlockKind,
        slot: u64,
        delay_update_triggered: bool,
        tier_update_triggered: bool,
    ) {
        if delay_update_triggered {
            match block_kind {
                BlockKind::RankingBlock => {
                    self.delay_updates_rb = self.delay_updates_rb.saturating_add(1);
                    if let Some(previous) = self.last_delay_update_slot_rb {
                        self.delay_update_slot_interval_sum_rb = self
                            .delay_update_slot_interval_sum_rb
                            .saturating_add(slot.saturating_sub(previous));
                        self.delay_update_slot_interval_samples_rb =
                            self.delay_update_slot_interval_samples_rb.saturating_add(1);
                    }
                    self.last_delay_update_slot_rb = Some(slot);
                }
                BlockKind::EndorserBlock => {
                    self.delay_updates_eb = self.delay_updates_eb.saturating_add(1);
                    if let Some(previous) = self.last_delay_update_slot_eb {
                        self.delay_update_slot_interval_sum_eb = self
                            .delay_update_slot_interval_sum_eb
                            .saturating_add(slot.saturating_sub(previous));
                        self.delay_update_slot_interval_samples_eb =
                            self.delay_update_slot_interval_samples_eb.saturating_add(1);
                    }
                    self.last_delay_update_slot_eb = Some(slot);
                }
            }
        }

        if tier_update_triggered {
            match block_kind {
                BlockKind::RankingBlock => {
                    self.tier_updates_rb = self.tier_updates_rb.saturating_add(1);
                    if let Some(previous) = self.last_tier_update_slot_rb {
                        self.tier_update_slot_interval_sum_rb = self
                            .tier_update_slot_interval_sum_rb
                            .saturating_add(slot.saturating_sub(previous));
                        self.tier_update_slot_interval_samples_rb =
                            self.tier_update_slot_interval_samples_rb.saturating_add(1);
                    }
                    self.last_tier_update_slot_rb = Some(slot);
                }
                BlockKind::EndorserBlock => {
                    self.tier_updates_eb = self.tier_updates_eb.saturating_add(1);
                    if let Some(previous) = self.last_tier_update_slot_eb {
                        self.tier_update_slot_interval_sum_eb = self
                            .tier_update_slot_interval_sum_eb
                            .saturating_add(slot.saturating_sub(previous));
                        self.tier_update_slot_interval_samples_eb =
                            self.tier_update_slot_interval_samples_eb.saturating_add(1);
                    }
                    self.last_tier_update_slot_eb = Some(slot);
                }
            }
        }
    }

    fn record_block_inclusion(&mut self, tier_delay_slots: u64) {
        self.block_included_total += 1;
        if tier_delay_slots > 1 {
            self.block_included_with_delay += 1;
        }
    }

    fn record_inclusion(
        &mut self,
        actor_id: ActorId,
        posted_fee: Option<u64>,
        bytes: u64,
        latency: u64,
        tier_delay_slots: u64,
        initial_value: u64,
        urgency: &UrgencyProfile,
        urgency_component_index: Option<u16>,
    ) {
        let value_stats = settlement_value_stats(initial_value, urgency, latency, posted_fee);
        self.included += 1;
        if tier_delay_slots > 1 {
            self.settled_with_delay += 1;
        }
        self.total_included_bytes = self.total_included_bytes.saturating_add(bytes);
        if let Some(fee) = posted_fee {
            self.total_fees = self.total_fees.saturating_add(fee as u128);
        }
        self.settled_initial_value_total = self
            .settled_initial_value_total
            .saturating_add(initial_value as u128);
        self.retained_value_total = self
            .retained_value_total
            .saturating_add(value_stats.retained_value as u128);
        self.net_utility_total = self
            .net_utility_total
            .saturating_add(value_stats.net_utility);
        if let Some(retention_ratio) = value_stats.retention_ratio {
            self.settled_value_retention_ratio_sum += retention_ratio;
            self.settled_value_retention_ratio_samples =
                self.settled_value_retention_ratio_samples.saturating_add(1);
        }
        self.latency_samples_slots.push(latency);
        let entry = self.per_actor.entry(actor_id).or_default();
        entry.included += 1;
        if let Some(fee) = posted_fee {
            entry.fees_paid = entry.fees_paid.saturating_add(fee as u128);
        }
        entry.settled_initial_value_total = entry
            .settled_initial_value_total
            .saturating_add(initial_value as u128);
        entry.retained_value_total = entry
            .retained_value_total
            .saturating_add(value_stats.retained_value as u128);
        entry.net_utility_total = entry
            .net_utility_total
            .saturating_add(value_stats.net_utility);
        if let Some(retention_ratio) = value_stats.retention_ratio {
            entry.settled_value_retention_ratio_sum += retention_ratio;
            entry.settled_value_retention_ratio_samples = entry
                .settled_value_retention_ratio_samples
                .saturating_add(1);
        }
        entry.latency_samples_slots.push(latency);
        if let Some(idx) = urgency_component_index {
            let uc = self.per_urgency_class.entry((actor_id, idx)).or_default();
            uc.included += 1;
            if let Some(fee) = posted_fee {
                uc.fees_paid = uc.fees_paid.saturating_add(fee as u128);
            }
            uc.settled_initial_value_total = uc
                .settled_initial_value_total
                .saturating_add(initial_value as u128);
            uc.retained_value_total = uc
                .retained_value_total
                .saturating_add(value_stats.retained_value as u128);
            uc.net_utility_total = uc
                .net_utility_total
                .saturating_add(value_stats.net_utility);
            if let Some(retention_ratio) = value_stats.retention_ratio {
                uc.settled_value_retention_ratio_sum += retention_ratio;
                uc.settled_value_retention_ratio_samples = uc
                    .settled_value_retention_ratio_samples
                    .saturating_add(1);
            }
            uc.latency_samples_slots.push(latency);
        }
    }
}

#[derive(Clone)]
struct TimeSeriesPoint {
    slot: u64,
    rb_tier_count: usize,
    rb_tier_prices: Vec<u64>,
    rb_tier_delays: Vec<u64>,
    rb_tier_capacities: Vec<u64>,
    rb_tier_utilisations: Vec<f64>,
    eb_tier_count: usize,
    eb_tier_prices: Vec<u64>,
    eb_tier_delays: Vec<u64>,
    eb_tier_capacities: Vec<u64>,
    eb_tier_utilisations: Vec<f64>,
    cumulative_inclusions: u64,
    cumulative_rb_inclusions: u64,
    cumulative_eb_inclusions: u64,
    cumulative_block_inclusions_total: u64,
    cumulative_block_inclusions_with_delay: u64,
    cumulative_submitted_bytes: u64,
    cumulative_included_bytes: u64,
    cumulative_fees: u128,
    cumulative_rb_tier_assignments_total: u64,
    cumulative_rb_tier_assignments_max_priced: u64,
    cumulative_rb_tier_assignments_by_tier: Vec<u64>,
    cumulative_eb_tier_assignments_total: u64,
    cumulative_eb_tier_assignments_max_priced: u64,
    cumulative_eb_tier_assignments_by_tier: Vec<u64>,
}

fn record_inclusion(
    metrics: &mut PricingMetrics,
    txs: &mut BTreeMap<TransactionId, Transaction>,
    tx_id: TransactionId,
    time: Timestamp,
    block_kind: BlockKind,
) {
    let Some(tx) = txs.get_mut(&tx_id) else {
        return;
    };
    if tx.included_in_block.is_some() {
        return;
    }
    tx.included_in_block = Some(time);
    let included_slot = (time - Timestamp::zero()).as_secs();
    let (posted_fee, tier_delay_slots) = match block_kind {
        BlockKind::RankingBlock => (tx.posted_fee, tx.tier_delay_slots.unwrap_or(1)),
        BlockKind::EndorserBlock => (
            tx.eb_posted_fee.or(tx.posted_fee),
            tx.eb_tier_delay_slots.or(tx.tier_delay_slots).unwrap_or(1),
        ),
    };
    metrics.record_block_inclusion(tier_delay_slots);
    let latency = included_slot.saturating_sub(tx.submission_slot);
    metrics.record_inclusion(
        tx.actor_id,
        posted_fee,
        tx.bytes,
        latency,
        tier_delay_slots,
        tx.value,
        &tx.urgency,
        tx.urgency_component_index,
    );
}

struct SettlementValueStats {
    retained_value: u64,
    net_utility: i128,
    retention_ratio: Option<f64>,
}

fn settlement_value_stats(
    initial_value: u64,
    urgency: &UrgencyProfile,
    latency: u64,
    posted_fee: Option<u64>,
) -> SettlementValueStats {
    let retained_value = urgency.value_at_delay(initial_value, latency);
    let paid_fee = posted_fee.unwrap_or(0);
    let net_utility = retained_value as i128 - paid_fee as i128;
    let retention_ratio = if initial_value == 0 {
        None
    } else {
        Some(retained_value as f64 / initial_value as f64)
    };
    SettlementValueStats {
        retained_value,
        net_utility,
        retention_ratio,
    }
}

fn tier_delay_unit_label(unit: TierDelayUnit) -> &'static str {
    match unit {
        TierDelayUnit::Slots => "slots",
        TierDelayUnit::Blocks => "blocks",
    }
}

#[derive(Default)]
struct LatencyStats {
    mean: f64,
    p50: f64,
    p95: f64,
    p99: f64,
}

fn compute_latency_stats(samples: &[u64]) -> LatencyStats {
    if samples.is_empty() {
        return LatencyStats::default();
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let count = sorted.len() as f64;
    let sum = sorted.iter().map(|v| *v as f64).sum::<f64>();
    LatencyStats {
        mean: sum / count,
        p50: percentile(&sorted, 50.0),
        p95: percentile(&sorted, 95.0),
        p99: percentile(&sorted, 99.0),
    }
}

fn percentile(sorted: &[u64], pct: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = ((pct / 100.0) * (sorted.len().saturating_sub(1)) as f64).round() as usize;
    sorted[rank] as f64
}

fn format_metrics_table(
    metrics: &PricingMetrics,
    actor_names: &BTreeMap<ActorId, String>,
    urgency_class_names: &BTreeMap<(ActorId, u16), String>,
    seed: u64,
    tier_delay_unit: TierDelayUnit,
) -> String {
    let latency = compute_latency_stats(&metrics.latency_samples_slots);
    let inclusion_rate = if metrics.submissions == 0 {
        0.0
    } else {
        metrics.included as f64 / metrics.submissions as f64
    };
    let unique_inclusion_rate = if metrics.unique_generated == 0 {
        0.0
    } else {
        metrics.included as f64 / metrics.unique_generated as f64
    };
    let fee_per_byte = if metrics.total_included_bytes == 0 {
        0.0
    } else {
        metrics.total_fees as f64 / metrics.total_included_bytes as f64
    };
    let fee_per_tx = if metrics.included == 0 {
        0.0
    } else {
        metrics.total_fees as f64 / metrics.included as f64
    };
    let retained_value_ratio_generated = if metrics.generated_value_total == 0 {
        0.0
    } else {
        metrics.retained_value_total as f64 / metrics.generated_value_total as f64
    };
    let retained_value_ratio_settled = if metrics.settled_initial_value_total == 0 {
        0.0
    } else {
        metrics.retained_value_total as f64 / metrics.settled_initial_value_total as f64
    };
    let settled_value_retention_mean = if metrics.settled_value_retention_ratio_samples == 0 {
        0.0
    } else {
        metrics.settled_value_retention_ratio_sum
            / metrics.settled_value_retention_ratio_samples as f64
    };
    let net_utility_per_generated_tx = if metrics.unique_generated == 0 {
        0.0
    } else {
        metrics.net_utility_total as f64 / metrics.unique_generated as f64
    };
    let net_utility_per_included_tx = if metrics.included == 0 {
        0.0
    } else {
        metrics.net_utility_total as f64 / metrics.included as f64
    };
    let overflow_pending_ratio_mean = if metrics.overflow_pending_ratio_samples == 0 {
        0.0
    } else {
        metrics.overflow_pending_ratio_sum / metrics.overflow_pending_ratio_samples as f64
    };
    let delay_update_mean_interval_rb = if metrics.delay_update_slot_interval_samples_rb == 0 {
        0.0
    } else {
        metrics.delay_update_slot_interval_sum_rb as f64
            / metrics.delay_update_slot_interval_samples_rb as f64
    };
    let delay_update_mean_interval_eb = if metrics.delay_update_slot_interval_samples_eb == 0 {
        0.0
    } else {
        metrics.delay_update_slot_interval_sum_eb as f64
            / metrics.delay_update_slot_interval_samples_eb as f64
    };
    let tier_update_mean_interval_rb = if metrics.tier_update_slot_interval_samples_rb == 0 {
        0.0
    } else {
        metrics.tier_update_slot_interval_sum_rb as f64
            / metrics.tier_update_slot_interval_samples_rb as f64
    };
    let tier_update_mean_interval_eb = if metrics.tier_update_slot_interval_samples_eb == 0 {
        0.0
    } else {
        metrics.tier_update_slot_interval_sum_eb as f64
            / metrics.tier_update_slot_interval_samples_eb as f64
    };

    let mut out = String::new();
    use std::fmt::Write as _;
    writeln!(out, "Seed: {}", seed).ok();
    writeln!(out).ok();
    writeln!(out, "Metric                      | Value").ok();
    writeln!(out, "---------------------------|--------------------").ok();
    writeln!(
        out,
        "Unique txs generated       | {}",
        metrics.unique_generated
    )
    .ok();
    writeln!(
        out,
        "Unique tx inclusion rate   | {:.2}%",
        unique_inclusion_rate * 100.0
    )
    .ok();
    writeln!(out, "Submissions                | {}", metrics.submissions).ok();
    writeln!(out, "Included                   | {}", metrics.included).ok();
    writeln!(out, "Rejected                   | {}", metrics.rejected).ok();
    writeln!(
        out,
        "Rejected (too_expensive)   | {}",
        metrics.rejected_too_expensive
    )
    .ok();
    writeln!(
        out,
        "Rejected (tier_backlog)    | {}",
        metrics.rejected_tier_backlog_full
    )
    .ok();
    writeln!(
        out,
        "Rejected (invalid_quote)   | {}",
        metrics.rejected_invalid_quoted_assignment
    )
    .ok();
    writeln!(
        out,
        "Rejected (history_unavail) | {}",
        metrics.rejected_quoted_history_unavailable
    )
    .ok();
    writeln!(
        out,
        "Retries scheduled          | {}",
        metrics.retries_scheduled_total
    )
    .ok();
    writeln!(
        out,
        "Retries scheduled (rb)     | {}",
        metrics.retries_scheduled_rb
    )
    .ok();
    writeln!(
        out,
        "Retries scheduled (eb)     | {}",
        metrics.retries_scheduled_eb
    )
    .ok();
    writeln!(
        out,
        "Retries by lane+tier       | {}",
        join_labeled_counts(&metrics.retries_scheduled_by_lane_tier)
    )
    .ok();
    writeln!(
        out,
        "Overflow checks            | {}",
        metrics.overflow_checks_total
    )
    .ok();
    writeln!(
        out,
        "Overflow checks (overfull) | {}",
        metrics.overflow_checks_overfull
    )
    .ok();
    writeln!(
        out,
        "Overflow rejects           | {}",
        metrics.overflow_rejects_total
    )
    .ok();
    writeln!(
        out,
        "Overflow rejects retried   | {}",
        metrics.overflow_rejects_retry_scheduled
    )
    .ok();
    writeln!(
        out,
        "Overflow mean pending/cap  | {:.4}",
        overflow_pending_ratio_mean
    )
    .ok();
    writeln!(
        out,
        "Delay updates fired (rb)   | {}",
        metrics.delay_updates_rb
    )
    .ok();
    writeln!(
        out,
        "Delay updates fired (eb)   | {}",
        metrics.delay_updates_eb
    )
    .ok();
    writeln!(
        out,
        "Tier checks fired (rb)     | {}",
        metrics.tier_updates_rb
    )
    .ok();
    writeln!(
        out,
        "Tier checks fired (eb)     | {}",
        metrics.tier_updates_eb
    )
    .ok();
    writeln!(
        out,
        "Delay mean interval (rb)   | {:.2}",
        delay_update_mean_interval_rb
    )
    .ok();
    writeln!(
        out,
        "Delay mean interval (eb)   | {:.2}",
        delay_update_mean_interval_eb
    )
    .ok();
    writeln!(
        out,
        "Tier mean interval (rb)    | {:.2}",
        tier_update_mean_interval_rb
    )
    .ok();
    writeln!(
        out,
        "Tier mean interval (eb)    | {:.2}",
        tier_update_mean_interval_eb
    )
    .ok();
    writeln!(
        out,
        "Overflow checks lane+tier  | {}",
        join_labeled_counts(&metrics.overflow_checks_by_lane_tier)
    )
    .ok();
    writeln!(
        out,
        "Overflow rejects lane+tier | {}",
        join_labeled_counts(&metrics.overflow_rejects_by_lane_tier)
    )
    .ok();
    writeln!(
        out,
        "TX rejected after assign   | {}",
        metrics.rejected_after_assignment_non_overflow
    )
    .ok();
    writeln!(
        out,
        "TX rejected after assign (overflow) | {}",
        metrics.rejected_after_assignment_overflow
    )
    .ok();
    writeln!(
        out,
        "Never-stale invariant      | {}",
        if metrics.rejected_after_assignment_non_overflow == 0 {
            "PASS"
        } else {
            "FAIL"
        }
    )
    .ok();
    writeln!(
        out,
        "Inclusion rate             | {:.2}%",
        inclusion_rate * 100.0
    )
    .ok();
    writeln!(
        out,
        "Synthetic delay unit       | {}",
        tier_delay_unit_label(tier_delay_unit)
    )
    .ok();
    writeln!(out, "Latency mean (slots)       | {:.2}", latency.mean).ok();
    writeln!(out, "Latency p50 (slots)        | {:.2}", latency.p50).ok();
    writeln!(out, "Latency p95 (slots)        | {:.2}", latency.p95).ok();
    writeln!(out, "Latency p99 (slots)        | {:.2}", latency.p99).ok();
    writeln!(out, "Fees total                 | {}", metrics.total_fees).ok();
    writeln!(out, "Fee per byte               | {:.2}", fee_per_byte).ok();
    writeln!(out, "Fee per tx                 | {:.2}", fee_per_tx).ok();
    writeln!(
        out,
        "Generated value total      | {}",
        metrics.generated_value_total
    )
    .ok();
    writeln!(
        out,
        "Included initial value total| {}",
        metrics.settled_initial_value_total
    )
    .ok();
    writeln!(
        out,
        "Retained value total       | {}",
        metrics.retained_value_total
    )
    .ok();
    writeln!(
        out,
        "Retained / generated value | {:.2}%",
        retained_value_ratio_generated * 100.0
    )
    .ok();
    writeln!(
        out,
        "Retained / included initial| {:.2}%",
        retained_value_ratio_settled * 100.0
    )
    .ok();
    writeln!(
        out,
        "Retained mean / included tx| {:.2}%",
        settled_value_retention_mean * 100.0
    )
    .ok();
    writeln!(
        out,
        "Net utility total          | {}",
        metrics.net_utility_total
    )
    .ok();
    writeln!(
        out,
        "Net utility / generated tx | {:.2}",
        net_utility_per_generated_tx
    )
    .ok();
    writeln!(
        out,
        "Net utility / included tx  | {:.2}",
        net_utility_per_included_tx
    )
    .ok();
    writeln!(
        out,
        "RB tier assignments total  | {}",
        metrics.rb_tier_assignments_total
    )
    .ok();
    writeln!(
        out,
        "RB max-priced assigned     | {}",
        metrics.rb_tier_assignments_to_max_priced_tier
    )
    .ok();
    writeln!(
        out,
        "RB max-priced assigned %   | {:.2}%",
        ratio_pct(
            metrics.rb_tier_assignments_to_max_priced_tier,
            metrics.rb_tier_assignments_total
        )
    )
    .ok();
    writeln!(
        out,
        "RB assignments by tier     | {}",
        join_indexed_counts(&metrics.rb_tier_assignments_by_tier)
    )
    .ok();
    writeln!(
        out,
        "EB tier assignments total  | {}",
        metrics.eb_tier_assignments_total
    )
    .ok();
    writeln!(
        out,
        "EB max-priced assigned     | {}",
        metrics.eb_tier_assignments_to_max_priced_tier
    )
    .ok();
    writeln!(
        out,
        "EB max-priced assigned %   | {:.2}%",
        ratio_pct(
            metrics.eb_tier_assignments_to_max_priced_tier,
            metrics.eb_tier_assignments_total
        )
    )
    .ok();
    writeln!(
        out,
        "EB assignments by tier     | {}",
        join_indexed_counts(&metrics.eb_tier_assignments_by_tier)
    )
    .ok();

    if !metrics.per_actor.is_empty() {
        writeln!(out).ok();
        writeln!(
            out,
            "Actor                      | Submissions | Included | Rejected | Retries | Incl. rate | Latency mean (slots) | Fees paid"
        )
        .ok();
        writeln!(
            out,
            "---------------------------|-------------|----------|----------|---------|-----------|----------------------|----------"
        )
        .ok();
        for (actor_id, stats) in &metrics.per_actor {
            let name = actor_names
                .get(actor_id)
                .cloned()
                .unwrap_or_else(|| format!("actor_{}", actor_id));
            let latency = compute_latency_stats(&stats.latency_samples_slots);
            let incl_rate = if stats.submissions == 0 {
                0.0
            } else {
                stats.included as f64 / stats.submissions as f64
            };
            writeln!(
                out,
                "{:<27}| {:>11} | {:>8} | {:>8} | {:>7} | {:>9.2}% | {:>20.2} | {:>8}",
                name,
                stats.submissions,
                stats.included,
                stats.rejected,
                stats.retries_scheduled,
                incl_rate * 100.0,
                latency.mean,
                stats.fees_paid
            )
            .ok();
        }

        writeln!(out).ok();
        writeln!(
            out,
            "Actor welfare              | Generated | Included | Unique incl. rate | Retained/gen | Retained mean | Net utility total | Net util/gen tx"
        )
        .ok();
        writeln!(
            out,
            "---------------------------|-----------|----------|-------------------|--------------|---------------|-------------------|----------------"
        )
        .ok();
        for (actor_id, stats) in &metrics.per_actor {
            let name = actor_names
                .get(actor_id)
                .cloned()
                .unwrap_or_else(|| format!("actor_{}", actor_id));
            let unique_incl_rate = if stats.unique_generated == 0 {
                0.0
            } else {
                stats.included as f64 / stats.unique_generated as f64
            };
            let retained_over_generated = if stats.generated_value_total == 0 {
                0.0
            } else {
                stats.retained_value_total as f64 / stats.generated_value_total as f64
            };
            let retained_mean = if stats.settled_value_retention_ratio_samples == 0 {
                0.0
            } else {
                stats.settled_value_retention_ratio_sum
                    / stats.settled_value_retention_ratio_samples as f64
            };
            let net_per_generated = if stats.unique_generated == 0 {
                0.0
            } else {
                stats.net_utility_total as f64 / stats.unique_generated as f64
            };
            writeln!(
                out,
                "{:<27}| {:>9} | {:>8} | {:>17.2}% | {:>11.2}% | {:>12.2}% | {:>17} | {:>14.2}",
                name,
                stats.unique_generated,
                stats.included,
                unique_incl_rate * 100.0,
                retained_over_generated * 100.0,
                retained_mean * 100.0,
                stats.net_utility_total,
                net_per_generated
            )
            .ok();
        }

        // Per urgency-class welfare breakdown
        if !metrics.per_urgency_class.is_empty() {
            writeln!(out).ok();
            writeln!(
                out,
                "Urgency class welfare       | Generated | Included | Unique incl. rate | Retained/gen | Retained mean | Net utility total | Net util/gen tx | Latency mean"
            )
            .ok();
            writeln!(
                out,
                "----------------------------|-----------|----------|-------------------|--------------|---------------|-------------------|-----------------|--------------"
            )
            .ok();
            for ((actor_id, comp_idx), stats) in &metrics.per_urgency_class {
                let name = urgency_class_names
                    .get(&(*actor_id, *comp_idx))
                    .cloned()
                    .unwrap_or_else(|| format!("actor_{}:comp_{}", actor_id, comp_idx));
                let unique_incl_rate = if stats.unique_generated == 0 {
                    0.0
                } else {
                    stats.included as f64 / stats.unique_generated as f64
                };
                let retained_over_generated = if stats.generated_value_total == 0 {
                    0.0
                } else {
                    stats.retained_value_total as f64 / stats.generated_value_total as f64
                };
                let retained_mean = if stats.settled_value_retention_ratio_samples == 0 {
                    0.0
                } else {
                    stats.settled_value_retention_ratio_sum
                        / stats.settled_value_retention_ratio_samples as f64
                };
                let net_per_generated = if stats.unique_generated == 0 {
                    0.0
                } else {
                    stats.net_utility_total as f64 / stats.unique_generated as f64
                };
                let latency = compute_latency_stats(&stats.latency_samples_slots);
                writeln!(
                    out,
                    "{:<28}| {:>9} | {:>8} | {:>17.2}% | {:>11.2}% | {:>12.2}% | {:>17} | {:>15.2} | {:>12.2}",
                    name,
                    stats.unique_generated,
                    stats.included,
                    unique_incl_rate * 100.0,
                    retained_over_generated * 100.0,
                    retained_mean * 100.0,
                    stats.net_utility_total,
                    net_per_generated,
                    latency.mean,
                )
                .ok();
            }
        }
    }
    out
}

fn format_time_series_csv(points: &[TimeSeriesPoint]) -> String {
    let mut out = String::new();
    use std::fmt::Write as _;
    writeln!(
        out,
        "slot,rb_tier_count,rb_tier_prices,rb_tier_delays,rb_tier_capacities,rb_tier_utilisations,eb_tier_count,eb_tier_prices,eb_tier_delays,eb_tier_capacities,eb_tier_utilisations,cumulative_inclusions,cumulative_rb_inclusions,cumulative_eb_inclusions,cumulative_block_inclusions_total,cumulative_block_inclusions_with_delay,cumulative_submitted_bytes,cumulative_included_bytes,cumulative_fees,cumulative_rb_tier_assignments_total,cumulative_rb_tier_assignments_max_priced,cumulative_rb_tier_assignments_by_tier,cumulative_eb_tier_assignments_total,cumulative_eb_tier_assignments_max_priced,cumulative_eb_tier_assignments_by_tier"
    )
    .ok();
    for point in points {
        writeln!(
            out,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            point.slot,
            point.rb_tier_count,
            join_u64(&point.rb_tier_prices),
            join_u64(&point.rb_tier_delays),
            join_u64(&point.rb_tier_capacities),
            join_f64(&point.rb_tier_utilisations),
            point.eb_tier_count,
            join_u64(&point.eb_tier_prices),
            join_u64(&point.eb_tier_delays),
            join_u64(&point.eb_tier_capacities),
            join_f64(&point.eb_tier_utilisations),
            point.cumulative_inclusions,
            point.cumulative_rb_inclusions,
            point.cumulative_eb_inclusions,
            point.cumulative_block_inclusions_total,
            point.cumulative_block_inclusions_with_delay,
            point.cumulative_submitted_bytes,
            point.cumulative_included_bytes,
            point.cumulative_fees,
            point.cumulative_rb_tier_assignments_total,
            point.cumulative_rb_tier_assignments_max_priced,
            join_u64(&point.cumulative_rb_tier_assignments_by_tier),
            point.cumulative_eb_tier_assignments_total,
            point.cumulative_eb_tier_assignments_max_priced,
            join_u64(&point.cumulative_eb_tier_assignments_by_tier)
        )
        .ok();
    }
    out
}

fn format_time_series_html(points: &[TimeSeriesPoint], tier_delay_unit: TierDelayUnit) -> String {
    if points.is_empty() {
        return String::new();
    }

    fn capacity_percentages(
        capacities_by_tier: &[Vec<Option<u64>>],
        slot_count: usize,
    ) -> Vec<Vec<Option<f64>>> {
        let mut lane_totals = vec![0u64; slot_count];
        for series in capacities_by_tier {
            for (slot_index, value) in series.iter().enumerate().take(slot_count) {
                if let Some(capacity) = value {
                    lane_totals[slot_index] = lane_totals[slot_index].saturating_add(*capacity);
                }
            }
        }

        capacities_by_tier
            .iter()
            .map(|series| {
                series
                    .iter()
                    .enumerate()
                    .map(|(slot_index, value)| {
                        let capacity = value.as_ref().copied()?;
                        let total = lane_totals.get(slot_index).copied().unwrap_or(0);
                        if total == 0 {
                            None
                        } else {
                            Some(capacity as f64 * 100.0 / total as f64)
                        }
                    })
                    .collect()
            })
            .collect()
    }

    let max_rb_tiers = points
        .iter()
        .map(|point| {
            point
                .rb_tier_prices
                .len()
                .max(point.rb_tier_capacities.len())
        })
        .max()
        .unwrap_or(0);
    let max_eb_tiers = points
        .iter()
        .map(|point| {
            point
                .eb_tier_prices
                .len()
                .max(point.eb_tier_capacities.len())
        })
        .max()
        .unwrap_or(0);

    let slots: Vec<u64> = points.iter().map(|point| point.slot).collect();
    let rb_tier_counts: Vec<usize> = points.iter().map(|point| point.rb_tier_count).collect();
    let eb_tier_counts: Vec<usize> = points.iter().map(|point| point.eb_tier_count).collect();
    let cumulative_inclusions: Vec<u64> = points
        .iter()
        .map(|point| point.cumulative_inclusions)
        .collect();
    let cumulative_rb_inclusions: Vec<u64> = points
        .iter()
        .map(|point| point.cumulative_rb_inclusions)
        .collect();
    let cumulative_eb_inclusions: Vec<u64> = points
        .iter()
        .map(|point| point.cumulative_eb_inclusions)
        .collect();
    let cumulative_block_inclusions_total: Vec<u64> = points
        .iter()
        .map(|point| point.cumulative_block_inclusions_total)
        .collect();
    let cumulative_block_inclusions_with_delay: Vec<u64> = points
        .iter()
        .map(|point| point.cumulative_block_inclusions_with_delay)
        .collect();
    let cumulative_submitted_bytes: Vec<u64> = points
        .iter()
        .map(|point| point.cumulative_submitted_bytes)
        .collect();
    let cumulative_included_bytes: Vec<u64> = points
        .iter()
        .map(|point| point.cumulative_included_bytes)
        .collect();
    let mut attempted_bytes_delta: Vec<u64> = Vec::with_capacity(points.len());
    let mut included_bytes_delta: Vec<u64> = Vec::with_capacity(points.len());
    let mut rb_inclusions_delta: Vec<u64> = Vec::with_capacity(points.len());
    let mut eb_inclusions_delta: Vec<u64> = Vec::with_capacity(points.len());
    let mut block_inclusions_total_delta: Vec<u64> = Vec::with_capacity(points.len());
    let mut block_inclusions_with_delay_delta: Vec<u64> = Vec::with_capacity(points.len());
    let mut previous_cumulative_attempted_bytes = 0u64;
    let mut previous_cumulative_bytes = 0u64;
    let mut previous_cumulative_rb = 0u64;
    let mut previous_cumulative_eb = 0u64;
    let mut previous_block_inclusions_total = 0u64;
    let mut previous_block_inclusions_with_delay = 0u64;
    for point in points {
        let attempted_delta = point
            .cumulative_submitted_bytes
            .saturating_sub(previous_cumulative_attempted_bytes);
        attempted_bytes_delta.push(attempted_delta);
        previous_cumulative_attempted_bytes = point.cumulative_submitted_bytes;
        let delta = point
            .cumulative_included_bytes
            .saturating_sub(previous_cumulative_bytes);
        included_bytes_delta.push(delta);
        previous_cumulative_bytes = point.cumulative_included_bytes;
        rb_inclusions_delta.push(
            point
                .cumulative_rb_inclusions
                .saturating_sub(previous_cumulative_rb),
        );
        previous_cumulative_rb = point.cumulative_rb_inclusions;
        eb_inclusions_delta.push(
            point
                .cumulative_eb_inclusions
                .saturating_sub(previous_cumulative_eb),
        );
        previous_cumulative_eb = point.cumulative_eb_inclusions;
        block_inclusions_total_delta.push(
            point
                .cumulative_block_inclusions_total
                .saturating_sub(previous_block_inclusions_total),
        );
        previous_block_inclusions_total = point.cumulative_block_inclusions_total;
        block_inclusions_with_delay_delta.push(
            point
                .cumulative_block_inclusions_with_delay
                .saturating_sub(previous_block_inclusions_with_delay),
        );
        previous_block_inclusions_with_delay = point.cumulative_block_inclusions_with_delay;
    }
    let cumulative_fees: Vec<f64> = points
        .iter()
        .map(|point| point.cumulative_fees as f64)
        .collect();

    let mut rb_prices_by_tier: Vec<Vec<Option<u64>>> = vec![Vec::new(); max_rb_tiers];
    let mut rb_delays_by_tier: Vec<Vec<Option<u64>>> = vec![Vec::new(); max_rb_tiers];
    let mut rb_caps_by_tier: Vec<Vec<Option<u64>>> = vec![Vec::new(); max_rb_tiers];
    let mut rb_utils_by_tier: Vec<Vec<Option<f64>>> = vec![Vec::new(); max_rb_tiers];
    let mut eb_prices_by_tier: Vec<Vec<Option<u64>>> = vec![Vec::new(); max_eb_tiers];
    let mut eb_delays_by_tier: Vec<Vec<Option<u64>>> = vec![Vec::new(); max_eb_tiers];
    let mut eb_caps_by_tier: Vec<Vec<Option<u64>>> = vec![Vec::new(); max_eb_tiers];
    let mut eb_utils_by_tier: Vec<Vec<Option<f64>>> = vec![Vec::new(); max_eb_tiers];

    for point in points {
        for tier_index in 0..max_rb_tiers {
            rb_prices_by_tier[tier_index].push(point.rb_tier_prices.get(tier_index).copied());
            rb_delays_by_tier[tier_index].push(point.rb_tier_delays.get(tier_index).copied());
            rb_caps_by_tier[tier_index].push(point.rb_tier_capacities.get(tier_index).copied());
            rb_utils_by_tier[tier_index].push(point.rb_tier_utilisations.get(tier_index).copied());
        }
        for tier_index in 0..max_eb_tiers {
            eb_prices_by_tier[tier_index].push(point.eb_tier_prices.get(tier_index).copied());
            eb_delays_by_tier[tier_index].push(point.eb_tier_delays.get(tier_index).copied());
            eb_caps_by_tier[tier_index].push(point.eb_tier_capacities.get(tier_index).copied());
            eb_utils_by_tier[tier_index].push(point.eb_tier_utilisations.get(tier_index).copied());
        }
    }

    let rb_caps_pct_by_tier = capacity_percentages(&rb_caps_by_tier, slots.len());
    let eb_caps_pct_by_tier = capacity_percentages(&eb_caps_by_tier, slots.len());

    let payload = json!({
        "title": "Tiered Pricing Time Series",
        "delay_unit_label": tier_delay_unit_label(tier_delay_unit),
        "slots": slots,
        "rb_tier_counts": rb_tier_counts,
        "eb_tier_counts": eb_tier_counts,
        "rb_prices_by_tier": rb_prices_by_tier,
        "rb_delays_by_tier": rb_delays_by_tier,
        "rb_caps_by_tier": rb_caps_by_tier,
        "rb_caps_pct_by_tier": rb_caps_pct_by_tier,
        "rb_utils_by_tier": rb_utils_by_tier,
        "eb_prices_by_tier": eb_prices_by_tier,
        "eb_delays_by_tier": eb_delays_by_tier,
        "eb_caps_by_tier": eb_caps_by_tier,
        "eb_caps_pct_by_tier": eb_caps_pct_by_tier,
        "eb_utils_by_tier": eb_utils_by_tier,
        "cumulative_inclusions": cumulative_inclusions,
        "cumulative_rb_inclusions": cumulative_rb_inclusions,
        "cumulative_eb_inclusions": cumulative_eb_inclusions,
        "cumulative_block_inclusions_total": cumulative_block_inclusions_total,
        "cumulative_block_inclusions_with_delay": cumulative_block_inclusions_with_delay,
        "cumulative_submitted_bytes": cumulative_submitted_bytes,
        "attempted_bytes_delta": attempted_bytes_delta,
        "cumulative_included_bytes": cumulative_included_bytes,
        "included_bytes_delta": included_bytes_delta,
        "rb_inclusions_delta": rb_inclusions_delta,
        "eb_inclusions_delta": eb_inclusions_delta,
        "block_inclusions_total_delta": block_inclusions_total_delta,
        "block_inclusions_with_delay_delta": block_inclusions_with_delay_delta,
        "cumulative_fees": cumulative_fees,
        "include_cumulative": true,
    });

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Tiered Pricing Time Series</title>
  <script src="https://cdn.plot.ly/plotly-2.30.0.min.js"></script>
  <style>
    body {{
      font-family: system-ui, -apple-system, Segoe UI, sans-serif;
      margin: 0;
      padding: 16px;
      background: #f7f7f7;
    }}
    .chart {{
      background: #fff;
      border: 1px solid #e0e0e0;
      border-radius: 8px;
      margin: 16px 0;
      padding: 8px;
    }}
    h1 {{
      margin: 0 0 8px 0;
      font-size: 20px;
    }}
  </style>
</head>
<body>
  <h1>Tiered Pricing Time Series</h1>
  <div id="tier-count" class="chart"></div>
  <div id="tier-prices" class="chart"></div>
  <div id="tx-volume" class="chart"></div>
  <div id="tier-delays" class="chart"></div>
  <div id="tier-capacities" class="chart"></div>
  <div id="tier-utils" class="chart"></div>
  <div id="cumulative" class="chart"></div>
  <div id="inclusion-split" class="chart"></div>
  <div id="delay-settlement" class="chart"></div>

  <script>
    const payload = {payload};
    const slots = payload.slots;
    const delayUnitLabel = payload.delay_unit_label;

    function tierTraces(seriesKey, laneLabel, dash) {{
      const traces = [];
      payload[seriesKey].forEach((series, tierIndex) => {{
        traces.push({{
          x: slots,
          y: series,
          mode: "lines",
          name: `${{laneLabel}} Tier ${{tierIndex}}`,
          legendgroup: laneLabel,
          line: {{ width: 2, dash }},
        }});
      }});
      return traces;
    }}

    function movingAverage(values, windowSize) {{
      const safeWindow = Math.max(1, windowSize | 0);
      const output = [];
      const rolling = [];
      let sum = 0;
      values.forEach((value) => {{
        const numeric = Number(value) || 0;
        rolling.push(numeric);
        sum += numeric;
        if (rolling.length > safeWindow) {{
          sum -= rolling.shift();
        }}
        output.push(sum / rolling.length);
      }});
      return output;
    }}

    const volumeWindow = Math.max(3, Math.min(15, Math.round(slots.length / 30)));
    const smoothedAttemptedBytes = movingAverage(payload.attempted_bytes_delta, volumeWindow);
    const smoothedIncludedBytes = movingAverage(payload.included_bytes_delta, volumeWindow);

    Plotly.newPlot("tier-count", [
      {{
        x: slots,
        y: payload.rb_tier_counts,
        mode: "lines",
        name: "RB tier count",
        line: {{ width: 2, color: "rgb(255, 127, 14)" }},
      }},
      {{
        x: slots,
        y: payload.eb_tier_counts,
        mode: "lines",
        name: "EB tier count",
        line: {{ width: 2, color: "rgb(44, 160, 44)" }},
      }}
    ], {{
      title: "Tier Count by Lane",
      xaxis: {{ title: "Slot" }},
      yaxis: {{ title: "Count" }},
      margin: {{ t: 40, r: 20, b: 40, l: 50 }},
    }});

    Plotly.newPlot("tier-prices", [
      ...tierTraces("rb_prices_by_tier", "RB", "solid"),
      ...tierTraces("eb_prices_by_tier", "EB", "dot"),
    ], {{
      title: "Tier Prices (per byte, RB vs EB)",
      xaxis: {{ title: "Slot" }},
      yaxis: {{ title: "Price" }},
      margin: {{ t: 40, r: 20, b: 40, l: 50 }},
    }});

    Plotly.newPlot("inclusion-split", [
      {{
        x: slots,
        y: payload.rb_inclusions_delta,
        type: "bar",
        name: "RB inclusions",
        marker: {{ color: "rgba(255, 127, 14, 0.80)" }},
      }},
      {{
        x: slots,
        y: payload.eb_inclusions_delta,
        type: "bar",
        name: "EB inclusions",
        marker: {{ color: "rgba(44, 160, 44, 0.80)" }},
      }},
    ], {{
      title: "Transaction Inclusions per Update (RB vs EB)",
      barmode: "stack",
      xaxis: {{ title: "Slot" }},
      yaxis: {{ title: "Inclusions" }},
      margin: {{ t: 40, r: 20, b: 40, l: 50 }},
    }});

    Plotly.newPlot("delay-settlement", [
      {{
        x: slots,
        y: payload.block_inclusions_with_delay_delta,
        type: "bar",
        name: "Inclusions with quoted delay > 1",
        marker: {{ color: "rgba(255, 127, 14, 0.70)" }},
      }},
      {{
        x: slots,
        y: payload.block_inclusions_total_delta,
        mode: "lines",
        name: "All inclusions",
        line: {{ width: 1.5, color: "rgb(255, 127, 14)", dash: "dot" }},
        visible: "legendonly",
      }},
    ], {{
      title: "Delayed-Tier Inclusions",
      barmode: "group",
      xaxis: {{ title: "Slot" }},
      yaxis: {{ title: "Inclusions per update", rangemode: "tozero" }},
      margin: {{ t: 40, r: 20, b: 40, l: 50 }},
    }});

    Plotly.newPlot("tx-volume", [
      {{
        x: slots,
        y: payload.attempted_bytes_delta,
        type: "bar",
        name: "Attempted bytes per update",
        marker: {{ color: "rgba(214, 39, 40, 0.20)" }},
      }},
      {{
        x: slots,
        y: payload.included_bytes_delta,
        type: "bar",
        name: "Included bytes per update",
        marker: {{ color: "rgba(31, 119, 180, 0.25)" }},
      }},
      {{
        x: slots,
        y: smoothedAttemptedBytes,
        mode: "lines",
        name: `Attempted moving average (${{volumeWindow}} points)`,
        line: {{ width: 3, color: "rgb(214, 39, 40)" }},
      }},
      {{
        x: slots,
        y: smoothedIncludedBytes,
        mode: "lines",
        name: `Included moving average (${{volumeWindow}} points)`,
        line: {{ width: 3, color: "rgb(31, 119, 180)" }},
      }},
    ], {{
      title: "Attempted vs Included Transaction Volume (bytes)",
      barmode: "overlay",
      xaxis: {{ title: "Slot" }},
      yaxis: {{ title: "Bytes" }},
      margin: {{ t: 40, r: 20, b: 40, l: 50 }},
    }});

    Plotly.newPlot("tier-delays", [
      ...tierTraces("rb_delays_by_tier", "RB", "solid"),
      ...tierTraces("eb_delays_by_tier", "EB", "dot"),
    ], {{
      title: `Tier Delays (${{delayUnitLabel}}, RB vs EB)`,
      xaxis: {{ title: "Slot" }},
      yaxis: {{ title: `Delay (${{delayUnitLabel}})` }},
      margin: {{ t: 40, r: 20, b: 40, l: 50 }},
    }});

    Plotly.newPlot("tier-capacities", [
      ...tierTraces("rb_caps_pct_by_tier", "RB", "solid"),
      ...tierTraces("eb_caps_pct_by_tier", "EB", "dot"),
    ], {{
      title: "Tier Capacity Share (% of lane capacity, RB vs EB)",
      xaxis: {{ title: "Slot" }},
      yaxis: {{ title: "Capacity share (%)", range: [0, 100] }},
      margin: {{ t: 40, r: 20, b: 40, l: 50 }},
    }});

    Plotly.newPlot("tier-utils", [
      ...tierTraces("rb_utils_by_tier", "RB", "solid"),
      ...tierTraces("eb_utils_by_tier", "EB", "dot"),
    ], {{
      title: "Tier Utilisation (RB vs EB)",
      xaxis: {{ title: "Slot" }},
      yaxis: {{ title: "Utilisation" }},
      margin: {{ t: 40, r: 20, b: 40, l: 50 }},
    }});

    if (payload.include_cumulative) {{
      Plotly.newPlot("cumulative", [
        {{
          x: slots,
          y: payload.cumulative_inclusions,
          mode: "lines",
          name: "Cumulative inclusions (total)",
          line: {{ width: 2, color: "rgb(31, 119, 180)" }},
        }},
        {{
          x: slots,
          y: payload.cumulative_rb_inclusions,
          mode: "lines",
          name: "Cumulative RB inclusions",
          line: {{ width: 2, color: "rgb(255, 127, 14)" }},
        }},
        {{
          x: slots,
          y: payload.cumulative_eb_inclusions,
          mode: "lines",
          name: "Cumulative EB inclusions",
          line: {{ width: 2, color: "rgb(44, 160, 44)" }},
        }},
        {{
          x: slots,
          y: payload.cumulative_submitted_bytes,
          mode: "lines",
          name: "Cumulative attempted bytes",
          line: {{ width: 2, color: "rgb(214, 39, 40)" }},
        }},
        {{
          x: slots,
          y: payload.cumulative_included_bytes,
          mode: "lines",
          name: "Cumulative included bytes",
        }},
        {{
          x: slots,
          y: payload.cumulative_fees,
          mode: "lines",
          name: "Cumulative fees",
        }},
      ], {{
        title: "Cumulative Inclusions / Bytes / Fees",
        xaxis: {{ title: "Slot" }},
        yaxis: {{ title: "Value" }},
        margin: {{ t: 40, r: 20, b: 40, l: 50 }},
      }});
    }}
  </script>
</body>
</html>
"#
    )
}

struct DiagnosticsLogger {
    writer: Option<BufWriter<File>>,
    interval_slots: u64,
    last_logged_slot: Option<u64>,
    start_wall: Instant,
}

impl DiagnosticsLogger {
    async fn new(
        output_path: &Option<PathBuf>,
        output_dir: &PathBuf,
        start_wall: Instant,
    ) -> Result<Self> {
        if output_path.is_none() {
            return Ok(Self {
                writer: None,
                interval_slots: 10,
                last_logged_slot: None,
                start_wall,
            });
        }

        fs::create_dir_all(output_dir).await?;
        let file = File::create(output_dir.join("diagnostics.log")).await?;
        let mut writer = BufWriter::new(file);
        writer
            .write_all(
                b"kind,wall_time_s,slot,event_time_s,event_count,tasks_in_flight,actors_running,actors_total,running_actor_ids,running_actor_names,last_task_started_by,last_task_started_name,last_task_finished_by,last_task_finished_name,last_wait_actor,last_wait_until_s,last_woken_actor,last_advance_to_s,wait_queue_len,last_node_handler_started_by,last_node_handler_started_name,last_node_handler_started_kind,last_node_handler_finished_by,last_node_handler_finished_name,last_node_handler_finished_kind,tx_producer_slot,tx_producer_phase,tx_generated,tx_pending,submissions,rejected,included,rb_generated,eb_generated,rb_tier_count,rb_tier_prices,eb_tier_count,eb_tier_prices\n",
            )
            .await?;

        Ok(Self {
            writer: Some(writer),
            interval_slots: 10,
            last_logged_slot: None,
            start_wall,
        })
    }

    async fn log_slot(
        &mut self,
        slot: u64,
        event_time: Timestamp,
        event_count: u64,
        tasks_in_flight: u64,
        actors_running: u64,
        actors_total: u64,
        running_actor_ids: &[u64],
        running_actor_names: &str,
        last_task_started_by: Option<u64>,
        last_task_started_name: &str,
        last_task_finished_by: Option<u64>,
        last_task_finished_name: &str,
        last_wait_actor: &str,
        last_wait_until_s: &str,
        last_woken_actor: &str,
        last_advance_to_s: &str,
        wait_queue_len: u64,
        last_node_handler_started_by: &str,
        last_node_handler_started_name: &str,
        last_node_handler_started_kind: &str,
        last_node_handler_finished_by: &str,
        last_node_handler_finished_name: &str,
        last_node_handler_finished_kind: &str,
        tx_producer_slot: Option<u64>,
        tx_producer_phase: &str,
        txs: &BTreeMap<TransactionId, Transaction>,
        pricing: &PricingMetrics,
        rb_generated: u64,
        eb_generated: u64,
        last_rb_tier_count: usize,
        last_rb_tier_prices: &[u64],
        last_eb_tier_count: usize,
        last_eb_tier_prices: &[u64],
    ) -> Result<()> {
        if slot % self.interval_slots != 0 {
            return Ok(());
        }
        if self.last_logged_slot == Some(slot) {
            return Ok(());
        }
        self.last_logged_slot = Some(slot);
        self.write_line(
            "slot",
            slot,
            event_time,
            event_count,
            tasks_in_flight,
            actors_running,
            actors_total,
            running_actor_ids,
            running_actor_names,
            last_task_started_by,
            last_task_started_name,
            last_task_finished_by,
            last_task_finished_name,
            last_wait_actor,
            last_wait_until_s,
            last_woken_actor,
            last_advance_to_s,
            wait_queue_len,
            last_node_handler_started_by,
            last_node_handler_started_name,
            last_node_handler_started_kind,
            last_node_handler_finished_by,
            last_node_handler_finished_name,
            last_node_handler_finished_kind,
            tx_producer_slot,
            tx_producer_phase,
            txs,
            pricing,
            rb_generated,
            eb_generated,
            last_rb_tier_count,
            last_rb_tier_prices,
            last_eb_tier_count,
            last_eb_tier_prices,
        )
        .await
    }

    async fn log_heartbeat(
        &mut self,
        slot: u64,
        event_time: Timestamp,
        event_count: u64,
        tasks_in_flight: u64,
        actors_running: u64,
        actors_total: u64,
        running_actor_ids: &[u64],
        running_actor_names: &str,
        last_task_started_by: Option<u64>,
        last_task_started_name: &str,
        last_task_finished_by: Option<u64>,
        last_task_finished_name: &str,
        last_wait_actor: &str,
        last_wait_until_s: &str,
        last_woken_actor: &str,
        last_advance_to_s: &str,
        wait_queue_len: u64,
        last_node_handler_started_by: &str,
        last_node_handler_started_name: &str,
        last_node_handler_started_kind: &str,
        last_node_handler_finished_by: &str,
        last_node_handler_finished_name: &str,
        last_node_handler_finished_kind: &str,
        tx_producer_slot: Option<u64>,
        tx_producer_phase: &str,
        txs: &BTreeMap<TransactionId, Transaction>,
        pricing: &PricingMetrics,
        rb_generated: u64,
        eb_generated: u64,
        last_rb_tier_count: usize,
        last_rb_tier_prices: &[u64],
        last_eb_tier_count: usize,
        last_eb_tier_prices: &[u64],
    ) -> Result<()> {
        self.write_line(
            "heartbeat",
            slot,
            event_time,
            event_count,
            tasks_in_flight,
            actors_running,
            actors_total,
            running_actor_ids,
            running_actor_names,
            last_task_started_by,
            last_task_started_name,
            last_task_finished_by,
            last_task_finished_name,
            last_wait_actor,
            last_wait_until_s,
            last_woken_actor,
            last_advance_to_s,
            wait_queue_len,
            last_node_handler_started_by,
            last_node_handler_started_name,
            last_node_handler_started_kind,
            last_node_handler_finished_by,
            last_node_handler_finished_name,
            last_node_handler_finished_kind,
            tx_producer_slot,
            tx_producer_phase,
            txs,
            pricing,
            rb_generated,
            eb_generated,
            last_rb_tier_count,
            last_rb_tier_prices,
            last_eb_tier_count,
            last_eb_tier_prices,
        )
        .await
    }

    async fn write_line(
        &mut self,
        kind: &str,
        slot: u64,
        event_time: Timestamp,
        event_count: u64,
        tasks_in_flight: u64,
        actors_running: u64,
        actors_total: u64,
        running_actor_ids: &[u64],
        running_actor_names: &str,
        last_task_started_by: Option<u64>,
        last_task_started_name: &str,
        last_task_finished_by: Option<u64>,
        last_task_finished_name: &str,
        last_wait_actor: &str,
        last_wait_until_s: &str,
        last_woken_actor: &str,
        last_advance_to_s: &str,
        wait_queue_len: u64,
        last_node_handler_started_by: &str,
        last_node_handler_started_name: &str,
        last_node_handler_started_kind: &str,
        last_node_handler_finished_by: &str,
        last_node_handler_finished_name: &str,
        last_node_handler_finished_kind: &str,
        tx_producer_slot: Option<u64>,
        tx_producer_phase: &str,
        txs: &BTreeMap<TransactionId, Transaction>,
        pricing: &PricingMetrics,
        rb_generated: u64,
        eb_generated: u64,
        last_rb_tier_count: usize,
        last_rb_tier_prices: &[u64],
        last_eb_tier_count: usize,
        last_eb_tier_prices: &[u64],
    ) -> Result<()> {
        let Some(writer) = &mut self.writer else {
            return Ok(());
        };

        let wall_time_s = self.start_wall.elapsed().as_secs_f64();
        let event_time_s = (event_time - Timestamp::zero()).as_secs_f64();
        let pending = count_pending_txs(txs);
        let rb_price_list = last_rb_tier_prices
            .iter()
            .map(|price| price.to_string())
            .collect::<Vec<_>>()
            .join(";");
        let eb_price_list = last_eb_tier_prices
            .iter()
            .map(|price| price.to_string())
            .collect::<Vec<_>>()
            .join(";");
        let running_ids = join_u64(running_actor_ids);
        let started_id = last_task_started_by
            .map(|id| id.to_string())
            .unwrap_or_default();
        let finished_id = last_task_finished_by
            .map(|id| id.to_string())
            .unwrap_or_default();
        let txp_slot = tx_producer_slot
            .map(|id| id.to_string())
            .unwrap_or_default();

        writer
            .write_all(
                format!(
                    "{kind},{wall_time_s:.3},{slot},{event_time_s:.6},{event_count},{tasks_in_flight},{actors_running},{actors_total},{running_ids},{running_actor_names},{started_id},{last_task_started_name},{finished_id},{last_task_finished_name},{last_wait_actor},{last_wait_until_s},{last_woken_actor},{last_advance_to_s},{wait_queue_len},{last_node_handler_started_by},{last_node_handler_started_name},{last_node_handler_started_kind},{last_node_handler_finished_by},{last_node_handler_finished_name},{last_node_handler_finished_kind},{txp_slot},{tx_producer_phase},{},{},{},{},{},{},{},{},{},{},{}\n",
                    txs.len(),
                    pending,
                    pricing.submissions,
                    pricing.rejected,
                    pricing.included,
                    rb_generated,
                    eb_generated,
                    last_rb_tier_count,
                    rb_price_list,
                    last_eb_tier_count,
                    eb_price_list
                )
                .as_bytes(),
            )
            .await?;
        writer.flush().await?;
        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        if let Some(writer) = &mut self.writer {
            writer.flush().await?;
        }
        Ok(())
    }
}

fn count_pending_txs(txs: &BTreeMap<TransactionId, Transaction>) -> u64 {
    txs.values().filter(|tx| tx.tx_type.is_none()).count() as u64
}

fn join_u64(values: &[u64]) -> String {
    values
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(";")
}

fn join_indexed_counts(values: &[u64]) -> String {
    let pairs = values
        .iter()
        .enumerate()
        .filter(|(_, count)| **count > 0)
        .map(|(index, count)| format!("{index}:{count}"))
        .collect::<Vec<_>>();
    if pairs.is_empty() {
        "-".to_string()
    } else {
        pairs.join(";")
    }
}

fn join_labeled_counts(values: &BTreeMap<String, u64>) -> String {
    let pairs = values
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(label, count)| format!("{label}:{count}"))
        .collect::<Vec<_>>();
    if pairs.is_empty() {
        "-".to_string()
    } else {
        pairs.join(";")
    }
}

fn ratio_pct(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 * 100.0 / denominator as f64
    }
}

fn tier_id_to_index(tier: TierId) -> Option<usize> {
    tier.to_string().parse().ok()
}

fn ensure_count_capacity(counts: &mut Vec<u64>, index: usize) {
    if counts.len() <= index {
        counts.resize(index + 1, 0);
    }
}

fn merge_cumulative_counts(existing: &mut Vec<u64>, incoming: &[u64]) {
    if existing.len() < incoming.len() {
        existing.resize(incoming.len(), 0);
    }
    for (index, value) in incoming.iter().enumerate() {
        existing[index] = existing[index].max(*value);
    }
}

fn format_actor_names(ids: &[u64], registry: &BTreeMap<u64, String>) -> String {
    ids.iter()
        .map(|id| {
            registry
                .get(id)
                .cloned()
                .unwrap_or_else(|| format!("actor-{id}"))
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn format_actor_name(id: Option<u64>, registry: &BTreeMap<u64, String>) -> String {
    match id {
        Some(id) => registry
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("actor-{id}")),
        None => String::new(),
    }
}

fn join_f64(values: &[f64]) -> String {
    values
        .iter()
        .map(|v| format!("{:.4}", v))
        .collect::<Vec<_>>()
        .join(";")
}

#[derive(Clone, Copy)]
enum TransactionType {
    Leios,
    Praos,
}

struct InputBlock {
    bytes: u64,
    generated: Timestamp,
    txs: Vec<TransactionId>,
    included_in_eb: Option<Timestamp>,
    included_in_block: Option<Timestamp>,
}
impl InputBlock {
    fn new(bytes: u64, generated: Timestamp, txs: Vec<TransactionId>) -> Self {
        Self {
            bytes,
            generated,
            txs,
            included_in_eb: None,
            included_in_block: None,
        }
    }
    fn is_empty(&self) -> bool {
        self.txs.is_empty()
    }
}
struct EndorserBlock {
    generated: Timestamp,
    txs: Vec<TransactionId>,
    ibs: Vec<InputBlockId>,
    ebs: Vec<EndorserBlockId>,
    included_in_eb: Option<Timestamp>,
    included_in_block: Option<Timestamp>,
}
impl EndorserBlock {
    fn new(
        generated: Timestamp,
        txs: Vec<TransactionId>,
        ibs: Vec<InputBlockId>,
        ebs: Vec<EndorserBlockId>,
    ) -> Self {
        Self {
            generated,
            txs,
            ibs,
            ebs,
            included_in_eb: None,
            included_in_block: None,
        }
    }
    fn is_empty(&self) -> bool {
        self.txs.is_empty() && self.ibs.is_empty() && self.ebs.is_empty()
    }
}

#[derive(Default)]
struct MessageStats {
    sent: u64,
    received: u64,
}
impl MessageStats {
    fn display(&self, name: &str) {
        let percent_received = self.received as f64 / self.sent as f64 * 100.0;
        info!(
            "{} {} message(s) were sent. {} of them were received ({:.3}%).",
            self.sent, name, self.received, percent_received
        );
    }
}

struct Stats {
    mean: f64,
    std_dev: f64,
}

fn compute_stats<Iter: IntoIterator<Item = f64>>(data: Iter) -> Stats {
    let v: Variance = data.into_iter().collect();
    Stats {
        mean: v.mean(),
        std_dev: v.population_variance().sqrt(),
    }
}

#[allow(clippy::large_enum_variant)]
enum OutputTarget {
    AggregatedEventStream {
        aggregation: TraceAggregator,
        format: OutputFormat,
        file: TraceSink,
    },
    EventStream {
        format: OutputFormat,
        file: TraceSink,
    },
    None,
}

impl OutputTarget {
    async fn write(&mut self, event: OutputEvent) -> Result<()> {
        match self {
            Self::AggregatedEventStream {
                aggregation,
                format,
                file,
            } => {
                if let Some(summary) = aggregation.process(event) {
                    Self::write_line(*format, file, summary).await?;
                }
            }
            Self::EventStream { format, file } => {
                Self::write_line(*format, file, event).await?;
            }
            Self::None => {}
        }
        Ok(())
    }

    async fn write_line<T: Serialize, W: AsyncWrite + Unpin>(
        format: OutputFormat,
        file: &mut W,
        event: T,
    ) -> Result<()> {
        match format {
            OutputFormat::JsonStream => {
                let mut string = serde_json::to_string(&event)?;
                string.push('\n');
                file.write_all(string.as_bytes()).await?;
            }
            OutputFormat::CborStream => {
                let bytes = minicbor_serde::to_vec(&event)?;
                file.write_all(&bytes).await?;
            }
        }
        Ok(())
    }

    async fn flush(self) -> Result<()> {
        match self {
            Self::AggregatedEventStream {
                aggregation,
                format,
                mut file,
            } => {
                if let Some(summary) = aggregation.finish() {
                    Self::write_line(format, &mut file, summary).await?;
                }
                file.shutdown().await?;
            }
            Self::EventStream { mut file, .. } => {
                file.shutdown().await?;
            }
            Self::None => {}
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        PricingMetrics, TimeSeriesPoint, format_metrics_table, format_time_series_html,
        settlement_value_stats,
    };
    use sim_core::{
        config::TierDelayUnit,
        model::{ActorId, UrgencyProfile},
    };

    #[test]
    fn settlement_value_stats_uses_actual_latency_and_fee() {
        let urgency = UrgencyProfile::LinearDecay {
            value_drop_per_slot: 100,
        };

        let stats = settlement_value_stats(1_000, &urgency, 3, Some(250));

        assert_eq!(stats.retained_value, 800);
        assert_eq!(stats.net_utility, 550);
        assert_eq!(stats.retention_ratio, Some(0.8));
    }

    #[test]
    fn pricing_metrics_accumulate_welfare_per_actor() {
        let actor = ActorId::new(7);
        let urgency = UrgencyProfile::LinearDecay {
            value_drop_per_slot: 50,
        };
        let mut metrics = PricingMetrics::default();

        metrics.record_generated(actor, 1_000, None);
        metrics.record_inclusion(actor, Some(100), 400, 4, 2, 1_000, &urgency, None);

        assert_eq!(metrics.unique_generated, 1);
        assert_eq!(metrics.included, 1);
        assert_eq!(metrics.retained_value_total, 850);
        assert_eq!(metrics.net_utility_total, 750);

        let actor_metrics = metrics.per_actor.get(&actor).expect("actor metrics");
        assert_eq!(actor_metrics.unique_generated, 1);
        assert_eq!(actor_metrics.retained_value_total, 850);
        assert_eq!(actor_metrics.net_utility_total, 750);
    }

    #[test]
    fn reporting_surfaces_block_delay_unit() {
        let metrics = PricingMetrics::default();
        let text = format_metrics_table(&metrics, &BTreeMap::new(), &BTreeMap::new(), 42, TierDelayUnit::Blocks);
        assert!(text.contains("Synthetic delay unit       | blocks"));

        let html = format_time_series_html(
            &[TimeSeriesPoint {
                slot: 0,
                rb_tier_count: 1,
                rb_tier_prices: vec![10],
                rb_tier_delays: vec![1],
                rb_tier_capacities: vec![100],
                rb_tier_utilisations: vec![0.5],
                eb_tier_count: 1,
                eb_tier_prices: vec![5],
                eb_tier_delays: vec![2],
                eb_tier_capacities: vec![200],
                eb_tier_utilisations: vec![0.4],
                cumulative_inclusions: 0,
                cumulative_rb_inclusions: 0,
                cumulative_eb_inclusions: 0,
                cumulative_block_inclusions_total: 0,
                cumulative_block_inclusions_with_delay: 0,
                cumulative_submitted_bytes: 0,
                cumulative_included_bytes: 0,
                cumulative_fees: 0,
                cumulative_rb_tier_assignments_total: 0,
                cumulative_rb_tier_assignments_max_priced: 0,
                cumulative_rb_tier_assignments_by_tier: vec![0],
                cumulative_eb_tier_assignments_total: 0,
                cumulative_eb_tier_assignments_max_priced: 0,
                cumulative_eb_tier_assignments_by_tier: vec![0],
            }],
            TierDelayUnit::Blocks,
        );
        assert!(html.contains("Tier Delays (${delayUnitLabel}, RB vs EB)"));
        assert!(html.contains("\"delay_unit_label\":\"blocks\""));
    }
}
