use neleus_core_types::{InstrumentId, UnixNanos};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Value at Risk (VAR)
// =============================================================================

/// VAR calculation method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VarMethod {
    /// Historical simulation
    Historical,
    /// Parametric (variance-covariance)
    Parametric,
    /// Monte Carlo simulation
    MonteCarlo,
}

/// VAR configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarConfig {
    /// Calculation method
    pub method: VarMethod,
    /// Confidence level (e.g., 0.95 for 95% VAR)
    pub confidence_level: f64,
    /// Holding period in days
    pub holding_period_days: u32,
    /// Lookback period for historical data
    pub lookback_days: u32,
    /// Number of Monte Carlo simulations
    pub monte_carlo_sims: u32,
    /// Decay factor for EWMA (0-1, higher = more weight on recent)
    pub ewma_decay: f64,
}

impl Default for VarConfig {
    fn default() -> Self {
        Self {
            method: VarMethod::Historical,
            confidence_level: 0.95,
            holding_period_days: 1,
            lookback_days: 252,
            monte_carlo_sims: 10000,
            ewma_decay: 0.94,
        }
    }
}

/// VAR calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarResult {
    /// VAR value (as positive dollar amount)
    pub var_value: f64,
    /// VAR as percentage of portfolio
    pub var_pct: f64,
    /// Confidence level used
    pub confidence_level: f64,
    /// Holding period in days
    pub holding_period_days: u32,
    /// Calculation method used
    pub method: VarMethod,
    /// Component VAR by position
    pub component_var: HashMap<InstrumentId, f64>,
    /// Marginal VAR by position
    pub marginal_var: HashMap<InstrumentId, f64>,
    /// Calculation timestamp
    pub calculated_at: UnixNanos,
}

/// VAR calculator
#[derive(Debug, Clone)]
pub struct VarCalculator {
    config: VarConfig,
    /// Historical returns by instrument
    returns_history: HashMap<InstrumentId, Vec<f64>>,
    /// Portfolio returns
    portfolio_returns: Vec<f64>,
    /// Current positions
    positions: HashMap<InstrumentId, f64>,
    /// Current portfolio value
    portfolio_value: f64,
}

impl VarCalculator {
    pub fn new(config: VarConfig) -> Self {
        Self {
            config,
            returns_history: HashMap::new(),
            portfolio_returns: Vec::new(),
            positions: HashMap::new(),
            portfolio_value: 0.0,
        }
    }

    /// Add historical return for an instrument
    pub fn add_return(&mut self, instrument_id: InstrumentId, daily_return: f64) {
        let returns = self
            .returns_history
            .entry(instrument_id)
            .or_insert_with(Vec::new);
        returns.push(daily_return);

        // Keep only lookback period
        while returns.len() > self.config.lookback_days as usize {
            returns.remove(0);
        }
    }

    /// Add portfolio return
    pub fn add_portfolio_return(&mut self, daily_return: f64) {
        self.portfolio_returns.push(daily_return);

        while self.portfolio_returns.len() > self.config.lookback_days as usize {
            self.portfolio_returns.remove(0);
        }
    }

    /// Update current positions
    pub fn update_positions(
        &mut self,
        positions: HashMap<InstrumentId, f64>,
        portfolio_value: f64,
    ) {
        self.positions = positions;
        self.portfolio_value = portfolio_value;
    }

    /// Calculate VAR
    pub fn calculate(&self, timestamp: UnixNanos) -> VarResult {
        let var_value = match self.config.method {
            VarMethod::Historical => self.historical_var(),
            VarMethod::Parametric => self.parametric_var(),
            VarMethod::MonteCarlo => self.monte_carlo_var(),
        };

        let var_pct = if self.portfolio_value > 0.0 {
            var_value / self.portfolio_value * 100.0
        } else {
            0.0
        };

        let (component_var, marginal_var) = self.calculate_component_var();

        VarResult {
            var_value,
            var_pct,
            confidence_level: self.config.confidence_level,
            holding_period_days: self.config.holding_period_days,
            method: self.config.method,
            component_var,
            marginal_var,
            calculated_at: timestamp,
        }
    }

    fn historical_var(&self) -> f64 {
        if self.portfolio_returns.is_empty() {
            return 0.0;
        }

        let mut sorted_returns = self.portfolio_returns.clone();
        sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let percentile_index =
            ((1.0 - self.config.confidence_level) * sorted_returns.len() as f64).floor() as usize;
        let percentile_index = percentile_index.max(0).min(sorted_returns.len() - 1);

        let var_return = sorted_returns[percentile_index];

        // Scale for holding period (square root of time)
        let holding_scalar = (self.config.holding_period_days as f64).sqrt();

        // Convert to positive dollar amount
        (var_return.abs() * self.portfolio_value * holding_scalar).max(0.0)
    }

    fn parametric_var(&self) -> f64 {
        if self.portfolio_returns.len() < 2 {
            return 0.0;
        }

        // Calculate mean and std dev
        let mean = self.portfolio_returns.iter().sum::<f64>() / self.portfolio_returns.len() as f64;
        let variance = self
            .portfolio_returns
            .iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>()
            / (self.portfolio_returns.len() - 1) as f64;
        let std_dev = variance.sqrt();

        // Z-score for confidence level
        let z_score = self.normal_inv_cdf(self.config.confidence_level);

        // Scale for holding period
        let holding_scalar = (self.config.holding_period_days as f64).sqrt();

        // VAR = portfolio_value * z * sigma * sqrt(t)
        (z_score * std_dev * self.portfolio_value * holding_scalar).abs()
    }

