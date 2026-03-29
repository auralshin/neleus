use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;

use neleus_adapters_hyperliquid::{
    HyperliquidConfig, HyperliquidExecutionClient, OrderStatus, PlaceOrderResponse,
};
use neleus_core_types::OrderSide;
use neleus_persistence::{FillRecord, OrderRecord, PnlSummary, TradeMonitor};

// ---------------------------------------------------------------------------
// Result types exposed to Python
// ---------------------------------------------------------------------------

#[pyclass(name = "OrderResult")]
#[derive(Debug, Clone)]
pub struct PyOrderResult {
    /// "ok" or "err"
    #[pyo3(get)]
    pub status: String,
    /// Client-assigned order id (cloid), if provided at submission time.
    #[pyo3(get)]
    pub cloid: Option<String>,
    /// Exchange-assigned order id, available once the order is acknowledged.
    #[pyo3(get)]
    pub order_id: Option<u64>,
    /// True when the order was immediately filled (IOC / market).
    #[pyo3(get)]
    pub filled: bool,
    /// Non-None when the exchange returns an error for this order.
    #[pyo3(get)]
    pub error: Option<String>,
}

#[pymethods]
impl PyOrderResult {
    pub fn is_ok(&self) -> bool {
        self.status == "ok" && self.error.is_none()
    }

    pub fn __repr__(&self) -> String {
        format!(
            "OrderResult(status='{}', cloid={:?}, order_id={:?}, filled={})",
            self.status, self.cloid, self.order_id, self.filled
        )
    }
}

// ---------------------------------------------------------------------------

#[pyclass(name = "OpenOrder")]
#[derive(Debug, Clone)]
pub struct PyOpenOrder {
    #[pyo3(get)]
    pub coin: String,
    #[pyo3(get)]
    pub order_id: u64,
    #[pyo3(get)]
    pub cloid: Option<String>,
    /// "buy" or "sell"
    #[pyo3(get)]
    pub side: String,
    #[pyo3(get)]
    pub size: f64,
    #[pyo3(get)]
    pub filled_size: f64,
    #[pyo3(get)]
    pub price: Option<f64>,
    #[pyo3(get)]
    pub status: String,
    #[pyo3(get)]
    pub timestamp_ms: u64,
}

#[pymethods]
impl PyOpenOrder {
    pub fn remaining_size(&self) -> f64 {
        self.size - self.filled_size
    }

    pub fn __repr__(&self) -> String {
        format!(
            "OpenOrder(coin='{}', oid={}, side='{}', sz={:.6}, price={:?})",
            self.coin, self.order_id, self.side, self.size, self.price
        )
    }
}

// ---------------------------------------------------------------------------

#[pyclass(name = "FillRecord")]
#[derive(Debug, Clone)]
pub struct PyFillRecord {
    #[pyo3(get)]
    pub coin: String,
    #[pyo3(get)]
    pub order_id: u64,
    /// "buy" or "sell"
    #[pyo3(get)]
    pub side: String,
    #[pyo3(get)]
    pub price: f64,
    #[pyo3(get)]
    pub size: f64,
    #[pyo3(get)]
    pub fee: f64,
    #[pyo3(get)]
    pub timestamp_ms: u64,
}

#[pymethods]
impl PyFillRecord {
    pub fn notional(&self) -> f64 {
        self.price * self.size
    }

    pub fn __repr__(&self) -> String {
        format!(
            "FillRecord(coin='{}', side='{}', sz={:.6}, px={:.4}, fee={:.4})",
            self.coin, self.side, self.size, self.price, self.fee
        )
    }
}

// ---------------------------------------------------------------------------
// HyperliquidTrader — live trading client
// ---------------------------------------------------------------------------

#[pyclass(name = "HyperliquidTrader")]
pub struct PyHyperliquidTrader {
    /// Wrapped behind an async Mutex so futures can hold it across `.await`.
    client: Arc<Mutex<HyperliquidExecutionClient>>,
    runtime: tokio::runtime::Runtime,
    is_testnet: bool,
}

