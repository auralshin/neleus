//! Dashboard data and summaries

use crate::{AgentEvent, AgentMetricsSnapshot};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Dashboard data holder
pub struct DashboardData {
    agent_metrics: RwLock<HashMap<String, AgentMetricsSnapshot>>,
    recent_events: RwLock<VecDeque<AgentEvent>>,
    max_events: usize,
}

impl DashboardData {
    pub fn new() -> Self {
        Self::with_capacity(1000)
    }
    
    pub fn with_capacity(max_events: usize) -> Self {
        Self {
            agent_metrics: RwLock::new(HashMap::new()),
            recent_events: RwLock::new(VecDeque::with_capacity(max_events)),
            max_events,
        }
    }
    
    /// Update metrics for an agent
    pub fn update_agent_metrics(&self, agent_id: &str, metrics: &AgentMetricsSnapshot) {
        self.agent_metrics.write().insert(agent_id.to_string(), metrics.clone());
    }
    
    /// Add an event
    pub fn add_event(&self, event: AgentEvent) {
        let mut events = self.recent_events.write();
        if events.len() >= self.max_events {
            events.pop_front();
        }
        events.push_back(event);
    }
    
    /// Get current metrics for all agents
    pub fn get_all_metrics(&self) -> HashMap<String, AgentMetricsSnapshot> {
        self.agent_metrics.read().clone()
    }
    
    /// Get recent events
    pub fn get_recent_events(&self, limit: usize) -> Vec<AgentEvent> {
        self.recent_events.read()
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Get dashboard summary
    pub fn get_summary(&self) -> DashboardSummary {
        let metrics = self.agent_metrics.read();
        
        let mut total_pnl = 0.0;
        let mut total_exposure = 0.0;
        let mut total_positions = 0;
        let mut total_orders = 0;
        let mut running_agents = 0;
        let mut agents_with_errors = 0;
        
        for (_, m) in metrics.iter() {
            total_pnl += m.pnl.total_pnl;
            total_exposure += m.positions.gross_exposure;
            total_positions += m.positions.open_positions as usize;
            total_orders += m.orders.open_orders as usize;
            running_agents += 1;
            if m.system.error_count > 0 {
                agents_with_errors += 1;
            }
        }
        
        DashboardSummary {
            timestamp: Utc::now(),
            total_agents: metrics.len(),
            running_agents,
            agents_with_errors,
            total_pnl,
            total_exposure,
            total_positions,
            total_open_orders: total_orders,
            recent_events_count: self.recent_events.read().len(),
        }
    }
}

impl Default for DashboardData {
    fn default() -> Self {
        Self::new()
    }
}

/// Dashboard summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Total number of agents
    pub total_agents: usize,
    /// Number of running agents
    pub running_agents: usize,
    /// Agents with errors
    pub agents_with_errors: usize,
    /// Total P&L across all agents
    pub total_pnl: f64,
    /// Total exposure across all agents
    pub total_exposure: f64,
    /// Total open positions
    pub total_positions: usize,
    /// Total open orders
    pub total_open_orders: usize,
    /// Number of recent events
    pub recent_events_count: usize,
}
