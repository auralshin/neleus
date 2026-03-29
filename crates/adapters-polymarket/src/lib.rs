use neleus_core_types::Venue;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod auth;
pub mod client;
pub mod websocket;

pub use auth::{L2Authenticator, OrderData, PolymarketSigner};
pub use client::PolymarketClient;
pub use websocket::PolymarketWebSocket;

#[derive(Error, Debug)]
pub enum PolymarketError {
    #[error("Missing credentials")]
    MissingCredentials,
    #[error("Invalid private key: {0}")]
    InvalidPrivateKey(String),
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
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("Nonce already used")]
    NonceAlreadyUsed,
}

#[derive(Debug, Clone)]
pub struct PolymarketConfig {
    /// CLOB API base URL
    pub clob_url: String,

    /// Gamma API base URL (for market discovery)
    pub gamma_url: String,

    /// WebSocket URL for CLOB subscriptions
    pub ws_url: String,

    /// Polygon signer address
    pub signer_address: Option<String>,

    /// Private key for signing
    pub private_key: Option<String>,

    /// API key (L2 auth)
    pub api_key: Option<String>,

    /// API secret (L2 auth)
    pub api_secret: Option<String>,

    /// API passphrase (L2 auth)
    pub api_passphrase: Option<String>,

    /// Chain ID (137 for Polygon mainnet, 80002 for Polygon Amoy testnet)
    pub chain_id: u64,

    /// Signature type for wallet
    pub signature_type: SignatureType,

    /// Funder address (proxy wallet address)
    pub funder_address: Option<String>,

    /// Reconnect configuration
    pub reconnect: ReconnectConfig,

    /// Rate limit configuration
    pub rate_limit: RateLimitConfig,
}

impl PolymarketConfig {
    pub fn mainnet() -> Self {
        Self {
            clob_url: "https://clob.polymarket.com".to_string(),
            gamma_url: "https://gamma-api.polymarket.com".to_string(),
            ws_url: "wss://ws-subscriptions-clob.polymarket.com/ws/".to_string(),
            signer_address: None,
            private_key: None,
            api_key: None,
            api_secret: None,
            api_passphrase: None,
            chain_id: 137,
            signature_type: SignatureType::GnosisSafe,
            funder_address: None,
            reconnect: ReconnectConfig::default(),
            rate_limit: RateLimitConfig::default(),
        }
    }

    pub fn testnet() -> Self {
        Self {
            clob_url: "https://clob-staging.polymarket.com".to_string(),
            gamma_url: "https://gamma-api-staging.polymarket.com".to_string(),
            ws_url: "wss://ws-subscriptions-clob-staging.polymarket.com/ws/".to_string(),
            signer_address: None,
            private_key: None,
            api_key: None,
            api_secret: None,
            api_passphrase: None,
            chain_id: 80002,
            signature_type: SignatureType::GnosisSafe,
            funder_address: None,
            reconnect: ReconnectConfig::default(),
            rate_limit: RateLimitConfig::default(),
        }
    }

    pub fn with_private_key(mut self, address: String, private_key: String) -> Self {
        self.signer_address = Some(address);
        self.private_key = Some(private_key);
        self
    }

    pub fn with_api_credentials(
        mut self,
        api_key: String,
        api_secret: String,
        api_passphrase: String,
    ) -> Self {
        self.api_key = Some(api_key);
        self.api_secret = Some(api_secret);
        self.api_passphrase = Some(api_passphrase);
        self
    }

    pub fn with_signature_type(mut self, signature_type: SignatureType) -> Self {
        self.signature_type = signature_type;
        self
    }

