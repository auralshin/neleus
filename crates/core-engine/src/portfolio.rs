use neleus_core_types::{InstrumentId, StrategyId, UnixNanos};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Portfolio Position Tracking
// =============================================================================

/// Aggregated position across strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioPosition {
    /// Instrument ID
    pub instrument_id: InstrumentId,
    /// Net quantity (positive = long, negative = short)
    pub net_quantity: f64,
    /// Average entry price
    pub avg_entry_price: f64,
    /// Realized PnL
    pub realized_pnl: f64,
    /// Unrealized PnL
    pub unrealized_pnl: f64,
    /// Current market price
    pub current_price: f64,
    /// Strategy breakdown
    pub strategy_positions: HashMap<StrategyId, StrategyPosition>,
    /// Last update time
    pub updated_at: UnixNanos,
}

/// Position from a single strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPosition {
    pub quantity: f64,
    pub avg_entry_price: f64,
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
}

impl PortfolioPosition {
    pub fn new(instrument_id: InstrumentId) -> Self {
        Self {
            instrument_id,
            net_quantity: 0.0,
            avg_entry_price: 0.0,
            realized_pnl: 0.0,
            unrealized_pnl: 0.0,
            current_price: 0.0,
            strategy_positions: HashMap::new(),
            updated_at: UnixNanos::ZERO,
        }
    }

    pub fn notional(&self) -> f64 {
        self.net_quantity.abs() * self.current_price
    }

    pub fn total_pnl(&self) -> f64 {
        self.realized_pnl + self.unrealized_pnl
    }

    pub fn is_long(&self) -> bool {
        self.net_quantity > 0.0
    }

    pub fn is_short(&self) -> bool {
        self.net_quantity < 0.0
    }

    pub fn is_flat(&self) -> bool {
        self.net_quantity.abs() < 1e-10
    }

    /// Update with strategy position
    pub fn update_strategy(&mut self, strategy_id: StrategyId, position: StrategyPosition) {
        self.strategy_positions.insert(strategy_id, position);
        self.recalculate();
    }

    /// Recalculate aggregates from strategy positions
    fn recalculate(&mut self) {
        self.net_quantity = 0.0;
        self.realized_pnl = 0.0;
        self.unrealized_pnl = 0.0;

        let mut total_notional = 0.0;

        for pos in self.strategy_positions.values() {
            self.net_quantity += pos.quantity;
            self.realized_pnl += pos.realized_pnl;
            self.unrealized_pnl += pos.unrealized_pnl;

            if pos.quantity != 0.0 {
                total_notional += pos.quantity * pos.avg_entry_price;
            }
        }

        if self.net_quantity.abs() > 1e-10 {
            self.avg_entry_price = total_notional / self.net_quantity;
        }
    }
}

// =============================================================================
// Strategy Performance Tracking
// =============================================================================

/// Strategy performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPerformance {
    /// Strategy ID
    pub strategy_id: StrategyId,
    /// Total PnL
    pub total_pnl: f64,
    /// Realized PnL
    pub realized_pnl: f64,
    /// Unrealized PnL
    pub unrealized_pnl: f64,
    /// Total trades
    pub total_trades: u64,
    /// Winning trades
    pub winning_trades: u64,
    /// Win rate
    pub win_rate: f64,
    /// Sharpe ratio (rolling)
    pub sharpe_ratio: f64,
    /// Maximum drawdown
    pub max_drawdown_pct: f64,
    /// Current drawdown
    pub current_drawdown_pct: f64,
    /// Profit factor
    pub profit_factor: f64,
    /// Average trade PnL
    pub avg_trade_pnl: f64,
    /// Capital allocated
    pub capital_allocated: f64,
    /// Capital utilization
    pub capital_utilization: f64,
    /// Return on capital
    pub return_on_capital: f64,
    /// Daily returns for rolling calculations
    daily_returns: Vec<f64>,
    /// Equity curve (timestamp, equity)
    equity_curve: Vec<(UnixNanos, f64)>,
    /// Peak equity for drawdown
    peak_equity: f64,
}

impl StrategyPerformance {
    pub fn new(strategy_id: StrategyId, initial_capital: f64) -> Self {
        Self {
            strategy_id,
            total_pnl: 0.0,
            realized_pnl: 0.0,
            unrealized_pnl: 0.0,
            total_trades: 0,
            winning_trades: 0,
            win_rate: 0.0,
            sharpe_ratio: 0.0,
            max_drawdown_pct: 0.0,
            current_drawdown_pct: 0.0,
            profit_factor: 0.0,
            avg_trade_pnl: 0.0,
            capital_allocated: initial_capital,
            capital_utilization: 0.0,
            return_on_capital: 0.0,
            daily_returns: Vec::new(),
            equity_curve: Vec::new(),
            peak_equity: initial_capital,
        }
    }

