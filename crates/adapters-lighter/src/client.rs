use crate::auth::LighterSigner;
use crate::config::{LighterConfig, LighterError, RateLimitConfig, ReconnectConfig, WeightedRateLimiter};
use crate::types::{
    LighterMarketDataMessage, LighterMarketInfo, LighterOrderRequest, OrderBookData,
    LighterTradeData, LighterFillData, LighterOrderData, WsSubscription, WsSubscriptionType,
};
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use neleus_core_bus::{Message, MessageKind, Topic};
use std::collections::HashMap;
use std::sync::Arc;
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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
    pub rate_limiter: WeightedRateLimiter,
    pending_orders: HashMap<String, LighterOrderRequest>,
    pub signer: Option<LighterSigner>,
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

#[allow(dead_code)]
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

/// Market data handler that publishes messages to the event bus
pub struct LighterBusConnectedHandler<F>
where
    F: Fn(Message) + Send + Sync,
{
    #[allow(dead_code)]
    subscriptions: Vec<WsSubscription>,
    bus_publisher: Arc<F>,
    markets: HashMap<u32, String>, // market_id -> symbol
}

impl<F> LighterBusConnectedHandler<F>
where
    F: Fn(Message) + Send + Sync,
{
    pub fn new(subscriptions: Vec<WsSubscription>, bus_publisher: F, markets: HashMap<u32, String>) -> Self {
        Self {
            subscriptions,
            bus_publisher: Arc::new(bus_publisher),
            markets,
        }
    }

    fn publish_to_bus(&self, topic: Topic, payload: Vec<u8>) {
        let msg = Message::new(MessageKind::Data, topic, payload);
        (self.bus_publisher)(msg);
    }

    fn get_symbol(&self, market_id: u32) -> String {
        self.markets.get(&market_id).cloned().unwrap_or_else(|| format!("market_{}", market_id))
    }

    fn handle_orderbook(&self, data: &OrderBookData) {
        let symbol = self.get_symbol(data.market_id);
        let payload = serde_json::json!({
            "venue": "lighter",
            "symbol": symbol,
            "market_id": data.market_id,
            "timestamp": data.timestamp,
            "bids": data.bids,
            "asks": data.asks,
        });
        let topic = Topic::orderbook(&symbol);
        self.publish_to_bus(topic, payload.to_string().into_bytes());
    }

    fn handle_trade(&self, data: &LighterTradeData) {
        let symbol = self.get_symbol(data.market_id);
        let payload = serde_json::json!({
            "venue": "lighter",
            "symbol": symbol,
            "market_id": data.market_id,
            "trade_id": data.trade_id,
            "price": data.price,
            "size": data.size,
            "side": data.side,
            "timestamp": data.timestamp,
        });
        let topic = Topic::trades(&symbol);
        self.publish_to_bus(topic, payload.to_string().into_bytes());
    }

    fn handle_fill(&self, data: &LighterFillData) {
        let symbol = self.get_symbol(data.market_id);
        let payload = serde_json::json!({
            "type": "fill",
            "venue": "lighter",
            "symbol": symbol,
            "fill_id": data.fill_id,
            "order_id": data.order_id,
            "price": data.price,
            "size": data.size,
            "side": data.side,
            "fee": data.fee,
            "timestamp": data.timestamp,
        });
        let topic = Topic::fill_events();
        self.publish_to_bus(topic, payload.to_string().into_bytes());
    }

    fn handle_order(&self, data: &LighterOrderData) {
        let symbol = self.get_symbol(data.market_id);
        let payload = serde_json::json!({
            "type": "order",
            "venue": "lighter",
            "symbol": symbol,
            "order_id": data.order_id,
            "price": data.price,
            "size": data.size,
            "side": data.side,
            "order_type": data.order_type,
            "status": data.status,
            "timestamp": data.timestamp,
        });
        let topic = Topic::order_events();
        self.publish_to_bus(topic, payload.to_string().into_bytes());
    }
}

