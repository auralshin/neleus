use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use neleus_core_types::{InstrumentId, OrderId, UnixNanos};

/// Order side for execution algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecOrderSide {
    Buy,
    Sell,
}

/// Execution algorithm state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionState {
    Pending,
    Active,
    Paused,
    Completed,
    Cancelled,
    Failed,
}

/// Market conditions for adaptive execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketConditions {
    /// Current bid price
    pub bid: f64,
    /// Current ask price
    pub ask: f64,
    /// Bid size at top of book
    pub bid_size: f64,
    /// Ask size at top of book
    pub ask_size: f64,
    /// Recent trade volume (last N seconds)
    pub recent_volume: f64,
    /// Volume time window in seconds
    pub volume_window_secs: u64,
    /// Current volatility (e.g., ATR or realized vol)
    pub volatility: f64,
    /// Order book imbalance (-1 to 1)
    pub book_imbalance: f64,
    /// Spread in basis points
    pub spread_bps: f64,
}

impl MarketConditions {
    pub fn new(
        bid: f64,
        ask: f64,
        bid_size: f64,
        ask_size: f64,
        recent_volume: f64,
        volatility: f64,
    ) -> Self {
        let mid = (bid + ask) / 2.0;
        let spread_bps = if mid > 0.0 { (ask - bid) / mid * 10000.0 } else { 0.0 };
        let total_size = bid_size + ask_size;
        let book_imbalance = if total_size > 0.0 {
            (bid_size - ask_size) / total_size
        } else {
            0.0
        };
        
        Self {
            bid,
            ask,
            bid_size,
            ask_size,
            recent_volume,
            volume_window_secs: 60,
            volatility,
            book_imbalance,
            spread_bps,
        }
    }
    
    pub fn mid_price(&self) -> f64 {
        (self.bid + self.ask) / 2.0
    }
}

// =============================================================================
// TWAP (Time-Weighted Average Price)
// =============================================================================

/// TWAP execution parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwapParams {
    /// Instrument to trade
    pub instrument_id: InstrumentId,
    /// Order side
    pub side: ExecOrderSide,
    /// Total quantity to execute
    pub total_quantity: f64,
    /// Duration in nanoseconds
    pub duration_ns: u64,
    /// Number of slices
    pub num_slices: u32,
    /// Randomize timing (avoid detection)
    pub randomize_timing: bool,
    /// Maximum randomization offset (as fraction of interval)
    pub max_timing_variance: f64,
    /// Limit price (optional, won't trade beyond)
    pub limit_price: Option<f64>,
    /// Catch up if behind schedule
    pub catch_up_enabled: bool,
    /// Maximum slice size (for catch up)
    pub max_slice_multiplier: f64,
}

impl Default for TwapParams {
    fn default() -> Self {
        Self {
            instrument_id: InstrumentId::new(
                neleus_core_types::Venue::Simulated,
                "BTC",
                neleus_core_types::InstrumentType::Perp,
            ),
            side: ExecOrderSide::Buy,
            total_quantity: 0.0,
            duration_ns: 600_000_000_000, // 10 minutes
            num_slices: 10,
            randomize_timing: true,
            max_timing_variance: 0.2,
            limit_price: None,
            catch_up_enabled: true,
            max_slice_multiplier: 2.0,
        }
    }
}

/// TWAP execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwapExecution {
    /// Unique execution ID
    pub id: String,
    /// Parameters
    pub params: TwapParams,
    /// Current state
    pub state: ExecutionState,
    /// Start time
    pub start_time: UnixNanos,
    /// Quantity executed so far
    pub executed_quantity: f64,
    /// Number of slices completed
    pub slices_completed: u32,
    /// Average execution price
    pub avg_price: f64,
    /// Total notional value executed
    pub total_notional: f64,
    /// Next scheduled slice time
    pub next_slice_time: UnixNanos,
    /// Child order IDs
    pub child_orders: Vec<OrderId>,
    /// Slippage vs arrival price (bps)
    pub slippage_bps: f64,
    /// Arrival price (price when algo started)
    pub arrival_price: f64,
}

impl TwapExecution {
    pub fn new(id: String, params: TwapParams, start_time: UnixNanos, arrival_price: f64) -> Self {
        let slice_interval = params.duration_ns / params.num_slices as u64;
        
        Self {
            id,
            params,
            state: ExecutionState::Active,
            start_time,
            executed_quantity: 0.0,
            slices_completed: 0,
            avg_price: 0.0,
            total_notional: 0.0,
            next_slice_time: start_time + UnixNanos::from_nanos(slice_interval),
            child_orders: Vec::new(),
            slippage_bps: 0.0,
            arrival_price,
        }
    }
    