    /// Record a completed trade
    pub fn record_trade(&mut self, pnl: f64, timestamp: UnixNanos) {
        self.total_trades += 1;
        self.realized_pnl += pnl;

        if pnl > 0.0 {
            self.winning_trades += 1;
        }

        if self.total_trades > 0 {
            self.win_rate = self.winning_trades as f64 / self.total_trades as f64;
            self.avg_trade_pnl = self.realized_pnl / self.total_trades as f64;
        }

        self.total_pnl = self.realized_pnl + self.unrealized_pnl;
        self.update_equity(timestamp);
    }

    /// Update unrealized PnL
    pub fn update_unrealized(&mut self, unrealized: f64, timestamp: UnixNanos) {
        self.unrealized_pnl = unrealized;
        self.total_pnl = self.realized_pnl + self.unrealized_pnl;
        self.update_equity(timestamp);
    }

    fn update_equity(&mut self, timestamp: UnixNanos) {
        let equity = self.capital_allocated + self.total_pnl;
        self.equity_curve.push((timestamp, equity));

        if equity > self.peak_equity {
            self.peak_equity = equity;
        }

        if self.peak_equity > 0.0 {
            self.current_drawdown_pct = (self.peak_equity - equity) / self.peak_equity * 100.0;
            if self.current_drawdown_pct > self.max_drawdown_pct {
                self.max_drawdown_pct = self.current_drawdown_pct;
            }
        }

        if self.capital_allocated > 0.0 {
            self.return_on_capital = self.total_pnl / self.capital_allocated * 100.0;
        }
    }

    /// Record daily return for Sharpe calculation
    pub fn record_daily_return(&mut self, daily_return: f64) {
        self.daily_returns.push(daily_return);

        // Calculate rolling Sharpe (annualized)
        if self.daily_returns.len() >= 2 {
            let mean = self.daily_returns.iter().sum::<f64>() / self.daily_returns.len() as f64;
            let variance = self
                .daily_returns
                .iter()
                .map(|r| (r - mean).powi(2))
                .sum::<f64>()
                / (self.daily_returns.len() - 1) as f64;
            let std_dev = variance.sqrt();

            if std_dev > 0.0 {
                self.sharpe_ratio = (mean / std_dev) * (252.0_f64).sqrt();
            }
        }
    }
}

// =============================================================================
// Capital Allocation
// =============================================================================

/// Capital allocation method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllocationMethod {
    /// Equal allocation to all strategies
    Equal,
    /// Risk parity (equal risk contribution)
    RiskParity,
    /// Performance-weighted (more capital to better performers)
    PerformanceWeighted,
    /// Volatility-adjusted
    VolatilityAdjusted,
    /// Kelly criterion based
    Kelly,
    /// Fixed weights (manual)
    Fixed,
}

/// Capital allocation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationConfig {
    /// Allocation method
    pub method: AllocationMethod,
    /// Total portfolio capital
    pub total_capital: f64,
    /// Minimum allocation per strategy
    pub min_allocation: f64,
    /// Maximum allocation per strategy
    pub max_allocation: f64,
    /// Rebalance threshold (as fraction)
    pub rebalance_threshold: f64,
    /// Fixed weights (for Fixed method)
    pub fixed_weights: HashMap<StrategyId, f64>,
    /// Kelly fraction (0-1, typically 0.25-0.5)
    pub kelly_fraction: f64,
    /// Lookback period for performance calculation (days)
    pub performance_lookback_days: u32,
}

impl Default for AllocationConfig {
    fn default() -> Self {
        Self {
            method: AllocationMethod::Equal,
            total_capital: 100000.0,
            min_allocation: 0.0,
            max_allocation: 1.0,
            rebalance_threshold: 0.05,
            fixed_weights: HashMap::new(),
            kelly_fraction: 0.25,
            performance_lookback_days: 30,
        }
    }
}

/// Capital allocator
#[derive(Debug, Clone)]
pub struct CapitalAllocator {
    config: AllocationConfig,
    current_allocations: HashMap<StrategyId, f64>,
    strategy_volatilities: HashMap<StrategyId, f64>,
}

impl CapitalAllocator {
    pub fn new(config: AllocationConfig) -> Self {
        Self {
            config,
            current_allocations: HashMap::new(),
            strategy_volatilities: HashMap::new(),
        }
    }

