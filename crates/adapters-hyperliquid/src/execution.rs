use crate::config::{HyperliquidConfig, HyperliquidError, NonceManager};
use crate::historical::HyperliquidHistoricalClient;
use crate::signing::{order_to_wire, HyperliquidSigner, SignableAction};
use crate::types::*;
use neleus_core_types::OrderSide;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

pub struct HyperliquidExecutionClient {
    config: HyperliquidConfig,
    http_client: HttpClient,
    pub(crate) signer: Option<HyperliquidSigner>,
    pub(crate) execution: HyperliquidExecution,
    asset_indices: HashMap<String, u32>,
}

pub struct HyperliquidExecution {
    config: HyperliquidConfig,
    nonce_manager: NonceManager,
    pending_orders: HashMap<String, HyperliquidOrderRequest>,
    pub open_order_count: u32,
    pub max_open_orders: u32,
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

impl HyperliquidExecutionClient {
    pub fn new(config: HyperliquidConfig) -> Self {
        Self {
            execution: HyperliquidExecution::new(config.clone()),
            http_client: HttpClient::new(),
            signer: None,
            asset_indices: HashMap::new(),
            config,
        }
    }

    pub fn with_signer(mut self, private_key: &str) -> Result<Self, HyperliquidError> {
        let signer = HyperliquidSigner::new(private_key, self.config.testnet)?;
        self.signer = Some(signer);
        Ok(self)
    }

    pub async fn load_asset_metadata(&mut self) -> Result<(), HyperliquidError> {
        let client = HyperliquidHistoricalClient::new(self.config.clone());
        let meta = client.fetch_meta().await?;

        self.asset_indices.clear();
        for (index, asset) in meta.universe.iter().enumerate() {
            self.asset_indices.insert(asset.name.clone(), index as u32);
        }

        info!("Loaded {} asset indices", self.asset_indices.len());
        Ok(())
    }

    pub fn get_asset_index(&self, coin: &str) -> Option<u32> {
        self.asset_indices.get(coin).copied()
    }

    pub async fn submit_market_order(
        &mut self,
        coin: &str,
        is_buy: bool,
        size: f64,
        slippage_bps: u32,
    ) -> Result<PlaceOrderResponse, HyperliquidError> {
        let mid_price = self.fetch_mid_price(coin).await?;
        let slippage = 1.0 + (slippage_bps as f64 / 10000.0);
        let limit_price = if is_buy {
            mid_price * slippage
        } else {
            mid_price / slippage
        };

        let request = HyperliquidOrderRequest {
            coin: coin.to_string(),
            is_buy,
            limit_px: limit_price,
            sz: size,
            reduce_only: false,
            order_type: HyperliquidOrderTypeRequest::Limit {
                tif: HyperliquidTif::Ioc,
            },
            cloid: Some(format!("{:016x}", self.execution.next_nonce())),
        };

        self.submit_order(&request).await
    }

    pub async fn submit_limit_order(
        &mut self,
        coin: &str,
        is_buy: bool,
        size: f64,
        price: f64,
        post_only: bool,
        reduce_only: bool,
    ) -> Result<PlaceOrderResponse, HyperliquidError> {
        let tif = if post_only {
            HyperliquidTif::Alo
        } else {
            HyperliquidTif::Gtc
        };

        let request = HyperliquidOrderRequest {
            coin: coin.to_string(),
            is_buy,
            limit_px: price,
            sz: size,
            reduce_only,
            order_type: HyperliquidOrderTypeRequest::Limit { tif },
            cloid: Some(format!("{:016x}", self.execution.next_nonce())),
        };

        self.submit_order(&request).await
    }

    pub async fn submit_order(
        &mut self,
        request: &HyperliquidOrderRequest,
    ) -> Result<PlaceOrderResponse, HyperliquidError> {
        let signer = self
            .signer
            .as_ref()
            .ok_or(HyperliquidError::MissingCredentials)?;

        if !self.execution.can_place_order() {
            return Err(HyperliquidError::OrderLimitExceeded {
                current: self.execution.open_order_count,
                max: self.execution.max_open_orders,
            });
        }

        let asset = self.get_asset_index(&request.coin).ok_or_else(|| {
            HyperliquidError::InvalidResponse(format!(
                "Unknown asset: {}. Call load_asset_metadata() first.",
                request.coin
            ))
        })?;

        let order_wire = order_to_wire(request, asset);
        let action = SignableAction::Order {
            orders: vec![order_wire],
            grouping: "na".to_string(),
        };

        let nonce = self.execution.next_nonce();
        let signed = signer.sign_request(action, nonce, None)?;

        let url = format!("{}/exchange", self.config.rest_url);
        let response = self
            .http_client
            .post(&url)
            .json(&signed)
            .send()
            .await
            .map_err(|e| HyperliquidError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 429 {
                return Err(HyperliquidError::RateLimitExceeded);
            }
            let text = response.text().await.unwrap_or_default();
            return Err(HyperliquidError::RequestError(format!(
                "HTTP {}: {}",
                status, text
            )));
        }

        let result: PlaceOrderResponse = response
            .json()
            .await
            .map_err(|e| HyperliquidError::InvalidResponse(e.to_string()))?;

        if result.status == "ok" {
            if let Some(cloid) = &request.cloid {
                self.execution
                    .on_order_submitted(cloid.clone(), request.clone());
            }
        }

        info!(
            "Order submitted: {} {} {} @ {}",
            if request.is_buy { "BUY" } else { "SELL" },
            request.sz,
            request.coin,
            request.limit_px
        );

        Ok(result)
    }

