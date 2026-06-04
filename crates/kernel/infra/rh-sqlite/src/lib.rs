//! Adaptador SQLite para `core-rh`.
//!
//! ## Arquitectura
//!
//! `UsersSqliteStore` envolve `Connection` em `Arc<Mutex<>>`, tornando-o
//! `Send + Sync + Clone` e thread-safe. Operações multi-passo (afetações
//! auditadas) usam transacções `IMMEDIATE` para garantir atomicidade.
//!
//! ## Outbox de auditoria (migração 4)
//!
//! Os métodos `*_audited` de `PersonAssignmentRepository` persistem estado
//! e evidência COSO na **mesma transação** via `rh_audit_outbox`. A entrega
//! ao porto de auditoria é feita por `drain_audit_outbox` (idempotente,
//! resiliente a *poison messages*).

use std::sync::{Arc, Mutex, MutexGuard};

use adapter_sqlite::{
    open_relational_connection, run_relational_migrations, SqliteRelationalConfig,
};
use chrono::NaiveDate;
use core_rh::{
    resolve_current_user, PersonAssignment, PersonAssignmentId, PersonAssignmentRepository,
    RhAuditEvent, RhAuditOutbox, RhAuditPort, RhError, Role, RoleId, RoleRepository, UserContext,
    UserId, UserIdentity, UserRepository, UserRole,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use thiserror::Error;

// ── Migrações ─────────────────────────────────────────────────────────────────

pub const RH_SQLITE_MIGRATIONS: &[&str] = &[
    // M0 — baseline de utilizadores
    r#"
    CREATE TABLE IF NOT EXISTS local_user (
        UserId      TEXT PRIMARY KEY,
        Username    TEXT NOT NULL UNIQUE,
        DisplayName TEXT NOT NULL,
        Email       TEXT,
        Role        TEXT NOT NULL,
        IsActive    INTEGER NOT NULL DEFAULT 1
    );

    CREATE TABLE IF NOT EXISTS current_user_context (
        ContextId INTEGER PRIMARY KEY CHECK (ContextId = 1),
        UserId    TEXT NOT NULL,
        FOREIGN KEY (UserId) REFERENCES local_user(UserId)
    );

    CREATE INDEX IF NOT EXISTS idx_local_user_username ON local_user (Username);
    "#,
    // M1 — catálogo de roles funcionais
    r#"
    CREATE TABLE IF NOT EXISTS platform_roles (
        role_id     TEXT NOT NULL PRIMARY KEY,
        name        TEXT NOT NULL,
        description TEXT,
        is_active   INTEGER NOT NULL DEFAULT 1
    );

    CREATE INDEX IF NOT EXISTS idx_platform_roles_active ON platform_roles (is_active);
    "#,
    // M2 — afetações temporais pessoa ↔ posição (COSO effective dating)
    r#"
    CREATE TABLE IF NOT EXISTS person_assignment (
        assignment_id  TEXT NOT NULL PRIMARY KEY,
        person_id      TEXT NOT NULL REFERENCES local_user(UserId),
        position_id    TEXT NOT NULL,
        unit_id        TEXT NOT NULL,
        basis          TEXT NOT NULL,
        valid_from     TEXT NOT NULL,
        valid_until    TEXT,
        version        INTEGER NOT NULL DEFAULT 0
    );

    CREATE INDEX IF NOT EXISTS idx_pa_person_id   ON person_assignment (person_id);
    CREATE INDEX IF NOT EXISTS idx_pa_position_id ON person_assignment (position_id);
    "#,
    // M3 — outbox de evidência COSO para afetações
    // `delivered`: 0 = pendente, 1 = entregue, 2 = dead-letter.
    r#"
    CREATE TABLE IF NOT EXISTS rh_audit_outbox (
        seq        INTEGER PRIMARY KEY AUTOINCREMENT,
        event_json TEXT NOT NULL,
        created_at TEXT NOT NULL,
        delivered  INTEGER NOT NULL DEFAULT 0,
        attempts   INTEGER NOT NULL DEFAULT 0,
        last_error TEXT
    );

    CREATE INDEX IF NOT EXISTS idx_rh_audit_outbox_pending
        ON rh_audit_outbox (delivered, seq);
    "#,
];

/// Tentativas de entrega antes de um evento ser marcado dead-letter.
pub const MAX_OUTBOX_ATTEMPTS: i64 = 5;

// ── Erro ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum UsersSqliteError {
    #[error(transparent)]
    SqliteAdapter(#[from] support_errors::MiniError),
    #[error("erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("erro RH: {0}")]
    Rh(#[from] RhError),
    #[error("papel de utilizador inválido: {0}")]
    InvalidUserRole(String),
    #[error("utilizador não encontrado: {0}")]
    UserNotFound(String),
    #[error("não existe utilizador atual configurado")]
    MissingCurrentUser,
}

// ── Store ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct UsersSqliteStore {
    conn: Arc<Mutex<Connection>>,
}

impl UsersSqliteStore {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, UsersSqliteError> {
        let conn = open_relational_connection(config)?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn from_connection(conn: Connection) -> Result<Self, UsersSqliteError> {
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn migrate(&self) -> Result<(), UsersSqliteError> {
        let conn = self.lock_raw()?;
        run_relational_migrations(&conn, RH_SQLITE_MIGRATIONS)?;
        Ok(())
    }

    fn lock_raw(&self) -> Result<MutexGuard<'_, Connection>, UsersSqliteError> {
        self.conn
            .lock()
            .map_err(|_| UsersSqliteError::Sqlite(rusqlite::Error::InvalidQuery))
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, RhError> {
        self.conn
            .lock()
            .map_err(|_| RhError::OperationFailed("connection mutex poisoned".into()))
    }

    // ── Operações de sessão (API pública concreta, sem port) ──────────────────

    pub fn upsert_user(&self, user: &UserIdentity) -> Result<(), UsersSqliteError> {
        user.validate()?;
        let conn = self.lock_raw()?;
        conn.execute(
            r#"INSERT INTO local_user (UserId, Username, DisplayName, Email, Role, IsActive)
               VALUES (?1, ?2, ?3, ?4, ?5, 1)
               ON CONFLICT(UserId) DO UPDATE SET
                   Username    = excluded.Username,
                   DisplayName = excluded.DisplayName,
                   Email       = excluded.Email,
                   Role        = excluded.Role,
                   IsActive    = 1"#,
            params![
                user.user_id,
                user.username,
                user.display_name,
                user.email,
                encode_user_role(&user.role),
            ],
        )?;
        Ok(())
    }

    pub fn upsert_user_in_tx(
        tx: &rusqlite::Transaction<'_>,
        user: &UserIdentity,
    ) -> Result<(), UsersSqliteError> {
        user.validate()?;
        tx.execute(
            r#"INSERT INTO local_user (UserId, Username, DisplayName, Email, Role, IsActive)
               VALUES (?1, ?2, ?3, ?4, ?5, 1)
               ON CONFLICT(UserId) DO UPDATE SET
                   Username    = excluded.Username,
                   DisplayName = excluded.DisplayName,
                   Email       = excluded.Email,
                   Role        = excluded.Role,
                   IsActive    = 1"#,
            params![
                user.user_id,
                user.username,
                user.display_name,
                user.email,
                encode_user_role(&user.role),
            ],
        )?;
        Ok(())
    }

    pub fn deactivate_user_in_tx(
        tx: &rusqlite::Transaction<'_>,
        user_id: &str,
    ) -> Result<(), UsersSqliteError> {
        let changed = tx.execute(
            "UPDATE local_user SET IsActive = 0 WHERE UserId = ?1",
            [user_id],
        )?;
        if changed == 0 {
            return Err(UsersSqliteError::UserNotFound(user_id.to_string()));
        }
        tx.execute(
            "DELETE FROM current_user_context WHERE UserId = ?1",
            [user_id],
        )?;
        Ok(())
    }

    pub fn set_current_user_in_tx(
        tx: &rusqlite::Transaction<'_>,
        user_id: &str,
    ) -> rusqlite::Result<()> {
        tx.execute(
            r#"INSERT INTO current_user_context (ContextId, UserId)
               VALUES (1, ?1)
               ON CONFLICT(ContextId) DO UPDATE SET UserId = excluded.UserId"#,
            [user_id],
        )?;
        Ok(())
    }

    pub fn clear_current_user_in_tx(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
        tx.execute("DELETE FROM current_user_context", [])?;
        Ok(())
    }

    pub fn get_user_by_id(&self, user_id: &str) -> Result<Option<UserIdentity>, UsersSqliteError> {
        let conn = self.lock_raw()?;
        let mut stmt = conn.prepare(
            "SELECT UserId, Username, DisplayName, Email, Role
             FROM local_user WHERE UserId = ?1 AND IsActive = 1",
        )?;
        Ok(stmt.query_row([user_id], decode_user).optional()?)
    }

    pub fn get_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserIdentity>, UsersSqliteError> {
        let conn = self.lock_raw()?;
        let mut stmt = conn.prepare(
            "SELECT UserId, Username, DisplayName, Email, Role
             FROM local_user WHERE Username = ?1 AND IsActive = 1",
        )?;
        Ok(stmt.query_row([username], decode_user).optional()?)
    }

    pub fn list_users(&self) -> Result<Vec<UserIdentity>, UsersSqliteError> {
        let conn = self.lock_raw()?;
        let mut stmt = conn.prepare(
            "SELECT UserId, Username, DisplayName, Email, Role
             FROM local_user WHERE IsActive = 1
             ORDER BY DisplayName ASC, Username ASC",
        )?;
        let rows = stmt.query_map([], decode_user)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn deactivate_user(&self, user_id: &str) -> Result<(), UsersSqliteError> {
        let mut conn = self.lock_raw()?;
        let tx = conn.transaction()?;
        let changed = tx.execute(
            "UPDATE local_user SET IsActive = 0 WHERE UserId = ?1",
            [user_id],
        )?;
        if changed == 0 {
            return Err(UsersSqliteError::UserNotFound(user_id.to_string()));
        }
        tx.execute(
            "DELETE FROM current_user_context WHERE UserId = ?1",
            [user_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn set_current_user(&self, user_id: &str) -> Result<UserContext, UsersSqliteError> {
        let user = self
            .get_user_by_id(user_id)?
            .ok_or_else(|| UsersSqliteError::UserNotFound(user_id.to_string()))?;
        let conn = self.lock_raw()?;
        conn.execute(
            r#"INSERT INTO current_user_context (ContextId, UserId)
               VALUES (1, ?1)
               ON CONFLICT(ContextId) DO UPDATE SET UserId = excluded.UserId"#,
            [user_id],
        )?;
        Ok(resolve_current_user(user)?)
    }

    pub fn clear_current_user(&self) -> Result<(), UsersSqliteError> {
        let conn = self.lock_raw()?;
        conn.execute("DELETE FROM current_user_context", [])?;
        Ok(())
    }

    pub fn resolve_current_user(&self) -> Result<UserContext, UsersSqliteError> {
        let conn = self.lock_raw()?;
        let user_id: String = conn
            .query_row(
                "SELECT UserId FROM current_user_context WHERE ContextId = 1",
                [],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(UsersSqliteError::MissingCurrentUser)?;
        drop(conn);

        let user = self
            .get_user_by_id(&user_id)?
            .ok_or(UsersSqliteError::MissingCurrentUser)?;
        Ok(resolve_current_user(user)?)
    }

    // ── Helpers de drain do outbox ─────────────────────────────────────────────

    fn drain_rh_audit<F>(&self, mut deliver: F) -> Result<usize, RhError>
    where
        F: FnMut(&str) -> Result<(), RhError>,
    {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT seq, event_json, attempts FROM rh_audit_outbox
                 WHERE delivered = 0 ORDER BY seq",
            )
            .map_err(op)?;
        let pending: Vec<(i64, String, i64)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(op)?
            .collect::<Result<_, _>>()
            .map_err(op)?;
        drop(stmt);

        let mut delivered = 0usize;
        for (seq, json, attempts) in pending {
            match deliver(&json) {
                Ok(()) => {
                    conn.execute(
                        "UPDATE rh_audit_outbox SET delivered = 1 WHERE seq = ?1",
                        params![seq],
                    )
                    .map_err(op)?;
                    delivered += 1;
                }
                Err(e) => {
                    let next = attempts + 1;
                    let flag = if next >= MAX_OUTBOX_ATTEMPTS { 2 } else { 0 };
                    conn.execute(
                        "UPDATE rh_audit_outbox
                         SET attempts = ?1, delivered = ?2, last_error = ?3
                         WHERE seq = ?4",
                        params![next, flag, e.to_string(), seq],
                    )
                    .map_err(op)?;
                }
            }
        }
        Ok(delivered)
    }

    fn count_outbox(&self, delivered: i64) -> Result<u64, RhError> {
        let conn = self.lock()?;
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM rh_audit_outbox WHERE delivered = ?1",
                params![delivered],
                |r| r.get(0),
            )
            .map_err(op)?;
        Ok(n as u64)
    }
}

// ── Helpers de conversão ──────────────────────────────────────────────────────

fn encode_date(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

fn decode_date(s: &str) -> Result<NaiveDate, rusqlite::Error> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e: chrono::format::ParseError| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(UsersSqliteError::Sqlite(
                rusqlite::Error::InvalidParameterName(e.to_string()),
            )),
        )
    })
}

fn decode_assignment(row: &rusqlite::Row<'_>) -> rusqlite::Result<PersonAssignment> {
    let from_str: String = row.get(5)?;
    let until_str: Option<String> = row.get(6)?;
    let valid_from = decode_date(&from_str)?;
    let valid_until = until_str.as_deref().map(decode_date).transpose()?;
    Ok(PersonAssignment {
        id: PersonAssignmentId(row.get(0)?),
        person_id: UserId::new(row.get::<_, String>(1)?).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                1,
                rusqlite::types::Type::Text,
                Box::new(UsersSqliteError::Rh(e)),
            )
        })?,
        position_id: row.get(2)?,
        unit_id: row.get(3)?,
        basis: row.get(4)?,
        valid_from,
        valid_until,
        version: row.get::<_, i64>(7)? as u32,
    })
}