    fn monte_carlo_var(&self) -> f64 {
        if self.portfolio_returns.len() < 2 {
            return 0.0;
        }

        // Estimate distribution parameters
        let mean = self.portfolio_returns.iter().sum::<f64>() / self.portfolio_returns.len() as f64;
        let variance = self
            .portfolio_returns
            .iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>()
            / (self.portfolio_returns.len() - 1) as f64;
        let std_dev = variance.sqrt();

        // Generate simulated returns
        let mut simulated_pnls: Vec<f64> =
            Vec::with_capacity(self.config.monte_carlo_sims as usize);

        // Simple pseudo-random generator for deterministic results
        let mut seed = 12345u64;
        for _ in 0..self.config.monte_carlo_sims {
            // Box-Muller for normal distribution
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let u1 = (seed % 1000000) as f64 / 1000000.0;
            seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
            let u2 = (seed % 1000000) as f64 / 1000000.0;

            let u1_clamped = u1.max(1e-10).min(1.0 - 1e-10);
            let z = (-2.0 * u1_clamped.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();

            let simulated_return = mean + z * std_dev;
            let holding_period_return =
                simulated_return * (self.config.holding_period_days as f64).sqrt();
            simulated_pnls.push(holding_period_return * self.portfolio_value);
        }

        // Sort and find percentile
        simulated_pnls.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let percentile_index =
            ((1.0 - self.config.confidence_level) * simulated_pnls.len() as f64).floor() as usize;
        let percentile_index = percentile_index.max(0).min(simulated_pnls.len() - 1);

        simulated_pnls[percentile_index].abs()
    }

    fn calculate_component_var(&self) -> (HashMap<InstrumentId, f64>, HashMap<InstrumentId, f64>) {
        let mut component_var = HashMap::new();
        let mut marginal_var = HashMap::new();

        // Simplified: allocate VAR proportionally to position size
        let total_notional: f64 = self.positions.values().map(|v| v.abs()).sum();

        let base_var = match self.config.method {
            VarMethod::Historical => self.historical_var(),
            VarMethod::Parametric => self.parametric_var(),
            VarMethod::MonteCarlo => self.monte_carlo_var(),
        };

        for (instrument_id, position) in &self.positions {
            let weight = if total_notional > 0.0 {
                position.abs() / total_notional
            } else {
                0.0
            };

            component_var.insert(instrument_id.clone(), base_var * weight);

            // Marginal VAR is simplified as VAR per unit of notional
            if position.abs() > 0.0 {
                marginal_var.insert(instrument_id.clone(), base_var * weight / position.abs());
            }
        }

        (component_var, marginal_var)
    }

    /// Inverse CDF of standard normal (Acklam's approximation)
    fn normal_inv_cdf(&self, p: f64) -> f64 {
        let a1 = -3.969683028665376e1;
        let a2 = 2.209460984245205e2;
        let a3 = -2.759285104469687e2;
        let a4 = 1.383577518672690e2;
        let a5 = -3.066479806614716e1;
        let a6 = 2.506628277459239e0;

        let b1 = -5.447609879822406e1;
        let b2 = 1.615858368580409e2;
        let b3 = -1.556989798598866e2;
        let b4 = 6.680131188771972e1;
        let b5 = -1.328068155288572e1;

        let c1 = -7.784894002430293e-3;
        let c2 = -3.223964580411365e-1;
        let c3 = -2.400758277161838e0;
        let c4 = -2.549732539343734e0;
        let c5 = 4.374664141464968e0;
        let c6 = 2.938163982698783e0;

        let d1 = 7.784695709041462e-3;
        let d2 = 3.224671290700398e-1;
        let d3 = 2.445134137142996e0;
        let d4 = 3.754408661907416e0;

        let p_low = 0.02425;
        let p_high = 1.0 - p_low;

        if p < p_low {
            let q = (-2.0 * p.ln()).sqrt();
            (((((c1 * q + c2) * q + c3) * q + c4) * q + c5) * q + c6)
                / ((((d1 * q + d2) * q + d3) * q + d4) * q + 1.0)
        } else if p <= p_high {
            let q = p - 0.5;
            let r = q * q;
            (((((a1 * r + a2) * r + a3) * r + a4) * r + a5) * r + a6) * q
                / (((((b1 * r + b2) * r + b3) * r + b4) * r + b5) * r + 1.0)
        } else {
            let q = (-2.0 * (1.0 - p).ln()).sqrt();
            -(((((c1 * q + c2) * q + c3) * q + c4) * q + c5) * q + c6)
                / ((((d1 * q + d2) * q + d3) * q + d4) * q + 1.0)
        }
    }
}

// =============================================================================
// Expected Shortfall (CVaR)
// =============================================================================

/// CVaR (Expected Shortfall) result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CvarResult {
    /// CVaR value (average loss beyond VAR)
    pub cvar_value: f64,
    /// CVaR as percentage of portfolio
    pub cvar_pct: f64,
    /// Corresponding VAR value
    pub var_value: f64,
    /// Confidence level used
    pub confidence_level: f64,
    /// Holding period in days
    pub holding_period_days: u32,
    /// Calculation timestamp
    pub calculated_at: UnixNanos,
}

