use anyhow::{Context, Result};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use serde::{Deserialize, Serialize};
use tokio_postgres::NoTls;
use tracing::{info, warn};

/// TimescaleDB schema for price time series data
const TIMESCALE_SCHEMA: &str = r#"
-- Enable TimescaleDB extension
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- Market data ticks (OHLCV)
CREATE TABLE IF NOT EXISTS market_ticks (
    time TIMESTAMPTZ NOT NULL,
    venue TEXT NOT NULL,
    symbol TEXT NOT NULL,
    instrument_type TEXT NOT NULL,
    open DOUBLE PRECISION NOT NULL,
    high DOUBLE PRECISION NOT NULL,
    low DOUBLE PRECISION NOT NULL,
    close DOUBLE PRECISION NOT NULL,
    volume DOUBLE PRECISION NOT NULL,
    trade_count INTEGER,
    vwap DOUBLE PRECISION,
    PRIMARY KEY (time, venue, symbol, instrument_type)
);

-- Convert to hypertable (time-series optimized)
SELECT create_hypertable('market_ticks', 'time', if_not_exists => TRUE);

-- Create indexes for fast lookups
CREATE INDEX IF NOT EXISTS idx_market_ticks_venue_symbol 
    ON market_ticks (venue, symbol, time DESC);
CREATE INDEX IF NOT EXISTS idx_market_ticks_symbol 
    ON market_ticks (symbol, time DESC);

-- Tick-by-tick trade data
CREATE TABLE IF NOT EXISTS trades (
    time TIMESTAMPTZ NOT NULL,
    venue TEXT NOT NULL,
    symbol TEXT NOT NULL,
    instrument_type TEXT NOT NULL,
    trade_id TEXT,
    side TEXT NOT NULL,
    price DOUBLE PRECISION NOT NULL,
    size DOUBLE PRECISION NOT NULL,
    is_buyer_maker BOOLEAN,
    PRIMARY KEY (time, venue, symbol, trade_id)
);

SELECT create_hypertable('trades', 'time', if_not_exists => TRUE);

CREATE INDEX IF NOT EXISTS idx_trades_venue_symbol 
    ON trades (venue, symbol, time DESC);

-- Order book snapshots
CREATE TABLE IF NOT EXISTS order_book_snapshots (
    time TIMESTAMPTZ NOT NULL,
    venue TEXT NOT NULL,
    symbol TEXT NOT NULL,
    instrument_type TEXT NOT NULL,
    bids JSONB NOT NULL,
    asks JSONB NOT NULL,
    sequence_number BIGINT,
    PRIMARY KEY (time, venue, symbol)
);

SELECT create_hypertable('order_book_snapshots', 'time', if_not_exists => TRUE);

CREATE INDEX IF NOT EXISTS idx_orderbook_venue_symbol 
    ON order_book_snapshots (venue, symbol, time DESC);

-- Quote updates (BBO)
CREATE TABLE IF NOT EXISTS quotes (
    time TIMESTAMPTZ NOT NULL,
    venue TEXT NOT NULL,
    symbol TEXT NOT NULL,
    instrument_type TEXT NOT NULL,
    bid_price DOUBLE PRECISION NOT NULL,
    bid_size DOUBLE PRECISION NOT NULL,
    ask_price DOUBLE PRECISION NOT NULL,
    ask_size DOUBLE PRECISION NOT NULL,
    PRIMARY KEY (time, venue, symbol)
);

SELECT create_hypertable('quotes', 'time', if_not_exists => TRUE);

CREATE INDEX IF NOT EXISTS idx_quotes_venue_symbol 
    ON quotes (venue, symbol, time DESC);

-- Funding rates (for perpetual futures)
CREATE TABLE IF NOT EXISTS funding_rates (
    time TIMESTAMPTZ NOT NULL,
    venue TEXT NOT NULL,
    symbol TEXT NOT NULL,
    rate DOUBLE PRECISION NOT NULL,
    next_funding_time TIMESTAMPTZ,
    PRIMARY KEY (time, venue, symbol)
);

SELECT create_hypertable('funding_rates', 'time', if_not_exists => TRUE);

CREATE INDEX IF NOT EXISTS idx_funding_venue_symbol 
    ON funding_rates (venue, symbol, time DESC);