fn decode_user(row: &rusqlite::Row<'_>) -> Result<UserIdentity, rusqlite::Error> {
    let role: String = row.get(4)?;
    Ok(UserIdentity {
        user_id: row.get(0)?,
        username: row.get(1)?,
        display_name: row.get(2)?,
        email: row.get(3)?,
        role: decode_user_role(&role).map_err(to_from_sql_error)?,
    })
}

fn encode_user_role(value: &UserRole) -> &'static str {
    value.as_str()
}

fn decode_user_role(value: &str) -> Result<UserRole, UsersSqliteError> {
    UserRole::parse(value).map_err(|_| UsersSqliteError::InvalidUserRole(value.to_string()))
}

fn to_from_sql_error(err: UsersSqliteError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
}

fn op(e: rusqlite::Error) -> RhError {
    RhError::OperationFailed(e.to_string())
}

fn enqueue_audit_in_tx(
    tx: &rusqlite::Transaction<'_>,
    event: &RhAuditEvent,
) -> Result<(), RhError> {
    let json = serde_json::to_string(event).map_err(|e| RhError::OperationFailed(e.to_string()))?;
    tx.execute(
        "INSERT INTO rh_audit_outbox (event_json, created_at, delivered) VALUES (?1, ?2, 0)",
        params![json, event.occurred_at.to_rfc3339()],
    )
    .map_err(op)?;
    Ok(())
}

