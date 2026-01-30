//! Signal storage

use crate::{Result, Signal, SignalHubError, SignalQuery};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::VecDeque;

/// Signal store trait
#[async_trait]
pub trait SignalStore: Send + Sync {
    /// Store a signal
    async fn store(&self, signal: &Signal) -> Result<()>;
    
    /// Query signals
    async fn query(&self, query: SignalQuery) -> Result<Vec<Signal>>;
    
    /// Get signal by ID
    async fn get(&self, signal_id: &str) -> Result<Option<Signal>>;
    
    /// Delete old signals
    async fn cleanup(&self, older_than_days: u32) -> Result<u64>;
}

/// In-memory signal store
pub struct MemorySignalStore {
    signals: RwLock<VecDeque<Signal>>,
    max_size: usize,
}

impl MemorySignalStore {
    pub fn new() -> Self {
        Self::with_capacity(100_000)
    }
    
    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            signals: RwLock::new(VecDeque::with_capacity(max_size)),
            max_size,
        }
    }
}

impl Default for MemorySignalStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SignalStore for MemorySignalStore {
    async fn store(&self, signal: &Signal) -> Result<()> {
        let mut signals = self.signals.write();
        
        if signals.len() >= self.max_size {
            signals.pop_front();
        }
        
        signals.push_back(signal.clone());
        Ok(())
    }
    
    async fn query(&self, query: SignalQuery) -> Result<Vec<Signal>> {
        let signals = self.signals.read();
        
        let mut results: Vec<_> = signals.iter()
            .filter(|s| {
                // Apply filters
                if let Some(ref source_id) = query.source_id {
                    if &s.source_id != source_id {
                        return false;
                    }
                }
                
                if let Some(ref signal_type) = query.signal_type {
                    if &s.signal_type != signal_type {
                        return false;
                    }
                }
                
                if let Some(ref instruments) = query.instruments {
                    let has_match = s.instruments.iter().any(|i| instruments.contains(i));
                    if !has_match {
                        return false;
                    }
                }
                
                if let Some(ref direction) = query.direction {
                    if &s.direction != direction {
                        return false;
                    }
                }
                
                if let Some(ref start) = query.start_time {
                    if &s.timestamp < start {
                        return false;
                    }
                }
                
                if let Some(ref end) = query.end_time {
                    if &s.timestamp > end {
                        return false;
                    }
                }
                
                if let Some(min_strength) = query.min_strength {
                    if s.strength < min_strength {
                        return false;
                    }
                }
                
                if let Some(ref tags) = query.tags {
                    let has_match = s.tags.iter().any(|t| tags.contains(t));
                    if !has_match && !tags.is_empty() {
                        return false;
                    }
                }
                
                if let Some(ref model_id) = query.model_id {
                    if s.model_id.as_ref() != Some(model_id) {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect();
        
        // Apply offset
        if let Some(offset) = query.offset {
            if offset < results.len() {
                results = results.into_iter().skip(offset).collect();
            } else {
                results.clear();
            }
        }
        
        // Apply limit
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }
        
        Ok(results)
    }
    
    async fn get(&self, signal_id: &str) -> Result<Option<Signal>> {
        let signals = self.signals.read();
        Ok(signals.iter().find(|s| s.id == signal_id).cloned())
    }
    
    async fn cleanup(&self, older_than_days: u32) -> Result<u64> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(older_than_days as i64);
        let mut signals = self.signals.write();
        
        let initial_len = signals.len();
        signals.retain(|s| s.timestamp > cutoff);
        
        Ok((initial_len - signals.len()) as u64)
    }
}
