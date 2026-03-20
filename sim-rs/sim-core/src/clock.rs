use std::{
    cmp::Reverse,
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, AtomicUsize},
    },
    task::{Context, Poll},
    time::Duration,
};

pub use coordinator::ClockCoordinator;
use coordinator::ClockEvent;
use futures::FutureExt;
pub use mock::MockClockCoordinator;
use timestamp::AtomicTimestamp;
pub use timestamp::Timestamp;
use tokio::sync::{mpsc, oneshot};

mod coordinator;
mod mock;
mod timestamp;

// wrapper struct which holds a SimulationEvent,
// but is ordered by a timestamp (in reverse)
#[derive(Clone)]
pub(crate) struct FutureEvent<T>(pub Timestamp, pub T);
impl<T> FutureEvent<T> {
    fn key(&self) -> Reverse<Timestamp> {
        Reverse(self.0)
    }
}

impl<T> PartialEq for FutureEvent<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl<T> Eq for FutureEvent<T> {}

impl<T> PartialOrd for FutureEvent<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for FutureEvent<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key().cmp(&other.key())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActorState {
    Running,
    Waiting,
}

pub(crate) const NO_ACTOR_ID: u64 = u64::MAX;
pub(crate) const NO_TIMESTAMP: u64 = u64::MAX;

#[derive(Clone)]
pub struct Clock {
    timestamp_resolution: Duration,
    time: Arc<AtomicTimestamp>,
    waiters: Arc<AtomicUsize>,
    tasks: TaskInitiator,
    running: Arc<AtomicUsize>,
    actor_states: Arc<Mutex<Vec<ActorState>>>,
    last_task_started_by: Arc<AtomicU64>,
    last_task_finished_by: Arc<AtomicU64>,
    last_wait_actor: Arc<AtomicU64>,
    last_wait_until: Arc<AtomicU64>,
    last_woken_actor: Arc<AtomicU64>,
    last_advance_to: Arc<AtomicU64>,
    wait_queue_len: Arc<AtomicUsize>,
    tx: mpsc::UnboundedSender<ClockEvent>,
}

impl Clock {
    fn new(
        timestamp_resolution: Duration,
        time: Arc<AtomicTimestamp>,
        waiters: Arc<AtomicUsize>,
        tasks: TaskInitiator,
        running: Arc<AtomicUsize>,
        actor_states: Arc<Mutex<Vec<ActorState>>>,
        last_task_started_by: Arc<AtomicU64>,
        last_task_finished_by: Arc<AtomicU64>,
        last_wait_actor: Arc<AtomicU64>,
        last_wait_until: Arc<AtomicU64>,
        last_woken_actor: Arc<AtomicU64>,
        last_advance_to: Arc<AtomicU64>,
        wait_queue_len: Arc<AtomicUsize>,
        tx: mpsc::UnboundedSender<ClockEvent>,
    ) -> Self {
        Self {
            timestamp_resolution,
            time,
            waiters,
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
            tx,
        }
    }

    pub fn now(&self) -> Timestamp {
        self.time.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn barrier(&self) -> ClockBarrier {
        let id = self
            .waiters
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        {
            let mut states = self.actor_states.lock().expect("actor states lock");
            if states.len() <= id {
                states.resize(id + 1, ActorState::Running);
            }
            states[id] = ActorState::Running;
        }
        ClockBarrier {
            id,
            timestamp_resolution: self.timestamp_resolution,
            time: self.time.clone(),
            waiters: self.waiters.clone(),
            tasks: self.tasks.clone(),
            running: self.running.clone(),
            actor_states: self.actor_states.clone(),
            last_task_started_by: self.last_task_started_by.clone(),
            last_task_finished_by: self.last_task_finished_by.clone(),
            last_wait_actor: self.last_wait_actor.clone(),
            last_wait_until: self.last_wait_until.clone(),
            last_woken_actor: self.last_woken_actor.clone(),
            last_advance_to: self.last_advance_to.clone(),
            wait_queue_len: self.wait_queue_len.clone(),
            tx: self.tx.clone(),
        }
    }
}

pub struct ClockBarrier {
    id: usize,
    timestamp_resolution: Duration,
    time: Arc<AtomicTimestamp>,
    waiters: Arc<AtomicUsize>,
    tasks: TaskInitiator,
    running: Arc<AtomicUsize>,
    actor_states: Arc<Mutex<Vec<ActorState>>>,
    last_task_started_by: Arc<AtomicU64>,
    last_task_finished_by: Arc<AtomicU64>,
    last_wait_actor: Arc<AtomicU64>,
    last_wait_until: Arc<AtomicU64>,
    last_woken_actor: Arc<AtomicU64>,
    last_advance_to: Arc<AtomicU64>,
    wait_queue_len: Arc<AtomicUsize>,
    tx: mpsc::UnboundedSender<ClockEvent>,
}

impl ClockBarrier {
    pub fn id(&self) -> u64 {
        self.id as u64
    }

