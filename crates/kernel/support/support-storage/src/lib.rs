mod codec;
mod error;
mod key;
mod memory;
mod namespace;
mod protector;
mod raw;
mod storage;
mod value;

pub use codec::{JsonStorageCodec, StorageCodec};
pub use error::{
    StorageError, BACKEND_FAILED, DESERIALIZATION_FAILED, INVALID_KEY, INVALID_NAMESPACE,
    OPERATION_FAILED, PROTECT_FAILED, SERIALIZATION_FAILED, STORAGE_COMPONENT, UNPROTECT_FAILED,
};
pub use key::StorageKey;
pub use memory::MemoryStorage;
pub use namespace::StorageNamespace;
pub use protector::CryptoStorageProtector;
pub use raw::RawStorage;
pub use storage::{ProtectedStorage, Storage, StorageProtector};
pub use value::StorageValue;
