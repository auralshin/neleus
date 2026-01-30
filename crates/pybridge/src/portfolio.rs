use pyo3::prelude::*;
use std::collections::HashMap;

#[pyclass(eq, eq_int, name = "AllocationMethod")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyAllocationMethod {
    Equal,
    RiskParity,
    PerformanceWeighted,
    VolatilityAdjusted,
    Kelly,
    Fixed,
}

#[pyclass(eq, eq_int, name = "StrategyState")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyStrategyState {
    Active,
    Paused,
    Disabled,
    Liquidating,
    Error,
}

#[pyclass(name = "StrategyPerformance")]
#[derive(Debug, Clone)]
pub struct PyStrategyPerformance {
    #[pyo3(get)]
    pub strategy_id: String,
    #[pyo3(get)]
    pub total_pnl: f64,
    #[pyo3(get)]
    pub realized_pnl: f64,
    #[pyo3(get)]
    pub unrealized_pnl: f64,
    #[pyo3(get)]
    pub total_trades: u64,
    #[pyo3(get)]
    pub winning_trades: u64,
    #[pyo3(get)]
    pub win_rate: f64,
    #[pyo3(get)]
    pub sharpe_ratio: f64,
    #[pyo3(get)]
    pub max_drawdown_pct: f64,
    #[pyo3(get)]
    pub current_drawdown_pct: f64,
    #[pyo3(get)]
    pub profit_factor: f64,
    #[pyo3(get)]
    pub avg_trade_pnl: f64,
    #[pyo3(get)]
    pub capital_allocated: f64,
    #[pyo3(get)]
    pub return_on_capital: f64,
}

#[pymethods]
impl PyStrategyPerformance {
    #[new]
    #[pyo3(signature = (strategy_id, capital_allocated=100000.0))]
    pub fn new(strategy_id: String, capital_allocated: f64) -> Self {
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
            capital_allocated,
            return_on_capital: 0.0,
        }
    }
}

#[pyclass(name = "PortfolioStats")]
#[derive(Debug, Clone)]
pub struct PyPortfolioStats {
    #[pyo3(get)]
    pub total_pnl: f64,
    #[pyo3(get)]
    pub portfolio_return: f64,
    #[pyo3(get)]
    pub portfolio_sharpe: f64,
    #[pyo3(get)]
    pub total_trades: u64,
    #[pyo3(get)]
    pub strategy_count: usize,
    #[pyo3(get)]
    pub active_return: f64,
}

#[pymethods]
impl PyPortfolioStats {
    #[new]
    pub fn new() -> Self {
        Self {
            total_pnl: 0.0,
            portfolio_return: 0.0,
            portfolio_sharpe: 0.0,
            total_trades: 0,
            strategy_count: 0,
            active_return: 0.0,
        }
    }
}

#[pyclass(name = "NettingResult")]
#[derive(Debug, Clone)]
pub struct PyNettingResult {
    #[pyo3(get)]
    pub instrument_symbol: String,
    #[pyo3(get)]
    pub gross_long: f64,
    #[pyo3(get)]
    pub gross_short: f64,
    #[pyo3(get)]
    pub net_position: f64,
    #[pyo3(get)]
    pub netting_efficiency: f64,
    #[pyo3(get)]
    pub capital_saved: f64,
}

#[pymethods]
impl PyNettingResult {
    #[new]
    pub fn new(instrument_symbol: String) -> Self {
        Self {
            instrument_symbol,
            gross_long: 0.0,
            gross_short: 0.0,
            net_position: 0.0,
            netting_efficiency: 0.0,
            capital_saved: 0.0,
        }
    }
}

#[pyclass(name = "StrategyAttribution")]
#[derive(Debug, Clone)]
pub struct PyStrategyAttribution {
    #[pyo3(get)]
    pub strategy_id: String,
    #[pyo3(get)]
    pub total_return: f64,
    #[pyo3(get)]
    pub active_return: f64,
    #[pyo3(get)]
    pub tracking_error: f64,
    #[pyo3(get)]
    pub information_ratio: f64,
    #[pyo3(get)]
    pub portfolio_contribution: f64,
    #[pyo3(get)]
    pub risk_contribution: f64,
}

#[pymethods]
impl PyStrategyAttribution {
    #[new]
    pub fn new(strategy_id: String) -> Self {
        Self {
            strategy_id,
            total_return: 0.0,
            active_return: 0.0,
            tracking_error: 0.0,
            information_ratio: 0.0,
            portfolio_contribution: 0.0,
            risk_contribution: 0.0,
        }
    }
}

