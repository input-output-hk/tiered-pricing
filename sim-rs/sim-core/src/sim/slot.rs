use std::time::Duration;

use crate::{
    clock::{ClockBarrier, Timestamp},
    config::SimConfiguration,
    events::EventTracker,
};

pub struct SlotWitness {
    clock: ClockBarrier,
    tracker: EventTracker,
    slots: Option<u64>,
}

impl SlotWitness {
    pub fn new(clock: ClockBarrier, tracker: EventTracker, config: &SimConfiguration) -> Self {
        Self {
            clock,
            tracker,
            slots: config.slots,
        }
    }

    pub async fn run(&mut self) {
        let mut slot = 0;
        let mut next_slot_at = Timestamp::zero();
        loop {
            if self.slots == Some(slot) {
                return;
            }
            self.tracker.track_global_slot(slot);
            self.tracker.track_clock_diagnostics(
                slot,
                self.clock.tasks_in_flight() as u64,
                self.clock.actors_running() as u64,
                self.clock.actors_total() as u64,
                self.clock.running_actor_ids(),
                self.clock.last_task_started_by(),
                self.clock.last_task_finished_by(),
                self.clock.last_wait_actor(),
                self.clock.last_wait_until_nanos(),
                self.clock.last_woken_actor(),
                self.clock.last_advance_to_nanos(),
                self.clock.wait_queue_len(),
            );
            slot += 1;
            next_slot_at += Duration::from_secs(1);
            self.clock.wait_until(next_slot_at).await;
        }
    }
}
