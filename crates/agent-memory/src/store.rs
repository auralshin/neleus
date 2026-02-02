//! Memory storage backends.

use async_trait::async_trait;
use chrono::Utc;
use parking_lot::RwLock;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::path::Path;
use tokio::sync::Mutex as TokioMutex;
use uuid::Uuid;

use crate::entry::{MemoryEntry, MemoryQuery, MemoryType};
use crate::error::MemoryResult;

/// Trait for memory storage backends.
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Store a memory entry.
    async fn store(&self, entry: MemoryEntry) -> MemoryResult<Uuid>;

    /// Retrieve a memory entry by ID.
    async fn get(&self, id: Uuid) -> MemoryResult<Option<MemoryEntry>>;

    /// Query memories based on criteria.
    async fn query(&self, query: MemoryQuery) -> MemoryResult<Vec<MemoryEntry>>;

    /// Delete a memory entry.
    async fn delete(&self, id: Uuid) -> MemoryResult<bool>;

    /// Delete expired memories.
    async fn cleanup_expired(&self) -> MemoryResult<usize>;

    /// Get total count of memories for an agent.
    async fn count(&self, agent_id: &str) -> MemoryResult<usize>;

    /// Clear all memories for an agent.
    async fn clear(&self, agent_id: &str) -> MemoryResult<usize>;
}

// =============================================================================
// In-Memory Store (for testing and caching)
// =============================================================================

/// In-memory storage backend, useful for testing and short-term caching.
pub struct InMemoryStore {
    entries: RwLock<HashMap<Uuid, MemoryEntry>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MemoryStore for InMemoryStore {
    async fn store(&self, entry: MemoryEntry) -> MemoryResult<Uuid> {
        let id = entry.id;
        self.entries.write().insert(id, entry);
        Ok(id)
    }

    async fn get(&self, id: Uuid) -> MemoryResult<Option<MemoryEntry>> {
        let mut entries = self.entries.write();
        if let Some(entry) = entries.get_mut(&id) {
            entry.mark_accessed();
            Ok(Some(entry.clone()))
        } else {
            Ok(None)
        }
    }

    async fn query(&self, query: MemoryQuery) -> MemoryResult<Vec<MemoryEntry>> {
        let entries = self.entries.read();
        let now = Utc::now();

        let mut results: Vec<_> = entries
            .values()
            .filter(|e| {
                // Filter by agent
                if let Some(ref agent_id) = query.agent_id {
                    if e.agent_id != *agent_id {
                        return false;
                    }
                }

                // Filter by memory type
                if let Some(ref types) = query.memory_types {
                    if !types.contains(&e.memory_type) {
                        return false;
                    }
                }

                // Filter by importance
                if let Some(min_importance) = query.min_importance {
                    if e.importance < min_importance {
                        return false;
                    }
                }

                // Filter by time range
                if let Some(since) = query.since {
                    if e.created_at < since {
                        return false;
                    }
                }
                if let Some(until) = query.until {
                    if e.created_at > until {
                        return false;
                    }
                }

                // Filter expired unless explicitly included
                if !query.include_expired {
                    if let Some(expires_at) = e.expires_at {
                        if now > expires_at {
                            return false;
                        }
                    }
                }

                // Text search (simple contains)
                if let Some(ref text) = query.query_text {
                    let text_lower = text.to_lowercase();
                    if !e.content.to_lowercase().contains(&text_lower) {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Sort by relevance score
        results.sort_by(|a, b| {
            b.relevance_score()
                .partial_cmp(&a.relevance_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        results.truncate(query.limit);

        Ok(results)
    }

    async fn delete(&self, id: Uuid) -> MemoryResult<bool> {
        Ok(self.entries.write().remove(&id).is_some())
    }

    async fn cleanup_expired(&self) -> MemoryResult<usize> {
        let now = Utc::now();
        let mut entries = self.entries.write();
        let before = entries.len();
        entries.retain(|_, e| {
            e.expires_at.map(|exp| now <= exp).unwrap_or(true)
        });
        Ok(before - entries.len())
    }

    async fn count(&self, agent_id: &str) -> MemoryResult<usize> {
        Ok(self
            .entries
            .read()
            .values()
            .filter(|e| e.agent_id == agent_id)
            .count())
    }

    async fn clear(&self, agent_id: &str) -> MemoryResult<usize> {
        let mut entries = self.entries.write();
        let before = entries.len();
        entries.retain(|_, e| e.agent_id != agent_id);
        Ok(before - entries.len())
    }
}

// =============================================================================
// SQLite Store (persistent storage)
// =============================================================================

/// SQLite-based persistent memory store.
pub struct SqliteMemoryStore {
    conn: TokioMutex<Connection>,
}

impl SqliteMemoryStore {
    /// Create a new SQLite memory store.
    pub fn new(path: impl AsRef<Path>) -> MemoryResult<Self> {
        let conn = Connection::open(path)?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: TokioMutex::new(conn),
        })
    }

    /// Create an in-memory SQLite database (for testing).
    pub fn in_memory() -> MemoryResult<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: TokioMutex::new(conn),
        })
    }

    fn init_schema(conn: &Connection) -> MemoryResult<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                memory_type TEXT NOT NULL,
                content TEXT NOT NULL,
                embedding BLOB,
                importance REAL NOT NULL,
                metadata TEXT,
                created_at TEXT NOT NULL,
                accessed_at TEXT NOT NULL,
                access_count INTEGER NOT NULL DEFAULT 0,
                expires_at TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_memories_agent 
                ON memories(agent_id);
            CREATE INDEX IF NOT EXISTS idx_memories_type 
                ON memories(agent_id, memory_type);
            CREATE INDEX IF NOT EXISTS idx_memories_created 
                ON memories(agent_id, created_at);
            CREATE INDEX IF NOT EXISTS idx_memories_importance 
                ON memories(agent_id, importance);
            "#,
        )?;
        Ok(())
    }

    fn entry_from_row(row: &rusqlite::Row) -> rusqlite::Result<MemoryEntry> {
        let id_str: String = row.get(0)?;
        let memory_type_str: String = row.get(2)?;
        let embedding_blob: Option<Vec<u8>> = row.get(4)?;
        let metadata_str: String = row.get(6)?;
        let created_str: String = row.get(7)?;
        let accessed_str: String = row.get(8)?;
        let expires_str: Option<String> = row.get(10)?;

        Ok(MemoryEntry {
            id: Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4()),
            agent_id: row.get(1)?,
            memory_type: serde_json::from_str(&format!("\"{}\"", memory_type_str))
                .unwrap_or(MemoryType::Context),
            content: row.get(3)?,
            embedding: embedding_blob.map(|b| {
                b.chunks(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect()
            }),
            importance: row.get(5)?,
            metadata: serde_json::from_str(&metadata_str).unwrap_or(serde_json::Value::Null),
            created_at: chrono::DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            accessed_at: chrono::DateTime::parse_from_rfc3339(&accessed_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            access_count: row.get(9)?,
            expires_at: expires_str.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            }),
        })
    }
}

