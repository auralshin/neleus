use crate::config::HyperliquidError;
use crate::types::{
    CancelRequest, HyperliquidOrderRequest, HyperliquidOrderTypeRequest, HyperliquidTif,
};
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize)]
pub struct Eip712Domain {
    pub name: String,
    pub version: String,
    pub chain_id: u64,
    pub verifying_contract: String,
}

impl Eip712Domain {
    pub fn hyperliquid_mainnet() -> Self {
        Self {
            name: "Exchange".to_string(),
            version: "1".to_string(),
            chain_id: 1337,
            verifying_contract: "0x0000000000000000000000000000000000000000".to_string(),
        }
    }

    pub fn hyperliquid_testnet() -> Self {
        Self {
            name: "Exchange".to_string(),
            version: "1".to_string(),
            chain_id: 1337,
            verifying_contract: "0x0000000000000000000000000000000000000000".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum SignableAction {
    #[serde(rename = "order")]
    Order {
        orders: Vec<OrderWire>,
        grouping: String,
    },
    #[serde(rename = "cancel")]
    Cancel { cancels: Vec<CancelRequest> },
    #[serde(rename = "cancelByCloid")]
    CancelByCloid { cancels: Vec<CloidCancel> },
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderWire {
    #[serde(rename = "a")]
    pub asset: u32,
    #[serde(rename = "b")]
    pub is_buy: bool,
    #[serde(rename = "p")]
    pub limit_px: String,
    #[serde(rename = "s")]
    pub sz: String,
    #[serde(rename = "r")]
    pub reduce_only: bool,
    #[serde(rename = "t")]
    pub order_type: OrderTypeWire,
    #[serde(rename = "c", skip_serializing_if = "Option::is_none")]
    pub cloid: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderTypeWire {
    pub limit: LimitOrderType,
}

#[derive(Debug, Clone, Serialize)]
pub struct LimitOrderType {
    pub tif: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CloidCancel {
    pub asset: u32,
    pub cloid: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SignedRequest {
    pub action: SignableAction,
    pub nonce: u64,
    pub signature: Signature,
    #[serde(rename = "vaultAddress", skip_serializing_if = "Option::is_none")]
    pub vault_address: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Signature {
    pub r: String,
    pub s: String,
    pub v: u8,
}

pub struct HyperliquidSigner {
    private_key: [u8; 32],
    address: String,
    domain: Eip712Domain,
}

impl HyperliquidSigner {
    pub fn new(private_key_hex: &str, testnet: bool) -> Result<Self, HyperliquidError> {
        let key_hex = private_key_hex
            .strip_prefix("0x")
            .unwrap_or(private_key_hex);
        let key_bytes =
            hex::decode(key_hex).map_err(|e| HyperliquidError::InvalidPrivateKey(e.to_string()))?;

        if key_bytes.len() != 32 {
            return Err(HyperliquidError::InvalidPrivateKey(
                "Private key must be 32 bytes".to_string(),
            ));
        }

        let mut private_key = [0u8; 32];
        private_key.copy_from_slice(&key_bytes);

        let address = Self::derive_address(&private_key);

        let domain = if testnet {
            Eip712Domain::hyperliquid_testnet()
        } else {
            Eip712Domain::hyperliquid_mainnet()
        };

        Ok(Self {
            private_key,
            address,
            domain,
        })
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    fn derive_address(private_key: &[u8; 32]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(private_key);
        let hash = hasher.finalize();
        format!("0x{}", hex::encode(&hash[..20]))
    }

    pub fn hash_action(&self, action: &SignableAction, nonce: u64) -> [u8; 32] {
        let action_json = serde_json::to_string(action).unwrap_or_default();

        let mut hasher = Sha256::new();
        hasher.update(self.domain.name.as_bytes());
        hasher.update(self.domain.version.as_bytes());
        hasher.update(&self.domain.chain_id.to_be_bytes());
        hasher.update(action_json.as_bytes());
        hasher.update(&nonce.to_be_bytes());

        let mut result = [0u8; 32];
        result.copy_from_slice(&hasher.finalize());
        result
    }

    pub fn sign(&self, action: &SignableAction, nonce: u64) -> Result<Signature, HyperliquidError> {
        let hash = self.hash_action(action, nonce);

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(&self.private_key)
            .map_err(|e| HyperliquidError::SigningError(e.to_string()))?;
        mac.update(&hash);
        let sig = mac.finalize().into_bytes();

        Ok(Signature {
            r: hex::encode(&sig[..32]),
            s: hex::encode(&hash[..32]),
            v: 27,
        })
    }

    pub fn sign_request(
        &self,
        action: SignableAction,
        nonce: u64,
        vault_address: Option<String>,
    ) -> Result<SignedRequest, HyperliquidError> {
        let signature = self.sign(&action, nonce)?;
        Ok(SignedRequest {
            action,
            nonce,
            signature,
            vault_address,
        })
    }
}

// Helper to convert order request to wire format
pub fn order_to_wire(request: &HyperliquidOrderRequest, asset: u32) -> OrderWire {
    let tif_str = match &request.order_type {
        HyperliquidOrderTypeRequest::Limit { tif } => match tif {
            HyperliquidTif::Gtc => "Gtc",
            HyperliquidTif::Ioc => "Ioc",
            HyperliquidTif::Alo => "Alo",
        },
        HyperliquidOrderTypeRequest::Trigger { .. } => "Gtc",
    };

    OrderWire {
        asset,
        is_buy: request.is_buy,
        limit_px: format!("{:.5}", request.limit_px),
        sz: format!("{:.8}", request.sz),
        reduce_only: request.reduce_only,
        order_type: OrderTypeWire {
            limit: LimitOrderType {
                tif: tif_str.to_string(),
            },
        },
        cloid: request.cloid.clone(),
    }
}
