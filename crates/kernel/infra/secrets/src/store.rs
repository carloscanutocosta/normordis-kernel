use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use support_crypto::{
    decrypt_bytes_with_passphrase, encrypt_bytes_with_passphrase, EncryptedPayload,
};
use support_errors::MiniError;
use zeroize::Zeroizing;

use crate::config::RecoveryPassphrasePolicy;
#[cfg(windows)]
use crate::config::SecretsConfig;
use crate::error::{secret_error, PROTECT_FAILED, UNPROTECT_FAILED, WEAK_PASSPHRASE};

pub const PORTABLE_PASSPHRASE_BACKEND: &str = "portable-passphrase-v1";
#[cfg(windows)]
pub const WINDOWS_DPAPI_BACKEND: &str = "windows-dpapi";

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectedSecret {
    pub version: u8,
    pub backend: String,
    pub ciphertext_b64: String,
}

impl std::fmt::Debug for ProtectedSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProtectedSecret")
            .field("version", &self.version)
            .field("backend", &self.backend)
            .field("ciphertext_b64", &"[REDACTED]")
            .finish()
    }
}

impl ProtectedSecret {
    pub fn new(backend: impl Into<String>, ciphertext: &[u8]) -> Self {
        Self {
            version: 1,
            backend: backend.into(),
            ciphertext_b64: STANDARD_NO_PAD.encode(ciphertext),
        }
    }

    pub fn ciphertext(&self) -> Result<Vec<u8>, MiniError> {
        STANDARD_NO_PAD.decode(&self.ciphertext_b64).map_err(|_| {
            secret_error(
                UNPROTECT_FAILED,
                "failed to decode protected secret payload",
            )
        })
    }
}

pub trait SecretProtector: Send + Sync + 'static {
    fn protect(&self, plaintext: &[u8]) -> Result<ProtectedSecret, MiniError>;
    fn unprotect(&self, protected: &ProtectedSecret) -> Result<Zeroizing<Vec<u8>>, MiniError>;
}

pub struct PassphraseSecretProtector {
    passphrase: Zeroizing<String>,
}

impl PassphraseSecretProtector {
    pub fn new(passphrase: impl Into<String>) -> Result<Self, MiniError> {
        Self::with_policy(passphrase, RecoveryPassphrasePolicy::default())
    }

    pub fn with_policy(
        passphrase: impl Into<String>,
        policy: RecoveryPassphrasePolicy,
    ) -> Result<Self, MiniError> {
        let passphrase = passphrase.into();
        validate_recovery_passphrase(&passphrase, policy)?;

        Ok(Self {
            passphrase: Zeroizing::new(passphrase),
        })
    }
}

impl std::fmt::Debug for PassphraseSecretProtector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PassphraseSecretProtector")
            .field("passphrase", &"[REDACTED]")
            .finish()
    }
}

impl SecretProtector for PassphraseSecretProtector {
    fn protect(&self, plaintext: &[u8]) -> Result<ProtectedSecret, MiniError> {
        let payload =
            encrypt_bytes_with_passphrase(plaintext, &self.passphrase, Some(portable_aad()))
                .map_err(|_| secret_error(PROTECT_FAILED, "failed to protect secret"))?;
        let json = serde_json::to_vec(&payload)
            .map_err(|_| secret_error(PROTECT_FAILED, "failed to protect secret"))?;

        Ok(ProtectedSecret::new(PORTABLE_PASSPHRASE_BACKEND, &json))
    }

    fn unprotect(&self, protected: &ProtectedSecret) -> Result<Zeroizing<Vec<u8>>, MiniError> {
        if protected.version != 1 || protected.backend != PORTABLE_PASSPHRASE_BACKEND {
            return Err(secret_error(UNPROTECT_FAILED, "failed to unprotect secret"));
        }

        let bytes = protected.ciphertext()?;
        let payload: EncryptedPayload = serde_json::from_slice(&bytes)
            .map_err(|_| secret_error(UNPROTECT_FAILED, "failed to unprotect secret"))?;
        let plaintext =
            decrypt_bytes_with_passphrase(&payload, &self.passphrase, Some(portable_aad()))
                .map_err(|_| secret_error(UNPROTECT_FAILED, "failed to unprotect secret"))?;

        Ok(Zeroizing::new(plaintext))
    }
}

