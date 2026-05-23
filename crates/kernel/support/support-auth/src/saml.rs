use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Metadata técnica de Service Provider SAML (v2).
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlMetadata {
    pub entity_id: String,
    pub acs_url: String,
    pub certificate_pem: String,
}

/// Pedido de autenticação SAML construído pelo SP (v2).
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlAuthnRequest {
    pub id: String,
    pub issuer: String,
    pub destination: String,
    pub relay_state: String,
    pub issued_at: DateTime<Utc>,
}

/// Envelope técnico mínimo de uma resposta SAML recebida (v2).
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlAssertionEnvelope {
    pub in_response_to: String,
    pub response_issuer: String,
    pub assertion_issuer: String,
    pub audience: String,
    pub subject: String,
    pub name_id: String,
    pub session_index: String,
    pub status_code: String,
    pub recipient: String,
    pub attributes: HashMap<String, Vec<String>>,
    pub not_before: DateTime<Utc>,
    pub not_on_or_after: DateTime<Utc>,
    pub issued_at: DateTime<Utc>,
}

/// Principal técnico normalizado após validação SAML (v2).
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlPrincipal {
    pub subject: String,
    pub name_id: String,
    pub audience: String,
    pub session_index: String,
    pub attributes: HashMap<String, Vec<String>>,
}
