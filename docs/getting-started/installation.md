# Installation

Neleus currently requires the Rust extension. The CLI and Python package sit on top of the `neleus_core` module built from `crates/pybridge`.

## Requirements

- Python 3.10+
- Rust toolchain
- `maturin`

## Install From Source

```bash
python3 -m venv .venv
source .venv/bin/activate

pip install maturin
maturin develop --release -m crates/pybridge/Cargo.toml
pip install -e python/
```

## Verify The Install

```bash
neleus about
neleus market search BTC
```

If the Rust extension is missing, Neleus will fail early on import. Build the pybridge first, then reinstall the Python package.

## Documentation Site

This repository ships a Zensical docs site under `docs/`.

Run it locally:

```bash
pip install -r docs/requirements.txt
zensical serve
```

Build static output:

```bash
zensical build
```
