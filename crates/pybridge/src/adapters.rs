use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;
use super::types::*;

use neleus_adapters_hyperliquid::{
    CandleInterval, HyperliquidCandle, HyperliquidConfig, HyperliquidHistoricalClient,
    HyperliquidMeta, HyperliquidSpotMarketInfo, HyperliquidSpotMeta, HyperliquidSpotTokenInfo,
    HyperliquidWsMarketData, HyperliquidWsMessage, L2BookData, PriceLevel,
};

#[pyclass(name = "HyperliquidClient")]
pub struct PyHyperliquidClient {
    config: HyperliquidConfig,
    runtime: tokio::runtime::Runtime,
}

#[pymethods]
impl PyHyperliquidClient {
    #[new]
    #[pyo3(signature = (testnet=false))]
    pub fn new(testnet: bool) -> PyResult<Self> {
        let config = if testnet {
            HyperliquidConfig::testnet()
        } else {
            HyperliquidConfig::mainnet()
        };

        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;

        Ok(Self { config, runtime })
    }

    #[pyo3(signature = (coin, interval="1h", start_time_ms=None, end_time_ms=None))]
    pub fn fetch_candles(
        &self,
        coin: &str,
        interval: &str,
        start_time_ms: Option<u64>,
        end_time_ms: Option<u64>,
    ) -> PyResult<Vec<PyHyperliquidCandle>> {
        let interval = match interval {
            "1m" => CandleInterval::Min1,
            "5m" => CandleInterval::Min5,
            "15m" => CandleInterval::Min15,
            "1h" => CandleInterval::Hour1,
            "4h" => CandleInterval::Hour4,
            "1d" => CandleInterval::Day1,
            _ => {
                return Err(PyValueError::new_err(format!(
                    "Invalid interval: {}",
                    interval
                )))
            }
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let end = end_time_ms.unwrap_or(now);
        let start = start_time_ms.unwrap_or(end - 30 * 24 * 60 * 60 * 1000);

        let client = HyperliquidHistoricalClient::new(self.config.clone());

        let candles = self
            .runtime
            .block_on(async {
                client
                    .fetch_candles_range(coin, interval, start, end, 5000)
                    .await
            })
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to fetch candles: {}", e)))?;

        Ok(candles.into_iter().map(PyHyperliquidCandle::from).collect())
    }

    #[pyo3(signature = (dex=None))]
    pub fn fetch_meta(&self, dex: Option<String>) -> PyResult<PyHyperliquidMeta> {
        let client = HyperliquidHistoricalClient::new(self.config.clone());

        let meta = self
            .runtime
            .block_on(async { client.fetch_meta_with_dex(dex.as_deref()).await })
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to fetch meta: {}", e)))?;

        Ok(PyHyperliquidMeta::from(meta))
    }

    pub fn fetch_all_perp_metas(&self) -> PyResult<Vec<PyHyperliquidMeta>> {
        let client = HyperliquidHistoricalClient::new(self.config.clone());

        let metas = self
            .runtime
            .block_on(async { client.fetch_all_perp_metas().await })
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to fetch all perp metas: {}", e)))?;

        Ok(metas.into_iter().map(PyHyperliquidMeta::from).collect())
    }

    pub fn fetch_spot_meta(&self) -> PyResult<PyHyperliquidSpotMeta> {
        let client = HyperliquidHistoricalClient::new(self.config.clone());

        let meta = self
            .runtime
            .block_on(async { client.fetch_spot_meta().await })
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to fetch spot meta: {}", e)))?;

        Ok(PyHyperliquidSpotMeta::from(meta))
    }

    pub fn stream_l2_book(&self, coin: &str) -> PyResult<PyHyperliquidL2BookStream> {
        let config = self.config.clone();
        let stream_coin = coin.to_string();
        let (tx, rx) = mpsc::channel::<L2BookStreamEvent>();
        let thread_coin = stream_coin.clone();
        let spawn_result = thread::Builder::new()
            .name(format!("neleus-hl-l2-{}", thread_coin))
            .spawn(move || {
                let runtime = match tokio::runtime::Runtime::new() {
                    Ok(runtime) => runtime,
                    Err(err) => {
                        let _ = tx.send(L2BookStreamEvent::Error(format!(
                            "Failed to create runtime for {}: {}",
                            thread_coin, err
                        )));
                        return;
                    }
                };

                let mut market_data = HyperliquidWsMarketData::new(config);
                market_data.subscribe_l2_book(&thread_coin);

                let callback_tx = tx.clone();
                let callback_coin = thread_coin.clone();
                let result = runtime.block_on(async move {
                    market_data
                        .connect(move |message| {
                            if let HyperliquidWsMessage::L2Book { data } = message {
                                let _ = callback_tx
                                    .send(L2BookStreamEvent::Update(PyHyperliquidL2BookUpdate::from(
                                        data,
                                    )));
                            }
                        })
                        .await
                });

                if let Err(err) = result {
                    let _ = tx.send(L2BookStreamEvent::Error(format!(
                        "L2 book stream failed for {}: {}",
                        callback_coin, err
                    )));
                }
            });

        spawn_result.map_err(|err| {
            PyRuntimeError::new_err(format!(
                "Failed to spawn L2 book stream thread for {}: {}",
                stream_coin, err
            ))
        })?;

        Ok(PyHyperliquidL2BookStream {
            coin: stream_coin,
            is_testnet: self.config.testnet,
            receiver: Mutex::new(rx),
        })
    }

    pub fn rest_url(&self) -> String {
        self.config.rest_url.clone()
    }

    pub fn ws_url(&self) -> String {
        self.config.ws_url.clone()
    }

    pub fn is_testnet(&self) -> bool {
        self.config.testnet
    }
}

#[pyclass(name = "HyperliquidCandle")]
#[derive(Debug, Clone)]
pub struct PyHyperliquidCandle {
    #[pyo3(get)]
    pub timestamp: u64,
    #[pyo3(get)]
    pub open: f64,
    #[pyo3(get)]
    pub high: f64,
    #[pyo3(get)]
    pub low: f64,
    #[pyo3(get)]
    pub close: f64,
    #[pyo3(get)]
    pub volume: f64,
    #[pyo3(get)]
    pub num_trades: u64,
}

impl From<HyperliquidCandle> for PyHyperliquidCandle {
    fn from(c: HyperliquidCandle) -> Self {
        Self {
            timestamp: c.timestamp,
            open: c.open_f64(),
            high: c.high_f64(),
            low: c.low_f64(),
            close: c.close_f64(),
            volume: c.volume_f64(),
            num_trades: c.num_trades,
        }
    }
}

#[pymethods]
impl PyHyperliquidCandle {
    pub fn __repr__(&self) -> String {
        format!(
            "HyperliquidCandle(t={}, o={:.2}, h={:.2}, l={:.2}, c={:.2}, v={:.2})",
            self.timestamp, self.open, self.high, self.low, self.close, self.volume
        )
    }