impl VarCalculator {
    /// Calculate Expected Shortfall (CVaR)
    pub fn calculate_cvar(&self, timestamp: UnixNanos) -> CvarResult {
        let var_result = self.calculate(timestamp);

        let cvar_value = self.calculate_cvar_value(&var_result);

        let cvar_pct = if self.portfolio_value > 0.0 {
            cvar_value / self.portfolio_value * 100.0
        } else {
            0.0
        };

        CvarResult {
            cvar_value,
            cvar_pct,
            var_value: var_result.var_value,
            confidence_level: self.config.confidence_level,
            holding_period_days: self.config.holding_period_days,
            calculated_at: timestamp,
        }
    }

    fn calculate_cvar_value(&self, var_result: &VarResult) -> f64 {
        if self.portfolio_returns.is_empty() {
            return 0.0;
        }

        let mut sorted_returns = self.portfolio_returns.clone();
        sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let cutoff_index =
            ((1.0 - self.config.confidence_level) * sorted_returns.len() as f64).floor() as usize;
        let cutoff_index = cutoff_index.max(1);

        // Average of returns worse than VAR
        let tail_returns = &sorted_returns[..cutoff_index];

        if tail_returns.is_empty() {
            return var_result.var_value;
        }

        let avg_tail_return = tail_returns.iter().sum::<f64>() / tail_returns.len() as f64;

        let holding_scalar = (self.config.holding_period_days as f64).sqrt();

        (avg_tail_return.abs() * self.portfolio_value * holding_scalar).max(var_result.var_value)
    }
}

// =============================================================================
// Scenario Analysis
// =============================================================================

/// Predefined stress scenarios
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StressScenario {
    /// Flash crash (-10% in 5 minutes)
    FlashCrash,
    /// Market correction (-20% over weeks)
    MarketCorrection,
    /// Liquidity crisis
    LiquidityCrisis,
    /// Volatility spike
    VolatilitySpike,
    /// Rate shock
    RateShock,
    /// Black swan event
    BlackSwan,
    /// Custom scenario
    Custom,
}

/// Stress test parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestParams {
    /// Scenario type
    pub scenario: StressScenario,
    /// Price shock (as fraction, e.g., -0.1 for -10%)
    pub price_shock: f64,
    /// Volatility multiplier
    pub volatility_multiplier: f64,
    /// Spread widening (basis points)
    pub spread_widening_bps: f64,
    /// Liquidity reduction (as fraction)
    pub liquidity_reduction: f64,
    /// Correlation shock (change in correlation)
    pub correlation_shock: f64,
    /// Description
    pub description: String,
}

impl StressTestParams {
    pub fn flash_crash() -> Self {
        Self {
            scenario: StressScenario::FlashCrash,
            price_shock: -0.10,
            volatility_multiplier: 5.0,
            spread_widening_bps: 500.0,
            liquidity_reduction: 0.80,
            correlation_shock: 0.3,
            description: "Flash crash: 10% price drop, 5x volatility, 80% liquidity reduction"
                .to_string(),
        }
    }

    pub fn market_correction() -> Self {
        Self {
            scenario: StressScenario::MarketCorrection,
            price_shock: -0.20,
            volatility_multiplier: 2.0,
            spread_widening_bps: 100.0,
            liquidity_reduction: 0.30,
            correlation_shock: 0.2,
            description: "Market correction: 20% price drop, 2x volatility".to_string(),
        }
    }

    pub fn liquidity_crisis() -> Self {
        Self {
            scenario: StressScenario::LiquidityCrisis,
            price_shock: -0.05,
            volatility_multiplier: 3.0,
            spread_widening_bps: 1000.0,
            liquidity_reduction: 0.90,
            correlation_shock: 0.4,
            description: "Liquidity crisis: 90% liquidity reduction, massive spread widening"
                .to_string(),
        }
    }

    pub fn volatility_spike() -> Self {
        Self {
            scenario: StressScenario::VolatilitySpike,
            price_shock: -0.05,
            volatility_multiplier: 4.0,
            spread_widening_bps: 200.0,
            liquidity_reduction: 0.40,
            correlation_shock: 0.3,
            description: "Volatility spike: 4x volatility increase".to_string(),
        }
    }

    pub fn black_swan() -> Self {
        Self {
            scenario: StressScenario::BlackSwan,
            price_shock: -0.30,
            volatility_multiplier: 10.0,
            spread_widening_bps: 2000.0,
            liquidity_reduction: 0.95,
            correlation_shock: 0.5,
            description: "Black swan: 30% price drop, extreme conditions".to_string(),
        }
    }
}

/// Stress test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestResult {
    /// Scenario tested
    pub scenario: StressScenario,
    /// Description
    pub description: String,
    /// Portfolio PnL under scenario
    pub portfolio_pnl: f64,
    /// Portfolio PnL as percentage
    pub portfolio_pnl_pct: f64,
    /// Position-level impacts
    pub position_impacts: HashMap<InstrumentId, PositionImpact>,
    /// Estimated slippage from liquidity reduction
    pub estimated_slippage: f64,
    /// Margin call triggered?
    pub margin_call: bool,
    /// Liquidation triggered?
    pub liquidation: bool,
}

/// Impact on individual position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionImpact {
    /// Position size
    pub position: f64,
    /// PnL from price shock
    pub price_impact: f64,
    /// PnL from slippage
    pub slippage_impact: f64,
    /// Total PnL
    pub total_impact: f64,
}

