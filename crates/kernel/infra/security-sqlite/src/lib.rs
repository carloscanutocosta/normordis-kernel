use std::sync::{Arc, Mutex};

use adapter_sqlite::{
    open_relational_connection, run_relational_migrations, SqliteRelationalConfig,
};
use chrono::{DateTime, Utc};
use core_security::{
    validate_policy, AuditDecision, Delegation, DelegationId, DelegationRequest, ListOptions,
    Policy, PolicyMode, RevocationRequest, Rule, SecurityAuditLog, SecurityAuthDecision,
    SecurityError, SecurityPolicyRepository,
};
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;
use uuid::Uuid;

// ── Migrations ────────────────────────────────────────────────────────────────

/// Migration 1 — esquema inicial: políticas, delegações, índices.
const MIGRATION_1: &str = r#"
    CREATE TABLE IF NOT EXISTS security_policies (
        policy_id  TEXT PRIMARY KEY,
        version    TEXT NOT NULL,
        mode       TEXT NOT NULL,
        rules      TEXT NOT NULL,
        created_at TEXT NOT NULL,
        revoked    INTEGER NOT NULL DEFAULT 0,
        revoked_at TEXT,
        revoked_by TEXT,
        reason     TEXT
    );

    CREATE TABLE IF NOT EXISTS security_delegations (
        delegation_id TEXT PRIMARY KEY,
        principal     TEXT NOT NULL,
        operation     TEXT NOT NULL,
        resource      TEXT,
        granted_by    TEXT NOT NULL,
        granted_at    TEXT NOT NULL,
        valid_from    TEXT NOT NULL,
        valid_to      TEXT NOT NULL,
        conditions    TEXT,
        revoked       INTEGER NOT NULL DEFAULT 0
    );

    CREATE INDEX IF NOT EXISTS idx_security_policies_active
        ON security_policies (revoked, policy_id);
    CREATE INDEX IF NOT EXISTS idx_security_delegations_principal
        ON security_delegations (principal, valid_from, valid_to);
    CREATE INDEX IF NOT EXISTS idx_security_delegations_active
        ON security_delegations (principal, revoked);
"#;

/// Migration 2 — rastreabilidade de cadeia e revogação em cascata.
const MIGRATION_2: &str = r#"
    ALTER TABLE security_delegations ADD COLUMN granted_via TEXT
        REFERENCES security_delegations(delegation_id);
    CREATE INDEX IF NOT EXISTS idx_security_delegations_granted_via
        ON security_delegations (granted_via);
"#;