#[pymethods]
impl PyHyperliquidTrader {
    /// Create a new trader.
    ///
    /// Parameters
    /// ----------
    /// private_key : str
    ///     Hex-encoded 32-byte private key (with or without "0x" prefix).
    /// testnet : bool
    ///     Connect to testnet when True (default False).
    /// load_metadata : bool
    ///     Automatically load asset metadata on construction so orders can be
    ///     placed immediately (default True).  Set to False and call
    ///     ``load_asset_metadata()`` manually if you want to control timing.
    #[new]
    #[pyo3(signature = (private_key, testnet = false, load_metadata = true, ws_url = None, rest_url = None))]
    pub fn new(private_key: &str, testnet: bool, load_metadata: bool, ws_url: Option<String>, rest_url: Option<String>) -> PyResult<Self> {
        let mut config = if testnet {
            HyperliquidConfig::testnet()
        } else {
            HyperliquidConfig::mainnet()
        };

        if let Some(url) = ws_url {
            config.ws_url = url;
        }
        if let Some(url) = rest_url {
            config.rest_url = url;
        }

        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("tokio runtime: {}", e)))?;

        let client = HyperliquidExecutionClient::new(config)
            .with_signer(private_key)
            .map_err(|e| PyRuntimeError::new_err(format!("signer init: {}", e)))?;

        let client = Arc::new(Mutex::new(client));

        if load_metadata {
            let c = client.clone();
            runtime
                .block_on(async move { c.lock().await.load_asset_metadata().await })
                .map_err(|e| {
                    PyRuntimeError::new_err(format!("load_asset_metadata failed: {}", e))
                })?;
        }

        Ok(Self {
            client,
            runtime,
            is_testnet: testnet,
        })
    }

    /// Reload the on-chain asset→index map.  Call this after new perpetuals
    /// are listed to ensure they can be traded.
    pub fn load_asset_metadata(&self) -> PyResult<()> {
        let c = self.client.clone();
        self.runtime
            .block_on(async move { c.lock().await.load_asset_metadata().await })
            .map_err(|e| PyRuntimeError::new_err(format!("load_asset_metadata: {}", e)))
    }

    /// Ethereum address derived from the private key.
    pub fn address(&self) -> PyResult<String> {
        let c = self.client.clone();
        Ok(self
            .runtime
            .block_on(async move {
                c.lock().await.address().map(String::from)
            })
            .unwrap_or_else(|| "unknown".to_string()))
    }

    pub fn is_testnet(&self) -> bool {
        self.is_testnet
    }

    // -----------------------------------------------------------------------
    // Order placement
    // -----------------------------------------------------------------------

    /// Submit a market order (IOC limit at mid ± slippage).
    ///
    /// Parameters
    /// ----------
    /// coin : str       — e.g. "BTC", "ETH"
    /// is_buy : bool    — True for buy, False for sell
    /// size : float     — base-asset quantity
    /// slippage_bps : int — permitted slippage in basis points (default 50 = 0.5 %)
    #[pyo3(signature = (coin, is_buy, size, slippage_bps = 50))]
    pub fn place_market_order(
        &self,
        coin: &str,
        is_buy: bool,
        size: f64,
        slippage_bps: u32,
    ) -> PyResult<PyOrderResult> {
        let c = self.client.clone();
        let coin = coin.to_string();
        let response = self
            .runtime
            .block_on(async move {
                c.lock()
                    .await
                    .submit_market_order(&coin, is_buy, size, slippage_bps)
                    .await
            })
            .map_err(|e| PyRuntimeError::new_err(format!("market order: {}", e)))?;
        Ok(parse_order_response(response, None))
    }

    /// Submit a limit order.
    ///
    /// Parameters
    /// ----------
    /// coin : str
    /// is_buy : bool
    /// size : float
    /// price : float
    /// post_only : bool — use ALO (Add Liquidity Only) time-in-force
    /// reduce_only : bool — reduce-only flag
    #[pyo3(signature = (coin, is_buy, size, price, post_only = false, reduce_only = false))]
    pub fn place_limit_order(
        &self,
        coin: &str,
        is_buy: bool,
        size: f64,
        price: f64,
        post_only: bool,
        reduce_only: bool,
    ) -> PyResult<PyOrderResult> {
        let c = self.client.clone();
        let coin = coin.to_string();
        let response = self
            .runtime
            .block_on(async move {
                c.lock()
                    .await
                    .submit_limit_order(&coin, is_buy, size, price, post_only, reduce_only)
                    .await
            })
            .map_err(|e| PyRuntimeError::new_err(format!("limit order: {}", e)))?;
        Ok(parse_order_response(response, None))
    }

    // -----------------------------------------------------------------------
    // Cancellation
    // -----------------------------------------------------------------------

    /// Cancel by exchange order id.  Returns True on success.
    pub fn cancel_order(&self, coin: &str, order_id: u64) -> PyResult<bool> {
        let c = self.client.clone();
        let coin = coin.to_string();
        let resp = self
            .runtime
            .block_on(async move { c.lock().await.cancel_order(&coin, order_id).await })
            .map_err(|e| PyRuntimeError::new_err(format!("cancel_order: {}", e)))?;
        Ok(resp.status == "ok")
    }

    /// Cancel by client order id (cloid).  Returns True on success.
    pub fn cancel_order_by_cloid(&self, coin: &str, cloid: &str) -> PyResult<bool> {
        let c = self.client.clone();
        let coin = coin.to_string();
        let cloid = cloid.to_string();
        let resp = self
            .runtime
            .block_on(async move {
                c.lock().await.cancel_order_by_cloid(&coin, &cloid).await
            })
            .map_err(|e| PyRuntimeError::new_err(format!("cancel_by_cloid: {}", e)))?;
        Ok(resp.status == "ok")
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Fetch currently open orders from the exchange.
    pub fn get_open_orders(&self) -> PyResult<Vec<PyOpenOrder>> {
        let c = self.client.clone();
        let orders = self
            .runtime
            .block_on(async move { c.lock().await.fetch_open_orders().await })
            .map_err(|e| PyRuntimeError::new_err(format!("fetch_open_orders: {}", e)))?;

        Ok(orders
            .into_iter()
            .map(|o| PyOpenOrder {
                coin: o.coin,
                order_id: o.order_id,
                cloid: o.client_order_id,
                side: side_str(o.side),
                size: o.size,
                filled_size: o.filled_size,
                price: o.price,
                status: format!("{:?}", o.status).to_lowercase(),
                timestamp_ms: o.timestamp,
            })
            .collect())
    }

    /// Fetch recent fills from the exchange.
    #[pyo3(signature = (limit = 50))]
    pub fn get_fills(&self, limit: usize) -> PyResult<Vec<PyFillRecord>> {
        let c = self.client.clone();
        let fills = self
            .runtime
            .block_on(async move { c.lock().await.fetch_fills(limit).await })
            .map_err(|e| PyRuntimeError::new_err(format!("fetch_fills: {}", e)))?;

        Ok(fills
            .into_iter()
            .map(|f| PyFillRecord {
                coin: f.coin,
                order_id: f.order_id,
                side: side_str(f.side),
                price: f.price,
                size: f.size,
                fee: f.fee,
                timestamp_ms: f.timestamp,
            })
            .collect())
    }

    // -----------------------------------------------------------------------
    // State helpers
    // -----------------------------------------------------------------------

    pub fn open_order_count(&self) -> PyResult<u32> {
        let c = self.client.clone();
        Ok(self
            .runtime
            .block_on(async move { c.lock().await.open_order_count() }))
    }

    pub fn can_place_order(&self) -> PyResult<bool> {
        let c = self.client.clone();
        Ok(self
            .runtime
            .block_on(async move { c.lock().await.can_place_order() }))
    }

    pub fn max_open_orders(&self) -> PyResult<u32> {
        let c = self.client.clone();
        Ok(self
            .runtime
            .block_on(async move { c.lock().await.max_open_orders() }))
    }

    pub fn __repr__(&self) -> String {
        let addr = self.client.try_lock().ok().map(|g| {
            g.address()
                .map(String::from)
                .unwrap_or_else(|| "unknown".to_string())
        }).unwrap_or_else(|| "locked".to_string());
        format!(
            "HyperliquidTrader(address='{}', testnet={})",
            addr, self.is_testnet
        )
    }
}

