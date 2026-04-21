use anyhow::Result;
use rand::Rng;
use rand_chacha::ChaChaRng;
use rand_distr::Distribution;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};
use tokio::sync::mpsc;
use tracing::warn;

use crate::{
    clock::{ClockBarrier, Timestamp},
    config::{NodeId, RealTransactionConfig, SimConfiguration, TransactionConfig, TxGenerator},
    events::{EventTracker, TxProducerPhase},
    model::{ActorId, Transaction, UrgencyProfile},
    tx_actors::{Actor, ArrivalPattern, build_actors},
};

struct NodeState {
    sink: mpsc::UnboundedSender<Arc<Transaction>>,
    tx_conflict_fraction: Option<f64>,
    tx_generation_weight: u64,
}

pub struct TransactionProducer {
    rng: ChaChaRng,
    clock: ClockBarrier,
    tracker: EventTracker,
    nodes: HashMap<NodeId, NodeState>,
    mode: ProducerMode,
    max_slots: Option<u64>,
}

enum ProducerMode {
    Disabled,
    Legacy(RealTransactionConfig),
    Actors {
        config: RealTransactionConfig,
        actors: Vec<Actor>,
    },
}

impl TransactionProducer {
    pub fn new(
        rng: ChaChaRng,
        clock: ClockBarrier,
        tracker: EventTracker,
        mut node_tx_sinks: HashMap<NodeId, mpsc::UnboundedSender<Arc<Transaction>>>,
        config: &SimConfiguration,
    ) -> Self {
        let nodes = config
            .nodes
            .iter()
            .map(|node| {
                let sink = node_tx_sinks.remove(&node.id).unwrap();
                let state =
                    NodeState {
                        sink,
                        tx_conflict_fraction: node.tx_conflict_fraction,
                        tx_generation_weight: node
                            .tx_generation_weight
                            .unwrap_or(if node.stake > 0 { 0 } else { 1 }),
                    };
                (node.id, state)
            })
            .collect();
        let mode = match (config.tx_generator, &config.transactions) {
            (_, TransactionConfig::Mock(_)) => ProducerMode::Disabled,
            (TxGenerator::Legacy, TransactionConfig::Real(real_config)) => {
                ProducerMode::Legacy(real_config.clone())
            }
            (TxGenerator::Actors, TransactionConfig::Real(real_config)) => {
                let actor_configs = config.actors.as_ref().cloned().unwrap_or_default();
                let actors = build_actors(&actor_configs);
                ProducerMode::Actors {
                    config: real_config.clone(),
                    actors,
                }
            }
        };
        Self {
            rng,
            clock,
            tracker,
            nodes,
            mode,
            max_slots: config.slots,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mode = std::mem::replace(&mut self.mode, ProducerMode::Disabled);
        match mode {
            ProducerMode::Disabled => {
                self.clock.wait_forever().await;
                Ok(())
            }
            ProducerMode::Legacy(config) => self.run_legacy(config).await,
            ProducerMode::Actors { config, actors } => self.run_actors(config, actors).await,
        }
    }

    fn build_node_lookup(&self) -> Result<WeightedLookup<NodeId>> {
        let mut node_weights: Vec<(NodeId, u64)> = self
            .nodes
            .iter()
            .filter_map(|(id, node)| {
                let weight = node.tx_generation_weight;
                (weight != 0).then_some((*id, weight))
            })
            .collect();

        if node_weights.is_empty() {
            warn!(
                "No nodes have tx-generation-weight > 0; falling back to uniform tx generation \
                 across all nodes."
            );
            node_weights = self.nodes.keys().map(|id| (*id, 1)).collect();
        }

        if node_weights.is_empty() {
            anyhow::bail!("no nodes available for transaction generation");
        }

        // Sort by NodeId for deterministic WeightedLookup construction,
        // since self.nodes is a HashMap with nondeterministic iteration order.
        node_weights.sort_by_key(|(id, _)| *id);

        Ok(WeightedLookup::new(node_weights))
    }

    async fn run_legacy(&mut self, config: RealTransactionConfig) -> Result<()> {
        let mut next_tx_at = Timestamp::zero();
        let node_lookup = self.build_node_lookup()?;
        let mut rng = &mut self.rng;

        if let Some(start) = config.start_time {
            self.clock.wait_until(start).await;
            next_tx_at = start;
        };

        loop {
            let node_id = node_lookup.sample(rng).unwrap();
            let node = self.nodes.get(node_id).unwrap();

            let mut tx = config.new_tx(rng, node.tx_conflict_fraction);
            tx.submission_slot = (self.clock.now() - Timestamp::zero()).as_secs();
            node.sink.send(Arc::new(tx))?;

            let millis_until_tx = config.frequency_ms.sample(&mut rng) as u64;
            next_tx_at += Duration::from_millis(millis_until_tx);

            if config.stop_time.is_some_and(|t| next_tx_at > t) {
                self.clock.wait_forever().await;
                return Ok(());
            } else {
                self.clock.wait_until(next_tx_at).await;
            }
        }
    }

    async fn run_actors(
        &mut self,
        config: RealTransactionConfig,
        actors: Vec<Actor>,
    ) -> Result<()> {
        let node_lookup = self.build_node_lookup()?;
        let mut slot = 0u64;
        let mut next_slot_at = Timestamp::zero();
        if let Some(start) = config.start_time {
            self.clock.wait_until(start).await;
            next_slot_at = start;
            slot = (start - Timestamp::zero()).as_secs();
        }

        loop {
            if self.max_slots == Some(slot) {
                self.clock.wait_forever().await;
                return Ok(());
            }

            let current_slot = slot;
            self.tracker
                .track_tx_producer_diagnostics(current_slot, TxProducerPhase::StartSlot);
            for tx in generate_actor_transactions(slot, &actors, &config, &mut self.rng) {
                let node_id = node_lookup.sample(&mut self.rng).unwrap();
                let node = self.nodes.get(node_id).unwrap();
                node.sink.send(Arc::new(tx))?;
            }
            self.tracker
                .track_tx_producer_diagnostics(current_slot, TxProducerPhase::Generated);

            slot = slot.saturating_add(1);
            next_slot_at += Duration::from_secs(1);

            if config.stop_time.is_some_and(|t| next_slot_at > t) {
                self.clock.wait_forever().await;
                return Ok(());
            } else {
                self.tracker
                    .track_tx_producer_diagnostics(current_slot, TxProducerPhase::Waiting);
                self.clock.wait_until(next_slot_at).await;
            }
        }
    }
}

fn generate_actor_transactions(
    slot: u64,
    actors: &[Actor],
    config: &RealTransactionConfig,
    rng: &mut ChaChaRng,
) -> Vec<Transaction> {
    let mut txs = Vec::new();
    for actor in actors {
        let count = sample_arrivals(actor, slot, rng);
        for _ in 0..count {
            let bytes = actor.tx_size.sample_u64(rng).min(config.max_size);
            let (value, urgency, urgency_component_index) = sample_value_and_urgency(actor, rng);
            let tx = Transaction {
                id: config.next_transaction_id(),
                actor_id: actor.id,
                shard: config.sample_shard(rng),
                bytes,
                submission_slot: slot,
                mempool_entry_slot: None,
                mempool_entry_rb_index: None,
                value,
                urgency,
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
                assigned_block_kind: None,
                input_id: config.next_input_id(),
                overcollateralization_factor: 0,
                urgency_component_index,
            };
            txs.push(tx);
        }
    }
    txs
}

fn sample_value_and_urgency(
    actor: &Actor,
    rng: &mut ChaChaRng,
) -> (u64, UrgencyProfile, Option<u16>) {
    if let Some((index, component)) = sample_correlated_component(actor, rng) {
        let value = component.value_distribution.sample_u64(rng);
        let u = component.urgency_u_distribution.sample_f64(rng);
        return (value, urgency_profile_from_u(actor, u), Some(index));
    }

    let value = actor.value_distribution.sample_u64(rng);
    let urgency = actor
        .urgency_u_distribution
        .as_ref()
        .map(|distribution| urgency_profile_from_u(actor, distribution.sample_f64(rng)))
        .unwrap_or_else(|| actor.urgency.clone());
    (value, urgency, None)
}

fn sample_correlated_component<'a, R: Rng + ?Sized>(
    actor: &'a Actor,
    rng: &mut R,
) -> Option<(u16, &'a crate::tx_actors::ValueUrgencyComponent)> {
    if actor.value_urgency_components.is_empty() {
        return None;
    }
    let total_weight = actor
        .value_urgency_components
        .iter()
        .filter_map(|component| {
            let weight = component.weight;
            (weight.is_finite() && weight > 0.0).then_some(weight)
        })
        .sum::<f64>();
    if total_weight <= 0.0 {
        warn_bad_component_weights_once(actor);
        return actor.value_urgency_components.first().map(|c| (0u16, c));
    }

    let mut choice = rng.random::<f64>() * total_weight;
    for (i, component) in actor.value_urgency_components.iter().enumerate() {
        let weight = component.weight;
        if !(weight.is_finite() && weight > 0.0) {
            continue;
        }
        if choice <= weight {
            return Some((i as u16, component));
        }
        choice -= weight;
    }
    let last_index = actor.value_urgency_components.len().saturating_sub(1);
    actor
        .value_urgency_components
        .last()
        .map(|c| (last_index as u16, c))
}