    /// Calculate target allocations
    pub fn calculate_allocations(
        &self,
        strategies: &HashMap<StrategyId, StrategyPerformance>,
    ) -> HashMap<StrategyId, f64> {
        if strategies.is_empty() {
            return HashMap::new();
        }

        let weights = match self.config.method {
            AllocationMethod::Equal => self.equal_weights(strategies),
            AllocationMethod::RiskParity => self.risk_parity_weights(strategies),
            AllocationMethod::PerformanceWeighted => self.performance_weights(strategies),
            AllocationMethod::VolatilityAdjusted => self.volatility_weights(strategies),
            AllocationMethod::Kelly => self.kelly_weights(strategies),
            AllocationMethod::Fixed => self.fixed_weights(strategies),
        };

        // Convert weights to dollar allocations
        weights
            .into_iter()
            .map(|(id, weight)| {
                let clamped_weight = weight
                    .max(self.config.min_allocation)
                    .min(self.config.max_allocation);
                (id, self.config.total_capital * clamped_weight)
            })
            .collect()
    }

    fn equal_weights(
        &self,
        strategies: &HashMap<StrategyId, StrategyPerformance>,
    ) -> HashMap<StrategyId, f64> {
        let n = strategies.len() as f64;
        strategies.keys().map(|id| (id.clone(), 1.0 / n)).collect()
    }

    fn risk_parity_weights(
        &self,
        strategies: &HashMap<StrategyId, StrategyPerformance>,
    ) -> HashMap<StrategyId, f64> {
        // Inverse volatility weighting
        let inv_vols: Vec<(StrategyId, f64)> = strategies
            .iter()
            .map(|(id, _perf)| {
                let vol = self.strategy_volatilities.get(id).copied().unwrap_or(0.1);
                let inv_vol = if vol > 0.0 { 1.0 / vol } else { 1.0 };
                (id.clone(), inv_vol)
            })
            .collect();

        let total_inv_vol: f64 = inv_vols.iter().map(|(_, v)| v).sum();

        if total_inv_vol > 0.0 {
            inv_vols
                .into_iter()
                .map(|(id, inv_vol)| (id, inv_vol / total_inv_vol))
                .collect()
        } else {
            self.equal_weights(strategies)
        }
    }

    fn performance_weights(
        &self,
        strategies: &HashMap<StrategyId, StrategyPerformance>,
    ) -> HashMap<StrategyId, f64> {
        // Weight by Sharpe ratio (or return if no Sharpe)
        let scores: Vec<(StrategyId, f64)> = strategies
            .iter()
            .map(|(id, perf)| {
                let score = if perf.sharpe_ratio != 0.0 {
                    perf.sharpe_ratio.max(0.0) // Only positive Sharpes contribute
                } else {
                    (perf.return_on_capital / 100.0).max(0.0)
                };
                (id.clone(), score)
            })
            .collect();

        let total_score: f64 = scores.iter().map(|(_, s)| s).sum();

        if total_score > 0.0 {
            scores
                .into_iter()
                .map(|(id, score)| (id, score / total_score))
                .collect()
        } else {
            self.equal_weights(strategies)
        }
    }

    fn volatility_weights(
        &self,
        strategies: &HashMap<StrategyId, StrategyPerformance>,
    ) -> HashMap<StrategyId, f64> {
        // Target equal volatility contribution
        self.risk_parity_weights(strategies)
    }

    fn kelly_weights(
        &self,
        strategies: &HashMap<StrategyId, StrategyPerformance>,
    ) -> HashMap<StrategyId, f64> {
        // Kelly criterion: f = (bp - q) / b
        // where b = odds, p = win rate, q = 1 - p
        let kelly_fractions: Vec<(StrategyId, f64)> = strategies
            .iter()
            .filter_map(|(id, perf)| {
                if perf.win_rate > 0.0 && perf.total_trades > 10 {
                    let p = perf.win_rate;
                    let q = 1.0 - p;

                    // Estimate odds from profit factor
                    let b = if perf.profit_factor > 0.0 {
                        perf.profit_factor
                    } else {
                        1.0
                    };

                    let kelly = (b * p - q) / b;
                    let fractional_kelly = kelly * self.config.kelly_fraction;

                    Some((id.clone(), fractional_kelly.max(0.0)))
                } else {
                    None
                }
            })
            .collect();

        let total_kelly: f64 = kelly_fractions.iter().map(|(_, k)| k).sum();

        if total_kelly > 0.0 {
            kelly_fractions
                .into_iter()
                .map(|(id, k)| (id, k / total_kelly))
                .collect()
        } else {
            self.equal_weights(strategies)
        }
    }