-- Indicators (ATR, Bollinger Bands, etc.)
CREATE TABLE IF NOT EXISTS indicators (
    time TIMESTAMPTZ NOT NULL,
    venue TEXT NOT NULL,
    symbol TEXT NOT NULL,
    indicator_name TEXT NOT NULL,
    period INTEGER NOT NULL,
    value DOUBLE PRECISION NOT NULL,
    metadata JSONB,
    PRIMARY KEY (time, venue, symbol, indicator_name, period)
);

SELECT create_hypertable('indicators', 'time', if_not_exists => TRUE);

CREATE INDEX IF NOT EXISTS idx_indicators_venue_symbol_name 
    ON indicators (venue, symbol, indicator_name, time DESC);

-- Continuous aggregates for common queries
-- 1-minute OHLCV from trades
CREATE MATERIALIZED VIEW IF NOT EXISTS trades_1m
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 minute', time) AS bucket,
    venue,
    symbol,
    instrument_type,
    FIRST(price, time) AS open,
    MAX(price) AS high,
    MIN(price) AS low,
    LAST(price, time) AS close,
    SUM(size) AS volume,
    COUNT(*) AS trade_count,
    SUM(price * size) / NULLIF(SUM(size), 0) AS vwap
FROM trades
GROUP BY bucket, venue, symbol, instrument_type;

-- 5-minute OHLCV
CREATE MATERIALIZED VIEW IF NOT EXISTS trades_5m
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('5 minutes', time) AS bucket,
    venue,
    symbol,
    instrument_type,
    FIRST(price, time) AS open,
    MAX(price) AS high,
    MIN(price) AS low,
    LAST(price, time) AS close,
    SUM(size) AS volume,
    COUNT(*) AS trade_count,
    SUM(price * size) / NULLIF(SUM(size), 0) AS vwap
FROM trades
GROUP BY bucket, venue, symbol, instrument_type;

-- 15-minute OHLCV
CREATE MATERIALIZED VIEW IF NOT EXISTS trades_15m
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('15 minutes', time) AS bucket,
    venue,
    symbol,
    instrument_type,
    FIRST(price, time) AS open,
    MAX(price) AS high,
    MIN(price) AS low,
    LAST(price, time) AS close,
    SUM(size) AS volume,
    COUNT(*) AS trade_count,
    SUM(price * size) / NULLIF(SUM(size), 0) AS vwap
FROM trades
GROUP BY bucket, venue, symbol, instrument_type;

-- 1-hour OHLCV
CREATE MATERIALIZED VIEW IF NOT EXISTS trades_1h
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', time) AS bucket,
    venue,
    symbol,
    instrument_type,
    FIRST(price, time) AS open,
    MAX(price) AS high,
    MIN(price) AS low,
    LAST(price, time) AS close,
    SUM(size) AS volume,
    COUNT(*) AS trade_count,
    SUM(price * size) / NULLIF(SUM(size), 0) AS vwap
FROM trades
GROUP BY bucket, venue, symbol, instrument_type;

-- Compression policies (compress data older than 7 days)
SELECT add_compression_policy('market_ticks', INTERVAL '7 days', if_not_exists => TRUE);
SELECT add_compression_policy('trades', INTERVAL '7 days', if_not_exists => TRUE);
SELECT add_compression_policy('order_book_snapshots', INTERVAL '7 days', if_not_exists => TRUE);
SELECT add_compression_policy('quotes', INTERVAL '7 days', if_not_exists => TRUE);
SELECT add_compression_policy('funding_rates', INTERVAL '30 days', if_not_exists => TRUE);
SELECT add_compression_policy('indicators', INTERVAL '30 days', if_not_exists => TRUE);

-- Retention policies (optional - delete data older than X days)
-- Uncomment to enable:
-- SELECT add_retention_policy('market_ticks', INTERVAL '365 days', if_not_exists => TRUE);
-- SELECT add_retention_policy('trades', INTERVAL '90 days', if_not_exists => TRUE);

-- =============================================================================
-- Execution Tracking Tables
-- =============================================================================

