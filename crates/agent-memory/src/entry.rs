//! Memory entry types and structures.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of memory entry - determines storage behavior and retrieval priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// Market observations (prices, volumes, events)
    Observation,
    /// Trading decisions made by the agent
    Decision,
    /// Actions taken (tool executions, orders)
    Action,
    /// Outcomes of actions (trade results, P&L)
    Outcome,
    /// Learned insights from experience
    Learning,
    /// Contextual information (agent state, session info)
    Context,
    /// Conversation history
    Conversation,
}

impl MemoryType {
    /// Default time-to-live for this memory type in seconds.
    pub fn default_ttl_secs(&self) -> Option<i64> {
        match self {
            MemoryType::Observation => Some(3600 * 24),     // 1 day
            MemoryType::Decision => Some(3600 * 24 * 7),    // 1 week
            MemoryType::Action => Some(3600 * 24 * 7),      // 1 week
            MemoryType::Outcome => None,                     // Never expire
            MemoryType::Learning => None,                    // Never expire
            MemoryType::Context => Some(3600 * 24),         // 1 day
            MemoryType::Conversation => Some(3600 * 24 * 3), // 3 days
        }
    }

    /// Base importance weight for this memory type.
    pub fn importance_weight(&self) -> f64 {
        match self {
            MemoryType::Observation => 0.3,
            MemoryType::Decision => 0.7,
            MemoryType::Action => 0.5,
            MemoryType::Outcome => 0.9,
            MemoryType::Learning => 1.0,
            MemoryType::Context => 0.2,
            MemoryType::Conversation => 0.4,
        }
    }
}

/// A single memory entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique identifier
    pub id: Uuid,
    /// Agent that owns this memory
    pub agent_id: String,
    /// Type of memory
    pub memory_type: MemoryType,
    /// Content of the memory (text)
    pub content: String,
    /// Optional embedding vector for semantic search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    /// Importance score (0.0 - 1.0)
    pub importance: f64,
    /// Additional structured metadata
    #[serde(default)]
    pub metadata: serde_json::Value,
    /// When this memory was created
    pub created_at: DateTime<Utc>,
    /// When this memory was last accessed
    pub accessed_at: DateTime<Utc>,
    /// Number of times this memory was accessed
    pub access_count: u32,
    /// Optional expiration time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

impl MemoryEntry {
    /// Create a new memory entry.
    pub fn new(
        agent_id: impl Into<String>,
        memory_type: MemoryType,
        content: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        let expires_at = memory_type
            .default_ttl_secs()
            .map(|ttl| now + chrono::Duration::seconds(ttl));

        Self {
            id: Uuid::new_v4(),
            agent_id: agent_id.into(),
            memory_type,
            content: content.into(),
            embedding: None,
            importance: memory_type.importance_weight(),
            metadata: serde_json::Value::Null,
            created_at: now,
            accessed_at: now,
            access_count: 0,
            expires_at,
        }
    }

    /// Set the importance score.
    pub fn with_importance(mut self, importance: f64) -> Self {
        self.importance = importance.clamp(0.0, 1.0);
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set embedding vector.
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Set custom TTL.
    pub fn with_ttl_secs(mut self, ttl_secs: i64) -> Self {
        self.expires_at = Some(self.created_at + chrono::Duration::seconds(ttl_secs));
        self
    }

    /// Check if this memory has expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| Utc::now() > exp)
            .unwrap_or(false)
    }

    /// Calculate recency score (0.0 - 1.0, higher = more recent).
    pub fn recency_score(&self) -> f64 {
        let age_hours = (Utc::now() - self.accessed_at).num_hours() as f64;
        // Exponential decay with half-life of 24 hours
        (-age_hours / 24.0).exp()
    }

    /// Calculate relevance score combining importance, recency, and access frequency.
    pub fn relevance_score(&self) -> f64 {
        let recency = self.recency_score();
        let frequency = (self.access_count as f64).ln_1p() / 10.0; // Logarithmic scaling
        
        // Weighted combination
        0.5 * self.importance + 0.3 * recency + 0.2 * frequency.min(1.0)
    }

    /// Mark this memory as accessed, updating access time and count.
    pub fn mark_accessed(&mut self) {
        self.accessed_at = Utc::now();
        self.access_count += 1;
    }
}

/// Builder for creating memory queries.
#[derive(Debug, Clone, Default)]
pub struct MemoryQuery {
    pub agent_id: Option<String>,
    pub memory_types: Option<Vec<MemoryType>>,
    pub query_text: Option<String>,
    pub query_embedding: Option<Vec<f32>>,
    pub min_importance: Option<f64>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub limit: usize,
    pub include_expired: bool,
}

impl MemoryQuery {
    pub fn new() -> Self {
        Self {
            limit: 10,
            ..Default::default()
        }
    }

    pub fn agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    pub fn memory_type(mut self, memory_type: MemoryType) -> Self {
        self.memory_types = Some(vec![memory_type]);
        self
    }

    pub fn memory_types(mut self, types: Vec<MemoryType>) -> Self {
        self.memory_types = Some(types);
        self
    }

    pub fn text(mut self, query: impl Into<String>) -> Self {
        self.query_text = Some(query.into());
        self
    }

    pub fn embedding(mut self, embedding: Vec<f32>) -> Self {
        self.query_embedding = Some(embedding);
        self
    }

    pub fn min_importance(mut self, importance: f64) -> Self {
        self.min_importance = Some(importance);
        self
    }

    pub fn since(mut self, since: DateTime<Utc>) -> Self {
        self.since = Some(since);
        self
    }

    pub fn until(mut self, until: DateTime<Utc>) -> Self {
        self.until = Some(until);
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn include_expired(mut self, include: bool) -> Self {
        self.include_expired = include;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_entry_creation() {
        let entry = MemoryEntry::new("agent-1", MemoryType::Decision, "Buy BTC at $50000");
        
        assert_eq!(entry.agent_id, "agent-1");
        assert_eq!(entry.memory_type, MemoryType::Decision);
        assert_eq!(entry.content, "Buy BTC at $50000");
        assert!(!entry.is_expired());
    }

    #[test]
    fn test_memory_type_ttl() {
        assert!(MemoryType::Observation.default_ttl_secs().is_some());
        assert!(MemoryType::Learning.default_ttl_secs().is_none());
    }

    #[test]
    fn test_relevance_score() {
        let entry = MemoryEntry::new("agent-1", MemoryType::Learning, "Important insight");
        let score = entry.relevance_score();
        
        // Learning type has high importance, recent entry should have high score
        assert!(score > 0.5);
    }
}
