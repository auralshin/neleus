use crate::config::{HyperliquidConfig, HyperliquidError, ReconnectConfig};
use crate::types::*;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use neleus_core_bus::{Message, MessageKind, Topic};
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

            delay_ms = (delay_ms as f64 * self.reconnect_config.backoff_multiplier) as u64;
            delay_ms = delay_ms.min(self.reconnect_config.max_delay_ms);

            let (tx, rx) = mpsc::unbounded_channel();
            self.tx = tx;
            self.rx = rx;
        }
    }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
pub struct BusConnectedMarketDataHandler<F>
where
    F: Fn(Message) + Send + Sync,
{
    subscriptions: Vec<WsSubscription>,
    bus_publisher: Arc<F>,
    is_connected: bool,
}

impl<F> BusConnectedMarketDataHandler<F>
where
    F: Fn(Message) + Send + Sync,
{
    pub fn new(subscriptions: Vec<WsSubscription>, bus_publisher: F) -> Self {
        Self {
            subscriptions,
            bus_publisher: Arc::new(bus_publisher),
            is_connected: false,
        }
    }

    fn publish_to_bus(&self, topic: Topic, payload: Vec<u8>) {
        let msg = Message::new(MessageKind::Data, topic, payload);
        (self.bus_publisher)(msg);
    }

    fn handle_all_mids(&self, data: &AllMidsData) {
        for (coin, mid) in &data.mids {
            if let Ok(price) = mid.parse::<f64>() {
                let payload = serde_json::json!({
                    "venue": "hyperliquid",
                    "symbol": coin,
                    "mid_price": price,
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64
                });
                let topic = Topic::market_data(coin);
                self.publish_to_bus(topic, payload.to_string().into_bytes());
            }
        }
    }

    fn handle_l2_book(&self, data: &L2BookData) {
        let payload = serde_json::json!({
            "venue": "hyperliquid",
            "symbol": data.coin,
            "time": data.time,
            "levels": data.levels,
        });
        let topic = Topic::orderbook(&data.coin);
        self.publish_to_bus(topic, payload.to_string().into_bytes());
    }

    fn handle_trades(&self, trades: &[TradeData]) {
        for trade in trades {
            let payload = serde_json::json!({
                "venue": "hyperliquid",
                "symbol": trade.coin,
                "side": trade.side,
                "price": trade.px,
                "size": trade.sz,
                "time": trade.time,
                "trade_id": trade.tid,
            });
            let topic = Topic::trades(&trade.coin);
            self.publish_to_bus(topic, payload.to_string().into_bytes());
        }
    }

    fn handle_user_data(&self, data: &UserData) {
        match data {
            UserData::Fill { data: fill } => {
                let payload = serde_json::json!({
                    "type": "fill",
                    "venue": "hyperliquid",
                    "symbol": fill.coin,
                    "side": fill.side,
                    "price": fill.px,
                    "size": fill.sz,
                    "time": fill.time,
                    "order_id": fill.oid,
                    "trade_id": fill.tid,
                    "fee": fill.fee,
                });
                let topic = Topic::fill_events();
                self.publish_to_bus(topic, payload.to_string().into_bytes());
            }
            UserData::Order { data: order } => {
                let payload = serde_json::json!({
                    "type": "order",
                    "venue": "hyperliquid",
                    "symbol": order.coin,
                    "side": order.side,
                    "price": order.limit_px,
                    "size": order.sz,
                    "order_id": order.oid,
                    "timestamp": order.timestamp,
                    "order_type": order.order_type,
                });
                let topic = Topic::order_events();
                self.publish_to_bus(topic, payload.to_string().into_bytes());
            }
            UserData::Notification { data: notif } => {
                let payload = serde_json::json!({
                    "type": "notification",
                    "venue": "hyperliquid",
                    "message": notif.notification,
                });
                let topic = Topic::system();
                self.publish_to_bus(topic, payload.to_string().into_bytes());
            }
        }
    }
}