    /// Calculate quantity for next slice
    pub fn next_slice_quantity(&self) -> f64 {
        let remaining = self.params.total_quantity - self.executed_quantity;
        let remaining_slices = self.params.num_slices - self.slices_completed;
        
        if remaining_slices == 0 {
            return 0.0;
        }
        
        let base_slice = remaining / remaining_slices as f64;
        
        // If catch up enabled and behind schedule, increase slice size
        if self.params.catch_up_enabled {
            let expected_executed = self.params.total_quantity 
                * (self.slices_completed as f64 / self.params.num_slices as f64);
            let shortfall = expected_executed - self.executed_quantity;
            
            if shortfall > 0.0 {
                let catch_up = base_slice + shortfall / remaining_slices as f64;
                return catch_up.min(base_slice * self.params.max_slice_multiplier);
            }
        }
        
        base_slice
    }
    
    /// Calculate next slice time with optional randomization
    pub fn calculate_next_slice_time(&self, current_time: UnixNanos) -> UnixNanos {
        let base_interval = self.params.duration_ns / self.params.num_slices as u64;
        
        if self.params.randomize_timing {
            // Add random variance to avoid detection
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            
            let mut hasher = DefaultHasher::new();
            current_time.0.hash(&mut hasher);
            self.slices_completed.hash(&mut hasher);
            let hash = hasher.finish();
            
            // Generate pseudo-random variance between -max_variance and +max_variance
            let variance_factor = (hash % 1000) as f64 / 1000.0 * 2.0 - 1.0;
            let variance_ns = (base_interval as f64 * self.params.max_timing_variance * variance_factor) as i64;
            
            let adjusted_interval = (base_interval as i64 + variance_ns).max(0) as u64;
            current_time + UnixNanos::from_nanos(adjusted_interval)
        } else {
            current_time + UnixNanos::from_nanos(base_interval)
        }
    }
    
    /// Record a fill
    pub fn record_fill(&mut self, quantity: f64, price: f64) {
        let new_notional = quantity * price;
        self.total_notional += new_notional;
        self.executed_quantity += quantity;
        
        if self.executed_quantity > 0.0 {
            self.avg_price = self.total_notional / self.executed_quantity;
        }
        
        // Update slippage
        if self.arrival_price > 0.0 {
            let execution_cost = match self.params.side {
                ExecOrderSide::Buy => (self.avg_price - self.arrival_price) / self.arrival_price,
                ExecOrderSide::Sell => (self.arrival_price - self.avg_price) / self.arrival_price,
            };
            self.slippage_bps = execution_cost * 10000.0;
        }
    }
    
    /// Check if execution is complete
    pub fn is_complete(&self) -> bool {
        self.executed_quantity >= self.params.total_quantity * 0.9999 // Allow tiny rounding
    }
    
    /// Check if limit price would be violated
    pub fn check_limit_price(&self, current_price: f64) -> bool {
        match self.params.limit_price {
            Some(limit) => match self.params.side {
                ExecOrderSide::Buy => current_price <= limit,
                ExecOrderSide::Sell => current_price >= limit,
            },
            None => true,
        }
    }
    
    /// Get progress percentage
    pub fn progress_pct(&self) -> f64 {
        if self.params.total_quantity > 0.0 {
            (self.executed_quantity / self.params.total_quantity) * 100.0
        } else {
            0.0
        }
    }
    
    /// Get remaining quantity
    pub fn remaining_quantity(&self) -> f64 {
        (self.params.total_quantity - self.executed_quantity).max(0.0)
    }
}

// =============================================================================
// VWAP (Volume-Weighted Average Price)
// =============================================================================

/// VWAP execution parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VwapParams {
    /// Instrument to trade
    pub instrument_id: InstrumentId,
    /// Order side
    pub side: ExecOrderSide,
    /// Total quantity to execute
    pub total_quantity: f64,
    /// Target participation rate (fraction of market volume)
    pub participation_rate: f64,
    /// Minimum slice size
    pub min_slice: f64,
    /// Maximum slice size
    pub max_slice: f64,
    /// Limit price (optional)
    pub limit_price: Option<f64>,
    /// Volume profile (optional, for predictive VWAP)
    pub volume_profile: Option<Vec<f64>>,
    /// Maximum duration (0 = no limit)
    pub max_duration_ns: u64,
    /// Update interval in nanoseconds
    pub update_interval_ns: u64,
}

