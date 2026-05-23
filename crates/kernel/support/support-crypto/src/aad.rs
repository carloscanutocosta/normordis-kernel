use serde::{Deserialize, Serialize};

use crate::constants::STORAGE_AAD_PREFIX;
use crate::error::CryptoError;
use crate::payload::EncryptedPayload;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageAad {
    pub namespace: String,
    pub record_id: String,
    pub field: String,
}

impl StorageAad {
    pub fn new(
        namespace: impl Into<String>,
        record_id: impl Into<String>,
        field: impl Into<String>,
    ) -> Result<Self, CryptoError> {
        let aad = Self {
            namespace: namespace.into(),
            record_id: record_id.into(),
            field: field.into(),
        };
        aad.validate()?;
        Ok(aad)
    }

    pub fn canonical(&self) -> String {
        format!(
            "{STORAGE_AAD_PREFIX}:{}:{}:{}",
            self.namespace, self.record_id, self.field
        )
    }

    pub fn aad_bytes(&self) -> Vec<u8> {
        self.canonical().into_bytes()
    }

    fn validate(&self) -> Result<(), CryptoError> {
        validate_segment(&self.namespace)?;
        validate_segment(&self.record_id)?;
        validate_segment(&self.field)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageEnvelope {
    pub aad: StorageAad,
    pub payload: EncryptedPayload,
}

impl StorageEnvelope {
    pub fn new(aad: StorageAad, payload: EncryptedPayload) -> Self {
        Self { aad, payload }
    }

    pub fn aad_bytes(&self) -> Vec<u8> {
        self.aad.aad_bytes()
    }
}

pub(crate) fn validate_segment(value: &str) -> Result<(), CryptoError> {
    if value.is_empty() || value.chars().any(char::is_whitespace) || value.contains(':') {
        return Err(CryptoError::InvalidAad);
    }

    Ok(())
}