/// Scenario analyzer
#[derive(Debug, Clone)]
pub struct ScenarioAnalyzer {
    /// Current positions (instrument -> (quantity, avg_price, current_price))
    positions: HashMap<InstrumentId, (f64, f64, f64)>,
    /// Current equity
    equity: f64,
    /// Margin requirement
    margin_requirement: f64,
    /// Maintenance margin
    maintenance_margin: f64,
}

impl ScenarioAnalyzer {
    pub fn new(equity: f64, margin_requirement: f64, maintenance_margin: f64) -> Self {
        Self {
            positions: HashMap::new(),
            equity,
            margin_requirement,
            maintenance_margin,
        }
    }

    /// Update positions
    pub fn update_position(
        &mut self,
        instrument_id: InstrumentId,
        quantity: f64,
        avg_price: f64,
        current_price: f64,
    ) {
        self.positions
            .insert(instrument_id, (quantity, avg_price, current_price));
    }

    /// Update equity
    pub fn update_equity(&mut self, equity: f64) {
        self.equity = equity;
    }

    /// Run stress test
    pub fn run_stress_test(&self, params: &StressTestParams) -> StressTestResult {
        let mut position_impacts = HashMap::new();
        let mut total_pnl = 0.0;
        let mut total_slippage = 0.0;

        for (instrument_id, (quantity, _avg_price, current_price)) in &self.positions {
            // Price impact
            let shocked_price = current_price * (1.0 + params.price_shock);
            let price_pnl = (shocked_price - current_price) * quantity;

            // Slippage from liquidity reduction
            let notional = quantity.abs() * current_price;
            let base_slippage_bps = 5.0; // Base slippage
            let stressed_slippage_bps = base_slippage_bps + params.spread_widening_bps / 2.0;
            let slippage_pnl = -notional * stressed_slippage_bps / 10000.0;

            let total_impact = price_pnl + slippage_pnl;

            position_impacts.insert(
                instrument_id.clone(),
                PositionImpact {
                    position: *quantity,
                    price_impact: price_pnl,
                    slippage_impact: slippage_pnl,
                    total_impact,
                },
            );

            total_pnl += total_impact;
            total_slippage += slippage_pnl.abs();
        }

        let portfolio_pnl_pct = if self.equity > 0.0 {
            total_pnl / self.equity * 100.0
        } else {
            0.0
        };

        // Check for margin call / liquidation
        let new_equity = self.equity + total_pnl;
        let position_value: f64 = self
            .positions
            .values()
            .map(|(qty, _, price)| qty.abs() * price)
            .sum();

        let margin_call = new_equity < position_value * self.margin_requirement;
        let liquidation = new_equity < position_value * self.maintenance_margin;

        StressTestResult {
            scenario: params.scenario,
            description: params.description.clone(),
            portfolio_pnl: total_pnl,
            portfolio_pnl_pct,
            position_impacts,
            estimated_slippage: total_slippage,
            margin_call,
            liquidation,
        }
    }

    /// Run all predefined scenarios
    pub fn run_all_scenarios(&self) -> Vec<StressTestResult> {
        vec![
            self.run_stress_test(&StressTestParams::flash_crash()),
            self.run_stress_test(&StressTestParams::market_correction()),
            self.run_stress_test(&StressTestParams::liquidity_crisis()),
            self.run_stress_test(&StressTestParams::volatility_spike()),
            self.run_stress_test(&StressTestParams::black_swan()),
        ]
    }
}

// =============================================================================
// Greeks Calculation
// =============================================================================

/// Option/position Greeks
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Greeks {
    /// Delta: change in value per unit change in underlying
    pub delta: f64,
    /// Gamma: rate of change of delta
    pub gamma: f64,
    /// Vega: sensitivity to volatility
    pub vega: f64,
    /// Theta: time decay
    pub theta: f64,
    /// Rho: sensitivity to interest rates
    pub rho: f64,
}

/// Greeks calculator for perpetual futures
#[derive(Debug, Clone)]
pub struct GreeksCalculator {
    /// Risk-free rate
    risk_free_rate: f64,
}

impl GreeksCalculator {
    pub fn new(risk_free_rate: f64) -> Self {
        Self { risk_free_rate }
    }

    /// Get the risk-free rate used for calculations
    pub fn risk_free_rate(&self) -> f64 {
        self.risk_free_rate
    }

    /// Calculate Greeks for a perpetual future position
    pub fn calculate_perp_greeks(
        &self,
        quantity: f64,
        price: f64,
        _volatility: f64,
        funding_rate: f64,
    ) -> Greeks {
        // For perpetual futures:
        // Delta = quantity (1:1 with underlying)
        // Gamma = 0 (linear payoff)
        // Vega = position * volatility sensitivity
        // Theta = funding cost
        // Rho = minimal for perps

        let notional = quantity * price;

        Greeks {
            delta: quantity,
            gamma: 0.0,
            // Vega: 1% vol change impact (simplified)
            vega: notional * 0.01,
            // Theta: daily funding cost
            theta: -notional * funding_rate / 3.0, // 3 funding periods per day
            rho: notional * 0.0001,                // Minimal rate sensitivity
        }
    }

