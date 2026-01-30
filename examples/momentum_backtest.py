import asyncio
from decimal import Decimal
from typing import Optional

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


class MomentumStrategy(Strategy):
    def __init__(
        self,
        lookback: int = 20,
        threshold: float = 0.02,
        position_size: float = 0.1,
        max_position: float = 1.0,
        # Risk Management Parameters
        use_stop_loss: bool = True,
        stop_loss_pct: float = 0.02,  # 2% stop loss
        use_take_profit: bool = True,
        take_profit_pct: float = 0.05,  # 5% take profit
        use_atr_stops: bool = False,
        atr_period: int = 14,
        atr_multiplier: float = 2.0,
        max_risk_per_trade: float = 0.01,  # 1% of capital per trade
        trailing_stop: bool = False,
        trailing_stop_pct: float = 0.03,  # 3% trailing stop
        strategy_id: Optional[str] = None,
    ):
        super().__init__(strategy_id or "MomentumStrategy")
        self.lookback = lookback
        self.threshold = Decimal(str(threshold))
        self.position_size = Decimal(str(position_size))
        self.max_position = Decimal(str(max_position))
        
        # Risk management settings
        self.use_stop_loss = use_stop_loss
        self.stop_loss_pct = Decimal(str(stop_loss_pct))
        self.use_take_profit = use_take_profit
        self.take_profit_pct = Decimal(str(take_profit_pct))
        self.use_atr_stops = use_atr_stops
        self.atr_period = atr_period
        self.atr_multiplier = Decimal(str(atr_multiplier))
        self.max_risk_per_trade = Decimal(str(max_risk_per_trade))
        self.trailing_stop = trailing_stop
        self.trailing_stop_pct = Decimal(str(trailing_stop_pct))
        
        # State tracking
        self.closes: list[Decimal] = []
        self.highs: list[Decimal] = []
        self.lows: list[Decimal] = []
        self.position: Decimal = Decimal("0")
        self.instrument: Optional[InstrumentId] = None
        self.entry_price: Optional[Decimal] = None
        self.stop_loss_price: Optional[Decimal] = None
        self.take_profit_price: Optional[Decimal] = None
        self.highest_price_since_entry: Optional[Decimal] = None
        
        # Performance tracking
        self.total_trades = 0
        self.winning_trades = 0
        self.losing_trades = 0
        self.stopped_out = 0
        self.profit_taken = 0
    
    def on_start(self, ctx: StrategyContext) -> None:
        print(f"[{self.strategy_id}] Starting momentum strategy with risk management")
        print(f"  Lookback: {self.lookback} bars")
        print(f"  Threshold: {self.threshold:.2%}")
        print(f"  Position size: {self.position_size}")
        print(f"  Max position: {self.max_position}")
        print(f"\n  Risk Management:")
        if self.use_stop_loss:
            if self.use_atr_stops:
                print(f"    ATR Stop Loss: {self.atr_multiplier}x ATR ({self.atr_period} period)")
            else:
                print(f"    Fixed Stop Loss: {self.stop_loss_pct:.2%}")
        if self.use_take_profit:
            print(f"    Take Profit: {self.take_profit_pct:.2%}")
        if self.trailing_stop:
            print(f"    Trailing Stop: {self.trailing_stop_pct:.2%}")
        print(f"    Max Risk/Trade: {self.max_risk_per_trade:.2%}")
        
        self.instrument = InstrumentId(
            venue=Venue.Hyperliquid,
            symbol="ETH",
            instrument_type=InstrumentType.Perp,
        )
    
    def calculate_atr(self) -> Decimal:
        """Calculate Average True Range for ATR-based stops"""
        if len(self.highs) < self.atr_period:
            return Decimal("0")
        
        true_ranges = []
        for i in range(-self.atr_period, 0):
            high = self.highs[i]
            low = self.lows[i]
            prev_close = self.closes[i - 1] if i > -self.atr_period else self.closes[i]
            
            tr = max(
                high - low,
                abs(high - prev_close),
                abs(low - prev_close)
            )
            true_ranges.append(tr)
        
        return sum(true_ranges) / len(true_ranges)
    
    def calculate_position_size_with_risk(
        self, 
        current_price: Decimal, 
        stop_distance: Decimal,
        account_value: Decimal = Decimal("10000")
    ) -> Decimal:
        """Calculate position size based on risk per trade"""
        if stop_distance == 0:
            return self.position_size
        
        # Risk amount in dollars
        risk_amount = account_value * self.max_risk_per_trade
        
        # Position size = risk amount / stop distance
        risk_based_size = risk_amount / (current_price * stop_distance)
        
        # Cap at configured position size
        return min(risk_based_size, self.position_size)
    
    def check_stop_loss(self, current_price: Decimal) -> bool:
        """Check if stop loss is hit"""
        if not self.use_stop_loss or self.stop_loss_price is None:
            return False
        
        if self.position > 0:  # Long position
            return current_price <= self.stop_loss_price
        elif self.position < 0:  # Short position
            return current_price >= self.stop_loss_price
        
        return False
    
    def check_take_profit(self, current_price: Decimal) -> bool:
        """Check if take profit is hit"""
        if not self.use_take_profit or self.take_profit_price is None:
            return False
        
        if self.position > 0:  # Long position
            return current_price >= self.take_profit_price
        elif self.position < 0:  # Short position
            return current_price <= self.take_profit_price
        
        return False
    
    def update_trailing_stop(self, current_price: Decimal):
        """Update trailing stop loss"""
        if not self.trailing_stop or self.position == 0:
            return
        
        if self.position > 0:  # Long position
            if self.highest_price_since_entry is None or current_price > self.highest_price_since_entry:
                self.highest_price_since_entry = current_price
                new_stop = current_price * (Decimal("1") - self.trailing_stop_pct)
                if self.stop_loss_price is None or new_stop > self.stop_loss_price:
                    self.stop_loss_price = new_stop
        
        elif self.position < 0:  # Short position
            if self.highest_price_since_entry is None or current_price < self.highest_price_since_entry:
                self.highest_price_since_entry = current_price
                new_stop = current_price * (Decimal("1") + self.trailing_stop_pct)
                if self.stop_loss_price is None or new_stop < self.stop_loss_price:
                    self.stop_loss_price = new_stop
    
    def on_data(self, ctx: StrategyContext, data) -> None:
        if not isinstance(data, Bar):
            return
        
        self.instrument = data.instrument_id
        
        # Store OHLC data
        self.closes.append(Decimal(str(data.close)))
        self.highs.append(Decimal(str(data.high)))
        self.lows.append(Decimal(str(data.low)))
        
        # Keep only necessary history
        if len(self.closes) > self.lookback + self.atr_period + 1:
            self.closes = self.closes[-(self.lookback + self.atr_period + 1):]
            self.highs = self.highs[-(self.lookback + self.atr_period + 1):]
            self.lows = self.lows[-(self.lookback + self.atr_period + 1):]
        
        current_price = self.closes[-1]
        
        # Check risk management exits first
        if self.position != 0:
            # Update trailing stop
            self.update_trailing_stop(current_price)
            
            # Check stop loss
            if self.check_stop_loss(current_price):
                self._close_position(ctx, current_price, "Stop Loss")
                self.stopped_out += 1
                return
            
            # Check take profit
            if self.check_take_profit(current_price):
                self._close_position(ctx, current_price, "Take Profit")
                self.profit_taken += 1
                return
        
        # Need enough data for momentum calculation
        if len(self.closes) < self.lookback + 1:
            return
        
        # Calculate momentum
        old_price = self.closes[-self.lookback - 1]
        
        if old_price == 0:
            return
        
        momentum = (current_price - old_price) / old_price
        
        # Generate signals with risk management
        self._generate_signals(ctx, momentum, current_price)
    
    def _close_position(self, ctx: StrategyContext, current_price: Decimal, reason: str):
        """Close current position"""
        if self.position == 0:
            return
        
        print(f"  [{reason}] Closing position at ${current_price:.2f}")
        
        # Track P&L
        if self.entry_price is not None:
            pnl_pct = (current_price - self.entry_price) / self.entry_price
            if self.position < 0:  # Short position
                pnl_pct = -pnl_pct
            
            if pnl_pct > 0:
                self.winning_trades += 1
            else:
                self.losing_trades += 1
            
            print(f"    Entry: ${self.entry_price:.2f}, Exit: ${current_price:.2f}, P&L: {pnl_pct:.2%}")
        
        # Close position
        if self.position > 0:
            ctx.market_order(self.instrument, OrderSide.Sell, float(abs(self.position)))
        else:
            ctx.market_order(self.instrument, OrderSide.Buy, float(abs(self.position)))
        
        # Reset state
        self.position = Decimal("0")
        self.entry_price = None
        self.stop_loss_price = None
        self.take_profit_price = None
        self.highest_price_since_entry = None
    
    def _generate_signals(
        self,
        ctx: StrategyContext,
        momentum: Decimal,
        current_price: Decimal,
    ) -> None:
        """Generate trading signals with risk management"""
        
        # If we have a position, let risk management handle exits
        if self.position != 0:
            return
        
        # Calculate ATR if using ATR-based stops
        atr = self.calculate_atr() if self.use_atr_stops else Decimal("0")
        
        # Determine entry signal
        should_long = momentum > self.threshold
        should_short = momentum < -self.threshold
        
        if not should_long and not should_short:
            return
        
        # Calculate stop loss distance
        if self.use_atr_stops and atr > 0:
            stop_distance = atr * self.atr_multiplier / current_price
        else:
            stop_distance = self.stop_loss_pct
        
        # Calculate position size based on risk
        position_size = self.calculate_position_size_with_risk(
            current_price, stop_distance
        )
        
        # Cap at max position
        position_size = min(position_size, self.max_position)
        
        # Minimum trade size
        min_trade = Decimal("0.001")
        if position_size < min_trade:
            return
        
        # Enter long position
        if should_long:
            print(f"  [BUY] Entry at ${current_price:.2f}, Size: {position_size:.4f}, Momentum: {momentum:.2%}")
            
            ctx.market_order(
                self.instrument,
                OrderSide.Buy,
                float(position_size),
            )
            
            self.position = position_size
            self.entry_price = current_price
            self.highest_price_since_entry = current_price
            
            # Set stop loss
            if self.use_stop_loss:
                if self.use_atr_stops and atr > 0:
                    self.stop_loss_price = current_price - (atr * self.atr_multiplier)
                else:
                    self.stop_loss_price = current_price * (Decimal("1") - self.stop_loss_pct)
                print(f"    Stop Loss: ${self.stop_loss_price:.2f}")
            
            # Set take profit
            if self.use_take_profit:
                self.take_profit_price = current_price * (Decimal("1") + self.take_profit_pct)
                print(f"    Take Profit: ${self.take_profit_price:.2f}")
            
            self.total_trades += 1
        
        # Enter short position
        elif should_short:
            print(f"  [SELL] Entry at ${current_price:.2f}, Size: {position_size:.4f}, Momentum: {momentum:.2%}")
            
            ctx.market_order(
                self.instrument,
                OrderSide.Sell,
                float(position_size),
            )
            
            self.position = -position_size
            self.entry_price = current_price
            self.highest_price_since_entry = current_price
            
            # Set stop loss
            if self.use_stop_loss:
                if self.use_atr_stops and atr > 0:
                    self.stop_loss_price = current_price + (atr * self.atr_multiplier)
                else:
                    self.stop_loss_price = current_price * (Decimal("1") + self.stop_loss_pct)
                print(f"    Stop Loss: ${self.stop_loss_price:.2f}")
            
            # Set take profit
            if self.use_take_profit:
                self.take_profit_price = current_price * (Decimal("1") - self.take_profit_pct)
                print(f"    Take Profit: ${self.take_profit_price:.2f}")
            
            self.total_trades += 1
    
    def on_stop(self, ctx: StrategyContext) -> None:
        print(f"\n[{self.strategy_id}] Strategy stopped")
        print(f"  Final position: {self.position}")
        print(f"  Total bars processed: {len(self.closes)}")
        print(f"\n  Performance Statistics:")
        print(f"    Total trades: {self.total_trades}")
        print(f"    Winning trades: {self.winning_trades}")
        print(f"    Losing trades: {self.losing_trades}")
        if self.total_trades > 0:
            win_rate = (self.winning_trades / self.total_trades) * 100
            print(f"    Win rate: {win_rate:.1f}%")
        print(f"    Stopped out: {self.stopped_out}")
        print(f"    Profit taken: {self.profit_taken}")