#[async_trait]
impl MemoryStore for SqliteMemoryStore {
    async fn store(&self, entry: MemoryEntry) -> MemoryResult<Uuid> {
        let conn = self.conn.lock().await;
        let id = entry.id;

        let embedding_blob: Option<Vec<u8>> = entry.embedding.map(|e| {
            e.iter().flat_map(|f| f.to_le_bytes()).collect()
        });

        let memory_type_str = serde_json::to_string(&entry.memory_type)?
            .trim_matches('"')
            .to_string();

        conn.execute(
            r#"
            INSERT OR REPLACE INTO memories 
            (id, agent_id, memory_type, content, embedding, importance, 
             metadata, created_at, accessed_at, access_count, expires_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                id.to_string(),
                entry.agent_id,
                memory_type_str,
                entry.content,
                embedding_blob,
                entry.importance,
                serde_json::to_string(&entry.metadata)?,
                entry.created_at.to_rfc3339(),
                entry.accessed_at.to_rfc3339(),
                entry.access_count,
                entry.expires_at.map(|e| e.to_rfc3339()),
            ],
        )?;

        Ok(id)
    }

    async fn get(&self, id: Uuid) -> MemoryResult<Option<MemoryEntry>> {
        let conn = self.conn.lock().await;
        
        // Update access time and count
        conn.execute(
            "UPDATE memories SET accessed_at = ?1, access_count = access_count + 1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), id.to_string()],
        )?;

        let mut stmt = conn.prepare(
            "SELECT id, agent_id, memory_type, content, embedding, importance, 
                    metadata, created_at, accessed_at, access_count, expires_at 
             FROM memories WHERE id = ?1",
        )?;

        let entry = stmt
            .query_row(params![id.to_string()], Self::entry_from_row)
            .optional()?;

        Ok(entry)
    }

