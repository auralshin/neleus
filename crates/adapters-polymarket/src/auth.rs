use crate::{ApiCredentials, NonceManager, PolymarketConfig, PolymarketError};
use ethers_core::types::{
    transaction::eip712::{EIP712Domain, Eip712},
    H160, U256,
};
use ethers_signers::{LocalWallet, Signer as EthersSigner};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::str::FromStr;

type HmacSha256 = Hmac<Sha256>;

/// EIP-712 message for L1 authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClobAuthMessage {
    pub timestamp: u64,
    pub nonce: u64,
}

impl Eip712 for ClobAuthMessage {
    type Error = PolymarketError;

    fn type_hash() -> Result<[u8; 32], Self::Error> {
        Ok(ethers_core::utils::keccak256(
            "ClobAuth(uint256 timestamp,uint256 nonce)".as_bytes(),
        ))
    }

    fn struct_hash(&self) -> Result<[u8; 32], Self::Error> {
        use ethers_core::abi::{encode, Token};
        let type_hash = Self::type_hash()?;
        let tokens = vec![
            Token::Uint(U256::from_big_endian(&type_hash)),
            Token::Uint(U256::from(self.timestamp)),
            Token::Uint(U256::from(self.nonce)),
        ];
        Ok(ethers_core::utils::keccak256(&encode(&tokens)))
    }

    fn domain(&self) -> Result<EIP712Domain, Self::Error> {
        Ok(EIP712Domain {
            name: Some("ClobAuthDomain".to_string()),
            version: Some("1".to_string()),
            chain_id: Some(U256::from(137)), // Polygon mainnet
            verifying_contract: None,
            salt: None,
        })
    }
}

/// Authentication signer for Polymarket
#[allow(dead_code)]
pub struct PolymarketSigner {
    wallet: LocalWallet,
    nonce_manager: NonceManager,
    config: PolymarketConfig,
}

impl PolymarketSigner {
    pub fn new(config: PolymarketConfig) -> Result<Self, PolymarketError> {
        let private_key = config
            .private_key
            .as_ref()
            .ok_or(PolymarketError::MissingCredentials)?;

        let wallet = LocalWallet::from_str(private_key)
            .map_err(|e| PolymarketError::InvalidPrivateKey(e.to_string()))?;

        Ok(Self {
            wallet,
            nonce_manager: NonceManager::new(),
            config,
        })
    }

    /// Sign an EIP-712 message for L1 authentication
    pub async fn sign_l1_auth(&mut self) -> Result<(String, u64, u64), PolymarketError> {
        let timestamp = Self::current_timestamp();
        let nonce = self.nonce_manager.next_nonce();

        let message = ClobAuthMessage { timestamp, nonce };

        let signature = self
            .wallet
            .sign_typed_data(&message)
            .await
            .map_err(|e| PolymarketError::SigningError(e.to_string()))?;

        let sig_hex = format!("0x{}", hex::encode(signature.to_vec()));

        Ok((sig_hex, timestamp, nonce))
    }

    /// Sign an order (EIP-712 message for order)
    pub async fn sign_order(
        &self,
        order_data: &OrderData,
    ) -> Result<String, PolymarketError> {
        // Create EIP-712 typed data for the order
        let typed_data = self.create_order_typed_data(order_data)?;

        let signature = self
            .wallet
            .sign_typed_data(&typed_data)
            .await
            .map_err(|e| PolymarketError::SigningError(e.to_string()))?;

        Ok(format!("0x{}", hex::encode(signature.to_vec())))
    }

    fn create_order_typed_data(&self, order_data: &OrderData) -> Result<OrderTypedData, PolymarketError> {
        Ok(OrderTypedData {
            maker: order_data.maker.clone(),
            taker: order_data.taker.clone(),
            token_id: order_data.token_id.clone(),
            maker_amount: order_data.maker_amount.clone(),
            taker_amount: order_data.taker_amount.clone(),
            side: order_data.side.clone(),
            fee_rate_bps: order_data.fee_rate_bps.clone(),
            nonce: order_data.nonce.clone(),
            signer: order_data.signer.clone(),
            expiration: order_data.expiration.clone(),
        })
    }

    fn current_timestamp() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    pub fn get_address(&self) -> String {
        format!("{:?}", self.wallet.address())
    }
}

/// Order data structure for signing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderData {
    pub maker: String,
    pub taker: String,
    pub token_id: String,
    pub maker_amount: String,
    pub taker_amount: String,
    pub side: String,
    pub fee_rate_bps: String,
    pub nonce: String,
    pub signer: String,
    pub expiration: String,
}

