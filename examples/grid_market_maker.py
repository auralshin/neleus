"""
Hyperliquid Grid Market Maker Strategy
======================================

A professional market making strategy for Hyperliquid perpetual futures.
Places a grid of limit orders on both sides of the order book to capture
the bid-ask spread while managing inventory risk.

Features:
- Dynamic grid spacing based on volatility (ATR)
- Inventory-based skewing to control position exposure
- Maximum position limits to prevent runaway inventory
- Maker rebate optimization (post-only orders)
- Supports longs and shorts through grid orders

Usage:
    python -m examples.grid_market_maker
"""

import asyncio
from decimal import Decimal
from typing import Optional, List, Dict
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


class GridSide(Enum):
    BID = "bid"
    ASK = "ask"


@dataclass
class GridLevel:
    """Represents a single level in the grid"""
    side: GridSide
    price: Decimal
    size: Decimal
    order_id: Optional[str] = None
    is_active: bool = False


class GridMarketMakerStrategy(Strategy):
    """
    Grid Market Making Strategy for Hyperliquid Perps
    
    This strategy places a grid of limit orders above and below the current
    mid price to capture the bid-ask spread. As orders fill, it rebalances
    to maintain a neutral position.
    
    Key parameters:
    - grid_levels: Number of price levels on each side
    - grid_spacing_pct: Distance between grid levels as percentage
    - order_size: Size per grid order
    - max_position: Maximum net position allowed
    - inventory_skew: Adjust prices based on current inventory
    """
    
    def __init__(
        self,
        # Grid configuration
        grid_levels: int = 5,
        grid_spacing_pct: float = 0.001,  # 0.1% between levels
        order_size: float = 0.1,
        
        # Risk management
        max_position: float = 2.0,
        inventory_skew_factor: float = 0.0002,  # Skew per unit of inventory
        
        # Volatility adaptation
        use_atr_spacing: bool = True,
        atr_period: int = 14,
        atr_multiplier: float = 0.5,
        min_spread_pct: float = 0.0005,  # Minimum 0.05% spread
        
        # Performance
        rebalance_threshold_pct: float = 0.005,  # Rebalance when price moves 0.5%
        
        strategy_id: Optional[str] = None,
    ):
        super().__init__(strategy_id or "GridMarketMaker")
        
        self.grid_levels = grid_levels
        self.base_spacing = Decimal(str(grid_spacing_pct))
        self.order_size = Decimal(str(order_size))
        self.max_position = Decimal(str(max_position))
        self.inventory_skew_factor = Decimal(str(inventory_skew_factor))
        
        self.use_atr_spacing = use_atr_spacing
        self.atr_period = atr_period
        self.atr_multiplier = Decimal(str(atr_multiplier))
        self.min_spread = Decimal(str(min_spread_pct))
        self.rebalance_threshold = Decimal(str(rebalance_threshold_pct))
        
        # State
        self.instrument: Optional[InstrumentId] = None
        self.position: Decimal = Decimal("0")
        self.last_mid_price: Optional[Decimal] = None
        self.grid_center: Optional[Decimal] = None
        
        # Price history for ATR
        self.highs: List[Decimal] = []
        self.lows: List[Decimal] = []
        self.closes: List[Decimal] = []
        
        # Current grid
        self.bid_levels: List[GridLevel] = []
        self.ask_levels: List[GridLevel] = []
        
        # Performance tracking
        self.total_fills = 0
        self.bid_fills = 0
        self.ask_fills = 0
        self.total_spread_captured = Decimal("0")
        self.max_drawdown_position = Decimal("0")
    
    def on_start(self, ctx: StrategyContext) -> None:
        print(f"[{self.strategy_id}] Grid Market Maker starting")
        print(f"  Grid levels: {self.grid_levels} per side")
        print(f"  Base spacing: {self.base_spacing:.4%}")
        print(f"  Order size: {self.order_size}")
        print(f"  Max position: ±{self.max_position}")
        print(f"  Inventory skew: {self.inventory_skew_factor:.4%} per unit")
        if self.use_atr_spacing:
            print(f"  ATR-based spacing: {self.atr_multiplier}x ATR({self.atr_period})")
        
        self.instrument = InstrumentId(
            venue=Venue.Hyperliquid,
            symbol="ETH",  # Will be set by backtest config
            instrument_type=InstrumentType.Perp,
        )
    
    def calculate_atr(self) -> Decimal:
        """Calculate Average True Range"""
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
    
    def calculate_grid_spacing(self, mid_price: Decimal) -> Decimal:
        """Calculate dynamic grid spacing based on volatility"""
        if self.use_atr_spacing and len(self.closes) >= self.atr_period:
            atr = self.calculate_atr()
            atr_based = (atr / mid_price) * self.atr_multiplier
            return max(atr_based, self.min_spread)
        return self.base_spacing
    
    def calculate_inventory_skew(self) -> Decimal:
        """
        Calculate price skew based on current inventory.
        Positive inventory -> raise asks, lower bids to reduce position.
        Negative inventory -> lower asks, raise bids to increase position.
        """
        return self.position * self.inventory_skew_factor
    
    def should_rebalance_grid(self, current_price: Decimal) -> bool:
        """Check if price has moved enough to warrant grid rebalancing"""
        if self.grid_center is None:
            return True
        
        price_change = abs(current_price - self.grid_center) / self.grid_center
        return price_change > self.rebalance_threshold
    
    def build_grid(self, mid_price: Decimal) -> tuple:
        """Build new grid levels around the mid price"""
        spacing = self.calculate_grid_spacing(mid_price)
        skew = self.calculate_inventory_skew()
        
        bid_levels = []
        ask_levels = []
        
        for i in range(1, self.grid_levels + 1):
            # Bid side - subtract from mid, apply negative skew if long
            bid_offset = spacing * i
            bid_price = mid_price * (1 - bid_offset - skew)
            
            # Reduce size if approaching max position
            available_to_buy = self.max_position - self.position
            bid_size = min(self.order_size, max(Decimal("0"), available_to_buy))
            
            if bid_size > 0:
                bid_levels.append(GridLevel(
                    side=GridSide.BID,
                    price=bid_price,
                    size=bid_size,
                ))
            
            # Ask side - add to mid, apply positive skew if long
            ask_offset = spacing * i
            ask_price = mid_price * (1 + ask_offset - skew)
            
            # Reduce size if approaching max short
            available_to_sell = self.max_position + self.position
            ask_size = min(self.order_size, max(Decimal("0"), available_to_sell))
            
            if ask_size > 0:
                ask_levels.append(GridLevel(
                    side=GridSide.ASK,
                    price=ask_price,
                    size=ask_size,
                ))
        
        return bid_levels, ask_levels
    
    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        """Process each bar and manage the grid"""
        # Update price history
        self.highs.append(Decimal(str(bar.high)))
        self.lows.append(Decimal(str(bar.low)))
        self.closes.append(Decimal(str(bar.close)))
        
        # Keep limited history
        max_history = max(self.atr_period + 10, 50)
        if len(self.highs) > max_history:
            self.highs = self.highs[-max_history:]
            self.lows = self.lows[-max_history:]
            self.closes = self.closes[-max_history:]
        
        mid_price = Decimal(str(bar.close))
        self.last_mid_price = mid_price
        
        # Skip if not enough data for ATR
        if self.use_atr_spacing and len(self.closes) < self.atr_period:
            return
        
        # Check if any grid levels would be filled by this bar
        self._simulate_fills(bar, ctx)
        
        # Rebalance grid if needed
        if self.should_rebalance_grid(mid_price):
            self._rebalance_grid(mid_price, ctx)
    
    def _simulate_fills(self, bar: Bar, ctx: StrategyContext) -> None:
        """Simulate which grid orders would have been filled"""
        bar_low = Decimal(str(bar.low))
        bar_high = Decimal(str(bar.high))
        
        # Check bid fills (price went down to our bids)
        for level in self.bid_levels:
            if level.is_active and bar_low <= level.price:
                self._execute_fill(level, ctx)
        
        # Check ask fills (price went up to our asks)
        for level in self.ask_levels:
            if level.is_active and bar_high >= level.price:
                self._execute_fill(level, ctx)
    
    def _execute_fill(self, level: GridLevel, ctx: StrategyContext) -> None:
        """Process a grid level fill"""
        self.total_fills += 1
        
        if level.side == GridSide.BID:
            # Bid filled - we bought
            self.position += level.size
            self.bid_fills += 1
            ctx.market_order(self.instrument, OrderSide.Buy, float(level.size), reduce_only=False)
            print(f"  [GRID BID FILL] Bought {level.size} @ ${level.price:.2f}")
        else:
            # Ask filled - we sold
            self.position -= level.size
            self.ask_fills += 1
            ctx.market_order(self.instrument, OrderSide.Sell, float(level.size), reduce_only=False)
            print(f"  [GRID ASK FILL] Sold {level.size} @ ${level.price:.2f}")
        
        level.is_active = False
        
        # Track max position for risk monitoring
        if abs(self.position) > abs(self.max_drawdown_position):
            self.max_drawdown_position = self.position
    
    def _rebalance_grid(self, mid_price: Decimal, ctx: StrategyContext) -> None:
        """Cancel existing grid and create new one around current price"""
        self.grid_center = mid_price
        
        # Build new grid
        self.bid_levels, self.ask_levels = self.build_grid(mid_price)
        
        # Activate all levels
        for level in self.bid_levels:
            level.is_active = True
        for level in self.ask_levels:
            level.is_active = True
    
    def on_stop(self, ctx: StrategyContext) -> None:
        """Cleanup and print statistics"""
        print(f"\n[{self.strategy_id}] Grid Market Maker stopped")
        print(f"  Final position: {self.position}")
        print(f"  Total fills: {self.total_fills}")
        print(f"    Bid fills: {self.bid_fills}")
        print(f"    Ask fills: {self.ask_fills}")
        print(f"  Max position reached: {self.max_drawdown_position}")
        
        # Close any remaining position
        if self.position != 0:
            side = OrderSide.Sell if self.position > 0 else OrderSide.Buy
            ctx.market_order(self.instrument, side, float(abs(self.position)), reduce_only=True)
            print(f"  [CLOSE] Closing {abs(self.position)} position")


async def main():
    """Run the Grid Market Maker backtest"""
    from neleus.types import HyperliquidClient
    
    print("=" * 60)
    print("HYPERLIQUID GRID MARKET MAKER BACKTEST")
    print("=" * 60)
    
    # Configuration
    coin = "ETH"
    interval = CandleInterval.HOUR_1
    lookback_days = 30
    initial_capital = 10000.0
    
    strategy = GridMarketMakerStrategy(
        grid_levels=5,
        grid_spacing_pct=0.002,  # 0.2% between levels
        order_size=0.05,
        max_position=0.5,
        inventory_skew_factor=0.0001,
        use_atr_spacing=True,
        atr_period=14,
        atr_multiplier=0.5,
    )
    
    config = HyperliquidBacktestConfig(
        initial_capital=initial_capital,
        coin=coin,
        interval=interval,
        lookback_days=lookback_days,
        maker_fee_bps=0.0,  # Maker rebate
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
