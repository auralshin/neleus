# Neleus Python Package

The Python user layer for the Neleus trading framework. Write strategies in Python, run them anywhere.

## Installation

```bash
pip install neleus
```

Or install from source:

```bash
cd python
pip install -e .
```

## Quick Start

### 1. Create a New Project

```bash
neleus create my_trading_project
cd my_trading_project
```

This creates:
```
my_trading_project/
├── .env              # API keys and secrets (DO NOT COMMIT)
├── .env.example      # Example environment file
├── .gitignore        # Pre-configured gitignore
├── config.yaml       # Main project configuration
├── run_backtest.py   # Backtest runner script
├── strategies/       # Your trading strategies
├── configs/          # Strategy-specific configs
├── data/             # Market data cache
├── logs/             # Application logs
└── reports/          # Backtest reports
```

### 2. Configure Your Credentials

Edit `.env` with your API keys:

```bash
# Hyperliquid
HYPERLIQUID_PRIVATE_KEY=your_private_key_here
HYPERLIQUID_TESTNET=true

# Lighter
LIGHTER_API_KEY=your_api_key_here
LIGHTER_API_SECRET=your_secret_here
```

### 3. Create a Strategy

```bash
neleus new strategy --name MyMomentum
```

This creates:
- `strategies/my_momentum.py` - Your strategy code
- `configs/my_momentum.yaml` - Strategy configuration

### 4. Edit Your Strategy

```python
# strategies/my_momentum.py

from neleus import Strategy, StrategyContext, Bar, OrderSide

class MyMomentum(Strategy):
    def __init__(self, lookback: int = 20, threshold: float = 0.02):
        super().__init__("MyMomentum")
        self.lookback = lookback
        self.threshold = threshold
        self.prices = []
    
    def on_start(self, ctx: StrategyContext):
        ctx.subscribe_bars(ctx.instruments[0])
    
    def on_data(self, ctx: StrategyContext, data):
        if isinstance(data, Bar):
            self.prices.append(data.close)
            if len(self.prices) > self.lookback:
                momentum = (self.prices[-1] - self.prices[-self.lookback]) / self.prices[-self.lookback]
                if momentum > self.threshold:
                    ctx.market_order(data.instrument_id, OrderSide.BUY, 0.1)
                elif momentum < -self.threshold:
                    ctx.market_order(data.instrument_id, OrderSide.SELL, 0.1)
```

### 5. Run Backtest

```bash
neleus backtest
```

Or with options:

```bash
neleus backtest --strategy my_momentum --start 2024-01-01 --end 2024-06-01
```

### 6. Open Dashboard

```bash
neleus ui
```

Opens a web dashboard at `http://localhost:8080` where you can:
- View price charts and orderbook
- Configure strategy parameters
- Run backtests with visual results
- Monitor positions and PnL

## CLI Commands

| Command | Description |
|---------|-------------|
| `neleus create [name]` | Create a new Neleus project |
| `neleus new strategy --name NAME` | Create a new strategy |
| `neleus backtest` | Run backtest |
| `neleus run --mode paper` | Start paper trading |
| `neleus run --mode live` | Start live trading |
| `neleus ui` | Open web dashboard |
| `neleus config show` | Show current config |
| `neleus config validate` | Validate configuration |

## Configuration

### Project Config (`config.yaml`)

```yaml
project:
  name: "my_project"

venues:
  hyperliquid:
    enabled: true
    mode: paper  # backtest | paper | live

instruments:
  - symbol: BTC
    venue: hyperliquid
    type: perp

risk:
  max_position_size: 1.0
  max_drawdown_pct: 10.0
  daily_loss_limit: 500.0

backtest:
  start_date: "2024-01-01"
  end_date: "2024-12-31"
  initial_capital: 10000.0
```

### Strategy Config (`configs/my_strategy.yaml`)

```yaml
strategy:
  name: "MyStrategy"
  enabled: true

parameters:
  lookback: 20
  threshold: 0.02
  position_size: 0.1

instruments:
  - symbol: BTC
    venue: hyperliquid
    type: perp
```

### Environment Variables (`.env`)

```bash
# Venue Credentials
HYPERLIQUID_PRIVATE_KEY=
HYPERLIQUID_TESTNET=true
LIGHTER_API_KEY=
LIGHTER_API_SECRET=

# Override Config
NELEUS_LOG_LEVEL=INFO
NELEUS_MAX_POSITION_SIZE=1.0
NELEUS_DASHBOARD_PORT=8080
```

## Python API

```python
from neleus import (
    Strategy,
    StrategyContext,
    BacktestNode,
    BacktestConfig,
    load_project_config,
    start_dashboard,
)

# Load config
config = load_project_config()

# Create strategy
class MyStrategy(Strategy):
    def on_data(self, ctx, data):
        pass

# Run backtest
from datetime import datetime
bt_config = BacktestConfig(
    start_time=datetime(2024, 1, 1),
    end_time=datetime(2024, 6, 1),
)
node = BacktestNode(bt_config)
node.add_strategy(MyStrategy())
results = node.run()
print(results.summary())

# Generate report
from neleus import generate_html_report
generate_html_report(results, "report.html")

# Start dashboard
start_dashboard(port=8080)
```

## Supported Venues

| Venue | Market Data | Trading | Status |
|-------|-------------|---------|--------|
| Hyperliquid |   |   | Active |
| Lighter |   |   | Active |

## Requirements

- Python 3.10+
- No native dependencies (pure Python + optional Rust bindings)
- Works on Linux, macOS, Windows

## Development

```bash
# Install dev dependencies
pip install -e ".[dev]"

# Run tests
pytest

# Format code
black .
ruff check .

# Type check
mypy neleus
```

## License

MIT License - see LICENSE file.
