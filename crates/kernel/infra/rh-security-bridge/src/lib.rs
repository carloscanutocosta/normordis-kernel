//! Bridge entre `core-rh` e `core-security`: pertença a roles com validade temporal.
//!
//! ## Responsabilidade
//!
//! `core-security` define o port `RoleMembershipRepository`.
//! Este crate implementa-o usando SQLite, com uma tabela própria gerida pelo
//! administrador de segurança — independente da estrutura de cargos/contratos do `core-rh`.
//!
//! ## Integração com SecurityService
//!
//! ```no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use core_security::{NoopSecurityAuditLog, SecurityService};
//! use rh_security_bridge::RhSecurityBridgeStore;
//! use adapter_sqlite::SqliteRelationalConfig;
//!
//! let repo = security_sqlite::SecuritySqliteStore::open(
//!     &SqliteRelationalConfig::read_write_create("security.db")
//! )?;
//! let roles = RhSecurityBridgeStore::open(
//!     &SqliteRelationalConfig::read_write_create("roles.db")
//! )?;
//!
//! let svc = SecurityService::with_all(repo, NoopSecurityAuditLog, roles);
//! # Ok(())
//! # }
//! ```

use std::sync::{Arc, Mutex};

use adapter_sqlite::{
    open_relational_connection, run_relational_migrations, SqliteRelationalConfig,
};
use chrono::{DateTime, Utc};
use core_security::{RoleId, RoleMembershipRepository, SecurityError};
use rusqlite::params;
use rusqlite::Connection;
use thiserror::Error;
use uuid::Uuid;

// ── Migration ─────────────────────────────────────────────────────────────────

const MIGRATION_1: &str = r#"
    CREATE TABLE IF NOT EXISTS security_role_members (
        member_id    TEXT PRIMARY KEY,
        role_id      TEXT NOT NULL,
        principal_id TEXT NOT NULL,
        assigned_by  TEXT NOT NULL,
        assigned_at  TEXT NOT NULL,
        valid_from   TEXT NOT NULL,
        valid_to     TEXT,
        revoked      INTEGER NOT NULL DEFAULT 0,
        revoked_at   TEXT,
        revoked_by   TEXT
    );

    CREATE INDEX IF NOT EXISTS idx_srm_principal
        ON security_role_members (principal_id, valid_from, revoked);
    CREATE INDEX IF NOT EXISTS idx_srm_role
        ON security_role_members (role_id, valid_from, revoked);
"#;

