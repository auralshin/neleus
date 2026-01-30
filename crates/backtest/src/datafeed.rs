use neleus_core_engine::OrderSide;
use neleus_core_types::{InstrumentId, InstrumentType, UnixNanos, Venue};
use serde::{Deserialize, Serialize};

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
    use crate::config::{BacktestConfig, FillModelConfig, LatencyModelConfig, SimulationMode};
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
