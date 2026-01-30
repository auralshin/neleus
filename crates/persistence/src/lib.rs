use anyhow::{Context, Result};
use neleus_core_bus::{EventSink, Message};
use std::time::Duration;

use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use tokio::sync::mpsc as async_mpsc;
use tokio_postgres::{Error as PgError, NoTls};
use tracing::{error, info};

pub mod timescale;
pub use timescale::{
    Candle, FundingRate, Indicator, OrderBookSnapshot, Quote, TimescaleConfig, TimescaleStore,
    Trade,
};

pub mod replay;
pub use replay::{
    HistoricalReplayer, MarketEvent, ReplayConfig, ReplayProgress, ReplayStats,
};

const POSTGRES_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS event_log (
    id BIGSERIAL PRIMARY KEY,
    message_id BIGINT NOT NULL,
    kind TEXT NOT NULL,
    topic TEXT NOT NULL,
    priority INTEGER NOT NULL,
    timestamp BIGINT NOT NULL,
    correlation_id BIGINT,
    reply_to TEXT,
    payload BYTEA NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_event_log_timestamp ON event_log (timestamp);
CREATE INDEX IF NOT EXISTS idx_event_log_topic ON event_log (topic);
CREATE INDEX IF NOT EXISTS idx_event_log_created_at ON event_log (created_at);
"#;

#[derive(Debug, Clone)]
pub struct PostgresEventStoreConfig {
    pub connection_string: String,

    pub batch_size: usize,

    pub pool_size: usize,

    pub flush_interval_ms: u64,
}

impl Default for PostgresEventStoreConfig {
    fn default() -> Self {
        Self {
            connection_string: "postgresql://postgres:postgres@localhost:5432/neleus".to_string(),
            batch_size: 1000,
            pool_size: 4,
            flush_interval_ms: 100,
        }
    }
}

pub struct PostgresEventStore {
    sender: async_mpsc::UnboundedSender<Message>,
}

impl PostgresEventStore {
    pub async fn new(config: PostgresEventStoreConfig) -> Result<Self> {
        let pg_config = config.connection_string.parse::<tokio_postgres::Config>()?;

        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };
        let mgr = Manager::from_config(pg_config, NoTls, mgr_config);
        let pool = Pool::builder(mgr)
            .max_size(config.pool_size)
            .build()
            .context("failed to create connection pool")?;

        let client = pool.get().await?;
        client.batch_execute(POSTGRES_SCHEMA).await?;
        info!("PostgreSQL schema initialized");

        let (sender, receiver) = async_mpsc::unbounded_channel();

        tokio::spawn(run_postgres_worker(config, pool, receiver));

        Ok(Self { sender })
    }
}

impl EventSink for PostgresEventStore {
    fn append(&self, message: &Message) {
        let _ = self.sender.send(message.clone());
    }
}

async fn run_postgres_worker(
    config: PostgresEventStoreConfig,
    pool: Pool,
    mut receiver: async_mpsc::UnboundedReceiver<Message>,
) {
    let mut buffer = Vec::with_capacity(config.batch_size);
    let flush_interval = Duration::from_millis(config.flush_interval_ms);
    let mut last_flush = tokio::time::Instant::now();

    loop {
        tokio::select! {
            msg_opt = receiver.recv() => {
                match msg_opt {
                    Some(message) => {
                        buffer.push(message);


                        let should_flush = buffer.len() >= config.batch_size
                            || last_flush.elapsed() >= flush_interval;

                        if should_flush {
                            if let Err(e) = flush_postgres_messages(&pool, &mut buffer).await {
                                error!("Failed to flush PostgreSQL messages: {}", e);
                            }
                            last_flush = tokio::time::Instant::now();
                        }
                    }
                    None => {

                        if let Err(e) = flush_postgres_messages(&pool, &mut buffer).await {
                            error!("Failed to flush PostgreSQL messages on shutdown: {}", e);
                        }
                        break;
                    }
                }
            }
            _ = tokio::time::sleep_until(last_flush + flush_interval) => {

                if !buffer.is_empty() {
                    if let Err(e) = flush_postgres_messages(&pool, &mut buffer).await {
                        error!("Failed to flush PostgreSQL messages: {}", e);
                    }
                    last_flush = tokio::time::Instant::now();
                }
            }
        }
    }

    info!("PostgreSQL event store worker stopped");
}

