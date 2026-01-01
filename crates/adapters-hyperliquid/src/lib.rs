#[derive(Debug, Clone)]
pub struct HyperliquidConfig {
    pub ws_url: String,
    pub rest_url: String,
}

impl HyperliquidConfig {
    pub fn mainnet() -> Self {
        Self {
            ws_url: "wss://api.hyperliquid.xyz/ws".to_string(),
            rest_url: "https://api.hyperliquid.xyz".to_string(),
        }
    }

    pub fn testnet() -> Self {
        Self {
            ws_url: "wss://api.hyperliquid-testnet.xyz/ws".to_string(),
            rest_url: "https://api.hyperliquid-testnet.xyz".to_string(),
        }
    }
}

pub struct HyperliquidAdapter {
    pub config: HyperliquidConfig,
}

impl HyperliquidAdapter {
    pub fn new(config: HyperliquidConfig) -> Self {
        Self { config }
    }
}