    async fn query(&self, query: MemoryQuery) -> MemoryResult<Vec<MemoryEntry>> {
        let conn = self.conn.lock().await;
        let now = Utc::now();

        let mut sql = String::from(
            "SELECT id, agent_id, memory_type, content, embedding, importance, 
                    metadata, created_at, accessed_at, access_count, expires_at 
             FROM memories WHERE 1=1",
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref agent_id) = query.agent_id {
            sql.push_str(" AND agent_id = ?");
            params_vec.push(Box::new(agent_id.clone()));
        }

        if let Some(ref types) = query.memory_types {
            let type_strs: Vec<String> = types
                .iter()
                .map(|t| {
                    serde_json::to_string(t)
                        .unwrap_or_default()
                        .trim_matches('"')
                        .to_string()
                })
                .collect();
            let placeholders = type_strs.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            sql.push_str(&format!(" AND memory_type IN ({})", placeholders));
            for t in type_strs {
                params_vec.push(Box::new(t));
            }
        }

        if let Some(min_importance) = query.min_importance {
            sql.push_str(" AND importance >= ?");
            params_vec.push(Box::new(min_importance));
        }

        if let Some(since) = query.since {
            sql.push_str(" AND created_at >= ?");
            params_vec.push(Box::new(since.to_rfc3339()));
        }

        if let Some(until) = query.until {
            sql.push_str(" AND created_at <= ?");
            params_vec.push(Box::new(until.to_rfc3339()));
        }

        if !query.include_expired {
            sql.push_str(" AND (expires_at IS NULL OR expires_at > ?)");
            params_vec.push(Box::new(now.to_rfc3339()));
        }

        if let Some(ref text) = query.query_text {
            sql.push_str(" AND content LIKE ?");
            params_vec.push(Box::new(format!("%{}%", text)));
        }

        sql.push_str(" ORDER BY importance DESC, accessed_at DESC");
        sql.push_str(&format!(" LIMIT {}", query.limit));

        let mut stmt = conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        
        let entries = stmt
            .query_map(params_refs.as_slice(), Self::entry_from_row)?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    async fn delete(&self, id: Uuid) -> MemoryResult<bool> {
        let conn = self.conn.lock().await;
        let changes = conn.execute("DELETE FROM memories WHERE id = ?1", params![id.to_string()])?;
        Ok(changes > 0)
    }

    async fn cleanup_expired(&self) -> MemoryResult<usize> {
        let conn = self.conn.lock().await;
        let changes = conn.execute(
            "DELETE FROM memories WHERE expires_at IS NOT NULL AND expires_at < ?1",
            params![Utc::now().to_rfc3339()],
        )?;
        Ok(changes)
    }

    async fn count(&self, agent_id: &str) -> MemoryResult<usize> {
        let conn = self.conn.lock().await;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE agent_id = ?1",
            params![agent_id],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    async fn clear(&self, agent_id: &str) -> MemoryResult<usize> {
        let conn = self.conn.lock().await;
        let changes = conn.execute("DELETE FROM memories WHERE agent_id = ?1", params![agent_id])?;
        Ok(changes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_inmemory_store_basic() {
        let store = InMemoryStore::new();
        
        let entry = MemoryEntry::new("agent-1", MemoryType::Decision, "Buy BTC");
        let id = store.store(entry).await.unwrap();
        
        let retrieved = store.get(id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Buy BTC");
        assert_eq!(retrieved.access_count, 1);
    }

    #[tokio::test]
    async fn test_sqlite_store_basic() {
        let store = SqliteMemoryStore::in_memory().unwrap();
        
        let entry = MemoryEntry::new("agent-1", MemoryType::Decision, "Sell ETH");
        let id = store.store(entry).await.unwrap();
        
        let retrieved = store.get(id).await.unwrap().unwrap();
        assert_eq!(retrieved.content, "Sell ETH");
    }

    #[tokio::test]
    async fn test_query_by_type() {
        let store = InMemoryStore::new();
        
        store.store(MemoryEntry::new("agent-1", MemoryType::Decision, "Decision 1")).await.unwrap();
        store.store(MemoryEntry::new("agent-1", MemoryType::Observation, "Obs 1")).await.unwrap();
        store.store(MemoryEntry::new("agent-1", MemoryType::Decision, "Decision 2")).await.unwrap();
        
        let query = MemoryQuery::new()
            .agent("agent-1")
            .memory_type(MemoryType::Decision);
        
        let results = store.query(query).await.unwrap();
        assert_eq!(results.len(), 2);
    }
}