    fn fixed_weights(
        &self,
        strategies: &HashMap<StrategyId, StrategyPerformance>,
    ) -> HashMap<StrategyId, f64> {
        let mut weights = HashMap::new();

        for id in strategies.keys() {
            let weight = self.config.fixed_weights.get(id).copied().unwrap_or(0.0);
            weights.insert(id.clone(), weight);
        }

        // Normalize if needed
        let total: f64 = weights.values().sum();
        if total > 0.0 && (total - 1.0).abs() > 0.01 {
            for weight in weights.values_mut() {
                *weight /= total;
            }
        }

        weights
    }

    /// Check if rebalancing is needed
    pub fn needs_rebalance(&self, strategies: &HashMap<StrategyId, StrategyPerformance>) -> bool {
        let target = self.calculate_allocations(strategies);

        for (id, target_alloc) in &target {
            if let Some(&current_alloc) = self.current_allocations.get(id) {
                let diff = (target_alloc - current_alloc).abs() / self.config.total_capital;
                if diff > self.config.rebalance_threshold {
                    return true;
                }
            }
        }

        false
    }

    /// Update strategy volatility
    pub fn update_volatility(&mut self, strategy_id: StrategyId, volatility: f64) {
        self.strategy_volatilities.insert(strategy_id, volatility);
    }

    /// Apply new allocations
    pub fn apply_allocations(&mut self, allocations: HashMap<StrategyId, f64>) {
        self.current_allocations = allocations;
    }
}

// =============================================================================
// Cross-Strategy Netting
// =============================================================================

/// Netting result for a position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NettingResult {
    /// Instrument ID
    pub instrument_id: InstrumentId,
    /// Gross long exposure
    pub gross_long: f64,
    /// Gross short exposure
    pub gross_short: f64,
    /// Net position
    pub net_position: f64,
    /// Netting efficiency (reduction in gross exposure)
    pub netting_efficiency: f64,
    /// Capital saved through netting
    pub capital_saved: f64,
}

/// Cross-strategy position netter
#[derive(Debug, Clone)]
pub struct PositionNetter {
    /// Positions by instrument
    positions: HashMap<InstrumentId, PortfolioPosition>,
}

impl PositionNetter {
    pub fn new() -> Self {
        Self {
            positions: HashMap::new(),
        }
    }

    /// Update position from strategy
    pub fn update_position(
        &mut self,
        instrument_id: InstrumentId,
        strategy_id: StrategyId,
        quantity: f64,
        avg_price: f64,
        realized_pnl: f64,
        unrealized_pnl: f64,
    ) {
        let position = self
            .positions
            .entry(instrument_id.clone())
            .or_insert_with(|| PortfolioPosition::new(instrument_id));

        position.update_strategy(
            strategy_id,
            StrategyPosition {
                quantity,
                avg_entry_price: avg_price,
                realized_pnl,
                unrealized_pnl,
            },
        );
    }

    /// Calculate netting for all instruments
    pub fn calculate_netting(&self, margin_requirement: f64) -> Vec<NettingResult> {
        self.positions
            .values()
            .map(|pos| {
                let gross_long: f64 = pos
                    .strategy_positions
                    .values()
                    .filter(|p| p.quantity > 0.0)
                    .map(|p| p.quantity)
                    .sum();

                let gross_short: f64 = pos
                    .strategy_positions
                    .values()
                    .filter(|p| p.quantity < 0.0)
                    .map(|p| p.quantity.abs())
                    .sum();

                let gross_total = gross_long + gross_short;
                let net_position = pos.net_quantity;
                let net_exposure = net_position.abs();

                let netting_efficiency = if gross_total > 0.0 {
                    1.0 - (net_exposure / gross_total)
                } else {
                    0.0
                };

                let gross_margin = gross_total * pos.current_price * margin_requirement;
                let net_margin = net_exposure * pos.current_price * margin_requirement;
                let capital_saved = gross_margin - net_margin;

                NettingResult {
                    instrument_id: pos.instrument_id.clone(),
                    gross_long,
                    gross_short,
                    net_position,
                    netting_efficiency,
                    capital_saved,
                }
            })
            .collect()
    }

    /// Get net positions for all instruments
    pub fn net_positions(&self) -> &HashMap<InstrumentId, PortfolioPosition> {
        &self.positions
    }

    /// Get total gross exposure
    pub fn gross_exposure(&self) -> f64 {
        self.positions
            .values()
            .map(|pos| {
                pos.strategy_positions
                    .values()
                    .map(|p| p.quantity.abs() * pos.current_price)
                    .sum::<f64>()
            })
            .sum()
    }

    /// Get total net exposure
    pub fn net_exposure(&self) -> f64 {
        self.positions
            .values()
            .map(|pos| pos.net_quantity.abs() * pos.current_price)
            .sum()
    }
}

