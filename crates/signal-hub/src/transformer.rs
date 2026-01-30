//! Signal transformations

use crate::{Result, Signal, SignalDirection, SignalHubError, SignalType};
use serde::{Deserialize, Serialize};

/// Signal transformation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignalTransformation {
    /// Normalize strength to 0-1 range
    NormalizeStrength {
        min_input: f64,
        max_input: f64,
    },
    
    /// Map signal type
    MapSignalType {
        from: String,
        to: SignalType,
    },
    
    /// Add a tag
    AddTag {
        tag: String,
    },
    
    /// Set TTL if not present
    DefaultTtl {
        seconds: u64,
    },
    
    /// Map instrument names
    MapInstrument {
        from: String,
        to: String,
    },
    
    /// Invert direction
    InvertDirection,
    
    /// Scale position size
    ScalePositionSize {
        factor: f64,
    },
    
    /// Filter out weak signals (drop if below threshold)
    FilterStrength {
        min_strength: f64,
    },
    
    /// Add metadata
    AddMetadata {
        key: String,
        value: String,
    },
}

/// Apply a transformation to a signal
pub fn apply_transformation(mut signal: Signal, transform: &SignalTransformation) -> Result<Signal> {
    match transform {
        SignalTransformation::NormalizeStrength { min_input, max_input } => {
            if max_input > min_input {
                let range = max_input - min_input;
                signal.strength = ((signal.strength - min_input) / range).clamp(0.0, 1.0);
            }
        }
        
        SignalTransformation::MapSignalType { from, to } => {
            // Check if current type matches the "from" string representation
            let current_type_str = format!("{:?}", signal.signal_type).to_lowercase();
            if current_type_str == from.to_lowercase() {
                signal.signal_type = *to;
            }
        }
        
        SignalTransformation::AddTag { tag } => {
            if !signal.tags.contains(tag) {
                signal.tags.push(tag.clone());
            }
        }
        
        SignalTransformation::DefaultTtl { seconds } => {
            if signal.ttl_seconds.is_none() {
                signal.ttl_seconds = Some(*seconds);
                signal.expires_at = Some(signal.timestamp + chrono::Duration::seconds(*seconds as i64));
            }
        }
        
        SignalTransformation::MapInstrument { from, to } => {
            for instrument in &mut signal.instruments {
                if instrument == from {
                    *instrument = to.clone();
                }
            }
        }
        
        SignalTransformation::InvertDirection => {
            signal.direction = match signal.direction {
                SignalDirection::Long => SignalDirection::Short,
                SignalDirection::Short => SignalDirection::Long,
                other => other,
            };
        }
        
        SignalTransformation::ScalePositionSize { factor } => {
            if let Some(ref mut size) = signal.position_size {
                *size *= factor;
            }
        }
        
        SignalTransformation::FilterStrength { min_strength } => {
            if signal.strength < *min_strength {
                return Err(SignalHubError::InvalidSignal(
                    format!("Signal strength {} below threshold {}", signal.strength, min_strength)
                ));
            }
        }
        
        SignalTransformation::AddMetadata { key, value } => {
            signal.metadata.insert(key.clone(), value.clone());
        }
    }
    
    Ok(signal)
}

/// Transformation pipeline
#[derive(Debug, Clone, Default)]
pub struct TransformationPipeline {
    transforms: Vec<SignalTransformation>,
}

impl TransformationPipeline {
    /// Create a new empty pipeline
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a transformation
    pub fn add(mut self, transform: SignalTransformation) -> Self {
        self.transforms.push(transform);
        self
    }
    
    /// Apply all transformations
    pub fn apply(&self, mut signal: Signal) -> Result<Signal> {
        for transform in &self.transforms {
            signal = apply_transformation(signal, transform)?;
        }
        Ok(signal)
    }
}
