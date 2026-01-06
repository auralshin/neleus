use hmac::{Hmac, Mac};
use neleus_core_types::{InstrumentId, InstrumentType, OrderSide, Venue};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
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
    config: RateLimitConfig,

    current_weight: u32,

    window_start: std::time::Instant,

    window_duration: std::time::Duration,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsSubscriptionType {
    OrderBook,

    Trades,

    Orders,

    Fills,

    Account,
}

#[derive(Debug, Clone)]
pub struct WsSubscription {
    pub subscription_type: WsSubscriptionType,
    pub market_id: Option<u32>,
}

impl WsSubscription {
    pub fn to_message(&self) -> LighterWsMessage {
        match &self.subscription_type {
            WsSubscriptionType::OrderBook => LighterWsMessage::Subscribe {
                channel: "orderbook".to_string(),
                market_id: self.market_id,
            },
            WsSubscriptionType::Trades => LighterWsMessage::Subscribe {
                channel: "trades".to_string(),
                market_id: self.market_id,
            },
            WsSubscriptionType::Orders => LighterWsMessage::Subscribe {
                channel: "orders".to_string(),
                market_id: None,
            },
            WsSubscriptionType::Fills => LighterWsMessage::Subscribe {
                channel: "fills".to_string(),
                market_id: None,
            },
            WsSubscriptionType::Account => LighterWsMessage::Subscribe {
                channel: "account".to_string(),
                market_id: None,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum LighterWsMessage {
    Subscribe {
        channel: String,
        market_id: Option<u32>,
    },
    Unsubscribe {
        channel: String,
        market_id: Option<u32>,
    },
    Ping,
    Pong,
}

impl LighterWsMessage {
    pub fn to_json(&self) -> String {
        match self {
            LighterWsMessage::Subscribe { channel, market_id } => {
                if let Some(id) = market_id {
                    format!(
                        r#"{{"op":"subscribe","channel":"{}","market_id":{}}}"#,
                        channel, id
                    )
                } else {
                    format!(r#"{{"op":"subscribe","channel":"{}"}}"#, channel)
                }
            }
            LighterWsMessage::Unsubscribe { channel, market_id } => {
                if let Some(id) = market_id {
                    format!(
                        r#"{{"op":"unsubscribe","channel":"{}","market_id":{}}}"#,
                        channel, id
                    )
                } else {
                    format!(r#"{{"op":"unsubscribe","channel":"{}"}}"#, channel)
                }
            }
            LighterWsMessage::Ping => r#"{"op":"ping"}"#.to_string(),
            LighterWsMessage::Pong => r#"{"op":"pong"}"#.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LighterMarketInfo {
    pub market_id: u32,
    pub symbol: String,
    pub base_asset: String,
    pub quote_asset: String,
    pub price_decimals: u32,
    pub quantity_decimals: u32,
    pub min_order_size: f64,
    pub tick_size: f64,
    pub maker_fee: f64,
    pub taker_fee: f64,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct LighterTrade {
    pub market_id: u32,
    pub trade_id: u64,
    pub price: f64,
    pub quantity: f64,
    pub side: OrderSide,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct LighterBookLevel {
    pub price: f64,
    pub quantity: f64,
}

#[derive(Debug, Clone)]
pub struct LighterOrderBook {
    pub market_id: u32,
    pub bids: Vec<LighterBookLevel>,
    pub asks: Vec<LighterBookLevel>,
    pub timestamp: u64,
    pub sequence: u64,
}

#[derive(Debug, Clone)]
pub struct LighterUserOrder {
    pub order_id: String,
    pub market_id: u32,
    pub side: OrderSide,
    pub order_type: LighterOrderType,
    pub price: f64,
    pub quantity: f64,
    pub filled_quantity: f64,
    pub status: LighterOrderStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LighterOrderType {
    Limit,
    Market,
    LimitPostOnly,
    LimitIoc,
    LimitFok,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LighterOrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    Expired,
}

#[derive(Debug, Clone)]
pub struct LighterUserFill {
    pub fill_id: String,
    pub order_id: String,
    pub market_id: u32,
    pub side: OrderSide,
    pub price: f64,
    pub quantity: f64,
    pub fee: f64,
    pub is_maker: bool,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LighterOrderRequest {
    pub market_id: u32,
    pub side: OrderSide,
    pub order_type: LighterOrderType,
    pub price: f64,
    pub quantity: f64,
    pub client_order_id: Option<String>,
    pub reduce_only: bool,
}

#[derive(Debug, Clone)]
pub struct LighterCancelRequest {
    pub order_id: String,
    pub market_id: u32,
}

pub struct LighterWsMarketData {
    config: LighterConfig,
    subscriptions: Vec<WsSubscription>,
    is_connected: bool,
}

impl LighterWsMarketData {
    pub fn new(config: LighterConfig) -> Self {
        Self {
            config,
            subscriptions: Vec::new(),
            is_connected: false,
        }
    }

    pub fn subscribe_orderbook(&mut self, market_id: u32) {
        self.subscriptions.push(WsSubscription {
            subscription_type: WsSubscriptionType::OrderBook,
            market_id: Some(market_id),
        });
    }

    pub fn subscribe_trades(&mut self, market_id: u32) {
        self.subscriptions.push(WsSubscription {
            subscription_type: WsSubscriptionType::Trades,
            market_id: Some(market_id),
        });
    }

    pub fn get_subscription_messages(&self) -> Vec<String> {
        self.subscriptions
            .iter()
            .map(|s| s.to_message().to_json())
            .collect()
    }
}

pub struct LighterWsUserStream {
    config: LighterConfig,
    is_connected: bool,
    is_authenticated: bool,
}

impl LighterWsUserStream {
    pub fn new(config: LighterConfig) -> Self {
        Self {
            config,
            is_connected: false,
            is_authenticated: false,
        }
    }

    pub fn get_subscription_messages(&self) -> Vec<String> {
        vec![
            WsSubscription {
                subscription_type: WsSubscriptionType::Orders,
                market_id: None,
            }
            .to_message()
            .to_json(),
            WsSubscription {
                subscription_type: WsSubscriptionType::Fills,
                market_id: None,
            }
            .to_message()
            .to_json(),
            WsSubscription {
                subscription_type: WsSubscriptionType::Account,
                market_id: None,
            }
            .to_message()
            .to_json(),
        ]
    }
}

pub struct LighterRestPublic {
    config: LighterConfig,

    markets: HashMap<u32, LighterMarketInfo>,

    symbol_to_market: HashMap<String, u32>,
}

impl LighterRestPublic {
    pub fn new(config: LighterConfig) -> Self {
        Self {
            config,
            markets: HashMap::new(),
            symbol_to_market: HashMap::new(),
        }
    }

    pub fn markets_url(&self) -> String {
        format!("{}/api/v1/markets", self.config.rest_url)
    }

    pub fn orderbook_url(&self, market_id: u32) -> String {
        format!("{}/api/v1/orderbook/{}", self.config.rest_url, market_id)
    }

    pub fn trades_url(&self, market_id: u32) -> String {
        format!("{}/api/v1/trades/{}", self.config.rest_url, market_id)
    }

    pub fn cache_market(&mut self, info: LighterMarketInfo) {
        self.symbol_to_market
            .insert(info.symbol.clone(), info.market_id);
        self.markets.insert(info.market_id, info);
    }

    pub fn get_market(&self, market_id: u32) -> Option<&LighterMarketInfo> {
        self.markets.get(&market_id)
    }

    pub fn get_market_by_symbol(&self, symbol: &str) -> Option<&LighterMarketInfo> {
        self.symbol_to_market
            .get(symbol)
            .and_then(|id| self.markets.get(id))
    }
}

pub struct LighterExecution {
    config: LighterConfig,
    rate_limiter: WeightedRateLimiter,

    pending_orders: HashMap<String, LighterOrderRequest>,

    signer: Option<LighterSigner>,
}

impl LighterExecution {
    pub fn new(config: LighterConfig) -> Self {
        let rate_config = RateLimitConfig::for_tier(config.account_tier);
        let signer = if config.api_key.is_some() && config.api_secret.is_some() {
            LighterSigner::new(
                config.api_key.clone().unwrap(),
                config.api_secret.clone().unwrap(),
            )
            .ok()
        } else {
            None
        };

        Self {
            rate_limiter: WeightedRateLimiter::new(rate_config),
            signer,
            config,
            pending_orders: HashMap::new(),
        }
    }

    pub fn orders_url(&self) -> String {
        format!("{}/api/v1/orders", self.config.rest_url)
    }

    pub fn cancel_url(&self, order_id: &str) -> String {
        format!("{}/api/v1/orders/{}", self.config.rest_url, order_id)
    }

    pub fn can_place_order(&self) -> bool {
        self.rate_limiter
            .can_perform(self.rate_limiter.config.weights.place_order)
    }

    pub fn has_credentials(&self) -> bool {
        self.signer.is_some()
    }

    pub fn build_order_headers(
        &self,
        request: &LighterOrderRequest,
    ) -> Result<HashMap<String, String>, LighterError> {
        let signer = self
            .signer
            .as_ref()
            .ok_or(LighterError::MissingCredentials)?;

        let body = serde_json::to_string(request)
            .map_err(|e| LighterError::SigningError(e.to_string()))?;

        signer.sign_request("POST", "/api/v1/orders", &body)
    }

    pub fn build_cancel_headers(
        &self,
        order_id: &str,
    ) -> Result<HashMap<String, String>, LighterError> {
        let signer = self
            .signer
            .as_ref()
            .ok_or(LighterError::MissingCredentials)?;
        let path = format!("/api/v1/orders/{}", order_id);
        signer.sign_request("DELETE", &path, "")
    }

    pub fn on_order_submitted(&mut self, client_order_id: String, request: LighterOrderRequest) {
        self.rate_limiter
            .record(self.rate_limiter.config.weights.place_order);
        self.pending_orders.insert(client_order_id, request);
    }

    pub fn on_cancel_submitted(&mut self) {
        self.rate_limiter
            .record(self.rate_limiter.config.weights.cancel_order);
    }

    pub fn on_order_closed(&mut self, client_order_id: &str) {
        self.pending_orders.remove(client_order_id);
    }
}

pub struct LighterSigner {
    api_key: String,
    api_secret: Vec<u8>,
}

impl LighterSigner {
    pub fn new(api_key: String, api_secret: String) -> Result<Self, LighterError> {
        let secret_bytes = hex::decode(&api_secret)
            .or_else(|_| api_secret.as_bytes().to_vec().pipe(Ok))
            .map_err(|e: std::convert::Infallible| {
                LighterError::SigningError(format!("{:?}", e))
            })?;

        Ok(Self {
            api_key,
            api_secret: secret_bytes,
        })
    }

    pub fn sign_request(
        &self,
        method: &str,
        path: &str,
        body: &str,
    ) -> Result<HashMap<String, String>, LighterError> {
        let timestamp = Self::current_timestamp_ms();
        let nonce = Self::generate_nonce();

        let message = format!("{}{}{}{}{}", timestamp, nonce, method, path, body);

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(&self.api_secret)
            .map_err(|e| LighterError::SigningError(e.to_string()))?;
        mac.update(message.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        let mut headers = HashMap::new();
        headers.insert("X-API-KEY".to_string(), self.api_key.clone());
        headers.insert("X-TIMESTAMP".to_string(), timestamp.to_string());
        headers.insert("X-NONCE".to_string(), nonce);
        headers.insert("X-SIGNATURE".to_string(), signature);
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        Ok(headers)
    }

    fn current_timestamp_ms() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    fn generate_nonce() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("{:x}", ts)
    }
}

trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
    }
}

impl<T> Pipe for T {}

pub struct LighterAdapter {
    pub config: LighterConfig,
    pub market_data: LighterWsMarketData,
    pub user_stream: LighterWsUserStream,
    pub rest_public: LighterRestPublic,
    pub execution: LighterExecution,
}

impl LighterAdapter {
    pub fn new(config: LighterConfig) -> Self {
        Self {
            market_data: LighterWsMarketData::new(config.clone()),
            user_stream: LighterWsUserStream::new(config.clone()),
            rest_public: LighterRestPublic::new(config.clone()),
            execution: LighterExecution::new(config.clone()),
            config,
        }
    }

    pub fn venue(&self) -> Venue {
        Venue::Lighter
    }

    pub fn to_instrument_id(&self, market_id: u32) -> Option<InstrumentId> {
        self.rest_public
            .get_market(market_id)
            .map(|m| InstrumentId::new(Venue::Lighter, &m.symbol, InstrumentType::Perp))
    }

    pub fn to_market_id(&self, symbol: &str) -> Option<u32> {
        self.rest_public
            .get_market_by_symbol(symbol)
            .map(|m| m.market_id)
    }
}

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage};
use tracing::{debug, error, info, warn};

#[async_trait]
pub trait WsMessageHandler: Send + Sync {
    async fn on_message(&mut self, message: String);
    async fn on_connected(&mut self);
    async fn on_disconnected(&mut self);
    async fn on_error(&mut self, error: String);
}

pub struct WsClient {
    url: String,
    reconnect_config: ReconnectConfig,
    tx: mpsc::UnboundedSender<String>,
    rx: mpsc::UnboundedReceiver<String>,
}

impl WsClient {
    pub fn new(url: String, reconnect_config: ReconnectConfig) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            url,
            reconnect_config,
            tx,
            rx,
        }
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<String> {
        self.tx.clone()
    }

    pub async fn run<H: WsMessageHandler + 'static>(mut self, mut handler: H) {
        let mut attempt = 0;
        let mut delay_ms = self.reconnect_config.initial_delay_ms;

        loop {
            info!("Connecting to WebSocket: {}", self.url);

            match connect_async(&self.url).await {
                Ok((ws_stream, _)) => {
                    info!("WebSocket connected");
                    attempt = 0;
                    delay_ms = self.reconnect_config.initial_delay_ms;
                    handler.on_connected().await;

                    let (mut write, mut read) = ws_stream.split();

                    let mut rx = self.rx;
                    let send_task = tokio::spawn(async move {
                        while let Some(msg) = rx.recv().await {
                            if let Err(e) = write.send(WsMessage::Text(msg)).await {
                                error!("Failed to send WebSocket message: {}", e);
                                break;
                            }
                        }
                    });

                    while let Some(result) = read.next().await {
                        match result {
                            Ok(WsMessage::Text(text)) => {
                                debug!("Received WebSocket message: {}", text);
                                handler.on_message(text).await;
                            }
                            Ok(WsMessage::Binary(data)) => {
                                if let Ok(text) = String::from_utf8(data) {
                                    handler.on_message(text).await;
                                }
                            }
                            Ok(WsMessage::Ping(_)) => {
                                debug!("Received ping");
                            }
                            Ok(WsMessage::Pong(_)) => {
                                debug!("Received pong");
                            }
                            Ok(WsMessage::Close(_)) => {
                                info!("WebSocket closed by server");
                                break;
                            }
                            Err(e) => {
                                error!("WebSocket error: {}", e);
                                handler.on_error(e.to_string()).await;
                                break;
                            }
                            _ => {}
                        }
                    }

                    send_task.abort();
                    handler.on_disconnected().await;

                    if self.reconnect_config.max_attempts.is_some()
                        && attempt >= self.reconnect_config.max_attempts.unwrap()
                    {
                        error!("Max reconnection attempts reached");
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to connect to WebSocket: {}", e);
                    handler.on_error(e.to_string()).await;
                    attempt += 1;

                    if self.reconnect_config.max_attempts.is_some()
                        && attempt >= self.reconnect_config.max_attempts.unwrap()
                    {
                        error!("Max reconnection attempts reached");
                        break;
                    }
                }
            }

            info!("Reconnecting in {}ms (attempt {})", delay_ms, attempt + 1);
            sleep(Duration::from_millis(delay_ms)).await;

            delay_ms = ((delay_ms as f64) * self.reconnect_config.backoff_multiplier) as u64;
            delay_ms = delay_ms.min(self.reconnect_config.max_delay_ms);

            let (tx, rx) = mpsc::unbounded_channel();
            self.tx = tx;
            self.rx = rx;
        }
    }
}

pub struct LighterMarketDataHandler {
    subscriptions: Vec<WsSubscription>,
    message_callback: Box<dyn Fn(LighterMarketDataMessage) + Send + Sync>,
}

impl LighterMarketDataHandler {
    pub fn new<F>(subscriptions: Vec<WsSubscription>, callback: F) -> Self
    where
        F: Fn(LighterMarketDataMessage) + Send + Sync + 'static,
    {
        Self {
            subscriptions,
            message_callback: Box::new(callback),
        }
    }
}

#[async_trait]
impl WsMessageHandler for LighterMarketDataHandler {
    async fn on_message(&mut self, message: String) {
        match serde_json::from_str::<LighterMarketDataMessage>(&message) {
            Ok(msg) => (self.message_callback)(msg),
            Err(e) => warn!("Failed to parse Lighter message: {}", e),
        }
    }

    async fn on_connected(&mut self) {
        info!("Lighter market data connected");
    }

    async fn on_disconnected(&mut self) {
        warn!("Lighter market data disconnected");
    }

    async fn on_error(&mut self, error: String) {
        error!("Lighter market data error: {}", error);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LighterMarketDataMessage {
    #[serde(rename = "orderbook")]
    OrderBook { data: OrderBookData },
    #[serde(rename = "trade")]
    Trade { data: LighterTradeData },
    #[serde(rename = "order")]
    Order { data: LighterOrderData },
    #[serde(rename = "fill")]
    Fill { data: LighterFillData },
    #[serde(rename = "account")]
    Account { data: AccountData },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookData {
    pub market_id: u32,
    pub timestamp: u64,
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LighterTradeData {
    pub market_id: u32,
    pub trade_id: u64,
    pub price: String,
    pub size: String,
    pub side: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LighterOrderData {
    pub order_id: String,
    pub market_id: u32,
    pub user_address: String,
    pub price: String,
    pub size: String,
    pub side: String,
    pub order_type: String,
    pub status: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LighterFillData {
    pub fill_id: u64,
    pub order_id: String,
    pub market_id: u32,
    pub price: String,
    pub size: String,
    pub side: String,
    pub fee: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountData {
    pub address: String,
    pub balances: HashMap<String, String>,
}

impl LighterWsMarketData {
    pub async fn connect<F>(mut self, callback: F) -> Result<(), LighterError>
    where
        F: Fn(LighterMarketDataMessage) + Send + Sync + 'static,
    {
        let subscriptions = std::mem::take(&mut self.subscriptions);

        let client = WsClient::new(self.config.ws_url.clone(), self.config.reconnect.clone());

        let sender = client.sender();

        let sub_messages = subscriptions
            .iter()
            .map(|s| s.to_message().to_json())
            .collect::<Vec<_>>();
        tokio::spawn(async move {
            sleep(Duration::from_millis(100)).await;
            for msg in sub_messages {
                let _ = sender.send(msg);
            }
        });

        let handler = LighterMarketDataHandler::new(subscriptions, callback);
        client.run(handler).await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_urls() {
        let mainnet = LighterConfig::mainnet();
        assert_eq!(mainnet.ws_url, "wss://mainnet.zklighter.elliot.ai/stream");
        assert!(!mainnet.testnet);

        let testnet = LighterConfig::testnet();
        assert_eq!(testnet.ws_url, "wss://testnet.zklighter.elliot.ai/stream");
        assert!(testnet.testnet);
    }

    #[test]
    fn test_rate_limiter() {
        let config = RateLimitConfig::for_tier(AccountTier::Standard);
        let mut limiter = WeightedRateLimiter::new(config);

        assert!(limiter.can_perform(1));
        limiter.record(1);
        assert_eq!(limiter.remaining_capacity(), 1199);
    }

    #[test]
    fn test_subscription_message() {
        let sub = WsSubscription {
            subscription_type: WsSubscriptionType::OrderBook,
            market_id: Some(1),
        };
        let msg = sub.to_message().to_json();
        assert!(msg.contains("orderbook"));
        assert!(msg.contains("market_id"));
    }

    #[test]
    fn test_tier_rate_limits() {
        let standard = RateLimitConfig::for_tier(AccountTier::Standard);
        let premium = RateLimitConfig::for_tier(AccountTier::Premium);

        assert_eq!(standard.requests_per_minute, 1200);
        assert_eq!(premium.requests_per_minute, 6000);
    }
}
