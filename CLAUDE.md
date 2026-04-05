# Neleus — Claude Code Guide

## Design principle

**All computation belongs in Rust. Python is the CLI and interactive surface only.**

- Market data, execution logic, backtesting, strategy runtime, order management — all in Rust crates.
- Python code in `python/neleus/` exists solely to expose the Rust extension to the terminal CLI and to interactive user sessions. No heavy logic, no data processing, no algorithms in Python.
- Maximise speed: keep the hot path in Rust and call it from Python via PyO3 bindings.

---

## Project layout

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

python/                     Python package ("neleus") — CLI + thin wrappers only
  neleus/
    neleus_core.*.so        Compiled Rust extension (do NOT edit manually)
    __init__.py             Public Python API re-exports
    types.py                Python-side type aliases (imports from neleus_core)
    market.py               Thin wrapper: list_markets, scan_markets, analyze_market
    backtest_runner.py      BacktestRunner wrapper
    runtime.py              run_project_once / run_project_daemon
    strategy.py             Strategy / Actor base classes
    node.py                 HyperliquidBacktestNode
    config/                 Project config loading, credential discovery
    cli/
      main.py               typer CLI entry point
      repl.py               Interactive REPL (TTY-only)
      ui.py                 Rich rendering helpers
  pyproject.toml            Python package + maturin config
```

---

## Virtual environment

**Always use the root `.venv`.** There is one canonical dev environment:

```
.venv/          Root venv — use this for all dev and CLI work
```

```bash
# Activate once per shell session
source .venv/bin/activate

# Or prefix every command
.venv/bin/python
.venv/bin/maturin
.venv/bin/neleus
```

Do not use `python/.venv`. It is a legacy artefact and may be removed.

---

## Build commands

### Type-check Rust (fast, no link)
```bash
cargo check -p neleus                      # pybridge crate
cargo check -p neleus-adapters-hyperliquid
cargo check --workspace
```

### Build Python extension (required after any Rust change)
```bash
VIRTUAL_ENV=$PWD/.venv \
  .venv/bin/maturin develop --release -m crates/pybridge/Cargo.toml
```

Then install the Python package in editable mode (first time or after pyproject.toml changes):
```bash
.venv/bin/pip install -e python/
```

### Apple Silicon code-signing fix
After the maturin build, if Python crashes with `SIGKILL (Code Signature Invalid)`:
```bash
codesign -s - --force python/neleus/neleus_core.cpython-313-darwin.so
```
This happens when the `.so` is copied rather than compiled in place. Always re-sign after a copy.

### Run tests
```bash
# Rust unit tests (do NOT run cargo test -p neleus — pybridge cannot link standalone)
cargo test -p neleus-adapters-hyperliquid
cargo test -p neleus-core-types

# Python / integration tests
.venv/bin/python -m pytest python/
```

### Release wheel
```bash
cd python && ../.venv/bin/maturin build --profile packaging --strip
```

---

## Adding a new Rust type to Python

1. Implement the domain logic in the appropriate crate (e.g. `adapters-hyperliquid`).
2. Add a `#[pyclass]` wrapper in **`crates/pybridge/src/adapters.rs`** — this is the single translation layer.
3. Register it in **`crates/pybridge/src/lib.rs`** with `m.add_class::<PyFoo>()?`. Without this the class is silently absent.
4. Add a type alias in **`python/neleus/types.py`** (both the `TYPE_CHECKING` stub and the `try` import blocks).
5. Re-export from **`python/neleus/__init__.py`** (import line + `__all__` entry).
6. Rebuild: `VIRTUAL_ENV=$PWD/.venv .venv/bin/maturin develop --release -m crates/pybridge/Cargo.toml`

---

## Hyperliquid adapter specifics

### Config
```rust
HyperliquidConfig::mainnet()   // api.hyperliquid.xyz
HyperliquidConfig::testnet()   // api.hyperliquid-testnet.xyz
```

### Asset ID encoding
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

---

## Market scopes

| Scope | Aliases | Notes |
|---|---|---|
| `perps` | `perp`, `perpetual` | Validator-operated perps only |
| `all-perps` | `all-perp`, `all_perps` | All perps across all DEXs |
| `hip3` | `hip-3` | Builder-deployed perps only |
| `spot` | `spots` | Spot pairs |
| `hip4` | `hip-4`, `outcome`, `outcomes` | Testnet-only outcome markets |

---

## Database (optional)

Configured in `neleus.toml` under `[database]`. Three backends:

| Backend | Use case |
|---|---|
| `none` | Default, no persistence |
| `postgres` | PostgreSQL event store + trade monitoring (`hl_orders`, `hl_fills`) |
| `timescale` | TimescaleDB hypertables for market data + trade monitoring |

Schema is auto-initialised on first connection (`TradeMonitor` constructor + `TimescaleStore` constructor).

---

## Common mistakes to avoid

- **Do not put computation in Python.** If it's more than a format call or a thin dispatch, it belongs in a Rust crate.
- **Do not edit `neleus_core.*.so` directly.** It is a compiled binary.
- **Do not add `#[pyclass]` in Rust without registering it in `lib.rs`.** The class will silently not exist in Python.
- **Do not run `cargo test -p neleus`** — the pybridge crate cannot link standalone (needs Python symbols). Use `cargo check` for type-checking.
- **Do not use `python/.venv`.** Always use the root `.venv`.
- **After copying a `.so` on Apple Silicon**, always `codesign -s - --force <path>` before importing.
- **HIP-4 scope requires `testnet=True`**. Calling without it raises `ValueError`.

---

## CLI entry points

```bash
.venv/bin/neleus --help
.venv/bin/neleus market list --scope perps
.venv/bin/neleus market list --scope hip4 --testnet
.venv/bin/neleus market analyze BTC
.venv/bin/neleus market book BTC
.venv/bin/neleus market scan --scope perps
.venv/bin/neleus new my_bot
.venv/bin/neleus run --mode daemon
.venv/bin/neleus db init
```

With the venv activated (`source .venv/bin/activate`) the `neleus` prefix is enough:

```bash
neleus market list --scope perps
neleus market list --scope hip4 --testnet
```
