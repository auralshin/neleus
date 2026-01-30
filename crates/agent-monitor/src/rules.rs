//! Alert rules and evaluation

use crate::{AgentMetricsSnapshot, Alert, AlertSeverity};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Alert rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Unique rule ID
    pub id: String,
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: Option<String>,
    /// Condition to evaluate
    pub condition: AlertCondition,
    /// Alert severity when triggered
    pub severity: AlertSeverity,
    /// Cooldown period (seconds) before re-triggering
    #[serde(default = "default_cooldown")]
    pub cooldown_seconds: u64,
    /// Whether rule is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Agents this rule applies to (None = all)
    pub agent_filter: Option<Vec<String>>,
}

fn default_cooldown() -> u64 { 300 }
fn default_true() -> bool { true }

/// Alert condition types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AlertCondition {
    /// Threshold on a metric
    Threshold {
        metric: MetricPath,
        operator: ComparisonOperator,
        value: f64,
    },
    
    /// Rate of change threshold
    RateOfChange {
        metric: MetricPath,
        window_seconds: u64,
        operator: ComparisonOperator,
        value: f64,
    },
    
    /// Absolute change threshold
    AbsoluteChange {
        metric: MetricPath,
        window_seconds: u64,
        max_change: f64,
    },
    
    /// Compound condition (AND)
    And {
        conditions: Vec<AlertCondition>,
    },
    
    /// Compound condition (OR)
    Or {
        conditions: Vec<AlertCondition>,
    },
    
    /// No data received
    NoData {
        timeout_seconds: u64,
    },
}

/// Path to a metric value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricPath {
    // P&L metrics
    RealizedPnl,
    UnrealizedPnl,
    TotalPnl,
    DailyPnl,
    
    // Position metrics
    OpenPositions,
    TotalNotional,
    NetExposure,
    GrossExposure,
    
    // Order metrics
    OpenOrders,
    OrdersToday,
    FillRate,
    
    // Risk metrics
    CurrentDrawdown,
    MaxDrawdown,
    Var95,
    SharpeRatio,
    WinRate,
    RiskScore,
    DailyLossUsed,
    
    // Performance metrics
    TotalReturn,
    DailyReturn,
    Volatility,
    
    // System metrics
    Uptime,
    MemoryBytes,
    CpuPercent,
    EventLatencyMs,
    OrderLatencyMs,
    ErrorCount,
    WarningCount,
    
    /// Custom metric path
    Custom { path: String },
}

/// Comparison operators
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperator {
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equal,
    NotEqual,
}

impl ComparisonOperator {
    pub fn evaluate(&self, left: f64, right: f64) -> bool {
        match self {
            Self::GreaterThan => left > right,
            Self::GreaterThanOrEqual => left >= right,
            Self::LessThan => left < right,
            Self::LessThanOrEqual => left <= right,
            Self::Equal => (left - right).abs() < f64::EPSILON,
            Self::NotEqual => (left - right).abs() >= f64::EPSILON,
        }
    }
}

/// Rule evaluator
pub struct RuleEvaluator;

impl RuleEvaluator {
    /// Evaluate a rule against metrics
    pub fn evaluate(rule: &AlertRule, agent_id: &str, metrics: &AgentMetricsSnapshot) -> Option<Alert> {
        if !rule.enabled {
            return None;
        }
        
        // Check agent filter
        if let Some(ref filter) = rule.agent_filter {
            if !filter.contains(&agent_id.to_string()) {
                return None;
            }
        }
        
        // Evaluate condition
        if Self::evaluate_condition(&rule.condition, metrics) {
            Some(Alert {
                id: Uuid::new_v4().to_string(),
                rule_id: rule.id.clone(),
                agent_id: agent_id.to_string(),
                severity: rule.severity.clone(),
                message: Self::format_message(rule, metrics),
                timestamp: Utc::now(),
                resolved: false,
                resolved_at: None,
                metadata: std::collections::HashMap::new(),
            })
        } else {
            None
        }
    }
    
    fn evaluate_condition(condition: &AlertCondition, metrics: &AgentMetricsSnapshot) -> bool {
        match condition {
            AlertCondition::Threshold { metric, operator, value } => {
                if let Some(metric_value) = Self::get_metric_value(metric, metrics) {
                    operator.evaluate(metric_value, *value)
                } else {
                    false
                }
            }
            
            AlertCondition::RateOfChange { .. } => {
                // Would need historical data to evaluate
                false
            }
            
            AlertCondition::AbsoluteChange { .. } => {
                // Would need historical data to evaluate
                false
            }
            
            AlertCondition::And { conditions } => {
                conditions.iter().all(|c| Self::evaluate_condition(c, metrics))
            }
            
            AlertCondition::Or { conditions } => {
                conditions.iter().any(|c| Self::evaluate_condition(c, metrics))
            }
            
            AlertCondition::NoData { .. } => {
                // Handled separately in monitor
                false
            }
        }
    }
    
