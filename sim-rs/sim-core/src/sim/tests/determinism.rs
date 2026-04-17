use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    clock::{ClockCoordinator, Timestamp},
    config::SimConfiguration,
    events::{Event, EventTracker},
    sim::Simulation,
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
