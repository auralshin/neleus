// Module declarations
pub mod advanced;
pub mod config;
pub mod datafeed;
pub mod execution;
pub mod node;
pub mod optimization;
pub mod simulation;

pub use config::{
    BacktestConfig, FillModelConfig, FillModelType, LatencyModelConfig, LatencyModelType,
    LatencySimulator, SimulationMode,
};

pub use datafeed::{
    CsvDataFeed, CsvError, DataFeed, HistoricalData, HistoricalDataPoint, InMemoryDataFeed,
};

#[cfg(feature = "hyperliquid")]
pub use datafeed::{HyperliquidBacktestBuilder, HyperliquidDataFeed};

pub use simulation::{SimulatedBook, SimulatedOrder, SimulatedVenue};

pub use node::{BacktestNode, BacktestResults};

pub use advanced::{
    L2OrderBook, MarketOrderSimulation, MultiFeedMerger, SlippageModel, SlippageModelConfig,
    SlippageModelType,
};

pub use execution::{
    ExecutionAlgorithmManager, ExecutionAlgorithmState, ExecutionAlgorithmType, IcebergConfig,
    PovConfig, TwapConfig, VwapConfig,
};

pub use optimization::{
    OptimizationMetric, ParameterDef, ParameterSensitivity, ParameterSweep, ParameterSweepConfig,
    SweepResult, WalkForwardAnalysis, WalkForwardConfig, WalkForwardResult, WalkForwardSplitter,
    WalkForwardWindow,
};

pub use csv;

#[cfg(test)]
mod tests {
    use super::*;
    use neleus_core_engine::OrderSide;
    use neleus_core_types::{InstrumentId, InstrumentType, UnixNanos, Venue};
    use std::time::Instant;

    #[test]
    fn test_in_memory_feed() {
        let trades = vec![
            (1000, "BTC", 50000.0, 1.0, true),
            (2000, "BTC", 50100.0, 0.5, false),
            (3000, "BTC", 50050.0, 2.0, true),
        ];
        let mut feed = InMemoryDataFeed::from_trades(trades);

        let first = feed.next().unwrap();
        assert_eq!(first.timestamp.as_millis(), 1000);

        let second = feed.next().unwrap();
        assert_eq!(second.timestamp.as_millis(), 2000);
    }

    #[test]
    fn test_simulated_book() {
        let instrument_id = InstrumentId::new(Venue::Simulated, "ETH", InstrumentType::Perp);
        let mut book = SimulatedBook::new(instrument_id);

        book.update_snapshot(
            vec![(2000.0, 10.0), (1999.0, 20.0)],
            vec![(2001.0, 10.0), (2002.0, 15.0)],
            UnixNanos::from_millis(1000),
        );

        assert_eq!(book.best_bid(), Some(2000.0));
        assert_eq!(book.best_ask(), Some(2001.0));
        assert_eq!(book.spread(), Some(1.0));
    }

