//! Agent scheduler - manages agent operating windows

use crate::{AgentId, AgentRegistry, ScheduleSpec};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;

/// Scheduled agent entry
struct ScheduledAgent {
    spec: ScheduleSpec,
    last_action: Option<ScheduleAction>,
    last_action_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScheduleAction {
    Start,
    Stop,
}

/// Agent scheduler - starts/stops agents based on schedules
pub struct AgentScheduler {
    schedules: DashMap<AgentId, ScheduledAgent>,
}

impl AgentScheduler {
    /// Create a new scheduler
    pub fn new() -> Self {
        Self {
            schedules: DashMap::new(),
        }
    }
    
    /// Register an agent with a schedule
    pub async fn register(&self, agent_id: AgentId, spec: ScheduleSpec) {
        self.schedules.insert(agent_id, ScheduledAgent {
            spec,
            last_action: None,
            last_action_time: None,
        });
    }
    
    /// Unregister an agent
    pub async fn unregister(&self, agent_id: &str) {
        self.schedules.remove(agent_id);
    }
    
    /// Run the scheduler loop
    pub async fn run_loop(&self, registry: Arc<AgentRegistry>) {
        let mut ticker = interval(Duration::from_secs(60)); // Check every minute
        
        loop {
            ticker.tick().await;
            
            let now = Utc::now();
            
            for mut entry in self.schedules.iter_mut() {
                let agent_id = entry.key().clone();
                let scheduled = entry.value_mut();
                
                // Check if current time matches start schedule
                if self.matches_schedule(&now, &scheduled.spec.start, &scheduled.spec.timezone) {
                    if scheduled.last_action != Some(ScheduleAction::Start) {
                        tracing::info!(agent_id = %agent_id, "Scheduled start triggered");
                        
                        if let Ok(agent) = registry.get(&agent_id) {
                            if !agent.state().can_trade() {
                                // Would trigger start here
                                // For now, just log
                                tracing::info!(agent_id = %agent_id, "Would start agent");
                            }
                        }
                        
                        scheduled.last_action = Some(ScheduleAction::Start);
                        scheduled.last_action_time = Some(now);
                    }
                }
                
                // Check if current time matches stop schedule
                if self.matches_schedule(&now, &scheduled.spec.stop, &scheduled.spec.timezone) {
                    if scheduled.last_action != Some(ScheduleAction::Stop) {
                        tracing::info!(agent_id = %agent_id, "Scheduled stop triggered");
                        
                        if let Ok(agent) = registry.get(&agent_id) {
                            if agent.state().can_trade() {
                                // Would trigger stop here
                                tracing::info!(agent_id = %agent_id, "Would stop agent");
                            }
                        }
                        
                        scheduled.last_action = Some(ScheduleAction::Stop);
                        scheduled.last_action_time = Some(now);
                    }
                }
                
                // Check blackout dates
                if self.is_blackout(&now, &scheduled.spec.blackout_dates) {
                    if let Ok(agent) = registry.get(&agent_id) {
                        if agent.state().can_trade() {
                            tracing::info!(agent_id = %agent_id, "Blackout period - pausing agent");
                            // Would pause agent here
                        }
                    }
                }
            }
        }
    }
    
    fn matches_schedule(&self, now: &DateTime<Utc>, cron_expr: &str, _timezone: &str) -> bool {
        // TODO: Proper cron parsing with timezone support
        // For now, just a stub
        let _ = (now, cron_expr);
        false
    }
    
    fn is_blackout(&self, now: &DateTime<Utc>, blackout_dates: &[String]) -> bool {
        let date_str = now.format("%Y-%m-%d").to_string();
        blackout_dates.contains(&date_str)
    }
}

impl Default for AgentScheduler {
    fn default() -> Self {
        Self::new()
    }
}