fn portable_aad() -> &'static [u8] {
    b"mini-kernel:v1:secrets:portable-key"
}

fn validate_recovery_passphrase(
    passphrase: &str,
    policy: RecoveryPassphrasePolicy,
) -> Result<(), MiniError> {
    if passphrase.trim().chars().count() < policy.min_chars {
        return Err(secret_error(
            WEAK_PASSPHRASE,
            "recovery passphrase does not meet minimum policy",
        ));
    }

    Ok(())
}

#[cfg(windows)]
#[derive(Debug, Clone)]
pub struct DpapiSecretProtector {
    config: SecretsConfig,
}

#[cfg(windows)]
impl DpapiSecretProtector {
    pub fn new(config: SecretsConfig) -> Self {
        Self { config }
    }
}

#[cfg(windows)]
impl Default for DpapiSecretProtector {
    fn default() -> Self {
        Self::new(SecretsConfig::default())
    }
}

#[cfg(windows)]
impl SecretProtector for DpapiSecretProtector {
    fn protect(&self, plaintext: &[u8]) -> Result<ProtectedSecret, MiniError> {
        windows::protect(plaintext, self.config)
    }

    fn unprotect(&self, protected: &ProtectedSecret) -> Result<Zeroizing<Vec<u8>>, MiniError> {
        windows::unprotect(protected)
    }
}

#[cfg(not(windows))]
#[derive(Debug, Clone, Default)]
pub struct UnsupportedSecretProtector;

#[cfg(not(windows))]
impl SecretProtector for UnsupportedSecretProtector {
    fn protect(&self, _plaintext: &[u8]) -> Result<ProtectedSecret, MiniError> {
        Err(secret_error(
            crate::error::UNSUPPORTED_PLATFORM,
            "secret protection is not supported on this platform",
        ))
    }

    fn unprotect(&self, _protected: &ProtectedSecret) -> Result<Zeroizing<Vec<u8>>, MiniError> {
        Err(secret_error(
            crate::error::UNSUPPORTED_PLATFORM,
            "secret protection is not supported on this platform",
        ))
    }
}

