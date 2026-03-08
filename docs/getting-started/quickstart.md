# Quickstart

This page focuses on workflows that work before you create a project.

## No-Project Commands

```bash
neleus about
neleus market search GAS --scope all-perps
neleus market list --scope perps
neleus market analyze GAS --scope hip3 --dex flx
neleus market scan --scope perps
neleus market book flx:GAS-PERP
```

If you only know the plain asset name for a HIP-3 market, use `--scope hip3 --dex <dex>`. Neleus resolves the market and routes the Hyperliquid request correctly.

## Mainnet By Default

The market-facing commands use Hyperliquid mainnet by default.

Use testnet only when you need it:

```bash
neleus market search BTC --testnet
```

## Create A Strategy Project

```bash
neleus new my_strategy_project
cd my_strategy_project
```

With PostgreSQL-backed trade monitoring enabled from the start:

```bash
neleus new my_strategy_project \
  --db-backend postgres \
  --db-dsn postgresql://user:password@localhost:5432/neleus \
  --trade-monitoring

cd my_strategy_project
neleus db init
```

The scaffold writes the DSN into a local `.env` file and leaves
`database.dsn = ""` in committed `neleus.toml`, so credentials do not need to
live in source control.

Generated layout:

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

## Run A Backtest

```bash
neleus backtest --strategy momentum
```

## Run The Strategy Runtime

One-shot:

```bash
neleus run --mode once --strategy momentum
```

Daemon:

```bash
neleus run --mode daemon --strategy momentum
```

## Database-Backed Trade Monitoring

When your project has:

- `database.backend = "postgres"` or `database.backend = "timescale"`
- a configured DSN
- `database.trade_monitoring = true`

the project runtime will automatically connect to the database and record every generated order from:

- `neleus run --mode once`
- `neleus run --mode daemon`
- `run_project_once(...)`
- `run_project_daemon(...)`
