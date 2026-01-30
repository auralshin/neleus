use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageKind {
    Data,
    Event,
    Command,
    System,
}

impl MessageKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageKind::Data => "data",
            MessageKind::Event => "event",
            MessageKind::Command => "command",
            MessageKind::System => "system",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Topic(pub String);

impl Topic {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn market_data(instrument: &str) -> Self {
        Self(format!("data.market.{}", instrument))
    }

    pub fn trades(instrument: &str) -> Self {
        Self(format!("data.trades.{}", instrument))
    }

    pub fn orderbook(instrument: &str) -> Self {
        Self(format!("data.book.{}", instrument))
    }

    pub fn order_events() -> Self {
        Self("events.orders".to_string())
    }

    pub fn position_events() -> Self {
        Self("events.positions".to_string())
    }

    pub fn fill_events() -> Self {
        Self("events.fills".to_string())
    }

    pub fn strategy(strategy_id: &str) -> Self {
        Self(format!("strategy.{}", strategy_id))
    }

    pub fn commands() -> Self {
        Self("commands".to_string())
    }

    pub fn system() -> Self {
        Self("system".to_string())
    }

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[inline]
    pub fn matches(&self, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        if pattern.ends_with(".*") {
            let prefix = &pattern[..pattern.len() - 2];
            return self.0.starts_with(prefix);
        }
        self.0 == pattern
    }
}

impl From<&str> for Topic {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Priority {
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: u64,

    pub kind: MessageKind,

    pub topic: Topic,

    pub payload: Vec<u8>,

    pub priority: Priority,

    pub timestamp: u64,

    pub correlation_id: Option<u64>,

    pub reply_to: Option<Topic>,
}

impl Message {
    #[inline(always)]
    pub fn new(kind: MessageKind, topic: Topic, payload: Vec<u8>) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self {
            id: COUNTER.fetch_add(1, Ordering::Relaxed),
            kind,
            topic,
            payload,
            priority: Priority::Normal,
            timestamp: current_time_nanos(),
            correlation_id: None,
            reply_to: None,
        }
    }

    #[inline(always)]
    pub fn data(topic: Topic, payload: Vec<u8>) -> Self {
        Self::new(MessageKind::Data, topic, payload)
    }

    #[inline(always)]
    pub fn event(topic: Topic, payload: Vec<u8>) -> Self {
        Self::new(MessageKind::Event, topic, payload)
    }

    #[inline(always)]
    pub fn command(topic: Topic, payload: Vec<u8>) -> Self {
        Self::new(MessageKind::Command, topic, payload)
    }

    #[inline]
    pub fn system(payload: Vec<u8>) -> Self {
        Self::new(MessageKind::System, Topic::system(), payload)
    }

    #[inline]
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    #[inline]
    pub fn with_correlation(mut self, correlation_id: u64) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    #[inline]
    pub fn with_reply_to(mut self, topic: Topic) -> Self {
        self.reply_to = Some(topic);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriberId(pub u64);

impl SubscriberId {
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for SubscriberId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Subscription {
    pub id: SubscriberId,
    pub pattern: String,
    pub kinds: Vec<MessageKind>,
}

impl Subscription {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            id: SubscriberId::new(),
            pattern: pattern.into(),
            kinds: vec![MessageKind::Data, MessageKind::Event, MessageKind::Command],
        }
    }

    pub fn with_kinds(mut self, kinds: Vec<MessageKind>) -> Self {
        self.kinds = kinds;
        self
    }

    #[inline]
    pub fn matches(&self, message: &Message) -> bool {
        self.kinds.contains(&message.kind) && message.topic.matches(&self.pattern)
    }
}

pub trait EventSink: Send + Sync {
    fn append(&self, message: &Message);
}

pub trait Bus {
    fn publish(&mut self, message: Message);

    fn poll(&mut self) -> Option<Message>;

