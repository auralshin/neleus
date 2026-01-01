#[derive(Debug, Clone)]
pub struct LighterConfig {
    pub ws_url: String,
    pub rest_url: String,
}

impl LighterConfig {
    pub fn mainnet() -> Self {
        Self {
            ws_url: "wss://mainnet.zklighter.elliot.ai/stream".to_string(),
            rest_url: "https://mainnet.zklighter.elliot.ai".to_string(),
        }
    }

    pub fn testnet() -> Self {
        Self {
            ws_url: "wss://testnet.zklighter.elliot.ai/stream".to_string(),
            rest_url: "https://testnet.zklighter.elliot.ai".to_string(),
        }
    }
}

pub struct LighterAdapter {
    pub config: LighterConfig,
}

impl LighterAdapter {
    pub fn new(config: LighterConfig) -> Self {
        Self { config }
    }
}
