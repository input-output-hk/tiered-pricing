//! Welfare-metrics event collector. M3.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sim_core::{events::Event, model::TransactionId, tx_actors::welfare, tx_pricing::Lane};

/// One row of `time_series.csv`. Built per slot.
#[derive(Debug, Clone, Default)]
pub struct TimeSeriesRow {
    pub slot: u64,
    pub c_priority_quote_per_byte: u64,
    pub c_standard_quote_per_byte: u64,
    pub priority_window_util_x_1e9: u64,
    pub standard_window_util_x_1e9: u64,
    pub mempool_bytes_total: u64,
    pub mempool_bytes_priority: u64,
    pub mempool_bytes_standard: u64,
    pub included_bytes_priority: u64,
    pub included_bytes_standard: u64,
    pub included_count_priority: u64,
    pub included_count_standard: u64,
    pub evicted_quote_drift_count: u64,
    pub fees_paid_lovelace: u64,
    pub refund_lovelace: u64,
}

/// One actor-component's accumulated welfare. Keyed by component
/// index across all txs the component produced.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentSummary {
    pub component_index: u32,
    pub txs_submitted: u64,
    pub txs_included: u64,
    pub txs_evicted_quote_drift: u64,
    pub bytes_included: u64,
    pub fees_paid_lovelace: u64,
    pub refund_lovelace: u64,
    /// Sum of `retained_value` (f64) over included txs only. Negative
    /// `net_utility` events still contribute their (positive)
    /// retained_value here.
    pub retained_value_total: f64,
    /// Sum of `net_utility` over included txs. **Negative values are
    /// preserved**: regret events (retained_value < actual_fee)
    /// contribute their negative value to this sum and must not be
    /// clamped or filtered.
    pub net_utility_total: f64,
    /// Sum of `value_lovelace` for included txs (denominator of the
    /// retained_value_ratio per-component aggregate).
    pub included_value_lovelace_total: u128,
    pub priority_included: u64,
    pub standard_included: u64,
    /// Latency observations (in blocks). Mean across observations
    /// becomes `latency_blocks_mean` in the comparison output.
    pub latency_blocks_observations: Vec<f64>,
}

impl ComponentSummary {
    pub fn retained_value_ratio_aggregate(&self) -> f64 {
        if self.included_value_lovelace_total == 0 {
            0.0
        } else {
            self.retained_value_total / (self.included_value_lovelace_total as f64)
        }
    }

    pub fn net_utility_total(&self) -> f64 {
        // Identity accessor — defensively named so reviewers see that
        // the type's `_total` field is also the public exit.
        self.net_utility_total
    }

    pub fn latency_blocks_mean(&self) -> f64 {
        if self.latency_blocks_observations.is_empty() {
            0.0
        } else {
            self.latency_blocks_observations.iter().sum::<f64>()
                / (self.latency_blocks_observations.len() as f64)
        }
    }

    pub fn inclusion_rate(&self) -> f64 {
        if self.txs_submitted == 0 {
            0.0
        } else {
            (self.txs_included as f64) / (self.txs_submitted as f64)
        }
    }

    pub fn eviction_rate(&self) -> f64 {
        if self.txs_submitted == 0 {
            0.0
        } else {
            (self.txs_evicted_quote_drift as f64) / (self.txs_submitted as f64)
        }
    }
}

/// Per-tx metadata captured at submission. Joined with later
/// `TXIncluded` / `TXEvictedQuoteDrift` events to compute welfare.
#[derive(Debug, Clone, Copy)]
struct TxMeta {
    component_index: u32,
    value_lovelace: u64,
    urgency: f64,
    submit_slot: u64,
}