#[pyclass(eq, eq_int, name = "VarMethod")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyVarMethod {
    Historical,
    Parametric,
    MonteCarlo,
}

#[pyclass(eq, eq_int, name = "VolatilityRegime")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyVolatilityRegime {
    Low,
    Normal,
    High,
    Extreme,
}

#[pyclass(eq, eq_int, name = "StressScenario")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyStressScenario {
    FlashCrash,
    MarketCorrection,
    LiquidityCrisis,
    VolatilitySpike,
    RateShock,
    BlackSwan,
    Custom,
}

#[pyclass(name = "VarConfig")]
#[derive(Debug, Clone)]
pub struct PyVarConfig {
    #[pyo3(get, set)]
    pub method: PyVarMethod,
    #[pyo3(get, set)]
    pub confidence_level: f64,
    #[pyo3(get, set)]
    pub holding_period_days: u32,
    #[pyo3(get, set)]
    pub lookback_days: u32,
    #[pyo3(get, set)]
    pub monte_carlo_sims: u32,
}

#[pymethods]
impl PyVarConfig {
    #[new]
    #[pyo3(signature = (method=PyVarMethod::Historical, confidence_level=0.95, holding_period_days=1, lookback_days=252, monte_carlo_sims=10000))]
    pub fn new(
        method: PyVarMethod,
        confidence_level: f64,
        holding_period_days: u32,
        lookback_days: u32,
        monte_carlo_sims: u32,
    ) -> Self {
        Self {
            method,
            confidence_level,
            holding_period_days,
            lookback_days,
            monte_carlo_sims,
        }
    }
}

#[pyclass(name = "VarResult")]
#[derive(Debug, Clone)]
pub struct PyVarResult {
    #[pyo3(get)]
    pub var_value: f64,
    #[pyo3(get)]
    pub var_pct: f64,
    #[pyo3(get)]
    pub confidence_level: f64,
    #[pyo3(get)]
    pub holding_period_days: u32,
    #[pyo3(get)]
    pub component_var: HashMap<String, f64>,
    #[pyo3(get)]
    pub marginal_var: HashMap<String, f64>,
}

#[pymethods]
impl PyVarResult {
    #[new]
    pub fn new(
        var_value: f64,
        var_pct: f64,
        confidence_level: f64,
        holding_period_days: u32,
    ) -> Self {
        Self {
            var_value,
            var_pct,
            confidence_level,
            holding_period_days,
            component_var: HashMap::new(),
            marginal_var: HashMap::new(),
        }
    }
}

#[pyclass(name = "CvarResult")]
#[derive(Debug, Clone)]
pub struct PyCvarResult {
    #[pyo3(get)]
    pub cvar_value: f64,
    #[pyo3(get)]
    pub cvar_pct: f64,
    #[pyo3(get)]
    pub var_value: f64,
    #[pyo3(get)]
    pub confidence_level: f64,
    #[pyo3(get)]
    pub holding_period_days: u32,
}

#[pymethods]
impl PyCvarResult {
    #[new]
    pub fn new(
        cvar_value: f64,
        cvar_pct: f64,
        var_value: f64,
        confidence_level: f64,
        holding_period_days: u32,
    ) -> Self {
        Self {
            cvar_value,
            cvar_pct,
            var_value,
            confidence_level,
            holding_period_days,
        }
    }
}

#[pyclass(name = "StressTestParams")]
#[derive(Debug, Clone)]
pub struct PyStressTestParams {
    #[pyo3(get, set)]
    pub scenario: PyStressScenario,
    #[pyo3(get, set)]
    pub price_shock: f64,
    #[pyo3(get, set)]
    pub volatility_multiplier: f64,
    #[pyo3(get, set)]
    pub spread_widening_bps: f64,
    #[pyo3(get, set)]
    pub liquidity_reduction: f64,
    #[pyo3(get, set)]
    pub correlation_shock: f64,
    #[pyo3(get, set)]
    pub description: String,
}

#[pymethods]
impl PyStressTestParams {
    #[new]
    #[pyo3(signature = (scenario, price_shock=-0.1, volatility_multiplier=2.0, spread_widening_bps=100.0, liquidity_reduction=0.5, correlation_shock=0.2, description="Custom scenario".to_string()))]
    pub fn new(
        scenario: PyStressScenario,
        price_shock: f64,
        volatility_multiplier: f64,
        spread_widening_bps: f64,
        liquidity_reduction: f64,
        correlation_shock: f64,
        description: String,
    ) -> Self {
        Self {
            scenario,
            price_shock,
            volatility_multiplier,
            spread_widening_bps,
            liquidity_reduction,
            correlation_shock,
            description,
        }
    }