    pub fn to_bar(&self, instrument_id: PyInstrumentId) -> PyBar {
        PyBar {
            instrument_id,
            open: self.open,
            high: self.high,
            low: self.low,
            close: self.close,
            volume: self.volume,
            timestamp_ns: self.timestamp * 1_000_000,
        }
    }
}

#[pyclass(name = "HyperliquidMeta")]
#[derive(Debug, Clone)]
pub struct PyHyperliquidMeta {
    #[pyo3(get)]
    pub symbols: Vec<PyHyperliquidAsset>,
    #[pyo3(get)]
    pub collateral_token: Option<String>,
}

impl From<HyperliquidMeta> for PyHyperliquidMeta {
    fn from(m: HyperliquidMeta) -> Self {
        Self {
            collateral_token: m.collateral_token,
            symbols: m
                .universe
                .into_iter()
                .map(PyHyperliquidAsset::from)
                .collect(),
        }
    }
}

#[pymethods]
impl PyHyperliquidMeta {
    pub fn symbol_names(&self) -> Vec<String> {
        self.symbols.iter().map(|s| s.name.clone()).collect()
    }

    pub fn get_asset(&self, name: &str) -> Option<PyHyperliquidAsset> {
        self.symbols.iter().find(|s| s.name == name).cloned()
    }
}

#[pyclass(name = "HyperliquidSpotMeta")]
#[derive(Debug, Clone)]
pub struct PyHyperliquidSpotMeta {
    #[pyo3(get)]
    pub tokens: Vec<PyHyperliquidSpotToken>,
    #[pyo3(get)]
    pub markets: Vec<PyHyperliquidSpotMarket>,
}

impl From<HyperliquidSpotMeta> for PyHyperliquidSpotMeta {
    fn from(m: HyperliquidSpotMeta) -> Self {
        Self {
            tokens: m.tokens.into_iter().map(PyHyperliquidSpotToken::from).collect(),
            markets: m
                .universe
                .into_iter()
                .map(PyHyperliquidSpotMarket::from)
                .collect(),
        }
    }
}

#[pymethods]
impl PyHyperliquidSpotMeta {
    pub fn market_names(&self) -> Vec<String> {
        self.markets.iter().map(|m| m.name.clone()).collect()
    }

