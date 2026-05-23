use support_crypto::{
    decrypt_bytes_with_key, encrypt_bytes_with_key, KeyProvider, KeyResolver, StorageAad,
    StorageEnvelope,
};

use crate::error::StorageError;
use crate::key::StorageKey;
use crate::namespace::StorageNamespace;
use crate::storage::StorageProtector;

#[derive(Debug)]
pub struct CryptoStorageProtector<P>
where
    P: KeyProvider + KeyResolver + Send + Sync,
{
    keys: P,
}

impl<P> CryptoStorageProtector<P>
where
    P: KeyProvider + KeyResolver + Send + Sync,
{
    pub fn new(keys: P) -> Self {
        Self { keys }
    }

    pub fn protect(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
        plaintext: &[u8],
    ) -> Result<StorageEnvelope, StorageError> {
        let aad = StorageAad::new(namespace.as_str(), key.as_str(), "value")
            .map_err(|_| StorageError::ProtectFailed)?;
        let aad_bytes = aad.aad_bytes();
        let secret_key = self
            .keys
            .current_key()
            .map_err(|_| StorageError::ProtectFailed)?;
        let payload = encrypt_bytes_with_key(plaintext, &secret_key, Some(&aad_bytes), None)
            .map_err(|_| StorageError::ProtectFailed)?;

        Ok(StorageEnvelope::new(aad, payload))
    }

    pub fn unprotect(&self, envelope: &StorageEnvelope) -> Result<Vec<u8>, StorageError> {
        let key = self
            .keys
            .key_for_id(envelope.payload.key_id.as_deref())
            .map_err(|_| StorageError::UnprotectFailed)?;
        let aad_bytes = envelope.aad_bytes();

        decrypt_bytes_with_key(&envelope.payload, &key, Some(&aad_bytes))
            .map_err(|_| StorageError::UnprotectFailed)
    }
}

impl<P> StorageProtector for CryptoStorageProtector<P>
where
    P: KeyProvider + KeyResolver + Send + Sync,
{
    fn protect(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
        plaintext: &[u8],
    ) -> Result<StorageEnvelope, StorageError> {
        self.protect(namespace, key, plaintext)
    }

    fn unprotect(&self, envelope: &StorageEnvelope) -> Result<Vec<u8>, StorageError> {
        self.unprotect(envelope)
    }
}

#[cfg(test)]
mod tests {
    use support_crypto::{SecretKey, StaticKeyProvider};

    use super::*;

    fn protector() -> CryptoStorageProtector<StaticKeyProvider> {
        CryptoStorageProtector::new(StaticKeyProvider::new(
            SecretKey::new([42; support_crypto::KEY_LENGTH_BYTES]),
            None,
        ))
    }

    #[test]
    fn crypto_storage_protector_encrypts_and_decrypts() {
        let namespace = StorageNamespace::new("documents").unwrap();
        let key = StorageKey::new("doc-1").unwrap();
        let envelope = protector()
            .protect(&namespace, &key, br#"{"secret":true}"#)
            .unwrap();

        let plaintext = protector().unprotect(&envelope).unwrap();

        assert_eq!(plaintext, br#"{"secret":true}"#);
        assert_ne!(envelope.payload.ciphertext_b64, r#"{"secret":true}"#);
    }

    #[test]
    fn storage_envelope_contains_expected_aad() {
        let namespace = StorageNamespace::new("documents").unwrap();
        let key = StorageKey::new("doc-1").unwrap();
        let envelope = protector().protect(&namespace, &key, b"secret").unwrap();

        assert_eq!(envelope.aad.namespace, "documents");
        assert_eq!(envelope.aad.record_id, "doc-1");
        assert_eq!(envelope.aad.field, "value");
    }

    #[test]
    fn crypto_error_converts_to_storage_error() {
        let namespace = StorageNamespace::new("documents").unwrap();
        let key = StorageKey::new("doc-1").unwrap();
        let mut envelope = protector().protect(&namespace, &key, b"secret").unwrap();
        envelope.payload.ciphertext_b64.push('A');

        let err = protector().unprotect(&envelope).unwrap_err();

        assert_eq!(err, StorageError::UnprotectFailed);
        assert_eq!(
            err.to_mini_error().to_public().code,
            crate::UNPROTECT_FAILED
        );
    }
}
