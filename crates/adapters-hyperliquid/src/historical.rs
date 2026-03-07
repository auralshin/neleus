use crate::config::{HyperliquidConfig, HyperliquidError};
use crate::types::TradeData;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperliquidMeta {
    #[serde(rename = "collateralToken", default)]
    pub collateral_token: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperliquidSpotMeta {
    pub tokens: Vec<HyperliquidSpotTokenInfo>,
    pub universe: Vec<HyperliquidSpotMarketInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperliquidSpotTokenInfo {
    pub name: String,
    #[serde(rename = "szDecimals")]
    pub sz_decimals: u32,
    #[serde(rename = "weiDecimals", default)]
    pub wei_decimals: Option<u32>,
    pub index: u32,
    #[serde(rename = "tokenId", default)]
    pub token_id: Option<String>,
    #[serde(rename = "isCanonical", default)]
    pub is_canonical: Option<bool>,
    #[serde(rename = "fullName", default)]
    pub full_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperliquidSpotMarketInfo {
    pub name: String,
    pub index: u32,
    pub tokens: Vec<u32>,
    #[serde(rename = "isCanonical", default)]
    pub is_canonical: Option<bool>,
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
        _coin: &str,
    ) -> Result<Vec<TradeData>, HyperliquidError> {
        Err(HyperliquidError::RequestError(
            "Historical trades not available via REST. Use candle data or collect trades via WebSocket.".to_string()
        ))
    }

    pub async fn fetch_meta(&self) -> Result<HyperliquidMeta, HyperliquidError> {
        self.fetch_meta_with_dex(None).await
    }

    pub async fn fetch_meta_with_dex(
        &self,
        dex: Option<&str>,
    ) -> Result<HyperliquidMeta, HyperliquidError> {
        let url = format!("{}/info", self.config.rest_url);

        #[derive(Serialize)]
        struct MetaRequest {
            #[serde(rename = "type")]
            req_type: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            dex: Option<String>,
        }

        let request = MetaRequest {
            req_type: "meta".to_string(),
            dex: dex.map(str::to_string),
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

    pub async fn fetch_all_perp_metas(&self) -> Result<Vec<HyperliquidMeta>, HyperliquidError> {
        let url = format!("{}/info", self.config.rest_url);

        #[derive(Serialize)]
        struct AllPerpMetasRequest {
            #[serde(rename = "type")]
            req_type: String,
        }

        let request = AllPerpMetasRequest {
            req_type: "allPerpMetas".to_string(),
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

        let metas: Vec<HyperliquidMeta> = response
            .json()
            .await
            .map_err(|e| HyperliquidError::InvalidResponse(e.to_string()))?;

        Ok(metas)
    }

    pub async fn fetch_spot_meta(&self) -> Result<HyperliquidSpotMeta, HyperliquidError> {
        let url = format!("{}/info", self.config.rest_url);

        #[derive(Serialize)]
        struct SpotMetaRequest {
            #[serde(rename = "type")]
            req_type: String,
        }

        let request = SpotMetaRequest {
            req_type: "spotMeta".to_string(),
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

        let meta: HyperliquidSpotMeta = response
            .json()
            .await
            .map_err(|e| HyperliquidError::InvalidResponse(e.to_string()))?;

        Ok(meta)
    }
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

pub struct HyperliquidDataFeed {
    data: Vec<HyperliquidDataPoint>,
    index: usize,
    coin: String,
    interval: CandleInterval,
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
