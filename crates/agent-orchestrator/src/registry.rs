//! Agent registry - stores and manages all registered agents

use crate::{Agent, AgentId, AgentStats, OrchestratorError, Result};
use dashmap::DashMap;
use std::sync::Arc;

/// Registry for all deployed agents
pub struct AgentRegistry {
    agents: DashMap<AgentId, Arc<Agent>>,
}

impl AgentRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            agents: DashMap::new(),
        }
    }
    
    /// Register a new agent
    pub fn register(&self, agent: Agent) -> Result<()> {
        let agent_id = agent.id.clone();
        
        if self.agents.contains_key(&agent_id) {
            return Err(OrchestratorError::AgentAlreadyExists(agent_id));
        }
        
        self.agents.insert(agent_id, Arc::new(agent));
        Ok(())
    }
    
    /// Unregister an agent
    pub fn unregister(&self, agent_id: &str) -> Result<()> {
        self.agents.remove(agent_id)
            .map(|_| ())
            .ok_or_else(|| OrchestratorError::AgentNotFound(agent_id.to_string()))
    }
    
    /// Check if an agent exists
    pub fn contains(&self, agent_id: &str) -> bool {
        self.agents.contains_key(agent_id)
    }
    
    /// Get an agent by ID
    pub fn get(&self, agent_id: &str) -> Result<Arc<Agent>> {
        self.agents.get(agent_id)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| OrchestratorError::AgentNotFound(agent_id.to_string()))
    }
    
    /// Get a mutable reference to an agent
    pub fn get_mut(&self, agent_id: &str) -> Result<Arc<Agent>> {
        // DashMap doesn't provide true mutable access, but Arc<Agent> uses interior mutability
        self.get(agent_id)
    }
    
    /// List all agents
    pub fn list_all(&self) -> Vec<Arc<Agent>> {
        self.agents.iter()
            .map(|entry| entry.value().clone())
            .collect()
    }
    
    /// Get count of agents
    pub fn count(&self) -> usize {
        self.agents.len()
    }
    
    /// Filter agents by state
    pub fn filter_by_state(&self, state: crate::AgentState) -> Vec<Arc<Agent>> {
        self.agents.iter()
            .filter(|entry| entry.value().state() == state)
            .map(|entry| entry.value().clone())
            .collect()
    }
    
    /// Get agents by labels
    pub fn filter_by_label(&self, key: &str, value: &str) -> Vec<Arc<Agent>> {
        self.agents.iter()
            .filter(|entry| {
                entry.value().spec().labels.get(key) == Some(&value.to_string())
            })
            .map(|entry| entry.value().clone())
            .collect()
    }
    
    /// Get statistics for all agents
    pub fn get_all_stats(&self) -> Vec<AgentStats> {
        self.agents.iter()
            .map(|entry| entry.value().stats())
            .collect()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
