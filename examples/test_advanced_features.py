#!/usr/bin/env python3
"""
Test script for Neleus advanced features:
- Execution Algorithms (TWAP, VWAP, Iceberg, Adaptive)
- Portfolio Management (Allocation, Netting, Attribution, Orchestration)
- Advanced Risk (VAR, CVaR, Scenarios, Greeks, Dynamic Limits)
"""

import sys
import time
from datetime import datetime

# Import neleus_core types
try:
    from neleus_core import (
        # Base types
        Venue as PyVenue, InstrumentId as PyInstrumentId, InstrumentType as PyInstrumentType, 
        OrderSide as PyOrderSide,
        
        # Execution algorithms
        TwapParams as PyTwapParams, VwapParams as PyVwapParams, 
        IcebergParams as PyIcebergParams, AdaptiveParams as PyAdaptiveParams,
        ExecutionState as PyExecutionState, AdaptiveMode as PyAdaptiveMode, 
        MarketConditions as PyMarketConditions,
        
        # Portfolio management
        AllocationMethod as PyAllocationMethod, StrategyState as PyStrategyState, 
        StrategyPerformance as PyStrategyPerformance,
        PortfolioStats as PyPortfolioStats, NettingResult as PyNettingResult, 
        StrategyAttribution as PyStrategyAttribution,
        
        # Advanced risk
        VarMethod as PyVarMethod, VolatilityRegime as PyVolatilityRegime, 
        StressScenario as PyStressScenario,
        VarConfig as PyVarConfig, VarResult as PyVarResult, CvarResult as PyCvarResult,
        StressTestParams as PyStressTestParams, PositionImpact as PyPositionImpact, 
        StressTestResult as PyStressTestResult,
        Greeks as PyGreeks, CurrentLimits as PyCurrentLimits, 
        RiskLimitCheckResult as PyRiskLimitCheckResult,
        RiskReport as PyRiskReport, DynamicLimitsConfig as PyDynamicLimitsConfig,
    )
    print("  Successfully imported neleus_core types")
except ImportError as e:
    print(f"  Failed to import neleus_core: {e}")   
    sys.exit(1)


def test_execution_algorithms():
    """Test execution algorithm types"""
    print("\n" + "="*60)
    print("TESTING EXECUTION ALGORITHMS")
    print("="*60)
    
    # Create instrument
    btc_perp = PyInstrumentId(PyVenue.Hyperliquid, "BTC-PERP", PyInstrumentType.Perp)
    print(f"\n  Testing with instrument: {btc_perp}")
    
    # Test TWAP parameters
    print("\n--- TWAP Parameters ---")
    twap = PyTwapParams(
        instrument_id=btc_perp,
        side=PyOrderSide.Buy,
        total_quantity=10.0,
        duration_secs=3600,  # 1 hour
        num_slices=12,       # Every 5 minutes
        limit_price=50000.0
    )
    print(f"  Instrument: {twap.instrument_id}")
    print(f"  Side: {twap.side}")
    print(f"  Total Quantity: {twap.total_quantity}")
    print(f"  Duration: {twap.duration_secs} seconds")
    print(f"  Num Slices: {twap.num_slices}")
    print(f"  Limit Price: ${twap.limit_price}")
    
    # Test VWAP parameters
    print("\n--- VWAP Parameters ---")
    vwap = PyVwapParams(
        instrument_id=btc_perp,
        side=PyOrderSide.Sell,
        total_quantity=5.0,
        participation_rate=0.1,  # 10% of volume
        min_slice=0.1,
        max_slice=1.0,
        limit_price=51000.0
    )
    print(f"  Participation Rate: {vwap.participation_rate * 100}%")
    print(f"  Min Slice: {vwap.min_slice}")
    print(f"  Max Slice: {vwap.max_slice}")
    
    # Test Iceberg parameters
    print("\n--- Iceberg Parameters ---")
    iceberg = PyIcebergParams(
        instrument_id=btc_perp,
        side=PyOrderSide.Buy,
        total_quantity=100.0,
        display_quantity=5.0,  # Show only 5%
        limit_price=49500.0,
        variance_pct=0.2  # 20% variance in display size
    )
    print(f"  Total Quantity: {iceberg.total_quantity}")
    print(f"  Display Quantity: {iceberg.display_quantity}")
    print(f"  Variance: {iceberg.variance_pct * 100}%")
    print(f"  Hidden Ratio: {(1 - iceberg.display_quantity/iceberg.total_quantity) * 100:.1f}%")
    
    # Test Adaptive parameters
    print("\n--- Adaptive Execution Parameters ---")
    adaptive = PyAdaptiveParams(
        instrument_id=btc_perp,
        side=PyOrderSide.Buy,
        total_quantity=20.0,
        urgency=0.7,        # High urgency
        risk_aversion=0.3,  # Low risk aversion
        max_duration_secs=7200,
        limit_price=50500.0
    )
    print(f"  Urgency: {adaptive.urgency}")
    print(f"  Risk Aversion: {adaptive.risk_aversion}")
    print(f"  Max Duration: {adaptive.max_duration_secs} seconds")
    
    # Test Market Conditions
    print("\n--- Market Conditions ---")
    conditions = PyMarketConditions(
        bid=50000.0,
        ask=50010.0,
        bid_size=100.0,
        ask_size=80.0,
        recent_volume=5000.0,
        volatility=0.02
    )
    print(f"  Bid: ${conditions.bid}")
    print(f"  Ask: ${conditions.ask}")
    print(f"  Mid Price: ${conditions.mid_price()}")
    print(f"  Spread: {conditions.spread_bps():.2f} bps")
    print(f"  Book Imbalance: {conditions.book_imbalance():.2f}")
    print(f"  Recent Volume: {conditions.recent_volume}")
    print(f"  Volatility: {conditions.volatility * 100:.1f}%")
    
    # Test execution states
    print("\n--- Execution States ---")
    for state in [PyExecutionState.Pending, PyExecutionState.Active, 
                  PyExecutionState.Completed, PyExecutionState.Cancelled]:
        print(f"  State: {state}")
    
    # Test adaptive modes
    print("\n--- Adaptive Modes ---")
    for mode in [PyAdaptiveMode.Passive, PyAdaptiveMode.Neutral, 
                 PyAdaptiveMode.Aggressive, PyAdaptiveMode.Opportunistic]:
        print(f"  Mode: {mode}")
    
    print("\n  Execution algorithms test passed!")
    return True