#[cfg(windows)]
mod windows {
    use std::ptr::{null, null_mut};
    use std::slice;

    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_LOCAL_MACHINE,
        CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };
    use zeroize::Zeroizing;

    use super::ProtectedSecret;
    use crate::config::{SecretScope, SecretsConfig};
    use crate::error::{secret_error, PROTECT_FAILED, UNPROTECT_FAILED};

    pub fn protect(
        plaintext: &[u8],
        config: SecretsConfig,
    ) -> Result<ProtectedSecret, support_errors::MiniError> {
        let input = CRYPT_INTEGER_BLOB {
            cbData: plaintext
                .len()
                .try_into()
                .map_err(|_| secret_error(PROTECT_FAILED, "failed to protect secret"))?,
            pbData: plaintext.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: null_mut(),
        };
        let flags = dpapi_flags(config);

        let ok = unsafe {
            CryptProtectData(
                &input,
                null(),
                null_mut(),
                null_mut(),
                null_mut(),
                flags,
                &mut output,
            )
        };

        if ok == 0 {
            return Err(secret_error(PROTECT_FAILED, "failed to protect secret"));
        }

        let protected = unsafe { blob_to_vec(&output) };
        unsafe {
            LocalFree(output.pbData as *mut _);
        }

        Ok(ProtectedSecret::new(
            super::WINDOWS_DPAPI_BACKEND,
            &protected,
        ))
    }

    pub fn unprotect(
        protected: &ProtectedSecret,
    ) -> Result<Zeroizing<Vec<u8>>, support_errors::MiniError> {
        if protected.version != 1 || protected.backend != super::WINDOWS_DPAPI_BACKEND {
            return Err(secret_error(UNPROTECT_FAILED, "failed to unprotect secret"));
        }

        let ciphertext = protected.ciphertext()?;
        let input = CRYPT_INTEGER_BLOB {
            cbData: ciphertext
                .len()
                .try_into()
                .map_err(|_| secret_error(UNPROTECT_FAILED, "failed to unprotect secret"))?,
            pbData: ciphertext.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: null_mut(),
        };

        let ok = unsafe {
            CryptUnprotectData(
                &input,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };

        if ok == 0 {
            return Err(secret_error(UNPROTECT_FAILED, "failed to unprotect secret"));
        }

        let plaintext = unsafe { blob_to_vec(&output) };
        unsafe {
            LocalFree(output.pbData as *mut _);
        }

        Ok(Zeroizing::new(plaintext))
    }

    fn dpapi_flags(config: SecretsConfig) -> u32 {
        let mut flags = CRYPTPROTECT_UI_FORBIDDEN;
        if config.scope == SecretScope::LocalMachine {
            flags |= CRYPTPROTECT_LOCAL_MACHINE;
        }
        flags
    }

    unsafe fn blob_to_vec(blob: &CRYPT_INTEGER_BLOB) -> Vec<u8> {
        slice::from_raw_parts(blob.pbData, blob.cbData as usize).to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Default)]
    struct EchoProtector;

    impl SecretProtector for EchoProtector {
        fn protect(&self, plaintext: &[u8]) -> Result<ProtectedSecret, MiniError> {
            Ok(ProtectedSecret::new("test-echo", plaintext))
        }

        fn unprotect(&self, protected: &ProtectedSecret) -> Result<Zeroizing<Vec<u8>>, MiniError> {
            if protected.backend != "test-echo" {
                return Err(secret_error(UNPROTECT_FAILED, "failed to unprotect secret"));
            }
            Ok(Zeroizing::new(protected.ciphertext()?))
        }
    }

    #[test]
    fn protected_secret_roundtrips_ciphertext_encoding() {
        let protected = ProtectedSecret::new("test", b"secret");

        assert_eq!(protected.ciphertext().unwrap(), b"secret");
    }

    #[test]
    fn protected_secret_debug_redacts_ciphertext() {
        let protected = ProtectedSecret::new("test", b"secret");
        let debug = format!("{protected:?}");

        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains(&protected.ciphertext_b64));
    }

    #[test]
    fn custom_protector_roundtrips_for_contract_tests() {
        let protector = EchoProtector;
        let protected = protector.protect(b"secret").unwrap();
        let plaintext = protector.unprotect(&protected).unwrap();

        assert_eq!(&*plaintext, b"secret");
    }

    #[test]
    fn passphrase_protector_roundtrips_portable_secret() {
        let protector = PassphraseSecretProtector::new("recovery-passphrase").unwrap();
        let protected = protector.protect(b"secret").unwrap();
        let other_instance = PassphraseSecretProtector::new("recovery-passphrase").unwrap();
        let plaintext = other_instance.unprotect(&protected).unwrap();

        assert_eq!(&*plaintext, b"secret");
        assert_eq!(protected.backend, PORTABLE_PASSPHRASE_BACKEND);
    }

    #[test]
    fn passphrase_protector_rejects_wrong_passphrase() {
        let protector = PassphraseSecretProtector::new("recovery-passphrase").unwrap();
        let protected = protector.protect(b"secret").unwrap();
        let other_instance = PassphraseSecretProtector::new("wrong-passphrase").unwrap();

        let err = other_instance.unprotect(&protected).unwrap_err();

        assert_eq!(err.to_public().code, UNPROTECT_FAILED);
    }

    #[test]
    fn passphrase_protector_debug_redacts_passphrase() {
        let protector = PassphraseSecretProtector::new("recovery-passphrase").unwrap();
        let debug = format!("{protector:?}");

        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("recovery-passphrase"));
    }

    #[test]
    fn passphrase_protector_rejects_weak_passphrase() {
        let err = PassphraseSecretProtector::new("short").unwrap_err();

        assert_eq!(err.to_public().code, WEAK_PASSPHRASE);
    }

    #[cfg(windows)]
    #[test]
    fn dpapi_protector_roundtrips_secret() {
        let protector = DpapiSecretProtector::default();
        let protected = protector.protect(b"secret").unwrap();
        let plaintext = protector.unprotect(&protected).unwrap();

        assert_eq!(&*plaintext, b"secret");
        assert_eq!(protected.backend, "windows-dpapi");
    }
}
