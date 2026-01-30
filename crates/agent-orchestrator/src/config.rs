//! Configuration types for the agent orchestrator

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Orchestrator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Health check configuration
    #[serde(default)]
    pub health_check: HealthCheckConfig,
    
    /// State persistence configuration
    #[serde(default)]
    pub persistence: PersistenceConfig,
    
    /// Maximum number of concurrent agents
    #[serde(default = "default_max_agents")]
    pub max_agents: usize,
    
    /// Auto-restart failed agents
    #[serde(default = "default_true")]
    pub auto_restart: bool,
    
    /// Maximum restart attempts before giving up
    #[serde(default = "default_max_restarts")]
    pub max_restart_attempts: u32,
    
    /// Delay between restart attempts
    #[serde(default = "default_restart_delay")]
    pub restart_delay_seconds: u64,
    
    /// Graceful shutdown timeout
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout_seconds: u64,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            health_check: HealthCheckConfig::default(),
            persistence: PersistenceConfig::default(),
            max_agents: default_max_agents(),
            auto_restart: true,
            max_restart_attempts: default_max_restarts(),
            restart_delay_seconds: default_restart_delay(),
            shutdown_timeout_seconds: default_shutdown_timeout(),
        }
    }
}

fn default_max_agents() -> usize { 100 }
fn default_true() -> bool { true }
fn default_max_restarts() -> u32 { 3 }
fn default_restart_delay() -> u64 { 5 }
fn default_shutdown_timeout() -> u64 { 30 }

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Interval between health checks
    #[serde(default = "default_health_interval")]
    pub interval_seconds: u64,
    
    /// Timeout for individual health check
    #[serde(default = "default_health_timeout")]
    pub timeout_seconds: u64,
    
    /// Number of consecutive failures before marking unhealthy
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,
    
    /// Number of consecutive successes before marking healthy
    #[serde(default = "default_success_threshold")]
    pub success_threshold: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            interval_seconds: default_health_interval(),
            timeout_seconds: default_health_timeout(),
            failure_threshold: default_failure_threshold(),
            success_threshold: default_success_threshold(),
        }
    }
}

fn default_health_interval() -> u64 { 30 }
fn default_health_timeout() -> u64 { 5 }
fn default_failure_threshold() -> u32 { 3 }
fn default_success_threshold() -> u32 { 1 }

/// State persistence configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    /// Backend type
    #[serde(default)]
    pub backend: PersistenceBackend,
    
    /// Path for file-based persistence
    #[serde(default)]
    pub path: Option<String>,
    
    /// Database connection string (if using database)
    #[serde(default)]
    pub connection_string: Option<String>,
    
    /// Sync interval for periodic state saves
    #[serde(default = "default_sync_interval")]
    pub sync_interval_seconds: u64,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            backend: PersistenceBackend::Memory,
            path: None,
            connection_string: None,
            sync_interval_seconds: default_sync_interval(),
        }
    }
}

fn default_sync_interval() -> u64 { 60 }

/// Persistence backend type
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceBackend {
    #[default]
    Memory,
    File,
    Sqlite,
    Postgres,
}

/// Agent specification - defines how to deploy a trading agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    /// Optional agent ID (auto-generated if not provided)
    pub agent_id: Option<String>,
    
    /// Human-readable name
    pub name: String,
    
    /// Strategy identifier (module path or name)
    pub strategy_id: String,
    
    /// Strategy configuration
    #[serde(default)]
    pub strategy_config: HashMap<String, serde_json::Value>,
    
    /// Venue configuration
    pub venue: VenueSpec,
    
    /// Instruments to trade
    pub instruments: Vec<String>,
    
    /// Risk limits
    #[serde(default)]
    pub risk_limits: RiskLimits,
    
    /// Capital allocation
    #[serde(default)]
    pub capital: CapitalSpec,
    
    /// Operating schedule (optional)
    #[serde(default)]
    pub schedule: Option<ScheduleSpec>,
    
    /// Deployment environment
    #[serde(default)]
    pub environment: EnvironmentSpec,
    
    /// Signal sources to subscribe to
    #[serde(default)]
    pub signal_sources: Vec<SignalSourceSpec>,
    
    /// Labels for organization
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

