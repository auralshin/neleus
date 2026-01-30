"""
Hyperliquid Funding Rate Strategy
=================================

A professional funding rate arbitrage strategy for Hyperliquid perpetual futures.
Captures funding payments by taking positions when funding is extreme.

Strategy Logic:
- When funding rate is very positive (longs pay shorts): Go SHORT
- When funding rate is very negative (shorts pay longs): Go LONG
- Exit when funding normalizes or price moves against position

Features:
- Dynamic position sizing based on funding magnitude
- Maximum position limits and risk controls
- Price-based stop loss to limit adverse moves
- Tracks funding payments captured

Note: This strategy requires funding rate data which may need to be 
simulated in backtest mode. In live trading, use the WebSocket feed
to get real-time funding rates.

Usage:
    python -m examples.funding_rate_strategy
"""

import asyncio
from decimal import Decimal
from typing import Optional, List
from dataclasses import dataclass

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


@dataclass
class FundingRateEstimate:
    """Estimated funding rate based on price action"""
    rate: Decimal
    timestamp: int
    is_positive: bool


class FundingRateStrategy(Strategy):
    """
    Funding Rate Arbitrage Strategy for Hyperliquid Perps
    
    This strategy exploits extreme funding rates by taking the opposite
    position to collect funding payments. Works best in ranging markets
    where price doesn't trend strongly.
    
    Key parameters:
    - entry_threshold: Minimum funding rate magnitude to enter (annualized)
    - exit_threshold: Funding rate to exit position
    - position_size: Base position size
    - max_position: Maximum position allowed
    - stop_loss_pct: Stop loss as percentage of entry price
    """
    
    def __init__(
        self,
        # Entry/Exit thresholds (annualized funding rate)
        entry_threshold: float = 0.30,  # 30% annualized
        exit_threshold: float = 0.10,   # 10% annualized
        
        # Position sizing
        base_position_size: float = 0.1,
        max_position: float = 1.0,
        scale_with_funding: bool = True,  # Scale size with funding magnitude
        
        # Risk management
        stop_loss_pct: float = 0.03,     # 3% stop loss
        take_profit_pct: float = 0.02,    # 2% take profit (on top of funding)
        max_hold_bars: int = 72,          # Max hold time (3 days at 1h)
        
        # Funding estimation (for backtest)
        lookback_period: int = 8,  # Hours to estimate funding
        
        strategy_id: Optional[str] = None,
    ):
        super().__init__(strategy_id or "FundingRateStrategy")
        
        self.entry_threshold = Decimal(str(entry_threshold))
        self.exit_threshold = Decimal(str(exit_threshold))
        self.base_position_size = Decimal(str(base_position_size))
        self.max_position = Decimal(str(max_position))
        self.scale_with_funding = scale_with_funding
        self.stop_loss_pct = Decimal(str(stop_loss_pct))
        self.take_profit_pct = Decimal(str(take_profit_pct))
        self.max_hold_bars = max_hold_bars
        self.lookback_period = lookback_period
        
        # State
        self.instrument: Optional[InstrumentId] = None
        self.position: Decimal = Decimal("0")
        self.entry_price: Optional[Decimal] = None
        self.entry_bar: int = 0
        self.bars_since_entry: int = 0
        
        # Price history for funding estimation
        self.closes: List[Decimal] = []
        self.volumes: List[Decimal] = []
        
        # Tracking
        self.current_bar = 0
        self.estimated_funding_collected = Decimal("0")
        self.trades: List[dict] = []
        self.funding_entries = 0
        self.funding_exits = 0
        self.stopped_out = 0
        self.profit_taken = 0
        self.time_exits = 0
    
    def on_start(self, ctx: StrategyContext) -> None:
        print(f"[{self.strategy_id}] Funding Rate Strategy starting")
        print(f"  Entry threshold: {self.entry_threshold * 100:.0f}% annualized")
        print(f"  Exit threshold: {self.exit_threshold * 100:.0f}% annualized")
        print(f"  Base position: {self.base_position_size}")
        print(f"  Max position: {self.max_position}")
        print(f"  Stop loss: {self.stop_loss_pct:.1%}")
        print(f"  Take profit: {self.take_profit_pct:.1%}")
        print(f"  Max hold: {self.max_hold_bars} bars")
        
        self.instrument = InstrumentId(
            venue=Venue.Hyperliquid,
            symbol="ETH",
            instrument_type=InstrumentType.Perp,
        )
    
    def estimate_funding_rate(self) -> FundingRateEstimate:
        """
        Estimate funding rate from price momentum.
        
        In reality, you'd get this from the WebSocket or API.
        This is a proxy based on recent price action:
        - Strong upward momentum = positive funding (longs pay)
        - Strong downward momentum = negative funding (shorts pay)
        """
        if len(self.closes) < self.lookback_period:
            return FundingRateEstimate(
                rate=Decimal("0"),
                timestamp=0,
                is_positive=True
            )
        
        # Calculate momentum over lookback period
        current = self.closes[-1]
        past = self.closes[-self.lookback_period]
        momentum = (current - past) / past
        
        # Calculate volatility for scaling
        returns = []
        for i in range(-self.lookback_period + 1, 0):
            ret = (self.closes[i] - self.closes[i-1]) / self.closes[i-1]
            returns.append(ret)
        
        if returns:
            avg_return = sum(returns) / len(returns)
            variance = sum((r - avg_return) ** 2 for r in returns) / len(returns)
            volatility = variance ** Decimal("0.5")
        else:
            volatility = Decimal("0.001")
        
        # Estimate funding: momentum / volatility gives a rough z-score
        # Scale to approximate annualized funding rate
        if volatility > 0:
            funding_estimate = (momentum / volatility) * Decimal("0.1")  # Scale factor
        else:
            funding_estimate = Decimal("0")
        
        # Clip to reasonable bounds (-100% to +100% annualized)
        funding_estimate = max(min(funding_estimate, Decimal("1.0")), Decimal("-1.0"))
        
        return FundingRateEstimate(
            rate=funding_estimate,
            timestamp=self.current_bar,
            is_positive=funding_estimate > 0
        )
    
    def calculate_position_size(self, funding: FundingRateEstimate) -> Decimal:
        """Calculate position size based on funding magnitude"""
        if not self.scale_with_funding:
            return self.base_position_size
        
        # Scale size with funding magnitude
        magnitude = abs(funding.rate)
        scale = min(magnitude / self.entry_threshold, Decimal("2.0"))  # Max 2x
        
        size = self.base_position_size * scale
        return min(size, self.max_position)
    
    def check_stop_loss(self, current_price: Decimal) -> bool:
        """Check if stop loss is hit"""
        if self.entry_price is None or self.position == 0:
            return False
        
        if self.position > 0:  # Long
            stop_price = self.entry_price * (1 - self.stop_loss_pct)
            return current_price <= stop_price
        else:  # Short
            stop_price = self.entry_price * (1 + self.stop_loss_pct)
            return current_price >= stop_price
    
    def check_take_profit(self, current_price: Decimal) -> bool:
        """Check if take profit is hit"""
        if self.entry_price is None or self.position == 0:
            return False
        
        if self.position > 0:  # Long
            target = self.entry_price * (1 + self.take_profit_pct)
            return current_price >= target
        else:  # Short
            target = self.entry_price * (1 - self.take_profit_pct)
            return current_price <= target
    
    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        """Process each bar"""
        self.current_bar += 1
        current_price = Decimal(str(bar.close))
        
        # Update history
        self.closes.append(current_price)
        self.volumes.append(Decimal(str(bar.volume)))
        
        # Keep limited history
        max_history = self.lookback_period * 2
        if len(self.closes) > max_history:
            self.closes = self.closes[-max_history:]
            self.volumes = self.volumes[-max_history:]
        
        # Get funding estimate
        funding = self.estimate_funding_rate()
        
        # Update bars since entry
        if self.position != 0:
            self.bars_since_entry += 1
        
        # Check exits first
        if self.position != 0:
            should_exit = False
            exit_reason = ""
            
            # Stop loss
            if self.check_stop_loss(current_price):
                should_exit = True
                exit_reason = "STOP_LOSS"
                self.stopped_out += 1
            
            # Take profit
            elif self.check_take_profit(current_price):
                should_exit = True
                exit_reason = "TAKE_PROFIT"
                self.profit_taken += 1
            
            # Funding normalized
            elif abs(funding.rate) < self.exit_threshold:
                # Only exit if funding has flipped or is very low
                if self.position > 0 and not funding.is_positive:
                    should_exit = True
                    exit_reason = "FUNDING_FLIP"
                    self.funding_exits += 1
                elif self.position < 0 and funding.is_positive:
                    should_exit = True
                    exit_reason = "FUNDING_FLIP"
                    self.funding_exits += 1
            
            # Time limit
            elif self.bars_since_entry >= self.max_hold_bars:
                should_exit = True
                exit_reason = "TIME_LIMIT"
                self.time_exits += 1
            
            if should_exit:
                self._close_position(ctx, current_price, exit_reason)
        
        # Check entries (only if flat)
        if self.position == 0 and abs(funding.rate) >= self.entry_threshold:
            size = self.calculate_position_size(funding)
            
            if funding.is_positive:
                # Positive funding = go SHORT to receive funding
                self._open_position(ctx, OrderSide.Sell, size, current_price, funding)
            else:
                # Negative funding = go LONG to receive funding
                self._open_position(ctx, OrderSide.Buy, size, current_price, funding)
    
    def _open_position(
        self, 
        ctx: StrategyContext, 
        side: OrderSide, 
        size: Decimal, 
        price: Decimal,
        funding: FundingRateEstimate
    ) -> None:
        """Open a new position"""
        self.position = size if side == OrderSide.Buy else -size
        self.entry_price = price
        self.bars_since_entry = 0
        self.funding_entries += 1
        
        ctx.market_order(self.instrument, side, float(size), reduce_only=False)
        
        direction = "LONG" if side == OrderSide.Buy else "SHORT"
        print(f"  [{direction}] Entry @ ${price:.2f}, Size: {size:.4f}")
        print(f"    Funding rate: {funding.rate * 100:.1f}% annualized")
    
    def _close_position(
        self, 
        ctx: StrategyContext, 
        price: Decimal, 
        reason: str
    ) -> None:
        """Close the current position"""
        size = abs(self.position)
        side = OrderSide.Sell if self.position > 0 else OrderSide.Buy
        
        # Calculate P&L
        if self.position > 0:
            pnl_pct = (price - self.entry_price) / self.entry_price
        else:
            pnl_pct = (self.entry_price - price) / self.entry_price
        
        ctx.market_order(self.instrument, side, float(size), reduce_only=True)
        
        print(f"  [CLOSE] {reason} @ ${price:.2f}")
        print(f"    Entry: ${self.entry_price:.2f}, P&L: {pnl_pct * 100:.2f}%")
        print(f"    Held for {self.bars_since_entry} bars")
        
        self.trades.append({
            "entry_price": float(self.entry_price),
            "exit_price": float(price),
            "pnl_pct": float(pnl_pct),
            "bars_held": self.bars_since_entry,
            "reason": reason,
        })
        
        self.position = Decimal("0")
        self.entry_price = None
        self.bars_since_entry = 0
    
    def on_stop(self, ctx: StrategyContext) -> None:
        """Cleanup and print statistics"""
        if self.position != 0:
            current_price = self.closes[-1] if self.closes else Decimal("0")
            self._close_position(ctx, current_price, "STRATEGY_STOP")
        
        print(f"\n[{self.strategy_id}] Funding Rate Strategy stopped")
        print(f"  Total trades: {len(self.trades)}")
        print(f"  Entry reasons: {self.funding_entries}")
        print(f"  Exit reasons:")
        print(f"    Funding flip: {self.funding_exits}")
        print(f"    Stop loss: {self.stopped_out}")
        print(f"    Take profit: {self.profit_taken}")
        print(f"    Time limit: {self.time_exits}")
        
        if self.trades:
            wins = [t for t in self.trades if t["pnl_pct"] > 0]
            losses = [t for t in self.trades if t["pnl_pct"] <= 0]
            avg_pnl = sum(t["pnl_pct"] for t in self.trades) / len(self.trades)
            print(f"  Win rate: {len(wins)/len(self.trades)*100:.1f}%")
            print(f"  Avg P&L per trade: {avg_pnl*100:.2f}%")