    pub fn token_names(&self) -> Vec<String> {
        self.tokens.iter().map(|t| t.name.clone()).collect()
    }
}

#[pyclass(name = "HyperliquidSpotToken")]
#[derive(Debug, Clone)]
pub struct PyHyperliquidSpotToken {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub sz_decimals: u32,
    #[pyo3(get)]
    pub wei_decimals: Option<u32>,
    #[pyo3(get)]
    pub index: u32,
    #[pyo3(get)]
    pub token_id: Option<String>,
    #[pyo3(get)]
    pub is_canonical: Option<bool>,
    #[pyo3(get)]
    pub full_name: Option<String>,
}

impl From<HyperliquidSpotTokenInfo> for PyHyperliquidSpotToken {
    fn from(t: HyperliquidSpotTokenInfo) -> Self {
        Self {
            name: t.name,
            sz_decimals: t.sz_decimals,
            wei_decimals: t.wei_decimals,
            index: t.index,
            token_id: t.token_id,
            is_canonical: t.is_canonical,
            full_name: t.full_name,
        }
    }
}

#[pyclass(name = "HyperliquidSpotMarket")]
#[derive(Debug, Clone)]
pub struct PyHyperliquidSpotMarket {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub index: u32,
    #[pyo3(get)]
    pub tokens: Vec<u32>,
    #[pyo3(get)]
    pub is_canonical: Option<bool>,
}

impl From<HyperliquidSpotMarketInfo> for PyHyperliquidSpotMarket {
    fn from(m: HyperliquidSpotMarketInfo) -> Self {
        Self {
            name: m.name,
            index: m.index,
            tokens: m.tokens,
            is_canonical: m.is_canonical,
        }
    }
}

#[derive(Debug, Clone)]
enum L2BookStreamEvent {
    Update(PyHyperliquidL2BookUpdate),
    Error(String),
}

#[pyclass(name = "HyperliquidL2Level")]
#[derive(Debug, Clone)]
pub struct PyHyperliquidL2Level {
    #[pyo3(get)]
    pub price: f64,
    #[pyo3(get)]
    pub size: f64,
    #[pyo3(get)]
    pub num_orders: u32,
}

impl From<PriceLevel> for PyHyperliquidL2Level {
    fn from(level: PriceLevel) -> Self {
        Self {
            price: level.px.parse::<f64>().unwrap_or(0.0),
            size: level.sz.parse::<f64>().unwrap_or(0.0),
            num_orders: level.n,
        }
    }
}

#[pyclass(name = "HyperliquidL2BookUpdate")]
#[derive(Debug, Clone)]
pub struct PyHyperliquidL2BookUpdate {
    #[pyo3(get)]
    pub coin: String,
    #[pyo3(get)]
    pub timestamp_ms: u64,
    #[pyo3(get)]
    pub bids: Vec<PyHyperliquidL2Level>,
    #[pyo3(get)]
    pub asks: Vec<PyHyperliquidL2Level>,
    #[pyo3(get)]
    pub best_bid: f64,
    #[pyo3(get)]
    pub best_ask: f64,
    #[pyo3(get)]
    pub mid_price: f64,
    #[pyo3(get)]
    pub spread: f64,
    #[pyo3(get)]
    pub spread_bps: f64,
    #[pyo3(get)]
    pub total_bid_size: f64,
    #[pyo3(get)]
    pub total_ask_size: f64,
    #[pyo3(get)]
    pub imbalance: f64,
}

impl From<L2BookData> for PyHyperliquidL2BookUpdate {
    fn from(data: L2BookData) -> Self {
        let bids: Vec<PyHyperliquidL2Level> = data
            .levels
            .first()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(PyHyperliquidL2Level::from)
            .collect();
        let asks: Vec<PyHyperliquidL2Level> = data
            .levels
            .get(1)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(PyHyperliquidL2Level::from)
            .collect();

        let best_bid = bids.first().map(|level| level.price).unwrap_or(0.0);
        let best_ask = asks.first().map(|level| level.price).unwrap_or(0.0);
        let mid_price = if best_bid > 0.0 && best_ask > 0.0 {
            (best_bid + best_ask) / 2.0
        } else {
            0.0
        };
        let spread = if best_bid > 0.0 && best_ask > 0.0 {
            best_ask - best_bid
        } else {
            0.0
        };
        let spread_bps = if mid_price > 0.0 {
            (spread / mid_price) * 10_000.0
        } else {
            0.0
        };
        let total_bid_size: f64 = bids.iter().map(|level| level.size).sum();
        let total_ask_size: f64 = asks.iter().map(|level| level.size).sum();
        let imbalance = if total_bid_size + total_ask_size > 0.0 {
            (total_bid_size - total_ask_size) / (total_bid_size + total_ask_size)
        } else {
            0.0
        };

        Self {
            coin: data.coin,
            timestamp_ms: data.time,
            bids,
            asks,
            best_bid,
            best_ask,
            mid_price,
            spread,
            spread_bps,
            total_bid_size,
            total_ask_size,
            imbalance,
        }
    }
}

#[pymethods]
impl PyHyperliquidL2BookUpdate {
    pub fn __repr__(&self) -> String {
        format!(
            "HyperliquidL2BookUpdate(coin='{}', mid={:.6}, spread_bps={:.3})",
            self.coin, self.mid_price, self.spread_bps
        )
    }
}

#[pyclass(name = "HyperliquidL2BookStream", unsendable)]
pub struct PyHyperliquidL2BookStream {
    #[pyo3(get)]
    pub coin: String,
    #[pyo3(get)]
    pub is_testnet: bool,
    receiver: Mutex<mpsc::Receiver<L2BookStreamEvent>>,
}

#[pymethods]
impl PyHyperliquidL2BookStream {
    #[pyo3(signature = (timeout_ms=1000))]
    pub fn next_update(&self, timeout_ms: u64) -> PyResult<Option<PyHyperliquidL2BookUpdate>> {
        let receiver = self
            .receiver
            .lock()
            .map_err(|_| PyRuntimeError::new_err("L2 book stream lock poisoned"))?;

        match receiver.recv_timeout(Duration::from_millis(timeout_ms)) {
            Ok(L2BookStreamEvent::Update(update)) => Ok(Some(update)),
            Ok(L2BookStreamEvent::Error(err)) => Err(PyRuntimeError::new_err(err)),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) => Ok(None),
        }
    }
}

