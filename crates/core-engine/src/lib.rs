use neleus_core_bus::{Bus, BusConfig, EventSink, InMemoryBus, Message, MessageKind, Topic};
use neleus_core_types::{InstrumentId, OrderId, SequenceNumber, StrategyId, UnixNanos, Venue};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

// Export new modules
pub mod advanced_risk;
pub mod circuit_breaker;
pub mod execution;
pub mod portfolio;

pub use advanced_risk::*;
pub use circuit_breaker::*;
pub use execution::*;
pub use portfolio::*;

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub instance_id: String,

    pub max_events_per_tick: usize,

    pub enable_event_log: bool,

    pub clock_mode: ClockMode,

    /// Capital management configuration
    pub capital_config: CapitalConfig,

    /// Position management configuration
    pub position_config: PositionManagementConfig,

    /// Leverage and margin configuration
    pub leverage_config: LeverageConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            instance_id: "neleus-1".to_string(),
            max_events_per_tick: 1000,
            enable_event_log: true,
            clock_mode: ClockMode::Live,
            capital_config: CapitalConfig::default(),
            position_config: PositionManagementConfig::default(),
            leverage_config: LeverageConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockMode {
    Live,

    Simulated,
}

/// Capital management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapitalConfig {
    /// Initial capital (quote currency)
    pub initial_capital: f64,

    /// Unit spend per trade (for fixed allocation)
    pub unit_spend: f64,

    /// Whether to lock profits (don't risk realized gains)
    pub lock_profits: bool,

    /// Automatically redeploy profits into new positions
    pub auto_redeploy_profits: bool,

    /// Percentage of profits to redeploy (0.0 - 1.0)
    pub profit_redeploy_percentage: f64,

    /// Minimum capital threshold before redeployment
    pub min_capital_for_redeploy: f64,

    /// Maximum capital allocation per position (as fraction of total)
    pub max_capital_per_position: f64,

    /// Reserve capital percentage (keep uninvested)
    pub reserve_capital_pct: f64,
}

impl Default for CapitalConfig {
    fn default() -> Self {
        Self {
            initial_capital: 100000.0,
            unit_spend: 1000.0,
            lock_profits: false,
            auto_redeploy_profits: true,
            profit_redeploy_percentage: 0.5,
            min_capital_for_redeploy: 100.0,
            max_capital_per_position: 0.1,
            reserve_capital_pct: 0.05,
        }
    }
}

/// Position management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionManagementConfig {
    /// Maximum number of open positions
    pub max_open_positions: usize,

    /// Maximum positions per instrument
    pub max_positions_per_instrument: usize,

    /// Maximum positions per venue
    pub max_positions_per_venue: HashMap<Venue, usize>,

    /// Holding period constraints
    pub holding_period: HoldingPeriodConfig,

    /// Allow pyramiding (adding to winning positions)
    pub allow_pyramiding: bool,

    /// Maximum pyramid levels
    pub max_pyramid_levels: usize,

    /// Pyramid scale factor (reduce size on each add)
    pub pyramid_scale_factor: f64,

    /// Enable spread trading
    pub enable_spread_trading: bool,

    /// Spread trading configuration
    pub spread_config: SpreadTradingConfig,

    /// Close all positions at end of session
    pub close_at_session_end: bool,

    /// Session end time (in seconds from midnight UTC)
    pub session_end_time: Option<u32>,
}

impl Default for PositionManagementConfig {
    fn default() -> Self {
        Self {
            max_open_positions: 10,
            max_positions_per_instrument: 1,
            max_positions_per_venue: HashMap::new(),
            holding_period: HoldingPeriodConfig::default(),
            allow_pyramiding: false,
            max_pyramid_levels: 3,
            pyramid_scale_factor: 0.5,
            enable_spread_trading: false,
            spread_config: SpreadTradingConfig::default(),
            close_at_session_end: false,
            session_end_time: None,
        }
    }
}

/// Holding period configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldingPeriodConfig {
    /// Enable holding period constraints
    pub enabled: bool,

    /// Minimum holding period in seconds
    pub min_holding_seconds: u64,

    /// Maximum holding period in seconds (0 = unlimited)
    pub max_holding_seconds: u64,

    /// Force close at max holding period
    pub force_close_at_max: bool,

    /// Minimum holding period for profit-taking
    pub min_holding_for_profit: u64,
}

impl Default for HoldingPeriodConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_holding_seconds: 60,
            max_holding_seconds: 86400,
            force_close_at_max: false,
            min_holding_for_profit: 0,
        }
    }
}

/// Spread trading configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadTradingConfig {
    /// Enable spread trading
    pub enabled: bool,

    /// Spread pairs (leg1, leg2, ratio)
    pub pairs: Vec<(InstrumentId, InstrumentId, f64)>,

    /// Hedge ratio calculation method
    pub hedge_ratio_method: HedgeRatioMethod,

    /// Rebalance threshold (as fraction of hedge ratio)
    pub rebalance_threshold: f64,

    /// Maximum leg imbalance percentage
    pub max_leg_imbalance: f64,
}

impl Default for SpreadTradingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            pairs: Vec::new(),
            hedge_ratio_method: HedgeRatioMethod::Fixed,
            rebalance_threshold: 0.05,
            max_leg_imbalance: 0.1,
        }
    }
}

/// Hedge ratio calculation method for spread trading
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HedgeRatioMethod {
    /// Fixed ratio (specified in config)
    Fixed,
    /// Price ratio (current prices)
    PriceRatio,
    /// Beta-based (regression)
    Beta,
    /// Volatility-adjusted
    VolatilityAdjusted,
}

/// Leverage and margin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeverageConfig {
    /// Enable leverage trading
    pub enabled: bool,

    /// Maximum leverage multiplier
    pub max_leverage: f64,

    /// Maintenance margin percentage
    pub maintenance_margin_pct: f64,

    /// Initial margin percentage
    pub initial_margin_pct: f64,

    /// Borrow rate (annual percentage)
    pub borrow_rate_annual: f64,

    /// Funding interval in seconds (for perpetual futures)
    pub funding_interval_seconds: u64,

    /// Liquidation buffer (close before actual liquidation)
    pub liquidation_buffer_pct: f64,

    /// Margin call threshold (as percentage of maintenance margin)
    pub margin_call_threshold: f64,

    /// Auto-reduce positions on margin call
    pub auto_reduce_on_margin_call: bool,
}

