use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{Key, KeyInit, XChaCha20Poly1305, XNonce};
use rand::RngCore;
use zeroize::Zeroizing;

use crate::constants::{
    CURRENT_ALGORITHM, CURRENT_CRYPTO_VERSION, CURRENT_KDF, EXTERNAL_KEY, KEY_LENGTH_BYTES,
    NONCE_LENGTH_BYTES, SALT_LENGTH_BYTES,
};
use crate::error::CryptoError;
use crate::key::{KeyId, SecretKey};
use crate::payload::{EncryptedPayload, KdfConfig};

pub fn encrypt_bytes_with_passphrase(
    plaintext: &[u8],
    passphrase: &str,
    aad: Option<&[u8]>,
) -> Result<EncryptedPayload, CryptoError> {
    if passphrase.trim().is_empty() {
        return Err(CryptoError::EmptyPassphrase);
    }

    let mut salt = [0_u8; SALT_LENGTH_BYTES];
    let mut nonce = [0_u8; NONCE_LENGTH_BYTES];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce);

    let kdf = KdfConfig {
        salt_b64: encode_base64(&salt),
        ..KdfConfig::default()
    };
    let key = derive_key_from_passphrase(passphrase, &kdf)?;
    encrypt_bytes_with_key_and_material(plaintext, &key, aad, None, nonce, kdf)
}

pub fn encrypt_bytes_with_key(
    plaintext: &[u8],
    key: &SecretKey,
    aad: Option<&[u8]>,
    key_id: Option<&KeyId>,
) -> Result<EncryptedPayload, CryptoError> {
    let mut nonce = [0_u8; NONCE_LENGTH_BYTES];
    rand::thread_rng().fill_bytes(&mut nonce);
    encrypt_bytes_with_key_and_material(
        plaintext,
        key,
        aad,
        key_id.map(|value| value.as_str().to_string()),
        nonce,
        KdfConfig::external_key(),
    )
}

pub fn decrypt_bytes_with_passphrase(
    payload: &EncryptedPayload,
    passphrase: &str,
    aad: Option<&[u8]>,
) -> Result<Vec<u8>, CryptoError> {
    if passphrase.trim().is_empty() {
        return Err(CryptoError::EmptyPassphrase);
    }
    validate_encrypted_payload(payload)?;
    if payload.kdf.algorithm != CURRENT_KDF {
        return Err(CryptoError::UnsupportedKdf(payload.kdf.algorithm.clone()));
    }
    let key = derive_key_from_passphrase(passphrase, &payload.kdf)?;
    decrypt_bytes_with_key(payload, &key, aad)
}

pub fn decrypt_bytes_with_key(
    payload: &EncryptedPayload,
    key: &SecretKey,
    aad: Option<&[u8]>,
) -> Result<Vec<u8>, CryptoError> {
    validate_encrypted_payload(payload)?;

    let nonce = decode_base64(&payload.nonce_b64)?;
    if nonce.len() != NONCE_LENGTH_BYTES {
        return Err(CryptoError::InvalidNonce);
    }
    let ciphertext = decode_base64(&payload.ciphertext_b64)?;
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&key.0));
    cipher
        .decrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &ciphertext,
                aad: aad.unwrap_or_default(),
            },
        )
        .map_err(|_| CryptoError::DecryptionFailed)
}

pub fn encrypt_text_with_passphrase(
    plaintext: &str,
    passphrase: &str,
    aad: Option<&[u8]>,
) -> Result<EncryptedPayload, CryptoError> {
    encrypt_bytes_with_passphrase(plaintext.as_bytes(), passphrase, aad)
}

pub fn encrypt_text_with_key(
    plaintext: &str,
    key: &SecretKey,
    aad: Option<&[u8]>,
    key_id: Option<&KeyId>,
) -> Result<EncryptedPayload, CryptoError> {
    encrypt_bytes_with_key(plaintext.as_bytes(), key, aad, key_id)
}

pub fn decrypt_text_with_passphrase(
    payload: &EncryptedPayload,
    passphrase: &str,
    aad: Option<&[u8]>,
) -> Result<String, CryptoError> {
    let bytes = decrypt_bytes_with_passphrase(payload, passphrase, aad)?;
    String::from_utf8(bytes).map_err(|_| CryptoError::DecryptionFailed)
}

