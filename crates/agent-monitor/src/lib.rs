//! # Neleus Agent Monitor
//!
//! Continuous monitoring and alerting for deployed trading agents.
//!
//! ## Features
//!
//! - **Real-time Metrics**: Track P&L, positions, orders, latency
//! - **Alerting**: Configurable alerts for thresholds and anomalies
//! - **Dashboards**: HTTP API for dashboard integration
//! - **Audit Trail**: Complete history of agent activities
//! - **Risk Monitoring**: Real-time risk metric tracking
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                         Agent Monitor                                │
//! │  ┌───────────────┐  ┌───────────────┐  ┌────────────────────────┐  │
//! │  │   Collector   │  │   Analyzer    │  │      Alerter           │  │
//! │  │ (Metrics/Logs)│─▶│ (Thresholds)  │─▶│ (Email/Slack/Webhook)  │  │
//! │  └───────────────┘  └───────────────┘  └────────────────────────┘  │
//! │           │                 │                       │               │
//! │           ▼                 ▼                       ▼               │
//! │  ┌─────────────────────────────────────────────────────────────┐   │
//! │  │                    Time-Series Store                         │   │
//! │  │   [Metrics] [Events] [Alerts] [Audit Log]                   │   │
//! │  └─────────────────────────────────────────────────────────────┘   │
//! │                              │                                      │
//! │                              ▼                                      │
//! │  ┌─────────────────────────────────────────────────────────────┐   │
//! │  │                    Dashboard API                             │   │
//! │  │   /metrics  /alerts  /agents  /history                      │   │
//! │  └─────────────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

pub mod alerts;
pub mod api;
pub mod collector;
pub mod dashboard;
pub mod metrics;
pub mod rules;

pub use alerts::*;
pub use collector::*;
pub use dashboard::*;
pub use metrics::*;
pub use rules::*;

/// Monitor errors
#[derive(Error, Debug)]
pub enum MonitorError {
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Metric not found: {0}")]
    MetricNotFound(String),

    #[error("Alert rule error: {0}")]
    AlertRuleError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, MonitorError>;

/// Agent Monitor - central monitoring service
pub struct AgentMonitor {
    /// Metrics collector
    collector: Arc<MetricsCollector>,
    /// Alert manager
    alerter: Arc<AlertManager>,
    /// Dashboard data
    dashboard: Arc<DashboardData>,
    /// Configuration
    config: MonitorConfig,
    /// Active agent subscriptions
    agents: DashMap<String, AgentMonitorState>,
}

/// State tracked for each monitored agent
#[derive(Debug, Clone)]
struct AgentMonitorState {
    agent_id: String,
    last_heartbeat: DateTime<Utc>,
    metrics_buffer: VecDeque<AgentMetricsSnapshot>,
    alert_state: HashMap<String, AlertState>,
}

#[derive(Debug, Clone)]
struct AlertState {
    rule_id: String,
    triggered: bool,
    triggered_at: Option<DateTime<Utc>>,
    trigger_count: u32,
}

impl AgentMonitor {
    /// Create a new agent monitor
    pub fn new(config: MonitorConfig) -> Self {
        let collector = Arc::new(MetricsCollector::new(config.collection.clone()));
        let alerter = Arc::new(AlertManager::new(config.alerting.clone()));
        let dashboard = Arc::new(DashboardData::new());

        Self {
            collector,
            alerter,
            dashboard,
            config,
            agents: DashMap::new(),
        }
    }

    /// Register an agent for monitoring
    pub fn register_agent(&self, agent_id: String) {
        self.agents.insert(
            agent_id.clone(),
            AgentMonitorState {
                agent_id,
                last_heartbeat: Utc::now(),
                metrics_buffer: VecDeque::with_capacity(1000),
                alert_state: HashMap::new(),
            },
        );
    }

    /// Unregister an agent
    pub fn unregister_agent(&self, agent_id: &str) {
        self.agents.remove(agent_id);
    }

    /// Record metrics for an agent
    pub async fn record_metrics(&self, agent_id: &str, metrics: AgentMetricsSnapshot) {
        // Update agent state
        if let Some(mut state) = self.agents.get_mut(agent_id) {
            state.last_heartbeat = Utc::now();

            // Add to buffer
            if state.metrics_buffer.len() >= 1000 {
                state.metrics_buffer.pop_front();
            }
            state.metrics_buffer.push_back(metrics.clone());
        }

        // Store in collector
        self.collector.record(agent_id, metrics.clone()).await;

        // Update dashboard
        self.dashboard.update_agent_metrics(agent_id, &metrics);

        // Check alert rules
        self.check_alerts(agent_id, &metrics).await;
    }

    /// Record an event for an agent
    pub async fn record_event(&self, agent_id: &str, event: AgentEvent) {
        self.collector.record_event(agent_id, event.clone()).await;
        self.dashboard.add_event(event);
    }

    /// Get current metrics for an agent
    pub fn get_agent_metrics(&self, agent_id: &str) -> Option<AgentMetricsSnapshot> {
        self.agents
            .get(agent_id)
            .and_then(|state| state.metrics_buffer.back().cloned())
    }