impl Default for LeverageConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_leverage: 1.0,
            maintenance_margin_pct: 0.05,
            initial_margin_pct: 0.10,
            borrow_rate_annual: 0.05,
            funding_interval_seconds: 28800,
            liquidation_buffer_pct: 0.02,
            margin_call_threshold: 1.2,
            auto_reduce_on_margin_call: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineState {
    Idle,

    Starting,

    Running,

    Stopping,

    Stopped,

    Paused,

    Faulted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineSnapshot {
    pub instance_id: String,
    pub state: EngineState,
    pub timestamp: UnixNanos,
    pub processed_ticks: u64,
    pub processed_messages: u64,
    pub strategy_count: usize,
    pub subscription_count: usize,
    pub last_tick_duration_micros: u64,
}

impl Default for EngineSnapshot {
    fn default() -> Self {
        Self {
            instance_id: "unknown".to_string(),
            state: EngineState::Idle,
            timestamp: UnixNanos::ZERO,
            processed_ticks: 0,
            processed_messages: 0,
            strategy_count: 0,
            subscription_count: 0,
            last_tick_duration_micros: 0,
        }
    }
}

pub trait TelemetrySink: Send + Sync {
    fn on_engine_snapshot(&self, snapshot: EngineSnapshot);
    fn on_bus_stats(&self, stats: neleus_core_bus::BusStats, pending: usize);
}

pub struct Clock {
    mode: ClockMode,

    simulated_time: UnixNanos,
}

impl Clock {
    pub fn new(mode: ClockMode) -> Self {
        Self {
            mode,
            simulated_time: UnixNanos::ZERO,
        }
    }

    pub fn now(&self) -> UnixNanos {
        match self.mode {
            ClockMode::Live => UnixNanos::now(),
            ClockMode::Simulated => self.simulated_time,
        }
    }

    pub fn advance_to(&mut self, time: UnixNanos) {
        if self.mode == ClockMode::Simulated && time > self.simulated_time {
            self.simulated_time = time;
        }
    }

    pub fn set_time(&mut self, time: UnixNanos) {
        if self.mode == ClockMode::Simulated {
            self.simulated_time = time;
        }
    }
}

pub trait StrategyHandler: Send {
    fn id(&self) -> &StrategyId;

    fn on_start(&mut self, ctx: &mut StrategyContext);

    fn on_stop(&mut self, ctx: &mut StrategyContext);

    fn on_data(&mut self, ctx: &mut StrategyContext, data: &MarketDataEvent);

    fn on_event(&mut self, ctx: &mut StrategyContext, event: &TradingEvent);

    fn on_timer(&mut self, ctx: &mut StrategyContext, timer_name: &str);
}

pub struct StrategyContext {
    pub time: UnixNanos,

    commands: Vec<StrategyCommand>,

    subscriptions: Vec<DataSubscription>,

    timers: Vec<Timer>,
}

impl StrategyContext {
    pub fn new(time: UnixNanos) -> Self {
        Self {
            time,
            commands: Vec::new(),
            subscriptions: Vec::new(),
            timers: Vec::new(),
        }
    }

    pub fn submit_market_order(
        &mut self,
        instrument_id: InstrumentId,
        side: OrderSide,
        quantity: f64,
    ) -> OrderId {
        let order_id = OrderId::generate();
        self.commands.push(StrategyCommand::SubmitOrder {
            order_id: order_id.clone(),
            instrument_id,
            side,
            order_type: OrderType::Market,
            price: None,
            quantity,
        });
        order_id
    }

    pub fn submit_limit_order(
        &mut self,
        instrument_id: InstrumentId,
        side: OrderSide,
        price: f64,
        quantity: f64,
    ) -> OrderId {
        let order_id = OrderId::generate();
        self.commands.push(StrategyCommand::SubmitOrder {
            order_id: order_id.clone(),
            instrument_id,
            side,
            order_type: OrderType::Limit,
            price: Some(price),
            quantity,
        });
        order_id
    }

    pub fn cancel_order(&mut self, order_id: OrderId) {
        self.commands
            .push(StrategyCommand::CancelOrder { order_id });
    }

    pub fn subscribe_trades(&mut self, instrument_id: InstrumentId) {
        self.subscriptions
            .push(DataSubscription::Trades { instrument_id });
    }

    pub fn subscribe_book(&mut self, instrument_id: InstrumentId, depth: u32) {
        self.subscriptions.push(DataSubscription::OrderBook {
            instrument_id,
            depth,
        });
    }

    pub fn subscribe_quotes(&mut self, instrument_id: InstrumentId) {
        self.subscriptions
            .push(DataSubscription::Quotes { instrument_id });
    }

    pub fn schedule_timer(&mut self, name: String, interval_ms: u64) {
        self.timers.push(Timer {
            name,
            interval_ms,
            next_fire: self.time + UnixNanos::from_millis(interval_ms),
        });
    }

    pub fn drain_commands(&mut self) -> Vec<StrategyCommand> {
        std::mem::take(&mut self.commands)
    }

    pub fn drain_subscriptions(&mut self) -> Vec<DataSubscription> {
        std::mem::take(&mut self.subscriptions)
    }

    pub fn drain_timers(&mut self) -> Vec<Timer> {
        std::mem::take(&mut self.timers)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrategyCommand {
    SubmitOrder {
        order_id: OrderId,
        instrument_id: InstrumentId,
        side: OrderSide,
        order_type: OrderType,
        price: Option<f64>,
        quantity: f64,
    },
    CancelOrder {
        order_id: OrderId,
    },
    ModifyOrder {
        order_id: OrderId,
        new_price: Option<f64>,
        new_quantity: Option<f64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DataSubscription {
    Trades {
        instrument_id: InstrumentId,
    },
    OrderBook {
        instrument_id: InstrumentId,
        depth: u32,
    },
    Quotes {
        instrument_id: InstrumentId,
    },
    Bars {
        instrument_id: InstrumentId,
        interval_secs: u64,
    },
}

#[derive(Debug, Clone)]
pub struct Timer {
    pub name: String,
    pub interval_ms: u64,
    pub next_fire: UnixNanos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    Market,
    Limit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketDataEvent {
    Trade {
        instrument_id: InstrumentId,
        price: f64,
        quantity: f64,
        side: OrderSide,
        ts: UnixNanos,
    },
    Quote {
        instrument_id: InstrumentId,
        bid_price: f64,
        bid_size: f64,
        ask_price: f64,
        ask_size: f64,
        ts: UnixNanos,
    },
    BookUpdate {
        instrument_id: InstrumentId,
        bids: Vec<(f64, f64)>,
        asks: Vec<(f64, f64)>,
        ts: UnixNanos,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradingEvent {
    OrderSubmitted {
        order_id: OrderId,
        ts: UnixNanos,
    },
    OrderAccepted {
        order_id: OrderId,
        venue_order_id: String,
        ts: UnixNanos,
    },
    OrderRejected {
        order_id: OrderId,
        reason: String,
        ts: UnixNanos,
    },
    OrderFilled {
        order_id: OrderId,
        fill_price: f64,
        fill_quantity: f64,
        remaining_quantity: f64,
        ts: UnixNanos,
    },
    OrderCanceled {
        order_id: OrderId,
        ts: UnixNanos,
    },
    PositionUpdate {
        instrument_id: InstrumentId,
        quantity: f64,
        avg_price: f64,
        unrealized_pnl: f64,
        ts: UnixNanos,
    },
}

pub struct Engine<B: Bus> {
    config: EngineConfig,

    bus: B,

    clock: Clock,

    state: EngineState,

    sequence: SequenceNumber,

    strategies: Vec<Box<dyn StrategyHandler>>,

    timers: HashMap<StrategyId, Vec<Timer>>,

    subscriptions: Vec<DataSubscription>,

    telemetry: Option<Arc<dyn TelemetrySink>>,

    /// Optional risk manager for pre-trade risk checks
    risk_manager: Option<DynamicLimitManager>,

    /// Track current daily P&L for risk checks
    current_daily_pnl: f64,

    /// Track current leverage for risk checks
    current_leverage: f64,

    /// Position tracking engine for position limits enforcement
    position_engine: PositionEngine,

    processed_ticks: u64,

    processed_messages: u64,

    last_tick_duration_micros: u64,
}

impl Engine<InMemoryBus> {
    pub fn new(config: EngineConfig) -> Self {
        let clock_mode = config.clock_mode;
        let mut bus_config = BusConfig::default();
        bus_config.enable_logging = config.enable_event_log;
        Self {
            config,
            bus: InMemoryBus::with_config(bus_config),
            clock: Clock::new(clock_mode),
            state: EngineState::Idle,
            sequence: SequenceNumber::default(),
            strategies: Vec::new(),
            timers: HashMap::new(),
            subscriptions: Vec::new(),
            telemetry: None,
            risk_manager: None,
            current_daily_pnl: 0.0,
            current_leverage: 0.0,
            position_engine: PositionEngine::new(),
            processed_ticks: 0,
            processed_messages: 0,
            last_tick_duration_micros: 0,
        }
    }

    /// Create engine with persistence via EventSink
    pub fn with_event_sink(config: EngineConfig, event_sink: Arc<dyn EventSink>) -> Self {
        let clock_mode = config.clock_mode;
        let mut bus_config = BusConfig::default();
        bus_config.enable_logging = config.enable_event_log;
        bus_config.event_sink = Some(event_sink);
        Self {
            config,
            bus: InMemoryBus::with_config(bus_config),
            clock: Clock::new(clock_mode),
            state: EngineState::Idle,
            sequence: SequenceNumber::default(),
            strategies: Vec::new(),
            timers: HashMap::new(),
            subscriptions: Vec::new(),
            telemetry: None,
            risk_manager: None,
            current_daily_pnl: 0.0,
            current_leverage: 0.0,
            position_engine: PositionEngine::new(),
            processed_ticks: 0,
            processed_messages: 0,
            last_tick_duration_micros: 0,
        }
    }
}

impl<B: Bus> Engine<B> {
    pub fn with_bus(config: EngineConfig, bus: B) -> Self {
        let clock_mode = config.clock_mode;
        Self {
            config,
            bus,
            clock: Clock::new(clock_mode),
            state: EngineState::Idle,
            sequence: SequenceNumber::default(),
            strategies: Vec::new(),
            timers: HashMap::new(),
            subscriptions: Vec::new(),
            telemetry: None,
            risk_manager: None,
            current_daily_pnl: 0.0,
            current_leverage: 0.0,
            position_engine: PositionEngine::new(),
            processed_ticks: 0,
            processed_messages: 0,
            last_tick_duration_micros: 0,
        }
    }

    /// Get position engine reference for external position queries
    pub fn position_engine(&self) -> &PositionEngine {
        &self.position_engine
    }

    /// Get mutable position engine reference
    pub fn position_engine_mut(&mut self) -> &mut PositionEngine {
        &mut self.position_engine
    }

    /// Set risk manager for pre-trade risk checks
    pub fn set_risk_manager(&mut self, config: DynamicLimitsConfig, initial_equity: f64) {
        self.risk_manager = Some(DynamicLimitManager::new(config, initial_equity));
    }

    /// Update daily P&L for risk checks
    pub fn update_daily_pnl(&mut self, pnl: f64) {
        self.current_daily_pnl = pnl;
    }

    /// Update current leverage for risk checks
    pub fn update_leverage(&mut self, leverage: f64) {
        self.current_leverage = leverage;
    }

    pub fn attach_telemetry(&mut self, telemetry: Arc<dyn TelemetrySink>) {
        self.telemetry = Some(telemetry);
        self.emit_telemetry();
    }

    pub fn state(&self) -> EngineState {
        self.state
    }

    pub fn time(&self) -> UnixNanos {
        self.clock.now()
    }

    pub fn add_strategy(&mut self, strategy: Box<dyn StrategyHandler>) {
        let id = strategy.id().clone();
        self.timers.insert(id, Vec::new());
        self.strategies.push(strategy);
    }

    pub fn start(&mut self) {
        if self.state != EngineState::Idle && self.state != EngineState::Stopped {
            return;
        }

        self.state = EngineState::Starting;

        for strategy in &mut self.strategies {
            let mut ctx = StrategyContext::new(self.clock.now());
            strategy.on_start(&mut ctx);

            for sub in ctx.drain_subscriptions() {
                self.subscriptions.push(sub);
            }

            let id = strategy.id().clone();
            for timer in ctx.drain_timers() {
                if let Some(timers) = self.timers.get_mut(&id) {
                    timers.push(timer);
                }
            }
        }

        self.state = EngineState::Running;
        tracing::info!(
            engine_id = %self.config.instance_id,
            "engine started"
        );
        self.emit_telemetry();
    }

    pub fn stop(&mut self) {
        if self.state != EngineState::Running {
            return;
        }

        self.state = EngineState::Stopping;

        for strategy in &mut self.strategies {
            let mut ctx = StrategyContext::new(self.clock.now());
            strategy.on_stop(&mut ctx);
        }

        self.state = EngineState::Stopped;
        tracing::info!(
            engine_id = %self.config.instance_id,
            "engine stopped"
        );
        self.emit_telemetry();
    }

    pub fn is_running(&self) -> bool {
        self.state == EngineState::Running
    }

    pub fn tick(&mut self) -> usize {
        if !self.is_running() {
            return 0;
        }

        let start = std::time::Instant::now();
        let mut processed = 0;
        let current_time = self.clock.now();

        self.process_timers(current_time);

        for _ in 0..self.config.max_events_per_tick {
            if let Some(message) = self.bus.poll() {
                self.process_message(message);
                processed += 1;
            } else {
                break;
            }
        }

        self.processed_ticks += 1;
        self.processed_messages += processed as u64;
        self.last_tick_duration_micros = start.elapsed().as_micros() as u64;
        self.emit_telemetry();

        processed
    }

    #[inline]
    fn emit_telemetry(&self) {
        if let Some(telemetry) = &self.telemetry {
            let snapshot = EngineSnapshot {
                instance_id: self.config.instance_id.clone(),
                state: self.state,
                timestamp: self.clock.now(),
                processed_ticks: self.processed_ticks,
                processed_messages: self.processed_messages,
                strategy_count: self.strategies.len(),
                subscription_count: self.subscriptions.len(),
                last_tick_duration_micros: self.last_tick_duration_micros,
            };
            telemetry.on_engine_snapshot(snapshot);
            telemetry.on_bus_stats(self.bus.stats(), self.bus.pending_count());
        }
    }

    fn process_timers(&mut self, current_time: UnixNanos) {
        let mut all_commands = Vec::new();

        for strategy in &mut self.strategies {
            let id = strategy.id().clone();
            let mut ctx = StrategyContext::new(current_time);

            if let Some(timers) = self.timers.get_mut(&id) {
                for timer in timers.iter_mut() {
                    if current_time >= timer.next_fire {
                        strategy.on_timer(&mut ctx, &timer.name);
                        timer.next_fire = current_time + UnixNanos::from_millis(timer.interval_ms);
                    }
                }
            }

            all_commands.extend(ctx.drain_commands());
        }

        self.process_strategy_commands(all_commands);
    }

    pub fn tick_collect_commands(&mut self) -> Vec<StrategyCommand> {
        if !self.is_running() {
            return Vec::new();
        }

        let current_time = self.clock.now();
        let mut all_commands = Vec::new();

        for strategy in &mut self.strategies {
            let id = strategy.id().clone();
            let mut ctx = StrategyContext::new(current_time);

            if let Some(timers) = self.timers.get_mut(&id) {
                for timer in timers.iter_mut() {
                    if current_time >= timer.next_fire {
                        strategy.on_timer(&mut ctx, &timer.name);
                        timer.next_fire = current_time + UnixNanos::from_millis(timer.interval_ms);
                    }
                }
            }

            all_commands.extend(ctx.drain_commands());
        }

        for _ in 0..self.config.max_events_per_tick {
            if self.bus.poll().is_none() {
                break;
            }
        }

        all_commands
    }

    pub fn on_market_data(&mut self, event: MarketDataEvent) -> Vec<StrategyCommand> {
        let current_time = self.clock.now();
        let mut all_commands = Vec::new();

        for strategy in &mut self.strategies {
            let mut ctx = StrategyContext::new(current_time);
            strategy.on_data(&mut ctx, &event);
            all_commands.extend(ctx.drain_commands());
        }

        all_commands
    }

    pub fn on_trading_event(&mut self, event: &TradingEvent) -> Vec<StrategyCommand> {
        let current_time = self.clock.now();
        let mut all_commands = Vec::new();

        for strategy in &mut self.strategies {
            let mut ctx = StrategyContext::new(current_time);
            strategy.on_event(&mut ctx, event);
            all_commands.extend(ctx.drain_commands());
        }

        all_commands
    }

    #[inline]
    fn process_message(&mut self, message: Message) {
        self.sequence = self.sequence.next();
        self.processed_messages += 1;

        match message.kind {
            MessageKind::Data => {
                // Try to parse as MarketDataEvent
                if let Ok(payload_str) = std::str::from_utf8(&message.payload) {
                    if let Ok(event) = serde_json::from_str::<MarketDataEvent>(payload_str) {
                        let commands = self.on_market_data(event);
                        self.process_strategy_commands(commands);
                    } else {
                        tracing::trace!("Received non-MarketDataEvent data message");
                    }
                }
            }
            MessageKind::Event => {
                // Try to parse as TradingEvent
                if let Ok(payload_str) = std::str::from_utf8(&message.payload) {
                    if let Ok(event) = serde_json::from_str::<TradingEvent>(payload_str) {
                        // Update internal position tracking for fills
                        if let TradingEvent::OrderFilled {
                            order_id: _,
                            fill_price,
                            fill_quantity,
                            remaining_quantity: _,
                            ts: _,
                        } = &event
                        {
                            // Track P&L from fills
                            let trade_value = fill_price * fill_quantity;
                            tracing::debug!(trade_value = trade_value, "Processed order fill");
                        }

                        let commands = self.on_trading_event(&event);
                        self.process_strategy_commands(commands);
                    } else {
                        tracing::trace!("Received non-TradingEvent event message");
                    }
                }
            }
            MessageKind::Command => {
                // Commands from external sources - parse and execute
                if let Ok(payload_str) = std::str::from_utf8(&message.payload) {
                    if let Ok(cmd) = serde_json::from_str::<StrategyCommand>(payload_str) {
                        self.process_strategy_commands(vec![cmd]);
                    } else {
                        tracing::trace!("Received unparseable command message");
                    }
                }
            }
            MessageKind::System => {
                // System messages for lifecycle management
                if let Ok(payload_str) = std::str::from_utf8(&message.payload) {
                    tracing::info!(system_message = payload_str, "System message received");
                    // Handle specific system commands
                    if payload_str.contains("shutdown") {
                        self.stop();
                    } else if payload_str.contains("pause") {
                        self.state = EngineState::Paused;
                    } else if payload_str.contains("resume") && self.state == EngineState::Paused {
                        self.state = EngineState::Running;
                    }
                }
            }
        }
    }

    fn process_strategy_commands(&mut self, commands: Vec<StrategyCommand>) {
        for cmd in commands {
            // Clone and potentially modify the command
            let mut cmd = cmd;

            // Check position and risk limits for order submissions
            if let StrategyCommand::SubmitOrder {
                ref order_id,
                ref instrument_id,
                ref mut quantity,
                ref price,
                ref side,
                ..
            } = cmd
            {
                // === Position Management Checks ===
                let position_config = &self.config.position_config;

                // Check max open positions
                let current_position_count = self.position_engine.position_count();
                let existing_position = self.position_engine.get_position(instrument_id);
                let is_new_position = existing_position.map_or(true, |p| p.is_flat());

                if is_new_position && current_position_count >= position_config.max_open_positions {
                    tracing::error!(
                        order_id = %order_id,
                        instrument = %instrument_id,
                        current = current_position_count,
                        max = position_config.max_open_positions,
                        "Position limit: max open positions reached"
                    );
                    let reject_event = TradingEvent::OrderRejected {
                        order_id: order_id.clone(),
                        reason: format!(
                            "Max open positions limit reached ({}/{})",
                            current_position_count, position_config.max_open_positions
                        ),
                        ts: self.clock.now(),
                    };
                    let payload = format!("{:?}", reject_event).into_bytes();
                    let msg = Message::event(Topic::order_events(), payload);
                    self.bus.publish(msg);
                    continue;
                }

                // Check max positions per venue
                if let Some(&max_venue_positions) = position_config
                    .max_positions_per_venue
                    .get(&instrument_id.venue)
                {
                    let venue_count = self.position_engine.positions_by_venue(instrument_id.venue);
                    if is_new_position && venue_count >= max_venue_positions {
                        tracing::error!(
                            order_id = %order_id,
                            venue = ?instrument_id.venue,
                            current = venue_count,
                            max = max_venue_positions,
                            "Position limit: max positions per venue reached"
                        );
                        let reject_event = TradingEvent::OrderRejected {
                            order_id: order_id.clone(),
                            reason: format!(
                                "Max positions for venue {:?} reached ({}/{})",
                                instrument_id.venue, venue_count, max_venue_positions
                            ),
                            ts: self.clock.now(),
                        };
                        let payload = format!("{:?}", reject_event).into_bytes();
                        let msg = Message::event(Topic::order_events(), payload);
                        self.bus.publish(msg);
                        continue;
                    }
                }

                // Check pyramiding limits (adding to winning position)
                if !is_new_position && !position_config.allow_pyramiding {
                    if let Some(pos) = existing_position {
                        let is_adding_to_position = (pos.quantity > 0.0 && *side == OrderSide::Buy)
                            || (pos.quantity < 0.0 && *side == OrderSide::Sell);

                        if is_adding_to_position {
                            tracing::error!(
                                order_id = %order_id,
                                instrument = %instrument_id,
                                "Position limit: pyramiding not allowed"
                            );
                            let reject_event = TradingEvent::OrderRejected {
                                order_id: order_id.clone(),
                                reason: "Pyramiding (adding to position) not allowed".to_string(),
                                ts: self.clock.now(),
                            };
                            let payload = format!("{:?}", reject_event).into_bytes();
                            let msg = Message::event(Topic::order_events(), payload);
                            self.bus.publish(msg);
                            continue;
                        }
                    }
                }

                // === Risk Manager Checks ===
                if let Some(risk_manager) = &self.risk_manager {
                    // Calculate notional value (use price if limit order, or estimate for market)
                    let notional = *quantity * price.unwrap_or(1.0);

                    match risk_manager.check_order(
                        notional,
                        self.current_daily_pnl,
                        self.current_leverage,
                    ) {
                        RiskLimitCheck::Allowed => {
                            // Order allowed, proceed
                        }
                        RiskLimitCheck::ReduceSize(factor) => {
                            // Actually reduce the order size
                            let original_qty = *quantity;
                            *quantity = original_qty * factor;
                            tracing::warn!(
                                order_id = %order_id,
                                instrument = %instrument_id,
                                original_qty = original_qty,
                                reduced_qty = *quantity,
                                factor = factor,
                                "Risk limit: order size reduced"
                            );
                        }
                        RiskLimitCheck::Rejected(reason) => {
                            // Order rejected by risk limits
                            tracing::error!(
                                order_id = %order_id,
                                instrument = %instrument_id,
                                reason = %reason,
                                "Risk limit: order rejected"
                            );
                            // Publish rejection event and skip this order
                            let reject_event = TradingEvent::OrderRejected {
                                order_id: order_id.clone(),
                                reason: format!("Risk limit: {}", reason),
                                ts: self.clock.now(),
                            };
                            let payload = format!("{:?}", reject_event).into_bytes();
                            let msg = Message::event(Topic::order_events(), payload);
                            self.bus.publish(msg);
                            continue;
                        }
                    }
                }
            }

            let payload = format!("{:?}", cmd).into_bytes();
            let msg = Message::command(Topic::commands(), payload);
            self.bus.publish(msg);
        }
    }

    pub fn publish(&mut self, message: Message) {
        self.bus.publish(message);
    }

    pub fn advance_time(&mut self, time: UnixNanos) {
        self.clock.advance_to(time);
    }

    pub fn pending_count(&self) -> usize {
        self.bus.pending_count()
    }
}

pub trait Component: Send {
    fn name(&self) -> &str;
    fn start(&mut self) -> Result<(), ComponentError>;
    fn stop(&mut self) -> Result<(), ComponentError>;
    fn is_running(&self) -> bool;
}

#[derive(Debug)]
pub enum ComponentError {
    StartFailed(String),
    StopFailed(String),
    ConnectionFailed(String),
    Other(String),
}

impl std::fmt::Display for ComponentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentError::StartFailed(msg) => write!(f, "Start failed: {}", msg),
            ComponentError::StopFailed(msg) => write!(f, "Stop failed: {}", msg),
            ComponentError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            ComponentError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for ComponentError {}

pub trait ExecutionClient: Component {
    fn venue(&self) -> Venue;
    fn submit_order(&mut self, order: &StrategyCommand) -> Result<(), ComponentError>;
    fn cancel_order(&mut self, order_id: &OrderId) -> Result<(), ComponentError>;
}

pub trait DataClient: Component {
    fn venue(&self) -> Venue;
    fn subscribe(&mut self, subscription: &DataSubscription) -> Result<(), ComponentError>;
    fn unsubscribe(&mut self, subscription: &DataSubscription) -> Result<(), ComponentError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OmsOrderState {
    PendingSubmit,

    Submitted,

    Accepted,

    PartiallyFilled,

    Filled,

    Canceled,

    Rejected,

    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmsOrder {
    pub order_id: OrderId,
    pub client_order_id: String,
    pub venue_order_id: Option<String>,
    pub instrument_id: InstrumentId,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: Option<f64>,
    pub quantity: f64,
    pub filled_quantity: f64,
    pub avg_fill_price: Option<f64>,
    pub state: OmsOrderState,
    pub created_at: UnixNanos,
    pub updated_at: UnixNanos,
    pub strategy_id: StrategyId,
    pub venue: Venue,
}

impl OmsOrder {
    pub fn remaining_quantity(&self) -> f64 {
        self.quantity - self.filled_quantity
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            OmsOrderState::PendingSubmit
                | OmsOrderState::Submitted
                | OmsOrderState::Accepted
                | OmsOrderState::PartiallyFilled
        )
    }

    pub fn is_closed(&self) -> bool {
        matches!(
            self.state,
            OmsOrderState::Filled
                | OmsOrderState::Canceled
                | OmsOrderState::Rejected
                | OmsOrderState::Expired
        )
    }
}

pub struct OrderManagementSystem {
    orders: HashMap<OrderId, OmsOrder>,

    active_by_instrument: HashMap<InstrumentId, Vec<OrderId>>,

    orders_by_strategy: HashMap<StrategyId, Vec<OrderId>>,

    cloid_to_oid: HashMap<String, OrderId>,

    venue_oid_to_oid: HashMap<String, OrderId>,

    order_sequence: u64,
}

impl OrderManagementSystem {
    pub fn new() -> Self {
        Self {
            orders: HashMap::new(),
            active_by_instrument: HashMap::new(),
            orders_by_strategy: HashMap::new(),
            cloid_to_oid: HashMap::new(),
            venue_oid_to_oid: HashMap::new(),
            order_sequence: 0,
        }
    }

    pub fn generate_client_order_id(&mut self) -> String {
        self.order_sequence += 1;
        format!("CLOID-{:016x}", self.order_sequence)
    }

    pub fn create_order(
        &mut self,
        instrument_id: InstrumentId,
        side: OrderSide,
        order_type: OrderType,
        price: Option<f64>,
        quantity: f64,
        strategy_id: StrategyId,
        now: UnixNanos,
    ) -> OmsOrder {
        let order_id = OrderId::generate();
        let client_order_id = self.generate_client_order_id();

        let order = OmsOrder {
            order_id: order_id.clone(),
            client_order_id: client_order_id.clone(),
            venue_order_id: None,
            instrument_id: instrument_id.clone(),
            venue: instrument_id.venue,
            side,
            order_type,
            price,
            quantity,
            filled_quantity: 0.0,
            avg_fill_price: None,
            state: OmsOrderState::PendingSubmit,
            created_at: now,
            updated_at: now,
            strategy_id: strategy_id.clone(),
        };

        self.orders.insert(order_id.clone(), order.clone());
        self.cloid_to_oid.insert(client_order_id, order_id.clone());

        self.active_by_instrument
            .entry(instrument_id)
            .or_default()
            .push(order_id.clone());

        self.orders_by_strategy
            .entry(strategy_id)
            .or_default()
            .push(order_id);

        order
    }

    pub fn on_submitted(&mut self, order_id: &OrderId, now: UnixNanos) {
        if let Some(order) = self.orders.get_mut(order_id) {
            order.state = OmsOrderState::Submitted;
            order.updated_at = now;
        }
    }

    pub fn on_accepted(&mut self, order_id: &OrderId, venue_order_id: String, now: UnixNanos) {
        if let Some(order) = self.orders.get_mut(order_id) {
            order.state = OmsOrderState::Accepted;
            order.venue_order_id = Some(venue_order_id.clone());
            order.updated_at = now;
            self.venue_oid_to_oid
                .insert(venue_order_id, order_id.clone());
        }
    }

    pub fn on_fill(
        &mut self,
        order_id: &OrderId,
        fill_qty: f64,
        fill_price: f64,
        now: UnixNanos,
    ) -> Option<OmsOrder> {
        let instrument_id = self.orders.get(order_id)?.instrument_id.clone();

        if let Some(order) = self.orders.get_mut(order_id) {
            let prev_value = order.filled_quantity * order.avg_fill_price.unwrap_or(0.0);
            let new_value = fill_qty * fill_price;
            order.filled_quantity += fill_qty;
            order.avg_fill_price = Some((prev_value + new_value) / order.filled_quantity);
            order.updated_at = now;

            let is_filled = order.filled_quantity >= order.quantity;
            if is_filled {
                order.state = OmsOrderState::Filled;
            } else {
                order.state = OmsOrderState::PartiallyFilled;
            }

            let result = order.clone();

            if is_filled {
                self.remove_from_active(order_id, &instrument_id);
            }

            return Some(result);
        }
        None
    }

    pub fn on_canceled(&mut self, order_id: &OrderId, now: UnixNanos) {
        let instrument_id = self.orders.get(order_id).map(|o| o.instrument_id.clone());

        if let Some(order) = self.orders.get_mut(order_id) {
            order.state = OmsOrderState::Canceled;
            order.updated_at = now;
        }

        if let Some(instr) = instrument_id {
            self.remove_from_active(order_id, &instr);
        }
    }

    pub fn on_rejected(&mut self, order_id: &OrderId, now: UnixNanos) {
        let instrument_id = self.orders.get(order_id).map(|o| o.instrument_id.clone());

        if let Some(order) = self.orders.get_mut(order_id) {
            order.state = OmsOrderState::Rejected;
            order.updated_at = now;
        }

        if let Some(instr) = instrument_id {
            self.remove_from_active(order_id, &instr);
        }
    }

    fn remove_from_active(&mut self, order_id: &OrderId, instrument_id: &InstrumentId) {
        if let Some(orders) = self.active_by_instrument.get_mut(instrument_id) {
            orders.retain(|id| id != order_id);
        }
    }

    pub fn get_order(&self, order_id: &OrderId) -> Option<&OmsOrder> {
        self.orders.get(order_id)
    }

    pub fn get_by_cloid(&self, cloid: &str) -> Option<&OmsOrder> {
        self.cloid_to_oid
            .get(cloid)
            .and_then(|id| self.orders.get(id))
    }

    pub fn get_by_venue_oid(&self, venue_oid: &str) -> Option<&OmsOrder> {
        self.venue_oid_to_oid
            .get(venue_oid)
            .and_then(|id| self.orders.get(id))
    }

    pub fn active_orders(&self, instrument_id: &InstrumentId) -> Vec<&OmsOrder> {
        self.active_by_instrument
            .get(instrument_id)
            .map(|ids| ids.iter().filter_map(|id| self.orders.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn orders_for_strategy(&self, strategy_id: &StrategyId) -> Vec<&OmsOrder> {
        self.orders_by_strategy
            .get(strategy_id)
            .map(|ids| ids.iter().filter_map(|id| self.orders.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn active_order_count(&self) -> usize {
        self.active_by_instrument.values().map(|v| v.len()).sum()
    }
}

impl Default for OrderManagementSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub instrument_id: InstrumentId,
    pub venue: Venue,
    pub quantity: f64,
    pub avg_entry_price: f64,
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
    pub total_bought: f64,
    pub total_sold: f64,
    pub total_commission: f64,
    pub updated_at: UnixNanos,
}

impl Position {
    pub fn new(instrument_id: InstrumentId, now: UnixNanos) -> Self {
        Self {
            venue: instrument_id.venue,
            instrument_id,
            quantity: 0.0,
            avg_entry_price: 0.0,
            realized_pnl: 0.0,
            unrealized_pnl: 0.0,
            total_bought: 0.0,
            total_sold: 0.0,
            total_commission: 0.0,
            updated_at: now,
        }
    }

    pub fn is_flat(&self) -> bool {
        self.quantity.abs() < 1e-10
    }

    pub fn is_long(&self) -> bool {
        self.quantity > 1e-10
    }

    pub fn is_short(&self) -> bool {
        self.quantity < -1e-10
    }

    pub fn notional(&self, current_price: f64) -> f64 {
        self.quantity.abs() * current_price
    }

    pub fn update_unrealized(&mut self, current_price: f64, now: UnixNanos) {
        if !self.is_flat() {
            self.unrealized_pnl = (current_price - self.avg_entry_price) * self.quantity;
        } else {
            self.unrealized_pnl = 0.0;
        }
        self.updated_at = now;
    }

    pub fn total_pnl(&self) -> f64 {
        self.realized_pnl + self.unrealized_pnl
    }
}

pub struct PositionEngine {
    positions: HashMap<InstrumentId, Position>,

    last_prices: HashMap<InstrumentId, f64>,
}

impl PositionEngine {
    pub fn new() -> Self {
        Self {
            positions: HashMap::new(),
            last_prices: HashMap::new(),
        }
    }

    pub fn get_or_create(&mut self, instrument_id: &InstrumentId, now: UnixNanos) -> &mut Position {
        if !self.positions.contains_key(instrument_id) {
            self.positions.insert(
                instrument_id.clone(),
                Position::new(instrument_id.clone(), now),
            );
        }
        self.positions.get_mut(instrument_id).unwrap()
    }

    pub fn on_fill(
        &mut self,
        instrument_id: &InstrumentId,
        side: OrderSide,
        quantity: f64,
        price: f64,
        commission: f64,
        now: UnixNanos,
    ) {
        let last_price = self.last_prices.get(instrument_id).copied();

        let position = self.get_or_create(instrument_id, now);

        let signed_qty = match side {
            OrderSide::Buy => quantity,
            OrderSide::Sell => -quantity,
        };

        let old_qty = position.quantity;
        let new_qty = old_qty + signed_qty;

        match side {
            OrderSide::Buy => position.total_bought += quantity,
            OrderSide::Sell => position.total_sold += quantity,
        }
        position.total_commission += commission;

        if old_qty.signum() != 0.0
            && (old_qty.signum() != new_qty.signum() || new_qty.abs() < old_qty.abs())
        {
            let closing_qty = (old_qty.abs()).min(signed_qty.abs());
            let realized = (price - position.avg_entry_price) * closing_qty * old_qty.signum();
            position.realized_pnl += realized - commission;
        }

        if new_qty.signum() == signed_qty.signum() || old_qty == 0.0 {
            if old_qty.signum() == signed_qty.signum() && old_qty != 0.0 {
                let old_value = old_qty.abs() * position.avg_entry_price;
                let new_value = quantity * price;
                position.avg_entry_price = (old_value + new_value) / (old_qty.abs() + quantity);
            } else if old_qty.abs() < quantity {
                position.avg_entry_price = price;
            }
        }

        position.quantity = new_qty;
        position.updated_at = now;

        if let Some(current_price) = last_price {
            position.update_unrealized(current_price, now);
        }
    }

    pub fn get_position_after_fill(&self, instrument_id: &InstrumentId) -> Option<Position> {
        self.positions.get(instrument_id).cloned()
    }

    pub fn on_price_update(&mut self, instrument_id: &InstrumentId, price: f64, now: UnixNanos) {
        self.last_prices.insert(instrument_id.clone(), price);

        if let Some(position) = self.positions.get_mut(instrument_id) {
            position.update_unrealized(price, now);
        }
    }

    pub fn get_position(&self, instrument_id: &InstrumentId) -> Option<&Position> {
        self.positions.get(instrument_id)
    }

    pub fn all_positions(&self) -> Vec<&Position> {
        self.positions.values().collect()
    }

    pub fn open_positions(&self) -> Vec<&Position> {
        self.positions.values().filter(|p| !p.is_flat()).collect()
    }

    pub fn total_realized_pnl(&self) -> f64 {
        self.positions.values().map(|p| p.realized_pnl).sum()
    }

    pub fn total_unrealized_pnl(&self) -> f64 {
        self.positions.values().map(|p| p.unrealized_pnl).sum()
    }

    pub fn total_pnl(&self) -> f64 {
        self.total_realized_pnl() + self.total_unrealized_pnl()
    }

    pub fn total_notional(&self) -> f64 {
        self.positions
            .iter()
            .filter_map(|(id, p)| self.last_prices.get(id).map(|&price| p.notional(price)))
            .sum()
    }

    pub fn last_price(&self, instrument_id: &InstrumentId) -> Option<f64> {
        self.last_prices.get(instrument_id).copied()
    }

    pub fn instrument_notional(&self, instrument_id: &InstrumentId) -> Option<f64> {
        let price = self.last_prices.get(instrument_id)?;
        let position = self.positions.get(instrument_id)?;
        Some(position.notional(*price))
    }

    pub fn total_notional_excluding(&self, instrument_id: &InstrumentId) -> f64 {
        self.positions
            .iter()
            .filter(|(id, _)| *id != instrument_id)
            .filter_map(|(id, p)| self.last_prices.get(id).map(|&price| p.notional(price)))
            .sum()
    }

    /// Count of currently open (non-flat) positions
    pub fn position_count(&self) -> usize {
        self.positions.values().filter(|p| !p.is_flat()).count()
    }

    /// Count of open positions for a specific venue
    pub fn positions_by_venue(&self, venue: Venue) -> usize {
        self.positions
            .iter()
            .filter(|(id, p)| id.venue == venue && !p.is_flat())
            .count()
    }

    /// Get all instruments with open positions
    pub fn open_instrument_ids(&self) -> Vec<InstrumentId> {
        self.positions
            .iter()
            .filter(|(_, p)| !p.is_flat())
            .map(|(id, _)| id.clone())
            .collect()
    }
}

impl Default for PositionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Position sizing method
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PositionSizingMethod {
    /// Fixed position size (constant quantity)
    Fixed,
    /// Fixed notional value (constant dollar amount)
    FixedNotional,
    /// Percentage of portfolio equity
    PercentEquity,
    /// Kelly Criterion based sizing
    Kelly,
    /// Risk-based sizing (based on distance to stop loss)
    RiskBased,
    /// Volatility-based sizing (ATR or other volatility measures)
    VolatilityBased,
}

/// Position sizing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSizingConfig {
    /// Position sizing method to use
    pub method: PositionSizingMethod,

    /// Fixed size (for Fixed method)
    pub fixed_size: f64,

    /// Fixed notional value (for FixedNotional method)
    pub fixed_notional: f64,

    /// Percentage of equity to risk per trade (0.0 - 1.0)
    pub equity_percentage: f64,

    /// Kelly fraction multiplier (typically 0.1 to 0.5 for fractional Kelly)
    pub kelly_fraction: f64,

    /// Risk amount per trade (for RiskBased method)
    pub risk_per_trade: f64,

    /// ATR multiplier for volatility-based sizing
    pub atr_multiplier: f64,

    /// Target volatility for portfolio (annualized)
    pub target_volatility: f64,

    /// Minimum position size
    pub min_size: f64,

    /// Maximum position size
    pub max_size: f64,

    /// Round to tick size
    pub round_to_tick: bool,
}

impl Default for PositionSizingConfig {
    fn default() -> Self {
        Self {
            method: PositionSizingMethod::Fixed,
            fixed_size: 1.0,
            fixed_notional: 10000.0,
            equity_percentage: 0.02, // 2% per trade
            kelly_fraction: 0.25,    // Quarter Kelly
            risk_per_trade: 100.0,
            atr_multiplier: 1.5,
            target_volatility: 0.15, // 15% annualized
            min_size: 0.01,
            max_size: 1000.0,
            round_to_tick: true,
        }
    }
}

/// Stop loss type
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopLossType {
    /// No stop loss
    None,
    /// Fixed price distance from entry
    Fixed,
    /// Percentage distance from entry
    Percentage,
    /// ATR-based stop loss
    ATR,
    /// Trailing stop loss (fixed distance)
    Trailing,
    /// Trailing stop loss (percentage-based)
    TrailingPercentage,
    /// Trailing stop loss (ATR-based)
    TrailingATR,
    /// Time-based stop loss (exit after X bars/seconds)
    TimeBased,
    /// Chandelier stop (highest high - ATR * multiplier)
    Chandelier,
    /// Parabolic SAR
    ParabolicSAR,
}

/// Stop loss configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopLossConfig {
    /// Stop loss type
    pub stop_type: StopLossType,

    /// Enable stop loss
    pub enabled: bool,

    /// Fixed price distance (for Fixed type)
    pub fixed_distance: f64,

    /// Percentage distance (0.0 - 1.0, for Percentage type)
    pub percentage: f64,

    /// ATR multiplier (for ATR-based stops)
    pub atr_multiplier: f64,

    /// ATR period (for ATR-based stops)
    pub atr_period: usize,

    /// Trailing distance (for Trailing type)
    pub trailing_distance: f64,

    /// Trailing percentage (for TrailingPercentage type)
    pub trailing_percentage: f64,

    /// Time limit in seconds (for TimeBased type)
    pub time_limit_seconds: u64,

    /// Chandelier multiplier (for Chandelier type)
    pub chandelier_multiplier: f64,

    /// Chandelier lookback period
    pub chandelier_period: usize,

    /// Use limit orders for stops (vs market orders)
    pub use_limit_orders: bool,

    /// Limit order offset (if using limit orders)
    pub limit_order_offset_pct: f64,

    /// Move stop to breakeven after X% profit
    pub breakeven_after_profit_pct: Option<f64>,

    /// Lock in profit percentage after reaching target
    pub lock_profit_pct: Option<f64>,
}

impl Default for StopLossConfig {
    fn default() -> Self {
        Self {
            stop_type: StopLossType::Percentage,
            enabled: false,
            fixed_distance: 10.0,
            percentage: 0.02, // 2% stop loss
            atr_multiplier: 2.0,
            atr_period: 14,
            trailing_distance: 10.0,
            trailing_percentage: 0.03,
            time_limit_seconds: 3600,
            chandelier_multiplier: 3.0,
            chandelier_period: 22,
            use_limit_orders: false,
            limit_order_offset_pct: 0.001, // 0.1% slippage buffer
            breakeven_after_profit_pct: Some(0.01), // Move to breakeven after 1% profit
            lock_profit_pct: Some(0.5),    // Lock in 50% of profit
        }
    }
}

/// Take profit type
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TakeProfitType {
    /// No take profit
    None,
    /// Fixed price target
    Fixed,
    /// Percentage target from entry
    Percentage,
    /// Risk-reward ratio based (e.g., 2:1, 3:1)
    RiskReward,
    /// ATR-based target
    ATR,
    /// Fibonacci levels
    Fibonacci,
    /// Multiple partial take profits
    Partial,
    /// Trailing take profit (lock in profits as price moves favorably)
    Trailing,
}

/// Partial take profit level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialTakeProfitLevel {
    /// Percentage of position to close (0.0 - 1.0)
    pub size_pct: f64,

    /// Target distance (interpretation depends on method)
    pub target: f64,
}

/// Take profit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TakeProfitConfig {
    /// Take profit type
    pub take_profit_type: TakeProfitType,

    /// Enable take profit
    pub enabled: bool,

    /// Fixed price distance (for Fixed type)
    pub fixed_distance: f64,

    /// Percentage target (0.0 - 1.0, for Percentage type)
    pub percentage: f64,

    /// Risk-reward ratio (for RiskReward type, e.g., 2.0 means 2:1)
    pub risk_reward_ratio: f64,

    /// ATR multiplier (for ATR-based targets)
    pub atr_multiplier: f64,

    /// ATR period (for ATR-based targets)
    pub atr_period: usize,

    /// Fibonacci levels to use (e.g., vec![0.382, 0.618, 1.0, 1.618])
    pub fibonacci_levels: Vec<f64>,

    /// Partial take profit levels
    pub partial_levels: Vec<PartialTakeProfitLevel>,

    /// Trailing take profit distance
    pub trailing_distance: f64,

    /// Use limit orders for take profits (vs market orders)
    pub use_limit_orders: bool,
}

impl Default for TakeProfitConfig {
    fn default() -> Self {
        Self {
            take_profit_type: TakeProfitType::RiskReward,
            enabled: false,
            fixed_distance: 20.0,
            percentage: 0.05,       // 5% take profit
            risk_reward_ratio: 2.0, // 2:1 risk-reward
            atr_multiplier: 3.0,
            atr_period: 14,
            fibonacci_levels: vec![0.382, 0.618, 1.0, 1.618],
            partial_levels: vec![
                PartialTakeProfitLevel {
                    size_pct: 0.5,
                    target: 0.02, // 2% profit, close 50%
                },
                PartialTakeProfitLevel {
                    size_pct: 0.5,
                    target: 0.04, // 4% profit, close remaining 50%
                },
            ],
            trailing_distance: 15.0,
            use_limit_orders: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    pub max_position_size: f64,

    pub max_position_size_by_instrument: HashMap<InstrumentId, f64>,

    pub max_notional_by_instrument: HashMap<InstrumentId, f64>,

    pub max_notional: f64,

    pub max_concentration_pct: f64,

    pub max_unrealized_loss: f64,

    pub max_daily_loss: f64,

    pub rapid_drawdown_pct: f64,

    pub rapid_drawdown_window_secs: u64,

    pub correlation_groups: Vec<CorrelationGroupLimit>,

    pub liquidity_limits: HashMap<InstrumentId, LiquidityLimit>,

    pub max_orders_per_minute: u32,

    pub enable_kill_switch: bool,

    /// Position sizing configuration
    pub position_sizing: PositionSizingConfig,

    /// Stop loss configuration
    pub stop_loss: StopLossConfig,

    /// Take profit configuration
    pub take_profit: TakeProfitConfig,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_position_size: 100.0,
            max_position_size_by_instrument: HashMap::new(),
            max_notional_by_instrument: HashMap::new(),
            max_notional: 1_000_000.0,
            max_concentration_pct: 0.0,
            max_unrealized_loss: 50_000.0,
            max_daily_loss: 100_000.0,
            rapid_drawdown_pct: 0.0,
            rapid_drawdown_window_secs: 0,
            correlation_groups: Vec::new(),
            liquidity_limits: HashMap::new(),
            max_orders_per_minute: 120,
            enable_kill_switch: true,
            position_sizing: PositionSizingConfig::default(),
            stop_loss: StopLossConfig::default(),
            take_profit: TakeProfitConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationGroupLimit {
    pub name: String,
    pub instruments: Vec<InstrumentId>,
    pub max_notional: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityLimit {
    pub max_order_size: Option<f64>,
    pub max_order_notional: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum RiskCheckResult {
    Allowed,

    Rejected(String),

    KillSwitch(String),
}

pub struct RiskManager {
    config: RiskConfig,

    kill_switch_active: bool,
    kill_switch_reason: Option<String>,

    daily_pnl_start: f64,
    daily_realized_pnl: f64,

    orders_this_minute: u32,
    minute_start: std::time::Instant,

    equity_high_water: Option<f64>,
    equity_history: VecDeque<(std::time::Instant, f64)>,
    last_equity: Option<f64>,
}

impl RiskManager {
    pub fn new(config: RiskConfig) -> Self {
        Self {
            config,
            kill_switch_active: false,
            kill_switch_reason: None,
            daily_pnl_start: 0.0,
            daily_realized_pnl: 0.0,
            orders_this_minute: 0,
            minute_start: std::time::Instant::now(),
            equity_high_water: None,
            equity_history: VecDeque::new(),
            last_equity: None,
        }
    }

    pub fn is_kill_switch_active(&self) -> bool {
        self.kill_switch_active
    }

    pub fn kill_switch_reason(&self) -> Option<&str> {
        self.kill_switch_reason.as_deref()
    }

    pub fn trigger_kill_switch(&mut self, reason: String) {
        self.kill_switch_active = true;
        self.kill_switch_reason = Some(reason);
    }

    pub fn reset_kill_switch(&mut self) {
        self.kill_switch_active = false;
        self.kill_switch_reason = None;
    }

    pub fn check_order(
        &mut self,
        instrument_id: &InstrumentId,
        side: OrderSide,
        quantity: f64,
        positions: &PositionEngine,
    ) -> RiskCheckResult {
        self.check_order_with_price(instrument_id, side, quantity, None, positions)
    }

    pub fn check_order_with_price(
        &mut self,
        instrument_id: &InstrumentId,
        side: OrderSide,
        quantity: f64,
        price: Option<f64>,
        positions: &PositionEngine,
    ) -> RiskCheckResult {
        if self.kill_switch_active {
            return RiskCheckResult::KillSwitch(
                self.kill_switch_reason.clone().unwrap_or_default(),
            );
        }

        self.update_order_rate();
        if self.orders_this_minute >= self.config.max_orders_per_minute {
            return RiskCheckResult::Rejected(format!(
                "Order rate limit: {}/min",
                self.config.max_orders_per_minute
            ));
        }

        let current_pos = positions
            .get_position(instrument_id)
            .map(|p| p.quantity)
            .unwrap_or(0.0);

        let new_pos = match side {
            OrderSide::Buy => current_pos + quantity,
            OrderSide::Sell => current_pos - quantity,
        };

        let position_limit = self
            .config
            .max_position_size_by_instrument
            .get(instrument_id)
            .copied()
            .unwrap_or(self.config.max_position_size);

        if new_pos.abs() > position_limit {
            return RiskCheckResult::Rejected(format!(
                "Position size {} would exceed limit {}",
                new_pos.abs(),
                position_limit
            ));
        }

        let order_price = price.or_else(|| positions.last_price(instrument_id));

        if let Some(price) = order_price {
            if let Some(limit) = self.config.max_notional_by_instrument.get(instrument_id) {
                let new_notional = new_pos.abs() * price;
                if new_notional > *limit {
                    return RiskCheckResult::Rejected(format!(
                        "Notional {} exceeds limit {}",
                        new_notional, limit
                    ));
                }
            }
        }

        if let Some(limit) = self.config.liquidity_limits.get(instrument_id) {
            if let Some(max_size) = limit.max_order_size {
                if quantity > max_size {
                    return RiskCheckResult::Rejected(format!(
                        "Order size {} exceeds liquidity limit {}",
                        quantity, max_size
                    ));
                }
            }
            if let (Some(max_notional), Some(price)) = (limit.max_order_notional, order_price) {
                let order_notional = quantity.abs() * price;
                if order_notional > max_notional {
                    return RiskCheckResult::Rejected(format!(
                        "Order notional {} exceeds liquidity limit {}",
                        order_notional, max_notional
                    ));
                }
            }
        }

        let total_notional = if let Some(price) = order_price {
            let other_notional = positions.total_notional_excluding(instrument_id);
            other_notional + new_pos.abs() * price
        } else {
            positions.total_notional()
        };

        if total_notional > self.config.max_notional {
            return RiskCheckResult::Rejected(format!(
                "Notional {} exceeds limit {}",
                total_notional, self.config.max_notional
            ));
        }

        if self.config.max_concentration_pct > 0.0 {
            if let Some(price) = order_price {
                let instrument_notional = new_pos.abs() * price;
                if total_notional > 0.0 {
                    let concentration = instrument_notional / total_notional;
                    if concentration > self.config.max_concentration_pct {
                        return RiskCheckResult::Rejected(format!(
                            "Concentration {:.2}% exceeds limit {:.2}%",
                            concentration * 100.0,
                            self.config.max_concentration_pct * 100.0
                        ));
                    }
                }
            }
        }

        if !self.config.correlation_groups.is_empty() {
            for group in &self.config.correlation_groups {
                if !group.instruments.iter().any(|id| id == instrument_id) {
                    continue;
                }
                let mut group_notional = 0.0;
                for instrument in &group.instruments {
                    let notional = if instrument == instrument_id {
                        if let Some(price) = order_price {
                            new_pos.abs() * price
                        } else {
                            positions.instrument_notional(instrument).unwrap_or(0.0)
                        }
                    } else {
                        positions.instrument_notional(instrument).unwrap_or(0.0)
                    };
                    group_notional += notional;
                }
                if group_notional > group.max_notional {
                    return RiskCheckResult::Rejected(format!(
                        "Correlation group {} notional {} exceeds limit {}",
                        group.name, group_notional, group.max_notional
                    ));
                }
            }
        }

        self.orders_this_minute += 1;
        RiskCheckResult::Allowed
    }

    pub fn check_realtime(&mut self, positions: &PositionEngine) -> RiskCheckResult {
        if self.kill_switch_active {
            return RiskCheckResult::KillSwitch(
                self.kill_switch_reason.clone().unwrap_or_default(),
            );
        }

        if !self.config.enable_kill_switch {
            return RiskCheckResult::Allowed;
        }

        let unrealized = positions.total_unrealized_pnl();
        if unrealized < -self.config.max_unrealized_loss {
            let reason = format!(
                "Unrealized loss {} exceeds limit {}",
                unrealized, self.config.max_unrealized_loss
            );
            self.trigger_kill_switch(reason.clone());
            return RiskCheckResult::KillSwitch(reason);
        }

        let daily_pnl = self.daily_realized_pnl + unrealized;
        if daily_pnl < -self.config.max_daily_loss {
            let reason = format!(
                "Daily loss {} exceeds limit {}",
                daily_pnl, self.config.max_daily_loss
            );
            self.trigger_kill_switch(reason.clone());
            return RiskCheckResult::KillSwitch(reason);
        }

        if self.config.max_concentration_pct > 0.0 {
            let total_notional = positions.total_notional();
            if total_notional > 0.0 {
                for position in positions.open_positions() {
                    if let Some(notional) = positions.instrument_notional(&position.instrument_id) {
                        let concentration = notional / total_notional;
                        if concentration > self.config.max_concentration_pct {
                            let reason = format!(
                                "Concentration {:.2}% exceeds limit {:.2}% for {}",
                                concentration * 100.0,
                                self.config.max_concentration_pct * 100.0,
                                position.instrument_id
                            );
                            self.trigger_kill_switch(reason.clone());
                            return RiskCheckResult::KillSwitch(reason);
                        }
                    }
                }
            }
        }

        if !self.config.correlation_groups.is_empty() {
            for group in &self.config.correlation_groups {
                let mut group_notional = 0.0;
                for instrument in &group.instruments {
                    group_notional += positions.instrument_notional(instrument).unwrap_or(0.0);
                }
                if group_notional > group.max_notional {
                    let reason = format!(
                        "Correlation group {} notional {} exceeds limit {}",
                        group.name, group_notional, group.max_notional
                    );
                    self.trigger_kill_switch(reason.clone());
                    return RiskCheckResult::KillSwitch(reason);
                }
            }
        }

        if let Some(reason) = self.check_rapid_drawdown() {
            self.trigger_kill_switch(reason.clone());
            return RiskCheckResult::KillSwitch(reason);
        }

        RiskCheckResult::Allowed
    }

    pub fn record_equity(&mut self, equity: f64) {
        let now = std::time::Instant::now();
        self.last_equity = Some(equity);
        if self.equity_high_water.map_or(true, |high| equity > high) {
            self.equity_high_water = Some(equity);
        }
        self.equity_history.push_back((now, equity));
        self.trim_equity_history(now);
    }

    pub fn on_fill(&mut self, realized_pnl: f64) {
        self.daily_realized_pnl += realized_pnl;
    }

    pub fn reset_daily(&mut self) {
        self.daily_pnl_start = 0.0;
        self.daily_realized_pnl = 0.0;
    }

    fn update_order_rate(&mut self) {
        let now = std::time::Instant::now();
        if now.duration_since(self.minute_start).as_secs() >= 60 {
            self.orders_this_minute = 0;
            self.minute_start = now;
        }
    }

    fn trim_equity_history(&mut self, now: std::time::Instant) {
        if self.config.rapid_drawdown_window_secs == 0 {
            return;
        }
        let window = std::time::Duration::from_secs(self.config.rapid_drawdown_window_secs);
        while let Some((ts, _)) = self.equity_history.front() {
            if now.duration_since(*ts) > window {
                self.equity_history.pop_front();
            } else {
                break;
            }
        }
    }

    fn check_rapid_drawdown(&self) -> Option<String> {
        if self.config.rapid_drawdown_pct <= 0.0 {
            return None;
        }
        if self.config.rapid_drawdown_window_secs == 0 {
            return None;
        }
        let current = self.last_equity?;
        let mut max_equity = current;
        for (_, equity) in &self.equity_history {
            if *equity > max_equity {
                max_equity = *equity;
            }
        }
        if max_equity <= 0.0 {
            return None;
        }
        let drawdown = (max_equity - current) / max_equity;
        if drawdown >= self.config.rapid_drawdown_pct {
            return Some(format!(
                "Rapid drawdown {:.2}% exceeds limit {:.2}%",
                drawdown * 100.0,
                self.config.rapid_drawdown_pct * 100.0
            ));
        }
        None
    }

    /// Calculate position size based on configured method
    pub fn calculate_position_size(
        &self,
        entry_price: f64,
        stop_price: Option<f64>,
        equity: f64,
        atr: Option<f64>,
        win_rate: Option<f64>,
        avg_win: Option<f64>,
        avg_loss: Option<f64>,
    ) -> f64 {
        let config = &self.config.position_sizing;

        let raw_size = match config.method {
            PositionSizingMethod::Fixed => config.fixed_size,

            PositionSizingMethod::FixedNotional => {
                if entry_price > 0.0 {
                    config.fixed_notional / entry_price
                } else {
                    config.fixed_size
                }
            }

            PositionSizingMethod::PercentEquity => {
                if entry_price > 0.0 {
                    (equity * config.equity_percentage) / entry_price
                } else {
                    config.fixed_size
                }
            }

            PositionSizingMethod::Kelly => {
                // Kelly: f = (p * b - q) / b
                // where p = win rate, q = 1-p, b = avg_win/avg_loss
                if let (Some(p), Some(w), Some(l)) = (win_rate, avg_win, avg_loss) {
                    if l > 0.0 {
                        let b = w / l;
                        let q = 1.0 - p;
                        let kelly = ((p * b - q) / b).max(0.0);
                        let fractional_kelly = kelly * config.kelly_fraction;
                        if entry_price > 0.0 {
                            (equity * fractional_kelly) / entry_price
                        } else {
                            config.fixed_size
                        }
                    } else {
                        config.fixed_size
                    }
                } else {
                    config.fixed_size
                }
            }

            PositionSizingMethod::RiskBased => {
                // Risk-based: size = risk_amount / (entry_price - stop_price)
                if let Some(stop) = stop_price {
                    let risk_per_unit = (entry_price - stop).abs();
                    if risk_per_unit > 0.0 {
                        config.risk_per_trade / risk_per_unit
                    } else {
                        config.fixed_size
                    }
                } else {
                    config.fixed_size
                }
            }

            PositionSizingMethod::VolatilityBased => {
                // Volatility-based: size based on ATR and target volatility
                if let Some(atr_value) = atr {
                    if atr_value > 0.0 && entry_price > 0.0 {
                        // Target dollar volatility per unit
                        let target_dollar_vol = equity * config.target_volatility;
                        // ATR represents typical price movement
                        let expected_move = atr_value * config.atr_multiplier;
                        // Size to achieve target volatility
                        target_dollar_vol / expected_move
                    } else {
                        config.fixed_size
                    }
                } else {
                    config.fixed_size
                }
            }
        };

        // Apply min/max constraints
        raw_size.max(config.min_size).min(config.max_size)
    }

    /// Calculate stop loss price based on configured method
    pub fn calculate_stop_loss(
        &self,
        entry_price: f64,
        side: OrderSide,
        atr: Option<f64>,
        high: Option<f64>,
        low: Option<f64>,
    ) -> Option<f64> {
        let config = &self.config.stop_loss;

        if !config.enabled {
            return None;
        }

        let stop_price = match config.stop_type {
            StopLossType::None => return None,

            StopLossType::Fixed => match side {
                OrderSide::Buy => entry_price - config.fixed_distance,
                OrderSide::Sell => entry_price + config.fixed_distance,
            },

            StopLossType::Percentage => match side {
                OrderSide::Buy => entry_price * (1.0 - config.percentage),
                OrderSide::Sell => entry_price * (1.0 + config.percentage),
            },

            StopLossType::ATR => {
                if let Some(atr_value) = atr {
                    let distance = atr_value * config.atr_multiplier;
                    match side {
                        OrderSide::Buy => entry_price - distance,
                        OrderSide::Sell => entry_price + distance,
                    }
                } else {
                    // Fallback to percentage if ATR not available
                    match side {
                        OrderSide::Buy => entry_price * (1.0 - config.percentage),
                        OrderSide::Sell => entry_price * (1.0 + config.percentage),
                    }
                }
            }

            StopLossType::Chandelier => {
                if let (Some(atr_value), Some(h)) = (atr, high) {
                    let distance = atr_value * config.chandelier_multiplier;
                    match side {
                        OrderSide::Buy => h - distance,
                        OrderSide::Sell => {
                            if let Some(l) = low {
                                l + distance
                            } else {
                                entry_price + distance
                            }
                        }
                    }
                } else {
                    return None;
                }
            }

            StopLossType::Trailing => match side {
                OrderSide::Buy => entry_price - config.trailing_distance,
                OrderSide::Sell => entry_price + config.trailing_distance,
            },

            StopLossType::TrailingPercentage => match side {
                OrderSide::Buy => entry_price * (1.0 - config.trailing_percentage),
                OrderSide::Sell => entry_price * (1.0 + config.trailing_percentage),
            },

            StopLossType::TrailingATR => {
                if let Some(atr_value) = atr {
                    let distance = atr_value * config.atr_multiplier;
                    match side {
                        OrderSide::Buy => entry_price - distance,
                        OrderSide::Sell => entry_price + distance,
                    }
                } else {
                    return None;
                }
            }

            StopLossType::TimeBased => {
                // Time-based stops are handled externally by timestamp
                return None;
            }

            StopLossType::ParabolicSAR => {
                // Parabolic SAR requires external calculation
                return None;
            }
        };

        Some(stop_price)
    }

    /// Calculate take profit price based on configured method
    pub fn calculate_take_profit(
        &self,
        entry_price: f64,
        side: OrderSide,
        stop_price: Option<f64>,
        atr: Option<f64>,
    ) -> Option<f64> {
        let config = &self.config.take_profit;

        if !config.enabled {
            return None;
        }

        let tp_price = match config.take_profit_type {
            TakeProfitType::None => return None,

            TakeProfitType::Fixed => match side {
                OrderSide::Buy => entry_price + config.fixed_distance,
                OrderSide::Sell => entry_price - config.fixed_distance,
            },

            TakeProfitType::Percentage => match side {
                OrderSide::Buy => entry_price * (1.0 + config.percentage),
                OrderSide::Sell => entry_price * (1.0 - config.percentage),
            },

            TakeProfitType::RiskReward => {
                if let Some(stop) = stop_price {
                    let risk = (entry_price - stop).abs();
                    let reward = risk * config.risk_reward_ratio;
                    match side {
                        OrderSide::Buy => entry_price + reward,
                        OrderSide::Sell => entry_price - reward,
                    }
                } else {
                    // Fallback to percentage
                    match side {
                        OrderSide::Buy => entry_price * (1.0 + config.percentage),
                        OrderSide::Sell => entry_price * (1.0 - config.percentage),
                    }
                }
            }

            TakeProfitType::ATR => {
                if let Some(atr_value) = atr {
                    let distance = atr_value * config.atr_multiplier;
                    match side {
                        OrderSide::Buy => entry_price + distance,
                        OrderSide::Sell => entry_price - distance,
                    }
                } else {
                    return None;
                }
            }

            TakeProfitType::Fibonacci => {
                // Return the first (most conservative) Fibonacci level
                if let Some(stop) = stop_price {
                    let risk = (entry_price - stop).abs();
                    if let Some(&fib) = config.fibonacci_levels.first() {
                        let reward = risk * fib;
                        match side {
                            OrderSide::Buy => entry_price + reward,
                            OrderSide::Sell => entry_price - reward,
                        }
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }

            TakeProfitType::Partial => {
                // Return the first partial target
                if let Some(level) = config.partial_levels.first() {
                    let distance = level.target;
                    match side {
                        OrderSide::Buy => entry_price * (1.0 + distance),
                        OrderSide::Sell => entry_price * (1.0 - distance),
                    }
                } else {
                    return None;
                }
            }

            TakeProfitType::Trailing => match side {
                OrderSide::Buy => entry_price + config.trailing_distance,
                OrderSide::Sell => entry_price - config.trailing_distance,
            },
        };

        Some(tp_price)
    }

    /// Get all partial take profit levels
    pub fn get_partial_take_profit_levels(
        &self,
        entry_price: f64,
        side: OrderSide,
        position_size: f64,
    ) -> Vec<(f64, f64)> {
        // Returns Vec<(price, size_to_close)>
        let config = &self.config.take_profit;

        if !config.enabled || config.take_profit_type != TakeProfitType::Partial {
            return Vec::new();
        }

        config
            .partial_levels
            .iter()
            .map(|level| {
                let price = match side {
                    OrderSide::Buy => entry_price * (1.0 + level.target),
                    OrderSide::Sell => entry_price * (1.0 - level.target),
                };
                let size = position_size * level.size_pct;
                (price, size)
            })
            .collect()
    }

    /// Update trailing stop based on current price
    pub fn update_trailing_stop(
        &self,
        entry_price: f64,
        current_price: f64,
        side: OrderSide,
        current_stop: f64,
        atr: Option<f64>,
    ) -> f64 {
        let config = &self.config.stop_loss;

        if !config.enabled {
            return current_stop;
        }

        let new_stop = match config.stop_type {
            StopLossType::Trailing => {
                match side {
                    OrderSide::Buy => {
                        let proposed = current_price - config.trailing_distance;
                        proposed.max(current_stop) // Only move up, never down
                    }
                    OrderSide::Sell => {
                        let proposed = current_price + config.trailing_distance;
                        proposed.min(current_stop) // Only move down, never up
                    }
                }
            }

            StopLossType::TrailingPercentage => match side {
                OrderSide::Buy => {
                    let proposed = current_price * (1.0 - config.trailing_percentage);
                    proposed.max(current_stop)
                }
                OrderSide::Sell => {
                    let proposed = current_price * (1.0 + config.trailing_percentage);
                    proposed.min(current_stop)
                }
            },

            StopLossType::TrailingATR => {
                if let Some(atr_value) = atr {
                    let distance = atr_value * config.atr_multiplier;
                    match side {
                        OrderSide::Buy => {
                            let proposed = current_price - distance;
                            proposed.max(current_stop)
                        }
                        OrderSide::Sell => {
                            let proposed = current_price + distance;
                            proposed.min(current_stop)
                        }
                    }
                } else {
                    current_stop
                }
            }

            _ => current_stop,
        };

        // Check for breakeven move
        if let Some(breakeven_pct) = config.breakeven_after_profit_pct {
            let profit_pct = match side {
                OrderSide::Buy => (current_price - entry_price) / entry_price,
                OrderSide::Sell => (entry_price - current_price) / entry_price,
            };

            if profit_pct >= breakeven_pct {
                // Move stop to breakeven (or slightly better)
                let breakeven_stop = match side {
                    OrderSide::Buy => entry_price.max(new_stop),
                    OrderSide::Sell => entry_price.min(new_stop),
                };
                return breakeven_stop;
            }
        }

        new_stop
    }

    /// Calculate available capital considering locks and reserves
    pub fn calculate_available_capital(
        &self,
        total_equity: f64,
        realized_pnl: f64,
        initial_capital: f64,
        locked_profits: f64,
        capital_config: &CapitalConfig,
    ) -> f64 {
        let mut available = total_equity;

        // Apply reserve requirement
        let reserve_amount = total_equity * capital_config.reserve_capital_pct;
        available -= reserve_amount;

        // Lock profits if enabled
        if capital_config.lock_profits && realized_pnl > 0.0 {
            available = available.min(initial_capital + locked_profits);
        }

        available.max(0.0)
    }

    /// Calculate margin requirement for position
    pub fn calculate_margin_requirement(
        &self,
        position_value: f64,
        leverage_config: &LeverageConfig,
        is_initial: bool,
    ) -> f64 {
        if !leverage_config.enabled {
            return position_value; // No leverage, full capital required
        }

        let margin_pct = if is_initial {
            leverage_config.initial_margin_pct
        } else {
            leverage_config.maintenance_margin_pct
        };

        position_value * margin_pct
    }

    /// Check if position violates holding period constraints
    pub fn check_holding_period(
        &self,
        entry_time: UnixNanos,
        current_time: UnixNanos,
        config: &HoldingPeriodConfig,
        is_profitable: bool,
    ) -> RiskCheckResult {
        if !config.enabled {
            return RiskCheckResult::Allowed;
        }

        let holding_seconds = (current_time.0 - entry_time.0) / 1_000_000_000;

        // Check minimum holding period
        if holding_seconds < config.min_holding_seconds {
            return RiskCheckResult::Rejected(format!(
                "Minimum holding period not met: {}s < {}s",
                holding_seconds, config.min_holding_seconds
            ));
        }

        // Check minimum holding for profit
        if is_profitable && holding_seconds < config.min_holding_for_profit {
            return RiskCheckResult::Rejected(format!(
                "Minimum holding period for profit not met: {}s < {}s",
                holding_seconds, config.min_holding_for_profit
            ));
        }

        // Check maximum holding period
        if config.max_holding_seconds > 0 && holding_seconds > config.max_holding_seconds {
            if config.force_close_at_max {
                return RiskCheckResult::KillSwitch(format!(
                    "Maximum holding period exceeded: {}s > {}s (force close)",
                    holding_seconds, config.max_holding_seconds
                ));
            }
        }

        RiskCheckResult::Allowed
    }

    /// Calculate margin health ratio (1.0 = at maintenance margin, > 1.0 = healthy)
    pub fn calculate_margin_health(
        &self,
        equity: f64,
        position_value: f64,
        leverage_config: &LeverageConfig,
    ) -> f64 {
        if !leverage_config.enabled || position_value <= 0.0 {
            return f64::INFINITY;
        }

        let required_margin = position_value * leverage_config.maintenance_margin_pct;
        if required_margin <= 0.0 {
            return f64::INFINITY;
        }

        equity / required_margin
    }

    /// Check for margin call
    pub fn check_margin_call(
        &self,
        equity: f64,
        position_value: f64,
        leverage_config: &LeverageConfig,
    ) -> Option<String> {
        if !leverage_config.enabled {
            return None;
        }

        let margin_health = self.calculate_margin_health(equity, position_value, leverage_config);

        // Check for liquidation danger
        if margin_health < (1.0 + leverage_config.liquidation_buffer_pct) {
            return Some(format!(
                "Liquidation danger: margin health {:.2} too low",
                margin_health
            ));
        }

        // Check for margin call
        if margin_health < leverage_config.margin_call_threshold {
            return Some(format!(
                "Margin call: margin health {:.2} below threshold {:.2}",
                margin_health, leverage_config.margin_call_threshold
            ));
        }

        None
    }

    /// Calculate borrow cost for leveraged position
    pub fn calculate_borrow_cost(
        &self,
        borrowed_amount: f64,
        holding_seconds: u64,
        leverage_config: &LeverageConfig,
    ) -> f64 {
        if !leverage_config.enabled || borrowed_amount <= 0.0 {
            return 0.0;
        }

        let annual_rate = leverage_config.borrow_rate_annual;
        let seconds_per_year = 365.25 * 24.0 * 3600.0;
        let rate_per_second = annual_rate / seconds_per_year;

        borrowed_amount * rate_per_second * (holding_seconds as f64)
    }

    /// Check if pyramiding is allowed
    pub fn check_pyramiding(
        &self,
        _instrument_id: &InstrumentId,
        current_position_count: usize,
        position_config: &PositionManagementConfig,
    ) -> RiskCheckResult {
        if !position_config.allow_pyramiding {
            if current_position_count > 0 {
                return RiskCheckResult::Rejected("Pyramiding not allowed".to_string());
            }
        } else {
            if current_position_count >= position_config.max_pyramid_levels {
                return RiskCheckResult::Rejected(format!(
                    "Maximum pyramid levels {} reached",
                    position_config.max_pyramid_levels
                ));
            }
        }

        RiskCheckResult::Allowed
    }

    /// Calculate pyramiding size (scaled down based on level)
    pub fn calculate_pyramid_size(
        &self,
        base_size: f64,
        pyramid_level: usize,
        position_config: &PositionManagementConfig,
    ) -> f64 {
        if pyramid_level == 0 {
            return base_size;
        }

        base_size
            * position_config
                .pyramid_scale_factor
                .powi(pyramid_level as i32)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeTier {
    pub maker_fee_bps: f64,
    pub taker_fee_bps: f64,
    pub volume_threshold: f64,
}

pub struct FeeCalculator {
    venue: Venue,
    fee_tiers: Vec<FeeTier>,
    current_30d_volume: f64,
}

impl FeeCalculator {
    pub fn new(venue: Venue) -> Self {
        let fee_tiers = match venue {
            Venue::Hyperliquid => vec![
                FeeTier {
                    maker_fee_bps: 0.2,
                    taker_fee_bps: 0.5,
                    volume_threshold: 0.0,
                },
                FeeTier {
                    maker_fee_bps: 0.1,
                    taker_fee_bps: 0.4,
                    volume_threshold: 1_000_000.0,
                },
                FeeTier {
                    maker_fee_bps: 0.0,
                    taker_fee_bps: 0.35,
                    volume_threshold: 10_000_000.0,
                },
            ],
            Venue::Lighter => vec![
                FeeTier {
                    maker_fee_bps: 0.0,
                    taker_fee_bps: 0.4,
                    volume_threshold: 0.0,
                },
                FeeTier {
                    maker_fee_bps: -0.1,
                    taker_fee_bps: 0.3,
                    volume_threshold: 5_000_000.0,
                },
            ],
            _ => vec![FeeTier {
                maker_fee_bps: 1.0,
                taker_fee_bps: 1.0,
                volume_threshold: 0.0,
            }],
        };

        Self {
            venue,
            fee_tiers,
            current_30d_volume: 0.0,
        }
    }

    pub fn calculate_fee(&self, notional: f64, is_maker: bool) -> f64 {
        let tier = self.get_current_tier();
        let bps = if is_maker {
            tier.maker_fee_bps
        } else {
            tier.taker_fee_bps
        };
        notional * bps / 10_000.0
    }

    fn get_current_tier(&self) -> &FeeTier {
        self.fee_tiers
            .iter()
            .rev()
            .find(|t| self.current_30d_volume >= t.volume_threshold)
            .unwrap_or(&self.fee_tiers[0])
    }

    pub fn update_volume(&mut self, volume: f64) {
        self.current_30d_volume = volume;
    }

    pub fn maker_fee_bps(&self) -> f64 {
        self.get_current_tier().maker_fee_bps
    }

    pub fn taker_fee_bps(&self) -> f64 {
        self.get_current_tier().taker_fee_bps
    }

    /// Get the venue this calculator is configured for
    pub fn venue(&self) -> Venue {
        self.venue
    }
}

pub struct FundingTracker {
    venue: Venue,

    rates: HashMap<InstrumentId, f64>,

    next_funding: HashMap<InstrumentId, UnixNanos>,

    total_funding: f64,
}

impl FundingTracker {
    pub fn new(venue: Venue) -> Self {
        Self {
            venue,
            rates: HashMap::new(),
            next_funding: HashMap::new(),
            total_funding: 0.0,
        }
    }

    pub fn update_rate(&mut self, instrument_id: InstrumentId, rate: f64, next_time: UnixNanos) {
        self.rates.insert(instrument_id.clone(), rate);
        self.next_funding.insert(instrument_id, next_time);
    }

    pub fn get_rate(&self, instrument_id: &InstrumentId) -> Option<f64> {
        self.rates.get(instrument_id).copied()
    }

    pub fn projected_funding(&self, instrument_id: &InstrumentId, position_value: f64) -> f64 {
        self.rates
            .get(instrument_id)
            .map(|r| position_value * r / 100.0 / 3.0)
            .unwrap_or(0.0)
    }

    pub fn record_payment(&mut self, amount: f64) {
        self.total_funding += amount;
    }

    pub fn total_funding(&self) -> f64 {
        self.total_funding
    }

    /// Get the venue this tracker is configured for
    pub fn venue(&self) -> Venue {
        self.venue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestStrategy {
        id: StrategyId,
        started: bool,
        stopped: bool,
    }

    impl TestStrategy {
        fn new(id: &str) -> Self {
            Self {
                id: StrategyId::new(id),
                started: false,
                stopped: false,
            }
        }
    }

    impl StrategyHandler for TestStrategy {
        fn id(&self) -> &StrategyId {
            &self.id
        }

        fn on_start(&mut self, ctx: &mut StrategyContext) {
            self.started = true;
            ctx.subscribe_trades(InstrumentId::new(
                Venue::Simulated,
                "BTC",
                neleus_core_types::InstrumentType::Perp,
            ));
        }

        fn on_stop(&mut self, _ctx: &mut StrategyContext) {
            self.stopped = true;
        }

        fn on_data(&mut self, _ctx: &mut StrategyContext, _data: &MarketDataEvent) {}

        fn on_event(&mut self, _ctx: &mut StrategyContext, _event: &TradingEvent) {}

        fn on_timer(&mut self, _ctx: &mut StrategyContext, _name: &str) {}
    }

    #[test]
    fn test_engine_lifecycle() {
        let config = EngineConfig::default();
        let mut engine = Engine::new(config);

        assert_eq!(engine.state(), EngineState::Idle);

        engine.add_strategy(Box::new(TestStrategy::new("test")));
        engine.start();

        assert_eq!(engine.state(), EngineState::Running);

        engine.stop();

        assert_eq!(engine.state(), EngineState::Stopped);
    }

    #[test]
    fn test_clock_modes() {
        let live_clock = Clock::new(ClockMode::Live);
        let t1 = live_clock.now();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let t2 = live_clock.now();
        assert!(t2 > t1);

        let mut sim_clock = Clock::new(ClockMode::Simulated);
        assert_eq!(sim_clock.now(), UnixNanos::ZERO);
        sim_clock.set_time(UnixNanos::from_millis(1000));
        assert_eq!(sim_clock.now().as_millis(), 1000);
    }
}
