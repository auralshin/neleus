//! Health checking for trading agents

use crate::{AgentId, AgentRegistry, HealthCheckConfig, OrchestratorError, Result};
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, timeout};

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Whether the check passed
    pub healthy: bool,
    /// Timestamp of the check
    pub timestamp: i64,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Details
    pub details: Option<String>,
}

/// Health probe trait - agents implement this
#[async_trait]
pub trait HealthProbe: Send + Sync {
    /// Check if the agent is healthy
    async fn check(&self) -> bool;
    
    /// Get the agent ID
    fn agent_id(&self) -> &str;
}

/// Health status for an agent
#[derive(Clone)]
struct AgentHealthStatus {
    probe: Arc<Box<dyn HealthProbe>>,
    consecutive_failures: u32,
    consecutive_successes: u32,
    last_result: Option<HealthCheckResult>,
    is_healthy: bool,
}

/// Health checker - monitors agent health
pub struct HealthChecker {
    config: HealthCheckConfig,
    agents: DashMap<AgentId, AgentHealthStatus>,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new(config: HealthCheckConfig) -> Self {
        Self {
            config,
            agents: DashMap::new(),
        }
    }
    
    /// Register an agent for health checking
    pub async fn register(&self, agent_id: AgentId, probe: Box<dyn HealthProbe>) {
        self.agents.insert(agent_id, AgentHealthStatus {
            probe: Arc::new(probe),
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_result: None,
            is_healthy: true,
        });
    }
    
    /// Unregister an agent
    pub async fn unregister(&self, agent_id: &str) {
        self.agents.remove(agent_id);
    }
    
    /// Check health of a specific agent
    pub async fn check_agent(&self, agent_id: &str) -> Result<HealthCheckResult> {
        let status = self.agents.get(agent_id)
            .ok_or_else(|| OrchestratorError::AgentNotFound(agent_id.to_string()))?;
        
        let start = std::time::Instant::now();
        let timeout_duration = Duration::from_secs(self.config.timeout_seconds);
        
        let healthy = match timeout(timeout_duration, status.probe.check()).await {
            Ok(result) => result,
            Err(_) => false, // Timeout
        };
        
        let latency_ms = start.elapsed().as_millis() as u64;
        
        Ok(HealthCheckResult {
            healthy,
            timestamp: chrono::Utc::now().timestamp(),
            latency_ms,
            details: None,
        })
    }
    
    /// Check health of all agents
    pub async fn check_all(&self) -> Vec<(AgentId, HealthCheckResult)> {
        let mut results = Vec::new();
        
        for entry in self.agents.iter() {
            let agent_id = entry.key().clone();
            if let Ok(result) = self.check_agent(&agent_id).await {
                results.push((agent_id, result));
            }
        }
        
        results
    }
    
    /// Run the health check loop
    pub async fn run_loop(&self, registry: Arc<AgentRegistry>) {
        let mut ticker = interval(Duration::from_secs(self.config.interval_seconds));
        
        loop {
            ticker.tick().await;
            
            let results = self.check_all().await;
            
            for (agent_id, result) in results {
                // Update status
                if let Some(mut status) = self.agents.get_mut(&agent_id) {
                    if result.healthy {
                        status.consecutive_failures = 0;
                        status.consecutive_successes += 1;
                        
                        if status.consecutive_successes >= self.config.success_threshold 
                            && !status.is_healthy 
                        {
                            status.is_healthy = true;
                            tracing::info!(
                                agent_id = %agent_id,
                                "Agent health recovered"
                            );
                        }
                    } else {
                        status.consecutive_successes = 0;
                        status.consecutive_failures += 1;
                        
                        if status.consecutive_failures >= self.config.failure_threshold 
                            && status.is_healthy 
                        {
                            status.is_healthy = false;
                            tracing::warn!(
                                agent_id = %agent_id,
                                failures = status.consecutive_failures,
                                "Agent marked unhealthy"
                            );
                            
                            // TODO: Trigger recovery action
                            // registry.trigger_recovery(&agent_id);
                        }
                    }
                    
                    status.last_result = Some(result);
                }
            }
        }
    }
    
    /// Get health status for an agent
    pub fn get_status(&self, agent_id: &str) -> Option<HealthCheckResult> {
        self.agents.get(agent_id).and_then(|s| s.last_result.clone())
    }
    
    /// Check if agent is currently healthy
    pub fn is_healthy(&self, agent_id: &str) -> bool {
        self.agents.get(agent_id).map(|s| s.is_healthy).unwrap_or(false)
    }
}

/// Extended health probe with detailed checks
#[async_trait]
pub trait DetailedHealthProbe: HealthProbe {
    /// Get detailed health information
    async fn detailed_check(&self) -> DetailedHealthResult;
}

/// Detailed health result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedHealthResult {
    /// Overall health
    pub healthy: bool,
    /// Individual component health
    pub components: Vec<ComponentHealth>,
    /// Metrics snapshot
    pub metrics: HealthMetrics,
}

/// Component health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,
    /// Whether component is healthy
    pub healthy: bool,
    /// Status message
    pub message: Option<String>,
}

/// Health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    /// Memory usage in bytes
    pub memory_bytes: u64,
    /// CPU usage percentage
    pub cpu_percent: f32,
    /// Event processing latency (ms)
    pub event_latency_ms: f64,
    /// Order queue depth
    pub order_queue_depth: u32,
    /// WebSocket connection status
    pub ws_connected: bool,
    /// Last heartbeat timestamp
    pub last_heartbeat: i64,
}
