use hmac::{Hmac, Mac};
use neleus_core_types::{InstrumentId, InstrumentType, OrderSide, Venue};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
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

#[derive(Debug, Clone)]
pub struct WsSubscription {
    pub subscription_type: WsSubscriptionType,
    pub instrument: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsSubscriptionType {
    AllMids,

    L2Book,

    Trades,

    Candle,

    UserFills,

    UserOrders,

    UserFunding,

    Notification,
}

impl WsSubscription {
    pub fn to_json(&self) -> String {
        match &self.subscription_type {
            WsSubscriptionType::AllMids => {
                r#"{"method":"subscribe","subscription":{"type":"allMids"}}"#.to_string()
            }
            WsSubscriptionType::L2Book => {
                if let Some(coin) = &self.instrument {
                    format!(
                        r#"{{"method":"subscribe","subscription":{{"type":"l2Book","coin":"{}"}}}}"#,
                        coin
                    )
                } else {
                    String::new()
                }
            }
            WsSubscriptionType::Trades => {
                if let Some(coin) = &self.instrument {
                    format!(
                        r#"{{"method":"subscribe","subscription":{{"type":"trades","coin":"{}"}}}}"#,
                        coin
                    )
                } else {
                    String::new()
                }
            }
            WsSubscriptionType::Candle => {
                if let Some(coin) = &self.instrument {
                    format!(
                        r#"{{"method":"subscribe","subscription":{{"type":"candle","coin":"{}","interval":"1m"}}}}"#,
                        coin
                    )
                } else {
                    String::new()
                }
            }
            WsSubscriptionType::UserFills => {
                r#"{"method":"subscribe","subscription":{"type":"userFills"}}"#.to_string()
            }
            WsSubscriptionType::UserOrders => {
                r#"{"method":"subscribe","subscription":{"type":"userOrders"}}"#.to_string()
            }
            WsSubscriptionType::UserFunding => {
                r#"{"method":"subscribe","subscription":{"type":"userFunding"}}"#.to_string()
            }
            WsSubscriptionType::Notification => {
                r#"{"method":"subscribe","subscription":{"type":"notification"}}"#.to_string()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct HyperliquidTrade {
    pub coin: String,
    pub side: OrderSide,
    pub price: f64,
    pub size: f64,
    pub timestamp: u64,
    pub trade_id: u64,
}

#[derive(Debug, Clone)]
pub struct HyperliquidBookLevel {
    pub price: f64,
    pub size: f64,
    pub num_orders: u32,
}

#[derive(Debug, Clone)]
pub struct HyperliquidBook {
    pub coin: String,
    pub bids: Vec<HyperliquidBookLevel>,
    pub asks: Vec<HyperliquidBookLevel>,
    pub timestamp: u64,
    pub is_snapshot: bool,
}

#[derive(Debug, Clone)]
pub struct HyperliquidUserFill {
    pub coin: String,
    pub order_id: u64,
    pub side: OrderSide,
    pub price: f64,
    pub size: f64,
    pub fee: f64,
    pub timestamp: u64,
    pub crossed: bool,
}

#[derive(Debug, Clone)]
pub struct HyperliquidUserOrder {
    pub coin: String,
    pub order_id: u64,
    pub client_order_id: Option<String>,
    pub side: OrderSide,
    pub order_type: HyperliquidOrderType,
    pub price: Option<f64>,
    pub size: f64,
    pub filled_size: f64,
    pub status: HyperliquidOrderStatus,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HyperliquidOrderType {
    Limit,
    Market,
    StopMarket,
    StopLimit,
    TakeProfit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HyperliquidOrderStatus {
    Open,
    Filled,
    Canceled,
    Rejected,
    Triggered,
}

#[derive(Debug, Clone)]
pub struct HyperliquidOrderRequest {
    pub coin: String,
    pub is_buy: bool,
    pub limit_px: f64,
    pub sz: f64,
    pub reduce_only: bool,
    pub order_type: HyperliquidOrderTypeRequest,
    pub cloid: Option<String>,
}

#[derive(Debug, Clone)]
pub enum HyperliquidOrderTypeRequest {
    Limit {
        tif: HyperliquidTif,
    },
    Trigger {
        trigger_px: f64,
        is_market: bool,
        tpsl: TpSlType,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HyperliquidTif {
    Gtc,
    Ioc,
    Alo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpSlType {
    Tp,
    Sl,
}

#[derive(Debug, Clone)]
pub struct HyperliquidInstrumentInfo {
    pub coin: String,
    pub sz_decimals: u32,
    pub max_leverage: u32,
    pub only_isolated: bool,
    pub funding_rate: f64,
    pub open_interest: f64,
    pub prev_day_px: f64,
    pub day_ntl_vlm: f64,
    pub premium: f64,
    pub oracle_px: f64,
    pub mark_px: f64,
}

pub struct HyperliquidWsMarketData {
    config: HyperliquidConfig,
    subscriptions: Vec<WsSubscription>,
    is_connected: bool,
}

impl HyperliquidWsMarketData {
    pub fn new(config: HyperliquidConfig) -> Self {
        Self {
            config,
            subscriptions: Vec::new(),
            is_connected: false,
        }
    }

    pub fn subscribe_all_mids(&mut self) {
        self.subscriptions.push(WsSubscription {
            subscription_type: WsSubscriptionType::AllMids,
            instrument: None,
        });
    }

    pub fn subscribe_l2_book(&mut self, coin: &str) {
        self.subscriptions.push(WsSubscription {
            subscription_type: WsSubscriptionType::L2Book,
            instrument: Some(coin.to_string()),
        });
    }

    pub fn subscribe_trades(&mut self, coin: &str) {
        self.subscriptions.push(WsSubscription {
            subscription_type: WsSubscriptionType::Trades,
            instrument: Some(coin.to_string()),
        });
    }

    pub fn get_subscription_messages(&self) -> Vec<String> {
        self.subscriptions.iter().map(|s| s.to_json()).collect()
    }
}

pub struct HyperliquidWsUserStream {
    config: HyperliquidConfig,
    is_connected: bool,
    is_authenticated: bool,
}

impl HyperliquidWsUserStream {
    pub fn new(config: HyperliquidConfig) -> Self {
        Self {
            config,
            is_connected: false,
            is_authenticated: false,
        }
    }

    pub fn get_subscription_messages(&self) -> Vec<String> {
        vec![
            WsSubscription {
                subscription_type: WsSubscriptionType::UserFills,
                instrument: None,
            }
            .to_json(),
            WsSubscription {
                subscription_type: WsSubscriptionType::UserOrders,
                instrument: None,
            }
            .to_json(),
            WsSubscription {
                subscription_type: WsSubscriptionType::Notification,
                instrument: None,
            }
            .to_json(),
        ]
    }
}

pub struct HyperliquidRestInfo {
    config: HyperliquidConfig,

    instruments: HashMap<String, HyperliquidInstrumentInfo>,
}

impl HyperliquidRestInfo {
    pub fn new(config: HyperliquidConfig) -> Self {
        Self {
            config,
            instruments: HashMap::new(),
        }
    }

    pub fn info_url(&self) -> String {
        format!("{}/info", self.config.rest_url)
    }

    pub fn cache_instrument(&mut self, info: HyperliquidInstrumentInfo) {
        self.instruments.insert(info.coin.clone(), info);
    }

    pub fn get_instrument(&self, coin: &str) -> Option<&HyperliquidInstrumentInfo> {
        self.instruments.get(coin)
    }
}

pub struct HyperliquidExecution {
    config: HyperliquidConfig,
    nonce_manager: NonceManager,

    pending_orders: HashMap<String, HyperliquidOrderRequest>,

    open_order_count: u32,
    max_open_orders: u32,
}

impl HyperliquidExecution {
    pub fn new(config: HyperliquidConfig) -> Self {
        Self {
            config,
            nonce_manager: NonceManager::new(),
            pending_orders: HashMap::new(),
            open_order_count: 0,
            max_open_orders: 1000,
        }
    }

    pub fn exchange_url(&self) -> String {
        format!("{}/exchange", self.config.rest_url)
    }

    pub fn can_place_order(&self) -> bool {
        self.open_order_count < self.max_open_orders
    }

    pub fn next_nonce(&mut self) -> u64 {
        self.nonce_manager.next_nonce()
    }

    pub fn build_place_order(&mut self, request: &HyperliquidOrderRequest) -> PlaceOrderAction {
        let nonce = self.next_nonce();
        PlaceOrderAction {
            nonce,
            orders: vec![request.clone()],
            grouping: OrderGrouping::Na,
        }
    }

    pub fn build_cancel_order(&mut self, coin: &str, order_id: u64) -> CancelOrderAction {
        let nonce = self.next_nonce();
        CancelOrderAction {
            nonce,
            cancels: vec![CancelRequest {
                coin: coin.to_string(),
                order_id,
            }],
        }
    }

    pub fn update_order_limit(&mut self, max_orders: u32) {
        self.max_open_orders = max_orders.min(5000);
    }

    pub fn on_order_submitted(&mut self, cloid: String, request: HyperliquidOrderRequest) {
        self.pending_orders.insert(cloid, request);
        self.open_order_count += 1;
    }

    pub fn on_order_closed(&mut self, cloid: &str) {
        self.pending_orders.remove(cloid);
        self.open_order_count = self.open_order_count.saturating_sub(1);
    }
}

#[derive(Debug, Clone)]
pub struct PlaceOrderAction {
    pub nonce: u64,
    pub orders: Vec<HyperliquidOrderRequest>,
    pub grouping: OrderGrouping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderGrouping {
    Na,
    NormalTpsl,
    PositionTpsl,
}

#[derive(Debug, Clone)]
pub struct CancelOrderAction {
    pub nonce: u64,
    pub cancels: Vec<CancelRequest>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CancelRequest {
    pub coin: String,
    #[serde(rename = "oid")]
    pub order_id: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Eip712Domain {
    pub name: String,
    pub version: String,
    pub chain_id: u64,
    pub verifying_contract: String,
}

impl Eip712Domain {
    pub fn hyperliquid_mainnet() -> Self {
        Self {
            name: "Exchange".to_string(),
            version: "1".to_string(),
            chain_id: 1337,
            verifying_contract: "0x0000000000000000000000000000000000000000".to_string(),
        }
    }

    pub fn hyperliquid_testnet() -> Self {
        Self {
            name: "Exchange".to_string(),
            version: "1".to_string(),
            chain_id: 1337,
            verifying_contract: "0x0000000000000000000000000000000000000000".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum SignableAction {
    #[serde(rename = "order")]
    Order {
        orders: Vec<OrderWire>,
        grouping: String,
    },
    #[serde(rename = "cancel")]
    Cancel { cancels: Vec<CancelRequest> },
    #[serde(rename = "cancelByCloid")]
    CancelByCloid { cancels: Vec<CloidCancel> },
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderWire {
    #[serde(rename = "a")]
    pub asset: u32,
    #[serde(rename = "b")]
    pub is_buy: bool,
    #[serde(rename = "p")]
    pub limit_px: String,
    #[serde(rename = "s")]
    pub sz: String,
    #[serde(rename = "r")]
    pub reduce_only: bool,
    #[serde(rename = "t")]
    pub order_type: OrderTypeWire,
    #[serde(rename = "c", skip_serializing_if = "Option::is_none")]
    pub cloid: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderTypeWire {
    pub limit: LimitOrderType,
}

#[derive(Debug, Clone, Serialize)]
pub struct LimitOrderType {
    pub tif: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CloidCancel {
    pub asset: u32,
    pub cloid: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SignedRequest {
    pub action: SignableAction,
    pub nonce: u64,
    pub signature: Signature,
    #[serde(rename = "vaultAddress", skip_serializing_if = "Option::is_none")]
    pub vault_address: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Signature {
    pub r: String,
    pub s: String,
    pub v: u8,
}

pub struct HyperliquidSigner {
    private_key: [u8; 32],

    address: String,

    domain: Eip712Domain,
}

impl HyperliquidSigner {
    pub fn new(private_key_hex: &str, testnet: bool) -> Result<Self, HyperliquidError> {
        let key_hex = private_key_hex
            .strip_prefix("0x")
            .unwrap_or(private_key_hex);
        let key_bytes =
            hex::decode(key_hex).map_err(|e| HyperliquidError::InvalidPrivateKey(e.to_string()))?;

        if key_bytes.len() != 32 {
            return Err(HyperliquidError::InvalidPrivateKey(
                "Private key must be 32 bytes".to_string(),
            ));
        }

        let mut private_key = [0u8; 32];
        private_key.copy_from_slice(&key_bytes);

        let address = Self::derive_address(&private_key);

        let domain = if testnet {
            Eip712Domain::hyperliquid_testnet()
        } else {
            Eip712Domain::hyperliquid_mainnet()
        };

        Ok(Self {
            private_key,
            address,
            domain,
        })
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    fn derive_address(private_key: &[u8; 32]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(private_key);
        let hash = hasher.finalize();
        format!("0x{}", hex::encode(&hash[..20]))
    }

    pub fn hash_action(&self, action: &SignableAction, nonce: u64) -> [u8; 32] {
        let action_json = serde_json::to_string(action).unwrap_or_default();

        let mut hasher = Sha256::new();
        hasher.update(self.domain.name.as_bytes());
        hasher.update(self.domain.version.as_bytes());
        hasher.update(&self.domain.chain_id.to_be_bytes());
        hasher.update(action_json.as_bytes());
        hasher.update(&nonce.to_be_bytes());

        let mut result = [0u8; 32];
        result.copy_from_slice(&hasher.finalize());
        result
    }

    pub fn sign(&self, action: &SignableAction, nonce: u64) -> Result<Signature, HyperliquidError> {
        let hash = self.hash_action(action, nonce);

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(&self.private_key)
            .map_err(|e| HyperliquidError::SigningError(e.to_string()))?;
        mac.update(&hash);
        let sig = mac.finalize().into_bytes();

        Ok(Signature {
            r: hex::encode(&sig[..32]),
            s: hex::encode(&hash[..32]),
            v: 27,
        })
    }

    pub fn sign_request(
        &self,
        action: SignableAction,
        nonce: u64,
        vault_address: Option<String>,
    ) -> Result<SignedRequest, HyperliquidError> {
        let signature = self.sign(&action, nonce)?;
        Ok(SignedRequest {
            action,
            nonce,
            signature,
            vault_address,
        })
    }
}

pub struct HyperliquidAdapter {
    pub config: HyperliquidConfig,
    pub market_data: HyperliquidWsMarketData,
    pub user_stream: HyperliquidWsUserStream,
    pub rest_info: HyperliquidRestInfo,
    pub execution: HyperliquidExecution,
}

impl HyperliquidAdapter {
    pub fn new(config: HyperliquidConfig) -> Self {
        Self {
            market_data: HyperliquidWsMarketData::new(config.clone()),
            user_stream: HyperliquidWsUserStream::new(config.clone()),
            rest_info: HyperliquidRestInfo::new(config.clone()),
            execution: HyperliquidExecution::new(config.clone()),
            config,
        }
    }

    pub fn venue(&self) -> Venue {
        Venue::Hyperliquid
    }

    pub fn to_instrument_id(&self, coin: &str) -> InstrumentId {
        InstrumentId::new(Venue::Hyperliquid, coin, InstrumentType::Perp)
    }

    pub fn to_coin(&self, instrument_id: &InstrumentId) -> Option<String> {
        if instrument_id.venue == Venue::Hyperliquid {
            Some(instrument_id.symbol.clone())
        } else {
            None
        }
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

            delay_ms = (delay_ms as f64 * self.reconnect_config.backoff_multiplier) as u64;
            delay_ms = delay_ms.min(self.reconnect_config.max_delay_ms);

            let (tx, rx) = mpsc::unbounded_channel();
            self.tx = tx;
            self.rx = rx;
        }
    }
}

pub struct HyperliquidMarketDataHandler {
    subscriptions: Vec<WsSubscription>,
    message_callback: Box<dyn Fn(HyperliquidWsMessage) + Send + Sync>,
}

impl HyperliquidMarketDataHandler {
    pub fn new<F>(subscriptions: Vec<WsSubscription>, callback: F) -> Self
    where
        F: Fn(HyperliquidWsMessage) + Send + Sync + 'static,
    {
        Self {
            subscriptions,
            message_callback: Box::new(callback),
        }
    }
}

#[async_trait]
impl WsMessageHandler for HyperliquidMarketDataHandler {
    async fn on_message(&mut self, message: String) {
        match serde_json::from_str::<HyperliquidWsMessage>(&message) {
            Ok(msg) => (self.message_callback)(msg),
            Err(e) => warn!("Failed to parse Hyperliquid message: {}", e),
        }
    }

    async fn on_connected(&mut self) {
        info!("Hyperliquid market data connected");
    }

    async fn on_disconnected(&mut self) {
        warn!("Hyperliquid market data disconnected");
    }

    async fn on_error(&mut self, error: String) {
        error!("Hyperliquid market data error: {}", error);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "channel")]
pub enum HyperliquidWsMessage {
    #[serde(rename = "allMids")]
    AllMids { data: AllMidsData },
    #[serde(rename = "l2Book")]
    L2Book { data: L2BookData },
    #[serde(rename = "trades")]
    Trades { data: Vec<TradeData> },
    #[serde(rename = "user")]
    User { data: UserData },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllMidsData {
    pub mids: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2BookData {
    pub coin: String,
    pub time: u64,
    pub levels: Vec<Vec<PriceLevel>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub px: String,
    pub sz: String,
    pub n: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeData {
    pub coin: String,
    pub side: String,
    pub px: String,
    pub sz: String,
    pub time: u64,
    pub tid: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UserData {
    #[serde(rename = "fill")]
    Fill { data: FillData },
    #[serde(rename = "order")]
    Order { data: OrderData },
    #[serde(rename = "notification")]
    Notification { data: NotificationData },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillData {
    pub coin: String,
    pub px: String,
    pub sz: String,
    pub side: String,
    pub time: u64,
    pub oid: u64,
    pub tid: u64,
    pub fee: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderData {
    pub coin: String,
    pub side: String,
    pub limit_px: String,
    pub sz: String,
    pub oid: u64,
    pub timestamp: u64,
    pub order_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationData {
    pub notification: String,
}

impl HyperliquidWsMarketData {
    pub async fn connect<F>(mut self, callback: F) -> Result<(), HyperliquidError>
    where
        F: Fn(HyperliquidWsMessage) + Send + Sync + 'static,
    {
        let subscriptions = std::mem::take(&mut self.subscriptions);

        let client = WsClient::new(self.config.ws_url.clone(), self.config.reconnect.clone());

        let sender = client.sender();

        let sub_messages = subscriptions
            .iter()
            .map(|s| s.to_json())
            .collect::<Vec<_>>();
        tokio::spawn(async move {
            sleep(Duration::from_millis(100)).await;
            for msg in sub_messages {
                let _ = sender.send(msg);
            }
        });

        let handler = HyperliquidMarketDataHandler::new(subscriptions, callback);
        client.run(handler).await;

        Ok(())
    }
}

use reqwest::Client as HttpClient;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandleInterval {
    #[serde(rename = "1m")]
    Min1,
    #[serde(rename = "5m")]
    Min5,
    #[serde(rename = "15m")]
    Min15,
    #[serde(rename = "1h")]
    Hour1,
    #[serde(rename = "4h")]
    Hour4,
    #[serde(rename = "1d")]
    Day1,
}

impl CandleInterval {
    pub fn as_str(&self) -> &'static str {
        match self {
            CandleInterval::Min1 => "1m",
            CandleInterval::Min5 => "5m",
            CandleInterval::Min15 => "15m",
            CandleInterval::Hour1 => "1h",
            CandleInterval::Hour4 => "4h",
            CandleInterval::Day1 => "1d",
        }
    }

    pub fn duration_ms(&self) -> u64 {
        match self {
            CandleInterval::Min1 => 60_000,
            CandleInterval::Min5 => 300_000,
            CandleInterval::Min15 => 900_000,
            CandleInterval::Hour1 => 3_600_000,
            CandleInterval::Hour4 => 14_400_000,
            CandleInterval::Day1 => 86_400_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperliquidCandle {
    #[serde(rename = "t")]
    pub timestamp: u64,

    #[serde(rename = "o")]
    pub open: String,

    #[serde(rename = "h")]
    pub high: String,

    #[serde(rename = "l")]
    pub low: String,

    #[serde(rename = "c")]
    pub close: String,

    #[serde(rename = "v")]
    pub volume: String,

    #[serde(rename = "n")]
    pub num_trades: u64,
}

impl HyperliquidCandle {
    pub fn open_f64(&self) -> f64 {
        self.open.parse().unwrap_or(0.0)
    }
    pub fn high_f64(&self) -> f64 {
        self.high.parse().unwrap_or(0.0)
    }
    pub fn low_f64(&self) -> f64 {
        self.low.parse().unwrap_or(0.0)
    }
    pub fn close_f64(&self) -> f64 {
        self.close.parse().unwrap_or(0.0)
    }
    pub fn volume_f64(&self) -> f64 {
        self.volume.parse().unwrap_or(0.0)
    }
}

#[derive(Debug, Serialize)]
struct CandleSnapshotRequest {
    #[serde(rename = "type")]
    req_type: String,
    req: CandleSnapshotParams,
}

#[derive(Debug, Serialize)]
struct CandleSnapshotParams {
    coin: String,
    interval: String,
    #[serde(rename = "startTime")]
    start_time: u64,
    #[serde(rename = "endTime")]
    end_time: u64,
}

pub struct HyperliquidHistoricalClient {
    config: HyperliquidConfig,
    http_client: HttpClient,
}

impl HyperliquidHistoricalClient {
    pub fn new(config: HyperliquidConfig) -> Self {
        Self {
            config,
            http_client: HttpClient::new(),
        }
    }

    pub async fn fetch_candles(
        &self,
        coin: &str,
        interval: CandleInterval,
        start_time_ms: u64,
        end_time_ms: u64,
    ) -> Result<Vec<HyperliquidCandle>, HyperliquidError> {
        let url = format!("{}/info", self.config.rest_url);

        let request = CandleSnapshotRequest {
            req_type: "candleSnapshot".to_string(),
            req: CandleSnapshotParams {
                coin: coin.to_string(),
                interval: interval.as_str().to_string(),
                start_time: start_time_ms,
                end_time: end_time_ms,
            },
        };

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| HyperliquidError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(HyperliquidError::RequestError(format!(
                "HTTP {}: {}",
                status, text
            )));
        }

        let candles: Vec<HyperliquidCandle> = response
            .json()
            .await
            .map_err(|e| HyperliquidError::InvalidResponse(e.to_string()))?;

        Ok(candles)
    }

    pub async fn fetch_candles_range(
        &self,
        coin: &str,
        interval: CandleInterval,
        start_time_ms: u64,
        end_time_ms: u64,
        max_candles_per_request: usize,
    ) -> Result<Vec<HyperliquidCandle>, HyperliquidError> {
        let mut all_candles = Vec::new();
        let mut current_start = start_time_ms;
        let interval_ms = interval.duration_ms();
        let max_range = interval_ms * max_candles_per_request as u64;

        while current_start < end_time_ms {
            let chunk_end = (current_start + max_range).min(end_time_ms);

            let candles = self
                .fetch_candles(coin, interval, current_start, chunk_end)
                .await?;

            if candles.is_empty() {
                break;
            }

            let last_ts = candles.last().map(|c| c.timestamp).unwrap_or(chunk_end);
            all_candles.extend(candles);

            current_start = last_ts + interval_ms;

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Ok(all_candles)
    }

    pub async fn fetch_recent_trades(
        &self,
        coin: &str,
    ) -> Result<Vec<TradeData>, HyperliquidError> {
        Err(HyperliquidError::RequestError(
            "Historical trades not available via REST. Use candle data or collect trades via WebSocket.".to_string()
        ))
    }

    pub async fn fetch_meta(&self) -> Result<HyperliquidMeta, HyperliquidError> {
        let url = format!("{}/info", self.config.rest_url);

        #[derive(Serialize)]
        struct MetaRequest {
            #[serde(rename = "type")]
            req_type: String,
        }

        let request = MetaRequest {
            req_type: "meta".to_string(),
        };

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| HyperliquidError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(HyperliquidError::RequestError(format!(
                "HTTP {}: {}",
                status, text
            )));
        }

        let meta: HyperliquidMeta = response
            .json()
            .await
            .map_err(|e| HyperliquidError::InvalidResponse(e.to_string()))?;

        Ok(meta)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperliquidMeta {
    pub universe: Vec<HyperliquidAssetInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperliquidAssetInfo {
    pub name: String,
    #[serde(rename = "szDecimals")]
    pub sz_decimals: u32,
    #[serde(rename = "maxLeverage")]
    pub max_leverage: Option<u32>,
}

pub struct HyperliquidDataFeed {
    data: Vec<HyperliquidDataPoint>,

    index: usize,

    coin: String,

    interval: CandleInterval,
}

#[derive(Debug, Clone)]
pub struct HyperliquidDataPoint {
    pub timestamp_ms: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub num_trades: u64,
}

impl HyperliquidDataFeed {
    pub fn new(coin: String, interval: CandleInterval) -> Self {
        Self {
            data: Vec::new(),
            index: 0,
            coin,
            interval,
        }
    }

    pub async fn load(
        &mut self,
        config: &HyperliquidConfig,
        start_time_ms: u64,
        end_time_ms: u64,
    ) -> Result<usize, HyperliquidError> {
        let client = HyperliquidHistoricalClient::new(config.clone());

        let candles = client
            .fetch_candles_range(&self.coin, self.interval, start_time_ms, end_time_ms, 5000)
            .await?;

        self.data = candles
            .into_iter()
            .map(|c| HyperliquidDataPoint {
                timestamp_ms: c.timestamp,
                open: c.open_f64(),
                high: c.high_f64(),
                low: c.low_f64(),
                close: c.close_f64(),
                volume: c.volume_f64(),
                num_trades: c.num_trades,
            })
            .collect();

        self.data.sort_by_key(|d| d.timestamp_ms);
        self.index = 0;

        Ok(self.data.len())
    }

    pub fn next(&mut self) -> Option<&HyperliquidDataPoint> {
        if self.index < self.data.len() {
            let point = &self.data[self.index];
            self.index += 1;
            Some(point)
        } else {
            None
        }
    }

    pub fn peek_timestamp(&self) -> Option<u64> {
        self.data.get(self.index).map(|d| d.timestamp_ms)
    }

    pub fn reset(&mut self) {
        self.index = 0;
    }

    pub fn data(&self) -> &[HyperliquidDataPoint] {
        &self.data
    }

    pub fn coin(&self) -> &str {
        &self.coin
    }

    pub fn interval(&self) -> CandleInterval {
        self.interval
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_urls() {
        let mainnet = HyperliquidConfig::mainnet();
        assert_eq!(mainnet.ws_url, "wss://api.hyperliquid.xyz/ws");
        assert!(!mainnet.testnet);

        let testnet = HyperliquidConfig::testnet();
        assert_eq!(testnet.ws_url, "wss://api.hyperliquid-testnet.xyz/ws");
        assert!(testnet.testnet);
    }

    #[test]
    fn test_nonce_manager() {
        let mut manager = NonceManager::new();

        let n1 = manager.next_nonce();
        let n2 = manager.next_nonce();
        let n3 = manager.next_nonce();

        assert!(n2 > n1);
        assert!(n3 > n2);
        assert!(manager.is_valid_nonce(n3));
    }

    #[test]
    fn test_subscription_json() {
        let sub = WsSubscription {
            subscription_type: WsSubscriptionType::L2Book,
            instrument: Some("BTC".to_string()),
        };

        let json = sub.to_json();
        assert!(json.contains("l2Book"));
        assert!(json.contains("BTC"));
    }

    #[test]
    fn test_order_limits() {
        let config = HyperliquidConfig::mainnet();
        let mut execution = HyperliquidExecution::new(config);

        assert!(execution.can_place_order());
        assert_eq!(execution.max_open_orders, 1000);

        execution.update_order_limit(5000);
        assert_eq!(execution.max_open_orders, 5000);

        execution.update_order_limit(10000);
        assert_eq!(execution.max_open_orders, 5000);
    }
}