    fn subscribe(&mut self, subscription: Subscription);

    fn unsubscribe(&mut self, subscriber_id: SubscriberId);

    fn pending_count(&self) -> usize;

    fn stats(&self) -> BusStats {
        BusStats::default()
    }

    fn is_empty(&self) -> bool {
        self.pending_count() == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPolicy {
    DropOldest,

    DropNewest,

    NeverDrop,
}

#[derive(Clone)]
pub struct BusConfig {
    pub max_queue_size: usize,

    pub drop_policy: DropPolicy,

    pub enable_logging: bool,

    pub event_sink: Option<Arc<dyn EventSink>>,
}

impl std::fmt::Debug for BusConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BusConfig")
            .field("max_queue_size", &self.max_queue_size)
            .field("drop_policy", &self.drop_policy)
            .field("enable_logging", &self.enable_logging)
            .field("event_sink", &self.event_sink.is_some())
            .finish()
    }
}

impl Default for BusConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 100_000,
            drop_policy: DropPolicy::DropOldest,
            enable_logging: false,
            event_sink: None,
        }
    }
}

pub struct InMemoryBus {
    queues: [VecDeque<Message>; 4],

    subscriptions: HashMap<SubscriberId, Subscription>,

    config: BusConfig,

    event_log: Vec<Message>,

    stats: BusStats,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct BusStats {
    pub messages_published: u64,
    pub messages_delivered: u64,
    pub messages_dropped: u64,
}

impl InMemoryBus {
    pub fn new() -> Self {
        Self::with_config(BusConfig::default())
    }

    pub fn with_config(config: BusConfig) -> Self {
        Self {
            queues: [
                VecDeque::new(),
                VecDeque::new(),
                VecDeque::new(),
                VecDeque::new(),
            ],
            subscriptions: HashMap::new(),
            config,
            event_log: Vec::new(),
            stats: BusStats::default(),
        }
    }

    pub fn stats(&self) -> &BusStats {
        &self.stats
    }

    pub fn event_log(&self) -> &[Message] {
        &self.event_log
    }

    pub fn clear_event_log(&mut self) {
        self.event_log.clear();
    }

    fn queue_index(priority: Priority) -> usize {
        priority as usize
    }

    fn enqueue(&mut self, message: Message) {
        let idx = Self::queue_index(message.priority);
        let queue = &mut self.queues[idx];

        if queue.len() >= self.config.max_queue_size {
            match self.config.drop_policy {
                DropPolicy::DropOldest => {
                    queue.pop_front();
                    self.stats.messages_dropped += 1;
                }
                DropPolicy::DropNewest => {
                    self.stats.messages_dropped += 1;
                    return;
                }
                DropPolicy::NeverDrop => {}
            }
        }

        if self.config.enable_logging {
            self.event_log.push(message.clone());
        }

        if let Some(sink) = &self.config.event_sink {
            sink.append(&message);
        }

        queue.push_back(message);
        self.stats.messages_published += 1;
    }
}

impl Default for InMemoryBus {
    fn default() -> Self {
        Self::new()
    }
}

impl Bus for InMemoryBus {
    fn publish(&mut self, message: Message) {
        self.enqueue(message);
    }

    fn poll(&mut self) -> Option<Message> {
        for priority in [
            Priority::Critical,
            Priority::High,
            Priority::Normal,
            Priority::Low,
        ] {
            let idx = Self::queue_index(priority);
            if let Some(msg) = self.queues[idx].pop_front() {
                self.stats.messages_delivered += 1;
                return Some(msg);
            }
        }
        None
    }

    fn subscribe(&mut self, subscription: Subscription) {
        self.subscriptions.insert(subscription.id, subscription);
    }

    fn unsubscribe(&mut self, subscriber_id: SubscriberId) {
        self.subscriptions.remove(&subscriber_id);
    }

    fn pending_count(&self) -> usize {
        self.queues.iter().map(|q| q.len()).sum()
    }

