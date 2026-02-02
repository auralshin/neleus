//! Memory manager - high-level interface for agent memory.

use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

use crate::entry::{MemoryEntry, MemoryQuery, MemoryType};
use crate::error::MemoryResult;
use crate::store::{InMemoryStore, MemoryStore, SqliteMemoryStore};
use crate::vector::VectorIndex;

/// Configuration for the memory manager.
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Path to SQLite database (None for in-memory)
    pub db_path: Option<String>,
    /// Whether to use vector index for semantic search
    pub enable_vector_search: bool,
    /// Dimension of embedding vectors (if using vector search)
    pub embedding_dimension: usize,
    /// Maximum memories per agent (for cleanup)
    pub max_memories_per_agent: usize,
    /// How often to run cleanup (in operations)
    pub cleanup_interval: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            db_path: None,
            enable_vector_search: true,
            embedding_dimension: 1536, // OpenAI ada-002
            max_memories_per_agent: 10000,
            cleanup_interval: 100,
        }
    }
}

/// High-level memory manager for agents.
///
/// Combines persistent storage with vector indexing for semantic search.
pub struct MemoryManager {
    /// Primary storage backend
    store: Arc<dyn MemoryStore>,
    /// Short-term cache
    cache: Arc<InMemoryStore>,
    /// Vector index for semantic search
    vector_index: Option<Arc<VectorIndex>>,
    /// Configuration
    config: MemoryConfig,
    /// Operation counter for cleanup scheduling
    op_count: std::sync::atomic::AtomicUsize,
}

impl MemoryManager {
    /// Create a new memory manager with the given configuration.
    pub fn new(config: MemoryConfig) -> MemoryResult<Self> {
        let store: Arc<dyn MemoryStore> = if let Some(ref path) = config.db_path {
            Arc::new(SqliteMemoryStore::new(Path::new(path))?)
        } else {
            Arc::new(SqliteMemoryStore::in_memory()?)
        };

        let vector_index = if config.enable_vector_search {
            Some(Arc::new(VectorIndex::new(config.embedding_dimension)))
        } else {
            None
        };

        Ok(Self {
            store,
            cache: Arc::new(InMemoryStore::new()),
            vector_index,
            config,
            op_count: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    /// Create a memory manager with default configuration.
    pub fn default_manager() -> MemoryResult<Self> {
        Self::new(MemoryConfig::default())
    }

    /// Store a new memory.
    pub async fn remember(
        &self,
        agent_id: &str,
        content: impl Into<String>,
        memory_type: MemoryType,
        importance: Option<f64>,
        metadata: Option<serde_json::Value>,
        embedding: Option<Vec<f32>>,
    ) -> MemoryResult<Uuid> {
        let mut entry = MemoryEntry::new(agent_id, memory_type, content);

        if let Some(imp) = importance {
            entry = entry.with_importance(imp);
        }
        if let Some(meta) = metadata {
            entry = entry.with_metadata(meta);
        }
        if let Some(emb) = embedding.clone() {
            entry = entry.with_embedding(emb);
        }

        let id = entry.id;

        // Store in both cache and persistent storage
        self.cache.store(entry.clone()).await?;
        self.store.store(entry).await?;

        // Add to vector index if embedding provided
        if let (Some(ref index), Some(emb)) = (&self.vector_index, embedding) {
            index.add(id, emb)?;
        }

        // Periodic cleanup
        self.maybe_cleanup(agent_id).await;

        Ok(id)
    }

    /// Recall memories matching a query.
    pub async fn recall(
        &self,
        agent_id: &str,
        query_text: Option<&str>,
        memory_type: Option<MemoryType>,
        limit: usize,
        query_embedding: Option<Vec<f32>>,
    ) -> MemoryResult<Vec<MemoryEntry>> {
        // If we have an embedding, do vector search first
        if let (Some(ref index), Some(ref emb)) = (&self.vector_index, &query_embedding) {
            let similar_ids = index.search(emb, limit * 2)?;
            
            // Fetch the actual entries
            let mut results = Vec::new();
            for (id, _similarity) in similar_ids {
                if let Some(entry) = self.store.get(id).await? {
                    if entry.agent_id == agent_id {
                        if let Some(mt) = memory_type {
                            if entry.memory_type != mt {
                                continue;
                            }
                        }
                        results.push(entry);
                    }
                }
                if results.len() >= limit {
                    break;
                }
            }
            return Ok(results);
        }

        // Otherwise, use text-based query
        let mut query = MemoryQuery::new()
            .agent(agent_id)
            .limit(limit);

        if let Some(text) = query_text {
            query = query.text(text);
        }
        if let Some(mt) = memory_type {
            query = query.memory_type(mt);
        }

        self.store.query(query).await
    }

    /// Get a specific memory by ID.
    pub async fn get(&self, id: Uuid) -> MemoryResult<Option<MemoryEntry>> {
        // Try cache first
        if let Some(entry) = self.cache.get(id).await? {
            return Ok(Some(entry));
        }
        self.store.get(id).await
    }

    /// Forget (delete) a memory.
    pub async fn forget(&self, id: Uuid) -> MemoryResult<bool> {
        self.cache.delete(id).await?;
        
        if let Some(ref index) = self.vector_index {
            index.remove(id);
        }
        
        self.store.delete(id).await
    }

    /// Clear all memories for an agent.
    pub async fn clear_agent(&self, agent_id: &str) -> MemoryResult<usize> {
        self.cache.clear(agent_id).await?;
        self.store.clear(agent_id).await
    }

    /// Get memory count for an agent.
    pub async fn count(&self, agent_id: &str) -> MemoryResult<usize> {
        self.store.count(agent_id).await
    }

    /// Cleanup expired memories.
    pub async fn cleanup(&self) -> MemoryResult<usize> {
        self.cache.cleanup_expired().await?;
        self.store.cleanup_expired().await
    }

    async fn maybe_cleanup(&self, _agent_id: &str) {
        let count = self.op_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count % self.config.cleanup_interval == 0 {
            let _ = self.cleanup().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_manager_basic() {
        let manager = MemoryManager::default_manager().unwrap();

        let id = manager
            .remember(
                "agent-1",
                "BTC price is $50000",
                MemoryType::Observation,
                Some(0.5),
                None,
                None,
            )
            .await
            .unwrap();

        let entry = manager.get(id).await.unwrap().unwrap();
        assert_eq!(entry.content, "BTC price is $50000");
    }

    #[tokio::test]
    async fn test_recall() {
        let manager = MemoryManager::default_manager().unwrap();

        manager
            .remember("agent-1", "Bought BTC at $50000", MemoryType::Decision, None, None, None)
            .await
            .unwrap();
        manager
            .remember("agent-1", "Sold ETH at $3000", MemoryType::Decision, None, None, None)
            .await
            .unwrap();
        manager
            .remember("agent-1", "Market is bullish", MemoryType::Observation, None, None, None)
            .await
            .unwrap();

        let results = manager
            .recall("agent-1", Some("BTC"), Some(MemoryType::Decision), 10, None)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("BTC"));
    }
}