async fn flush_postgres_messages(pool: &Pool, buffer: &mut Vec<Message>) -> Result<(), PgError> {
    if buffer.is_empty() {
        return Ok(());
    }

    let mut client = pool
        .get()
        .await
        .map_err(|_e| tokio_postgres::Error::__private_api_timeout())?;

    let stmt = "INSERT INTO event_log \
        (message_id, kind, topic, priority, timestamp, correlation_id, reply_to, payload) \
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)";

    let transaction = client.transaction().await?;

    for message in buffer.drain(..) {
        let correlation_id: Option<i64> = message.correlation_id.map(|v| v as i64);
        let reply_to: Option<&str> = message.reply_to.as_ref().map(|t| t.as_str());

        transaction
            .execute(
                stmt,
                &[
                    &(message.id as i64),
                    &message.kind.as_str(),
                    &message.topic.as_str(),
                    &(message.priority.as_u8() as i32),
                    &(message.timestamp as i64),
                    &correlation_id,
                    &reply_to,
                    &message.payload,
                ],
            )
            .await?;
    }

    transaction.commit().await?;

    Ok(())
}

use neleus_core_bus::{MessageKind, Priority, Topic};

#[derive(Debug, Clone, Default)]
pub struct ReplayQuery {
    pub start_timestamp: Option<u64>,

    pub end_timestamp: Option<u64>,

    pub topics: Vec<String>,

    pub kinds: Vec<MessageKind>,

    pub limit: Option<usize>,

    pub offset: Option<usize>,
}

