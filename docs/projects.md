# Strategy Projects

Create a project when you want to write Python strategy code on top of the Neleus Rust core.

This page covers:

- how to scaffold a project with all supported options
- what files Neleus generates and what each one does
- the full recommended workflow from scaffold to live trading
- strategy discovery and config files
- the complete runtime execution flow

---

## Scaffold A New Project

Create a new project directory:

```bash
neleus new my_bot
cd my_bot
```

Initialize the current directory instead:

```bash
mkdir my_bot
cd my_bot
neleus init
```

### Scaffold With Live Trading Credentials

Pass credentials at creation time. They are written to `.env` and never touch `neleus.toml`:

```bash
neleus new my_bot \
  --private-key 0xYOUR_PRIVATE_KEY \
  --account-address 0xYOUR_WALLET_ADDRESS
```

Credentials can also be picked up from environment variables automatically — `neleus new` reads `HYPERLIQUID_SIGNER_PRIVATE_KEY` and `HYPERLIQUID_ACCOUNT_ADDRESS` if they are already set in the shell:

```bash
export HYPERLIQUID_SIGNER_PRIVATE_KEY=0xYOUR_PRIVATE_KEY
export HYPERLIQUID_ACCOUNT_ADDRESS=0xYOUR_WALLET_ADDRESS
neleus new my_bot    # writes them into .env automatically
```

### Scaffold With Database Monitoring

```bash
neleus new my_bot \
  --db-backend postgres \
  --db-dsn postgresql://user:password@localhost:5432/neleus \
  --trade-monitoring
```

Supported `--db-backend` values:

- `none` — no persistence (default)
- `postgres` — PostgreSQL event store + trade monitoring (`hl_orders`, `hl_fills` tables)
- `timescale` — TimescaleDB hypertables for market data + trade monitoring tables

When `--db-dsn` is passed, Neleus stores the value in a local `.env` file and keeps `database.dsn = ""` in `neleus.toml` so credentials are not committed.

### Scaffold With Everything

```bash
neleus new my_bot \
  --private-key 0xYOUR_PRIVATE_KEY \
  --account-address 0xYOUR_WALLET_ADDRESS \
  --db-backend postgres \
  --db-dsn postgresql://user:password@localhost:5432/neleus \
  --trade-monitoring
```

Then initialize the DB schema:

```bash
cd my_bot
neleus db init
```

---

## What Gets Generated

```text
my_bot/
├── .gitignore          # excludes .env and Python caches
├── .env                # created when --private-key or --db-dsn is passed; never committed
├── .env.example        # documents all supported env vars with placeholder values; safe to commit
├── neleus.toml         # project config; credentials are never written here
├── main.py             # simple Python entrypoint
└── strategies/
    ├── __init__.py
    └── momentum.py     # scaffold strategy
```

### `.gitignore`

Excludes `.env` and Python bytecode caches. You do not need to configure this.

### `.env`

Created only when `--private-key`, `--account-address`, or `--db-dsn` is passed. Contains your actual secrets. **Never committed.**

After scaffolding with credentials, `.env` looks like:

```bash
# Local secrets — do not commit this file.
HYPERLIQUID_SIGNER_PRIVATE_KEY=0xabc123...
HYPERLIQUID_ACCOUNT_ADDRESS=0xdef456...
```

You can edit this file at any time to add or change values.

### `.env.example`

Documents all supported environment variables with placeholder values. **Safe to commit.** Team members copy this to `.env` and fill in their own values.

```bash
HYPERLIQUID_TESTNET=false
HYPERLIQUID_ACCOUNT_ADDRESS=0x...
HYPERLIQUID_SIGNER_PRIVATE_KEY=0x...
NELEUS_DB_BACKEND=none
NELEUS_DB_DSN=
```

### `neleus.toml`

The project configuration file. Contains:

- project metadata (`name`, `version`)
- Hyperliquid network selection (`testnet = false`)
- default market (`symbol`, `timeframe`, `lookback_bars`)
- backtest cost parameters
- runtime mode and poll interval
- database adapter settings

Credentials are **never** written here. See [Configuration](configuration.md) for all available keys.

### `main.py`

A Python entry point for running the strategy without the CLI:

```python
from pathlib import Path
from neleus.runtime import run_project_once

if __name__ == "__main__":
    result = run_project_once(Path(__file__).parent)
    print(result.to_dict())
```

For live trading via Python directly:

```python
import os
from pathlib import Path
from neleus.runtime import run_project_once

result = run_project_once(
    Path(__file__).parent,
    private_key=os.environ["HYPERLIQUID_SIGNER_PRIVATE_KEY"],
)
print(result.to_dict())
```

### `strategies/momentum.py`

The scaffold strategy demonstrates the core pattern:

- subclass `Strategy`
- keep rolling state (e.g. price history) on `self`
- react to incoming `Bar` objects in `on_bar`
- emit order signals through `StrategyContext`

---

## Recommended Project Workflow

### 1. Scaffold

```bash
neleus new my_bot --private-key 0x... --account-address 0x...
cd my_bot
```

### 2. If using a database, initialize the schema