/// Venue configuration for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VenueSpec {
    Hyperliquid {
        network: String,
        wallet_address: Option<String>,
        #[serde(default)]
        use_vault: bool,
    },
    Lighter {
        network: String,
    },
    Polymarket {
        network: String,
    },
    Simulated {
        #[serde(default)]
        slippage_bps: f64,
        #[serde(default)]
        latency_ms: u64,
    },
}

/// Risk limits for an agent
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RiskLimits {
    /// Maximum position size per instrument
    #[serde(default)]
    pub max_position_size: Option<f64>,
    
    /// Maximum notional value per instrument
    #[serde(default)]
    pub max_notional: Option<f64>,
    
    /// Maximum total exposure
    #[serde(default)]
    pub max_total_exposure: Option<f64>,
    
    /// Maximum daily loss before auto-stop
    #[serde(default)]
    pub max_daily_loss: Option<f64>,
    
    /// Maximum drawdown before auto-stop
    #[serde(default)]
    pub max_drawdown_pct: Option<f64>,
    
    /// Maximum orders per second
    #[serde(default)]
    pub max_orders_per_second: Option<u32>,
    
    /// Maximum open orders
    #[serde(default)]
    pub max_open_orders: Option<u32>,
}

/// Capital allocation for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapitalSpec {
    /// Initial capital allocation
    pub initial: f64,
    
    /// Maximum capital (for scaling)
    #[serde(default)]
    pub maximum: Option<f64>,
    
    /// Default leverage
    #[serde(default = "default_leverage")]
    pub default_leverage: f64,
    
    /// Currency
    #[serde(default = "default_currency")]
    pub currency: String,
}

impl Default for CapitalSpec {
    fn default() -> Self {
        Self {
            initial: 10000.0,
            maximum: None,
            default_leverage: 1.0,
            currency: "USD".to_string(),
        }
    }
}

fn default_leverage() -> f64 { 1.0 }
fn default_currency() -> String { "USD".to_string() }

/// Operating schedule for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleSpec {
    /// Cron expression for when to start trading
    pub start: String,
    
    /// Cron expression for when to stop trading
    pub stop: String,
    
    /// Timezone for schedule (IANA format)
    #[serde(default = "default_timezone")]
    pub timezone: String,
    
    /// Holidays/blackout periods
    #[serde(default)]
    pub blackout_dates: Vec<String>,
}

fn default_timezone() -> String { "UTC".to_string() }

/// Deployment environment configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvironmentSpec {
    /// Environment name (dev, staging, production)
    #[serde(default = "default_env")]
    pub name: String,
    
    /// Whether to enable paper trading mode
    #[serde(default)]
    pub paper_trading: bool,
    
    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,
    
    /// Enable detailed telemetry
    #[serde(default)]
    pub telemetry_enabled: bool,
    
    /// Secrets reference (for TEE deployment)
    #[serde(default)]
    pub secrets_ref: Option<String>,
}

fn default_env() -> String { "development".to_string() }
fn default_log_level() -> String { "info".to_string() }

/// External signal source specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalSourceSpec {
    /// Source identifier
    pub id: String,
    
    /// Source type
    #[serde(flatten)]
    pub source_type: SignalSourceType,
    
    /// Signal filtering
    #[serde(default)]
    pub filters: HashMap<String, String>,
}

/// Types of signal sources
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SignalSourceType {
    /// HTTP webhook
    Webhook {
        /// Secret for authentication
        secret: Option<String>,
    },
    /// Redis pub/sub
    Redis {
        url: String,
        channel: String,
    },
    /// Kafka topic
    Kafka {
        brokers: Vec<String>,
        topic: String,
        group_id: String,
    },
    /// gRPC stream
    Grpc {
        endpoint: String,
    },
    /// Internal signal hub
    SignalHub {
        signal_type: String,
    },
}
