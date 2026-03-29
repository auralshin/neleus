use crate::config::{HyperliquidConfig, HyperliquidError};
use crate::execution::HyperliquidExecutionClient;
use crate::types::{
    HyperliquidOrderRequest, HyperliquidOrderTypeRequest, HyperliquidTif, WsSubscription,
    WsSubscriptionType,
};
use neleus_core_engine::{
    Component, ComponentError, DataClient, DataSubscription, ExecutionClient, OrderType,
    StrategyCommand,
};
use neleus_core_types::{InstrumentId, InstrumentType, OrderId, Venue};

pub struct HyperliquidExecutionAdapter {
    client: HyperliquidExecutionClient,
    is_running: bool,
}

impl HyperliquidExecutionAdapter {
    pub fn new(config: HyperliquidConfig) -> Self {
        Self {
            client: HyperliquidExecutionClient::new(config),
            is_running: false,
        }
    }

    pub fn with_signer(
        config: HyperliquidConfig,
        private_key: &str,
    ) -> Result<Self, HyperliquidError> {
        let client = HyperliquidExecutionClient::new(config).with_signer(private_key)?;
        Ok(Self {
            client,
            is_running: false,
        })
    }

    pub fn inner(&self) -> &HyperliquidExecutionClient {
        &self.client
    }

    pub fn inner_mut(&mut self) -> &mut HyperliquidExecutionClient {
        &mut self.client
    }
}

impl Component for HyperliquidExecutionAdapter {
    fn name(&self) -> &str {
        "hyperliquid-execution"
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

impl ExecutionClient for HyperliquidExecutionAdapter {
    fn venue(&self) -> Venue {
        Venue::Hyperliquid
    }

    fn submit_order(&mut self, order: &StrategyCommand) -> Result<(), ComponentError> {
        if let StrategyCommand::SubmitOrder {
            order_id,
            instrument_id,
            side,
            order_type,
            price,
            quantity,
        } = order
        {
            let is_buy = matches!(side, neleus_core_engine::OrderSide::Buy);
            let coin = &instrument_id.symbol;

            let request = HyperliquidOrderRequest {
                coin: coin.to_string(),
                is_buy,
                limit_px: price.unwrap_or(0.0),
                sz: *quantity,
                reduce_only: false,
                order_type: match order_type {
                    OrderType::Market => HyperliquidOrderTypeRequest::Limit {
                        tif: HyperliquidTif::Ioc,
                    },
                    OrderType::Limit => HyperliquidOrderTypeRequest::Limit {
                        tif: HyperliquidTif::Gtc,
                    },
                },
                cloid: Some(order_id.to_string()),
            };

            let client = &mut self.client;
            let rt = tokio::runtime::Handle::try_current()
                .map_err(|e| ComponentError::Other(format!("No tokio runtime: {}", e)))?;

            rt.block_on(async { client.submit_order(&request).await })
                .map_err(|e| ComponentError::Other(format!("Order submission failed: {}", e)))?;

            Ok(())
        } else {
            Err(ComponentError::Other("Invalid command type".to_string()))
        }
    }

    fn cancel_order(&mut self, order_id: &OrderId) -> Result<(), ComponentError> {
        Err(ComponentError::Other(format!(
            "Cancel requires coin. Use client.cancel_order_by_cloid(coin, cloid) directly. OrderId: {}",
            order_id
        )))
    }
}

#[allow(dead_code)]
pub struct HyperliquidDataAdapter {
    config: HyperliquidConfig,
    subscriptions: Vec<DataSubscription>,
    is_running: bool,
}

impl HyperliquidDataAdapter {
    pub fn new(config: HyperliquidConfig) -> Self {
        Self {
            config,
            subscriptions: Vec::new(),
            is_running: false,
        }
    }

    pub fn subscriptions(&self) -> &[DataSubscription] {
        &self.subscriptions
    }

    fn to_ws_subscription(sub: &DataSubscription) -> Option<WsSubscription> {
        match sub {
            DataSubscription::Trades { instrument_id } => Some(WsSubscription {
                subscription_type: WsSubscriptionType::Trades,
                instrument: Some(instrument_id.symbol.to_string()),
            }),
            DataSubscription::OrderBook { instrument_id, .. } => Some(WsSubscription {
                subscription_type: WsSubscriptionType::L2Book,
                instrument: Some(instrument_id.symbol.to_string()),
            }),
            DataSubscription::Quotes { instrument_id } => Some(WsSubscription {
                subscription_type: WsSubscriptionType::AllMids,
                instrument: Some(instrument_id.symbol.to_string()),
            }),
            DataSubscription::Bars { .. } => None,
        }
    }
}

impl Component for HyperliquidDataAdapter {
    fn name(&self) -> &str {
        "hyperliquid-data"
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

impl DataClient for HyperliquidDataAdapter {
    fn venue(&self) -> Venue {
        Venue::Hyperliquid
    }

    fn subscribe(&mut self, subscription: &DataSubscription) -> Result<(), ComponentError> {
        if Self::to_ws_subscription(subscription).is_some() {
            self.subscriptions.push(subscription.clone());
            Ok(())
        } else {
            Err(ComponentError::Other(format!(
                "Unsupported subscription type: {:?}",
                subscription
            )))
        }
    }

    fn unsubscribe(&mut self, subscription: &DataSubscription) -> Result<(), ComponentError> {
        self.subscriptions.retain(|s| s != subscription);
        Ok(())
    }
}

// Helper adapter struct for compatibility
pub struct HyperliquidAdapter {
    pub config: HyperliquidConfig,
}

impl HyperliquidAdapter {
    pub fn new(config: HyperliquidConfig) -> Self {
        Self { config }
    }

    pub fn venue(&self) -> Venue {
        Venue::Hyperliquid
    }

    pub fn to_instrument_id(&self, coin: &str) -> InstrumentId {
        InstrumentId::new(Venue::Hyperliquid, coin, InstrumentType::Perp)
    }

    pub fn to_coin(&self, instrument_id: &InstrumentId) -> Option<String> {
        if instrument_id.venue == Venue::Hyperliquid {
            Some(instrument_id.symbol.to_string())
        } else {
            None
        }
    }
}