/// Migration 3 — log imutável de decisões de autorização.
const MIGRATION_3: &str = r#"
    CREATE TABLE IF NOT EXISTS security_auth_decisions (
        id             INTEGER PRIMARY KEY AUTOINCREMENT,
        logged_at      TEXT NOT NULL,
        principal      TEXT NOT NULL,
        operation      TEXT NOT NULL,
        resource       TEXT,
        correlation_id TEXT NOT NULL,
        decision       TEXT NOT NULL,
        granted_by_kind TEXT,
        deny_reason    TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_security_auth_decisions_principal
        ON security_auth_decisions (principal, logged_at);
    CREATE INDEX IF NOT EXISTS idx_security_auth_decisions_operation
        ON security_auth_decisions (operation, logged_at);
"#;

pub const SECURITY_SQLITE_MIGRATIONS: &[&str] = &[MIGRATION_1, MIGRATION_2, MIGRATION_3];

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum SecuritySqliteError {
    #[error(transparent)]
    Adapter(#[from] support_errors::MiniError),
    #[error("erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("timestamp inválido: {0}")]
    InvalidTimestamp(String),
    #[error("mode de policy desconhecido: {0}")]
    UnknownMode(String),
    #[error("rules inválidas: {0}")]
    InvalidRules(String),
}

impl From<SecuritySqliteError> for SecurityError {
    fn from(e: SecuritySqliteError) -> Self {
        SecurityError::OperationFailed(e.to_string())
    }
}

// ── Store ─────────────────────────────────────────────────────────────────────

/// Adapter SQLite para `SecurityPolicyRepository` e `SecurityAuditLog`.
///
/// Internamente usa `Arc<Mutex<Connection>>` — é `Clone`, `Send` e `Sync`.
/// Pode ser partilhado entre o repositório e o log de auditoria na mesma BD.
///
/// ## Threading
///
/// As operações SQLite são síncronas sob o `Mutex`. Para cargas concorrentes
/// intensas considerar um pool de ligações ou `tokio-rusqlite`.
#[derive(Clone)]
pub struct SecuritySqliteStore {
    conn: Arc<Mutex<Connection>>,
}

impl SecuritySqliteStore {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, SecuritySqliteError> {
        let conn = open_relational_connection(config)?;
        let store = Self { conn: Arc::new(Mutex::new(conn)) };
        store.migrate()?;
        Ok(store)
    }

    pub fn from_connection(conn: Connection) -> Result<Self, SecuritySqliteError> {
        let store = Self { conn: Arc::new(Mutex::new(conn)) };
        store.migrate()?;
        Ok(store)
    }

    pub fn migrate(&self) -> Result<(), SecuritySqliteError> {
        let conn = self.conn.lock().map_err(|_| {
            SecuritySqliteError::Sqlite(rusqlite::Error::InvalidQuery)
        })?;
        run_relational_migrations(&conn, SECURITY_SQLITE_MIGRATIONS)?;
        Ok(())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn dt_to_str(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

fn str_to_dt(s: &str) -> Result<DateTime<Utc>, SecuritySqliteError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| SecuritySqliteError::InvalidTimestamp(s.to_string()))
}

fn mode_to_str(m: &PolicyMode) -> &'static str {
    match m {
        PolicyMode::Baseline => "baseline",
        PolicyMode::Strict => "strict",
    }
}

fn str_to_mode(s: &str) -> Result<PolicyMode, SecuritySqliteError> {
    match s {
        "baseline" => Ok(PolicyMode::Baseline),
        "strict" => Ok(PolicyMode::Strict),
        other => Err(SecuritySqliteError::UnknownMode(other.to_string())),
    }
}

fn rules_to_json(rules: &[Rule]) -> Result<String, SecuritySqliteError> {
    serde_json::to_string(rules).map_err(|e| SecuritySqliteError::InvalidRules(e.to_string()))
}

fn json_to_rules(s: &str) -> Result<Vec<Rule>, SecuritySqliteError> {
    serde_json::from_str(s).map_err(|e| SecuritySqliteError::InvalidRules(e.to_string()))
}

fn page_sql(opts: Option<ListOptions>) -> (i64, i64) {
    match opts {
        None => (-1, 0),
        Some(o) => (o.limit as i64, o.offset as i64),
    }
}

// ── SecurityPolicyRepository ──────────────────────────────────────────────────

impl SecurityPolicyRepository for SecuritySqliteStore {
    async fn save_policy(&self, policy: &Policy, now: DateTime<Utc>) -> Result<(), SecurityError> {
        validate_policy(policy)?;
        let rules_json =
            rules_to_json(&policy.rules).map_err(|e| SecurityError::OperationFailed(e.to_string()))?;

        let conn =
            self.conn.lock().map_err(|_| SecurityError::RepoUnavailable("lock poisoned".into()))?;

        let existing: Option<String> = conn
            .query_row(
                "SELECT version FROM security_policies WHERE policy_id = ?1",
                params![policy.policy_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| SecurityError::OperationFailed(e.to_string()))?;

        if let Some(existing_version) = existing {
            if existing_version != policy.version {
                return Err(SecurityError::AlreadyExists(format!(
                    "policy_id '{}' já existe com version '{}'",
                    policy.policy_id, existing_version
                )));
            }
            return Ok(());
        }

        conn.execute(
            "INSERT INTO security_policies
                 (policy_id, version, mode, rules, created_at, revoked)
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![
                policy.policy_id,
                policy.version,
                mode_to_str(&policy.mode),
                rules_json,
                dt_to_str(now),
            ],
        )
        .map_err(|e| SecurityError::OperationFailed(e.to_string()))?;
        Ok(())
    }

    async fn get_policy(&self, policy_id: &str) -> Result<Option<Policy>, SecurityError> {
        let conn =
            self.conn.lock().map_err(|_| SecurityError::RepoUnavailable("lock poisoned".into()))?;
        let row = conn
            .query_row(
                "SELECT policy_id, version, mode, rules
                 FROM security_policies WHERE policy_id = ?1",
                params![policy_id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| SecurityError::OperationFailed(e.to_string()))?;

        let Some((id, version, mode_s, rules_s)) = row else {
            return Ok(None);
        };
        let mode =
            str_to_mode(&mode_s).map_err(|e| SecurityError::OperationFailed(e.to_string()))?;
        let rules =
            json_to_rules(&rules_s).map_err(|e| SecurityError::OperationFailed(e.to_string()))?;
        Ok(Some(Policy { policy_id: id, version, mode, rules }))
    }

    async fn list_active_policies(
        &self,
        opts: Option<ListOptions>,
    ) -> Result<Vec<Policy>, SecurityError> {
        let conn =
            self.conn.lock().map_err(|_| SecurityError::RepoUnavailable("lock poisoned".into()))?;
        let (limit, offset) = page_sql(opts);

        let mut stmt = conn
            .prepare(
                "SELECT policy_id, version, mode, rules
                 FROM security_policies WHERE revoked = 0
                 ORDER BY policy_id
                 LIMIT ?1 OFFSET ?2",
            )
            .map_err(|e| SecurityError::OperationFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![limit, offset], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| SecurityError::OperationFailed(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            let (id, version, mode_s, rules_s) =
                row.map_err(|e| SecurityError::OperationFailed(e.to_string()))?;
            let mode = str_to_mode(&mode_s)
                .map_err(|e| SecurityError::OperationFailed(e.to_string()))?;
            let rules = json_to_rules(&rules_s)
                .map_err(|e| SecurityError::OperationFailed(e.to_string()))?;
            result.push(Policy { policy_id: id, version, mode, rules });
        }
        Ok(result)
    }

    async fn revoke_policy(
        &self,
        req: &RevocationRequest,
        now: DateTime<Utc>,
    ) -> Result<(), SecurityError> {
        req.validate()?;
        let conn =
            self.conn.lock().map_err(|_| SecurityError::RepoUnavailable("lock poisoned".into()))?;
        let affected = conn
            .execute(
                "UPDATE security_policies
                 SET revoked = 1, revoked_at = ?1, revoked_by = ?2, reason = ?3
                 WHERE policy_id = ?4 AND revoked = 0",
                params![dt_to_str(now), req.revoked_by, req.reason, req.policy_id],
            )
            .map_err(|e| SecurityError::OperationFailed(e.to_string()))?;
        if affected == 0 {
            return Err(SecurityError::PolicyNotFound(req.policy_id.clone()));
        }
        Ok(())
    }

    async fn delegate_permission(
        &self,
        req: &DelegationRequest,
        now: DateTime<Utc>,
    ) -> Result<Delegation, SecurityError> {
        req.validate()?;
        let delegation_id = DelegationId(Uuid::new_v4().to_string());
        let delegation = Delegation {
            delegation_id: delegation_id.clone(),
            principal: req.principal.clone(),
            operation: req.operation.clone(),
            resource: req.resource.clone(),
            granted_by: req.granted_by.clone(),
            granted_at: now,
            valid_from: req.valid_from,
            valid_to: req.valid_to,
            conditions: req.conditions.clone(),
            revoked: false,
            granted_via: req.granted_via.clone(),
        };

        let conn =
            self.conn.lock().map_err(|_| SecurityError::RepoUnavailable("lock poisoned".into()))?;
        conn.execute(
            "INSERT INTO security_delegations
                 (delegation_id, principal, operation, resource, granted_by,
                  granted_at, valid_from, valid_to, conditions, revoked, granted_via)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, ?10)",
            params![
                delegation_id.as_str(),
                req.principal,
                req.operation,
                req.resource,
                req.granted_by,
                dt_to_str(now),
                dt_to_str(req.valid_from),
                dt_to_str(req.valid_to),
                req.conditions,
                req.granted_via.as_ref().map(|id| id.as_str()),
            ],
        )
        .map_err(|e| SecurityError::OperationFailed(e.to_string()))?;
        Ok(delegation)
    }

    async fn list_delegations(
        &self,
        principal: &str,
        now: DateTime<Utc>,
        opts: Option<ListOptions>,
    ) -> Result<Vec<Delegation>, SecurityError> {
        let now_s = dt_to_str(now);
        let (limit, offset) = page_sql(opts);

        let conn =
            self.conn.lock().map_err(|_| SecurityError::RepoUnavailable("lock poisoned".into()))?;
        let mut stmt = conn
            .prepare(
                "SELECT delegation_id, principal, operation, resource, granted_by,
                        granted_at, valid_from, valid_to, conditions, revoked, granted_via
                 FROM security_delegations
                 WHERE principal = ?1
                   AND revoked = 0
                   AND valid_from <= ?2
                   AND valid_to > ?2
                 ORDER BY delegation_id
                 LIMIT ?3 OFFSET ?4",
            )
            .map_err(|e| SecurityError::OperationFailed(e.to_string()))?;

        let rows = stmt
            .query_map(params![principal, now_s, limit, offset], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, String>(7)?,
                    r.get::<_, Option<String>>(8)?,
                    r.get::<_, i64>(9)?,
                    r.get::<_, Option<String>>(10)?,
                ))
            })
            .map_err(|e| SecurityError::OperationFailed(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            let (id_s, principal_s, operation, resource, granted_by, granted_at_s, vfrom_s,
                vto_s, conditions, revoked_i, granted_via_s) =
                row.map_err(|e| SecurityError::OperationFailed(e.to_string()))?;
            result.push(Delegation {
                delegation_id: DelegationId(id_s),
                principal: principal_s,
                operation,
                resource,
                granted_by,
                granted_at: str_to_dt(&granted_at_s)
                    .map_err(|e| SecurityError::OperationFailed(e.to_string()))?,
                valid_from: str_to_dt(&vfrom_s)
                    .map_err(|e| SecurityError::OperationFailed(e.to_string()))?,
                valid_to: str_to_dt(&vto_s)
                    .map_err(|e| SecurityError::OperationFailed(e.to_string()))?,
                conditions,
                revoked: revoked_i != 0,
                granted_via: granted_via_s.map(DelegationId),
            });
        }
        Ok(result)
    }

    async fn revoke_delegation(
        &self,
        delegation_id: &DelegationId,
        _now: DateTime<Utc>,
    ) -> Result<(), SecurityError> {
        let conn =
            self.conn.lock().map_err(|_| SecurityError::RepoUnavailable("lock poisoned".into()))?;

        // Verificar que existe e não está revogada
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM security_delegations WHERE delegation_id = ?1 AND revoked = 0",
                params![delegation_id.as_str()],
                |r| r.get(0),
            )
            .map_err(|e| SecurityError::OperationFailed(e.to_string()))?;

        if !exists {
            return Err(SecurityError::DelegationNotFound(delegation_id.as_str().into()));
        }

        // Revogação em cascata via CTE recursiva
        conn.execute_batch(&format!(
            "WITH RECURSIVE cascade(id) AS (
                SELECT '{}'
                UNION ALL
                SELECT sd.delegation_id FROM security_delegations sd
                INNER JOIN cascade c ON sd.granted_via = c.id
                WHERE sd.revoked = 0
            )
            UPDATE security_delegations SET revoked = 1
            WHERE delegation_id IN (SELECT id FROM cascade)",
            delegation_id.as_str().replace('\'', "''")
        ))
        .map_err(|e| SecurityError::OperationFailed(e.to_string()))?;

        Ok(())
    }
}

