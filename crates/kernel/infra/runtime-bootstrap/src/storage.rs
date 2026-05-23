use std::sync::atomic::{AtomicBool, Ordering};

use adapter_sqlite::{SqliteConfig, SqliteRawStorage};
use core_config::{StorageBackend, StorageProfile};
use support_crypto::{KeyProvider, KeyResolver};
use support_storage::{
    CryptoStorageProtector, JsonStorageCodec, MemoryStorage, ProtectedStorage, Storage,
    StorageError, StorageKey, StorageNamespace, StorageValue,
};

use crate::error::RuntimeError;

pub type ProtectedSqliteStorage<P> =
    ProtectedStorage<SqliteRawStorage, JsonStorageCodec, CryptoStorageProtector<P>>;
pub type ProtectedMemoryStorage<P> =
    ProtectedStorage<MemoryStorage, JsonStorageCodec, CryptoStorageProtector<P>>;

pub enum RuntimeStorage<P>
where
    P: KeyProvider + KeyResolver + Send + Sync,
{
    Sqlite {
        storage: ProtectedSqliteStorage<P>,
        shutdown: AtomicBool,
    },
    Memory(ProtectedMemoryStorage<P>),
}

impl<P> RuntimeStorage<P>
where
    P: KeyProvider + KeyResolver + Send + Sync,
{
    pub fn open_sqlite_config(config: SqliteConfig, keys: P) -> Result<Self, RuntimeError> {
        let raw = SqliteRawStorage::open(config).map_err(|_| RuntimeError::RuntimeOpenFailed)?;
        Ok(Self::Sqlite {
            storage: ProtectedStorage::new(
                raw,
                JsonStorageCodec,
                CryptoStorageProtector::new(keys),
            ),
            shutdown: AtomicBool::new(false),
        })
    }

    pub fn open(profile: &StorageProfile, keys: P) -> Result<Self, RuntimeError> {
        match profile.backend {
            StorageBackend::Sqlite => {
                let path = profile
                    .database_path
                    .as_ref()
                    .ok_or(RuntimeError::InvalidStorageProfile)?;
                Self::open_sqlite_config(SqliteConfig::new(path), keys)
            }
            StorageBackend::Memory => Ok(Self::Memory(ProtectedStorage::new(
                MemoryStorage::default(),
                JsonStorageCodec,
                CryptoStorageProtector::new(keys),
            ))),
        }
    }

    pub fn shutdown(&self) -> Result<(), StorageError> {
        match self {
            Self::Sqlite { storage, shutdown } => {
                if shutdown.swap(true, Ordering::AcqRel) {
                    return Ok(());
                }
                storage.raw().shutdown()
            }
            Self::Memory(_) => Ok(()),
        }
    }
}

impl<P> Storage for RuntimeStorage<P>
where
    P: KeyProvider + KeyResolver + Send + Sync,
{
    fn put_json(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
        value: &StorageValue,
    ) -> Result<(), StorageError> {
        match self {
            Self::Sqlite { storage, .. } => storage.put_json(namespace, key, value),
            Self::Memory(storage) => storage.put_json(namespace, key, value),
        }
    }

    fn put_json_if_absent(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
        value: &StorageValue,
    ) -> Result<bool, StorageError> {
        match self {
            Self::Sqlite { storage, .. } => storage.put_json_if_absent(namespace, key, value),
            Self::Memory(storage) => storage.put_json_if_absent(namespace, key, value),
        }
    }

    fn get_json(
        &self,
        namespace: &StorageNamespace,
        key: &StorageKey,
    ) -> Result<Option<StorageValue>, StorageError> {
        match self {
            Self::Sqlite { storage, .. } => storage.get_json(namespace, key),
            Self::Memory(storage) => storage.get_json(namespace, key),
        }
    }

    fn delete(&self, namespace: &StorageNamespace, key: &StorageKey) -> Result<(), StorageError> {
        match self {
            Self::Sqlite { storage, .. } => storage.delete(namespace, key),
            Self::Memory(storage) => storage.delete(namespace, key),
        }
    }
}
