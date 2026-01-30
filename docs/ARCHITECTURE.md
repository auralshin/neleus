# Neleus Architecture: Rust Core + Python Interface

## Overview

Neleus follows a **hybrid architecture** where:
- **Rust** handles all performance-critical operations (the "engine")
- **Python** provides an ergonomic interface for strategy development (the "steering wheel")

This design gives you the best of both worlds: **Rust's performance** with **Python's ease of use**.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     Python Layer (Interface)                │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Strategy Development (Your Code)                    │  │
│  │  - Signal generation                                 │  │
│  │  - Risk management logic                             │  │
│  │  - Data analysis (numpy, pandas, scipy)              │  │
│  └──────────────────────────────────────────────────────┘  │
│                           ↕                                 │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Python Bindings (PyO3)                              │  │
│  │  - Strategy, StrategyContext                         │  │
│  │  - Bar, Order, InstrumentId                          │  │
│  │  - HyperliquidBacktestNode                           │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                           ↕ (PyO3 Bridge)
┌─────────────────────────────────────────────────────────────┐
│                     Rust Core (Engine)                      │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Backtest Engine                                     │  │
│  │  - Event loop                                        │  │
│  │  - Order matching                                    │  │
│  │  - Position tracking                                 │  │
│  │  - Performance metrics                               │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Risk Management                                     │  │
│  │  - Stop loss/take profit                             │  │
│  │  - Position sizing                                   │  │
│  │  - Margin calculations                               │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Market Data Processing                              │  │
│  │  - OHLC aggregation                                  │  │
│  │  - Tick data handling                                │  │
│  │  - WebSocket streaming                               │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Venue Adapters                                      │  │
│  │  - Hyperliquid, Lighter, Polymarket                  │  │
│  │  - Order routing                                     │  │
│  │  - API authentication                                │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Division of Responsibilities

### Rust Core (Performance-Critical)

**What Rust Does:**
1. **Backtesting Engine** - Fast event loop, order matching
2. **Order Execution** - Low-latency order routing
3. **Position Tracking** - Real-time P&L calculations
4. **Risk Management** - Stop loss, take profit, margin checks
5. **Market Data** - OHLC aggregation, tick processing
6. **Database Operations** - TimescaleDB queries, event persistence
7. **WebSocket Handling** - Real-time market data streaming

**Why Rust:**
- Zero-cost abstractions
- No garbage collection pauses
- Memory safety without runtime overhead
- Excellent concurrency with tokio
- Can match C++ performance

### Python Interface (Developer Experience)

**What Python Does:**
1. **Strategy Logic** - Signal generation, trading rules
2. **Data Analysis** - NumPy, Pandas, SciPy calculations
3. **Configuration** - Strategy parameters, backtest settings
4. **Visualization** - Plotly charts, equity curves
5. **Research** - Jupyter notebooks, exploratory analysis

**Why Python:**
- Rapid prototyping
- Rich ecosystem (numpy, scipy, pandas, sklearn)
- Easy to learn and maintain
- Perfect for research and analysis
- Great for data science workflows

## Example: How They Work Together

### Simple Strategy

```python
import numpy as np
from scipy import stats
from neleus import Strategy, StrategyContext, Bar, OrderSide

class MyStrategy(Strategy):
    def __init__(self):
        super().__init__("MyStrategy")
        self.prices = []
    
    def on_data(self, ctx: StrategyContext, data):
        # Python: Collect and analyze data
        if isinstance(data, Bar):
            self.prices.append(float(data.close))
            
            # Python: Calculate signals using scipy/numpy
            if len(self.prices) > 20:
                returns = np.diff(self.prices) / self.prices[:-1]
                t_stat, p_value = stats.ttest_1samp(returns[-20:], 0)
                
                # Python: Decide to trade
                if t_stat > 2.0 and self.position == 0:
                    # Rust: Execute order (fast!)
                    ctx.market_order(self.instrument, OrderSide.Buy, 0.1)
                    self.position = 0.1
```

**Flow:**
1. **Rust** feeds market data to Python strategy
2. **Python** calculates signals using scipy/numpy
3. **Python** calls `ctx.market_order()`
4. **Rust** executes order in backtest engine
5. **Rust** updates positions and P&L
6. **Python** receives fill confirmation
7. **Rust** continues event loop

## Performance Characteristics

### Rust Operations (Microseconds)
- Order matching: 1-10 μs
- Position update: < 1 μs
- Risk checks: 1-5 μs
- Event dispatch: < 1 μs

### Python Operations (Milliseconds)
- Signal calculation: 0.1-10 ms
- NumPy operations: 0.1-1 ms
- Data analysis: 1-100 ms

**Result:** Python overhead is negligible compared to strategy logic time.

## Code Organization

```
neleus/
├── crates/                    # Rust core
│   ├── core-engine/          # Backtest engine (Rust)
│   ├── core-types/           # Data structures (Rust)
│   ├── backtest/             # Backtesting (Rust)
│   ├── persistence/          # TimescaleDB (Rust)
│   ├── adapters-*/           # Venue APIs (Rust)
│   ├── pybridge/             # Python bindings (PyO3)
│   │
│   │   # Managed Service Components
│   ├── agent-orchestrator/   # Agent lifecycle (CI/CD)
│   ├── signal-hub/           # External signal integration
│   └── agent-monitor/        # Metrics and alerting
│
├── python/                    # Python package
│   └── neleus/               # Python interface
│       ├── __init__.py       # Exports from Rust
│       ├── strategy.py       # Python helpers
│       ├── types.py          # Type hints
│       ├── signals.py        # Signal Hub client
│       ├── agents.py         # Agent Manager client
│       └── config/           # Configuration
│           └── deployment.py # Deployment configs
│
└── examples/                  # Strategy examples
    ├── momentum_backtest.py  # Simple example
    └── test_python_integration.py  # Integration test
```

