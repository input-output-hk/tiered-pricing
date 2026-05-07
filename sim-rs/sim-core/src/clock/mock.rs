use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, AtomicUsize},
    },
    time::Duration,
};

use tokio::sync::{mpsc, oneshot};

use crate::clock::{
    ActorState, Clock, NO_ACTOR_ID, NO_TIMESTAMP, TaskInitiator, Timestamp,
    coordinator::ClockEvent, timestamp::AtomicTimestamp,
};

pub struct MockClockCoordinator {
    time: Arc<AtomicTimestamp>,
    tx: mpsc::UnboundedSender<ClockEvent>,
    rx: mpsc::UnboundedReceiver<ClockEvent>,
    waiter_count: Arc<AtomicUsize>,
    tasks: Arc<AtomicUsize>,
    running: Arc<AtomicUsize>,
    actor_states: Arc<Mutex<Vec<ActorState>>>,
    last_task_started_by: Arc<AtomicU64>,
    last_task_finished_by: Arc<AtomicU64>,
    last_wait_actor: Arc<AtomicU64>,
    last_wait_until: Arc<AtomicU64>,
    last_woken_actor: Arc<AtomicU64>,
    last_advance_to: Arc<AtomicU64>,
    wait_queue_len: Arc<AtomicUsize>,
    waiters: BTreeMap<usize, Waiter>,
}

impl Default for MockClockCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl MockClockCoordinator {
    pub fn new() -> Self {
        let time = Arc::new(AtomicTimestamp::new(Timestamp::zero()));
        let (tx, rx) = mpsc::unbounded_channel();
        let waiter_count = Arc::new(AtomicUsize::new(0));
        let tasks = Arc::new(AtomicUsize::new(0));
        let running = Arc::new(AtomicUsize::new(0));
        let actor_states = Arc::new(Mutex::new(Vec::new()));
        let last_task_started_by = Arc::new(AtomicU64::new(NO_ACTOR_ID));
        let last_task_finished_by = Arc::new(AtomicU64::new(NO_ACTOR_ID));
        let last_wait_actor = Arc::new(AtomicU64::new(NO_ACTOR_ID));
        let last_wait_until = Arc::new(AtomicU64::new(NO_TIMESTAMP));
        let last_woken_actor = Arc::new(AtomicU64::new(NO_ACTOR_ID));
        let last_advance_to = Arc::new(AtomicU64::new(NO_TIMESTAMP));
        let wait_queue_len = Arc::new(AtomicUsize::new(0));
        Self {
            time,
            tx,
            rx,
            waiter_count,
            tasks,
            running,
            actor_states,
            last_task_started_by,
            last_task_finished_by,
            last_wait_actor,
            last_wait_until,
            last_woken_actor,
            last_advance_to,
            wait_queue_len,
            waiters: BTreeMap::new(),
        }
    }

    pub fn clock(&self) -> Clock {
        Clock::new(
            Duration::from_nanos(1),
            self.time.clone(),
            self.waiter_count.clone(),
            TaskInitiator::new(self.tasks.clone()),
            self.running.clone(),
            self.actor_states.clone(),
            self.last_task_started_by.clone(),
            self.last_task_finished_by.clone(),
            self.last_wait_actor.clone(),
            self.last_wait_until.clone(),
            self.last_woken_actor.clone(),
            self.last_advance_to.clone(),
            self.wait_queue_len.clone(),
            self.tx.clone(),
        )
    }

    pub fn now(&self) -> Timestamp {
        self.time.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn advance_time(&mut self, until: Timestamp) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                ClockEvent::Wait { actor, until, done } => {
                    self.last_wait_actor
                        .store(actor as u64, std::sync::atomic::Ordering::Release);
                    let wait_until = until.map(|ts| ts.as_nanos()).unwrap_or(NO_TIMESTAMP);
                    self.last_wait_until
                        .store(wait_until, std::sync::atomic::Ordering::Release);
                    if self.waiters.insert(actor, Waiter { until, done }).is_some() {
                        panic!("waiter {actor} waited twice");
                    }
                    self.wait_queue_len.store(
                        self.waiters.values().filter(|w| w.until.is_some()).count(),
                        std::sync::atomic::Ordering::Release,
                    );
                }
                ClockEvent::CancelWait { actor } => {
                    if self.waiters.remove(&actor).is_none() {
                        panic!("waiter {actor} cancelled a wait twice");
                    }
                    self.wait_queue_len.store(
                        self.waiters.values().filter(|w| w.until.is_some()).count(),
                        std::sync::atomic::Ordering::Release,
                    );
                }
                ClockEvent::FinishTask => {
                    if self.tasks.fetch_sub(1, std::sync::atomic::Ordering::AcqRel) == 0 {
                        panic!("cancelled too many tasks");
                    }
                }
            }
        }
        assert_eq!(
            self.waiters.len(),
            self.waiter_count.load(std::sync::atomic::Ordering::Acquire),
            "not every worker is waiting for time to pass"
        );

        self.time.store(until, std::sync::atomic::Ordering::Release);
        self.last_advance_to
            .store(until.as_nanos(), std::sync::atomic::Ordering::Release);
        self.waiters = std::mem::take(&mut self.waiters)
            .into_iter()
            .filter_map(|(actor, waiter)| {
                if let Some(t) = &waiter.until {
                    if *t < until {
                        panic!("advanced time too far (waited for {until:?}, next event at {t:?})");
                    }
                    if *t == until {
                        self.last_woken_actor
                            .store(actor as u64, std::sync::atomic::Ordering::Release);
                        let _ = waiter.done.send(());
                        return None;
                    }
                }
                Some((actor, waiter))
            })
            .collect();
    }
}

struct Waiter {
    until: Option<Timestamp>,
    done: oneshot::Sender<()>,
}
