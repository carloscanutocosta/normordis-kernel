use support_crypto::StorageEnvelope;

use crate::error::StorageError;
use crate::key::StorageKey;
use crate::namespace::StorageNamespace;

pub trait RawStorage: Send + Sync {
    fn put_raw(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
        envelope: &StorageEnvelope,
    ) -> Result<(), StorageError>;

    fn put_raw_if_absent(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
        envelope: &StorageEnvelope,
    ) -> Result<bool, StorageError> {
        if self.get_raw(namespace, key)?.is_some() {
            return Ok(false);
        }
        self.put_raw(namespace, key, envelope)?;
        Ok(true)
    }

    fn get_raw(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
    ) -> Result<Option<StorageEnvelope>, StorageError>;

    fn delete_raw(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
    ) -> Result<(), StorageError>;
}
