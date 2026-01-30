"""
Mean Reversion Strategy Backtest

A simple mean reversion strategy that:
- Buys when price is below moving average by threshold
- Sells when price is above moving average by threshold
- Uses standard deviation to measure "extremeness"
"""
import asyncio
from decimal import Decimal
from neleus import (
    Strategy,
    StrategyContext,
    Bar,
    OrderSide,
    HyperliquidBacktestConfig,
    HyperliquidBacktestNode,
    CandleInterval,
)


class MeanReversionStrategy(Strategy):
    """
    Mean reversion strategy using moving average and standard deviation.
    
    Logic:
    - Calculate moving average over lookback period
    - Calculate standard deviation
    - Buy when price < MA - (threshold * std_dev)
    - Sell when price > MA + (threshold * std_dev)
    - Close position when price returns to MA
    """
    
    def __init__(
        self,
        lookback: int = 20,
        threshold: float = 2.0,  # Number of standard deviations
        position_size: float = 0.2,
        max_position: float = 0.5,
    ):
        super().__init__()
        self.lookback = lookback
        self.threshold = Decimal(str(threshold))
        self.position_size = Decimal(str(position_size))
        self.max_position = Decimal(str(max_position))
        
        # State
        self.closes: list[Decimal] = []
        self.position = Decimal("0")
        self.instrument = None
    
    def on_start(self, ctx: StrategyContext) -> None:
        """Called when strategy starts."""
        print(f"[{self.strategy_id}] Mean Reversion strategy started")
        print(f"  Lookback: {self.lookback}")
        print(f"  Threshold: {self.threshold} std devs")
        print(f"  Position size: {self.position_size}")
    
    def on_data(self, ctx: StrategyContext, data) -> None:
        """Process incoming market data."""
        if not isinstance(data, Bar):
            return
        
        # Track instrument
        self.instrument = data.instrument_id
        
        # Store close price
        self.closes.append(Decimal(str(data.close)))
        
        # Keep only lookback prices
        if len(self.closes) > self.lookback:
            self.closes = self.closes[-self.lookback:]
        
        # Need enough history
        if len(self.closes) < self.lookback:
            return
        
        # Calculate moving average
        ma = sum(self.closes) / Decimal(len(self.closes))
        
        # Calculate standard deviation
        variance = sum((p - ma) ** 2 for p in self.closes) / Decimal(len(self.closes))
        std_dev = variance.sqrt()
        
        current_price = self.closes[-1]
        
        # Calculate z-score (how many std devs from mean)
        if std_dev > 0:
            z_score = (current_price - ma) / std_dev
        else:
            z_score = Decimal("0")
        
        # Generate trading signals
        self._generate_signals(ctx, z_score, current_price, ma)
    
    def _generate_signals(
        self,
        ctx: StrategyContext,
        z_score: Decimal,
        current_price: Decimal,
        ma: Decimal,
    ) -> None:
        """Generate buy/sell signals based on z-score."""
        
        # Price is significantly below MA - oversold, buy
        if z_score < -self.threshold:
            target_position = min(
                self.position_size * (abs(z_score) / self.threshold),
                self.max_position,
            )
        
        # Price is significantly above MA - overbought, sell
        elif z_score > self.threshold:
            target_position = max(
                -self.position_size * (abs(z_score) / self.threshold),
                -self.max_position,
            )
        
        # Price near MA - reduce position (mean reversion happened)
        else:
            target_position = self.position * Decimal("0.5")
        
        # Calculate trade size
        trade_size = target_position - self.position
        
        # Only trade if size is significant
        min_trade = Decimal("0.001")
        if abs(trade_size) < min_trade:
            return
        
        # Execute trade
        if trade_size > 0:
            ctx.market_order(
                self.instrument,
                OrderSide.Buy,
                float(abs(trade_size)),
            )
            self.position += abs(trade_size)
        else:
            ctx.market_order(
                self.instrument,
                OrderSide.Sell,
                float(abs(trade_size)),
            )
            self.position -= abs(trade_size)
    
    def on_stop(self, ctx: StrategyContext) -> None:
        """Called when strategy stops."""
        print(f"[{self.strategy_id}] Strategy stopped")
        print(f"  Final position: {self.position}")
        print(f"  Total bars processed: {len(self.closes)}")


async def main():
    """Run mean reversion backtest."""
    
    print("=" * 60)
    print("Neleus Mean Reversion Strategy Backtest")
    print("=" * 60)
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
    
    # Create strategy
    strategy = MeanReversionStrategy(
        lookback=20,           # 20-period moving average
        threshold=2.0,         # 2 standard deviations
        position_size=0.2,
        max_position=0.5,
    )
    
    # Create backtest node
    node = HyperliquidBacktestNode(config)
    node.add_strategy(strategy)
    
    # Run backtest
    print("Running backtest...")
    print()
    
    results = await node.run_async()
    
    # Print results
    print(results.summary())
    
    # Additional info
    if results.equity_curve:
        initial = results.equity_curve[0][1]
        final = results.equity_curve[-1][1]
        print(f"Equity: ${initial:,.2f} → ${final:,.2f}")
    
    print(f"\nTotal trades: {len(results.fills)}")
    if results.fills:
        print(f"Sample trades:")
        for fill in results.fills[:5]:
            print(f"  qty={fill['quantity']:.4f} @ ${fill['price']:,.2f} (fee: ${fill['commission']:.4f})")
    
    return results


if __name__ == "__main__":
    results = asyncio.run(main())