```bash
neleus db init
neleus db status    # verify the connection is healthy
```

### 3. Inspect and edit the strategy

```bash
neleus strategy list
neleus strategy show momentum
# edit strategies/momentum.py
```

### 4. Configure the market in `neleus.toml`

```toml
[market]
symbol = "ETH-PERP"
timeframe = "15m"
lookback_bars = 100
```

### 5. Backtest first

```bash
neleus backtest --strategy momentum
```

### 6. Dry-run — see what orders the strategy would generate

```bash
neleus run --mode once --strategy momentum
```

This fetches real candles, runs the strategy, and prints the generated orders. Nothing is submitted to the exchange.

### 7. Live run on testnet

```bash
neleus run --mode once --strategy momentum --testnet --live
```

With `--live`, `HyperliquidTrader` is instantiated with your private key from `.env`. Each order the strategy generates is signed and submitted to Hyperliquid's `/exchange` API.

### 8. Live run on mainnet

```bash
neleus run --mode once --strategy momentum --live
```

### 9. Run continuously in daemon mode

```bash
neleus run --mode daemon --strategy momentum --live
```

The daemon polls every `poll_interval_seconds`, processes new candles, and executes any orders the strategy generates on each cycle.

---

## How `neleus run` Executes Your Strategy

The runtime is bar-driven. On each cycle:

1. Load the strategy class from `strategies/<name>.py`
2. Instantiate it: `strategy_class()` — no constructor arguments are passed
3. Call `strategy.on_start(ctx)`
4. Fetch recent candles from Hyperliquid
5. For each candle:
   a. Convert the candle dict into a `Bar`
   b. Call `strategy.on_bar(ctx, bar)` then `strategy.on_data(ctx, bar)`
   c. Drain generated order requests via `ctx.drain_order_requests()`
   d. **If `--live`:** submit each order to Hyperliquid via `HyperliquidTrader`
   e. If trade monitoring is configured, record each order to the database
6. Call `strategy.on_stop(ctx)`

### Without `--live` (default, safe)

Generated orders are collected, displayed in the terminal output, and optionally recorded to the database. Nothing is submitted to Hyperliquid. This is the right mode for strategy development and testing.

### With `--live`

`HyperliquidTrader` is created once at startup with the private key from `.env` or environment. For each order generated:

- `market_order` call → `place_market_order(coin, is_buy, size, slippage_bps=50)`
- `limit_order` call with a price → `place_limit_order(coin, is_buy, size, price, reduce_only)`

The exchange response (order ID, fill status, any rejection message) is logged. If the exchange rejects an order, the error is logged and the strategy continues — a single rejected order does not stop the runtime.

---

## Live Trading Setup Checklist

Before running `neleus run --live`:

- [ ] `HYPERLIQUID_SIGNER_PRIVATE_KEY` is set in `.env`
- [ ] `HYPERLIQUID_ACCOUNT_ADDRESS` is set in `.env`
- [ ] Wallet has sufficient balance on the target network
- [ ] `[hyperliquid] testnet = false` in `neleus.toml` for mainnet (or `true` for testnet)
- [ ] Tested with `--testnet --live` first
- [ ] Verified expected order behavior using `neleus run --mode once` (no `--live`)

---

## Add More Strategies

```bash
neleus strategy new breakout
```

Creates `strategies/breakout.py` with a strategy scaffold.

```bash
neleus strategy list    # show all discovered strategies
neleus strategy show breakout  # print source
```

Neleus discovers strategies from `strategies/*.py`. Files starting with `_` are skipped.

---

## Strategy-Specific Config Files

Add optional config files under `configs/` for parameter control in backtests:

```text
my_bot/
├── configs/
│   └── momentum.yaml
└── strategies/
    └── momentum.py
```

`configs/momentum.yaml` example:

```yaml
strategy:
  enabled: true
  class: MomentumStrategy
  parameters:
    lookback: 30
```

`neleus backtest` reads `configs/<strategy>.yaml` and passes `strategy.parameters` to the strategy constructor.

`neleus run` instantiates the strategy with no constructor arguments — so all constructor parameters need defaults.

---

## Database-Backed Trade Monitoring

When `database.trade_monitoring = true` and a DSN is configured, the runtime automatically:

1. Creates a `TradeMonitor` once at startup
2. After each bar, assigns a UUID `cloid` to every generated order
3. Records the order to `hl_orders` with the project `testnet` flag

In live mode, execution happens before recording:

- Order is submitted to Hyperliquid
- Exchange response is logged (order ID, fill status)
- Order record is written to the database

The strategy does not need explicit DB calls.

---

## Useful Project Commands

```bash
# Project management
neleus info
neleus strategy list
neleus strategy new breakout
neleus strategy show breakout

# Database
neleus db status
neleus db init

# Development (no execution)
neleus backtest --strategy momentum
neleus run --mode once --strategy momentum

# Live on testnet
neleus run --mode once  --strategy momentum --testnet --live
neleus run --mode daemon --strategy momentum --testnet --live

# Live on mainnet
neleus run --mode once  --strategy momentum --live
neleus run --mode daemon --strategy momentum --live
```
