use std::{cmp::Reverse, collections::HashMap, fmt::Debug, hash::Hash, time::Duration};

use anyhow::Result;
use priority_queue::PriorityQueue;
use tokio::{select, sync::mpsc};
use tracing::warn;

use crate::{
    clock::{ClockBarrier, Timestamp},
    config::NodeId,
};

use super::connection::Connection;

const NETWORK_YIELD_INTERVAL: usize = 1_024;

pub struct NetworkCoordinator<TProtocol, TMessage> {
    source: mpsc::UnboundedReceiver<Message<TProtocol, TMessage>>,
    sinks: HashMap<NodeId, mpsc::UnboundedSender<(NodeId, TMessage)>>,
    connections: HashMap<Link, Connection<TProtocol, TMessage>>,
    events: PriorityQueue<Link, (Reverse<Timestamp>, Reverse<u64>)>,
    next_seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Link {
    from: NodeId,
    to: NodeId,
}

pub struct EdgeConfig {
    pub from: NodeId,
    pub to: NodeId,
    pub latency: Duration,
    pub bandwidth_bps: Option<u64>,
}

impl<TProtocol: Clone + Eq + Hash + Ord, TMessage: Debug> NetworkCoordinator<TProtocol, TMessage> {
    pub fn new(source: mpsc::UnboundedReceiver<Message<TProtocol, TMessage>>) -> Self {
        Self {
            source,
            sinks: HashMap::new(),
            connections: HashMap::new(),
            events: PriorityQueue::new(),
            next_seq: 0,
        }
    }

    pub fn listen(&mut self, to: NodeId) -> mpsc::UnboundedReceiver<(NodeId, TMessage)> {
        let (sink, source) = mpsc::unbounded_channel();
        self.sinks.insert(to, sink);
        source
    }

    fn next_priority(&mut self, timestamp: Timestamp) -> (Reverse<Timestamp>, Reverse<u64>) {
        let seq = self.next_seq;
        self.next_seq += 1;
        (Reverse(timestamp), Reverse(seq))
    }

    pub fn add_edge(&mut self, config: EdgeConfig) {
        let link = Link {
            from: config.from,
            to: config.to,
        };
        let connection = Connection::new(config.latency, config.bandwidth_bps);
        self.connections.insert(link, connection);
    }

    pub async fn run(&mut self, clock: &mut ClockBarrier) -> Result<()> {
        let mut processed_events = 0usize;
        loop {
            let waiter = match self.events.peek() {
                Some((_, (Reverse(timestamp), _))) => clock.wait_until(*timestamp),
                None => clock.wait_forever(),
            };
            select! {
                biased;
                () = waiter => {
                    let (link, (Reverse(timestamp), _)) = self.events.pop().unwrap();
                    let now = clock.now();
                    assert!(now >= timestamp);
                    let connection = self.connections.get_mut(&link).unwrap();
                    for (body, _) in connection.recv_many(now) {
                        clock.start_task();
                        let send_result = self
                            .sinks
                            .get(&link.to)
                            .unwrap()
                            .send((link.from, body));
                        if send_result.is_err() {
                            warn!("dropping network message to closed sink {}", link.to);
                            clock.finish_task();
                        }
                        processed_events += 1;
                        if processed_events % NETWORK_YIELD_INTERVAL == 0 {
                            tokio::task::yield_now().await;
                        }
                    }
                    if let Some(timestamp) = connection.next_arrival_time() {
                        let priority = self.next_priority(timestamp);
                        self.events.push(link, priority);
                    }
                },
                Some(message) = self.source.recv() => {
                    self.schedule_message(message, clock.now());
                    clock.finish_task();
                    processed_events += 1;
                    if processed_events % NETWORK_YIELD_INTERVAL == 0 {
                        tokio::task::yield_now().await;
                    }
                }
            }
        }
    }

    fn schedule_message(&mut self, message: Message<TProtocol, TMessage>, now: Timestamp) {
        let link = Link {
            from: message.from,
            to: message.to,
        };
        let connection = self.connections.get_mut(&link).unwrap();
        connection.send(message.body, message.bytes, message.protocol, now);
        if let Some(timestamp) = connection.next_arrival_time() {
            let priority = self.next_priority(timestamp);
            self.events.push(link, priority);
        }
    }
}

pub struct Message<TProtocol, TMessage> {
    pub from: NodeId,
    pub to: NodeId,
    pub protocol: TProtocol,
    pub body: TMessage,
    pub bytes: u64,
}