// ── RhAuditOutbox ─────────────────────────────────────────────────────────────

impl RhAuditOutbox for UsersSqliteStore {
    fn enqueue_audit(&self, event: &RhAuditEvent) -> Result<(), RhError> {
        let conn = self.lock()?;
        let json =
            serde_json::to_string(event).map_err(|e| RhError::OperationFailed(e.to_string()))?;
        conn.execute(
            "INSERT INTO rh_audit_outbox (event_json, created_at, delivered) VALUES (?1, ?2, 0)",
            params![json, event.occurred_at.to_rfc3339()],
        )
        .map_err(op)?;
        Ok(())
    }

    fn drain_audit_outbox(&self, audit: &dyn RhAuditPort) -> Result<usize, RhError> {
        self.drain_rh_audit(|json| {
            let event: RhAuditEvent =
                serde_json::from_str(json).map_err(|e| RhError::OperationFailed(e.to_string()))?;
            audit.record(&event)
        })
    }

    fn pending_audit_count(&self) -> Result<u64, RhError> {
        self.count_outbox(0)
    }

    fn dead_letter_audit_count(&self) -> Result<u64, RhError> {
        self.count_outbox(2)
    }
}

// ── UserRepository ────────────────────────────────────────────────────────────

impl UserRepository for UsersSqliteStore {
    fn get_by_id(&self, user_id: &UserId) -> Result<Option<UserIdentity>, RhError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT UserId, Username, DisplayName, Email, Role
                 FROM local_user WHERE UserId = ?1 AND IsActive = 1",
            )
            .map_err(op)?;
        stmt.query_row([user_id.as_str()], decode_user)
            .optional()
            .map_err(op)?
            .map(Ok)
            .transpose()
            .map_err(|e: UsersSqliteError| RhError::OperationFailed(e.to_string()))
    }

    fn get_by_username(&self, username: &str) -> Result<Option<UserIdentity>, RhError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT UserId, Username, DisplayName, Email, Role
                 FROM local_user WHERE Username = ?1 AND IsActive = 1",
            )
            .map_err(op)?;
        stmt.query_row([username], decode_user)
            .optional()
            .map_err(op)?
            .map(Ok)
            .transpose()
            .map_err(|e: UsersSqliteError| RhError::OperationFailed(e.to_string()))
    }

    fn list_active(&self) -> Result<Vec<UserIdentity>, RhError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT UserId, Username, DisplayName, Email, Role
                 FROM local_user WHERE IsActive = 1
                 ORDER BY DisplayName ASC, Username ASC",
            )
            .map_err(op)?;
        let rows = stmt.query_map([], decode_user).map_err(op)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(op)
    }

    fn upsert(&self, user: &UserIdentity) -> Result<(), RhError> {
        user.validate()?;
        let conn = self.lock()?;
        conn.execute(
            r#"INSERT INTO local_user (UserId, Username, DisplayName, Email, Role, IsActive)
               VALUES (?1, ?2, ?3, ?4, ?5, 1)
               ON CONFLICT(UserId) DO UPDATE SET
                   Username    = excluded.Username,
                   DisplayName = excluded.DisplayName,
                   Email       = excluded.Email,
                   Role        = excluded.Role,
                   IsActive    = 1"#,
            params![
                user.user_id,
                user.username,
                user.display_name,
                user.email,
                encode_user_role(&user.role),
            ],
        )
        .map(|_| ())
        .map_err(op)
    }

    fn upsert_audited(&self, user: &UserIdentity, event: &RhAuditEvent) -> Result<(), RhError> {
        user.validate()?;
        let mut conn = self.lock()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(op)?;
        tx.execute(
            r#"INSERT INTO local_user (UserId, Username, DisplayName, Email, Role, IsActive)
               VALUES (?1, ?2, ?3, ?4, ?5, 1)
               ON CONFLICT(UserId) DO UPDATE SET
                   Username    = excluded.Username,
                   DisplayName = excluded.DisplayName,
                   Email       = excluded.Email,
                   Role        = excluded.Role,
                   IsActive    = 1"#,
            params![
                user.user_id,
                user.username,
                user.display_name,
                user.email,
                encode_user_role(&user.role),
            ],
        )
        .map_err(op)?;
        enqueue_audit_in_tx(&tx, event)?;
        tx.commit().map_err(op)?;
        Ok(())
    }

    fn deactivate(&self, user_id: &UserId) -> Result<(), RhError> {
        let mut conn = self.lock()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(op)?;
        let changed = tx
            .execute(
                "UPDATE local_user SET IsActive = 0 WHERE UserId = ?1",
                [user_id.as_str()],
            )
            .map_err(op)?;
        if changed == 0 {
            return Err(RhError::UserNotFound(user_id.as_str().to_owned()));
        }
        tx.execute(
            "DELETE FROM current_user_context WHERE UserId = ?1",
            [user_id.as_str()],
        )
        .map_err(op)?;
        tx.commit().map_err(op)?;
        Ok(())
    }

    fn deactivate_audited(&self, user_id: &UserId, event: &RhAuditEvent) -> Result<(), RhError> {
        let mut conn = self.lock()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(op)?;
        let changed = tx
            .execute(
                "UPDATE local_user SET IsActive = 0 WHERE UserId = ?1",
                [user_id.as_str()],
            )
            .map_err(op)?;
        if changed == 0 {
            return Err(RhError::UserNotFound(user_id.as_str().to_owned()));
        }
        tx.execute(
            "DELETE FROM current_user_context WHERE UserId = ?1",
            [user_id.as_str()],
        )
        .map_err(op)?;
        enqueue_audit_in_tx(&tx, event)?;
        tx.commit().map_err(op)?;
        Ok(())
    }
}