    pub fn now(&self) -> Timestamp {
        self.time.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn start_task(&self) {
        self.tasks.start_task();
        self.last_task_started_by
            .store(self.id as u64, std::sync::atomic::Ordering::Release);
    }

    pub fn finish_task(&self) {
        self.last_task_finished_by
            .store(self.id as u64, std::sync::atomic::Ordering::Release);
        let _ = self.tx.send(ClockEvent::FinishTask);
    }

    pub fn task_initiator(&self) -> TaskInitiator {
        self.tasks.clone()
    }

    pub fn tasks_in_flight(&self) -> usize {
        self.tasks.tasks_in_flight()
    }

    pub fn actors_total(&self) -> usize {
        self.waiters.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn actors_running(&self) -> usize {
        self.running.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn running_actor_ids(&self) -> Vec<u64> {
        let states = self.actor_states.lock().expect("actor states lock");
        states
            .iter()
            .enumerate()
            .filter_map(|(id, state)| (*state == ActorState::Running).then_some(id as u64))
            .collect()
    }

    pub fn last_task_started_by(&self) -> Option<u64> {
        match self
            .last_task_started_by
            .load(std::sync::atomic::Ordering::Acquire)
        {
            NO_ACTOR_ID => None,
            id => Some(id),
        }
    }

    pub fn last_task_finished_by(&self) -> Option<u64> {
        match self
            .last_task_finished_by
            .load(std::sync::atomic::Ordering::Acquire)
        {
            NO_ACTOR_ID => None,
            id => Some(id),
        }
    }

    pub fn last_wait_actor(&self) -> Option<u64> {
        match self
            .last_wait_actor
            .load(std::sync::atomic::Ordering::Acquire)
        {
            NO_ACTOR_ID => None,
            id => Some(id),
        }
    }

    pub fn last_wait_until_nanos(&self) -> Option<u64> {
        match self
            .last_wait_until
            .load(std::sync::atomic::Ordering::Acquire)
        {
            NO_TIMESTAMP => None,
            ts => Some(ts),
        }
    }

    pub fn last_woken_actor(&self) -> Option<u64> {
        match self
            .last_woken_actor
            .load(std::sync::atomic::Ordering::Acquire)
        {
            NO_ACTOR_ID => None,
            id => Some(id),
        }
    }

    pub fn last_advance_to_nanos(&self) -> Option<u64> {
        match self
            .last_advance_to
            .load(std::sync::atomic::Ordering::Acquire)
        {
            NO_TIMESTAMP => None,
            ts => Some(ts),
        }
    }

    pub fn wait_queue_len(&self) -> u64 {
        self.wait_queue_len
            .load(std::sync::atomic::Ordering::Acquire) as u64
    }

    pub fn wait_until(&mut self, timestamp: Timestamp) -> Waiter<'_> {
        self.wait(Some(timestamp.with_resolution(self.timestamp_resolution)))
    }

    pub fn wait_forever(&mut self) -> Waiter<'_> {
        self.wait(None)
    }

    fn wait(&mut self, until: Option<Timestamp>) -> Waiter<'_> {
        let (tx, rx) = oneshot::channel();
        let done = until.is_some_and(|ts| ts == self.now())
            || self
                .tx
                .send(ClockEvent::Wait {
                    actor: self.id,
                    until,
                    done: tx,
                })
                .is_err();

        Waiter {
            id: self.id,
            tx: &self.tx,
            rx,
            done,
        }
    }
}

#[derive(Clone)]
pub struct TaskInitiator {
    tasks: Arc<AtomicUsize>,
}

impl TaskInitiator {
    pub fn new(tasks: Arc<AtomicUsize>) -> Self {
        Self { tasks }
    }
    pub fn start_task(&self) {
        self.tasks.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
    }

    pub fn tasks_in_flight(&self) -> usize {
        self.tasks.load(std::sync::atomic::Ordering::Acquire)
    }
}

pub struct Waiter<'a> {
    id: usize,
    tx: &'a mpsc::UnboundedSender<ClockEvent>,
    rx: oneshot::Receiver<()>,
    done: bool,
}

impl Future for Waiter<'_> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.done {
            return Poll::Ready(());
        }
        match self.rx.poll_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                if result.is_ok() {
                    self.done = true;
                }
                Poll::Ready(())
            }
        }
    }
}

impl Drop for Waiter<'_> {
    fn drop(&mut self) {
        if !self.done {
            let _ = self.tx.send(ClockEvent::CancelWait { actor: self.id });
        }
    }
}
