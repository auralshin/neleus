use crate::datafeed::{DataFeed, HistoricalDataPoint};
use neleus_core_engine::OrderSide;
use neleus_core_types::{InstrumentId, UnixNanos};
use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, BTreeMap};

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

    bids: BTreeMap<OrderedFloat, f64>,

    asks: BTreeMap<OrderedFloat, f64>,

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
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
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
