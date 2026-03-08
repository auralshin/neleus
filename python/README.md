# Neleus

Neleus is a Hyperliquid-first CLI and Python toolkit backed by a Rust core.

It is built for terminal-first trading workflows:

- search Hyperliquid spot, perp, and HIP-3 markets
- run technical analysis and ranked market scans
- stream live order book data in the terminal
- scaffold Python strategy projects
- backtest strategies against Hyperliquid market data
- run strategies once or as a daemon with optional database-backed trade monitoring

## Links

- GitHub: [https://github.com/auralshin/neleus](https://github.com/auralshin/neleus)
- Documentation: [https://auralshin.github.io/neleus/](https://auralshin.github.io/neleus/)
- Website: [https://neleus.trade](https://neleus.trade)

## Install From Source

```bash
pip install maturin
maturin develop --release
pip install -e .
```

## CLI Examples

No-project market workflows:

```bash
neleus about
neleus market search GAS --scope all-perps
neleus market list --scope hip3 --dex xyz
neleus market analyze GAS --scope hip3 --dex flx
neleus market scan --scope perps
neleus market book flx:GAS-PERP
```

Project workflow:

```bash
neleus new my_strategy_project --db-backend postgres --trade-monitoring
cd my_strategy_project
neleus db init
neleus backtest --strategy momentum
neleus run --mode once --strategy momentum
```

## Python Strategy Workflow

Neleus scaffolds a Python project with:

- `neleus.toml` for market, runtime, and database settings
- a `strategies/` directory for strategy code
- runtime helpers like `run_project_once(...)` and `run_project_daemon(...)`
- a Rust bridge for market data, Hyperliquid adapters, and core trading types

For full usage guides, strategy examples, and configuration details, see the
hosted docs: [https://auralshin.github.io/neleus/](https://auralshin.github.io/neleus/)
