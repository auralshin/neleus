//! Metrics types and collection

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::CollectionConfig;

/// Metrics snapshot for an agent at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetricsSnapshot {
    /// Timestamp of the snapshot
    pub timestamp: DateTime<Utc>,
    
    /// Agent ID
    pub agent_id: String,
    
    /// P&L metrics
    pub pnl: PnlMetrics,
    
    /// Position metrics
    pub positions: PositionMetrics,
    
    /// Order metrics
    pub orders: OrderMetrics,
    
    /// Risk metrics
    pub risk: RiskMetrics,
    
    /// Performance metrics
    pub performance: PerformanceMetrics,
    
    /// System metrics
    pub system: SystemMetrics,
}

/// P&L metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PnlMetrics {
    /// Total realized P&L
    pub realized_pnl: f64,
    /// Total unrealized P&L
    pub unrealized_pnl: f64,
    /// Total P&L (realized + unrealized)
    pub total_pnl: f64,
    /// Daily P&L
    pub daily_pnl: f64,
    /// P&L per instrument
    pub pnl_by_instrument: HashMap<String, f64>,
    /// Fees paid
    pub total_fees: f64,
    /// Funding payments (for perpetuals)
    pub funding_paid: f64,
}

/// Position metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PositionMetrics {
    /// Number of open positions
    pub open_positions: u32,
    /// Total notional value
    pub total_notional: f64,
    /// Long exposure
    pub long_exposure: f64,
    /// Short exposure
    pub short_exposure: f64,
    /// Net exposure
    pub net_exposure: f64,
    /// Gross exposure
    pub gross_exposure: f64,
    /// Position details
    pub positions: Vec<PositionDetail>,
}

/// Individual position detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionDetail {
    /// Instrument
    pub instrument: String,
    /// Size (positive = long, negative = short)
    pub size: f64,
    /// Entry price
    pub entry_price: f64,
    /// Current price
    pub current_price: f64,
    /// Unrealized P&L
    pub unrealized_pnl: f64,
    /// Leverage
    pub leverage: f64,
    /// Liquidation price (if applicable)
    pub liquidation_price: Option<f64>,
}

/// Order metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrderMetrics {
    /// Number of open orders
    pub open_orders: u32,
    /// Orders placed today
    pub orders_today: u32,
    /// Orders filled today
    pub fills_today: u32,
    /// Order fill rate
    pub fill_rate: f64,
    /// Average fill time (ms)
    pub avg_fill_time_ms: f64,
    /// Orders by type
    pub orders_by_type: HashMap<String, u32>,
    /// Recent order events
    pub recent_orders: Vec<OrderEvent>,
}

/// Order event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderEvent {
    /// Order ID
    pub order_id: String,
    /// Instrument
    pub instrument: String,
    /// Side (buy/sell)
    pub side: String,
    /// Order type
    pub order_type: String,
    /// Price
    pub price: Option<f64>,
    /// Size
    pub size: f64,
    /// Status
    pub status: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Risk metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RiskMetrics {
    /// Current drawdown
    pub current_drawdown: f64,
    /// Maximum drawdown
    pub max_drawdown: f64,
    /// Value at Risk (VaR)
    pub var_95: f64,
    /// Sharpe ratio (rolling)
    pub sharpe_ratio: f64,
    /// Sortino ratio
    pub sortino_ratio: f64,
    /// Win rate
    pub win_rate: f64,
    /// Profit factor
    pub profit_factor: f64,
    /// Daily loss limit used
    pub daily_loss_used: f64,
    /// Daily loss limit remaining
    pub daily_loss_remaining: f64,
    /// Risk level (0-100)
    pub risk_score: f64,
}

/// Performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total return
    pub total_return: f64,
    /// Daily return
    pub daily_return: f64,
    /// Volatility (annualized)
    pub volatility: f64,
    /// Number of trades
    pub trade_count: u32,
    /// Average trade size
    pub avg_trade_size: f64,
    /// Average holding time (seconds)
    pub avg_holding_time_seconds: u64,
    /// Best trade
    pub best_trade: f64,
    /// Worst trade
    pub worst_trade: f64,
    /// Consecutive wins
    pub consecutive_wins: u32,
    /// Consecutive losses
    pub consecutive_losses: u32,
}

/// System metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// Agent uptime in seconds
    pub uptime_seconds: u64,
    /// Memory usage (bytes)
    pub memory_bytes: u64,
    /// CPU usage percentage
    pub cpu_percent: f32,
    /// Event processing latency (ms)
    pub event_latency_ms: f64,
    /// Order submission latency (ms)
    pub order_latency_ms: f64,
    /// WebSocket connection status
    pub ws_connected: bool,
    /// Last heartbeat
    pub last_heartbeat: DateTime<Utc>,
    /// Error count (last hour)
    pub error_count: u32,
    /// Warning count (last hour)
    pub warning_count: u32,
}

/// Metrics collector
pub struct MetricsCollector {
    config: CollectionConfig,
    storage: parking_lot::RwLock<MetricsStorage>,
}

struct MetricsStorage {
    metrics: HashMap<String, Vec<AgentMetricsSnapshot>>,
    events: HashMap<String, Vec<crate::AgentEvent>>,
    max_entries: usize,
}

impl MetricsCollector {
    pub fn new(config: CollectionConfig) -> Self {
        Self {
            config,
            storage: parking_lot::RwLock::new(MetricsStorage {
                metrics: HashMap::new(),
                events: HashMap::new(),
                max_entries: 10000,
            }),
        }
    }
    
    pub async fn record(&self, agent_id: &str, metrics: AgentMetricsSnapshot) {
        let mut storage = self.storage.write();
        let max_entries = storage.max_entries;
        
        let agent_metrics = storage.metrics.entry(agent_id.to_string()).or_insert_with(Vec::new);
        
        if agent_metrics.len() >= max_entries {
            agent_metrics.remove(0);
        }
        
        agent_metrics.push(metrics);
    }
    
    pub async fn record_event(&self, agent_id: &str, event: crate::AgentEvent) {
        let mut storage = self.storage.write();
        let max_entries = storage.max_entries;
        
        let agent_events = storage.events.entry(agent_id.to_string()).or_insert_with(Vec::new);
        
        if agent_events.len() >= max_entries {
            agent_events.remove(0);
        }
        
        agent_events.push(event);
    }
    
    pub fn get_metrics(&self, agent_id: &str, limit: usize) -> Vec<AgentMetricsSnapshot> {
        let storage = self.storage.read();
        
        storage.metrics.get(agent_id)
            .map(|m| m.iter().rev().take(limit).cloned().collect())
            .unwrap_or_default()
    }
    
    pub fn get_events(&self, agent_id: &str, limit: usize) -> Vec<crate::AgentEvent> {
        let storage = self.storage.read();
        
        storage.events.get(agent_id)
            .map(|e| e.iter().rev().take(limit).cloned().collect())
            .unwrap_or_default()
    }
}