impl ReplayQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_time_range(mut self, start: u64, end: u64) -> Self {
        self.start_timestamp = Some(start);
        self.end_timestamp = Some(end);
        self
    }

    pub fn with_topic(mut self, topic: impl Into<String>) -> Self {
        self.topics.push(topic.into());
        self
    }

    pub fn with_kind(mut self, kind: MessageKind) -> Self {
        self.kinds.push(kind);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

pub struct PostgresEventReader {
    pool: Pool,
}

impl PostgresEventReader {
    pub async fn new(config: PostgresEventStoreConfig) -> Result<Self> {
        let pg_config = config.connection_string.parse::<tokio_postgres::Config>()?;

        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };
        let mgr = Manager::from_config(pg_config, NoTls, mgr_config);
        let pool = Pool::builder(mgr)
            .max_size(config.pool_size)
            .build()
            .context("failed to create connection pool")?;

        Ok(Self { pool })
    }

    pub async fn query(&self, query: ReplayQuery) -> Result<Vec<Message>> {
        let client = self
            .pool
            .get()
            .await
            .context("failed to get connection from pool")?;

        let mut conditions = Vec::new();
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> = Vec::new();
        let mut param_idx = 1;

        if let Some(start) = query.start_timestamp {
            conditions.push(format!("timestamp >= ${}", param_idx));
            params.push(Box::new(start as i64));
            param_idx += 1;
        }

        if let Some(end) = query.end_timestamp {
            conditions.push(format!("timestamp <= ${}", param_idx));
            params.push(Box::new(end as i64));
            param_idx += 1;
        }

        if !query.topics.is_empty() {
            let topic_conditions: Vec<_> = query
                .topics
                .iter()
                .map(|t| {
                    let cond = if t.contains('*') {
                        let pattern = t.replace(".*", "%").replace('*', "%");
                        let c = format!("topic LIKE ${}", param_idx);
                        params.push(Box::new(pattern));
                        c
                    } else {
                        let c = format!("topic = ${}", param_idx);
                        params.push(Box::new(t.clone()));
                        c
                    };
                    param_idx += 1;
                    cond
                })
                .collect();
            conditions.push(format!("({})", topic_conditions.join(" OR ")));
        }

        if !query.kinds.is_empty() {
            let kinds_str: Vec<_> = query.kinds.iter().map(|k| k.as_str()).collect();
            conditions.push(format!("kind = ANY(${})", param_idx));
            params.push(Box::new(kinds_str));
            // param_idx not needed after this point
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let limit_clause = query
            .limit
            .map(|l| format!(" LIMIT {}", l))
            .unwrap_or_default();

        let offset_clause = query
            .offset
            .map(|o| format!(" OFFSET {}", o))
            .unwrap_or_default();

        let sql = format!(
            "SELECT message_id, kind, topic, priority, timestamp, correlation_id, reply_to, payload \
             FROM event_log {} ORDER BY timestamp ASC{}{}",
            where_clause, limit_clause, offset_clause
        );

        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .iter()
            .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let rows = client
            .query(&sql, &param_refs)
            .await
            .context("failed to execute replay query")?;

        let messages: Vec<Message> = rows
            .iter()
            .map(|row| {
                let message_id: i64 = row.get(0);
                let kind_str: String = row.get(1);
                let topic_str: String = row.get(2);
                let priority_val: i32 = row.get(3);
                let timestamp: i64 = row.get(4);
                let correlation_id: Option<i64> = row.get(5);
                let reply_to: Option<String> = row.get(6);
                let payload: Vec<u8> = row.get(7);

                let kind = match kind_str.as_str() {
                    "data" => MessageKind::Data,
                    "event" => MessageKind::Event,
                    "command" => MessageKind::Command,
                    "system" => MessageKind::System,
                    _ => MessageKind::Data,
                };

                let priority = match priority_val {
                    0 => Priority::Low,
                    1 => Priority::Normal,
                    2 => Priority::High,
                    3 => Priority::Critical,
                    _ => Priority::Normal,
                };

                Message {
                    id: message_id as u64,
                    kind,
                    topic: Topic::new(topic_str),
                    payload,
                    priority,
                    timestamp: timestamp as u64,
                    correlation_id: correlation_id.map(|v| v as u64),
                    reply_to: reply_to.map(Topic::new),
                }
            })
            .collect();

        Ok(messages)
    }

    pub async fn count(&self, query: ReplayQuery) -> Result<u64> {
        let client = self
            .pool
            .get()
            .await
            .context("failed to get connection from pool")?;

        let mut conditions = Vec::new();
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> = Vec::new();
        let mut param_idx = 1;

        if let Some(start) = query.start_timestamp {
            conditions.push(format!("timestamp >= ${}", param_idx));
            params.push(Box::new(start as i64));
            param_idx += 1;
        }

        if let Some(end) = query.end_timestamp {
            conditions.push(format!("timestamp <= ${}", param_idx));
            params.push(Box::new(end as i64));
            let _ = param_idx; // Suppress unused warning
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!("SELECT COUNT(*) FROM event_log {}", where_clause);

        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .iter()
            .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let row = client
            .query_one(&sql, &param_refs)
            .await
            .context("failed to count events")?;

        let count: i64 = row.get(0);
        Ok(count as u64)
    }

    pub async fn stream(&self, query: ReplayQuery, batch_size: usize) -> Result<EventStream<'_>> {
        let count = self.count(query.clone()).await?;
        Ok(EventStream {
            reader: self,
            query,
            batch_size,
            total_count: count,
            current_offset: 0,
            buffer: Vec::new(),
        })
    }

    pub async fn get_topics(&self) -> Result<Vec<String>> {
        let client = self
            .pool
            .get()
            .await
            .context("failed to get connection from pool")?;

        let rows = client
            .query("SELECT DISTINCT topic FROM event_log ORDER BY topic", &[])
            .await?;

        Ok(rows.iter().map(|r| r.get(0)).collect())
    }

    pub async fn get_time_range(&self) -> Result<Option<(u64, u64)>> {
        let client = self
            .pool
            .get()
            .await
            .context("failed to get connection from pool")?;

        let row = client
            .query_one("SELECT MIN(timestamp), MAX(timestamp) FROM event_log", &[])
            .await?;

        let min: Option<i64> = row.get(0);
        let max: Option<i64> = row.get(1);

        match (min, max) {
            (Some(min), Some(max)) => Ok(Some((min as u64, max as u64))),
            _ => Ok(None),
        }
    }
}

pub struct EventStream<'a> {
    reader: &'a PostgresEventReader,
    query: ReplayQuery,
    batch_size: usize,
    total_count: u64,
    current_offset: usize,
    buffer: Vec<Message>,
}

