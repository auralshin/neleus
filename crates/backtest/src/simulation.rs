use crate::config::FillModelConfig;
use crate::datafeed::HistoricalData;
use neleus_core_engine::{OrderSide, OrderType, StrategyCommand, TradingEvent};
use neleus_core_types::{InstrumentId, OrderId, UnixNanos, Venue};
use std::collections::{HashMap, VecDeque};

const PRICE_EPSILON: f64 = 1e-10;

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

    pub fn update_snapshot_from_slices(
        &mut self,
        bids: &[(f64, f64)],
        asks: &[(f64, f64)],
        ts: UnixNanos,
    ) {
        self.bids.clear();
        self.bids.extend_from_slice(bids);
        self.asks.clear();
        self.asks.extend_from_slice(asks);
        self.bids.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
        self.asks.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
        self.last_update = ts;
    }

    pub fn update_snapshot(&mut self, bids: Vec<(f64, f64)>, asks: Vec<(f64, f64)>, ts: UnixNanos) {
        self.update_snapshot_from_slices(&bids, &asks, ts);
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

        match find_level_index(book, price, side == OrderSide::Buy) {
            Ok(index) => {
                if is_delete || quantity == 0.0 {
                    book.remove(index);
                } else {
                    book[index].1 = quantity;
                }
            }
            Err(index) => {
                if !(is_delete || quantity == 0.0) {
                    book.insert(index, (price, quantity));
                }
            }
        }

        self.last_update = ts;
    }
}

fn find_level_index(
    book: &[(f64, f64)],
    price: f64,
    descending: bool,
) -> Result<usize, usize> {
    let search = book.binary_search_by(|(level_price, _)| {
        if descending {
            level_price.total_cmp(&price).reverse()
        } else {
            level_price.total_cmp(&price)
        }
    });

    // `total_cmp` is bitwise-exact, so floats that are representationally
    // distinct but within PRICE_EPSILON (e.g. due to f64 rounding in
    // upstream data) would not match via binary search.  The epsilon
    // fallback checks the two neighbours of the insertion point — the
    // only positions that could be within epsilon in a sorted book.
    match search {
        Ok(index) => Ok(index),
        Err(index) => {
            if index > 0 && (book[index - 1].0 - price).abs() < PRICE_EPSILON {
                Ok(index - 1)
            } else if index < book.len() && (book[index].0 - price).abs() < PRICE_EPSILON {
                Ok(index)
            } else {
                Err(index)
            }
        }
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

#[allow(dead_code)]
pub struct SimulatedVenue {
    pub venue: Venue,

    books: HashMap<InstrumentId, SimulatedBook>,

    pending_orders: HashMap<OrderId, SimulatedOrder>,

    fill_config: FillModelConfig,

    slippage_fraction: f64,

    /// Commission rate in decimal (e.g., 0.001 = 0.1%)
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
            slippage_fraction: fill_config.slippage_bps as f64 / 10_000.0,
            fill_config,
            commission_rate,
            order_sequence: 0,
            pending_events: VecDeque::new(),
        }
    }

    pub fn get_or_create_book(&mut self, instrument_id: &InstrumentId) -> &mut SimulatedBook {
        self.books
            .entry(instrument_id.clone())
            .or_insert_with(|| SimulatedBook::new(instrument_id.clone()))
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
                let bids = [(*bid_price, *bid_size)];
                let asks = [(*ask_price, *ask_size)];
                book.update_snapshot_from_slices(&bids, &asks, ts);
            }
            HistoricalData::BookSnapshot {
                instrument_id,
                bids,
                asks,
            } => {
                let book = self.get_or_create_book(instrument_id);
                book.update_snapshot_from_slices(bids, asks, ts);
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

        let fill_price = match order.side {
            OrderSide::Buy => price * (1.0 + self.slippage_fraction),
            OrderSide::Sell => price * (1.0 - self.slippage_fraction),
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

                let fill_price = match order.side {
                    OrderSide::Buy => trade_price * (1.0 + self.slippage_fraction),
                    OrderSide::Sell => trade_price * (1.0 - self.slippage_fraction),
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

    pub fn pop_event(&mut self) -> Option<TradingEvent> {
        self.pending_events.pop_front()
    }
}
