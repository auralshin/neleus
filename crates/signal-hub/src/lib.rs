//! # Neleus Signal Hub
//!
//! Centralized hub for receiving, validating, and distributing trading signals
//! from external AI/ML models, quant systems, and other signal sources.
//!
//! ## Features
//!
//! - **Multi-Source Integration**: HTTP webhooks, WebSocket streams, gRPC, Redis, Kafka
//! - **Signal Validation**: Schema validation, rate limiting, authentication
//! - **Signal Routing**: Route signals to specific agents based on subscriptions
//! - **Signal Transformation**: Normalize signals from different sources
//! - **Audit Trail**: Log all signals for compliance and debugging
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                         External Signal Sources                      │
//! │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐   │
//! │  │ AI/ML   │  │ Quant   │  │ On-Chain│  │ Social  │  │ Custom  │   │
//! │  │ Models  │  │ Systems │  │ Oracle  │  │ Signals │  │ Webhook │   │
//! │  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘   │
//! └───────┼────────────┼────────────┼────────────┼────────────┼────────┘
//!         │            │            │            │            │
//!         ▼            ▼            ▼            ▼            ▼
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                          Signal Hub                                  │
//! │  ┌───────────────┐  ┌───────────────┐  ┌───────────────────────┐   │
//! │  │   Receivers   │  │   Validator   │  │      Router           │   │
//! │  │ HTTP/WS/gRPC  │─▶│ Auth + Schema │─▶│ Agent Subscriptions   │   │
//! │  └───────────────┘  └───────────────┘  └───────────────────────┘   │
//! │                             │                      │                │
//! │                             ▼                      ▼                │
//! │  ┌───────────────────────────────────────────────────────────┐     │
//! │  │                    Signal Store                           │     │
//! │  │   (Time-series storage for replay and analytics)          │     │
//! │  └───────────────────────────────────────────────────────────┘     │
//! └─────────────────────────────────────────────────────────────────────┘
//!                                    │
//!                                    ▼
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                         Trading Agents                               │
//! │     [Agent 1]        [Agent 2]        [Agent 3]       ...           │
//! └─────────────────────────────────────────────────────────────────────┘
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

pub mod receiver;
pub mod router;
pub mod signal;
pub mod store;
pub mod subscription;
pub mod transformer;
pub mod validator;

pub use receiver::*;
pub use router::*;
pub use signal::*;
pub use store::*;
pub use subscription::*;
pub use transformer::*;
pub use validator::*;