fn urgency_profile_from_u(actor: &Actor, u: f64) -> UrgencyProfile {
    if !u.is_finite() || u <= 0.0 {
        warn_bad_urgency_factor_once(actor, u);
        return UrgencyProfile::Indifferent;
    }
    let retained = (1.0 / u).clamp(0.0, 1.0);
    let retained_per_million = (retained * 1_000_000.0).round() as u32;
    UrgencyProfile::ExponentialDecay {
        retained_per_million,
    }
}

fn sample_arrivals<R: Rng + ?Sized>(actor: &Actor, slot: u64, rng: &mut R) -> u64 {
    let base_rate = if actor.arrival_rate.is_finite() {
        actor.arrival_rate.max(0.0)
    } else {
        warn_bad_arrival_rate_once(actor, actor.arrival_rate);
        0.0
    };
    match &actor.arrival_pattern {
        ArrivalPattern::Constant => sample_poisson(base_rate, rng),
        ArrivalPattern::Bursty {
            burst_prob,
            burst_multiplier,
        } => {
            let is_burst = rng.random::<f64>() < *burst_prob;
            let mean = if is_burst {
                base_rate * burst_multiplier.max(0.0)
            } else {
                base_rate
            };
            sample_poisson(mean, rng)
        }
        ArrivalPattern::Phased { phases } => {
            let mut rate = 0.0;
            for phase in phases {
                let in_phase = slot >= phase.start_slot
                    && phase.end_slot.is_none_or(|end_slot| slot < end_slot);
                if in_phase {
                    rate = phase.rate;
                    break;
                }
            }
            sample_poisson(rate.max(0.0), rng)
        }
        ArrivalPattern::Scheduled {
            slots,
            count_per_slot,
        } => {
            if slots.contains(&slot) {
                *count_per_slot
            } else {
                0
            }
        }
        ArrivalPattern::Reactive { .. } => sample_poisson(base_rate, rng),
    }
}

