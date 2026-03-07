# Market Workflows

The market command group is the main no-project surface of Neleus.

## Search Markets

Use search when you know part of a market name but not the exact symbol.

```bash
neleus market search BTC
neleus market search TSLA --scope hip3
neleus market search PURR --scope spot
```

Useful flags:

- `--scope perps|hip3|all-perps|spot`
- `--dex xyz`
- `--output json`

## List Market Catalogs

Use list when you want counts, grouping, and a full market inventory.

```bash
neleus market list --scope perps
neleus market list --scope hip3
neleus market list --scope hip3 --dex xyz
neleus market list --scope all-perps
neleus market list --scope spot
```

## Analyze A Single Market

```bash
neleus market analyze BTC-PERP
neleus market analyze ETH-PERP --timeframe 15m --lookback-bars 300
neleus market analyze BTC-PERP --output json
```

The analysis view includes:

- last price
- percentage change over the analyzed window
- volatility estimate
- RSI
- SMA and EMA structure
- Bollinger bands
- support and resistance
- directional bias and a short trading read

## Run A TA Scan

Use the scanner when you want ranked terminal output instead of one-off analysis.

```bash
neleus market scan --scope perps
neleus market scan --scope hip3 --dex xyz --search TSLA
neleus market scan --symbols BTC-PERP,ETH-PERP,SOL-PERP --sort score
```

Important flags:

- `--scope perps|hip3|all-perps|spot`
- `--symbols BTC-PERP,ETH-PERP,...`
- `--search <text>`
- `--max-markets <n>`
- `--limit <n>`
- `--sort score|change|volatility|rsi`
- `--timeframe 1m|5m|15m|1h|4h|1d`

The scanner is intentionally bounded. It is designed for fast terminal triage, not exhaustive exchange-wide screening in one command.

## Watch A Live Order Book

```bash
neleus market book BTC-PERP
neleus market book HYPE-PERP --depth 15
```

The live view is backed by the Rust Hyperliquid WebSocket path and shows:

- live bids and asks
- best bid and ask
- spread and spread in basis points
- total displayed depth
- order book imbalance

Press `Ctrl+C` to stop streaming.

## Hyperliquid API Model

Current Neleus market workflows use Hyperliquid's public market-data surface:

- `POST /info` for market metadata and candles
- WebSocket subscriptions for live market streams

These flows do not require API keys.
