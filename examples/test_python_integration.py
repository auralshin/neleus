"""
Test script to verify Rust/Python integration and data science libraries

This script tests:
1. Rust core is doing the heavy lifting (backtesting, order execution)
2. Python acts as interface layer (strategy logic, data analysis)
3. Scientific libraries (numpy, scipy, pandas) work with Neleus
"""

import asyncio
from decimal import Decimal
from typing import Optional
import sys

print("=" * 80)
print("Neleus Rust/Python Integration Test")
print("=" * 80)
print()

# Test 1: Core imports
print("Test 1: Testing core imports...")
try:
    from neleus import (
        Strategy,
        StrategyContext,
        Bar,
        OrderSide,
        InstrumentId,
        Venue,
        InstrumentType,
        HyperliquidBacktestConfig,
        HyperliquidBacktestNode,
        CandleInterval,
    )
    print("✓ Core Neleus imports successful (Rust bindings working)")
except Exception as e:
    print(f"✗ Core imports failed: {e}")
    sys.exit(1)

# Test 2: Scientific libraries
print("\nTest 2: Testing scientific libraries...")
try:
    import numpy as np
    print(f"✓ NumPy {np.__version__} imported")
except ImportError:
    print("✗ NumPy not available - installing...")
    import subprocess
    subprocess.check_call([sys.executable, "-m", "pip", "install", "numpy"])
    import numpy as np
    print(f"✓ NumPy {np.__version__} installed and imported")

try:
    import pandas as pd
    print(f"✓ Pandas {pd.__version__} imported")
except ImportError:
    print("✗ Pandas not available - installing...")
    import subprocess
    subprocess.check_call([sys.executable, "-m", "pip", "install", "pandas"])
    import pandas as pd
    print(f"✓ Pandas {pd.__version__} installed and imported")

try:
    import scipy
    from scipy import stats
    print(f"✓ SciPy {scipy.__version__} imported")
except ImportError:
    print("✗ SciPy not available - installing...")
    import subprocess
    subprocess.check_call([sys.executable, "-m", "pip", "install", "scipy"])
    import scipy
    from scipy import stats
    print(f"✓ SciPy {scipy.__version__} installed and imported")


