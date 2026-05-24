use support_errors::MiniError;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::constants::KEY_LENGTH_BYTES;
use crate::error::CryptoError;

pub type KeyResult = Result<SecretKey, Box<MiniError>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyId(String);

impl KeyId {
    pub fn new(value: impl Into<String>) -> Result<Self, CryptoError> {
        let value = value.into();
        if value.is_empty() || value.chars().any(char::is_whitespace) || value.contains(':') {
            return Err(CryptoError::InvalidKeyId);
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SecretKey(pub [u8; KEY_LENGTH_BYTES]);

impl SecretKey {
    pub fn new(bytes: [u8; KEY_LENGTH_BYTES]) -> Self {
        Self(bytes)
    }
}

impl std::fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretKey([REDACTED])")
    }
}

pub trait KeyProvider {
    fn current_key(&self) -> KeyResult;
}

pub trait KeyResolver {
    fn key_for_id(&self, key_id: Option<&str>) -> KeyResult;
}

pub struct StaticKeyProvider {
    key_id: Option<KeyId>,
    key: [u8; KEY_LENGTH_BYTES],
}

impl StaticKeyProvider {
    pub fn new(key: SecretKey, key_id: Option<KeyId>) -> Self {
        Self { key_id, key: key.0 }
    }

    pub fn key_id(&self) -> Option<&KeyId> {
        self.key_id.as_ref()
    }
}

impl std::fmt::Debug for StaticKeyProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StaticKeyProvider")
            .field("key_id", &self.key_id)
            .field("key", &"[REDACTED]")
            .finish()
    }
}

impl Drop for StaticKeyProvider {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

impl KeyProvider for StaticKeyProvider {
    fn current_key(&self) -> KeyResult {
        Ok(SecretKey::new(self.key))
    }
}

impl KeyResolver for StaticKeyProvider {
    fn key_for_id(&self, key_id: Option<&str>) -> KeyResult {
        if let (Some(expected), Some(requested)) = (self.key_id.as_ref(), key_id) {
            if expected.as_str() != requested {
                return Err(Box::new(CryptoError::InvalidKey.to_mini_error()));
            }
        }

        Ok(SecretKey::new(self.key))
    }
}
