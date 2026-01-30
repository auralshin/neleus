# Neleus Features (Current Codebase)

This list is derived from the Rust and Python sources in this repo.

## Rust Core: Types and Domain Models
- Venues: Hyperliquid, Lighter, Polymarket, Simulated; instrument types: perp and spot.
- Typed IDs for orders, trades, strategies, positions, accounts; sequence numbers.
- Fixed-point math types for price, quantity, money, and currency.
- Order types: market, limit, stop market, stop limit, take profit, trailing stop.
- Time-in-force values: GTC, IOC, FOK, GTD, DAY.
- Market data models: trade ticks, quote ticks (BBO), order book snapshots/deltas, bars.
- Trading domain models: orders, fills, positions, account state/balance, order commands/events.

## Messaging and Engine
- In-memory message bus with topics, priority queues, subscriptions, correlation IDs.
- Event sink support for persistence (bus message logging).
- Strategy engine with lifecycle callbacks, subscriptions, and timers.
- Order Management System with client/venue ID mapping and state tracking.
- Position engine with realized/unrealized PnL and open position tracking.
- Telemetry hooks with engine snapshots and bus stats.

## Risk, Capital, and Portfolio
- Capital management (initial capital, unit spend, profit locking, redeploy, reserves).
- Position management (max open positions, per-instrument/venue limits, holding periods, pyramiding).
- Spread trading config (pairs, hedge ratios, rebalance thresholds).
- Leverage and margin config (maintenance/initial margin, funding interval, margin calls).
- Risk manager: position/notional limits, concentration, daily/unrealized loss, drawdown windows,
  correlation group limits, liquidity limits, order-rate limits, kill switch.
- Position sizing methods: fixed, fixed-notional, percent equity, Kelly, risk-based, volatility-based.
- Stop loss types: fixed, percent, ATR, trailing, time-based, chandelier, parabolic SAR.
- Take profit types: fixed, percent, risk-reward, ATR, Fibonacci, partial, trailing.
- Portfolio aggregation with strategy performance metrics and netting/attribution models.

## Advanced Risk Analytics
- VaR (historical, parametric, Monte Carlo) with component and marginal VaR.
- CVaR (expected shortfall).
- Scenario/stress testing (flash crash, correction, liquidity crisis, volatility spike, black swan).
- Greeks calculator and aggregation.
- Correlation-based sizing.
- Dynamic risk limits based on volatility regime and drawdown.

## Execution Algorithms
- TWAP, VWAP (volume profile), Iceberg (refresh strategies), Adaptive execution (mode switching).
- Market condition model for adaptive execution decisions.
- Execution state tracking and performance metrics.

## Backtesting and Simulation (Rust)
- Simulation modes: bar-based, trade-based, order book-based.
- Fill models: immediate, next tick, probabilistic, order book.
- Slippage models: zero, fixed bps, volume impact, spread-based, L2 simulation.
- Latency models: zero, fixed (optional jitter), uniform, log-normal.
- Simulated order book and venue for fills and slippage simulation.
- Data feeds: in-memory, CSV, JSONL, Hyperliquid candles; multi-feed merging.
- Backtest results: PnL, return, drawdown, Sharpe/Sortino/Calmar, win rate, profit factor, equity curve.
- Walk-forward analysis and parameter sweeps with sensitivity analysis.
- Backtest execution algo manager includes TWAP, VWAP, Iceberg, and POV.

## Persistence and Replay
- Postgres event log sink with batch writes.
- TimescaleDB schema for ticks, trades, order book snapshots, quotes, funding rates, indicators.
- TimescaleDB aggregates (1m/5m/15m/1h) and compression policies.
- Execution tracking tables for TWAP/VWAP/Iceberg/Adaptive runs.
- Historical replay engine with speed control and progress tracking.

## Venue Adapters
- Hyperliquid: REST/WS, signed requests, nonce manager, rate limits, reconnect backoff,
  subscriptions (mids, L2 book, trades, candles, user fills/orders/funding/notifications),
  execution and data adapters.
- Lighter: REST/WS, account tier weighted rate limits, orderbook/trades/orders/fills/account
  subscriptions, execution and data adapters.
- Polymarket: L1/L2 auth, REST endpoints (price, midpoint, book, trades, markets, orders, positions),
  WebSocket subscriptions (book/price/user updates), execution and data adapters.

## Python API and Tooling
- PyO3 bridge exposing core types, backtest config/results, risk config, execution algos,
  engine, LiveNode, and Timescale/Postgres stores.
- Strategy base class with lifecycle and data callbacks; Actor for background tasks.
- Hyperliquid historical client in Python bindings (candle and metadata fetch).
- BacktestRunner for YAML/TOML project configs and dynamic strategy loading.
- Config loader/validator with .env overrides and dataclass schemas.
- CLI for project scaffolding, strategy templates, backtests, UI launch, and project info
  (live command is currently a scaffold).

## UI, Visualization, and Monitoring
- FastAPI dashboard server with strategy editor endpoints and WebSocket broadcast; demo
  portfolio/risk/performance data.
- Lightweight static dashboard server with basic config/strategy/backtest endpoints.
- Backtest plotting and HTML report generation (equity, drawdown, trade scatter, PnL distribution,
  rolling Sharpe) plus a live plotter buffer.
- Axum-based monitoring dashboard with engine/bus snapshots and log buffering.

## Examples and Tests
- Python examples: momentum, mean reversion, latency benchmark, Timescale replay, advanced features.
- Rust integration tests for adapters and persistence.

## Managed Service (Agent Platform)

Neleus is not just a backtesting framework—it's a fully managed service for always-on trading bots.

### Agent Orchestrator (CI/CD for Trading Bots)
- Agent lifecycle management: deploy, start, stop, pause, resume, terminate.
- State persistence with automatic restoration after restarts.
- Health checking with liveness/readiness probes and configurable thresholds.
- Auto-restart for failed agents with exponential backoff.
- Rolling upgrades with zero-downtime and rollback capability.
- Cron-based scheduling for market hours operation.
- Agent states: created, initializing, ready, running, paused, stopping, stopped, error, upgrading.

### Signal Hub (External AI/Quant Integration)
- HTTP receiver for REST API signal ingestion from any source.
- Webhook support for TradingView, custom systems, and third-party services.
- Signal validation with schema checking, rate limiting, and deduplication.
- Signal routing to specific agents based on subscriptions.
- Signal transformation and normalization from heterogeneous sources.
- Historical signal storage and query API.
- Signal types: entry, exit, scale_in, scale_out, rebalance, risk_alert.

### Agent Monitor (Continuous Monitoring)
- Real-time metrics collection: P&L, positions, orders, fills, risk, system health.
- Configurable alert rules with severity levels (info, warning, critical).
- Notification channels: Slack, webhooks, email.
- Dashboard REST API for building custom monitoring UIs.
- Event logging with comprehensive history.
- Heartbeat monitoring with timeout detection.

### Deployment Configuration
- Environment presets: local development, Docker, Kubernetes.
- Secret management with environment, Vault, and AWS Secrets Manager backends.
- Future TEE (Trusted Execution Environment) support for strategy privacy.
- YAML/JSON configuration with schema validation.