#[async_trait]
impl<F> WsMessageHandler for LighterBusConnectedHandler<F>
where
    F: Fn(Message) + Send + Sync + 'static,
{
    async fn on_message(&mut self, message: String) {
        match serde_json::from_str::<LighterMarketDataMessage>(&message) {
            Ok(msg) => {
                match &msg {
                    LighterMarketDataMessage::OrderBook { data } => self.handle_orderbook(data),
                    LighterMarketDataMessage::Trade { data } => self.handle_trade(data),
                    LighterMarketDataMessage::Fill { data } => self.handle_fill(data),
                    LighterMarketDataMessage::Order { data } => self.handle_order(data),
                    LighterMarketDataMessage::Account { .. } => {
                        // Account updates - could publish to a dedicated topic
                    }
                }
            }
            Err(e) => warn!("Failed to parse Lighter message: {}", e),
        }
    }

    async fn on_connected(&mut self) {
        info!("Lighter bus-connected handler: connected");
        let payload = serde_json::json!({
            "type": "connected",
            "venue": "lighter",
        });
        let msg = Message::new(MessageKind::System, Topic::system(), payload.to_string().into_bytes());
        (self.bus_publisher)(msg);
    }

    async fn on_disconnected(&mut self) {
        warn!("Lighter bus-connected handler: disconnected");
        let payload = serde_json::json!({
            "type": "disconnected",
            "venue": "lighter",
        });
        let msg = Message::new(MessageKind::System, Topic::system(), payload.to_string().into_bytes());
        (self.bus_publisher)(msg);
    }

    async fn on_error(&mut self, error: String) {
        error!("Lighter bus-connected handler error: {}", error);
        let payload = serde_json::json!({
            "type": "error",
            "venue": "lighter",
            "error": error,
        });
        let msg = Message::new(MessageKind::System, Topic::system(), payload.to_string().into_bytes());
        (self.bus_publisher)(msg);
    }
}

/// Helper to create a bus-connected Lighter WebSocket client
pub struct LighterBusClient {
    config: LighterConfig,
    subscriptions: Vec<WsSubscription>,
    markets: HashMap<u32, String>,
}

impl LighterBusClient {
    pub fn new(config: LighterConfig) -> Self {
        Self {
            config,
            subscriptions: Vec::new(),
            markets: HashMap::new(),
        }
    }

    pub fn with_market(mut self, market_id: u32, symbol: &str) -> Self {
        self.markets.insert(market_id, symbol.to_string());
        self
    }

    pub fn subscribe(mut self, subscription: WsSubscription) -> Self {
        self.subscriptions.push(subscription);
        self
    }

    pub fn subscribe_orderbook(self, market_id: u32) -> Self {
        self.subscribe(WsSubscription {
            subscription_type: WsSubscriptionType::OrderBook,
            market_id: Some(market_id),
        })
    }

    pub fn subscribe_trades(self, market_id: u32) -> Self {
        self.subscribe(WsSubscription {
            subscription_type: WsSubscriptionType::Trades,
            market_id: Some(market_id),
        })
    }

    /// Connect and publish all messages to the bus via the provided publisher function
    pub async fn connect_to_bus<F>(self, bus_publisher: F) -> Result<(), LighterError>
    where
        F: Fn(Message) + Send + Sync + 'static,
    {
        let client = WsClient::new(self.config.ws_url.clone(), self.config.reconnect.clone());
        let sender = client.sender();

        let sub_messages: Vec<String> = self.subscriptions.iter().map(|s| s.to_message().to_json()).collect();
        tokio::spawn(async move {
            sleep(Duration::from_millis(100)).await;
            for msg in sub_messages {
                let _ = sender.send(msg);
            }
        });

        let handler = LighterBusConnectedHandler::new(self.subscriptions, bus_publisher, self.markets);
        client.run(handler).await;

        Ok(())
    }
}