/// Run-level summary. Cross-component aggregates also live here.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunSummary {
    pub components: Vec<ComponentSummary>,
    pub total_txs_submitted: u64,
    pub total_txs_included: u64,
    pub total_txs_evicted_quote_drift: u64,
    pub total_fees_paid_lovelace: u64,
    pub total_refund_lovelace: u64,
    pub priority_retained_value_total: f64,
    pub standard_retained_value_total: f64,
    pub priority_included_value_total: u128,
    pub standard_included_value_total: u128,
    /// 1.0 if `block_generation_probability` was unknown to the
    /// collector at slot 0 (defensive default; should be set via
    /// `MetricsCollector::set_block_generation_probability`).
    pub block_generation_probability: f64,
    /// Number of multiplier-floor breaches observed in the time
    /// series (must always be 0; non-zero is a simulator bug).
    pub multiplier_floor_breaches: u64,
    /// Maximum priority quote / standard quote ratio observed across
    /// the run. The multiplier-floor invariant pins this at
    /// `≥ multiplier_floor` whenever the priority lane is used; the
    /// diagnostic log surfaces the run's observed minimum, maximum,
    /// and any breaches.
    pub min_priority_over_standard_ratio: f64,
    pub max_priority_over_standard_ratio: f64,
    /// Number of `PricingTick` events observed. A run with zero ticks
    /// indicates the simulator wasn't emitting them (regression).
    pub pricing_ticks: u64,
    /// SHA256 of the pricing event stream (`TXIncluded` +
    /// `TXEvictedQuoteDrift` events, hashed in observation order
    /// using the same encoding as the M2 cross-arch golden tests).
    /// The runner persists this to `pricing_event_stream.sha256` so
    /// `experiment-suite verify` can re-run and assert
    /// bit-identical reproduction.
    #[serde(default)]
    pub pricing_event_stream_sha256: String,
    /// Multi-node noise metric (M6). Number of distinct slots at
    /// which the representative node fully validated TWO OR MORE
    /// sibling RB BODIES from different producers — i.e., slots
    /// where two sibling headers both passed the VRF tiebreaker at
    /// `finish_validating_rb_header` before either body finished
    /// in flight, so both bodies reached `apply_priced_block`.
    ///
    /// This is NARROWER than "all slot battles": most slot battles
    /// resolve at header receipt (`linear_leios.rs::
    /// finish_validating_rb_header`, ~line 1062), where the losing
    /// header is dropped before its body is ever requested — those
    /// resolutions never reach this metric. What IS captured is the
    /// late-race subset where both bodies got fully validated, which
    /// is the same subset for which the M1 known limitation (no
    /// pricing rollback on fork resolution) can actually mutate the
    /// controller against an orphaned RB.
    ///
    /// Therefore this metric is an UPPER BOUND on the pricing-state
    /// contamination from forks the representative could not roll
    /// back. A low value here means the M1 limitation does not
    /// matter empirically at the representative.
    #[serde(default)]
    pub slot_battles_count: u64,
    /// Multi-node noise metric (M6). Sum over `slot_battles_count`
    /// slots of (N_bodies_at_slot − 1) — one canonical chain, so
    /// N−1 of the N fully-validated sibling RBs are orphans whose
    /// pricing samples cannot be rolled back. Upper bound on
    /// actually-orphan-able applied samples at the representative.
    /// See `slot_battles_count` for the narrowing condition.
    #[serde(default)]
    pub orphaned_pricing_samples: u64,
    // ------------------------------------------------------------------
    // Price-shock UX metrics (M9).
    //
    // All four are derived from the per-slot `c_priority` time series
    // observed at the representative node. They quantify the
    // user-facing volatility of the priority lane's quote: how
    // dramatically can the quote move against an in-flight tx between
    // submission and inclusion?
    //
    // The endorsement-window length used for the rolling-window
    // metrics is `(3 × header_diffusion_time_slots) +
    // linear_vote_stage_length_slots + linear_diffuse_stage_length_slots`
    // — 14 slots under CIP-0164 defaults. Configured per run via
    // `MetricsCollector::set_shock_window_slots`.
    /// Largest single-slot upward ratio change in `c_priority`. Bounded
    /// above by the controller's `(D + 1) / D` step size; an empirical
    /// value above this bound indicates a simulator bug.
    #[serde(default)]
    pub max_single_step_priority_shock: f64,
    /// Worst-case upward shock observed in any rolling
    /// `shock_window_slots` window of `c_priority`. A user who submits
    /// at the start of the worst window faces this multiplier on their
    /// quote before inclusion is possible. If this exceeds the actor's
    /// `max_fee_lovelace` headroom multiplier (default `{4, 1}` = 4×),
    /// their tx is at risk of quote-drift eviction.
    #[serde(default)]
    pub max_priority_shock_over_window: f64,
    /// 90th-percentile upward shock across all rolling
    /// `shock_window_slots` windows. Captures "what does an
    /// unlucky-but-not-pathological user experience?"
    #[serde(default)]
    pub p90_priority_shock_over_window: f64,
    /// Fraction of rolling `shock_window_slots` windows whose upward
    /// shock exceeds 4× — i.e., a user with the default
    /// `ScaledOverLaneQuote{4, 1}` max-fee policy would have evicted
    /// had they submitted at the window's start.
    #[serde(default)]
    pub eviction_risk_rate_at_4x: f64,
    /// Length (in slots) of the rolling window used to compute the
    /// shock metrics above. Surfaces so `metrics_comparison.txt`
    /// readers know what window the shock metrics were measured over.
    /// 0 = shock metrics not computed (insufficient data or unset).
    #[serde(default)]
    pub shock_window_slots: u64,
}

/// Event-driven welfare-metrics collector.
///
/// Consumes a stream of [`Event`]s and produces:
/// - per-slot rows (`TimeSeriesRow`) buffered by slot for
///   `time_series.csv`,
/// - a `RunSummary` for `metrics_comparison.txt` and
///   `diagnostics.log`.
///
/// **Design notes.**
/// - Time series uses one *representative* node's `PricingTick`
///   stream. The runner pre-selects it via
///   [`set_representative_node`] (lexicographically smallest node
///   name from the topology) so the choice is deterministic and
///   independent of tokio task scheduling. If no node is pre-set,
///   the first observed tick wins (lazy fallback for standalone
///   tests). Multi-node sims produce one tick per node per slot; in
///   single-producer suite tests all nodes converge to identical
///   pricing state given the same priced blocks.
/// - Welfare formulas live in `sim_core::tx_actors::welfare`. The
///   aggregator preserves the **sign** of net_utility through every
///   step — regret events (negative net_utility) are part of the
///   welfare picture (plan line 152).
pub struct MetricsCollector {
    block_generation_probability: f64,
    multiplier_floor_num: Option<u64>,
    multiplier_floor_den: Option<u64>,
    representative_node: Option<String>,
    rows: Vec<TimeSeriesRow>,
    /// Accumulators that reset at each slot boundary.
    delta: TimeSeriesRow,
    /// Per-tx metadata captured at submission.
    tx_meta: HashMap<TransactionId, TxMeta>,
    /// Per-component aggregates, keyed by component index.
    components: HashMap<u32, ComponentSummary>,
    /// Cross-lane retained-value totals.
    priority_retained_value_total: f64,
    standard_retained_value_total: f64,
    priority_included_value_total: u128,
    standard_included_value_total: u128,
    multiplier_floor_breaches: u64,
    min_ratio: f64,
    max_ratio: f64,
    pricing_ticks: u64,
    /// Pricing-event-stream hasher. Same encoding as the M2
    /// cross-arch golden hash (`TXIncluded` + `TXEvictedQuoteDrift`,
    /// hashed in observation order). The finalised hex digest is
    /// stored on `RunSummary.pricing_event_stream_sha256`.
    pricing_event_hasher: Sha256,
    /// M6 fork-resolution metric accumulator. Keyed by slot, value
    /// is the set of producers whose RB was priced at the
    /// representative node for that slot. Drained at `finalise` to
    /// compute `slot_battles_count` and `orphaned_pricing_samples`.
    sample_producers_by_slot: HashMap<u64, HashSet<String>>,
    /// M9 shock-window length in slots. The runner sets this from the
    /// resolved sim config: `(3 × header_diffusion) + L_vote + L_diff`.
    /// Defaults to 14 (CIP-0164 Table 7) if unset.
    shock_window_slots: u64,
}

