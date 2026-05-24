mod aad;
mod constants;
mod crypto;
mod error;
mod key;
mod payload;
mod policy;

pub use aad::{StorageAad, StorageEnvelope};
pub use constants::{
    COMPONENT, CURRENT_ALGORITHM, CURRENT_CRYPTO_VERSION, CURRENT_KDF, DECRYPT_FAILED,
    EMPTY_PASSPHRASE, ENCRYPT_FAILED, EXTERNAL_KEY, INVALID_AAD, INVALID_KDF, INVALID_KEY,
    INVALID_PAYLOAD, KEY_LENGTH_BYTES, POLICY_VIOLATION, STORAGE_AAD_PREFIX,
};
pub use crypto::{
    decrypt_bytes_with_key, decrypt_bytes_with_passphrase, decrypt_text_with_key,
    decrypt_text_with_passphrase, derive_key_from_passphrase, encrypt_bytes_with_key,
    encrypt_bytes_with_passphrase, encrypt_text_with_key, encrypt_text_with_passphrase,
    validate_encrypted_payload,
};
pub use error::CryptoError;
pub use key::{KeyId, KeyProvider, KeyResolver, KeyResult, SecretKey, StaticKeyProvider};
pub use payload::{EncryptedPayload, KdfConfig};
pub use policy::CryptoPolicy;