// ── PersonAssignmentRepository ────────────────────────────────────────────────

impl PersonAssignmentRepository for UsersSqliteStore {
    fn get(&self, id: &PersonAssignmentId) -> Result<Option<PersonAssignment>, RhError> {
        let conn = self.lock()?;
        conn.query_row(
            "SELECT assignment_id, person_id, position_id, unit_id, basis,
                    valid_from, valid_until, version
             FROM person_assignment WHERE assignment_id = ?1",
            params![id.as_str()],
            decode_assignment,
        )
        .optional()
        .map_err(op)
    }

    fn find_at(
        &self,
        person_id: &UserId,
        date: NaiveDate,
    ) -> Result<Option<PersonAssignment>, RhError> {
        let date_s = encode_date(date);
        let conn = self.lock()?;
        conn.query_row(
            "SELECT assignment_id, person_id, position_id, unit_id, basis,
                    valid_from, valid_until, version
             FROM person_assignment
             WHERE person_id = ?1
               AND valid_from <= ?2
               AND (valid_until IS NULL OR valid_until > ?2)",
            params![person_id.as_str(), date_s],
            decode_assignment,
        )
        .optional()
        .map_err(op)
    }

    fn find_holder_at(
        &self,
        position_id: &str,
        date: NaiveDate,
    ) -> Result<Option<PersonAssignment>, RhError> {
        let date_s = encode_date(date);
        let conn = self.lock()?;
        conn.query_row(
            "SELECT assignment_id, person_id, position_id, unit_id, basis,
                    valid_from, valid_until, version
             FROM person_assignment
             WHERE position_id = ?1
               AND valid_from <= ?2
               AND (valid_until IS NULL OR valid_until > ?2)",
            params![position_id, date_s],
            decode_assignment,
        )
        .optional()
        .map_err(op)
    }

