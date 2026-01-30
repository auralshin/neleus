//! Signal routing to subscribed agents

use crate::{Result, Signal, SignalHubError, SignalSubscription, SubscriptionId};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Signal handler trait - agents implement this to receive signals
#[async_trait]
pub trait SignalHandler: Send + Sync {
    /// Handle an incoming signal
    async fn on_signal(&self, signal: &Signal) -> Result<()>;
    
    /// Get the handler's agent ID
    fn agent_id(&self) -> &str;
}

/// Signal router - routes signals to subscribed agents
pub struct SignalRouter {
    /// Subscriptions indexed by subscription ID
    subscriptions: DashMap<SubscriptionId, ActiveSubscription>,
    /// Agent ID to subscription IDs mapping
    agent_subscriptions: DashMap<String, Vec<SubscriptionId>>,
    /// Broadcast channel for signal distribution
    signal_tx: broadcast::Sender<Signal>,
}

struct ActiveSubscription {
    agent_id: String,
    subscription: SignalSubscription,
    handler: Option<Arc<dyn SignalHandler>>,
}

impl SignalRouter {
    /// Create a new router
    pub fn new() -> Self {
        let (signal_tx, _) = broadcast::channel(10000);
        Self {
            subscriptions: DashMap::new(),
            agent_subscriptions: DashMap::new(),
            signal_tx,
        }
    }
    
    /// Add a subscription
    pub fn add_subscription(
        &self,
        agent_id: String,
        subscription: SignalSubscription,
    ) -> Result<SubscriptionId> {
        let subscription_id = uuid::Uuid::new_v4().to_string();
        
        self.subscriptions.insert(
            subscription_id.clone(),
            ActiveSubscription {
                agent_id: agent_id.clone(),
                subscription,
                handler: None,
            },
        );
        
        self.agent_subscriptions
            .entry(agent_id)
            .or_insert_with(Vec::new)
            .push(subscription_id.clone());
        
        Ok(subscription_id)
    }
    
    /// Add subscription with handler
    pub fn add_subscription_with_handler(
        &self,
        agent_id: String,
        subscription: SignalSubscription,
        handler: Arc<dyn SignalHandler>,
    ) -> Result<SubscriptionId> {
        let subscription_id = uuid::Uuid::new_v4().to_string();
        
        self.subscriptions.insert(
            subscription_id.clone(),
            ActiveSubscription {
                agent_id: agent_id.clone(),
                subscription,
                handler: Some(handler),
            },
        );
        
        self.agent_subscriptions
            .entry(agent_id)
            .or_insert_with(Vec::new)
            .push(subscription_id.clone());
        
        Ok(subscription_id)
    }
    
    /// Remove a subscription
    pub fn remove_subscription(&self, subscription_id: &SubscriptionId) -> Result<()> {
        let sub = self.subscriptions.remove(subscription_id)
            .ok_or_else(|| SignalHubError::SubscriptionNotFound(subscription_id.clone()))?;
        
        // Remove from agent's subscription list
        if let Some(mut subs) = self.agent_subscriptions.get_mut(&sub.1.agent_id) {
            subs.retain(|id| id != subscription_id);
        }
        
        Ok(())
    }
    
    /// Remove all subscriptions for an agent
    pub fn remove_agent_subscriptions(&self, agent_id: &str) {
        if let Some((_, subscription_ids)) = self.agent_subscriptions.remove(agent_id) {
            for sub_id in subscription_ids {
                self.subscriptions.remove(&sub_id);
            }
        }
    }
    
    /// Route a signal to matching subscriptions
    pub async fn route(&self, signal: &Signal) -> Result<usize> {
        let mut routed_count = 0;
        
        for sub_entry in self.subscriptions.iter() {
            let sub = sub_entry.value();
            
            // Check if subscription matches this signal
            if self.matches_subscription(signal, &sub.subscription) {
                // If handler is registered, deliver directly
                if let Some(ref handler) = sub.handler {
                    if let Err(e) = handler.on_signal(signal).await {
                        tracing::warn!(
                            agent_id = %sub.agent_id,
                            error = %e,
                            "Failed to deliver signal to handler"
                        );
                    } else {
                        routed_count += 1;
                    }
                } else {
                    // Otherwise, just count as routed (would be picked up via broadcast)
                    routed_count += 1;
                }
            }
        }
        
        // Also broadcast to any listeners
        let _ = self.signal_tx.send(signal.clone());
        
        Ok(routed_count)
    }
    
    /// Subscribe to signal broadcast
    pub fn subscribe_broadcast(&self) -> broadcast::Receiver<Signal> {
        self.signal_tx.subscribe()
    }
    
    /// Get subscription count
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }
    
    fn matches_subscription(&self, signal: &Signal, subscription: &SignalSubscription) -> bool {
        // Check source filter
        if let Some(ref sources) = subscription.source_ids {
            if !sources.contains(&signal.source_id) {
                return false;
            }
        }
        
        // Check signal type filter
        if let Some(ref types) = subscription.signal_types {
            if !types.contains(&signal.signal_type) {
                return false;
            }
        }
        
        // Check instrument filter
        if let Some(ref instruments) = subscription.instruments {
            let has_match = signal.instruments.iter().any(|i| instruments.contains(i));
            if !has_match && !signal.instruments.is_empty() {
                return false;
            }
        }
        
        // Check tag filter
        if let Some(ref tags) = subscription.tags {
            let has_match = signal.tags.iter().any(|t| tags.contains(t));
            if !has_match && !tags.is_empty() {
                return false;
            }
        }
        
        // Check minimum strength
        if let Some(min_strength) = subscription.min_strength {
            if signal.strength < min_strength {
                return false;
            }
        }
        
        true
    }
}

impl Default for SignalRouter {
    fn default() -> Self {
        Self::new()
    }
}