    #[staticmethod]
    pub fn flash_crash() -> Self {
        Self {
            scenario: PyStressScenario::FlashCrash,
            price_shock: -0.10,
            volatility_multiplier: 5.0,
            spread_widening_bps: 500.0,
            liquidity_reduction: 0.80,
            correlation_shock: 0.3,
            description: "Flash crash: 10% price drop, 5x volatility, 80% liquidity reduction"
                .to_string(),
        }
    }

    #[staticmethod]
    pub fn market_correction() -> Self {
        Self {
            scenario: PyStressScenario::MarketCorrection,
            price_shock: -0.20,
            volatility_multiplier: 2.0,
            spread_widening_bps: 100.0,
            liquidity_reduction: 0.30,
            correlation_shock: 0.2,
            description: "Market correction: 20% price drop, 2x volatility".to_string(),
        }
    }

    #[staticmethod]
    pub fn liquidity_crisis() -> Self {
        Self {
            scenario: PyStressScenario::LiquidityCrisis,
            price_shock: -0.05,
            volatility_multiplier: 3.0,
            spread_widening_bps: 1000.0,
            liquidity_reduction: 0.90,
            correlation_shock: 0.4,
            description: "Liquidity crisis: 90% liquidity reduction, massive spread widening"
                .to_string(),
        }
    }

    #[staticmethod]
    pub fn black_swan() -> Self {
        Self {
            scenario: PyStressScenario::BlackSwan,
            price_shock: -0.30,
            volatility_multiplier: 10.0,
            spread_widening_bps: 2000.0,
            liquidity_reduction: 0.95,
            correlation_shock: 0.5,
            description: "Black swan: 30% price drop, extreme conditions".to_string(),
        }
    }
}

#[pyclass(name = "PositionImpact")]
#[derive(Debug, Clone)]
pub struct PyPositionImpact {
    #[pyo3(get)]
    pub instrument_symbol: String,
    #[pyo3(get)]
    pub position: f64,
    #[pyo3(get)]
    pub price_impact: f64,
    #[pyo3(get)]
    pub slippage_impact: f64,
    #[pyo3(get)]
    pub total_impact: f64,
}

#[pymethods]
impl PyPositionImpact {
    #[new]
    pub fn new(instrument_symbol: String, position: f64) -> Self {
        Self {
            instrument_symbol,
            position,
            price_impact: 0.0,
            slippage_impact: 0.0,
            total_impact: 0.0,
        }
    }
}

#[pyclass(name = "StressTestResult")]
#[derive(Debug, Clone)]
pub struct PyStressTestResult {
    #[pyo3(get)]
    pub scenario: PyStressScenario,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub portfolio_pnl: f64,
    #[pyo3(get)]
    pub portfolio_pnl_pct: f64,
    #[pyo3(get)]
    pub position_impacts: Vec<PyPositionImpact>,
    #[pyo3(get)]
    pub estimated_slippage: f64,
    #[pyo3(get)]
    pub margin_call: bool,
    #[pyo3(get)]
    pub liquidation: bool,
}

#[pymethods]
impl PyStressTestResult {
    #[new]
    pub fn new(scenario: PyStressScenario, description: String) -> Self {
        Self {
            scenario,
            description,
            portfolio_pnl: 0.0,
            portfolio_pnl_pct: 0.0,
            position_impacts: Vec::new(),
            estimated_slippage: 0.0,
            margin_call: false,
            liquidation: false,
        }
    }
}

#[pyclass(name = "Greeks")]
#[derive(Debug, Clone)]
pub struct PyGreeks {
    #[pyo3(get)]
    pub delta: f64,
    #[pyo3(get)]
    pub gamma: f64,
    #[pyo3(get)]
    pub vega: f64,
    #[pyo3(get)]
    pub theta: f64,
    #[pyo3(get)]
    pub rho: f64,
}

#[pymethods]
impl PyGreeks {
    #[new]
    #[pyo3(signature = (delta=0.0, gamma=0.0, vega=0.0, theta=0.0, rho=0.0))]
    pub fn new(delta: f64, gamma: f64, vega: f64, theta: f64, rho: f64) -> Self {
        Self {
            delta,
            gamma,
            vega,
            theta,
            rho,
        }
    }