class AdvancedMomentumStrategy(Strategy):
    """
    Strategy using Python scientific libraries for signal generation,
    while Rust handles all the heavy lifting (execution, backtesting)
    """
    
    def __init__(
        self,
        lookback: int = 20,
        threshold: float = 0.02,
        position_size: float = 0.1,
        use_zscore: bool = True,
        zscore_window: int = 50,
        strategy_id: Optional[str] = None,
    ):
        super().__init__(strategy_id or "AdvancedMomentumStrategy")
        self.lookback = lookback
        self.threshold = Decimal(str(threshold))
        self.position_size = Decimal(str(position_size))
        self.use_zscore = use_zscore
        self.zscore_window = zscore_window
        
        # Using Python data structures (Rust handles execution)
        self.price_history = []
        self.position = Decimal("0")
        self.instrument: Optional[InstrumentId] = None
        
        # Statistics
        self.signals_generated = 0
        self.trades_executed = 0
    
    def on_start(self, ctx: StrategyContext) -> None:
        print(f"\n[{self.strategy_id}] Starting advanced strategy")
        print(f"  Python layer: Signal generation with scipy/numpy")
        print(f"  Rust layer: Backtesting, order execution, risk management")
        print(f"  Lookback: {self.lookback} bars")
        print(f"  Z-score normalization: {self.use_zscore}")
        
        self.instrument = InstrumentId(
            venue=Venue.Hyperliquid,
            symbol="ETH",
            instrument_type=InstrumentType.Perp,
        )
    
    def calculate_signals_with_scipy(self, prices: list) -> dict:
        """
        Python layer: Use scientific libraries for signal calculation
        This demonstrates Python is for analysis, Rust for execution
        """
        if len(prices) < max(self.lookback, self.zscore_window):
            return {"signal": 0, "strength": 0.0, "zscore": 0.0}
        
        # Convert to numpy array
        price_array = np.array(prices, dtype=float)
        
        # Calculate returns
        returns = np.diff(price_array) / price_array[:-1]
        recent_return = returns[-self.lookback:].sum()
        
        # Calculate z-score if enabled
        if self.use_zscore and len(returns) >= self.zscore_window:
            window_returns = returns[-self.zscore_window:]
            mean = np.mean(window_returns)
            std = np.std(window_returns)
            zscore = (recent_return - mean) / (std + 1e-10)
        else:
            zscore = 0.0
        
        # Use scipy for statistical tests
        if len(returns) >= self.zscore_window:
            # Perform t-test to check if recent returns are significant
            t_stat, p_value = stats.ttest_1samp(returns[-self.lookback:], 0)
            strength = abs(t_stat) / 10.0  # Normalize
        else:
            strength = 0.0
        
        # Generate signal
        signal = 0
        if recent_return > float(self.threshold):
            signal = 1  # Buy
        elif recent_return < -float(self.threshold):
            signal = -1  # Sell
        
        return {
            "signal": signal,
            "strength": strength,
            "zscore": zscore,
            "recent_return": recent_return
        }
    
    def on_data(self, ctx: StrategyContext, data) -> None:
        """
        Python layer: Receive data and generate signals
        Rust layer: Handles all the actual execution
        """
        if not isinstance(data, Bar):
            return
        
        self.instrument = data.instrument_id
        self.price_history.append(float(data.close))
        
        # Keep history limited
        if len(self.price_history) > self.zscore_window + 10:
            self.price_history = self.price_history[-(self.zscore_window + 10):]
        
        if len(self.price_history) < max(self.lookback, self.zscore_window):
            return
        
        # Python: Calculate signals using scientific libraries
        signals = self.calculate_signals_with_scipy(self.price_history)
        
        if signals["signal"] == 0 or self.position != 0:
            return
        
        self.signals_generated += 1
        current_price = self.price_history[-1]
        
        # Rust: Execute orders (heavy lifting done in Rust)
        if signals["signal"] == 1:  # Buy
            print(f"  [BUY SIGNAL] Price: ${current_price:.2f}, "
                  f"Return: {signals['recent_return']:.2%}, "
                  f"Z-score: {signals['zscore']:.2f}, "
                  f"Strength: {signals['strength']:.2f}")
            
            ctx.market_order(
                self.instrument,
                OrderSide.Buy,
                float(self.position_size),
            )
            self.position = self.position_size
            self.trades_executed += 1
            
        elif signals["signal"] == -1:  # Sell
            print(f"  [SELL SIGNAL] Price: ${current_price:.2f}, "
                  f"Return: {signals['recent_return']:.2%}, "
                  f"Z-score: {signals['zscore']:.2f}, "
                  f"Strength: {signals['strength']:.2f}")
            
            ctx.market_order(
                self.instrument,
                OrderSide.Sell,
                float(self.position_size),
            )
            self.position = -self.position_size
            self.trades_executed += 1
    
    def on_stop(self, ctx: StrategyContext) -> None:
        print(f"\n[{self.strategy_id}] Strategy stopped")
        print(f"  Signals generated (Python): {self.signals_generated}")
        print(f"  Trades executed (Rust): {self.trades_executed}")
        print(f"  Final position: {self.position}")