pub const RH_SECURITY_BRIDGE_MIGRATIONS: &[&str] = &[MIGRATION_1];

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum RhSecurityBridgeError {
    #[error(transparent)]
    Adapter(#[from] support_errors::MiniError),
    #[error("erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("membro não encontrado: {0}")]
    MemberNotFound(String),
}

impl From<RhSecurityBridgeError> for SecurityError {
    fn from(e: RhSecurityBridgeError) -> Self {
        SecurityError::OperationFailed(e.to_string())
    }
}

// ── MemberId ──────────────────────────────────────────────────────────────────

/// Identificador de uma atribuição de role a um principal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberId(pub String);

impl MemberId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ── Store ─────────────────────────────────────────────────────────────────────

/// Adapter SQLite para `RoleMembershipRepository`.
///
/// Gere membros com validade temporal (`valid_from` / `valid_to`).
/// O administrador de segurança controla quem tem que role e quando.
///
/// ## Threading
///
/// `Arc<Mutex<Connection>>` — é `Clone`, `Send` e `Sync`.
#[derive(Clone)]
pub struct RhSecurityBridgeStore {
    conn: Arc<Mutex<Connection>>,
}

impl RhSecurityBridgeStore {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, RhSecurityBridgeError> {
        let conn = open_relational_connection(config)?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn from_connection(conn: Connection) -> Result<Self, RhSecurityBridgeError> {
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn migrate(&self) -> Result<(), RhSecurityBridgeError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| RhSecurityBridgeError::Sqlite(rusqlite::Error::InvalidQuery))?;
        run_relational_migrations(&conn, RH_SECURITY_BRIDGE_MIGRATIONS)?;
        Ok(())
    }

    /// Atribui `principal_id` ao `role_id` com validade temporal.
    ///
    /// `valid_to = None` significa sem expiração.
    /// Devolve o `MemberId` gerado para posterior revogação pontual.
    pub fn assign_principal_to_role(
        &self,
        principal_id: &str,
        role_id: &RoleId,
        assigned_by: &str,
        valid_from: DateTime<Utc>,
        valid_to: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> Result<MemberId, RhSecurityBridgeError> {
        let member_id = MemberId(Uuid::new_v4().to_string());
        let conn = self
            .conn
            .lock()
            .map_err(|_| RhSecurityBridgeError::Sqlite(rusqlite::Error::InvalidQuery))?;
        conn.execute(
            "INSERT INTO security_role_members
                 (member_id, role_id, principal_id, assigned_by, assigned_at,
                  valid_from, valid_to, revoked)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
            params![
                member_id.as_str(),
                role_id.as_str(),
                principal_id,
                assigned_by,
                dt_to_str(now),
                dt_to_str(valid_from),
                valid_to.map(dt_to_str),
            ],
        )?;
        Ok(member_id)
    }

    /// Revoga uma atribuição de membro.
    ///
    /// Falha com `MemberNotFound` se a atribuição não existir ou já estiver revogada.
    pub fn revoke_membership(
        &self,
        member_id: &MemberId,
        revoked_by: &str,
        now: DateTime<Utc>,
    ) -> Result<(), RhSecurityBridgeError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| RhSecurityBridgeError::Sqlite(rusqlite::Error::InvalidQuery))?;
        let affected = conn.execute(
            "UPDATE security_role_members
             SET revoked = 1, revoked_at = ?1, revoked_by = ?2
             WHERE member_id = ?3 AND revoked = 0",
            params![dt_to_str(now), revoked_by, member_id.as_str()],
        )?;
        if affected == 0 {
            return Err(RhSecurityBridgeError::MemberNotFound(
                member_id.as_str().into(),
            ));
        }
        Ok(())
    }

    /// Lista os roles activos do `principal_id` no momento `now`.
    pub fn list_principal_roles(
        &self,
        principal_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Vec<RoleId>, RhSecurityBridgeError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| RhSecurityBridgeError::Sqlite(rusqlite::Error::InvalidQuery))?;
        let now_s = dt_to_str(now);
        let mut stmt = conn.prepare(
            "SELECT DISTINCT role_id FROM security_role_members
             WHERE principal_id = ?1
               AND revoked = 0
               AND valid_from <= ?2
               AND (valid_to IS NULL OR valid_to > ?2)
             ORDER BY role_id",
        )?;
        let roles = stmt
            .query_map(params![principal_id, now_s], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(roles.into_iter().map(RoleId).collect())
    }

    /// Lista os membros activos do `role_id` no momento `now`.
    pub fn list_role_members(
        &self,
        role_id: &RoleId,
        now: DateTime<Utc>,
    ) -> Result<Vec<String>, RhSecurityBridgeError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| RhSecurityBridgeError::Sqlite(rusqlite::Error::InvalidQuery))?;
        let now_s = dt_to_str(now);
        let mut stmt = conn.prepare(
            "SELECT DISTINCT principal_id FROM security_role_members
             WHERE role_id = ?1
               AND revoked = 0
               AND valid_from <= ?2
               AND (valid_to IS NULL OR valid_to > ?2)
             ORDER BY principal_id",
        )?;
        let members = stmt
            .query_map(params![role_id.as_str(), now_s], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(members)
    }
}

// ── RoleMembershipRepository ──────────────────────────────────────────────────

