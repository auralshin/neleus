# CLI Overview

The CLI is split into two layers:

- Global market commands that work without a project
- Project commands that operate on a directory containing `neleus.toml`

## Global Commands

| Command | Purpose |
| --- | --- |
| `neleus about` | Show branding, links, and a short command guide |
| `neleus market search <query>` | Find markets by name across supported scopes |
| `neleus market list` | Show market catalogs for perps, HIP-3, all-perps, or spot |
| `neleus market analyze <symbol>` | Run single-market technical analysis |
| `neleus market scan` | Rank a bounded set of markets by scan score |
| `neleus market book <symbol>` | Stream a live L2 order book in the terminal |

## Project Commands

| Command | Purpose |
| --- | --- |
| `neleus new <name>` | Scaffold a new project, optionally with DB monitoring enabled |
| `neleus init` | Initialize the current directory as a project |
| `neleus backtest` | Run strategy backtests |
| `neleus run --mode once|daemon` | Execute the runtime once or continuously |
| `neleus strategy list|new|show` | Manage strategy source files |
| `neleus db status` | Show the configured backend, DSN, pool size, and monitoring state |
| `neleus db init` | Initialize the `hl_orders` / `hl_fills` schema and Timescale tables when applicable |
| `neleus info` | Show project configuration and strategy discovery |

## Current Limits

- A dedicated `neleus trade` command is not exposed yet.
- Database support currently focuses on runtime order/fill monitoring through `TradeMonitor`.
- The TA scanner intentionally limits how many markets it scores per run so terminal use stays responsive.