impl Default for VwapParams {
    fn default() -> Self {
        Self {
            instrument_id: InstrumentId::new(
                neleus_core_types::Venue::Simulated,
                "BTC",
                neleus_core_types::InstrumentType::Perp,
            ),
            side: ExecOrderSide::Buy,
            total_quantity: 0.0,
            participation_rate: 0.1,
            min_slice: 0.0,
            max_slice: f64::MAX,
            limit_price: None,
            volume_profile: None,
            max_duration_ns: 0,
            update_interval_ns: 1_000_000_000, // 1 second
        }
    }
}

/// VWAP execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VwapExecution {
    /// Unique execution ID
    pub id: String,
    /// Parameters
    pub params: VwapParams,
    /// Current state
    pub state: ExecutionState,
    /// Start time
    pub start_time: UnixNanos,
    /// Quantity executed so far
    pub executed_quantity: f64,
    /// Average execution price
    pub avg_price: f64,
    /// Total notional value executed
    pub total_notional: f64,
    /// Market VWAP over execution period
    pub market_vwap: f64,
    /// Total market volume observed
    pub market_volume: f64,
    /// Actual participation rate achieved
    pub actual_participation_rate: f64,
    /// Child order IDs
    pub child_orders: Vec<OrderId>,
    /// Performance vs market VWAP (bps)
    pub performance_bps: f64,
    /// Volume history for tracking
    volume_history: VecDeque<(UnixNanos, f64, f64)>, // (time, volume, vwap_component)
}

impl VwapExecution {
    pub fn new(id: String, params: VwapParams, start_time: UnixNanos) -> Self {
        Self {
            id,
            params,
            state: ExecutionState::Active,
            start_time,
            executed_quantity: 0.0,
            avg_price: 0.0,
            total_notional: 0.0,
            market_vwap: 0.0,
            market_volume: 0.0,
            actual_participation_rate: 0.0,
            child_orders: Vec::new(),
            performance_bps: 0.0,
            volume_history: VecDeque::new(),
        }
    }
    
    /// Calculate slice quantity based on market volume
    pub fn calculate_slice_quantity(&self, market_volume_delta: f64) -> f64 {
        let target_quantity = market_volume_delta * self.params.participation_rate;
        let remaining = self.remaining_quantity();
        
        // Clamp to min/max and remaining
        target_quantity
            .max(self.params.min_slice)
            .min(self.params.max_slice)
            .min(remaining)
    }
    
    /// Update with market volume observation
    pub fn update_market_volume(&mut self, time: UnixNanos, volume: f64, price: f64) {
        self.volume_history.push_back((time, volume, volume * price));
        self.market_volume += volume;
        
        // Recalculate market VWAP
        let total_notional: f64 = self.volume_history.iter().map(|(_, _, n)| n).sum();
        let total_volume: f64 = self.volume_history.iter().map(|(_, v, _)| v).sum();
        
        if total_volume > 0.0 {
            self.market_vwap = total_notional / total_volume;
        }
        
        // Update actual participation rate
        if self.market_volume > 0.0 {
            self.actual_participation_rate = self.executed_quantity / self.market_volume;
        }
    }
    
    /// Record a fill
    pub fn record_fill(&mut self, quantity: f64, price: f64) {
        let new_notional = quantity * price;
        self.total_notional += new_notional;
        self.executed_quantity += quantity;
        
        if self.executed_quantity > 0.0 {
            self.avg_price = self.total_notional / self.executed_quantity;
        }
        
        // Update performance vs VWAP
        if self.market_vwap > 0.0 {
            let execution_cost = match self.params.side {
                ExecOrderSide::Buy => (self.avg_price - self.market_vwap) / self.market_vwap,
                ExecOrderSide::Sell => (self.market_vwap - self.avg_price) / self.market_vwap,
            };
            self.performance_bps = execution_cost * 10000.0;
        }
    }
    
    /// Get remaining quantity
    pub fn remaining_quantity(&self) -> f64 {
        (self.params.total_quantity - self.executed_quantity).max(0.0)
    }
    
    /// Check if complete
    pub fn is_complete(&self) -> bool {
        self.executed_quantity >= self.params.total_quantity * 0.9999
    }
    
