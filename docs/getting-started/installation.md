# Installation

Neleus requires the Rust extension (`neleus_core`) compiled via PyO3/maturin. The CLI and Python package are thin wrappers around it — all market data, execution, and backtesting logic lives in Rust.

## Requirements

- Python 3.10+
- Rust toolchain (`rustup.rs`)
- `maturin` (installed below)

## Install From Source

All commands run from the **repository root**. Use the root `.venv` — do not create one inside `python/`.

```bash
# 1. Create and activate the virtual environment
python3 -m venv .venv
source .venv/bin/activate     # macOS / Linux
# .venv\Scripts\activate      # Windows

# 2. Install maturin
pip install maturin

# 3. Build and install the Rust extension
VIRTUAL_ENV=$PWD/.venv \
  .venv/bin/maturin develop --release -m crates/pybridge/Cargo.toml

# 4. Install the Python package in editable mode
pip install -e python/
```

### Apple Silicon — code-signing fix

If Python crashes with `SIGKILL (Code Signature Invalid)` after the build:

```bash
codesign -s - --force python/neleus/neleus_core.cpython-313-darwin.so
```

This is required when macOS copies the `.so` rather than compiling it in place.

## Verify The Install

```bash
neleus --help
neleus market list --scope perps
```

If the Rust extension is missing, Neleus will fail at import with a clear error message. Run step 3 above and reinstall.

## Rebuild After Rust Changes

Any edit to a Rust crate requires a rebuild before the Python CLI sees the change:

```bash
VIRTUAL_ENV=$PWD/.venv \
  .venv/bin/maturin develop --release -m crates/pybridge/Cargo.toml
```

## Documentation Site

This repository ships an MkDocs Material site under `docs/`. It is automatically deployed to GitHub Pages on every push to `main`.

Run locally:

```bash
pip install -r docs/requirements.txt
mkdocs serve
```

Build static output:

```bash
mkdocs build
```