/// Signal Hub errors
#[derive(Error, Debug)]
pub enum SignalHubError {
    #[error("Invalid signal: {0}")]
    InvalidSignal(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Source not found: {0}")]
    SourceNotFound(String),

    #[error("Subscription not found: {0}")]
    SubscriptionNotFound(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Receiver error: {0}")]
    ReceiverError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, SignalHubError>;

/// Signal Hub - central coordination point for all external signals
pub struct SignalHub {
    /// Signal validator
    validator: Arc<SignalValidator>,
    /// Signal router
    router: Arc<SignalRouter>,
    /// Signal store
    store: Arc<dyn SignalStore>,
    /// Registered sources
    sources: DashMap<String, SignalSourceConfig>,
    /// Active receivers
    receivers: DashMap<String, Arc<dyn SignalReceiver>>,
    /// Configuration
    config: SignalHubConfig,
    /// Metrics
    metrics: Arc<RwLock<SignalHubMetrics>>,
}

impl SignalHub {
    /// Create a new signal hub
    pub fn new(config: SignalHubConfig, store: Arc<dyn SignalStore>) -> Self {
        Self {
            validator: Arc::new(SignalValidator::new(config.validation.clone())),
            router: Arc::new(SignalRouter::new()),
            store,
            sources: DashMap::new(),
            receivers: DashMap::new(),
            config,
            metrics: Arc::new(RwLock::new(SignalHubMetrics::default())),
        }
    }

    /// Register a signal source
    pub fn register_source(&self, source_id: String, config: SignalSourceConfig) -> Result<()> {
        tracing::info!(source_id = %source_id, "Registering signal source");
        self.sources.insert(source_id, config);
        Ok(())
    }

    /// Unregister a signal source
    pub fn unregister_source(&self, source_id: &str) -> Result<()> {
        self.sources
            .remove(source_id)
            .ok_or_else(|| SignalHubError::SourceNotFound(source_id.to_string()))?;
        Ok(())
    }

    /// Process an incoming signal
    pub async fn process_signal(&self, mut signal: Signal) -> Result<SignalId> {
        let signal_id = signal.id.clone();
        let start = std::time::Instant::now();

        // Validate source
        let source_config = self
            .sources
            .get(&signal.source_id)
            .ok_or_else(|| SignalHubError::SourceNotFound(signal.source_id.clone()))?;

        // Authenticate
        if !self.authenticate_signal(&signal, &source_config)? {
            return Err(SignalHubError::AuthenticationFailed(
                "Invalid credentials".to_string(),
            ));
        }

        // Validate signal
        self.validator.validate(&signal)?;

        // Apply transformations
        if let Some(transforms) = &source_config.transformations {
            signal = self.apply_transformations(signal, transforms)?;
        }

        // Store signal
        self.store.store(&signal).await?;

        // Route to subscribers
        let routed_count = self.router.route(&signal).await?;

        // Update metrics
        {
            let mut metrics = self.metrics.write();
            metrics.signals_received += 1;
            metrics.signals_routed += routed_count as u64;
            metrics.last_signal_time = Some(Utc::now());
            metrics.avg_latency_ms = (metrics.avg_latency_ms + start.elapsed().as_millis() as f64) / 2.0;
        }

        tracing::debug!(
            signal_id = %signal_id,
            source = %signal.source_id,
            routed_to = routed_count,
            "Signal processed"
        );

        Ok(signal_id)
    }

    /// Subscribe an agent to signals
    pub fn subscribe(
        &self,
        agent_id: String,
        subscription: SignalSubscription,
    ) -> Result<SubscriptionId> {
        self.router.add_subscription(agent_id, subscription)
    }

    /// Unsubscribe an agent
    pub fn unsubscribe(&self, subscription_id: &SubscriptionId) -> Result<()> {
        self.router.remove_subscription(subscription_id)
    }

    /// Query historical signals
    pub async fn query_signals(&self, query: SignalQuery) -> Result<Vec<Signal>> {
        self.store.query(query).await
    }

    /// Get signal hub metrics
    pub fn metrics(&self) -> SignalHubMetrics {
        self.metrics.read().clone()
    }

    /// Start HTTP receiver
    pub async fn start_http_receiver(&self, addr: std::net::SocketAddr) -> Result<()> {
        let receiver = HttpSignalReceiver::new(self.clone());
        receiver.start(addr).await
    }

    fn authenticate_signal(&self, signal: &Signal, config: &SignalSourceConfig) -> Result<bool> {
        // Check authentication based on source config
        match &config.auth {
            SignalAuth::None => Ok(true),
            SignalAuth::ApiKey { key } => {
                Ok(signal.metadata.get("api_key") == Some(key))
            }
            SignalAuth::Hmac { secret } => {
                // Verify HMAC signature
                let _ = secret; // TODO: Implement HMAC verification
                Ok(true)
            }
            SignalAuth::Jwt { public_key } => {
                // Verify JWT
                let _ = public_key; // TODO: Implement JWT verification
                Ok(true)
            }
        }
    }

    fn apply_transformations(
        &self,
        mut signal: Signal,
        transforms: &[SignalTransformation],
    ) -> Result<Signal> {
        for transform in transforms {
            signal = apply_transformation(signal, transform)?;
        }
        Ok(signal)
    }
}

// Manual Clone implementation since we can't derive it
impl Clone for SignalHub {
    fn clone(&self) -> Self {
        Self {
            validator: self.validator.clone(),
            router: self.router.clone(),
            store: self.store.clone(),
            sources: self.sources.clone(),
            receivers: self.receivers.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

/// Signal Hub configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalHubConfig {
    /// HTTP receiver config
    #[serde(default)]
    pub http: HttpReceiverConfig,

    /// Validation config
    #[serde(default)]
    pub validation: ValidationConfig,

    /// Storage config
    #[serde(default)]
    pub storage: StorageConfig,

    /// Rate limiting
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
}

impl Default for SignalHubConfig {
    fn default() -> Self {
        Self {
            http: HttpReceiverConfig::default(),
            validation: ValidationConfig::default(),
            storage: StorageConfig::default(),
            rate_limit: RateLimitConfig::default(),
        }
    }
}

/// HTTP receiver configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpReceiverConfig {
    /// Listen address
    #[serde(default = "default_http_addr")]
    pub addr: String,
    /// Enable CORS
    #[serde(default)]
    pub enable_cors: bool,
}

impl Default for HttpReceiverConfig {
    fn default() -> Self {
        Self {
            addr: default_http_addr(),
            enable_cors: true,
        }
    }
}

fn default_http_addr() -> String {
    "0.0.0.0:8082".to_string()
}

/// Validation configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Require timestamp within window
    #[serde(default)]
    pub max_age_seconds: Option<u64>,
    /// Reject duplicate signals
    #[serde(default)]
    pub reject_duplicates: bool,
    /// Required fields
    #[serde(default)]
    pub required_fields: Vec<String>,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Backend type
    #[serde(default)]
    pub backend: StorageBackend,
    /// Retention period in days
    #[serde(default = "default_retention")]
    pub retention_days: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: StorageBackend::Memory,
            retention_days: default_retention(),
        }
    }
}

fn default_retention() -> u32 {
    30
}

/// Storage backend type
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageBackend {
    #[default]
    Memory,
    File,
    Sqlite,
    Timescale,
}

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum signals per second per source
    #[serde(default = "default_rate")]
    pub max_per_second: u32,
    /// Burst allowance
    #[serde(default = "default_burst")]
    pub burst: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_per_second: default_rate(),
            burst: default_burst(),
        }
    }
}