    /// Get progress percentage
    pub fn progress_pct(&self) -> f64 {
        if self.params.total_quantity > 0.0 {
            (self.executed_quantity / self.params.total_quantity) * 100.0
        } else {
            0.0
        }
    }
}

// =============================================================================
// Iceberg Orders
// =============================================================================

/// Iceberg order parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcebergParams {
    /// Instrument to trade
    pub instrument_id: InstrumentId,
    /// Order side
    pub side: ExecOrderSide,
    /// Total quantity (hidden + visible)
    pub total_quantity: f64,
    /// Display quantity (visible portion)
    pub display_quantity: f64,
    /// Limit price
    pub limit_price: f64,
    /// Variance in display quantity (0-1, to avoid detection)
    pub display_variance: f64,
    /// Refresh strategy
    pub refresh_strategy: IcebergRefreshStrategy,
    /// Price improvement enabled (try to improve by 1 tick)
    pub price_improve: bool,
    /// Minimum time between refreshes (ns)
    pub min_refresh_interval_ns: u64,
}

/// Iceberg refresh strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IcebergRefreshStrategy {
    /// Immediate refresh on fill
    Immediate,
    /// Delayed refresh (random delay)
    Delayed,
    /// Conditional refresh based on book state
    Conditional,
}

impl Default for IcebergParams {
    fn default() -> Self {
        Self {
            instrument_id: InstrumentId::new(
                neleus_core_types::Venue::Simulated,
                "BTC",
                neleus_core_types::InstrumentType::Perp,
            ),
            side: ExecOrderSide::Buy,
            total_quantity: 0.0,
            display_quantity: 0.0,
            limit_price: 0.0,
            display_variance: 0.1,
            refresh_strategy: IcebergRefreshStrategy::Immediate,
            price_improve: false,
            min_refresh_interval_ns: 100_000_000, // 100ms
        }
    }
}

/// Iceberg order state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcebergExecution {
    /// Unique execution ID
    pub id: String,
    /// Parameters
    pub params: IcebergParams,
    /// Current state
    pub state: ExecutionState,
    /// Start time
    pub start_time: UnixNanos,
    /// Quantity executed
    pub executed_quantity: f64,
    /// Average execution price
    pub avg_price: f64,
    /// Total notional
    pub total_notional: f64,
    /// Current visible order ID
    pub current_order_id: Option<OrderId>,
    /// Current visible quantity
    pub current_display_quantity: f64,
    /// Number of refreshes
    pub refresh_count: u32,
    /// Last refresh time
    pub last_refresh_time: UnixNanos,
    /// All child order IDs
    pub child_orders: Vec<OrderId>,
}

impl IcebergExecution {
    pub fn new(id: String, params: IcebergParams, start_time: UnixNanos) -> Self {
        Self {
            id,
            params,
            state: ExecutionState::Active,
            start_time,
            executed_quantity: 0.0,
            avg_price: 0.0,
            total_notional: 0.0,
            current_order_id: None,
            current_display_quantity: 0.0,
            refresh_count: 0,
            last_refresh_time: start_time,
            child_orders: Vec::new(),
        }
    }
    
    /// Calculate next display quantity with variance
    pub fn next_display_quantity(&self) -> f64 {
        let remaining = self.remaining_quantity();
        let base = self.params.display_quantity.min(remaining);
        
        if self.params.display_variance > 0.0 {
            // Add random variance
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            
            let mut hasher = DefaultHasher::new();
            self.refresh_count.hash(&mut hasher);
            self.executed_quantity.to_bits().hash(&mut hasher);
            let hash = hasher.finish();
            
            let variance_factor = (hash % 1000) as f64 / 1000.0 * 2.0 - 1.0;
            let variance = base * self.params.display_variance * variance_factor;
            
            (base + variance).max(0.001).min(remaining)
        } else {
            base
        }
    }
    
    /// Check if should refresh
    pub fn should_refresh(&self, current_time: UnixNanos) -> bool {
        if self.current_order_id.is_some() {
            return false;
        }
        
        if self.is_complete() {
            return false;
        }
        
        let elapsed = current_time.0.saturating_sub(self.last_refresh_time.0);
        elapsed >= self.params.min_refresh_interval_ns
    }
    