#[async_trait]
impl<F> WsMessageHandler for BusConnectedMarketDataHandler<F>
where
    F: Fn(Message) + Send + Sync + 'static,
{
    async fn on_message(&mut self, message: String) {
        match serde_json::from_str::<HyperliquidWsMessage>(&message) {
            Ok(msg) => match &msg {
                HyperliquidWsMessage::AllMids { data } => self.handle_all_mids(data),
                HyperliquidWsMessage::L2Book { data } => self.handle_l2_book(data),
                HyperliquidWsMessage::Trades { data } => self.handle_trades(data),
                HyperliquidWsMessage::User { data } => self.handle_user_data(data),
            },
            Err(e) => warn!("Failed to parse Hyperliquid message: {}", e),
        }
    }

    async fn on_connected(&mut self) {
        self.is_connected = true;
        info!("Hyperliquid bus-connected handler: connected");

        let payload = serde_json::json!({
            "type": "connected",
            "venue": "hyperliquid",
        });
        let msg = Message::new(
            MessageKind::System,
            Topic::system(),
            payload.to_string().into_bytes(),
        );
        (self.bus_publisher)(msg);
    }

    async fn on_disconnected(&mut self) {
        self.is_connected = false;
        warn!("Hyperliquid bus-connected handler: disconnected");

        let payload = serde_json::json!({
            "type": "disconnected",
            "venue": "hyperliquid",
        });
        let msg = Message::new(
            MessageKind::System,
            Topic::system(),
            payload.to_string().into_bytes(),
        );
        (self.bus_publisher)(msg);
    }

    async fn on_error(&mut self, error: String) {
        error!("Hyperliquid bus-connected handler error: {}", error);

        let payload = serde_json::json!({
            "type": "error",
            "venue": "hyperliquid",
            "error": error,
        });
        let msg = Message::new(
            MessageKind::System,
            Topic::system(),
            payload.to_string().into_bytes(),
        );
        (self.bus_publisher)(msg);
    }
}

pub struct HyperliquidBusClient {
    config: HyperliquidConfig,
    subscriptions: Vec<WsSubscription>,
}

impl HyperliquidBusClient {
    pub fn new(config: HyperliquidConfig) -> Self {
        Self {
            config,
            subscriptions: Vec::new(),
        }
    }

    pub fn subscribe(mut self, subscription: WsSubscription) -> Self {
        self.subscriptions.push(subscription);
        self
    }

    pub fn subscribe_all_mids(self) -> Self {
        self.subscribe(WsSubscription {
            subscription_type: WsSubscriptionType::AllMids,
            instrument: None,
        })
    }

    pub fn subscribe_l2_book(self, coin: &str) -> Self {
        self.subscribe(WsSubscription {
            subscription_type: WsSubscriptionType::L2Book,
            instrument: Some(coin.to_string()),
        })
    }

    pub fn subscribe_trades(self, coin: &str) -> Self {
        self.subscribe(WsSubscription {
            subscription_type: WsSubscriptionType::Trades,
            instrument: Some(coin.to_string()),
        })
    }

    pub fn subscribe_user_fills(self) -> Self {
        self.subscribe(WsSubscription {
            subscription_type: WsSubscriptionType::UserFills,
            instrument: None,
        })
    }

    pub fn subscribe_user_orders(self) -> Self {
        self.subscribe(WsSubscription {
            subscription_type: WsSubscriptionType::UserOrders,
            instrument: None,
        })
    }

    pub async fn connect_to_bus<F>(self, bus_publisher: F) -> Result<(), HyperliquidError>
    where
        F: Fn(Message) + Send + Sync + 'static,
    {
        let client = WsClient::new(self.config.ws_url.clone(), self.config.reconnect.clone());
        let sender = client.sender();

        let sub_messages: Vec<String> = self.subscriptions.iter().map(|s| s.to_json()).collect();
        tokio::spawn(async move {
            sleep(Duration::from_millis(100)).await;
            for msg in sub_messages {
                let _ = sender.send(msg);
            }
        });

        let handler = BusConnectedMarketDataHandler::new(self.subscriptions, bus_publisher);
        client.run(handler).await;

        Ok(())
    }
}