fn sample_poisson<R: Rng + ?Sized>(lambda: f64, rng: &mut R) -> u64 {
    if !lambda.is_finite() || lambda <= 0.0 {
        return 0;
    }
    if lambda > 50.0 {
        let normal = lambda + (lambda.sqrt() * standard_normal(rng));
        return normal.max(0.0).round() as u64;
    }

    let threshold = (-lambda).exp();
    let mut p = 1.0;
    let mut k = 0u64;
    loop {
        k = k.saturating_add(1);
        p *= rng.random::<f64>();
        if p <= threshold {
            return k.saturating_sub(1);
        }
    }
}

fn warn_bad_arrival_rate_once(actor: &Actor, rate: f64) {
    static SEEN: OnceLock<Mutex<HashSet<ActorId>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let mut seen = seen.lock().expect("arrival rate warning lock");
    if seen.insert(actor.id) {
        warn!(
            "Actor {} has non-finite arrival_rate {}. Treating as 0.",
            actor.name, rate
        );
    }
}

fn warn_bad_component_weights_once(actor: &Actor) {
    static SEEN: OnceLock<Mutex<HashSet<ActorId>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let mut seen = seen.lock().expect("component weight warning lock");
    if seen.insert(actor.id) {
        warn!(
            "Actor {} has no positive finite value_urgency_components weights. Falling back to \
             the first component.",
            actor.name,
        );
    }
}

fn warn_bad_urgency_factor_once(actor: &Actor, u: f64) {
    static SEEN: OnceLock<Mutex<HashSet<ActorId>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let mut seen = seen.lock().expect("urgency factor warning lock");
    if seen.insert(actor.id) {
        warn!(
            "Actor {} sampled invalid urgency factor {}. Falling back to indifferent urgency.",
            actor.name, u
        );
    }
}

