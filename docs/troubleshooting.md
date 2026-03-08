# Troubleshooting

## Rust Extension Missing

Symptoms:

- import errors mentioning `neleus_core`
- the CLI fails immediately on startup

Fix:

```bash
pip install maturin
cd python
maturin develop --release
pip install -e .
```

## Missing Python Dependencies

If you run the package without installing the Python dependencies, imports such as `yaml` can fail.

Install the package dependencies:

```bash
pip install -e python/
```

## Hyperliquid Testnet Confusion

The CLI defaults to Hyperliquid mainnet.

Use `--testnet` only when you explicitly want the testnet market surface.

## Scanner Is Not Exchange-Wide

`market scan` is intentionally bounded with `--max-markets` so the CLI remains fast and useful.

If you need a narrower focus:

- use `--search`
- use `--symbols`
- use `--scope` and `--dex`