    /// Record a fill
    pub fn record_fill(&mut self, quantity: f64, price: f64) {
        let new_notional = quantity * price;
        self.total_notional += new_notional;
        self.executed_quantity += quantity;
        
        if self.executed_quantity > 0.0 {
            self.avg_price = self.total_notional / self.executed_quantity;
        }
        
        // Clear current order if filled
        if self.current_display_quantity <= quantity {
            self.current_order_id = None;
            self.current_display_quantity = 0.0;
        } else {
            self.current_display_quantity -= quantity;
        }
    }
    
    /// Mark as refreshed
    pub fn mark_refreshed(&mut self, order_id: OrderId, quantity: f64, time: UnixNanos) {
        self.current_order_id = Some(order_id.clone());
        self.current_display_quantity = quantity;
        self.refresh_count += 1;
        self.last_refresh_time = time;
        self.child_orders.push(order_id);
    }
    
    /// Get remaining quantity
    pub fn remaining_quantity(&self) -> f64 {
        (self.params.total_quantity - self.executed_quantity).max(0.0)
    }
    
    /// Check if complete
    pub fn is_complete(&self) -> bool {
        self.executed_quantity >= self.params.total_quantity * 0.9999
    }
    
    /// Get progress percentage
    pub fn progress_pct(&self) -> f64 {
        if self.params.total_quantity > 0.0 {
            (self.executed_quantity / self.params.total_quantity) * 100.0
        } else {
            0.0
        }
    }
}

// =============================================================================
// Adaptive Execution
// =============================================================================

/// Adaptive execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdaptiveMode {
    /// Passive: wait for good prices, use limit orders
    Passive,
    /// Neutral: balance speed and price
    Neutral,
    /// Aggressive: prioritize speed, use market orders
    Aggressive,
    /// Opportunistic: take good prices when available
    Opportunistic,
}

/// Adaptive execution parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveParams {
    /// Instrument to trade
    pub instrument_id: InstrumentId,
    /// Order side
    pub side: ExecOrderSide,
    /// Total quantity to execute
    pub total_quantity: f64,
    /// Urgency level (0-1, higher = more aggressive)
    pub urgency: f64,
    /// Risk aversion (0-1, higher = more conservative)
    pub risk_aversion: f64,
    /// Maximum execution time (ns, 0 = no limit)
    pub max_duration_ns: u64,
    /// Price limit (optional)
    pub limit_price: Option<f64>,
    /// Volatility threshold for mode switching
    pub high_volatility_threshold: f64,
    /// Spread threshold (bps) for passive mode
    pub wide_spread_threshold_bps: f64,
    /// Minimum improvement over mid (bps)
    pub min_improvement_bps: f64,
}

impl Default for AdaptiveParams {
    fn default() -> Self {
        Self {
            instrument_id: InstrumentId::new(
                neleus_core_types::Venue::Simulated,
                "BTC",
                neleus_core_types::InstrumentType::Perp,
            ),
            side: ExecOrderSide::Buy,
            total_quantity: 0.0,
            urgency: 0.5,
            risk_aversion: 0.5,
            max_duration_ns: 0,
            limit_price: None,
            high_volatility_threshold: 0.02,
            wide_spread_threshold_bps: 10.0,
            min_improvement_bps: 1.0,
        }
    }
}

/// Adaptive execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveExecution {
    /// Unique execution ID
    pub id: String,
    /// Parameters
    pub params: AdaptiveParams,
    /// Current state
    pub state: ExecutionState,
    /// Current adaptive mode
    pub current_mode: AdaptiveMode,
    /// Start time
    pub start_time: UnixNanos,
    /// Quantity executed
    pub executed_quantity: f64,
    /// Average execution price
    pub avg_price: f64,
    /// Total notional
    pub total_notional: f64,
    /// Child order IDs
    pub child_orders: Vec<OrderId>,
    /// Mode history
    pub mode_changes: Vec<(UnixNanos, AdaptiveMode)>,
    /// Arrival price
    pub arrival_price: f64,
    /// Implementation shortfall (vs arrival, bps)
    pub implementation_shortfall_bps: f64,
}

impl AdaptiveExecution {
    pub fn new(id: String, params: AdaptiveParams, start_time: UnixNanos, arrival_price: f64) -> Self {
        let initial_mode = Self::calculate_initial_mode(&params);
        
        Self {
            id,
            params,
            state: ExecutionState::Active,
            current_mode: initial_mode,
            start_time,
            executed_quantity: 0.0,
            avg_price: 0.0,
            total_notional: 0.0,
            child_orders: Vec::new(),
            mode_changes: vec![(start_time, initial_mode)],
            arrival_price,
            implementation_shortfall_bps: 0.0,
        }
    }
    