-- TWAP/VWAP/Iceberg/Adaptive execution tracking
CREATE TABLE IF NOT EXISTS executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    time TIMESTAMPTZ NOT NULL,
    execution_id TEXT NOT NULL,
    algo_type TEXT NOT NULL,  -- 'twap', 'vwap', 'iceberg', 'adaptive'
    venue TEXT NOT NULL,
    symbol TEXT NOT NULL,
    side TEXT NOT NULL,
    state TEXT NOT NULL,  -- 'pending', 'active', 'completed', 'cancelled', 'failed'
    total_quantity DOUBLE PRECISION NOT NULL,
    filled_quantity DOUBLE PRECISION NOT NULL,
    remaining_quantity DOUBLE PRECISION NOT NULL,
    avg_price DOUBLE PRECISION,
    arrival_price DOUBLE PRECISION,
    implementation_shortfall DOUBLE PRECISION,
    slippage_bps DOUBLE PRECISION,
    algo_params JSONB,
    start_time TIMESTAMPTZ,
    end_time TIMESTAMPTZ,
    metadata JSONB
);

SELECT create_hypertable('executions', 'time', if_not_exists => TRUE);

CREATE INDEX IF NOT EXISTS idx_executions_id ON executions (execution_id, time DESC);
CREATE INDEX IF NOT EXISTS idx_executions_symbol ON executions (venue, symbol, time DESC);
CREATE INDEX IF NOT EXISTS idx_executions_algo ON executions (algo_type, time DESC);

-- Execution slices (individual child orders)
CREATE TABLE IF NOT EXISTS execution_slices (
    time TIMESTAMPTZ NOT NULL,
    execution_id TEXT NOT NULL,
    slice_index INTEGER NOT NULL,
    order_id TEXT,
    target_quantity DOUBLE PRECISION NOT NULL,
    filled_quantity DOUBLE PRECISION NOT NULL,
    target_price DOUBLE PRECISION,
    fill_price DOUBLE PRECISION,
    state TEXT NOT NULL,
    market_conditions JSONB,
    PRIMARY KEY (time, execution_id, slice_index)
);

SELECT create_hypertable('execution_slices', 'time', if_not_exists => TRUE);

CREATE INDEX IF NOT EXISTS idx_execution_slices_id 
    ON execution_slices (execution_id, time DESC);

-- =============================================================================
-- Portfolio Management Tables
-- =============================================================================

-- Strategy performance tracking
CREATE TABLE IF NOT EXISTS strategy_performance (
    time TIMESTAMPTZ NOT NULL,
    strategy_id TEXT NOT NULL,
    total_pnl DOUBLE PRECISION NOT NULL,
    realized_pnl DOUBLE PRECISION NOT NULL,
    unrealized_pnl DOUBLE PRECISION NOT NULL,
    total_trades BIGINT NOT NULL,
    winning_trades BIGINT NOT NULL,
    win_rate DOUBLE PRECISION NOT NULL,
    sharpe_ratio DOUBLE PRECISION,
    max_drawdown_pct DOUBLE PRECISION NOT NULL,
    current_drawdown_pct DOUBLE PRECISION NOT NULL,
    profit_factor DOUBLE PRECISION,
    avg_trade_pnl DOUBLE PRECISION,
    capital_allocated DOUBLE PRECISION NOT NULL,
    return_on_capital DOUBLE PRECISION,
    state TEXT NOT NULL,  -- 'active', 'paused', 'disabled', 'liquidating', 'error'
    PRIMARY KEY (time, strategy_id)
);

SELECT create_hypertable('strategy_performance', 'time', if_not_exists => TRUE);

CREATE INDEX IF NOT EXISTS idx_strategy_performance_id 
    ON strategy_performance (strategy_id, time DESC);

-- Portfolio positions (aggregated across strategies)
CREATE TABLE IF NOT EXISTS portfolio_positions (
    time TIMESTAMPTZ NOT NULL,
    venue TEXT NOT NULL,
    symbol TEXT NOT NULL,
    net_quantity DOUBLE PRECISION NOT NULL,
    avg_entry_price DOUBLE PRECISION NOT NULL,
    market_value DOUBLE PRECISION NOT NULL,
    unrealized_pnl DOUBLE PRECISION NOT NULL,
    realized_pnl DOUBLE PRECISION NOT NULL,
    leverage DOUBLE PRECISION,
    margin_used DOUBLE PRECISION,
    strategy_breakdown JSONB,  -- {strategy_id: quantity}
    PRIMARY KEY (time, venue, symbol)
);

SELECT create_hypertable('portfolio_positions', 'time', if_not_exists => TRUE);

CREATE INDEX IF NOT EXISTS idx_portfolio_positions_symbol 
    ON portfolio_positions (venue, symbol, time DESC);