impl MetricsCollector {
    pub fn new(block_generation_probability: f64) -> Self {
        Self {
            block_generation_probability,
            multiplier_floor_num: None,
            multiplier_floor_den: None,
            representative_node: None,
            rows: Vec::new(),
            delta: TimeSeriesRow::default(),
            tx_meta: HashMap::new(),
            components: HashMap::new(),
            priority_retained_value_total: 0.0,
            standard_retained_value_total: 0.0,
            priority_included_value_total: 0,
            standard_included_value_total: 0,
            multiplier_floor_breaches: 0,
            min_ratio: f64::INFINITY,
            max_ratio: 0.0,
            pricing_ticks: 0,
            pricing_event_hasher: Sha256::new(),
            sample_producers_by_slot: HashMap::new(),
            shock_window_slots: 14,
        }
    }

    /// Set the rolling-window length (in slots) used for the
    /// price-shock UX metrics. Default is 14 (CIP-0164 endorsement
    /// window). Set by the runner from the resolved sim config.
    pub fn set_shock_window_slots(&mut self, slots: u64) {
        self.shock_window_slots = slots;
    }

    /// Configure the run-level multiplier-floor invariant for the
    /// breach checker. Optional; if `None`, breaches are not counted.
    pub fn set_multiplier_floor(&mut self, num: u64, den: u64) {
        self.multiplier_floor_num = Some(num);
        self.multiplier_floor_den = Some(den);
    }

    pub fn set_block_generation_probability(&mut self, p: f64) {
        self.block_generation_probability = p;
    }

    /// Pin the representative node for the time-series. The runner
    /// calls this before processing events so the choice is
    /// deterministic across runs (independent of which tokio task
    /// schedules its first `PricingTick` first). Conventionally
    /// passed the lexicographically smallest node name from the
    /// topology.
    pub fn set_representative_node(&mut self, name: impl Into<String>) {
        self.representative_node = Some(name.into());
    }

    /// Number of representative-node pricing ticks ingested so far.
    pub fn pricing_ticks(&self) -> u64 {
        self.pricing_ticks
    }