/// EIP-712 typed data for orders
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderTypedData {
    pub maker: String,
    pub taker: String,
    pub token_id: String,
    pub maker_amount: String,
    pub taker_amount: String,
    pub side: String,
    pub fee_rate_bps: String,
    pub nonce: String,
    pub signer: String,
    pub expiration: String,
}

impl Eip712 for OrderTypedData {
    type Error = PolymarketError;

    fn type_hash() -> Result<[u8; 32], Self::Error> {
        Ok(ethers_core::utils::keccak256(
            "Order(address maker,address taker,uint256 tokenId,uint256 makerAmount,uint256 takerAmount,uint8 side,uint256 feeRateBps,uint256 nonce,address signer,uint256 expiration)"
                .as_bytes(),
        ))
    }

    fn struct_hash(&self) -> Result<[u8; 32], Self::Error> {
        use ethers_core::abi::{encode, Token};
        let type_hash = Self::type_hash()?;
        
        // Parse addresses
        let maker_addr = H160::from_str(&self.maker)
            .map_err(|e| PolymarketError::InvalidSignature(e.to_string()))?;
        let taker_addr = H160::from_str(&self.taker)
            .map_err(|e| PolymarketError::InvalidSignature(e.to_string()))?;
        let signer_addr = H160::from_str(&self.signer)
            .map_err(|e| PolymarketError::InvalidSignature(e.to_string()))?;

        let tokens = vec![
            Token::Uint(U256::from_big_endian(&type_hash)),
            Token::Address(maker_addr),
            Token::Address(taker_addr),
            Token::Uint(U256::from_dec_str(&self.token_id).unwrap_or_default()),
            Token::Uint(U256::from_dec_str(&self.maker_amount).unwrap_or_default()),
            Token::Uint(U256::from_dec_str(&self.taker_amount).unwrap_or_default()),
            Token::Uint(U256::from_dec_str(&self.side).unwrap_or_default()),
            Token::Uint(U256::from_dec_str(&self.fee_rate_bps).unwrap_or_default()),
            Token::Uint(U256::from_dec_str(&self.nonce).unwrap_or_default()),
            Token::Address(signer_addr),
            Token::Uint(U256::from_dec_str(&self.expiration).unwrap_or_default()),
        ];
        
        Ok(ethers_core::utils::keccak256(&encode(&tokens)))
    }

    fn domain(&self) -> Result<EIP712Domain, Self::Error> {
        Ok(EIP712Domain {
            name: Some("Polymarket CTF Exchange".to_string()),
            version: Some("1".to_string()),
            chain_id: Some(U256::from(137)),
            verifying_contract: None,
            salt: None,
        })
    }
}

/// L2 Authentication using HMAC-SHA256
pub struct L2Authenticator {
    api_key: String,
    api_secret: String,
    api_passphrase: String,
}

impl L2Authenticator {
    pub fn new(api_key: String, api_secret: String, api_passphrase: String) -> Self {
        Self {
            api_key,
            api_secret,
            api_passphrase,
        }
    }

    pub fn from_credentials(creds: &ApiCredentials) -> Self {
        Self {
            api_key: creds.api_key.clone(),
            api_secret: creds.secret.clone(),
            api_passphrase: creds.passphrase.clone(),
        }
    }

    /// Create HMAC signature for L2 authentication
    pub fn sign_request(
        &self,
        timestamp: u64,
        method: &str,
        path: &str,
        body: &str,
    ) -> Result<String, PolymarketError> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        
        // Message format: timestamp + method + path + body
        let message = format!("{}{}{}{}", timestamp, method, path, body);

        let mut mac = HmacSha256::new_from_slice(self.api_secret.as_bytes())
            .map_err(|e| PolymarketError::SigningError(e.to_string()))?;

        mac.update(message.as_bytes());
        let result = mac.finalize();
        let signature = STANDARD.encode(result.into_bytes());

        Ok(signature)
    }

    pub fn get_headers(
        &self,
        timestamp: u64,
        signature: &str,
        address: &str,
    ) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("POLY_ADDRESS".to_string(), address.to_string());
        headers.insert("POLY_SIGNATURE".to_string(), signature.to_string());
        headers.insert("POLY_TIMESTAMP".to_string(), timestamp.to_string());
        headers.insert("POLY_API_KEY".to_string(), self.api_key.clone());
        headers.insert("POLY_PASSPHRASE".to_string(), self.api_passphrase.clone());
        headers
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn api_passphrase(&self) -> &str {
        &self.api_passphrase
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l2_authenticator() {
        let auth = L2Authenticator::new(
            "test_key".to_string(),
            "test_secret".to_string(),
            "test_passphrase".to_string(),
        );

        let timestamp = 1234567890;
        let signature = auth
            .sign_request(timestamp, "GET", "/orders", "")
            .unwrap();

        assert!(!signature.is_empty());
    }
}
