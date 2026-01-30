//! Alert management and notifications

use crate::{AgentMetricsSnapshot, AlertRule, RuleEvaluator};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// Alert severity levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Alert instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Unique alert ID
    pub id: String,
    /// Rule that triggered this alert
    pub rule_id: String,
    /// Agent that triggered the alert
    pub agent_id: String,
    /// Severity level
    pub severity: AlertSeverity,
    /// Alert message
    pub message: String,
    /// When the alert was triggered
    pub timestamp: DateTime<Utc>,
    /// Whether alert has been resolved
    pub resolved: bool,
    /// When alert was resolved
    pub resolved_at: Option<DateTime<Utc>>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    /// Enable alerting
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Maximum active alerts to keep
    #[serde(default = "default_max_alerts")]
    pub max_active_alerts: usize,
    
    /// Alert history size
    #[serde(default = "default_history_size")]
    pub history_size: usize,
    
    /// Notification channels
    #[serde(default)]
    pub channels: Vec<NotificationChannel>,
    
    /// Default cooldown between same alerts (seconds)
    #[serde(default = "default_cooldown")]
    pub default_cooldown_seconds: u64,
}

impl Default for AlertingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_active_alerts: 1000,
            history_size: 10000,
            channels: vec![],
            default_cooldown_seconds: 300,
        }
    }
}

fn default_true() -> bool { true }
fn default_max_alerts() -> usize { 1000 }
fn default_history_size() -> usize { 10000 }
fn default_cooldown() -> u64 { 300 }

/// Notification channel types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NotificationChannel {
    /// Log to console/file
    Log,
    
    /// Webhook notification
    Webhook {
        url: String,
        secret: Option<String>,
    },
    
    /// Slack notification
    Slack {
        webhook_url: String,
        channel: Option<String>,
    },
    
    /// Discord notification
    Discord {
        webhook_url: String,
    },
    
    /// Telegram notification
    Telegram {
        bot_token: String,
        chat_id: String,
    },
    
    /// Email notification
    Email {
        smtp_host: String,
        smtp_port: u16,
        username: String,
        password: String,
        from: String,
        to: Vec<String>,
    },
}

/// Alert manager
pub struct AlertManager {
    config: AlertingConfig,
    rules: DashMap<String, AlertRule>,
    active_alerts: Arc<RwLock<HashMap<String, Alert>>>,
    alert_history: Arc<RwLock<VecDeque<Alert>>>,
    cooldowns: DashMap<String, DateTime<Utc>>,
}