    pub fn with_funder(mut self, funder_address: String) -> Self {
        self.funder_address = Some(funder_address);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureType {
    /// Standard Ethereum wallet (MetaMask)
    EOA = 0,
    /// Custom proxy wallet (Magic Link users)
    PolyProxy = 1,
    /// Gnosis Safe multisig proxy wallet
    GnosisSafe = 2,
}

impl SignatureType {
    pub fn as_u8(&self) -> u8 {
        *self as u8
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

/// Nonce manager for L1 authentication
#[derive(Debug)]
pub struct NonceManager {
    current_nonce: u64,
}

impl NonceManager {
    pub fn new() -> Self {
        Self {
            current_nonce: Self::current_time_ms(),
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
        self.current_nonce
    }

    pub fn reset(&mut self) {
        self.current_nonce = Self::current_time_ms();
    }
}

impl Default for NonceManager {
    fn default() -> Self {
        Self::new()
    }
}

// API Credentials structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCredentials {
    #[serde(rename = "apiKey")]
    pub api_key: String,
    pub secret: String,
    pub passphrase: String,
}

// WebSocket subscription types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsSubscriptionType {
    /// Subscribe to market orderbook updates
    Market,
    /// Subscribe to user's order updates (requires auth)
    User,
}

#[derive(Debug, Clone)]
pub struct WsSubscription {
    pub subscription_type: WsSubscriptionType,
    pub token_id: Option<String>,
}

impl WsSubscription {
    pub fn market(token_id: String) -> Self {
        Self {
            subscription_type: WsSubscriptionType::Market,
            token_id: Some(token_id),
        }
    }

    pub fn user() -> Self {
        Self {
            subscription_type: WsSubscriptionType::User,
            token_id: None,
        }
    }

    pub fn to_json(&self) -> String {
        match &self.subscription_type {
            WsSubscriptionType::Market => {
                if let Some(token_id) = &self.token_id {
                    format!(
                        r#"{{"auth":{{}},"markets":["{}"],"type":"market"}}"#,
                        token_id
                    )
                } else {
                    String::new()
                }
            }
            WsSubscriptionType::User => r#"{"type":"user"}"#.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PolymarketOrderType {
    GTC,
    FOK,
    GTD,
}

impl Default for PolymarketOrderType {
    fn default() -> Self {
        PolymarketOrderType::GTC
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymarketMarket {
    pub token_id: String,
    pub condition_id: String,
    pub market_slug: String,
    pub question: String,
    pub end_date_iso: Option<String>,
    pub description: Option<String>,
    pub outcomes: Vec<String>,
    pub outcome_prices: Option<Vec<String>>,
    pub volume: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymarketBookLevel {
    pub price: String,
    pub size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymarketBook {
    pub market: String,
    pub asset_id: String,
    pub hash: String,
    pub bids: Vec<PolymarketBookLevel>,
    pub asks: Vec<PolymarketBookLevel>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymarketOrder {
    pub order_id: String,
    pub market: String,
    pub asset_id: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub side: String,
    pub price: String,
    pub size: String,
    pub original_size: String,
    pub created_at: String,
    pub last_update: String,
    pub status: PolymarketOrderStatus,
    pub maker_address: String,
    pub owner: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PolymarketOrderStatus {
    Live,
    Matched,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymarketTrade {
    pub id: String,
    pub market: String,
    pub asset_id: String,
    pub side: String,
    pub price: String,
    pub size: String,
    pub fee_rate_bps: String,
    pub timestamp: String,
    pub maker_address: String,
    pub taker_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolymarketPosition {
    pub asset_id: String,
    pub market: String,
    pub size: String,
    pub realized_pnl: Option<String>,
}

use neleus_core_engine::{
    Component, ComponentError, DataClient, DataSubscription, ExecutionClient, StrategyCommand,
};
use neleus_core_types::OrderId;

pub struct PolymarketExecutionAdapter {
    config: PolymarketConfig,
    client: Option<client::PolymarketClient>,
    is_running: bool,
}

impl PolymarketExecutionAdapter {
    pub fn new(config: PolymarketConfig) -> Self {
        Self {
            config,
            client: None,
            is_running: false,
        }
    }

    pub async fn connect(&mut self) -> Result<(), PolymarketError> {
        let client = client::PolymarketClient::new(self.config.clone());
        self.client = Some(client);
        Ok(())
    }

    pub fn client(&self) -> Option<&client::PolymarketClient> {
        self.client.as_ref()
    }
}

impl Component for PolymarketExecutionAdapter {
    fn name(&self) -> &str {
        "polymarket-execution"
    }

    fn start(&mut self) -> Result<(), ComponentError> {
        self.is_running = true;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), ComponentError> {
        self.is_running = false;
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.is_running
    }
}

impl ExecutionClient for PolymarketExecutionAdapter {
    fn venue(&self) -> Venue {
        Venue::Polymarket
    }

    fn submit_order(&mut self, order: &StrategyCommand) -> Result<(), ComponentError> {
        let _client = self
            .client
            .as_ref()
            .ok_or_else(|| ComponentError::Other("Client not connected".to_string()))?;

        if let StrategyCommand::SubmitOrder {
            order_id: _,
            instrument_id: _,
            side: _,
            order_type: _,
            price: _,
            quantity: _,
        } = order
        {
            // Polymarket uses token_id based ordering
            // Would need to resolve instrument_id to token_id
            // This is a placeholder for full implementation
            Err(ComponentError::Other(
                "Polymarket order submission requires token_id resolution. Use client directly."
                    .to_string(),
            ))
        } else {
            Err(ComponentError::Other("Invalid command type".to_string()))
        }
    }

    fn cancel_order(&mut self, _order_id: &OrderId) -> Result<(), ComponentError> {
        let _client = self
            .client
            .as_ref()
            .ok_or_else(|| ComponentError::Other("Client not connected".to_string()))?;

        // Polymarket cancellation uses their own order_id format
        Err(ComponentError::Other(
            "Polymarket cancel requires Polymarket order_id. Use client directly.".to_string(),
        ))
    }
}

/// Wrapper that implements DataClient for Polymarket
#[allow(dead_code)]
pub struct PolymarketDataAdapter {
    config: PolymarketConfig,
    subscriptions: Vec<DataSubscription>,
    is_running: bool,
}

impl PolymarketDataAdapter {
    pub fn new(config: PolymarketConfig) -> Self {
        Self {
            config,
            subscriptions: Vec::new(),
            is_running: false,
        }
    }

    /// Get WebSocket subscriptions in Polymarket format
    pub fn get_ws_subscriptions(&self) -> Vec<WsSubscription> {
        self.subscriptions
            .iter()
            .filter_map(|sub| {
                match sub {
                    DataSubscription::OrderBook { instrument_id, .. } => {
                        // instrument_id.symbol should contain the token_id for Polymarket
                        Some(WsSubscription::market(instrument_id.symbol.to_string()))
                    }
                    _ => None,
                }
            })
            .collect()
    }
}

impl Component for PolymarketDataAdapter {
    fn name(&self) -> &str {
        "polymarket-data"
    }

    fn start(&mut self) -> Result<(), ComponentError> {
        self.is_running = true;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), ComponentError> {
        self.is_running = false;
        self.subscriptions.clear();
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.is_running
    }
}

impl DataClient for PolymarketDataAdapter {
    fn venue(&self) -> Venue {
        Venue::Polymarket
    }

    fn subscribe(&mut self, subscription: &DataSubscription) -> Result<(), ComponentError> {
        match subscription {
            DataSubscription::OrderBook { .. } => {
                self.subscriptions.push(subscription.clone());
                Ok(())
            }
            _ => Err(ComponentError::Other(format!(
                "Polymarket only supports OrderBook subscriptions: {:?}",
                subscription
            ))),
        }
    }

    fn unsubscribe(&mut self, subscription: &DataSubscription) -> Result<(), ComponentError> {
        self.subscriptions.retain(|s| s != subscription);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = PolymarketConfig::mainnet();
        assert_eq!(config.chain_id, 137);
        assert_eq!(config.clob_url, "https://clob.polymarket.com");
    }

    #[test]
    fn test_nonce_manager() {
        let mut nonce_manager = NonceManager::new();
        let nonce1 = nonce_manager.next_nonce();
        let nonce2 = nonce_manager.next_nonce();
        assert!(nonce2 > nonce1);
    }

    #[test]
    fn test_ws_subscription_market() {
        let sub = WsSubscription::market("123456".to_string());
        let json = sub.to_json();
        assert!(json.contains("123456"));
        assert!(json.contains("market"));
    }
}
