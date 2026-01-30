# Neleus Examples & Tutorials

Complete collection of examples and tutorials for Neleus trading strategies.

## Table of Contents

1. [Basic Examples](#basic-examples)
2. [Momentum Strategies](#momentum-strategies)
3. [Mean Reversion Strategies](#mean-reversion-strategies)
4. [Market Making](#market-making)
5. [Multi-Instrument Strategies](#multi-instrument-strategies)
6. [Risk Management](#risk-management)
7. [Advanced Examples](#advanced-examples)

---

## Basic Examples

### Example 1: Simple Buy and Hold

The simplest possible strategy - buy once and hold.

```python
from neleus import Strategy, StrategyContext, Bar, OrderSide

class BuyAndHoldStrategy(Strategy):
    def __init__(self):
        super().__init__("BuyAndHold")
        self.bought = False
    
    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        """Buy on first bar and hold."""
        if not self.bought:
            ctx.market_order(bar.instrument_id, OrderSide.Buy, 1.0)
            self.bought = True
```

**Run it:**
```bash
neleus backtest --strategy buy_and_hold --symbol BTC-PERP
```

---

### Example 2: Simple Moving Average Crossover

Classic two-SMA crossover strategy.

```python
from neleus import Strategy, StrategyContext, Bar, OrderSide
from typing import List

class SMACrossoverStrategy(Strategy):
    def __init__(self, fast_period: int = 10, slow_period: int = 30):
        super().__init__("SMACrossover")
        self.fast_period = fast_period
        self.slow_period = slow_period
        self.prices: List[float] = []
        self.position = 0  # 0 = no position, 1 = long, -1 = short
    
    def calculate_sma(self, period: int) -> float:
        """Calculate simple moving average."""
        if len(self.prices) < period:
            return None
        return sum(self.prices[-period:]) / period
    
    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        """Generate signals on SMA crossover."""
        self.prices.append(bar.close)
        
        # Need enough data for slow SMA
        if len(self.prices) < self.slow_period:
            return
        
        fast_sma = self.calculate_sma(self.fast_period)
        slow_sma = self.calculate_sma(self.slow_period)
        
        # Golden cross - fast crosses above slow
        if fast_sma > slow_sma and self.position <= 0:
            if self.position == -1:
                # Close short first
                ctx.market_order(bar.instrument_id, OrderSide.Buy, 1.0)
            # Go long
            ctx.market_order(bar.instrument_id, OrderSide.Buy, 1.0)
            self.position = 1
            print(f"BUY: Fast SMA {fast_sma:.2f} > Slow SMA {slow_sma:.2f}")
        
        # Death cross - fast crosses below slow
        elif fast_sma < slow_sma and self.position >= 0:
            if self.position == 1:
                # Close long first
                ctx.market_order(bar.instrument_id, OrderSide.Sell, 1.0)
            # Go short
            ctx.market_order(bar.instrument_id, OrderSide.Sell, 1.0)
            self.position = -1
            print(f"SELL: Fast SMA {fast_sma:.2f} < Slow SMA {slow_sma:.2f}")
```

**Backtest:**
```python
from neleus import backtest, BacktestConfig, InstrumentId, Venue, InstrumentType

strategy = SMACrossoverStrategy(fast_period=10, slow_period=30)
instrument = InstrumentId(Venue.Hyperliquid, "BTC", InstrumentType.Perp)
config = BacktestConfig(initial_capital=100000.0, start_date="2024-01-01")

results = backtest(strategy, instrument, config)
print(results.summary())
```

---

## Momentum Strategies

### Example 3: Rate of Change (ROC) Momentum

Trade based on price momentum.

```python
from neleus import Strategy, StrategyContext, Bar, OrderSide
from typing import List

class MomentumStrategy(Strategy):
    def __init__(
        self, 
        lookback: int = 20,
        entry_threshold: float = 0.02,  # 2%
        exit_threshold: float = 0.005,  # 0.5%
        position_size: float = 1.0
    ):
        super().__init__("Momentum")
        self.lookback = lookback
        self.entry_threshold = entry_threshold
        self.exit_threshold = exit_threshold
        self.position_size = position_size
        
        self.prices: List[float] = []
        self.position = 0
    
    def calculate_momentum(self) -> float:
        """Calculate rate of change."""
        if len(self.prices) < self.lookback:
            return 0.0
        return (self.prices[-1] - self.prices[-self.lookback]) / self.prices[-self.lookback]
    
    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        self.prices.append(bar.close)
        
        if len(self.prices) < self.lookback:
            return
        
        momentum = self.calculate_momentum()
        
        # Entry logic
        if self.position == 0:
            if momentum > self.entry_threshold:
                # Strong positive momentum
                ctx.market_order(bar.instrument_id, OrderSide.Buy, self.position_size)
                self.position = 1
                print(f"LONG entry: momentum = {momentum:.2%}")
            
            elif momentum < -self.entry_threshold:
                # Strong negative momentum
                ctx.market_order(bar.instrument_id, OrderSide.Sell, self.position_size)
                self.position = -1
                print(f"SHORT entry: momentum = {momentum:.2%}")
        
        # Exit logic
        elif self.position == 1 and momentum < -self.exit_threshold:
            # Exit long
            ctx.market_order(bar.instrument_id, OrderSide.Sell, self.position_size)
            self.position = 0
            print(f"LONG exit: momentum = {momentum:.2%}")
        
        elif self.position == -1 and momentum > self.exit_threshold:
            # Exit short
            ctx.market_order(bar.instrument_id, OrderSide.Buy, self.position_size)
            self.position = 0
            print(f"SHORT exit: momentum = {momentum:.2%}")
```

**Usage:**
```bash
neleus backtest \
  --strategy momentum \
  --symbol ETH-PERP \
  --timeframe 1h \
  --start 2024-01-01 \
  --capital 100000
```

---

### Example 4: RSI Momentum

Use Relative Strength Index for momentum trading.

```python
from neleus import Strategy, StrategyContext, Bar, OrderSide
from typing import List

class RSIStrategy(Strategy):
    def __init__(
        self,
        period: int = 14,
        oversold: float = 30.0,
        overbought: float = 70.0,
    ):
        super().__init__("RSI")
        self.period = period
        self.oversold = oversold
        self.overbought = overbought
        
        self.prices: List[float] = []
        self.position = 0
    
    def calculate_rsi(self) -> float:
        """Calculate RSI."""
        if len(self.prices) < self.period + 1:
            return 50.0  # Neutral
        
        # Calculate price changes
        changes = [self.prices[i] - self.prices[i-1] 
                   for i in range(1, len(self.prices))]
        
        gains = [max(0, change) for change in changes[-self.period:]]
        losses = [-min(0, change) for change in changes[-self.period:]]
        
        avg_gain = sum(gains) / self.period
        avg_loss = sum(losses) / self.period
        
        if avg_loss == 0:
            return 100.0
        
        rs = avg_gain / avg_loss
        rsi = 100 - (100 / (1 + rs))
        return rsi
    
    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        self.prices.append(bar.close)
        
        if len(self.prices) < self.period + 1:
            return
        
        rsi = self.calculate_rsi()
        
        # Buy when oversold
        if rsi < self.oversold and self.position != 1:
            if self.position == -1:
                ctx.market_order(bar.instrument_id, OrderSide.Buy, 1.0)
            ctx.market_order(bar.instrument_id, OrderSide.Buy, 1.0)
            self.position = 1
            print(f"BUY: RSI = {rsi:.2f} (oversold)")
        
        # Sell when overbought
        elif rsi > self.overbought and self.position != -1:
            if self.position == 1:
                ctx.market_order(bar.instrument_id, OrderSide.Sell, 1.0)
            ctx.market_order(bar.instrument_id, OrderSide.Sell, 1.0)
            self.position = -1
            print(f"SELL: RSI = {rsi:.2f} (overbought)")
```

---

## Mean Reversion Strategies

### Example 5: Bollinger Bands

Classic mean reversion using Bollinger Bands.

```python
from neleus import Strategy, StrategyContext, Bar, OrderSide
from typing import List
import math

class BollingerBandsStrategy(Strategy):
    def __init__(
        self,
        period: int = 20,
        num_std: float = 2.0,
        position_size: float = 1.0,
    ):
        super().__init__("BollingerBands")
        self.period = period
        self.num_std = num_std
        self.position_size = position_size
        
        self.prices: List[float] = []
        self.position = 0
    
    def calculate_bollinger_bands(self):
        """Calculate Bollinger Bands."""
        if len(self.prices) < self.period:
            return None, None, None
        
        recent = self.prices[-self.period:]
        
        # Simple Moving Average
        sma = sum(recent) / self.period
        
        # Standard Deviation
        variance = sum((p - sma) ** 2 for p in recent) / self.period
        std = math.sqrt(variance)
        
        # Bands
        upper = sma + (self.num_std * std)
        lower = sma - (self.num_std * std)
        
        return upper, sma, lower
    
    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        self.prices.append(bar.close)
        
        if len(self.prices) < self.period:
            return
        
        upper, middle, lower = self.calculate_bollinger_bands()
        current_price = bar.close
        
        # Entry signals
        if self.position == 0:
            if current_price < lower:
                # Price below lower band - buy (expect reversion up)
                ctx.market_order(bar.instrument_id, OrderSide.Buy, self.position_size)
                self.position = 1
                print(f"BUY: Price {current_price:.2f} < Lower Band {lower:.2f}")
            
            elif current_price > upper:
                # Price above upper band - sell (expect reversion down)
                ctx.market_order(bar.instrument_id, OrderSide.Sell, self.position_size)
                self.position = -1
                print(f"SELL: Price {current_price:.2f} > Upper Band {upper:.2f}")
        
        # Exit at middle band (SMA)
        elif self.position == 1 and current_price >= middle:
            ctx.market_order(bar.instrument_id, OrderSide.Sell, self.position_size)
            self.position = 0
            print(f"EXIT LONG: Price {current_price:.2f} reached middle {middle:.2f}")
        
        elif self.position == -1 and current_price <= middle:
            ctx.market_order(bar.instrument_id, OrderSide.Buy, self.position_size)
            self.position = 0
            print(f"EXIT SHORT: Price {current_price:.2f} reached middle {middle:.2f}")
```

---

## Market Making

### Example 6: Simple Market Maker

Basic market making strategy with bid-ask spread.

```python
from neleus import Strategy, StrategyContext, QuoteTick, OrderSide, OrderType

class SimpleMarketMaker(Strategy):
    def __init__(
        self,
        spread_bps: float = 10.0,  # 10 basis points
        order_size: float = 1.0,
        max_position: float = 10.0,
    ):
        super().__init__("MarketMaker")
        self.spread_bps = spread_bps
        self.order_size = order_size
        self.max_position = max_position
        
        self.bid_order_id = None
        self.ask_order_id = None
    
    def on_start(self, ctx: StrategyContext) -> None:
        """Subscribe to quotes."""
        from neleus import InstrumentId, Venue, InstrumentType
        self.instrument = InstrumentId(Venue.Hyperliquid, "ETH", InstrumentType.Perp)
        ctx.subscribe_quotes(self.instrument)
    
    def on_quote(self, ctx: StrategyContext, quote: QuoteTick) -> None:
        """Update quotes to make markets."""
        # Cancel existing orders
        if self.bid_order_id:
            ctx.cancel_order(self.bid_order_id)
        if self.ask_order_id:
            ctx.cancel_order(self.ask_order_id)
        
        # Calculate mid price
        mid_price = (quote.bid_price + quote.ask_price) / 2
        spread = mid_price * (self.spread_bps / 10000)
        
        # Get current position
        position = ctx.get_position(self.instrument)
        current_size = position.size if position else 0.0
        
        # Place new orders (respecting position limits)
        if abs(current_size) < self.max_position:
            # Place buy order
            bid_price = mid_price - spread / 2
            self.bid_order_id = ctx.limit_order(
                self.instrument,
                OrderSide.Buy,
                self.order_size,
                bid_price
            )
            
            # Place sell order
            ask_price = mid_price + spread / 2
            self.ask_order_id = ctx.limit_order(
                self.instrument,
                OrderSide.Sell,
                self.order_size,
                ask_price
            )
```

---

## Multi-Instrument Strategies

### Example 7: Pairs Trading

Statistical arbitrage between correlated instruments.

```python
from neleus import Strategy, StrategyContext, Bar, OrderSide, InstrumentId
from typing import Dict, List

class PairsTradingStrategy(Strategy):
    def __init__(
        self,
        lookback: int = 20,
        entry_zscore: float = 2.0,
        exit_zscore: float = 0.5,
        position_size: float = 1.0,
    ):
        super().__init__("PairsTrading")
        self.lookback = lookback
        self.entry_zscore = entry_zscore
        self.exit_zscore = exit_zscore
        self.position_size = position_size
        
        # Store prices for both instruments
        self.prices: Dict[str, List[float]] = {}
        self.position = 0  # 0 = flat, 1 = long spread, -1 = short spread
    
    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        """Track prices for both instruments."""
        symbol = bar.instrument_id.symbol
        
        if symbol not in self.prices:
            self.prices[symbol] = []
        
        self.prices[symbol].append(bar.close)
        
        # Need both instruments
        if len(self.prices) != 2:
            return
        
        # Get the two symbols
        symbols = list(self.prices.keys())
        sym1, sym2 = symbols[0], symbols[1]
        
        # Need enough data
        if len(self.prices[sym1]) < self.lookback or len(self.prices[sym2]) < self.lookback:
            return
        
        # Calculate spread
        spread = [self.prices[sym1][i] - self.prices[sym2][i] 
                 for i in range(len(self.prices[sym1]))]
        
        recent_spread = spread[-self.lookback:]
        
        # Calculate z-score
        mean_spread = sum(recent_spread) / len(recent_spread)
        std_spread = (sum((s - mean_spread)**2 for s in recent_spread) / len(recent_spread)) ** 0.5
        
        if std_spread == 0:
            return
        
        current_spread = spread[-1]
        zscore = (current_spread - mean_spread) / std_spread
        
        # Trading logic
        if self.position == 0:
            if zscore > self.entry_zscore:
                # Spread too high - short spread (short sym1, long sym2)
                ctx.market_order(bar.instrument_id, OrderSide.Sell, self.position_size)
                self.position = -1
                print(f"SHORT SPREAD: z-score = {zscore:.2f}")
            
            elif zscore < -self.entry_zscore:
                # Spread too low - long spread (long sym1, short sym2)
                ctx.market_order(bar.instrument_id, OrderSide.Buy, self.position_size)
                self.position = 1
                print(f"LONG SPREAD: z-score = {zscore:.2f}")
        
        # Exit logic
        elif abs(zscore) < self.exit_zscore:
            # Spread reverted to mean - exit
            if self.position == 1:
                ctx.market_order(bar.instrument_id, OrderSide.Sell, self.position_size)
            else:
                ctx.market_order(bar.instrument_id, OrderSide.Buy, self.position_size)
            
            self.position = 0
            print(f"EXIT: z-score = {zscore:.2f}")
```

**Usage:**
```python
# Backtest BTC-ETH pairs trade
strategy = PairsTradingStrategy()
# Subscribe to both instruments in on_start()
```

---

## Risk Management

### Example 8: Strategy with Stop Loss and Take Profit

Add risk management to any strategy.

```python
from neleus import Strategy, StrategyContext, Bar, OrderSide, Fill
from typing import Optional

class MomentumWithRisk(Strategy):
    def __init__(
        self,
        lookback: int = 20,
        threshold: float = 0.02,
        stop_loss_pct: float = 0.02,  # 2%
        take_profit_pct: float = 0.05,  # 5%
    ):
        super().__init__("MomentumWithRisk")
        self.lookback = lookback
        self.threshold = threshold
        self.stop_loss_pct = stop_loss_pct
        self.take_profit_pct = take_profit_pct
        
        self.prices = []
        self.position = 0
        self.entry_price: Optional[float] = None
    
    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        self.prices.append(bar.close)
        
        if len(self.prices) < self.lookback:
            return
        
        momentum = (self.prices[-1] - self.prices[-self.lookback]) / self.prices[-self.lookback]
        current_price = bar.close
        
        # Check stop loss / take profit first
        if self.position != 0 and self.entry_price is not None:
            pnl_pct = (current_price - self.entry_price) / self.entry_price
            
            # Adjust for short positions
            if self.position == -1:
                pnl_pct = -pnl_pct
            
            # Stop loss hit
            if pnl_pct <= -self.stop_loss_pct:
                if self.position == 1:
                    ctx.market_order(bar.instrument_id, OrderSide.Sell, 1.0)
                else:
                    ctx.market_order(bar.instrument_id, OrderSide.Buy, 1.0)
                
                self.position = 0
                print(f"STOP LOSS: PnL = {pnl_pct:.2%}")
                return
            
            # Take profit hit
            if pnl_pct >= self.take_profit_pct:
                if self.position == 1:
                    ctx.market_order(bar.instrument_id, OrderSide.Sell, 1.0)
                else:
                    ctx.market_order(bar.instrument_id, OrderSide.Buy, 1.0)
                
                self.position = 0
                print(f"TAKE PROFIT: PnL = {pnl_pct:.2%}")
                return
        
        # Entry logic (same as before)
        if self.position == 0:
            if momentum > self.threshold:
                ctx.market_order(bar.instrument_id, OrderSide.Buy, 1.0)
                self.position = 1
                self.entry_price = current_price
            elif momentum < -self.threshold:
                ctx.market_order(bar.instrument_id, OrderSide.Sell, 1.0)
                self.position = -1
                self.entry_price = current_price
```

---

## Advanced Examples

### Example 9: Walk-Forward Optimization

Optimize parameters with walk-forward analysis.

```python
from neleus import BacktestConfig, InstrumentId, Venue, InstrumentType
from neleus.backtest import WalkForwardAnalysis, ParameterDef
from strategies.momentum_strategy import MomentumStrategy

# Define parameters to optimize
params = [
    ParameterDef(name="lookback", min_val=10, max_val=50, step=5),
    ParameterDef(name="threshold", min_val=0.01, max_val=0.05, step=0.01),
]

# Configure walk-forward
wf = WalkForwardAnalysis(
    train_days=180,  # 6 months training
    test_days=30,    # 1 month testing
    step_days=30,    # Roll forward 1 month
)

# Run walk-forward optimization
instrument = InstrumentId(Venue.Hyperliquid, "ETH", InstrumentType.Perp)
config = BacktestConfig(initial_capital=100000.0)

results = wf.run(
    strategy_class=MomentumStrategy,
    instrument=instrument,
    config=config,
    parameters=params,
    start_date="2023-01-01",
    end_date="2024-01-01",
)

# Analyze results
print(f"Average Return: {results.avg_return:.2%}")
print(f"Average Sharpe: {results.avg_sharpe:.2f}")
print(f"Best Parameters: {results.best_params}")
```

---

### Example 10: Multi-Strategy Portfolio

Run multiple strategies together.

```python
from neleus import LiveNode, HyperliquidVenueConfig, Network
from strategies.momentum_strategy import MomentumStrategy
from strategies.mean_reversion_strategy import MeanReversionStrategy

# Configure venue
venue_config = HyperliquidVenueConfig(
    network=Network.Testnet,
    wallet_address="0x...",
)

# Create node
node = LiveNode(venue_config)

# Add multiple strategies
node.add_strategy(MomentumStrategy(lookback=20), capital_allocation=0.5)
node.add_strategy(MeanReversionStrategy(period=30), capital_allocation=0.5)

# Start trading
await node.start()
```

---

## Running Examples

### From CLI

```bash
# Run any example
neleus backtest --strategy momentum --symbol ETH-PERP

# With custom parameters (edit the strategy file first)
neleus backtest --strategy momentum_with_risk --symbol BTC-PERP
```

### From Python

```python
from neleus import backtest, BacktestConfig, InstrumentId, Venue, InstrumentType
from strategies.momentum_strategy import MomentumStrategy

# Configure
strategy = MomentumStrategy(lookback=20, threshold=0.02)
instrument = InstrumentId(Venue.Hyperliquid, "ETH", InstrumentType.Perp)
config = BacktestConfig(
    initial_capital=100000.0,
    start_date="2024-01-01",
    end_date="2024-06-01",
)

# Run
results = backtest(strategy, instrument, config)

# Analyze
print(f"Return: {results.return_pct:.2f}%")
print(f"Sharpe: {results.sharpe_ratio:.2f}")
print(f"Max DD: {results.max_drawdown_pct:.2f}%")
```

---

## Next Steps

- Explore the [API Reference](./API_REFERENCE.md) for more details
- Learn about [Risk Management](./RISK_MANAGEMENT.md)
- Read the [Configuration Guide](./CONFIGURATION.md)
- Check out [Live Trading Guide](./LIVE_TRADING.md)