def test_portfolio_management():
    """Test portfolio management types"""
    print("\n" + "="*60)
    print("TESTING PORTFOLIO MANAGEMENT")
    print("="*60)
    
    # Test allocation methods
    print("\n--- Allocation Methods ---")
    for method in [PyAllocationMethod.Equal, PyAllocationMethod.RiskParity,
                   PyAllocationMethod.PerformanceWeighted, 
                   PyAllocationMethod.VolatilityAdjusted, PyAllocationMethod.Kelly]:
        print(f"  Method: {method}")
    
    # Test strategy states
    print("\n--- Strategy States ---")
    for state in [PyStrategyState.Active, PyStrategyState.Paused,
                  PyStrategyState.Disabled, PyStrategyState.Liquidating]:
        print(f"  State: {state}")
    
    # Test strategy performance
    print("\n--- Strategy Performance ---")
    perf = PyStrategyPerformance("momentum_btc", capital_allocated=100000.0)
    print(f"  Strategy ID: {perf.strategy_id}")
    print(f"  Capital Allocated: ${perf.capital_allocated:,.2f}")
    print(f"  Total PnL: ${perf.total_pnl:,.2f}")
    print(f"  Win Rate: {perf.win_rate * 100:.1f}%")
    print(f"  Sharpe Ratio: {perf.sharpe_ratio:.2f}")
    print(f"  Max Drawdown: {perf.max_drawdown_pct * 100:.1f}%")
    
    # Test portfolio stats
    print("\n--- Portfolio Stats ---")
    stats = PyPortfolioStats()
    print(f"  Total PnL: ${stats.total_pnl:,.2f}")
    print(f"  Portfolio Return: {stats.portfolio_return * 100:.2f}%")
    print(f"  Portfolio Sharpe: {stats.portfolio_sharpe:.2f}")
    print(f"  Strategy Count: {stats.strategy_count}")
    
    # Test netting result
    print("\n--- Netting Result ---")
    netting = PyNettingResult("BTC-PERP")
    print(f"  Instrument: {netting.instrument_symbol}")
    print(f"  Gross Long: {netting.gross_long}")
    print(f"  Gross Short: {netting.gross_short}")
    print(f"  Net Position: {netting.net_position}")
    print(f"  Netting Efficiency: {netting.netting_efficiency * 100:.1f}%")
    print(f"  Capital Saved: ${netting.capital_saved:,.2f}")
    
    # Test strategy attribution
    print("\n--- Strategy Attribution ---")
    attr = PyStrategyAttribution("mean_reversion_eth")
    print(f"  Strategy: {attr.strategy_id}")
    print(f"  Total Return: {attr.total_return * 100:.2f}%")
    print(f"  Active Return: {attr.active_return * 100:.2f}%")
    print(f"  Information Ratio: {attr.information_ratio:.2f}")
    print(f"  Portfolio Contribution: {attr.portfolio_contribution * 100:.2f}%")
    print(f"  Risk Contribution: {attr.risk_contribution * 100:.2f}%")
    
    print("\n  Portfolio management test passed!")
    return True


