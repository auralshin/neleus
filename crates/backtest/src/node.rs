use crate::config::{BacktestConfig, LatencySimulator};
use crate::datafeed::{DataFeed, HistoricalData, HistoricalDataPoint};
use crate::simulation::SimulatedVenue;
use neleus_core_bus::InMemoryBus;
use neleus_core_engine::{
    ClockMode, Engine, EngineConfig, MarketDataEvent, OrderSide, StrategyCommand, StrategyHandler,
    TradingEvent,
};
use neleus_core_types::{InstrumentId, OrderId, UnixNanos, Venue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

    total_realized_pnl: f64,

    total_unrealized_pnl: f64,

    positions: HashMap<InstrumentId, PositionTracker>,

    /// Map order_id -> (instrument_id, side) for fill processing
    order_info: HashMap<OrderId, (InstrumentId, OrderSide)>,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
struct PositionTracker {
    quantity: f64,
    avg_entry: f64,
    realized_pnl: f64,
    unrealized_pnl: f64,
    last_price: f64,
}

impl PositionTracker {
    fn new() -> Self {
        Self::default()
    }

    /// Update position on fill, returns realized P&L from this trade
    fn on_fill(&mut self, side: OrderSide, fill_price: f64, fill_quantity: f64) -> f64 {
        let signed_qty = match side {
            OrderSide::Buy => fill_quantity,
            OrderSide::Sell => -fill_quantity,
        };

        let mut realized_pnl = 0.0;

        // Check if this trade reduces or closes position
        let is_reducing =
            (self.quantity > 0.0 && signed_qty < 0.0) || (self.quantity < 0.0 && signed_qty > 0.0);

        if is_reducing {
            // Calculate realized P&L for closing portion
            let closing_qty = signed_qty.abs().min(self.quantity.abs());
            if self.quantity > 0.0 {
                // Long position being reduced
                realized_pnl = closing_qty * (fill_price - self.avg_entry);
            } else {
                // Short position being reduced
                realized_pnl = closing_qty * (self.avg_entry - fill_price);
            }

            self.realized_pnl += realized_pnl;

            // Check if position flips
            let new_quantity = self.quantity + signed_qty;
            if (self.quantity > 0.0 && new_quantity < 0.0)
                || (self.quantity < 0.0 && new_quantity > 0.0)
            {
                // Position flipped - reset avg entry for remaining
                self.avg_entry = fill_price;
                self.quantity = new_quantity;
            } else if new_quantity.abs() < 1e-10 {
                // Position fully closed
                self.quantity = 0.0;
                self.avg_entry = 0.0;
            } else {
                // Partial close
                self.quantity = new_quantity;
            }
        } else {
            // Adding to position - update average entry
            let new_quantity = self.quantity + signed_qty;
            if self.quantity.abs() < 1e-10 {
                // New position
                self.avg_entry = fill_price;
            } else {
                // Adding to existing position
                let total_cost = self.quantity.abs() * self.avg_entry + fill_quantity * fill_price;
                self.avg_entry = total_cost / new_quantity.abs();
            }
            self.quantity = new_quantity;
        }

        self.last_price = fill_price;
        self.update_unrealized_pnl(fill_price);

        realized_pnl
    }

    fn update_unrealized_pnl(&mut self, current_price: f64) {
        if self.quantity.abs() < 1e-10 {
            self.unrealized_pnl = 0.0;
        } else if self.quantity > 0.0 {
            self.unrealized_pnl = self.quantity * (current_price - self.avg_entry);
        } else {
            self.unrealized_pnl = self.quantity.abs() * (self.avg_entry - current_price);
        }
        self.last_price = current_price;
    }

    #[allow(dead_code)]
    fn total_pnl(&self) -> f64 {
        self.realized_pnl + self.unrealized_pnl
    }

    #[allow(dead_code)]
    fn is_flat(&self) -> bool {
        self.quantity.abs() < 1e-10
    }
}

impl BacktestNode {
    pub fn new(config: BacktestConfig) -> Self {
        let engine_config = EngineConfig {
            instance_id: "backtest".to_string(),
            max_events_per_tick: 100,
            enable_event_log: false,
            clock_mode: ClockMode::Simulated,
            capital_config: Default::default(),
            position_config: Default::default(),
            leverage_config: Default::default(),
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
            total_realized_pnl: 0.0,
            total_unrealized_pnl: 0.0,
            positions: HashMap::new(),
            order_info: HashMap::new(),
            config,
        }
    }

    pub fn add_strategy(&mut self, strategy: Box<dyn StrategyHandler>) {
        self.engine.add_strategy(strategy);
    }

    pub fn set_data_feed(&mut self, feed: Box<dyn DataFeed>) {
        self.data_feed = Some(feed);
    }

    /// Get the latency simulator for configuration
    pub fn latency_simulator(&self) -> &LatencySimulator {
        &self.latency_sim
    }

    pub fn run(&mut self) -> &BacktestResults {
        self.engine.start();
        self.current_time = self.config.start_time;

        while let Some(data_point) = self.next_data_point() {
            if data_point.timestamp > self.config.end_time {
                break;
            }

            let HistoricalDataPoint {
                timestamp,
                sequence: _,
                data,
            } = data_point;

            self.current_time = timestamp;
            self.engine.advance_time(self.current_time);

            self.venue.on_data(&data, self.current_time);

            if let Some(market_event) = Self::into_market_event(data, self.current_time) {
                let commands = self.engine.on_market_data(market_event);
                self.process_strategy_commands(commands);
            }

            let timer_commands = self.engine.tick_collect_commands();
            self.process_strategy_commands(timer_commands);

            while let Some(event) = self.venue.pop_event() {
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

    fn into_market_event(data: HistoricalData, ts: UnixNanos) -> Option<MarketDataEvent> {
        match data {
            HistoricalData::Trade {
                instrument_id,
                price,
                quantity,
                side,
            } => Some(MarketDataEvent::Trade {
                instrument_id,
                price,
                quantity,
                side,
                ts,
            }),
            HistoricalData::Quote {
                instrument_id,
                bid_price,
                bid_size,
                ask_price,
                ask_size,
            } => Some(MarketDataEvent::Quote {
                instrument_id,
                bid_price,
                bid_size,
                ask_price,
                ask_size,
                ts,
            }),
            HistoricalData::BookSnapshot {
                instrument_id,
                bids,
                asks,
            } => Some(MarketDataEvent::BookUpdate {
                instrument_id,
                bids,
                asks,
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
                instrument_id,
                price: close,
                quantity: volume,
                side: OrderSide::Buy,
                ts,
            }),
            HistoricalData::BookDelta { .. } => None,
        }
    }

    fn process_strategy_commands(&mut self, commands: Vec<StrategyCommand>) {
        for cmd in commands {
            match &cmd {
                StrategyCommand::SubmitOrder {
                    order_id,
                    instrument_id,
                    side,
                    ..
                } => {
                    // Track order info for fill processing
                    self.order_info
                        .insert(order_id.clone(), (instrument_id.clone(), *side));
                    self.venue.submit_order(&cmd, self.current_time);
                }
                StrategyCommand::CancelOrder { order_id } => {
                    self.order_info.remove(order_id);
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
                order_id,
                fill_price,
                fill_quantity,
                remaining_quantity,
                ts,
            } => {
                let commission = fill_price * fill_quantity * self.config.commission_rate;
                self.results.total_commission += commission;
                self.results.total_volume += fill_price * fill_quantity;

                // Look up order info and update position
                let trade_pnl = if let Some((instrument_id, side)) = self.order_info.get(order_id) {
                    let position = self
                        .positions
                        .entry(instrument_id.clone())
                        .or_insert_with(PositionTracker::new);
                    let previous_unrealized = position.unrealized_pnl;

                    let realized_pnl = position.on_fill(*side, *fill_price, *fill_quantity);
                    self.total_realized_pnl += realized_pnl;
                    self.total_unrealized_pnl += position.unrealized_pnl - previous_unrealized;

                    // Remove order info if fully filled
                    if *remaining_quantity < 1e-10 {
                        self.order_info.remove(order_id);
                    }

                    realized_pnl
                } else {
                    0.0
                };

                self.current_equity = self.config.initial_balance
                    + self.total_realized_pnl
                    + self.total_unrealized_pnl
                    - self.results.total_commission;

                // Record trade
                if trade_pnl > 0.0 {
                    self.results.winning_trades += 1;
                } else if trade_pnl < 0.0 {
                    self.results.losing_trades += 1;
                }

                self.results
                    .record_trade(trade_pnl, *ts, self.current_equity);
            }
            TradingEvent::PositionUpdate {
                instrument_id,
                quantity: _,
                avg_price: _,
                unrealized_pnl,
                ts: _,
            } => {
                if let Some(position) = self.positions.get_mut(instrument_id) {
                    self.total_unrealized_pnl += unrealized_pnl - position.unrealized_pnl;
                    position.unrealized_pnl = *unrealized_pnl;
                    self.current_equity = self.config.initial_balance
                        + self.total_realized_pnl
                        + self.total_unrealized_pnl
                        - self.results.total_commission;
                }
            }
            _ => {}
        }
    }

    /// Update all position unrealized P&L based on current market prices
    #[allow(dead_code)]
    fn update_mark_to_market(&mut self, instrument_id: &InstrumentId, price: f64) {
        if let Some(position) = self.positions.get_mut(instrument_id) {
            let previous_unrealized = position.unrealized_pnl;
            position.update_unrealized_pnl(price);
            self.total_unrealized_pnl += position.unrealized_pnl - previous_unrealized;
        }
        self.current_equity = self.config.initial_balance
            + self.total_realized_pnl
            + self.total_unrealized_pnl
            - self.results.total_commission;
    }

    fn finalize_results(&mut self) {
        self.results.data_points_processed = self.data_count;
        self.results.end_time = self.current_time;
        self.results.total_pnl =
            self.total_realized_pnl + self.total_unrealized_pnl - self.results.total_commission;
        self.results.final_balance = self.config.initial_balance + self.results.total_pnl;
        self.current_equity = self.results.final_balance;
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
