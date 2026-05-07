use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    clock::{ClockCoordinator, Timestamp},
    config::{SimConfiguration, TxGenerator},
    events::{Event, EventTracker},
    sim::Simulation,
    tx_actors::ActorsFile,
    tx_pricing::PricingFile,
};

/// Run a full simulation for the given config and collect all emitted events.
async fn run_collecting_events(config: SimConfiguration) -> Vec<String> {
    let (events_tx, mut events_rx) = mpsc::unbounded_channel::<(Event, Timestamp)>();
    let token = CancellationToken::new();

    let clock_coordinator = ClockCoordinator::new(config.timestamp_resolution);
    let clock = clock_coordinator.clock();
    let tracker = EventTracker::new(events_tx, clock.clone(), &config.nodes);
    let mut simulation = Simulation::new(config, tracker, clock_coordinator)
        .await
        .expect("failed to create simulation");

    simulation.run(token).await.expect("simulation failed");
    simulation.shutdown().expect("shutdown failed");

    // Drain the event channel into debug-formatted strings for comparison.
    let mut events = Vec::new();
    while let Ok((event, timestamp)) = events_rx.try_recv() {
        events.push(format!("{timestamp:?}|{event:?}"));
    }
    events
}

fn make_test_config() -> SimConfiguration {
    let mut params: crate::config::RawParameters =
        serde_yaml::from_slice(include_bytes!("../../../../parameters/config.default.yaml"))
            .unwrap();
    params.leios_variant = crate::config::LeiosVariant::LinearWithTxReferences;
    params.seed = 42;
    params.simulate_transactions = true;

    let topology: crate::config::RawTopology =
        serde_yaml::from_str(include_str!("../../../../parameters/topology.default.yaml")).unwrap();

    let mut config = SimConfiguration::build(params, topology.into()).unwrap();
    config.slots = Some(50);
    config
}

/// Minimal actor demand used by the tiered/EIP determinism tests. Pinned inline so the
/// test doesn't depend on files outside the crate and is trivial to reason about.
///
/// 2 txs/slot × 400 bytes = 800 bytes/slot — enough to exercise pricing mechanics
/// without overwhelming a 150-slot run.
const MINIMAL_DEMAND_TOML: &str = r#"
[[actors]]
name = "default"
arrival_rate = 2.0
tx_size = { kind = "constant", params = [400] }
value_distribution = { kind = "constant", params = [10000000] }
urgency = { kind = "indifferent" }
"#;

/// Build a `SimConfiguration` with a specific pricing mechanism injected post-build.
///
/// `SimConfiguration::build` bails when `tx_generator == Actors` and `pricing.is_none()`,
/// so we build via the legacy path (pricing ignored), then swap the generator and
/// inject pricing + actors directly. `pricing`, `actors`, and `tx_generator` are
/// `pub(crate)` on `SimConfiguration`, which we can reach from this tests module.
fn make_test_config_with_pricing(pricing_toml: &str, slots: u64, seed: u64) -> SimConfiguration {
    let mut params: crate::config::RawParameters =
        serde_yaml::from_slice(include_bytes!("../../../../parameters/config.default.yaml"))
            .unwrap();
    params.leios_variant = crate::config::LeiosVariant::LinearWithTxReferences;
    params.seed = seed;
    params.simulate_transactions = true;

    let topology: crate::config::RawTopology =
        serde_yaml::from_str(include_str!("../../../../parameters/topology.default.yaml")).unwrap();

    let mut config = SimConfiguration::build(params, topology.into()).unwrap();
    config.slots = Some(slots);

    let mut pricing = toml::from_str::<PricingFile>(pricing_toml)
        .expect("pricing toml should parse")
        .pricing_mechanism;
    // Apply the exact same normalization + validation the production loader runs
    // in `config::load_pricing_config`. This ensures determinism tests exercise
    // the production EB-pool shape (e.g. `ContinuousRbEb*` policies get
    // `eb_total_capacity` set from the raw params, not left at `None`).
    pricing
        .normalize_and_validate(config.max_eb_size)
        .expect("normalized pricing config must be valid");
    let actors = toml::from_str::<ActorsFile>(MINIMAL_DEMAND_TOML)
        .expect("demand toml should parse")
        .actors;

    config.tx_generator = TxGenerator::Actors;
    config.pricing = Some(pricing);
    config.actors = Some(actors);
    config
}

