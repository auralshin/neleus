//! Trading agent implementation

use crate::{
    AgentId, AgentPersistedState, AgentSpec, AgentState, AgentStats, HealthProbe,
    OrchestratorError, Result,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Trading agent - wraps a strategy and manages its execution
pub struct Agent {
    /// Unique identifier
    pub id: AgentId,
    /// Agent specification
    spec: RwLock<AgentSpec>,
    /// Current state
    state: RwLock<AgentState>,
    /// Runtime metrics
    metrics: RwLock<AgentMetrics>,
    /// Creation timestamp
    created_at: DateTime<Utc>,
    /// Last state change timestamp
    last_state_change: RwLock<DateTime<Utc>>,
    /// Error information
    last_error: RwLock<Option<String>>,
    /// Restart count
    restart_count: RwLock<u32>,
}

#[derive(Debug, Clone, Default)]
struct AgentMetrics {
    started_at: Option<DateTime<Utc>>,
    orders_placed: u64,
    trades_executed: u64,
    realized_pnl: f64,
    unrealized_pnl: f64,
    position_count: u32,
}

impl Agent {
    /// Create a new agent from a specification
    pub fn new(id: AgentId, spec: AgentSpec) -> Result<Self> {
        Ok(Self {
            id,
            spec: RwLock::new(spec),
            state: RwLock::new(AgentState::Created),
            metrics: RwLock::new(AgentMetrics::default()),
            created_at: Utc::now(),
            last_state_change: RwLock::new(Utc::now()),
            last_error: RwLock::new(None),
            restart_count: RwLock::new(0),
        })
    }

    /// Reconstruct agent from persisted state
    pub fn from_persisted(
        id: AgentId,
        spec: AgentSpec,
        state: AgentPersistedState,
    ) -> Result<Self> {
        let agent = Self::new(id, spec)?;

        // Restore metrics from persisted state
        {
            let mut metrics = agent.metrics.write();
            metrics.realized_pnl = state.realized_pnl;
            metrics.orders_placed = state.orders_placed;
            metrics.trades_executed = state.trades_executed;
        }

        *agent.restart_count.write() = state.restart_count;

        Ok(agent)
    }

    /// Get current state
    pub fn state(&self) -> AgentState {
        *self.state.read()
    }

    /// Get agent specification
    pub fn spec(&self) -> AgentSpec {
        self.spec.read().clone()
    }

    /// Transition to a new state
    pub fn transition_to(&self, target: AgentState) -> Result<()> {
        let mut state = self.state.write();

        if !state.can_transition_to(target) {
            return Err(OrchestratorError::InvalidStateTransition {
                from: *state,
                to: target,
            });
        }

        tracing::debug!(
            agent_id = %self.id,
            from = ?*state,
            to = ?target,
            "Agent state transition"
        );

        *state = target;
        *self.last_state_change.write() = Utc::now();

        // Clear error on successful transition to non-error state
        if target != AgentState::Error {
            *self.last_error.write() = None;
        }

        Ok(())
    }

    /// Initialize the agent (load config, connect to venues)
    pub async fn initialize(&self) -> Result<()> {
        let spec = self.spec.read().clone();

        tracing::info!(
            agent_id = %self.id,
            strategy = %spec.strategy_id,
            "Initializing agent"
        );

        // TODO: Initialize venue connections
        // TODO: Load strategy module
        // TODO: Subscribe to market data

        Ok(())
    }

    /// Start trading
    pub async fn start(&self) -> Result<()> {
        {
            let mut metrics = self.metrics.write();
            metrics.started_at = Some(Utc::now());
        }

        tracing::info!(agent_id = %self.id, "Agent started trading");

        // TODO: Start the trading event loop

        Ok(())
    }

    /// Stop gracefully (close positions, cancel orders)
    pub async fn stop_gracefully(&self) -> Result<()> {
        tracing::info!(agent_id = %self.id, "Stopping agent gracefully");

        // TODO: Cancel all open orders
        // TODO: Close all positions (optional based on config)
        // TODO: Wait for confirmations

        Ok(())
    }

    /// Stop immediately (emergency stop)
    pub async fn stop_immediately(&self) -> Result<()> {
        tracing::warn!(agent_id = %self.id, "Emergency stop agent");

        // TODO: Cancel all orders immediately
        // TODO: Close positions at market

        Ok(())
    }

    /// Pause trading (keep positions, stop new trades)
    pub async fn pause(&self) -> Result<()> {
        tracing::info!(agent_id = %self.id, "Agent paused");
        Ok(())
    }

    /// Resume trading
    pub async fn resume(&self) -> Result<()> {
        tracing::info!(agent_id = %self.id, "Agent resumed");
        Ok(())
    }

    /// Upgrade to new specification (hot-swap)
    pub async fn upgrade(&self, new_spec: AgentSpec) -> Result<()> {
        tracing::info!(agent_id = %self.id, "Upgrading agent");

        // Validate new spec
        // TODO: More thorough validation

        // Swap specification
        *self.spec.write() = new_spec;

        // Reload strategy
        // TODO: Hot-swap strategy code

        Ok(())
    }

    /// Get agent statistics
    pub fn stats(&self) -> AgentStats {
        let metrics = self.metrics.read();
        let state = *self.state.read();

        let uptime_seconds = if let Some(started) = metrics.started_at {
            if state == AgentState::Running {
                (Utc::now() - started).num_seconds() as u64
            } else {
                0
            }
        } else {
            0
        };

        AgentStats {
            agent_id: self.id.clone(),
            state,
            created_at: self.created_at,
            started_at: metrics.started_at,
            uptime_seconds,
            orders_placed: metrics.orders_placed,
            trades_executed: metrics.trades_executed,
            realized_pnl: metrics.realized_pnl,
            unrealized_pnl: metrics.unrealized_pnl,
            last_health_check: None,
            restart_count: *self.restart_count.read(),
            last_error: self.last_error.read().clone(),
        }
    }

    /// Get persisted state for saving
    pub fn persisted_state(&self) -> AgentPersistedState {
        let metrics = self.metrics.read();
        let state = *self.state.read();

        AgentPersistedState {
            spec: Some(self.spec.read().clone()),
            was_running: state == AgentState::Running,
            realized_pnl: metrics.realized_pnl,
            orders_placed: metrics.orders_placed,
            trades_executed: metrics.trades_executed,
            restart_count: *self.restart_count.read(),
            positions: vec![], // TODO: Serialize positions
        }
    }

    /// Get health probe for this agent
    pub fn health_probe(&self) -> Box<dyn HealthProbe> {
        Box::new(AgentHealthProbe {
            agent_id: self.id.clone(),
            state: Arc::new(self.state.read().clone()),
        })
    }

    /// Record an order placed
    pub fn record_order(&self) {
        self.metrics.write().orders_placed += 1;
    }

    /// Record a trade executed
    pub fn record_trade(&self, pnl: f64) {
        let mut metrics = self.metrics.write();
        metrics.trades_executed += 1;
        metrics.realized_pnl += pnl;
    }

    /// Update unrealized P&L
    pub fn update_unrealized_pnl(&self, pnl: f64) {
        self.metrics.write().unrealized_pnl = pnl;
    }

    /// Set error state with message
    pub fn set_error(&self, error: String) {
        *self.last_error.write() = Some(error);
        let _ = self.transition_to(AgentState::Error);
    }

    /// Increment restart counter
    pub fn increment_restart(&self) {
        *self.restart_count.write() += 1;
    }
}

/// Health probe implementation for an agent
struct AgentHealthProbe {
    agent_id: AgentId,
    state: Arc<AgentState>,
}

#[async_trait]
impl HealthProbe for AgentHealthProbe {
    async fn check(&self) -> bool {
        // Basic health: agent should be in a healthy state
        !matches!(*self.state, AgentState::Error | AgentState::Stopped)
    }

    fn agent_id(&self) -> &str {
        &self.agent_id
    }
}
