use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use serde::{Deserialize, Serialize};
use tokio_postgres::NoTls;
use tracing::info;

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS hl_orders (
    time          TIMESTAMPTZ          NOT NULL,
    cloid         TEXT                 NOT NULL,
    order_id      BIGINT,
    coin          TEXT                 NOT NULL,
    side          TEXT                 NOT NULL,
    order_type    TEXT                 NOT NULL,
    size          DOUBLE PRECISION     NOT NULL,
    price         DOUBLE PRECISION,
    reduce_only   BOOLEAN              NOT NULL DEFAULT FALSE,
    status        TEXT                 NOT NULL,
    filled_size   DOUBLE PRECISION     NOT NULL DEFAULT 0,
    avg_fill_price DOUBLE PRECISION,
    is_testnet    BOOLEAN              NOT NULL DEFAULT FALSE,
    PRIMARY KEY (time, cloid)
);

CREATE INDEX IF NOT EXISTS idx_hl_orders_coin   ON hl_orders (coin, time DESC);
CREATE INDEX IF NOT EXISTS idx_hl_orders_status ON hl_orders (status);
CREATE INDEX IF NOT EXISTS idx_hl_orders_cloid  ON hl_orders (cloid);

CREATE TABLE IF NOT EXISTS hl_fills (
    time       TIMESTAMPTZ      NOT NULL,
    order_id   BIGINT           NOT NULL,
    coin       TEXT             NOT NULL,
    side       TEXT             NOT NULL,
    price      DOUBLE PRECISION NOT NULL,
    size       DOUBLE PRECISION NOT NULL,
    fee        DOUBLE PRECISION NOT NULL,
    cloid      TEXT,
    is_testnet BOOLEAN          NOT NULL DEFAULT FALSE,
    PRIMARY KEY (time, order_id, coin)
);