def test_advanced_risk():
    """Test advanced risk management types"""
    print("\n" + "="*60)
    print("TESTING ADVANCED RISK MANAGEMENT")
    print("="*60)
    
    # Test VAR methods
    print("\n--- VAR Methods ---")
    for method in [PyVarMethod.Historical, PyVarMethod.Parametric, PyVarMethod.MonteCarlo]:
        print(f"  Method: {method}")
    
    # Test volatility regimes
    print("\n--- Volatility Regimes ---")
    for regime in [PyVolatilityRegime.Low, PyVolatilityRegime.Normal,
                   PyVolatilityRegime.High, PyVolatilityRegime.Extreme]:
        print(f"  Regime: {regime}")
    
    # Test VAR configuration
    print("\n--- VAR Configuration ---")
    var_config = PyVarConfig(
        method=PyVarMethod.Historical,
        confidence_level=0.95,
        holding_period_days=1,
        lookback_days=252,
        monte_carlo_sims=10000
    )
    print(f"  Method: {var_config.method}")
    print(f"  Confidence Level: {var_config.confidence_level * 100:.0f}%")
    print(f"  Holding Period: {var_config.holding_period_days} day(s)")
    print(f"  Lookback: {var_config.lookback_days} days")
    print(f"  Monte Carlo Sims: {var_config.monte_carlo_sims:,}")
    
    # Test VAR result
    print("\n--- VAR Result ---")
    var_result = PyVarResult(
        var_value=25000.0,
        var_pct=0.05,
        confidence_level=0.95,
        holding_period_days=1
    )
    print(f"  VAR Value: ${var_result.var_value:,.2f}")
    print(f"  VAR %: {var_result.var_pct * 100:.1f}%")
    print(f"  Confidence: {var_result.confidence_level * 100:.0f}%")
    print(f"  Holding Period: {var_result.holding_period_days} day(s)")
    
    # Test CVaR result
    print("\n--- CVaR (Expected Shortfall) Result ---")
    cvar_result = PyCvarResult(
        cvar_value=35000.0,
        cvar_pct=0.07,
        var_value=25000.0,
        confidence_level=0.95,
        holding_period_days=1
    )
    print(f"  CVaR Value: ${cvar_result.cvar_value:,.2f}")
    print(f"  CVaR %: {cvar_result.cvar_pct * 100:.1f}%")
    print(f"  VAR Value: ${cvar_result.var_value:,.2f}")
    print(f"  Tail Risk Multiple: {cvar_result.cvar_value / cvar_result.var_value:.2f}x VAR")
    
    # Test stress test scenarios (predefined)
    print("\n--- Stress Test Scenarios (Predefined) ---")
    
    flash_crash = PyStressTestParams.flash_crash()
    print(f"  Flash Crash: {flash_crash.description}")
    print(f"    Price Shock: {flash_crash.price_shock * 100:.0f}%")
    print(f"    Vol Multiplier: {flash_crash.volatility_multiplier}x")
    print(f"    Liquidity Reduction: {flash_crash.liquidity_reduction * 100:.0f}%")
    
    correction = PyStressTestParams.market_correction()
    print(f"\n  Market Correction: {correction.description}")
    print(f"    Price Shock: {correction.price_shock * 100:.0f}%")
    
    liquidity = PyStressTestParams.liquidity_crisis()
    print(f"\n  Liquidity Crisis: {liquidity.description}")
    print(f"    Spread Widening: {liquidity.spread_widening_bps} bps")
    
    black_swan = PyStressTestParams.black_swan()
    print(f"\n  Black Swan: {black_swan.description}")
    print(f"    Price Shock: {black_swan.price_shock * 100:.0f}%")
    print(f"    Vol Multiplier: {black_swan.volatility_multiplier}x")
    
    # Test custom stress scenario
    print("\n--- Custom Stress Scenario ---")
    custom = PyStressTestParams(
        scenario=PyStressScenario.Custom,
        price_shock=-0.15,
        volatility_multiplier=3.0,
        spread_widening_bps=200.0,
        liquidity_reduction=0.6,
        correlation_shock=0.25,
        description="Regulatory crackdown scenario"
    )
    print(f"  Description: {custom.description}")
    print(f"  Price Shock: {custom.price_shock * 100:.0f}%")
    
    # Test stress test result
    print("\n--- Stress Test Result ---")
    stress_result = PyStressTestResult(
        scenario=PyStressScenario.FlashCrash,
        description="Flash crash stress test"
    )
    print(f"  Scenario: {stress_result.scenario}")
    print(f"  Portfolio PnL: ${stress_result.portfolio_pnl:,.2f}")
    print(f"  Margin Call: {stress_result.margin_call}")
    print(f"  Liquidation: {stress_result.liquidation}")
    
    # Test Greeks
    print("\n--- Greeks ---")
    greeks = PyGreeks(
        delta=0.85,
        gamma=0.02,
        vega=0.15,
        theta=-0.05,
        rho=0.01
    )
    print(f"  {greeks}")
    print(f"  Delta: {greeks.delta}")
    print(f"  Gamma: {greeks.gamma}")
    print(f"  Vega: {greeks.vega}")
    print(f"  Theta: {greeks.theta}")
    print(f"  Rho: {greeks.rho}")
    
    # Test current limits
    print("\n--- Current Risk Limits ---")
    limits = PyCurrentLimits()
    print(f"  Position Limit: ${limits.position_limit:,.2f}")
    print(f"  Daily Loss Limit: ${limits.daily_loss_limit:,.2f}")
    print(f"  Leverage Limit: {limits.leverage_limit}x")
    print(f"  Volatility Regime: {limits.volatility_regime}")
    print(f"  Current Volatility: {limits.current_volatility * 100:.1f}%")
    print(f"  Drawdown Factor: {limits.drawdown_factor}")
    print(f"  Trading Allowed: {limits.trading_allowed}")
    
    # Test dynamic limits config
    print("\n--- Dynamic Limits Configuration ---")
    dynamic_config = PyDynamicLimitsConfig(
        base_position_limit=100000.0,
        base_daily_loss_limit=5000.0,
        base_leverage_limit=5.0,
        low_vol_threshold=0.01,
        normal_vol_threshold=0.02,
        high_vol_threshold=0.04,
        volatility_lookback_days=20
    )
    print(f"  Base Position Limit: ${dynamic_config.base_position_limit:,.2f}")
    print(f"  Base Daily Loss Limit: ${dynamic_config.base_daily_loss_limit:,.2f}")
    print(f"  Volatility Thresholds:")
    print(f"    Low: <{dynamic_config.low_vol_threshold * 100:.0f}%")
    print(f"    Normal: <{dynamic_config.normal_vol_threshold * 100:.0f}%")
    print(f"    High: <{dynamic_config.high_vol_threshold * 100:.0f}%")
    print(f"    Extreme: >={dynamic_config.high_vol_threshold * 100:.0f}%")
    
    # Test risk report
    print("\n--- Comprehensive Risk Report ---")
    report = PyRiskReport(timestamp_ns=int(time.time() * 1_000_000_000))
    print(f"  Timestamp: {datetime.fromtimestamp(report.timestamp_ns / 1_000_000_000)}")
    print(f"  VAR 95%: ${report.var_95:,.2f}")
    print(f"  VAR 99%: ${report.var_99:,.2f}")
    print(f"  CVaR 95%: ${report.cvar_95:,.2f}")
    print(f"  Volatility Regime: {report.volatility_regime}")
    print(f"  Trading Allowed: {report.trading_allowed}")
    print(f"  Summary: {report.summary()}")
    
    print("\n  Advanced risk test passed!")
    return True


