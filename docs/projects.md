# Strategy Projects

Create a project when you want to write Python strategy code on top of the Neleus Rust core.

This page covers:

- how to scaffold a project
- what files Neleus generates
- the recommended workflow from scaffold to runtime
- where strategy-specific config files live

## Scaffold A New Project

Create a new directory:

```bash
neleus new my_strategy_project
cd my_strategy_project
```

Initialize the current directory instead:

```bash
mkdir my_strategy_project
cd my_strategy_project
neleus init
```

### Scaffold With Database Monitoring

You can scaffold the database section directly from the CLI:

```bash
neleus new my_strategy_project \
  --db-backend postgres \
  --db-dsn postgresql://user:password@localhost:5432/neleus \
  --trade-monitoring
```

If you pass `--db-dsn`, Neleus stores it in a local `.env` file and keeps
`database.dsn = ""` in `neleus.toml` so credentials are not scaffolded into a
committed config file.

Supported `--db-backend` values:

- `none`: leave persistence disabled
- `postgres`: enable the PostgreSQL adapter
- `timescale`: enable the TimescaleDB adapter

## What Gets Generated

The scaffolded project looks like this:

```text
my_strategy_project/
├── .gitignore
├── .env                 # only created when --db-dsn is supplied
├── neleus.toml
├── .env.example
├── main.py
└── strategies/
    ├── __init__.py
    └── momentum.py
```

### `neleus.toml`

This is the project config. It holds:

- project metadata
- Hyperliquid network selection
- default market symbol and timeframe
- backtest parameters
- runtime mode and polling interval
- database adapter settings
- trade-monitoring settings

Scaffolded example:

```toml
[project]
name = "my_strategy_project"
version = "0.1.0"

[hyperliquid]
testnet = false

[market]
symbol = "BTC-PERP"
timeframe = "1h"
lookback_bars = 200

[backtest]
initial_capital = 10000.0
maker_fee_bps = 2.0
taker_fee_bps = 5.0
slippage_bps = 5.0

[runtime]
mode = "once"
poll_interval_seconds = 60

[database]
backend = "none"
dsn = ""
pool_size = 4
batch_size = 1000
flush_interval_ms = 100
trade_monitoring = false
```

### `.env.example`

This is where you document local overrides such as:

- `HYPERLIQUID_TESTNET`
- `NELEUS_DB_BACKEND`
- `NELEUS_DB_DSN`

If you scaffold with `--db-dsn`, Neleus also creates a local `.env` file and
adds `.env` to the project `.gitignore`.

### `main.py`

The scaffold includes a simple Python entrypoint:

```python
from pathlib import Path

from neleus.runtime import run_project_once


if __name__ == "__main__":
    result = run_project_once(Path(__file__).parent)
    print(result.to_dict())
```

You can run that directly with `python main.py`, but the CLI is the primary workflow:

```bash
neleus run --mode once
```

### `strategies/momentum.py`

This is your first strategy file. It shows the basic pattern:

- subclass `Strategy`
- keep your own state on `self`
- react to `on_bar`
- emit orders through `StrategyContext`

## Recommended Project Workflow

1. Scaffold the project with `neleus new`.
2. If you enabled a database backend, initialize the schema:

   ```bash
   neleus db init
   ```

3. Inspect the generated strategy:

   ```bash
   neleus strategy list
   neleus strategy show momentum
   ```

4. Edit `neleus.toml` to choose the market and timeframe you want.
5. Edit the strategy file under `strategies/`.
6. Backtest the strategy:

   ```bash
   neleus backtest --strategy momentum
   ```

7. Run the strategy once against recent market data:

   ```bash
   neleus run --mode once --strategy momentum
   ```

8. Run it continuously:

   ```bash
   neleus run --mode daemon --strategy momentum
   ```

## Add More Strategies

Create a new strategy file:

```bash
neleus strategy new breakout
```

That creates:

```text
strategies/breakout.py
```

List discovered strategies:

```bash
neleus strategy list
```

Neleus discovers strategies from `strategies/*.py`, skipping files that start with `_`.

## Strategy-Specific Config Files

You can add an optional config file for a strategy under `configs/`.

Example:

```text
my_strategy_project/
├── configs/
│   └── momentum.yaml
└── strategies/
    └── momentum.py
```

Example config:

```yaml
strategy:
  enabled: true
  class: MomentumStrategy
  parameters:
    lookback: 30
```

Important:

- `neleus backtest` reads `configs/<strategy>.yaml` and passes `strategy.parameters` into the strategy constructor.
- the current `neleus run` project runtime instantiates the strategy with no constructor arguments

That means constructor parameters should have defaults if you want the same strategy to work in both backtests and the runtime.

## Useful Project Commands

```bash
neleus strategy list
neleus strategy new breakout
neleus strategy show breakout
neleus db status
neleus db init
neleus info
neleus backtest --strategy breakout
neleus run --mode once --strategy breakout
neleus run --mode daemon --strategy breakout
```

## Database-Backed Trade Monitoring

When a project has:

- `database.backend = "postgres"` or `database.backend = "timescale"`
- a configured `database.dsn`
- `database.trade_monitoring = true`

the runtime will automatically:

1. create a `TradeMonitor` once at startup
2. run your strategy normally
3. drain generated order requests from each `StrategyContext`
4. record each generated order to the configured database

Each generated order is tagged with:

- a UUID client order ID (`cloid`)
- the project's `testnet` flag

This applies to:

- `neleus run --mode once`
- `neleus run --mode daemon`
- `run_project_once(...)`
- `run_project_daemon(...)`

The strategy code does not need any explicit DB calls for this flow.

## What The Current Runtime Actually Does

For project strategies, the current runtime flow is:

1. load the strategy class from `strategies/<name>.py`
2. instantiate it
3. call `on_start(...)`
4. fetch recent Hyperliquid candles
5. convert each candle into a `Bar`
6. call `on_bar(...)` and then `on_data(...)`
7. drain any generated order requests
8. if trade monitoring is enabled, record the generated orders through `TradeMonitor`
9. call `on_stop(...)`

That makes the current `neleus run` flow bar-driven and simple to reason about. It is useful for strategy development, but it is not yet a full live execution engine.

For the actual strategy-writing API, continue to [Writing Strategies](strategy-writing.md).
