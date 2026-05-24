#![allow(clippy::result_large_err)]

mod config;
mod error;
mod provider;
mod store;

pub use config::{RecoveryPassphrasePolicy, SecretScope, SecretsConfig};
pub use error::{
    secret_error, PROTECT_FAILED, SECRETS_COMPONENT, UNPROTECT_FAILED, UNSUPPORTED_PLATFORM,
    WEAK_PASSPHRASE,
};
pub use provider::{
    create_portable_key_provider, generate_secret_key, load_portable_key_provider,
    ProtectedKeyProvider,
};
pub use store::{
    PassphraseSecretProtector, ProtectedSecret, SecretProtector, PORTABLE_PASSPHRASE_BACKEND,
};

#[cfg(windows)]
pub use store::DpapiSecretProtector;
