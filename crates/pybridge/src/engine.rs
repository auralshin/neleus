use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use std::sync::Mutex;
use neleus_core_types::UnixNanos;
use super::types::*;

use neleus_core_bus::InMemoryBus;
use neleus_core_engine::{
    CapitalConfig, ClockMode, DynamicLimitsConfig, Engine, EngineConfig, EngineState,
    LeverageConfig, MarketDataEvent, PositionManagementConfig,
};

#[pyclass(name = "EngineConfig")]
#[derive(Debug, Clone)]
pub struct PyEngineConfig {
    #[pyo3(get, set)]
    pub instance_id: String,
    #[pyo3(get, set)]
    pub max_events_per_tick: usize,
    #[pyo3(get, set)]
    pub enable_event_log: bool,
    #[pyo3(get, set)]
    pub simulated_mode: bool,
    #[pyo3(get, set)]
    pub initial_capital: f64,
    #[pyo3(get, set)]
    pub max_open_positions: usize,
    #[pyo3(get, set)]
    pub max_leverage: f64,
}

#[pymethods]
impl PyEngineConfig {
    #[new]
    #[pyo3(signature = (
        instance_id = "neleus-1".to_string(),
        max_events_per_tick = 1000,
        enable_event_log = true,
        simulated_mode = false,
        initial_capital = 100000.0,
        max_open_positions = 10,
        max_leverage = 10.0
    ))]
    pub fn new(
        instance_id: String,
        max_events_per_tick: usize,
        enable_event_log: bool,
        simulated_mode: bool,
        initial_capital: f64,
        max_open_positions: usize,
        max_leverage: f64,
    ) -> Self {
        Self {
            instance_id,
            max_events_per_tick,
            enable_event_log,
            simulated_mode,
            initial_capital,
            max_open_positions,
            max_leverage,
        }
    }

    pub fn __repr__(&self) -> String {
        format!(
            "EngineConfig(instance_id='{}', simulated={})",
            self.instance_id, self.simulated_mode
        )
    }
}

impl From<&PyEngineConfig> for EngineConfig {
    fn from(config: &PyEngineConfig) -> Self {
        let mut capital_config = CapitalConfig::default();
        capital_config.initial_capital = config.initial_capital;

        let mut position_config = PositionManagementConfig::default();
        position_config.max_open_positions = config.max_open_positions;

        let mut leverage_config = LeverageConfig::default();
        leverage_config.max_leverage = config.max_leverage;

        Self {
            instance_id: config.instance_id.clone(),
            max_events_per_tick: config.max_events_per_tick,
            enable_event_log: config.enable_event_log,
            clock_mode: if config.simulated_mode {
                ClockMode::Simulated
            } else {
                ClockMode::Live
            },
            capital_config,
            position_config,
            leverage_config,
        }
    }
}

#[pyclass(eq, eq_int, name = "EngineState")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyEngineState {
    Idle,
    Starting,
    Running,
    Stopping,
    Stopped,
    Paused,
}

impl From<EngineState> for PyEngineState {
    fn from(state: EngineState) -> Self {
        match state {
            EngineState::Idle => PyEngineState::Idle,
            EngineState::Starting => PyEngineState::Starting,
            EngineState::Running => PyEngineState::Running,
            EngineState::Stopping => PyEngineState::Stopping,
            EngineState::Stopped => PyEngineState::Stopped,
            EngineState::Paused => PyEngineState::Paused,
            EngineState::Faulted => PyEngineState::Stopped,
        }
    }
}

#[pyclass(name = "Engine")]
pub struct PyEngine {
    engine: Mutex<Engine<InMemoryBus>>,
    config: PyEngineConfig,
}

#[pymethods]
impl PyEngine {
    #[new]
    pub fn new(config: PyEngineConfig) -> PyResult<Self> {
        let engine_config = EngineConfig::from(&config);
        let engine = Engine::new(engine_config);

        Ok(Self {
            engine: Mutex::new(engine),
            config,
        })
    }

    pub fn start(&self) -> PyResult<()> {
        let mut engine = self
            .engine
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        engine.start();
        Ok(())
    }

    pub fn stop(&self) -> PyResult<()> {
        let mut engine = self
            .engine
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        engine.stop();
        Ok(())
    }

    pub fn state(&self) -> PyResult<PyEngineState> {
        let engine = self
            .engine
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        Ok(engine.state().into())
    }

    pub fn time_ns(&self) -> PyResult<u64> {
        let engine = self
            .engine
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        Ok(engine.time().0)
    }

    pub fn advance_time(&self, time_ns: u64) -> PyResult<()> {
        let mut engine = self
            .engine
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        engine.advance_time(UnixNanos(time_ns));
        Ok(())
    }

    pub fn tick(&self) -> PyResult<usize> {
        let mut engine = self
            .engine
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        let commands = engine.tick_collect_commands();
        Ok(commands.len())
    }

