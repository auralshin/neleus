//! Agent state persistence

use crate::{AgentId, AgentSpec, OrchestratorError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Persisted agent state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentPersistedState {
    /// Agent specification (for reconstruction)
    pub spec: Option<AgentSpec>,
    /// Whether agent was running before shutdown
    pub was_running: bool,
    /// Realized P&L
    pub realized_pnl: f64,
    /// Number of orders placed
    pub orders_placed: u64,
    /// Number of trades executed
    pub trades_executed: u64,
    /// Restart count
    pub restart_count: u32,
    /// Open positions (serialized)
    pub positions: Vec<PersistedPosition>,
}

/// Persisted position information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedPosition {
    /// Instrument identifier
    pub instrument: String,
    /// Position size (positive = long, negative = short)
    pub size: f64,
    /// Average entry price
    pub avg_entry: f64,
    /// Unrealized P&L at time of save
    pub unrealized_pnl: f64,
}

/// State persister trait
#[async_trait]
pub trait StatePersister: Send + Sync {
    /// Save agent state
    async fn save_agent_state(&self, agent_id: &str, state: &AgentPersistedState) -> Result<()>;
    
    /// Load agent state
    async fn load_agent_state(&self, agent_id: &str) -> Result<Option<AgentPersistedState>>;
    
    /// Delete agent state
    async fn delete_agent_state(&self, agent_id: &str) -> Result<()>;
    
    /// Load all agent states
    async fn load_all_states(&self) -> Result<HashMap<AgentId, AgentPersistedState>>;
}

/// In-memory state persister (for testing)
pub struct MemoryStatePersister {
    states: RwLock<HashMap<AgentId, AgentPersistedState>>,
}

impl MemoryStatePersister {
    pub fn new() -> Self {
        Self {
            states: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryStatePersister {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StatePersister for MemoryStatePersister {
    async fn save_agent_state(&self, agent_id: &str, state: &AgentPersistedState) -> Result<()> {
        self.states.write().insert(agent_id.to_string(), state.clone());
        Ok(())
    }
    
    async fn load_agent_state(&self, agent_id: &str) -> Result<Option<AgentPersistedState>> {
        Ok(self.states.read().get(agent_id).cloned())
    }
    
    async fn delete_agent_state(&self, agent_id: &str) -> Result<()> {
        self.states.write().remove(agent_id);
        Ok(())
    }
    
    async fn load_all_states(&self) -> Result<HashMap<AgentId, AgentPersistedState>> {
        Ok(self.states.read().clone())
    }
}

/// File-based state persister
pub struct FileStatePersister {
    base_path: std::path::PathBuf,
}

impl FileStatePersister {
    pub fn new(base_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }
    
    fn agent_path(&self, agent_id: &str) -> std::path::PathBuf {
        self.base_path.join(format!("{}.json", agent_id))
    }
}

#[async_trait]
impl StatePersister for FileStatePersister {
    async fn save_agent_state(&self, agent_id: &str, state: &AgentPersistedState) -> Result<()> {
        let path = self.agent_path(agent_id);
        let content = serde_json::to_string_pretty(state)
            .map_err(|e| OrchestratorError::StatePersistenceError(e.to_string()))?;
        
        tokio::fs::create_dir_all(&self.base_path).await
            .map_err(|e| OrchestratorError::StatePersistenceError(e.to_string()))?;
        
        tokio::fs::write(&path, content).await
            .map_err(|e| OrchestratorError::StatePersistenceError(e.to_string()))?;
        
        Ok(())
    }
    
    async fn load_agent_state(&self, agent_id: &str) -> Result<Option<AgentPersistedState>> {
        let path = self.agent_path(agent_id);
        
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => {
                let state = serde_json::from_str(&content)
                    .map_err(|e| OrchestratorError::StatePersistenceError(e.to_string()))?;
                Ok(Some(state))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(OrchestratorError::StatePersistenceError(e.to_string())),
        }
    }
    
    async fn delete_agent_state(&self, agent_id: &str) -> Result<()> {
        let path = self.agent_path(agent_id);
        
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(OrchestratorError::StatePersistenceError(e.to_string())),
        }
    }
    
    async fn load_all_states(&self) -> Result<HashMap<AgentId, AgentPersistedState>> {
        let mut states = HashMap::new();
        
        let mut entries = match tokio::fs::read_dir(&self.base_path).await {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(states),
            Err(e) => return Err(OrchestratorError::StatePersistenceError(e.to_string())),
        };
        
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Some(agent_id) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(Some(state)) = self.load_agent_state(agent_id).await {
                        states.insert(agent_id.to_string(), state);
                    }
                }
            }
        }
        
        Ok(states)
    }
}