#[pyclass(name = "HyperliquidAsset")]
#[derive(Debug, Clone)]
pub struct PyHyperliquidAsset {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub sz_decimals: u32,
    #[pyo3(get)]
    pub max_leverage: Option<u32>,
}

impl From<neleus_adapters_hyperliquid::HyperliquidAssetInfo> for PyHyperliquidAsset {
    fn from(a: neleus_adapters_hyperliquid::HyperliquidAssetInfo) -> Self {
        Self {
            name: a.name,
            sz_decimals: a.sz_decimals,
            max_leverage: a.max_leverage,
        }
    }
}

#[pymethods]
impl PyHyperliquidAsset {
    pub fn __repr__(&self) -> String {
        format!(
            "HyperliquidAsset(name='{}', sz_decimals={}, max_leverage={:?})",
            self.name, self.sz_decimals, self.max_leverage
        )
    }
}

use neleus_persistence::{
    Candle as TimescaleCandle, FundingRate as TimescaleFundingRate, PostgresEventStore,
    PostgresEventStoreConfig, Quote as TimescaleQuote, TimescaleConfig, TimescaleStore,
    Trade as TimescaleTrade,
};

#[pyclass(name = "PostgresEventStoreConfig")]
#[derive(Debug, Clone)]
pub struct PyPostgresEventStoreConfig {
    #[pyo3(get, set)]
    pub connection_string: String,
    #[pyo3(get, set)]
    pub batch_size: usize,
    #[pyo3(get, set)]
    pub pool_size: usize,
    #[pyo3(get, set)]
    pub flush_interval_ms: u64,
}

