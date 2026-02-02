//! Message types for agent communication.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of message being sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// Request for data from another agent
    DataRequest,
    /// Response to a data request
    DataResponse,
    /// Share a trading signal
    SignalShare,
    /// Coordination between agents
    Coordination,
    /// Alert or warning
    Alert,
    /// Status update
    Status,
    /// Heartbeat/ping
    Heartbeat,
    /// Custom message type
    Custom(u8),
}

/// Priority level for message delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessagePriority {
    /// Low priority - can be delayed
    Low = 0,
    /// Normal priority - standard delivery
    Normal = 1,
    /// High priority - expedited delivery
    High = 2,
    /// Critical priority - immediate delivery
    Critical = 3,
}

impl Default for MessagePriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// A message sent between agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Unique message ID
    pub id: Uuid,
    /// Sender agent ID
    pub from_agent: String,
    /// Recipient agent ID (None for broadcast)
    pub to_agent: Option<String>,
    /// Topic for pub/sub (None for direct messages)
    pub topic: Option<String>,
    /// Message type
    pub message_type: MessageType,
    /// Message priority
    pub priority: MessagePriority,
    /// Message payload (JSON)
    pub payload: serde_json::Value,
    /// Correlation ID for request/response
    pub correlation_id: Option<Uuid>,
    /// Whether this is a reply
    pub is_reply: bool,
    /// When the message was created
    pub created_at: DateTime<Utc>,
    /// Time-to-live in milliseconds (None = no expiry)
    pub ttl_ms: Option<u64>,
}

impl AgentMessage {
    /// Create a new direct message.
    pub fn direct(
        from: impl Into<String>,
        to: impl Into<String>,
        message_type: MessageType,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            from_agent: from.into(),
            to_agent: Some(to.into()),
            topic: None,
            message_type,
            priority: MessagePriority::Normal,
            payload,
            correlation_id: None,
            is_reply: false,
            created_at: Utc::now(),
            ttl_ms: None,
        }
    }

    /// Create a broadcast message to a topic.
    pub fn broadcast(
        from: impl Into<String>,
        topic: impl Into<String>,
        message_type: MessageType,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            from_agent: from.into(),
            to_agent: None,
            topic: Some(topic.into()),
            message_type,
            priority: MessagePriority::Normal,
            payload,
            correlation_id: None,
            is_reply: false,
            created_at: Utc::now(),
            ttl_ms: None,
        }
    }

    /// Create a request message (with correlation ID).
    pub fn request(
        from: impl Into<String>,
        to: impl Into<String>,
        message_type: MessageType,
        payload: serde_json::Value,
    ) -> Self {
        let mut msg = Self::direct(from, to, message_type, payload);
        msg.correlation_id = Some(Uuid::new_v4());
        msg
    }

    /// Create a reply to a request message.
    pub fn reply_to(request: &AgentMessage, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            from_agent: request.to_agent.clone().unwrap_or_default(),
            to_agent: Some(request.from_agent.clone()),
            topic: None,
            message_type: MessageType::DataResponse,
            priority: request.priority,
            payload,
            correlation_id: request.correlation_id,
            is_reply: true,
            created_at: Utc::now(),
            ttl_ms: None,
        }
    }

    /// Set message priority.
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set TTL.
    pub fn with_ttl_ms(mut self, ttl_ms: u64) -> Self {
        self.ttl_ms = Some(ttl_ms);
        self
    }

    /// Check if the message has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl_ms {
            let elapsed = (Utc::now() - self.created_at).num_milliseconds() as u64;
            elapsed > ttl
        } else {
            false
        }
    }

    /// Get the age of the message in milliseconds.
    pub fn age_ms(&self) -> u64 {
        (Utc::now() - self.created_at).num_milliseconds().max(0) as u64
    }
}

/// Subscription handle for a topic.
#[derive(Debug, Clone)]
pub struct Subscription {
    pub id: Uuid,
    pub agent_id: String,
    pub topic: String,
    pub created_at: DateTime<Utc>,
}

impl Subscription {
    pub fn new(agent_id: impl Into<String>, topic: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_id: agent_id.into(),
            topic: topic.into(),
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_message() {
        let msg = AgentMessage::direct(
            "agent-1",
            "agent-2",
            MessageType::DataRequest,
            serde_json::json!({"symbol": "BTC"}),
        );

        assert_eq!(msg.from_agent, "agent-1");
        assert_eq!(msg.to_agent, Some("agent-2".to_string()));
        assert!(msg.topic.is_none());
        assert!(!msg.is_expired());
    }

    #[test]
    fn test_broadcast_message() {
        let msg = AgentMessage::broadcast(
            "agent-1",
            "market-data",
            MessageType::SignalShare,
            serde_json::json!({"signal": "buy"}),
        );

        assert!(msg.to_agent.is_none());
        assert_eq!(msg.topic, Some("market-data".to_string()));
    }

    #[test]
    fn test_request_reply() {
        let request = AgentMessage::request(
            "agent-1",
            "agent-2",
            MessageType::DataRequest,
            serde_json::json!({}),
        );

        assert!(request.correlation_id.is_some());

        let reply = AgentMessage::reply_to(&request, serde_json::json!({"data": "here"}));
        assert_eq!(reply.correlation_id, request.correlation_id);
        assert!(reply.is_reply);
        assert_eq!(reply.to_agent, Some("agent-1".to_string()));
    }

    #[test]
    fn test_message_expiry() {
        let msg = AgentMessage::direct(
            "agent-1",
            "agent-2",
            MessageType::Alert,
            serde_json::json!({}),
        )
        .with_ttl_ms(0);

        // Should be expired immediately with 0ms TTL
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert!(msg.is_expired());
    }
}
