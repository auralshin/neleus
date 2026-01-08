use neleus_core_bus::{Bus, BusConfig, InMemoryBus, Message, MessageKind, Topic};
use neleus_core_types::{InstrumentId, OrderId, SequenceNumber, StrategyId, UnixNanos, Venue};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub instance_id: String,

    pub max_events_per_tick: usize,

    pub enable_event_log: bool,

    pub clock_mode: ClockMode,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            instance_id: "neleus-1".to_string(),
            max_events_per_tick: 1000,
            enable_event_log: true,
            clock_mode: ClockMode::Live,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockMode {
    Live,

    Simulated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineState {
    Idle,

    Starting,

    Running,

    Stopping,

    Stopped,

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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
            processed_ticks: 0,
            processed_messages: 0,
            last_tick_duration_micros: 0,
        }
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

    fn process_message(&mut self, message: Message) {
        self.sequence = self.sequence.next();

        match message.kind {
            MessageKind::Data => {}
            MessageKind::Event => {}
            MessageKind::Command => {}
            MessageKind::System => {}
        }
    }

    fn process_strategy_commands(&mut self, commands: Vec<StrategyCommand>) {
        for cmd in commands {
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
}

impl Default for PositionEngine {
    fn default() -> Self {
        Self::new()
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
        let mut live_clock = Clock::new(ClockMode::Live);
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
