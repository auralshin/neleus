"""
Statistical Momentum Strategy with SciPy/NumPy

A momentum strategy that uses:
- NumPy for fast vectorized calculations
- SciPy for statistical tests (normality, stationarity)
- Pandas for rolling window calculations
- Z-score normalization for signal strength

Strategy Logic:
- Calculate returns and z-scores using NumPy
- Use rolling volatility for position sizing
- Apply statistical filters to avoid false signals
- Dynamic position sizing based on momentum strength
"""
import asyncio
from decimal import Decimal
import numpy as np
import pandas as pd
import scipy
from scipy import stats
from typing import Optional

from neleus import (
    Strategy,
    StrategyContext,
    Bar,
    OrderSide,
    HyperliquidBacktestConfig,
    HyperliquidBacktestNode,
    CandleInterval,
)


class StatisticalMomentumStrategy(Strategy):
    """
    Advanced momentum strategy using scientific computing libraries.
    
    Features:
    - NumPy for fast vectorized calculations
    - Pandas for rolling statistics
    - SciPy for statistical validation
    - Z-score based signal generation
    - Volatility-adjusted position sizing
    """
    
    def __init__(
        self,
        lookback: int = 20,
        signal_threshold: float = 1.5,  # Z-score threshold
        use_zscore_normalization: bool = True,
        check_stationarity: bool = True,
        volatility_lookback: int = 20,
        max_position: float = 0.5,
    ):
        super().__init__()
        self.lookback = lookback
        self.signal_threshold = signal_threshold
        self.use_zscore_normalization = use_zscore_normalization
        self.check_stationarity = check_stationarity
        self.volatility_lookback = volatility_lookback
        self.max_position = max_position
        
        # Data storage
        self.prices = []
        self.returns = []
        self.timestamps = []
        
        # Position tracking
        self.position = 0.0
        self.instrument = None
        
        # Statistics
        self.signals_generated = 0
        self.trades_executed = 0
    
    def on_start(self, ctx: StrategyContext) -> None:
        """Called when strategy starts."""
        print(f"[{self.strategy_id}] Starting advanced strategy")
        print(f"  Python layer: Signal generation with scipy/numpy")
        print(f"  Rust layer: Backtesting, order execution, risk management")
        print(f"  Lookback: {self.lookback} bars")
        print(f"  Z-score normalization: {self.use_zscore_normalization}")
    
    def on_data(self, ctx: StrategyContext, data) -> None:
        """Process incoming market data with NumPy/SciPy."""
        if not isinstance(data, Bar):
            return
        
        self.instrument = data.instrument_id
        
        # Store data
        self.prices.append(float(data.close))
        self.timestamps.append(data.timestamp_ns)
        
        # Need enough history
        if len(self.prices) < self.lookback + 1:
            return
        
        # Keep only necessary history (efficient memory usage)
        max_history = max(self.lookback * 2, 100)
        if len(self.prices) > max_history:
            self.prices = self.prices[-max_history:]
            self.timestamps = self.timestamps[-max_history:]
        
        # Calculate returns using NumPy (vectorized, fast)
        prices_array = np.array(self.prices)
        returns = np.diff(np.log(prices_array))  # Log returns
        
        if len(returns) < self.lookback:
            return
        
        # Get recent returns for signal generation
        recent_returns = returns[-self.lookback:]
        
        # Calculate momentum signal
        signal = self._calculate_signal(recent_returns, prices_array)
        
        if signal is not None:
            self._execute_signal(ctx, signal, prices_array[-1], returns)
    
    def _calculate_signal(
        self,
        returns: np.ndarray,
        prices: np.ndarray,
    ) -> Optional[float]:
        """
        Calculate trading signal using statistical methods.
        
        Returns:
            Signal strength between -1 and 1, or None if no signal
        """
        # 1. Calculate cumulative return (momentum)
        cumulative_return = np.sum(returns)
        
        # 2. Calculate mean and std of recent returns
        mean_return = np.mean(returns)
        std_return = np.std(returns, ddof=1)  # Sample std
        
        if std_return < 1e-8:  # Avoid division by near-zero
            return None
        
        # 3. Calculate z-score of mean return (is trend significant?)
        if self.use_zscore_normalization:
            # Test if mean return is significantly different from zero
            z_score = mean_return / (std_return / np.sqrt(len(returns)))
        else:
            # Use raw cumulative return
            z_score = cumulative_return / std_return
        
        # 4. Check for statistical significance using t-test
        if self.check_stationarity and len(returns) > 10:
            t_stat, p_value = stats.ttest_1samp(returns, 0)
            
            # Only trade if returns are significantly different from zero
            if p_value > 0.10:  # Not significant at 10% level
                return None
        
        # 5. Normalize signal to [-1, 1] range
        # Use tanh for smooth scaling - more aggressive with lower threshold
        signal_strength = float(np.tanh(z_score / (self.signal_threshold * 0.5)))
        
        # 6. Filter weak signals
        if abs(signal_strength) < 0.1:  # Lower threshold for entry
            return None
        
        self.signals_generated += 1
        
        # Log signal details
        current_price = prices[-1]
        pct_return = cumulative_return * 100
        print(f"  [{'BUY' if signal_strength > 0 else 'SELL'} SIGNAL] "
              f"Price: ${current_price:.2f}, "
              f"Mean return: {mean_return*100:.4f}%, "
              f"Cum return: {pct_return:.2f}%, "
              f"Z-score: {z_score:.2f}, "
              f"Strength: {signal_strength:.2f}")
        
        return signal_strength
    
    def _execute_signal(
        self,
        ctx: StrategyContext,
        signal: float,
        current_price: float,
        all_returns: np.ndarray,
    ) -> None:
        """
        Execute trades based on signal with volatility-adjusted sizing.
        """
        # Calculate position size using volatility scaling
        position_size = self._calculate_position_size(signal, all_returns[-self.volatility_lookback:])
        
        # Calculate trade needed
        target_position = position_size
        trade_size = target_position - self.position
        
        # Minimum trade threshold
        min_trade = 0.01
        if abs(trade_size) < min_trade:
            return
        
        # Execute trade
        if trade_size > 0:
            ctx.market_order(
                self.instrument,
                OrderSide.Buy,
                abs(trade_size),
            )
        else:
            ctx.market_order(
                self.instrument,
                OrderSide.Sell,
                abs(trade_size),
            )
        
        self.position = target_position
        self.trades_executed += 1
    
    def _calculate_position_size(
        self,
        signal: float,
        recent_returns: np.ndarray,
    ) -> float:
        """
        Calculate position size using volatility scaling.
        
        Uses inverse volatility scaling:
        - Higher volatility → Smaller position
        - Lower volatility → Larger position
        """
        # Calculate realized volatility (annualized)
        volatility = np.std(recent_returns, ddof=1) * np.sqrt(252)
        
        if volatility < 0.01:  # Avoid division by small numbers
            volatility = 0.01
        
        # Target volatility (e.g., 20% annualized)
        target_volatility = 0.20
        
        # Scale position inversely to volatility
        vol_scalar = target_volatility / volatility
        vol_scalar = np.clip(vol_scalar, 0.2, 2.0)  # Limit scaling
        
        # Base position scaled by signal strength and volatility
        base_position = 0.1  # 10% base
        position = base_position * signal * vol_scalar
        
        # Cap at max position
        position = np.clip(position, -self.max_position, self.max_position)
        
        return float(position)
    
    def on_stop(self, ctx: StrategyContext) -> None:
        """Called when strategy stops."""
        print(f"[{self.strategy_id}] Strategy stopped")
        print(f"  Final position: {self.position}")
        print(f"  Total bars processed: {len(self.prices)}")
        print()
        print(f"  Performance Statistics:")
        print(f"    Total signals generated: {self.signals_generated}")
        print(f"    Trades executed: {self.trades_executed}")
        if self.signals_generated > 0:
            print(f"    Signal-to-trade ratio: {self.trades_executed / self.signals_generated:.2%}")