    pub fn on_trade(
        &self,
        instrument: &PyInstrumentId,
        price: f64,
        quantity: f64,
        side: PyOrderSide,
        time_ns: u64,
    ) -> PyResult<()> {
        let mut engine = self
            .engine
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;

        let event = MarketDataEvent::Trade {
            instrument_id: instrument.to_rust(),
            price,
            quantity,
            side: match side {
                PyOrderSide::Buy => neleus_core_engine::OrderSide::Buy,
                PyOrderSide::Sell => neleus_core_engine::OrderSide::Sell,
            },
            ts: UnixNanos(time_ns),
        };

        let _commands = engine.on_market_data(event);
        Ok(())
    }

    pub fn on_quote(
        &self,
        instrument: &PyInstrumentId,
        bid_price: f64,
        bid_size: f64,
        ask_price: f64,
        ask_size: f64,
        time_ns: u64,
    ) -> PyResult<()> {
        let mut engine = self
            .engine
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;

        let event = MarketDataEvent::Quote {
            instrument_id: instrument.to_rust(),
            bid_price,
            bid_size,
            ask_price,
            ask_size,
            ts: UnixNanos(time_ns),
        };

        let _commands = engine.on_market_data(event);
        Ok(())
    }

    pub fn set_risk_limits(
        &self,
        max_position_notional: f64,
        max_daily_loss: f64,
        max_leverage: f64,
    ) -> PyResult<()> {
        let mut engine = self
            .engine
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;

        let config = DynamicLimitsConfig {
            base_position_limit: max_position_notional,
            base_daily_loss_limit: max_daily_loss,
            base_leverage_limit: max_leverage,
            ..Default::default()
        };

        engine.set_risk_manager(config, self.config.initial_capital);
        Ok(())
    }

    pub fn update_daily_pnl(&self, pnl: f64) -> PyResult<()> {
        let mut engine = self
            .engine
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        engine.update_daily_pnl(pnl);
        Ok(())
    }

    pub fn update_leverage(&self, leverage: f64) -> PyResult<()> {
        let mut engine = self
            .engine
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        engine.update_leverage(leverage);
        Ok(())
    }

    pub fn pending_count(&self) -> PyResult<usize> {
        let engine = self
            .engine
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        Ok(engine.pending_count())
    }

    pub fn config(&self) -> PyEngineConfig {
        self.config.clone()
    }

    pub fn __repr__(&self) -> String {
        let state = self.state().unwrap_or(PyEngineState::Idle);
        format!(
            "Engine(instance_id='{}', state={:?})",
            self.config.instance_id, state
        )
    }
}

use neleus_core_engine::{CircuitBreakerConfig, CircuitState};

#[pyclass(name = "LiveNodeState", eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyLiveNodeState {
    Disconnected,
    Connecting,
    Connected,
    Trading,
    Error,
    ShuttingDown,
}

#[pyclass(name = "LiveNodeConfig")]
#[derive(Debug, Clone)]
pub struct PyLiveNodeConfig {
    #[pyo3(get, set)]
    pub instance_id: String,

    #[pyo3(get, set)]
    pub venue: PyVenue,

    #[pyo3(get, set)]
    pub api_key: Option<String>,

    #[pyo3(get, set)]
    pub api_secret: Option<String>,

    #[pyo3(get, set)]
    pub use_testnet: bool,

    #[pyo3(get, set)]
    pub initial_capital: f64,

    #[pyo3(get, set)]
    pub max_position_notional: f64,

    #[pyo3(get, set)]
    pub max_daily_loss: f64,

    #[pyo3(get, set)]
    pub max_leverage: f64,

    #[pyo3(get, set)]
    pub circuit_breaker_threshold: u32,

    #[pyo3(get, set)]
    pub circuit_breaker_recovery_ms: u64,

    #[pyo3(get, set)]
    pub reconnect_initial_delay_ms: u64,

    #[pyo3(get, set)]
    pub reconnect_max_delay_ms: u64,

    #[pyo3(get, set)]
    pub paper_trading: bool,
}

