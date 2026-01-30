"""
Hyperliquid Breakout Trading Strategy
=====================================

A professional breakout trading strategy for Hyperliquid perpetual futures.
Identifies and trades price breakouts from consolidation ranges.

Strategy Logic:
- Detect consolidation (low volatility) periods
- Wait for price to break above resistance -> GO LONG
- Wait for price to break below support -> GO SHORT
- Use ATR-based stops and volatility expansion targets

Features:
- Donchian channel-based breakout detection
- Volume confirmation for breakouts
- Dynamic ATR-based stop losses and targets
- Proper long/short position management
- Pyramiding disabled by default

Usage:
    python -m examples.breakout_strategy
"""

import asyncio
from decimal import Decimal
from typing import Optional, List
from dataclasses import dataclass
from enum import Enum

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


class BreakoutDirection(Enum):
    LONG = "long"
    SHORT = "short"
    NONE = "none"


@dataclass
class ConsolidationRange:
    """Detected consolidation range"""
    high: Decimal
    low: Decimal
    width_pct: Decimal
    bars: int
    is_valid: bool


class BreakoutStrategy(Strategy):
    """
    Breakout Trading Strategy for Hyperliquid Perps
    
    This strategy identifies consolidation zones and trades breakouts
    in the direction of the move. Uses Donchian channels for 
    range detection.
    
    Key parameters:
    - channel_period: Lookback for Donchian channel
    - consolidation_threshold: Max range width to consider consolidation
    - volume_multiplier: Required volume increase for confirmation
    - atr_stop_multiplier: ATR multiplier for stop loss
    - atr_target_multiplier: ATR multiplier for take profit
    """
    
    def __init__(
        self,
        # Channel settings
        channel_period: int = 20,
        consolidation_threshold: float = 0.04,  # 4% max range width
        min_consolidation_bars: int = 10,
        
        # Entry confirmation
        breakout_buffer_pct: float = 0.002,  # 0.2% beyond channel
        volume_multiplier: float = 1.5,      # 1.5x average volume
        
        # Position sizing
        position_size: float = 0.1,
        max_position: float = 0.3,
        
        # Risk management
        atr_period: int = 14,
        atr_stop_multiplier: float = 2.0,
        atr_target_multiplier: float = 3.0,
        max_hold_bars: int = 48,
        
        # Trend filter
        use_trend_filter: bool = True,
        trend_ema_period: int = 50,
        
        strategy_id: Optional[str] = None,
    ):
        super().__init__(strategy_id or "BreakoutStrategy")
        
        self.channel_period = channel_period
        self.consolidation_threshold = Decimal(str(consolidation_threshold))
        self.min_consolidation_bars = min_consolidation_bars
        self.breakout_buffer = Decimal(str(breakout_buffer_pct))
        self.volume_multiplier = Decimal(str(volume_multiplier))
        self.position_size = Decimal(str(position_size))
        self.max_position = Decimal(str(max_position))
        self.atr_period = atr_period
        self.atr_stop_mult = Decimal(str(atr_stop_multiplier))
        self.atr_target_mult = Decimal(str(atr_target_multiplier))
        self.max_hold_bars = max_hold_bars
        self.use_trend_filter = use_trend_filter
        self.trend_ema_period = trend_ema_period
        
        # State
        self.instrument: Optional[InstrumentId] = None
        self.position: Decimal = Decimal("0")
        self.entry_price: Optional[Decimal] = None
        self.stop_loss: Optional[Decimal] = None
        self.take_profit: Optional[Decimal] = None
        self.bars_since_entry: int = 0
        
        # Price history
        self.highs: List[Decimal] = []
        self.lows: List[Decimal] = []
        self.closes: List[Decimal] = []
        self.volumes: List[Decimal] = []
        
        # EMA for trend filter
        self.ema: Optional[Decimal] = None
        self.ema_multiplier: Decimal = Decimal(str(2 / (trend_ema_period + 1)))
        
        # Tracking
        self.current_bar = 0
        self.total_trades = 0
        self.winning_trades = 0
        self.losing_trades = 0
        self.long_trades = 0
        self.short_trades = 0
        self.stopped_out = 0
        self.profit_taken = 0
    
    def on_start(self, ctx: StrategyContext) -> None:
        print(f"[{self.strategy_id}] Breakout Strategy starting")
        print(f"  Channel period: {self.channel_period}")
        print(f"  Consolidation threshold: {self.consolidation_threshold:.1%}")
        print(f"  Min consolidation bars: {self.min_consolidation_bars}")
        print(f"  Volume multiplier: {self.volume_multiplier}x")
        print(f"  ATR stop: {self.atr_stop_mult}x, target: {self.atr_target_mult}x")
        if self.use_trend_filter:
            print(f"  Trend filter: EMA({self.trend_ema_period})")
        
        self.instrument = InstrumentId(
            venue=Venue.Hyperliquid,
            symbol="ETH",
            instrument_type=InstrumentType.Perp,
        )
    
    def calculate_atr(self) -> Decimal:
        """Calculate Average True Range"""
        if len(self.highs) < self.atr_period + 1:
            return Decimal("0")
        
        true_ranges = []
        for i in range(-self.atr_period, 0):
            high = self.highs[i]
            low = self.lows[i]
            prev_close = self.closes[i - 1]
            
            tr = max(
                high - low,
                abs(high - prev_close),
                abs(low - prev_close)
            )
            true_ranges.append(tr)
        
        return sum(true_ranges) / len(true_ranges)
    
    def update_ema(self, price: Decimal) -> None:
        """Update exponential moving average"""
        if self.ema is None:
            self.ema = price
        else:
            self.ema = (price * self.ema_multiplier) + (self.ema * (1 - self.ema_multiplier))
    
    def get_donchian_channel(self) -> tuple:
        """Calculate Donchian channel (highest high, lowest low) - excluding current bar"""
        if len(self.highs) < self.channel_period + 1:
            return None, None
        
        # Use previous N bars, NOT including the current bar
        channel_highs = self.highs[-(self.channel_period + 1):-1]
        channel_lows = self.lows[-(self.channel_period + 1):-1]
        
        return max(channel_highs), min(channel_lows)
    
    def detect_consolidation(self) -> ConsolidationRange:
        """Detect if we're in a consolidation range"""
        channel_high, channel_low = self.get_donchian_channel()
        
        if channel_high is None or channel_low is None:
            return ConsolidationRange(
                high=Decimal("0"),
                low=Decimal("0"),
                width_pct=Decimal("1"),
                bars=0,
                is_valid=False
            )
        
        mid_price = (channel_high + channel_low) / 2
        width_pct = (channel_high - channel_low) / mid_price if mid_price > 0 else Decimal("1")
        
        is_consolidating = width_pct <= self.consolidation_threshold
        
        return ConsolidationRange(
            high=channel_high,
            low=channel_low,
            width_pct=width_pct,
            bars=self.channel_period,
            is_valid=is_consolidating
        )
    
    def check_volume_confirmation(self) -> bool:
        """Check if current volume confirms the breakout"""
        if len(self.volumes) < self.channel_period:
            return True  # Not enough data, allow trade
        
        avg_volume = sum(self.volumes[-self.channel_period:-1]) / (self.channel_period - 1)
        current_volume = self.volumes[-1]
        
        return current_volume >= avg_volume * self.volume_multiplier
    
    def get_trend_bias(self, price: Decimal) -> BreakoutDirection:
        """Get trend bias from EMA filter"""
        if not self.use_trend_filter or self.ema is None:
            return BreakoutDirection.NONE  # No filter
        
        if price > self.ema:
            return BreakoutDirection.LONG
        else:
            return BreakoutDirection.SHORT
    
    def detect_breakout(self, bar: Bar, consolidation: ConsolidationRange) -> BreakoutDirection:
        """Detect if a breakout has occurred"""
        if not consolidation.is_valid:
            return BreakoutDirection.NONE
        
        current_close = Decimal(str(bar.close))
        current_high = Decimal(str(bar.high))
        current_low = Decimal(str(bar.low))
        
        # Calculate breakout levels with buffer
        upper_breakout = consolidation.high * (1 + self.breakout_buffer)
        lower_breakout = consolidation.low * (1 - self.breakout_buffer)
        
        # Long breakout: high pierces upper level AND close above channel high
        if current_high >= upper_breakout and current_close > consolidation.high:
            return BreakoutDirection.LONG
        
        # Short breakout: low pierces lower level AND close below channel low
        if current_low <= lower_breakout and current_close < consolidation.low:
            return BreakoutDirection.SHORT
        
        return BreakoutDirection.NONE
    
    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        """Process each bar"""
        self.current_bar += 1
        
        current_price = Decimal(str(bar.close))
        
        # Update history
        self.highs.append(Decimal(str(bar.high)))
        self.lows.append(Decimal(str(bar.low)))
        self.closes.append(current_price)
        self.volumes.append(Decimal(str(bar.volume)))
        
        # Update EMA
        self.update_ema(current_price)
        
        # Keep limited history
        max_history = max(self.channel_period, self.atr_period, self.trend_ema_period) + 10
        if len(self.highs) > max_history:
            self.highs = self.highs[-max_history:]
            self.lows = self.lows[-max_history:]
            self.closes = self.closes[-max_history:]
            self.volumes = self.volumes[-max_history:]
        
        # Track position duration
        if self.position != 0:
            self.bars_since_entry += 1
        
        # Check exits first
        if self.position != 0:
            self._check_exits(ctx, bar)
        
        # Check entries (only if flat)
        if self.position == 0 and len(self.closes) >= self.channel_period:
            self._check_entry(ctx, bar)
    
    def _check_exits(self, ctx: StrategyContext, bar: Bar) -> None:
        """Check exit conditions for current position"""
        current_price = Decimal(str(bar.close))
        current_high = Decimal(str(bar.high))
        current_low = Decimal(str(bar.low))
        
        should_exit = False
        exit_reason = ""
        exit_price = current_price
        
        if self.position > 0:  # Long position
            # Check stop loss (hit intrabar)
            if current_low <= self.stop_loss:
                should_exit = True
                exit_reason = "STOP_LOSS"
                exit_price = self.stop_loss
                self.stopped_out += 1
            # Check take profit (hit intrabar)
            elif current_high >= self.take_profit:
                should_exit = True
                exit_reason = "TAKE_PROFIT"
                exit_price = self.take_profit
                self.profit_taken += 1
        
        elif self.position < 0:  # Short position
            # Check stop loss
            if current_high >= self.stop_loss:
                should_exit = True
                exit_reason = "STOP_LOSS"
                exit_price = self.stop_loss
                self.stopped_out += 1
            # Check take profit
            elif current_low <= self.take_profit:
                should_exit = True
                exit_reason = "TAKE_PROFIT"
                exit_price = self.take_profit
                self.profit_taken += 1
        
        # Time-based exit
        if not should_exit and self.bars_since_entry >= self.max_hold_bars:
            should_exit = True
            exit_reason = "TIME_LIMIT"
            exit_price = current_price
        
        if should_exit:
            self._close_position(ctx, exit_price, exit_reason)
    
    def _check_entry(self, ctx: StrategyContext, bar: Bar) -> None:
        """Check for entry opportunities"""
        consolidation = self.detect_consolidation()
        
        if not consolidation.is_valid:
            return
        
        breakout = self.detect_breakout(bar, consolidation)
        
        if breakout == BreakoutDirection.NONE:
            return
        
        # Check volume confirmation
        if not self.check_volume_confirmation():
            return
        
        # Check trend filter
        current_price = Decimal(str(bar.close))
        trend_bias = self.get_trend_bias(current_price)
        
        if self.use_trend_filter and trend_bias != BreakoutDirection.NONE:
            if trend_bias != breakout:
                return  # Breakout against trend, skip
        
        # Calculate ATR for stops
        atr = self.calculate_atr()
        if atr == 0:
            atr = current_price * Decimal("0.02")  # Default 2%
        
        # Execute entry
        if breakout == BreakoutDirection.LONG:
            self._open_long(ctx, current_price, atr)
        elif breakout == BreakoutDirection.SHORT:
            self._open_short(ctx, current_price, atr)
    
    def _open_long(self, ctx: StrategyContext, price: Decimal, atr: Decimal) -> None:
        """Open a long position"""
        size = min(self.position_size, self.max_position)
        
        self.position = size
        self.entry_price = price
        self.stop_loss = price - (atr * self.atr_stop_mult)
        self.take_profit = price + (atr * self.atr_target_mult)
        self.bars_since_entry = 0
        self.long_trades += 1
        
        ctx.market_order(self.instrument, OrderSide.Buy, float(size), reduce_only=False)
        
        print(f"  [LONG BREAKOUT] Entry @ ${price:.2f}, Size: {size:.4f}")
        print(f"    Stop: ${self.stop_loss:.2f}, Target: ${self.take_profit:.2f}")
        print(f"    Risk: {(price - self.stop_loss) / price * 100:.2f}%")
    
    def _open_short(self, ctx: StrategyContext, price: Decimal, atr: Decimal) -> None:
        """Open a short position"""
        size = min(self.position_size, self.max_position)
        
        self.position = -size
        self.entry_price = price
        self.stop_loss = price + (atr * self.atr_stop_mult)
        self.take_profit = price - (atr * self.atr_target_mult)
        self.bars_since_entry = 0
        self.short_trades += 1
        
        ctx.market_order(self.instrument, OrderSide.Sell, float(size), reduce_only=False)
        
        print(f"  [SHORT BREAKOUT] Entry @ ${price:.2f}, Size: {size:.4f}")
        print(f"    Stop: ${self.stop_loss:.2f}, Target: ${self.take_profit:.2f}")
        print(f"    Risk: {(self.stop_loss - price) / price * 100:.2f}%")
    
    def _close_position(self, ctx: StrategyContext, price: Decimal, reason: str) -> None:
        """Close the current position"""
        size = abs(self.position)
        side = OrderSide.Sell if self.position > 0 else OrderSide.Buy
        
        # Calculate P&L
        if self.position > 0:
            pnl_pct = (price - self.entry_price) / self.entry_price
        else:
            pnl_pct = (self.entry_price - price) / self.entry_price
        
        self.total_trades += 1
        if pnl_pct > 0:
            self.winning_trades += 1
        else:
            self.losing_trades += 1
        
        ctx.market_order(self.instrument, side, float(size), reduce_only=True)
        
        print(f"  [CLOSE] {reason} @ ${price:.2f}")
        print(f"    Entry: ${self.entry_price:.2f}, P&L: {pnl_pct * 100:.2f}%")
        print(f"    Bars held: {self.bars_since_entry}")
        
        self.position = Decimal("0")
        self.entry_price = None
        self.stop_loss = None
        self.take_profit = None
        self.bars_since_entry = 0
    
    def on_stop(self, ctx: StrategyContext) -> None:
        """Cleanup and print statistics"""
        if self.position != 0:
            current_price = self.closes[-1] if self.closes else Decimal("0")
            self._close_position(ctx, current_price, "STRATEGY_STOP")
        
        print(f"\n[{self.strategy_id}] Breakout Strategy stopped")
        print(f"  Total trades: {self.total_trades}")
        print(f"    Long trades: {self.long_trades}")
        print(f"    Short trades: {self.short_trades}")
        print(f"  Winning trades: {self.winning_trades}")
        print(f"  Losing trades: {self.losing_trades}")
        print(f"  Stopped out: {self.stopped_out}")
        print(f"  Profit taken: {self.profit_taken}")
        if self.total_trades > 0:
            print(f"  Win rate: {self.winning_trades/self.total_trades*100:.1f}%")


async def main():
    """Run the Breakout Strategy backtest"""
    print("=" * 60)
    print("HYPERLIQUID BREAKOUT STRATEGY BACKTEST")
    print("=" * 60)
    
    # Configuration
    coin = "ETH"
    interval = CandleInterval.HOUR_1
    lookback_days = 60
    initial_capital = 10000.0
    
    strategy = BreakoutStrategy(
        channel_period=20,
        consolidation_threshold=0.15,  # 15% range - more relaxed for volatile crypto
        min_consolidation_bars=5,
        breakout_buffer_pct=0.001,
        volume_multiplier=1.0,  # Disable volume filter
        position_size=0.1,
        max_position=0.3,
        atr_period=14,
        atr_stop_multiplier=2.0,
        atr_target_multiplier=3.0,
        max_hold_bars=72,
        use_trend_filter=False,  # Disable trend filter for more signals
        trend_ema_period=50,
    )
    
    config = HyperliquidBacktestConfig(
        initial_capital=initial_capital,
        coin=coin,
        interval=interval,
        lookback_days=lookback_days,
        maker_fee_bps=0.0,  # Maker rebate converted to 0
        taker_fee_bps=5.0,
        slippage_bps=2.0,
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
