use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Normal operation - all requests pass through
    Closed,
    /// Circuit tripped - all requests fail fast
    Open,
    /// Testing recovery - limited requests allowed
    HalfOpen,
}

/// Configuration for the circuit breaker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: u32,
    /// Number of successes needed to close the circuit from half-open
    pub success_threshold: u32,
    /// Time window for counting failures (milliseconds)
    pub failure_window_ms: u64,
    /// Time to wait before transitioning from Open to HalfOpen (milliseconds)
    pub recovery_timeout_ms: u64,
    /// Maximum number of requests allowed in HalfOpen state
    pub half_open_max_requests: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            failure_window_ms: 60_000,    // 1 minute
            recovery_timeout_ms: 30_000,   // 30 seconds
            half_open_max_requests: 3,
        }
    }
}

/// Timestamped failure record
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct FailureRecord {
    timestamp: Instant,
    error: String,
}

/// Circuit breaker for a specific component/service
#[derive(Debug)]
pub struct CircuitBreaker {
    name: String,
    config: CircuitBreakerConfig,
    state: RwLock<CircuitState>,
    failures: RwLock<VecDeque<FailureRecord>>,
    last_failure_time: RwLock<Option<Instant>>,
    opened_at: RwLock<Option<Instant>>,
    half_open_successes: AtomicUsize,
    half_open_requests: AtomicUsize,
    total_successes: AtomicU64,
    total_failures: AtomicU64,
    total_rejections: AtomicU64,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given name and config
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self {
            name: name.into(),
            config,
            state: RwLock::new(CircuitState::Closed),
            failures: RwLock::new(VecDeque::new()),
            last_failure_time: RwLock::new(None),
            opened_at: RwLock::new(None),
            half_open_successes: AtomicUsize::new(0),
            half_open_requests: AtomicUsize::new(0),
            total_successes: AtomicU64::new(0),
            total_failures: AtomicU64::new(0),
            total_rejections: AtomicU64::new(0),
        }
    }

    /// Create a circuit breaker with default configuration
    pub fn with_defaults(name: impl Into<String>) -> Self {
        Self::new(name, CircuitBreakerConfig::default())
    }

    /// Get the current state of the circuit breaker
    pub fn state(&self) -> CircuitState {
        let mut state = *self.state.read().unwrap();
        
        // Check if we should transition from Open to HalfOpen
        if state == CircuitState::Open {
            if let Some(opened_at) = *self.opened_at.read().unwrap() {
                if opened_at.elapsed() > Duration::from_millis(self.config.recovery_timeout_ms) {
                    // Transition to HalfOpen
                    let mut state_guard = self.state.write().unwrap();
                    *state_guard = CircuitState::HalfOpen;
                    state = CircuitState::HalfOpen;
                    self.half_open_successes.store(0, Ordering::SeqCst);
                    self.half_open_requests.store(0, Ordering::SeqCst);
                    tracing::info!(
                        "Circuit breaker '{}' transitioning from Open to HalfOpen after {}ms",
                        self.name,
                        self.config.recovery_timeout_ms
                    );
                }
            }
        }
        
        state
    }

    /// Check if a request can proceed
    /// Returns Ok(()) if allowed, Err with reason if rejected
    pub fn allow_request(&self) -> Result<(), CircuitBreakerError> {
        let state = self.state();
        
        match state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open => {
                self.total_rejections.fetch_add(1, Ordering::SeqCst);
                Err(CircuitBreakerError::CircuitOpen {
                    name: self.name.clone(),
                    recovery_timeout_ms: self.config.recovery_timeout_ms,
                })
            }
            CircuitState::HalfOpen => {
                let current_requests = self.half_open_requests.fetch_add(1, Ordering::SeqCst);
                if current_requests >= self.config.half_open_max_requests as usize {
                    self.total_rejections.fetch_add(1, Ordering::SeqCst);
                    Err(CircuitBreakerError::HalfOpenLimitReached {
                        name: self.name.clone(),
                        max_requests: self.config.half_open_max_requests,
                    })
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Record a successful operation
    pub fn record_success(&self) {
        self.total_successes.fetch_add(1, Ordering::SeqCst);
        
        let state = self.state();
        
        if state == CircuitState::HalfOpen {
            let successes = self.half_open_successes.fetch_add(1, Ordering::SeqCst) + 1;
            
            if successes >= self.config.success_threshold as usize {
                // Transition to Closed
                let mut state_guard = self.state.write().unwrap();
                *state_guard = CircuitState::Closed;
                *self.opened_at.write().unwrap() = None;
                self.failures.write().unwrap().clear();
                
                tracing::info!(
                    "Circuit breaker '{}' closed after {} consecutive successes",
                    self.name,
                    successes
                );
            }
        }
    }

    /// Record a failed operation
    pub fn record_failure(&self, error: impl Into<String>) {
        self.total_failures.fetch_add(1, Ordering::SeqCst);
        
        let now = Instant::now();
        let error_msg = error.into();
        
        // Add to failure history
        {
            let mut failures = self.failures.write().unwrap();
            failures.push_back(FailureRecord {
                timestamp: now,
                error: error_msg.clone(),
            });
            
            // Remove old failures outside the window
            let window = Duration::from_millis(self.config.failure_window_ms);
            while let Some(front) = failures.front() {
                if now.duration_since(front.timestamp) > window {
                    failures.pop_front();
                } else {
                    break;
                }
            }
        }
        
        *self.last_failure_time.write().unwrap() = Some(now);
        
        let state = self.state();
        
        match state {
            CircuitState::Closed => {
                let failure_count = self.failures.read().unwrap().len();
                
                if failure_count >= self.config.failure_threshold as usize {
                    // Trip the circuit
                    let mut state_guard = self.state.write().unwrap();
                    *state_guard = CircuitState::Open;
                    *self.opened_at.write().unwrap() = Some(now);
                    
                    tracing::warn!(
                        "Circuit breaker '{}' OPENED after {} failures in {}ms window. Last error: {}",
                        self.name,
                        failure_count,
                        self.config.failure_window_ms,
                        error_msg
                    );
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in HalfOpen state reopens the circuit
                let mut state_guard = self.state.write().unwrap();
                *state_guard = CircuitState::Open;
                *self.opened_at.write().unwrap() = Some(now);
                self.half_open_successes.store(0, Ordering::SeqCst);
                self.half_open_requests.store(0, Ordering::SeqCst);
                
                tracing::warn!(
                    "Circuit breaker '{}' reopened from HalfOpen state. Error: {}",
                    self.name,
                    error_msg
                );
            }
            CircuitState::Open => {
                // Already open, just update failure time
            }
        }
    }

    /// Get the name of this circuit breaker
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get statistics about this circuit breaker
    pub fn stats(&self) -> CircuitBreakerStats {
        CircuitBreakerStats {
            name: self.name.clone(),
            state: self.state(),
            total_successes: self.total_successes.load(Ordering::SeqCst),
            total_failures: self.total_failures.load(Ordering::SeqCst),
            total_rejections: self.total_rejections.load(Ordering::SeqCst),
            recent_failures: self.failures.read().unwrap().len(),
        }
    }

    /// Force reset the circuit breaker to closed state
    pub fn reset(&self) {
        let mut state_guard = self.state.write().unwrap();
        *state_guard = CircuitState::Closed;
        *self.opened_at.write().unwrap() = None;
        self.failures.write().unwrap().clear();
        self.half_open_successes.store(0, Ordering::SeqCst);
        self.half_open_requests.store(0, Ordering::SeqCst);
        
        tracing::info!("Circuit breaker '{}' manually reset to Closed state", self.name);
    }

    /// Execute a fallible operation with circuit breaker protection
    pub fn call<T, E, F>(&self, operation: F) -> Result<T, CircuitBreakerError>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::fmt::Display,
    {
        self.allow_request()?;
        
        match operation() {
            Ok(result) => {
                self.record_success();
                Ok(result)
            }
            Err(e) => {
                self.record_failure(e.to_string());
                Err(CircuitBreakerError::OperationFailed {
                    name: self.name.clone(),
                    error: e.to_string(),
                })
            }
        }
    }

    /// Execute an async operation with circuit breaker protection
    pub async fn call_async<T, E, F, Fut>(&self, operation: F) -> Result<T, CircuitBreakerError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        self.allow_request()?;
        
        match operation().await {
            Ok(result) => {
                self.record_success();
                Ok(result)
            }
            Err(e) => {
                self.record_failure(e.to_string());
                Err(CircuitBreakerError::OperationFailed {
                    name: self.name.clone(),
                    error: e.to_string(),
                })
            }
        }
    }
}

/// Statistics for a circuit breaker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerStats {
    pub name: String,
    pub state: CircuitState,
    pub total_successes: u64,
    pub total_failures: u64,
    pub total_rejections: u64,
    pub recent_failures: usize,
}

/// Errors from circuit breaker operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum CircuitBreakerError {
    #[error("Circuit breaker '{name}' is OPEN. Recovery in {recovery_timeout_ms}ms")]
    CircuitOpen {
        name: String,
        recovery_timeout_ms: u64,
    },

    #[error("Circuit breaker '{name}' is in HalfOpen state. Max {max_requests} requests allowed")]
    HalfOpenLimitReached {
        name: String,
        max_requests: u32,
    },

    #[error("Operation failed for circuit breaker '{name}': {error}")]
    OperationFailed {
        name: String,
        error: String,
    },
}

