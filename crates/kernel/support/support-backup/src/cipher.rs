use support_crypto::{decrypt_bytes_with_passphrase, encrypt_bytes_with_passphrase, EncryptedPayload};

use crate::error::BackupError;

pub fn encrypt(data: &[u8], passphrase: &str) -> Result<Vec<u8>, BackupError> {
    let payload = encrypt_bytes_with_passphrase(data, passphrase, None)
        .map_err(|_| BackupError::EncryptFailed)?;
    serde_json::to_vec(&payload).map_err(|_| BackupError::EncryptFailed)
}

pub fn decrypt(data: &[u8], passphrase: &str) -> Result<Vec<u8>, BackupError> {
    let payload: EncryptedPayload =
        serde_json::from_slice(data).map_err(|_| BackupError::DecryptFailed)?;
    decrypt_bytes_with_passphrase(&payload, passphrase, None)
        .map_err(|_| BackupError::DecryptFailed)
}
