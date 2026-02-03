# Getting Started with Neleus

Welcome to Neleus! This guide will help you get started with building, backtesting, and deploying trading strategies.

## What is Neleus?

Neleus is a high-performance trading and backtesting framework that combines:
- **Rust Core**: Fast execution engine, order management, and risk controls
- **Python Interface**: Easy strategy development and data analysis
- **One Codebase, Many Modes**: Write strategies once, run them in backtest, paper, or live

## Installation

### Prerequisites

- **Python 3.8+** (Python 3.10+ recommended)
- **Rust toolchain** (install from [rustup.rs](https://rustup.rs))
- macOS, Linux, or Windows (WSL recommended)

### Step 1: Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Step 2: Clone and Install Neleus

```bash
# Clone the repository
git clone https://github.com/auralshin/neleus.git
cd neleus

# Create and activate virtual environment
python3 -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate

# Install maturin (builds Rust extensions for Python)
pip install maturin

# Build and install the Rust extension
cd crates/pybridge
maturin develop --release
cd ../..

# Install Python package
pip install -e python/
```

### Step 3: Verify Installation

```bash
neleus --version
python -c "from neleus import Strategy; print('Neleus installed successfully!')"
```

## Quick Start: Your First Strategy

### 1. Create a New Project

```bash
neleus new my_trading_bot
cd my_trading_bot
```

This creates a complete project structure:
```
my_trading_bot/
├── neleus.toml           # Project configuration
├── strategies/           # Your trading strategies
├── config/              # Venue and environment configs
├── data/                # Market data cache
├── backtests/           # Backtest results
├── logs/                # Log files
└── .env.example         # Environment variables template
```

### 2. Configure API Keys (Optional for Backtesting)

```bash
cp .env.example .env
# Edit .env with your API keys
```

For backtesting, you can skip this step initially.

### 3. Create Your First Strategy

```bash
neleus strategy add my_momentum --template momentum
```

Or edit `strategies/momentum_strategy.py`:

```python
from neleus import Strategy, StrategyContext, Bar, OrderSide, InstrumentId, Venue, InstrumentType
from typing import Optional
from decimal import Decimal

class MomentumStrategy(Strategy):
    def __init__(self, lookback: int = 20, threshold: float = 0.02):
        super().__init__("MomentumStrategy")
        self.lookback = lookback
        self.threshold = threshold
        self.prices: List[float] = []
    
    def on_bar(self, ctx: StrategyContext, bar: Bar) -> None:
        """Called for each new bar."""
        self.prices.append(float(bar.close))
        
        # Need enough data
        if len(self.prices) < self.lookback:
            return
        
        # Keep only lookback period
        self.prices = self.prices[-self.lookback:]
        
        # Calculate momentum (rate of change)
        momentum = (self.prices[-1] - self.prices[0]) / self.prices[0]
        
        # Generate signals
        if momentum > self.threshold:
            # Strong upward momentum - buy
            ctx.market_order(bar.instrument_id, OrderSide.Buy, 0.1)
        elif momentum < -self.threshold:
            # Strong downward momentum - sell
            ctx.market_order(bar.instrument_id, OrderSide.Sell, 0.1)
```

### 4. Run a Backtest

```bash
neleus backtest --strategy momentum --symbol ETH-PERP --capital 100000
```

Or use the Python API:

```python
import asyncio
from neleus import (
    InstrumentId,
    Venue,
    InstrumentType,
    HyperliquidBacktestConfig,
    HyperliquidBacktestNode,
    CandleInterval,
)
from strategies.momentum_strategy import MomentumStrategy

async def main():
    # Configure backtest
    config = HyperliquidBacktestConfig(
        initial_capital=100000.0,
        commission_bps=5.0,
        slippage_bps=2.0,
        start_date="2024-01-01",
        end_date="2024-06-01",
        symbol="ETH",
        interval=CandleInterval.OneHour,
    )
    
    # Create strategy
    strategy = MomentumStrategy(lookback=20, threshold=0.02)
    
    # Run backtest
    node = HyperliquidBacktestNode(config)
    node.add_strategy(strategy)
    results = await node.run()
    
    # View results
    print(results.summary())

if __name__ == "__main__":
    asyncio.run(main())
```

### 5. Launch the Dashboard

```bash
neleus ui
```

This opens a web interface at `http://localhost:8765` where you can:
- View real-time charts
- Monitor portfolio and positions
- Run backtests with visual results
- Edit and test strategies
- View risk metrics

## Next Steps

### Learn More About Strategies
- [Strategy Development Guide](./STRATEGY_GUIDE.md)
- [API Reference](./API_REFERENCE.md)
- [Examples](./EXAMPLES.md)

### Configure Trading
- [Configuration Guide](./CONFIGURATION.md)
- [Venue Setup](./VENUES.md)
- [Risk Management](./RISK_MANAGEMENT.md)

### Advanced Topics
- [Backtesting Deep Dive](./BACKTESTING.md)
- [Live Trading](./LIVE_TRADING.md)
- [Architecture](./ARCHITECTURE.md)
- [Performance Optimization](./PERFORMANCE_OPTIMIZATIONS.md)

## Common Issues

### Import Error: neleus_core not found

**Solution**: The Rust extension wasn't built. Run:
```bash
cd crates/pybridge
maturin develop --release
```

### Strategy Not Found

**Solution**: Make sure your strategy file is in the `strategies/` directory and the class name ends with "Strategy".

### API Connection Issues

**Solution**: 
1. Check your `.env` file has correct API keys
2. Verify network connectivity
3. For testnet: set `HYPERLIQUID_NETWORK=testnet` in `.env`

### Backtest Takes Too Long

**Solution**: 
1. Use shorter time periods
2. Use larger timeframes (e.g., 1h instead of 1m)
3. Build with `--release` flag for optimized performance

## Getting Help

- **Documentation**: [docs/](./index.md)
- **Examples**: [examples/](../examples/)
- **Issues**: [GitHub Issues](https://github.com/auralshin/neleus/issues)
- **Discussions**: [GitHub Discussions](https://github.com/auralshin/neleus/discussions)

## Project Philosophy

Neleus follows a "write once, run anywhere" philosophy:

1. **Research**: Use backtesting to develop and validate strategies
2. **Paper Trade**: Test in live markets without real capital
3. **Live Deploy**: Same code, real trading

The Rust core ensures your backtest results are **deterministic** and **reproducible** - critical for production trading systems.
