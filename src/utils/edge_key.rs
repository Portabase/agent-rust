use base64::{Engine as _, engine::general_purpose};
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;
use tracing::error;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct EdgeKey {
    #[serde(rename = "serverUrl")]
    pub server_url: String,
    #[serde(rename = "agentId")]
    pub agent_id: String,
    #[serde(rename = "publicKey")]
    pub public_key: String,
}

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum EdgeKeyError {
    #[error("Base64 decoding error: {0}")]
    Base64Error(#[from] base64::DecodeError),
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("EDGE_KEY INVALID")]
    InvalidKey,
}

pub fn decode_edge_key(edge_key: &str) -> Result<EdgeKey, EdgeKeyError> {
    let mut key = edge_key.to_string();
    let padding_needed = (4 - key.len() % 4) % 4;
    key.push_str(&"=".repeat(padding_needed));

    let decoded_bytes = general_purpose::URL_SAFE.decode(&key)?;
    let decoded_str = String::from_utf8_lossy(&decoded_bytes);

    let parsed: Value = serde_json::from_str(&decoded_str)?;
    if parsed.get("serverUrl").is_some()
        && parsed.get("agentId").is_some()
        && parsed.get("publicKey").is_some()
    {
        let edge_key: EdgeKey = serde_json::from_value(parsed)?;
        Ok(edge_key)
    } else {
        error!("EDGE_KEY INVALID");
        Err(EdgeKeyError::InvalidKey)
    }
}