    fn list_active_for_person(
        &self,
        person_id: &UserId,
        as_of: NaiveDate,
    ) -> Result<Vec<PersonAssignment>, RhError> {
        let as_of_s = encode_date(as_of);
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT assignment_id, person_id, position_id, unit_id, basis,
                        valid_from, valid_until, version
                 FROM person_assignment
                 WHERE person_id = ?1
                   AND (valid_until IS NULL OR valid_until > ?2)
                 ORDER BY valid_from DESC",
            )
            .map_err(op)?;
        let rows = stmt
            .query_map(params![person_id.as_str(), as_of_s], decode_assignment)
            .map_err(op)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(op)
    }

    fn has_overlap(
        &self,
        person_id: &UserId,
        valid_from: NaiveDate,
        valid_until: Option<NaiveDate>,
        exclude_id: Option<&PersonAssignmentId>,
    ) -> Result<bool, RhError> {
        let from_s = encode_date(valid_from);
        let until_s = valid_until
            .map(encode_date)
            .unwrap_or_else(|| "9999-12-31".into());
        let exclude = exclude_id.map(|id| id.as_str().to_owned());
        let conn = self.lock()?;
        let found: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM person_assignment
                 WHERE person_id = ?1
                   AND valid_from < ?2
                   AND (valid_until IS NULL OR valid_until > ?3)
                   AND (?4 IS NULL OR assignment_id != ?4)",
                params![person_id.as_str(), until_s, from_s, exclude],
                |row| row.get(0),
            )
            .map_err(op)?;
        Ok(found)
    }

    fn upsert(&self, a: &PersonAssignment) -> Result<(), RhError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO person_assignment
                (assignment_id, person_id, position_id, unit_id, basis,
                 valid_from, valid_until, version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(assignment_id) DO UPDATE SET
                 person_id   = excluded.person_id,
                 position_id = excluded.position_id,
                 unit_id     = excluded.unit_id,
                 basis       = excluded.basis,
                 valid_from  = excluded.valid_from,
                 valid_until = excluded.valid_until,
                 version     = excluded.version",
            params![
                a.id.as_str(),
                a.person_id.as_str(),
                a.position_id,
                a.unit_id,
                a.basis,
                encode_date(a.valid_from),
                a.valid_until.map(encode_date),
                a.version as i64,
            ],
        )
        .map(|_| ())
        .map_err(op)
    }

    fn upsert_audited(&self, a: &PersonAssignment, event: &RhAuditEvent) -> Result<(), RhError> {
        let mut conn = self.lock()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(op)?;
        tx.execute(
            "INSERT INTO person_assignment
                (assignment_id, person_id, position_id, unit_id, basis,
                 valid_from, valid_until, version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(assignment_id) DO UPDATE SET
                 person_id   = excluded.person_id,
                 position_id = excluded.position_id,
                 unit_id     = excluded.unit_id,
                 basis       = excluded.basis,
                 valid_from  = excluded.valid_from,
                 valid_until = excluded.valid_until,
                 version     = excluded.version",
            params![
                a.id.as_str(),
                a.person_id.as_str(),
                a.position_id,
                a.unit_id,
                a.basis,
                encode_date(a.valid_from),
                a.valid_until.map(encode_date),
                a.version as i64,
            ],
        )
        .map_err(op)?;
        enqueue_audit_in_tx(&tx, event)?;
        tx.commit().map_err(op)?;
        Ok(())
    }

    fn close(
        &self,
        id: &PersonAssignmentId,
        valid_until: NaiveDate,
        version: u32,
    ) -> Result<(), RhError> {
        let conn = self.lock()?;
        let changed = conn
            .execute(
                "UPDATE person_assignment
                 SET valid_until = ?1, version = version + 1
                 WHERE assignment_id = ?2 AND version = ?3",
                params![encode_date(valid_until), id.as_str(), version as i64],
            )
            .map_err(op)?;
        if changed == 0 {
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM person_assignment WHERE assignment_id = ?1",
                    params![id.as_str()],
                    |r| r.get(0),
                )
                .map_err(op)?;
            return if exists {
                Err(RhError::OperationFailed("versão em conflito".into()))
            } else {
                Err(RhError::AssignmentNotFound(id.as_str().to_owned()))
            };
        }
        Ok(())
    }

    fn close_audited(
        &self,
        id: &PersonAssignmentId,
        valid_until: NaiveDate,
        version: u32,
        event: &RhAuditEvent,
    ) -> Result<(), RhError> {
        let mut conn = self.lock()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(op)?;
        let changed = tx
            .execute(
                "UPDATE person_assignment
                 SET valid_until = ?1, version = version + 1
                 WHERE assignment_id = ?2 AND version = ?3",
                params![encode_date(valid_until), id.as_str(), version as i64],
            )
            .map_err(op)?;
        if changed == 0 {
            // rollback implícito ao drop de tx
            let exists: bool = tx
                .query_row(
                    "SELECT COUNT(*) > 0 FROM person_assignment WHERE assignment_id = ?1",
                    params![id.as_str()],
                    |r| r.get(0),
                )
                .map_err(op)?;
            return if exists {
                Err(RhError::OperationFailed("versão em conflito".into()))
            } else {
                Err(RhError::AssignmentNotFound(id.as_str().to_owned()))
            };
        }
        enqueue_audit_in_tx(&tx, event)?;
        tx.commit().map_err(op)?;
        Ok(())
    }
}