def test_integration_example():
    """Example of how these components work together"""
    print("\n" + "="*60)
    print("INTEGRATION EXAMPLE: Multi-Strategy Portfolio with Risk Management")
    print("="*60)
    
    # Create instruments
    btc_perp = PyInstrumentId(PyVenue.Hyperliquid, "BTC-PERP", PyInstrumentType.Perp)
    eth_perp = PyInstrumentId(PyVenue.Hyperliquid, "ETH-PERP", PyInstrumentType.Perp)
    
    print("\n📊 Portfolio Setup:")
    print(f"  Instruments: {btc_perp}, {eth_perp}")
    
    # Simulate two strategies
    strategies = {
        "momentum_btc": PyStrategyPerformance("momentum_btc", capital_allocated=50000.0),
        "mean_reversion_eth": PyStrategyPerformance("mean_reversion_eth", capital_allocated=50000.0)
    }
    
    print(f"  Strategies: {list(strategies.keys())}")
    print(f"  Total Capital: ${sum(s.capital_allocated for s in strategies.values()):,.2f}")
    
    # Define execution for large order
    print("\n📈 Execution Plan for Large BTC Order:")
    
    # Market conditions
    conditions = PyMarketConditions(
        bid=50000.0,
        ask=50020.0,
        bid_size=50.0,
        ask_size=45.0,
        recent_volume=2000.0,
        volatility=0.025
    )
    
    # Choose execution algo based on conditions
    spread_bps = conditions.spread_bps()
    print(f"  Current Spread: {spread_bps:.1f} bps")
    print(f"  Book Imbalance: {conditions.book_imbalance():.2f}")
    print(f"  Volatility: {conditions.volatility * 100:.1f}%")
    
    if conditions.volatility > 0.03:
        print("  → High volatility detected, using TWAP for time diversification")
        algo = "TWAP"
    elif spread_bps > 10:
        print("  → Wide spreads detected, using Iceberg to hide size")
        algo = "Iceberg"
    else:
        print("  → Normal conditions, using Adaptive execution")
        algo = "Adaptive"
    
    print(f"  Selected Algorithm: {algo}")
    
    # Risk checks before execution
    print("\n⚠️ Pre-Trade Risk Checks:")
    
    var_config = PyVarConfig(
        method=PyVarMethod.Historical,
        confidence_level=0.95
    )
    
    dynamic_limits = PyDynamicLimitsConfig(
        base_position_limit=100000.0,
        base_daily_loss_limit=5000.0
    )
    
    limits = PyCurrentLimits()
    
    print(f"  Position Limit: ${limits.position_limit:,.2f}")
    print(f"  Daily Loss Limit: ${limits.daily_loss_limit:,.2f}")
    print(f"  Trading Allowed: {limits.trading_allowed}")
    
    # Simulate stress test
    print("\n🔥 Running Stress Tests:")
    for scenario_fn, name in [(PyStressTestParams.flash_crash, "Flash Crash"),
                               (PyStressTestParams.market_correction, "Market Correction"),
                               (PyStressTestParams.black_swan, "Black Swan")]:
        scenario = scenario_fn()
        # Simulate impact (would be calculated in actual system)
        simulated_pnl = 100000 * scenario.price_shock
        print(f"  {name}: ${simulated_pnl:,.0f} ({scenario.price_shock * 100:.0f}%)")
    
    # Attribution example
    print("\n📊 Performance Attribution:")
    for name, perf in strategies.items():
        attr = PyStrategyAttribution(name)
        print(f"  {name}:")
        print(f"    Capital: ${perf.capital_allocated:,.0f}")
        print(f"    Portfolio Contribution: {attr.portfolio_contribution * 100:.1f}%")
    
    print("\n  Integration example completed!")
    return True