    fn get_metric_value(path: &MetricPath, metrics: &AgentMetricsSnapshot) -> Option<f64> {
        match path {
            MetricPath::RealizedPnl => Some(metrics.pnl.realized_pnl),
            MetricPath::UnrealizedPnl => Some(metrics.pnl.unrealized_pnl),
            MetricPath::TotalPnl => Some(metrics.pnl.total_pnl),
            MetricPath::DailyPnl => Some(metrics.pnl.daily_pnl),
            MetricPath::OpenPositions => Some(metrics.positions.open_positions as f64),
            MetricPath::TotalNotional => Some(metrics.positions.total_notional),
            MetricPath::NetExposure => Some(metrics.positions.net_exposure),
            MetricPath::GrossExposure => Some(metrics.positions.gross_exposure),
            MetricPath::OpenOrders => Some(metrics.orders.open_orders as f64),
            MetricPath::OrdersToday => Some(metrics.orders.orders_today as f64),
            MetricPath::FillRate => Some(metrics.orders.fill_rate),
            MetricPath::CurrentDrawdown => Some(metrics.risk.current_drawdown),
            MetricPath::MaxDrawdown => Some(metrics.risk.max_drawdown),
            MetricPath::Var95 => Some(metrics.risk.var_95),
            MetricPath::SharpeRatio => Some(metrics.risk.sharpe_ratio),
            MetricPath::WinRate => Some(metrics.risk.win_rate),
            MetricPath::RiskScore => Some(metrics.risk.risk_score),
            MetricPath::DailyLossUsed => Some(metrics.risk.daily_loss_used),
            MetricPath::TotalReturn => Some(metrics.performance.total_return),
            MetricPath::DailyReturn => Some(metrics.performance.daily_return),
            MetricPath::Volatility => Some(metrics.performance.volatility),
            MetricPath::Uptime => Some(metrics.system.uptime_seconds as f64),
            MetricPath::MemoryBytes => Some(metrics.system.memory_bytes as f64),
            MetricPath::CpuPercent => Some(metrics.system.cpu_percent as f64),
            MetricPath::EventLatencyMs => Some(metrics.system.event_latency_ms),
            MetricPath::OrderLatencyMs => Some(metrics.system.order_latency_ms),
            MetricPath::ErrorCount => Some(metrics.system.error_count as f64),
            MetricPath::WarningCount => Some(metrics.system.warning_count as f64),
            MetricPath::Custom { .. } => None, // Would need custom logic
        }
    }
    
    fn format_message(rule: &AlertRule, metrics: &AgentMetricsSnapshot) -> String {
        match &rule.condition {
            AlertCondition::Threshold { metric, operator, value } => {
                let actual = Self::get_metric_value(metric, metrics)
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "N/A".to_string());
                
                format!(
                    "{}: {:?} is {} (threshold: {:?} {:.2})",
                    rule.name, metric, actual, operator, value
                )
            }
            _ => rule.name.clone(),
        }
    }
}

/// Predefined alert rules
pub mod presets {
    use super::*;
    
    /// Create a drawdown alert rule
    pub fn max_drawdown_rule(threshold: f64) -> AlertRule {
        AlertRule {
            id: "max_drawdown".to_string(),
            name: "Maximum Drawdown Alert".to_string(),
            description: Some(format!("Alert when drawdown exceeds {:.1}%", threshold * 100.0)),
            condition: AlertCondition::Threshold {
                metric: MetricPath::CurrentDrawdown,
                operator: ComparisonOperator::GreaterThan,
                value: threshold,
            },
            severity: AlertSeverity::Critical,
            cooldown_seconds: 300,
            enabled: true,
            agent_filter: None,
        }
    }
    
    /// Create a daily loss limit rule
    pub fn daily_loss_limit_rule(threshold: f64) -> AlertRule {
        AlertRule {
            id: "daily_loss_limit".to_string(),
            name: "Daily Loss Limit Warning".to_string(),
            description: Some(format!("Alert when daily loss exceeds {:.1}%", threshold * 100.0)),
            condition: AlertCondition::Threshold {
                metric: MetricPath::DailyLossUsed,
                operator: ComparisonOperator::GreaterThan,
                value: threshold,
            },
            severity: AlertSeverity::High,
            cooldown_seconds: 600,
            enabled: true,
            agent_filter: None,
        }
    }
    
    /// Create a high latency alert rule
    pub fn high_latency_rule(threshold_ms: f64) -> AlertRule {
        AlertRule {
            id: "high_latency".to_string(),
            name: "High Order Latency".to_string(),
            description: Some(format!("Alert when order latency exceeds {}ms", threshold_ms)),
            condition: AlertCondition::Threshold {
                metric: MetricPath::OrderLatencyMs,
                operator: ComparisonOperator::GreaterThan,
                value: threshold_ms,
            },
            severity: AlertSeverity::Medium,
            cooldown_seconds: 60,
            enabled: true,
            agent_filter: None,
        }
    }
    
    /// Create error count alert rule
    pub fn error_count_rule(max_errors: u32) -> AlertRule {
        AlertRule {
            id: "error_count".to_string(),
            name: "High Error Count".to_string(),
            description: Some(format!("Alert when error count exceeds {}", max_errors)),
            condition: AlertCondition::Threshold {
                metric: MetricPath::ErrorCount,
                operator: ComparisonOperator::GreaterThan,
                value: max_errors as f64,
            },
            severity: AlertSeverity::High,
            cooldown_seconds: 300,
            enabled: true,
            agent_filter: None,
        }
    }
}