CREATE INDEX IF NOT EXISTS idx_hl_fills_coin  ON hl_fills (coin, time DESC);
CREATE INDEX IF NOT EXISTS idx_hl_fills_cloid ON hl_fills (cloid);
"#;

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRecord {
    pub time: DateTime<Utc>,
    pub cloid: String,
    pub order_id: Option<i64>,
    pub coin: String,
    /// "buy" or "sell"
    pub side: String,
    /// "market", "limit", "post_only"
    pub order_type: String,
    pub size: f64,
    pub price: Option<f64>,
    pub reduce_only: bool,
    /// "submitted" | "open" | "filled" | "canceled" | "rejected"
    pub status: String,
    pub filled_size: f64,
    pub avg_fill_price: Option<f64>,
    pub is_testnet: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillRecord {
    pub time: DateTime<Utc>,
    pub order_id: i64,
    pub coin: String,
    /// "buy" or "sell"
    pub side: String,
    pub price: f64,
    pub size: f64,
    pub fee: f64,
    pub cloid: Option<String>,
    pub is_testnet: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PnlSummary {
    pub coin: String,
    /// Total notional value of buy fills
    pub buy_notional: f64,
    /// Total notional value of sell fills
    pub sell_notional: f64,
    /// Net realized PnL (sell - buy notional, ignoring fees)
    pub realized_pnl: f64,
    /// Total fees paid
    pub total_fee: f64,
    /// Net PnL after fees
    pub net_pnl: f64,
}

// ---------------------------------------------------------------------------
// TradeMonitor
// ---------------------------------------------------------------------------

pub struct TradeMonitor {
    pool: Pool,
}

impl TradeMonitor {
    pub async fn new(connection_string: &str, pool_size: usize) -> Result<Self> {
        let pg_config = connection_string
            .parse::<tokio_postgres::Config>()
            .context("invalid connection string")?;

        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };
        let mgr = Manager::from_config(pg_config, NoTls, mgr_config);
        let pool = Pool::builder(mgr)
            .max_size(pool_size)
            .build()
            .context("failed to create trade monitor connection pool")?;

        let client = pool.get().await.context("failed to connect to database")?;
        client
            .batch_execute(SCHEMA)
            .await
            .context("failed to initialize trade monitor schema")?;

        info!("Trade monitor schema initialized");

        Ok(Self { pool })
    }

    // -----------------------------------------------------------------------
    // Write operations
    // -----------------------------------------------------------------------

    /// Insert or update an order record. On conflict the status, fill info,
    /// and order_id are refreshed.
    pub async fn record_order(&self, order: &OrderRecord) -> Result<()> {
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO hl_orders \
                 (time, cloid, order_id, coin, side, order_type, size, price, \
                  reduce_only, status, filled_size, avg_fill_price, is_testnet) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13) \
                 ON CONFLICT (time, cloid) DO UPDATE \
                 SET order_id       = EXCLUDED.order_id, \
                     status         = EXCLUDED.status, \
                     filled_size    = EXCLUDED.filled_size, \
                     avg_fill_price = EXCLUDED.avg_fill_price",
                &[
                    &order.time,
                    &order.cloid,
                    &order.order_id,
                    &order.coin,
                    &order.side,
                    &order.order_type,
                    &order.size,
                    &order.price,
                    &order.reduce_only,
                    &order.status,
                    &order.filled_size,
                    &order.avg_fill_price,
                    &order.is_testnet,
                ],
            )
            .await
            .context("failed to record order")?;
        Ok(())
    }

    /// Update the status and fill progress of an existing order identified by
    /// its client order id (`cloid`).
    pub async fn update_order_status(
        &self,
        cloid: &str,
        status: &str,
        order_id: Option<i64>,
        filled_size: f64,
        avg_fill_price: Option<f64>,
    ) -> Result<u64> {
        let client = self.pool.get().await?;
        let rows = client
            .execute(
                "UPDATE hl_orders \
                 SET status         = $1, \
                     order_id       = COALESCE($2, order_id), \
                     filled_size    = $3, \
                     avg_fill_price = $4 \
                 WHERE cloid = $5",
                &[&status, &order_id, &filled_size, &avg_fill_price, &cloid],
            )
            .await
            .context("failed to update order status")?;
        Ok(rows)
    }

    /// Insert a fill record. Duplicate fills (same time + order_id + coin) are
    /// silently ignored.
    pub async fn record_fill(&self, fill: &FillRecord) -> Result<()> {
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO hl_fills \
                 (time, order_id, coin, side, price, size, fee, cloid, is_testnet) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) \
                 ON CONFLICT (time, order_id, coin) DO NOTHING",
                &[
                    &fill.time,
                    &fill.order_id,
                    &fill.coin,
                    &fill.side,
                    &fill.price,
                    &fill.size,
                    &fill.fee,
                    &fill.cloid,
                    &fill.is_testnet,
                ],
            )
            .await
            .context("failed to record fill")?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Query operations
    // -----------------------------------------------------------------------

    /// Return orders currently in "submitted" or "open" state.
    pub async fn get_open_orders(&self, is_testnet: bool) -> Result<Vec<OrderRecord>> {
        let client = self.pool.get().await?;
        let rows = client
            .query(
                "SELECT time, cloid, order_id, coin, side, order_type, size, price, \
                        reduce_only, status, filled_size, avg_fill_price, is_testnet \
                 FROM hl_orders \
                 WHERE status IN ('submitted', 'open') AND is_testnet = $1 \
                 ORDER BY time DESC",
                &[&is_testnet],
            )
            .await?;
        Ok(rows.iter().map(row_to_order).collect())
    }

    /// Return recent orders, optionally filtered by coin.
    pub async fn get_orders(
        &self,
        coin: Option<&str>,
        limit: i64,
        is_testnet: bool,
    ) -> Result<Vec<OrderRecord>> {
        let client = self.pool.get().await?;
        let rows = match coin {
            Some(c) => {
                client
                    .query(
                        "SELECT time, cloid, order_id, coin, side, order_type, size, price, \
                                reduce_only, status, filled_size, avg_fill_price, is_testnet \
                         FROM hl_orders \
                         WHERE coin = $1 AND is_testnet = $2 \
                         ORDER BY time DESC LIMIT $3",
                        &[&c, &is_testnet, &limit],
                    )
                    .await?
            }
            None => {
                client
                    .query(
                        "SELECT time, cloid, order_id, coin, side, order_type, size, price, \
                                reduce_only, status, filled_size, avg_fill_price, is_testnet \
                         FROM hl_orders \
                         WHERE is_testnet = $1 \
                         ORDER BY time DESC LIMIT $2",
                        &[&is_testnet, &limit],
                    )
                    .await?
            }
        };
        Ok(rows.iter().map(row_to_order).collect())
    }

    /// Return recent fills, optionally filtered by coin.
    pub async fn get_fills(
        &self,
        coin: Option<&str>,
        limit: i64,
        is_testnet: bool,
    ) -> Result<Vec<FillRecord>> {
        let client = self.pool.get().await?;
        let rows = match coin {
            Some(c) => {
                client
                    .query(
                        "SELECT time, order_id, coin, side, price, size, fee, cloid, is_testnet \
                         FROM hl_fills \
                         WHERE coin = $1 AND is_testnet = $2 \
                         ORDER BY time DESC LIMIT $3",
                        &[&c, &is_testnet, &limit],
                    )
                    .await?
            }
            None => {
                client
                    .query(
                        "SELECT time, order_id, coin, side, price, size, fee, cloid, is_testnet \
                         FROM hl_fills \
                         WHERE is_testnet = $1 \
                         ORDER BY time DESC LIMIT $2",
                        &[&is_testnet, &limit],
                    )
                    .await?
            }
        };
        Ok(rows.iter().map(row_to_fill).collect())
    }

    /// Aggregate realized PnL per coin from recorded fills.
    /// PnL is calculated as sell_notional - buy_notional (net of fees).
    pub async fn get_pnl_summary(
        &self,
        coin: Option<&str>,
        is_testnet: bool,
    ) -> Result<Vec<PnlSummary>> {
        let client = self.pool.get().await?;
        let rows = match coin {
            Some(c) => {
                client
                    .query(
                        "SELECT coin, \
                                SUM(CASE WHEN side='buy'  THEN price*size ELSE 0 END) AS buy_n, \
                                SUM(CASE WHEN side='sell' THEN price*size ELSE 0 END) AS sell_n, \
                                SUM(fee) AS total_fee \
                         FROM hl_fills \
                         WHERE coin = $1 AND is_testnet = $2 \
                         GROUP BY coin",
                        &[&c, &is_testnet],
                    )
                    .await?
            }
            None => {
                client
                    .query(
                        "SELECT coin, \
                                SUM(CASE WHEN side='buy'  THEN price*size ELSE 0 END) AS buy_n, \
                                SUM(CASE WHEN side='sell' THEN price*size ELSE 0 END) AS sell_n, \
                                SUM(fee) AS total_fee \
                         FROM hl_fills \
                         WHERE is_testnet = $1 \
                         GROUP BY coin \
                         ORDER BY coin",
                        &[&is_testnet],
                    )
                    .await?
            }
        };

        Ok(rows
            .iter()
            .map(|r| {
                let coin: String = r.get(0);
                let buy_n: f64 = r.get::<_, Option<f64>>(1).unwrap_or(0.0);
                let sell_n: f64 = r.get::<_, Option<f64>>(2).unwrap_or(0.0);
                let fee: f64 = r.get::<_, Option<f64>>(3).unwrap_or(0.0);
                let realized = sell_n - buy_n;
                PnlSummary {
                    coin,
                    buy_notional: buy_n,
                    sell_notional: sell_n,
                    realized_pnl: realized,
                    total_fee: fee,
                    net_pnl: realized - fee,
                }
            })
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Row helpers
// ---------------------------------------------------------------------------

fn row_to_order(row: &tokio_postgres::Row) -> OrderRecord {
    OrderRecord {
        time: row.get(0),
        cloid: row.get(1),
        order_id: row.get(2),
        coin: row.get(3),
        side: row.get(4),
        order_type: row.get(5),
        size: row.get(6),
        price: row.get(7),
        reduce_only: row.get(8),
        status: row.get(9),
        filled_size: row.get(10),
        avg_fill_price: row.get(11),
        is_testnet: row.get(12),
    }
}

fn row_to_fill(row: &tokio_postgres::Row) -> FillRecord {
    FillRecord {
        time: row.get(0),
        order_id: row.get(1),
        coin: row.get(2),
        side: row.get(3),
        price: row.get(4),
        size: row.get(5),
        fee: row.get(6),
        cloid: row.get(7),
        is_testnet: row.get(8),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pnl_summary_fields() {
        let s = PnlSummary {
            coin: "BTC".into(),
            buy_notional: 100_000.0,
            sell_notional: 101_000.0,
            realized_pnl: 1_000.0,
            total_fee: 20.0,
            net_pnl: 980.0,
        };
        assert_eq!(s.net_pnl, s.realized_pnl - s.total_fee);
    }
}