-- Capital allocations
CREATE TABLE IF NOT EXISTS capital_allocations (
    time TIMESTAMPTZ NOT NULL,
    strategy_id TEXT NOT NULL,
    allocation_method TEXT NOT NULL,  -- 'equal', 'risk_parity', 'performance', 'kelly', 'fixed'
    capital_allocated DOUBLE PRECISION NOT NULL,
    allocation_pct DOUBLE PRECISION NOT NULL,
    target_allocation_pct DOUBLE PRECISION,
    rebalance_needed BOOLEAN DEFAULT FALSE,
    metadata JSONB,
    PRIMARY KEY (time, strategy_id)
);

SELECT create_hypertable('capital_allocations', 'time', if_not_exists => TRUE);

CREATE INDEX IF NOT EXISTS idx_capital_allocations_strategy 
    ON capital_allocations (strategy_id, time DESC);

-- Netting results
CREATE TABLE IF NOT EXISTS netting_results (
    time TIMESTAMPTZ NOT NULL,
    venue TEXT NOT NULL,
    symbol TEXT NOT NULL,
    gross_long DOUBLE PRECISION NOT NULL,
    gross_short DOUBLE PRECISION NOT NULL,
    net_position DOUBLE PRECISION NOT NULL,
    netting_efficiency DOUBLE PRECISION NOT NULL,
    capital_saved DOUBLE PRECISION NOT NULL,
    strategy_contributions JSONB,
    PRIMARY KEY (time, venue, symbol)
);

SELECT create_hypertable('netting_results', 'time', if_not_exists => TRUE);

-- =============================================================================
-- Risk Analytics Tables
-- =============================================================================

-- VAR/CVaR results
CREATE TABLE IF NOT EXISTS risk_var (
    time TIMESTAMPTZ NOT NULL,
    var_method TEXT NOT NULL,  -- 'historical', 'parametric', 'monte_carlo'
    confidence_level DOUBLE PRECISION NOT NULL,
    holding_period_days INTEGER NOT NULL,
    var_value DOUBLE PRECISION NOT NULL,
    var_pct DOUBLE PRECISION NOT NULL,
    cvar_value DOUBLE PRECISION,
    cvar_pct DOUBLE PRECISION,
    component_var JSONB,  -- {symbol: var_value}
    marginal_var JSONB,   -- {symbol: marginal_var}
    portfolio_value DOUBLE PRECISION,
    PRIMARY KEY (time, var_method, confidence_level)
);

SELECT create_hypertable('risk_var', 'time', if_not_exists => TRUE);

CREATE INDEX IF NOT EXISTS idx_risk_var_method 
    ON risk_var (var_method, time DESC);

-- Stress test results
CREATE TABLE IF NOT EXISTS stress_tests (
    time TIMESTAMPTZ NOT NULL,
    scenario TEXT NOT NULL,  -- 'flash_crash', 'market_correction', 'liquidity_crisis', etc.
    description TEXT,
    price_shock DOUBLE PRECISION,
    volatility_multiplier DOUBLE PRECISION,
    spread_widening_bps DOUBLE PRECISION,
    liquidity_reduction DOUBLE PRECISION,
    portfolio_pnl DOUBLE PRECISION NOT NULL,
    portfolio_pnl_pct DOUBLE PRECISION NOT NULL,
    position_impacts JSONB,  -- [{symbol, position, impact}]
    margin_call BOOLEAN DEFAULT FALSE,
    liquidation BOOLEAN DEFAULT FALSE,
    PRIMARY KEY (time, scenario)
);

SELECT create_hypertable('stress_tests', 'time', if_not_exists => TRUE);

CREATE INDEX IF NOT EXISTS idx_stress_tests_scenario 
    ON stress_tests (scenario, time DESC);

-- Greeks tracking (for derivatives/perps)
CREATE TABLE IF NOT EXISTS portfolio_greeks (
    time TIMESTAMPTZ NOT NULL,
    venue TEXT NOT NULL,
    symbol TEXT NOT NULL,
    delta DOUBLE PRECISION NOT NULL,
    gamma DOUBLE PRECISION NOT NULL,
    vega DOUBLE PRECISION NOT NULL,
    theta DOUBLE PRECISION NOT NULL,
    rho DOUBLE PRECISION NOT NULL,
    position_size DOUBLE PRECISION,
    notional_value DOUBLE PRECISION,
    PRIMARY KEY (time, venue, symbol)
);

