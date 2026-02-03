# Neleus Quick Reference

Essential commands and code snippets for quick reference.

## 🚀 Installation

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and setup
git clone https://github.com/auralshin/neleus.git
cd neleus
python3 -m venv .venv && source .venv/bin/activate
pip install maturin
cd crates/pybridge && maturin develop --release
pip install -e python/
```

## 📋 Common CLI Commands

```bash
# Create new project
neleus new my_bot

# Add strategy
neleus strategy add momentum --template momentum

# Run backtest
neleus backtest --strategy momentum --symbol ETH-PERP --capital 100000

# Start dashboard
neleus ui

# Paper trading
neleus live --strategy momentum --paper

# Live trading (testnet)
neleus live --strategy momentum --real
```

## 💻 Basic Strategy Template

```python
from neleus import Strategy, StrategyContext, Bar, OrderSide

class MyStrategy(Strategy):
    def __init__(self):
        super().__init__("MyStrategy")
        self.prices = []
    
    def on_bar(self, ctx: StrategyContext, bar: Bar):
        self.prices.append(bar.close)
        
        # Your logic here
        if len(self.prices) >= 20:
            # Example: Simple momentum
            momentum = (self.prices[-1] - self.prices[-20]) / self.prices[-20]
            if momentum > 0.02:
                ctx.market_order(bar.instrument_id, OrderSide.Buy, 0.1)
```

## 📊 Order Types

```python
# Market order
ctx.market_order(instrument, OrderSide.Buy, 1.0)

# Limit order
ctx.limit_order(instrument, OrderSide.Buy, 1.0, 50000.0)

# Stop loss
ctx.stop_order(instrument, OrderSide.Sell, 1.0, 49000.0)

# Cancel order
ctx.cancel_order(order_id)

# Cancel all
ctx.cancel_all_orders()
```

## 🎯 Common Patterns

### Momentum Strategy
```python
momentum = (self.prices[-1] - self.prices[-lookback]) / self.prices[-lookback]
if momentum > threshold:
    ctx.market_order(instrument, OrderSide.Buy, size)
```

### Mean Reversion (Bollinger Bands)
```python
sma = sum(prices[-period:]) / period
std = (sum((p - sma)**2 for p in prices[-period:]) / period) ** 0.5
upper = sma + (2.0 * std)
lower = sma - (2.0 * std)

if price < lower:
    ctx.market_order(instrument, OrderSide.Buy, size)
```

### RSI
```python
gains = [max(0, prices[i] - prices[i-1]) for i in range(1, len(prices))]
losses = [-min(0, prices[i] - prices[i-1]) for i in range(1, len(prices))]
avg_gain = sum(gains[-period:]) / period
avg_loss = sum(losses[-period:]) / period
rsi = 100 - (100 / (1 + avg_gain / avg_loss))

if rsi < 30:  # Oversold
    ctx.market_order(instrument, OrderSide.Buy, size)
```

## ⚙️ Configuration

### neleus.toml
```toml
[project]
name = "my_bot"

[trading]
default_venue = "hyperliquid"
network = "testnet"

[backtest]
initial_capital = 100000.0
commission_bps = 5.0
slippage_bps = 2.0

[risk]
max_position_pct = 10.0
max_daily_loss_pct = 5.0
max_leverage = 5.0
```

### .env
```bash
HYPERLIQUID_PRIVATE_KEY=0x...
HYPERLIQUID_NETWORK=testnet
LIGHTER_API_KEY=...
```

## 🔍 Backtest Configuration

```python
import asyncio
from neleus import (
    HyperliquidBacktestConfig,
    HyperliquidBacktestNode,
    CandleInterval,
)

async def run_backtest():
    config = HyperliquidBacktestConfig(
        initial_capital=100000.0,
        commission_bps=5.0,
    slippage_bps=2.0,
    start_date="2024-01-01",
    end_date="2024-06-01",
    fill_model=FillModel.NextTick,
)
```

## 📈 Running Backtests

```python
from neleus import backtest, InstrumentId, Venue, InstrumentType