// ── RoleRepository ────────────────────────────────────────────────────────────

impl RoleRepository for UsersSqliteStore {
    type Error = UsersSqliteError;

    fn get(&self, id: &RoleId) -> Result<Option<Role>, Self::Error> {
        let raw: Option<(String, String, Option<String>, i64)> = self
            .lock_raw()?
            .query_row(
                "SELECT role_id, name, description, is_active
                 FROM platform_roles WHERE role_id = ?1",
                params![id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;

        match raw {
            Some((rid, name, desc, active)) => Ok(Some(Role::new(rid, name, desc, active != 0)?)),
            None => Ok(None),
        }
    }

    fn list_active(&self) -> Result<Vec<Role>, Self::Error> {
        let conn = self.lock_raw()?;
        let mut stmt = conn.prepare(
            "SELECT role_id, name, description, is_active
             FROM platform_roles WHERE is_active = 1 ORDER BY name ASC",
        )?;
        let raw: Vec<(String, String, Option<String>, i64)> = stmt
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        raw.into_iter()
            .map(|(id, name, desc, active)| {
                Role::new(id, name, desc, active != 0).map_err(UsersSqliteError::Rh)
            })
            .collect()
    }

    fn exists_and_active(&self, id: &RoleId) -> Result<bool, Self::Error> {
        let found: bool = self.lock_raw()?.query_row(
            "SELECT COUNT(*) > 0 FROM platform_roles
             WHERE role_id = ?1 AND is_active = 1",
            params![id.as_str()],
            |row| row.get(0),
        )?;
        Ok(found)
    }

    fn upsert(&self, role: &Role) -> Result<(), Self::Error> {
        role.validate().map_err(UsersSqliteError::Rh)?;
        self.lock_raw()?.execute(
            "INSERT INTO platform_roles (role_id, name, description, is_active)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(role_id) DO UPDATE SET
                 name        = excluded.name,
                 description = excluded.description,
                 is_active   = excluded.is_active",
            params![
                role.id.as_str(),
                role.name,
                role.description,
                role.is_active as i64,
            ],
        )?;
        Ok(())
    }

    fn deactivate(&self, id: &RoleId) -> Result<(), Self::Error> {
        let changed = self.lock_raw()?.execute(
            "UPDATE platform_roles SET is_active = 0 WHERE role_id = ?1",
            params![id.as_str()],
        )?;
        if changed == 0 {
            return Err(UsersSqliteError::Rh(RhError::RoleNotFound(
                id.as_str().to_owned(),
            )));
        }
        Ok(())
    }
}
