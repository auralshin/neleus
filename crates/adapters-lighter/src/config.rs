use thiserror::Error;

#[derive(Error, Debug)]
pub enum LighterError {
    #[error("Missing credentials")]
    MissingCredentials,
    #[error("Invalid API key")]
    InvalidApiKey,
    #[error("Signing error: {0}")]
    SigningError(String),
    #[error("Request error: {0}")]
    RequestError(String),
    #[error("Rate limit exceeded: retry after {retry_after_ms}ms")]
    RateLimitExceeded { retry_after_ms: u64 },
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Websocket error: {0}")]
    WebsocketError(String),
    #[error("Market not found: {0}")]
    MarketNotFound(String),
}

#[derive(Debug, Clone)]
pub struct LighterConfig {
    pub ws_url: String,
    pub rest_url: String,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub account_tier: AccountTier,
    pub testnet: bool,
    pub reconnect: ReconnectConfig,
}

impl LighterConfig {
    pub fn mainnet() -> Self {
        Self {
            ws_url: "wss://mainnet.zklighter.elliot.ai/stream".to_string(),
            rest_url: "https://mainnet.zklighter.elliot.ai".to_string(),
            api_key: None,
            api_secret: None,
            account_tier: AccountTier::Standard,
            testnet: false,
            reconnect: ReconnectConfig::default(),
        }
    }

    pub fn testnet() -> Self {
        Self {
            ws_url: "wss://testnet.zklighter.elliot.ai/stream".to_string(),
            rest_url: "https://testnet.zklighter.elliot.ai".to_string(),
            api_key: None,
            api_secret: None,
            account_tier: AccountTier::Standard,
            testnet: true,
            reconnect: ReconnectConfig::default(),
        }
    }

    pub fn with_credentials(mut self, api_key: String, api_secret: String) -> Self {
        self.api_key = Some(api_key);
        self.api_secret = Some(api_secret);
        self
    }

    pub fn with_tier(mut self, tier: AccountTier) -> Self {
        self.account_tier = tier;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountTier {
    Standard,
    Premium,
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub weights: OperationWeights,
}

impl RateLimitConfig {
    pub fn for_tier(tier: AccountTier) -> Self {
        match tier {
            AccountTier::Standard => Self {
                requests_per_minute: 1200,
                weights: OperationWeights::default(),
            },
            AccountTier::Premium => Self {
                requests_per_minute: 6000,
                weights: OperationWeights::default(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct OperationWeights {
    pub place_order: u32,
    pub cancel_order: u32,
    pub get_order: u32,
    pub get_orders: u32,
    pub get_positions: u32,
    pub get_account: u32,
    pub get_markets: u32,
    pub get_orderbook: u32,
    pub get_trades: u32,
}

impl Default for OperationWeights {
    fn default() -> Self {
        Self {
            place_order: 1,
            cancel_order: 1,
            get_order: 1,
            get_orders: 5,
            get_positions: 5,
            get_account: 1,
            get_markets: 1,
            get_orderbook: 5,
            get_trades: 5,
        }
    }
}

pub struct WeightedRateLimiter {
    pub config: RateLimitConfig,
    pub current_weight: u32,
    pub window_start: std::time::Instant,
    pub window_duration: std::time::Duration,
}

impl WeightedRateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            current_weight: 0,
            window_start: std::time::Instant::now(),
            window_duration: std::time::Duration::from_secs(60),
        }
    }

    pub fn can_perform(&self, weight: u32) -> bool {
        let now = std::time::Instant::now();
        if now.duration_since(self.window_start) >= self.window_duration {
            return weight <= self.config.requests_per_minute;
        }
        self.current_weight + weight <= self.config.requests_per_minute
    }

    pub fn record(&mut self, weight: u32) {
        let now = std::time::Instant::now();
        if now.duration_since(self.window_start) >= self.window_duration {
            self.window_start = now;
            self.current_weight = 0;
        }
        self.current_weight += weight;
    }

    pub fn remaining_capacity(&self) -> u32 {
        self.config
            .requests_per_minute
            .saturating_sub(self.current_weight)
    }

    pub fn time_until_reset(&self) -> std::time::Duration {
        let elapsed = std::time::Instant::now().duration_since(self.window_start);
        self.window_duration.saturating_sub(elapsed)
    }
}
