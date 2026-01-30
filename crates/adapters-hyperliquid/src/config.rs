use thiserror::Error;

#[derive(Error, Debug)]
pub enum HyperliquidError {
    #[error("Missing credentials")]
    MissingCredentials,
    #[error("Invalid private key: {0}")]
    InvalidPrivateKey(String),
    #[error("Signing error: {0}")]
    SigningError(String),
    #[error("Request error: {0}")]
    RequestError(String),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Order limit exceeded: {current}/{max}")]
    OrderLimitExceeded { current: u32, max: u32 },
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Websocket error: {0}")]
    WebsocketError(String),
}

#[derive(Debug, Clone)]
pub struct HyperliquidConfig {
    pub ws_url: String,
    pub rest_url: String,
    pub account_address: Option<String>,
    pub private_key: Option<String>,
    pub testnet: bool,
    pub reconnect: ReconnectConfig,
    pub rate_limit: RateLimitConfig,
}

impl HyperliquidConfig {
    pub fn mainnet() -> Self {
        Self {
            ws_url: "wss://api.hyperliquid.xyz/ws".to_string(),
            rest_url: "https://api.hyperliquid.xyz".to_string(),
            account_address: None,
            private_key: None,
            testnet: false,
            reconnect: ReconnectConfig::default(),
            rate_limit: RateLimitConfig::default(),
        }
    }

    pub fn testnet() -> Self {
        Self {
            ws_url: "wss://api.hyperliquid-testnet.xyz/ws".to_string(),
            rest_url: "https://api.hyperliquid-testnet.xyz".to_string(),
            account_address: None,
            private_key: None,
            testnet: true,
            reconnect: ReconnectConfig::default(),
            rate_limit: RateLimitConfig::default(),
        }
    }

    pub fn with_credentials(mut self, address: String, private_key: String) -> Self {
        self.account_address = Some(address);
        self.private_key = Some(private_key);
        self
    }
}

#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub max_attempts: Option<u32>,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
            max_attempts: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_second: u32,
    pub burst_capacity: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 10,
            burst_capacity: 20,
        }
    }
}

#[derive(Debug)]
pub struct NonceManager {
    current_nonce: u64,
    used_nonces: Vec<u64>,
    max_tracked: usize,
}

impl NonceManager {
    pub fn new() -> Self {
        Self {
            current_nonce: Self::current_time_ms(),
            used_nonces: Vec::with_capacity(100),
            max_tracked: 100,
        }
    }

    fn current_time_ms() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    pub fn next_nonce(&mut self) -> u64 {
        let now = Self::current_time_ms();
        self.current_nonce = self.current_nonce.max(now) + 1;
        self.used_nonces.push(self.current_nonce);
        if self.used_nonces.len() > self.max_tracked {
            self.used_nonces.remove(0);
        }
        self.current_nonce
    }

    pub fn is_valid_nonce(&self, nonce: u64) -> bool {
        let now = Self::current_time_ms();
        let two_days_ms = 2 * 24 * 60 * 60 * 1000;
        let one_day_ms = 24 * 60 * 60 * 1000;
        let min_valid = now.saturating_sub(two_days_ms);
        let max_valid = now + one_day_ms;
        nonce > min_valid && nonce < max_valid
    }

    pub fn reset(&mut self) {
        self.current_nonce = Self::current_time_ms();
        self.used_nonces.clear();
    }
}

impl Default for NonceManager {
    fn default() -> Self {
        Self::new()
    }
}