async def main():
    print("=" * 80)
    print("Neleus Momentum Strategy Backtest with Risk Management")
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
    
    # Strategy with risk management
    strategy = MomentumStrategy(
        lookback=20,
        threshold=0.02,
        position_size=0.1,
        max_position=0.5,
        # Risk management
        use_stop_loss=True,
        stop_loss_pct=0.02,  # 2% stop loss
        use_take_profit=True,
        take_profit_pct=0.05,  # 5% take profit
        use_atr_stops=False,  # Use fixed % stops
        trailing_stop=True,
        trailing_stop_pct=0.03,  # 3% trailing stop
        max_risk_per_trade=0.01,  # 1% risk per trade
    )
    
    # Create backtest node
    node = HyperliquidBacktestNode(config)
    node.add_strategy(strategy)
    
    # Run backtest
    print("Running backtest...")
    print()
    
    results = await node.run_async()
    
    # Print results
    print("\n" + "=" * 80)
    print("BACKTEST RESULTS")
    print("=" * 80)
    print(results.summary())
    
    # Equity curve
    if results.equity_curve:
        initial = results.equity_curve[0][1]
        final = results.equity_curve[-1][1]
        returns = ((final - initial) / initial) * 100
        print(f"\nEquity: ${initial:,.2f} → ${final:,.2f} ({returns:+.2f}%)")
    
    # Trade summary
    print(f"\nTotal trades: {len(results.fills)}")
    if results.fills:
        print(f"\nFirst 5 trades:")
        for i, fill in enumerate(results.fills[:5], 1):
            print(f"  {i}. {fill['side']:4s} qty={fill['quantity']:.4f} @ ${fill['price']:,.2f} (fee: ${fill['commission']:.4f})")
        
        if len(results.fills) > 5:
            print(f"  ... and {len(results.fills) - 5} more trades")
    
    return results


if __name__ == "__main__":
    results = asyncio.run(main())