    fn calculate_initial_mode(params: &AdaptiveParams) -> AdaptiveMode {
        if params.urgency > 0.8 {
            AdaptiveMode::Aggressive
        } else if params.urgency < 0.3 && params.risk_aversion > 0.6 {
            AdaptiveMode::Passive
        } else if params.urgency < 0.5 {
            AdaptiveMode::Opportunistic
        } else {
            AdaptiveMode::Neutral
        }
    }
    
    /// Update mode based on market conditions
    pub fn update_mode(&mut self, conditions: &MarketConditions, current_time: UnixNanos) {
        let new_mode = self.calculate_mode(conditions, current_time);
        
        if new_mode != self.current_mode {
            self.mode_changes.push((current_time, new_mode));
            self.current_mode = new_mode;
        }
    }
    
    fn calculate_mode(&self, conditions: &MarketConditions, current_time: UnixNanos) -> AdaptiveMode {
        // Check time pressure
        let time_elapsed_ratio = if self.params.max_duration_ns > 0 {
            let elapsed = current_time.0.saturating_sub(self.start_time.0);
            elapsed as f64 / self.params.max_duration_ns as f64
        } else {
            0.0
        };
        
        let remaining_ratio = self.remaining_quantity() / self.params.total_quantity;
        
        // If running out of time and still have quantity, go aggressive
        if time_elapsed_ratio > 0.8 && remaining_ratio > 0.3 {
            return AdaptiveMode::Aggressive;
        }
        
        // High volatility -> be more passive (avoid adverse selection)
        if conditions.volatility > self.params.high_volatility_threshold {
            if self.params.risk_aversion > 0.5 {
                return AdaptiveMode::Passive;
            } else {
                return AdaptiveMode::Neutral;
            }
        }
        
        // Wide spread -> be passive
        if conditions.spread_bps > self.params.wide_spread_threshold_bps {
            return AdaptiveMode::Passive;
        }
        
        // Favorable book imbalance -> be opportunistic
        let favorable_imbalance = match self.params.side {
            ExecOrderSide::Buy => conditions.book_imbalance > 0.3, // More bids = selling pressure
            ExecOrderSide::Sell => conditions.book_imbalance < -0.3, // More asks = buying pressure
        };
        
        if favorable_imbalance {
            return AdaptiveMode::Opportunistic;
        }
        
        // Default based on urgency
        if self.params.urgency > 0.7 {
            AdaptiveMode::Aggressive
        } else if self.params.urgency < 0.3 {
            AdaptiveMode::Passive
        } else {
            AdaptiveMode::Neutral
        }
    }
    
    /// Calculate slice quantity based on current mode
    pub fn calculate_slice_quantity(&self, conditions: &MarketConditions) -> f64 {
        let remaining = self.remaining_quantity();
        
        let base_fraction = match self.current_mode {
            AdaptiveMode::Passive => 0.05,
            AdaptiveMode::Neutral => 0.10,
            AdaptiveMode::Aggressive => 0.25,
            AdaptiveMode::Opportunistic => 0.15,
        };
        
        // Adjust for book depth
        let available_size = match self.params.side {
            ExecOrderSide::Buy => conditions.ask_size,
            ExecOrderSide::Sell => conditions.bid_size,
        };
        
        let depth_adjusted = remaining * base_fraction;
        depth_adjusted.min(available_size * 0.5).min(remaining)
    }
    
    /// Determine order type based on current mode
    pub fn order_type(&self) -> OrderTypeDecision {
        match self.current_mode {
            AdaptiveMode::Passive => OrderTypeDecision::Limit,
            AdaptiveMode::Neutral => OrderTypeDecision::LimitWithImprovement,
            AdaptiveMode::Aggressive => OrderTypeDecision::Market,
            AdaptiveMode::Opportunistic => OrderTypeDecision::LimitAtMid,
        }
    }
    
    /// Record a fill
    pub fn record_fill(&mut self, quantity: f64, price: f64) {
        let new_notional = quantity * price;
        self.total_notional += new_notional;
        self.executed_quantity += quantity;
        
        if self.executed_quantity > 0.0 {
            self.avg_price = self.total_notional / self.executed_quantity;
        }
        
        // Update implementation shortfall
        if self.arrival_price > 0.0 {
            let shortfall = match self.params.side {
                ExecOrderSide::Buy => (self.avg_price - self.arrival_price) / self.arrival_price,
                ExecOrderSide::Sell => (self.arrival_price - self.avg_price) / self.arrival_price,
            };
            self.implementation_shortfall_bps = shortfall * 10000.0;
        }
    }
    