impl Default for PositionNetter {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Performance Attribution
// =============================================================================

/// Attribution factor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributionFactor {
    /// Market/beta contribution
    Market,
    /// Sector contribution
    Sector,
    /// Style factor (momentum, value, etc.)
    Style,
    /// Specific/idiosyncratic contribution
    Specific,
    /// Timing contribution
    Timing,
    /// Selection contribution
    Selection,
}

/// Attribution result for a strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyAttribution {
    /// Strategy ID
    pub strategy_id: StrategyId,
    /// Total return
    pub total_return: f64,
    /// Factor contributions
    pub factor_contributions: HashMap<String, f64>,
    /// Active return (alpha)
    pub active_return: f64,
    /// Tracking error
    pub tracking_error: f64,
    /// Information ratio
    pub information_ratio: f64,
    /// Contribution to portfolio return
    pub portfolio_contribution: f64,
    /// Risk contribution to portfolio
    pub risk_contribution: f64,
}

/// Performance attributor
#[derive(Debug, Clone)]
pub struct PerformanceAttributor {
    /// Strategy performances
    performances: HashMap<StrategyId, StrategyPerformance>,
    /// Strategy weights
    weights: HashMap<StrategyId, f64>,
    /// Benchmark return (if any)
    benchmark_return: f64,
}

impl PerformanceAttributor {
    pub fn new() -> Self {
        Self {
            performances: HashMap::new(),
            weights: HashMap::new(),
            benchmark_return: 0.0,
        }
    }

    /// Update strategy performance
    pub fn update_performance(&mut self, performance: StrategyPerformance) {
        self.performances
            .insert(performance.strategy_id.clone(), performance);
    }

    /// Update strategy weights
    pub fn update_weights(&mut self, weights: HashMap<StrategyId, f64>) {
        self.weights = weights;
    }

    /// Set benchmark return
    pub fn set_benchmark(&mut self, benchmark_return: f64) {
        self.benchmark_return = benchmark_return;
    }

    /// Calculate attribution for all strategies
    pub fn calculate_attribution(&self) -> Vec<StrategyAttribution> {
        let total_weight: f64 = self.weights.values().sum();

        let _portfolio_return: f64 = self
            .performances
            .iter()
            .map(|(id, perf)| {
                let weight = self.weights.get(id).copied().unwrap_or(0.0) / total_weight.max(1.0);
                weight * perf.return_on_capital
            })
            .sum();

        self.performances
            .iter()
            .map(|(id, perf)| {
                let weight = self.weights.get(id).copied().unwrap_or(0.0) / total_weight.max(1.0);

                // Active return vs benchmark
                let active_return = perf.return_on_capital - self.benchmark_return;

                // Portfolio contribution
                let portfolio_contribution = weight * perf.return_on_capital;

                // Simple risk contribution based on volatility
                let returns = &perf.daily_returns;
                let tracking_error = if returns.len() >= 2 {
                    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
                    let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>()
                        / (returns.len() - 1) as f64;
                    variance.sqrt() * (252.0_f64).sqrt() // Annualized
                } else {
                    0.0
                };

                let information_ratio = if tracking_error > 0.0 {
                    active_return / tracking_error
                } else {
                    0.0
                };

                // Risk contribution (simplified)
                let risk_contribution = weight * weight * tracking_error * tracking_error;

                StrategyAttribution {
                    strategy_id: id.clone(),
                    total_return: perf.return_on_capital,
                    factor_contributions: HashMap::new(), // Would need factor model
                    active_return,
                    tracking_error,
                    information_ratio,
                    portfolio_contribution,
                    risk_contribution,
                }
            })
            .collect()
    }

    /// Get portfolio-level statistics
    pub fn portfolio_statistics(&self) -> PortfolioStatistics {
        let total_weight: f64 = self.weights.values().sum();

        let portfolio_return: f64 = self
            .performances
            .iter()
            .map(|(id, perf)| {
                let weight = self.weights.get(id).copied().unwrap_or(0.0) / total_weight.max(1.0);
                weight * perf.return_on_capital
            })
            .sum();

        let total_pnl: f64 = self.performances.values().map(|p| p.total_pnl).sum();

        let total_trades: u64 = self.performances.values().map(|p| p.total_trades).sum();

        // Portfolio Sharpe (weighted average approximation)
        let portfolio_sharpe: f64 = self
            .performances
            .iter()
            .map(|(id, perf)| {
                let weight = self.weights.get(id).copied().unwrap_or(0.0) / total_weight.max(1.0);
                weight * perf.sharpe_ratio
            })
            .sum();

        PortfolioStatistics {
            total_pnl,
            portfolio_return,
            portfolio_sharpe,
            total_trades,
            strategy_count: self.performances.len(),
            active_return: portfolio_return - self.benchmark_return,
        }
    }
}

