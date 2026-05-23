use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pedido técnico de bind a um directório LDAP.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdapBindRequest {
    pub dn: String,
    pub password: String,
}

/// Pedido técnico de pesquisa LDAP.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdapSearchRequest {
    pub base_dn: String,
    pub filter: String,
    pub attributes: Vec<String>,
}

/// Entrada LDAP devolvida por um cliente concreto.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdapEntry {
    pub dn: String,
    pub attributes: HashMap<String, Vec<String>>,
}

/// Principal técnico normalizado a partir do directório.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdapPrincipal {
    pub dn: String,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub groups: Vec<String>,
    pub attributes: HashMap<String, Vec<String>>,
}

/// Plano técnico de autenticação LDAP (v2).
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdapAuthenticationPlan {
    pub service_bind: LdapBindRequest,
    pub user_search: LdapSearchRequest,
    pub user_password: String,
}
