use std::collections::HashMap;
use std::sync::RwLock;

use support_crypto::StorageEnvelope;

use crate::error::StorageError;
use crate::key::StorageKey;
use crate::namespace::StorageNamespace;
use crate::raw::RawStorage;

#[derive(Debug, Default)]
pub struct MemoryStorage {
    inner: RwLock<HashMap<String, HashMap<String, StorageEnvelope>>>,
}

impl MemoryStorage {
    pub fn len(&self) -> Result<usize, StorageError> {
        let inner = self.inner.read().map_err(|_| StorageError::BackendFailed)?;
        Ok(inner.values().map(HashMap::len).sum())
    }

    pub fn is_empty(&self) -> Result<bool, StorageError> {
        Ok(self.len()? == 0)
    }
}

impl RawStorage for MemoryStorage {
    fn put_raw(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
        envelope: &StorageEnvelope,
    ) -> Result<(), StorageError> {
        let mut inner = self
            .inner
            .write()
            .map_err(|_| StorageError::BackendFailed)?;
        inner
            .entry(namespace.as_str().to_string())
            .or_default()
            .insert(key.as_str().to_string(), envelope.clone());
        Ok(())
    }

    fn put_raw_if_absent(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
        envelope: &StorageEnvelope,
    ) -> Result<bool, StorageError> {
        let mut inner = self
            .inner
            .write()
            .map_err(|_| StorageError::BackendFailed)?;
        let items = inner.entry(namespace.as_str().to_string()).or_default();
        if items.contains_key(key.as_str()) {
            return Ok(false);
        }
        items.insert(key.as_str().to_string(), envelope.clone());
        Ok(true)
    }

    fn get_raw(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
    ) -> Result<Option<StorageEnvelope>, StorageError> {
        let inner = self.inner.read().map_err(|_| StorageError::BackendFailed)?;
        Ok(inner
            .get(namespace.as_str())
            .and_then(|items| items.get(key.as_str()))
            .cloned())
    }

    fn delete_raw(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
    ) -> Result<(), StorageError> {
        let mut inner = self
            .inner
            .write()
            .map_err(|_| StorageError::BackendFailed)?;
        if let Some(items) = inner.get_mut(namespace.as_str()) {
            items.remove(key.as_str());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use support_crypto::{SecretKey, StaticKeyProvider};

    use super::*;
    use crate::CryptoStorageProtector;

    #[test]
    fn memory_storage_stores_envelope_not_plain_value() {
        let namespace = StorageNamespace::new("documents").unwrap();
        let key = StorageKey::new("doc-1").unwrap();
        let protector = CryptoStorageProtector::new(StaticKeyProvider::new(
            SecretKey::new([9; support_crypto::KEY_LENGTH_BYTES]),
            None,
        ));
        let envelope = protector
            .protect(&namespace, &key, br#"{"secret":"plain"}"#)
            .unwrap();
        let storage = MemoryStorage::default();

        storage.put_raw(&namespace, &key, &envelope).unwrap();
        let stored = storage.get_raw(&namespace, &key).unwrap().unwrap();

        assert_eq!(stored.aad, envelope.aad);
        assert_ne!(stored.payload.ciphertext_b64, r#"{"secret":"plain"}"#);
    }
}
