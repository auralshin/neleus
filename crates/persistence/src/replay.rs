use anyhow::Result;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

use crate::timescale::{Candle, Quote, TimescaleStore, Trade};

/// Configuration for historical replay
#[derive(Debug, Clone)]
pub struct ReplayConfig {
    /// Start time for replay
    pub start_time: DateTime<Utc>,
    
    /// End time for replay
    pub end_time: DateTime<Utc>,
    
    /// Venues to include in replay
    pub venues: Vec<String>,
    
    /// Symbols to include in replay
    pub symbols: Vec<String>,
    
    /// Speed multiplier (1.0 = real-time, 0 = as fast as possible)
    pub speed_multiplier: f64,
    
    /// Include trade ticks
    pub include_trades: bool,
    
    /// Include quotes (BBO)
    pub include_quotes: bool,
    
    /// Include OHLCV candles
    pub include_candles: bool,
    
    /// Candle interval for replay (e.g., "1m", "5m", "15m", "1h")
    pub candle_interval: Option<String>,
    
    /// Buffer size for event channel
    pub buffer_size: usize,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            start_time: Utc::now() - chrono::Duration::days(1),
            end_time: Utc::now(),
            venues: vec![],
            symbols: vec![],
            speed_multiplier: 0.0, // As fast as possible
            include_trades: true,
            include_quotes: true,
            include_candles: false,
            candle_interval: None,
            buffer_size: 10000,
        }
    }
}

/// Market data event types for replay
#[derive(Debug, Clone)]
pub enum MarketEvent {
    Trade(Trade),
    Quote(Quote),
    Candle(Candle),
}

impl MarketEvent {
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            MarketEvent::Trade(t) => t.time,
            MarketEvent::Quote(q) => q.time,
            MarketEvent::Candle(c) => c.time,
        }
    }

    pub fn venue(&self) -> &str {
        match self {
            MarketEvent::Trade(t) => &t.venue,
            MarketEvent::Quote(q) => &q.venue,
            MarketEvent::Candle(c) => &c.venue,
        }
    }

    pub fn symbol(&self) -> &str {
        match self {
            MarketEvent::Trade(t) => &t.symbol,
            MarketEvent::Quote(q) => &q.symbol,
            MarketEvent::Candle(c) => &c.symbol,
        }
    }
}

/// Progress tracking for replay
#[derive(Debug, Clone)]
pub struct ReplayProgress {
    pub current_time: DateTime<Utc>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub events_processed: usize,
    pub progress_pct: f64,
}

impl ReplayProgress {
    pub fn new(start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Self {
        Self {
            current_time: start_time,
            start_time,
            end_time,
            events_processed: 0,
            progress_pct: 0.0,
        }
    }

    pub fn update(&mut self, current_time: DateTime<Utc>, events_processed: usize) {
        self.current_time = current_time;
        self.events_processed = events_processed;

        let total_duration = (self.end_time - self.start_time).num_milliseconds() as f64;
        let elapsed = (current_time - self.start_time).num_milliseconds() as f64;

        self.progress_pct = if total_duration > 0.0 {
            (elapsed / total_duration * 100.0).min(100.0)
        } else {
            100.0
        };
    }

    pub fn is_complete(&self) -> bool {
        self.current_time >= self.end_time
    }
}

/// Historical replay engine using TimescaleDB
pub struct HistoricalReplayer {
    store: Arc<TimescaleStore>,
    config: ReplayConfig,
}

impl HistoricalReplayer {
    pub fn new(store: Arc<TimescaleStore>, config: ReplayConfig) -> Self {
        Self { store, config }
    }

    /// Start the replay and return a stream of market events
    pub async fn replay(
        &self,
    ) -> Result<(
        mpsc::Receiver<MarketEvent>,
        mpsc::Receiver<ReplayProgress>,
    )> {
        let (event_tx, event_rx) = mpsc::channel(self.config.buffer_size);
        let (progress_tx, progress_rx) = mpsc::channel(10);

        let store = Arc::clone(&self.store);
        let config = self.config.clone();

        // Spawn replay task
        tokio::spawn(async move {
            if let Err(e) = Self::replay_task(store, config, event_tx, progress_tx).await {
                tracing::error!("Replay task failed: {}", e);
            }
        });

        Ok((event_rx, progress_rx))
    }