fn default_rate() -> u32 {
    100
}
fn default_burst() -> u32 {
    50
}

/// Signal source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalSourceConfig {
    /// Source name
    pub name: String,
    /// Source type
    pub source_type: SignalSourceType,
    /// Authentication
    #[serde(default)]
    pub auth: SignalAuth,
    /// Signal transformations
    #[serde(default)]
    pub transformations: Option<Vec<SignalTransformation>>,
    /// Rate limit override
    #[serde(default)]
    pub rate_limit: Option<RateLimitConfig>,
    /// Allowed signal types
    #[serde(default)]
    pub allowed_signal_types: Vec<String>,
}

/// Signal source type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalSourceType {
    /// AI/ML model endpoint
    AiModel,
    /// Quantitative trading system
    QuantSystem,
    /// On-chain oracle
    OnChainOracle,
    /// Social/sentiment analysis
    SocialSignal,
    /// Custom webhook
    CustomWebhook,
    /// Internal (from strategy)
    Internal,
}

/// Signal authentication method
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignalAuth {
    #[default]
    None,
    ApiKey {
        key: String,
    },
    Hmac {
        secret: String,
    },
    Jwt {
        public_key: String,
    },
}

/// Signal Hub metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SignalHubMetrics {
    /// Total signals received
    pub signals_received: u64,
    /// Total signals routed to agents
    pub signals_routed: u64,
    /// Signals rejected (validation failed)
    pub signals_rejected: u64,
    /// Active subscriptions
    pub active_subscriptions: u64,
    /// Registered sources
    pub registered_sources: u64,
    /// Last signal time
    pub last_signal_time: Option<DateTime<Utc>>,
    /// Average processing latency (ms)
    pub avg_latency_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_signal_hub_creation() {
        let config = SignalHubConfig::default();
        let store = Arc::new(MemorySignalStore::new());
        let hub = SignalHub::new(config, store);

        assert_eq!(hub.metrics().signals_received, 0);
    }
}
