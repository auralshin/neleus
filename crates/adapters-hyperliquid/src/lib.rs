// Module declarations
mod adapters;
mod config;
mod execution;
mod historical;
mod signing;
mod types;
mod websocket;

// Re-export public API
pub use config::{
    HyperliquidConfig, HyperliquidError, NonceManager, RateLimitConfig, ReconnectConfig,
};

pub use types::{
    AllMidsData, CancelOrderAction, CancelOrderResponse, CancelRequest, FillData, HyperliquidBook,
    HyperliquidBookLevel, HyperliquidInstrumentInfo, HyperliquidOrderRequest,
    HyperliquidOrderStatus, HyperliquidOrderType, HyperliquidOrderTypeRequest, HyperliquidTif,
    HyperliquidTrade, HyperliquidUserFill, HyperliquidUserOrder, HyperliquidWsMessage, L2BookData,
    NotificationData, OrderData, OrderGrouping, OrderIdResponse, OrderStatus, PlaceOrderAction,
    PlaceOrderResponse, PriceLevel, TpSlType, TradeData, UserData, WsSubscription,
    WsSubscriptionType,
};

pub use signing::{
    CloidCancel, Eip712Domain, HyperliquidSigner, LimitOrderType, OrderTypeWire, OrderWire,
    SignableAction, Signature, SignedRequest,
};

pub use websocket::{
    BusConnectedMarketDataHandler, HyperliquidBusClient, HyperliquidMarketDataHandler, WsClient,
    WsMessageHandler,
};

pub use historical::{
    CandleInterval, HyperliquidAssetInfo, HyperliquidCandle, HyperliquidDataFeed,
    HyperliquidDataPoint, HyperliquidHistoricalClient, HyperliquidMeta, HyperliquidSpotMarketInfo,
    HyperliquidSpotMeta, HyperliquidSpotTokenInfo, HyperliquidOutcomeMeta, HyperliquidOutcome, 
    HyperliquidOutcomeSideSpec,
};

pub use execution::{HyperliquidExecution, HyperliquidExecutionClient};

pub use adapters::{HyperliquidAdapter, HyperliquidDataAdapter, HyperliquidExecutionAdapter};

use std::collections::HashMap;

#[allow(dead_code)]
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

    pub async fn connect<F>(mut self, callback: F) -> Result<(), HyperliquidError>
    where
        F: Fn(HyperliquidWsMessage) + Send + Sync + 'static,
    {
        use tokio::time::{sleep, Duration};

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

#[allow(dead_code)]
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