impl Default for PerformanceAttributor {
    fn default() -> Self {
        Self::new()
    }
}

/// Portfolio-level statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioStatistics {
    pub total_pnl: f64,
    pub portfolio_return: f64,
    pub portfolio_sharpe: f64,
    pub total_trades: u64,
    pub strategy_count: usize,
    pub active_return: f64,
}

// =============================================================================
// Strategy Orchestrator
// =============================================================================

/// Strategy state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyState {
    Active,
    Paused,
    Disabled,
    Liquidating,
    Error,
}

/// Strategy orchestration config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationConfig {
    /// Maximum strategies
    pub max_strategies: usize,
    /// Auto-disable on drawdown threshold
    pub drawdown_disable_threshold: f64,
    /// Auto-pause on consecutive losses
    pub consecutive_loss_pause: u32,
    /// Auto-resume after cooldown (seconds)
    pub cooldown_seconds: u64,
    /// Enable auto-rebalancing
    pub auto_rebalance: bool,
    /// Rebalance frequency (seconds)
    pub rebalance_interval_seconds: u64,
}

impl Default for OrchestrationConfig {
    fn default() -> Self {
        Self {
            max_strategies: 20,
            drawdown_disable_threshold: 0.20, // 20%
            consecutive_loss_pause: 5,
            cooldown_seconds: 3600, // 1 hour
            auto_rebalance: true,
            rebalance_interval_seconds: 86400, // Daily
        }
    }
}

/// Strategy orchestrator
#[derive(Debug)]
pub struct StrategyOrchestrator {
    config: OrchestrationConfig,
    /// Strategy states
    states: HashMap<StrategyId, StrategyState>,
    /// Strategy pause times (for cooldown)
    pause_times: HashMap<StrategyId, UnixNanos>,
    /// Consecutive losses per strategy
    consecutive_losses: HashMap<StrategyId, u32>,
    /// Last rebalance time
    last_rebalance: UnixNanos,
}

impl StrategyOrchestrator {
    pub fn new(config: OrchestrationConfig) -> Self {
        Self {
            config,
            states: HashMap::new(),
            pause_times: HashMap::new(),
            consecutive_losses: HashMap::new(),
            last_rebalance: UnixNanos::ZERO,
        }
    }

    /// Register a strategy
    pub fn register_strategy(&mut self, strategy_id: StrategyId) -> bool {
        if self.states.len() >= self.config.max_strategies {
            return false;
        }

        self.states.insert(strategy_id, StrategyState::Active);
        true
    }

    /// Get strategy state
    pub fn get_state(&self, strategy_id: &StrategyId) -> Option<StrategyState> {
        self.states.get(strategy_id).copied()
    }

    /// Check if strategy can trade
    pub fn can_trade(&self, strategy_id: &StrategyId) -> bool {
        matches!(self.get_state(strategy_id), Some(StrategyState::Active))
    }

    /// Pause a strategy
    pub fn pause(&mut self, strategy_id: &StrategyId, timestamp: UnixNanos) {
        if let Some(state) = self.states.get_mut(strategy_id) {
            *state = StrategyState::Paused;
            self.pause_times.insert(strategy_id.clone(), timestamp);
        }
    }

    /// Resume a strategy
    pub fn resume(&mut self, strategy_id: &StrategyId) {
        if let Some(state) = self.states.get_mut(strategy_id) {
            *state = StrategyState::Active;
            self.pause_times.remove(strategy_id);
            self.consecutive_losses.remove(strategy_id);
        }
    }

    /// Disable a strategy
    pub fn disable(&mut self, strategy_id: &StrategyId) {
        if let Some(state) = self.states.get_mut(strategy_id) {
            *state = StrategyState::Disabled;
        }
    }

    /// Enable a strategy
    pub fn enable(&mut self, strategy_id: &StrategyId) {
        if let Some(state) = self.states.get_mut(strategy_id) {
            *state = StrategyState::Active;
        }
    }

    /// Record a trade result
    pub fn record_trade_result(
        &mut self,
        strategy_id: &StrategyId,
        pnl: f64,
        timestamp: UnixNanos,
    ) {
        if pnl < 0.0 {
            let losses = self
                .consecutive_losses
                .entry(strategy_id.clone())
                .or_insert(0);
            *losses += 1;

            if *losses >= self.config.consecutive_loss_pause {
                self.pause(strategy_id, timestamp);
            }
        } else {
            self.consecutive_losses.insert(strategy_id.clone(), 0);
        }
    }

