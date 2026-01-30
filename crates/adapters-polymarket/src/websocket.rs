use crate::{
    auth::L2Authenticator, PolymarketConfig, PolymarketError, WsSubscription,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing::{debug, error, info, warn};

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum PolymarketWsMessage {
    #[serde(rename = "book")]
    Book {
        market: String,
        asset_id: String,
        hash: String,
        bids: Vec<BookLevel>,
        asks: Vec<BookLevel>,
        timestamp: String,
    },
    #[serde(rename = "price_change")]
    PriceChange {
        market: String,
        asset_id: String,
        price: String,
    },
    #[serde(rename = "last_trade_price")]
    LastTradePrice {
        market: String,
        asset_id: String,
        price: String,
    },
    #[serde(rename = "tick_size")]
    TickSize {
        market: String,
        asset_id: String,
        tick_size: String,
    },
    #[serde(rename = "user")]
    UserUpdate {
        #[serde(rename = "type")]
        update_type: String,
        order: Option<serde_json::Value>,
        fill: Option<serde_json::Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookLevel {
    pub price: String,
    pub size: String,
}

/// WebSocket client for Polymarket CLOB
pub struct PolymarketWebSocket {
    config: PolymarketConfig,
    ws_stream: Option<Arc<Mutex<WsStream>>>,
    message_tx: Option<mpsc::UnboundedSender<PolymarketWsMessage>>,
    l2_auth: Option<L2Authenticator>,
}

impl PolymarketWebSocket {
    pub fn new(config: PolymarketConfig) -> Self {
        Self {
            config,
            ws_stream: None,
            message_tx: None,
            l2_auth: None,
        }
    }

    pub fn with_l2_auth(mut self, auth: L2Authenticator) -> Self {
        self.l2_auth = Some(auth);
        self
    }

    /// Connect to the WebSocket server
    pub async fn connect(&mut self) -> Result<(), PolymarketError> {
        info!("Connecting to Polymarket WebSocket: {}", self.config.ws_url);

        let (ws_stream, _) = connect_async(&self.config.ws_url)
            .await
            .map_err(|e| PolymarketError::WebsocketError(e.to_string()))?;

        info!("WebSocket connected successfully");
        self.ws_stream = Some(Arc::new(Mutex::new(ws_stream)));

        Ok(())
    }

    /// Subscribe to market updates
    pub async fn subscribe(&mut self, subscription: WsSubscription) -> Result<(), PolymarketError> {
        let ws_stream = self
            .ws_stream
            .as_ref()
            .ok_or(PolymarketError::WebsocketError(
                "Not connected".to_string(),
            ))?;

        let message = subscription.to_json();
        debug!("Subscribing with message: {}", message);

        let mut stream = ws_stream.lock().await;
        stream
            .send(Message::Text(message))
            .await
            .map_err(|e| PolymarketError::WebsocketError(e.to_string()))?;

        Ok(())
    }

    /// Start receiving messages
    pub async fn start_receiving(
        &mut self,
    ) -> Result<mpsc::UnboundedReceiver<PolymarketWsMessage>, PolymarketError> {
        let ws_stream = self
            .ws_stream
            .as_ref()
            .ok_or(PolymarketError::WebsocketError(
                "Not connected".to_string(),
            ))?
            .clone();

        let (tx, rx) = mpsc::unbounded_channel();
        self.message_tx = Some(tx.clone());

        tokio::spawn(async move {
            Self::receive_loop(ws_stream, tx).await;
        });

        Ok(rx)
    }

    async fn receive_loop(
        ws_stream: Arc<Mutex<WsStream>>,
        tx: mpsc::UnboundedSender<PolymarketWsMessage>,
    ) {
        loop {
            let message = {
                let mut stream = ws_stream.lock().await;
                stream.next().await
            };

            match message {
                Some(Ok(Message::Text(text))) => {
                    debug!("Received WebSocket message: {}", text);
                    match serde_json::from_str::<PolymarketWsMessage>(&text) {
                        Ok(msg) => {
                            if let Err(e) = tx.send(msg) {
                                error!("Failed to send message to channel: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse WebSocket message: {}, error: {}", text, e);
                        }
                    }
                }
                Some(Ok(Message::Ping(data))) => {
                    debug!("Received ping, sending pong");
                    let mut stream = ws_stream.lock().await;
                    if let Err(e) = stream.send(Message::Pong(data)).await {
                        error!("Failed to send pong: {}", e);
                        break;
                    }
                }
                Some(Ok(Message::Close(_))) => {
                    info!("WebSocket connection closed by server");
                    break;
                }
                Some(Err(e)) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                None => {
                    warn!("WebSocket stream ended");
                    break;
                }
                _ => {}
            }
        }
    }

    /// Disconnect from the WebSocket server
    pub async fn disconnect(&mut self) -> Result<(), PolymarketError> {
        if let Some(ws_stream) = self.ws_stream.take() {
            let mut stream = ws_stream.lock().await;
            stream
                .close(None)
                .await
                .map_err(|e| PolymarketError::WebsocketError(e.to_string()))?;
            info!("WebSocket disconnected");
        }
        Ok(())
    }

    /// Subscribe to multiple markets
    pub async fn subscribe_markets(&mut self, token_ids: Vec<String>) -> Result<(), PolymarketError> {
        for token_id in token_ids {
            let subscription = WsSubscription::market(token_id);
            self.subscribe(subscription).await?;
        }
        Ok(())
    }

    /// Subscribe to user updates (requires L2 auth)
    pub async fn subscribe_user_updates(&mut self) -> Result<(), PolymarketError> {
        if self.l2_auth.is_none() {
            return Err(PolymarketError::MissingCredentials);
        }

        let subscription = WsSubscription::user();
        self.subscribe(subscription).await?;
        Ok(())
    }

    /// Reconnect with exponential backoff
    pub async fn reconnect(&mut self) -> Result<(), PolymarketError> {
        let mut delay = self.config.reconnect.initial_delay_ms;
        let mut attempts = 0;

        loop {
            match self.connect().await {
                Ok(_) => {
                    info!("Reconnected successfully");
                    return Ok(());
                }
                Err(e) => {
                    attempts += 1;
                    if let Some(max_attempts) = self.config.reconnect.max_attempts {
                        if attempts >= max_attempts {
                            error!("Max reconnection attempts reached");
                            return Err(e);
                        }
                    }

                    warn!("Reconnection attempt {} failed: {}", attempts, e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;

                    delay = (delay as f64 * self.config.reconnect.backoff_multiplier) as u64;
                    delay = delay.min(self.config.reconnect.max_delay_ms);
                }
            }
        }
    }
}

impl Drop for PolymarketWebSocket {
    fn drop(&mut self) {
        if self.ws_stream.is_some() {
            warn!("PolymarketWebSocket dropped without explicit disconnect");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_creation() {
        let config = PolymarketConfig::mainnet();
        let ws = PolymarketWebSocket::new(config);
        assert!(ws.ws_stream.is_none());
    }

    #[test]
    fn test_book_level_serialization() {
        let level = BookLevel {
            price: "0.65".to_string(),
            size: "100".to_string(),
        };

        let json = serde_json::to_string(&level).unwrap();
        let deserialized: BookLevel = serde_json::from_str(&json).unwrap();

        assert_eq!(level.price, deserialized.price);
        assert_eq!(level.size, deserialized.size);
    }
}
