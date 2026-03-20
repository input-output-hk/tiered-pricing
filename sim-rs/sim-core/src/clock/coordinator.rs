use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::Duration,
};

use tokio::sync::{mpsc, oneshot};

use crate::clock::TaskInitiator;

use super::{ActorState, Clock, Timestamp, timestamp::AtomicTimestamp};

pub struct ClockCoordinator {
    timestamp_resolution: Duration,
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
}

impl ClockCoordinator {
    pub fn new(timestamp_resolution: Duration) -> Self {
        let time = Arc::new(AtomicTimestamp::new(Timestamp::zero()));
        let (tx, rx) = mpsc::unbounded_channel();
        let waiter_count = Arc::new(AtomicUsize::new(0));
        let tasks = Arc::new(AtomicUsize::new(0));
        let running = Arc::new(AtomicUsize::new(0));
        let actor_states = Arc::new(Mutex::new(Vec::new()));
        let last_task_started_by = Arc::new(AtomicU64::new(super::NO_ACTOR_ID));
        let last_task_finished_by = Arc::new(AtomicU64::new(super::NO_ACTOR_ID));
        let last_wait_actor = Arc::new(AtomicU64::new(super::NO_ACTOR_ID));
        let last_wait_until = Arc::new(AtomicU64::new(super::NO_TIMESTAMP));
        let last_woken_actor = Arc::new(AtomicU64::new(super::NO_ACTOR_ID));
        let last_advance_to = Arc::new(AtomicU64::new(super::NO_TIMESTAMP));
        let wait_queue_len = Arc::new(AtomicUsize::new(0));
        Self {
            timestamp_resolution,
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
        }
    }

