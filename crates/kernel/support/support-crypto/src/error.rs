use support_errors::{Component, ErrorCode, MiniError};
use thiserror::Error;

use crate::constants::{
    COMPONENT, DECRYPT_FAILED, EMPTY_PASSPHRASE, ENCRYPT_FAILED, INVALID_AAD, INVALID_KDF,
    INVALID_KEY, INVALID_PAYLOAD, POLICY_VIOLATION,
};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CryptoError {
    #[error("passphrase vazia")]
    EmptyPassphrase,
    #[error("payload com versao nao suportada: {0}")]
    UnsupportedVersion(u8),
    #[error("algoritmo de cifragem nao suportado: {0}")]
    UnsupportedAlgorithm(String),
    #[error("algoritmo de derivacao nao suportado: {0}")]
    UnsupportedKdf(String),
    #[error("salt invalido")]
    InvalidSalt,
    #[error("nonce invalido")]
    InvalidNonce,
    #[error("chave invalida")]
    InvalidKey,
    #[error("identificador de chave invalido")]
    InvalidKeyId,
    #[error("payload base64 invalido")]
    InvalidEncoding,
    #[error("falha de cifragem autenticada")]
    EncryptionFailed,
    #[error("falha de decifragem autenticada")]
    DecryptionFailed,
    #[error("configuracao Argon2 invalida")]
    InvalidKdfConfig,
    #[error("AAD de storage invalido")]
    InvalidAad,
    #[error("politica criptografica violada")]
    PolicyViolation,
}

impl CryptoError {
    pub fn to_mini_error(&self) -> MiniError {
        let code = match self {
            Self::EmptyPassphrase => EMPTY_PASSPHRASE,
            Self::InvalidKey | Self::InvalidKeyId => INVALID_KEY,
            Self::UnsupportedKdf(_) | Self::InvalidKdfConfig | Self::InvalidSalt => INVALID_KDF,
            Self::InvalidAad => INVALID_AAD,
            Self::PolicyViolation => POLICY_VIOLATION,
            Self::EncryptionFailed => ENCRYPT_FAILED,
            Self::DecryptionFailed => DECRYPT_FAILED,
            Self::UnsupportedVersion(_)
            | Self::UnsupportedAlgorithm(_)
            | Self::InvalidNonce
            | Self::InvalidEncoding => INVALID_PAYLOAD,
        };

        MiniError::new(
            ErrorCode::new(code).expect("support-crypto error codes must be valid"),
            Component::new(COMPONENT).expect("support-crypto component must be valid"),
            public_message_for_code(code),
        )
    }
}

impl From<CryptoError> for MiniError {
    fn from(value: CryptoError) -> Self {
        value.to_mini_error()
    }
}

fn public_message_for_code(code: &str) -> &'static str {
    match code {
        EMPTY_PASSPHRASE => "crypto passphrase is invalid",
        INVALID_KEY => "crypto key is invalid",
        INVALID_KDF => "crypto key derivation configuration is invalid",
        ENCRYPT_FAILED => "failed to encrypt payload",
        DECRYPT_FAILED => "failed to decrypt payload",
        INVALID_AAD => "storage encryption context is invalid",
        POLICY_VIOLATION => "crypto policy requirements were not met",
        _ => "encrypted payload is invalid",
    }
}
