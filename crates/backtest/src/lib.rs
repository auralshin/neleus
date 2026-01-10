use neleus_core_bus::InMemoryBus;
use neleus_core_engine::{
    ClockMode, Engine, EngineConfig, MarketDataEvent, OrderSide, OrderType, StrategyCommand,
    StrategyHandler, TradingEvent,
};
use neleus_core_types::{InstrumentId, InstrumentType, OrderId, UnixNanos, Venue};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

pub use csv;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestConfig {
    pub start_time: UnixNanos,

    pub end_time: UnixNanos,

    pub sim_mode: SimulationMode,

    pub fill_model: FillModelConfig,

    pub latency_model: LatencyModelConfig,

    pub initial_balance: f64,

    pub commission_rate: f64,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            start_time: UnixNanos::ZERO,
            end_time: UnixNanos::from_millis(u64::MAX / 1_000_000),
            sim_mode: SimulationMode::TradeBased,
            fill_model: FillModelConfig::default(),
            latency_model: LatencyModelConfig::default(),
            initial_balance: 100_000.0,
            commission_rate: 0.0004,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimulationMode {
    BarBased,

    TradeBased,

    OrderBookBased,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FillModelType {
    Immediate,

    NextTick,

    Probabilistic,

    OrderBook,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillModelConfig {
    pub model_type: FillModelType,

    pub slippage_bps: u32,

    pub partial_fills: bool,

    pub max_fill_rate: f64,

    pub fill_probability: f64,
}

impl Default for FillModelConfig {
    fn default() -> Self {
        Self {
            model_type: FillModelType::Immediate,
            slippage_bps: 10,
            partial_fills: false,
            max_fill_rate: 1.0,
            fill_probability: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LatencyModelType {
    Zero,

    Fixed,

    Uniform,

    LogNormal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyModelConfig {
    pub model_type: LatencyModelType,

    pub order_latency_ns: u64,

    pub data_latency_ns: u64,

    pub enable_jitter: bool,

    pub jitter_ns: u64,

    pub min_latency_ns: u64,

    pub max_latency_ns: u64,
}

impl Default for LatencyModelConfig {
    fn default() -> Self {
        Self {
            model_type: LatencyModelType::Zero,
            order_latency_ns: 1_000_000,
            data_latency_ns: 500_000,
            enable_jitter: false,
            jitter_ns: 100_000,
            min_latency_ns: 500_000,
            max_latency_ns: 2_000_000,
        }
    }
}

pub struct LatencySimulator {
    config: LatencyModelConfig,
    rng: rand::rngs::ThreadRng,
}

impl LatencySimulator {
    pub fn new(config: LatencyModelConfig) -> Self {
        Self {
            config,
            rng: rand::thread_rng(),
        }
    }

    pub fn order_latency(&mut self) -> u64 {
        self.simulate_latency(self.config.order_latency_ns)
    }

    pub fn data_latency(&mut self) -> u64 {
        self.simulate_latency(self.config.data_latency_ns)
    }

    fn simulate_latency(&mut self, base: u64) -> u64 {
        match self.config.model_type {
            LatencyModelType::Zero => 0,
            LatencyModelType::Fixed => {
                if self.config.enable_jitter {
                    let jitter = self.rng.gen_range(0..self.config.jitter_ns);
                    base + jitter
                } else {
                    base
                }
            }
            LatencyModelType::Uniform => self
                .rng
                .gen_range(self.config.min_latency_ns..=self.config.max_latency_ns),
            LatencyModelType::LogNormal => {
                let u: f64 = self.rng.gen();
                let factor = (-2.0 * u.ln()).sqrt()
                    * (2.0 * std::f64::consts::PI * self.rng.gen::<f64>()).cos();
                let latency = base as f64 * (1.0 + 0.3 * factor).max(0.1);
                latency as u64
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalDataPoint {
    pub timestamp: UnixNanos,
    pub sequence: u64,
    pub data: HistoricalData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HistoricalData {
    Trade {
        instrument_id: InstrumentId,
        price: f64,
        quantity: f64,
        side: OrderSide,
    },
    Quote {
        instrument_id: InstrumentId,
        bid_price: f64,
        bid_size: f64,
        ask_price: f64,
        ask_size: f64,
    },
    Bar {
        instrument_id: InstrumentId,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    },
    BookSnapshot {
        instrument_id: InstrumentId,
        bids: Vec<(f64, f64)>,
        asks: Vec<(f64, f64)>,
    },
    BookDelta {
        instrument_id: InstrumentId,
        side: OrderSide,
        price: f64,
        quantity: f64,
        is_delete: bool,
    },
}

impl Ord for HistoricalDataPoint {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .timestamp
            .cmp(&self.timestamp)
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

impl PartialOrd for HistoricalDataPoint {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for HistoricalDataPoint {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp && self.sequence == other.sequence
    }
}

impl Eq for HistoricalDataPoint {}

pub trait DataFeed {
    fn next(&mut self) -> Option<HistoricalDataPoint>;

    fn peek_timestamp(&self) -> Option<UnixNanos>;

    fn reset(&mut self);
}

pub struct InMemoryDataFeed {
    data: Vec<HistoricalDataPoint>,
    index: usize,
}

impl InMemoryDataFeed {
    pub fn new(mut data: Vec<HistoricalDataPoint>) -> Self {
        data.sort_by(|a, b| {
            a.timestamp
                .cmp(&b.timestamp)
                .then_with(|| a.sequence.cmp(&b.sequence))
        });
        Self { data, index: 0 }
    }

    pub fn from_trades(trades: Vec<(u64, &str, f64, f64, bool)>) -> Self {
        let data: Vec<_> = trades
            .into_iter()
            .enumerate()
            .map(
                |(i, (ts_ms, symbol, price, qty, is_buy))| HistoricalDataPoint {
                    timestamp: UnixNanos::from_millis(ts_ms),
                    sequence: i as u64,
                    data: HistoricalData::Trade {
                        instrument_id: InstrumentId::new(
                            Venue::Simulated,
                            symbol,
                            InstrumentType::Perp,
                        ),
                        price,
                        quantity: qty,
                        side: if is_buy {
                            OrderSide::Buy
                        } else {
                            OrderSide::Sell
                        },
                    },
                },
            )
            .collect();
        Self::new(data)
    }
}

impl DataFeed for InMemoryDataFeed {
    fn next(&mut self) -> Option<HistoricalDataPoint> {
        if self.index < self.data.len() {
            let point = self.data[self.index].clone();
            self.index += 1;
            Some(point)
        } else {
            None
        }
    }

    fn peek_timestamp(&self) -> Option<UnixNanos> {
        self.data.get(self.index).map(|p| p.timestamp)
    }

    fn reset(&mut self) {
        self.index = 0;
    }
}

pub struct CsvDataFeed {
    inner: InMemoryDataFeed,
}

impl CsvDataFeed {
    pub fn from_trades_file(
        path: &std::path::Path,
        venue: Venue,
        instrument_type: InstrumentType,
    ) -> Result<Self, CsvError> {
        use std::fs::File;
        use std::io::BufReader;

        let file = File::open(path).map_err(|e| CsvError::IoError(e.to_string()))?;
        let mut reader = csv::Reader::from_reader(BufReader::new(file));

        let mut data = Vec::new();
        let mut seq = 0u64;

        for result in reader.records() {
            let record = result.map_err(|e| CsvError::ParseError(e.to_string()))?;

            let timestamp_ms: u64 = record
                .get(0)
                .ok_or(CsvError::MissingColumn("timestamp_ms".into()))?
                .parse()
                .map_err(|_| CsvError::ParseError("invalid timestamp".into()))?;

            let symbol = record
                .get(1)
                .ok_or(CsvError::MissingColumn("symbol".into()))?;

            let price: f64 = record
                .get(2)
                .ok_or(CsvError::MissingColumn("price".into()))?
                .parse()
                .map_err(|_| CsvError::ParseError("invalid price".into()))?;

            let quantity: f64 = record
                .get(3)
                .ok_or(CsvError::MissingColumn("quantity".into()))?
                .parse()
                .map_err(|_| CsvError::ParseError("invalid quantity".into()))?;

            let side_str = record
                .get(4)
                .ok_or(CsvError::MissingColumn("side".into()))?;

            let side = match side_str.to_lowercase().as_str() {
                "buy" | "b" | "1" => OrderSide::Buy,
                "sell" | "s" | "0" | "-1" => OrderSide::Sell,
                _ => return Err(CsvError::ParseError(format!("invalid side: {}", side_str))),
            };

            data.push(HistoricalDataPoint {
                timestamp: UnixNanos::from_millis(timestamp_ms),
                sequence: seq,
                data: HistoricalData::Trade {
                    instrument_id: InstrumentId::new(venue, symbol, instrument_type),
                    price,
                    quantity,
                    side,
                },
            });
            seq += 1;
        }

        Ok(Self {
            inner: InMemoryDataFeed::new(data),
        })
    }

    pub fn from_bars_file(
        path: &std::path::Path,
        venue: Venue,
        instrument_type: InstrumentType,
    ) -> Result<Self, CsvError> {
        use std::fs::File;
        use std::io::BufReader;

        let file = File::open(path).map_err(|e| CsvError::IoError(e.to_string()))?;
        let mut reader = csv::Reader::from_reader(BufReader::new(file));

        let mut data = Vec::new();
        let mut seq = 0u64;

        for result in reader.records() {
            let record = result.map_err(|e| CsvError::ParseError(e.to_string()))?;

            let timestamp_ms: u64 = record
                .get(0)
                .ok_or(CsvError::MissingColumn("timestamp_ms".into()))?
                .parse()
                .map_err(|_| CsvError::ParseError("invalid timestamp".into()))?;

            let symbol = record
                .get(1)
                .ok_or(CsvError::MissingColumn("symbol".into()))?;

            let open: f64 = record
                .get(2)
                .ok_or(CsvError::MissingColumn("open".into()))?
                .parse()
                .map_err(|_| CsvError::ParseError("invalid open".into()))?;

            let high: f64 = record
                .get(3)
                .ok_or(CsvError::MissingColumn("high".into()))?
                .parse()
                .map_err(|_| CsvError::ParseError("invalid high".into()))?;

            let low: f64 = record
                .get(4)
                .ok_or(CsvError::MissingColumn("low".into()))?
                .parse()
                .map_err(|_| CsvError::ParseError("invalid low".into()))?;

            let close: f64 = record
                .get(5)
                .ok_or(CsvError::MissingColumn("close".into()))?
                .parse()
                .map_err(|_| CsvError::ParseError("invalid close".into()))?;

            let volume: f64 = record
                .get(6)
                .ok_or(CsvError::MissingColumn("volume".into()))?
                .parse()
                .map_err(|_| CsvError::ParseError("invalid volume".into()))?;

            data.push(HistoricalDataPoint {
                timestamp: UnixNanos::from_millis(timestamp_ms),
                sequence: seq,
                data: HistoricalData::Bar {
                    instrument_id: InstrumentId::new(venue, symbol, instrument_type),
                    open,
                    high,
                    low,
                    close,
                    volume,
                },
            });
            seq += 1;
        }

        Ok(Self {
            inner: InMemoryDataFeed::new(data),
        })
    }

    pub fn from_jsonl_file(path: &std::path::Path) -> Result<Self, CsvError> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        let file = File::open(path).map_err(|e| CsvError::IoError(e.to_string()))?;
        let reader = BufReader::new(file);

        let mut data = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(|e| CsvError::IoError(e.to_string()))?;
            if line.trim().is_empty() {
                continue;
            }
            let point: HistoricalDataPoint =
                serde_json::from_str(&line).map_err(|e| CsvError::ParseError(e.to_string()))?;
            data.push(point);
        }

        Ok(Self {
            inner: InMemoryDataFeed::new(data),
        })
    }
}

impl DataFeed for CsvDataFeed {
    fn next(&mut self) -> Option<HistoricalDataPoint> {
        self.inner.next()
    }

    fn peek_timestamp(&self) -> Option<UnixNanos> {
        self.inner.peek_timestamp()
    }

    fn reset(&mut self) {
        self.inner.reset()
    }
}

#[derive(Debug, Clone)]
pub enum CsvError {
    IoError(String),
    ParseError(String),
    MissingColumn(String),
}

#[cfg(feature = "hyperliquid")]
pub mod hyperliquid_feed {
    use super::*;
    use neleus_adapters_hyperliquid::{
        CandleInterval, HyperliquidConfig, HyperliquidDataFeed as HlDataFeed, HyperliquidError,
    };

    pub struct HyperliquidDataFeed {
        inner: HlDataFeed,
        data_cache: Vec<HistoricalDataPoint>,
        index: usize,
    }

    impl HyperliquidDataFeed {
        pub fn new(coin: &str, interval: CandleInterval) -> Self {
            Self {
                inner: HlDataFeed::new(coin.to_string(), interval),
                data_cache: Vec::new(),
                index: 0,
            }
        }

        pub async fn load(
            &mut self,
            config: &HyperliquidConfig,
            start_time_ms: u64,
            end_time_ms: u64,
        ) -> Result<usize, HyperliquidError> {
            let count = self.inner.load(config, start_time_ms, end_time_ms).await?;

            self.data_cache = self
                .inner
                .data()
                .iter()
                .enumerate()
                .map(|(seq, point)| HistoricalDataPoint {
                    timestamp: UnixNanos::from_millis(point.timestamp_ms),
                    sequence: seq as u64,
                    data: HistoricalData::Bar {
                        instrument_id: InstrumentId::new(
                            Venue::Hyperliquid,
                            self.inner.coin(),
                            InstrumentType::Perp,
                        ),
                        open: point.open,
                        high: point.high,
                        low: point.low,
                        close: point.close,
                        volume: point.volume,
                    },
                })
                .collect();

            self.index = 0;
            Ok(count)
        }

        pub fn raw_data(&self) -> &[neleus_adapters_hyperliquid::HyperliquidDataPoint] {
            self.inner.data()
        }

        pub fn len(&self) -> usize {
            self.data_cache.len()
        }

        pub fn is_empty(&self) -> bool {
            self.data_cache.is_empty()
        }
    }

    impl DataFeed for HyperliquidDataFeed {
        fn next(&mut self) -> Option<HistoricalDataPoint> {
            if self.index < self.data_cache.len() {
                let point = self.data_cache[self.index].clone();
                self.index += 1;
                Some(point)
            } else {
                None
            }
        }

        fn peek_timestamp(&self) -> Option<UnixNanos> {
            self.data_cache.get(self.index).map(|p| p.timestamp)
        }

        fn reset(&mut self) {
            self.index = 0;
        }
    }

    pub struct HyperliquidBacktestBuilder {
        coin: String,
        interval: CandleInterval,
        config: HyperliquidConfig,
        start_time_ms: Option<u64>,
        end_time_ms: Option<u64>,
        initial_balance: f64,
        commission_rate: f64,
    }

    impl HyperliquidBacktestBuilder {
        pub fn new(coin: &str) -> Self {
            Self {
                coin: coin.to_string(),
                interval: CandleInterval::Hour1,
                config: HyperliquidConfig::mainnet(),
                start_time_ms: None,
                end_time_ms: None,
                initial_balance: 100_000.0,
                commission_rate: 0.0004,
            }
        }

        pub fn testnet(mut self) -> Self {
            self.config = HyperliquidConfig::testnet();
            self
        }

        pub fn interval(mut self, interval: CandleInterval) -> Self {
            self.interval = interval;
            self
        }

        pub fn time_range(mut self, start_ms: u64, end_ms: u64) -> Self {
            self.start_time_ms = Some(start_ms);
            self.end_time_ms = Some(end_ms);
            self
        }

        pub fn last_days(mut self, days: u64) -> Self {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            self.end_time_ms = Some(now);
            self.start_time_ms = Some(now - (days * 24 * 60 * 60 * 1000));
            self
        }

        pub fn initial_balance(mut self, balance: f64) -> Self {
            self.initial_balance = balance;
            self
        }

        pub fn commission_rate(mut self, rate: f64) -> Self {
            self.commission_rate = rate;
            self
        }

        pub async fn build(
            self,
        ) -> Result<(HyperliquidDataFeed, BacktestConfig), HyperliquidError> {
            let start = self.start_time_ms.ok_or_else(|| {
                HyperliquidError::RequestError("start_time_ms not set".to_string())
            })?;
            let end = self
                .end_time_ms
                .ok_or_else(|| HyperliquidError::RequestError("end_time_ms not set".to_string()))?;

            let mut feed = HyperliquidDataFeed::new(&self.coin, self.interval);
            feed.load(&self.config, start, end).await?;

            let config = BacktestConfig {
                start_time: UnixNanos::from_millis(start),
                end_time: UnixNanos::from_millis(end),
                sim_mode: SimulationMode::BarBased,
                fill_model: FillModelConfig::default(),
                latency_model: LatencyModelConfig::default(),
                initial_balance: self.initial_balance,
                commission_rate: self.commission_rate,
            };

            Ok((feed, config))
        }
    }
}

#[cfg(feature = "hyperliquid")]
pub use hyperliquid_feed::{HyperliquidBacktestBuilder, HyperliquidDataFeed};

#[derive(Debug, Clone)]
pub struct SimulatedBook {
    pub instrument_id: InstrumentId,

    bids: Vec<(f64, f64)>,

    asks: Vec<(f64, f64)>,

    last_price: Option<f64>,

    last_update: UnixNanos,
}

impl SimulatedBook {
    pub fn new(instrument_id: InstrumentId) -> Self {
        Self {
            instrument_id,
            bids: Vec::new(),
            asks: Vec::new(),
            last_price: None,
            last_update: UnixNanos::ZERO,
        }
    }

    pub fn best_bid(&self) -> Option<f64> {
        self.bids.first().map(|(p, _)| *p)
    }

    pub fn best_ask(&self) -> Option<f64> {
        self.asks.first().map(|(p, _)| *p)
    }

    pub fn mid_price(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some((bid + ask) / 2.0),
            _ => self.last_price,
        }
    }

    pub fn spread(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(ask - bid),
            _ => None,
        }
    }

    pub fn update_snapshot(&mut self, bids: Vec<(f64, f64)>, asks: Vec<(f64, f64)>, ts: UnixNanos) {
        self.bids = bids;
        self.asks = asks;
        self.bids.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        self.asks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        self.last_update = ts;
    }

    pub fn update_trade(&mut self, price: f64, _quantity: f64, ts: UnixNanos) {
        self.last_price = Some(price);
        self.last_update = ts;
    }

    pub fn apply_delta(
        &mut self,
        side: OrderSide,
        price: f64,
        quantity: f64,
        is_delete: bool,
        ts: UnixNanos,
    ) {
        let book = match side {
            OrderSide::Buy => &mut self.bids,
            OrderSide::Sell => &mut self.asks,
        };

        if is_delete || quantity == 0.0 {
            book.retain(|(p, _)| (*p - price).abs() > 1e-10);
        } else {
            if let Some(level) = book.iter_mut().find(|(p, _)| (*p - price).abs() < 1e-10) {
                level.1 = quantity;
            } else {
                book.push((price, quantity));
            }
        }

        match side {
            OrderSide::Buy => self.bids.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap()),
            OrderSide::Sell => self.asks.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap()),
        }

        self.last_update = ts;
    }
}

#[derive(Debug, Clone)]
pub struct SimulatedOrder {
    pub order_id: OrderId,
    pub instrument_id: InstrumentId,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: Option<f64>,
    pub quantity: f64,
    pub filled_quantity: f64,
    pub submit_time: UnixNanos,
    pub accept_time: Option<UnixNanos>,
}

impl SimulatedOrder {
    pub fn remaining(&self) -> f64 {
        self.quantity - self.filled_quantity
    }

    pub fn is_filled(&self) -> bool {
        self.remaining() <= 1e-10
    }
}

pub struct SimulatedVenue {
    pub venue: Venue,

    books: HashMap<InstrumentId, SimulatedBook>,

    pending_orders: HashMap<OrderId, SimulatedOrder>,

    fill_config: FillModelConfig,

    commission_rate: f64,

    order_sequence: u64,

    pending_events: VecDeque<TradingEvent>,
}

impl SimulatedVenue {
    pub fn new(venue: Venue, fill_config: FillModelConfig, commission_rate: f64) -> Self {
        Self {
            venue,
            books: HashMap::new(),
            pending_orders: HashMap::new(),
            fill_config,
            commission_rate,
            order_sequence: 0,
            pending_events: VecDeque::new(),
        }
    }

    pub fn get_or_create_book(&mut self, instrument_id: &InstrumentId) -> &mut SimulatedBook {
        if !self.books.contains_key(instrument_id) {
            self.books.insert(
                instrument_id.clone(),
                SimulatedBook::new(instrument_id.clone()),
            );
        }
        self.books.get_mut(instrument_id).unwrap()
    }

    pub fn on_data(&mut self, data: &HistoricalData, ts: UnixNanos) {
        match data {
            HistoricalData::Trade {
                instrument_id,
                price,
                quantity,
                ..
            } => {
                let book = self.get_or_create_book(instrument_id);
                book.update_trade(*price, *quantity, ts);
                self.try_fill_orders(instrument_id, *price, ts);
            }
            HistoricalData::Quote {
                instrument_id,
                bid_price,
                bid_size,
                ask_price,
                ask_size,
            } => {
                let book = self.get_or_create_book(instrument_id);
                book.update_snapshot(
                    vec![(*bid_price, *bid_size)],
                    vec![(*ask_price, *ask_size)],
                    ts,
                );
            }
            HistoricalData::BookSnapshot {
                instrument_id,
                bids,
                asks,
            } => {
                let book = self.get_or_create_book(instrument_id);
                book.update_snapshot(bids.clone(), asks.clone(), ts);
            }
            HistoricalData::BookDelta {
                instrument_id,
                side,
                price,
                quantity,
                is_delete,
            } => {
                let book = self.get_or_create_book(instrument_id);
                book.apply_delta(*side, *price, *quantity, *is_delete, ts);
            }
            HistoricalData::Bar {
                instrument_id,
                close,
                ..
            } => {
                let book = self.get_or_create_book(instrument_id);
                book.update_trade(*close, 0.0, ts);
                self.try_fill_orders(instrument_id, *close, ts);
            }
        }
    }

    pub fn submit_order(&mut self, cmd: &StrategyCommand, ts: UnixNanos) {
        if let StrategyCommand::SubmitOrder {
            order_id,
            instrument_id,
            side,
            order_type,
            price,
            quantity,
        } = cmd
        {
            self.order_sequence += 1;

            let order = SimulatedOrder {
                order_id: order_id.clone(),
                instrument_id: instrument_id.clone(),
                side: *side,
                order_type: *order_type,
                price: *price,
                quantity: *quantity,
                filled_quantity: 0.0,
                submit_time: ts,
                accept_time: None,
            };

            self.pending_events.push_back(TradingEvent::OrderSubmitted {
                order_id: order_id.clone(),
                ts,
            });

            if *order_type == OrderType::Market {
                self.try_fill_market_order(order, ts);
            } else {
                let mut order = order;
                order.accept_time = Some(ts);
                self.pending_events.push_back(TradingEvent::OrderAccepted {
                    order_id: order.order_id.clone(),
                    venue_order_id: format!("SIM-{}", self.order_sequence),
                    ts,
                });
                self.pending_orders.insert(order.order_id.clone(), order);
            }
        }
    }

    pub fn cancel_order(&mut self, order_id: &OrderId, ts: UnixNanos) {
        if self.pending_orders.remove(order_id).is_some() {
            self.pending_events.push_back(TradingEvent::OrderCanceled {
                order_id: order_id.clone(),
                ts,
            });
        }
    }

    fn try_fill_market_order(&mut self, order: SimulatedOrder, ts: UnixNanos) {
        let book = match self.books.get(&order.instrument_id) {
            Some(b) => b,
            None => {
                self.pending_events.push_back(TradingEvent::OrderRejected {
                    order_id: order.order_id,
                    reason: "No market data".to_string(),
                    ts,
                });
                return;
            }
        };

        let base_price = match order.side {
            OrderSide::Buy => book.best_ask().or(book.last_price),
            OrderSide::Sell => book.best_bid().or(book.last_price),
        };

        let Some(price) = base_price else {
            self.pending_events.push_back(TradingEvent::OrderRejected {
                order_id: order.order_id,
                reason: "No price available".to_string(),
                ts,
            });
            return;
        };

        let slippage = self.fill_config.slippage_bps as f64 / 10_000.0;
        let fill_price = match order.side {
            OrderSide::Buy => price * (1.0 + slippage),
            OrderSide::Sell => price * (1.0 - slippage),
        };

        self.pending_events.push_back(TradingEvent::OrderFilled {
            order_id: order.order_id,
            fill_price,
            fill_quantity: order.quantity,
            remaining_quantity: 0.0,
            ts,
        });
    }

    fn try_fill_orders(&mut self, instrument_id: &InstrumentId, trade_price: f64, ts: UnixNanos) {
        let mut to_remove = Vec::new();

        for (order_id, order) in &mut self.pending_orders {
            if &order.instrument_id != instrument_id {
                continue;
            }

            let can_fill = match (order.side, order.price) {
                (OrderSide::Buy, Some(limit)) => trade_price <= limit,
                (OrderSide::Sell, Some(limit)) => trade_price >= limit,
                _ => false,
            };

            if can_fill {
                let fill_qty = order.remaining();
                order.filled_quantity += fill_qty;

                let slippage = self.fill_config.slippage_bps as f64 / 10_000.0;
                let fill_price = match order.side {
                    OrderSide::Buy => trade_price * (1.0 + slippage),
                    OrderSide::Sell => trade_price * (1.0 - slippage),
                };

                self.pending_events.push_back(TradingEvent::OrderFilled {
                    order_id: order_id.clone(),
                    fill_price,
                    fill_quantity: fill_qty,
                    remaining_quantity: order.remaining(),
                    ts,
                });

                if order.is_filled() {
                    to_remove.push(order_id.clone());
                }
            }
        }

        for order_id in to_remove {
            self.pending_orders.remove(&order_id);
        }
    }

    pub fn drain_events(&mut self) -> Vec<TradingEvent> {
        self.pending_events.drain(..).collect()
    }
}

pub struct BacktestNode {
    config: BacktestConfig,

    engine: Engine<InMemoryBus>,

    data_feed: Option<Box<dyn DataFeed>>,

    venue: SimulatedVenue,

    latency_sim: LatencySimulator,

    current_time: UnixNanos,

    data_count: u64,

    results: BacktestResults,

    current_equity: f64,

    positions: HashMap<InstrumentId, PositionTracker>,
}

#[derive(Debug, Clone, Default)]
struct PositionTracker {
    quantity: f64,
    avg_entry: f64,
    realized_pnl: f64,
}

impl BacktestNode {
    pub fn new(config: BacktestConfig) -> Self {
        let engine_config = EngineConfig {
            instance_id: "backtest".to_string(),
            max_events_per_tick: 100,
            enable_event_log: false,
            clock_mode: ClockMode::Simulated,
        };

        let latency_sim = LatencySimulator::new(config.latency_model.clone());
        let start_time = config.start_time;

        Self {
            venue: SimulatedVenue::new(
                Venue::Simulated,
                config.fill_model.clone(),
                config.commission_rate,
            ),
            engine: Engine::new(engine_config),
            data_feed: None,
            latency_sim,
            current_time: config.start_time,
            data_count: 0,
            results: BacktestResults::new(config.initial_balance, start_time),
            current_equity: config.initial_balance,
            positions: HashMap::new(),
            config,
        }
    }

    pub fn add_strategy(&mut self, strategy: Box<dyn StrategyHandler>) {
        self.engine.add_strategy(strategy);
    }

    pub fn set_data_feed(&mut self, feed: Box<dyn DataFeed>) {
        self.data_feed = Some(feed);
    }

    pub fn run(&mut self) -> &BacktestResults {
        self.engine.start();
        self.current_time = self.config.start_time;

        while let Some(data_point) = self.next_data_point() {
            if data_point.timestamp > self.config.end_time {
                break;
            }

            self.current_time = data_point.timestamp;
            self.engine.advance_time(self.current_time);

            self.venue.on_data(&data_point.data, self.current_time);

            if let Some(market_event) = self.to_market_event(&data_point.data, self.current_time) {
                let commands = self.engine.on_market_data(market_event);
                self.process_strategy_commands(commands);
            }

            let timer_commands = self.engine.tick_collect_commands();
            self.process_strategy_commands(timer_commands);

            for event in self.venue.drain_events() {
                self.process_trading_event(&event);
                let commands = self.engine.on_trading_event(&event);
                self.process_strategy_commands(commands);
            }

            self.data_count += 1;
        }

        self.engine.stop();
        self.finalize_results();

        &self.results
    }

    fn to_market_event(&self, data: &HistoricalData, ts: UnixNanos) -> Option<MarketDataEvent> {
        match data {
            HistoricalData::Trade {
                instrument_id,
                price,
                quantity,
                side,
            } => Some(MarketDataEvent::Trade {
                instrument_id: instrument_id.clone(),
                price: *price,
                quantity: *quantity,
                side: *side,
                ts,
            }),
            HistoricalData::Quote {
                instrument_id,
                bid_price,
                bid_size,
                ask_price,
                ask_size,
            } => Some(MarketDataEvent::Quote {
                instrument_id: instrument_id.clone(),
                bid_price: *bid_price,
                bid_size: *bid_size,
                ask_price: *ask_price,
                ask_size: *ask_size,
                ts,
            }),
            HistoricalData::BookSnapshot {
                instrument_id,
                bids,
                asks,
            } => Some(MarketDataEvent::BookUpdate {
                instrument_id: instrument_id.clone(),
                bids: bids.clone(),
                asks: asks.clone(),
                ts,
            }),
            HistoricalData::Bar {
                instrument_id,
                open: _,
                high: _,
                low: _,
                close,
                volume,
            } => Some(MarketDataEvent::Trade {
                instrument_id: instrument_id.clone(),
                price: *close,
                quantity: *volume,
                side: OrderSide::Buy,
                ts,
            }),
            HistoricalData::BookDelta { .. } => None,
        }
    }

    fn process_strategy_commands(&mut self, commands: Vec<StrategyCommand>) {
        for cmd in commands {
            match &cmd {
                StrategyCommand::SubmitOrder { .. } => {
                    self.venue.submit_order(&cmd, self.current_time);
                }
                StrategyCommand::CancelOrder { order_id } => {
                    self.venue.cancel_order(order_id, self.current_time);
                }
                StrategyCommand::ModifyOrder { .. } => {}
            }
        }
    }

    fn next_data_point(&mut self) -> Option<HistoricalDataPoint> {
        self.data_feed.as_mut()?.next()
    }

    fn process_trading_event(&mut self, event: &TradingEvent) {
        match event {
            TradingEvent::OrderFilled {
                order_id: _,
                fill_price,
                fill_quantity,
                remaining_quantity: _,
                ts,
            } => {
                let commission = fill_price * fill_quantity * self.config.commission_rate;
                self.results.total_trades += 1;
                self.results.total_commission += commission;
                self.results.total_volume += fill_price * fill_quantity;

                let trade_pnl = 0.0;
                self.results
                    .record_trade(trade_pnl, *ts, self.current_equity);
            }
            _ => {}
        }
    }

    fn finalize_results(&mut self) {
        self.results.data_points_processed = self.data_count;
        self.results.end_time = self.current_time;
        self.results.final_balance = self.current_equity;
        self.results.finalize();
    }

    pub fn results(&self) -> &BacktestResults {
        &self.results
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResults {
    pub initial_balance: f64,

    pub final_balance: f64,

    pub total_pnl: f64,

    pub return_pct: f64,

    pub total_trades: u64,

    pub winning_trades: u64,

    pub losing_trades: u64,

    pub win_rate: f64,

    pub total_volume: f64,

    pub total_commission: f64,

    pub data_points_processed: u64,

    pub start_time: UnixNanos,

    pub end_time: UnixNanos,

    pub max_drawdown_pct: f64,

    pub sharpe_ratio: f64,

    pub sortino_ratio: f64,

    pub calmar_ratio: f64,

    pub profit_factor: f64,

    pub avg_trade_pnl: f64,

    pub avg_win: f64,

    pub avg_loss: f64,

    pub largest_win: f64,

    pub largest_loss: f64,

    #[serde(skip)]
    pub equity_curve: Vec<(UnixNanos, f64)>,

    #[serde(skip)]
    pub daily_returns: Vec<f64>,
}

impl BacktestResults {
    pub fn new(initial_balance: f64, start_time: UnixNanos) -> Self {
        Self {
            initial_balance,
            final_balance: initial_balance,
            total_pnl: 0.0,
            return_pct: 0.0,
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            win_rate: 0.0,
            total_volume: 0.0,
            total_commission: 0.0,
            data_points_processed: 0,
            start_time,
            end_time: UnixNanos::ZERO,
            max_drawdown_pct: 0.0,
            sharpe_ratio: 0.0,
            sortino_ratio: 0.0,
            calmar_ratio: 0.0,
            profit_factor: 0.0,
            avg_trade_pnl: 0.0,
            avg_win: 0.0,
            avg_loss: 0.0,
            largest_win: 0.0,
            largest_loss: 0.0,
            equity_curve: vec![(start_time, initial_balance)],
            daily_returns: Vec::new(),
        }
    }

    pub fn record_trade(&mut self, pnl: f64, timestamp: UnixNanos, equity: f64) {
        self.total_trades += 1;

        if pnl > 0.0 {
            self.winning_trades += 1;
            if pnl > self.largest_win {
                self.largest_win = pnl;
            }
        } else if pnl < 0.0 {
            self.losing_trades += 1;
            if pnl < self.largest_loss {
                self.largest_loss = pnl;
            }
        }

        self.equity_curve.push((timestamp, equity));
    }

    pub fn finalize(&mut self) {
        self.total_pnl = self.final_balance - self.initial_balance - self.total_commission;

        if self.initial_balance > 0.0 {
            self.return_pct = (self.total_pnl / self.initial_balance) * 100.0;
        }

        if self.total_trades > 0 {
            self.win_rate = self.winning_trades as f64 / self.total_trades as f64;
            self.avg_trade_pnl = self.total_pnl / self.total_trades as f64;
        }

        if self.winning_trades > 0 {
            let gross_profit = self.largest_win * self.winning_trades as f64 * 0.5;
            self.avg_win = gross_profit / self.winning_trades as f64;
        }
        if self.losing_trades > 0 {
            let gross_loss = self.largest_loss.abs() * self.losing_trades as f64 * 0.5;
            self.avg_loss = gross_loss / self.losing_trades as f64;
        }

        if self.avg_loss > 0.0 && self.losing_trades > 0 {
            let gross_profit = self.avg_win * self.winning_trades as f64;
            let gross_loss = self.avg_loss * self.losing_trades as f64;
            self.profit_factor = gross_profit / gross_loss;
        }

        self.calculate_drawdown();
        self.calculate_risk_ratios();
    }

    fn calculate_drawdown(&mut self) {
        let mut peak = self.initial_balance;
        let mut max_dd = 0.0;

        for (_, equity) in &self.equity_curve {
            if *equity > peak {
                peak = *equity;
            }
            let dd = (peak - *equity) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }

        self.max_drawdown_pct = max_dd * 100.0;
    }

    fn calculate_risk_ratios(&mut self) {
        self.calculate_daily_returns();

        if self.daily_returns.is_empty() {
            return;
        }

        let n = self.daily_returns.len() as f64;
        let mean: f64 = self.daily_returns.iter().sum::<f64>() / n;

        let variance: f64 = self
            .daily_returns
            .iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>()
            / n;
        let std_dev = variance.sqrt();

        let downside_variance: f64 = self
            .daily_returns
            .iter()
            .filter(|r| **r < 0.0)
            .map(|r| r.powi(2))
            .sum::<f64>()
            / n;
        let downside_dev = downside_variance.sqrt();

        let annualization = 252.0_f64.sqrt();
        let risk_free_rate = 0.0;

        if std_dev > 0.0 {
            self.sharpe_ratio = ((mean - risk_free_rate / 252.0) / std_dev) * annualization;
        }

        if downside_dev > 0.0 {
            self.sortino_ratio = ((mean - risk_free_rate / 252.0) / downside_dev) * annualization;
        }

        if self.max_drawdown_pct > 0.0 {
            let annual_return = self.return_pct * 365.0 / self.trading_days() as f64;
            self.calmar_ratio = annual_return / self.max_drawdown_pct;
        }
    }

    fn calculate_daily_returns(&mut self) {
        if self.equity_curve.len() < 2 {
            return;
        }

        let mut daily_equities: Vec<(u64, f64)> = Vec::new();
        let mut current_day = 0u64;
        let mut day_equity = self.initial_balance;

        for (ts, equity) in &self.equity_curve {
            let day = ts.as_secs() / 86400;
            if day != current_day && current_day != 0 {
                daily_equities.push((current_day, day_equity));
            }
            current_day = day;
            day_equity = *equity;
        }

        if current_day != 0 {
            daily_equities.push((current_day, day_equity));
        }

        self.daily_returns.clear();
        for i in 1..daily_equities.len() {
            let prev = daily_equities[i - 1].1;
            let curr = daily_equities[i].1;
            if prev > 0.0 {
                self.daily_returns.push((curr - prev) / prev);
            }
        }
    }

    pub fn trading_days(&self) -> u64 {
        if self.end_time <= self.start_time {
            return 1;
        }
        let secs = self.end_time.as_secs() - self.start_time.as_secs();
        (secs / 86400).max(1)
    }

    pub fn summary(&self) -> String {
        format!(
            r#"
═══════════════════════════════════════════════════════════════
                      BACKTEST RESULTS
═══════════════════════════════════════════════════════════════

  Period:            {} days
  Data Points:       {}
  
  PERFORMANCE
  ───────────────────────────────────────────────────────────────
  Initial Balance:   ${:.2}
  Final Balance:     ${:.2}
  Total PnL:         ${:.2} ({:+.2}%)
  Total Commission:  ${:.2}
  
  RISK METRICS
  ───────────────────────────────────────────────────────────────
  Max Drawdown:      {:.2}%
  Sharpe Ratio:      {:.3}
  Sortino Ratio:     {:.3}
  Calmar Ratio:      {:.3}
  
  TRADING STATISTICS
  ───────────────────────────────────────────────────────────────
  Total Trades:      {}
  Win Rate:          {:.1}%
  Profit Factor:     {:.2}
  Avg Trade PnL:     ${:.2}
  Largest Win:       ${:.2}
  Largest Loss:      ${:.2}
  Total Volume:      ${:.2}

═══════════════════════════════════════════════════════════════
"#,
            self.trading_days(),
            self.data_points_processed,
            self.initial_balance,
            self.final_balance,
            self.total_pnl,
            self.return_pct,
            self.total_commission,
            self.max_drawdown_pct,
            self.sharpe_ratio,
            self.sortino_ratio,
            self.calmar_ratio,
            self.total_trades,
            self.win_rate * 100.0,
            self.profit_factor,
            self.avg_trade_pnl,
            self.largest_win,
            self.largest_loss,
            self.total_volume,
        )
    }
}

use std::collections::BinaryHeap;

pub struct MultiFeedMerger {
    feeds: Vec<Box<dyn DataFeed>>,
    heap: BinaryHeap<HeapEntry>,
}

struct HeapEntry {
    data_point: HistoricalDataPoint,
    feed_idx: usize,
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .data_point
            .timestamp
            .cmp(&self.data_point.timestamp)
            .then_with(|| other.data_point.sequence.cmp(&self.data_point.sequence))
    }
}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for HeapEntry {
    fn eq(&self, other: &Self) -> bool {
        self.data_point.timestamp == other.data_point.timestamp
            && self.data_point.sequence == other.data_point.sequence
    }
}

impl Eq for HeapEntry {}

impl MultiFeedMerger {
    pub fn new() -> Self {
        Self {
            feeds: Vec::new(),
            heap: BinaryHeap::new(),
        }
    }

    pub fn add_feed(&mut self, mut feed: Box<dyn DataFeed>) {
        let idx = self.feeds.len();

        if let Some(dp) = feed.next() {
            self.heap.push(HeapEntry {
                data_point: dp,
                feed_idx: idx,
            });
        }
        self.feeds.push(feed);
    }
}

impl Default for MultiFeedMerger {
    fn default() -> Self {
        Self::new()
    }
}

impl DataFeed for MultiFeedMerger {
    fn next(&mut self) -> Option<HistoricalDataPoint> {
        let entry = self.heap.pop()?;

        if let Some(next_dp) = self.feeds[entry.feed_idx].next() {
            self.heap.push(HeapEntry {
                data_point: next_dp,
                feed_idx: entry.feed_idx,
            });
        }

        Some(entry.data_point)
    }

    fn peek_timestamp(&self) -> Option<UnixNanos> {
        self.heap.peek().map(|e| e.data_point.timestamp)
    }

    fn reset(&mut self) {
        self.heap.clear();
        for (idx, feed) in self.feeds.iter_mut().enumerate() {
            feed.reset();
            if let Some(dp) = feed.next() {
                self.heap.push(HeapEntry {
                    data_point: dp,
                    feed_idx: idx,
                });
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct L2OrderBook {
    pub instrument_id: InstrumentId,

    bids: std::collections::BTreeMap<OrderedFloat, f64>,

    asks: std::collections::BTreeMap<OrderedFloat, f64>,

    sequence: u64,

    last_update: UnixNanos,

    max_depth: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct OrderedFloat(f64);

impl Eq for OrderedFloat {}

impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .partial_cmp(&other.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl PartialOrd for OrderedFloat {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::hash::Hash for OrderedFloat {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl L2OrderBook {
    pub fn new(instrument_id: InstrumentId, max_depth: usize) -> Self {
        Self {
            instrument_id,
            bids: std::collections::BTreeMap::new(),
            asks: std::collections::BTreeMap::new(),
            sequence: 0,
            last_update: UnixNanos::ZERO,
            max_depth,
        }
    }

    pub fn apply_snapshot(
        &mut self,
        bids: &[(f64, f64)],
        asks: &[(f64, f64)],
        ts: UnixNanos,
        seq: u64,
    ) {
        self.bids.clear();
        self.asks.clear();

        for (price, qty) in bids {
            if *qty > 0.0 {
                self.bids.insert(OrderedFloat(*price), *qty);
            }
        }
        for (price, qty) in asks {
            if *qty > 0.0 {
                self.asks.insert(OrderedFloat(*price), *qty);
            }
        }

        self.sequence = seq;
        self.last_update = ts;
        self.trim_depth();
    }

    pub fn apply_delta(
        &mut self,
        side: OrderSide,
        price: f64,
        quantity: f64,
        ts: UnixNanos,
        seq: u64,
    ) {
        let book = match side {
            OrderSide::Buy => &mut self.bids,
            OrderSide::Sell => &mut self.asks,
        };

        let key = OrderedFloat(price);
        if quantity <= 0.0 {
            book.remove(&key);
        } else {
            book.insert(key, quantity);
        }

        self.sequence = seq;
        self.last_update = ts;
        self.trim_depth();
    }

    fn trim_depth(&mut self) {
        if self.max_depth == 0 {
            return;
        }

        while self.bids.len() > self.max_depth {
            self.bids.pop_first();
        }

        while self.asks.len() > self.max_depth {
            self.asks.pop_last();
        }
    }

    pub fn best_bid(&self) -> Option<(f64, f64)> {
        self.bids.last_key_value().map(|(k, v)| (k.0, *v))
    }

    pub fn best_ask(&self) -> Option<(f64, f64)> {
        self.asks.first_key_value().map(|(k, v)| (k.0, *v))
    }

    pub fn mid_price(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some((bid, _)), Some((ask, _))) => Some((bid + ask) / 2.0),
            _ => None,
        }
    }

    pub fn spread(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some((bid, _)), Some((ask, _))) => Some(ask - bid),
            _ => None,
        }
    }

    pub fn micro_price(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some((bid, bid_qty)), Some((ask, ask_qty))) => {
                let total = bid_qty + ask_qty;
                if total > 0.0 {
                    Some((bid * ask_qty + ask * bid_qty) / total)
                } else {
                    Some((bid + ask) / 2.0)
                }
            }
            _ => None,
        }
    }

    pub fn liquidity_at_levels(&self, levels: usize) -> (f64, f64) {
        let bid_liquidity: f64 = self.bids.values().rev().take(levels).sum();
        let ask_liquidity: f64 = self.asks.values().take(levels).sum();
        (bid_liquidity, ask_liquidity)
    }

    pub fn simulate_market_order(&self, side: OrderSide, quantity: f64) -> MarketOrderSimulation {
        let book = match side {
            OrderSide::Buy => &self.asks,
            OrderSide::Sell => &self.bids,
        };

        let mut remaining = quantity;
        let mut total_cost = 0.0;
        let mut filled_qty = 0.0;
        let mut levels_consumed = 0;

        let prices: Vec<_> = match side {
            OrderSide::Buy => book.iter().map(|(k, v)| (k.0, *v)).collect(),
            OrderSide::Sell => book.iter().rev().map(|(k, v)| (k.0, *v)).collect(),
        };

        for (price, available) in prices {
            if remaining <= 0.0 {
                break;
            }

            let fill_at_level = remaining.min(available);
            total_cost += price * fill_at_level;
            filled_qty += fill_at_level;
            remaining -= fill_at_level;
            levels_consumed += 1;
        }

        let avg_price = if filled_qty > 0.0 {
            total_cost / filled_qty
        } else {
            0.0
        };

        MarketOrderSimulation {
            avg_fill_price: avg_price,
            filled_quantity: filled_qty,
            remaining_quantity: remaining,
            levels_consumed,
            price_impact_bps: self.calculate_impact_bps(side, avg_price),
        }
    }

    fn calculate_impact_bps(&self, side: OrderSide, avg_price: f64) -> f64 {
        let reference = match side {
            OrderSide::Buy => self.best_ask().map(|(p, _)| p),
            OrderSide::Sell => self.best_bid().map(|(p, _)| p),
        };

        match reference {
            Some(ref_price) if ref_price > 0.0 => {
                ((avg_price - ref_price) / ref_price).abs() * 10_000.0
            }
            _ => 0.0,
        }
    }

    pub fn depth(&self, levels: usize) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
        let bids: Vec<_> = self
            .bids
            .iter()
            .rev()
            .take(levels)
            .map(|(k, v)| (k.0, *v))
            .collect();
        let asks: Vec<_> = self
            .asks
            .iter()
            .take(levels)
            .map(|(k, v)| (k.0, *v))
            .collect();
        (bids, asks)
    }
}

#[derive(Debug, Clone)]
pub struct MarketOrderSimulation {
    pub avg_fill_price: f64,
    pub filled_quantity: f64,
    pub remaining_quantity: f64,
    pub levels_consumed: usize,
    pub price_impact_bps: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlippageModelType {
    Zero,

    FixedBps,

    VolumeImpact,

    SpreadBased,

    L2Simulation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlippageModelConfig {
    pub model_type: SlippageModelType,

    pub fixed_bps: f64,

    pub volume_impact_coef: f64,

    pub adv: f64,

    pub spread_cross_fraction: f64,
}

impl Default for SlippageModelConfig {
    fn default() -> Self {
        Self {
            model_type: SlippageModelType::FixedBps,
            fixed_bps: 5.0,
            volume_impact_coef: 0.1,
            adv: 1_000_000.0,
            spread_cross_fraction: 0.5,
        }
    }
}

pub struct SlippageModel {
    config: SlippageModelConfig,
}

impl SlippageModel {
    pub fn new(config: SlippageModelConfig) -> Self {
        Self { config }
    }

    pub fn calculate_fill_price(
        &self,
        side: OrderSide,
        quantity: f64,
        mid_price: f64,
        spread: Option<f64>,
        book: Option<&L2OrderBook>,
    ) -> f64 {
        match self.config.model_type {
            SlippageModelType::Zero => mid_price,

            SlippageModelType::FixedBps => {
                let slip = mid_price * self.config.fixed_bps / 10_000.0;
                match side {
                    OrderSide::Buy => mid_price + slip,
                    OrderSide::Sell => mid_price - slip,
                }
            }

            SlippageModelType::VolumeImpact => {
                let participation = (quantity / self.config.adv).sqrt();
                let impact = self.config.volume_impact_coef * participation * mid_price;
                match side {
                    OrderSide::Buy => mid_price + impact,
                    OrderSide::Sell => mid_price - impact,
                }
            }

            SlippageModelType::SpreadBased => {
                let half_spread = spread.unwrap_or(0.0) * self.config.spread_cross_fraction;
                match side {
                    OrderSide::Buy => mid_price + half_spread,
                    OrderSide::Sell => mid_price - half_spread,
                }
            }

            SlippageModelType::L2Simulation => {
                if let Some(book) = book {
                    let sim = book.simulate_market_order(side, quantity);
                    if sim.filled_quantity > 0.0 {
                        sim.avg_fill_price
                    } else {
                        mid_price
                    }
                } else {
                    let slip = mid_price * self.config.fixed_bps / 10_000.0;
                    match side {
                        OrderSide::Buy => mid_price + slip,
                        OrderSide::Sell => mid_price - slip,
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionAlgorithmType {
    TWAP,

    VWAP,

    Iceberg,

    POV,

    IS,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwapConfig {
    pub total_quantity: f64,

    pub duration_nanos: u64,

    pub num_slices: usize,

    pub randomize_timing: bool,

    pub randomize_range: f64,

    pub limit_price: Option<f64>,
}

impl Default for TwapConfig {
    fn default() -> Self {
        Self {
            total_quantity: 1.0,
            duration_nanos: 60_000_000_000,
            num_slices: 10,
            randomize_timing: true,
            randomize_range: 0.1,
            limit_price: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VwapConfig {
    pub total_quantity: f64,

    pub duration_nanos: u64,

    pub num_buckets: usize,

    pub volume_profile: Vec<f64>,

    pub min_slice_size: f64,

    pub limit_price: Option<f64>,
}

impl Default for VwapConfig {
    fn default() -> Self {
        let num_buckets = 10;
        Self {
            total_quantity: 1.0,
            duration_nanos: 60_000_000_000,
            num_buckets,
            volume_profile: vec![1.0 / num_buckets as f64; num_buckets],
            min_slice_size: 0.001,
            limit_price: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcebergConfig {
    pub total_quantity: f64,

    pub display_quantity: f64,

    pub min_refill_quantity: f64,

    pub randomize_display: bool,

    pub randomize_range: f64,

    pub limit_price: f64,

    pub side: OrderSide,
}

impl Default for IcebergConfig {
    fn default() -> Self {
        Self {
            total_quantity: 10.0,
            display_quantity: 1.0,
            min_refill_quantity: 0.5,
            randomize_display: true,
            randomize_range: 0.2,
            limit_price: 0.0,
            side: OrderSide::Buy,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PovConfig {
    pub total_quantity: f64,

    pub participation_rate: f64,

    pub max_participation_rate: f64,

    pub min_trade_interval_nanos: u64,

    pub limit_price: Option<f64>,
}

impl Default for PovConfig {
    fn default() -> Self {
        Self {
            total_quantity: 1.0,
            participation_rate: 0.1,
            max_participation_rate: 0.25,
            min_trade_interval_nanos: 1_000_000_000,
            limit_price: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionAlgorithmState {
    pub algo_type: ExecutionAlgorithmType,

    pub instrument_id: InstrumentId,

    pub side: OrderSide,

    pub filled_quantity: f64,

    pub remaining_quantity: f64,

    pub start_time: UnixNanos,

    pub end_time: UnixNanos,

    pub vwap_numerator: f64,

    pub slices_executed: usize,

    pub total_slices: usize,

    pub next_slice_time: UnixNanos,

    pub is_complete: bool,

    pub child_order_ids: Vec<OrderId>,
}

pub struct ExecutionAlgorithmManager {
    algorithms: HashMap<String, ExecutionAlgorithmState>,

    twap_configs: HashMap<String, TwapConfig>,

    vwap_configs: HashMap<String, VwapConfig>,

    iceberg_configs: HashMap<String, IcebergConfig>,

    pov_configs: HashMap<String, PovConfig>,

    pending_commands: VecDeque<StrategyCommand>,

    order_counter: u64,

    volume_tracker: HashMap<InstrumentId, f64>,
}

impl ExecutionAlgorithmManager {
    pub fn new() -> Self {
        Self {
            algorithms: HashMap::new(),
            twap_configs: HashMap::new(),
            vwap_configs: HashMap::new(),
            iceberg_configs: HashMap::new(),
            pov_configs: HashMap::new(),
            pending_commands: VecDeque::new(),
            order_counter: 0,
            volume_tracker: HashMap::new(),
        }
    }

    pub fn start_twap(
        &mut self,
        algo_id: String,
        instrument_id: InstrumentId,
        side: OrderSide,
        config: TwapConfig,
        current_time: UnixNanos,
    ) {
        let end_time = UnixNanos::from_nanos(current_time.0 + config.duration_nanos);
        let slice_interval = config.duration_nanos / config.num_slices as u64;

        let state = ExecutionAlgorithmState {
            algo_type: ExecutionAlgorithmType::TWAP,
            instrument_id: instrument_id.clone(),
            side,
            filled_quantity: 0.0,
            remaining_quantity: config.total_quantity,
            start_time: current_time,
            end_time,
            vwap_numerator: 0.0,
            slices_executed: 0,
            total_slices: config.num_slices,
            next_slice_time: current_time,
            is_complete: false,
            child_order_ids: Vec::new(),
        };

        self.algorithms.insert(algo_id.clone(), state);
        self.twap_configs.insert(algo_id, config);
    }

    pub fn start_vwap(
        &mut self,
        algo_id: String,
        instrument_id: InstrumentId,
        side: OrderSide,
        config: VwapConfig,
        current_time: UnixNanos,
    ) {
        let end_time = UnixNanos::from_nanos(current_time.0 + config.duration_nanos);

        let state = ExecutionAlgorithmState {
            algo_type: ExecutionAlgorithmType::VWAP,
            instrument_id,
            side,
            filled_quantity: 0.0,
            remaining_quantity: config.total_quantity,
            start_time: current_time,
            end_time,
            vwap_numerator: 0.0,
            slices_executed: 0,
            total_slices: config.num_buckets,
            next_slice_time: current_time,
            is_complete: false,
            child_order_ids: Vec::new(),
        };

        self.algorithms.insert(algo_id.clone(), state);
        self.vwap_configs.insert(algo_id, config);
    }

    pub fn start_iceberg(
        &mut self,
        algo_id: String,
        instrument_id: InstrumentId,
        config: IcebergConfig,
        current_time: UnixNanos,
    ) {
        let state = ExecutionAlgorithmState {
            algo_type: ExecutionAlgorithmType::Iceberg,
            instrument_id,
            side: config.side,
            filled_quantity: 0.0,
            remaining_quantity: config.total_quantity,
            start_time: current_time,
            end_time: UnixNanos::from_nanos(u64::MAX),
            vwap_numerator: 0.0,
            slices_executed: 0,
            total_slices: (config.total_quantity / config.display_quantity).ceil() as usize,
            next_slice_time: current_time,
            is_complete: false,
            child_order_ids: Vec::new(),
        };

        self.algorithms.insert(algo_id.clone(), state);
        self.iceberg_configs.insert(algo_id, config);
    }

    pub fn start_pov(
        &mut self,
        algo_id: String,
        instrument_id: InstrumentId,
        side: OrderSide,
        config: PovConfig,
        current_time: UnixNanos,
    ) {
        let state = ExecutionAlgorithmState {
            algo_type: ExecutionAlgorithmType::POV,
            instrument_id: instrument_id.clone(),
            side,
            filled_quantity: 0.0,
            remaining_quantity: config.total_quantity,
            start_time: current_time,
            end_time: UnixNanos::from_nanos(u64::MAX),
            vwap_numerator: 0.0,
            slices_executed: 0,
            total_slices: 0,
            next_slice_time: current_time,
            is_complete: false,
            child_order_ids: Vec::new(),
        };

        self.volume_tracker.insert(instrument_id, 0.0);

        self.algorithms.insert(algo_id.clone(), state);
        self.pov_configs.insert(algo_id, config);
    }

    pub fn on_trade(&mut self, instrument_id: &InstrumentId, quantity: f64) {
        if let Some(vol) = self.volume_tracker.get_mut(instrument_id) {
            *vol += quantity;
        }
    }

    pub fn cancel_algorithm(&mut self, algo_id: &str) -> Option<ExecutionAlgorithmState> {
        let state = self.algorithms.remove(algo_id)?;

        for order_id in &state.child_order_ids {
            self.pending_commands
                .push_back(StrategyCommand::CancelOrder {
                    order_id: order_id.clone(),
                });
        }

        self.twap_configs.remove(algo_id);
        self.vwap_configs.remove(algo_id);
        self.iceberg_configs.remove(algo_id);
        self.pov_configs.remove(algo_id);

        Some(state)
    }

    pub fn on_time(&mut self, current_time: UnixNanos) {
        let algo_ids: Vec<_> = self.algorithms.keys().cloned().collect();

        for algo_id in algo_ids {
            if let Some(state) = self.algorithms.get(&algo_id) {
                if state.is_complete {
                    continue;
                }

                match state.algo_type {
                    ExecutionAlgorithmType::TWAP => {
                        self.process_twap(&algo_id, current_time);
                    }
                    ExecutionAlgorithmType::VWAP => {
                        self.process_vwap(&algo_id, current_time);
                    }
                    ExecutionAlgorithmType::Iceberg => {}
                    ExecutionAlgorithmType::POV => {
                        self.process_pov(&algo_id, current_time);
                    }
                    ExecutionAlgorithmType::IS => {}
                }
            }
        }
    }

    fn process_twap(&mut self, algo_id: &str, current_time: UnixNanos) {
        let state = match self.algorithms.get(algo_id) {
            Some(s) => s.clone(),
            None => return,
        };

        let config = match self.twap_configs.get(algo_id) {
            Some(c) => c.clone(),
            None => return,
        };

        if current_time < state.next_slice_time {
            return;
        }

        if current_time > state.end_time || state.remaining_quantity <= 0.0 {
            if let Some(s) = self.algorithms.get_mut(algo_id) {
                s.is_complete = true;
            }
            return;
        }

        let slices_remaining = state.total_slices - state.slices_executed;
        let base_slice_qty = state.remaining_quantity / slices_remaining.max(1) as f64;

        let order_id = self.next_order_id(algo_id);
        let order_type = if config.limit_price.is_some() {
            OrderType::Limit
        } else {
            OrderType::Market
        };

        self.pending_commands
            .push_back(StrategyCommand::SubmitOrder {
                order_id: order_id.clone(),
                instrument_id: state.instrument_id.clone(),
                side: state.side,
                order_type,
                price: config.limit_price,
                quantity: base_slice_qty,
            });

        if let Some(s) = self.algorithms.get_mut(algo_id) {
            s.slices_executed += 1;
            s.child_order_ids.push(order_id);

            let slice_interval = config.duration_nanos / config.num_slices as u64;
            let mut next_time =
                state.start_time.as_nanos() + (s.slices_executed as u128 * slice_interval as u128);

            if config.randomize_timing {
                let mut rng = rand::thread_rng();
                let jitter = (slice_interval as f64 * config.randomize_range) as i64;
                let offset = rng.gen_range(-jitter..=jitter);
                next_time = (next_time as i128 + offset as i128).max(0) as u128;
            }

            s.next_slice_time = UnixNanos::from_nanos(next_time as u64);
        }
    }

    fn process_vwap(&mut self, algo_id: &str, current_time: UnixNanos) {
        let state = match self.algorithms.get(algo_id) {
            Some(s) => s.clone(),
            None => return,
        };

        let config = match self.vwap_configs.get(algo_id) {
            Some(c) => c.clone(),
            None => return,
        };

        let elapsed = current_time
            .as_nanos()
            .saturating_sub(state.start_time.as_nanos());
        let bucket_size = config.duration_nanos as u128 / config.num_buckets as u128;
        let current_bucket = (elapsed / bucket_size).min(config.num_buckets as u128 - 1) as usize;

        if current_bucket < state.slices_executed {
            return;
        }

        if current_time > state.end_time || state.remaining_quantity <= 0.0 {
            if let Some(s) = self.algorithms.get_mut(algo_id) {
                s.is_complete = true;
            }
            return;
        }

        let bucket_fraction = config
            .volume_profile
            .get(current_bucket)
            .copied()
            .unwrap_or(0.0);
        let bucket_qty = (config.total_quantity * bucket_fraction).max(config.min_slice_size);
        let slice_qty = bucket_qty.min(state.remaining_quantity);

        if slice_qty < config.min_slice_size {
            return;
        }

        let order_id = self.next_order_id(algo_id);
        let order_type = if config.limit_price.is_some() {
            OrderType::Limit
        } else {
            OrderType::Market
        };

        self.pending_commands
            .push_back(StrategyCommand::SubmitOrder {
                order_id: order_id.clone(),
                instrument_id: state.instrument_id.clone(),
                side: state.side,
                order_type,
                price: config.limit_price,
                quantity: slice_qty,
            });

        if let Some(s) = self.algorithms.get_mut(algo_id) {
            s.slices_executed = current_bucket + 1;
            s.child_order_ids.push(order_id);
        }
    }

    fn process_pov(&mut self, algo_id: &str, current_time: UnixNanos) {
        let state = match self.algorithms.get(algo_id) {
            Some(s) => s.clone(),
            None => return,
        };

        let config = match self.pov_configs.get(algo_id) {
            Some(c) => c.clone(),
            None => return,
        };

        if current_time.as_nanos() < state.next_slice_time.as_nanos() {
            return;
        }

        let market_volume = self
            .volume_tracker
            .get(&state.instrument_id)
            .copied()
            .unwrap_or(0.0);
        if market_volume <= 0.0 {
            return;
        }

        let target_qty = market_volume * config.participation_rate;
        let excess_qty = target_qty - state.filled_quantity;

        if excess_qty <= 0.0 || state.remaining_quantity <= 0.0 {
            return;
        }

        let max_qty = market_volume * config.max_participation_rate - state.filled_quantity;
        let slice_qty = excess_qty.min(max_qty).min(state.remaining_quantity);

        if slice_qty <= 0.0 {
            return;
        }

        let order_id = self.next_order_id(algo_id);
        let order_type = if config.limit_price.is_some() {
            OrderType::Limit
        } else {
            OrderType::Market
        };

        self.pending_commands
            .push_back(StrategyCommand::SubmitOrder {
                order_id: order_id.clone(),
                instrument_id: state.instrument_id.clone(),
                side: state.side,
                order_type,
                price: config.limit_price,
                quantity: slice_qty,
            });

        if let Some(s) = self.algorithms.get_mut(algo_id) {
            s.slices_executed += 1;
            s.child_order_ids.push(order_id);
            s.next_slice_time = UnixNanos::from_nanos(
                current_time.as_nanos() as u64 + config.min_trade_interval_nanos,
            );
        }
    }

    pub fn on_fill(&mut self, order_id: &OrderId, fill_qty: f64, fill_price: f64) {
        for (algo_id, state) in self.algorithms.iter_mut() {
            if state.child_order_ids.contains(order_id) {
                state.filled_quantity += fill_qty;
                state.remaining_quantity -= fill_qty;
                state.vwap_numerator += fill_qty * fill_price;

                if state.remaining_quantity <= 0.0 {
                    state.is_complete = true;
                }

                if state.algo_type == ExecutionAlgorithmType::Iceberg && !state.is_complete {
                    if let Some(config) = self.iceberg_configs.get(algo_id) {
                        if state.remaining_quantity >= config.min_refill_quantity {
                            let mut display_qty = config.display_quantity;

                            if config.randomize_display {
                                let mut rng = rand::thread_rng();
                                let range = display_qty * config.randomize_range;
                                let offset = rng.gen_range(-range..range);
                                display_qty =
                                    (display_qty + offset).max(config.min_refill_quantity);
                            }

                            display_qty = display_qty.min(state.remaining_quantity);

                            let new_order_id = OrderId::new(format!(
                                "{}-slice-{}",
                                algo_id, state.slices_executed
                            ));

                            self.pending_commands
                                .push_back(StrategyCommand::SubmitOrder {
                                    order_id: new_order_id.clone(),
                                    instrument_id: state.instrument_id.clone(),
                                    side: config.side,
                                    order_type: OrderType::Limit,
                                    price: Some(config.limit_price),
                                    quantity: display_qty,
                                });

                            state.slices_executed += 1;
                            state.child_order_ids.push(new_order_id);
                        }
                    }
                }

                break;
            }
        }
    }

    pub fn drain_commands(&mut self) -> Vec<StrategyCommand> {
        self.pending_commands.drain(..).collect()
    }

    pub fn get_state(&self, algo_id: &str) -> Option<&ExecutionAlgorithmState> {
        self.algorithms.get(algo_id)
    }

    pub fn achieved_vwap(&self, algo_id: &str) -> Option<f64> {
        self.algorithms.get(algo_id).and_then(|state| {
            if state.filled_quantity > 0.0 {
                Some(state.vwap_numerator / state.filled_quantity)
            } else {
                None
            }
        })
    }

    pub fn all_complete(&self) -> bool {
        self.algorithms.values().all(|s| s.is_complete)
    }

    pub fn active_count(&self) -> usize {
        self.algorithms.values().filter(|s| !s.is_complete).count()
    }

    fn next_order_id(&mut self, algo_id: &str) -> OrderId {
        self.order_counter += 1;
        OrderId::new(format!("{}-child-{}", algo_id, self.order_counter))
    }
}

impl Default for ExecutionAlgorithmManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkForwardConfig {
    pub num_windows: usize,

    pub in_sample_fraction: f64,

    pub anchored: bool,

    pub min_in_sample_periods: usize,
}

impl Default for WalkForwardConfig {
    fn default() -> Self {
        Self {
            num_windows: 5,
            in_sample_fraction: 0.7,
            anchored: false,
            min_in_sample_periods: 100,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WalkForwardWindow {
    pub window_idx: usize,
    pub train_start: UnixNanos,
    pub train_end: UnixNanos,
    pub test_start: UnixNanos,
    pub test_end: UnixNanos,
}

pub struct WalkForwardSplitter {
    config: WalkForwardConfig,
}

impl WalkForwardSplitter {
    pub fn new(config: WalkForwardConfig) -> Self {
        Self { config }
    }

    pub fn generate_windows(&self, start: UnixNanos, end: UnixNanos) -> Vec<WalkForwardWindow> {
        let total_nanos = end.as_nanos() - start.as_nanos();
        let mut windows = Vec::new();

        if self.config.anchored {
            let test_window_size = total_nanos / (self.config.num_windows as u128 + 1);

            for i in 0..self.config.num_windows {
                let test_start_nanos = start.as_nanos()
                    + (total_nanos * (i + 1) as u128) / (self.config.num_windows + 1) as u128;
                let test_end_nanos = test_start_nanos + test_window_size;

                windows.push(WalkForwardWindow {
                    window_idx: i,
                    train_start: start,
                    train_end: UnixNanos::from_nanos(test_start_nanos as u64),
                    test_start: UnixNanos::from_nanos(test_start_nanos as u64),
                    test_end: UnixNanos::from_nanos(test_end_nanos.min(end.as_nanos()) as u64),
                });
            }
        } else {
            let window_size = total_nanos / self.config.num_windows as u128;
            let train_size = (window_size as f64 * self.config.in_sample_fraction) as u128;

            for i in 0..self.config.num_windows {
                let window_start = start.as_nanos() + (window_size * i as u128);
                let train_end_nanos = window_start + train_size;
                let window_end = window_start + window_size;

                windows.push(WalkForwardWindow {
                    window_idx: i,
                    train_start: UnixNanos::from_nanos(window_start as u64),
                    train_end: UnixNanos::from_nanos(train_end_nanos as u64),
                    test_start: UnixNanos::from_nanos(train_end_nanos as u64),
                    test_end: UnixNanos::from_nanos(window_end.min(end.as_nanos()) as u64),
                });
            }
        }

        windows
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkForwardResult {
    pub window_idx: usize,
    pub train_start: UnixNanos,
    pub train_end: UnixNanos,
    pub test_start: UnixNanos,
    pub test_end: UnixNanos,

    pub best_params: HashMap<String, f64>,

    pub train_results: BacktestResults,

    pub test_results: BacktestResults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkForwardAnalysis {
    pub windows: Vec<WalkForwardResult>,

    pub combined_oos_pnl: f64,
    pub combined_oos_return_pct: f64,
    pub combined_oos_sharpe: f64,

    pub robustness_ratio: f64,
}

impl WalkForwardAnalysis {
    pub fn from_results(windows: Vec<WalkForwardResult>) -> Self {
        let combined_oos_pnl: f64 = windows.iter().map(|w| w.test_results.total_pnl).sum();

        let initial_balance: f64 = windows
            .first()
            .map(|w| w.test_results.initial_balance)
            .unwrap_or(100_000.0);

        let combined_oos_return_pct = (combined_oos_pnl / initial_balance) * 100.0;

        let oos_sharpes: Vec<_> = windows
            .iter()
            .map(|w| w.test_results.sharpe_ratio)
            .filter(|s| s.is_finite())
            .collect();
        let combined_oos_sharpe = if !oos_sharpes.is_empty() {
            oos_sharpes.iter().sum::<f64>() / oos_sharpes.len() as f64
        } else {
            0.0
        };

        let is_sharpes: Vec<_> = windows
            .iter()
            .map(|w| w.train_results.sharpe_ratio)
            .filter(|s| s.is_finite() && *s > 0.0)
            .collect();
        let avg_is_sharpe = if !is_sharpes.is_empty() {
            is_sharpes.iter().sum::<f64>() / is_sharpes.len() as f64
        } else {
            1.0
        };

        let robustness_ratio = if avg_is_sharpe > 0.0 {
            combined_oos_sharpe / avg_is_sharpe
        } else {
            0.0
        };

        Self {
            windows,
            combined_oos_pnl,
            combined_oos_return_pct,
            combined_oos_sharpe,
            robustness_ratio,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDef {
    pub name: String,
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

impl ParameterDef {
    pub fn new(name: impl Into<String>, min: f64, max: f64, step: f64) -> Self {
        Self {
            name: name.into(),
            min,
            max,
            step,
        }
    }

    pub fn values(&self) -> Vec<f64> {
        let mut vals = Vec::new();
        let mut v = self.min;
        while v <= self.max + 1e-10 {
            vals.push(v);
            v += self.step;
        }
        vals
    }

    pub fn count(&self) -> usize {
        self.values().len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSweepConfig {
    pub parameters: Vec<ParameterDef>,

    pub target_metric: OptimizationMetric,

    pub maximize: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationMetric {
    TotalPnl,
    ReturnPct,
    SharpeRatio,
    SortinoRatio,
    CalmarRatio,
    ProfitFactor,
    WinRate,
    MaxDrawdown,
}

impl OptimizationMetric {
    pub fn extract(&self, results: &BacktestResults) -> f64 {
        match self {
            Self::TotalPnl => results.total_pnl,
            Self::ReturnPct => results.return_pct,
            Self::SharpeRatio => results.sharpe_ratio,
            Self::SortinoRatio => results.sortino_ratio,
            Self::CalmarRatio => results.calmar_ratio,
            Self::ProfitFactor => results.profit_factor,
            Self::WinRate => results.win_rate,
            Self::MaxDrawdown => results.max_drawdown_pct,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepResult {
    pub params: HashMap<String, f64>,
    pub metric_value: f64,
    pub results: BacktestResults,
}

pub struct ParameterSweep {
    config: ParameterSweepConfig,
}

impl ParameterSweep {
    pub fn new(config: ParameterSweepConfig) -> Self {
        Self { config }
    }

    pub fn generate_combinations(&self) -> Vec<HashMap<String, f64>> {
        let mut combos = vec![HashMap::new()];

        for param in &self.config.parameters {
            let mut new_combos = Vec::new();
            for combo in &combos {
                for value in param.values() {
                    let mut new_combo = combo.clone();
                    new_combo.insert(param.name.clone(), value);
                    new_combos.push(new_combo);
                }
            }
            combos = new_combos;
        }

        combos
    }

    pub fn total_combinations(&self) -> usize {
        self.config.parameters.iter().map(|p| p.count()).product()
    }

    pub fn find_best<'a>(&self, results: &'a [SweepResult]) -> Option<&'a SweepResult> {
        if results.is_empty() {
            return None;
        }

        if self.config.maximize {
            results.iter().max_by(|a, b| {
                a.metric_value
                    .partial_cmp(&b.metric_value)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        } else {
            results.iter().min_by(|a, b| {
                a.metric_value
                    .partial_cmp(&b.metric_value)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        }
    }

    pub fn parameter_sensitivity(
        &self,
        results: &[SweepResult],
    ) -> HashMap<String, ParameterSensitivity> {
        let mut sensitivities = HashMap::new();

        for param in &self.config.parameters {
            let mut value_metrics: HashMap<String, Vec<f64>> = HashMap::new();

            for result in results {
                if let Some(val) = result.params.get(&param.name) {
                    let key = format!("{:.6}", val);
                    value_metrics
                        .entry(key)
                        .or_default()
                        .push(result.metric_value);
                }
            }

            let mut averages: Vec<(f64, f64)> = value_metrics
                .iter()
                .map(|(k, v)| {
                    let param_val: f64 = k.parse().unwrap_or(0.0);
                    let avg_metric = v.iter().sum::<f64>() / v.len() as f64;
                    (param_val, avg_metric)
                })
                .collect();
            averages.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

            let best = if self.config.maximize {
                averages
                    .iter()
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            } else {
                averages
                    .iter()
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            };

            let metric_range = averages.iter().map(|(_, m)| *m).fold(f64::NAN, f64::max)
                - averages.iter().map(|(_, m)| *m).fold(f64::NAN, f64::min);

            sensitivities.insert(
                param.name.clone(),
                ParameterSensitivity {
                    param_name: param.name.clone(),
                    best_value: best.map(|(v, _)| *v).unwrap_or(param.min),
                    best_metric: best.map(|(_, m)| *m).unwrap_or(0.0),
                    metric_range,
                    value_metrics: averages,
                },
            );
        }

        sensitivities
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSensitivity {
    pub param_name: String,
    pub best_value: f64,
    pub best_metric: f64,
    pub metric_range: f64,

    pub value_metrics: Vec<(f64, f64)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_feed() {
        let trades = vec![
            (1000, "BTC", 50000.0, 1.0, true),
            (2000, "BTC", 50100.0, 0.5, false),
            (3000, "BTC", 50050.0, 2.0, true),
        ];
        let mut feed = InMemoryDataFeed::from_trades(trades);

        let first = feed.next().unwrap();
        assert_eq!(first.timestamp.as_millis(), 1000);

        let second = feed.next().unwrap();
        assert_eq!(second.timestamp.as_millis(), 2000);
    }

    #[test]
    fn test_simulated_book() {
        let instrument_id = InstrumentId::new(Venue::Simulated, "ETH", InstrumentType::Perp);
        let mut book = SimulatedBook::new(instrument_id);

        book.update_snapshot(
            vec![(2000.0, 10.0), (1999.0, 20.0)],
            vec![(2001.0, 10.0), (2002.0, 15.0)],
            UnixNanos::from_millis(1000),
        );

        assert_eq!(book.best_bid(), Some(2000.0));
        assert_eq!(book.best_ask(), Some(2001.0));
        assert_eq!(book.spread(), Some(1.0));
    }

    #[test]
    fn test_simulated_venue_market_order() {
        let fill_config = FillModelConfig::default();
        let mut venue = SimulatedVenue::new(Venue::Simulated, fill_config, 0.0004);

        let instrument_id = InstrumentId::new(Venue::Simulated, "BTC", InstrumentType::Perp);
        let ts = UnixNanos::from_millis(1000);

        venue.on_data(
            &HistoricalData::Quote {
                instrument_id: instrument_id.clone(),
                bid_price: 50000.0,
                bid_size: 10.0,
                ask_price: 50010.0,
                ask_size: 10.0,
            },
            ts,
        );

        let cmd = StrategyCommand::SubmitOrder {
            order_id: OrderId::new("test-1"),
            instrument_id,
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            price: None,
            quantity: 1.0,
        };

        venue.submit_order(&cmd, ts);

        let events = venue.drain_events();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_backtest_node() {
        let config = BacktestConfig {
            start_time: UnixNanos::from_millis(0),
            end_time: UnixNanos::from_millis(10000),
            ..Default::default()
        };

        let trades = vec![
            (1000, "BTC", 50000.0, 1.0, true),
            (2000, "BTC", 50100.0, 0.5, false),
            (3000, "BTC", 50050.0, 2.0, true),
        ];

        let mut node = BacktestNode::new(config);
        node.set_data_feed(Box::new(InMemoryDataFeed::from_trades(trades)));

        let results = node.run();
        assert_eq!(results.data_points_processed, 3);
    }

    #[test]
    fn test_l2_orderbook() {
        let instrument_id = InstrumentId::new(Venue::Simulated, "ETH", InstrumentType::Perp);
        let mut book = L2OrderBook::new(instrument_id, 10);

        book.apply_snapshot(
            &[(2000.0, 10.0), (1999.0, 20.0), (1998.0, 30.0)],
            &[(2001.0, 15.0), (2002.0, 25.0), (2003.0, 35.0)],
            UnixNanos::from_millis(1000),
            1,
        );

        assert_eq!(book.best_bid().unwrap().0, 2000.0);
        assert_eq!(book.best_ask().unwrap().0, 2001.0);
        assert_eq!(book.spread().unwrap(), 1.0);

        let micro = book.micro_price().unwrap();
        assert!(micro > 2000.0 && micro < 2001.0);

        let sim = book.simulate_market_order(OrderSide::Buy, 20.0);
        assert!(sim.filled_quantity <= 20.0);
        assert!(sim.levels_consumed >= 1);
    }

    #[test]
    fn test_walk_forward_splitter() {
        let config = WalkForwardConfig {
            num_windows: 3,
            in_sample_fraction: 0.7,
            anchored: false,
            min_in_sample_periods: 10,
        };

        let splitter = WalkForwardSplitter::new(config);
        let start = UnixNanos::from_secs(0);
        let end = UnixNanos::from_secs(30 * 86400);

        let windows = splitter.generate_windows(start, end);
        assert_eq!(windows.len(), 3);

        for w in &windows {
            assert!(w.train_end <= w.test_start);
            assert!(w.test_start < w.test_end);
        }
    }

    #[test]
    fn test_parameter_sweep() {
        let config = ParameterSweepConfig {
            parameters: vec![
                ParameterDef::new("fast_period", 5.0, 15.0, 5.0),
                ParameterDef::new("slow_period", 20.0, 30.0, 5.0),
            ],
            target_metric: OptimizationMetric::SharpeRatio,
            maximize: true,
        };

        let sweep = ParameterSweep::new(config);
        let combos = sweep.generate_combinations();

        assert_eq!(combos.len(), 9);
        assert_eq!(sweep.total_combinations(), 9);
    }

    #[test]
    fn test_slippage_models() {
        let config = SlippageModelConfig {
            model_type: SlippageModelType::VolumeImpact,
            volume_impact_coef: 0.1,
            adv: 1_000_000.0,
            ..Default::default()
        };

        let model = SlippageModel::new(config);

        let small_fill = model.calculate_fill_price(OrderSide::Buy, 100.0, 50000.0, None, None);

        let large_fill = model.calculate_fill_price(OrderSide::Buy, 10000.0, 50000.0, None, None);

        assert!(large_fill > small_fill);
    }

    #[test]
    fn test_multi_feed_merger() {
        let feed1 = Box::new(InMemoryDataFeed::from_trades(vec![
            (1000, "BTC", 50000.0, 1.0, true),
            (3000, "BTC", 50100.0, 1.0, true),
        ]));

        let feed2 = Box::new(InMemoryDataFeed::from_trades(vec![
            (2000, "ETH", 3000.0, 2.0, false),
            (4000, "ETH", 3010.0, 2.0, false),
        ]));

        let mut merger = MultiFeedMerger::new();
        merger.add_feed(feed1);
        merger.add_feed(feed2);

        let first = merger.next().unwrap();
        assert_eq!(first.timestamp.as_millis(), 1000);

        let second = merger.next().unwrap();
        assert_eq!(second.timestamp.as_millis(), 2000);

        let third = merger.next().unwrap();
        assert_eq!(third.timestamp.as_millis(), 3000);
    }
}
