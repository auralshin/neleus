use neleus_core_types::{
    FixedPoint, InstrumentId, InstrumentType, OrderId, Price, Quantity, StrategyId, UnixNanos,
    Venue,
};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[pyfunction]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyVenue {
    Hyperliquid,
    Lighter,
    Simulated,
}

impl From<PyVenue> for Venue {
    fn from(v: PyVenue) -> Self {
        match v {
            PyVenue::Hyperliquid => Venue::Hyperliquid,
            PyVenue::Lighter => Venue::Lighter,
            PyVenue::Simulated => Venue::Simulated,
        }
    }
}

impl From<Venue> for PyVenue {
    fn from(v: Venue) -> Self {
        match v {
            Venue::Hyperliquid => PyVenue::Hyperliquid,
            Venue::Lighter => PyVenue::Lighter,
            Venue::Simulated => PyVenue::Simulated,
        }
    }
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyInstrumentType {
    Perp,
    Spot,
}

impl From<PyInstrumentType> for InstrumentType {
    fn from(t: PyInstrumentType) -> Self {
        match t {
            PyInstrumentType::Perp => InstrumentType::Perp,
            PyInstrumentType::Spot => InstrumentType::Spot,
        }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyInstrumentId {
    #[pyo3(get)]
    pub venue: PyVenue,
    #[pyo3(get)]
    pub symbol: String,
    #[pyo3(get)]
    pub instrument_type: PyInstrumentType,
}

#[pymethods]
impl PyInstrumentId {
    #[new]
    pub fn new(venue: PyVenue, symbol: String, instrument_type: PyInstrumentType) -> Self {
        Self {
            venue,
            symbol,
            instrument_type,
        }
    }

    #[staticmethod]
    pub fn parse(s: &str) -> PyResult<Self> {
        InstrumentId::parse(s)
            .map(|id| Self::from_rust(&id))
            .ok_or_else(|| PyValueError::new_err(format!("Invalid instrument ID: {}", s)))
    }

    pub fn __str__(&self) -> String {
        format!(
            "{}:{}.{:?}",
            match self.venue {
                PyVenue::Hyperliquid => "HYPERLIQUID",
                PyVenue::Lighter => "LIGHTER",
                PyVenue::Simulated => "SIMULATED",
            },
            self.symbol,
            self.instrument_type
        )
    }

    pub fn __repr__(&self) -> String {
        format!("InstrumentId({})", self.__str__())
    }
}

impl PyInstrumentId {
    pub fn to_rust(&self) -> InstrumentId {
        InstrumentId::new(self.venue.into(), &self.symbol, self.instrument_type.into())
    }

    pub fn from_rust(id: &InstrumentId) -> Self {
        Self {
            venue: id.venue.into(),
            symbol: id.symbol.clone(),
            instrument_type: match id.kind {
                InstrumentType::Perp => PyInstrumentType::Perp,
                InstrumentType::Spot => PyInstrumentType::Spot,
            },
        }
    }
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyOrderSide {
    Buy,
    Sell,
}

#[pymethods]
impl PyOrderSide {
    pub fn opposite(&self) -> PyOrderSide {
        match self {
            PyOrderSide::Buy => PyOrderSide::Sell,
            PyOrderSide::Sell => PyOrderSide::Buy,
        }
    }
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyOrderType {
    Market,
    Limit,
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyTimeInForce {
    GTC,
    IOC,
    FOK,
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyTradeTick {
    #[pyo3(get)]
    pub instrument_id: PyInstrumentId,
    #[pyo3(get)]
    pub price: f64,
    #[pyo3(get)]
    pub quantity: f64,
    #[pyo3(get)]
    pub side: PyOrderSide,
    #[pyo3(get)]
    pub timestamp_ns: u64,
}

#[pymethods]
impl PyTradeTick {
    #[new]
    pub fn new(
        instrument_id: PyInstrumentId,
        price: f64,
        quantity: f64,
        side: PyOrderSide,
        timestamp_ns: u64,
    ) -> Self {
        Self {
            instrument_id,
            price,
            quantity,
            side,
            timestamp_ns,
        }
    }

    pub fn notional(&self) -> f64 {
        self.price * self.quantity
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyQuoteTick {
    #[pyo3(get)]
    pub instrument_id: PyInstrumentId,
    #[pyo3(get)]
    pub bid_price: f64,
    #[pyo3(get)]
    pub bid_size: f64,
    #[pyo3(get)]
    pub ask_price: f64,
    #[pyo3(get)]
    pub ask_size: f64,
    #[pyo3(get)]
    pub timestamp_ns: u64,
}

#[pymethods]
impl PyQuoteTick {
    #[new]
    pub fn new(
        instrument_id: PyInstrumentId,
        bid_price: f64,
        bid_size: f64,
        ask_price: f64,
        ask_size: f64,
        timestamp_ns: u64,
    ) -> Self {
        Self {
            instrument_id,
            bid_price,
            bid_size,
            ask_price,
            ask_size,
            timestamp_ns,
        }
    }

    pub fn mid_price(&self) -> f64 {
        (self.bid_price + self.ask_price) / 2.0
    }

    pub fn spread(&self) -> f64 {
        self.ask_price - self.bid_price
    }

    pub fn spread_bps(&self) -> f64 {
        self.spread() / self.mid_price() * 10000.0
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyBookLevel {
    #[pyo3(get)]
    pub price: f64,
    #[pyo3(get)]
    pub size: f64,
}

#[pymethods]
impl PyBookLevel {
    #[new]
    pub fn new(price: f64, size: f64) -> Self {
        Self { price, size }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyOrderBook {
    #[pyo3(get)]
    pub instrument_id: PyInstrumentId,
    #[pyo3(get)]
    pub bids: Vec<PyBookLevel>,
    #[pyo3(get)]
    pub asks: Vec<PyBookLevel>,
    #[pyo3(get)]
    pub timestamp_ns: u64,
}

#[pymethods]
impl PyOrderBook {
    #[new]
    pub fn new(
        instrument_id: PyInstrumentId,
        bids: Vec<PyBookLevel>,
        asks: Vec<PyBookLevel>,
        timestamp_ns: u64,
    ) -> Self {
        Self {
            instrument_id,
            bids,
            asks,
            timestamp_ns,
        }
    }

    pub fn best_bid(&self) -> Option<PyBookLevel> {
        self.bids.first().cloned()
    }

    pub fn best_ask(&self) -> Option<PyBookLevel> {
        self.asks.first().cloned()
    }

    pub fn mid_price(&self) -> Option<f64> {
        match (self.bids.first(), self.asks.first()) {
            (Some(bid), Some(ask)) => Some((bid.price + ask.price) / 2.0),
            _ => None,
        }
    }

    pub fn spread(&self) -> Option<f64> {
        match (self.bids.first(), self.asks.first()) {
            (Some(bid), Some(ask)) => Some(ask.price - bid.price),
            _ => None,
        }
    }

    pub fn bid_depth(&self, depth: usize) -> f64 {
        self.bids.iter().take(depth).map(|l| l.size).sum()
    }

    pub fn ask_depth(&self, depth: usize) -> f64 {
        self.asks.iter().take(depth).map(|l| l.size).sum()
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyBar {
    #[pyo3(get)]
    pub instrument_id: PyInstrumentId,
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
    pub timestamp_ns: u64,
}

#[pymethods]
impl PyBar {
    #[new]
    pub fn new(
        instrument_id: PyInstrumentId,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        timestamp_ns: u64,
    ) -> Self {
        Self {
            instrument_id,
            open,
            high,
            low,
            close,
            volume,
            timestamp_ns,
        }
    }

    pub fn range(&self) -> f64 {
        self.high - self.low
    }

    pub fn body(&self) -> f64 {
        (self.close - self.open).abs()
    }

    pub fn is_bullish(&self) -> bool {
        self.close > self.open
    }

    pub fn is_bearish(&self) -> bool {
        self.close < self.open
    }
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyOrderState {
    New,
    Submitted,
    Accepted,
    PartiallyFilled,
    Filled,
    PendingCancel,
    Canceled,
    Rejected,
    Expired,
}

#[pymethods]
impl PyOrderState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PyOrderState::Filled
                | PyOrderState::Canceled
                | PyOrderState::Rejected
                | PyOrderState::Expired
        )
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self,
            PyOrderState::Submitted | PyOrderState::Accepted | PyOrderState::PartiallyFilled
        )
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyOrder {
    #[pyo3(get)]
    pub order_id: String,
    #[pyo3(get)]
    pub instrument_id: PyInstrumentId,
    #[pyo3(get)]
    pub side: PyOrderSide,
    #[pyo3(get)]
    pub order_type: PyOrderType,
    #[pyo3(get)]
    pub price: Option<f64>,
    #[pyo3(get)]
    pub quantity: f64,
    #[pyo3(get)]
    pub filled_quantity: f64,
    #[pyo3(get)]
    pub state: PyOrderState,
    #[pyo3(get)]
    pub created_ns: u64,
}

#[pymethods]
impl PyOrder {
    #[new]
    #[pyo3(signature = (order_id, instrument_id, side, order_type, quantity, price=None, filled_quantity=0.0, state=PyOrderState::New, created_ns=0))]
    pub fn new(
        order_id: String,
        instrument_id: PyInstrumentId,
        side: PyOrderSide,
        order_type: PyOrderType,
        quantity: f64,
        price: Option<f64>,
        filled_quantity: f64,
        state: PyOrderState,
        created_ns: u64,
    ) -> Self {
        Self {
            order_id,
            instrument_id,
            side,
            order_type,
            price,
            quantity,
            filled_quantity,
            state,
            created_ns,
        }
    }

    pub fn remaining_quantity(&self) -> f64 {
        self.quantity - self.filled_quantity
    }

    pub fn is_filled(&self) -> bool {
        self.state == PyOrderState::Filled
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            PyOrderState::Submitted | PyOrderState::Accepted | PyOrderState::PartiallyFilled
        )
    }

    pub fn fill_percentage(&self) -> f64 {
        if self.quantity == 0.0 {
            0.0
        } else {
            self.filled_quantity / self.quantity * 100.0
        }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyFill {
    #[pyo3(get)]
    pub fill_id: String,
    #[pyo3(get)]
    pub order_id: String,
    #[pyo3(get)]
    pub instrument_id: PyInstrumentId,
    #[pyo3(get)]
    pub side: PyOrderSide,
    #[pyo3(get)]
    pub price: f64,
    #[pyo3(get)]
    pub quantity: f64,
    #[pyo3(get)]
    pub commission: f64,
    #[pyo3(get)]
    pub is_maker: bool,
    #[pyo3(get)]
    pub timestamp_ns: u64,
}

#[pymethods]
impl PyFill {
    #[new]
    pub fn new(
        fill_id: String,
        order_id: String,
        instrument_id: PyInstrumentId,
        side: PyOrderSide,
        price: f64,
        quantity: f64,
        commission: f64,
        is_maker: bool,
        timestamp_ns: u64,
    ) -> Self {
        Self {
            fill_id,
            order_id,
            instrument_id,
            side,
            price,
            quantity,
            commission,
            is_maker,
            timestamp_ns,
        }
    }

    pub fn notional(&self) -> f64 {
        self.price * self.quantity
    }

    pub fn net_value(&self) -> f64 {
        self.notional() - self.commission
    }
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyPositionSide {
    Flat,
    Long,
    Short,
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyPosition {
    #[pyo3(get)]
    pub instrument_id: PyInstrumentId,
    #[pyo3(get)]
    pub side: PyPositionSide,
    #[pyo3(get)]
    pub quantity: f64,
    #[pyo3(get)]
    pub avg_entry_price: f64,
    #[pyo3(get)]
    pub realized_pnl: f64,
    #[pyo3(get)]
    pub unrealized_pnl: f64,
}

#[pymethods]
impl PyPosition {
    #[new]
    pub fn new(
        instrument_id: PyInstrumentId,
        side: PyPositionSide,
        quantity: f64,
        avg_entry_price: f64,
        realized_pnl: f64,
        unrealized_pnl: f64,
    ) -> Self {
        Self {
            instrument_id,
            side,
            quantity,
            avg_entry_price,
            realized_pnl,
            unrealized_pnl,
        }
    }

    pub fn is_flat(&self) -> bool {
        self.side == PyPositionSide::Flat
    }

    pub fn is_long(&self) -> bool {
        self.side == PyPositionSide::Long
    }

    pub fn is_short(&self) -> bool {
        self.side == PyPositionSide::Short
    }

    pub fn total_pnl(&self) -> f64 {
        self.realized_pnl + self.unrealized_pnl
    }

    pub fn notional(&self) -> f64 {
        self.quantity * self.avg_entry_price
    }

    pub fn pnl_percentage(&self) -> f64 {
        let notional = self.notional();
        if notional == 0.0 {
            0.0
        } else {
            self.total_pnl() / notional * 100.0
        }
    }
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PySubscriptionType {
    Trades,
    Quotes,
    OrderBook,
    Bars,
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyOrderRequest {
    #[pyo3(get)]
    pub instrument_id: PyInstrumentId,
    #[pyo3(get)]
    pub side: PyOrderSide,
    #[pyo3(get)]
    pub order_type: PyOrderType,
    #[pyo3(get)]
    pub price: Option<f64>,
    #[pyo3(get)]
    pub quantity: f64,
    #[pyo3(get)]
    pub time_in_force: PyTimeInForce,
    #[pyo3(get)]
    pub reduce_only: bool,
}

#[pyclass]
#[derive(Debug)]
pub struct PyStrategyContext {
    #[pyo3(get)]
    pub timestamp_ns: u64,

    order_requests: Vec<PyOrderRequest>,

    cancel_requests: Vec<String>,

    subscriptions: Vec<(PyInstrumentId, PySubscriptionType, Option<u32>)>,

    positions: HashMap<String, PyPosition>,

    orders: HashMap<String, PyOrder>,
}

#[pymethods]
impl PyStrategyContext {
    #[new]
    pub fn new(timestamp_ns: u64) -> Self {
        Self {
            timestamp_ns,
            order_requests: Vec::new(),
            cancel_requests: Vec::new(),
            subscriptions: Vec::new(),
            positions: HashMap::new(),
            orders: HashMap::new(),
        }
    }

    #[pyo3(signature = (instrument_id, side, quantity, reduce_only=false))]
    pub fn market_order(
        &mut self,
        instrument_id: PyInstrumentId,
        side: PyOrderSide,
        quantity: f64,
        reduce_only: bool,
    ) -> String {
        let order_id = format!(
            "{:016x}",
            self.timestamp_ns ^ (self.order_requests.len() as u64)
        );
        self.order_requests.push(PyOrderRequest {
            instrument_id,
            side,
            order_type: PyOrderType::Market,
            price: None,
            quantity,
            time_in_force: PyTimeInForce::IOC,
            reduce_only,
        });
        order_id
    }

    #[pyo3(signature = (instrument_id, side, price, quantity, time_in_force=PyTimeInForce::GTC, reduce_only=false))]
    pub fn limit_order(
        &mut self,
        instrument_id: PyInstrumentId,
        side: PyOrderSide,
        price: f64,
        quantity: f64,
        time_in_force: PyTimeInForce,
        reduce_only: bool,
    ) -> String {
        let order_id = format!(
            "{:016x}",
            self.timestamp_ns ^ (self.order_requests.len() as u64)
        );
        self.order_requests.push(PyOrderRequest {
            instrument_id,
            side,
            order_type: PyOrderType::Limit,
            price: Some(price),
            quantity,
            time_in_force,
            reduce_only,
        });
        order_id
    }

    pub fn cancel_order(&mut self, order_id: String) {
        self.cancel_requests.push(order_id);
    }

    pub fn cancel_all_orders(&mut self, instrument_id: &PyInstrumentId) {
        let key = instrument_id.__str__();
        let order_ids: Vec<String> = self
            .orders
            .iter()
            .filter(|(_, o)| o.instrument_id.__str__() == key && o.is_active())
            .map(|(id, _)| id.clone())
            .collect();
        for id in order_ids {
            self.cancel_requests.push(id);
        }
    }

    pub fn subscribe_trades(&mut self, instrument_id: PyInstrumentId) {
        self.subscriptions
            .push((instrument_id, PySubscriptionType::Trades, None));
    }

    pub fn subscribe_quotes(&mut self, instrument_id: PyInstrumentId) {
        self.subscriptions
            .push((instrument_id, PySubscriptionType::Quotes, None));
    }

    #[pyo3(signature = (instrument_id, depth=10))]
    pub fn subscribe_book(&mut self, instrument_id: PyInstrumentId, depth: u32) {
        self.subscriptions
            .push((instrument_id, PySubscriptionType::OrderBook, Some(depth)));
    }

    pub fn get_position(&self, instrument_id: &PyInstrumentId) -> Option<PyPosition> {
        self.positions.get(&instrument_id.__str__()).cloned()
    }

    pub fn get_order(&self, order_id: &str) -> Option<PyOrder> {
        self.orders.get(order_id).cloned()
    }

    pub fn active_orders(&self) -> Vec<PyOrder> {
        self.orders
            .values()
            .filter(|o| o.is_active())
            .cloned()
            .collect()
    }

    pub fn open_positions(&self) -> Vec<PyPosition> {
        self.positions
            .values()
            .filter(|p| !p.is_flat())
            .cloned()
            .collect()
    }

    pub fn has_position(&self, instrument_id: &PyInstrumentId) -> bool {
        self.positions
            .get(&instrument_id.__str__())
            .map(|p| !p.is_flat())
            .unwrap_or(false)
    }

    pub fn total_unrealized_pnl(&self) -> f64 {
        self.positions.values().map(|p| p.unrealized_pnl).sum()
    }

    pub fn total_realized_pnl(&self) -> f64 {
        self.positions.values().map(|p| p.realized_pnl).sum()
    }

    pub fn drain_order_requests(&mut self) -> Vec<PyOrderRequest> {
        std::mem::take(&mut self.order_requests)
    }
}

impl PyStrategyContext {
    pub fn drain_orders(&mut self) -> Vec<PyOrderRequest> {
        std::mem::take(&mut self.order_requests)
    }

    pub fn drain_cancels(&mut self) -> Vec<String> {
        std::mem::take(&mut self.cancel_requests)
    }

    pub fn drain_subscriptions(
        &mut self,
    ) -> Vec<(PyInstrumentId, PySubscriptionType, Option<u32>)> {
        std::mem::take(&mut self.subscriptions)
    }

    pub fn update_position(&mut self, position: PyPosition) {
        self.positions
            .insert(position.instrument_id.__str__(), position);
    }

    pub fn update_order(&mut self, order: PyOrder) {
        self.orders.insert(order.order_id.clone(), order);
    }

    pub fn set_timestamp(&mut self, ts: u64) {
        self.timestamp_ns = ts;
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyBacktestResults {
    #[pyo3(get)]
    pub initial_balance: f64,
    #[pyo3(get)]
    pub final_balance: f64,
    #[pyo3(get)]
    pub total_pnl: f64,
    #[pyo3(get)]
    pub return_pct: f64,
    #[pyo3(get)]
    pub total_trades: u64,
    #[pyo3(get)]
    pub winning_trades: u64,
    #[pyo3(get)]
    pub losing_trades: u64,
    #[pyo3(get)]
    pub total_volume: f64,
    #[pyo3(get)]
    pub total_commission: f64,
    #[pyo3(get)]
    pub max_drawdown_pct: f64,
    #[pyo3(get)]
    pub sharpe_ratio: f64,
    #[pyo3(get)]
    pub sortino_ratio: f64,
    #[pyo3(get)]
    pub calmar_ratio: f64,
    #[pyo3(get)]
    pub start_time_ns: u64,
    #[pyo3(get)]
    pub end_time_ns: u64,
    #[pyo3(get)]
    pub avg_trade_pnl: f64,
    #[pyo3(get)]
    pub max_consecutive_wins: u32,
    #[pyo3(get)]
    pub max_consecutive_losses: u32,
}

#[pymethods]
impl PyBacktestResults {
    pub fn win_rate(&self) -> f64 {
        if self.total_trades == 0 {
            0.0
        } else {
            self.winning_trades as f64 / self.total_trades as f64 * 100.0
        }
    }

    pub fn profit_factor(&self) -> f64 {
        if self.losing_trades == 0 {
            f64::INFINITY
        } else {
            self.winning_trades as f64 / self.losing_trades as f64
        }
    }

    pub fn expectancy(&self) -> f64 {
        if self.total_trades == 0 {
            0.0
        } else {
            self.total_pnl / self.total_trades as f64
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "Backtest Results:\n  PnL: ${:.2} ({:.2}%)\n  Trades: {} (Win Rate: {:.1}%)\n  Sharpe: {:.2}\n  Max DD: {:.2}%",
            self.total_pnl, self.return_pct, self.total_trades, self.win_rate(), self.sharpe_ratio, self.max_drawdown_pct
        )
    }
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyNetwork {
    Mainnet,
    Testnet,
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyFillModel {
    Immediate,
    NextTick,
    Probabilistic,
    OrderBook,
}

#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyLatencyModel {
    Zero,
    Fixed,
    Uniform,
    LogNormal,
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyHyperliquidConfig {
    #[pyo3(get, set)]
    pub network: PyNetwork,
    #[pyo3(get, set)]
    pub account_address: Option<String>,
    #[pyo3(get, set)]
    pub private_key: Option<String>,
    #[pyo3(get, set)]
    pub vault_address: Option<String>,
}

#[pymethods]
impl PyHyperliquidConfig {
    #[new]
    #[pyo3(signature = (network=PyNetwork::Mainnet, account_address=None, private_key=None, vault_address=None))]
    pub fn new(
        network: PyNetwork,
        account_address: Option<String>,
        private_key: Option<String>,
        vault_address: Option<String>,
    ) -> Self {
        Self {
            network,
            account_address,
            private_key,
            vault_address,
        }
    }

    #[staticmethod]
    pub fn mainnet() -> Self {
        Self {
            network: PyNetwork::Mainnet,
            account_address: None,
            private_key: None,
            vault_address: None,
        }
    }

    #[staticmethod]
    pub fn testnet() -> Self {
        Self {
            network: PyNetwork::Testnet,
            account_address: None,
            private_key: None,
            vault_address: None,
        }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyLighterConfig {
    #[pyo3(get, set)]
    pub network: PyNetwork,
    #[pyo3(get, set)]
    pub api_key: Option<String>,
    #[pyo3(get, set)]
    pub api_secret: Option<String>,
}

#[pymethods]
impl PyLighterConfig {
    #[new]
    #[pyo3(signature = (network=PyNetwork::Mainnet, api_key=None, api_secret=None))]
    pub fn new(network: PyNetwork, api_key: Option<String>, api_secret: Option<String>) -> Self {
        Self {
            network,
            api_key,
            api_secret,
        }
    }

    #[staticmethod]
    pub fn mainnet() -> Self {
        Self {
            network: PyNetwork::Mainnet,
            api_key: None,
            api_secret: None,
        }
    }

    #[staticmethod]
    pub fn testnet() -> Self {
        Self {
            network: PyNetwork::Testnet,
            api_key: None,
            api_secret: None,
        }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyBacktestConfig {
    #[pyo3(get, set)]
    pub start_time_ns: u64,
    #[pyo3(get, set)]
    pub end_time_ns: u64,
    #[pyo3(get, set)]
    pub initial_balance: f64,
    #[pyo3(get, set)]
    pub commission_rate: f64,
    #[pyo3(get, set)]
    pub slippage_bps: f64,
    #[pyo3(get, set)]
    pub fill_model: PyFillModel,
    #[pyo3(get, set)]
    pub latency_model: PyLatencyModel,
    #[pyo3(get, set)]
    pub latency_ms: f64,
}

#[pymethods]
impl PyBacktestConfig {
    #[new]
    #[pyo3(signature = (
        start_time_ns=0,
        end_time_ns=u64::MAX,
        initial_balance=100000.0,
        commission_rate=0.0004,
        slippage_bps=10.0,
        fill_model=PyFillModel::Immediate,
        latency_model=PyLatencyModel::Zero,
        latency_ms=0.0
    ))]
    pub fn new(
        start_time_ns: u64,
        end_time_ns: u64,
        initial_balance: f64,
        commission_rate: f64,
        slippage_bps: f64,
        fill_model: PyFillModel,
        latency_model: PyLatencyModel,
        latency_ms: f64,
    ) -> Self {
        Self {
            start_time_ns,
            end_time_ns,
            initial_balance,
            commission_rate,
            slippage_bps,
            fill_model,
            latency_model,
            latency_ms,
        }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyRiskConfig {
    #[pyo3(get, set)]
    pub max_position_size: f64,
    #[pyo3(get, set)]
    pub max_order_size: f64,
    #[pyo3(get, set)]
    pub max_daily_loss: f64,
    #[pyo3(get, set)]
    pub max_open_orders: u32,
    #[pyo3(get, set)]
    pub max_positions: u32,
    #[pyo3(get, set)]
    pub max_leverage: f64,
}

#[pymethods]
impl PyRiskConfig {
    #[new]
    #[pyo3(signature = (
        max_position_size=1000000.0,
        max_order_size=100000.0,
        max_daily_loss=10000.0,
        max_open_orders=100,
        max_positions=50,
        max_leverage=10.0
    ))]
    pub fn new(
        max_position_size: f64,
        max_order_size: f64,
        max_daily_loss: f64,
        max_open_orders: u32,
        max_positions: u32,
        max_leverage: f64,
    ) -> Self {
        Self {
            max_position_size,
            max_order_size,
            max_daily_loss,
            max_open_orders,
            max_positions,
            max_leverage,
        }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyTwapParams {
    #[pyo3(get, set)]
    pub instrument_id: PyInstrumentId,
    #[pyo3(get, set)]
    pub side: PyOrderSide,
    #[pyo3(get, set)]
    pub total_quantity: f64,
    #[pyo3(get, set)]
    pub duration_secs: u64,
    #[pyo3(get, set)]
    pub num_slices: u32,
    #[pyo3(get, set)]
    pub randomize_timing: bool,
    #[pyo3(get, set)]
    pub limit_price: Option<f64>,
}

#[pymethods]
impl PyTwapParams {
    #[new]
    #[pyo3(signature = (instrument_id, side, total_quantity, duration_secs, num_slices=10, randomize_timing=true, limit_price=None))]
    pub fn new(
        instrument_id: PyInstrumentId,
        side: PyOrderSide,
        total_quantity: f64,
        duration_secs: u64,
        num_slices: u32,
        randomize_timing: bool,
        limit_price: Option<f64>,
    ) -> Self {
        Self {
            instrument_id,
            side,
            total_quantity,
            duration_secs,
            num_slices,
            randomize_timing,
            limit_price,
        }
    }

    pub fn slice_quantity(&self) -> f64 {
        self.total_quantity / self.num_slices as f64
    }

    pub fn slice_interval_secs(&self) -> f64 {
        self.duration_secs as f64 / self.num_slices as f64
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyVwapParams {
    #[pyo3(get, set)]
    pub instrument_id: PyInstrumentId,
    #[pyo3(get, set)]
    pub side: PyOrderSide,
    #[pyo3(get, set)]
    pub total_quantity: f64,
    #[pyo3(get, set)]
    pub participation_rate: f64,
    #[pyo3(get, set)]
    pub min_slice: f64,
    #[pyo3(get, set)]
    pub max_slice: f64,
    #[pyo3(get, set)]
    pub limit_price: Option<f64>,
}

#[pymethods]
impl PyVwapParams {
    #[new]
    #[pyo3(signature = (instrument_id, side, total_quantity, participation_rate=0.1, min_slice=0.0, max_slice=1000000.0, limit_price=None))]
    pub fn new(
        instrument_id: PyInstrumentId,
        side: PyOrderSide,
        total_quantity: f64,
        participation_rate: f64,
        min_slice: f64,
        max_slice: f64,
        limit_price: Option<f64>,
    ) -> Self {
        Self {
            instrument_id,
            side,
            total_quantity,
            participation_rate,
            min_slice,
            max_slice,
            limit_price,
        }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyIcebergParams {
    #[pyo3(get, set)]
    pub instrument_id: PyInstrumentId,
    #[pyo3(get, set)]
    pub side: PyOrderSide,
    #[pyo3(get, set)]
    pub total_quantity: f64,
    #[pyo3(get, set)]
    pub display_quantity: f64,
    #[pyo3(get, set)]
    pub limit_price: f64,
    #[pyo3(get, set)]
    pub variance_pct: f64,
}

#[pymethods]
impl PyIcebergParams {
    #[new]
    #[pyo3(signature = (instrument_id, side, total_quantity, display_quantity, limit_price, variance_pct=0.1))]
    pub fn new(
        instrument_id: PyInstrumentId,
        side: PyOrderSide,
        total_quantity: f64,
        display_quantity: f64,
        limit_price: f64,
        variance_pct: f64,
    ) -> Self {
        Self {
            instrument_id,
            side,
            total_quantity,
            display_quantity,
            limit_price,
            variance_pct,
        }
    }
}

#[pymodule]
fn neleus_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(version, m)?)?;

    m.add_class::<PyVenue>()?;
    m.add_class::<PyInstrumentType>()?;
    m.add_class::<PyOrderSide>()?;
    m.add_class::<PyOrderType>()?;
    m.add_class::<PyTimeInForce>()?;
    m.add_class::<PyOrderState>()?;
    m.add_class::<PyPositionSide>()?;
    m.add_class::<PySubscriptionType>()?;
    m.add_class::<PyNetwork>()?;
    m.add_class::<PyFillModel>()?;
    m.add_class::<PyLatencyModel>()?;

    m.add_class::<PyInstrumentId>()?;
    m.add_class::<PyTradeTick>()?;
    m.add_class::<PyQuoteTick>()?;
    m.add_class::<PyBookLevel>()?;
    m.add_class::<PyOrderBook>()?;
    m.add_class::<PyBar>()?;

    m.add_class::<PyOrder>()?;
    m.add_class::<PyFill>()?;
    m.add_class::<PyPosition>()?;
    m.add_class::<PyOrderRequest>()?;

    m.add_class::<PyStrategyContext>()?;
    m.add_class::<PyBacktestResults>()?;

    m.add_class::<PyHyperliquidConfig>()?;
    m.add_class::<PyLighterConfig>()?;
    m.add_class::<PyBacktestConfig>()?;
    m.add_class::<PyRiskConfig>()?;

    m.add_class::<PyTwapParams>()?;
    m.add_class::<PyVwapParams>()?;
    m.add_class::<PyIcebergParams>()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instrument_id_conversion() {
        let py_id = PyInstrumentId::new(
            PyVenue::Hyperliquid,
            "BTC".to_string(),
            PyInstrumentType::Perp,
        );
        let rust_id = py_id.to_rust();
        assert_eq!(rust_id.venue, Venue::Hyperliquid);
        assert_eq!(rust_id.symbol, "BTC");
    }

    #[test]
    fn test_strategy_context() {
        let mut ctx = PyStrategyContext::new(1000);

        ctx.market_order(
            PyInstrumentId::new(
                PyVenue::Simulated,
                "ETH".to_string(),
                PyInstrumentType::Perp,
            ),
            PyOrderSide::Buy,
            1.0,
            false,
        );

        let orders = ctx.drain_orders();
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].quantity, 1.0);
    }

    #[test]
    fn test_position_pnl() {
        let pos = PyPosition::new(
            PyInstrumentId::new(
                PyVenue::Simulated,
                "BTC".to_string(),
                PyInstrumentType::Perp,
            ),
            PyPositionSide::Long,
            1.0,
            50000.0,
            100.0,
            500.0,
        );

        assert_eq!(pos.total_pnl(), 600.0);
        assert!(!pos.is_flat());
        assert!(pos.is_long());
    }

    #[test]
    fn test_backtest_results() {
        let results = PyBacktestResults {
            initial_balance: 100000.0,
            final_balance: 110000.0,
            total_pnl: 10000.0,
            return_pct: 10.0,
            total_trades: 100,
            winning_trades: 60,
            losing_trades: 40,
            total_volume: 1000000.0,
            total_commission: 400.0,
            max_drawdown_pct: 5.0,
            sharpe_ratio: 2.0,
            sortino_ratio: 2.5,
            calmar_ratio: 2.0,
            start_time_ns: 0,
            end_time_ns: 1000000000,
            avg_trade_pnl: 100.0,
            max_consecutive_wins: 10,
            max_consecutive_losses: 5,
        };

        assert_eq!(results.win_rate(), 60.0);
        assert!(results.profit_factor() > 1.0);
    }

    #[test]
    fn test_twap_params() {
        let twap = PyTwapParams::new(
            PyInstrumentId::new(
                PyVenue::Hyperliquid,
                "BTC".to_string(),
                PyInstrumentType::Perp,
            ),
            PyOrderSide::Buy,
            10.0,
            600,
            10,
            true,
            None,
        );

        assert_eq!(twap.slice_quantity(), 1.0);
        assert_eq!(twap.slice_interval_secs(), 60.0);
    }
}