    /// Check drawdown and potentially disable
    pub fn check_drawdown(&mut self, strategy_id: &StrategyId, drawdown_pct: f64) {
        if drawdown_pct >= self.config.drawdown_disable_threshold * 100.0 {
            if let Some(state) = self.states.get_mut(strategy_id) {
                *state = StrategyState::Liquidating;
            }
        }
    }

    /// Check for strategies to auto-resume
    pub fn check_cooldowns(&mut self, current_time: UnixNanos) -> Vec<StrategyId> {
        let cooldown_ns = self.config.cooldown_seconds * 1_000_000_000;
        let mut to_resume = Vec::new();

        for (strategy_id, pause_time) in &self.pause_times {
            if current_time.0.saturating_sub(pause_time.0) >= cooldown_ns {
                to_resume.push(strategy_id.clone());
            }
        }

        for strategy_id in &to_resume {
            self.resume(strategy_id);
        }

        to_resume
    }

    /// Check if rebalancing is due
    pub fn should_rebalance(&self, current_time: UnixNanos) -> bool {
        if !self.config.auto_rebalance {
            return false;
        }

        let interval_ns = self.config.rebalance_interval_seconds * 1_000_000_000;
        current_time.0.saturating_sub(self.last_rebalance.0) >= interval_ns
    }

    /// Mark rebalancing done
    pub fn mark_rebalanced(&mut self, timestamp: UnixNanos) {
        self.last_rebalance = timestamp;
    }

    /// Get all active strategies
    pub fn active_strategies(&self) -> Vec<StrategyId> {
        self.states
            .iter()
            .filter(|(_, state)| **state == StrategyState::Active)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get all strategy states
    pub fn all_states(&self) -> &HashMap<StrategyId, StrategyState> {
        &self.states
    }
}

// =============================================================================
// Portfolio Manager (combines all components)
// =============================================================================

/// Complete portfolio manager
#[derive(Debug)]
pub struct PortfolioManager {
    /// Capital allocator
    pub allocator: CapitalAllocator,
    /// Position netter
    pub netter: PositionNetter,
    /// Performance attributor
    pub attributor: PerformanceAttributor,
    /// Strategy orchestrator
    pub orchestrator: StrategyOrchestrator,
    /// Strategy performances
    performances: HashMap<StrategyId, StrategyPerformance>,
}

impl PortfolioManager {
    pub fn new(
        allocation_config: AllocationConfig,
        orchestration_config: OrchestrationConfig,
    ) -> Self {
        Self {
            allocator: CapitalAllocator::new(allocation_config),
            netter: PositionNetter::new(),
            attributor: PerformanceAttributor::new(),
            orchestrator: StrategyOrchestrator::new(orchestration_config),
            performances: HashMap::new(),
        }
    }

    /// Register a new strategy
    pub fn register_strategy(&mut self, strategy_id: StrategyId, initial_capital: f64) -> bool {
        if !self.orchestrator.register_strategy(strategy_id.clone()) {
            return false;
        }

        let perf = StrategyPerformance::new(strategy_id.clone(), initial_capital);
        self.performances.insert(strategy_id, perf);
        true
    }

    /// Update position from strategy
    pub fn update_position(
        &mut self,
        instrument_id: InstrumentId,
        strategy_id: StrategyId,
        quantity: f64,
        avg_price: f64,
        realized_pnl: f64,
        unrealized_pnl: f64,
        timestamp: UnixNanos,
    ) {
        self.netter.update_position(
            instrument_id,
            strategy_id.clone(),
            quantity,
            avg_price,
            realized_pnl,
            unrealized_pnl,
        );

        if let Some(perf) = self.performances.get_mut(&strategy_id) {
            perf.update_unrealized(unrealized_pnl, timestamp);
        }
    }

    /// Record a completed trade
    pub fn record_trade(&mut self, strategy_id: &StrategyId, pnl: f64, timestamp: UnixNanos) {
        if let Some(perf) = self.performances.get_mut(strategy_id) {
            perf.record_trade(pnl, timestamp);
            self.orchestrator
                .check_drawdown(strategy_id, perf.current_drawdown_pct);
        }

        self.orchestrator
            .record_trade_result(strategy_id, pnl, timestamp);
    }

    /// Get target allocations
    pub fn get_allocations(&self) -> HashMap<StrategyId, f64> {
        self.allocator.calculate_allocations(&self.performances)
    }

    /// Apply allocations and rebalance
    pub fn rebalance(&mut self, timestamp: UnixNanos) {
        let allocations = self.get_allocations();
        self.allocator.apply_allocations(allocations.clone());

        // Update strategy capital allocations
        for (strategy_id, allocation) in &allocations {
            if let Some(perf) = self.performances.get_mut(strategy_id) {
                perf.capital_allocated = *allocation;
            }
        }

        self.orchestrator.mark_rebalanced(timestamp);
    }