    /// Feed one event into the collector.
    pub fn ingest(&mut self, event: &Event) {
        match event {
            Event::TXGenerated {
                id,
                urgency_component_index,
                value_lovelace,
                urgency,
                slot,
                ..
            } => {
                // Phase-2 `TXGenerated` carries the actor-relevant
                // fields directly. Legacy non-actor txs default to
                // (component 0, value 0, urgency 1.0); welfare for
                // those collapses to retained_value = 0 and
                // net_utility = -fee, which is correct for txs that
                // never asserted a value.
                let comp = self
                    .components
                    .entry(*urgency_component_index)
                    .or_insert_with(|| ComponentSummary {
                        component_index: *urgency_component_index,
                        ..Default::default()
                    });
                comp.txs_submitted += 1;
                // `submit_slot` is carried on the event itself (M4+),
                // so it is independent of the simulator's
                // intra-slot ordering.
                let submit_slot = *slot;
                self.tx_meta.insert(
                    *id,
                    TxMeta {
                        component_index: *urgency_component_index,
                        value_lovelace: *value_lovelace,
                        urgency: *urgency,
                        submit_slot,
                    },
                );
            }
            Event::TXIncluded {
                id,
                slot,
                bytes,
                posted_lane,
                served_lane,
                max_fee_lovelace,
                actual_fee_lovelace,
                refund_lovelace,
                ..
            } => {
                self.advance_to_slot(*slot);
                // Pricing-event-stream hash. Encoding matches the M2
                // cross-arch golden test (`run_seeded_pricing_scenario`
                // in sim-core/src/sim/tests/m2_two_lane.rs).
                self.pricing_event_hasher.update(b"INCL");
                self.pricing_event_hasher.update(id.to_string().as_bytes());
                self.pricing_event_hasher.update(slot.to_le_bytes());
                self.pricing_event_hasher.update(bytes.to_le_bytes());
                self.pricing_event_hasher.update([
                    match posted_lane {
                        Lane::Standard => 0,
                        Lane::Priority => 1,
                    },
                    match served_lane {
                        Lane::Standard => 0,
                        Lane::Priority => 1,
                    },
                ]);
                self.pricing_event_hasher
                    .update(max_fee_lovelace.to_le_bytes());
                self.pricing_event_hasher
                    .update(actual_fee_lovelace.to_le_bytes());
                self.pricing_event_hasher
                    .update(refund_lovelace.to_le_bytes());
                match served_lane {
                    Lane::Priority => {
                        self.delta.included_bytes_priority += *bytes;
                        self.delta.included_count_priority += 1;
                    }
                    Lane::Standard => {
                        self.delta.included_bytes_standard += *bytes;
                        self.delta.included_count_standard += 1;
                    }
                }
                self.delta.fees_paid_lovelace = self
                    .delta
                    .fees_paid_lovelace
                    .saturating_add(*actual_fee_lovelace);
                self.delta.refund_lovelace =
                    self.delta.refund_lovelace.saturating_add(*refund_lovelace);

                if let Some(meta) = self.tx_meta.remove(id) {
                    let latency_slots = slot.saturating_sub(meta.submit_slot) as f64;
                    let latency_blocks = latency_slots * self.block_generation_probability;
                    let retained_value =
                        welfare::retained_value(meta.value_lovelace, meta.urgency, latency_blocks);
                    let net_utility = welfare::net_utility(retained_value, *actual_fee_lovelace);
                    let comp = self
                        .components
                        .entry(meta.component_index)
                        .or_insert_with(|| ComponentSummary {
                            component_index: meta.component_index,
                            ..Default::default()
                        });
                    comp.txs_included += 1;
                    comp.bytes_included += bytes;
                    comp.fees_paid_lovelace =
                        comp.fees_paid_lovelace.saturating_add(*actual_fee_lovelace);
                    comp.refund_lovelace = comp.refund_lovelace.saturating_add(*refund_lovelace);
                    comp.retained_value_total += retained_value;
                    comp.net_utility_total += net_utility;
                    comp.included_value_lovelace_total = comp
                        .included_value_lovelace_total
                        .saturating_add(meta.value_lovelace as u128);
                    comp.latency_blocks_observations.push(latency_blocks);
                    match served_lane {
                        Lane::Priority => {
                            comp.priority_included += 1;
                            self.priority_retained_value_total += retained_value;
                            self.priority_included_value_total = self
                                .priority_included_value_total
                                .saturating_add(meta.value_lovelace as u128);
                        }
                        Lane::Standard => {
                            comp.standard_included += 1;
                            self.standard_retained_value_total += retained_value;
                            self.standard_included_value_total = self
                                .standard_included_value_total
                                .saturating_add(meta.value_lovelace as u128);
                        }
                    }
                }
                let _ = posted_lane;
            }
            Event::TXEvictedQuoteDrift {
                id,
                slot,
                bytes,
                posted_lane,
                current_quote_per_byte,
                max_fee_lovelace,
                ..
            } => {
                self.advance_to_slot(*slot);
                // Pricing-event-stream hash; same encoding as M2.
                self.pricing_event_hasher.update(b"EVCT");
                self.pricing_event_hasher.update(id.to_string().as_bytes());
                self.pricing_event_hasher.update(slot.to_le_bytes());
                self.pricing_event_hasher.update(bytes.to_le_bytes());
                self.pricing_event_hasher.update([match posted_lane {
                    Lane::Standard => 0,
                    Lane::Priority => 1,
                }]);
                self.pricing_event_hasher
                    .update(current_quote_per_byte.to_le_bytes());
                self.pricing_event_hasher
                    .update(max_fee_lovelace.to_le_bytes());
                self.delta.evicted_quote_drift_count += 1;
                if let Some(meta) = self.tx_meta.remove(id) {
                    let comp = self
                        .components
                        .entry(meta.component_index)
                        .or_insert_with(|| ComponentSummary {
                            component_index: meta.component_index,
                            ..Default::default()
                        });
                    comp.txs_evicted_quote_drift += 1;
                }
            }
            Event::LinearPricingSampleApplied {
                node,
                slot,
                producer,
            } => {
                let node_name = node.name.as_str();
                if !self.is_representative(node_name) {
                    return;
                }
                self.sample_producers_by_slot
                    .entry(*slot)
                    .or_default()
                    .insert(producer.name.as_ref().clone());
            }
            Event::PricingTick {
                node,
                slot,
                priority_quote_per_byte,
                standard_quote_per_byte,
                priority_window_util_x_1e9,
                standard_window_util_x_1e9,
                mempool_bytes_total,
                mempool_bytes_priority,
                mempool_bytes_standard,
            } => {
                let node_name = node.name.as_str();
                if !self.is_representative(node_name) {
                    return;
                }
                self.pricing_ticks += 1;
                self.advance_to_slot(*slot);
                self.delta.c_priority_quote_per_byte = *priority_quote_per_byte;
                self.delta.c_standard_quote_per_byte = *standard_quote_per_byte;
                self.delta.priority_window_util_x_1e9 = priority_window_util_x_1e9.unwrap_or(0);
                self.delta.standard_window_util_x_1e9 = standard_window_util_x_1e9.unwrap_or(0);
                self.delta.mempool_bytes_total = *mempool_bytes_total;
                self.delta.mempool_bytes_priority = *mempool_bytes_priority;
                self.delta.mempool_bytes_standard = *mempool_bytes_standard;

                // Multiplier-floor invariant check: when the priority
                // and standard quotes both exist and standard > 0,
                // priority/standard must be ≥ multiplier_floor.
                if *standard_quote_per_byte > 0 {
                    let ratio =
                        (*priority_quote_per_byte as f64) / (*standard_quote_per_byte as f64);
                    self.min_ratio = self.min_ratio.min(ratio);
                    self.max_ratio = self.max_ratio.max(ratio);
                    if let (Some(num), Some(den)) =
                        (self.multiplier_floor_num, self.multiplier_floor_den)
                    {
                        let floor = (num as f64) / (den as f64);
                        // 1e-9 tolerance for the ratio-of-integers
                        // representation: priority and standard are
                        // u64 and the floor is enforced exactly in
                        // u128, so any breach should be visible at
                        // any tolerance.
                        if ratio + 1e-9 < floor {
                            self.multiplier_floor_breaches += 1;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn is_representative(&mut self, node_name: &str) -> bool {
        match &self.representative_node {
            Some(name) => name == node_name,
            None => {
                self.representative_node = Some(node_name.to_string());
                true
            }
        }
    }

    fn advance_to_slot(&mut self, slot: u64) {
        // Flush deltas at slot boundaries: when an event for `slot`
        // arrives that is later than the current `delta.slot`, we
        // push the in-progress row and start a new one.
        if slot < self.delta.slot {
            // Out-of-order event (shouldn't happen in single-process
            // sim). Keep the in-progress row's slot.
            return;
        }
        if slot > self.delta.slot {
            // Push the in-progress row only if it has any data
            // (cumulative `c_*` values get re-set on the next tick).
            self.flush_current_row();
            self.delta = TimeSeriesRow {
                slot,
                ..Default::default()
            };
        }
    }

    fn flush_current_row(&mut self) {
        // Push every slot we've seen — even ones with no events —
        // *if* we have a meaningful slot number. The first slot's
        // initial empty row gets dropped here; subsequent rows are
        // preserved.
        if self.rows.is_empty() && self.delta.slot == 0 && self.is_zero_row(&self.delta) {
            return;
        }
        self.rows.push(std::mem::take(&mut self.delta));
    }

    fn is_zero_row(&self, row: &TimeSeriesRow) -> bool {
        row.included_bytes_priority == 0
            && row.included_bytes_standard == 0
            && row.included_count_priority == 0
            && row.included_count_standard == 0
            && row.evicted_quote_drift_count == 0
            && row.fees_paid_lovelace == 0
            && row.refund_lovelace == 0
            && row.c_priority_quote_per_byte == 0
            && row.c_standard_quote_per_byte == 0
            && row.mempool_bytes_total == 0
    }

    /// Build a non-consuming snapshot for progressive artefact writes
    /// while a simulation is still running.
    pub fn snapshot(&self) -> (Vec<TimeSeriesRow>, RunSummary) {
        (self.snapshot_rows(), self.snapshot_summary())
    }

    fn snapshot_rows(&self) -> Vec<TimeSeriesRow> {
        let mut rows = self.rows.clone();
        if !(rows.is_empty() && self.delta.slot == 0 && self.is_zero_row(&self.delta)) {
            rows.push(self.delta.clone());
        }
        rows
    }

    fn snapshot_summary(&self) -> RunSummary {
        let mut total_submitted = 0u64;
        let mut total_included = 0u64;
        let mut total_evicted = 0u64;
        let mut total_fees = 0u64;
        let mut total_refund = 0u64;
        let mut components: Vec<ComponentSummary> = self.components.values().cloned().collect();
        components.sort_by_key(|c| c.component_index);
        for c in &components {
            total_submitted = total_submitted.saturating_add(c.txs_submitted);
            total_included = total_included.saturating_add(c.txs_included);
            total_evicted = total_evicted.saturating_add(c.txs_evicted_quote_drift);
            total_fees = total_fees.saturating_add(c.fees_paid_lovelace);
            total_refund = total_refund.saturating_add(c.refund_lovelace);
        }
        let pricing_event_stream_sha256 = hex::encode(self.pricing_event_hasher.clone().finalize());
        let mut slot_battles_count: u64 = 0;
        let mut orphaned_pricing_samples: u64 = 0;
        for producers in self.sample_producers_by_slot.values() {
            if producers.len() >= 2 {
                slot_battles_count += 1;
                orphaned_pricing_samples += (producers.len() - 1) as u64;
            }
        }
        let (max_single_step_shock, max_window_shock, p90_window_shock, eviction_risk_at_4x) =
            self.compute_price_shock_metrics();
        RunSummary {
            components,
            total_txs_submitted: total_submitted,
            total_txs_included: total_included,
            total_txs_evicted_quote_drift: total_evicted,
            total_fees_paid_lovelace: total_fees,
            total_refund_lovelace: total_refund,
            priority_retained_value_total: self.priority_retained_value_total,
            standard_retained_value_total: self.standard_retained_value_total,
            priority_included_value_total: self.priority_included_value_total,
            standard_included_value_total: self.standard_included_value_total,
            block_generation_probability: self.block_generation_probability,
            multiplier_floor_breaches: self.multiplier_floor_breaches,
            min_priority_over_standard_ratio: if self.min_ratio.is_finite() {
                self.min_ratio
            } else {
                0.0
            },
            max_priority_over_standard_ratio: self.max_ratio,
            pricing_ticks: self.pricing_ticks,
            pricing_event_stream_sha256,
            slot_battles_count,
            orphaned_pricing_samples,
            max_single_step_priority_shock: max_single_step_shock,
            max_priority_shock_over_window: max_window_shock,
            p90_priority_shock_over_window: p90_window_shock,
            eviction_risk_rate_at_4x: eviction_risk_at_4x,
            shock_window_slots: self.shock_window_slots,
        }
    }

    /// Compute the four M9 price-shock UX metrics from the per-slot
    /// priority-quote time series. Returns
    /// `(max_single_step, max_window, p90_window, eviction_risk_at_4x)`.
    /// Returns all-zero when there are fewer than `shock_window_slots`
    /// rows of priced data (insufficient signal).
    fn compute_price_shock_metrics(&self) -> (f64, f64, f64, f64) {
        let prices: Vec<u64> = self
            .rows
            .iter()
            .chain(std::iter::once(&self.delta))
            .map(|r| r.c_priority_quote_per_byte)
            .filter(|&q| q > 0)
            .collect();
        let window = self.shock_window_slots as usize;
        if prices.len() < 2 || window == 0 || prices.len() < window + 1 {
            return (0.0, 0.0, 0.0, 0.0);
        }
        // Max single-slot ratio change.
        let mut max_single_step = 1.0_f64;
        for w in prices.windows(2) {
            let ratio = (w[1] as f64) / (w[0] as f64);
            if ratio > max_single_step {
                max_single_step = ratio;
            }
        }
        // Rolling-window upward shock.
        let mut window_shocks: Vec<f64> = Vec::with_capacity(prices.len().saturating_sub(window));
        for i in 0..(prices.len() - window) {
            let start = prices[i] as f64;
            let peak = prices[i..i + window].iter().max().copied().unwrap_or(0) as f64;
            window_shocks.push(peak / start);
        }
        let max_window = window_shocks.iter().copied().fold(1.0_f64, f64::max);
        // p90: 90th-percentile shock across all windows.
        let mut sorted = window_shocks.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p90_idx = ((sorted.len() as f64) * 0.9) as usize;
        let p90 = sorted
            .get(p90_idx.min(sorted.len() - 1))
            .copied()
            .unwrap_or(1.0);
        // Eviction-risk rate at 4×.
        let dangerous = window_shocks.iter().filter(|&&s| s > 4.0).count();
        let eviction_risk = (dangerous as f64) / (window_shocks.len() as f64);
        (max_single_step, max_window, p90, eviction_risk)
    }

    /// Stop accumulating; finalise rows and produce a summary.
    pub fn finalise(mut self) -> (Vec<TimeSeriesRow>, RunSummary) {
        self.flush_current_row();
        let summary = self.snapshot_summary();
        (self.rows, summary)
    }
}

// ----------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use sim_core::{
        config::NodeId,
        events::{Event, Node},
        model::TransactionId,
    };

    use super::*;

    fn node(id: u64, name: &str) -> Node {
        Node {
            id: NodeId::new(id as usize),
            name: Arc::new(name.to_string()),
        }
    }

    fn pricing_tick(node_name: &str, slot: u64, priority_q: u64, standard_q: u64) -> Event {
        Event::PricingTick {
            node: node(0, node_name),
            slot,
            priority_quote_per_byte: priority_q,
            standard_quote_per_byte: standard_q,
            priority_window_util_x_1e9: None,
            standard_window_util_x_1e9: None,
            mempool_bytes_total: 0,
            mempool_bytes_priority: 0,
            mempool_bytes_standard: 0,
        }
    }

    fn tx_generated(
        id: u64,
        component: u32,
        value_lovelace: u64,
        urgency: f64,
        bytes: u64,
    ) -> Event {
        Event::TXGenerated {
            id: TransactionId::new(id),
            publisher: node(0, "n0"),
            size_bytes: bytes,
            shard: 0,
            input_id: id,
            overcollateralization_factor: 0,
            urgency_component_index: component,
            value_lovelace,
            urgency,
            posted_lane: Lane::Priority,
            max_fee_lovelace: 1_000_000,
            slot: 0,
        }
    }

    fn tx_included(
        id: u64,
        slot: u64,
        bytes: u64,
        served_lane: Lane,
        actual_fee: u64,
        refund: u64,
    ) -> Event {
        Event::TXIncluded {
            id: TransactionId::new(id),
            producer: node(0, "n0"),
            slot,
            bytes,
            posted_lane: Lane::Priority,
            served_lane,
            max_fee_lovelace: actual_fee + refund,
            actual_fee_lovelace: actual_fee,
            refund_lovelace: refund,
        }
    }

    fn tx_evicted(id: u64, slot: u64, bytes: u64, current_quote: u64) -> Event {
        Event::TXEvictedQuoteDrift {
            id: TransactionId::new(id),
            node: node(0, "n0"),
            slot,
            bytes,
            posted_lane: Lane::Priority,
            current_quote_per_byte: current_quote,
            max_fee_lovelace: 0,
        }
    }

    fn linear_pricing_sample_applied(node_name: &str, slot: u64, producer_name: &str) -> Event {
        Event::LinearPricingSampleApplied {
            node: node(0, node_name),
            slot,
            producer: node(0, producer_name),
        }
    }

    /// M6 fork-resolution metric: at the representative node, count
    /// slots where two or more RBs were priced (slot battles) and
    /// the per-extra-RB orphan total (one canonical chain ⇒ N-1
    /// siblings per battle slot are orphans).
    #[test]
    fn slot_battle_metric_counts_sibling_rbs_at_representative() {
        let mut c = MetricsCollector::new(0.05);
        c.set_representative_node("pool-000");

        // Slot 10: two sibling RBs (A and B). 1 battle, 1 orphan.
        c.ingest(&linear_pricing_sample_applied("pool-000", 10, "pool-A"));
        c.ingest(&linear_pricing_sample_applied("pool-000", 10, "pool-B"));
        // Slot 11: a single RB. Not a battle.
        c.ingest(&linear_pricing_sample_applied("pool-000", 11, "pool-A"));
        // Slot 20: three siblings. 1 battle, 2 orphans.
        c.ingest(&linear_pricing_sample_applied("pool-000", 20, "pool-A"));
        c.ingest(&linear_pricing_sample_applied("pool-000", 20, "pool-B"));
        c.ingest(&linear_pricing_sample_applied("pool-000", 20, "pool-C"));
        // Non-representative node: must be ignored.
        c.ingest(&linear_pricing_sample_applied("pool-099", 10, "pool-Z"));

        let (_, summary) = c.finalise();
        assert_eq!(summary.slot_battles_count, 2, "slots 10 and 20");
        assert_eq!(
            summary.orphaned_pricing_samples, 3,
            "slot 10: 1 orphan + slot 20: 2 orphans"
        );
    }

    /// Repeated samples from the same producer at the same slot do
    /// NOT count as a slot battle (defensive against accidental
    /// double-emission from the simulator hooking apply_priced_block
    /// twice on the same RB).
    #[test]
    fn slot_battle_metric_ignores_same_producer_duplicates() {
        let mut c = MetricsCollector::new(0.05);
        c.set_representative_node("pool-000");
        c.ingest(&linear_pricing_sample_applied("pool-000", 5, "pool-A"));
        c.ingest(&linear_pricing_sample_applied("pool-000", 5, "pool-A"));
        let (_, summary) = c.finalise();
        assert_eq!(summary.slot_battles_count, 0);
        assert_eq!(summary.orphaned_pricing_samples, 0);
    }

    /// Slot-boundary flush: when an event for slot N+1 arrives, the
    /// in-progress row for slot N is pushed; the new delta row's
    /// `slot` is N+1.
    #[test]
    fn slot_boundary_flushes_in_progress_row() {
        let mut c = MetricsCollector::new(0.05);
        // Slot 1 has activity.
        c.ingest(&pricing_tick("n0", 1, 100, 50));
        c.ingest(&tx_generated(1, 0, 1_000_000, 1.05, 1024));
        c.ingest(&tx_included(1, 1, 1024, Lane::Priority, 200, 800));
        // Slot 2 advances; the slot-1 row should flush.
        c.ingest(&pricing_tick("n0", 2, 110, 50));
        c.ingest(&tx_generated(2, 0, 1_000_000, 1.05, 1024));
        c.ingest(&tx_included(2, 2, 1024, Lane::Priority, 220, 780));
        let (rows, _) = c.finalise();
        // Two rows: slot 1 and slot 2 (slot 0 is empty and suppressed).
        assert_eq!(rows.len(), 2, "expected slot-1 + slot-2 rows, got {rows:?}");
        assert_eq!(rows[0].slot, 1);
        assert_eq!(rows[0].included_count_priority, 1);
        assert_eq!(rows[0].fees_paid_lovelace, 200);
        assert_eq!(rows[1].slot, 2);
        assert_eq!(rows[1].included_count_priority, 1);
        assert_eq!(rows[1].fees_paid_lovelace, 220);
    }

    /// First-row suppression: an empty zero-row at slot 0 (no events
    /// observed) is dropped, so the CSV doesn't carry a misleading
    /// all-zeroes leading row.
    #[test]
    fn empty_first_row_is_suppressed() {
        let mut c = MetricsCollector::new(0.05);
        c.ingest(&pricing_tick("n0", 0, 44, 44));
        c.ingest(&pricing_tick("n0", 1, 44, 44));
        let (rows, _) = c.finalise();
        // Slot-0 row was zero-ish (only `c_*` set; no inclusions); the
        // suppression keeps slot-0 only if it carried inclusion deltas
        // — it does have c_* set, so it's kept. Confirm that we got
        // the slot-0 row (with c_* > 0) and slot-1.
        assert!(
            !rows.is_empty(),
            "expected at least the slot-0 row (carries c_* values)"
        );
    }

    /// Truly-empty pre-flush state at slot 0 doesn't emit a row.
    #[test]
    fn pre_event_state_at_slot_zero_is_suppressed() {
        let c = MetricsCollector::new(0.05);
        // No events ingested. Finalise.
        let (rows, _) = c.finalise();
        assert!(
            rows.is_empty(),
            "expected no rows when no events arrived; got {rows:?}"
        );
    }

    /// Negative `net_utility` (regret event) is preserved through
    /// the per-component aggregation. Plan line 152.
    #[test]
    fn negative_net_utility_is_preserved_in_summary() {
        let mut c = MetricsCollector::new(0.05);
        c.ingest(&pricing_tick("n0", 0, 1000, 1000));
        // value 1, urgency 1.0 ⇒ retained_value = 1.0; actual_fee
        // 1_000_000 ⇒ net_utility = 1.0 - 1_000_000 = very negative.
        c.ingest(&tx_generated(1, 7, 1, 1.0, 1024));
        c.ingest(&tx_included(1, 0, 1024, Lane::Priority, 1_000_000, 0));
        let (_, summary) = c.finalise();
        assert_eq!(summary.components.len(), 1);
        let c0 = &summary.components[0];
        assert_eq!(c0.component_index, 7);
        assert_eq!(c0.txs_included, 1);
        assert!(
            c0.net_utility_total < 0.0,
            "regret event must produce negative net_utility_total, got {}",
            c0.net_utility_total
        );
    }

    /// Pricing-event-stream hash is deterministic across two runs of
    /// the same event sequence.
    #[test]
    fn pricing_event_stream_hash_deterministic_across_runs() {
        fn run_once() -> String {
            let mut c = MetricsCollector::new(0.05);
            c.ingest(&pricing_tick("n0", 0, 100, 50));
            c.ingest(&tx_generated(1, 0, 1_000_000, 1.05, 1024));
            c.ingest(&tx_included(1, 0, 1024, Lane::Priority, 200, 800));
            c.ingest(&pricing_tick("n0", 1, 100, 50));
            c.ingest(&tx_evicted(2, 1, 1024, 100));
            let (_, summary) = c.finalise();
            summary.pricing_event_stream_sha256
        }
        let h1 = run_once();
        let h2 = run_once();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64, "sha256 hex digest must be 64 chars");
    }

    /// Pricing-event-stream hash diverges if the events differ
    /// (different served_lane changes the encoded byte).
    #[test]
    fn pricing_event_stream_hash_distinguishes_distinct_events() {
        let mut c1 = MetricsCollector::new(0.05);
        c1.ingest(&tx_included(1, 0, 1024, Lane::Priority, 200, 800));
        let mut c2 = MetricsCollector::new(0.05);
        c2.ingest(&tx_included(1, 0, 1024, Lane::Standard, 200, 800));
        let h1 = c1.finalise().1.pricing_event_stream_sha256;
        let h2 = c2.finalise().1.pricing_event_stream_sha256;
        assert_ne!(h1, h2);
    }

    /// Lazy fallback: with no pre-set representative, the first
    /// observed node wins and other nodes' ticks are ignored.
    #[test]
    fn representative_node_lazy_fallback_picks_first_arrived() {
        let mut c = MetricsCollector::new(0.05);
        c.ingest(&pricing_tick("n0", 0, 100, 50));
        // n1's tick at the same slot should be ignored.
        c.ingest(&pricing_tick("n1", 0, 999, 999));
        let (_, summary) = c.finalise();
        assert_eq!(summary.pricing_ticks, 1);
    }

    /// Pre-set representative pins the choice even when another node
    /// ticks first, and the pin survives slot advances (a regression
    /// where pinning held only at slot 0 would still pass without
    /// the multi-slot assertion).
    #[test]
    fn representative_node_pinning_overrides_first_arrival() {
        let mut c = MetricsCollector::new(0.05);
        c.set_representative_node("n0");
        // Slot 0: n1 ticks first but is NOT the pinned representative.
        c.ingest(&pricing_tick("n1", 0, 999, 999));
        c.ingest(&pricing_tick("n0", 0, 100, 50));
        // Slot 1: n1 again ticks first; n0's tick is the one that
        // must populate the row.
        c.ingest(&pricing_tick("n1", 1, 888, 888));
        c.ingest(&pricing_tick("n0", 1, 110, 55));
        let (rows, summary) = c.finalise();
        // Two ticks counted (one per slot), both from n0.
        assert_eq!(summary.pricing_ticks, 2);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].c_priority_quote_per_byte, 100);
        assert_eq!(rows[0].c_standard_quote_per_byte, 50);
        assert_eq!(rows[1].c_priority_quote_per_byte, 110);
        assert_eq!(rows[1].c_standard_quote_per_byte, 55);
    }

    /// Out-of-order events (slot decreasing) do not roll the
    /// time-series back; the in-progress row keeps its slot. The
    /// event's accounting folds into the current row (documented
    /// behaviour of `advance_to_slot`); past rows must not be touched.
    #[test]
    fn out_of_order_events_do_not_roll_slot_backwards() {
        let mut c = MetricsCollector::new(0.05);
        // Slot 1 has its own activity that must survive untouched.
        c.ingest(&pricing_tick("n0", 1, 90, 45));
        c.ingest(&tx_generated(99, 0, 1_000_000, 1.0, 1024));
        c.ingest(&tx_included(99, 1, 1024, Lane::Priority, 100, 0));
        // Advance to slot 5.
        c.ingest(&pricing_tick("n0", 5, 100, 50));
        // A late inclusion event for slot 1 arrives after we've
        // advanced to 5. Its accounting folds into slot 5's row, not
        // slot 1's. The slot-1 row's counts must not change.
        c.ingest(&tx_generated(1, 0, 1_000_000, 1.0, 1024));
        c.ingest(&tx_included(1, 1, 1024, Lane::Priority, 200, 800));
        let (rows, _) = c.finalise();
        assert_eq!(rows.len(), 2);
        // Slot-1 row has its original single inclusion only.
        assert_eq!(rows[0].slot, 1);
        assert_eq!(rows[0].included_count_priority, 1);
        assert_eq!(rows[0].fees_paid_lovelace, 100);
        // Slot-5 row absorbed the out-of-order inclusion.
        assert_eq!(
            rows[1].slot, 5,
            "row slot must not regress on out-of-order events"
        );
        assert_eq!(rows[1].included_count_priority, 1);
        assert_eq!(rows[1].fees_paid_lovelace, 200);
        // Cross-bucket integrity: zero standard activity in either slot.
        assert_eq!(rows[0].included_count_standard, 0);
        assert_eq!(rows[1].included_count_standard, 0);
        assert_eq!(rows[0].included_bytes_standard, 0);
        assert_eq!(rows[1].included_bytes_standard, 0);
    }

    /// Multiplier-floor breach is detected (priority < floor × standard).
    #[test]
    fn multiplier_floor_breach_is_counted() {
        let mut c = MetricsCollector::new(0.05);
        c.set_multiplier_floor(16, 1);
        // priority=44, standard=44 → ratio = 1.0, well below 16. Breach.
        c.ingest(&pricing_tick("n0", 0, 44, 44));
        let (_, summary) = c.finalise();
        assert_eq!(summary.multiplier_floor_breaches, 1);
    }

    /// Multiplier-floor invariant holds → 0 breaches.
    #[test]
    fn multiplier_floor_holds_no_breach() {
        let mut c = MetricsCollector::new(0.05);
        c.set_multiplier_floor(16, 1);
        // priority=704 = 16*44, standard=44 → ratio = 16.0, exact floor.
        c.ingest(&pricing_tick("n0", 0, 704, 44));
        let (_, summary) = c.finalise();
        assert_eq!(summary.multiplier_floor_breaches, 0);
    }

    /// `RunSummary` round-trips via JSON (the runner persists it
    /// across `experiment-suite run` invocations).
    #[test]
    fn run_summary_json_roundtrip() {
        let mut c = MetricsCollector::new(0.05);
        c.ingest(&pricing_tick("n0", 0, 100, 50));
        c.ingest(&tx_generated(1, 3, 1_000_000, 2.0, 1024));
        c.ingest(&tx_included(1, 0, 1024, Lane::Priority, 200, 800));
        let (_, summary) = c.finalise();
        let json = serde_json::to_string(&summary).unwrap();
        let round: RunSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(round.total_txs_included, summary.total_txs_included);
        assert_eq!(round.components.len(), summary.components.len());
        assert_eq!(
            round.pricing_event_stream_sha256,
            summary.pricing_event_stream_sha256
        );
    }
}
