use neleus_core_engine::{OrderSide, OrderType, StrategyCommand};
use neleus_core_types::{InstrumentId, OrderId, UnixNanos};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

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
        let _slice_interval = config.duration_nanos / config.num_slices as u64;

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