    pub fn clock(&self) -> Clock {
        Clock::new(
            self.timestamp_resolution,
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

    pub async fn run(&mut self) {
        let mut waiters = vec![];
        for _ in 0..self.waiter_count.load(Ordering::Acquire) {
            waiters.push(None);
        }
        {
            let mut states = self.actor_states.lock().expect("actor states lock");
            if states.len() < waiters.len() {
                states.resize(waiters.len(), ActorState::Running);
            }
            for state in &mut *states {
                *state = ActorState::Running;
            }
        }

        let mut queue: BTreeMap<Timestamp, Vec<usize>> = BTreeMap::new();
        let mut running = waiters.len();
        self.running.store(running, Ordering::Release);
        while let Some(event) = self.rx.recv().await {
            match event {
                ClockEvent::Wait { actor, until, done } => {
                    assert!(until.is_none_or(|t| t >= self.time.load(Ordering::Acquire)));
                    self.last_wait_actor.store(actor as u64, Ordering::Release);
                    let wait_until = until.map(|ts| ts.as_nanos()).unwrap_or(super::NO_TIMESTAMP);
                    self.last_wait_until.store(wait_until, Ordering::Release);
                    if waiters[actor].replace(Waiter { until, done }).is_some() {
                        panic!("An actor has somehow managed to wait twice");
                    }
                    running = running.checked_sub(1).unwrap();
                    self.running.store(running, Ordering::Release);
                    if let Some(state) = self
                        .actor_states
                        .lock()
                        .expect("actor states lock")
                        .get_mut(actor)
                    {
                        *state = ActorState::Waiting;
                    }
                    if let Some(timestamp) = until {
                        queue.entry(timestamp).or_default().push(actor);
                    }
                    let queue_len = queue.values().map(|entries| entries.len()).sum::<usize>();
                    self.wait_queue_len.store(queue_len, Ordering::Release);
                    while running == 0 && self.tasks.load(Ordering::Acquire) == 0 {
                        // advance time
                        let (timestamp, waiter_ids) = queue.pop_first().unwrap();
                        self.time.store(timestamp, Ordering::Release);
                        self.last_advance_to
                            .store(timestamp.as_nanos(), Ordering::Release);

                        for id in waiter_ids {
                            if waiters[id]
                                .as_ref()
                                .and_then(|w| w.until)
                                .is_some_and(|ts| ts == timestamp)
                            {
                                self.last_woken_actor.store(id as u64, Ordering::Release);
                                running += 1;
                                self.running.store(running, Ordering::Release);
                                if let Some(state) = self
                                    .actor_states
                                    .lock()
                                    .expect("actor states lock")
                                    .get_mut(id)
                                {
                                    *state = ActorState::Running;
                                }
                                let waiter = waiters[id].take().unwrap();
                                let _ = waiter.done.send(());
                            }
                        }
                        let queue_len = queue.values().map(|entries| entries.len()).sum::<usize>();
                        self.wait_queue_len.store(queue_len, Ordering::Release);
                    }
                }
                ClockEvent::CancelWait { actor } => {
                    if waiters[actor].take().is_some() {
                        running += 1;
                        self.running.store(running, Ordering::Release);
                        if let Some(state) = self
                            .actor_states
                            .lock()
                            .expect("actor states lock")
                            .get_mut(actor)
                        {
                            *state = ActorState::Running;
                        }
                        let queue_len = queue.values().map(|entries| entries.len()).sum::<usize>();
                        self.wait_queue_len.store(queue_len, Ordering::Release);
                    }
                }
                ClockEvent::FinishTask => {
                    let prev_tasks = self.tasks.fetch_sub(1, Ordering::AcqRel);
                    assert!(prev_tasks != 0, "Finished a task that was never started");
                    assert!(
                        running != 0,
                        "All tasks were completed while there were no actors to complete them"
                    );
                }
            }
        }
    }
}

struct Waiter {
    until: Option<Timestamp>,
    done: oneshot::Sender<()>,
}

#[derive(Debug)]
pub enum ClockEvent {
    Wait {
        actor: usize,
        until: Option<Timestamp>,
        done: oneshot::Sender<()>,
    },
    CancelWait {
        actor: usize,
    },
    FinishTask,
}

#[cfg(test)]
mod tests {
    use std::{task::Poll, time::Duration};

    use futures::poll;
    use tokio::pin;

    use super::ClockCoordinator;

    const TIMESTAMP_RESOLUTION: Duration = Duration::from_nanos(1);

    #[tokio::test]
    async fn should_wait_once_all_workers_are_waiting() {
        let mut coordinator = ClockCoordinator::new(TIMESTAMP_RESOLUTION);
        let clock = coordinator.clock();
        let t0 = clock.now();
        let t1 = t0 + Duration::from_millis(5);
        let t2 = t0 + Duration::from_millis(10);
        let mut actor1 = clock.barrier();
        let mut actor2 = clock.barrier();

        let run_future = coordinator.run();
        pin!(run_future);

        let mut wait1 = actor1.wait_until(t1);
        assert_eq!(poll!(&mut wait1), Poll::Pending); // the wait is pending
        assert_eq!(poll!(&mut run_future), Poll::Pending); // try advancing time
        assert_eq!(clock.now(), t0); // no time has passed
        assert_eq!(poll!(&mut wait1), Poll::Pending); // the 5ms wait is still pending, because clock 2 isn't finished

        let mut wait2 = actor2.wait_until(t2);
        assert_eq!(poll!(&mut wait2), Poll::Pending);
        assert_eq!(poll!(&mut run_future), Poll::Pending); // try advancing time
        assert_eq!(clock.now(), t1); // 5ms have passed
        assert_eq!(poll!(&mut wait2), Poll::Pending); // the 10ms wait is still pending
        assert_eq!(poll!(wait1), Poll::Ready(())); // the 5ms wait is done
    }

    #[tokio::test]
    async fn should_cancel_wait_when_wait_future_is_dropped() {
        let mut coordinator = ClockCoordinator::new(TIMESTAMP_RESOLUTION);
        let clock = coordinator.clock();
        let t0 = clock.now();
        let t1 = t0 + Duration::from_millis(5);
        let mut actor1 = clock.barrier();
        let mut actor2 = clock.barrier();

        let run_future = coordinator.run();
        pin!(run_future);

        {
            let wait1 = actor1.wait_until(t1);
            assert_eq!(poll!(wait1), Poll::Pending); // the wait is pending
            // and now it goes out of scope and gets dropped
        }
        assert_eq!(poll!(&mut run_future), Poll::Pending); // try advancing time
        assert_eq!(poll!(&mut run_future), Poll::Pending); // try advancing time
        assert_eq!(clock.now(), t0); // no time has passed

        let mut wait2 = actor2.wait_until(t1);
        assert_eq!(poll!(&mut wait2), Poll::Pending);
        assert_eq!(poll!(&mut run_future), Poll::Pending); // try advancing time
        assert_eq!(clock.now(), t0); // no time has passed
        assert_eq!(poll!(&mut wait2), Poll::Pending); // the remaining wait is still pending
    }

    #[tokio::test]
    async fn should_avoid_race_condition() {
        let mut coordinator = ClockCoordinator::new(TIMESTAMP_RESOLUTION);
        let clock = coordinator.clock();
        let t0 = clock.now();
        let t1 = t0 + Duration::from_millis(5);
        let t2 = t0 + Duration::from_millis(10);
        let mut actor1 = clock.barrier();
        let mut actor2 = clock.barrier();

        let run_future = coordinator.run();
        pin!(run_future);

        // make actor 1 wait for a short time, then cancel it, then wait for a long time
        {
            let wait1 = actor1.wait_until(t1);
            assert_eq!(poll!(wait1), Poll::Pending);
        }
        let mut wait1 = actor1.wait_until(t2);
        assert_eq!(poll!(&mut wait1), Poll::Pending);
        assert_eq!(poll!(&mut run_future), Poll::Pending);
        assert_eq!(clock.now(), t0); // no time has passed
        assert_eq!(poll!(&mut wait1), Poll::Pending);

        let wait2 = actor2.wait_until(t2);
        assert_eq!(poll!(wait2), Poll::Pending);
        while let Poll::Pending = poll!(&mut wait1) {
            assert_eq!(poll!(&mut run_future), Poll::Pending);
        }
        // We expect a long time to have passed, because the "short" wait was cancelled
        assert_eq!(clock.now(), t2);
    }

    #[tokio::test]
    async fn should_allow_time_to_stand_still() {
        let mut coordinator = ClockCoordinator::new(TIMESTAMP_RESOLUTION);
        let clock = coordinator.clock();
        let t0 = clock.now();
        let t1 = t0 + Duration::from_millis(5);
        let t2 = t0 + Duration::from_millis(10);
        let mut actor = clock.barrier();

        let run_future = coordinator.run();
        pin!(run_future);

        // The actor waits until t1, then cancels that wait,
        // before the coordinator has a chance to run
        {
            let wait1 = actor.wait_until(t1);
            assert_eq!(poll!(wait1), Poll::Pending);
        }

        // The actor should be able to wait until t1 without issue,
        // even though it has already cancelled a wait for t1.
        let mut wait1 = actor.wait_until(t1);
        assert_eq!(poll!(&mut wait1), Poll::Pending);
        assert_eq!(poll!(&mut run_future), Poll::Pending);
        assert_eq!(poll!(&mut wait1), Poll::Ready(()));
        drop(wait1);

        // Test waiting for another few moments just for good measure
        let mut wait2 = actor.wait_until(t2);
        assert_eq!(poll!(&mut wait2), Poll::Pending);
        assert_eq!(poll!(&mut run_future), Poll::Pending);
        assert_eq!(poll!(&mut wait2), Poll::Ready(()));
    }

    #[tokio::test]
    async fn should_allow_waiting_forever() {
        let mut coordinator = ClockCoordinator::new(TIMESTAMP_RESOLUTION);
        let clock = coordinator.clock();
        let t0 = clock.now();
        let t1 = t0 + Duration::from_millis(5);
        let mut actor1 = clock.barrier();
        let mut actor2 = clock.barrier();

        let run_future = coordinator.run();
        pin!(run_future);

        let mut wait1 = actor1.wait_until(t1);
        assert_eq!(poll!(&mut wait1), Poll::Pending); // the wait is pending
        assert_eq!(poll!(&mut run_future), Poll::Pending); // try advancing time
        assert_eq!(clock.now(), t0); // no time has passed
        assert_eq!(poll!(&mut wait1), Poll::Pending); // the 5ms wait is still pending, because clock 2 isn't finished

        let mut wait2 = actor2.wait_forever();
        assert_eq!(poll!(&mut wait2), Poll::Pending);
        assert_eq!(poll!(&mut run_future), Poll::Pending); // try advancing time
        assert_eq!(clock.now(), t1); // 5ms have passed
        assert_eq!(poll!(&mut wait2), Poll::Pending); // the eternal wait is still pending
        assert_eq!(poll!(wait1), Poll::Ready(())); // the 5ms wait is done
    }
}