impl RoleMembershipRepository for RhSecurityBridgeStore {
    async fn get_roles_for_principal(
        &self,
        principal_id: &str,
        now: DateTime<Utc>,
    ) -> Result<Vec<RoleId>, SecurityError> {
        self.list_principal_roles(principal_id, now)
            .map_err(SecurityError::from)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn dt_to_str(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use adapter_sqlite::SqliteRelationalConfig;
    use chrono::TimeZone;
    use core_security::RoleMembershipRepository;
    use tempfile::NamedTempFile;

    fn test_store() -> RhSecurityBridgeStore {
        let tmp = NamedTempFile::new().unwrap();
        RhSecurityBridgeStore::open(&SqliteRelationalConfig::read_write_create(tmp.path())).unwrap()
    }

    fn role(id: &str) -> RoleId {
        RoleId::new(id).unwrap()
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 15, 10, 0, 0).unwrap()
    }

    fn later() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 12, 31, 23, 59, 59).unwrap()
    }

    #[tokio::test]
    async fn assign_e_get_roles() {
        let store = test_store();
        let n = now();
        store
            .assign_principal_to_role("user:alice", &role("role:editor"), "admin", n, None, n)
            .unwrap();
        let roles = store
            .get_roles_for_principal("user:alice", n)
            .await
            .unwrap();
        assert_eq!(roles.len(), 1);
        assert_eq!(roles[0].as_str(), "role:editor");
    }

    #[tokio::test]
    async fn roles_expiram_por_valid_to() {
        let store = test_store();
        let n = now();
        let expiry = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
        store
            .assign_principal_to_role(
                "user:bob",
                &role("role:revisor"),
                "admin",
                n,
                Some(expiry),
                n,
            )
            .unwrap();

        let roles_before = store.get_roles_for_principal("user:bob", n).await.unwrap();
        assert_eq!(roles_before.len(), 1);

        let after = Utc.with_ymd_and_hms(2026, 9, 1, 0, 0, 0).unwrap();
        let roles_after = store
            .get_roles_for_principal("user:bob", after)
            .await
            .unwrap();
        assert!(roles_after.is_empty());
    }

    #[tokio::test]
    async fn roles_antes_de_valid_from_nao_activos() {
        let store = test_store();
        let n = now();
        let future = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
        store
            .assign_principal_to_role(
                "user:carol",
                &role("role:auditor"),
                "admin",
                future,
                None,
                n,
            )
            .unwrap();

        let roles = store
            .get_roles_for_principal("user:carol", n)
            .await
            .unwrap();
        assert!(roles.is_empty());

        let roles_later = store
            .get_roles_for_principal("user:carol", future)
            .await
            .unwrap();
        assert_eq!(roles_later.len(), 1);
    }

    #[tokio::test]
    async fn revoke_membership_remove_role() {
        let store = test_store();
        let n = now();
        let mid = store
            .assign_principal_to_role("user:dave", &role("role:editor"), "admin", n, None, n)
            .unwrap();

        let roles_before = store.get_roles_for_principal("user:dave", n).await.unwrap();
        assert_eq!(roles_before.len(), 1);

        store.revoke_membership(&mid, "admin", n).unwrap();

        let roles_after = store.get_roles_for_principal("user:dave", n).await.unwrap();
        assert!(roles_after.is_empty());
    }

    #[tokio::test]
    async fn revoke_nao_existente_falha() {
        let store = test_store();
        let err = store
            .revoke_membership(&MemberId("nao-existe".into()), "admin", now())
            .unwrap_err();
        assert!(matches!(err, RhSecurityBridgeError::MemberNotFound(_)));
    }

    #[tokio::test]
    async fn revoke_idempotente_falha_na_segunda() {
        let store = test_store();
        let n = now();
        let mid = store
            .assign_principal_to_role("user:eve", &role("role:editor"), "admin", n, None, n)
            .unwrap();
        store.revoke_membership(&mid, "admin", n).unwrap();
        let err = store.revoke_membership(&mid, "admin", n).unwrap_err();
        assert!(matches!(err, RhSecurityBridgeError::MemberNotFound(_)));
    }

    #[tokio::test]
    async fn multiplos_roles_por_principal() {
        let store = test_store();
        let n = now();
        store
            .assign_principal_to_role("user:alice", &role("role:editor"), "admin", n, None, n)
            .unwrap();
        store
            .assign_principal_to_role("user:alice", &role("role:auditor"), "admin", n, None, n)
            .unwrap();

        let roles = store
            .get_roles_for_principal("user:alice", n)
            .await
            .unwrap();
        assert_eq!(roles.len(), 2);
    }

    #[tokio::test]
    async fn list_role_members_activos() {
        let store = test_store();
        let n = now();
        store
            .assign_principal_to_role("user:alice", &role("role:editor"), "admin", n, None, n)
            .unwrap();
        store
            .assign_principal_to_role("user:bob", &role("role:editor"), "admin", n, None, n)
            .unwrap();
        store
            .assign_principal_to_role("user:carol", &role("role:auditor"), "admin", n, None, n)
            .unwrap();

        let editors = store.list_role_members(&role("role:editor"), n).unwrap();
        assert_eq!(editors.len(), 2);
        assert!(editors.contains(&"user:alice".to_string()));
        assert!(editors.contains(&"user:bob".to_string()));

        let auditors = store.list_role_members(&role("role:auditor"), n).unwrap();
        assert_eq!(auditors.len(), 1);
    }

    #[tokio::test]
    async fn list_role_members_exclui_revogados() {
        let store = test_store();
        let n = now();
        store
            .assign_principal_to_role("user:alice", &role("role:editor"), "admin", n, None, n)
            .unwrap();
        let mid = store
            .assign_principal_to_role("user:bob", &role("role:editor"), "admin", n, None, n)
            .unwrap();
        store.revoke_membership(&mid, "admin", n).unwrap();

        let editors = store.list_role_members(&role("role:editor"), n).unwrap();
        assert_eq!(editors.len(), 1);
        assert_eq!(editors[0], "user:alice");
    }

    #[tokio::test]
    async fn integracao_com_security_service() {
        use core_security::{
            DelegationRequest, GrantedBy, NoopSecurityAuditLog, Policy, PolicyMode, Rule,
            SecurityService, VerifiedPrincipal, WriteInvariantContext,
        };
        use security_sqlite::SecuritySqliteStore;

        let tmp_security = NamedTempFile::new().unwrap();
        let tmp_roles = NamedTempFile::new().unwrap();

        let security_store = SecuritySqliteStore::open(&SqliteRelationalConfig::read_write_create(
            tmp_security.path(),
        ))
        .unwrap();
        let role_store = RhSecurityBridgeStore::open(&SqliteRelationalConfig::read_write_create(
            tmp_roles.path(),
        ))
        .unwrap();

        let n = now();

        security_store
            .save_policy(
                &Policy {
                    policy_id: "pol-s".into(),
                    version: "1.0.0".into(),
                    mode: PolicyMode::Strict,
                    rules: vec![Rule {
                        code: "AUTH".into(),
                        enabled: true,
                        description: None,
                    }],
                },
                n,
            )
            .await
            .unwrap();

        security_store
            .delegate_permission(
                &DelegationRequest {
                    principal: "role:editor".into(),
                    operation: "doc.edit".into(),
                    resource: None,
                    granted_by: "admin".into(),
                    valid_from: n,
                    valid_to: later(),
                    conditions: None,
                    granted_via: None,
                },
                n,
            )
            .await
            .unwrap();

        role_store
            .assign_principal_to_role("user:alice", &role("role:editor"), "admin", n, None, n)
            .unwrap();

        let svc = SecurityService::with_all(security_store, NoopSecurityAuditLog, role_store);

        let dec = svc
            .authorize(
                &WriteInvariantContext {
                    operation: "doc.edit".into(),
                    correlation_id: "corr-1".into(),
                    principal: VerifiedPrincipal::human("user:alice"),
                },
                None,
                n,
            )
            .await
            .unwrap();

        assert!(matches!(dec.granted_by, GrantedBy::RoleDelegation(_, _)));
    }
}