#[pymethods]
impl PyPostgresEventStoreConfig {
    #[new]
    #[pyo3(signature = (connection_string=None, batch_size=1000, pool_size=4, flush_interval_ms=100))]
    pub fn new(
        connection_string: Option<String>,
        batch_size: usize,
        pool_size: usize,
        flush_interval_ms: u64,
    ) -> Self {
        Self {
            connection_string: connection_string.unwrap_or_else(|| {
                "postgresql://postgres:postgres@localhost:5432/neleus".to_string()
            }),
            batch_size,
            pool_size,
            flush_interval_ms,
        }
    }

    pub fn __repr__(&self) -> String {
        format!(
            "PostgresEventStoreConfig(connection_string='{}', batch_size={}, pool_size={}, flush_interval_ms={})",
            self.connection_string, self.batch_size, self.pool_size, self.flush_interval_ms
        )
    }
}

impl From<&PyPostgresEventStoreConfig> for PostgresEventStoreConfig {
    fn from(c: &PyPostgresEventStoreConfig) -> Self {
        Self {
            connection_string: c.connection_string.clone(),
            batch_size: c.batch_size,
            pool_size: c.pool_size,
            flush_interval_ms: c.flush_interval_ms,
        }
    }
}

#[pyclass(name = "TimescaleConfig")]
#[derive(Debug, Clone)]
pub struct PyTimescaleConfig {
    #[pyo3(get, set)]
    pub connection_string: String,
    #[pyo3(get, set)]
    pub pool_size: usize,
    #[pyo3(get, set)]
    pub batch_size: usize,
    #[pyo3(get, set)]
    pub flush_interval_ms: u64,
}

#[pymethods]
impl PyTimescaleConfig {
    #[new]
    #[pyo3(signature = (connection_string=None, pool_size=8, batch_size=5000, flush_interval_ms=100))]
    pub fn new(
        connection_string: Option<String>,
        pool_size: usize,
        batch_size: usize,
        flush_interval_ms: u64,
    ) -> Self {
        Self {
            connection_string: connection_string.unwrap_or_else(|| {
                "postgresql://postgres:postgres@localhost:5432/neleus_timeseries".to_string()
            }),
            pool_size,
            batch_size,
            flush_interval_ms,
        }
    }

    pub fn __repr__(&self) -> String {
        format!(
            "TimescaleConfig(connection_string='{}', pool_size={}, batch_size={}, flush_interval_ms={})",
            self.connection_string, self.pool_size, self.batch_size, self.flush_interval_ms
        )
    }
}

impl From<&PyTimescaleConfig> for TimescaleConfig {
    fn from(c: &PyTimescaleConfig) -> Self {
        Self {
            connection_string: c.connection_string.clone(),
            pool_size: c.pool_size,
            batch_size: c.batch_size,
            flush_interval_ms: c.flush_interval_ms,
        }
    }
}

#[pyclass(name = "TimescaleStore")]
pub struct PyTimescaleStore {
    inner: Arc<TimescaleStore>,
    runtime: tokio::runtime::Runtime,
}

#[pymethods]
impl PyTimescaleStore {
    #[new]
    #[pyo3(signature = (config=None))]
    pub fn new(config: Option<PyTimescaleConfig>) -> PyResult<Self> {
        let cfg = config
            .as_ref()
            .map(TimescaleConfig::from)
            .unwrap_or_default();
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;

        let store = runtime.block_on(TimescaleStore::new(cfg)).map_err(|e| {
            PyRuntimeError::new_err(format!("Failed to connect to TimescaleDB: {}", e))
        })?;

        Ok(Self {
            inner: Arc::new(store),
            runtime,
        })
    }

