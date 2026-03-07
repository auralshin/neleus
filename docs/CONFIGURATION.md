# Configuration

Projects use `neleus.toml`.

## Default Scaffold

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

## Database Section

The active persistence section is `[database]`.

`database.backend` accepts these values:

- `none`: disable persistence entirely
- `postgres`: enable the PostgreSQL adapter
- `timescale`: enable the TimescaleDB adapter

Fields:

- `backend`: persistence mode and adapter selection
- `dsn`: PostgreSQL connection string
- `pool_size`: connection pool size
- `batch_size`: batch insert size for the Timescale path
- `flush_interval_ms`: flush interval for the Timescale path
- `trade_monitoring`: automatically record generated orders and fills via `TradeMonitor`

Recommended secret handling:

- keep `database.dsn = ""` in committed `neleus.toml`
- set `NELEUS_DB_DSN` in a local `.env`
- let the scaffolded `.gitignore` keep that `.env` file out of version control

## Environment Variables

```bash
# Public Hyperliquid market data uses /info and does not need credentials.
HYPERLIQUID_TESTNET=false

# For future signed trading flows.
# HYPERLIQUID_ACCOUNT_ADDRESS=0x...
# HYPERLIQUID_SIGNER_PRIVATE_KEY=

NELEUS_DB_BACKEND=postgres
NELEUS_DB_DSN=
```

Currently supported DB-related environment overrides:

- `NELEUS_DB_BACKEND`
- `NELEUS_DB_DSN`
- `NELEUS_DB_POOL_SIZE`

## Scaffold With Database Monitoring

```bash
neleus new my_strategy_project \
  --db-backend postgres \
  --db-dsn postgresql://user:password@localhost:5432/neleus \
  --trade-monitoring
```

When you pass `--db-dsn`, Neleus writes that value into a local `.env` file and
keeps `database.dsn` empty in `neleus.toml`.

Then initialize the schema:

```bash
cd my_strategy_project
neleus db init
```

## Runtime Monitoring Behavior

When `database.trade_monitoring = true` and a DSN is configured, the runtime:

1. resolves a `DatabaseConfig` from project config and environment overrides
2. creates a `TradeMonitor` once at startup
3. records every generated order after `ctx.drain_order_requests()`

Each generated order is stored with:

- a UUID client order ID (`cloid`)
- the project `testnet` flag

## Python Config API

The top-level package re-exports `DatabaseConfig` and `get_db_config`.

```python
from pathlib import Path

from neleus import DatabaseConfig, get_db_config, load_project_config

config = load_project_config(Path("neleus.toml"))
db_cfg: DatabaseConfig = get_db_config(config)

print(db_cfg.backend)
print(db_cfg.trade_monitoring)
print(db_cfg.dsn)
```

## Notes

- Mainnet is the default.
- The currently implemented CLI market workflows do not require signing credentials.
- `neleus db status` checks the configured database connection for the current project.
- `neleus db init` creates `hl_orders` / `hl_fills` through `TradeMonitor`.
- for `timescale`, `neleus db init` also initializes the TimescaleDB market-data schema.
