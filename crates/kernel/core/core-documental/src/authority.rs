//! Snapshot de autoridade jurídica — capturado imutavelmente no momento de
//! entrada em custódia para reconstituição histórica.
//!
//! `AuthoritySnapshot` armazena apenas primitivos (String, DateTime) — sem
//! dependência de tipos de outros bounded contexts. A conversão a partir de
//! tipos externos é feita em `service::authority_from_user_context` (ACL).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Snapshot imutável de autoridade jurídica.
///
/// Congela quem (utilizador), em que posição (cargo), na unidade orgânica,
/// com que competência e ao abrigo de que instrumento (por delegação ou não).
///
/// Todos os campos são primitivos — o snapshot não depende de tipos externos.
/// Para construir a partir de `core_rh::UserContext` usa-se `authority_from_user_context`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoritySnapshot {
    pub user_id: String,
    pub position_id: String,
    pub unit_id: String,
    pub competency_id: String,
    pub delegation_id: Option<String>,
    pub captured_at: DateTime<Utc>,
}
