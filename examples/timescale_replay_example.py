"""
Example: TimescaleDB integration for efficient historical replay

This example demonstrates:
1. Setting up TimescaleDB store
2. Inserting historical market data (candles, trades)
3. Running historical replays with different speeds
4. Querying time-series data efficiently
"""

import time
from datetime import datetime, timedelta
import neleus

def main():
    # Initialize TimescaleDB store
    # Make sure TimescaleDB extension is installed in your PostgreSQL database:
    # CREATE EXTENSION IF NOT EXISTS timescaledb;
    
    print("=" * 80)
    print("Neleus TimescaleDB Historical Replay Example")
    print("=" * 80)
    
    # Example 1: Load historical data
    print("\n1. Loading historical market data...")
    print("-" * 80)
    
    # Note: In real usage, you would use the Rust API or Python bindings
    # This is a conceptual example showing the workflow
    
    # Configuration for TimescaleDB
    timescale_config = {
        "connection_string": "postgresql://postgres:postgres@localhost:5432/neleus_timeseries",
        "pool_size": 8,
        "batch_size": 5000,
        "flush_interval_ms": 100
    }
    
    print(f"  Connection: {timescale_config['connection_string']}")
    print(f"  Pool size: {timescale_config['pool_size']}")
    print(f"  Batch size: {timescale_config['batch_size']}")
    
    # Example 2: Historical replay configuration
    print("\n2. Configuring historical replay...")
    print("-" * 80)
    
    replay_config = {
        "start_time": datetime.now() - timedelta(days=30),  # Last 30 days
        "end_time": datetime.now(),
        "venues": ["hyperliquid", "lighter"],
        "symbols": ["BTC-PERP", "ETH-PERP"],
        "speed_multiplier": 10.0,  # 10x speed (or 0.0 for max speed)
        "include_trades": True,
        "include_quotes": True,
        "include_candles": True,
        "candle_interval": "1m",  # Use 1-minute aggregated candles
        "buffer_size": 10000
    }
    
    print(f"  Period: {replay_config['start_time']} to {replay_config['end_time']}")
    print(f"  Duration: {(replay_config['end_time'] - replay_config['start_time']).days} days")
    print(f"  Venues: {', '.join(replay_config['venues'])}")
    print(f"  Symbols: {', '.join(replay_config['symbols'])}")
    print(f"  Speed: {replay_config['speed_multiplier']}x")
    print(f"  Candle interval: {replay_config['candle_interval']}")
    
    # Example 3: Query capabilities
    print("\n3. TimescaleDB query capabilities...")
    print("-" * 80)
    
    queries = [
        "Get candles for specific period",
        "Get trades with limit",
        "Get latest quote (BBO)",
        "Get available time range",
        "Get all symbols for a venue",
        "Query continuous aggregates (1m, 5m, 15m, 1h)",
    ]
    
    for i, query in enumerate(queries, 1):
        print(f"  {i}. {query}")
    
    # Example 4: TimescaleDB features
    print("\n4. TimescaleDB optimization features...")
    print("-" * 80)
    
    features = [
        ("Hypertables", "Automatic time-based partitioning"),
        ("Compression", "Compress data older than 7 days"),
        ("Continuous Aggregates", "Pre-computed 1m, 5m, 15m, 1h candles"),
        ("Retention Policies", "Optional auto-deletion of old data"),
        ("Time Bucketing", "Efficient time-range queries"),
        ("Chunk-based Storage", "Parallel query execution"),
    ]
    
    for feature, description in features:
        print(f"  • {feature:25s} - {description}")
    
    # Example 5: Data types stored
    print("\n5. Market data types...")
    print("-" * 80)
    
    data_types = [
        ("market_ticks", "OHLCV candles with volume and VWAP"),
        ("trades", "Tick-by-tick trade data"),
        ("quotes", "Best bid/offer updates"),
        ("order_book_snapshots", "Full orderbook snapshots"),
        ("funding_rates", "Perpetual funding rates"),
        ("indicators", "Technical indicators (ATR, BB, etc.)"),
    ]
    
    for table, description in data_types:
        print(f"  • {table:25s} - {description}")
    
    # Example 6: Replay modes
    print("\n6. Historical replay modes...")
    print("-" * 80)
    
    replay_modes = [
        ("Max Speed", 0.0, "Process data as fast as possible"),
        ("Real-time", 1.0, "1:1 time ratio (1 sec data = 1 sec replay)"),
        ("Fast Forward", 10.0, "10x speed (1 hour data in 6 minutes)"),
        ("Super Fast", 100.0, "100x speed (1 day data in 14 minutes)"),
    ]
    
    print(f"  {'Mode':<15} {'Multiplier':<12} {'Description'}")
    print(f"  {'-'*15} {'-'*12} {'-'*40}")
    for mode, mult, desc in replay_modes:
        print(f"  {mode:<15} {mult:<12.1f} {desc}")
    
    # Example 7: Performance benefits
    print("\n7. Performance advantages vs. generic event_log...")
    print("-" * 80)
    
    benefits = [
        ("Query Speed", "10-100x faster for time-range queries"),
        ("Storage", "3-20x compression on historical data"),
        ("Aggregations", "Pre-computed rollups (1m, 5m, 1h)"),
        ("Indexing", "Optimized time-based indexes"),
        ("Parallelism", "Chunk-aware parallel scans"),
        ("Memory", "Lower memory footprint for large datasets"),
    ]
    
    for benefit, improvement in benefits:
        print(f"  • {benefit:20s} - {improvement}")
    
    # Example 8: Integration with backtest
    print("\n8. Backtest integration example...")
    print("-" * 80)
    
    print("""
  # Python backtest code example:
  
  from neleus import TimescaleStore, HistoricalReplayer, ReplayConfig
  import asyncio
  
  async def run_backtest():
      # Setup TimescaleDB
      store = await TimescaleStore.new({
          "connection_string": "postgresql://localhost/neleus_timeseries"
      })
      
      # Configure replay
      config = ReplayConfig(
          start_time=datetime(2024, 1, 1),
          end_time=datetime(2024, 3, 1),
          venues=["hyperliquid"],
          symbols=["BTC-PERP"],
          speed_multiplier=0.0,  # Max speed
          include_candles=True,
          candle_interval="1m"
      )
      
      # Start replay
      replayer = HistoricalReplayer(store, config)
      event_stream, progress_stream = await replayer.replay()
      
      # Process events
      async for event in event_stream:
          if event.is_candle():
              candle = event.as_candle()
              # Run your strategy logic here
              print(f"Candle: {candle.time} O:{candle.open} H:{candle.high} "
                    f"L:{candle.low} C:{candle.close} V:{candle.volume}")
          
          elif event.is_trade():
              trade = event.as_trade()
              print(f"Trade: {trade.time} {trade.side} {trade.price} x {trade.size}")
      
      # Check progress
      async for progress in progress_stream:
          print(f"Progress: {progress.progress_pct:.1f}% "
                f"({progress.events_processed} events)")
  
  # Run the backtest
  asyncio.run(run_backtest())
    """)
    
    # Example 9: Setup instructions
    print("\n9. Setup instructions...")
    print("-" * 80)
    
    print("""
  1. Install TimescaleDB extension:
     
     # For PostgreSQL 12+
     sudo apt install timescaledb-2-postgresql-14
     
     # Or using Docker:
     docker run -d --name timescaledb -p 5432:5432 \\
       -e POSTGRES_PASSWORD=postgres \\
       timescale/timescaledb:latest-pg14
  
  2. Create database:
     
     createdb neleus_timeseries
  
  3. Enable extension (done automatically by TimescaleStore):
     
     psql -d neleus_timeseries -c "CREATE EXTENSION IF NOT EXISTS timescaledb;"
  
  4. The Rust persistence crate will automatically:
     - Create hypertables for all market data tables
     - Set up continuous aggregates (1m, 5m, 15m, 1h)
     - Configure compression policies
     - Create optimized indexes
    """)
    
    # Example 10: Query examples
    print("\n10. Example queries...")
    print("-" * 80)
    
    print("""
  -- Get last 100 candles for BTC
  SELECT * FROM market_ticks 
  WHERE venue = 'hyperliquid' AND symbol = 'BTC-PERP'
  ORDER BY time DESC LIMIT 100;
  
  -- Get 1-minute aggregated trades (using continuous aggregate)
  SELECT * FROM trades_1m
  WHERE venue = 'hyperliquid' AND symbol = 'BTC-PERP'
    AND bucket >= NOW() - INTERVAL '1 day'
  ORDER BY bucket;
  
  -- Calculate VWAP over last hour
  SELECT 
    time_bucket('1 hour', time) as hour,
    SUM(price * size) / SUM(size) as vwap
  FROM trades
  WHERE venue = 'hyperliquid' AND symbol = 'BTC-PERP'
    AND time >= NOW() - INTERVAL '1 hour'
  GROUP BY hour;
  
  -- Get funding rate history
  SELECT * FROM funding_rates
  WHERE venue = 'hyperliquid' AND symbol = 'BTC-PERP'
    AND time >= NOW() - INTERVAL '7 days'
  ORDER BY time;
  
  -- Check hypertable stats
  SELECT * FROM timescaledb_information.hypertables;
  
  -- View compression stats
  SELECT * FROM timescaledb_information.compression_settings;
    """)
    
    print("\n" + "=" * 80)
    print("For more information, see the Rust documentation in:")
    print("  - crates/persistence/src/timescale.rs")
    print("  - crates/persistence/src/replay.rs")
    print("=" * 80)

if __name__ == "__main__":
    main()