    fn stats(&self) -> BusStats {
        self.stats
    }
}

pub trait MessagePayload: Sized {
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Option<Self>;
}

impl MessagePayload for String {
    fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        String::from_utf8(bytes.to_vec()).ok()
    }
}

pub struct RequestTracker {
    pending: HashMap<u64, std::time::Instant>,
    timeout_ms: u64,
}

impl RequestTracker {
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            pending: HashMap::new(),
            timeout_ms,
        }
    }

    pub fn track(&mut self, correlation_id: u64) {
        self.pending
            .insert(correlation_id, std::time::Instant::now());
    }

    pub fn complete(&mut self, correlation_id: u64) -> bool {
        self.pending.remove(&correlation_id).is_some()
    }

    pub fn is_pending(&self, correlation_id: u64) -> bool {
        self.pending.contains_key(&correlation_id)
    }

    pub fn cleanup_expired(&mut self) -> Vec<u64> {
        let timeout = std::time::Duration::from_millis(self.timeout_ms);
        let now = std::time::Instant::now();
        let expired: Vec<u64> = self
            .pending
            .iter()
            .filter(|(_, &time)| now.duration_since(time) > timeout)
            .map(|(&id, _)| id)
            .collect();

        for id in &expired {
            self.pending.remove(id);
        }
        expired
    }
}

fn current_time_nanos() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_pub_sub() {
        let mut bus = InMemoryBus::new();

        let msg = Message::data(Topic::trades("BTC-PERP"), b"test".to_vec());
        bus.publish(msg);

        assert_eq!(bus.pending_count(), 1);

        let received = bus.poll().unwrap();
        assert_eq!(received.topic.0, "data.trades.BTC-PERP");
    }

    #[test]
    fn test_priority_ordering() {
        let mut bus = InMemoryBus::new();

        bus.publish(Message::data(Topic::new("low"), vec![]).with_priority(Priority::Low));
        bus.publish(Message::data(Topic::new("high"), vec![]).with_priority(Priority::High));
        bus.publish(Message::data(Topic::new("normal"), vec![]).with_priority(Priority::Normal));
        bus.publish(
            Message::data(Topic::new("critical"), vec![]).with_priority(Priority::Critical),
        );

        assert_eq!(bus.poll().unwrap().topic.0, "critical");
        assert_eq!(bus.poll().unwrap().topic.0, "high");
        assert_eq!(bus.poll().unwrap().topic.0, "normal");
        assert_eq!(bus.poll().unwrap().topic.0, "low");
    }

    #[test]
    fn test_topic_matching() {
        let topic = Topic::trades("BTC-PERP");
        assert!(topic.matches("data.trades.BTC-PERP"));
        assert!(topic.matches("data.trades.*"));
        assert!(topic.matches("*"));
        assert!(!topic.matches("data.book.*"));
    }

    #[test]
    fn test_subscription_filter() {
        let sub = Subscription::new("data.trades.*").with_kinds(vec![MessageKind::Data]);

        let trade_msg = Message::data(Topic::trades("ETH-PERP"), vec![]);
        let event_msg = Message::event(Topic::order_events(), vec![]);

        assert!(sub.matches(&trade_msg));
        assert!(!sub.matches(&event_msg));
    }

    #[test]
    fn test_drop_policy() {
        let config = BusConfig {
            max_queue_size: 2,
            drop_policy: DropPolicy::DropOldest,
            enable_logging: false,
            event_sink: None,
        };
        let mut bus = InMemoryBus::with_config(config);

        bus.publish(Message::data(Topic::new("1"), vec![]));
        bus.publish(Message::data(Topic::new("2"), vec![]));
        bus.publish(Message::data(Topic::new("3"), vec![]));

        assert_eq!(bus.pending_count(), 2);
        assert_eq!(bus.poll().unwrap().topic.0, "2");
    }
}
