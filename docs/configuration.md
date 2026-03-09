# Configuration

Projects use `neleus.toml` as the primary config file. Secrets (private keys, DSNs) always live in a local `.env` file that is never committed.

---

## Default Scaffold

Running `neleus new my_bot` generates this `neleus.toml`:

```toml
[project]
name = "my_bot"
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

---

## `[hyperliquid]` Section

Controls which Hyperliquid network the runtime and market commands target.

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `testnet` | bool | `false` | When `true`, all API calls go to the Hyperliquid testnet. Overridable via `HYPERLIQUID_TESTNET`. |

**Never put your private key in `neleus.toml`.** Use `.env` or environment variables instead — see [Credentials](#credentials) below.

---

## `[market]` Section

Default values used by `neleus run` and `neleus backtest` when flags are not explicitly passed.

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `symbol` | string | `"BTC-PERP"` | Market symbol. Use the raw coin name (`BTC`, `ETH`) or the `-PERP` suffixed form. |
| `timeframe` | string | `"1h"` | Candle interval. Accepted values: `1m`, `5m`, `15m`, `1h`, `4h`, `1d`. |
| `lookback_bars` | int | `200` | How many bars to fetch per polling cycle. Minimum 20 for most indicators. |

---

## `[backtest]` Section

Controls cost simulation and initial capital for `neleus backtest`.

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `initial_capital` | float | `10000.0` | Starting capital in USDC. |
| `maker_fee_bps` | float | `2.0` | Maker fee in basis points (2.0 = 0.02 %). |
| `taker_fee_bps` | float | `5.0` | Taker fee in basis points (5.0 = 0.05 %). |
| `slippage_bps` | float | `5.0` | Simulated slippage in basis points. |

---

## `[runtime]` Section

Controls the default behavior of `neleus run`.

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `mode` | string | `"once"` | Default run mode. `"once"` or `"daemon"`. Can be overridden with `--mode`. |
| `poll_interval_seconds` | int | `60` | Seconds between polling cycles in daemon mode. Can be overridden with `--interval-seconds`. |

---

## `[database]` Section

Configures the optional persistence adapter.

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `backend` | string | `"none"` | Adapter: `"none"`, `"postgres"`, or `"timescale"`. |
| `dsn` | string | `""` | PostgreSQL connection string. **Keep this empty in committed config.** Use `NELEUS_DB_DSN` in `.env` instead. |
| `pool_size` | int | `4` | Connection pool size. |
| `batch_size` | int | `1000` | Batch insert size (TimescaleStore only). |
| `flush_interval_ms` | int | `100` | Flush interval in milliseconds (TimescaleStore only). |
| `trade_monitoring` | bool | `false` | When `true`, the runtime automatically records every generated order via `TradeMonitor`. |

---

## Credentials

Private keys and wallet addresses are **never stored in `neleus.toml`**. They belong in `.env`.

### Setting credentials at project creation

```bash
neleus new my_bot \
  --private-key 0xYOUR_PRIVATE_KEY \
  --account-address 0xYOUR_WALLET_ADDRESS
