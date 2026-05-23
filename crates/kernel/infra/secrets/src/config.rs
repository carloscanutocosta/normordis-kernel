#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretScope {
    CurrentUser,
    LocalMachine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecretsConfig {
    pub scope: SecretScope,
}

impl SecretsConfig {
    pub fn current_user() -> Self {
        Self {
            scope: SecretScope::CurrentUser,
        }
    }

    pub fn local_machine() -> Self {
        Self {
            scope: SecretScope::LocalMachine,
        }
    }
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self::current_user()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryPassphrasePolicy {
    pub min_chars: usize,
}

impl RecoveryPassphrasePolicy {
    pub fn production_default() -> Self {
        Self { min_chars: 16 }
    }
}

impl Default for RecoveryPassphrasePolicy {
    fn default() -> Self {
        Self::production_default()
    }
}
