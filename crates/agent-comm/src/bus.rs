//! Message bus implementations.

use async_trait::async_trait;
use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use crate::error::{CommError, CommResult};
use crate::message::{AgentMessage, MessagePriority, Subscription};

/// Trait for message bus implementations.
#[async_trait]
pub trait MessageBus: Send + Sync {
    /// Register an agent with the bus.
    async fn register(&self, agent_id: &str) -> CommResult<()>;

    /// Unregister an agent from the bus.
    async fn unregister(&self, agent_id: &str) -> CommResult<()>;

    /// Send a message (direct or broadcast based on message content).
    async fn send(&self, message: AgentMessage) -> CommResult<()>;

    /// Receive messages for an agent (blocking).
    async fn receive(&self, agent_id: &str) -> CommResult<AgentMessage>;

    /// Try to receive a message (non-blocking).
    async fn try_receive(&self, agent_id: &str) -> CommResult<Option<AgentMessage>>;

    /// Subscribe to a topic.
    async fn subscribe(&self, agent_id: &str, topic: &str) -> CommResult<Subscription>;

    /// Unsubscribe from a topic.
    async fn unsubscribe(&self, subscription_id: Uuid) -> CommResult<()>;

    /// Get pending message count for an agent.
    async fn pending_count(&self, agent_id: &str) -> CommResult<usize>;
}

// =============================================================================
// Local In-Process Message Bus
// =============================================================================

/// Agent mailbox for receiving messages.
struct AgentMailbox {
    /// High priority queue
    high: VecDeque<AgentMessage>,
    /// Normal priority queue
    normal: VecDeque<AgentMessage>,
    /// Low priority queue
    low: VecDeque<AgentMessage>,
    /// Notification sender
    notify: mpsc::Sender<()>,
}

impl AgentMailbox {
    fn new(notify: mpsc::Sender<()>) -> Self {
        Self {
            high: VecDeque::new(),
            normal: VecDeque::new(),
            low: VecDeque::new(),
            notify,
        }
    }

    fn push(&mut self, msg: AgentMessage) {
        match msg.priority {
            MessagePriority::Critical | MessagePriority::High => self.high.push_back(msg),
            MessagePriority::Normal => self.normal.push_back(msg),
            MessagePriority::Low => self.low.push_back(msg),
        }
        let _ = self.notify.try_send(());
    }

    fn pop(&mut self) -> Option<AgentMessage> {
        // Pop from highest priority first
        self.high
            .pop_front()
            .or_else(|| self.normal.pop_front())
            .or_else(|| self.low.pop_front())
    }

    fn len(&self) -> usize {
        self.high.len() + self.normal.len() + self.low.len()
    }
}

/// Local in-process message bus.
///
/// High-performance message passing for agents in the same process.
pub struct LocalMessageBus {
    /// Agent mailboxes
    mailboxes: DashMap<String, Arc<RwLock<AgentMailbox>>>,
    /// Notification receivers (for blocking receive)
    receivers: DashMap<String, Arc<RwLock<mpsc::Receiver<()>>>>,
    /// Topic subscriptions: topic -> [agent_ids]
    subscriptions: DashMap<String, Vec<String>>,
    /// Subscription registry: subscription_id -> (agent_id, topic)
    subscription_registry: DashMap<Uuid, (String, String)>,
    /// Broadcast channels for topics
    topic_channels: DashMap<String, broadcast::Sender<AgentMessage>>,
}

impl LocalMessageBus {
    /// Create a new local message bus.
    pub fn new() -> Self {
        Self {
            mailboxes: DashMap::new(),
            receivers: DashMap::new(),
            subscriptions: DashMap::new(),
            subscription_registry: DashMap::new(),
            topic_channels: DashMap::new(),
        }
    }

    /// Get or create a broadcast channel for a topic.
    fn get_topic_channel(&self, topic: &str) -> broadcast::Sender<AgentMessage> {
        self.topic_channels
            .entry(topic.to_string())
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(1024);
                tx
            })
            .clone()
    }
}

impl Default for LocalMessageBus {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageBus for LocalMessageBus {
    async fn register(&self, agent_id: &str) -> CommResult<()> {
        let (tx, rx) = mpsc::channel(100);
        let mailbox = AgentMailbox::new(tx);

        self.mailboxes
            .insert(agent_id.to_string(), Arc::new(RwLock::new(mailbox)));
        self.receivers
            .insert(agent_id.to_string(), Arc::new(RwLock::new(rx)));

        tracing::debug!("Agent {} registered with message bus", agent_id);
        Ok(())
    }

    async fn unregister(&self, agent_id: &str) -> CommResult<()> {
        self.mailboxes.remove(agent_id);
        self.receivers.remove(agent_id);

        // Remove from all subscriptions
        for mut entry in self.subscriptions.iter_mut() {
            entry.value_mut().retain(|id| id != agent_id);
        }

        tracing::debug!("Agent {} unregistered from message bus", agent_id);
        Ok(())
    }