    /// Get metrics history for an agent
    pub fn get_agent_history(
        &self,
        agent_id: &str,
        limit: usize,
    ) -> Option<Vec<AgentMetricsSnapshot>> {
        self.agents.get(agent_id).map(|state| {
            state
                .metrics_buffer
                .iter()
                .rev()
                .take(limit)
                .cloned()
                .collect()
        })
    }

    /// Get all active alerts
    pub fn get_active_alerts(&self) -> Vec<Alert> {
        self.alerter.get_active_alerts()
    }

    /// Get dashboard summary
    pub fn get_dashboard_summary(&self) -> DashboardSummary {
        self.dashboard.get_summary()
    }

    /// Add an alert rule
    pub fn add_alert_rule(&self, rule: AlertRule) {
        self.alerter.add_rule(rule);
    }

    /// Remove an alert rule
    pub fn remove_alert_rule(&self, rule_id: &str) {
        self.alerter.remove_rule(rule_id);
    }

    /// Start the monitor background tasks
    pub async fn start(&self) -> Result<()> {
        // Start heartbeat checker
        let agents = self.agents.clone();
        let alerter = self.alerter.clone();
        let heartbeat_timeout = self.config.heartbeat_timeout_seconds;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));

            loop {
                interval.tick().await;

                let now = Utc::now();
                for entry in agents.iter() {
                    let state = entry.value();
                    let age = (now - state.last_heartbeat).num_seconds();

                    if age > heartbeat_timeout as i64 {
                        alerter.trigger_alert(Alert {
                            id: Uuid::new_v4().to_string(),
                            rule_id: "heartbeat_timeout".to_string(),
                            agent_id: state.agent_id.clone(),
                            severity: AlertSeverity::Critical,
                            message: format!(
                                "Agent {} heartbeat timeout ({} seconds)",
                                state.agent_id, age
                            ),
                            timestamp: now,
                            resolved: false,
                            resolved_at: None,
                            metadata: HashMap::new(),
                        });
                    }
                }
            }
        });

        tracing::info!("Agent monitor started");
        Ok(())
    }

    async fn check_alerts(&self, agent_id: &str, metrics: &AgentMetricsSnapshot) {
        let alerts = self.alerter.evaluate_rules(agent_id, metrics);

        for alert in alerts {
            self.alerter.trigger_alert(alert);
        }
    }
}

/// Monitor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorConfig {
    /// Metrics collection config
    #[serde(default)]
    pub collection: CollectionConfig,

    /// Alerting config
    #[serde(default)]
    pub alerting: AlertingConfig,

    /// Dashboard API config
    #[serde(default)]
    pub dashboard: DashboardConfig,

    /// Heartbeat timeout in seconds
    #[serde(default = "default_heartbeat_timeout")]
    pub heartbeat_timeout_seconds: u64,

    /// Metrics retention in hours
    #[serde(default = "default_retention")]
    pub retention_hours: u64,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            collection: CollectionConfig::default(),
            alerting: AlertingConfig::default(),
            dashboard: DashboardConfig::default(),
            heartbeat_timeout_seconds: default_heartbeat_timeout(),
            retention_hours: default_retention(),
        }
    }
}

fn default_heartbeat_timeout() -> u64 {
    60
}
fn default_retention() -> u64 {
    168
} // 7 days

/// Collection configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CollectionConfig {
    /// Collection interval in seconds
    #[serde(default = "default_interval")]
    pub interval_seconds: u64,

    /// Enable detailed metrics
    #[serde(default)]
    pub detailed_metrics: bool,

    /// Enable order tracking
    #[serde(default = "default_true")]
    pub track_orders: bool,

    /// Enable position tracking
    #[serde(default = "default_true")]
    pub track_positions: bool,
}

fn default_interval() -> u64 {
    5
}
fn default_true() -> bool {
    true
}

/// Dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// HTTP listen address
    #[serde(default = "default_dashboard_addr")]
    pub addr: String,

    /// Enable authentication
    #[serde(default)]
    pub require_auth: bool,

    /// CORS enabled
    #[serde(default = "default_true_fn")]
    pub enable_cors: bool,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            addr: default_dashboard_addr(),
            require_auth: false,
            enable_cors: true,
        }
    }
}

fn default_dashboard_addr() -> String {
    "0.0.0.0:8083".to_string()
}
fn default_true_fn() -> bool {
    true
}

/// Agent event for audit log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    /// Event ID
    pub id: String,
    /// Agent ID
    pub agent_id: String,
    /// Event type
    pub event_type: AgentEventType,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event details
    pub details: HashMap<String, serde_json::Value>,
}

/// Agent event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentEventType {
    Started,
    Stopped,
    Paused,
    Resumed,
    OrderPlaced,
    OrderFilled,
    OrderCancelled,
    PositionOpened,
    PositionClosed,
    Error,
    Warning,
    ConfigChanged,
    SignalReceived,
    RiskLimitTriggered,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_creation() {
        let config = MonitorConfig::default();
        let monitor = AgentMonitor::new(config);

        monitor.register_agent("test-agent".to_string());
        assert!(monitor.get_agent_metrics("test-agent").is_none());
    }
}
