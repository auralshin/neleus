<p align="center">
  <img src="./logo.png" alt="Neleus logo" width="148" />
</p>

<h1 align="center">Neleus</h1>

<p align="center">
  <em>Trade Hyperliquid from the terminal.</em>
</p>

<p align="center">
  A Hyperliquid-first CLI and Python toolkit powered by a Rust core.
  Search markets, run technical scans, stream live order books,
  backtest strategies, and scaffold Python trading projects.
</p>


<p align="center">
  <a href="https://neleus.trade">Website</a>
  ·
  <a href="https://auralshin.github.io/neleus/">Docs</a>
  ·
  <a href="https://github.com/auralshin/neleus">GitHub</a>
</p>

<p align="center">
  <img alt="Rust Core" src="https://img.shields.io/badge/Core-Rust-0b1220?style=for-the-badge&logo=rust" />
  <img alt="CLI" src="https://img.shields.io/badge/Interface-CLI-0b1220?style=for-the-badge&logo=gnubash" />
  <img alt="Venue" src="https://img.shields.io/badge/Venue-Hyperliquid-0b1220?style=for-the-badge" />
</p>

## What Neleus Does

| Workflow | What You Get |
| --- | --- |
| Market discovery | Search and list Hyperliquid spot, default perps, and HIP-3 markets |
| Technical analysis | Single-market reads with RSI, trend, momentum, support, and resistance |
| Market scanning | Ranked terminal scans across a bounded set of markets |
| Live monitoring | Real-time L2 order book view from the Rust WebSocket path |
| Strategy development | Python project scaffolding with a Rust-backed runtime |
| Backtesting | Run strategy backtests against Hyperliquid market data |

## Quick Start

```bash
python3 -m venv .venv
source .venv/bin/activate

pip install maturin
cd python
maturin develop --release
pip install -e .
```

No project is required for the market-facing commands:

```bash
neleus about
neleus market search TSLA --scope hip3
neleus market analyze GAS --scope hip3 --dex flx
neleus market scan --scope perps
neleus market book flx:GAS-PERP
```

When you want to write strategy code:

```bash
neleus new my_strategy_project
cd my_strategy_project

neleus backtest --strategy momentum
neleus run --mode once --strategy momentum
neleus run --mode daemon --strategy momentum
```

## CLI Highlights

```text
neleus about
neleus market search <query>
neleus market list --scope perps|hip3|all-perps|spot
neleus market analyze <symbol> [--scope ...] [--dex ...]
neleus market scan --scope perps|hip3|all-perps|spot
neleus market book <symbol> [--scope ...] [--dex ...]
neleus new <name>
neleus init
neleus backtest
neleus run --mode once|daemon
neleus strategy list|new|show
neleus db status
neleus db init
neleus info
```

## Documentation

The full usage guides live in [`docs/index.md`](./docs/index.md) and are
deployed to [GitHub Pages](https://auralshin.github.io/neleus/).

Run the docs locally:

```bash
pip install -r docs/requirements.txt
mkdocs serve
```

Build the static site:

```bash
mkdocs build
```

The docs cover:
- installation and local setup
- no-project CLI workflows
- simple market search across perps, HIP-3, and spot
- market search, list, analysis, scan, and live order book usage
- HIP-3 routing details for commands like `neleus market analyze GAS --scope hip3 --dex flx`
- project scaffolding and strategy commands
- runtime and backtesting
- configuration, database adapters, and Hyperliquid usage notes

## Current Scope

Implemented now:
- no-project market search, listing, analysis, scans, and live order book monitoring
- Python project scaffolding
- strategy backtesting
- one-shot and daemon strategy runtimes
- database-backed runtime order/fill monitoring through `TradeMonitor`
- database schema inspection and initialization through `neleus db status` and `neleus db init`

Not implemented yet:
- a dedicated `neleus trade` command separate from the current runtime/core APIs
- broader live operations tooling beyond the current trade-monitoring path

## Links

- Website: [https://neleus.trade](https://neleus.trade)
- GitHub: [https://github.com/auralshin/neleus](https://github.com/auralshin/neleus)
- Docs: [https://auralshin.github.io/neleus/](https://auralshin.github.io/neleus/)
- Docs source: [./docs/index.md](./docs/index.md)