impl<'a> EventStream<'a> {
    pub fn total_count(&self) -> u64 {
        self.total_count
    }
    
    /// Get the number of messages currently buffered
    pub fn buffered_count(&self) -> usize {
        self.buffer.len()
    }

    pub fn progress(&self) -> f64 {
        if self.total_count == 0 {
            1.0
        } else {
            self.current_offset as f64 / self.total_count as f64
        }
    }

    pub async fn next_batch(&mut self) -> Result<Option<Vec<Message>>> {
        if self.current_offset >= self.total_count as usize {
            return Ok(None);
        }

        let mut batch_query = self.query.clone();
        batch_query.limit = Some(self.batch_size);
        batch_query.offset = Some(self.current_offset);

        let messages = self.reader.query(batch_query).await?;
        self.current_offset += messages.len();

        if messages.is_empty() {
            Ok(None)
        } else {
            Ok(Some(messages))
        }
    }
}

/// Statistics for event replay from PostgreSQL
#[derive(Debug, Clone, Default)]
pub struct EventReplayStats {
    pub events_replayed: u64,
}

pub struct EventReplayer<B: neleus_core_bus::Bus> {
    reader: PostgresEventReader,
    bus: B,
}

impl<B: neleus_core_bus::Bus> EventReplayer<B> {
    pub fn new(reader: PostgresEventReader, bus: B) -> Self {
        Self { reader, bus }
    }

    pub async fn replay(&mut self, query: ReplayQuery) -> Result<EventReplayStats> {
        let mut stats = EventReplayStats::default();
        let mut stream = self.reader.stream(query, 10_000).await?;

        while let Some(batch) = stream.next_batch().await? {
            for message in batch {
                self.bus.publish(message);
                stats.events_replayed += 1;
            }
        }

        Ok(stats)
    }

    pub async fn replay_with_timing(
        &mut self,
        query: ReplayQuery,
        speed_multiplier: f64,
    ) -> Result<EventReplayStats> {
        let mut stats = EventReplayStats::default();
        let messages = self.reader.query(query).await?;

        if messages.is_empty() {
            return Ok(stats);
        }

        let base_timestamp = messages[0].timestamp;
        let replay_start = std::time::Instant::now();

        for message in messages {
            let event_offset = message.timestamp - base_timestamp;
            let target_elapsed =
                std::time::Duration::from_nanos((event_offset as f64 / speed_multiplier) as u64);

            let actual_elapsed = replay_start.elapsed();
            if target_elapsed > actual_elapsed {
                tokio::time::sleep(target_elapsed - actual_elapsed).await;
            }

            self.bus.publish(message);
            stats.events_replayed += 1;
        }

        Ok(stats)
    }

    pub fn bus_mut(&mut self) -> &mut B {
        &mut self.bus
    }

    pub fn into_bus(self) -> B {
        self.bus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postgres_config_defaults() {
        let config = PostgresEventStoreConfig::default();
        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.pool_size, 4);
    }
}