    /// Get remaining quantity
    pub fn remaining_quantity(&self) -> f64 {
        (self.params.total_quantity - self.executed_quantity).max(0.0)
    }
    
    /// Check if complete
    pub fn is_complete(&self) -> bool {
        self.executed_quantity >= self.params.total_quantity * 0.9999
    }
    
    /// Get progress percentage
    pub fn progress_pct(&self) -> f64 {
        if self.params.total_quantity > 0.0 {
            (self.executed_quantity / self.params.total_quantity) * 100.0
        } else {
            0.0
        }
    }
}

/// Order type decision from adaptive algo
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderTypeDecision {
    Market,
    Limit,
    LimitAtMid,
    LimitWithImprovement,
}

// =============================================================================
// Execution Manager
// =============================================================================

/// Manages all active executions
#[derive(Debug, Default)]
pub struct ExecutionManager {
    twap_executions: std::collections::HashMap<String, TwapExecution>,
    vwap_executions: std::collections::HashMap<String, VwapExecution>,
    iceberg_executions: std::collections::HashMap<String, IcebergExecution>,
    adaptive_executions: std::collections::HashMap<String, AdaptiveExecution>,
    next_id: u64,
}

impl ExecutionManager {
    pub fn new() -> Self {
        Self::default()
    }
    
    fn generate_id(&mut self) -> String {
        self.next_id += 1;
        format!("EXEC-{}", self.next_id)
    }
    
    /// Start a TWAP execution
    pub fn start_twap(&mut self, params: TwapParams, start_time: UnixNanos, arrival_price: f64) -> String {
        let id = self.generate_id();
        let execution = TwapExecution::new(id.clone(), params, start_time, arrival_price);
        self.twap_executions.insert(id.clone(), execution);
        id
    }
    
    /// Start a VWAP execution
    pub fn start_vwap(&mut self, params: VwapParams, start_time: UnixNanos) -> String {
        let id = self.generate_id();
        let execution = VwapExecution::new(id.clone(), params, start_time);
        self.vwap_executions.insert(id.clone(), execution);
        id
    }
    
    /// Start an Iceberg execution
    pub fn start_iceberg(&mut self, params: IcebergParams, start_time: UnixNanos) -> String {
        let id = self.generate_id();
        let execution = IcebergExecution::new(id.clone(), params, start_time);
        self.iceberg_executions.insert(id.clone(), execution);
        id
    }
    
    /// Start an Adaptive execution
    pub fn start_adaptive(&mut self, params: AdaptiveParams, start_time: UnixNanos, arrival_price: f64) -> String {
        let id = self.generate_id();
        let execution = AdaptiveExecution::new(id.clone(), params, start_time, arrival_price);
        self.adaptive_executions.insert(id.clone(), execution);
        id
    }
    
    /// Get TWAP execution
    pub fn get_twap(&self, id: &str) -> Option<&TwapExecution> {
        self.twap_executions.get(id)
    }
    
    /// Get TWAP execution mutably
    pub fn get_twap_mut(&mut self, id: &str) -> Option<&mut TwapExecution> {
        self.twap_executions.get_mut(id)
    }
    
    /// Get VWAP execution
    pub fn get_vwap(&self, id: &str) -> Option<&VwapExecution> {
        self.vwap_executions.get(id)
    }
    
    /// Get VWAP execution mutably
    pub fn get_vwap_mut(&mut self, id: &str) -> Option<&mut VwapExecution> {
        self.vwap_executions.get_mut(id)
    }
    
    /// Get Iceberg execution
    pub fn get_iceberg(&self, id: &str) -> Option<&IcebergExecution> {
        self.iceberg_executions.get(id)
    }
    
    /// Get Iceberg execution mutably
    pub fn get_iceberg_mut(&mut self, id: &str) -> Option<&mut IcebergExecution> {
        self.iceberg_executions.get_mut(id)
    }
    
    /// Get Adaptive execution
    pub fn get_adaptive(&self, id: &str) -> Option<&AdaptiveExecution> {
        self.adaptive_executions.get(id)
    }
    
    /// Get Adaptive execution mutably
    pub fn get_adaptive_mut(&mut self, id: &str) -> Option<&mut AdaptiveExecution> {
        self.adaptive_executions.get_mut(id)
    }
    
