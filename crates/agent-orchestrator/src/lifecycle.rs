//! Agent lifecycle management

use crate::{Agent, AgentId, AgentState, OrchestratorError, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Lifecycle manager handles agent state transitions and recovery
pub struct LifecycleManager {
    /// Maximum restart attempts
    max_restart_attempts: u32,
    /// Delay between restart attempts
    restart_delay: Duration,
    /// Graceful shutdown timeout
    shutdown_timeout: Duration,
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub fn new(
        max_restart_attempts: u32,
        restart_delay_seconds: u64,
        shutdown_timeout_seconds: u64,
    ) -> Self {
        Self {
            max_restart_attempts,
            restart_delay: Duration::from_secs(restart_delay_seconds),
            shutdown_timeout: Duration::from_secs(shutdown_timeout_seconds),
        }
    }
    
    /// Attempt to restart a failed agent
    pub async fn restart_agent(&self, agent: Arc<Agent>) -> Result<()> {
        let mut attempts = 0;
        
        while attempts < self.max_restart_attempts {
            attempts += 1;
            agent.increment_restart();
            
            tracing::info!(
                agent_id = %agent.id,
                attempt = attempts,
                max_attempts = self.max_restart_attempts,
                "Attempting to restart agent"
            );
            
            // Wait before restart
            if attempts > 1 {
                sleep(self.restart_delay).await;
            }
            
            // Try to initialize
            match self.try_restart(&agent).await {
                Ok(()) => {
                    tracing::info!(agent_id = %agent.id, "Agent restarted successfully");
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!(
                        agent_id = %agent.id,
                        attempt = attempts,
                        error = %e,
                        "Restart attempt failed"
                    );
                }
            }
        }
        
        tracing::error!(
            agent_id = %agent.id,
            attempts = attempts,
            "Agent restart failed after maximum attempts"
        );
        
        Err(OrchestratorError::DeploymentFailed(
            format!("Failed to restart after {} attempts", attempts)
        ))
    }
    
    async fn try_restart(&self, agent: &Agent) -> Result<()> {
        // Reset to initializing state
        agent.transition_to(AgentState::Initializing)?;
        
        // Initialize
        agent.initialize().await?;
        
        // Move to ready
        agent.transition_to(AgentState::Ready)?;
        
        // Start trading
        agent.transition_to(AgentState::Running)?;
        agent.start().await?;
        
        Ok(())
    }
    
    /// Graceful shutdown with timeout
    pub async fn graceful_shutdown(&self, agent: Arc<Agent>) -> Result<()> {
        agent.transition_to(AgentState::Stopping)?;
        
        let shutdown_future = agent.stop_gracefully();
        
        match tokio::time::timeout(self.shutdown_timeout, shutdown_future).await {
            Ok(result) => result,
            Err(_) => {
                tracing::warn!(
                    agent_id = %agent.id,
                    "Graceful shutdown timed out, forcing stop"
                );
                agent.stop_immediately().await
            }
        }
    }
}

/// Lifecycle events
#[derive(Debug, Clone)]
pub enum LifecycleEvent {
    /// Agent initialized successfully
    Initialized { agent_id: AgentId },
    /// Agent started
    Started { agent_id: AgentId },
    /// Agent stopped
    Stopped { agent_id: AgentId, graceful: bool },
    /// Agent restarted
    Restarted { agent_id: AgentId, attempt: u32 },
    /// Agent failed to restart
    RestartFailed { agent_id: AgentId, attempts: u32, error: String },
    /// Agent crashed
    Crashed { agent_id: AgentId, error: String },
}

/// Shutdown policy for agents
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShutdownPolicy {
    /// Keep positions open, only cancel orders
    KeepPositions,
    /// Close all positions before shutdown
    ClosePositions,
    /// Reduce positions to within limits
    ReducePositions { max_exposure: f64 },
    /// Immediate stop (emergency)
    Immediate,
}

impl Default for ShutdownPolicy {
    fn default() -> Self {
        Self::KeepPositions
    }
}
