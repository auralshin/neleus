# Neleus Documentation

Welcome to the Neleus trading framework documentation! Neleus is a high-performance trading and backtesting system that combines a Rust core with a Python interface, allowing you to write strategies once and run them in backtest, paper, or live trading modes.

## 🚀 Quick Links

<div class="grid">
  <a href="./GETTING_STARTED.md" class="card">
    <h3>📖 Getting Started</h3>
    <p>Installation, quick start, and your first strategy</p>
  </a>
  
  <a href="./USAGE.md" class="card">
    <h3>📘 Usage Guide</h3>
    <p>Comprehensive guide for AI agents and Rust core</p>
  </a>
  
  <a href="./API_REFERENCE.md" class="card">
    <h3>📚 API Reference</h3>
    <p>Complete API documentation for all classes and methods</p>
  </a>
  
  <a href="./CLI_REFERENCE.md" class="card">
    <h3>⌨️ CLI Reference</h3>
    <p>Command-line interface documentation</p>
  </a>
  
  <a href="./EXAMPLES.md" class="card">
    <h3>💡 Examples</h3>
    <p>Example strategies and tutorials</p>
  </a>
</div>

---

## 📋 Table of Contents

### Getting Started
- [Installation & Setup](./GETTING_STARTED.md)
- [Quick Start Guide](./GETTING_STARTED.md#quick-start-your-first-strategy)
- [Project Structure](./GETTING_STARTED.md#create-a-new-project)
- **[Usage Guide](./USAGE.md)** - **NEW!** Comprehensive usage documentation
  - AI Agent Development
  - Rust Core Usage
  - CLI Reference
  - Configuration Guide
  - Best Practices

### Core Documentation
- **[API Reference](./API_REFERENCE.md)** - Complete API documentation
  - Strategy API
  - Market Data Types
  - Trading Operations
  - Risk Management
  - Execution Algorithms
  
- **[CLI Reference](./CLI_REFERENCE.md)** - Command-line tools
  - Project commands
  - Strategy management
  - Backtesting
  - Live trading
  - UI dashboard

- **[Configuration](./CONFIGURATION.md)** - Setup and configuration
  - Project configuration
  - Environment variables
  - Risk settings
  - Venue setup

### Strategy Development
- **[Examples & Tutorials](./EXAMPLES.md)** - Learn by example
  - Momentum strategies
  - Mean reversion
  - Market making
  - Multi-instrument strategies
  - Risk management examples

- **[Strategy Guide](./STRATEGY_GUIDE.md)** - Writing strategies
  - Strategy lifecycle
  - Data callbacks
  - Order management
  - Position tracking

### Advanced Topics
- **[Architecture](./ARCHITECTURE.md)** - System design
  - Rust core
  - Python bridge
  - Message bus
  - Event-driven design

- **[Managed Service](./MANAGED_SERVICE.md)** - Always-on trading platform
  - Agent Orchestrator (CI/CD for bots)
  - Signal Hub (AI/Quant integration)
  - Agent Monitor (metrics & alerts)
  - Deployment configuration

- **[Risk Management](./RISK_MANAGEMENT.md)** - Risk controls
  - Position sizing
  - Stop loss/take profit
  - Portfolio risk
  - Drawdown management

- **[Venues](./VENUES.md)** - Exchange integrations
  - Hyperliquid
  - Lighter (zkLighter)
  - Polymarket

- **[Backtesting](./BACKTESTING.md)** - Testing strategies
  - Backtest configuration
  - Simulation models
  - Walk-forward analysis
  - Performance metrics

- **[Live Trading](./LIVE_TRADING.md)** - Production deployment
  - Paper trading
  - Live trading setup
  - Monitoring
  - Best practices

- **[Performance](./PERFORMANCE_OPTIMIZATIONS.md)** - Optimization guide
  - Rust optimizations
  - Python performance
  - Data handling
  - Profiling

---

## 🎯 What is Neleus?

Neleus is a **hybrid Rust/Python trading framework** and **managed service platform** designed for:

### One Codebase, Many Modes
Write your strategy once and run it unchanged across:
- **Backtest** - Historical data testing
- **Paper Trading** - Live data, simulated execution
- **Live Trading** - Real money, real markets
- **Managed Service** - Always-on trading agents with CI/CD

### High Performance
- **Rust Core** - Fast execution, order management, and risk checks
- **Zero-Copy Bridge** - Efficient data passing between Rust and Python
- **Event-Driven** - Deterministic, reproducible backtests

### Developer Friendly
- **Python Strategies** - Write strategies in Python
- **Rich Ecosystem** - Use NumPy, Pandas, SciPy, scikit-learn
- **Visual Tools** - Web dashboard with charts and analytics
- **CLI Tools** - Productive command-line interface

---

## 🏗️ Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│             Python Layer (Strategy)                 │
│  ┌──────────────────────────────────────────────┐  │
│  │  Your Strategy Code                          │  │
│  │  - Signal generation                         │  │
│  │  - Risk logic                                │  │
│  │  - Data analysis                             │  │
│  └──────────────────────────────────────────────┘  │
│                       ↕                             │
│  ┌──────────────────────────────────────────────┐  │
│  │  Python Bindings (PyO3)                      │  │
│  └──────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
                       ↕
┌─────────────────────────────────────────────────────┐
│             Rust Core (Engine)                      │
│  ┌──────────────────────────────────────────────┐  │
│  │  Order Management System                     │  │
│  │  Position Tracking                           │  │
│  │  Risk Management                             │  │
│  │  Backtest Engine                             │  │
│  │  Market Data Processing                      │  │
│  │  Venue Adapters                              │  │
│  └──────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**Learn more:** [Architecture Documentation](./ARCHITECTURE.md)

---

## 🚦 Quick Start

### 1. Install Neleus

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and install Neleus
git clone https://github.com/auralshin/neleus.git
cd neleus
python3 -m venv .venv && source .venv/bin/activate
pip install maturin
cd crates/pybridge && maturin develop --release && cd ../..
pip install -e python/
```

### 2. Create a Project

```bash
neleus new my_trading_bot
cd my_trading_bot
```

### 3. Write a Strategy

```python
# strategies/momentum_strategy.py
from neleus import Strategy, StrategyContext, Bar, OrderSide

class MomentumStrategy(Strategy):
    def __init__(self, lookback=20, threshold=0.02):
        super().__init__()
        self.lookback = lookback
        self.threshold = threshold
        self.prices = []
    
    def on_bar(self, ctx: StrategyContext, bar: Bar):
        self.prices.append(bar.close)
        if len(self.prices) >= self.lookback:
            momentum = (self.prices[-1] - self.prices[-self.lookback]) / self.prices[-self.lookback]
            if momentum > self.threshold:
                ctx.market_order(bar.instrument_id, OrderSide.Buy, 0.1)
```

### 4. Run Backtest

```bash
neleus backtest --strategy momentum --symbol ETH-PERP --capital 100000
```

### 5. Launch Dashboard

```bash
neleus ui
```

**Learn more:** [Getting Started Guide](./GETTING_STARTED.md)

---

## 📚 Documentation Sections

### For Beginners
Start here if you're new to Neleus:
1. [Getting Started](./GETTING_STARTED.md) - Installation and first strategy
2. [Examples](./EXAMPLES.md) - Learn from example strategies
3. [CLI Reference](./CLI_REFERENCE.md) - Command-line tools

### For Strategy Developers
Build and test trading strategies:
1. [Strategy Guide](./STRATEGY_GUIDE.md) - Writing strategies
2. [API Reference](./API_REFERENCE.md) - Complete API docs
3. [Examples](./EXAMPLES.md) - Example strategies
4. [Backtesting](./BACKTESTING.md) - Testing strategies

### For Production Users
Deploy strategies to live trading:
1. [Configuration](./CONFIGURATION.md) - Setup and config
2. [Risk Management](./RISK_MANAGEMENT.md) - Risk controls
3. [Venues](./VENUES.md) - Exchange setup
4. [Live Trading](./LIVE_TRADING.md) - Production deployment
5. [Managed Service](./MANAGED_SERVICE.md) - Always-on trading platform

### For Contributors
Contribute to Neleus:
1. [Architecture](./ARCHITECTURE.md) - System design
2. [Performance](./PERFORMANCE_OPTIMIZATIONS.md) - Optimization
3. [Contributing Guide](../CONTRIBUTING.md) - How to contribute

---

## 🎓 Tutorials

### Basic Tutorials
- [Your First Strategy](./GETTING_STARTED.md#quick-start-your-first-strategy)
- [Running Backtests](./CLI_REFERENCE.md#neleus-backtest)
- [Using the Dashboard](./CLI_REFERENCE.md#neleus-ui)

### Strategy Development
- [Momentum Strategy](./EXAMPLES.md#example-3-rate-of-change-roc-momentum)
- [Mean Reversion](./EXAMPLES.md#example-5-bollinger-bands)
- [Market Making](./EXAMPLES.md#example-6-simple-market-maker)
- [Pairs Trading](./EXAMPLES.md#example-7-pairs-trading)

### Advanced Topics
- [Risk Management](./EXAMPLES.md#example-8-strategy-with-stop-loss-and-take-profit)
- [Walk-Forward Optimization](./EXAMPLES.md#example-9-walk-forward-optimization)
- [Multi-Strategy Portfolio](./EXAMPLES.md#example-10-multi-strategy-portfolio)

---

## 🔧 Key Features

### Strategy Development
-  Python strategy API
-  Event-driven callbacks (on_bar, on_trade, on_quote)
-  Multiple data types (bars, ticks, order book)
-  Built-in indicators
-  Portfolio management

### Backtesting
-  Fast Rust engine
-  Multiple fill models
-  Realistic slippage simulation
-  Latency modeling
-  Walk-forward analysis
-  Parameter optimization

### Risk Management
-  Position limits
-  Leverage controls
-  Stop loss/take profit
-  Drawdown protection
-  Daily loss limits
-  Dynamic risk sizing

### Execution
-  TWAP, VWAP, Iceberg orders
-  Smart order routing
-  Multiple venues
-  Real-time market data
-  Order state tracking

### Managed Service (Agent Platform)
-  Agent Orchestrator - CI/CD for trading bots
-  Signal Hub - External AI/ML integration
-  Agent Monitor - Real-time metrics & alerts
-  Auto-restart with health checks
-  Rolling upgrades with rollback
- 🔄 TEE deployment (coming soon)

### Venues
-  Hyperliquid (Testnet + Mainnet)
-  Lighter / zkLighter
-  Polymarket
- 🔄 More coming soon

### Tools
-  Web dashboard with charts
-  CLI for project management
-  Performance analytics
-  Live monitoring
-  Event replay

---

## 🌊 Philosophy

Neleus is built on three core principles:

### 1. Write Once, Run Anywhere
Your strategy code should work identically in backtest, paper, and live modes. No separate implementations, no surprises.

### 2. Rust Performance, Python Ergonomics
Use Rust for the hot path (order management, risk checks, execution) and Python for the creative work (strategy logic, research).

### 3. Deterministic and Reproducible
Backtests should be deterministic. Event logs should be replayable. Production should match backtest behavior.

---

## 🚀 Supported Venues

| Venue | Status | Testnet | Mainnet |
|-------|--------|---------|---------|
| **Hyperliquid** |  Full |  |  |
| **Lighter** |  Full |  |  |
| **Polymarket** |  Full | - |  |
| Binance | 🔄 Planned | - | - |
| dYdX | 🔄 Planned | - | - |

**Learn more:** [Venue Documentation](./VENUES.md)

---

## 📊 Performance

Neleus is designed for high-performance trading:

- **Backtests**: Process millions of events per second
- **Live Trading**: Sub-millisecond order placement
- **Memory**: Efficient memory usage with zero-copy data passing
- **Concurrency**: Async Rust for network I/O

**Learn more:** [Performance Optimizations](./PERFORMANCE_OPTIMIZATIONS.md)

---

## 🤝 Community & Support

### Getting Help
- 📖 **Documentation**: You're reading it!
- 💬 **Discussions**: [GitHub Discussions](https://github.com/auralshin/neleus/discussions)
- 🐛 **Issues**: [GitHub Issues](https://github.com/auralshin/neleus/issues)
- 📧 **Email**: support@neleus.dev

### Contributing
We welcome contributions! See our [Contributing Guide](../CONTRIBUTING.md).

### Roadmap
Check our [Status & Roadmap](./STATUS_AND_ROADMAP.md) for upcoming features.

---

## 📄 License

Neleus is open source software. See [LICENSE](../LICENSE) for details.

---

## 🙏 Acknowledgments

Neleus is built on the shoulders of giants:
- **Rust** - Systems programming language
- **PyO3** - Rust ↔ Python bridge
- **Tokio** - Async runtime
- **Pandas/NumPy** - Data analysis

---

## Next Steps

Ready to build trading strategies? Start here:

1. **New Users**: [Getting Started Guide](./GETTING_STARTED.md)
2. **Strategy Developers**: [API Reference](./API_REFERENCE.md)
3. **Live Trading**: [Configuration](./CONFIGURATION.md) → [Venues](./VENUES.md) → [Live Trading](./LIVE_TRADING.md)

Happy trading! 🌊