    #[test]
    fn test_simulated_venue_market_order() {
        use neleus_core_engine::{OrderType, StrategyCommand};
        use neleus_core_types::OrderId;

        let fill_config = FillModelConfig::default();
        let mut venue = SimulatedVenue::new(Venue::Simulated, fill_config, 0.0004);

        let instrument_id = InstrumentId::new(Venue::Simulated, "BTC", InstrumentType::Perp);
        let ts = UnixNanos::from_millis(1000);

        venue.on_data(
            &HistoricalData::Quote {
                instrument_id: instrument_id.clone(),
                bid_price: 50000.0,
                bid_size: 10.0,
                ask_price: 50010.0,
                ask_size: 10.0,
            },
            ts,
        );

        let cmd = StrategyCommand::SubmitOrder {
            order_id: OrderId::new("test-1"),
            instrument_id,
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            price: None,
            quantity: 1.0,
        };

        venue.submit_order(&cmd, ts);

        let events = venue.drain_events();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_backtest_node() {
        let config = BacktestConfig {
            start_time: UnixNanos::from_millis(0),
            end_time: UnixNanos::from_millis(10000),
            ..Default::default()
        };

        let trades = vec![
            (1000, "BTC", 50000.0, 1.0, true),
            (2000, "BTC", 50100.0, 0.5, false),
            (3000, "BTC", 50050.0, 2.0, true),
        ];

        let mut node = BacktestNode::new(config);
        node.set_data_feed(Box::new(InMemoryDataFeed::from_trades(trades)));

        let results = node.run();
        assert_eq!(results.data_points_processed, 3);
    }

    #[test]
    fn test_l2_orderbook() {
        let instrument_id = InstrumentId::new(Venue::Simulated, "ETH", InstrumentType::Perp);
        let mut book = L2OrderBook::new(instrument_id, 10);

        book.apply_snapshot(
            &[(2000.0, 10.0), (1999.0, 20.0), (1998.0, 30.0)],
            &[(2001.0, 15.0), (2002.0, 25.0), (2003.0, 35.0)],
            UnixNanos::from_millis(1000),
            1,
        );

        assert_eq!(book.best_bid().unwrap().0, 2000.0);
        assert_eq!(book.best_ask().unwrap().0, 2001.0);
        assert_eq!(book.spread().unwrap(), 1.0);

        let micro = book.micro_price().unwrap();
        assert!(micro > 2000.0 && micro < 2001.0);

        let sim = book.simulate_market_order(OrderSide::Buy, 20.0);
        assert!(sim.filled_quantity <= 20.0);
        assert!(sim.levels_consumed >= 1);
    }

    #[test]
    fn test_walk_forward_splitter() {
        let config = WalkForwardConfig {
            num_windows: 3,
            in_sample_fraction: 0.7,
            anchored: false,
            min_in_sample_periods: 10,
        };

        let splitter = WalkForwardSplitter::new(config);
        let start = UnixNanos::from_secs(0);
        let end = UnixNanos::from_secs(30 * 86400);

        let windows = splitter.generate_windows(start, end);
        assert_eq!(windows.len(), 3);

        for w in &windows {
            assert!(w.train_end <= w.test_start);
            assert!(w.test_start < w.test_end);
        }
    }

    #[test]
    fn test_parameter_sweep() {
        let config = ParameterSweepConfig {
            parameters: vec![
                ParameterDef::new("fast_period", 5.0, 15.0, 5.0),
                ParameterDef::new("slow_period", 20.0, 30.0, 5.0),
            ],
            target_metric: OptimizationMetric::SharpeRatio,
            maximize: true,
        };

        let sweep = ParameterSweep::new(config);
        let combos = sweep.generate_combinations();

        assert_eq!(combos.len(), 9);
        assert_eq!(sweep.total_combinations(), 9);
    }

    #[test]
    fn test_slippage_models() {
        let config = SlippageModelConfig {
            model_type: SlippageModelType::VolumeImpact,
            volume_impact_coef: 0.1,
            adv: 1_000_000.0,
            ..Default::default()
        };

        let model = SlippageModel::new(config);

        let small_fill = model.calculate_fill_price(OrderSide::Buy, 100.0, 50000.0, None, None);

        let large_fill = model.calculate_fill_price(OrderSide::Buy, 10000.0, 50000.0, None, None);

        assert!(large_fill > small_fill);
    }

    #[test]
    fn test_multi_feed_merger() {
        let feed1 = Box::new(InMemoryDataFeed::from_trades(vec![
            (1000, "BTC", 50000.0, 1.0, true),
            (3000, "BTC", 50100.0, 1.0, true),
        ]));

        let feed2 = Box::new(InMemoryDataFeed::from_trades(vec![
            (2000, "ETH", 3000.0, 2.0, false),
            (4000, "ETH", 3010.0, 2.0, false),
        ]));

        let mut merger = MultiFeedMerger::new();
        merger.add_feed(feed1);
        merger.add_feed(feed2);

        let first = merger.next().unwrap();
        assert_eq!(first.timestamp.as_millis(), 1000);

        let second = merger.next().unwrap();
        assert_eq!(second.timestamp.as_millis(), 2000);

        let third = merger.next().unwrap();
        assert_eq!(third.timestamp.as_millis(), 3000);
    }

    #[test]
    #[ignore = "manual performance harness"]
    fn bench_backtest_node_hot_path() {
        let points = 100_000u64;
        let trades: Vec<_> = (0..points)
            .map(|i| {
                let ts = (i + 1) * 1000;
                let price = 50_000.0 + (i % 200) as f64;
                let qty = 1.0 + (i % 10) as f64 * 0.1;
                let is_buy = i % 2 == 0;
                (ts, "BTC", price, qty, is_buy)
            })
            .collect();

        let config = BacktestConfig {
            start_time: UnixNanos::from_millis(0),
            end_time: UnixNanos::from_millis(points * 1000 + 1000),
            ..Default::default()
        };

        let mut node = BacktestNode::new(config);
        node.set_data_feed(Box::new(InMemoryDataFeed::from_trades(trades)));

        let started = Instant::now();
        let results = node.run();
        let elapsed = started.elapsed();

        eprintln!(
            "processed {} points in {:?} ({:.0} pts/sec)",
            results.data_points_processed,
            elapsed,
            results.data_points_processed as f64 / elapsed.as_secs_f64()
        );

        assert_eq!(results.data_points_processed, points);
    }
}
