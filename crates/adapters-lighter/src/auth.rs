use crate::config::LighterError;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashMap;

pub struct LighterSigner {
    api_key: String,
    api_secret: Vec<u8>,
}

impl LighterSigner {
    pub fn new(api_key: String, api_secret: String) -> Result<Self, LighterError> {
        let secret_bytes = hex::decode(&api_secret)
            .or_else(|_| api_secret.as_bytes().to_vec().pipe(Ok))
            .map_err(|e: std::convert::Infallible| {
                LighterError::SigningError(format!("{:?}", e))
            })?;

        Ok(Self {
            api_key,
            api_secret: secret_bytes,
        })
    }

    pub fn sign_request(
        &self,
        method: &str,
        path: &str,
        body: &str,
    ) -> Result<HashMap<String, String>, LighterError> {
        let timestamp = Self::current_timestamp_ms();
        let nonce = Self::generate_nonce();

        let message = format!("{}{}{}{}{}", timestamp, nonce, method, path, body);

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(&self.api_secret)
            .map_err(|e| LighterError::SigningError(e.to_string()))?;
        mac.update(message.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        let mut headers = HashMap::new();
        headers.insert("X-API-KEY".to_string(), self.api_key.clone());
        headers.insert("X-TIMESTAMP".to_string(), timestamp.to_string());
        headers.insert("X-NONCE".to_string(), nonce);
        headers.insert("X-SIGNATURE".to_string(), signature);
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        Ok(headers)
    }

    fn current_timestamp_ms() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    fn generate_nonce() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("{:x}", ts)
    }
}

trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
    }
}

impl<T> Pipe for T {}
