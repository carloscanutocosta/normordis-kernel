use support_crypto::StorageEnvelope;

use crate::codec::StorageCodec;
use crate::error::StorageError;
use crate::key::StorageKey;
use crate::namespace::StorageNamespace;
use crate::raw::RawStorage;
use crate::value::StorageValue;

pub trait StorageProtector: Send + Sync {
    fn protect(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
        plaintext: &[u8],
    ) -> Result<StorageEnvelope, StorageError>;

    fn unprotect(&self, envelope: &StorageEnvelope) -> Result<Vec<u8>, StorageError>;
}

pub trait Storage: Send + Sync {
    fn put_json(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
        value: &StorageValue,
    ) -> Result<(), StorageError>;

    fn put_json_if_absent(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
        value: &StorageValue,
    ) -> Result<bool, StorageError>;

    fn get_json(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
    ) -> Result<Option<StorageValue>, StorageError>;

    fn delete(&self, namespace: &StorageNamespace, key: &StorageKey) -> Result<(), StorageError>;

    fn exists(&self, namespace: &StorageNamespace, key: &StorageKey) -> Result<bool, StorageError> {
        Ok(self.get_json(namespace, key)?.is_some())
    }
}

#[derive(Debug)]
pub struct ProtectedStorage<R, C, P>
where
    R: RawStorage,
    C: StorageCodec,
    P: StorageProtector,
{
    raw: R,
    codec: C,
    protector: P,
}

impl<R, C, P> ProtectedStorage<R, C, P>
where
    R: RawStorage,
    C: StorageCodec,
    P: StorageProtector,
{
    pub fn new(raw: R, codec: C, protector: P) -> Self {
        Self {
            raw,
            codec,
            protector,
        }
    }

    pub fn raw(&self) -> &R {
        &self.raw
    }
}

