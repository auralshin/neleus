use crate::{
    auth::{L2Authenticator, PolymarketSigner},
    ApiCredentials, OrderData, PolymarketBook, PolymarketConfig, PolymarketError,
    PolymarketMarket, PolymarketOrder, PolymarketPosition, PolymarketTrade,
};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// REST API client for Polymarket CLOB
pub struct PolymarketClient {
    config: PolymarketConfig,
    http_client: Client,
    signer: Option<PolymarketSigner>,
    l2_auth: Option<L2Authenticator>,
}

impl PolymarketClient {
    pub fn new(config: PolymarketConfig) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            config,
            http_client,
            signer: None,
            l2_auth: None,
        }
    }

    /// Initialize with L1 authentication (private key)
    pub fn with_signer(mut self, signer: PolymarketSigner) -> Self {
        self.signer = Some(signer);
        self
    }

    /// Initialize with L2 authentication (API credentials)
    pub fn with_l2_auth(mut self, auth: L2Authenticator) -> Self {
        self.l2_auth = Some(auth);
        self
    }

    /// Create or derive API key using L1 authentication
    pub async fn create_api_key(&mut self) -> Result<ApiCredentials, PolymarketError> {
        let signer = self
            .signer
            .as_mut()
            .ok_or(PolymarketError::MissingCredentials)?;

        let (signature, timestamp, nonce) = signer.sign_l1_auth().await?;
        let address = signer.get_address();

        let url = format!("{}/auth/api-key", self.config.clob_url);

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("POLY_ADDRESS", address.parse().unwrap());
        headers.insert("POLY_SIGNATURE", signature.parse().unwrap());
        headers.insert("POLY_TIMESTAMP", timestamp.to_string().parse().unwrap());
        headers.insert("POLY_NONCE", nonce.to_string().parse().unwrap());

        let response = self
            .http_client
            .post(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| PolymarketError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(PolymarketError::AuthenticationFailed(error_text));
        }

        let creds: ApiCredentials = response
            .json()
            .await
            .map_err(|e| PolymarketError::InvalidResponse(e.to_string()))?;

        // Store L2 credentials
        self.l2_auth = Some(L2Authenticator::from_credentials(&creds));

        Ok(creds)
    }

    /// Derive existing API key using L1 authentication
    pub async fn derive_api_key(&mut self) -> Result<ApiCredentials, PolymarketError> {
        let signer = self
            .signer
            .as_mut()
            .ok_or(PolymarketError::MissingCredentials)?;

        let (signature, timestamp, nonce) = signer.sign_l1_auth().await?;
        let address = signer.get_address();

        let url = format!("{}/auth/derive-api-key", self.config.clob_url);

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("POLY_ADDRESS", address.parse().unwrap());
        headers.insert("POLY_SIGNATURE", signature.parse().unwrap());
        headers.insert("POLY_TIMESTAMP", timestamp.to_string().parse().unwrap());
        headers.insert("POLY_NONCE", nonce.to_string().parse().unwrap());

        let response = self
            .http_client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| PolymarketError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(PolymarketError::AuthenticationFailed(error_text));
        }

        let creds: ApiCredentials = response
            .json()
            .await
            .map_err(|e| PolymarketError::InvalidResponse(e.to_string()))?;

        // Store L2 credentials
        self.l2_auth = Some(L2Authenticator::from_credentials(&creds));

        Ok(creds)
    }

    // Public endpoints (no auth required)

    /// Get price for a token
    pub async fn get_price(&self, token_id: &str) -> Result<PriceResponse, PolymarketError> {
        let url = format!("{}/price?token_id={}", self.config.clob_url, token_id);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| PolymarketError::RequestError(e.to_string()))?;

        self.handle_response(response).await
    }

    /// Get midpoint price for a token
    pub async fn get_midpoint(&self, token_id: &str) -> Result<MidpointResponse, PolymarketError> {
        let url = format!("{}/midpoint?token_id={}", self.config.clob_url, token_id);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| PolymarketError::RequestError(e.to_string()))?;

        self.handle_response(response).await
    }

    /// Get order book for a token
    pub async fn get_book(&self, token_id: &str) -> Result<PolymarketBook, PolymarketError> {
        let url = format!("{}/book?token_id={}", self.config.clob_url, token_id);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| PolymarketError::RequestError(e.to_string()))?;

        self.handle_response(response).await
    }

    /// Get trades for a market
    pub async fn get_trades(
        &self,
        market: &str,
        limit: Option<u32>,
    ) -> Result<Vec<PolymarketTrade>, PolymarketError> {
        let mut url = format!("{}/trades?market={}", self.config.clob_url, market);
        if let Some(limit) = limit {
            url.push_str(&format!("&limit={}", limit));
        }

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| PolymarketError::RequestError(e.to_string()))?;

        self.handle_response(response).await
    }

    /// Get markets from Gamma API
    pub async fn get_markets(
        &self,
        limit: Option<u32>,
    ) -> Result<Vec<PolymarketMarket>, PolymarketError> {
        let mut url = format!("{}/markets", self.config.gamma_url);
        if let Some(limit) = limit {
            url.push_str(&format!("?limit={}", limit));
        }

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| PolymarketError::RequestError(e.to_string()))?;

        self.handle_response(response).await
    }

    // L2 authenticated endpoints

    /// Get user's open orders
    pub async fn get_orders(&self) -> Result<Vec<PolymarketOrder>, PolymarketError> {
        let url = format!("{}/orders", self.config.clob_url);
        self.authenticated_get(&url, &HashMap::new()).await
    }

    /// Get user's positions
    pub async fn get_positions(&self) -> Result<Vec<PolymarketPosition>, PolymarketError> {
        let url = format!("{}/positions", self.config.clob_url);
        self.authenticated_get(&url, &HashMap::new()).await
    }

    /// Place an order
    pub async fn place_order(
        &self,
        order_request: OrderRequest,
    ) -> Result<OrderResponse, PolymarketError> {
        let signer = self
            .signer
            .as_ref()
            .ok_or(PolymarketError::MissingCredentials)?;

        // Sign the order
        let order_data = self.build_order_data(&order_request)?;
        let signature = signer.sign_order(&order_data).await?;

        let url = format!("{}/order", self.config.clob_url);
        let body = json!({
            "order": order_data,
            "signature": signature,
            "owner": signer.get_address(),
        });

        self.authenticated_post(&url, &body).await
    }

    /// Cancel an order
    pub async fn cancel_order(&self, order_id: &str) -> Result<CancelResponse, PolymarketError> {
        let url = format!("{}/order", self.config.clob_url);
        let body = json!({
            "orderID": order_id,
        });

        self.authenticated_delete(&url, &body).await
    }

    /// Cancel all orders
    pub async fn cancel_all_orders(&self) -> Result<CancelAllResponse, PolymarketError> {
        let url = format!("{}/orders", self.config.clob_url);
        self.authenticated_delete(&url, &json!({})).await
    }

    // Helper methods

    fn build_order_data(&self, request: &OrderRequest) -> Result<OrderData, PolymarketError> {
        let signer = self
            .signer
            .as_ref()
            .ok_or(PolymarketError::MissingCredentials)?;

        let binding = signer.get_address();
        let funder = self.config.funder_address.as_ref().unwrap_or(&binding);

        Ok(OrderData {
            maker: funder.clone(),
            taker: "0x0000000000000000000000000000000000000000".to_string(),
            token_id: request.token_id.clone(),
            maker_amount: request.maker_amount.clone(),
            taker_amount: request.taker_amount.clone(),
            side: request.side.clone(),
            fee_rate_bps: request.fee_rate_bps.clone(),
            nonce: request.nonce.clone(),
            signer: signer.get_address(),
            expiration: request.expiration.clone(),
        })
    }

    async fn authenticated_get<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        params: &HashMap<String, String>,
    ) -> Result<T, PolymarketError> {
        let auth = self
            .l2_auth
            .as_ref()
            .ok_or(PolymarketError::MissingCredentials)?;

        let timestamp = Self::current_timestamp();
        let query_string = Self::build_query_string(params);
        let path = format!(
            "{}?{}",
            url.trim_start_matches(&self.config.clob_url),
            query_string
        );

        let signature = auth.sign_request(timestamp, "GET", &path, "")?;
        let signer_address = self
            .config
            .signer_address
            .as_ref()
            .ok_or(PolymarketError::MissingCredentials)?;

        let headers = auth.get_headers(timestamp, &signature, signer_address);

        let response = self
            .http_client
            .get(url)
            .query(params)
            .headers(Self::convert_headers(&headers))
            .send()
            .await
            .map_err(|e| PolymarketError::RequestError(e.to_string()))?;

        self.handle_response(response).await
    }

    async fn authenticated_post<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<T, PolymarketError> {
        let auth = self
            .l2_auth
            .as_ref()
            .ok_or(PolymarketError::MissingCredentials)?;

        let timestamp = Self::current_timestamp();
        let path = url.trim_start_matches(&self.config.clob_url);
        let body_str = serde_json::to_string(body).unwrap_or_default();

        let signature = auth.sign_request(timestamp, "POST", path, &body_str)?;
        let signer_address = self
            .config
            .signer_address
            .as_ref()
            .ok_or(PolymarketError::MissingCredentials)?;

        let headers = auth.get_headers(timestamp, &signature, signer_address);

        let response = self
            .http_client
            .post(url)
            .headers(Self::convert_headers(&headers))
            .json(body)
            .send()
            .await
            .map_err(|e| PolymarketError::RequestError(e.to_string()))?;

        self.handle_response(response).await
    }

    async fn authenticated_delete<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<T, PolymarketError> {
        let auth = self
            .l2_auth
            .as_ref()
            .ok_or(PolymarketError::MissingCredentials)?;

        let timestamp = Self::current_timestamp();
        let path = url.trim_start_matches(&self.config.clob_url);
        let body_str = serde_json::to_string(body).unwrap_or_default();

        let signature = auth.sign_request(timestamp, "DELETE", path, &body_str)?;
        let signer_address = self
            .config
            .signer_address
            .as_ref()
            .ok_or(PolymarketError::MissingCredentials)?;

        let headers = auth.get_headers(timestamp, &signature, signer_address);

        let response = self
            .http_client
            .delete(url)
            .headers(Self::convert_headers(&headers))
            .json(body)
            .send()
            .await
            .map_err(|e| PolymarketError::RequestError(e.to_string()))?;

        self.handle_response(response).await
    }

    async fn handle_response<T: for<'de> Deserialize<'de>>(
        &self,
        response: Response,
    ) -> Result<T, PolymarketError> {
        let status = response.status();

        if status.is_success() {
            response
                .json::<T>()
                .await
                .map_err(|e| PolymarketError::InvalidResponse(e.to_string()))
        } else if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1000);

            Err(PolymarketError::RateLimitExceeded {
                retry_after_ms: retry_after,
            })
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(PolymarketError::RequestError(format!(
                "Status {}: {}",
                status, error_text
            )))
        }
    }

    fn convert_headers(headers: &HashMap<String, String>) -> reqwest::header::HeaderMap {
        let mut header_map = reqwest::header::HeaderMap::new();
        for (key, value) in headers {
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                reqwest::header::HeaderValue::from_str(value),
            ) {
                header_map.insert(name, val);
            }
        }
        header_map
    }

    fn build_query_string(params: &HashMap<String, String>) -> String {
        params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&")
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

// Request/Response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub token_id: String,
    pub maker_amount: String,
    pub taker_amount: String,
    pub side: String,
    pub fee_rate_bps: String,
    pub nonce: String,
    pub expiration: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    pub order_id: String,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelResponse {
    pub success: bool,
    pub order_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelAllResponse {
    pub success: bool,
    pub cancelled: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceResponse {
    pub price: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidpointResponse {
    pub mid: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let config = PolymarketConfig::mainnet();
        let client = PolymarketClient::new(config);
        assert!(client.signer.is_none());
        assert!(client.l2_auth.is_none());
    }
}