#[pymethods]
impl PyLiveNodeConfig {
    #[new]
    #[pyo3(signature = (
        instance_id,
        venue,
        api_key = None,
        api_secret = None,
        use_testnet = false,
        initial_capital = 100000.0,
        max_position_notional = 50000.0,
        max_daily_loss = 5000.0,
        max_leverage = 10.0,
        circuit_breaker_threshold = 5,
        circuit_breaker_recovery_ms = 30000,
        reconnect_initial_delay_ms = 1000,
        reconnect_max_delay_ms = 30000,
        paper_trading = true
    ))]
    pub fn new(
        instance_id: String,
        venue: PyVenue,
        api_key: Option<String>,
        api_secret: Option<String>,
        use_testnet: bool,
        initial_capital: f64,
        max_position_notional: f64,
        max_daily_loss: f64,
        max_leverage: f64,
        circuit_breaker_threshold: u32,
        circuit_breaker_recovery_ms: u64,
        reconnect_initial_delay_ms: u64,
        reconnect_max_delay_ms: u64,
        paper_trading: bool,
    ) -> Self {
        Self {
            instance_id,
            venue,
            api_key,
            api_secret,
            use_testnet,
            initial_capital,
            max_position_notional,
            max_daily_loss,
            max_leverage,
            circuit_breaker_threshold,
            circuit_breaker_recovery_ms,
            reconnect_initial_delay_ms,
            reconnect_max_delay_ms,
            paper_trading,
        }
    }

    pub fn __repr__(&self) -> String {
        format!(
            "LiveNodeConfig(instance_id='{}', venue={:?}, testnet={}, paper={})",
            self.instance_id, self.venue, self.use_testnet, self.paper_trading
        )
    }
}

#[pyclass(name = "CircuitState", eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyCircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl From<CircuitState> for PyCircuitState {
    fn from(state: CircuitState) -> Self {
        match state {
            CircuitState::Closed => PyCircuitState::Closed,
            CircuitState::Open => PyCircuitState::Open,
            CircuitState::HalfOpen => PyCircuitState::HalfOpen,
        }
    }
}

#[pyclass(name = "LiveNode")]
pub struct PyLiveNode {
    config: PyLiveNodeConfig,
    engine: Mutex<Engine<InMemoryBus>>,
    state: Mutex<PyLiveNodeState>,
    circuit_breaker: std::sync::Arc<neleus_core_engine::CircuitBreaker>,
    daily_pnl: Mutex<f64>,
    total_orders: Mutex<u64>,
    total_fills: Mutex<u64>,
    connection_errors: Mutex<u32>,
}

#[pymethods]
impl PyLiveNode {
    #[new]
    pub fn new(config: PyLiveNodeConfig) -> PyResult<Self> {
        let engine_config = EngineConfig {
            instance_id: config.instance_id.clone(),
            max_events_per_tick: 1000,
            enable_event_log: true,
            clock_mode: ClockMode::Live,
            capital_config: CapitalConfig {
                initial_capital: config.initial_capital,
                ..Default::default()
            },
            position_config: PositionManagementConfig::default(),
            leverage_config: LeverageConfig {
                max_leverage: config.max_leverage,
                ..Default::default()
            },
        };

        let engine = Engine::new(engine_config);

        let cb_config = CircuitBreakerConfig {
            failure_threshold: config.circuit_breaker_threshold,
            recovery_timeout_ms: config.circuit_breaker_recovery_ms,
            ..Default::default()
        };
        let circuit_breaker = std::sync::Arc::new(neleus_core_engine::CircuitBreaker::new(
            &config.instance_id,
            cb_config,
        ));

        Ok(Self {
            config,
            engine: Mutex::new(engine),
            state: Mutex::new(PyLiveNodeState::Disconnected),
            circuit_breaker,
            daily_pnl: Mutex::new(0.0),
            total_orders: Mutex::new(0),
            total_fills: Mutex::new(0),
            connection_errors: Mutex::new(0),
        })
    }

    pub fn state(&self) -> PyResult<PyLiveNodeState> {
        Ok(*self
            .state
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?)
    }

    pub fn circuit_state(&self) -> PyCircuitState {
        self.circuit_breaker.state().into()
    }

    pub fn circuit_allows_request(&self) -> bool {
        self.circuit_breaker.allow_request().is_ok()
    }

    pub fn record_success(&self) {
        self.circuit_breaker.record_success();
    }

    pub fn record_failure(&self, error: String) {
        self.circuit_breaker.record_failure(error);
    }

    pub fn reset_circuit_breaker(&self) {
        self.circuit_breaker.reset();
    }

    pub fn connect(&self) -> PyResult<()> {
        if !self.circuit_allows_request() {
            return Err(PyRuntimeError::new_err(format!(
                "Circuit breaker is open - cannot connect. State: {:?}",
                self.circuit_state()
            )));
        }

        if self.config.api_key.is_none() && !self.config.paper_trading {
            return Err(PyRuntimeError::new_err(
                "API key required for live trading. Set paper_trading=True for simulation.",
            ));
        }

        let mut state = self
            .state
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        *state = PyLiveNodeState::Connecting;

        *state = PyLiveNodeState::Connected;

        let mut engine = self
            .engine
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        engine.start();

        self.record_success();
        Ok(())
    }

    pub fn disconnect(&self) -> PyResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        *state = PyLiveNodeState::ShuttingDown;

        let mut engine = self
            .engine
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        engine.stop();

