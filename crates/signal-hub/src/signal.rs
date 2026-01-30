//! Trading signal types and structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique signal identifier
pub type SignalId = String;

/// Trading signal - the core data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Unique identifier
    pub id: SignalId,
    
    /// Source that generated this signal
    pub source_id: String,
    
    /// Signal type (e.g., "entry", "exit", "rebalance", "risk")
    pub signal_type: SignalType,
    
    /// Target instrument(s)
    pub instruments: Vec<String>,
    
    /// Signal direction
    pub direction: SignalDirection,
    
    /// Signal strength/confidence (0.0 to 1.0)
    pub strength: f64,
    
    /// Target price (optional)
    pub target_price: Option<f64>,
    
    /// Stop loss price (optional)
    pub stop_loss: Option<f64>,
    
    /// Take profit price (optional)
    pub take_profit: Option<f64>,
    
    /// Suggested position size (as fraction of capital)
    pub position_size: Option<f64>,
    
    /// Time-to-live in seconds (signal expires after this)
    pub ttl_seconds: Option<u64>,
    
    /// Signal timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Expiration time
    pub expires_at: Option<DateTime<Utc>>,
    
    /// Priority (higher = more urgent)
    pub priority: SignalPriority,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    
    /// Tags for filtering/routing
    pub tags: Vec<String>,
    
    /// Model/strategy that generated this (for AI signals)
    pub model_id: Option<String>,
    
    /// Model version
    pub model_version: Option<String>,
    
    /// Raw features/reasoning (for explainability)
    pub features: Option<SignalFeatures>,
}

impl Signal {
    /// Create a new signal
    pub fn new(source_id: String, signal_type: SignalType, direction: SignalDirection) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source_id,
            signal_type,
            instruments: Vec::new(),
            direction,
            strength: 1.0,
            target_price: None,
            stop_loss: None,
            take_profit: None,
            position_size: None,
            ttl_seconds: None,
            timestamp: Utc::now(),
            expires_at: None,
            priority: SignalPriority::Normal,
            metadata: HashMap::new(),
            tags: Vec::new(),
            model_id: None,
            model_version: None,
            features: None,
        }
    }
    
    /// Check if signal has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else if let Some(ttl) = self.ttl_seconds {
            let expires = self.timestamp + chrono::Duration::seconds(ttl as i64);
            Utc::now() > expires
        } else {
            false
        }
    }
    
    /// Builder pattern methods
    pub fn with_instrument(mut self, instrument: impl Into<String>) -> Self {
        self.instruments.push(instrument.into());
        self
    }
    
    pub fn with_instruments(mut self, instruments: Vec<String>) -> Self {
        self.instruments = instruments;
        self
    }
    
    pub fn with_strength(mut self, strength: f64) -> Self {
        self.strength = strength.clamp(0.0, 1.0);
        self
    }
    
    pub fn with_target_price(mut self, price: f64) -> Self {
        self.target_price = Some(price);
        self
    }
    
    pub fn with_stop_loss(mut self, price: f64) -> Self {
        self.stop_loss = Some(price);
        self
    }
    
    pub fn with_take_profit(mut self, price: f64) -> Self {
        self.take_profit = Some(price);
        self
    }
    
    pub fn with_position_size(mut self, size: f64) -> Self {
        self.position_size = Some(size);
        self
    }
    
    pub fn with_ttl(mut self, seconds: u64) -> Self {
        self.ttl_seconds = Some(seconds);
        self.expires_at = Some(self.timestamp + chrono::Duration::seconds(seconds as i64));
        self
    }
    
    pub fn with_priority(mut self, priority: SignalPriority) -> Self {
        self.priority = priority;
        self
    }
    
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
    
    pub fn with_model(mut self, model_id: impl Into<String>, version: impl Into<String>) -> Self {
        self.model_id = Some(model_id.into());
        self.model_version = Some(version.into());
        self
    }
}

/// Signal type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    /// Entry signal
    Entry,
    /// Exit signal
    Exit,
    /// Add to position
    ScaleIn,
    /// Reduce position
    ScaleOut,
    /// Rebalance portfolio
    Rebalance,
    /// Risk alert
    RiskAlert,
    /// Price target update
    PriceTarget,
    /// Sentiment update
    Sentiment,
    /// Custom signal type
    Custom,
}

impl Default for SignalType {
    fn default() -> Self {
        Self::Entry
    }
}

/// Signal direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalDirection {
    /// Go long / bullish
    Long,
    /// Go short / bearish
    Short,
    /// Close position / neutral
    Neutral,
    /// Direction determined by other factors
    Unspecified,
}

impl Default for SignalDirection {
    fn default() -> Self {
        Self::Unspecified
    }
}

/// Signal priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Urgent = 3,
    Critical = 4,
}

impl Default for SignalPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Signal features for explainability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalFeatures {
    /// Key features that contributed to signal
    pub key_features: HashMap<String, f64>,
    /// Human-readable reasoning
    pub reasoning: Option<String>,
    /// Confidence breakdown by factor
    pub confidence_breakdown: Option<HashMap<String, f64>>,
    /// Historical accuracy of this signal type
    pub historical_accuracy: Option<f64>,
    /// Similar historical signals
    pub similar_signals: Option<Vec<String>>,
}

/// Batch of signals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalBatch {
    /// Batch identifier
    pub batch_id: String,
    /// Signals in the batch
    pub signals: Vec<Signal>,
    /// Batch timestamp
    pub timestamp: DateTime<Utc>,
    /// Source that generated the batch
    pub source_id: String,
}

impl SignalBatch {
    /// Create a new signal batch
    pub fn new(source_id: impl Into<String>, signals: Vec<Signal>) -> Self {
        Self {
            batch_id: Uuid::new_v4().to_string(),
            signals,
            timestamp: Utc::now(),
            source_id: source_id.into(),
        }
    }
}

/// Signal query for historical lookup
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SignalQuery {
    /// Filter by source
    pub source_id: Option<String>,
    /// Filter by signal type
    pub signal_type: Option<SignalType>,
    /// Filter by instruments
    pub instruments: Option<Vec<String>>,
    /// Filter by direction
    pub direction: Option<SignalDirection>,
    /// Start time
    pub start_time: Option<DateTime<Utc>>,
    /// End time
    pub end_time: Option<DateTime<Utc>>,
    /// Minimum strength
    pub min_strength: Option<f64>,
    /// Filter by tags
    pub tags: Option<Vec<String>>,
    /// Filter by model
    pub model_id: Option<String>,
    /// Maximum results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

impl SignalQuery {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_source(mut self, source_id: impl Into<String>) -> Self {
        self.source_id = Some(source_id.into());
        self
    }
    
    pub fn with_signal_type(mut self, signal_type: SignalType) -> Self {
        self.signal_type = Some(signal_type);
        self
    }
    
    pub fn with_instrument(mut self, instrument: impl Into<String>) -> Self {
        let instruments = self.instruments.get_or_insert_with(Vec::new);
        instruments.push(instrument.into());
        self
    }
    
    pub fn with_time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }
    
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}
