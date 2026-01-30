use crate::auth::LighterSigner;
use crate::client::{
    LighterExecution, LighterRestPublic, LighterWsMarketData, LighterWsUserStream,
};
use crate::config::{LighterConfig, LighterError};
use crate::types::LighterOrderRequest;
use neleus_core_engine::{
    Component, ComponentError, DataClient, DataSubscription, ExecutionClient, OrderType,
    StrategyCommand,
};
use neleus_core_types::{InstrumentId, InstrumentType, OrderId, Venue};

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

#[allow(dead_code)]
pub struct LighterExecutionAdapter {
    config: LighterConfig,
    execution: LighterExecution,
    http_client: reqwest::Client,
    is_running: bool,
}

impl LighterExecutionAdapter {
    pub fn new(config: LighterConfig) -> Self {
        Self {
            execution: LighterExecution::new(config.clone()),
            http_client: reqwest::Client::new(),
            config,
            is_running: false,
        }
    }

    pub fn with_credentials(
        config: LighterConfig,
        api_key: &str,
        api_secret: &str,
    ) -> Result<Self, LighterError> {
        let signer = LighterSigner::new(api_key.to_string(), api_secret.to_string())?;
        let mut execution = LighterExecution::new(config.clone());
        execution.signer = Some(signer);
        Ok(Self {
            execution,
            http_client: reqwest::Client::new(),
            config,
            is_running: false,
        })
    }

    pub fn execution(&self) -> &LighterExecution {
        &self.execution
    }

    pub fn execution_mut(&mut self) -> &mut LighterExecution {
        &mut self.execution
    }

    pub async fn submit_order_async(
        &mut self,
        request: &LighterOrderRequest,
    ) -> Result<String, LighterError> {
        if !self.execution.can_place_order() {
            return Err(LighterError::RateLimitExceeded {
                retry_after_ms: 1000,
            });
        }

        let headers = self.execution.build_order_headers(request)?;
        let url = self.execution.orders_url();

        let mut req_builder = self.http_client.post(&url);
        for (k, v) in headers {
            req_builder = req_builder.header(&k, &v);
        }
        req_builder = req_builder.json(request);

        let response = req_builder
            .send()
            .await
            .map_err(|e| LighterError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(LighterError::RequestError(format!(
                "HTTP {}: {}",
                status, text
            )));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| LighterError::InvalidResponse(e.to_string()))?;

        let order_id = result["order_id"].as_str().unwrap_or("unknown").to_string();

        if let Some(cloid) = &request.client_order_id {
            self.execution
                .on_order_submitted(cloid.clone(), request.clone());
        }

        Ok(order_id)
    }

    pub async fn cancel_order_async(&mut self, order_id: &str) -> Result<(), LighterError> {
        let headers = self.execution.build_cancel_headers(order_id)?;
        let url = self.execution.cancel_url(order_id);

        let mut req_builder = self.http_client.delete(&url);
        for (k, v) in headers {
            req_builder = req_builder.header(&k, &v);
        }

        let response = req_builder
            .send()
            .await
            .map_err(|e| LighterError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(LighterError::RequestError(format!(
                "HTTP {}: {}",
                status, text
            )));
        }

        self.execution.on_cancel_submitted();
        Ok(())
    }
}

impl Component for LighterExecutionAdapter {
    fn name(&self) -> &str {
        "lighter-execution"
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

impl ExecutionClient for LighterExecutionAdapter {
    fn venue(&self) -> Venue {
        Venue::Lighter
    }

    fn submit_order(&mut self, order: &StrategyCommand) -> Result<(), ComponentError> {
        if let StrategyCommand::SubmitOrder {
            order_id,
            instrument_id: _instrument_id,
            side,
            order_type,
            price,
            quantity,
        } = order
        {
            // Build Lighter order request
            // Note: market_id needs to be resolved from symbol - this is a simplified version
            let market_id = 0u32; // Would need market_id lookup from symbol

            let lighter_side = match side {
                neleus_core_engine::OrderSide::Buy => neleus_core_types::OrderSide::Buy,
                neleus_core_engine::OrderSide::Sell => neleus_core_types::OrderSide::Sell,
            };

            let request = LighterOrderRequest {
                market_id,
                side: lighter_side,
                order_type: match order_type {
                    OrderType::Market => crate::types::LighterOrderType::Market,
                    OrderType::Limit => crate::types::LighterOrderType::Limit,
                },
                price: price.unwrap_or(0.0),
                quantity: *quantity,
                client_order_id: Some(order_id.to_string()),
                reduce_only: false,
            };

            let rt = tokio::runtime::Handle::try_current()
                .map_err(|e| ComponentError::Other(format!("No tokio runtime: {}", e)))?;

            rt.block_on(async { self.submit_order_async(&request).await })
                .map_err(|e| ComponentError::Other(format!("Order submission failed: {}", e)))?;

            Ok(())
        } else {
            Err(ComponentError::Other("Invalid command type".to_string()))
        }
    }

    fn cancel_order(&mut self, order_id: &OrderId) -> Result<(), ComponentError> {
        let order_id_str = order_id.to_string();

        let rt = tokio::runtime::Handle::try_current()
            .map_err(|e| ComponentError::Other(format!("No tokio runtime: {}", e)))?;

        rt.block_on(async { self.cancel_order_async(&order_id_str).await })
            .map_err(|e| ComponentError::Other(format!("Order cancel failed: {}", e)))?;

        Ok(())
    }
}

/// Wrapper that implements DataClient for Lighter
#[allow(dead_code)]
pub struct LighterDataAdapter {
    config: LighterConfig,
    subscriptions: Vec<DataSubscription>,
    is_running: bool,
}

impl LighterDataAdapter {
    pub fn new(config: LighterConfig) -> Self {
        Self {
            config,
            subscriptions: Vec::new(),
            is_running: false,
        }
    }

    /// Get current subscriptions
    pub fn subscriptions(&self) -> &[DataSubscription] {
        &self.subscriptions
    }
}

impl Component for LighterDataAdapter {
    fn name(&self) -> &str {
        "lighter-data"
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

impl DataClient for LighterDataAdapter {
    fn venue(&self) -> Venue {
        Venue::Lighter
    }

    fn subscribe(&mut self, subscription: &DataSubscription) -> Result<(), ComponentError> {
        match subscription {
            DataSubscription::Trades { .. } | DataSubscription::OrderBook { .. } => {
                self.subscriptions.push(subscription.clone());
                Ok(())
            }
            DataSubscription::Quotes { .. } | DataSubscription::Bars { .. } => Err(
                ComponentError::Other(format!("Unsupported subscription type: {:?}", subscription)),
            ),
        }
    }

    fn unsubscribe(&mut self, subscription: &DataSubscription) -> Result<(), ComponentError> {
        self.subscriptions.retain(|s| s != subscription);
        Ok(())
    }
}
