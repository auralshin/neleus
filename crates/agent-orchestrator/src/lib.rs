//! # Neleus Agent Orchestrator
//!
//! CI/CD for trading agents - manages the full lifecycle of always-on trading bots.
//!
//! ## Features
//!
//! - **Agent Lifecycle**: Deploy, start, stop, restart, upgrade trading agents
//! - **Health Management**: Automatic health checks and recovery
//! - **Rolling Updates**: Zero-downtime strategy updates
//! - **State Persistence**: Agent state survives restarts
//! - **Multi-Agent**: Run multiple strategies concurrently
//! - **Scheduling**: Cron-based activation/deactivation windows
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Agent Orchestrator                       │
//! │  ┌───────────────┐  ┌───────────────┐  ┌────────────────┐  │
//! │  │ Agent Manager │  │ Health Checker│  │ State Persister│  │
//! │  └───────────────┘  └───────────────┘  └────────────────┘  │
//! │           │                  │                  │           │
//! │           ▼                  ▼                  ▼           │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │                   Agent Registry                     │   │
//! │  │   [Agent 1]  [Agent 2]  [Agent 3]  ...              │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

pub mod agent;
pub mod config;
pub mod health;
pub mod lifecycle;
pub mod registry;
pub mod scheduler;
pub mod state;

pub use agent::*;
pub use config::*;
pub use health::*;
pub use lifecycle::*;
pub use registry::*;
pub use scheduler::*;
pub use state::*;

/// Agent orchestrator errors
#[derive(Error, Debug)]
pub enum OrchestratorError {
    #[error("Agent not found: {0}")]
    AgentNotFound(AgentId),

    #[error("Agent already exists: {0}")]
    AgentAlreadyExists(AgentId),

    #[error("Invalid agent state transition: {from:?} -> {to:?}")]
    InvalidStateTransition { from: AgentState, to: AgentState },

    #[error("Agent deployment failed: {0}")]
    DeploymentFailed(String),

    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    #[error("State persistence error: {0}")]
    StatePersistenceError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, OrchestratorError>;

/// Unique identifier for a trading agent
pub type AgentId = String;

/// Agent execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    /// Agent is created but not yet initialized
    Created,
    /// Agent is initializing (loading config, connecting to venues)
    Initializing,
    /// Agent is ready to trade but not active
    Ready,
    /// Agent is actively trading
    Running,
    /// Agent is temporarily paused (positions may still be open)
    Paused,
    /// Agent is stopping gracefully (closing positions)
    Stopping,
    /// Agent has stopped
    Stopped,
    /// Agent encountered an error
    Error,
    /// Agent is being upgraded (hot-swap in progress)
    Upgrading,
}

impl AgentState {
    /// Check if agent is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, AgentState::Stopped | AgentState::Error)
    }

    /// Check if agent can accept new orders
    pub fn can_trade(&self) -> bool {
        matches!(self, AgentState::Running)
    }

    /// Valid state transitions
    pub fn can_transition_to(&self, target: AgentState) -> bool {
        use AgentState::*;
        matches!(
            (self, target),
            // Normal lifecycle
            (Created, Initializing) |
            (Initializing, Ready) |
            (Initializing, Error) |
            (Ready, Running) |
            (Running, Paused) |
            (Running, Stopping) |
            (Running, Error) |
            (Paused, Running) |
            (Paused, Stopping) |
            (Stopping, Stopped) |
            (Stopping, Error) |
            // Recovery
            (Error, Initializing) |
            (Stopped, Initializing) |
            // Upgrades
            (Running, Upgrading) |
            (Paused, Upgrading) |
            (Upgrading, Running) |
            (Upgrading, Error)
        )
    }
}

/// Summary statistics for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStats {
    /// Agent identifier
    pub agent_id: AgentId,
    /// Current state
    pub state: AgentState,
    /// Time agent was created
    pub created_at: DateTime<Utc>,
    /// Time agent started running (if applicable)
    pub started_at: Option<DateTime<Utc>>,
    /// Total runtime in seconds
    pub uptime_seconds: u64,
    /// Number of orders placed
    pub orders_placed: u64,
    /// Number of trades executed
    pub trades_executed: u64,
    /// Current P&L
    pub realized_pnl: f64,
    /// Unrealized P&L
    pub unrealized_pnl: f64,
    /// Last health check result
    pub last_health_check: Option<HealthCheckResult>,
    /// Number of restarts
    pub restart_count: u32,
    /// Last error message (if any)
    pub last_error: Option<String>,
}

/// Agent orchestrator - manages the lifecycle of trading agents
pub struct Orchestrator {
    /// Agent registry
    registry: Arc<AgentRegistry>,
    /// Health checker
    health_checker: Arc<HealthChecker>,
    /// State persister
    state_persister: Arc<dyn StatePersister>,
    /// Scheduler for agent windows
    scheduler: Arc<AgentScheduler>,
    /// Configuration
    config: OrchestratorConfig,
    /// Event listeners
    listeners: Arc<RwLock<Vec<Box<dyn OrchestratorListener>>>>,
}

