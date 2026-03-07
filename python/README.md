# Neleus Python Package

Neleus is a Hyperliquid-first CLI and Python toolkit backed by a Rust core.

The package provides:
- the `neleus` CLI
- Python strategy scaffolding and runtime helpers
- Hyperliquid market access through the Rust bridge

## Install From Source

```bash
pip install maturin
maturin develop --release -m ../crates/pybridge/Cargo.toml
pip install -e .
```

## Usage

No-project market workflows:

```bash
neleus market search BTC
neleus market scan --scope perps
neleus market book BTC-PERP
```

Project workflow:

```bash
neleus new my_strategy_project
cd my_strategy_project
neleus backtest --strategy momentum
```

Full documentation lives in [`../docs/index.md`](../docs/index.md) and can be hosted with Zensical.