// ── SecurityAuditLog ──────────────────────────────────────────────────────────

impl SecurityAuditLog for SecuritySqliteStore {
    async fn record_decision(&self, entry: &SecurityAuthDecision) -> Result<(), SecurityError> {
        let conn =
            self.conn.lock().map_err(|_| SecurityError::RepoUnavailable("lock poisoned".into()))?;

        let decision_s = match entry.decision {
            AuditDecision::Granted => "granted",
            AuditDecision::Denied => "denied",
        };

        conn.execute(
            "INSERT INTO security_auth_decisions
                 (logged_at, principal, operation, resource, correlation_id,
                  decision, granted_by_kind, deny_reason)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                dt_to_str(entry.logged_at),
                entry.principal,
                entry.operation,
                entry.resource,
                entry.correlation_id,
                decision_s,
                entry.granted_by_kind,
                entry.deny_reason,
            ],
        )
        .map_err(|e| SecurityError::OperationFailed(e.to_string()))?;
        Ok(())
    }
}

// ── Convenience façade ────────────────────────────────────────────────────────

impl SecuritySqliteStore {
    pub async fn save_policy(&self, policy: &Policy, now: DateTime<Utc>) -> Result<(), SecurityError> {
        SecurityPolicyRepository::save_policy(self, policy, now).await
    }