impl<R, C, P> Storage for ProtectedStorage<R, C, P>
where
    R: RawStorage,
    C: StorageCodec,
    P: StorageProtector,
{
    fn put_json(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
        value: &StorageValue,
    ) -> Result<(), StorageError> {
        let bytes = self.codec.encode(value)?;
        let envelope = self.protector.protect(namespace, key, &bytes)?;
        self.raw.put_raw(namespace, key, &envelope)
    }

    fn put_json_if_absent(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
        value: &StorageValue,
    ) -> Result<bool, StorageError> {
        let bytes = self.codec.encode(value)?;
        let envelope = self.protector.protect(namespace, key, &bytes)?;
        self.raw.put_raw_if_absent(namespace, key, &envelope)
    }

    fn get_json(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
    ) -> Result<Option<StorageValue>, StorageError> {
        let Some(envelope) = self.raw.get_raw(namespace, key)? else {
            return Ok(None);
        };
        let bytes = self.protector.unprotect(&envelope)?;
        self.codec.decode(&bytes).map(Some)
    }

    fn delete(&self, namespace: &StorageNamespace, key: &StorageKey) -> Result<(), StorageError> {
        self.raw.delete_raw(namespace, key)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;

    use serde_json::json;
    use support_crypto::{SecretKey, StaticKeyProvider};

    use super::*;
    use crate::{CryptoStorageProtector, JsonStorageCodec, MemoryStorage, RawStorage};

    fn storage(
    ) -> ProtectedStorage<MemoryStorage, JsonStorageCodec, CryptoStorageProtector<StaticKeyProvider>>
    {
        let keys =
            StaticKeyProvider::new(SecretKey::new([7; support_crypto::KEY_LENGTH_BYTES]), None);
        ProtectedStorage::new(
            MemoryStorage::default(),
            JsonStorageCodec,
            CryptoStorageProtector::new(keys),
        )
    }

    #[test]
    fn protected_storage_writes_and_reads_json() {
        let storage = storage();
        let namespace = StorageNamespace::new("documents").unwrap();
        let key = StorageKey::new("doc-1").unwrap();
        let value = json!({"title":"hello"});

        storage.put_json(&namespace, &key, &value).unwrap();

        assert_eq!(storage.get_json(&namespace, &key).unwrap(), Some(value));
    }

    #[test]
    fn delete_removes_value() {
        let storage = storage();
        let namespace = StorageNamespace::new("documents").unwrap();
        let key = StorageKey::new("doc-1").unwrap();

        storage
            .put_json(&namespace, &key, &json!({"title":"hello"}))
            .unwrap();
        storage.delete(&namespace, &key).unwrap();

        assert_eq!(storage.get_json(&namespace, &key).unwrap(), None);
    }

    #[test]
    fn namespaces_are_isolated() {
        let storage = storage();
        let a = StorageNamespace::new("documents").unwrap();
        let b = StorageNamespace::new("audit").unwrap();
        let key = StorageKey::new("same-key").unwrap();

        storage.put_json(&a, &key, &json!({"ns":"a"})).unwrap();
        storage.put_json(&b, &key, &json!({"ns":"b"})).unwrap();

        assert_eq!(storage.get_json(&a, &key).unwrap(), Some(json!({"ns":"a"})));
        assert_eq!(storage.get_json(&b, &key).unwrap(), Some(json!({"ns":"b"})));
    }

    #[test]
    fn overwrite_replaces_value() {
        let storage = storage();
        let namespace = StorageNamespace::new("documents").unwrap();
        let key = StorageKey::new("doc-1").unwrap();

        storage.put_json(&namespace, &key, &json!({"v":1})).unwrap();
        storage.put_json(&namespace, &key, &json!({"v":2})).unwrap();

        assert_eq!(
            storage.get_json(&namespace, &key).unwrap(),
            Some(json!({"v":2}))
        );
    }

    #[test]
    fn put_json_if_absent_does_not_overwrite_existing_value() {
        let storage = storage();
        let namespace = StorageNamespace::new("documents").unwrap();
        let key = StorageKey::new("doc-1").unwrap();

        assert!(storage
            .put_json_if_absent(&namespace, &key, &json!({"v":1}))
            .unwrap());
        assert!(!storage
            .put_json_if_absent(&namespace, &key, &json!({"v":2}))
            .unwrap());

        assert_eq!(
            storage.get_json(&namespace, &key).unwrap(),
            Some(json!({"v":1}))
        );
    }

    #[test]
    fn complex_json_is_preserved() {
        let storage = storage();
        let namespace = StorageNamespace::new("documents").unwrap();
        let key = StorageKey::new("doc-1").unwrap();
        let value = json!({
            "title": "hello",
            "tags": ["a", "b"],
            "meta": {"active": true, "score": 12.5},
            "empty": null
        });

        storage.put_json(&namespace, &key, &value).unwrap();

        assert_eq!(storage.get_json(&namespace, &key).unwrap(), Some(value));
    }

    #[test]
    fn multiple_threads_write_different_keys() {
        let storage = Arc::new(storage());
        let namespace = StorageNamespace::new("documents").unwrap();
        let mut handles = Vec::new();

        for index in 0..16 {
            let storage = Arc::clone(&storage);
            let namespace = namespace.clone();
            handles.push(thread::spawn(move || {
                let key = StorageKey::new(format!("doc-{index}")).unwrap();
                storage
                    .put_json(&namespace, &key, &json!({"index": index}))
                    .unwrap();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        for index in 0..16 {
            let key = StorageKey::new(format!("doc-{index}")).unwrap();
            assert_eq!(
                storage.get_json(&namespace, &key).unwrap(),
                Some(json!({"index": index}))
            );
        }
    }

    #[derive(Debug)]
    struct FailingRawStorage;

    impl RawStorage for FailingRawStorage {
        fn put_raw(
            &self,
            _namespace: &StorageNamespace,
            _key: &StorageKey,
            _envelope: &StorageEnvelope,
        ) -> Result<(), StorageError> {
            Err(StorageError::BackendFailed)
        }

        fn get_raw(
            &self,
            _namespace: &StorageNamespace,
            _key: &StorageKey,
        ) -> Result<Option<StorageEnvelope>, StorageError> {
            Err(StorageError::BackendFailed)
        }

        fn delete_raw(
            &self,
            _namespace: &StorageNamespace,
            _key: &StorageKey,
        ) -> Result<(), StorageError> {
            Err(StorageError::BackendFailed)
        }
    }

    #[test]
    fn backend_error_converts_to_storage_error() {
        let keys =
            StaticKeyProvider::new(SecretKey::new([7; support_crypto::KEY_LENGTH_BYTES]), None);
        let storage = ProtectedStorage::new(
            FailingRawStorage,
            JsonStorageCodec,
            CryptoStorageProtector::new(keys),
        );
        let namespace = StorageNamespace::new("documents").unwrap();
        let key = StorageKey::new("doc-1").unwrap();

        let err = storage
            .put_json(&namespace, &key, &json!({"title":"hello"}))
            .unwrap_err();

        assert_eq!(err, StorageError::BackendFailed);
        assert_eq!(err.to_mini_error().to_public().code, crate::BACKEND_FAILED);
    }
}