impl AlertManager {
    /// Create a new alert manager
    pub fn new(config: AlertingConfig) -> Self {
        Self {
            config,
            rules: DashMap::new(),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(VecDeque::new())),
            cooldowns: DashMap::new(),
        }
    }
    
    /// Add an alert rule
    pub fn add_rule(&self, rule: AlertRule) {
        self.rules.insert(rule.id.clone(), rule);
    }
    
    /// Remove an alert rule
    pub fn remove_rule(&self, rule_id: &str) {
        self.rules.remove(rule_id);
    }
    
    /// Get all rules
    pub fn get_rules(&self) -> Vec<AlertRule> {
        self.rules.iter().map(|r| r.value().clone()).collect()
    }
    
    /// Evaluate all rules against metrics
    pub fn evaluate_rules(&self, agent_id: &str, metrics: &AgentMetricsSnapshot) -> Vec<Alert> {
        let mut alerts = Vec::new();
        
        for rule_entry in self.rules.iter() {
            let rule = rule_entry.value();
            
            // Check cooldown
            let cooldown_key = format!("{}:{}", rule.id, agent_id);
            if let Some(last_trigger) = self.cooldowns.get(&cooldown_key) {
                let elapsed = (Utc::now() - *last_trigger).num_seconds();
                if elapsed < rule.cooldown_seconds as i64 {
                    continue;
                }
            }
            
            // Evaluate rule
            if let Some(alert) = RuleEvaluator::evaluate(rule, agent_id, metrics) {
                self.cooldowns.insert(cooldown_key, Utc::now());
                alerts.push(alert);
            }
        }
        
        alerts
    }
    
    /// Trigger an alert
    pub fn trigger_alert(&self, alert: Alert) {
        tracing::warn!(
            alert_id = %alert.id,
            rule_id = %alert.rule_id,
            agent_id = %alert.agent_id,
            severity = ?alert.severity,
            message = %alert.message,
            "Alert triggered"
        );
        
        // Add to active alerts
        {
            let mut active = self.active_alerts.write();
            if active.len() >= self.config.max_active_alerts {
                // Remove oldest
                if let Some(oldest_key) = active.keys().next().cloned() {
                    active.remove(&oldest_key);
                }
            }
            active.insert(alert.id.clone(), alert.clone());
        }
        
        // Add to history
        {
            let mut history = self.alert_history.write();
            if history.len() >= self.config.history_size {
                history.pop_front();
            }
            history.push_back(alert.clone());
        }
        
        // Send notifications
        self.send_notifications(&alert);
    }
    
    /// Resolve an alert
    pub fn resolve_alert(&self, alert_id: &str) {
        if let Some(mut alert) = self.active_alerts.write().remove(alert_id) {
            alert.resolved = true;
            alert.resolved_at = Some(Utc::now());
            
            tracing::info!(
                alert_id = %alert_id,
                "Alert resolved"
            );
            
            // Update in history
            let mut history = self.alert_history.write();
            if let Some(hist_alert) = history.iter_mut().find(|a| a.id == alert_id) {
                hist_alert.resolved = true;
                hist_alert.resolved_at = alert.resolved_at;
            }
        }
    }
    
    /// Get active alerts
    pub fn get_active_alerts(&self) -> Vec<Alert> {
        self.active_alerts.read().values().cloned().collect()
    }
    
    /// Get active alerts for an agent
    pub fn get_agent_alerts(&self, agent_id: &str) -> Vec<Alert> {
        self.active_alerts.read()
            .values()
            .filter(|a| a.agent_id == agent_id)
            .cloned()
            .collect()
    }
    
    /// Get alert history
    pub fn get_history(&self, limit: usize) -> Vec<Alert> {
        self.alert_history.read()
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
    
    fn send_notifications(&self, alert: &Alert) {
        for channel in &self.config.channels {
            match channel {
                NotificationChannel::Log => {
                    // Already logged above
                }
                NotificationChannel::Webhook { url, .. } => {
                    let url = url.clone();
                    let alert = alert.clone();
                    tokio::spawn(async move {
                        Self::send_webhook_notification(&url, &alert).await;
                    });
                }
                NotificationChannel::Slack { webhook_url, .. } => {
                    let url = webhook_url.clone();
                    let alert = alert.clone();
                    tokio::spawn(async move {
                        Self::send_slack_notification(&url, &alert).await;
                    });
                }
                _ => {
                    // Other channels would be implemented similarly
                }
            }
        }
    }
    
    async fn send_webhook_notification(url: &str, alert: &Alert) {
        let client = reqwest::Client::new();
        if let Err(e) = client.post(url)
            .json(alert)
            .send()
            .await
        {
            tracing::error!(error = %e, "Failed to send webhook notification");
        }
    }
    
    async fn send_slack_notification(url: &str, alert: &Alert) {
        let severity_emoji = match alert.severity {
            AlertSeverity::Critical => "🔴",
            AlertSeverity::High => "🟠",
            AlertSeverity::Medium => "🟡",
            AlertSeverity::Low => "🟢",
        };
        
        let payload = serde_json::json!({
            "text": format!(
                "{} *{}*\nAgent: {}\n{}",
                severity_emoji,
                format!("{:?}", alert.severity).to_uppercase(),
                alert.agent_id,
                alert.message
            )
        });
        
        let client = reqwest::Client::new();
        if let Err(e) = client.post(url)
            .json(&payload)
            .send()
            .await
        {
            tracing::error!(error = %e, "Failed to send Slack notification");
        }
    }
}
