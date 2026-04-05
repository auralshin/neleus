# Neleus — Claude Code Guide

## Project overview

Neleus is a Hyperliquid-first trading toolkit: a Rust workspace whose compiled extension is exposed to Python via PyO3/maturin, plus a Python CLI (`neleus`) and library.

```
crates/                     Rust workspace
  core-types/               Domain primitives (Venue, InstrumentId, Order, Fill, …)
  core-bus/                 Internal async message bus
  core-domain/              Strategy engine domain logic
  core-engine/              Live engine runtime
  backtest/                 Backtesting engine
  adapters-hyperliquid/     REST + WebSocket client, signing, execution
  adapters-lighter/         Lighter exchange adapter
  adapters-polymarket/      Polymarket adapter
  persistence/              PostgreSQL / TimescaleDB storage
  monitoring/               axum dashboard + Prometheus metrics
  pybridge/                 PyO3 bindings → neleus_core extension module
  agent-*/                  AI agent components (orchestrator, memory, comm, core, monitor)
  signal-hub/               Signal aggregation

python/                     Python package ("neleus")
  neleus/
    neleus_core.*.so        Compiled Rust extension (do NOT edit manually)
    __init__.py             Public Python API re-exports
    types.py                Python-side type aliases (imports from neleus_core)
    market.py               Market analysis, list_markets, scan_markets
    backtest_runner.py      BacktestRunner wrapper
    runtime.py              run_project_once / run_project_daemon
    strategy.py             Strategy / Actor base classes
    node.py                 HyperliquidBacktestNode
    config/                 Project config loading, credential discovery
    cli/
      main.py               typer CLI entry point
      repl.py               Interactive REPL
      ui.py                 Rich rendering helpers
  pyproject.toml            Python package + maturin config
  .venv/                    Dev virtual environment (python/.venv)
```

## Build commands

### Type-check Rust (fast, no link)
```bash
cargo check -p neleus                    # pybridge crate (package name is "neleus")
cargo check -p neleus-adapters-hyperliquid
cargo check --workspace
```

### Build Python extension (required after any Rust change)
```bash
# Always use the project venv — NOT the root .venv
VIRTUAL_ENV=$PWD/python/.venv \
  python/.venv/bin/maturin develop --release -m crates/pybridge/Cargo.toml
```

### Apple Silicon code-signing fix
After the maturin build, if Python crashes with `SIGKILL (Code Signature Invalid)`:
```bash
codesign -s - --force python/neleus/neleus_core.cpython-313-darwin.so
```
This happens when the `.so` is copied rather than compiled in place. Always re-sign after a copy.

### Run tests
```bash
# Rust unit tests (pybridge unit tests cannot link without Python symbols — skip them)
cargo test -p neleus-adapters-hyperliquid
cargo test -p neleus-core-types

# Python / integration tests
python/.venv/bin/python -m pytest python
```

### Release wheel
```bash
cd python && ../python/.venv/bin/maturin build --profile packaging --strip
```

## Two virtual environments — important

| Path | Purpose |
|---|---|
| `python/.venv/` | **Dev venv** — use this for all dev work |
| `.venv/` (root) | Secondary venv; maturin may default to it |

Always run Python with `python/.venv/bin/python` and maturin with `python/.venv/bin/maturin`. Mixing venvs causes stale `.so` imports.

## Architecture decisions

- **All market data and execution logic lives in Rust.** Python is a thin binding layer — computation belongs in the crates, not in `python/neleus/`.
- **`crates/pybridge/src/adapters.rs`** is the single file that translates Rust types to `#[pyclass]` wrappers. All new Rust-exposed types go here.
- **`crates/pybridge/src/lib.rs`** registers every `#[pyclass]` with `m.add_class::<Py…>()?`. A class not registered here will not be importable from Python.
- **`python/neleus/types.py`** re-exports `neleus_core` symbols under clean Python names. When a new pyclass is added to `lib.rs`, add a matching alias here and in `__init__.py`.
- **Editable install:** maturin places the compiled `.so` directly into `python/neleus/`. The file tracked in git is a placeholder — always rebuild after cloning.

## Hyperliquid adapter specifics

### Config
```rust
HyperliquidConfig::mainnet()   // api.hyperliquid.xyz
HyperliquidConfig::testnet()   // api.hyperliquid-testnet.xyz
```

### Asset ID encoding (from API docs)
| Market type | Asset ID formula |
|---|---|
| Perps | index in `meta` response |
| HIP-3 builder perps | `100000 + dex_index * 10000 + index_in_meta` |
| Spot | `10000 + spotMeta.universe[index]` |
| HIP-4 outcomes | `100_000_000 + (10 * outcome_id + side)` |

Outcome coin: `#<encoding>`, token name: `+<encoding>`. Only side 0 and 1 are valid.

### HIP-4 (outcome markets)
- `outcomeMeta` endpoint is **testnet-only**. Calling it on mainnet returns nothing useful.
- `fetch_outcome_meta()` is on `HyperliquidHistoricalClient` / `HyperliquidClient(testnet=True)`.
- `list_markets(scope="hip4", testnet=True)` — scope aliases: `hip4`, `hip-4`, `outcome`, `outcomes`.
- Scope aliases live in `python/neleus/market.py::SCOPE_ALIASES`.

## Market scopes

| Scope | Aliases | Notes |
|---|---|---|
| `perps` | `perp`, `perpetual` | Validator-operated perps only |
| `all-perps` | `all-perp`, `all_perps` | All perps across all DEXs |
| `hip3` | `hip-3` | Builder-deployed perps only |
| `spot` | `spots` | Spot pairs |
| `hip4` | `hip-4`, `outcome`, `outcomes` | Testnet-only outcome markets |

## Database (optional)

Configured in `neleus.toml` under `[database]`. Three backends:

| Backend | Use case |
|---|---|
| `none` | Default, no persistence |
| `postgres` | PostgreSQL event store + trade monitoring (`hl_orders`, `hl_fills`) |
| `timescale` | TimescaleDB hypertables for market data + trade monitoring |

Schema is auto-initialized on first connection (`TradeMonitor` constructor + `TimescaleStore` constructor).

## Common mistakes to avoid

- **Do not edit `neleus_core.*.so` directly.** It is a compiled binary.
- **Do not add `#[pyclass]` in Rust without registering it in `lib.rs`.** The class will silently not exist in Python.
- **Do not run `cargo test -p neleus`** — the pybridge crate cannot link standalone (needs Python symbols). Use `cargo check` for type-checking.
- **Do not mix `.venv` and `python/.venv`.** Use `python/.venv` for everything.
- **After copying a `.so` on Apple Silicon**, always `codesign -s - --force <path>` before importing.
- **HIP-4 scope requires `testnet=True`**. Calling it without will raise `ValueError`.

## CLI entry points

```bash
python/.venv/bin/neleus --help
python/.venv/bin/neleus market list --scope perps
python/.venv/bin/neleus market list --scope hip4 --testnet
python/.venv/bin/neleus market analyze BTC --testnet
python/.venv/bin/neleus market book BTC
python/.venv/bin/neleus market scan --scope perps
python/.venv/bin/neleus new my_bot
python/.venv/bin/neleus run --mode daemon
python/.venv/bin/neleus db init
```