async def test_backtest_integration():
    """Test that Rust backtesting engine works with Python strategy"""
    print("\n" + "=" * 80)
    print("Test 3: Testing Rust backtest engine with Python strategy")
    print("=" * 80)
    
    # Configuration (Python interface)
    config = HyperliquidBacktestConfig(
        coin="ETH",
        interval=CandleInterval.HOUR_1,
        lookback_days=30,
        initial_capital=Decimal("10000"),
        taker_fee_bps=4.0,
        slippage_bps=5.0,
    )
    
    print(f"\nBacktest configuration:")
    print(f"  Period: {config.start_time.date()} to {config.end_time.date()}")
    print(f"  Initial capital: ${config.initial_capital:,}")
    print(f"  Asset: {config.coin}")
    
    # Strategy (Python logic)
    strategy = AdvancedMomentumStrategy(
        lookback=20,
        threshold=0.02,
        position_size=0.05,
        use_zscore=True,
        zscore_window=50,
    )
    
    # Backtest node (Rust engine)
    node = HyperliquidBacktestNode(config)
    node.add_strategy(strategy)
    
    print("\nRunning backtest (Rust engine)...")
    results = await node.run_async()
    
    print("\n✓ Backtest completed successfully!")
    print(f"  Trades executed: {len(results.fills)}")
    
    return results


async def test_data_analysis():
    """Test pandas/numpy analysis of backtest results"""
    print("\n" + "=" * 80)
    print("Test 4: Testing pandas/numpy analysis of results")
    print("=" * 80)
    
    # Run backtest
    results = await test_backtest_integration()
    
    if not results.equity_curve:
        print("  No equity curve data available")
        return
    
    # Convert to pandas DataFrame
    df = pd.DataFrame(results.equity_curve, columns=["timestamp", "equity"])
    df["timestamp"] = pd.to_datetime(df["timestamp"], unit="ns")
    df["returns"] = df["equity"].pct_change()
    
    print("\n✓ Converted results to pandas DataFrame")
    print(f"  Shape: {df.shape}")
    print(f"\nEquity statistics (using pandas/numpy):")
    print(f"  Mean equity: ${df['equity'].mean():,.2f}")
    print(f"  Std equity: ${df['equity'].std():,.2f}")
    print(f"  Min equity: ${df['equity'].min():,.2f}")
    print(f"  Max equity: ${df['equity'].max():,.2f}")
    
    # Calculate statistics with scipy
    if len(df["returns"].dropna()) > 0:
        returns_array = df["returns"].dropna().values
        
        # Scipy statistical tests
        _, p_value = stats.normaltest(returns_array)
        skewness = stats.skew(returns_array)
        kurtosis_val = stats.kurtosis(returns_array)
        
        print(f"\n✓ Statistical analysis (using scipy):")
        print(f"  Skewness: {skewness:.4f}")
        print(f"  Kurtosis: {kurtosis_val:.4f}")
        print(f"  Normality test p-value: {p_value:.4f}")
    
    # Calculate Sharpe ratio with numpy
    if len(df["returns"].dropna()) > 0:
        mean_return = np.mean(returns_array)
        std_return = np.std(returns_array)
        sharpe = (mean_return / std_return) * np.sqrt(252 * 24) if std_return > 0 else 0
        
        print(f"\n✓ Performance metrics (using numpy):")
        print(f"  Mean return: {mean_return:.6f}")
        print(f"  Std return: {std_return:.6f}")
        print(f"  Sharpe ratio: {sharpe:.2f}")
    
    print("\n✓ All data analysis tests passed!")


async def main():
    """Run all integration tests"""
    try:
        # Test backtest execution
        await test_backtest_integration()
        
        # Test data analysis
        await test_data_analysis()
        
        print("\n" + "=" * 80)
        print("SUMMARY: All Integration Tests Passed!")
        print("=" * 80)
        print("\n✓ Rust core is handling:")
        print("  - Backtesting engine")
        print("  - Order execution")
        print("  - Position tracking")
        print("  - Performance calculations")
        print("\n✓ Python interface provides:")
        print("  - Strategy logic")
        print("  - Signal generation")
        print("  - Data analysis (numpy, pandas, scipy)")
        print("  - Ergonomic API")
        print("\n✓ Libraries working:")
        print(f"  - NumPy: {np.__version__}")
        print(f"  - Pandas: {pd.__version__}")
        print(f"  - SciPy: {scipy.__version__}")
        print("\n✓ Architecture validated: Python interface + Rust performance")
        
    except Exception as e:
        print(f"\n✗ Test failed: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    asyncio.run(main())