pub fn decrypt_text_with_key(
    payload: &EncryptedPayload,
    key: &SecretKey,
    aad: Option<&[u8]>,
) -> Result<String, CryptoError> {
    let bytes = decrypt_bytes_with_key(payload, key, aad)?;
    String::from_utf8(bytes).map_err(|_| CryptoError::DecryptionFailed)
}

pub fn derive_key_from_passphrase(
    passphrase: &str,
    config: &KdfConfig,
) -> Result<SecretKey, CryptoError> {
    if passphrase.trim().is_empty() {
        return Err(CryptoError::EmptyPassphrase);
    }
    if config.algorithm != CURRENT_KDF {
        return Err(CryptoError::UnsupportedKdf(config.algorithm.clone()));
    }

    let salt = decode_base64(&config.salt_b64)?;
    if salt.len() != SALT_LENGTH_BYTES {
        return Err(CryptoError::InvalidSalt);
    }

    let params = Params::new(
        config.memory_kib,
        config.iterations,
        config.parallelism,
        Some(KEY_LENGTH_BYTES),
    )
    .map_err(|_| CryptoError::InvalidKdfConfig)?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0_u8; KEY_LENGTH_BYTES]);
    argon2
        .hash_password_into(passphrase.as_bytes(), &salt, &mut *key)
        .map_err(|_| CryptoError::InvalidKdfConfig)?;
    Ok(SecretKey::new(*key))
}

fn encrypt_bytes_with_key_and_material(
    plaintext: &[u8],
    key: &SecretKey,
    aad: Option<&[u8]>,
    key_id: Option<String>,
    nonce: [u8; NONCE_LENGTH_BYTES],
    kdf: KdfConfig,
) -> Result<EncryptedPayload, CryptoError> {
    if let Some(value) = key_id.as_deref() {
        KeyId::new(value.to_string())?;
    }

    let cipher = XChaCha20Poly1305::new(Key::from_slice(&key.0));
    let ciphertext = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad: aad.unwrap_or_default(),
            },
        )
        .map_err(|_| CryptoError::EncryptionFailed)?;

    Ok(EncryptedPayload {
        version: CURRENT_CRYPTO_VERSION,
        algorithm: CURRENT_ALGORITHM.to_string(),
        key_id,
        nonce_b64: encode_base64(&nonce),
        ciphertext_b64: encode_base64(&ciphertext),
        kdf,
    })
}

pub fn validate_encrypted_payload(payload: &EncryptedPayload) -> Result<(), CryptoError> {
    if payload.version != CURRENT_CRYPTO_VERSION {
        return Err(CryptoError::UnsupportedVersion(payload.version));
    }
    if payload.algorithm != CURRENT_ALGORITHM {
        return Err(CryptoError::UnsupportedAlgorithm(payload.algorithm.clone()));
    }
    if let Some(key_id) = payload.key_id.as_deref() {
        KeyId::new(key_id.to_string())?;
    }

    match payload.kdf.algorithm.as_str() {
        CURRENT_KDF => {
            let salt = decode_base64(&payload.kdf.salt_b64)?;
            if salt.len() != SALT_LENGTH_BYTES {
                return Err(CryptoError::InvalidSalt);
            }
        }
        EXTERNAL_KEY => {
            if !payload.kdf.salt_b64.is_empty()
                || payload.kdf.memory_kib != 0
                || payload.kdf.iterations != 0
                || payload.kdf.parallelism != 0
            {
                return Err(CryptoError::InvalidKdfConfig);
            }
        }
        _ => return Err(CryptoError::UnsupportedKdf(payload.kdf.algorithm.clone())),
    }

    let nonce = decode_base64(&payload.nonce_b64)?;
    if nonce.len() != NONCE_LENGTH_BYTES {
        return Err(CryptoError::InvalidNonce);
    }
    decode_base64(&payload.ciphertext_b64)?;
    Ok(())
}

fn encode_base64(bytes: &[u8]) -> String {
    STANDARD_NO_PAD.encode(bytes)
}

fn decode_base64(value: &str) -> Result<Vec<u8>, CryptoError> {
    STANDARD_NO_PAD
        .decode(value)
        .map_err(|_| CryptoError::InvalidEncoding)
}
