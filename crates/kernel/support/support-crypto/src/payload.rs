use serde::{Deserialize, Serialize};

use crate::constants::{CURRENT_KDF, EXTERNAL_KEY};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdfConfig {
    pub algorithm: String,
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
    pub salt_b64: String,
}

impl KdfConfig {
    pub fn external_key() -> Self {
        Self {
            algorithm: EXTERNAL_KEY.to_string(),
            memory_kib: 0,
            iterations: 0,
            parallelism: 0,
            salt_b64: String::new(),
        }
    }
}

impl Default for KdfConfig {
    fn default() -> Self {
        Self {
            algorithm: CURRENT_KDF.to_string(),
            memory_kib: 19_456,
            iterations: 2,
            parallelism: 1,
            salt_b64: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedPayload {
    pub version: u8,
    pub algorithm: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
    pub nonce_b64: String,
    pub ciphertext_b64: String,
    pub kdf: KdfConfig,
}
