use neleus_adapters_lighter::{LighterConfig, LighterMarketDataMessage, LighterWsMarketData};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
#[ignore]
async fn test_lighter_market_data_connection() {
    let config = LighterConfig::testnet();
    let mut ws_client = LighterWsMarketData::new(config);

    ws_client.subscribe_trades(1);

    let message_count = Arc::new(AtomicUsize::new(0));
    let message_count_clone = message_count.clone();

    let callback = move |msg: LighterMarketDataMessage| {
        message_count_clone.fetch_add(1, Ordering::SeqCst);
        println!("Received message: {:?}", msg);
    };

    let connect_future = ws_client.connect(callback);
    let result = timeout(Duration::from_secs(5), connect_future).await;

    assert!(result.is_err());

    let count = message_count.load(Ordering::SeqCst);
    println!("Received {} messages in 5 seconds", count);
}

#[tokio::test]
#[ignore]
async fn test_lighter_orderbook_subscription() {
    let config = LighterConfig::testnet();
    let mut ws_client = LighterWsMarketData::new(config);

    ws_client.subscribe_orderbook(1);

    let orderbook_received = Arc::new(AtomicUsize::new(0));
    let orderbook_clone = orderbook_received.clone();

    let callback = move |msg: LighterMarketDataMessage| {
        if matches!(msg, LighterMarketDataMessage::OrderBook { .. }) {
            orderbook_clone.fetch_add(1, Ordering::SeqCst);
            println!("Received orderbook update");
        }
    };

    let connect_future = ws_client.connect(callback);
    let result = timeout(Duration::from_secs(3), connect_future).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_lighter_config() {
    let mainnet = LighterConfig::mainnet();
    assert!(!mainnet.testnet);
    assert_eq!(mainnet.ws_url, "wss://mainnet.zklighter.elliot.ai/stream");

    let testnet = LighterConfig::testnet();
    assert!(testnet.testnet);
    assert_eq!(testnet.ws_url, "wss://testnet.zklighter.elliot.ai/stream");
}