    /// Aggregate Greeks across positions
    pub fn aggregate_greeks(&self, position_greeks: &[Greeks]) -> Greeks {
        Greeks {
            delta: position_greeks.iter().map(|g| g.delta).sum(),
            gamma: position_greeks.iter().map(|g| g.gamma).sum(),
            vega: position_greeks.iter().map(|g| g.vega).sum(),
            theta: position_greeks.iter().map(|g| g.theta).sum(),
            rho: position_greeks.iter().map(|g| g.rho).sum(),
        }
    }
}

// =============================================================================
// Correlation-Based Sizing
// =============================================================================

/// Correlation matrix for portfolio instruments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationMatrix {
    /// Instrument IDs (order matters)
    pub instruments: Vec<InstrumentId>,
    /// Correlation values (flattened upper triangular)
    pub correlations: Vec<f64>,
    /// Last update time
    pub updated_at: UnixNanos,
}

impl CorrelationMatrix {
    pub fn new(instruments: Vec<InstrumentId>) -> Self {
        let n = instruments.len();
        let num_correlations = n * (n - 1) / 2;

        Self {
            instruments,
            correlations: vec![0.0; num_correlations],
            updated_at: UnixNanos::ZERO,
        }
    }

    /// Get correlation between two instruments
    pub fn get_correlation(&self, i: &InstrumentId, j: &InstrumentId) -> Option<f64> {
        let idx_i = self.instruments.iter().position(|x| x == i)?;
        let idx_j = self.instruments.iter().position(|x| x == j)?;

        if idx_i == idx_j {
            return Some(1.0); // Self-correlation
        }

        let (min_idx, max_idx) = if idx_i < idx_j {
            (idx_i, idx_j)
        } else {
            (idx_j, idx_i)
        };
        let flat_idx =
            min_idx * (2 * self.instruments.len() - min_idx - 1) / 2 + (max_idx - min_idx - 1);

        self.correlations.get(flat_idx).copied()
    }

    /// Set correlation between two instruments
    pub fn set_correlation(&mut self, i: &InstrumentId, j: &InstrumentId, corr: f64) -> bool {
        let idx_i = match self.instruments.iter().position(|x| x == i) {
            Some(idx) => idx,
            None => return false,
        };
        let idx_j = match self.instruments.iter().position(|x| x == j) {
            Some(idx) => idx,
            None => return false,
        };

        if idx_i == idx_j {
            return false;
        }

        let (min_idx, max_idx) = if idx_i < idx_j {
            (idx_i, idx_j)
        } else {
            (idx_j, idx_i)
        };
        let flat_idx =
            min_idx * (2 * self.instruments.len() - min_idx - 1) / 2 + (max_idx - min_idx - 1);

        if flat_idx < self.correlations.len() {
            self.correlations[flat_idx] = corr.max(-1.0).min(1.0);
            return true;
        }

        false
    }
}

/// Correlation-based position sizer
#[derive(Debug, Clone)]
pub struct CorrelationSizer {
    /// Correlation matrix
    pub correlations: CorrelationMatrix,
    /// Maximum portfolio correlation exposure
    pub max_correlation_exposure: f64,
    /// Correlation penalty factor (reduce size for correlated positions)
    pub correlation_penalty: f64,
    /// Historical returns for correlation calculation
    returns_history: HashMap<InstrumentId, Vec<f64>>,
    /// Lookback period
    lookback_periods: usize,
}

impl CorrelationSizer {
    pub fn new(
        max_correlation_exposure: f64,
        correlation_penalty: f64,
        lookback_periods: usize,
    ) -> Self {
        Self {
            correlations: CorrelationMatrix::new(Vec::new()),
            max_correlation_exposure,
            correlation_penalty,
            returns_history: HashMap::new(),
            lookback_periods,
        }
    }

    /// Add return observation
    pub fn add_return(&mut self, instrument_id: InstrumentId, daily_return: f64) {
        let returns = self
            .returns_history
            .entry(instrument_id.clone())
            .or_insert_with(Vec::new);
        returns.push(daily_return);

        while returns.len() > self.lookback_periods {
            returns.remove(0);
        }
    }

    /// Update correlation matrix from historical returns
    pub fn update_correlations(&mut self, timestamp: UnixNanos) {
        let instruments: Vec<InstrumentId> = self.returns_history.keys().cloned().collect();
        self.correlations = CorrelationMatrix::new(instruments.clone());
        self.correlations.updated_at = timestamp;

        for i in 0..instruments.len() {
            for j in (i + 1)..instruments.len() {
                if let (Some(returns_i), Some(returns_j)) = (
                    self.returns_history.get(&instruments[i]),
                    self.returns_history.get(&instruments[j]),
                ) {
                    if returns_i.len() == returns_j.len() && returns_i.len() >= 2 {
                        let corr = self.calculate_correlation(returns_i, returns_j);
                        self.correlations
                            .set_correlation(&instruments[i], &instruments[j], corr);
                    }
                }
            }
        }
    }

    fn calculate_correlation(&self, x: &[f64], y: &[f64]) -> f64 {
        let n = x.len() as f64;

        let mean_x = x.iter().sum::<f64>() / n;
        let mean_y = y.iter().sum::<f64>() / n;

        let mut cov = 0.0;
        let mut var_x = 0.0;
        let mut var_y = 0.0;

        for i in 0..x.len() {
            let dx = x[i] - mean_x;
            let dy = y[i] - mean_y;
            cov += dx * dy;
            var_x += dx * dx;
            var_y += dy * dy;
        }

        let denom = (var_x * var_y).sqrt();
        if denom > 0.0 {
            cov / denom
        } else {
            0.0
        }
    }

