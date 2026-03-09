# Troubleshooting

---

## Rust Extension Missing

**Symptoms:**

- Import errors mentioning `neleus_core`
- The CLI fails immediately on startup with an `ImportError`

**Fix:**

```bash
pip install maturin
cd python
maturin develop --release
pip install -e .
```

---

## Missing Python Dependencies

If imports such as `yaml`, `typer`, or `rich` fail, the Python package dependencies are not installed.

```bash
pip install -e python/
```

---

## `--live` Fails: No Private Key Found

**Symptom:**

```text
--live requires a private key. Set HYPERLIQUID_SIGNER_PRIVATE_KEY in .env or pass --private-key to neleus new/init.
```

**Fix:**

Either add the key to `.env` in your project directory:

```bash
HYPERLIQUID_SIGNER_PRIVATE_KEY=0xYourPrivateKey
```

Or export it in the shell before running:

```bash
export HYPERLIQUID_SIGNER_PRIVATE_KEY=0xYourPrivateKey
neleus run --mode once --live
```

Or re-scaffold with the flag:

```bash
neleus init --private-key 0xYourPrivateKey --account-address 0xYourAddress
```

---

## Order Rejected by Exchange

**Symptom:** A warning like:

```text
Order rejected by exchange: Order would cross existing order
```

**Possible causes:**

- Insufficient margin or balance
- The order price crosses an existing open order (post-only mode)
- Asset name is wrong (check `neleus market list`)
- Position size is below the minimum lot size

A single rejection does not stop the runtime — the strategy continues to the next bar.

---

## `load_asset_metadata` Failed on Startup

**Symptom:**

```text
PyRuntimeError: load_asset_metadata failed: ...
```

**Cause:** `HyperliquidTrader` connects to Hyperliquid at startup to load the asset → index map. This fails if:

- The network is unreachable
- The API endpoint changed
- `--testnet` is set but the private key is a mainnet key (or vice versa)

**Fix:** Verify connectivity and that `HYPERLIQUID_TESTNET` matches the network your key was created on.

---

## Strategy Is Not Discovered

**Symptom:** `neleus strategy list` is empty, or `neleus run` says no strategy was found.

**Causes and fixes:**

- The strategy file is not in `strategies/`
- The file name starts with `_` (skipped by discovery)
- There is no class that subclasses `Strategy` in the file
- You are not inside the project directory (no `neleus.toml` found in the tree)

Check discovery:

```bash
neleus info
neleus strategy list
```

---

## Daemon Stops After One Cycle

**Symptom:** `neleus run --mode daemon` exits after the first poll.

**Cause:** An unhandled exception in the strategy or the candle fetch. Check the terminal output for a traceback.

---

## Database Connection Failed

**Symptom:** `neleus db status` or `neleus db init` prints `Connection FAILED`.

**Checks:**

```bash
# Confirm the DSN is set
neleus db status

# Test the connection string directly
psql "postgresql://user:password@localhost:5432/neleus" -c "SELECT 1"
```

If the DSN is in `.env`, confirm the `.env` file is in the project root (same directory as `neleus.toml`).

---

## Hyperliquid Testnet Confusion

The CLI defaults to **mainnet**. Testnet is opt-in:

```bash
neleus run --mode once --testnet --live
neleus market analyze BTC-PERP --testnet
```

You can also set `testnet = true` in the `[hyperliquid]` section of `neleus.toml` to make it the project default, or set `HYPERLIQUID_TESTNET=true` in `.env`.

---

## Scanner Is Not Exchange-Wide

`neleus market scan` is intentionally bounded by `--max-markets` so the terminal stays responsive.

To scan more precisely:

```bash
# Target a list of specific symbols
neleus market scan --symbols BTC,ETH,SOL,ARB

# Filter by name first
neleus market scan --search "BTC" --max-markets 5

# Limit to a specific scope
neleus market scan --scope perps --max-markets 30
```

---

## `.env` Is Not Being Loaded

The `.env` file is loaded by `load_project_config`, which is called whenever the runtime resolves project settings. It must be in the **project root** — the same directory that contains `neleus.toml`.

If you are running from a subdirectory, the `.env` will not be found. Run `neleus` commands from the project root.
