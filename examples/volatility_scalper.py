"""
Hyperliquid Volatility Scalping Strategy
========================================

A professional volatility-based scalping strategy for Hyperliquid perpetual futures.
Exploits short-term volatility expansion for quick profits with tight stops.

Strategy Logic:
- Monitor volatility (ATR, Bollinger Band width)
- Enter on volatility contraction (squeeze) followed by expansion
- Take quick profits on volatility spikes
- Use tight stops to minimize losses

Features:
- Bollinger Band squeeze detection
- Keltner Channel overlay for confirmation
- Dynamic position sizing based on volatility
- Fast entry/exit for scalping
- Proper long/short handling

Usage:
    python -m examples.volatility_scalper
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


class SqueezeState(Enum):
    NO_SQUEEZE = "no_squeeze"
    SQUEEZE_ON = "squeeze_on"
    SQUEEZE_FIRE_LONG = "fire_long"
    SQUEEZE_FIRE_SHORT = "fire_short"


@dataclass
class VolatilityMetrics:
    """Current volatility metrics"""
    atr: Decimal
    atr_pct: Decimal
    bb_width: Decimal
    kc_width: Decimal
    is_squeeze: bool
    momentum: Decimal


class VolatilityScalperStrategy(Strategy):
    """
    Volatility Scalping Strategy for Hyperliquid Perps
    
    This strategy identifies "squeeze" conditions where Bollinger Bands
    contract inside Keltner Channels, then trades the breakout direction.
    
    Key parameters:
    - bb_period: Bollinger Bands period
    - bb_std: Bollinger Bands standard deviation
    - kc_period: Keltner Channel period
    - kc_atr_mult: Keltner Channel ATR multiplier
    - momentum_period: Period for momentum calculation
    """
    
    def __init__(
        self,
        # Bollinger Bands settings
        bb_period: int = 20,
        bb_std: float = 2.0,
        
        # Keltner Channel settings
        kc_period: int = 20,
        kc_atr_mult: float = 1.5,
        
        # Momentum settings
        momentum_period: int = 12,
        momentum_threshold: float = 0.0,  # Any momentum triggers
        
        # ATR settings
        atr_period: int = 14,
        
        # Position sizing
        base_position_size: float = 0.1,
        max_position: float = 0.2,
        scale_with_volatility: bool = True,
        
        # Risk management (tight for scalping)
        stop_loss_atr_mult: float = 1.0,   # 1x ATR stop
        take_profit_atr_mult: float = 2.0,  # 2x ATR target
        max_hold_bars: int = 12,            # Quick exits
        
        # Confirmation
        min_squeeze_bars: int = 6,  # Minimum bars in squeeze before trade
        require_volume_spike: bool = True,
        volume_spike_mult: float = 1.3,
        
        strategy_id: Optional[str] = None,
    ):
        super().__init__(strategy_id or "VolatilityScalper")
        
        self.bb_period = bb_period
        self.bb_std = Decimal(str(bb_std))
        self.kc_period = kc_period
        self.kc_atr_mult = Decimal(str(kc_atr_mult))
        self.momentum_period = momentum_period
        self.momentum_threshold = Decimal(str(momentum_threshold))
        self.atr_period = atr_period
        
        self.base_size = Decimal(str(base_position_size))
        self.max_position = Decimal(str(max_position))
        self.scale_with_volatility = scale_with_volatility
        
        self.stop_atr_mult = Decimal(str(stop_loss_atr_mult))
        self.tp_atr_mult = Decimal(str(take_profit_atr_mult))
        self.max_hold_bars = max_hold_bars
        
        self.min_squeeze_bars = min_squeeze_bars
        self.require_volume_spike = require_volume_spike
        self.volume_spike_mult = Decimal(str(volume_spike_mult))
        
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
        
        # Squeeze tracking
        self.squeeze_bars = 0
        self.was_in_squeeze = False
        
        # EMA for Keltner Channel
        self.kc_ema: Optional[Decimal] = None
        self.kc_ema_mult: Decimal = Decimal(str(2 / (kc_period + 1)))
        
        # Tracking
        self.current_bar = 0
        self.total_trades = 0
        self.winning_trades = 0
        self.losing_trades = 0
        self.long_trades = 0
        self.short_trades = 0
        self.squeeze_entries = 0
    
    def on_start(self, ctx: StrategyContext) -> None:
        print(f"[{self.strategy_id}] Volatility Scalper starting")
        print(f"  Bollinger Bands: period={self.bb_period}, std={self.bb_std}")
        print(f"  Keltner Channel: period={self.kc_period}, ATR mult={self.kc_atr_mult}")
        print(f"  Momentum period: {self.momentum_period}")
        print(f"  ATR period: {self.atr_period}")
        print(f"  Stop: {self.stop_atr_mult}x ATR, Target: {self.tp_atr_mult}x ATR")
        print(f"  Min squeeze bars: {self.min_squeeze_bars}")
        print(f"  Max hold bars: {self.max_hold_bars}")
        
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
    
    def calculate_bollinger_bands(self) -> tuple:
        """Calculate Bollinger Bands"""
        if len(self.closes) < self.bb_period:
            return None, None, None
        
        period_closes = self.closes[-self.bb_period:]
        
        sma = sum(period_closes) / len(period_closes)
        variance = sum((c - sma) ** 2 for c in period_closes) / len(period_closes)
        std = variance ** Decimal("0.5")
        
        upper = sma + (std * self.bb_std)
        lower = sma - (std * self.bb_std)
        
        return upper, sma, lower
    
    def calculate_keltner_channel(self) -> tuple:
        """Calculate Keltner Channel"""
        if self.kc_ema is None:
            return None, None, None
        
        atr = self.calculate_atr()
        if atr == 0:
            return None, None, None
        
        upper = self.kc_ema + (atr * self.kc_atr_mult)
        lower = self.kc_ema - (atr * self.kc_atr_mult)
        
        return upper, self.kc_ema, lower
    
    def update_kc_ema(self, price: Decimal) -> None:
        """Update Keltner Channel EMA"""
        if self.kc_ema is None:
            self.kc_ema = price
        else:
            self.kc_ema = (price * self.kc_ema_mult) + (self.kc_ema * (1 - self.kc_ema_mult))
    
    def calculate_momentum(self) -> Decimal:
        """Calculate momentum (linear regression slope)"""
        if len(self.closes) < self.momentum_period:
            return Decimal("0")
        
        # Simple momentum: current vs N bars ago
        current = self.closes[-1]
        past = self.closes[-self.momentum_period]
        
        return (current - past) / past if past != 0 else Decimal("0")
    
    def detect_squeeze_state(self) -> SqueezeState:
        """Detect squeeze state"""
        bb_upper, bb_mid, bb_lower = self.calculate_bollinger_bands()
        kc_upper, kc_mid, kc_lower = self.calculate_keltner_channel()
        
        if None in [bb_upper, bb_lower, kc_upper, kc_lower]:
            return SqueezeState.NO_SQUEEZE
        
        # Squeeze is ON when BB is inside KC
        is_squeeze = bb_lower > kc_lower and bb_upper < kc_upper
        
        if is_squeeze:
            self.squeeze_bars += 1
            self.was_in_squeeze = True
            return SqueezeState.SQUEEZE_ON
        else:
            if self.was_in_squeeze and self.squeeze_bars >= self.min_squeeze_bars:
                # Squeeze just released - check momentum for direction
                momentum = self.calculate_momentum()
                self.was_in_squeeze = False
                bars_held = self.squeeze_bars
                self.squeeze_bars = 0
                
                if momentum > self.momentum_threshold:
                    return SqueezeState.SQUEEZE_FIRE_LONG
                elif momentum < -self.momentum_threshold:
                    return SqueezeState.SQUEEZE_FIRE_SHORT
            
            self.squeeze_bars = 0
            self.was_in_squeeze = False
            return SqueezeState.NO_SQUEEZE
    
    def check_volume_confirmation(self) -> bool:
        """Check if volume confirms the breakout"""
        if not self.require_volume_spike:
            return True
        
        if len(self.volumes) < self.bb_period:
            return True
        
        avg_volume = sum(self.volumes[-self.bb_period:-1]) / (self.bb_period - 1)
        current_volume = self.volumes[-1]
        
        return current_volume >= avg_volume * self.volume_spike_mult
    
    def calculate_position_size(self, atr: Decimal, price: Decimal) -> Decimal:
        """Calculate position size based on volatility"""
        if not self.scale_with_volatility or atr == 0:
            return self.base_size
        
        # Lower volatility -> larger size, higher volatility -> smaller size
        vol_pct = atr / price
        
        # Target 1% risk per trade
        target_risk = Decimal("0.01")
        
        # Risk-adjusted size
        if self.stop_atr_mult > 0:
            stop_pct = (atr * self.stop_atr_mult) / price
            risk_size = (target_risk / stop_pct) * self.base_size
        else:
            risk_size = self.base_size
        
        return min(max(risk_size, self.base_size * Decimal("0.5")), self.max_position)
    
    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        """Process each bar"""
        self.current_bar += 1
        
        current_price = Decimal(str(bar.close))
        
        # Update history
        self.highs.append(Decimal(str(bar.high)))
        self.lows.append(Decimal(str(bar.low)))
        self.closes.append(current_price)
        self.volumes.append(Decimal(str(bar.volume)))
        
        # Update Keltner EMA
        self.update_kc_ema(current_price)
        
        # Keep limited history
        max_history = max(self.bb_period, self.kc_period, self.atr_period, self.momentum_period) + 10
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
        
        # Check entries (only if flat and enough data)
        if self.position == 0 and len(self.closes) >= max(self.bb_period, self.kc_period):
            squeeze_state = self.detect_squeeze_state()
            
            if squeeze_state == SqueezeState.SQUEEZE_FIRE_LONG:
                if self.check_volume_confirmation():
                    self._open_long(ctx, current_price)
            elif squeeze_state == SqueezeState.SQUEEZE_FIRE_SHORT:
                if self.check_volume_confirmation():
                    self._open_short(ctx, current_price)
    
    def _check_exits(self, ctx: StrategyContext, bar: Bar) -> None:
        """Check exit conditions"""
        current_price = Decimal(str(bar.close))
        current_high = Decimal(str(bar.high))
        current_low = Decimal(str(bar.low))
        
        should_exit = False
        exit_reason = ""
        exit_price = current_price
        
        if self.position > 0:  # Long
            if current_low <= self.stop_loss:
                should_exit = True
                exit_reason = "STOP_LOSS"
                exit_price = self.stop_loss
            elif current_high >= self.take_profit:
                should_exit = True
                exit_reason = "TAKE_PROFIT"
                exit_price = self.take_profit
        
        elif self.position < 0:  # Short
            if current_high >= self.stop_loss:
                should_exit = True
                exit_reason = "STOP_LOSS"
                exit_price = self.stop_loss
            elif current_low <= self.take_profit:
                should_exit = True
                exit_reason = "TAKE_PROFIT"
                exit_price = self.take_profit
        
        # Time-based exit (scalping = quick exits)
        if not should_exit and self.bars_since_entry >= self.max_hold_bars:
            should_exit = True
            exit_reason = "TIME_LIMIT"
        
        if should_exit:
            self._close_position(ctx, exit_price, exit_reason)
    
    def _open_long(self, ctx: StrategyContext, price: Decimal) -> None:
        """Open a long position on squeeze fire"""
        atr = self.calculate_atr()
        if atr == 0:
            atr = price * Decimal("0.02")
        
        size = self.calculate_position_size(atr, price)
        
        self.position = size
        self.entry_price = price
        self.stop_loss = price - (atr * self.stop_atr_mult)
        self.take_profit = price + (atr * self.tp_atr_mult)
        self.bars_since_entry = 0
        self.long_trades += 1
        self.squeeze_entries += 1
        
        ctx.market_order(self.instrument, OrderSide.Buy, float(size), reduce_only=False)
        
        momentum = self.calculate_momentum()
        print(f"  [SQUEEZE FIRE LONG] @ ${price:.2f}, Momentum: {momentum*100:.2f}%")
        print(f"    Size: {size:.4f}, Stop: ${self.stop_loss:.2f}, Target: ${self.take_profit:.2f}")
    
    def _open_short(self, ctx: StrategyContext, price: Decimal) -> None:
        """Open a short position on squeeze fire"""
        atr = self.calculate_atr()
        if atr == 0:
            atr = price * Decimal("0.02")
        
        size = self.calculate_position_size(atr, price)
        
        self.position = -size
        self.entry_price = price
        self.stop_loss = price + (atr * self.stop_atr_mult)
        self.take_profit = price - (atr * self.tp_atr_mult)
        self.bars_since_entry = 0
        self.short_trades += 1
        self.squeeze_entries += 1
        
        ctx.market_order(self.instrument, OrderSide.Sell, float(size), reduce_only=False)
        
        momentum = self.calculate_momentum()
        print(f"  [SQUEEZE FIRE SHORT] @ ${price:.2f}, Momentum: {momentum*100:.2f}%")
        print(f"    Size: {size:.4f}, Stop: ${self.stop_loss:.2f}, Target: ${self.take_profit:.2f}")
    
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
        
        direction = "Long" if self.position > 0 else "Short"
        print(f"  [CLOSE {direction}] {reason} @ ${price:.2f}")
        print(f"    Entry: ${self.entry_price:.2f}, P&L: {pnl_pct * 100:.2f}%, Bars: {self.bars_since_entry}")
        
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
        
        print(f"\n[{self.strategy_id}] Volatility Scalper stopped")
        print(f"  Total trades: {self.total_trades}")
        print(f"    Long trades: {self.long_trades}")
        print(f"    Short trades: {self.short_trades}")
        print(f"  Squeeze entries: {self.squeeze_entries}")
        print(f"  Winning trades: {self.winning_trades}")
        print(f"  Losing trades: {self.losing_trades}")
        if self.total_trades > 0:
            print(f"  Win rate: {self.winning_trades/self.total_trades*100:.1f}%")


async def main():
    """Run the Volatility Scalper backtest"""
    print("=" * 60)
    print("HYPERLIQUID VOLATILITY SCALPER BACKTEST")
    print("=" * 60)
    
    # Configuration - use shorter timeframe for scalping
    coin = "ETH"
    interval = CandleInterval.MIN_15  # 15-minute candles for scalping
    lookback_days = 30
    initial_capital = 10000.0
    
    strategy = VolatilityScalperStrategy(
        bb_period=20,
        bb_std=2.0,
        kc_period=20,
        kc_atr_mult=1.5,
        momentum_period=12,
        momentum_threshold=0.0,
        atr_period=14,
        base_position_size=0.1,
        max_position=0.2,
        scale_with_volatility=True,
        stop_loss_atr_mult=1.0,
        take_profit_atr_mult=2.0,
        max_hold_bars=8,  # Short hold time
        min_squeeze_bars=4,
        require_volume_spike=True,
        volume_spike_mult=1.2,
    )
    
    config = HyperliquidBacktestConfig(
        initial_capital=initial_capital,
        coin=coin,
        interval=interval,
        lookback_days=lookback_days,
        maker_fee_bps=0.0,
        taker_fee_bps=5.0,
        slippage_bps=1.5,
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