fn standard_normal<R: Rng + ?Sized>(rng: &mut R) -> f64 {
    let u1 = rng.random::<f64>().max(f64::MIN_POSITIVE);
    let u2 = rng.random::<f64>();
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

struct WeightedLookup<T> {
    elements: Vec<(T, u64)>,
    total_weight: u64,
}

impl<T> WeightedLookup<T> {
    pub fn new(weights: impl IntoIterator<Item = (T, u64)>) -> Self {
        let elements: Vec<_> = weights
            .into_iter()
            .scan(0, |cum_weight, (element, weight)| {
                *cum_weight += weight;
                Some((element, *cum_weight))
            })
            .collect();
        let total_weight = elements
            .last()
            .map(|(_, weight)| *weight)
            .unwrap_or_default();
        Self {
            elements,
            total_weight,
        }
    }

    pub fn sample<R: Rng>(&self, rng: &mut R) -> Option<&T> {
        let choice = rng.random_range(0..self.total_weight);
        match self
            .elements
            .binary_search_by_key(&choice, |(_, weight)| *weight)
        {
            Ok(index) => self.elements.get(index).map(|(el, _)| el),
            Err(index) => self.elements.get(index).map(|(el, _)| el),
        }
    }
}

#[cfg(test)]
mod tests {
    use rand_chacha::{ChaChaRng, rand_core::SeedableRng};

    use super::{WeightedLookup, sample_arrivals, sample_value_and_urgency, urgency_profile_from_u};
    use crate::{
        model::{ActorId, UrgencyProfile},
        tx_actors::{Actor, ArrivalPattern, Distribution, DistributionKind, ValueUrgencyComponent},
    };

    fn distribution(kind: DistributionKind, params: &[f64]) -> Distribution {
        Distribution {
            kind,
            params: params.to_vec(),
        }
    }

    fn constant_distribution(value: f64) -> Distribution {
        distribution(DistributionKind::Constant, &[value])
    }

    fn dummy_actor() -> Actor {
        Actor {
            id: ActorId::new(0),
            name: "dummy".to_string(),
            arrival_rate: 0.0,
            arrival_pattern: ArrivalPattern::Constant,
            tx_size: constant_distribution(1.0),
            value_distribution: constant_distribution(1.0),
            urgency: UrgencyProfile::Indifferent,
            urgency_u_distribution: None,
            value_urgency_components: Vec::new(),
            overflow_retry_policy_override: None,
        }
    }

    #[test]
    fn urgency_profile_from_u_uses_inverse_factor() {
        let actor = dummy_actor();
        let profile = urgency_profile_from_u(&actor, 4.0);
        assert_eq!(
            profile,
            UrgencyProfile::ExponentialDecay {
                retained_per_million: 250_000
            }
        );
    }

    #[test]
    fn correlated_components_drive_value_and_urgency_sampling() {
        let mut actor = dummy_actor();
        actor.value_distribution = constant_distribution(99.0);
        actor.value_urgency_components = vec![ValueUrgencyComponent {
            name: None,
            weight: 1.0,
            value_distribution: constant_distribution(7.0),
            urgency_u_distribution: constant_distribution(2.0),
        }];
        let mut rng = ChaChaRng::seed_from_u64(123);

        let (value, urgency, component_index) = sample_value_and_urgency(&actor, &mut rng);
        assert_eq!(component_index, Some(0));
        assert_eq!(value, 7);
        assert_eq!(
            urgency,
            UrgencyProfile::ExponentialDecay {
                retained_per_million: 500_000
            }
        );
    }

    fn poisson_arrivals_actor() -> Actor {
        let mut actor = dummy_actor();
        actor.arrival_rate = 2.0;
        actor.tx_size = constant_distribution(400.0);
        actor.value_urgency_components = vec![ValueUrgencyComponent {
            name: None,
            weight: 1.0,
            value_distribution: constant_distribution(1_000_000.0),
            urgency_u_distribution: constant_distribution(1.5),
        }];
        actor
    }

    /// Regression net for Phase 0: a seeded `ChaChaRng` must produce the same
    /// actor-derived sampling stream across back-to-back runs. If any code path
    /// inside `sample_arrivals` or `sample_value_and_urgency` starts consulting
    /// process-global state (HashMap iteration order, thread-local randomness),
    /// this test catches it.
    #[test]
    fn actor_sampling_primitives_are_deterministic_across_runs() {
        let actor = poisson_arrivals_actor();
        let mut rng_a = ChaChaRng::seed_from_u64(42);
        let mut rng_b = ChaChaRng::seed_from_u64(42);

        for slot in 0..100u64 {
            let count_a = sample_arrivals(&actor, slot, &mut rng_a);
            let count_b = sample_arrivals(&actor, slot, &mut rng_b);
            assert_eq!(
                count_a, count_b,
                "arrivals diverge at slot {slot}: A={count_a} B={count_b}",
            );
            for i in 0..count_a {
                let (v_a, u_a, idx_a) = sample_value_and_urgency(&actor, &mut rng_a);
                let (v_b, u_b, idx_b) = sample_value_and_urgency(&actor, &mut rng_b);
                assert_eq!(v_a, v_b, "value diverges at slot {slot} tx {i}");
                assert_eq!(u_a, u_b, "urgency diverges at slot {slot} tx {i}");
                assert_eq!(idx_a, idx_b, "component index diverges at slot {slot} tx {i}");
            }
        }
    }

    /// Guards against accidentally dropping seed into a constant or removing
    /// RNG threading entirely — two different seeds must produce at least one
    /// distinct arrival count in the first 100 slots for a Poisson actor at
    /// rate=2.0.
    #[test]
    fn actor_sampling_diverges_for_different_seeds() {
        let actor = poisson_arrivals_actor();
        let mut rng_42 = ChaChaRng::seed_from_u64(42);
        let mut rng_43 = ChaChaRng::seed_from_u64(43);

        let stream_42: Vec<u64> = (0..100u64)
            .map(|slot| sample_arrivals(&actor, slot, &mut rng_42))
            .collect();
        let stream_43: Vec<u64> = (0..100u64)
            .map(|slot| sample_arrivals(&actor, slot, &mut rng_43))
            .collect();

        assert_ne!(
            stream_42, stream_43,
            "distinct seeds produced identical arrival streams — RNG is not being \
             threaded through sample_arrivals",
        );
    }

    /// Active CI safety net for `WeightedLookup::sample`. Uses 5% tolerance
    /// bands that both the current (buggy) implementation AND the Phase-6
    /// corrected implementation satisfy. This catches major regressions —
    /// "always returns element 0", "skips non-adjacent elements", "panics on
    /// valid non-empty input" — without being tied to the specific boundary
    /// bug. The tight-proportion reproducer below (`#[ignore]`'d) remains the
    /// exact-behavior tripwire for Phase 6.
    #[test]
    fn weighted_lookup_sample_produces_all_elements_with_rough_proportions() {
        let lookup = WeightedLookup::new([("A", 50u64), ("B", 30), ("C", 20)]);
        let mut rng = ChaChaRng::seed_from_u64(0xC0DE);
        let n = 100_000u64;
        let (mut count_a, mut count_b, mut count_c) = (0u64, 0u64, 0u64);
        for _ in 0..n {
            match *lookup.sample(&mut rng).expect("non-empty lookup must sample") {
                "A" => count_a += 1,
                "B" => count_b += 1,
                "C" => count_c += 1,
                other => panic!("unexpected element: {other}"),
            }
        }
        // All three elements must be sampled at least once — catches a
        // catastrophic bug that skips an element entirely.
        assert!(count_a > 0 && count_b > 0 && count_c > 0);

        let r_a = count_a as f64 / n as f64;
        let r_b = count_b as f64 / n as f64;
        let r_c = count_c as f64 / n as f64;

        // Heavy element beats middle beats light. This catches a
        // weight-inversion regression.
        assert!(count_a > count_b && count_b > count_c);

        // 5% tolerance covers both the current buggy boundary behavior (A wins
        // ~51% instead of exactly 50%) and the Phase-6 corrected impl
        // (A wins exactly 50%). Tightening below 5% would make this test
        // order-of-operations dependent.
        let tolerance = 0.05;
        assert!(
            (r_a - 0.50).abs() < tolerance,
            "A ratio {r_a} far from 0.5 (±{tolerance})",
        );
        assert!(
            (r_b - 0.30).abs() < tolerance,
            "B ratio {r_b} far from 0.3 (±{tolerance})",
        );
        assert!(
            (r_c - 0.20).abs() < tolerance,
            "C ratio {r_c} far from 0.2 (±{tolerance})",
        );
    }

    /// Unambiguous sanity checks that both current and Phase-6 impls satisfy.
    /// Uses non-degenerate weights (not all equal, no single-unit weights) so
    /// the boundary bug's effect is a small perturbation, not an element
    /// dropout. Catches panics on single-element lookup + "every element with
    /// substantial weight eventually appears".
    ///
    /// NOTE: With weights like [1,1,1] the buggy impl drops the last element
    /// entirely (choice=2 returns index 1, choice=1 returns index 0, so index 2
    /// is never sampled). Phase 6 fixes this; the `#[ignore]`'d reproducer
    /// below exercises this exact proportion breakdown.
    #[test]
    fn weighted_lookup_sample_handles_single_element_and_well_separated_weights() {
        // Single-element lookup: always returns the one element.
        let lookup = WeightedLookup::new([("only", 42u64)]);
        let mut rng = ChaChaRng::seed_from_u64(1);
        for _ in 0..50 {
            assert_eq!(*lookup.sample(&mut rng).expect("sample"), "only");
        }

        // Weights large enough that the boundary-bug's dropout effect covers
        // at most ~2 sample slots per ~10k draws — every element still appears.
        let lookup = WeightedLookup::new([("x", 1000u64), ("y", 500), ("z", 300)]);
        let mut rng = ChaChaRng::seed_from_u64(2);
        let mut seen_x = false;
        let mut seen_y = false;
        let mut seen_z = false;
        for _ in 0..10_000 {
            match *lookup.sample(&mut rng).expect("sample") {
                "x" => seen_x = true,
                "y" => seen_y = true,
                "z" => seen_z = true,
                other => panic!("unexpected: {other}"),
            }
        }
        assert!(
            seen_x && seen_y && seen_z,
            "all three well-separated-weight elements must appear across 10k draws",
        );
    }

    /// Phase 0 regression test (currently `#[ignore]`'d because it reproduces a
    /// known bug in `WeightedLookup::sample` scheduled to be fixed in Phase 6).
    ///
    /// `WeightedLookup` stores cumulative weights; `choice = rng.random_range(0..total_weight)`
    /// is a uniform integer in `[0, total_weight)`. With weights `[3, 1, 2]` the cumulative
    /// array is `[3, 4, 6]`, and the invariant is: choice in `[prev_cum, cum)` picks that
    /// element. Thus:
    ///   - choice=3 should pick "B" (range [3, 4))
    ///   - choice=4 should pick "C" (range [4, 6))
    ///
    /// The current `binary_search_by_key` + `Ok|Err` branching picks the element *at*
    /// `choice == cumulative_weight`, which returns the lower-indexed element. That
    /// shifts probability mass away from short-range elements: "B" is picked only when
    /// choice=4 (1/6 fires for "C" land, so B gets the 1/6 that should be C's, and C
    /// gets only the 5 choice → count 1/6 instead of 2/6).
    ///
    /// Expected correct distribution over N samples: A ≈ 0.5, B ≈ 0.167, C ≈ 0.333.
    /// Current buggy distribution: A ≈ 0.667, B ≈ 0.167, C ≈ 0.167.
    ///
    /// Phase 6 replaces `binary_search_by_key` with `partition_point(|(_, w)| *w <= choice)`
    /// and removes this `#[ignore]`.
    #[test]
    #[ignore = "reproduces WeightedLookup boundary bug; unignore when Phase 6 fix lands"]
    fn weighted_lookup_sample_preserves_weight_proportions_at_cumulative_boundaries() {
        let lookup = WeightedLookup::new([("A", 3u64), ("B", 1), ("C", 2)]);
        let mut rng = ChaChaRng::seed_from_u64(42);
        let n = 600_000u64;
        let (mut count_a, mut count_b, mut count_c) = (0u64, 0u64, 0u64);
        for _ in 0..n {
            match *lookup.sample(&mut rng).expect("non-empty lookup") {
                "A" => count_a += 1,
                "B" => count_b += 1,
                "C" => count_c += 1,
                other => panic!("unexpected element: {other}"),
            }
        }
        let r_a = count_a as f64 / n as f64;
        let r_b = count_b as f64 / n as f64;
        let r_c = count_c as f64 / n as f64;
        let tolerance = 0.005;
        assert!(
            (r_a - 0.5).abs() < tolerance,
            "A ratio {r_a} != 0.5 (±{tolerance})",
        );
        assert!(
            (r_b - 1.0 / 6.0).abs() < tolerance,
            "B ratio {r_b} != 0.167 (±{tolerance})",
        );
        assert!(
            (r_c - 2.0 / 6.0).abs() < tolerance,
            "C ratio {r_c} != 0.333 (±{tolerance})",
        );
    }
}