instrument = InstrumentId(Venue.Hyperliquid, "ETH", InstrumentType.Perp)
results = backtest(strategy, instrument, config)

print(f"Return: {results.return_pct:.2f}%")
print(f"Sharpe: {results.sharpe_ratio:.2f}")
print(f"Max DD: {results.max_drawdown_pct:.2f}%")
```

## 🎛️ Risk Management

```python
from neleus import RiskConfig, StopLossConfig, StopLossType

# Risk config
risk = RiskConfig(
    max_position_pct=10.0,
    max_leverage=5.0,
    max_daily_loss_pct=5.0,
)

# Stop loss
stop = StopLossConfig(
    type=StopLossType.ATR,
    atr_period=14,
    atr_multiplier=2.0,
)
```

## 🔐 Venue Setup

### Hyperliquid
```python
from neleus import HyperliquidConfig, Network

config = HyperliquidConfig(
    network=Network.Testnet,
    private_key=os.getenv("HYPERLIQUID_PRIVATE_KEY"),
    wallet_address=os.getenv("HYPERLIQUID_WALLET"),
)
```

### Lighter
```python
from neleus import LighterConfig

config = LighterConfig(
    network=Network.Testnet,
    api_key=os.getenv("LIGHTER_API_KEY"),
    private_key=os.getenv("LIGHTER_PRIVATE_KEY"),
)
```

## 🔄 Live Trading

```python
from neleus import LiveNode

node = LiveNode(venue_config)
node.add_strategy(MyStrategy())
await node.start()
```

## 📝 Logging

```python
import logging

logger = logging.getLogger(__name__)
logger.info("Strategy started")
logger.debug(f"Processing bar: {bar.close}")
logger.warning("High volatility detected")
logger.error("Order failed")
```

## 🎨 Data Callbacks

```python
def on_bar(self, ctx, bar):
    """Called for each bar/candle"""
    pass

def on_trade(self, ctx, trade):
    """Called for each trade tick"""
    pass

def on_quote(self, ctx, quote):
    """Called for BBO updates"""
    pass

def on_order_book(self, ctx, book):
    """Called for order book updates"""
    pass

def on_fill(self, ctx, fill):
    """Called when order is filled"""
    pass

def on_timer(self, ctx, timer_id):
    """Called when timer expires"""
    pass
```

## 🛠️ Useful Methods

```python
# Position info
position = ctx.get_position(instrument)
positions = ctx.get_open_positions()
balance = ctx.get_balance()
equity = ctx.get_equity()

# Subscriptions
ctx.subscribe_bars(instrument, "1h")
ctx.subscribe_trades(instrument)
ctx.subscribe_quotes(instrument)
ctx.subscribe_order_book(instrument, depth=20)

# Timers
ctx.set_timer("rebalance", 3600_000)  # 1 hour
ctx.cancel_timer("rebalance")

# Close positions
ctx.close_position(instrument)
```

## 📊 Performance Analysis

```python
from neleus.plots import BacktestPlotter

plotter = BacktestPlotter(results)
plotter.plot_equity_curve()
plotter.plot_drawdown()
plotter.plot_monthly_returns()
plotter.save_html_report("results.html")
```

## 🐛 Debugging

```bash
# Check logs
tail -f logs/neleus.log

# Validate config
neleus build

# List strategies
neleus strategy list

# Show strategy code
neleus strategy show momentum
```

## 🆘 Common Issues

### Import Error
```bash
cd crates/pybridge && maturin develop --release
```

### API Connection
```bash
# Check .env
cat .env

# Use testnet first
HYPERLIQUID_NETWORK=testnet
```

### Strategy Not Found
```bash
# Must be in strategies/ directory
# Class name must end with "Strategy"
neleus strategy list
```

## 🔗 Quick Links

- [Full API Reference](./API_REFERENCE.md)
- [CLI Reference](./CLI_REFERENCE.md)
- [Examples](./EXAMPLES.md)
- [Configuration](./CONFIGURATION.md)
- [Getting Started](./GETTING_STARTED.md)

---

**Print this page for quick reference while developing!**