```

Neleus writes the values directly into `.env` inside the project directory. The committed `neleus.toml` is left clean.

### Setting credentials manually

After creating a project, copy `.env.example` to `.env` and fill in the values:

```bash
cp .env.example .env
```

Edit `.env`:

```bash
HYPERLIQUID_ACCOUNT_ADDRESS=0xYourWalletAddress
HYPERLIQUID_SIGNER_PRIVATE_KEY=0xYourPrivateKey
HYPERLIQUID_TESTNET=false
```

### Credential lookup order

When the runtime needs credentials it checks in this order:

1. `[hyperliquid]` section in `neleus.toml` (if keys are explicitly set there)
2. `.env` file in the project root (loaded automatically by `load_project_config`)
3. Shell environment variables

This means setting `HYPERLIQUID_SIGNER_PRIVATE_KEY` in your shell before running `neleus run --live` also works without a `.env` file.

---

## Environment Variables

All supported environment overrides. Set these in `.env` or export them in your shell.

### Hyperliquid

| Variable | Description |
| --- | --- |
| `HYPERLIQUID_SIGNER_PRIVATE_KEY` | Hex-encoded private key for signing orders (with or without `0x` prefix). Required for `neleus run --live`. |
| `HYPERLIQUID_ACCOUNT_ADDRESS` | Wallet address (`0x...`). Used to query open orders and fills. |
| `HYPERLIQUID_TESTNET` | Set to `true`, `1`, or `yes` to switch all API calls to testnet. |

### Database

| Variable | Description |
| --- | --- |
| `NELEUS_DB_BACKEND` | Override `database.backend`. Accepted: `none`, `postgres`, `timescale`. |
| `NELEUS_DB_DSN` | Override `database.dsn`. Full PostgreSQL connection string. Takes precedence over `neleus.toml`. |
| `NELEUS_DB_POOL_SIZE` | Override `database.pool_size`. |

### Risk

| Variable | Description |
| --- | --- |
| `NELEUS_MAX_POSITION_SIZE` | Override `risk.max_position_size`. |
| `NELEUS_MAX_DRAWDOWN_PCT` | Override `risk.max_drawdown_pct`. |
| `NELEUS_DAILY_LOSS_LIMIT` | Override `risk.daily_loss_limit`. |

### Logging

| Variable | Description |
| --- | --- |
| `NELEUS_LOG_LEVEL` | Override `logging.level`. E.g. `DEBUG`, `INFO`, `WARNING`. |

---

## Scaffolding With a Database

Pass `--db-backend` and `--db-dsn` at project creation time to wire up persistence immediately:

```bash
neleus new my_bot \
  --db-backend postgres \
  --db-dsn postgresql://user:password@localhost:5432/neleus \
  --trade-monitoring
```

- `database.dsn` in `neleus.toml` is left empty
- the DSN is written to `.env` as `NELEUS_DB_DSN=...`
- `.gitignore` already excludes `.env`

Initialize the schema immediately after scaffolding:

```bash
cd my_bot
neleus db init
```

For TimescaleDB:

```bash
neleus new my_bot \
  --db-backend timescale \
  --db-dsn postgresql://user:password@localhost:5432/neleus_ts \
  --trade-monitoring

cd my_bot
neleus db init   # creates hl_orders, hl_fills, and TimescaleDB hypertables
```

---

## Runtime Monitoring Behavior

When `database.trade_monitoring = true` and a DSN is configured, the runtime does this on every polling cycle:

1. Resolves `DatabaseConfig` from `neleus.toml` and env overrides
2. Creates a `TradeMonitor` once at startup (not per-bar)
3. After each bar, drains order requests from `StrategyContext`
4. In live mode (`--live`), submits each order to the exchange first
5. Records each generated order to the database with a UUID `cloid` and the project `testnet` flag

The strategy code does not need explicit DB calls for this to work.

---

## Python Config API

```python
from pathlib import Path
from neleus import get_db_config, get_hyperliquid_credentials, load_project_config

config = load_project_config(Path("neleus.toml"))

# Database config
db_cfg = get_db_config(config)
print(db_cfg.backend)          # "postgres"
print(db_cfg.trade_monitoring) # True
print(db_cfg.dsn)              # resolved from config or NELEUS_DB_DSN

# Hyperliquid credentials
creds = get_hyperliquid_credentials(config)
print(creds["signer_private_key"])  # resolved from config or HYPERLIQUID_SIGNER_PRIVATE_KEY
print(creds["account_address"])     # resolved from config or HYPERLIQUID_ACCOUNT_ADDRESS
```

---

## Security Notes

- **Never commit `.env`.** The scaffolded `.gitignore` already excludes it.
- **Never put private keys in `neleus.toml`.** Use `.env` or environment variables.
- The `.env.example` file is safe to commit — it contains placeholder values only.
- When using `neleus new --private-key`, the key goes straight to `.env` and never touches `neleus.toml`.
- `neleus db status` masks the password portion of the DSN before printing it.
