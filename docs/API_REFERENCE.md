# Neleus API Reference

Complete API reference for Neleus trading framework.

## Table of Contents

- [Strategy API](#strategy-api)
- [Strategy Context](#strategy-context)
- [Market Data Types](#market-data-types)
- [Trading Types](#trading-types)
- [Configuration](#configuration)
- [Backtest API](#backtest-api)
- [Live Trading](#live-trading)
- [Risk Management](#risk-management)
- [Execution Algorithms](#execution-algorithms)

---

## Strategy API

### Strategy Base Class

The `Strategy` class is the base for all trading strategies.

```python
class Strategy(ABC):
    """Base class for trading strategies."""
    
    def __init__(self, strategy_id: Optional[str] = None, config: Optional[Dict] = None)
```

**Parameters:**
- `strategy_id` (str, optional): Unique identifier for this strategy instance
- `config` (dict, optional): Configuration parameters

**Properties:**
- `strategy_id` (str): Strategy identifier
- `is_running` (bool): Whether strategy is active

#### Lifecycle Methods

##### `on_start(ctx: StrategyContext) -> None`

Called when the strategy starts.

```python
def on_start(self, ctx: StrategyContext) -> None:
    """Initialize strategy state and subscriptions."""
    self.instrument = InstrumentId(...)
    ctx.subscribe_bars(self.instrument, "1h")
```

**Use for:**
- Initializing state variables
- Setting up subscriptions
- Loading historical data

##### `on_stop(ctx: StrategyContext) -> None`

Called when the strategy stops.

```python
def on_stop(self, ctx: StrategyContext) -> None:
    """Cleanup and close positions."""
    # Close all positions
    for position in ctx.get_open_positions():
        ctx.close_position(position.instrument_id)
```

**Use for:**
- Closing positions
- Saving state
- Cleanup operations

#### Data Callback Methods

##### `on_bar(ctx: StrategyContext, bar: Bar) -> None`

Called for each new bar (OHLCV candle).

```python
def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
    """Process bar data."""
    print(f"Bar: {bar.close} at {bar.timestamp}")
    
    if self.should_buy(bar):
        ctx.market_order(bar.instrument_id, OrderSide.Buy, 1.0)
```

**Parameters:**
- `ctx` (StrategyContext): Context for placing orders
- `bar` (Bar): Bar data with OHLCV

##### `on_trade(ctx: StrategyContext, trade: TradeTick) -> None`

Called for each trade tick.

```python
def on_trade(self, ctx: StrategyContext, trade: TradeTick) -> None:
    """Process individual trades."""
    print(f"Trade: {trade.price} x {trade.size}")
```

**Parameters:**
- `ctx` (StrategyContext): Strategy context
- `trade` (TradeTick): Trade tick data

##### `on_quote(ctx: StrategyContext, quote: QuoteTick) -> None`

Called for each quote (BBO) update.

```python
def on_quote(self, ctx: StrategyContext, quote: QuoteTick) -> None:
    """Process best bid/offer updates."""
    spread = quote.ask_price - quote.bid_price
    if spread < self.min_spread:
        # Tight spread, good for market making
        pass
```

**Parameters:**
- `ctx` (StrategyContext): Strategy context
- `quote` (QuoteTick): Quote tick with bid/ask

##### `on_order_book(ctx: StrategyContext, book: OrderBook) -> None`

Called for order book updates.

```python
def on_order_book(self, ctx: StrategyContext, book: OrderBook) -> None:
    """Process order book depth."""
    bid_depth = sum(level.size for level in book.bids[:5])
    ask_depth = sum(level.size for level in book.asks[:5])
```

**Parameters:**
- `ctx` (StrategyContext): Strategy context
- `book` (OrderBook): Order book snapshot

##### `on_order_update(ctx: StrategyContext, order: Order) -> None`

Called when order state changes.

```python
def on_order_update(self, ctx: StrategyContext, order: Order) -> None:
    """Handle order state changes."""
    if order.state == OrderState.Filled:
        print(f"Order {order.order_id} filled at {order.filled_price}")
```

**Parameters:**
- `ctx` (StrategyContext): Strategy context
- `order` (Order): Updated order

##### `on_fill(ctx: StrategyContext, fill: Fill) -> None`

Called when an order is filled (partial or complete).

```python
def on_fill(self, ctx: StrategyContext, fill: Fill) -> None:
    """Process fill events."""
    self.total_volume += fill.size
    self.avg_price = (self.avg_price * prev_volume + fill.price * fill.size) / self.total_volume
```

**Parameters:**
- `ctx` (StrategyContext): Strategy context
- `fill` (Fill): Fill information

##### `on_timer(ctx: StrategyContext, timer_id: str) -> None`

Called when a timer expires.

```python
def on_start(self, ctx: StrategyContext) -> None:
    # Set timer for periodic updates
    ctx.set_timer("rebalance", 3600_000)  # 1 hour in ms

def on_timer(self, ctx: StrategyContext, timer_id: str) -> None:
    """Handle timer events."""
    if timer_id == "rebalance":
        self.rebalance_portfolio(ctx)
```

**Parameters:**
- `ctx` (StrategyContext): Strategy context
- `timer_id` (str): Timer identifier

---

## Strategy Context

The `StrategyContext` provides methods for trading operations.

### Order Placement

#### `market_order(instrument: InstrumentId, side: OrderSide, size: float)`

Place a market order.

```python
ctx.market_order(instrument_id, OrderSide.Buy, 1.5)
```

**Parameters:**
- `instrument` (InstrumentId): Trading instrument
- `side` (OrderSide): Buy or Sell
- `size` (float): Order size

**Returns:** Order ID

#### `limit_order(instrument: InstrumentId, side: OrderSide, size: float, price: float)`

Place a limit order.

```python
order_id = ctx.limit_order(instrument_id, OrderSide.Buy, 1.0, 50000.0)
```

**Parameters:**
- `instrument` (InstrumentId): Trading instrument
- `side` (OrderSide): Buy or Sell
- `size` (float): Order size
- `price` (float): Limit price

**Returns:** Order ID

#### `stop_order(instrument: InstrumentId, side: OrderSide, size: float, stop_price: float)`

Place a stop-loss order.

```python
ctx.stop_order(instrument_id, OrderSide.Sell, 1.0, 49000.0)
```

**Parameters:**
- `instrument` (InstrumentId): Trading instrument
- `side` (OrderSide): Buy or Sell
- `size` (float): Order size
- `stop_price` (float): Stop trigger price

#### `cancel_order(order_id: str)`

Cancel an existing order.

```python
ctx.cancel_order(order_id)
```

#### `cancel_all_orders(instrument: Optional[InstrumentId] = None)`

Cancel all orders, optionally filtered by instrument.

```python
# Cancel all orders
ctx.cancel_all_orders()

# Cancel orders for specific instrument
ctx.cancel_all_orders(instrument_id)
```

### Position Management

#### `get_position(instrument: InstrumentId) -> Optional[Position]`

Get current position for an instrument.

```python
position = ctx.get_position(instrument_id)
if position:
    print(f"Size: {position.size}, PnL: {position.unrealized_pnl}")
```

**Returns:** Position object or None

#### `get_open_positions() -> List[Position]`

Get all open positions.

```python
for position in ctx.get_open_positions():
    print(f"{position.instrument_id}: {position.size}")
```

#### `close_position(instrument: InstrumentId)`

Close an entire position.

```python
ctx.close_position(instrument_id)
```

### Account Information

#### `get_balance() -> float`

Get current account balance.

```python
balance = ctx.get_balance()
print(f"Available: ${balance:,.2f}")
```

#### `get_equity() -> float`

Get total equity (balance + unrealized PnL).

```python
equity = ctx.get_equity()
```

### Subscriptions

#### `subscribe_bars(instrument: InstrumentId, interval: str)`

Subscribe to bar data.

```python
ctx.subscribe_bars(instrument_id, "1h")  # 1-hour bars
ctx.subscribe_bars(instrument_id, "5m")  # 5-minute bars
```

**Intervals:** `"1m"`, `"5m"`, `"15m"`, `"1h"`, `"4h"`, `"1d"`

#### `subscribe_trades(instrument: InstrumentId)`

Subscribe to trade ticks.

```python
ctx.subscribe_trades(instrument_id)
```

#### `subscribe_quotes(instrument: InstrumentId)`

Subscribe to quote ticks (BBO).

```python
ctx.subscribe_quotes(instrument_id)
```

#### `subscribe_order_book(instrument: InstrumentId, depth: int = 10)`

Subscribe to order book updates.

```python
ctx.subscribe_order_book(instrument_id, depth=20)
```

### Timers

#### `set_timer(timer_id: str, interval_ms: int)`

Set a recurring timer.

```python
# Timer fires every 5 minutes
ctx.set_timer("check_spread", 300_000)
```

#### `cancel_timer(timer_id: str)`

Cancel a timer.

```python
ctx.cancel_timer("check_spread")
```

---

## Market Data Types

### InstrumentId

Unique identifier for a trading instrument.

```python
class InstrumentId:
    venue: Venue
    symbol: str
    instrument_type: InstrumentType
```

**Example:**
```python
instrument = InstrumentId(
    venue=Venue.Hyperliquid,
    symbol="ETH",
    instrument_type=InstrumentType.Perp,
)
```

### Bar

OHLCV bar/candlestick data.

```python
class Bar:
    instrument_id: InstrumentId
    timestamp: int          # Unix nanoseconds
    open: float
    high: float
    low: float
    close: float
    volume: float
    interval: str          # e.g., "1h"
```

**Usage:**
```python
def on_bar(self, ctx: StrategyContext, bar: Bar):
    print(f"Close: {bar.close}, Volume: {bar.volume}")
    # Calculate returns
    if self.prev_close:
        returns = (bar.close - self.prev_close) / self.prev_close
    self.prev_close = bar.close
```

### TradeTick

Individual trade execution.

```python
class TradeTick:
    instrument_id: InstrumentId
    timestamp: int
    price: float
    size: float
    side: OrderSide       # Buy or Sell
    trade_id: str
```

### QuoteTick

Best bid and offer (BBO).

```python
class QuoteTick:
    instrument_id: InstrumentId
    timestamp: int
    bid_price: float
    bid_size: float
    ask_price: float
    ask_size: float
```

**Usage:**
```python
def on_quote(self, ctx: StrategyContext, quote: QuoteTick):
    spread = quote.ask_price - quote.bid_price
    mid_price = (quote.bid_price + quote.ask_price) / 2
```

### OrderBook

Full order book depth.

```python
class OrderBook:
    instrument_id: InstrumentId
    timestamp: int
    bids: List[BookLevel]  # Sorted by price descending
    asks: List[BookLevel]  # Sorted by price ascending
    sequence: int
```

**BookLevel:**
```python
class BookLevel:
    price: float
    size: float
```

**Usage:**
```python
def on_order_book(self, ctx: StrategyContext, book: OrderBook):
    # Top 5 levels
    for i, level in enumerate(book.bids[:5]):
        print(f"Bid {i}: {level.price} @ {level.size}")
    
    # Calculate total depth
    bid_depth = sum(level.size for level in book.bids)
    ask_depth = sum(level.size for level in book.asks)
```

---

## Trading Types

### Order

Represents a trading order.

```python
class Order:
    order_id: str
    instrument_id: InstrumentId
    side: OrderSide
    order_type: OrderType
    size: float
    price: Optional[float]
    stop_price: Optional[float]
    time_in_force: TimeInForce
    state: OrderState
    filled_size: float
    filled_price: Optional[float]
    timestamp: int
```

### Fill

Represents an order fill (execution).

```python
class Fill:
    fill_id: str
    order_id: str
    instrument_id: InstrumentId
    side: OrderSide
    price: float
    size: float
    commission: float
    timestamp: int
```

### Position

Represents an open position.

```python
class Position:
    instrument_id: InstrumentId
    side: PositionSide      # Long or Short
    size: float
    entry_price: float
    current_price: float
    unrealized_pnl: float
    realized_pnl: float
    timestamp: int
```

### Enums

#### OrderSide
```python
class OrderSide(Enum):
    Buy = "buy"
    Sell = "sell"
```

#### OrderType
```python
class OrderType(Enum):
    Market = "market"
    Limit = "limit"
    StopMarket = "stop_market"
    StopLimit = "stop_limit"
    TakeProfit = "take_profit"
    TrailingStop = "trailing_stop"
```

#### OrderState
```python
class OrderState(Enum):
    Pending = "pending"
    Open = "open"
    PartiallyFilled = "partially_filled"
    Filled = "filled"
    Cancelled = "cancelled"
    Rejected = "rejected"
    Expired = "expired"
```

#### TimeInForce
```python
class TimeInForce(Enum):
    GTC = "gtc"  # Good Till Cancelled
    IOC = "ioc"  # Immediate Or Cancel
    FOK = "fok"  # Fill Or Kill
    GTD = "gtd"  # Good Till Date
    DAY = "day"  # Good for day
```

#### Venue
```python
class Venue(Enum):
    Hyperliquid = "hyperliquid"
    Lighter = "lighter"
    Polymarket = "polymarket"
    Simulated = "simulated"
```

#### InstrumentType
```python
class InstrumentType(Enum):
    Spot = "spot"
    Perp = "perp"          # Perpetual futures
    Future = "future"
    Option = "option"
```

---

## Configuration

### BacktestConfig

Configuration for backtesting.

```python
@dataclass
class BacktestConfig:
    initial_capital: float = 100000.0
    commission_bps: float = 5.0          # 5 basis points
    slippage_bps: float = 2.0            # 2 basis points
    start_date: Optional[str] = None     # "YYYY-MM-DD"
    end_date: Optional[str] = None
    fill_model: FillModel = FillModel.Immediate
    latency_model: LatencyModel = LatencyModel.Zero
```

**Example:**
```python
config = BacktestConfig(
    initial_capital=100000.0,
    commission_bps=5.0,
    slippage_bps=2.0,
    start_date="2024-01-01",
    end_date="2024-06-01",
    fill_model=FillModel.NextTick,
)
```

### RiskConfig

Risk management configuration.

```python
@dataclass
class RiskConfig:
    max_position_pct: float = 10.0        # Max 10% per position
    max_leverage: float = 5.0
    max_daily_loss_pct: float = 5.0       # Kill switch at 5% daily loss
    max_drawdown_pct: float = 20.0
    position_limits: Dict[str, float] = field(default_factory=dict)
    concentration_limit_pct: float = 25.0
    dynamic_limits: bool = True
```

---

## Backtest API

### backtest() Function

Run a backtest.

```python
def backtest(
    strategy: Strategy,
    instrument: InstrumentId,
    config: BacktestConfig,
    data: Optional[List[Bar]] = None,
) -> BacktestResults:
    """Run a strategy backtest."""
```

**Example:**
```python
from neleus import backtest, BacktestConfig, InstrumentId, Venue, InstrumentType

strategy = MyStrategy()
instrument = InstrumentId(Venue.Hyperliquid, "ETH", InstrumentType.Perp)
config = BacktestConfig(initial_capital=100000.0)

results = backtest(strategy, instrument, config)
print(results.summary())
```

### BacktestResults

Results from a backtest.

```python
class BacktestResults:
    return_pct: float
    sharpe_ratio: float
    sortino_ratio: float
    max_drawdown_pct: float
    total_trades: int
    winning_trades: int
    losing_trades: int
    total_commission: float
    equity_curve: List[float]
    
    def win_rate(self) -> float:
        """Calculate win rate percentage."""
        
    def profit_factor(self) -> float:
        """Calculate profit factor."""
        
    def summary(self) -> str:
        """Generate summary report."""
```

---

## Live Trading

### LiveNode

Node for live trading.

```python
from neleus import LiveNode, HyperliquidVenueConfig, Network

# Configure venue
venue_config = HyperliquidVenueConfig(
    network=Network.Testnet,
    wallet_address="0x...",
    api_key="...",
)

# Create live node
node = LiveNode(venue_config)

# Add strategy
node.add_strategy(MyStrategy())

# Start trading
await node.start()
```

### PaperNode

Paper trading (simulated with live data).

```python
from neleus import PaperNode

node = PaperNode(
    venue_config=venue_config,
    initial_capital=100000.0,
)
node.add_strategy(strategy)
await node.start()
```

---

## Risk Management

### Stop Loss Configuration

```python
from neleus import StopLossType, StopLossConfig

stop_config = StopLossConfig(
    type=StopLossType.ATR,
    atr_period=14,
    atr_multiplier=2.0,
)
```

### Position Sizing

```python
from neleus import PositionSizingMethod

# Kelly Criterion
size = ctx.calculate_position_size(
    method=PositionSizingMethod.Kelly,
    win_rate=0.6,
    avg_win_loss_ratio=2.0,
)

# Volatility-based
size = ctx.calculate_position_size(
    method=PositionSizingMethod.VolatilityBased,
    target_volatility=0.15,
)
```

---

## Execution Algorithms

### TWAP (Time-Weighted Average Price)

```python
from neleus import TwapParams

twap = TwapParams(
    total_size=10.0,
    duration_secs=3600,  # 1 hour
    num_slices=12,       # Execute every 5 minutes
)

ctx.execute_twap(instrument_id, OrderSide.Buy, twap)
```

### VWAP (Volume-Weighted Average Price)

```python
from neleus import VwapParams

vwap = VwapParams(
    total_size=10.0,
    target_participation=0.10,  # 10% of market volume
)

ctx.execute_vwap(instrument_id, OrderSide.Buy, vwap)
```

### Iceberg Orders

```python
from neleus import IcebergParams

iceberg = IcebergParams(
    total_size=100.0,
    display_size=10.0,   # Show only 10 at a time
    price_limit=50000.0,
)

ctx.execute_iceberg(instrument_id, OrderSide.Buy, iceberg)
```

---

## Utility Functions

### Data Loading

```python
from neleus import load_historical_data

bars = load_historical_data(
    venue=Venue.Hyperliquid,
    symbol="ETH",
    interval="1h",
    start="2024-01-01",
    end="2024-06-01",
)
```

### Performance Analysis

```python
from neleus.plots import BacktestPlotter

plotter = BacktestPlotter(results)
plotter.plot_equity_curve()
plotter.plot_drawdown()
plotter.plot_monthly_returns()
plotter.save_html_report("backtest_results.html")
```