    /// Calculate adjusted position size based on correlation to existing positions
    pub fn adjust_size(
        &self,
        instrument_id: &InstrumentId,
        base_size: f64,
        existing_positions: &HashMap<InstrumentId, f64>,
    ) -> f64 {
        if existing_positions.is_empty() {
            return base_size;
        }

        let mut max_correlation: f64 = 0.0;
        let mut weighted_correlation: f64 = 0.0;
        let mut total_position_value: f64 = 0.0;

        for (other_id, other_size) in existing_positions {
            if other_id == instrument_id {
                continue;
            }

            if let Some(corr) = self.correlations.get_correlation(instrument_id, other_id) {
                max_correlation = max_correlation.max(corr.abs());
                weighted_correlation += corr.abs() * other_size.abs();
                total_position_value += other_size.abs();
            }
        }

        // Average weighted correlation
        let avg_correlation = if total_position_value > 0.0 {
            weighted_correlation / total_position_value
        } else {
            0.0
        };

        // Apply penalty
        let penalty = 1.0 - (avg_correlation * self.correlation_penalty);
        let adjusted_size = base_size * penalty.max(0.1); // Minimum 10% of base size

        // Check against max correlation exposure
        if max_correlation > self.max_correlation_exposure {
            adjusted_size * 0.5 // Reduce by half if exceeding max correlation
        } else {
            adjusted_size
        }
    }
}

// =============================================================================
// Dynamic Risk Limits
// =============================================================================

/// Volatility regime
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VolatilityRegime {
    Low,
    Normal,
    High,
    Extreme,
}

/// Dynamic risk limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicLimitsConfig {
    /// Base position size limit
    pub base_position_limit: f64,
    /// Base daily loss limit
    pub base_daily_loss_limit: f64,
    /// Base leverage limit
    pub base_leverage_limit: f64,
    /// Volatility thresholds (low, normal, high)
    pub volatility_thresholds: (f64, f64, f64),
    /// Multipliers by regime (low, normal, high, extreme)
    pub regime_multipliers: (f64, f64, f64, f64),
    /// Drawdown reduction thresholds
    pub drawdown_thresholds: Vec<(f64, f64)>, // (drawdown_pct, limit_multiplier)
    /// Lookback for volatility calculation
    pub volatility_lookback_days: u32,
}

impl Default for DynamicLimitsConfig {
    fn default() -> Self {
        Self {
            base_position_limit: 100000.0,
            base_daily_loss_limit: 5000.0,
            base_leverage_limit: 5.0,
            volatility_thresholds: (0.01, 0.02, 0.04), // 1%, 2%, 4% daily vol
            regime_multipliers: (1.5, 1.0, 0.6, 0.3),
            drawdown_thresholds: vec![
                (0.05, 0.8),  // 5% drawdown -> 80% limits
                (0.10, 0.5),  // 10% drawdown -> 50% limits
                (0.15, 0.25), // 15% drawdown -> 25% limits
                (0.20, 0.0),  // 20% drawdown -> stop trading
            ],
            volatility_lookback_days: 20,
        }
    }
}

/// Current risk limits after dynamic adjustment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentLimits {
    /// Position size limit
    pub position_limit: f64,
    /// Daily loss limit
    pub daily_loss_limit: f64,
    /// Leverage limit
    pub leverage_limit: f64,
    /// Current volatility regime
    pub volatility_regime: VolatilityRegime,
    /// Current volatility
    pub current_volatility: f64,
    /// Drawdown adjustment factor
    pub drawdown_factor: f64,
    /// Trading allowed
    pub trading_allowed: bool,
}

/// Dynamic risk limit manager
#[derive(Debug, Clone)]
pub struct DynamicLimitManager {
    config: DynamicLimitsConfig,
    /// Recent daily returns for volatility
    daily_returns: Vec<f64>,
    /// Current drawdown percentage
    current_drawdown_pct: f64,
    /// Peak equity
    peak_equity: f64,
    /// Current equity
    current_equity: f64,
}

impl DynamicLimitManager {
    pub fn new(config: DynamicLimitsConfig, initial_equity: f64) -> Self {
        Self {
            config,
            daily_returns: Vec::new(),
            current_drawdown_pct: 0.0,
            peak_equity: initial_equity,
            current_equity: initial_equity,
        }
    }

    /// Add daily return
    pub fn add_daily_return(&mut self, daily_return: f64) {
        self.daily_returns.push(daily_return);

        while self.daily_returns.len() > self.config.volatility_lookback_days as usize {
            self.daily_returns.remove(0);
        }
    }

    /// Update equity
    pub fn update_equity(&mut self, equity: f64) {
        self.current_equity = equity;

        if equity > self.peak_equity {
            self.peak_equity = equity;
        }

        if self.peak_equity > 0.0 {
            self.current_drawdown_pct = (self.peak_equity - equity) / self.peak_equity;
        }
    }

    /// Calculate current volatility
    pub fn calculate_volatility(&self) -> f64 {
        if self.daily_returns.len() < 2 {
            return 0.02; // Default 2%
        }

        let mean = self.daily_returns.iter().sum::<f64>() / self.daily_returns.len() as f64;
        let variance = self
            .daily_returns
            .iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>()
            / (self.daily_returns.len() - 1) as f64;

        variance.sqrt()
    }