async fn assert_deterministic_for_pricing(pricing_toml: &str, label: &str) {
    // Two back-to-back runs with the same seed must produce identical event streams.
    let events_a = run_collecting_events(make_test_config_with_pricing(pricing_toml, 150, 42)).await;
    let events_b = run_collecting_events(make_test_config_with_pricing(pricing_toml, 150, 42)).await;

    assert!(
        !events_a.is_empty(),
        "[{label}] run A produced no events — test is vacuous",
    );
    assert_eq!(
        events_a.len(),
        events_b.len(),
        "[{label}] event count differs: A={}, B={}",
        events_a.len(),
        events_b.len(),
    );
    for (i, (a, b)) in events_a.iter().zip(events_b.iter()).enumerate() {
        assert_eq!(a, b, "[{label}] events diverge at index {i}");
    }

    // Different seed must produce a different (non-vacuous) event stream, but of the
    // same broad structure. This catches accidental seed-independence bugs.
    let events_c = run_collecting_events(make_test_config_with_pricing(pricing_toml, 150, 43)).await;
    assert!(
        !events_c.is_empty(),
        "[{label}] distinct-seed run produced no events — test is vacuous",
    );
    assert_ne!(
        events_a, events_c,
        "[{label}] runs with different seeds produced identical event streams \
         (randomness is not threaded through the pricing path)",
    );
}

#[tokio::test]
async fn simulation_is_deterministic_across_runs() {
    let config_a = make_test_config();
    let config_b = make_test_config();

    let events_a = run_collecting_events(config_a).await;
    let events_b = run_collecting_events(config_b).await;

    assert!(
        !events_a.is_empty(),
        "run A produced no events — test is vacuous"
    );
    assert_eq!(
        events_a.len(),
        events_b.len(),
        "event count differs: run A produced {} events, run B produced {}",
        events_a.len(),
        events_b.len(),
    );
    for (i, (a, b)) in events_a.iter().zip(events_b.iter()).enumerate() {
        assert_eq!(a, b, "events diverge at index {i}");
    }
}

const BASELINE_TOML: &str = include_str!(
    "../../../../parameters/phase-2-sweep/pricing/baseline_quick.toml"
);
const EIP1559_TOML: &str = include_str!(
    "../../../../parameters/phase-2-sweep/pricing/eip1559_quick.toml"
);
const EIP1559_PRIORITY_LANE_TOML: &str = include_str!(
    "../../../../parameters/phase-2-sweep/pricing/eip1559_priority_lane_x5_cap25_quick.toml"
);
const TIERED_SHARED_TOML: &str = include_str!(
    "../../../../parameters/phase-2-sweep/pricing/tiered_shared_single_pool_quick.toml"
);
const TIERED_NAIVE_RB_EB_TOML: &str = include_str!(
    "../../../../parameters/phase-2-sweep/pricing/naive_rb_eb_two_tier_quick.toml"
);
const TIERED_RB_TIER0_RESERVED_TOML: &str = include_str!(
    "../../../../parameters/phase-2-sweep/pricing/rb_tier0_reserved_30pct_quick.toml"
);
const TIERED_CONTINUOUS_RB_EB_TOML: &str = include_str!(
    "../../../../parameters/phase-2-sweep/pricing/continuous_rb_eb_quick.toml"
);
const TIERED_CONTINUOUS_RB_EB_FALLBACK_TOML: &str = include_str!(
    "../../../../parameters/phase-2-sweep/pricing/continuous_rb_eb_fallback_quick.toml"
);

#[tokio::test]
async fn baseline_pricing_is_deterministic() {
    assert_deterministic_for_pricing(BASELINE_TOML, "baseline").await;
}

#[tokio::test]
async fn eip1559_pricing_is_deterministic() {
    assert_deterministic_for_pricing(EIP1559_TOML, "eip1559").await;
}

#[tokio::test]
async fn eip1559_priority_lane_is_deterministic() {
    assert_deterministic_for_pricing(EIP1559_PRIORITY_LANE_TOML, "eip1559_priority_lane").await;
}

#[tokio::test]
async fn tiered_shared_policy_is_deterministic() {
    assert_deterministic_for_pricing(TIERED_SHARED_TOML, "tiered_shared").await;
}

#[tokio::test]
async fn tiered_naive_rb_eb_policy_is_deterministic() {
    assert_deterministic_for_pricing(TIERED_NAIVE_RB_EB_TOML, "tiered_naive_rb_eb").await;
}

#[tokio::test]
async fn tiered_rb_tier0_reserved_policy_is_deterministic() {
    assert_deterministic_for_pricing(TIERED_RB_TIER0_RESERVED_TOML, "tiered_rb_tier0_reserved")
        .await;
}

#[tokio::test]
async fn tiered_continuous_rb_eb_policy_is_deterministic() {
    assert_deterministic_for_pricing(TIERED_CONTINUOUS_RB_EB_TOML, "tiered_continuous_rb_eb").await;
}

#[tokio::test]
async fn tiered_continuous_rb_eb_fallback_policy_is_deterministic() {
    assert_deterministic_for_pricing(
        TIERED_CONTINUOUS_RB_EB_FALLBACK_TOML,
        "tiered_continuous_rb_eb_fallback",
    )
    .await;
}
