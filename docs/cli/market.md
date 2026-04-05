# Market Workflows

The market command group is the main no-project surface of Neleus. All market data is fetched via the Rust Hyperliquid adapter — no credentials required.

## Market Scopes

Every market command accepts a `--scope` flag:

| Scope | Aliases | Description |
| --- | --- | --- |
| `perps` | `perp`, `perpetual` | Validator-operated perpetual markets |
| `all-perps` | `all-perp`, `all_perps` | All perpetuals across all DEXs |
| `hip3` | `hip-3` | Builder-deployed HIP-3 perpetuals |
| `spot` | `spots` | Spot pairs |
| `hip4` | `hip-4`, `outcome`, `outcomes` | Binary outcome markets (testnet only) |

---

## Search Markets

Use search when you know part of a market name but not the exact symbol.

```bash
neleus market search BTC
neleus market search TSLA --scope hip3
neleus market search GAS --scope all-perps
neleus market search PURR --scope spot
```

Useful flags:

- `--scope perps|hip3|all-perps|spot|hip4`
- `--dex xyz` (HIP-3 only)
- `--output json`

---

## List Market Catalogs

Use list when you want counts, grouping, and a full market inventory.

```bash
neleus market list --scope perps
neleus market list --scope hip3
neleus market list --scope hip3 --dex xyz
neleus market list --scope all-perps
neleus market list --scope spot
```

### HIP-4 Outcome Markets

HIP-4 markets are binary outcome contracts and are **testnet-only**. Always pass `--testnet`:

```bash
neleus market list --scope hip4 --testnet
```

Each outcome market has two sides (0 = No, 1 = Yes). The coin encoding is `#<10 * outcome_id + side>` and the asset ID is `100_000_000 + encoding`.

---

## Analyze A Single Market

```bash
neleus market analyze BTC-PERP
neleus market analyze GAS --scope hip3 --dex flx
neleus market analyze TSLA --scope hip3 --dex xyz
neleus market analyze ETH-PERP --timeframe 15m --lookback-bars 300
neleus market analyze BTC-PERP --output json
```

For HIP-3 markets, use the plain asset name plus `--scope hip3 --dex <dex>` — Neleus resolves the routed market id internally.

The analysis view includes:

- last price and percentage change over the analyzed window
- volatility estimate
- RSI
- SMA and EMA structure
- Bollinger bands
- support and resistance levels
- directional bias and a short trading read

---

## Run A TA Scan

Use the scanner when you want ranked terminal output across a set of markets.

```bash
neleus market scan --scope perps
neleus market scan --scope hip3 --dex xyz --search TSLA
neleus market scan --symbols BTC-PERP,ETH-PERP,SOL-PERP --sort score
```

Flags:

- `--scope perps|hip3|all-perps|spot`
- `--symbols BTC-PERP,ETH-PERP,...`
- `--search <text>`
- `--max-markets <n>` (default 8)
- `--limit <n>`
- `--sort score|change|volatility|rsi`
- `--timeframe 1m|5m|15m|1h|4h|1d`

The scanner is bounded by design — use `--symbols` or `--search` to target specific markets.

---

## Watch A Live Order Book

```bash
neleus market book BTC-PERP
neleus market book GAS --scope hip3 --dex flx
neleus market book flx:GAS-PERP
neleus market book HYPE-PERP --depth 15
```

The live view is backed by the Rust Hyperliquid WebSocket path and shows:

- live bids and asks
- best bid and ask
- spread and spread in basis points
- total displayed depth
- order book imbalance

For HIP-3 markets, Neleus resolves the internal routed symbol automatically and shows it in the terminal footer.

Press `Ctrl+C` to stop streaming.

---

## Hyperliquid API Model

Current market workflows use Hyperliquid's public market-data surface:

- `POST /info` for market metadata and candles
- WebSocket subscriptions for live market streams
- `outcomeMeta` for HIP-4 (testnet endpoint only)

These flows do not require API keys.