SELECT create_hypertable('portfolio_greeks', 'time', if_not_exists => TRUE);

CREATE INDEX IF NOT EXISTS idx_portfolio_greeks_symbol 
    ON portfolio_greeks (venue, symbol, time DESC);

-- Dynamic risk limits
CREATE TABLE IF NOT EXISTS risk_limits (
    time TIMESTAMPTZ NOT NULL,
    volatility_regime TEXT NOT NULL,  -- 'low', 'normal', 'high', 'extreme'
    current_volatility DOUBLE PRECISION NOT NULL,
    position_limit DOUBLE PRECISION NOT NULL,
    daily_loss_limit DOUBLE PRECISION NOT NULL,
    leverage_limit DOUBLE PRECISION NOT NULL,
    drawdown_factor DOUBLE PRECISION NOT NULL,
    trading_allowed BOOLEAN NOT NULL,
    adjustment_reason TEXT,
    PRIMARY KEY (time)
);

SELECT create_hypertable('risk_limits', 'time', if_not_exists => TRUE);

-- Comprehensive risk reports
CREATE TABLE IF NOT EXISTS risk_reports (
    time TIMESTAMPTZ NOT NULL,
    var_95 DOUBLE PRECISION NOT NULL,
    var_99 DOUBLE PRECISION NOT NULL,
    cvar_95 DOUBLE PRECISION,
    volatility_regime TEXT NOT NULL,
    current_volatility DOUBLE PRECISION NOT NULL,
    trading_allowed BOOLEAN NOT NULL,
    position_limit DOUBLE PRECISION,
    daily_loss_limit DOUBLE PRECISION,
    leverage_limit DOUBLE PRECISION,
    stress_test_summary JSONB,  -- [{scenario, pnl_pct}]
    alerts JSONB,
    PRIMARY KEY (time)
);

SELECT create_hypertable('risk_reports', 'time', if_not_exists => TRUE);

-- Compression for new tables
SELECT add_compression_policy('executions', INTERVAL '7 days', if_not_exists => TRUE);
SELECT add_compression_policy('execution_slices', INTERVAL '7 days', if_not_exists => TRUE);
SELECT add_compression_policy('strategy_performance', INTERVAL '30 days', if_not_exists => TRUE);
SELECT add_compression_policy('portfolio_positions', INTERVAL '7 days', if_not_exists => TRUE);
SELECT add_compression_policy('capital_allocations', INTERVAL '30 days', if_not_exists => TRUE);
SELECT add_compression_policy('netting_results', INTERVAL '7 days', if_not_exists => TRUE);
SELECT add_compression_policy('risk_var', INTERVAL '30 days', if_not_exists => TRUE);
SELECT add_compression_policy('stress_tests', INTERVAL '30 days', if_not_exists => TRUE);
SELECT add_compression_policy('portfolio_greeks', INTERVAL '7 days', if_not_exists => TRUE);
SELECT add_compression_policy('risk_limits', INTERVAL '30 days', if_not_exists => TRUE);
SELECT add_compression_policy('risk_reports', INTERVAL '30 days', if_not_exists => TRUE);
"#;

#[derive(Debug, Clone)]
pub struct TimescaleConfig {
    pub connection_string: String,
    pub pool_size: usize,
    pub batch_size: usize,
    pub flush_interval_ms: u64,
}

impl Default for TimescaleConfig {
    fn default() -> Self {
        Self {
            connection_string: "postgresql://postgres:postgres@localhost:5432/neleus_timeseries"
                .to_string(),
            pool_size: 8,
            batch_size: 5000,
            flush_interval_ms: 100,
        }
    }
}

/// OHLCV candle data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub time: chrono::DateTime<chrono::Utc>,
    pub venue: String,
    pub symbol: String,
    pub instrument_type: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub trade_count: Option<i32>,
    pub vwap: Option<f64>,
}

/// Trade tick data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub time: chrono::DateTime<chrono::Utc>,
    pub venue: String,
    pub symbol: String,
    pub instrument_type: String,
    pub trade_id: Option<String>,
    pub side: String,
    pub price: f64,
    pub size: f64,
    pub is_buyer_maker: Option<bool>,
}

