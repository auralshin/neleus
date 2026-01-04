use neleus_adapters_hyperliquid::{
    HyperliquidConfig, HyperliquidWsMarketData, HyperliquidWsMessage,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
#[ignore]
async fn test_hyperliquid_market_data_connection() {
    let config = HyperliquidConfig::testnet();
    let mut ws_client = HyperliquidWsMarketData::new(config);

    ws_client.subscribe_trades("BTC");

    let message_count = Arc::new(AtomicUsize::new(0));
    let message_count_clone = message_count.clone();

    let callback = move |msg: HyperliquidWsMessage| {
        message_count_clone.fetch_add(1, Ordering::SeqCst);
        println!("Received message: {:?}", msg);
    };

    let connect_future = ws_client.connect(callback);
    let result = timeout(Duration::from_secs(5), connect_future).await;

    assert!(result.is_err());

    let count = message_count.load(Ordering::SeqCst);
    println!("Received {} messages in 5 seconds", count);
    assert!(count > 0, "Should have received at least one message");
}

#[tokio::test]
#[ignore]
async fn test_hyperliquid_orderbook_subscription() {
    let config = HyperliquidConfig::testnet();
    let mut ws_client = HyperliquidWsMarketData::new(config);

    ws_client.subscribe_l2_book("BTC");

    let orderbook_received = Arc::new(AtomicUsize::new(0));
    let orderbook_clone = orderbook_received.clone();

    let callback = move |msg: HyperliquidWsMessage| {
        if matches!(msg, HyperliquidWsMessage::L2Book { .. }) {
            orderbook_clone.fetch_add(1, Ordering::SeqCst);
            println!("Received L2 book update");
        }
    };

    let connect_future = ws_client.connect(callback);
    let result = timeout(Duration::from_secs(3), connect_future).await;

    assert!(result.is_err());

    let count = orderbook_received.load(Ordering::SeqCst);
    println!("Received {} orderbook updates", count);
    assert!(count > 0, "Should have received orderbook updates");
}

#[tokio::test]
async fn test_hyperliquid_config() {
    let mainnet = HyperliquidConfig::mainnet();
    assert!(!mainnet.testnet);
    assert_eq!(mainnet.ws_url, "wss://api.hyperliquid.xyz/ws");

    let testnet = HyperliquidConfig::testnet();
    assert!(testnet.testnet);
    assert_eq!(testnet.ws_url, "wss://api.hyperliquid-testnet.xyz/ws");
}
