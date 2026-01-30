// Module declarations
mod adapters;
mod auth;
mod client;
mod config;
mod types;

// Re-export public API
pub use config::{
    AccountTier, LighterConfig, LighterError, OperationWeights, RateLimitConfig, ReconnectConfig,
    WeightedRateLimiter,
};

pub use types::{
    AccountData, LighterBookLevel, LighterCancelRequest, LighterFillData, LighterMarketDataMessage,
    LighterMarketInfo, LighterOrderBook, LighterOrderData, LighterOrderRequest, LighterOrderStatus,
    LighterOrderType, LighterTrade, LighterTradeData, LighterUserFill, LighterUserOrder,
    LighterWsMessage, OrderBookData, WsSubscription, WsSubscriptionType,
};

pub use auth::LighterSigner;

pub use client::{
    LighterBusClient, LighterBusConnectedHandler, LighterExecution, LighterMarketDataHandler,
    LighterRestPublic, LighterWsMarketData, LighterWsUserStream, WsClient, WsMessageHandler,
};

pub use adapters::{LighterAdapter, LighterDataAdapter, LighterExecutionAdapter};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_urls() {
        let mainnet = LighterConfig::mainnet();
        assert_eq!(mainnet.ws_url, "wss://mainnet.zklighter.elliot.ai/stream");
        assert!(!mainnet.testnet);

        let testnet = LighterConfig::testnet();
        assert_eq!(testnet.ws_url, "wss://testnet.zklighter.elliot.ai/stream");
        assert!(testnet.testnet);
    }

    #[test]
    fn test_rate_limiter() {
        let config = RateLimitConfig::for_tier(AccountTier::Standard);
        let mut limiter = WeightedRateLimiter::new(config);

        assert!(limiter.can_perform(1));
        limiter.record(1);
        assert_eq!(limiter.remaining_capacity(), 1199);
    }

    #[test]
    fn test_subscription_message() {
        let sub = WsSubscription {
            subscription_type: WsSubscriptionType::OrderBook,
            market_id: Some(1),
        };
        let msg = sub.to_message().to_json();
        assert!(msg.contains("orderbook"));
        assert!(msg.contains("market_id"));
    }

    #[test]
    fn test_tier_rate_limits() {
        let standard = RateLimitConfig::for_tier(AccountTier::Standard);
        let premium = RateLimitConfig::for_tier(AccountTier::Premium);

        assert_eq!(standard.requests_per_minute, 1200);
        assert_eq!(premium.requests_per_minute, 6000);
    }
}