def main():
    """Run all tests"""
    print("="*60)
    print("NELEUS ADVANCED FEATURES TEST SUITE")
    print(f"Timestamp: {datetime.now().isoformat()}")
    print("="*60)
    
    results = []
    
    # Run all tests
    tests = [
        ("Execution Algorithms", test_execution_algorithms),
        ("Portfolio Management", test_portfolio_management),
        ("Advanced Risk", test_advanced_risk),
        ("Integration Example", test_integration_example),
    ]
    
    for name, test_fn in tests:
        try:
            result = test_fn()
            results.append((name, result))
        except Exception as e:
            print(f"\n  {name} FAILED: {e}")
            import traceback
            traceback.print_exc()
            results.append((name, False))
    
    # Summary
    print("\n" + "="*60)
    print("TEST SUMMARY")
    print("="*60)
    
    passed = sum(1 for _, r in results if r)
    total = len(results)
    
    for name, result in results:
        status = "  PASSED" if result else "  FAILED"
        print(f"  {status}: {name}")
    
    print(f"\nTotal: {passed}/{total} tests passed")
    
    if passed == total:
        print("\n🎉 All tests passed successfully!")
        return 0
    else:
        print("\n⚠️ Some tests failed!")
        return 1


if __name__ == "__main__":
    sys.exit(main())