    async fn send(&self, message: AgentMessage) -> CommResult<()> {
        // Skip expired messages
        if message.is_expired() {
            tracing::warn!("Dropping expired message {}", message.id);
            return Ok(());
        }

        // Broadcast to topic
        if let Some(ref topic) = message.topic {
            let channel = self.get_topic_channel(topic);
            let _ = channel.send(message.clone());

            // Also deliver to subscribed agents' mailboxes
            if let Some(subscribers) = self.subscriptions.get(topic) {
                for agent_id in subscribers.value() {
                    if let Some(mailbox) = self.mailboxes.get(agent_id) {
                        mailbox.write().await.push(message.clone());
                    }
                }
            }
            return Ok(());
        }

        // Direct message
        if let Some(ref to_agent) = message.to_agent {
            if let Some(mailbox) = self.mailboxes.get(to_agent) {
                mailbox.write().await.push(message);
                return Ok(());
            } else {
                return Err(CommError::AgentNotFound(to_agent.clone()));
            }
        }

        Err(CommError::SendFailed(
            "Message has no recipient or topic".to_string(),
        ))
    }

    async fn receive(&self, agent_id: &str) -> CommResult<AgentMessage> {
        // First check if there's a message in the mailbox
        if let Some(mailbox) = self.mailboxes.get(agent_id) {
            if let Some(msg) = mailbox.write().await.pop() {
                return Ok(msg);
            }
        } else {
            return Err(CommError::AgentNotFound(agent_id.to_string()));
        }

        // Wait for notification
        let receiver = self
            .receivers
            .get(agent_id)
            .ok_or_else(|| CommError::AgentNotFound(agent_id.to_string()))?;

        loop {
            receiver.write().await.recv().await;

            // Check mailbox again
            if let Some(mailbox) = self.mailboxes.get(agent_id) {
                if let Some(msg) = mailbox.write().await.pop() {
                    return Ok(msg);
                }
            }
        }
    }

    async fn try_receive(&self, agent_id: &str) -> CommResult<Option<AgentMessage>> {
        if let Some(mailbox) = self.mailboxes.get(agent_id) {
            Ok(mailbox.write().await.pop())
        } else {
            Err(CommError::AgentNotFound(agent_id.to_string()))
        }
    }

    async fn subscribe(&self, agent_id: &str, topic: &str) -> CommResult<Subscription> {
        let subscription = Subscription::new(agent_id, topic);

        // Add to subscriptions
        self.subscriptions
            .entry(topic.to_string())
            .or_insert_with(Vec::new)
            .push(agent_id.to_string());

        // Register subscription
        self.subscription_registry.insert(
            subscription.id,
            (agent_id.to_string(), topic.to_string()),
        );

        tracing::debug!("Agent {} subscribed to topic {}", agent_id, topic);
        Ok(subscription)
    }

    async fn unsubscribe(&self, subscription_id: Uuid) -> CommResult<()> {
        if let Some((_, (agent_id, topic))) = self.subscription_registry.remove(&subscription_id) {
            if let Some(mut subscribers) = self.subscriptions.get_mut(&topic) {
                subscribers.retain(|id| id != &agent_id);
            }
            tracing::debug!("Agent {} unsubscribed from topic {}", agent_id, topic);
            Ok(())
        } else {
            Err(CommError::TopicNotFound(subscription_id.to_string()))
        }
    }

    async fn pending_count(&self, agent_id: &str) -> CommResult<usize> {
        if let Some(mailbox) = self.mailboxes.get(agent_id) {
            Ok(mailbox.read().await.len())
        } else {
            Err(CommError::AgentNotFound(agent_id.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageType;

    #[tokio::test]
    async fn test_direct_message() {
        let bus = LocalMessageBus::new();

        bus.register("agent-1").await.unwrap();
        bus.register("agent-2").await.unwrap();

        let msg = AgentMessage::direct(
            "agent-1",
            "agent-2",
            MessageType::DataRequest,
            serde_json::json!({"test": true}),
        );

        bus.send(msg).await.unwrap();

        let received = bus.try_receive("agent-2").await.unwrap().unwrap();
        assert_eq!(received.from_agent, "agent-1");
    }

    #[tokio::test]
    async fn test_topic_subscription() {
        let bus = LocalMessageBus::new();

        bus.register("agent-1").await.unwrap();
        bus.register("agent-2").await.unwrap();
        bus.register("agent-3").await.unwrap();

        bus.subscribe("agent-2", "signals").await.unwrap();
        bus.subscribe("agent-3", "signals").await.unwrap();

        let msg = AgentMessage::broadcast(
            "agent-1",
            "signals",
            MessageType::SignalShare,
            serde_json::json!({"signal": "buy"}),
        );

        bus.send(msg).await.unwrap();

        // Both subscribed agents should receive
        assert!(bus.try_receive("agent-2").await.unwrap().is_some());
        assert!(bus.try_receive("agent-3").await.unwrap().is_some());
        // Sender should not receive their own broadcast
        assert!(bus.try_receive("agent-1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let bus = LocalMessageBus::new();
        bus.register("agent-1").await.unwrap();

        // Send low priority first
        let low = AgentMessage::direct(
            "sender",
            "agent-1",
            MessageType::Status,
            serde_json::json!({"order": 1}),
        )
        .with_priority(MessagePriority::Low);

        let high = AgentMessage::direct(
            "sender",
            "agent-1",
            MessageType::Alert,
            serde_json::json!({"order": 2}),
        )
        .with_priority(MessagePriority::High);

        bus.send(low).await.unwrap();
        bus.send(high).await.unwrap();

        // High priority should come first
        let first = bus.try_receive("agent-1").await.unwrap().unwrap();
        assert_eq!(first.payload["order"], 2);

        let second = bus.try_receive("agent-1").await.unwrap().unwrap();
        assert_eq!(second.payload["order"], 1);
    }
}