## Building the System

### Build Rust Core
```bash
cd crates/pybridge
../../.venv/bin/python -m maturin develop --release
```

This compiles the Rust code and installs it as a Python module.

### Import in Python
```python
from neleus import (
    Strategy,              # From Rust
    StrategyContext,       # From Rust
    HyperliquidBacktestNode,  # From Rust
)

import numpy as np        # Pure Python
import pandas as pd       # Pure Python
```

## Data Flow Examples

### 1. Backtest Execution

```python
# Python: Configure
config = HyperliquidBacktestConfig(
    coin="ETH",
    initial_capital=Decimal("10000"),
)

# Python: Define strategy
strategy = MyStrategy()

# Python: Create node (Rust engine)
node = HyperliquidBacktestNode(config)  # ← Rust
node.add_strategy(strategy)

# Rust: Run backtest (fast!)
results = await node.run_async()  # ← Rust engine
```

**What happens:**
1. Python configures parameters
2. Rust allocates backtest engine
3. Rust fetches historical data
4. Rust runs event loop:
   - Rust: Read next bar
   - Rust: Call Python `on_data()`
   - Python: Calculate signals
   - Python: Call `ctx.market_order()`
   - Rust: Match order
   - Rust: Update positions
5. Rust: Calculate performance metrics
6. Python: Analyze results with pandas

### 2. Live Trading (Future)

```python
# Python: Configure
config = HyperliquidLiveConfig(api_key="...")

# Python: Create live node
node = HyperliquidLiveNode(config)  # ← Rust
node.add_strategy(strategy)

# Rust: Connect WebSocket
# Rust: Stream market data
# Rust: Call Python strategy
# Python: Generate signals
# Rust: Send orders to exchange
# Rust: Track positions
```

## Scientific Libraries Integration

### NumPy
```python
def on_data(self, ctx, data):
    # Convert to numpy for fast calculations
    prices = np.array(self.price_history)
    returns = np.diff(prices) / prices[:-1]
    volatility = np.std(returns)
    
    # Rust executes the trade
    if volatility < threshold:
        ctx.market_order(...)
```

### Pandas
```python
def analyze_results(results):
    # Convert Rust results to pandas
    df = pd.DataFrame(results.equity_curve)
    df["returns"] = df["equity"].pct_change()
    
    # Analyze with pandas
    sharpe = df["returns"].mean() / df["returns"].std()
    max_dd = (df["equity"] / df["equity"].cummax() - 1).min()
```

### SciPy
```python
from scipy import stats

def on_data(self, ctx, data):
    returns = calculate_returns(self.prices)
    
    # Statistical test for mean reversion
    t_stat, p_value = stats.ttest_1samp(returns, 0)
    
    if p_value < 0.05:  # Significant
        ctx.market_order(...)
```

## Key Benefits

1. **Performance**: Rust handles hot paths (event loop, order matching)
2. **Productivity**: Python for strategy development and research
3. **Safety**: Rust's memory safety + type system
4. **Ecosystem**: Full access to Python's data science libraries
5. **Deterministic**: Rust backtests are reproducible
6. **Scalable**: Can handle tick-by-tick data in Rust

## When to Use Each

### Use Rust When:
- Writing new adapters for exchanges
- Implementing order matching logic
- Adding risk management features
- Optimizing hot paths
- Building database layers

### Use Python When:
- Developing trading strategies
- Calculating indicators
- Analyzing backtest results
- Research and prototyping
- Creating visualizations

## Testing the Integration

Run the comprehensive test:
```bash
python examples/test_python_integration.py
```

This verifies:
- ✓ Rust bindings work
- ✓ NumPy, Pandas, SciPy integrate
- ✓ Backtest engine executes correctly
- ✓ Data flows properly between layers

## Summary

**Architecture:** Python interface wrapping Rust core  
**Performance:** Rust-level speed with Python ergonomics  
**Flexibility:** Use any Python library while Rust handles execution  
**Best Practice:** Strategy logic in Python, execution in Rust  

This design lets you focus on **what** to trade (Python) while Neleus handles **how** to trade it (Rust).

---

## Managed Service Architecture

Beyond backtesting, Neleus provides a complete managed service for always-on trading agents:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Managed Service Layer                           │
│                                                                         │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐        │
│  │    Agent        │  │    Signal       │  │    Agent        │        │
│  │  Orchestrator   │  │      Hub        │  │    Monitor      │        │
│  │                 │  │                 │  │                 │        │
│  │  • Deploy/Stop  │  │  • Receive      │  │  • Metrics      │        │
│  │  • Health       │  │  • Route        │  │  • Alerts       │        │
│  │  • Upgrade      │  │  • Transform    │  │  • Dashboard    │        │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘        │
│           │                    │                    │                  │
└───────────┼────────────────────┼────────────────────┼──────────────────┘
            └────────────────────┼────────────────────┘
                                 ▼
                     ┌───────────────────────┐
                     │   Trading Agents      │
                     │   (Always-On)         │
                     └───────────────────────┘
```

**See:** [Managed Service Documentation](./MANAGED_SERVICE.md)
