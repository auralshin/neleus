"""
Hyperliquid RSI Mean Reversion Strategy
=======================================

A professional RSI-based mean reversion strategy for Hyperliquid perpetual futures.
Trades oversold/overbought conditions with proper long and short positions.

Strategy Logic:
- RSI below oversold threshold (e.g., 30) -> GO LONG (expecting bounce)
- RSI above overbought threshold (e.g., 70) -> GO SHORT (expecting drop)
- Exit when RSI reverts to neutral zone

Features:
- RSI with configurable period
- Multi-timeframe confirmation (optional)
- Dynamic position sizing based on RSI extremity
- Bollinger Band confirmation (optional)
- Proper stop loss and take profit management

Usage:
    python -m examples.rsi_mean_reversion
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
class RSIState:
    """Current RSI state"""
    value: Decimal
    is_oversold: bool
    is_overbought: bool
    extremity: Decimal  # How extreme (0-1)


@dataclass
class BollingerBands:
    """Bollinger Bands values"""
    upper: Decimal
    middle: Decimal
    lower: Decimal
    width_pct: Decimal


class RSIMeanReversionStrategy(Strategy):
    """
    RSI Mean Reversion Strategy for Hyperliquid Perps
    
    This strategy trades mean reversion based on RSI extremes.
    Goes long when RSI is oversold, short when overbought.
    
    Key parameters:
    - rsi_period: Period for RSI calculation
    - oversold_threshold: RSI level to go long
    - overbought_threshold: RSI level to go short
    - exit_threshold: RSI level to exit positions
    - position_size: Base position size
    - use_bb_confirmation: Require price at Bollinger Band
    """
    
    def __init__(
        self,
        # RSI settings
        rsi_period: int = 14,
        oversold_threshold: float = 30.0,
        overbought_threshold: float = 70.0,
        exit_oversold: float = 45.0,   # Exit long when RSI reaches this
        exit_overbought: float = 55.0,  # Exit short when RSI reaches this
        
        # Bollinger Band confirmation
        use_bb_confirmation: bool = True,
        bb_period: int = 20,
        bb_std: float = 2.0,
        
        # Position sizing
        base_position_size: float = 0.1,
        max_position: float = 0.3,
        scale_with_extremity: bool = True,  # More size at more extreme RSI
        
        # Risk management
        stop_loss_pct: float = 0.03,      # 3% stop loss
        take_profit_pct: float = 0.04,     # 4% take profit
        max_hold_bars: int = 48,           # Max hold time
        
        # Trend filter
        use_trend_filter: bool = False,    # Disabled for mean reversion
        trend_ema_period: int = 100,
        
        strategy_id: Optional[str] = None,
    ):
        super().__init__(strategy_id or "RSIMeanReversion")
        
        self.rsi_period = rsi_period
        self.oversold = Decimal(str(oversold_threshold))
        self.overbought = Decimal(str(overbought_threshold))
        self.exit_oversold = Decimal(str(exit_oversold))
        self.exit_overbought = Decimal(str(exit_overbought))
        
        self.use_bb = use_bb_confirmation
        self.bb_period = bb_period
        self.bb_std = Decimal(str(bb_std))
        
        self.base_size = Decimal(str(base_position_size))
        self.max_position = Decimal(str(max_position))
        self.scale_with_extremity = scale_with_extremity
        
        self.stop_loss_pct = Decimal(str(stop_loss_pct))
        self.take_profit_pct = Decimal(str(take_profit_pct))
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
        self.closes: List[Decimal] = []
        self.gains: List[Decimal] = []
        self.losses: List[Decimal] = []
        
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
        self.rsi_exits = 0
        self.stopped_out = 0
        self.profit_taken = 0
    
    def on_start(self, ctx: StrategyContext) -> None:
        print(f"[{self.strategy_id}] RSI Mean Reversion Strategy starting")
        print(f"  RSI period: {self.rsi_period}")
        print(f"  Oversold: <{self.oversold}, Overbought: >{self.overbought}")
        print(f"  Exit levels: {self.exit_oversold} / {self.exit_overbought}")
        print(f"  Base position: {self.base_size}")
        print(f"  Stop loss: {self.stop_loss_pct:.1%}, Take profit: {self.take_profit_pct:.1%}")
        if self.use_bb:
            print(f"  Bollinger Bands: period={self.bb_period}, std={self.bb_std}")
        
        self.instrument = InstrumentId(
            venue=Venue.Hyperliquid,
            symbol="ETH",
            instrument_type=InstrumentType.Perp,
        )
    
    def calculate_rsi(self) -> RSIState:
        """Calculate RSI and return state"""
        if len(self.closes) < self.rsi_period + 1:
            return RSIState(
                value=Decimal("50"),
                is_oversold=False,
                is_overbought=False,
                extremity=Decimal("0")
            )
        
        # Calculate price changes
        changes = []
        for i in range(-self.rsi_period, 0):
            change = self.closes[i] - self.closes[i - 1]
            changes.append(change)
        
        # Separate gains and losses
        gains = [max(c, Decimal("0")) for c in changes]
        losses = [abs(min(c, Decimal("0"))) for c in changes]
        
        avg_gain = sum(gains) / len(gains)
        avg_loss = sum(losses) / len(losses)
        
        if avg_loss == 0:
            rsi = Decimal("100")
        else:
            rs = avg_gain / avg_loss
            rsi = Decimal("100") - (Decimal("100") / (1 + rs))
        
        # Determine state
        is_oversold = rsi <= self.oversold
        is_overbought = rsi >= self.overbought
        
        # Calculate extremity (0-1 scale)
        if rsi <= self.oversold:
            extremity = (self.oversold - rsi) / self.oversold
        elif rsi >= self.overbought:
            extremity = (rsi - self.overbought) / (Decimal("100") - self.overbought)
        else:
            extremity = Decimal("0")
        
        return RSIState(
            value=rsi,
            is_oversold=is_oversold,
            is_overbought=is_overbought,
            extremity=min(extremity, Decimal("1"))
        )
    
    def calculate_bollinger_bands(self) -> Optional[BollingerBands]:
        """Calculate Bollinger Bands"""
        if len(self.closes) < self.bb_period:
            return None
        
        period_closes = self.closes[-self.bb_period:]
        
        # Calculate SMA (middle band)
        sma = sum(period_closes) / len(period_closes)
        
        # Calculate standard deviation
        variance = sum((c - sma) ** 2 for c in period_closes) / len(period_closes)
        std = variance ** Decimal("0.5")
        
        upper = sma + (std * self.bb_std)
        lower = sma - (std * self.bb_std)
        
        width_pct = (upper - lower) / sma if sma > 0 else Decimal("0")
        
        return BollingerBands(
            upper=upper,
            middle=sma,
            lower=lower,
            width_pct=width_pct
        )
    
    def update_ema(self, price: Decimal) -> None:
        """Update exponential moving average"""
        if self.ema is None:
            self.ema = price
        else:
            self.ema = (price * self.ema_multiplier) + (self.ema * (1 - self.ema_multiplier))
    
    def calculate_position_size(self, rsi_state: RSIState) -> Decimal:
        """Calculate position size based on RSI extremity"""
        if not self.scale_with_extremity:
            return self.base_size
        
        # Scale from 0.5x to 1.5x based on extremity
        scale = Decimal("0.5") + rsi_state.extremity
        size = self.base_size * scale
        
        return min(size, self.max_position)
    
    def check_bb_confirmation(self, price: Decimal, is_long: bool) -> bool:
        """Check if Bollinger Band confirms the signal"""
        if not self.use_bb:
            return True
        
        bb = self.calculate_bollinger_bands()
        if bb is None:
            return True
        
        if is_long:
            # For long, price should be at or below lower band
            return price <= bb.lower * Decimal("1.01")  # 1% tolerance
        else:
            # For short, price should be at or above upper band
            return price >= bb.upper * Decimal("0.99")
    
    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        """Process each bar"""
        self.current_bar += 1
        current_price = Decimal(str(bar.close))
        
        # Update history
        self.closes.append(current_price)
        
        # Update EMA
        self.update_ema(current_price)
        
        # Keep limited history
        max_history = max(self.rsi_period, self.bb_period, self.trend_ema_period) + 10
        if len(self.closes) > max_history:
            self.closes = self.closes[-max_history:]
        
        # Track position duration
        if self.position != 0:
            self.bars_since_entry += 1
        
        # Calculate RSI
        rsi = self.calculate_rsi()
        
        # Check exits first
        if self.position != 0:
            self._check_exits(ctx, bar, rsi)
        
        # Check entries (only if flat)
        if self.position == 0 and len(self.closes) >= self.rsi_period + 1:
            self._check_entry(ctx, bar, rsi, current_price)
    
    def _check_exits(self, ctx: StrategyContext, bar: Bar, rsi: RSIState) -> None:
        """Check exit conditions"""
        current_price = Decimal(str(bar.close))
        current_high = Decimal(str(bar.high))
        current_low = Decimal(str(bar.low))
        
        should_exit = False
        exit_reason = ""
        exit_price = current_price
        
        if self.position > 0:  # Long position
            # Check stop loss
            if current_low <= self.stop_loss:
                should_exit = True
                exit_reason = "STOP_LOSS"
                exit_price = self.stop_loss
                self.stopped_out += 1
            # Check take profit
            elif current_high >= self.take_profit:
                should_exit = True
                exit_reason = "TAKE_PROFIT"
                exit_price = self.take_profit
                self.profit_taken += 1
            # RSI exit (mean reversion complete)
            elif rsi.value >= self.exit_oversold:
                should_exit = True
                exit_reason = "RSI_NORMALIZED"
                self.rsi_exits += 1
        
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
            # RSI exit (mean reversion complete)
            elif rsi.value <= self.exit_overbought:
                should_exit = True
                exit_reason = "RSI_NORMALIZED"
                self.rsi_exits += 1
        
        # Time-based exit
        if not should_exit and self.bars_since_entry >= self.max_hold_bars:
            should_exit = True
            exit_reason = "TIME_LIMIT"
        
        if should_exit:
            self._close_position(ctx, exit_price, exit_reason, rsi.value)
    
    def _check_entry(
        self, 
        ctx: StrategyContext, 
        bar: Bar, 
        rsi: RSIState, 
        price: Decimal
    ) -> None:
        """Check for entry opportunities"""
        
        # Check trend filter if enabled
        if self.use_trend_filter and self.ema is not None:
            # For mean reversion, we might want to trade against trend
            # But skip if trend is too strong
            trend_strength = abs(price - self.ema) / self.ema
            if trend_strength > Decimal("0.05"):  # 5% from EMA
                return  # Skip, trend too strong
        
        # Long entry: RSI oversold + BB confirmation
        if rsi.is_oversold:
            if self.check_bb_confirmation(price, is_long=True):
                self._open_long(ctx, price, rsi)
        
        # Short entry: RSI overbought + BB confirmation
        elif rsi.is_overbought:
            if self.check_bb_confirmation(price, is_long=False):
                self._open_short(ctx, price, rsi)
    
    def _open_long(self, ctx: StrategyContext, price: Decimal, rsi: RSIState) -> None:
        """Open a long position"""
        size = self.calculate_position_size(rsi)
        
        self.position = size
        self.entry_price = price
        self.stop_loss = price * (1 - self.stop_loss_pct)
        self.take_profit = price * (1 + self.take_profit_pct)
        self.bars_since_entry = 0
        self.long_trades += 1
        
        ctx.market_order(self.instrument, OrderSide.Buy, float(size), reduce_only=False)
        
        print(f"  [LONG] RSI={rsi.value:.1f} (Oversold) @ ${price:.2f}")
        print(f"    Size: {size:.4f}, Stop: ${self.stop_loss:.2f}, Target: ${self.take_profit:.2f}")
    
    def _open_short(self, ctx: StrategyContext, price: Decimal, rsi: RSIState) -> None:
        """Open a short position"""
        size = self.calculate_position_size(rsi)
        
        self.position = -size
        self.entry_price = price
        self.stop_loss = price * (1 + self.stop_loss_pct)
        self.take_profit = price * (1 - self.take_profit_pct)
        self.bars_since_entry = 0
        self.short_trades += 1
        
        ctx.market_order(self.instrument, OrderSide.Sell, float(size), reduce_only=False)
        
        print(f"  [SHORT] RSI={rsi.value:.1f} (Overbought) @ ${price:.2f}")
        print(f"    Size: {size:.4f}, Stop: ${self.stop_loss:.2f}, Target: ${self.take_profit:.2f}")
    
    def _close_position(
        self, 
        ctx: StrategyContext, 
        price: Decimal, 
        reason: str,
        current_rsi: Decimal
    ) -> None:
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
        print(f"  [CLOSE {direction}] {reason} @ ${price:.2f}, RSI={current_rsi:.1f}")
        print(f"    Entry: ${self.entry_price:.2f}, P&L: {pnl_pct * 100:.2f}%")
        
        self.position = Decimal("0")
        self.entry_price = None
        self.stop_loss = None
        self.take_profit = None
        self.bars_since_entry = 0
    
    def on_stop(self, ctx: StrategyContext) -> None:
        """Cleanup and print statistics"""
        if self.position != 0:
            current_price = self.closes[-1] if self.closes else Decimal("0")
            rsi = self.calculate_rsi()
            self._close_position(ctx, current_price, "STRATEGY_STOP", rsi.value)
        
        print(f"\n[{self.strategy_id}] RSI Mean Reversion stopped")
        print(f"  Total trades: {self.total_trades}")
        print(f"    Long trades: {self.long_trades}")
        print(f"    Short trades: {self.short_trades}")
        print(f"  Winning trades: {self.winning_trades}")
        print(f"  Losing trades: {self.losing_trades}")
        print(f"  Exit reasons:")
        print(f"    RSI normalized: {self.rsi_exits}")
        print(f"    Stop loss: {self.stopped_out}")
        print(f"    Take profit: {self.profit_taken}")
        if self.total_trades > 0:
            print(f"  Win rate: {self.winning_trades/self.total_trades*100:.1f}%")


async def main():
    """Run the RSI Mean Reversion backtest"""
    print("=" * 60)
    print("HYPERLIQUID RSI MEAN REVERSION BACKTEST")
    print("=" * 60)
    
    # Configuration
    coin = "ETH"
    interval = CandleInterval.HOUR_1
    lookback_days = 60
    initial_capital = 10000.0
    
    strategy = RSIMeanReversionStrategy(
        rsi_period=14,
        oversold_threshold=25.0,
        overbought_threshold=75.0,
        exit_oversold=50.0,
        exit_overbought=50.0,
        use_bb_confirmation=True,
        bb_period=20,
        bb_std=2.0,
        base_position_size=0.1,
        max_position=0.3,
        scale_with_extremity=True,
        stop_loss_pct=0.025,
        take_profit_pct=0.03,
        max_hold_bars=36,
    )
    
    config = HyperliquidBacktestConfig(
        initial_capital=initial_capital,
        coin=coin,
        interval=interval,
        lookback_days=lookback_days,
        maker_fee_bps=0.0,
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
