use serde::{Deserialize, Serialize};

/// Challenge técnico WebAuthn (v2).
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnChallenge {
    pub id: String,
    pub value: String,
    pub user_id: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Relying party técnico (v2).
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnRelyingParty {
    pub id: String,
    pub name: String,
}

/// Utilizador técnico num pedido de registo WebAuthn (v2).
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnUser {
    pub id: String,
    pub name: String,
    pub display_name: String,
}

/// Descritor de credencial WebAuthn (v2).
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnCredentialDescriptor {
    pub type_: String,
    pub id: String,
    pub transports: Vec<String>,
}

/// Opções técnicas de registo WebAuthn (v2).
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnRegistrationOptions {
    pub challenge: WebAuthnChallenge,
    pub relying_party: WebAuthnRelyingParty,
    pub user: WebAuthnUser,
    pub timeout_millis: u64,
    pub attestation: String,
    pub exclude_credentials: Vec<WebAuthnCredentialDescriptor>,
}

/// Opções técnicas de autenticação WebAuthn (v2).
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnAuthenticationOptions {
    pub challenge: WebAuthnChallenge,
    pub relying_party_id: String,
    pub timeout_millis: u64,
    pub user_verification: String,
    pub allow_credentials: Vec<WebAuthnCredentialDescriptor>,
}