        *state = PyLiveNodeState::Disconnected;
        Ok(())
    }

    pub fn start_trading(&self) -> PyResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;

        if *state != PyLiveNodeState::Connected {
            return Err(PyRuntimeError::new_err(format!(
                "Cannot start trading - not connected. State: {:?}",
                *state
            )));
        }

        if !self.circuit_allows_request() {
            return Err(PyRuntimeError::new_err(
                "Circuit breaker is open - trading disabled",
            ));
        }

        *state = PyLiveNodeState::Trading;
        Ok(())
    }

    pub fn stop_trading(&self) -> PyResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;

        if *state == PyLiveNodeState::Trading {
            *state = PyLiveNodeState::Connected;
        }
        Ok(())
    }

    #[allow(unused_variables)]
    #[pyo3(signature = (instrument, side, order_type, quantity, price = None))]
    pub fn submit_order(
        &self,
        instrument: &PyInstrumentId,
        side: PyOrderSide,
        order_type: PyOrderType,
        quantity: f64,
        price: Option<f64>,
    ) -> PyResult<String> {
        let state = *self
            .state
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;

        if state != PyLiveNodeState::Trading {
            return Err(PyRuntimeError::new_err(format!(
                "Cannot submit order - not in trading state. State: {:?}",
                state
            )));
        }

        if !self.circuit_allows_request() {
            return Err(PyRuntimeError::new_err(
                "Circuit breaker is open - orders blocked",
            ));
        }

        let daily_pnl = *self
            .daily_pnl
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        if daily_pnl < -self.config.max_daily_loss {
            return Err(PyRuntimeError::new_err(format!(
                "Daily loss limit exceeded: {} < -{}",
                daily_pnl, self.config.max_daily_loss
            )));
        }

        let mut orders = self
            .total_orders
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        *orders += 1;
        let order_id = format!("{}_{}", self.config.instance_id, *orders);

        if self.config.paper_trading {
            self.record_success();
            return Ok(order_id);
        }

        self.record_success();
        Ok(order_id)
    }

    #[allow(unused_variables)]
    pub fn cancel_order(&self, order_id: String) -> PyResult<()> {
        if !self.circuit_allows_request() {
            return Err(PyRuntimeError::new_err(
                "Circuit breaker is open - cancels blocked",
            ));
        }

        self.record_success();
        Ok(())
    }

    #[allow(unused_variables)]
    pub fn on_fill(
        &self,
        instrument: &PyInstrumentId,
        side: PyOrderSide,
        price: f64,
        quantity: f64,
        realized_pnl: f64,
    ) -> PyResult<()> {
        {
            let mut fills = self
                .total_fills
                .lock()
                .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
            *fills += 1;
        }

        {
            let mut daily = self
                .daily_pnl
                .lock()
                .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
            *daily += realized_pnl;
        }

        let mut engine = self
            .engine
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        engine.update_daily_pnl(realized_pnl);

        self.record_success();
        Ok(())
    }

    pub fn on_connection_error(&self, error: String) -> PyResult<()> {
        {
            let mut errors = self
                .connection_errors
                .lock()
                .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
            *errors += 1;
        }

        self.record_failure(error);

        if self.circuit_state() == PyCircuitState::Open {
            let mut state = self
                .state
                .lock()
                .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
            *state = PyLiveNodeState::Error;
        }

        Ok(())
    }

    pub fn daily_pnl(&self) -> PyResult<f64> {
        Ok(*self
            .daily_pnl
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?)
    }

    pub fn total_orders(&self) -> PyResult<u64> {
        Ok(*self
            .total_orders
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?)
    }

    pub fn total_fills(&self) -> PyResult<u64> {
        Ok(*self
            .total_fills
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?)
    }

    pub fn connection_errors(&self) -> PyResult<u32> {
        Ok(*self
            .connection_errors
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?)
    }

    pub fn reset_daily_stats(&self) -> PyResult<()> {
        *self
            .daily_pnl
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))? = 0.0;
        Ok(())
    }

    pub fn config(&self) -> PyLiveNodeConfig {
        self.config.clone()
    }

    pub fn is_ready(&self) -> PyResult<bool> {
        let state = *self
            .state
            .lock()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        let circuit_ok = self.circuit_allows_request();
        let daily_ok = {
            let pnl = *self
                .daily_pnl
                .lock()
                .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
            pnl > -self.config.max_daily_loss
        };

        Ok(state == PyLiveNodeState::Trading && circuit_ok && daily_ok)
    }

    pub fn __repr__(&self) -> String {
        let state = self
            .state
            .lock()
            .map(|s| *s)
            .unwrap_or(PyLiveNodeState::Error);
        format!(
            "LiveNode(instance_id='{}', venue={:?}, state={:?}, circuit={:?})",
            self.config.instance_id,
            self.config.venue,
            state,
            self.circuit_state()
        )
    }
}