    pub async fn cancel_order(
        &mut self,
        coin: &str,
        order_id: u64,
    ) -> Result<CancelOrderResponse, HyperliquidError> {
        let signer = self
            .signer
            .as_ref()
            .ok_or(HyperliquidError::MissingCredentials)?;

        let action = SignableAction::Cancel {
            cancels: vec![CancelRequest {
                coin: coin.to_string(),
                order_id,
            }],
        };

        let nonce = self.execution.next_nonce();
        let signed = signer.sign_request(action, nonce, None)?;

        let url = format!("{}/exchange", self.config.rest_url);
        let response = self
            .http_client
            .post(&url)
            .json(&signed)
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

        let result: CancelOrderResponse = response
            .json()
            .await
            .map_err(|e| HyperliquidError::InvalidResponse(e.to_string()))?;

        info!("Order canceled: {} oid={}", coin, order_id);

        Ok(result)
    }

    pub async fn cancel_order_by_cloid(
        &mut self,
        coin: &str,
        cloid: &str,
    ) -> Result<CancelOrderResponse, HyperliquidError> {
        use crate::signing::CloidCancel;

        let signer = self
            .signer
            .as_ref()
            .ok_or(HyperliquidError::MissingCredentials)?;

        let asset = self
            .get_asset_index(coin)
            .ok_or_else(|| HyperliquidError::InvalidResponse(format!("Unknown asset: {}", coin)))?;

        let action = SignableAction::CancelByCloid {
            cancels: vec![CloidCancel {
                asset,
                cloid: cloid.to_string(),
            }],
        };

        let nonce = self.execution.next_nonce();
        let signed = signer.sign_request(action, nonce, None)?;

        let url = format!("{}/exchange", self.config.rest_url);
        let response = self
            .http_client
            .post(&url)
            .json(&signed)
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

        let result: CancelOrderResponse = response
            .json()
            .await
            .map_err(|e| HyperliquidError::InvalidResponse(e.to_string()))?;

        self.execution.on_order_closed(cloid);

        info!("Order canceled by cloid: {} cloid={}", coin, cloid);

        Ok(result)
    }

    async fn fetch_mid_price(&self, coin: &str) -> Result<f64, HyperliquidError> {
        #[derive(Serialize)]
        struct AllMidsRequest {
            #[serde(rename = "type")]
            req_type: String,
        }

        let url = format!("{}/info", self.config.rest_url);
        let request = AllMidsRequest {
            req_type: "allMids".to_string(),
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

        let mids: HashMap<String, String> = response
            .json()
            .await
            .map_err(|e| HyperliquidError::InvalidResponse(e.to_string()))?;

        mids.get(coin)
            .ok_or_else(|| HyperliquidError::InvalidResponse(format!("No mid price for {}", coin)))
            .and_then(|s| {
                s.parse::<f64>()
                    .map_err(|e| HyperliquidError::InvalidResponse(e.to_string()))
            })
    }

    pub async fn fetch_open_orders(&self) -> Result<Vec<HyperliquidUserOrder>, HyperliquidError> {
        let address = self
            .signer
            .as_ref()
            .map(|s| s.address().to_string())
            .ok_or(HyperliquidError::MissingCredentials)?;

        #[derive(Serialize)]
        struct OpenOrdersRequest {
            #[serde(rename = "type")]
            req_type: String,
            user: String,
        }

        let url = format!("{}/info", self.config.rest_url);
        let request = OpenOrdersRequest {
            req_type: "openOrders".to_string(),
            user: address,
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

        #[derive(Deserialize)]
        struct OpenOrderResponse {
            coin: String,
            #[serde(rename = "limitPx")]
            limit_px: String,
            oid: u64,
            side: String,
            sz: String,
            timestamp: u64,
        }

        let orders: Vec<OpenOrderResponse> = response
            .json()
            .await
            .map_err(|e| HyperliquidError::InvalidResponse(e.to_string()))?;

        Ok(orders
            .into_iter()
            .map(|o| HyperliquidUserOrder {
                coin: o.coin,
                order_id: o.oid,
                client_order_id: None,
                side: if o.side == "B" {
                    OrderSide::Buy
                } else {
                    OrderSide::Sell
                },
                order_type: HyperliquidOrderType::Limit,
                price: o.limit_px.parse().ok(),
                size: o.sz.parse().unwrap_or(0.0),
                filled_size: 0.0,
                status: HyperliquidOrderStatus::Open,
                timestamp: o.timestamp,
            })
            .collect())
    }

    pub async fn fetch_fills(
        &self,
        limit: usize,
    ) -> Result<Vec<HyperliquidUserFill>, HyperliquidError> {
        let address = self
            .signer
            .as_ref()
            .map(|s| s.address().to_string())
            .ok_or(HyperliquidError::MissingCredentials)?;

        #[derive(Serialize)]
        struct FillsRequest {
            #[serde(rename = "type")]
            req_type: String,
            user: String,
        }

        let url = format!("{}/info", self.config.rest_url);
        let request = FillsRequest {
            req_type: "userFills".to_string(),
            user: address,
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

        let fills: Vec<FillData> = response
            .json()
            .await
            .map_err(|e| HyperliquidError::InvalidResponse(e.to_string()))?;

        Ok(fills
            .into_iter()
            .take(limit)
            .map(|f| HyperliquidUserFill {
                coin: f.coin,
                order_id: f.oid,
                side: if f.side == "B" {
                    OrderSide::Buy
                } else {
                    OrderSide::Sell
                },
                price: f.px.parse().unwrap_or(0.0),
                size: f.sz.parse().unwrap_or(0.0),
                fee: f.fee.parse().unwrap_or(0.0),
                timestamp: f.time,
                crossed: false,
            })
            .collect())
    }

    pub fn address(&self) -> Option<&str> {
        self.signer.as_ref().map(|s| s.address())
    }
}
