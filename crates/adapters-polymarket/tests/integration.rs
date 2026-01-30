use neleus_adapters_polymarket::{
    PolymarketClient, PolymarketConfig, PolymarketSigner, PolymarketWebSocket, WsSubscription,
};

#[tokio::test]
#[ignore] // Ignore by default, run with --ignored flag
async fn test_get_markets() {
    let config = PolymarketConfig::mainnet();
    let client = PolymarketClient::new(config);

    match client.get_markets(Some(10)).await {
        Ok(markets) => {
            println!("Retrieved {} markets", markets.len());
            for market in markets.iter().take(3) {
                println!("Market: {} - {}", market.token_id, market.question);
            }
        }
        Err(e) => {
            eprintln!("Error getting markets: {}", e);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_get_price() {
    let config = PolymarketConfig::mainnet();
    let client = PolymarketClient::new(config);

    // You need a valid token_id to test this
    let token_id = "21742633143463906290569050155826241533067272736897614950488156847949938836455";

    match client.get_price(token_id).await {
        Ok(price) => {
            println!("Price for token {}: {}", token_id, price.price);
        }
        Err(e) => {
            eprintln!("Error getting price: {}", e);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_get_book() {
    let config = PolymarketConfig::mainnet();
    let client = PolymarketClient::new(config);

    // You need a valid token_id to test this
    let token_id = "21742633143463906290569050155826241533067272736897614950488156847949938836455";

    match client.get_book(token_id).await {
        Ok(book) => {
            println!("Order book for token {}", token_id);
            println!("Bids: {}, Asks: {}", book.bids.len(), book.asks.len());
            if !book.bids.is_empty() {
                println!("Best bid: {} @ {}", book.bids[0].size, book.bids[0].price);
            }
            if !book.asks.is_empty() {
                println!("Best ask: {} @ {}", book.asks[0].size, book.asks[0].price);
            }
        }
        Err(e) => {
            eprintln!("Error getting book: {}", e);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_websocket_connection() {
    let config = PolymarketConfig::mainnet();
    let mut ws = PolymarketWebSocket::new(config);

    match ws.connect().await {
        Ok(_) => {
            println!("WebSocket connected successfully");

            // Subscribe to a market
            let token_id =
                "21742633143463906290569050155826241533067272736897614950488156847949938836455";
            let subscription = WsSubscription::market(token_id.to_string());

            if let Err(e) = ws.subscribe(subscription).await {
                eprintln!("Failed to subscribe: {}", e);
            }

            // Start receiving messages
            match ws.start_receiving().await {
                Ok(mut rx) => {
                    println!("Listening for messages...");

                    // Listen for a few messages
                    for _ in 0..5 {
                        tokio::select! {
                            Some(msg) = rx.recv() => {
                                println!("Received message: {:?}", msg);
                            }
                            _ = tokio::time::sleep(tokio::time::Duration::from_secs(10)) => {
                                println!("Timeout waiting for messages");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to start receiving: {}", e);
                }
            }

            // Disconnect
            if let Err(e) = ws.disconnect().await {
                eprintln!("Failed to disconnect: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to connect: {}", e);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_authentication_flow() {
    // This test requires valid credentials set as environment variables:
    // POLYMARKET_PRIVATE_KEY and POLYMARKET_ADDRESS

    let private_key =
        std::env::var("POLYMARKET_PRIVATE_KEY").expect("POLYMARKET_PRIVATE_KEY not set");
    let address = std::env::var("POLYMARKET_ADDRESS").expect("POLYMARKET_ADDRESS not set");

    let config = PolymarketConfig::mainnet().with_private_key(address, private_key);

    let signer = PolymarketSigner::new(config.clone()).expect("Failed to create signer");
    let mut client = PolymarketClient::new(config).with_signer(signer);

    // Try to create or derive API key
    match client.create_api_key().await {
        Ok(creds) => {
            println!("API credentials created:");
            println!("API Key: {}", creds.api_key);
            println!("Passphrase: {}", creds.passphrase);
        }
        Err(e) => {
            // If creation fails, try deriving
            eprintln!("Failed to create API key: {}, trying to derive...", e);
            match client.derive_api_key().await {
                Ok(creds) => {
                    println!("API credentials derived:");
                    println!("API Key: {}", creds.api_key);
                }
                Err(e) => {
                    eprintln!("Failed to derive API key: {}", e);
                }
            }
        }
    }
}

#[test]
fn test_config_creation() {
    let mainnet = PolymarketConfig::mainnet();
    assert_eq!(mainnet.chain_id, 137);
    assert_eq!(mainnet.clob_url, "https://clob.polymarket.com");

    let testnet = PolymarketConfig::testnet();
    assert_eq!(testnet.chain_id, 80002);
    assert_eq!(testnet.clob_url, "https://clob-staging.polymarket.com");
}