    #[pyo3(signature = (time_epoch_secs, venue, symbol, instrument_type, open, high, low, close, volume, trade_count=None, vwap=None))]
    pub fn insert_candle(
        &self,
        time_epoch_secs: i64,
        venue: String,
        symbol: String,
        instrument_type: String,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        trade_count: Option<i32>,
        vwap: Option<f64>,
    ) -> PyResult<()> {
        use chrono::{TimeZone, Utc};
        let candle = TimescaleCandle {
            time: Utc.timestamp_opt(time_epoch_secs, 0).unwrap(),
            venue,
            symbol,
            instrument_type,
            open,
            high,
            low,
            close,
            volume,
            trade_count,
            vwap,
        };
        self.runtime
            .block_on(self.inner.insert_candle(&candle))
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to insert candle: {}", e)))
    }

    #[pyo3(signature = (time_epoch_secs, venue, symbol, instrument_type, side, price, size, trade_id=None, is_buyer_maker=None))]
    pub fn insert_trade(
        &self,
        time_epoch_secs: i64,
        venue: String,
        symbol: String,
        instrument_type: String,
        side: String,
        price: f64,
        size: f64,
        trade_id: Option<String>,
        is_buyer_maker: Option<bool>,
    ) -> PyResult<()> {
        use chrono::{TimeZone, Utc};
        let trade = TimescaleTrade {
            time: Utc.timestamp_opt(time_epoch_secs, 0).unwrap(),
            venue,
            symbol,
            instrument_type,
            trade_id,
            side,
            price,
            size,
            is_buyer_maker,
        };
        self.runtime
            .block_on(self.inner.insert_trade(&trade))
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to insert trade: {}", e)))
    }

    pub fn insert_quote(
        &self,
        time_epoch_secs: i64,
        venue: String,
        symbol: String,
        instrument_type: String,
        bid_price: f64,
        bid_size: f64,
        ask_price: f64,
        ask_size: f64,
    ) -> PyResult<()> {
        use chrono::{TimeZone, Utc};
        let quote = TimescaleQuote {
            time: Utc.timestamp_opt(time_epoch_secs, 0).unwrap(),
            venue,
            symbol,
            instrument_type,
            bid_price,
            bid_size,
            ask_price,
            ask_size,
        };
        self.runtime
            .block_on(self.inner.insert_quote(&quote))
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to insert quote: {}", e)))
    }

    #[pyo3(signature = (time_epoch_secs, venue, symbol, rate, next_funding_time_epoch_secs=None))]
    pub fn insert_funding_rate(
        &self,
        time_epoch_secs: i64,
        venue: String,
        symbol: String,
        rate: f64,
        next_funding_time_epoch_secs: Option<i64>,
    ) -> PyResult<()> {
        use chrono::{TimeZone, Utc};
        let funding = TimescaleFundingRate {
            time: Utc.timestamp_opt(time_epoch_secs, 0).unwrap(),
            venue,
            symbol,
            rate,
            next_funding_time: next_funding_time_epoch_secs
                .map(|t| Utc.timestamp_opt(t, 0).unwrap()),
        };
        self.runtime
            .block_on(self.inner.insert_funding_rate(&funding))
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to insert funding rate: {}", e)))
    }

    pub fn __repr__(&self) -> String {
        "TimescaleStore(connected)".to_string()
    }
}

#[pyclass(name = "PostgresEventStore")]
#[allow(dead_code)]
pub struct PyPostgresEventStore {
    inner: Arc<PostgresEventStore>,
}

#[pymethods]
impl PyPostgresEventStore {
    #[new]
    #[pyo3(signature = (config=None))]
    pub fn new(config: Option<PyPostgresEventStoreConfig>) -> PyResult<Self> {
        let cfg = config
            .as_ref()
            .map(PostgresEventStoreConfig::from)
            .unwrap_or_default();
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;

        let store = runtime
            .block_on(PostgresEventStore::new(cfg))
            .map_err(|e| {
                PyRuntimeError::new_err(format!("Failed to connect to PostgreSQL: {}", e))
            })?;

        Ok(Self {
            inner: Arc::new(store),
        })
    }

    pub fn __repr__(&self) -> String {
        "PostgresEventStore(connected)".to_string()
    }
}