/// Registry for managing multiple circuit breakers
#[derive(Debug, Default)]
pub struct CircuitBreakerRegistry {
    breakers: RwLock<std::collections::HashMap<String, Arc<CircuitBreaker>>>,
}

impl CircuitBreakerRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            breakers: RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Register a new circuit breaker
    pub fn register(&self, breaker: CircuitBreaker) -> Arc<CircuitBreaker> {
        let name = breaker.name.clone();
        let arc = Arc::new(breaker);
        self.breakers.write().unwrap().insert(name, arc.clone());
        arc
    }

    /// Get a circuit breaker by name
    pub fn get(&self, name: &str) -> Option<Arc<CircuitBreaker>> {
        self.breakers.read().unwrap().get(name).cloned()
    }

    /// Get or create a circuit breaker with default config
    pub fn get_or_create(&self, name: &str) -> Arc<CircuitBreaker> {
        {
            if let Some(breaker) = self.breakers.read().unwrap().get(name) {
                return breaker.clone();
            }
        }
        
        let breaker = CircuitBreaker::with_defaults(name);
        self.register(breaker)
    }

    /// Get or create a circuit breaker with custom config
    pub fn get_or_create_with_config(&self, name: &str, config: CircuitBreakerConfig) -> Arc<CircuitBreaker> {
        {
            if let Some(breaker) = self.breakers.read().unwrap().get(name) {
                return breaker.clone();
            }
        }
        
        let breaker = CircuitBreaker::new(name, config);
        self.register(breaker)
    }

    /// Get stats for all circuit breakers
    pub fn all_stats(&self) -> Vec<CircuitBreakerStats> {
        self.breakers
            .read()
            .unwrap()
            .values()
            .map(|b| b.stats())
            .collect()
    }

    /// Reset all circuit breakers
    pub fn reset_all(&self) {
        for breaker in self.breakers.read().unwrap().values() {
            breaker.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_initial_state() {
        let cb = CircuitBreaker::with_defaults("test");
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            failure_window_ms: 60_000,
            ..Default::default()
        };
        let cb = CircuitBreaker::new("test", config);

        // Record failures
        cb.record_failure("error 1");
        assert_eq!(cb.state(), CircuitState::Closed);
        
        cb.record_failure("error 2");
        assert_eq!(cb.state(), CircuitState::Closed);
        
        cb.record_failure("error 3");
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_allows_when_closed() {
        let cb = CircuitBreaker::with_defaults("test");
        assert!(cb.allow_request().is_ok());
    }

    #[test]
    fn test_circuit_breaker_rejects_when_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            ..Default::default()
        };
        let cb = CircuitBreaker::new("test", config);
        
        cb.record_failure("error");
        assert!(cb.allow_request().is_err());
    }

    #[test]
    fn test_circuit_breaker_call() {
        let cb = CircuitBreaker::with_defaults("test");
        
        // Successful call
        let result: Result<i32, CircuitBreakerError> = cb.call(|| Ok::<_, std::io::Error>(42));
        assert_eq!(result.unwrap(), 42);
        assert_eq!(cb.stats().total_successes, 1);
        
        // Failed call
        let result: Result<i32, CircuitBreakerError> = cb.call(|| Err::<i32, _>(std::io::Error::new(
            std::io::ErrorKind::Other, 
            "test error"
        )));
        assert!(result.is_err());
        assert_eq!(cb.stats().total_failures, 1);
    }

    #[test]
    fn test_registry() {
        let registry = CircuitBreakerRegistry::new();
        
        let cb1 = registry.get_or_create("api");
        let cb2 = registry.get_or_create("api");
        
        // Should return the same instance
        assert!(Arc::ptr_eq(&cb1, &cb2));
        
        // Create different breaker
        let cb3 = registry.get_or_create("database");
        assert!(!Arc::ptr_eq(&cb1, &cb3));
    }
}
