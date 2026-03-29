# CLAUDE.md — Neleus

Hyperliquid-first trading CLI and Python toolkit powered by a Rust core.

## Repository Layout

```
crates/                  # 18 Rust crates (Cargo workspace)
  core-types/            # Domain types: Venue, InstrumentId, OrderState, etc.
  core-bus/              # Event bus: Topics, MessageKind, Priority
  core-domain/           # Order/position models with state machines
  core-engine/           # Trading engine orchestrator
  adapters-hyperliquid/  # Hyperliquid REST + WebSocket client, signing, execution
  adapters-lighter/      # Lighter venue adapter
  adapters-polymarket/   # Polymarket venue adapter
  backtest/              # Historical backtesting engine
  monitoring/            # axum dashboard + Prometheus metrics
  persistence/           # PostgreSQL/TimescaleDB event stores
  signal-hub/            # Signal generation and distribution
  agent-*/               # Multi-strategy AI agent framework (orchestrator, core, comm, memory, monitor)
  pybridge/              # PyO3 C extension compiled as `neleus_core`
python/
  neleus/                # Python package wrapping the Rust extension
    __init__.py          # Exports + __version__
    strategy.py          # Strategy & Actor base classes
    types.py             # Rust bridge bindings + Python enums
    market.py            # Market API (search, analyze, scan, L2 book)
    runtime.py           # run_project_once / run_project_daemon
    backtest_runner.py   # Backtest orchestration
    node.py              # HyperliquidBacktestNode, CandleInterval
    cli/                 # Typer CLI application (main.py, ui.py)
    config/              # TOML config loading / strategy discovery
docs/                    # MkDocs source
.github/workflows/       # build-wheels.yml (CI), deploy-docs.yml
```

## Build Commands

### Rust

```bash
# Type-check the PyO3 bridge (fastest sanity check)
cargo check -p neleus-pybridge

# Type-check everything
cargo check --workspace

# Run Rust tests (many are #[ignore] — require live services)
cargo test --workspace

# Run a specific crate's tests
cargo test -p neleus-core-types
```

> `cargo test -p neleus-pybridge` fails at link — expected; PyO3 needs Python symbols at link time.

### Python extension (maturin)

```bash
python3 -m venv .venv && source .venv/bin/activate
pip install maturin

# Build and install the Rust extension in-place for development
maturin develop --release -m crates/pybridge/Cargo.toml

# Install the Python package in editable mode
pip install -e python/

# Build optimised wheel (uses the "packaging" profile)
maturin build --profile packaging --strip -m crates/pybridge/Cargo.toml
```

### Documentation

```bash
pip install -r docs/requirements.txt
mkdocs serve    # local preview
mkdocs build    # generate site/
```

## Cargo Profiles

| Profile | Purpose |
|---|---|
| `release` | Production — `opt-level=3`, `lto=fat`, `panic=abort` |
| `packaging` | PyPI wheels — `lto=thin`, 8 codegen-units (smaller, faster to build) |
| `profiling` | Release + debug info, no strip |
| `bench` | Release + `lto=thin` |

## Version Synchronisation

Version must be kept in sync across **three** files:

1. `Cargo.toml` (workspace `[package].version`)
2. `python/pyproject.toml` (`version = "..."`)
3. `python/neleus/__init__.py` (`__version__ = "..."`)

## Key Dependencies

| Crate | Purpose |
|---|---|
| `tokio` (full) | Async runtime |
| `serde` / `serde_json` | Serialization |
| `reqwest` (rustls-tls) | HTTP client |
| `tokio-tungstenite` | WebSocket |
| `axum` / `hyper` | Web framework / monitoring |
| `tokio-postgres` + `deadpool-postgres` | PostgreSQL async + pooling |
| `pyo3` v0.22 | Python bindings |
| `thiserror` / `anyhow` | Error handling |
| `tracing` / `tracing-subscriber` | Structured logging |
| `uuid` (v4 + serde) | ID generation |
| `chrono` | DateTime |

Python package runtime deps: `aiohttp`, `pyyaml`, `typer`, `rich`, `numpy`, `toml`.

## Code Conventions

### Rust

- **Naming**: types `PascalCase`, functions/variables `snake_case`, modules `snake_case`.
- **Errors**: `thiserror` for library errors (`#[derive(Error)]`), `anyhow::Result<T>` for fallible operations.
- **Serde**: `#[serde(rename_all = "snake_case")]` on most domain types.
- **Async**: `#[tokio::test]` for async tests; `async-trait` for trait impls.
- **Tests**: live/integration tests are `#[ignore]`; unit tests live in `#[cfg(test)]` modules.

### Python

- **Style**: `black` + `ruff`, line length 100, `from __future__ import annotations`.
- **Types**: full type hints; `TYPE_CHECKING` guards for forward refs.
- **Docstrings**: Google style — `Args`, `Returns`, `Raises`, `Example` sections.
- **Targets**: Python 3.10–3.13.

## Configuration

Projects use a `neleus.toml` file and a `.env` for secrets (never committed).

```toml
# neleus.toml skeleton
[hyperliquid]
testnet = true

[database]
dsn = "postgresql://..."

[backtest]
# ...
```

Runtime environment variables:

| Variable | Purpose |
|---|---|
| `NELEUS_DB_DSN` | PostgreSQL/TimescaleDB connection string |

## Testing Notes

- Most Hyperliquid and PostgreSQL integration tests are `#[ignore]`. Run them explicitly with `cargo test -- --ignored` when you have live credentials.
- `cargo test -p neleus-pybridge` is expected to fail at the link step — this is normal.
- Python tests use `pytest` with `pytest-asyncio`.

## CI/CD

- **build-wheels.yml**: builds and validates wheels for Linux (manylinux2014), macOS (x86_64 + aarch64), Windows across Python 3.10–3.13. Publishes to PyPI on release.
- **deploy-docs.yml**: deploys MkDocs to GitHub Pages on push to `main`.
- PR smoke test builds a single wheel; full matrix only runs on release/manual trigger.

## Persistence Schema

- **TimescaleDB schema**: `crates/persistence/src/timescale.rs`
- **Trade monitor** (plain PostgreSQL, no TimescaleDB): `crates/persistence/src/trade_monitor.rs` — tables `hl_orders`, `hl_fills`
- **Event log**: `crates/persistence/src/lib.rs`

## Monitoring

Axum server endpoints:

| Path | Description |
|---|---|
| `/` | Dashboard HTML |
| `/api/snapshot` | Current state snapshot |
| `/api/logs` | Log stream |
| `/metrics` | Prometheus metrics |