    /// Determine volatility regime
    pub fn determine_regime(&self) -> VolatilityRegime {
        let vol = self.calculate_volatility();
        let (low, normal, high) = self.config.volatility_thresholds;

        if vol < low {
            VolatilityRegime::Low
        } else if vol < normal {
            VolatilityRegime::Normal
        } else if vol < high {
            VolatilityRegime::High
        } else {
            VolatilityRegime::Extreme
        }
    }

    /// Get regime multiplier
    fn regime_multiplier(&self, regime: VolatilityRegime) -> f64 {
        let (low, normal, high, extreme) = self.config.regime_multipliers;

        match regime {
            VolatilityRegime::Low => low,
            VolatilityRegime::Normal => normal,
            VolatilityRegime::High => high,
            VolatilityRegime::Extreme => extreme,
        }
    }

    /// Get drawdown adjustment factor
    fn drawdown_factor(&self) -> f64 {
        for (threshold, multiplier) in &self.config.drawdown_thresholds {
            if self.current_drawdown_pct >= *threshold {
                return *multiplier;
            }
        }
        1.0
    }

    /// Calculate current limits
    pub fn calculate_limits(&self) -> CurrentLimits {
        let volatility = self.calculate_volatility();
        let regime = self.determine_regime();
        let regime_mult = self.regime_multiplier(regime);
        let drawdown_mult = self.drawdown_factor();

        let combined_mult = regime_mult * drawdown_mult;

        CurrentLimits {
            position_limit: self.config.base_position_limit * combined_mult,
            daily_loss_limit: self.config.base_daily_loss_limit * combined_mult,
            leverage_limit: self.config.base_leverage_limit * combined_mult,
            volatility_regime: regime,
            current_volatility: volatility,
            drawdown_factor: drawdown_mult,
            trading_allowed: drawdown_mult > 0.0,
        }
    }

    /// Check if an order respects current limits
    pub fn check_order(
        &self,
        notional: f64,
        current_daily_loss: f64,
        current_leverage: f64,
    ) -> RiskLimitCheck {
        let limits = self.calculate_limits();

        if !limits.trading_allowed {
            return RiskLimitCheck::Rejected("Trading halted due to drawdown".to_string());
        }

        if notional > limits.position_limit {
            return RiskLimitCheck::Rejected(format!(
                "Position {} exceeds limit {}",
                notional, limits.position_limit
            ));
        }

        if current_daily_loss.abs() > limits.daily_loss_limit {
            return RiskLimitCheck::Rejected(format!(
                "Daily loss {} exceeds limit {}",
                current_daily_loss.abs(),
                limits.daily_loss_limit
            ));
        }

        if current_leverage > limits.leverage_limit {
            return RiskLimitCheck::ReduceSize(limits.leverage_limit / current_leverage);
        }

        RiskLimitCheck::Allowed
    }
}

/// Risk limit check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLimitCheck {
    /// Order allowed
    Allowed,
    /// Order allowed but size should be reduced
    ReduceSize(f64),
    /// Order rejected
    Rejected(String),
}

// =============================================================================
// Unified Advanced Risk Manager
// =============================================================================

/// Complete advanced risk manager
#[derive(Debug)]
pub struct AdvancedRiskManager {
    /// VAR calculator
    pub var_calculator: VarCalculator,
    /// Scenario analyzer
    pub scenario_analyzer: ScenarioAnalyzer,
    /// Greeks calculator
    pub greeks_calculator: GreeksCalculator,
    /// Correlation sizer
    pub correlation_sizer: CorrelationSizer,
    /// Dynamic limit manager
    pub limit_manager: DynamicLimitManager,
}

impl AdvancedRiskManager {
    pub fn new(
        var_config: VarConfig,
        initial_equity: f64,
        margin_requirement: f64,
        maintenance_margin: f64,
        risk_free_rate: f64,
        limit_config: DynamicLimitsConfig,
    ) -> Self {
        Self {
            var_calculator: VarCalculator::new(var_config),
            scenario_analyzer: ScenarioAnalyzer::new(
                initial_equity,
                margin_requirement,
                maintenance_margin,
            ),
            greeks_calculator: GreeksCalculator::new(risk_free_rate),
            correlation_sizer: CorrelationSizer::new(0.7, 0.5, 60),
            limit_manager: DynamicLimitManager::new(limit_config, initial_equity),
        }
    }

    /// Add daily return observation
    pub fn add_daily_return(&mut self, instrument_id: InstrumentId, daily_return: f64) {
        self.var_calculator
            .add_return(instrument_id.clone(), daily_return);
        self.correlation_sizer
            .add_return(instrument_id, daily_return);
    }

    /// Add portfolio return
    pub fn add_portfolio_return(&mut self, daily_return: f64) {
        self.var_calculator.add_portfolio_return(daily_return);
        self.limit_manager.add_daily_return(daily_return);
    }

    /// Update position
    pub fn update_position(
        &mut self,
        instrument_id: InstrumentId,
        quantity: f64,
        avg_price: f64,
        current_price: f64,
    ) {
        self.scenario_analyzer.update_position(
            instrument_id.clone(),
            quantity,
            avg_price,
            current_price,
        );

        let mut positions = HashMap::new();
        positions.insert(instrument_id, quantity * current_price);
        self.var_calculator
            .update_positions(positions, self.scenario_analyzer.equity);
    }