class PandasMomentumStrategy(Strategy):
    """
    Alternative implementation using Pandas for rolling calculations.
    
    Demonstrates:
    - Pandas DataFrame for data management
    - Rolling windows for indicators
    - Multiple timeframe analysis
    """
    
    def __init__(
        self,
        fast_period: int = 10,
        slow_period: int = 30,
        signal_period: int = 5,
        position_size: float = 0.15,
    ):
        super().__init__()
        self.fast_period = fast_period
        self.slow_period = slow_period
        self.signal_period = signal_period
        self.position_size = position_size
        
        # Use Pandas DataFrame for data management
        self.df = pd.DataFrame(columns=['timestamp', 'price', 'returns'])
        self.position = 0.0
        self.instrument = None
    
    def on_start(self, ctx: StrategyContext) -> None:
        print(f"[{self.strategy_id}] Pandas-based momentum strategy started")
        print(f"  Fast MA: {self.fast_period}, Slow MA: {self.slow_period}")
    
    def on_data(self, ctx: StrategyContext, data) -> None:
        if not isinstance(data, Bar):
            return
        
        self.instrument = data.instrument_id
        
        # Add new data to DataFrame
        new_row = pd.DataFrame([{
            'timestamp': data.timestamp_ns,
            'price': float(data.close),
            'returns': None,  # Will calculate
        }])
        self.df = pd.concat([self.df, new_row], ignore_index=True)
        
        # Calculate returns
        self.df['returns'] = self.df['price'].pct_change()
        
        # Keep only necessary history
        max_history = self.slow_period * 3
        if len(self.df) > max_history:
            self.df = self.df.iloc[-max_history:].reset_index(drop=True)
        
        # Need enough history
        if len(self.df) < self.slow_period:
            return
        
        # Calculate indicators using Pandas rolling windows
        self.df['fast_ma'] = self.df['price'].rolling(window=self.fast_period).mean()
        self.df['slow_ma'] = self.df['price'].rolling(window=self.slow_period).mean()
        self.df['momentum'] = self.df['fast_ma'] - self.df['slow_ma']
        self.df['signal_line'] = self.df['momentum'].rolling(window=self.signal_period).mean()
        
        # Get latest values
        latest = self.df.iloc[-1]
        
        if pd.isna(latest['signal_line']):
            return
        
        # Generate signals
        momentum = latest['momentum']
        signal_line = latest['signal_line']
        
        # Crossover strategy
        prev = self.df.iloc[-2]
        
        # Buy signal: momentum crosses above signal line
        if momentum > signal_line and prev['momentum'] <= prev['signal_line']:
            if self.position <= 0:
                trade_size = self.position_size - self.position
                ctx.market_order(self.instrument, OrderSide.Buy, abs(trade_size))
                self.position = self.position_size
        
        # Sell signal: momentum crosses below signal line
        elif momentum < signal_line and prev['momentum'] >= prev['signal_line']:
            if self.position >= 0:
                trade_size = -self.position_size - self.position
                ctx.market_order(self.instrument, OrderSide.Sell, abs(trade_size))
                self.position = -self.position_size
    
    def on_stop(self, ctx: StrategyContext) -> None:
        print(f"[{self.strategy_id}] Strategy stopped")
        print(f"  DataFrame shape: {self.df.shape}")
        print(f"  Final position: {self.position}")


