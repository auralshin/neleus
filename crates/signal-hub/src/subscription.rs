//! Signal subscriptions

use crate::SignalType;
use serde::{Deserialize, Serialize};

/// Subscription identifier
pub type SubscriptionId = String;

/// Signal subscription - defines what signals an agent wants to receive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalSubscription {
    /// Filter by source IDs (None = all sources)
    pub source_ids: Option<Vec<String>>,
    
    /// Filter by signal types (None = all types)
    pub signal_types: Option<Vec<SignalType>>,
    
    /// Filter by instruments (None = all instruments)
    pub instruments: Option<Vec<String>>,
    
    /// Filter by tags (None = all tags)
    pub tags: Option<Vec<String>>,
    
    /// Minimum signal strength
    pub min_strength: Option<f64>,
    
    /// Whether to receive expired signals
    #[serde(default)]
    pub include_expired: bool,
    
    /// Whether this subscription is active
    #[serde(default = "default_true")]
    pub active: bool,
}

fn default_true() -> bool {
    true
}

impl Default for SignalSubscription {
    fn default() -> Self {
        Self {
            source_ids: None,
            signal_types: None,
            instruments: None,
            tags: None,
            min_strength: None,
            include_expired: false,
            active: true,
        }
    }
}

impl SignalSubscription {
    /// Create a new subscription for all signals
    pub fn all() -> Self {
        Self::default()
    }
    
    /// Create a subscription for specific sources
    pub fn for_sources(source_ids: Vec<String>) -> Self {
        Self {
            source_ids: Some(source_ids),
            ..Default::default()
        }
    }
    
    /// Create a subscription for specific instruments
    pub fn for_instruments(instruments: Vec<String>) -> Self {
        Self {
            instruments: Some(instruments),
            ..Default::default()
        }
    }
    
    /// Builder methods
    pub fn with_source(mut self, source_id: impl Into<String>) -> Self {
        self.source_ids
            .get_or_insert_with(Vec::new)
            .push(source_id.into());
        self
    }
    
    pub fn with_signal_type(mut self, signal_type: SignalType) -> Self {
        self.signal_types
            .get_or_insert_with(Vec::new)
            .push(signal_type);
        self
    }
    
    pub fn with_instrument(mut self, instrument: impl Into<String>) -> Self {
        self.instruments
            .get_or_insert_with(Vec::new)
            .push(instrument.into());
        self
    }
    
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags
            .get_or_insert_with(Vec::new)
            .push(tag.into());
        self
    }
    
    pub fn with_min_strength(mut self, strength: f64) -> Self {
        self.min_strength = Some(strength);
        self
    }
}