/// Quote (BBO) data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub time: chrono::DateTime<chrono::Utc>,
    pub venue: String,
    pub symbol: String,
    pub instrument_type: String,
    pub bid_price: f64,
    pub bid_size: f64,
    pub ask_price: f64,
    pub ask_size: f64,
}

/// Order book snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookSnapshot {
    pub time: chrono::DateTime<chrono::Utc>,
    pub venue: String,
    pub symbol: String,
    pub instrument_type: String,
    pub bids: Vec<(f64, f64)>, // (price, size)
    pub asks: Vec<(f64, f64)>,
    pub sequence_number: Option<i64>,
}

/// Funding rate data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingRate {
    pub time: chrono::DateTime<chrono::Utc>,
    pub venue: String,
    pub symbol: String,
    pub rate: f64,
    pub next_funding_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// Technical indicator data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Indicator {
    pub time: chrono::DateTime<chrono::Utc>,
    pub venue: String,
    pub symbol: String,
    pub indicator_name: String,
    pub period: i32,
    pub value: f64,
    pub metadata: Option<serde_json::Value>,
}

pub struct TimescaleStore {
    pool: Pool,
}

impl TimescaleStore {
    pub async fn new(config: TimescaleConfig) -> Result<Self> {
        let pg_config = config.connection_string.parse::<tokio_postgres::Config>()?;

        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };
        let mgr = Manager::from_config(pg_config, NoTls, mgr_config);
        let pool = Pool::builder(mgr)
            .max_size(config.pool_size)
            .build()
            .context("failed to create TimescaleDB connection pool")?;

        let client = pool.get().await?;

        // Initialize schema
        info!("Initializing TimescaleDB schema...");
        match client.batch_execute(TIMESCALE_SCHEMA).await {
            Ok(_) => info!("TimescaleDB schema initialized successfully"),
            Err(e) => {
                warn!("Failed to initialize TimescaleDB schema: {}", e);
                warn!("Some features may not work if TimescaleDB extension is not installed");
            }
        }