impl Orchestrator {
    /// Create a new orchestrator with the given configuration
    pub fn new(config: OrchestratorConfig, persister: Arc<dyn StatePersister>) -> Self {
        let registry = Arc::new(AgentRegistry::new());
        let health_checker = Arc::new(HealthChecker::new(config.health_check.clone()));
        let scheduler = Arc::new(AgentScheduler::new());

        Self {
            registry,
            health_checker,
            state_persister: persister,
            scheduler,
            config,
            listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Deploy a new trading agent
    pub async fn deploy_agent(&self, spec: AgentSpec) -> Result<AgentId> {
        let agent_id = spec
            .agent_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // Check if agent already exists
        if self.registry.contains(&agent_id) {
            return Err(OrchestratorError::AgentAlreadyExists(agent_id));
        }

        // Create agent instance
        let agent = Agent::new(agent_id.clone(), spec)?;

        // Register agent
        self.registry.register(agent)?;

        // Persist state
        self.state_persister
            .save_agent_state(&agent_id, &AgentPersistedState::default())
            .await?;

        // Notify listeners
        self.notify_listeners(OrchestratorEvent::AgentDeployed {
            agent_id: agent_id.clone(),
        })
        .await;

        tracing::info!(agent_id = %agent_id, "Agent deployed successfully");

        Ok(agent_id)
    }

    /// Start a trading agent
    pub async fn start_agent(&self, agent_id: &str) -> Result<()> {
        let agent = self.registry.get_mut(agent_id)?;

        // Initialize if needed
        if agent.state() == AgentState::Created || agent.state() == AgentState::Stopped {
            agent.transition_to(AgentState::Initializing)?;
            agent.initialize().await?;
            agent.transition_to(AgentState::Ready)?;
        }

        // Start trading
        agent.transition_to(AgentState::Running)?;
        agent.start().await?;

        // Register with health checker
        self.health_checker
            .register(agent_id.to_string(), agent.health_probe())
            .await;

        // Update schedule if configured
        if let Some(schedule) = &agent.spec().schedule {
            self.scheduler
                .register(agent_id.to_string(), schedule.clone())
                .await;
        }

        self.notify_listeners(OrchestratorEvent::AgentStarted {
            agent_id: agent_id.to_string(),
        })
        .await;

        tracing::info!(agent_id = %agent_id, "Agent started");

        Ok(())
    }

    /// Stop a trading agent gracefully
    pub async fn stop_agent(&self, agent_id: &str, force: bool) -> Result<()> {
        let agent = self.registry.get_mut(agent_id)?;

        if !force {
            agent.transition_to(AgentState::Stopping)?;
            agent.stop_gracefully().await?;
        } else {
            agent.stop_immediately().await?;
        }

        agent.transition_to(AgentState::Stopped)?;

        // Unregister from health checker
        self.health_checker.unregister(agent_id).await;

        // Persist final state
        let state = agent.persisted_state();
        self.state_persister
            .save_agent_state(agent_id, &state)
            .await?;

        self.notify_listeners(OrchestratorEvent::AgentStopped {
            agent_id: agent_id.to_string(),
        })
        .await;

        tracing::info!(agent_id = %agent_id, "Agent stopped");

        Ok(())
    }

    /// Pause a trading agent (keeps positions, stops new trades)
    pub async fn pause_agent(&self, agent_id: &str) -> Result<()> {
        let agent = self.registry.get_mut(agent_id)?;
        agent.transition_to(AgentState::Paused)?;
        agent.pause().await?;

        self.notify_listeners(OrchestratorEvent::AgentPaused {
            agent_id: agent_id.to_string(),
        })
        .await;

        Ok(())
    }

    /// Resume a paused agent
    pub async fn resume_agent(&self, agent_id: &str) -> Result<()> {
        let agent = self.registry.get_mut(agent_id)?;
        agent.transition_to(AgentState::Running)?;
        agent.resume().await?;

        self.notify_listeners(OrchestratorEvent::AgentResumed {
            agent_id: agent_id.to_string(),
        })
        .await;

        Ok(())
    }

    /// Upgrade an agent to a new strategy version (rolling update)
    pub async fn upgrade_agent(&self, agent_id: &str, new_spec: AgentSpec) -> Result<()> {
        let agent = self.registry.get_mut(agent_id)?;

        let previous_state = agent.state();
        agent.transition_to(AgentState::Upgrading)?;

        // Perform hot-swap upgrade
        match agent.upgrade(new_spec).await {
            Ok(()) => {
                // Restore to previous state after upgrade
                if previous_state == AgentState::Running {
                    agent.transition_to(AgentState::Running)?;
                } else {
                    agent.transition_to(AgentState::Ready)?;
                }

                self.notify_listeners(OrchestratorEvent::AgentUpgraded {
                    agent_id: agent_id.to_string(),
                })
                .await;

                Ok(())
            }
            Err(e) => {
                agent.transition_to(AgentState::Error)?;
                Err(e)
            }
        }
    }

    /// Remove an agent from the orchestrator
    pub async fn remove_agent(&self, agent_id: &str) -> Result<()> {
        // Stop if running
        if let Ok(agent) = self.registry.get(agent_id) {
            if agent.state().can_trade() {
                self.stop_agent(agent_id, false).await?;
            }
        }

        // Remove from registry
        self.registry.unregister(agent_id)?;

        // Remove persisted state
        self.state_persister.delete_agent_state(agent_id).await?;

        self.notify_listeners(OrchestratorEvent::AgentRemoved {
            agent_id: agent_id.to_string(),
        })
        .await;

        Ok(())
    }

    /// Get agent statistics
    pub fn get_agent_stats(&self, agent_id: &str) -> Result<AgentStats> {
        let agent = self.registry.get(agent_id)?;
        Ok(agent.stats())
    }

    /// List all agents
    pub fn list_agents(&self) -> Vec<AgentStats> {
        self.registry
            .list_all()
            .into_iter()
            .map(|a| a.stats())
            .collect()
    }

    /// Get agent by ID
    pub fn get_agent(&self, agent_id: &str) -> Result<Arc<Agent>> {
        self.registry.get(agent_id).map(|a| a.clone())
    }

    /// Add an event listener
    pub fn add_listener(&self, listener: Box<dyn OrchestratorListener>) {
        self.listeners.write().push(listener);
    }

    /// Start the orchestrator background tasks
    pub async fn start(&self) -> Result<()> {
        // Load persisted agent states
        let states = self.state_persister.load_all_states().await?;
        for (agent_id, state) in states {
            if let Err(e) = self.restore_agent(&agent_id, state).await {
                tracing::warn!(agent_id = %agent_id, error = %e, "Failed to restore agent");
            }
        }

        // Start health checker background task
        let health_checker = self.health_checker.clone();
        let registry = self.registry.clone();
        tokio::spawn(async move {
            health_checker.run_loop(registry).await;
        });

        // Start scheduler background task
        let scheduler = self.scheduler.clone();
        let registry = self.registry.clone();
        tokio::spawn(async move {
            scheduler.run_loop(registry).await;
        });

        tracing::info!("Orchestrator started");

        Ok(())
    }

    /// Shutdown the orchestrator
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Shutting down orchestrator...");

        // Stop all agents gracefully
        let agents = self.registry.list_all();
        for agent in agents {
            if agent.state().can_trade() {
                if let Err(e) = self.stop_agent(&agent.id, false).await {
                    tracing::warn!(agent_id = %agent.id, error = %e, "Failed to stop agent");
                }
            }
        }

        Ok(())
    }

    async fn restore_agent(&self, agent_id: &str, state: AgentPersistedState) -> Result<()> {
        // Reconstruct agent from persisted state
        tracing::info!(agent_id = %agent_id, "Restoring agent from persisted state");

        if let Some(ref spec) = state.spec {
            let agent = Agent::from_persisted(agent_id.to_string(), spec.clone(), state.clone())?;
            self.registry.register(agent)?;

            // Auto-start if was running before shutdown
            if state.was_running {
                self.start_agent(agent_id).await?;
            }
        }

        Ok(())
    }

    async fn notify_listeners(&self, event: OrchestratorEvent) {
        let listeners = self.listeners.read();
        for listener in listeners.iter() {
            listener.on_event(&event).await;
        }
    }
}

/// Events emitted by the orchestrator
#[derive(Debug, Clone)]
pub enum OrchestratorEvent {
    AgentDeployed { agent_id: AgentId },
    AgentStarted { agent_id: AgentId },
    AgentStopped { agent_id: AgentId },
    AgentPaused { agent_id: AgentId },
    AgentResumed { agent_id: AgentId },
    AgentUpgraded { agent_id: AgentId },
    AgentRemoved { agent_id: AgentId },
    AgentError { agent_id: AgentId, error: String },
    HealthCheckFailed { agent_id: AgentId, reason: String },
}

/// Listener for orchestrator events
#[async_trait]
pub trait OrchestratorListener: Send + Sync {
    async fn on_event(&self, event: &OrchestratorEvent);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_state_transitions() {
        assert!(AgentState::Created.can_transition_to(AgentState::Initializing));
        assert!(AgentState::Running.can_transition_to(AgentState::Paused));
        assert!(!AgentState::Stopped.can_transition_to(AgentState::Running));
    }
}