    pub async fn get_policy(&self, policy_id: &str) -> Result<Option<Policy>, SecurityError> {
        SecurityPolicyRepository::get_policy(self, policy_id).await
    }

    pub async fn list_active_policies(
        &self,
        opts: Option<ListOptions>,
    ) -> Result<Vec<Policy>, SecurityError> {
        SecurityPolicyRepository::list_active_policies(self, opts).await
    }

    pub async fn revoke_policy(
        &self,
        req: &RevocationRequest,
        now: DateTime<Utc>,
    ) -> Result<(), SecurityError> {
        SecurityPolicyRepository::revoke_policy(self, req, now).await
    }

    pub async fn delegate_permission(
        &self,
        req: &DelegationRequest,
        now: DateTime<Utc>,
    ) -> Result<Delegation, SecurityError> {
        SecurityPolicyRepository::delegate_permission(self, req, now).await
    }

    pub async fn list_delegations(
        &self,
        principal: &str,
        now: DateTime<Utc>,
        opts: Option<ListOptions>,
    ) -> Result<Vec<Delegation>, SecurityError> {
        SecurityPolicyRepository::list_delegations(self, principal, now, opts).await
    }

    pub async fn revoke_delegation(
        &self,
        delegation_id: &DelegationId,
        now: DateTime<Utc>,
    ) -> Result<(), SecurityError> {
        SecurityPolicyRepository::revoke_delegation(self, delegation_id, now).await
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use core_security::{ListOptions, PolicyMode, Rule, VerifiedPrincipal, WriteInvariantContext};
    use tempfile::NamedTempFile;

    fn test_store() -> SecuritySqliteStore {
        let tmp = NamedTempFile::new().unwrap();
        SecuritySqliteStore::open(&SqliteRelationalConfig::read_write_create(tmp.path())).unwrap()
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 15, 10, 0, 0).unwrap()
    }