    /// Get netting results
    pub fn get_netting(&self, margin_requirement: f64) -> Vec<NettingResult> {
        self.netter.calculate_netting(margin_requirement)
    }

    /// Get performance attribution
    pub fn get_attribution(&self) -> Vec<StrategyAttribution> {
        let mut attributor = self.attributor.clone();

        for (_id, perf) in &self.performances {
            attributor.update_performance(perf.clone());
        }

        let weights: HashMap<StrategyId, f64> = self
            .performances
            .iter()
            .map(|(id, perf)| (id.clone(), perf.capital_allocated))
            .collect();
        attributor.update_weights(weights);

        attributor.calculate_attribution()
    }

    /// Get portfolio statistics
    pub fn portfolio_stats(&self) -> PortfolioStatistics {
        let mut attributor = self.attributor.clone();

        for (_, perf) in &self.performances {
            attributor.update_performance(perf.clone());
        }

        let weights: HashMap<StrategyId, f64> = self
            .performances
            .iter()
            .map(|(id, perf)| (id.clone(), perf.capital_allocated))
            .collect();
        attributor.update_weights(weights);

        attributor.portfolio_statistics()
    }

    /// Check if strategy can trade
    pub fn can_trade(&self, strategy_id: &StrategyId) -> bool {
        self.orchestrator.can_trade(strategy_id)
    }

    /// Get strategy performance
    pub fn get_performance(&self, strategy_id: &StrategyId) -> Option<&StrategyPerformance> {
        self.performances.get(strategy_id)
    }

    /// Process periodic updates
    pub fn tick(&mut self, current_time: UnixNanos) {
        // Check cooldowns
        self.orchestrator.check_cooldowns(current_time);

        // Check if rebalance needed
        if self.orchestrator.should_rebalance(current_time) {
            if self.allocator.needs_rebalance(&self.performances) {
                self.rebalance(current_time);
            } else {
                self.orchestrator.mark_rebalanced(current_time);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capital_allocator_equal() {
        let config = AllocationConfig {
            method: AllocationMethod::Equal,
            total_capital: 100000.0,
            ..Default::default()
        };

        let allocator = CapitalAllocator::new(config);

        let mut strategies = HashMap::new();
        strategies.insert(
            StrategyId::new("strategy1"),
            StrategyPerformance::new(StrategyId::new("strategy1"), 50000.0),
        );
        strategies.insert(
            StrategyId::new("strategy2"),
            StrategyPerformance::new(StrategyId::new("strategy2"), 50000.0),
        );

        let allocations = allocator.calculate_allocations(&strategies);

        assert_eq!(allocations.len(), 2);
        for (_, alloc) in &allocations {
            assert!((alloc - 50000.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_position_netter() {
        let mut netter = PositionNetter::new();

        let instrument = InstrumentId::new(
            neleus_core_types::Venue::Simulated,
            "BTC",
            neleus_core_types::InstrumentType::Perp,
        );

        // Strategy 1 is long 10
        netter.update_position(
            instrument.clone(),
            StrategyId::new("s1"),
            10.0,
            50000.0,
            0.0,
            0.0,
        );

        // Strategy 2 is short 6
        netter.update_position(
            instrument.clone(),
            StrategyId::new("s2"),
            -6.0,
            50000.0,
            0.0,
            0.0,
        );

        let positions = netter.net_positions();
        let btc_pos = positions.get(&instrument).unwrap();

        assert!((btc_pos.net_quantity - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_strategy_orchestrator() {
        let config = OrchestrationConfig {
            consecutive_loss_pause: 3,
            ..Default::default()
        };

        let mut orchestrator = StrategyOrchestrator::new(config);
        let strategy_id = StrategyId::new("test");

        orchestrator.register_strategy(strategy_id.clone());
        assert!(orchestrator.can_trade(&strategy_id));

        // Record consecutive losses
        for _ in 0..3 {
            orchestrator.record_trade_result(&strategy_id, -100.0, UnixNanos::ZERO);
        }

        // Should be paused now
        assert!(!orchestrator.can_trade(&strategy_id));
    }

    #[test]
    fn test_portfolio_manager() {
        let mut manager =
            PortfolioManager::new(AllocationConfig::default(), OrchestrationConfig::default());

        assert!(manager.register_strategy(StrategyId::new("s1"), 50000.0));
        assert!(manager.register_strategy(StrategyId::new("s2"), 50000.0));

        let stats = manager.portfolio_stats();
        assert_eq!(stats.strategy_count, 2);
    }
}