    /// Update equity
    pub fn update_equity(&mut self, equity: f64) {
        self.scenario_analyzer.update_equity(equity);
        self.limit_manager.update_equity(equity);
    }

    /// Get comprehensive risk report
    pub fn generate_risk_report(&self, timestamp: UnixNanos) -> RiskReport {
        let var = self.var_calculator.calculate(timestamp);
        let cvar = self.var_calculator.calculate_cvar(timestamp);
        let stress_tests = self.scenario_analyzer.run_all_scenarios();
        let limits = self.limit_manager.calculate_limits();
        let trading_allowed = limits.trading_allowed;

        RiskReport {
            timestamp,
            var_95: var.var_value,
            var_99: var.var_value * 1.4, // Approximate 99% from 95%
            cvar_95: cvar.cvar_value,
            volatility_regime: limits.volatility_regime,
            current_volatility: limits.current_volatility,
            stress_tests,
            limits,
            trading_allowed,
        }
    }

    /// Update correlations
    pub fn update_correlations(&mut self, timestamp: UnixNanos) {
        self.correlation_sizer.update_correlations(timestamp);
    }

    /// Get adjusted position size
    pub fn get_adjusted_size(
        &self,
        instrument_id: &InstrumentId,
        base_size: f64,
        existing_positions: &HashMap<InstrumentId, f64>,
    ) -> f64 {
        self.correlation_sizer
            .adjust_size(instrument_id, base_size, existing_positions)
    }

    /// Check risk limits
    pub fn check_limits(
        &self,
        notional: f64,
        current_daily_loss: f64,
        current_leverage: f64,
    ) -> RiskLimitCheck {
        self.limit_manager
            .check_order(notional, current_daily_loss, current_leverage)
    }
}

/// Comprehensive risk report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskReport {
    pub timestamp: UnixNanos,
    pub var_95: f64,
    pub var_99: f64,
    pub cvar_95: f64,
    pub volatility_regime: VolatilityRegime,
    pub current_volatility: f64,
    pub stress_tests: Vec<StressTestResult>,
    pub limits: CurrentLimits,
    pub trading_allowed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_calculator() {
        let config = VarConfig {
            method: VarMethod::Historical,
            confidence_level: 0.95,
            ..Default::default()
        };

        let mut calculator = VarCalculator::new(config);

        // Add some synthetic returns
        for i in 0..100 {
            let return_val = ((i % 10) as f64 - 5.0) / 100.0; // -5% to +4%
            calculator.add_portfolio_return(return_val);
        }

        let mut positions = HashMap::new();
        positions.insert(
            InstrumentId::new(
                neleus_core_types::Venue::Simulated,
                "BTC",
                neleus_core_types::InstrumentType::Perp,
            ),
            50000.0,
        );
        calculator.update_positions(positions, 100000.0);

        let result = calculator.calculate(UnixNanos::ZERO);

        assert!(result.var_value > 0.0);
        assert!(result.var_pct > 0.0);
    }

    #[test]
    fn test_stress_test() {
        let mut analyzer = ScenarioAnalyzer::new(100000.0, 0.1, 0.05);

        analyzer.update_position(
            InstrumentId::new(
                neleus_core_types::Venue::Simulated,
                "BTC",
                neleus_core_types::InstrumentType::Perp,
            ),
            2.0,
            50000.0,
            50000.0,
        );

        let result = analyzer.run_stress_test(&StressTestParams::flash_crash());

        assert!(result.portfolio_pnl < 0.0); // Should be negative
        assert_eq!(result.scenario, StressScenario::FlashCrash);
    }

    #[test]
    fn test_dynamic_limits() {
        let config = DynamicLimitsConfig::default();
        let mut manager = DynamicLimitManager::new(config, 100000.0);

        // Add some returns
        for _ in 0..30 {
            manager.add_daily_return(0.01); // 1% daily returns = normal vol
        }

        let limits = manager.calculate_limits();
        assert!(limits.trading_allowed);
        assert_eq!(limits.volatility_regime, VolatilityRegime::Normal);

        // Simulate drawdown
        manager.update_equity(85000.0); // 15% drawdown

        let limits = manager.calculate_limits();
        assert!(limits.position_limit < 100000.0); // Should be reduced
    }

    #[test]
    fn test_correlation_sizer() {
        let mut sizer = CorrelationSizer::new(0.7, 0.5, 60);

        let btc = InstrumentId::new(
            neleus_core_types::Venue::Simulated,
            "BTC",
            neleus_core_types::InstrumentType::Perp,
        );
        let eth = InstrumentId::new(
            neleus_core_types::Venue::Simulated,
            "ETH",
            neleus_core_types::InstrumentType::Perp,
        );

        // Add correlated returns
        for i in 0..100 {
            let btc_return = (i % 10) as f64 / 100.0;
            let eth_return = btc_return * 0.9 + 0.01; // 90% correlated
            sizer.add_return(btc.clone(), btc_return);
            sizer.add_return(eth.clone(), eth_return);
        }

        sizer.update_correlations(UnixNanos::ZERO);

        let mut positions = HashMap::new();
        positions.insert(btc.clone(), 10000.0);

        // ETH size should be reduced due to correlation with BTC
        let adjusted = sizer.adjust_size(&eth, 10000.0, &positions);
        assert!(adjusted < 10000.0);
    }
}
