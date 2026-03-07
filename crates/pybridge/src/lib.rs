use pyo3::prelude::*;

mod types;
mod execution;
mod portfolio;
mod adapters;
mod engine;
mod agent_bindings;
mod trading;

pub use types::*;
pub use execution::*;
pub use portfolio::*;
pub use adapters::*;
pub use engine::*;
pub use agent_bindings::*;
pub use trading::*;

#[pyfunction]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pymodule]
fn neleus_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(version, m)?)?;

    m.add_class::<PyVenue>()?;
    m.add_class::<PyInstrumentType>()?;
    m.add_class::<PyOrderSide>()?;
    m.add_class::<PyOrderType>()?;
    m.add_class::<PyTimeInForce>()?;
    m.add_class::<PyOrderState>()?;
    m.add_class::<PyPositionSide>()?;
    m.add_class::<PySubscriptionType>()?;
    m.add_class::<PyNetwork>()?;
    m.add_class::<PyFillModel>()?;
    m.add_class::<PyLatencyModel>()?;

    m.add_class::<PyInstrumentId>()?;
    m.add_class::<PyTradeTick>()?;
    m.add_class::<PyQuoteTick>()?;
    m.add_class::<PyBookLevel>()?;
    m.add_class::<PyOrderBook>()?;
    m.add_class::<PyBar>()?;

    m.add_class::<PyOrder>()?;
    m.add_class::<PyFill>()?;
    m.add_class::<PyPosition>()?;
    m.add_class::<PyOrderRequest>()?;

    m.add_class::<PyStrategyContext>()?;
    m.add_class::<PyBacktestResults>()?;

    m.add_class::<PyHyperliquidConfig>()?;
    m.add_class::<PyLighterConfig>()?;
    m.add_class::<PyBacktestConfig>()?;
    m.add_class::<PyRiskConfig>()?;

    m.add_class::<PyTwapParams>()?;
    m.add_class::<PyVwapParams>()?;
    m.add_class::<PyIcebergParams>()?;

    m.add_class::<PyExecutionState>()?;
    m.add_class::<PyAdaptiveMode>()?;
    m.add_class::<PyAdaptiveParams>()?;
    m.add_class::<PyMarketConditions>()?;

    m.add_class::<PyAllocationMethod>()?;
    m.add_class::<PyStrategyState>()?;
    m.add_class::<PyStrategyPerformance>()?;
    m.add_class::<PyPortfolioStats>()?;
    m.add_class::<PyNettingResult>()?;
    m.add_class::<PyStrategyAttribution>()?;

    m.add_class::<PyVarMethod>()?;
    m.add_class::<PyVolatilityRegime>()?;
    m.add_class::<PyStressScenario>()?;
    m.add_class::<PyVarConfig>()?;
    m.add_class::<PyVarResult>()?;
    m.add_class::<PyCvarResult>()?;
    m.add_class::<PyStressTestParams>()?;
    m.add_class::<PyPositionImpact>()?;
    m.add_class::<PyStressTestResult>()?;
    m.add_class::<PyGreeks>()?;
    m.add_class::<PyCurrentLimits>()?;
    m.add_class::<PyRiskLimitCheckResult>()?;
    m.add_class::<PyRiskReport>()?;
    m.add_class::<PyDynamicLimitsConfig>()?;

    m.add_class::<PyHyperliquidClient>()?;
    m.add_class::<PyHyperliquidCandle>()?;
    m.add_class::<PyHyperliquidMeta>()?;
    m.add_class::<PyHyperliquidAsset>()?;
    m.add_class::<PyHyperliquidSpotMeta>()?;
    m.add_class::<PyHyperliquidSpotToken>()?;
    m.add_class::<PyHyperliquidSpotMarket>()?;
    m.add_class::<PyHyperliquidL2Level>()?;
    m.add_class::<PyHyperliquidL2BookUpdate>()?;
    m.add_class::<PyHyperliquidL2BookStream>()?;

    m.add_class::<PyPostgresEventStoreConfig>()?;
    m.add_class::<PyPostgresEventStore>()?;
    m.add_class::<PyTimescaleConfig>()?;
    m.add_class::<PyTimescaleStore>()?;

    // Live trading
    m.add_class::<PyOrderResult>()?;
    m.add_class::<PyOpenOrder>()?;
    m.add_class::<PyFillRecord>()?;
    m.add_class::<PyHyperliquidTrader>()?;

    // DB-backed trade monitoring
    m.add_class::<PyDbOrderRecord>()?;
    m.add_class::<PyDbFillRecord>()?;
    m.add_class::<PyPnlSummary>()?;
    m.add_class::<PyTradeMonitor>()?;

    m.add_class::<PyEngineConfig>()?;
    m.add_class::<PyEngineState>()?;
    m.add_class::<PyEngine>()?;

    m.add_class::<PyLiveNodeState>()?;
    m.add_class::<PyLiveNodeConfig>()?;
    m.add_class::<PyCircuitState>()?;
    m.add_class::<PyLiveNode>()?;

    // Agent AI types
    agent_bindings::register_agent_types(m)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instrument_id_conversion() {
        let py_id = PyInstrumentId::new(
            PyVenue::Hyperliquid,
            "BTC".to_string(),
            PyInstrumentType::Perp,
        );
        let rust_id = py_id.to_rust();
        assert_eq!(rust_id.venue, neleus_core_types::Venue::Hyperliquid);
        assert_eq!(rust_id.symbol, "BTC");
    }

    #[test]
    fn test_strategy_context() {
        let mut ctx = PyStrategyContext::new(1000);

        ctx.market_order(
            PyInstrumentId::new(
                PyVenue::Simulated,
                "ETH".to_string(),
                PyInstrumentType::Perp,
            ),
            PyOrderSide::Buy,
            1.0,
            false,
        );

        let orders = ctx.drain_orders();
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].quantity, 1.0);
    }

    #[test]
    fn test_position_pnl() {
        let pos = PyPosition::new(
            PyInstrumentId::new(
                PyVenue::Simulated,
                "BTC".to_string(),
                PyInstrumentType::Perp,
            ),
            PyPositionSide::Long,
            1.0,
            50000.0,
            100.0,
            500.0,
        );

        assert_eq!(pos.total_pnl(), 600.0);
        assert!(!pos.is_flat());
        assert!(pos.is_long());
    }

    #[test]
    fn test_backtest_results() {
        let results = PyBacktestResults {
            initial_balance: 100000.0,
            final_balance: 110000.0,
            total_pnl: 10000.0,
            return_pct: 10.0,
            total_trades: 100,
            winning_trades: 60,
            losing_trades: 40,
            total_volume: 1000000.0,
            total_commission: 400.0,
            max_drawdown_pct: 5.0,
            sharpe_ratio: 2.0,
            sortino_ratio: 2.5,
            calmar_ratio: 2.0,
            start_time_ns: 0,
            end_time_ns: 1000000000,
            avg_trade_pnl: 100.0,
            max_consecutive_wins: 10,
            max_consecutive_losses: 5,
        };

        assert_eq!(results.win_rate(), 60.0);
        assert!(results.profit_factor() > 1.0);
    }

    #[test]
    fn test_twap_params() {
        let twap = PyTwapParams::new(
            PyInstrumentId::new(
                PyVenue::Hyperliquid,
                "BTC".to_string(),
                PyInstrumentType::Perp,
            ),
            PyOrderSide::Buy,
            10.0,
            600,
            10,
            true,
            None,
        );

        assert_eq!(twap.slice_quantity(), 1.0);
        assert_eq!(twap.slice_interval_secs(), 60.0);
    }
}
