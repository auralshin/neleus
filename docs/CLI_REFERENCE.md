# Neleus CLI Reference

Complete reference for the Neleus command-line interface.

## Table of Contents

- [Installation](#installation)
- [Global Options](#global-options)
- [Project Commands](#project-commands)
- [Strategy Commands](#strategy-commands)
- [Trading Commands](#trading-commands)
- [Development Commands](#development-commands)

---

## Installation

After installing Neleus, the `neleus` command is available globally:

```bash
pip install neleus
neleus --version
```

## Global Options

```bash
neleus --help              # Show help
neleus --version           # Show version
neleus <command> --help    # Command-specific help
```

---

## Project Commands

### `neleus new`

Create a new Neleus trading project.

```bash
neleus new <project-name> [OPTIONS]
```

**Arguments:**
- `<project-name>` - Name of the new project

**Options:**
- `--template, -t <template>` - Project template (default: "default")

**Examples:**
```bash
# Create a new project
neleus new my_trading_bot

# Create with specific template
neleus new my_bot --template advanced
```

**What it creates:**
```
my_trading_bot/
├── neleus.toml              # Project configuration
├── .env.example             # Environment variables template
├── .gitignore               # Git ignore rules
├── README.md                # Project readme
├── strategies/              # Trading strategies
│   ├── __init__.py
│   ├── momentum_strategy.py
│   └── mean_reversion_strategy.py
├── config/                  # Configuration files
│   ├── __init__.py
│   └── venues.py
├── backtests/               # Backtest configurations
│   ├── __init__.py
│   └── results/
├── data/                    # Market data cache
├── logs/                    # Log files
└── notebooks/               # Jupyter notebooks
```

**Next steps:**
```bash
cd my_trading_bot
cp .env.example .env
neleus ui
```

---

### `neleus init`

Initialize Neleus in an existing directory.

```bash
neleus init
```

Creates the Neleus project structure in the current directory. Useful for adding Neleus to an existing Python project.

**Example:**
```bash
cd my_existing_project
neleus init
```

---

### `neleus info`

Display project information and structure.

```bash
neleus info
```

**Output:**
- Project name and version
- Directory structure
- Configured strategies
- Venue connections

**Example:**
```bash
$ neleus info

📁 my_trading_bot
├── 📁 strategies/
│   ├── 📄 momentum_strategy.py
│   └── 📄 mean_reversion_strategy.py
├── 📁 config/
├── 📁 backtests/
└── 📄 neleus.toml

Neleus v0.1.0
```

---

## Strategy Commands

### `neleus strategy list`

List all available strategies in the project.

```bash
neleus strategy list
```

**Output:**
```
📋 Available Strategies:

┌─────────────────────┬───────────────────────────────┐
│ Name                │ File                          │
├─────────────────────┼───────────────────────────────┤
│ momentum_strategy   │ strategies/momentum_strategy.py│
│ mean_reversion     │ strategies/mean_reversion.py  │
└─────────────────────┴───────────────────────────────┘
```

---

### `neleus strategy add`

Create a new strategy from a template.

```bash
neleus strategy add <name> [OPTIONS]
```

**Arguments:**
- `<name>` - Name for the new strategy

**Options:**
- `--template, -t <template>` - Strategy template
  - `momentum` - Momentum-based strategy (default)
  - `mean_reversion` - Mean reversion with Bollinger Bands
  - `market_making` - Market making strategy
  - `arbitrage` - Cross-venue arbitrage
  - `custom` - Empty template

**Examples:**
```bash
# Create momentum strategy
neleus strategy add my_momentum

# Create with specific template
neleus strategy add my_mm --template market_making
```

**Generated file:** `strategies/my_momentum.py`
```python
class MyMomentumStrategy(Strategy):
    def __init__(self):
        super().__init__()
    
    def on_bar(self, ctx: StrategyContext, bar: Bar):
        # Your logic here
        pass
```

---

### `neleus strategy show`

Display strategy source code.

```bash
neleus strategy show <name>
```

**Arguments:**
- `<name>` - Strategy name

**Example:**
```bash
neleus strategy show momentum

# Output: Displays the strategy code with syntax highlighting
```

---

## Trading Commands

### `neleus backtest`

Run a backtest on a strategy.

```bash
neleus backtest [OPTIONS]
```

**Options:**
- `--strategy, -s <name>` - Strategy to backtest (required)
- `--symbol <symbol>` - Trading symbol (default: "BTC-PERP")
- `--timeframe, -t <interval>` - Bar interval (default: "1h")
  - Available: `1m`, `5m`, `15m`, `1h`, `4h`, `1d`
- `--start <date>` - Start date (YYYY-MM-DD)
- `--end <date>` - End date (YYYY-MM-DD)
- `--capital, -c <amount>` - Initial capital (default: 100000.0)
- `--commission <bps>` - Commission in basis points (default: 5.0)
- `--slippage <bps>` - Slippage in basis points (default: 2.0)

**Examples:**
```bash
# Basic backtest
neleus backtest --strategy momentum --symbol ETH-PERP

# Full configuration
neleus backtest \
  --strategy momentum \
  --symbol BTC-PERP \
  --timeframe 1h \
  --start 2024-01-01 \
  --end 2024-06-01 \
  --capital 100000 \
  --commission 5.0 \
  --slippage 2.0

# Quick test on recent data
neleus backtest -s mean_reversion --symbol ETH-PERP -t 5m
```

**Output:**
```
🔬 Backtest Configuration
Strategy:  momentum
Symbol:    ETH-PERP
Timeframe: 1h
Capital:   $100,000.00

Running backtest... ✓ Complete!

Results: MomentumStrategy
┌─────────────────────┬───────────┐
│ Metric              │ Value     │
├─────────────────────┼───────────┤
│ Total Return        │ +15.30%   │
│ Max Drawdown        │ -8.20%    │
│ Sharpe Ratio        │ 1.85      │
│ Total Trades        │ 142       │
│ Win Rate            │ 58.4%     │
│ Total Commission    │ $2,350.00 │
└─────────────────────┴───────────┘
```

---

### `neleus live`

Start live or paper trading.

```bash
neleus live [OPTIONS]
```

**Options:**
- `--strategy, -s <name>` - Strategy to run (required)
- `--symbol <symbol>` - Trading symbol (default: "BTC-PERP")
- `--venue, -v <venue>` - Trading venue (default: "hyperliquid")
  - Available: `hyperliquid`, `lighter`, `polymarket`
- `--paper` / `--real` - Paper or live trading (default: `--paper`)

**⚠️ IMPORTANT:** 
- Always test with `--paper` first!
- Live trading (`--real`) requires API keys and uses real money
- You'll be prompted for confirmation before live trading

**Examples:**
```bash
# Paper trading (safe, simulated)
neleus live --strategy momentum --symbol ETH-PERP --paper

# Live trading (requires confirmation)
neleus live --strategy momentum --symbol BTC-PERP --real --venue hyperliquid
```

**Output:**
```
🚀 Starting PAPER trading
   Strategy: momentum
   Symbol:   ETH-PERP
   Venue:    hyperliquid

Press Ctrl+C to stop

[2024-01-15 10:30:00] Connected to Hyperliquid testnet
[2024-01-15 10:30:01] Strategy started
[2024-01-15 10:30:05] Market order: Buy 0.5 ETH-PERP @ 2,450.00
[2024-01-15 10:30:05] Order filled: +0.5 ETH-PERP
```

---

## Development Commands

### `neleus ui`

Start the web dashboard for visual strategy development and monitoring.

```bash
neleus ui [OPTIONS]
```

**Options:**
- `--port, -p <port>` - Port number (default: 8765)
- `--host, -h <host>` - Host address (default: "127.0.0.1")
- `--no-browser` - Don't auto-open browser

**Examples:**
```bash
# Start dashboard (opens browser automatically)
neleus ui

# Custom port
neleus ui --port 8080

# Bind to all interfaces
neleus ui --host 0.0.0.0 --port 8765

# Don't open browser
neleus ui --no-browser
```

**Dashboard Features:**
- 📊 **Charts**: TradingView-style price charts
- 📈 **Portfolio**: Real-time positions and P&L
- ⚙️ **Strategy Editor**: Edit and test strategies
- 🔄 **Backtesting**: Run backtests with visual results
- 📉 **Risk Metrics**: VaR, drawdown, Sharpe ratio
- 📝 **Logs**: Real-time strategy logs

**Access:** `http://localhost:8765`

---

### `neleus build`

Validate and compile the project.

```bash
neleus build
```

**Checks:**
- ✓ Validates `neleus.toml` configuration
- ✓ Checks all strategy files for syntax errors
- ✓ Validates environment configuration
- ✓ Tests venue connections

**Output:**
```
🔨 Building project...

  ✓ Validating neleus.toml
  ✓ Checking strategies
  ✓ Validating configuration

Build successful!
```

---

### `neleus test`

Run strategy tests.

```bash
neleus test [OPTIONS]
```

**Options:**
- `--strategy <name>` - Test specific strategy
- `--coverage` - Generate coverage report

**Examples:**
```bash
# Test all strategies
neleus test

# Test specific strategy
neleus test --strategy momentum

# With coverage
neleus test --coverage
```

---

## Configuration Files

### neleus.toml

Project configuration file.

```toml
[project]
name = "my_trading_bot"
version = "0.1.0"
description = "A Neleus trading project"

[trading]
default_venue = "hyperliquid"
network = "testnet"
default_timeframe = "1h"

[backtest]
initial_capital = 100000.0
commission_bps = 5.0
slippage_bps = 2.0
slippage_model = "fixed"

[risk]
max_position_pct = 10.0
max_daily_loss_pct = 5.0
max_leverage = 5.0
dynamic_limits = true

[ui]
port = 8765
auto_open = true
theme = "dark"

[logging]
level = "info"
file = "logs/neleus.log"
```

### .env

Environment variables for API keys (DO NOT COMMIT).

```bash
# Hyperliquid
HYPERLIQUID_PRIVATE_KEY=your_key_here
HYPERLIQUID_NETWORK=testnet

# Lighter
LIGHTER_API_KEY=your_key_here
LIGHTER_PRIVATE_KEY=your_key_here
LIGHTER_NETWORK=testnet

# UI
NELEUS_UI_PORT=8765
```

---

## Common Workflows

### 1. Create and Test a New Strategy

```bash
# Create project
neleus new my_bot
cd my_bot

# Add strategy
neleus strategy add my_strategy

# Edit strategy
code strategies/my_strategy.py

# Test with backtest
neleus backtest --strategy my_strategy --symbol ETH-PERP

# View in UI
neleus ui
```

### 2. Paper Trade a Strategy

```bash
# Configure API keys
cp .env.example .env
# Edit .env with your testnet keys

# Start paper trading
neleus live --strategy momentum --paper --symbol BTC-PERP

# Monitor in UI (separate terminal)
neleus ui
```

### 3. Deploy to Production

```bash
# Test thoroughly in paper mode first!
neleus live --strategy momentum --paper

# When ready, switch to live (testnet first!)
neleus live --strategy momentum --real --venue hyperliquid

# Monitor
neleus ui
```

### 4. Optimize Parameters

```bash
# Run multiple backtests with different parameters
for lookback in 10 20 30 40; do
  neleus backtest \
    --strategy momentum \
    --symbol ETH-PERP \
    --config "lookback=$lookback"
done

# Use walk-forward analysis in Python
python scripts/optimize.py
```

---

## Tips and Best Practices

### 1. Always Use Version Control

```bash
git init
git add .
git commit -m "Initial Neleus project"
```

### 2. Keep Secrets Secure

```bash
# Never commit .env
echo ".env" >> .gitignore

# Use environment-specific files
.env.testnet
.env.mainnet
```

### 3. Test Before Live Trading

```bash
# Order: Backtest → Paper → Live
neleus backtest --strategy my_strategy    # 1. Backtest
neleus live --paper --strategy my_strategy # 2. Paper
neleus live --real --strategy my_strategy  # 3. Live (careful!)
```

### 4. Monitor Logs

```bash
# Watch logs in real-time
tail -f logs/neleus.log

# Or use the dashboard
neleus ui
```

### 5. Use the Dashboard

The UI dashboard is your best friend:
- Live monitoring
- Visual backtesting
- Quick parameter tweaking
- Performance analytics

```bash
neleus ui --port 8765
```

---

## Managed Service Commands

### `neleus deploy`

Deploy trading agents to the orchestrator.

```bash
neleus deploy <name> [OPTIONS]
```

**Arguments:**
- `<name>` - Agent name to deploy

**Options:**
- `--config, -c <path>` - Path to agent config YAML
- `--strategy, -s <id>` - Strategy ID
- `--venue, -v <venue>` - Trading venue (default: hyperliquid)
- `--instruments, -i <list>` - Comma-separated instruments
- `--capital <float>` - Initial capital (default: 10000.0)
- `--mode, -m <mode>` - Trading mode: paper/live (default: paper)
- `--testnet/--mainnet` - Use testnet (default: testnet)
- `--url <url>` - Orchestrator URL

**Examples:**
```bash
# Deploy with CLI options
neleus deploy momentum-eth --strategy momentum_v2 --instruments ETH-PERP --capital 5000

# Deploy with config file
neleus deploy my-agent -c agent-config.yaml

# Batch deploy multiple agents
neleus deploy batch -c agents-batch.yaml

# Generate config template
neleus deploy template -o my-agent.yaml
```

---

### `neleus agents`

Manage deployed trading agents.

```bash
neleus agents <command> [OPTIONS]
```

**Commands:**

#### `neleus agents list`
List all deployed agents.
```bash
neleus agents list
```

#### `neleus agents status <agent-id>`
Get detailed status of an agent.
```bash
neleus agents status momentum-eth-01
```

#### `neleus agents start <agent-id>`
Start a stopped or paused agent.
```bash
neleus agents start momentum-eth-01
```

#### `neleus agents stop <agent-id>`
Stop a running agent.
```bash
neleus agents stop momentum-eth-01
```

#### `neleus agents pause <agent-id>`
Pause a running agent (can be resumed).
```bash
neleus agents pause momentum-eth-01
```

#### `neleus agents resume <agent-id>`
Resume a paused agent.
```bash
neleus agents resume momentum-eth-01
```

#### `neleus agents restart <agent-id>`
Restart an agent.
```bash
neleus agents restart momentum-eth-01
```

#### `neleus agents delete <agent-id>`
Delete an agent permanently.
```bash
neleus agents delete momentum-eth-01 --force
```

#### `neleus agents logs <agent-id>`
View agent logs.
```bash
neleus agents logs momentum-eth-01 --lines 100 --follow
```

---

### `neleus signals`

Send and manage external signals.

```bash
neleus signals <command> [OPTIONS]
```

**Commands:**

#### `neleus signals send`
Send a signal to the Signal Hub.
```bash
neleus signals send -i ETH-PERP -t entry -d long -c 0.85
neleus signals send -i BTC-PERP -t exit -d flat -s my_model
```

**Options:**
- `--instrument, -i <symbol>` - Instrument (required)
- `--type, -t <type>` - Signal type: entry, exit, scale_in, scale_out, risk_alert
- `--direction, -d <dir>` - Direction: long, short, flat
- `--confidence, -c <float>` - Confidence (0.0 to 1.0)
- `--source, -s <name>` - Signal source identifier
- `--metadata, -m <json>` - JSON metadata

#### `neleus signals list`
List recent signals.
```bash
neleus signals list -n 20
neleus signals list -i ETH-PERP -t entry
```

#### `neleus signals test`
Send a test signal to verify connectivity.
```bash
neleus signals test
neleus signals test --agent momentum-eth-01
```

#### `neleus signals subscribe <agent-id>`
Create a signal subscription for an agent.
```bash
neleus signals subscribe momentum-eth-01 -i ETH-PERP,BTC-PERP
```

#### `neleus signals sources`
List known signal sources.
```bash
neleus signals sources
```

---

### `neleus metrics`

View agent metrics and performance.

```bash
neleus metrics <command> [OPTIONS]
```

**Commands:**

#### `neleus metrics get <agent-id>`
Get metrics for a specific agent.
```bash
neleus metrics get momentum-eth-01
neleus metrics get momentum-eth-01 -c pnl
neleus metrics get momentum-eth-01 --format json
```

**Options:**
- `--category, -c <cat>` - Category: pnl, trades, positions, risk, latency
- `--format, -f <fmt>` - Output format: table, json

#### `neleus metrics summary`
Show metrics summary for all agents.
```bash
neleus metrics summary
```

#### `neleus metrics history <agent-id>`
View historical metrics for an agent.
```bash
neleus metrics history momentum-eth-01 -m pnl -p 7d
neleus metrics history momentum-eth-01 --format csv
```

**Options:**
- `--metric, -m <name>` - Metric: pnl, win_rate, sharpe, trades
- `--period, -p <period>` - Period: 1h, 24h, 7d, 30d

#### `neleus metrics export <agent-id>`
Export agent metrics to a file.
```bash
neleus metrics export momentum-eth-01 -o metrics.json
neleus metrics export momentum-eth-01 -p 90d -o quarter.json
```

#### `neleus metrics alerts`
View active alerts and notifications.
```bash
neleus metrics alerts
neleus metrics alerts --severity critical
neleus metrics alerts --agent momentum-eth-01
```

#### `neleus metrics dashboard`
Launch terminal dashboard for real-time metrics.
```bash
neleus metrics dashboard
neleus metrics dashboard --refresh 10
```

---

## Environment Variables

Managed service commands use these environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `NELEUS_ORCHESTRATOR_URL` | Agent orchestrator URL | `http://localhost:8080` |
| `NELEUS_SIGNAL_HUB_URL` | Signal hub URL | `http://localhost:8081` |
| `NELEUS_MONITOR_URL` | Monitor service URL | `http://localhost:8082` |

---

## Troubleshooting

### Command Not Found

```bash
# Ensure Neleus is installed
pip install neleus

# Or install from source
pip install -e python/
```

### Strategy Not Found

```bash
# List available strategies
neleus strategy list

# Check you're in a Neleus project
neleus info
```

### API Connection Issues

```bash
# Check .env file
cat .env

# Test connection
neleus build

# Use testnet first
# In .env: HYPERLIQUID_NETWORK=testnet
```

### Build Failures

```bash
# Ensure Rust extension is built
cd crates/pybridge
maturin develop --release

# Rebuild Python package
pip install -e python/ --force-reinstall
```

---

## Getting Help

```bash
# General help
neleus --help

# Command-specific help
neleus backtest --help
neleus live --help
neleus strategy --help

# Version info
neleus --version
```

For more help:
- 📚 [Documentation](./index.md)
- 💬 [GitHub Discussions](https://github.com/auralshin/neleus/discussions)
- 🐛 [Report Issues](https://github.com/auralshin/neleus/issues)