    async fn replay_task(
        store: Arc<TimescaleStore>,
        config: ReplayConfig,
        event_tx: mpsc::Sender<MarketEvent>,
        progress_tx: mpsc::Sender<ReplayProgress>,
    ) -> Result<()> {
        info!(
            "Starting historical replay from {} to {} (speed: {}x)",
            config.start_time, config.end_time, config.speed_multiplier
        );

        let mut progress = ReplayProgress::new(config.start_time, config.end_time);

        // If venues/symbols are empty, query all available
        let venues = if config.venues.is_empty() {
            vec!["hyperliquid".to_string(), "lighter".to_string(), "polymarket".to_string()]
        } else {
            config.venues.clone()
        };

        for venue in &venues {
            let symbols = if config.symbols.is_empty() {
                store
                    .get_available_symbols(venue)
                    .await
                    .unwrap_or_default()
            } else {
                config.symbols.clone()
            };

            for symbol in &symbols {
                info!("Replaying {} on {}", symbol, venue);

                // Replay candles if enabled
                if config.include_candles {
                    let candles = if let Some(interval) = &config.candle_interval {
                        store
                            .get_candles_aggregated(
                                venue,
                                symbol,
                                interval,
                                config.start_time,
                                config.end_time,
                            )
                            .await?
                    } else {
                        store
                            .get_candles(venue, symbol, config.start_time, config.end_time)
                            .await?
                    };

                    for candle in candles {
                        if event_tx
                            .send(MarketEvent::Candle(candle.clone()))
                            .await
                            .is_err()
                        {
                            return Ok(()); // Receiver dropped
                        }

                        progress.update(candle.time, progress.events_processed + 1);
                        let _ = progress_tx.try_send(progress.clone());

                        // Simulate timing if speed_multiplier > 0
                        if config.speed_multiplier > 0.0 {
                            tokio::time::sleep(tokio::time::Duration::from_millis(
                                (1000.0 / config.speed_multiplier) as u64,
                            ))
                            .await;
                        }
                    }
                }

                // Replay trades if enabled
                if config.include_trades {
                    let trades = store
                        .get_trades(venue, symbol, config.start_time, config.end_time, None)
                        .await?;

                    let mut last_time = config.start_time;

                    for trade in trades {
                        if event_tx
                            .send(MarketEvent::Trade(trade.clone()))
                            .await
                            .is_err()
                        {
                            return Ok(()); // Receiver dropped
                        }

                        progress.update(trade.time, progress.events_processed + 1);
                        
                        if progress.events_processed % 1000 == 0 {
                            let _ = progress_tx.try_send(progress.clone());
                        }

                        // Simulate timing if speed_multiplier > 0
                        if config.speed_multiplier > 0.0 && trade.time > last_time {
                            let time_diff = (trade.time - last_time).num_milliseconds() as f64;
                            let sleep_ms =
                                (time_diff / config.speed_multiplier).max(0.0) as u64;

                            if sleep_ms > 0 {
                                tokio::time::sleep(tokio::time::Duration::from_millis(sleep_ms))
                                    .await;
                            }
                        }

                        last_time = trade.time;
                    }
                }

                // Note: Quotes replay would go here if needed
                // Currently omitted as it can be very high volume
            }
        }

        info!(
            "Replay complete. Processed {} events",
            progress.events_processed
        );
        let _ = progress_tx.send(progress).await;

        Ok(())
    }

    /// Get replay statistics
    pub async fn get_stats(&self) -> Result<ReplayStats> {
        let venues = if self.config.venues.is_empty() {
            vec!["hyperliquid".to_string(), "lighter".to_string()]
        } else {
            self.config.venues.clone()
        };

        let mut total_candles = 0;
        let mut total_trades = 0;
        let mut symbols_count = 0;

        for venue in &venues {
            let symbols = if self.config.symbols.is_empty() {
                self.store
                    .get_available_symbols(venue)
                    .await
                    .unwrap_or_default()
            } else {
                self.config.symbols.clone()
            };

            symbols_count += symbols.len();

            for symbol in &symbols {
                // Count candles
                let candles = self
                    .store
                    .get_candles(venue, symbol, self.config.start_time, self.config.end_time)
                    .await?;
                total_candles += candles.len();

                // Count trades (limit to avoid huge queries)
                let trades = self
                    .store
                    .get_trades(
                        venue,
                        symbol,
                        self.config.start_time,
                        self.config.end_time,
                        Some(10000),
                    )
                    .await?;
                total_trades += trades.len();
            }
        }

        Ok(ReplayStats {
            start_time: self.config.start_time,
            end_time: self.config.end_time,
            venues: venues.len(),
            symbols: symbols_count,
            total_candles,
            total_trades,
        })
    }
}

/// Replay statistics
#[derive(Debug, Clone)]
pub struct ReplayStats {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub venues: usize,
    pub symbols: usize,
    pub total_candles: usize,
    pub total_trades: usize,
}

impl ReplayStats {
    pub fn duration(&self) -> chrono::Duration {
        self.end_time - self.start_time
    }

    pub fn estimated_events(&self) -> usize {
        self.total_candles + self.total_trades
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_progress() {
        let start = Utc::now();
        let end = start + chrono::Duration::hours(1);
        let mut progress = ReplayProgress::new(start, end);

        assert_eq!(progress.progress_pct, 0.0);

        let mid = start + chrono::Duration::minutes(30);
        progress.update(mid, 100);

        assert!(progress.progress_pct > 49.0 && progress.progress_pct < 51.0);
        assert_eq!(progress.events_processed, 100);
    }

    #[test]
    fn test_market_event_accessors() {
        let trade = Trade {
            time: Utc::now(),
            venue: "test".to_string(),
            symbol: "BTC".to_string(),
            instrument_type: "spot".to_string(),
            trade_id: Some("123".to_string()),
            side: "buy".to_string(),
            price: 50000.0,
            size: 1.0,
            is_buyer_maker: Some(true),
        };

        let event = MarketEvent::Trade(trade.clone());
        assert_eq!(event.venue(), "test");
        assert_eq!(event.symbol(), "BTC");
        assert_eq!(event.timestamp(), trade.time);
    }
}