    pub fn __repr__(&self) -> String {
        format!(
            "Greeks(delta={:.4}, gamma={:.4}, vega={:.4}, theta={:.4}, rho={:.4})",
            self.delta, self.gamma, self.vega, self.theta, self.rho
        )
    }
}

#[pyclass(name = "CurrentLimits")]
#[derive(Debug, Clone)]
pub struct PyCurrentLimits {
    #[pyo3(get)]
    pub position_limit: f64,
    #[pyo3(get)]
    pub daily_loss_limit: f64,
    #[pyo3(get)]
    pub leverage_limit: f64,
    #[pyo3(get)]
    pub volatility_regime: PyVolatilityRegime,
    #[pyo3(get)]
    pub current_volatility: f64,
    #[pyo3(get)]
    pub drawdown_factor: f64,
    #[pyo3(get)]
    pub trading_allowed: bool,
}

#[pymethods]
impl PyCurrentLimits {
    #[new]
    pub fn new() -> Self {
        Self {
            position_limit: 100000.0,
            daily_loss_limit: 5000.0,
            leverage_limit: 5.0,
            volatility_regime: PyVolatilityRegime::Normal,
            current_volatility: 0.02,
            drawdown_factor: 1.0,
            trading_allowed: true,
        }
    }
}

#[pyclass(eq, eq_int, name = "RiskLimitCheckResult")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyRiskLimitCheckResult {
    Allowed,
    ReduceSize,
    Rejected,
}

#[pyclass(name = "RiskReport")]
#[derive(Debug, Clone)]
pub struct PyRiskReport {
    #[pyo3(get)]
    pub timestamp_ns: u64,
    #[pyo3(get)]
    pub var_95: f64,
    #[pyo3(get)]
    pub var_99: f64,
    #[pyo3(get)]
    pub cvar_95: f64,
    #[pyo3(get)]
    pub volatility_regime: PyVolatilityRegime,
    #[pyo3(get)]
    pub current_volatility: f64,
    #[pyo3(get)]
    pub stress_tests: Vec<PyStressTestResult>,
    #[pyo3(get)]
    pub limits: PyCurrentLimits,
    #[pyo3(get)]
    pub trading_allowed: bool,
}

#[pymethods]
impl PyRiskReport {
    #[new]
    pub fn new(timestamp_ns: u64) -> Self {
        Self {
            timestamp_ns,
            var_95: 0.0,
            var_99: 0.0,
            cvar_95: 0.0,
            volatility_regime: PyVolatilityRegime::Normal,
            current_volatility: 0.02,
            stress_tests: Vec::new(),
            limits: PyCurrentLimits::new(),
            trading_allowed: true,
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "RiskReport: VAR95=${:.2}, VAR99=${:.2}, CVaR=${:.2}, Regime={:?}, Trading={}",
            self.var_95, self.var_99, self.cvar_95, self.volatility_regime, self.trading_allowed
        )
    }
}

#[pyclass(name = "DynamicLimitsConfig")]
#[derive(Debug, Clone)]
pub struct PyDynamicLimitsConfig {
    #[pyo3(get, set)]
    pub base_position_limit: f64,
    #[pyo3(get, set)]
    pub base_daily_loss_limit: f64,
    #[pyo3(get, set)]
    pub base_leverage_limit: f64,
    #[pyo3(get, set)]
    pub low_vol_threshold: f64,
    #[pyo3(get, set)]
    pub normal_vol_threshold: f64,
    #[pyo3(get, set)]
    pub high_vol_threshold: f64,
    #[pyo3(get, set)]
    pub volatility_lookback_days: u32,
}

#[pymethods]
impl PyDynamicLimitsConfig {
    #[new]
    #[pyo3(signature = (
        base_position_limit=100000.0,
        base_daily_loss_limit=5000.0,
        base_leverage_limit=5.0,
        low_vol_threshold=0.01,
        normal_vol_threshold=0.02,
        high_vol_threshold=0.04,
        volatility_lookback_days=20
    ))]
    pub fn new(
        base_position_limit: f64,
        base_daily_loss_limit: f64,
        base_leverage_limit: f64,
        low_vol_threshold: f64,
        normal_vol_threshold: f64,
        high_vol_threshold: f64,
        volatility_lookback_days: u32,
    ) -> Self {
        Self {
            base_position_limit,
            base_daily_loss_limit,
            base_leverage_limit,
            low_vol_threshold,
            normal_vol_threshold,
            high_vol_threshold,
            volatility_lookback_days,
        }
    }
}