        Ok(Self { pool })
    }

    // ========== Insert Methods ==========

    pub async fn insert_candle(&self, candle: &Candle) -> Result<()> {
        let client = self.pool.get().await?;

        client
            .execute(
                "INSERT INTO market_ticks \
                 (time, venue, symbol, instrument_type, open, high, low, close, volume, trade_count, vwap) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
                 ON CONFLICT (time, venue, symbol, instrument_type) DO UPDATE \
                 SET open = EXCLUDED.open, high = EXCLUDED.high, low = EXCLUDED.low, \
                     close = EXCLUDED.close, volume = EXCLUDED.volume, \
                     trade_count = EXCLUDED.trade_count, vwap = EXCLUDED.vwap",
                &[
                    &candle.time,
                    &candle.venue,
                    &candle.symbol,
                    &candle.instrument_type,
                    &candle.open,
                    &candle.high,
                    &candle.low,
                    &candle.close,
                    &candle.volume,
                    &candle.trade_count,
                    &candle.vwap,
                ],
            )
            .await?;

        Ok(())
    }

    pub async fn insert_candles(&self, candles: &[Candle]) -> Result<()> {
        if candles.is_empty() {
            return Ok(());
        }

        let mut client = self.pool.get().await?;
        let transaction = client.transaction().await?;

        let stmt = "INSERT INTO market_ticks \
                    (time, venue, symbol, instrument_type, open, high, low, close, volume, trade_count, vwap) \
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
                    ON CONFLICT (time, venue, symbol, instrument_type) DO NOTHING";

        for candle in candles {
            transaction
                .execute(
                    stmt,
                    &[
                        &candle.time,
                        &candle.venue,
                        &candle.symbol,
                        &candle.instrument_type,
                        &candle.open,
                        &candle.high,
                        &candle.low,
                        &candle.close,
                        &candle.volume,
                        &candle.trade_count,
                        &candle.vwap,
                    ],
                )
                .await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    pub async fn insert_trade(&self, trade: &Trade) -> Result<()> {
        let client = self.pool.get().await?;

        client
            .execute(
                "INSERT INTO trades \
                 (time, venue, symbol, instrument_type, trade_id, side, price, size, is_buyer_maker) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
                 ON CONFLICT (time, venue, symbol, trade_id) DO NOTHING",
                &[
                    &trade.time,
                    &trade.venue,
                    &trade.symbol,
                    &trade.instrument_type,
                    &trade.trade_id,
                    &trade.side,
                    &trade.price,
                    &trade.size,
                    &trade.is_buyer_maker,
                ],
            )
            .await?;

        Ok(())
    }

    pub async fn insert_trades(&self, trades: &[Trade]) -> Result<()> {
        if trades.is_empty() {
            return Ok(());
        }

        let mut client = self.pool.get().await?;
        let transaction = client.transaction().await?;

        let stmt = "INSERT INTO trades \
                    (time, venue, symbol, instrument_type, trade_id, side, price, size, is_buyer_maker) \
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
                    ON CONFLICT (time, venue, symbol, trade_id) DO NOTHING";

        for trade in trades {
            transaction
                .execute(
                    stmt,
                    &[
                        &trade.time,
                        &trade.venue,
                        &trade.symbol,
                        &trade.instrument_type,
                        &trade.trade_id,
                        &trade.side,
                        &trade.price,
                        &trade.size,
                        &trade.is_buyer_maker,
                    ],
                )
                .await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    pub async fn insert_quote(&self, quote: &Quote) -> Result<()> {
        let client = self.pool.get().await?;

        client
            .execute(
                "INSERT INTO quotes \
                 (time, venue, symbol, instrument_type, bid_price, bid_size, ask_price, ask_size) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
                 ON CONFLICT (time, venue, symbol) DO UPDATE \
                 SET bid_price = EXCLUDED.bid_price, bid_size = EXCLUDED.bid_size, \
                     ask_price = EXCLUDED.ask_price, ask_size = EXCLUDED.ask_size",
                &[
                    &quote.time,
                    &quote.venue,
                    &quote.symbol,
                    &quote.instrument_type,
                    &quote.bid_price,
                    &quote.bid_size,
                    &quote.ask_price,
                    &quote.ask_size,
                ],
            )
            .await?;

        Ok(())
    }

    pub async fn insert_funding_rate(&self, funding: &FundingRate) -> Result<()> {
        let client = self.pool.get().await?;

        client
            .execute(
                "INSERT INTO funding_rates (time, venue, symbol, rate, next_funding_time) \
                 VALUES ($1, $2, $3, $4, $5) \
                 ON CONFLICT (time, venue, symbol) DO UPDATE \
                 SET rate = EXCLUDED.rate, next_funding_time = EXCLUDED.next_funding_time",
                &[
                    &funding.time,
                    &funding.venue,
                    &funding.symbol,
                    &funding.rate,
                    &funding.next_funding_time,
                ],
            )
            .await?;

        Ok(())
    }

    pub async fn insert_indicator(&self, indicator: &Indicator) -> Result<()> {
        let client = self.pool.get().await?;

        client
            .execute(
                "INSERT INTO indicators \
                 (time, venue, symbol, indicator_name, period, value, metadata) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7) \
                 ON CONFLICT (time, venue, symbol, indicator_name, period) DO UPDATE \
                 SET value = EXCLUDED.value, metadata = EXCLUDED.metadata",
                &[
                    &indicator.time,
                    &indicator.venue,
                    &indicator.symbol,
                    &indicator.indicator_name,
                    &indicator.period,
                    &indicator.value,
                    &indicator.metadata,
                ],
            )
            .await?;

        Ok(())
    }

    // ========== Query Methods ==========

    pub async fn get_candles(
        &self,
        venue: &str,
        symbol: &str,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Candle>> {
        let client = self.pool.get().await?;

        let rows = client
            .query(
                "SELECT time, venue, symbol, instrument_type, open, high, low, close, volume, trade_count, vwap \
                 FROM market_ticks \
                 WHERE venue = $1 AND symbol = $2 AND time >= $3 AND time <= $4 \
                 ORDER BY time ASC",
                &[&venue, &symbol, &start_time, &end_time],
            )
            .await?;

        let candles = rows
            .iter()
            .map(|row| Candle {
                time: row.get(0),
                venue: row.get(1),
                symbol: row.get(2),
                instrument_type: row.get(3),
                open: row.get(4),
                high: row.get(5),
                low: row.get(6),
                close: row.get(7),
                volume: row.get(8),
                trade_count: row.get(9),
                vwap: row.get(10),
            })
            .collect();

        Ok(candles)
    }

    pub async fn get_trades(
        &self,
        venue: &str,
        symbol: &str,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
        limit: Option<i64>,
    ) -> Result<Vec<Trade>> {
        let client = self.pool.get().await?;

        let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();

        let query = format!(
            "SELECT time, venue, symbol, instrument_type, trade_id, side, price, size, is_buyer_maker \
             FROM trades \
             WHERE venue = $1 AND symbol = $2 AND time >= $3 AND time <= $4 \
             ORDER BY time ASC{}",
            limit_clause
        );

        let rows = client
            .query(&query, &[&venue, &symbol, &start_time, &end_time])
            .await?;

        let trades = rows
            .iter()
            .map(|row| Trade {
                time: row.get(0),
                venue: row.get(1),
                symbol: row.get(2),
                instrument_type: row.get(3),
                trade_id: row.get(4),
                side: row.get(5),
                price: row.get(6),
                size: row.get(7),
                is_buyer_maker: row.get(8),
            })
            .collect();

        Ok(trades)
    }

    pub async fn get_latest_quote(&self, venue: &str, symbol: &str) -> Result<Option<Quote>> {
        let client = self.pool.get().await?;

        let row = client
            .query_opt(
                "SELECT time, venue, symbol, instrument_type, bid_price, bid_size, ask_price, ask_size \
                 FROM quotes \
                 WHERE venue = $1 AND symbol = $2 \
                 ORDER BY time DESC LIMIT 1",
                &[&venue, &symbol],
            )
            .await?;

        Ok(row.map(|r| Quote {
            time: r.get(0),
            venue: r.get(1),
            symbol: r.get(2),
            instrument_type: r.get(3),
            bid_price: r.get(4),
            bid_size: r.get(5),
            ask_price: r.get(6),
            ask_size: r.get(7),
        }))
    }

    pub async fn get_time_range(
        &self,
        venue: &str,
        symbol: &str,
    ) -> Result<Option<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>> {
        let client = self.pool.get().await?;

        let row = client
            .query_one(
                "SELECT MIN(time), MAX(time) FROM market_ticks WHERE venue = $1 AND symbol = $2",
                &[&venue, &symbol],
            )
            .await?;

        let min: Option<chrono::DateTime<chrono::Utc>> = row.get(0);
        let max: Option<chrono::DateTime<chrono::Utc>> = row.get(1);

        match (min, max) {
            (Some(min), Some(max)) => Ok(Some((min, max))),
            _ => Ok(None),
        }
    }

    pub async fn get_available_symbols(&self, venue: &str) -> Result<Vec<String>> {
        let client = self.pool.get().await?;

        let rows = client
            .query(
                "SELECT DISTINCT symbol FROM market_ticks WHERE venue = $1 ORDER BY symbol",
                &[&venue],
            )
            .await?;

        Ok(rows.iter().map(|r| r.get(0)).collect())
    }

    // ========== Continuous Aggregates Queries ==========

    pub async fn get_candles_aggregated(
        &self,
        venue: &str,
        symbol: &str,
        interval: &str,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Candle>> {
        let client = self.pool.get().await?;

        let table = match interval {
            "1m" | "1min" => "trades_1m",
            "5m" | "5min" => "trades_5m",
            "15m" | "15min" => "trades_15m",
            "1h" | "1hour" => "trades_1h",
            _ => "market_ticks", // Fallback to raw data
        };

        let query = format!(
            "SELECT bucket as time, venue, symbol, instrument_type, open, high, low, close, volume, trade_count, vwap \
             FROM {} \
             WHERE venue = $1 AND symbol = $2 AND bucket >= $3 AND bucket <= $4 \
             ORDER BY bucket ASC",
            table
        );

        let rows = client
            .query(&query, &[&venue, &symbol, &start_time, &end_time])
            .await?;

        let candles = rows
            .iter()
            .map(|row| Candle {
                time: row.get(0),
                venue: row.get(1),
                symbol: row.get(2),
                instrument_type: row.get(3),
                open: row.get(4),
                high: row.get(5),
                low: row.get(6),
                close: row.get(7),
                volume: row.get(8),
                trade_count: row.get(9),
                vwap: row.get(10),
            })
            .collect();

        Ok(candles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timescale_config_defaults() {
        let config = TimescaleConfig::default();
        assert_eq!(config.pool_size, 8);
        assert_eq!(config.batch_size, 5000);
    }
}