// ---------------------------------------------------------------------------
// TradeMonitor — database-backed order/fill monitoring
// ---------------------------------------------------------------------------

#[pyclass(name = "DbOrderRecord")]
#[derive(Debug, Clone)]
pub struct PyDbOrderRecord {
    #[pyo3(get)]
    pub time_epoch_secs: i64,
    #[pyo3(get)]
    pub cloid: String,
    #[pyo3(get)]
    pub order_id: Option<i64>,
    #[pyo3(get)]
    pub coin: String,
    #[pyo3(get)]
    pub side: String,
    #[pyo3(get)]
    pub order_type: String,
    #[pyo3(get)]
    pub size: f64,
    #[pyo3(get)]
    pub price: Option<f64>,
    #[pyo3(get)]
    pub reduce_only: bool,
    #[pyo3(get)]
    pub status: String,
    #[pyo3(get)]
    pub filled_size: f64,
    #[pyo3(get)]
    pub avg_fill_price: Option<f64>,
    #[pyo3(get)]
    pub is_testnet: bool,
}

#[pymethods]
impl PyDbOrderRecord {
    pub fn is_open(&self) -> bool {
        matches!(self.status.as_str(), "submitted" | "open")
    }

    pub fn remaining_size(&self) -> f64 {
        self.size - self.filled_size
    }