    /// Cancel an execution
    pub fn cancel(&mut self, id: &str) -> bool {
        if let Some(exec) = self.twap_executions.get_mut(id) {
            exec.state = ExecutionState::Cancelled;
            return true;
        }
        if let Some(exec) = self.vwap_executions.get_mut(id) {
            exec.state = ExecutionState::Cancelled;
            return true;
        }
        if let Some(exec) = self.iceberg_executions.get_mut(id) {
            exec.state = ExecutionState::Cancelled;
            return true;
        }
        if let Some(exec) = self.adaptive_executions.get_mut(id) {
            exec.state = ExecutionState::Cancelled;
            return true;
        }
        false
    }
    
    /// Get all active executions count
    pub fn active_count(&self) -> usize {
        self.twap_executions.values().filter(|e| e.state == ExecutionState::Active).count()
            + self.vwap_executions.values().filter(|e| e.state == ExecutionState::Active).count()
            + self.iceberg_executions.values().filter(|e| e.state == ExecutionState::Active).count()
            + self.adaptive_executions.values().filter(|e| e.state == ExecutionState::Active).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_twap_execution() {
        let params = TwapParams {
            total_quantity: 100.0,
            num_slices: 10,
            duration_ns: 600_000_000_000,
            ..Default::default()
        };
        
        let mut exec = TwapExecution::new(
            "test-1".to_string(),
            params,
            UnixNanos::from_secs(0),
            50000.0,
        );
        
        // First slice should be 10.0
        assert!((exec.next_slice_quantity() - 10.0).abs() < 0.01);
        
        // Record some fills
        exec.record_fill(10.0, 50010.0);
        exec.slices_completed = 1;
        
        assert_eq!(exec.executed_quantity, 10.0);
        assert!((exec.avg_price - 50010.0).abs() < 0.01);
        assert!((exec.progress_pct() - 10.0).abs() < 0.01);
    }
    
    #[test]
    fn test_vwap_execution() {
        let params = VwapParams {
            total_quantity: 100.0,
            participation_rate: 0.1,
            ..Default::default()
        };
        
        let mut exec = VwapExecution::new(
            "test-2".to_string(),
            params,
            UnixNanos::from_secs(0),
        );
        
        // With 100 volume, should execute 10
        assert!((exec.calculate_slice_quantity(100.0) - 10.0).abs() < 0.01);
        
        exec.update_market_volume(UnixNanos::from_secs(1), 100.0, 50000.0);
        exec.record_fill(10.0, 49990.0);
        
        assert!((exec.actual_participation_rate - 0.1).abs() < 0.01);
    }
    
    #[test]
    fn test_iceberg_execution() {
        let params = IcebergParams {
            total_quantity: 100.0,
            display_quantity: 10.0,
            limit_price: 50000.0,
            display_variance: 0.0, // No variance for test
            ..Default::default()
        };
        
        let mut exec = IcebergExecution::new(
            "test-3".to_string(),
            params,
            UnixNanos::from_secs(0),
        );
        
        // Display should be 10.0
        assert!((exec.next_display_quantity() - 10.0).abs() < 0.01);
        
        exec.record_fill(10.0, 50000.0);
        assert_eq!(exec.executed_quantity, 10.0);
        assert_eq!(exec.remaining_quantity(), 90.0);
    }
    
    #[test]
    fn test_adaptive_execution() {
        let params = AdaptiveParams {
            total_quantity: 100.0,
            urgency: 0.9, // High urgency
            ..Default::default()
        };
        
        let exec = AdaptiveExecution::new(
            "test-4".to_string(),
            params,
            UnixNanos::from_secs(0),
            50000.0,
        );
        
        // High urgency should start aggressive
        assert_eq!(exec.current_mode, AdaptiveMode::Aggressive);
    }
    
    #[test]
    fn test_execution_manager() {
        let mut manager = ExecutionManager::new();
        
        let twap_id = manager.start_twap(
            TwapParams { total_quantity: 100.0, ..Default::default() },
            UnixNanos::from_secs(0),
            50000.0,
        );
        
        assert!(manager.get_twap(&twap_id).is_some());
        assert_eq!(manager.active_count(), 1);
        
        manager.cancel(&twap_id);
        assert_eq!(manager.get_twap(&twap_id).unwrap().state, ExecutionState::Cancelled);
    }
}
