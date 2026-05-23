use rand::RngCore;
use support_crypto::{KeyId, KeyProvider, KeyResolver, SecretKey, KEY_LENGTH_BYTES};
use support_errors::MiniError;

use crate::error::{secret_error, UNPROTECT_FAILED};
use crate::store::{PassphraseSecretProtector, ProtectedSecret, SecretProtector};

pub fn generate_secret_key() -> SecretKey {
    let mut key = [0_u8; KEY_LENGTH_BYTES];
    rand::thread_rng().fill_bytes(&mut key);
    SecretKey::new(key)
}

pub fn create_portable_key_provider(
    recovery_passphrase: impl Into<String>,
    key_id: KeyId,
) -> Result<
    (
        ProtectedSecret,
        ProtectedKeyProvider<PassphraseSecretProtector>,
    ),
    MiniError,
> {
    let protector = PassphraseSecretProtector::new(recovery_passphrase)?;
    let key = generate_secret_key();
    let protected_key = protector.protect(&key.0)?;
    let provider = ProtectedKeyProvider::new(protector, protected_key.clone(), key_id);

    Ok((protected_key, provider))
}

pub fn load_portable_key_provider(
    recovery_passphrase: impl Into<String>,
    protected_key: ProtectedSecret,
    key_id: KeyId,
) -> Result<ProtectedKeyProvider<PassphraseSecretProtector>, MiniError> {
    let protector = PassphraseSecretProtector::new(recovery_passphrase)?;

    Ok(ProtectedKeyProvider::new(protector, protected_key, key_id))
}

#[derive(Debug)]
pub struct ProtectedKeyProvider<P> {
    protector: P,
    protected_key: ProtectedSecret,
    key_id: KeyId,
}

impl<P> ProtectedKeyProvider<P>
where
    P: SecretProtector,
{
    pub fn new(protector: P, protected_key: ProtectedSecret, key_id: KeyId) -> Self {
        Self {
            protector,
            protected_key,
            key_id,
        }
    }

    pub fn key_id(&self) -> &KeyId {
        &self.key_id
    }

    fn unprotect_key(&self) -> Result<SecretKey, MiniError> {
        let bytes = self.protector.unprotect(&self.protected_key)?;
        let key: [u8; KEY_LENGTH_BYTES] = bytes.as_slice().try_into().map_err(|_| {
            secret_error(UNPROTECT_FAILED, "failed to unprotect secret key material")
        })?;

        Ok(SecretKey::new(key))
    }
}

impl<P> KeyProvider for ProtectedKeyProvider<P>
where
    P: SecretProtector,
{
    fn current_key(&self) -> Result<SecretKey, MiniError> {
        self.unprotect_key()
    }
}

impl<P> KeyResolver for ProtectedKeyProvider<P>
where
    P: SecretProtector,
{
    fn key_for_id(&self, key_id: Option<&str>) -> Result<SecretKey, MiniError> {
        if let Some(requested) = key_id {
            if requested != self.key_id.as_str() {
                return Err(secret_error(
                    UNPROTECT_FAILED,
                    "failed to resolve requested secret key",
                ));
            }
        }

        self.unprotect_key()
    }
}

#[cfg(test)]
mod tests {
    use zeroize::Zeroizing;

    use super::*;
    use crate::store::{ProtectedSecret, SecretProtector};

    #[derive(Debug, Clone, Default)]
    struct EchoProtector;

    impl SecretProtector for EchoProtector {
        fn protect(&self, plaintext: &[u8]) -> Result<ProtectedSecret, MiniError> {
            Ok(ProtectedSecret::new("test-echo", plaintext))
        }

        fn unprotect(&self, protected: &ProtectedSecret) -> Result<Zeroizing<Vec<u8>>, MiniError> {
            Ok(Zeroizing::new(protected.ciphertext()?))
        }
    }

    #[test]
    fn protected_key_provider_returns_current_key() {
        let key = generate_secret_key();
        let protected = EchoProtector.protect(&key.0).unwrap();
        let provider =
            ProtectedKeyProvider::new(EchoProtector, protected, KeyId::new("main-v1").unwrap());

        let current = provider.current_key().unwrap();

        assert_eq!(current.0, key.0);
    }

    #[test]
    fn protected_key_provider_rejects_unexpected_key_id() {
        let key = generate_secret_key();
        let protected = EchoProtector.protect(&key.0).unwrap();
        let provider =
            ProtectedKeyProvider::new(EchoProtector, protected, KeyId::new("main-v1").unwrap());

        let err = provider.key_for_id(Some("other")).unwrap_err();

        assert_eq!(err.to_public().code, UNPROTECT_FAILED);
    }

    #[test]
    fn portable_key_provider_can_be_loaded_from_protected_secret() {
        let key_id = KeyId::new("main-v1").unwrap();
        let (protected, provider) =
            create_portable_key_provider("recovery-passphrase", key_id.clone()).unwrap();
        let current = provider.current_key().unwrap();

        let loaded = load_portable_key_provider("recovery-passphrase", protected, key_id).unwrap();
        let loaded_current = loaded.current_key().unwrap();

        assert_eq!(loaded_current.0, current.0);
    }

    #[test]
    fn portable_key_provider_rejects_weak_recovery_passphrase() {
        let err =
            create_portable_key_provider("short", KeyId::new("main-v1").unwrap()).unwrap_err();

        assert_eq!(err.to_public().code, crate::WEAK_PASSPHRASE);
    }
}
