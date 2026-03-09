# CLI Overview

The CLI is split into two layers:

- **Global commands** — work without any project; useful for market research
- **Project commands** — operate on a directory containing `neleus.toml`

---

## Global Commands

| Command | Description |
| --- | --- |
| `neleus about` | Show branding, links, and a short command guide |
| `neleus version` | Show the installed version |
| `neleus market search <query>` | Find markets by name. Supports `--scope` and `--dex` for HIP-3 |
| `neleus market list` | Show market catalogs. `--scope`: `perps`, `hip3`, `spot`, `all-perps` |
| `neleus market analyze <symbol>` | Single-market technical analysis with TA indicators. Supports `--scope`, `--dex`, `--timeframe`, `--lookback-bars` |
| `neleus market scan` | Rank a bounded set of markets by composite TA score. Supports `--scope`, `--symbols`, `--sort`, `--max-markets` |
| `neleus market book <symbol>` | Stream a live L2 order book in the terminal. Works with HIP-3 routed symbols |

---

## Project Commands

### Project Scaffolding

| Command | Description |
| --- | --- |
| `neleus new <name>` | Scaffold a new project directory |
| `neleus init` | Initialize the current directory as a project |

Both commands accept:

| Flag | Description |
| --- | --- |
| `--private-key` | Hyperliquid signer private key. Written to `.env`, never to `neleus.toml`. Also reads from `HYPERLIQUID_SIGNER_PRIVATE_KEY` env var. |
| `--account-address` | Hyperliquid wallet address (`0x...`). Written to `.env`. Also reads from `HYPERLIQUID_ACCOUNT_ADDRESS` env var. |
| `--db-backend` | Storage adapter: `none` (default), `postgres`, or `timescale` |
| `--db-dsn` | PostgreSQL connection string. Written to `.env` if provided. |
| `--trade-monitoring` | Enable automatic order/fill recording via `TradeMonitor` |

### Strategy Execution

| Command | Description |
| --- | --- |
| `neleus run` | Execute the project strategy |
| `neleus backtest` | Run a backtest against Hyperliquid historical candles |

**`neleus run` flags:**

| Flag | Default | Description |
| --- | --- | --- |
| `--mode` | `once` | `once` runs one polling cycle; `daemon` polls continuously |
| `--strategy` | auto-detected | Strategy name (stem of the file in `strategies/`) |
| `--symbol` | from `neleus.toml` | Override the market symbol |
| `--timeframe` | from `neleus.toml` | Override the candle interval |
| `--lookback-bars` | from `neleus.toml` | Override the number of candles fetched |
| `--interval-seconds` | from `neleus.toml` | Override the daemon poll interval |
| `--testnet` / `--mainnet` | from `neleus.toml` | Force testnet or mainnet |
| `--live` | `false` | **Enable live order execution.** Requires `HYPERLIQUID_SIGNER_PRIVATE_KEY` in `.env` or environment. |

Without `--live`, the runtime collects generated orders and displays them but never submits anything to the exchange. This is the safe default for development.

With `--live`, each order the strategy generates is signed and submitted to Hyperliquid's `/exchange` API via the Rust `HyperliquidTrader`.

### Strategy Management

| Command | Description |
| --- | --- |
| `neleus strategy list` | List strategy files discovered in `strategies/` |
| `neleus strategy new <name>` | Create a new strategy file from the scaffold template |
| `neleus strategy show <name>` | Print strategy source code to the terminal |

### Database

| Command | Description |
| --- | --- |
| `neleus db status` | Show the configured backend, DSN (masked), pool size, and monitoring state |
| `neleus db init` | Create `hl_orders` / `hl_fills` tables. For `timescale`, also creates TimescaleDB hypertables |

### Project Info

| Command | Description |
| --- | --- |
| `neleus info` | Show project configuration, discovered strategies, and DB connection status |

---

## Live Trading Flow

```bash
# 1. Scaffold with credentials
neleus new my_bot --private-key 0x... --account-address 0x...

# 2. Develop and backtest
cd my_bot
neleus backtest --strategy momentum

# 3. Dry-run (no execution)
neleus run --mode once --strategy momentum

# 4. Live on testnet
neleus run --mode once --strategy momentum --testnet --live

# 5. Live on mainnet
neleus run --mode daemon --strategy momentum --live
```

---

## Notes

- All market commands default to Hyperliquid mainnet. Use `--testnet` when needed.
- `--live` requires `HYPERLIQUID_SIGNER_PRIVATE_KEY` in `.env` or the shell environment. Without it, the command exits with a clear error message.
- The TA scanner is intentionally bounded with `--max-markets` to keep the terminal responsive. Use `--symbols` or `--search` to target specific markets.
- `neleus db status` masks the password portion of the DSN in its output.
