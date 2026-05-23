use crate::aad::StorageEnvelope;
use crate::error::CryptoError;
use crate::payload::EncryptedPayload;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoPolicy {
    pub require_aad: bool,
    pub require_key_id: bool,
}

impl CryptoPolicy {
    pub fn permissive() -> Self {
        Self {
            require_aad: false,
            require_key_id: false,
        }
    }

    pub fn storage_default() -> Self {
        Self {
            require_aad: true,
            require_key_id: true,
        }
    }

    pub fn validate_payload(&self, payload: &EncryptedPayload) -> Result<(), CryptoError> {
        if self.require_key_id && payload.key_id.is_none() {
            return Err(CryptoError::PolicyViolation);
        }

        Ok(())
    }

    pub fn validate_storage_envelope(&self, envelope: &StorageEnvelope) -> Result<(), CryptoError> {
        self.validate_payload(&envelope.payload)?;

        if self.require_aad && envelope.aad.aad_bytes().is_empty() {
            return Err(CryptoError::PolicyViolation);
        }

        Ok(())
    }
}