    fn later() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 12, 31, 23, 59, 59).unwrap()
    }

    fn sample_policy(id: &str) -> Policy {
        Policy {
            policy_id: id.into(),
            version: "1.0.0".into(),
            mode: PolicyMode::Baseline,
            rules: vec![Rule {
                code: "MIN-AUTH".into(),
                enabled: true,
                description: Some("Autenticação mínima".into()),
            }],
        }
    }

    fn human_ctx(principal: &str, op: &str) -> WriteInvariantContext {
        WriteInvariantContext {
            operation: op.into(),
            correlation_id: "corr-test".into(),
            principal: VerifiedPrincipal::human(principal),
        }
    }

    fn grant_req(principal: &str, op: &str) -> DelegationRequest {
        DelegationRequest {
            principal: principal.into(),
            operation: op.into(),
            resource: None,
            granted_by: "admin".into(),
            valid_from: now(),
            valid_to: later(),
            conditions: None,
            granted_via: None,
        }
    }

    // ── Políticas ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn policy_round_trip() {
        let store = test_store();
        let policy = sample_policy("pol-1");
        store.save_policy(&policy, now()).await.unwrap();
        let loaded = store.get_policy("pol-1").await.unwrap().unwrap();
        assert_eq!(loaded.policy_id, "pol-1");
        assert_eq!(loaded.version, "1.0.0");
        assert_eq!(loaded.mode, PolicyMode::Baseline);
        assert_eq!(loaded.rules.len(), 1);
        assert_eq!(loaded.rules[0].code, "MIN-AUTH");
        assert_eq!(loaded.rules[0].description.as_deref(), Some("Autenticação mínima"));
    }

    #[tokio::test]
    async fn policy_idempotent_save() {
        let store = test_store();
        let policy = sample_policy("pol-idem");
        store.save_policy(&policy, now()).await.unwrap();
        store.save_policy(&policy, now()).await.unwrap();
        let active = store.list_active_policies(None).await.unwrap();
        assert_eq!(active.len(), 1);
    }

    #[tokio::test]
    async fn policy_conflict_on_different_version() {
        let store = test_store();
        let policy = sample_policy("pol-conflict");
        store.save_policy(&policy, now()).await.unwrap();
        let mut policy_v2 = policy.clone();
        policy_v2.version = "2.0.0".into();
        let err = store.save_policy(&policy_v2, now()).await.unwrap_err();
        assert!(matches!(err, SecurityError::AlreadyExists(_)));
    }

    #[tokio::test]
    async fn list_active_excludes_revoked() {
        let store = test_store();
        store.save_policy(&sample_policy("pol-a"), now()).await.unwrap();
        store.save_policy(&sample_policy("pol-b"), now()).await.unwrap();

        store
            .revoke_policy(
                &RevocationRequest {
                    policy_id: "pol-a".into(),
                    revoked_by: "admin".into(),
                    reason: Some("teste".into()),
                },
                now(),
            )
            .await
            .unwrap();

        let active = store.list_active_policies(None).await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].policy_id, "pol-b");
    }

    #[tokio::test]
    async fn revoke_not_found() {
        let store = test_store();
        let err = store
            .revoke_policy(
                &RevocationRequest {
                    policy_id: "nao-existe".into(),
                    revoked_by: "admin".into(),
                    reason: None,
                },
                now(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, SecurityError::PolicyNotFound(_)));
    }

    #[tokio::test]
    async fn strict_policy_round_trip() {
        let store = test_store();
        let policy = Policy {
            policy_id: "pol-strict".into(),
            version: "1.0.0".into(),
            mode: PolicyMode::Strict,
            rules: vec![
                Rule { code: "AUTH".into(), enabled: true, description: None },
                Rule { code: "AUDIT".into(), enabled: true, description: None },
            ],
        };
        store.save_policy(&policy, now()).await.unwrap();
        let loaded = store.get_policy("pol-strict").await.unwrap().unwrap();
        assert_eq!(loaded.mode, PolicyMode::Strict);
        assert_eq!(loaded.rules.len(), 2);
    }

    #[tokio::test]
    async fn get_policy_not_found() {
        let store = test_store();
        assert!(store.get_policy("nao-existe").await.unwrap().is_none());
    }

    // ── Paginação ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn list_active_policies_paginacao() {
        let store = test_store();
        for i in 0..5u8 {
            store
                .save_policy(
                    &Policy {
                        policy_id: format!("pol-{i:02}"),
                        version: "1.0.0".into(),
                        mode: PolicyMode::Baseline,
                        rules: vec![Rule {
                            code: "A".into(),
                            enabled: true,
                            description: None,
                        }],
                    },
                    now(),
                )
                .await
                .unwrap();
        }
        let page1 = store.list_active_policies(Some(ListOptions::page(1, 2))).await.unwrap();
        let page2 = store.list_active_policies(Some(ListOptions::page(2, 2))).await.unwrap();
        let page3 = store.list_active_policies(Some(ListOptions::page(3, 2))).await.unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 2);
        assert_eq!(page3.len(), 1);
    }

    // ── Delegações ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn delegation_round_trip() {
        let store = test_store();
        let n = now();
        let req = DelegationRequest {
            principal: "user:alice".into(),
            operation: "sign.document".into(),
            resource: Some("doc-123".into()),
            granted_by: "admin:root".into(),
            valid_from: n,
            valid_to: later(),
            conditions: None,
            granted_via: None,
        };
        let d = store.delegate_permission(&req, n).await.unwrap();
        assert_eq!(d.principal, "user:alice");
        assert_eq!(d.operation, "sign.document");
        assert!(!d.delegation_id.as_str().is_empty());

        let active = store.list_delegations("user:alice", n, None).await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].resource.as_deref(), Some("doc-123"));
    }

    #[tokio::test]
    async fn delegation_inactive_after_window() {
        let store = test_store();
        let n = now();
        let req = DelegationRequest {
            principal: "user:bob".into(),
            operation: "read.reports".into(),
            resource: None,
            granted_by: "admin:root".into(),
            valid_from: n,
            valid_to: Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap(),
            conditions: None,
            granted_via: None,
        };
        store.delegate_permission(&req, n).await.unwrap();

        let after = Utc.with_ymd_and_hms(2026, 3, 1, 0, 0, 0).unwrap();
        let expired = store.list_delegations("user:bob", after, None).await.unwrap();
        assert!(expired.is_empty());
    }

    #[tokio::test]
    async fn delegation_before_valid_from_not_listed() {
        let store = test_store();
        let n = now();
        let future_start = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
        let req = DelegationRequest {
            principal: "user:carol".into(),
            operation: "approve".into(),
            resource: None,
            granted_by: "admin".into(),
            valid_from: future_start,
            valid_to: Utc.with_ymd_and_hms(2026, 12, 31, 0, 0, 0).unwrap(),
            conditions: None,
            granted_via: None,
        };
        store.delegate_permission(&req, n).await.unwrap();
        let too_early = store.list_delegations("user:carol", n, None).await.unwrap();
        assert!(too_early.is_empty());
    }

    // ── Revogação de delegação ────────────────────────────────────────────────

    #[tokio::test]
    async fn revoke_delegation_not_found_sqlite() {
        let store = test_store();
        let err =
            store.revoke_delegation(&DelegationId("nao-existe".into()), now()).await.unwrap_err();
        assert!(matches!(err, SecurityError::DelegationNotFound(_)));
    }

    #[tokio::test]
    async fn revoke_delegation_idempotent_false() {
        let store = test_store();
        let n = now();
        let deleg = store.delegate_permission(&grant_req("user:dave", "read"), n).await.unwrap();
        store.revoke_delegation(&deleg.delegation_id, n).await.unwrap();
        let err =
            store.revoke_delegation(&deleg.delegation_id, n).await.unwrap_err();
        assert!(matches!(err, SecurityError::DelegationNotFound(_)));
    }

    #[tokio::test]
    async fn revoke_delegation_cascata_sqlite() {
        let store = test_store();
        let n = now();

        // Root: alice
        let root = store.delegate_permission(&grant_req("user:alice", "doc.sign"), n).await.unwrap();

        // Child: bob, granted_via root
        let child_req = DelegationRequest {
            principal: "user:bob".into(),
            operation: "doc.sign".into(),
            resource: None,
            granted_by: "user:alice".into(),
            valid_from: n,
            valid_to: later(),
            conditions: None,
            granted_via: Some(root.delegation_id.clone()),
        };
        let child = store.delegate_permission(&child_req, n).await.unwrap();

        // Grandchild: carol, granted_via child
        let grandchild_req = DelegationRequest {
            principal: "user:carol".into(),
            operation: "doc.sign".into(),
            resource: None,
            granted_by: "user:bob".into(),
            valid_from: n,
            valid_to: later(),
            conditions: None,
            granted_via: Some(child.delegation_id.clone()),
        };
        store.delegate_permission(&grandchild_req, n).await.unwrap();

        // Todos têm delegações activas antes da revogação
        assert_eq!(store.list_delegations("user:alice", n, None).await.unwrap().len(), 1);
        assert_eq!(store.list_delegations("user:bob", n, None).await.unwrap().len(), 1);
        assert_eq!(store.list_delegations("user:carol", n, None).await.unwrap().len(), 1);

        // Revogar root → cascata para child e grandchild
        store.revoke_delegation(&root.delegation_id, n).await.unwrap();

        assert!(store.list_delegations("user:alice", n, None).await.unwrap().is_empty());
        assert!(store.list_delegations("user:bob", n, None).await.unwrap().is_empty());
        assert!(store.list_delegations("user:carol", n, None).await.unwrap().is_empty());
    }

    // ── SecurityService + SecuritySqliteStore (integração completa) ───────────

    #[tokio::test]
    async fn service_bootstrap_sem_politicas() {
        use core_security::{GrantedBy, SecurityService};
        let store = test_store();
        let svc = SecurityService::new(store);
        let dec = svc.authorize(&human_ctx("user:alice", "doc.sign"), None, now()).await.unwrap();
        assert!(matches!(dec.granted_by, GrantedBy::Bootstrap));
    }

    #[tokio::test]
    async fn service_strict_com_delegacao_sqlite() {
        use core_security::{GrantedBy, Policy, PolicyMode, Rule, SecurityService};
        let store = test_store();
        let n = now();

        store
            .save_policy(
                &Policy {
                    policy_id: "pol-strict".into(),
                    version: "1.0.0".into(),
                    mode: PolicyMode::Strict,
                    rules: vec![Rule { code: "AUTH".into(), enabled: true, description: None }],
                },
                n,
            )
            .await
            .unwrap();

        store
            .delegate_permission(
                &DelegationRequest {
                    principal: "user:alice".into(),
                    operation: "doc.sign".into(),
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

        let svc = SecurityService::new(store);
        let dec = svc.authorize(&human_ctx("user:alice", "doc.sign"), None, n).await.unwrap();
        assert!(matches!(dec.granted_by, GrantedBy::Delegation(_)));
        assert_eq!(dec.principal, "user:alice");
        assert_eq!(dec.operation, "doc.sign");
    }

    #[tokio::test]
    async fn service_strict_sem_delegacao_nega() {
        use core_security::{Policy, PolicyMode, Rule, SecurityError, SecurityService};
        let store = test_store();
        let n = now();

        store
            .save_policy(
                &Policy {
                    policy_id: "pol-strict".into(),
                    version: "1.0.0".into(),
                    mode: PolicyMode::Strict,
                    rules: vec![Rule { code: "AUTH".into(), enabled: true, description: None }],
                },
                n,
            )
            .await
            .unwrap();

        let svc = SecurityService::new(store);
        let err =
            svc.authorize(&human_ctx("user:bob", "doc.sign"), None, n).await.unwrap_err();
        assert!(matches!(err, SecurityError::InvariantViolated(_)));
    }

    #[tokio::test]
    async fn service_recurso_especifico_sqlite() {
        use core_security::{GrantedBy, Policy, PolicyMode, Rule, SecurityError, SecurityService};
        let store = test_store();
        let n = now();

        store
            .save_policy(
                &Policy {
                    policy_id: "pol-s".into(),
                    version: "1.0.0".into(),
                    mode: PolicyMode::Strict,
                    rules: vec![Rule { code: "A".into(), enabled: true, description: None }],
                },
                n,
            )
            .await
            .unwrap();

        store
            .delegate_permission(
                &DelegationRequest {
                    principal: "user:carol".into(),
                    operation: "report.export".into(),
                    resource: Some("relatorio-2026".into()),
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

        let svc = SecurityService::new(store);
        let ctx = human_ctx("user:carol", "report.export");
        let dec = svc.authorize(&ctx, Some("relatorio-2026"), n).await.unwrap();
        assert!(matches!(dec.granted_by, GrantedBy::Delegation(_)));
        let err = svc.authorize(&ctx, Some("outro-relatorio"), n).await.unwrap_err();
        assert!(matches!(err, SecurityError::InvariantViolated(_)));
    }

    // ── Audit log SQLite ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn service_with_audit_sqlite_regista_decisoes() {
        use core_security::SecurityService;
        let tmp = NamedTempFile::new().unwrap();
        let store =
            SecuritySqliteStore::open(&SqliteRelationalConfig::read_write_create(tmp.path()))
                .unwrap();

        // SecuritySqliteStore implementa tanto SecurityPolicyRepository como SecurityAuditLog
        let svc = SecurityService::with_audit(store.clone(), store.clone());

        let _ = svc.authorize(&human_ctx("user:alice", "any.op"), None, now()).await;

        // Ler audit_log directamente da BD
        let conn = store.conn.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM security_auth_decisions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);

        let (decision, principal): (String, String) = conn
            .query_row(
                "SELECT decision, principal FROM security_auth_decisions LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(decision, "granted");
        assert_eq!(principal, "user:alice");
    }

    #[tokio::test]
    async fn revoke_delegation_remove_acesso_sqlite() {
        use core_security::{Policy, PolicyMode, Rule, SecurityError, SecurityService};
        let store = test_store();
        let n = now();

        store
            .save_policy(
                &Policy {
                    policy_id: "pol-s".into(),
                    version: "1.0.0".into(),
                    mode: PolicyMode::Strict,
                    rules: vec![Rule { code: "AUTH".into(), enabled: true, description: None }],
                },
                n,
            )
            .await
            .unwrap();

        let deleg =
            store.delegate_permission(&grant_req("user:alice", "doc.sign"), n).await.unwrap();
        let svc = SecurityService::new(store);

        assert!(svc.authorize(&human_ctx("user:alice", "doc.sign"), None, n).await.is_ok());

        svc.revoke_delegation(
            &deleg.delegation_id,
            &WriteInvariantContext {
                operation: "delegation.revoke".into(),
                correlation_id: "corr-rev".into(),
                principal: VerifiedPrincipal::system("daemon:admin"),
            },
            n,
        )
        .await
        .unwrap();

        let err =
            svc.authorize(&human_ctx("user:alice", "doc.sign"), None, n).await.unwrap_err();
        assert!(matches!(err, SecurityError::InvariantViolated(_)));
    }
}