async def main():
    """Run the Funding Rate Strategy backtest"""
    from neleus.types import HyperliquidClient
    
    print("=" * 60)
    print("HYPERLIQUID FUNDING RATE STRATEGY BACKTEST")
    print("=" * 60)
    
    # Configuration
    coin = "BTC"
    interval = CandleInterval.HOUR_1
    lookback_days = 30
    initial_capital = 10000.0
    
    strategy = FundingRateStrategy(
        entry_threshold=0.25,  # 25% annualized funding
        exit_threshold=0.10,   # 10% annualized
        base_position_size=0.05,
        max_position=0.2,
        scale_with_funding=True,
        stop_loss_pct=0.025,
        take_profit_pct=0.015,
        max_hold_bars=48,
        lookback_period=8,
    )
    
    config = HyperliquidBacktestConfig(
        initial_capital=initial_capital,
        coin=coin,
        interval=interval,
        lookback_days=lookback_days,
        maker_fee_bps=0.0,
        taker_fee_bps=5.0,
        slippage_bps=1.0,
    )
    
    node = HyperliquidBacktestNode(config)
    node.add_strategy(strategy)
    result = await node.run_async()
    
    print("\n" + "=" * 60)
    print("BACKTEST RESULTS")
    print("=" * 60)
    print(f"Initial Capital: ${initial_capital:,.2f}")
    print(f"Total Return:    {result.metrics.total_return * 100:.2f}%")
    print(f"Sharpe Ratio:    {result.metrics.sharpe_ratio:.2f}")
    print(f"Max Drawdown:    {result.metrics.max_drawdown * 100:.2f}%")
    print(f"Total Trades:    {result.metrics.total_trades}")
    print(f"Win Rate:        {result.metrics.win_rate * 100:.1f}%")
    print(f"Profit Factor:   {result.metrics.profit_factor:.2f}")
    print("=" * 60)


if __name__ == "__main__":
    asyncio.run(main())