async def main():
    """Run statistical momentum backtest."""
    
    print("=" * 80)
    print("Neleus Statistical Momentum Strategy (NumPy/SciPy/Pandas)")
    print("=" * 80)
    print()
    
    # Backtest configuration
    config = HyperliquidBacktestConfig(
        coin="ETH",
        interval=CandleInterval.HOUR_1,
        lookback_days=30,
        initial_capital=Decimal("10000"),
        taker_fee_bps=4.0,
        slippage_bps=5.0,
    )
    
    print(f"Configuration:")
    print(f"  Asset: {config.coin}")
    print(f"  Interval: {config.interval.value}")
    print(f"  Period: {config.start_time.date()} to {config.end_time.date()}")
    print(f"  Initial Capital: ${config.initial_capital:,}")
    print()
    
    # Choose strategy
    print("Select strategy:")
    print("  1. Statistical Momentum (NumPy/SciPy) - Advanced")
    print("  2. Pandas Momentum (Pandas) - Rolling windows")
    print()
    
    # For this example, we'll use the advanced strategy
    strategy = StatisticalMomentumStrategy(
        lookback=20,
        signal_threshold=1.0,  # Lower threshold = more signals
        use_zscore_normalization=True,
        check_stationarity=False,  # Disable for faster backtest
        volatility_lookback=20,
        max_position=0.5,
    )
    
    # Uncomment to use Pandas strategy instead:
    # strategy = PandasMomentumStrategy(
    #     fast_period=10,
    #     slow_period=30,
    #     signal_period=5,
    #     position_size=0.15,
    # )
    
    # Create backtest node
    node = HyperliquidBacktestNode(config)
    node.add_strategy(strategy)
    
    # Run backtest
    print("Running backtest...")
    print()
    
    results = await node.run_async()
    
    # Print results
    print()
    print("=" * 80)
    print("BACKTEST RESULTS")
    print("=" * 80)
    print()
    print(results.summary())
    
    # Additional analysis using NumPy
    if results.equity_curve and len(results.equity_curve) > 1:
        equity_series = np.array([eq[1] for eq in results.equity_curve])
        
        if len(equity_series) > 1:
            returns = np.diff(equity_series) / equity_series[:-1]
            
            print(f"\nEquity: ${equity_series[0]:,.2f} → ${equity_series[-1]:,.2f} "
                  f"({(equity_series[-1] / equity_series[0] - 1) * 100:+.2f}%)")
            
            if len(returns) > 1 and np.std(returns) > 0:
                print(f"\nAdvanced Statistics (NumPy):")
                print(f"  Mean return: {np.mean(returns):.6f}")
                print(f"  Std return: {np.std(returns, ddof=1):.6f}")
                print(f"  Sharpe ratio: {np.mean(returns) / np.std(returns, ddof=1) * np.sqrt(len(returns)):.2f}")
                
                # Skewness and Kurtosis (SciPy)
                if len(returns) > 3:
                    skew = stats.skew(returns)
                    kurt = stats.kurtosis(returns)
                    print(f"  Skewness: {skew:.4f}")
                    print(f"  Kurtosis: {kurt:.4f}")
                    
                    # Normality test
                    if len(returns) > 7:
                        _, p_value = stats.normaltest(returns)
                        print(f"  Normality test p-value: {p_value:.4f}")
    
    print(f"\nTotal trades: {len(results.fills)}")
    if results.fills:
        print(f"\nFirst 5 trades:")
        for i, fill in enumerate(results.fills[:5], 1):
            print(f"  {i}. {fill['side']} qty={fill['quantity']:.4f} @ "
                  f"${fill['price']:,.2f} (fee: ${fill['commission']:.4f})")
    
    return results


if __name__ == "__main__":
    # Verify scientific libraries are available
    print("Checking scientific computing libraries...")
    print(f"  ✓ NumPy {np.__version__} imported")
    print(f"  ✓ Pandas {pd.__version__} imported")
    print(f"  ✓ SciPy {scipy.__version__} imported")
    print()
    
    results = asyncio.run(main())
