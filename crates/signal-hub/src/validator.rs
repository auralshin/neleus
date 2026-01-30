//! Signal validation

use crate::{Result, Signal, SignalHubError, ValidationConfig};
use chrono::Utc;
use std::collections::HashSet;

/// Signal validator
pub struct SignalValidator {
    config: ValidationConfig,
    seen_ids: parking_lot::RwLock<HashSet<String>>,
}

impl SignalValidator {
    /// Create a new validator
    pub fn new(config: ValidationConfig) -> Self {
        Self {
            config,
            seen_ids: parking_lot::RwLock::new(HashSet::new()),
        }
    }
    
    /// Validate a signal
    pub fn validate(&self, signal: &Signal) -> Result<()> {
        // Check for duplicate
        if self.config.reject_duplicates {
            let mut seen = self.seen_ids.write();
            if seen.contains(&signal.id) {
                return Err(SignalHubError::InvalidSignal(
                    format!("Duplicate signal ID: {}", signal.id)
                ));
            }
            seen.insert(signal.id.clone());
            
            // Limit size of seen set
            if seen.len() > 100_000 {
                // Clear oldest entries (simple approach)
                seen.clear();
            }
        }
        
        // Check signal age
        if let Some(max_age) = self.config.max_age_seconds {
            let age = Utc::now() - signal.timestamp;
            if age.num_seconds() > max_age as i64 {
                return Err(SignalHubError::InvalidSignal(
                    format!("Signal too old: {} seconds", age.num_seconds())
                ));
            }
        }
        
        // Check required fields
        for field in &self.config.required_fields {
            match field.as_str() {
                "source_id" if signal.source_id.is_empty() => {
                    return Err(SignalHubError::InvalidSignal(
                        "Missing required field: source_id".to_string()
                    ));
                }
                "instruments" if signal.instruments.is_empty() => {
                    return Err(SignalHubError::InvalidSignal(
                        "Missing required field: instruments".to_string()
                    ));
                }
                "target_price" if signal.target_price.is_none() => {
                    return Err(SignalHubError::InvalidSignal(
                        "Missing required field: target_price".to_string()
                    ));
                }
                _ => {}
            }
        }
        
        // Validate strength is in range
        if signal.strength < 0.0 || signal.strength > 1.0 {
            return Err(SignalHubError::InvalidSignal(
                format!("Invalid strength: {} (must be 0.0-1.0)", signal.strength)
            ));
        }
        
        // Validate signal hasn't already expired
        if signal.is_expired() {
            return Err(SignalHubError::InvalidSignal(
                "Signal has already expired".to_string()
            ));
        }
        
        Ok(())
    }
}