    pub fn __repr__(&self) -> String {
        format!(
            "DbOrderRecord(cloid='{}', coin='{}', side='{}', sz={:.6}, status='{}')",
            self.cloid, self.coin, self.side, self.size, self.status
        )
    }
}

impl From<OrderRecord> for PyDbOrderRecord {
    fn from(r: OrderRecord) -> Self {
        Self {
            time_epoch_secs: r.time.timestamp(),
            cloid: r.cloid,
            order_id: r.order_id,
            coin: r.coin,
            side: r.side,
            order_type: r.order_type,
            size: r.size,
            price: r.price,
            reduce_only: r.reduce_only,
            status: r.status,
            filled_size: r.filled_size,
            avg_fill_price: r.avg_fill_price,
            is_testnet: r.is_testnet,
        }
    }
}

#[pyclass(name = "DbFillRecord")]
#[derive(Debug, Clone)]
pub struct PyDbFillRecord {
    #[pyo3(get)]
    pub time_epoch_secs: i64,
    #[pyo3(get)]
    pub order_id: i64,
    #[pyo3(get)]
    pub coin: String,
    #[pyo3(get)]
    pub side: String,
    #[pyo3(get)]
    pub price: f64,
    #[pyo3(get)]
    pub size: f64,
    #[pyo3(get)]
    pub fee: f64,
    #[pyo3(get)]
    pub cloid: Option<String>,
    #[pyo3(get)]
    pub is_testnet: bool,
}

#[pymethods]
impl PyDbFillRecord {
    pub fn notional(&self) -> f64 {
        self.price * self.size
    }

    pub fn __repr__(&self) -> String {
        format!(
            "DbFillRecord(coin='{}', side='{}', sz={:.6}, px={:.4}, fee={:.4})",
            self.coin, self.side, self.size, self.price, self.fee
        )
    }
}

impl From<FillRecord> for PyDbFillRecord {
    fn from(r: FillRecord) -> Self {
        Self {
            time_epoch_secs: r.time.timestamp(),
            order_id: r.order_id,
            coin: r.coin,
            side: r.side,
            price: r.price,
            size: r.size,
            fee: r.fee,
            cloid: r.cloid,
            is_testnet: r.is_testnet,
        }
    }
}

#[pyclass(name = "PnlSummary")]
#[derive(Debug, Clone)]
pub struct PyPnlSummary {
    #[pyo3(get)]
    pub coin: String,
    #[pyo3(get)]
    pub buy_notional: f64,
    #[pyo3(get)]
    pub sell_notional: f64,
    #[pyo3(get)]
    pub realized_pnl: f64,
    #[pyo3(get)]
    pub total_fee: f64,
    #[pyo3(get)]
    pub net_pnl: f64,
}

