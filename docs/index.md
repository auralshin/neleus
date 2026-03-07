# Neleus

<div class="hero">
  <img src="assets/logo.png" alt="Neleus logo" />
  <h1>Neleus</h1>
  <p><em>Trade Hyperliquid from the terminal.</em></p>
  <p>
    Neleus is a Hyperliquid-first CLI and Python toolkit powered by a Rust core.
    Use it to search markets, run technical scans, stream live order books,
    backtest strategies, and scaffold Python trading projects.
  </p>
  <div class="hero-links">
    <a href="getting-started/installation/">Get Started</a>
    <a href="cli/market/">Market Workflows</a>
    <a href="https://neleus.trade">Website</a>
    <a href="https://github.com/auralshin/neleus">GitHub</a>
  </div>
</div>

<div class="feature-grid">
  <div class="feature-card">
    <h3>No-project market tools</h3>
    <p>Search, list, analyze, scan, and monitor Hyperliquid markets directly from the CLI.</p>
  </div>
  <div class="feature-card">
    <h3>Rust-backed market core</h3>
    <p>REST and WebSocket paths are driven by the Rust Hyperliquid adapter, then exposed to Python and the CLI.</p>
  </div>
  <div class="feature-card">
    <h3>Python strategy workflow</h3>
    <p>Scaffold a project, write strategies in Python, backtest them, and run them once or as a daemon.</p>
  </div>
  <div class="feature-card">
    <h3>Terminal-first UX</h3>
    <p>Rich dashboards, ranked scans, live order book views, and concise command-driven workflows.</p>
  </div>
</div>

## What You Can Do Today

| Workflow | Command | Result |
| --- | --- | --- |
| Search markets | `neleus market search BTC` | Find matching spot, default perp, or HIP-3 markets |
| List market catalogs | `neleus market list --scope hip3 --dex xyz` | View market groups with counts and metadata |
| Analyze a market | `neleus market analyze BTC-PERP` | Get RSI, trend, levels, volatility, and a directional read |
| Scan setups | `neleus market scan --scope perps` | Rank a bounded market set by conviction-style TA score |
| Stream live depth | `neleus market book BTC-PERP` | Watch a live Hyperliquid L2 order book in the terminal |
| Scaffold a project | `neleus new my_strategy_project` | Generate a Python strategy project wired to the Rust core |

## Product Scope

Implemented now:
- market search, list, analysis, scanning, and live order book monitoring
- project scaffolding for Python strategies
- backtesting
- one-shot and daemon strategy runtimes
- database-backed runtime order/fill monitoring through `TradeMonitor`
- database schema inspection and initialization through `neleus db status` and `neleus db init`

Not implemented yet:
- a dedicated `neleus trade` command separate from the current runtime/core APIs
- broader live operations tooling beyond the current trade-monitoring path

## Suggested Reading Order

1. Start with [Installation](getting-started/installation.md)
2. Run the no-project commands in [Quickstart](getting-started/quickstart.md)
3. Learn the full market command surface in [Market Workflows](cli/market.md)
4. Scaffold a project in [Strategy Projects](projects.md)
5. Learn the actual strategy API in [Writing Strategies](strategy-writing.md)
6. Copy and adapt code from [Strategy Examples](strategy-examples.md)