#[pymethods]
impl PyPnlSummary {
    pub fn __repr__(&self) -> String {
        format!(
            "PnlSummary(coin='{}', realized={:.4}, fee={:.4}, net={:.4})",
            self.coin, self.realized_pnl, self.total_fee, self.net_pnl
        )
    }
}

impl From<PnlSummary> for PyPnlSummary {
    fn from(s: PnlSummary) -> Self {
        Self {
            coin: s.coin,
            buy_notional: s.buy_notional,
            sell_notional: s.sell_notional,
            realized_pnl: s.realized_pnl,
            total_fee: s.total_fee,
            net_pnl: s.net_pnl,
        }
    }
}

/// Database-backed monitor for Hyperliquid orders and fills.
///
/// Requires a PostgreSQL (or TimescaleDB) connection.  Creates the
/// ``hl_orders`` and ``hl_fills`` tables automatically on first use.
#[pyclass(name = "TradeMonitor")]
pub struct PyTradeMonitor {
    inner: Arc<TradeMonitor>,
    runtime: tokio::runtime::Runtime,
    is_testnet: bool,
}

#[pymethods]
impl PyTradeMonitor {
    /// Parameters
    /// ----------
    /// connection_string : str
    ///     PostgreSQL DSN, e.g. ``"postgresql://user:pass@localhost/db"``
    /// testnet : bool
    ///     Tag all records with this flag so mainnet and testnet data coexist
    ///     in the same tables.
    /// pool_size : int
    ///     Connection pool size (default 4).
    #[new]
    #[pyo3(signature = (connection_string, testnet = false, pool_size = 4))]
    pub fn new(connection_string: &str, testnet: bool, pool_size: usize) -> PyResult<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("tokio runtime: {}", e)))?;

        let cs = connection_string.to_string();
        let monitor = runtime
            .block_on(async move { TradeMonitor::new(&cs, pool_size).await })
            .map_err(|e| PyRuntimeError::new_err(format!("TradeMonitor::new: {}", e)))?;

        Ok(Self {
            inner: Arc::new(monitor),
            runtime,
            is_testnet: testnet,
        })
    }

    // -----------------------------------------------------------------------
    // Write
    // -----------------------------------------------------------------------

    /// Record an order that was just submitted.
    ///
    /// Parameters
    /// ----------
    /// cloid : str        — client order id
    /// coin : str
    /// side : str         — "buy" or "sell"
    /// order_type : str   — "market", "limit", "post_only"
    /// size : float
    /// price : float | None
    /// reduce_only : bool
    #[pyo3(signature = (cloid, coin, side, order_type, size, price = None, reduce_only = false))]
    pub fn record_order(
        &self,
        cloid: &str,
        coin: &str,
        side: &str,
        order_type: &str,
        size: f64,
        price: Option<f64>,
        reduce_only: bool,
    ) -> PyResult<()> {
        use chrono::Utc;
        let record = OrderRecord {
            time: Utc::now(),
            cloid: cloid.to_string(),
            order_id: None,
            coin: coin.to_string(),
            side: side.to_string(),
            order_type: order_type.to_string(),
            size,
            price,
            reduce_only,
            status: "submitted".to_string(),
            filled_size: 0.0,
            avg_fill_price: None,
            is_testnet: self.is_testnet,
        };

        let inner = self.inner.clone();
        self.runtime
            .block_on(async move { inner.record_order(&record).await })
            .map_err(|e| PyRuntimeError::new_err(format!("record_order: {}", e)))
    }

    /// Update order status after receiving an exchange acknowledgement.
    ///
    /// Parameters
    /// ----------
    /// cloid : str
    /// status : str       — "open" | "filled" | "canceled" | "rejected"
    /// order_id : int | None
    /// filled_size : float
    /// avg_fill_price : float | None
    #[pyo3(signature = (cloid, status, order_id = None, filled_size = 0.0, avg_fill_price = None))]
    pub fn update_order_status(
        &self,
        cloid: &str,
        status: &str,
        order_id: Option<i64>,
        filled_size: f64,
        avg_fill_price: Option<f64>,
    ) -> PyResult<u64> {
        let inner = self.inner.clone();
        let cloid = cloid.to_string();
        let status = status.to_string();
        self.runtime
            .block_on(async move {
                inner
                    .update_order_status(&cloid, &status, order_id, filled_size, avg_fill_price)
                    .await
            })
            .map_err(|e| PyRuntimeError::new_err(format!("update_order_status: {}", e)))
    }

    /// Record a fill received from the exchange.
    ///
    /// Parameters
    /// ----------
    /// order_id : int
    /// coin : str
    /// side : str    — "buy" or "sell"
    /// price : float
    /// size : float
    /// fee : float
    /// cloid : str | None
    #[pyo3(signature = (order_id, coin, side, price, size, fee, cloid = None))]
    pub fn record_fill(
        &self,
        order_id: i64,
        coin: &str,
        side: &str,
        price: f64,
        size: f64,
        fee: f64,
        cloid: Option<String>,
    ) -> PyResult<()> {
        use chrono::Utc;
        let fill = FillRecord {
            time: Utc::now(),
            order_id,
            coin: coin.to_string(),
            side: side.to_string(),
            price,
            size,
            fee,
            cloid,
            is_testnet: self.is_testnet,
        };

        let inner = self.inner.clone();
        self.runtime
            .block_on(async move { inner.record_fill(&fill).await })
            .map_err(|e| PyRuntimeError::new_err(format!("record_fill: {}", e)))
    }

    /// Convenience: record a live fill returned by `HyperliquidTrader.get_fills()`.
    pub fn record_live_fill(&self, fill: &PyFillRecord) -> PyResult<()> {
        self.record_fill(
            fill.order_id as i64,
            &fill.coin.clone(),
            &fill.side.clone(),
            fill.price,
            fill.size,
            fill.fee,
            None,
        )
    }

    // -----------------------------------------------------------------------
    // Read
    // -----------------------------------------------------------------------

    /// Return orders currently tracked as open/submitted.
    pub fn get_open_orders(&self) -> PyResult<Vec<PyDbOrderRecord>> {
        let inner = self.inner.clone();
        let testnet = self.is_testnet;
        self.runtime
            .block_on(async move { inner.get_open_orders(testnet).await })
            .map(|v| v.into_iter().map(PyDbOrderRecord::from).collect())
            .map_err(|e| PyRuntimeError::new_err(format!("get_open_orders: {}", e)))
    }

    /// Return recent orders, optionally filtered to a single coin.
    #[pyo3(signature = (coin = None, limit = 100))]
    pub fn get_orders(
        &self,
        coin: Option<&str>,
        limit: i64,
    ) -> PyResult<Vec<PyDbOrderRecord>> {
        let inner = self.inner.clone();
        let testnet = self.is_testnet;
        let coin = coin.map(String::from);
        self.runtime
            .block_on(async move {
                inner.get_orders(coin.as_deref(), limit, testnet).await
            })
            .map(|v| v.into_iter().map(PyDbOrderRecord::from).collect())
            .map_err(|e| PyRuntimeError::new_err(format!("get_orders: {}", e)))
    }

    /// Return recent fills, optionally filtered to a single coin.
    #[pyo3(signature = (coin = None, limit = 100))]
    pub fn get_fills(
        &self,
        coin: Option<&str>,
        limit: i64,
    ) -> PyResult<Vec<PyDbFillRecord>> {
        let inner = self.inner.clone();
        let testnet = self.is_testnet;
        let coin = coin.map(String::from);
        self.runtime
            .block_on(async move {
                inner.get_fills(coin.as_deref(), limit, testnet).await
            })
            .map(|v| v.into_iter().map(PyDbFillRecord::from).collect())
            .map_err(|e| PyRuntimeError::new_err(format!("get_fills: {}", e)))
    }

    /// Return per-coin realized PnL summary, optionally filtered to one coin.
    #[pyo3(signature = (coin = None))]
    pub fn get_pnl_summary(&self, coin: Option<&str>) -> PyResult<Vec<PyPnlSummary>> {
        let inner = self.inner.clone();
        let testnet = self.is_testnet;
        let coin = coin.map(String::from);
        self.runtime
            .block_on(async move {
                inner.get_pnl_summary(coin.as_deref(), testnet).await
            })
            .map(|v| v.into_iter().map(PyPnlSummary::from).collect())
            .map_err(|e| PyRuntimeError::new_err(format!("get_pnl_summary: {}", e)))
    }

    pub fn is_testnet(&self) -> bool {
        self.is_testnet
    }

    pub fn __repr__(&self) -> String {
        format!("TradeMonitor(testnet={})", self.is_testnet)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn side_str(side: OrderSide) -> String {
    match side {
        OrderSide::Buy => "buy".to_string(),
        OrderSide::Sell => "sell".to_string(),
    }
}

fn parse_order_response(response: PlaceOrderResponse, cloid: Option<String>) -> PyOrderResult {
    let mut order_id = None;
    let mut filled = false;
    let mut error = None;

    if let Some(ref resp_data) = response.response {
        if let Some(ref statuses) = resp_data.data {
            for status in &statuses.statuses {
                match status {
                    OrderStatus::Resting { resting } => {
                        order_id = Some(resting.oid);
                    }
                    OrderStatus::Filled { filled: f } => {
                        order_id = Some(f.oid);
                        filled = true;
                    }
                    OrderStatus::Error { error: e } => {
                        error = Some(e.clone());
                    }
                }
            }
        }
    }

    PyOrderResult {
        status: response.status,
        cloid,
        order_id,
        filled,
        error,
    }
}

use neleus_adapters_polymarket::{
    auth::{L2Authenticator, PolymarketSigner},
    client::{OrderRequest, PolymarketClient as RustPolymarketClient},
    PolymarketConfig,
};

#[pyclass(name = "PolymarketOrder")]
#[derive(Debug, Clone)]
pub struct PyPolymarketOrder {
    #[pyo3(get)]
    pub order_id: String,
    #[pyo3(get)]
    pub market: String,
    #[pyo3(get)]
    pub asset_id: String,
    #[pyo3(get)]
    pub order_type: String,
    #[pyo3(get)]
    pub side: String,
    #[pyo3(get)]
    pub price: String,
    #[pyo3(get)]
    pub size: String,
    #[pyo3(get)]
    pub original_size: String,
    #[pyo3(get)]
    pub created_at: String,
    #[pyo3(get)]
    pub status: String,
    #[pyo3(get)]
    pub owner: String,
}

impl From<neleus_adapters_polymarket::PolymarketOrder> for PyPolymarketOrder {
    fn from(o: neleus_adapters_polymarket::PolymarketOrder) -> Self {
        Self {
            order_id: o.order_id,
            market: o.market,
            asset_id: o.asset_id,
            order_type: o.order_type,
            side: o.side,
            price: o.price,
            size: o.size,
            original_size: o.original_size,
            created_at: o.created_at,
            status: format!("{:?}", o.status),
            owner: o.owner,
        }
    }
}

#[pyclass(name = "PolymarketPosition")]
#[derive(Debug, Clone)]
pub struct PyPolymarketPosition {
    #[pyo3(get)]
    pub asset_id: String,
    #[pyo3(get)]
    pub market: String,
    #[pyo3(get)]
    pub size: String,
    #[pyo3(get)]
    pub realized_pnl: Option<String>,
}

impl From<neleus_adapters_polymarket::PolymarketPosition> for PyPolymarketPosition {
    fn from(p: neleus_adapters_polymarket::PolymarketPosition) -> Self {
        Self {
            asset_id: p.asset_id,
            market: p.market,
            size: p.size,
            realized_pnl: p.realized_pnl,
        }
    }
}

#[pyclass(name = "PolymarketTrader")]
pub struct PyPolymarketTrader {
    client: Arc<Mutex<RustPolymarketClient>>,
    runtime: tokio::runtime::Runtime,
    is_testnet: bool,
}

#[pymethods]
impl PyPolymarketTrader {
    #[new]
    #[pyo3(signature = (private_key, testnet = false, funder_address = None, clob_url = None, gamma_url = None, ws_url = None, api_key = None, api_secret = None, api_passphrase = None))]
    pub fn new(
        private_key: &str,
        testnet: bool,
        funder_address: Option<String>,
        clob_url: Option<String>,
        gamma_url: Option<String>,
        ws_url: Option<String>,
        api_key: Option<String>,
        api_secret: Option<String>,
        api_passphrase: Option<String>,
    ) -> PyResult<Self> {
        let mut config = if testnet {
            PolymarketConfig::testnet()
        } else {
            PolymarketConfig::mainnet()
        };

        if let Some(url) = clob_url {
            config.clob_url = url;
        }
        if let Some(url) = gamma_url {
            config.gamma_url = url;
        }
        if let Some(url) = ws_url {
            config.ws_url = url;
        }

        if let Some(funder) = funder_address {
            config = config.with_funder(funder);
        }

        config.private_key = Some(private_key.to_string());

        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("tokio runtime: {}", e)))?;

        let signer = PolymarketSigner::new(config.clone())
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid private key: {:?}", e)))?;
            
        config.signer_address = Some(signer.get_address());

        let mut client = RustPolymarketClient::new(config.clone()).with_signer(signer);

        if let (Some(ak), Some(asect), Some(apass)) = (api_key, api_secret, api_passphrase) {
            let auth = L2Authenticator::new(ak, asect, apass);
            client = client.with_l2_auth(auth);
        } else {
            runtime.block_on(async {
                if client.derive_api_key().await.is_err() {
                    let _ = client.create_api_key().await;
                }
            });
        }

        Ok(Self {
            client: Arc::new(Mutex::new(client)),
            runtime,
            is_testnet: testnet,
        })
    }

    pub fn get_orders(&self) -> PyResult<Vec<PyPolymarketOrder>> {
        let client_arc = self.client.clone();
        let orders = self.runtime.block_on(async move {
            client_arc.lock().await.get_orders().await
        }).map_err(|e| PyRuntimeError::new_err(format!("fetch orders: {:?}", e)))?;

        Ok(orders.into_iter().map(PyPolymarketOrder::from).collect())
    }

    pub fn get_positions(&self) -> PyResult<Vec<PyPolymarketPosition>> {
        let client_arc = self.client.clone();
        let positions = self.runtime.block_on(async move {
            client_arc.lock().await.get_positions().await
        }).map_err(|e| PyRuntimeError::new_err(format!("fetch positions: {:?}", e)))?;

        Ok(positions.into_iter().map(PyPolymarketPosition::from).collect())
    }

    #[pyo3(signature = (token_id, maker_amount, taker_amount, side, fee_rate_bps = "0", nonce = "0", expiration = "0"))]
    pub fn place_order(
        &self,
        token_id: &str,
        maker_amount: &str,
        taker_amount: &str,
        side: &str,
        fee_rate_bps: &str,
        nonce: &str,
        expiration: &str,
    ) -> PyResult<String> {
        let req = OrderRequest {
            token_id: token_id.to_string(),
            maker_amount: maker_amount.to_string(),
            taker_amount: taker_amount.to_string(),
            side: side.to_string(),
            fee_rate_bps: fee_rate_bps.to_string(),
            nonce: nonce.to_string(),
            expiration: expiration.to_string(),
        };

        let client_arc = self.client.clone();
        let resp = self.runtime.block_on(async move {
            client_arc.lock().await.place_order(req).await
        }).map_err(|e| PyRuntimeError::new_err(format!("place order: {:?}", e)))?;

        Ok(resp.order_id)
    }

    pub fn cancel_order(&self, order_id: &str) -> PyResult<bool> {
        let client_arc = self.client.clone();
        let order_id = order_id.to_string();
        let resp = self.runtime.block_on(async move {
            client_arc.lock().await.cancel_order(&order_id).await
        }).map_err(|e| PyRuntimeError::new_err(format!("cancel order: {:?}", e)))?;

        Ok(resp.success)
    }

    pub fn cancel_all_orders(&self) -> PyResult<u32> {
        let client_arc = self.client.clone();
        let resp = self.runtime.block_on(async move {
            client_arc.lock().await.cancel_all_orders().await
        }).map_err(|e| PyRuntimeError::new_err(format!("cancel all orders: {:?}", e)))?;

        Ok(resp.cancelled)
    }
}

