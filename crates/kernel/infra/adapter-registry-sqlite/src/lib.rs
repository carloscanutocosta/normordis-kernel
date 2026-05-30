/*!
 * Adapter SQLite para o catálogo institucional de apps (domain-registry).
 *
 * Tabelas:
 *   platform_app_registry          — registo base (PK: app_id)
 *   platform_app_state_transitions — histórico de estados datado (append-only)
 *   platform_app_allowed_roles     — roles com acesso à app (many-to-many)
 *
 * Roles: lista vazia = acesso livre. Lista não-vazia = acesso restrito
 * a utilizadores com pelo menos um dos roles listados.
 */

use chrono::{DateTime, Utc};
use rusqlite::{params, types::Value, Connection, OptionalExtension};
use thiserror::Error;

use adapter_sqlite::{open_relational_connection, run_relational_migrations, SqliteRelationalConfig};
use domain_registry::{
    AppId, AppRegistration, AppRegistryFilter, AppRegistryRepository,
    AppState, AppStateTransition, AppVisibility, RegisterAppRequest,
    RegistryError, RoleId, TransitionStateRequest, UpdateAppMetadataRequest,
};
use support_errors::MiniError;

// ─── Migrations ───────────────────────────────────────────────────────────────

pub const REGISTRY_MIGRATIONS: &[&str] = &[
    // Migração 1 — catálogo e histórico de estados
    r#"
    CREATE TABLE IF NOT EXISTS platform_app_registry (
        app_id        TEXT    NOT NULL PRIMARY KEY,
        name          TEXT    NOT NULL,
        version       TEXT    NOT NULL,
        owner         TEXT    NOT NULL,
        domain        TEXT    NOT NULL,
        description   TEXT,
        capabilities  TEXT    NOT NULL DEFAULT '[]',
        visibility    TEXT    NOT NULL DEFAULT 'Internal',
        registered_at TEXT    NOT NULL,
        registered_by TEXT    NOT NULL
    );

    CREATE TABLE IF NOT EXISTS platform_app_state_transitions (
        app_id            TEXT NOT NULL REFERENCES platform_app_registry(app_id),
        state             TEXT NOT NULL,
        transitioned_at   TEXT NOT NULL,
        transitioned_by   TEXT NOT NULL,
        reason            TEXT,
        PRIMARY KEY (app_id, transitioned_at)
    );

    CREATE INDEX IF NOT EXISTS idx_registry_state_latest
        ON platform_app_state_transitions (app_id, transitioned_at DESC);

    CREATE INDEX IF NOT EXISTS idx_registry_domain
        ON platform_app_registry (domain);
    "#,
    // Migração 2 — roles com acesso por app
    r#"
    CREATE TABLE IF NOT EXISTS platform_app_allowed_roles (
        app_id  TEXT NOT NULL REFERENCES platform_app_registry(app_id),
        role_id TEXT NOT NULL,
        PRIMARY KEY (app_id, role_id)
    );

    CREATE INDEX IF NOT EXISTS idx_app_roles_role
        ON platform_app_allowed_roles (role_id);
    "#,
    // Migração 3 — audit trail das mudanças de roles (append-only)
    r#"
    CREATE TABLE IF NOT EXISTS platform_app_role_changes (
        change_id  INTEGER PRIMARY KEY AUTOINCREMENT,
        app_id     TEXT NOT NULL,
        roles_json TEXT NOT NULL,
        set_by     TEXT NOT NULL,
        set_at     TEXT NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_app_role_changes_app
        ON platform_app_role_changes (app_id, set_at DESC);
    "#,
    // Migração 4 — audit trail das mudanças de metadados (append-only, por campo)
    r#"
    CREATE TABLE IF NOT EXISTS platform_app_metadata_changes (
        change_id  INTEGER PRIMARY KEY AUTOINCREMENT,
        app_id     TEXT NOT NULL,
        field      TEXT NOT NULL,
        old_value  TEXT,
        new_value  TEXT,
        changed_by TEXT NOT NULL,
        changed_at TEXT NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_app_metadata_changes_app
        ON platform_app_metadata_changes (app_id, changed_at DESC);
    "#,
];

// ─── Erros ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum RegistrySqliteError {
    #[error(transparent)]
    Adapter(#[from] MiniError),
    #[error("erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("erro JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Domain(#[from] RegistryError),
}

// ─── Store ────────────────────────────────────────────────────────────────────

pub struct RegistrySqliteStore {
    conn: Connection,
}

impl RegistrySqliteStore {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, RegistrySqliteError> {
        let conn = open_relational_connection(config)?;
        run_relational_migrations(&conn, REGISTRY_MIGRATIONS)?;
        Ok(Self { conn })
    }
}

/// Registo de auditoria de uma mudança ao conjunto de roles de uma app.
/// Cada chamada a `set_allowed_roles` produz um destes registos (append-only).
#[derive(Debug, Clone)]
pub struct RoleChangeRecord {
    pub roles:  Vec<RoleId>,
    pub set_by: String,
    pub set_at: DateTime<Utc>,
}

/// Registo de auditoria de uma mudança a um campo de metadados de uma app.
/// Cada campo efectivamente alterado por `update_metadata` produz um destes
/// registos (append-only). `None` representa ausência de valor (campo nulo).
#[derive(Debug, Clone)]
pub struct MetadataChangeRecord {
    pub field:      String,
    pub old_value:  Option<String>,
    pub new_value:  Option<String>,
    pub changed_by: String,
    pub changed_at: DateTime<Utc>,
}

// ─── Helpers internos ─────────────────────────────────────────────────────────

fn decode_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

/// Escapa os metacaracteres LIKE (`\`, `%`, `_`) para que uma pesquisa por
/// substring trate esses caracteres literalmente. Usar com `ESCAPE '\'`.
fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

struct AppRow {
    app_id:            String,
    name:              String,
    version:           String,
    owner:             String,
    domain:            String,
    description:       Option<String>,
    capabilities_json: String,
    visibility_str:    String,
    registered_at_str: String,
    registered_by:     String,
}

const SELECT_COLS: &str =
    "app_id, name, version, owner, domain, description, \
     capabilities, visibility, registered_at, registered_by";

fn row_to_app_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AppRow> {
    Ok(AppRow {
        app_id:            row.get(0)?,
        name:              row.get(1)?,
        version:           row.get(2)?,
        owner:             row.get(3)?,
        domain:            row.get(4)?,
        description:       row.get(5)?,
        capabilities_json: row.get(6)?,
        visibility_str:    row.get(7)?,
        registered_at_str: row.get(8)?,
        registered_by:     row.get(9)?,
    })
}

fn build_registration(
    row: AppRow,
    state_history: Vec<AppStateTransition>,
    allowed_roles: Vec<RoleId>,
) -> Result<AppRegistration, RegistrySqliteError> {
    let capabilities: Vec<String> = serde_json::from_str(&row.capabilities_json)?;
    let visibility = AppVisibility::from_str(&row.visibility_str)?;
    let id = AppId::new(row.app_id)?;
    Ok(AppRegistration {
        id,
        name:          row.name,
        version:       row.version,
        owner:         row.owner,
        domain:        row.domain,
        description:   row.description,
        capabilities,
        visibility,
        allowed_roles,
        registered_at: decode_datetime(&row.registered_at_str),
        registered_by: row.registered_by,
        state_history,
    })
}

fn load_transitions_for(
    conn: &Connection,
    app_id: &str,
) -> Result<Vec<AppStateTransition>, RegistrySqliteError> {
    let mut stmt = conn.prepare(
        "SELECT state, transitioned_at, transitioned_by, reason \
         FROM platform_app_state_transitions \
         WHERE app_id = ?1 ORDER BY transitioned_at ASC",
    )?;
    let raw: Vec<(String, String, String, Option<String>)> = stmt
        .query_map(params![app_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    raw.into_iter()
        .map(|(s, at, by, reason)| {
            let state = AppState::from_str(&s)?;
            Ok(AppStateTransition {
                state,
                transitioned_at: decode_datetime(&at),
                transitioned_by: by,
                reason,
            })
        })
        .collect()
}

fn load_roles_for(
    conn: &Connection,
    app_id: &str,
) -> Result<Vec<RoleId>, RegistrySqliteError> {
    let mut stmt = conn.prepare(
        "SELECT role_id FROM platform_app_allowed_roles WHERE app_id = ?1",
    )?;
    let raw: Vec<String> = stmt
        .query_map(params![app_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    raw.into_iter()
        .map(|s| RoleId::new(s).map_err(|e| {
            RegistrySqliteError::Domain(RegistryError::Storage(e.to_string()))
        }))
        .collect()
}

/// Insere roles para uma app dentro de uma transacção já aberta (SAVEPOINT).
fn insert_roles(
    conn: &Connection,
    app_id: &str,
    roles: &[RoleId],
) -> Result<(), RegistrySqliteError> {
    for role in roles {
        conn.execute(
            "INSERT OR IGNORE INTO platform_app_allowed_roles (app_id, role_id) VALUES (?1, ?2)",
            params![app_id, role.as_str()],
        )?;
    }
    Ok(())
}

/// Regista uma entrada no audit trail de mudanças de roles (snapshot do conjunto).
fn insert_role_change_log(
    conn: &Connection,
    app_id: &str,
    roles: &[RoleId],
    set_by: &str,
    set_at: DateTime<Utc>,
) -> Result<(), RegistrySqliteError> {
    let roles_json = serde_json::to_string(
        &roles.iter().map(RoleId::as_str).collect::<Vec<_>>(),
    )?;
    conn.execute(
        "INSERT INTO platform_app_role_changes (app_id, roles_json, set_by, set_at) \
         VALUES (?1, ?2, ?3, ?4)",
        params![app_id, roles_json, set_by, set_at.to_rfc3339()],
    )?;
    Ok(())
}

/// Verdadeiro se o erro for `SQLITE_BUSY`/`SQLITE_LOCKED` — candidato a retry.
fn is_busy(e: &RegistrySqliteError) -> bool {
    matches!(
        e,
        RegistrySqliteError::Sqlite(rusqlite::Error::SqliteFailure(err, _))
        if matches!(
            err.code,
            rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
        )
    )
}

/// Executa `op` com retry sob contenção de escrita (backoff exponencial 20→640ms,
/// máx 5 tentativas). Alinha com o padrão de `numerador-sqlite` para deployment
/// multi-processo (workspace + ferramenta administrativa na mesma DB).
fn with_busy_retry<T>(
    mut op: impl FnMut() -> Result<T, RegistrySqliteError>,
) -> Result<T, RegistrySqliteError> {
    const MAX_RETRIES: u32 = 5;
    let mut attempt = 0u32;
    loop {
        match op() {
            Ok(v) => return Ok(v),
            Err(e) if is_busy(&e) && attempt < MAX_RETRIES => {
                attempt += 1;
                let delay = std::cmp::min(20_u64 * (1 << attempt), 640);
                std::thread::sleep(std::time::Duration::from_millis(delay));
            }
            Err(e) => return Err(e),
        }
    }
}

// ─── AppRegistryRepository impl ───────────────────────────────────────────────

impl AppRegistryRepository for RegistrySqliteStore {
    type Error = RegistrySqliteError;

    fn register(
        &self,
        request: &RegisterAppRequest,
        registered_at: DateTime<Utc>,
    ) -> Result<(), Self::Error> {
        let capabilities_json = serde_json::to_string(&request.capabilities)?;
        let at_str = registered_at.to_rfc3339();

        with_busy_retry(|| {
            self.conn.execute_batch("SAVEPOINT sp_register")?;

            let outcome: Result<(), RegistrySqliteError> = (|| {
                self.conn.execute(
                    &format!("INSERT INTO platform_app_registry ({SELECT_COLS}) \
                              VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)"),
                    params![
                        request.id.as_str(), request.name, request.version,
                        request.owner, request.domain, request.description,
                        capabilities_json, request.visibility.as_str(),
                        at_str, request.registered_by,
                    ],
                )?;
                self.conn.execute(
                    "INSERT INTO platform_app_state_transitions \
                     (app_id, state, transitioned_at, transitioned_by, reason) \
                     VALUES (?1, ?2, ?3, ?4, NULL)",
                    params![
                        request.id.as_str(), AppState::Draft.as_str(),
                        at_str, request.registered_by,
                    ],
                )?;
                insert_roles(&self.conn, request.id.as_str(), &request.allowed_roles)?;
                // Audit baseline: regista a concessão inicial de acesso (mesmo que vazia)
                // para que o role_change_log tenha a origem completa do histórico.
                insert_role_change_log(
                    &self.conn,
                    request.id.as_str(),
                    &request.allowed_roles,
                    &request.registered_by,
                    registered_at,
                )?;
                Ok(())
            })();

            match outcome {
                Ok(()) => self.conn.execute_batch("RELEASE SAVEPOINT sp_register").map_err(Into::into),
                Err(e) => {
                    let _ = self.conn.execute_batch("ROLLBACK TO SAVEPOINT sp_register");
                    let _ = self.conn.execute_batch("RELEASE SAVEPOINT sp_register");
                    Err(e)
                }
            }
        })
    }

    fn transition(
        &self,
        request: &TransitionStateRequest,
        transitioned_at: DateTime<Utc>,
    ) -> Result<(), Self::Error> {
        self.conn.execute(
            "INSERT INTO platform_app_state_transitions \
             (app_id, state, transitioned_at, transitioned_by, reason) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                request.app_id.as_str(), request.to_state.as_str(),
                transitioned_at.to_rfc3339(), request.transitioned_by, request.reason,
            ],
        )?;
        Ok(())
    }

    fn update_metadata(
        &self,
        request: &UpdateAppMetadataRequest,
        updated_at: DateTime<Utc>,
    ) -> Result<(), Self::Error> {
        // Nada a alterar — evita abrir transacção desnecessária.
        if request.version.is_none()
            && request.description.is_none()
            && request.capabilities.is_none()
            && request.visibility.is_none()
            && request.owner.is_none()
        {
            return Ok(());
        }

        with_busy_retry(|| {
            self.conn.execute_batch("SAVEPOINT sp_meta")?;

            let outcome: Result<(), RegistrySqliteError> = (|| {
                // Snapshot dos valores actuais para comparar e registar old→new.
                let snap: Option<(String, Option<String>, String, String, String)> = self
                    .conn
                    .query_row(
                        "SELECT version, description, capabilities, visibility, owner \
                         FROM platform_app_registry WHERE app_id = ?1",
                        params![request.app_id.as_str()],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                    )
                    .optional()?;

                // App não existe — no-op (o serviço já garante existência).
                let Some((cur_version, cur_desc, cur_caps, cur_vis, cur_owner)) = snap else {
                    return Ok(());
                };

                let mut set_parts: Vec<String> = Vec::new();
                let mut values: Vec<Value> = Vec::new();
                // (campo, old, new) — apenas mudanças efectivas.
                let mut audits: Vec<(&'static str, Option<String>, Option<String>)> = Vec::new();
                let mut idx = 1usize;

                if let Some(v) = &request.version {
                    if *v != cur_version {
                        audits.push(("version", Some(cur_version.clone()), Some(v.clone())));
                    }
                    set_parts.push(format!("version = ?{idx}"));
                    values.push(Value::Text(v.clone()));
                    idx += 1;
                }
                if let Some(d) = &request.description {
                    if *d != cur_desc {
                        audits.push(("description", cur_desc.clone(), d.clone()));
                    }
                    set_parts.push(format!("description = ?{idx}"));
                    values.push(match d { Some(s) => Value::Text(s.clone()), None => Value::Null });
                    idx += 1;
                }
                if let Some(caps) = &request.capabilities {
                    let new_json = serde_json::to_string(caps)?;
                    if new_json != cur_caps {
                        audits.push(("capabilities", Some(cur_caps.clone()), Some(new_json.clone())));
                    }
                    set_parts.push(format!("capabilities = ?{idx}"));
                    values.push(Value::Text(new_json));
                    idx += 1;
                }
                if let Some(vis) = &request.visibility {
                    let new_vis = vis.as_str().to_owned();
                    if new_vis != cur_vis {
                        audits.push(("visibility", Some(cur_vis.clone()), Some(new_vis.clone())));
                    }
                    set_parts.push(format!("visibility = ?{idx}"));
                    values.push(Value::Text(new_vis));
                    idx += 1;
                }
                if let Some(owner) = &request.owner {
                    if *owner != cur_owner {
                        audits.push(("owner", Some(cur_owner.clone()), Some(owner.clone())));
                    }
                    set_parts.push(format!("owner = ?{idx}"));
                    values.push(Value::Text(owner.clone()));
                    idx += 1;
                }

                if !set_parts.is_empty() {
                    values.push(Value::Text(request.app_id.as_str().to_owned()));
                    let sql = format!(
                        "UPDATE platform_app_registry SET {} WHERE app_id = ?{idx}",
                        set_parts.join(", ")
                    );
                    let params_refs: Vec<&dyn rusqlite::ToSql> =
                        values.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
                    self.conn.execute(&sql, params_refs.as_slice())?;
                }

                // Audit trail: uma linha por campo efectivamente alterado.
                let at_str = updated_at.to_rfc3339();
                for (field, old, new) in &audits {
                    self.conn.execute(
                        "INSERT INTO platform_app_metadata_changes \
                         (app_id, field, old_value, new_value, changed_by, changed_at) \
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![request.app_id.as_str(), field, old, new, request.updated_by, at_str],
                    )?;
                }
                Ok(())
            })();

            match outcome {
                Ok(()) => self.conn.execute_batch("RELEASE SAVEPOINT sp_meta").map_err(Into::into),
                Err(e) => {
                    let _ = self.conn.execute_batch("ROLLBACK TO SAVEPOINT sp_meta");
                    let _ = self.conn.execute_batch("RELEASE SAVEPOINT sp_meta");
                    Err(e)
                }
            }
        })
    }

    fn set_allowed_roles(
        &self,
        app_id: &AppId,
        roles: &[RoleId],
        set_by: &str,
        set_at: DateTime<Utc>,
    ) -> Result<(), Self::Error> {
        with_busy_retry(|| {
            self.conn.execute_batch("SAVEPOINT sp_roles")?;

            let outcome: Result<(), RegistrySqliteError> = (|| {
                self.conn.execute(
                    "DELETE FROM platform_app_allowed_roles WHERE app_id = ?1",
                    params![app_id.as_str()],
                )?;
                insert_roles(&self.conn, app_id.as_str(), roles)?;
                // Audit trail: regista quem mudou, quando e o conjunto resultante.
                insert_role_change_log(&self.conn, app_id.as_str(), roles, set_by, set_at)?;
                Ok(())
            })();

            match outcome {
                Ok(()) => self.conn.execute_batch("RELEASE SAVEPOINT sp_roles").map_err(Into::into),
                Err(e) => {
                    let _ = self.conn.execute_batch("ROLLBACK TO SAVEPOINT sp_roles");
                    let _ = self.conn.execute_batch("RELEASE SAVEPOINT sp_roles");
                    Err(e)
                }
            }
        })
    }

    fn list_for_roles(
        &self,
        roles: &[RoleId],
        limit: usize,
    ) -> Result<Vec<AppRegistration>, Self::Error> {
        // Apenas apps em estado Active aparecem no menu de utilizador —
        // Draft/Suspended/Deprecated/Retired não são navegáveis.
        const ACTIVE_FILTER: &str = "(\
            SELECT state FROM platform_app_state_transitions \
            WHERE app_id = r.app_id ORDER BY transitioned_at DESC LIMIT 1\
        ) = 'Active'";

        let sql = if roles.is_empty() {
            // Sem roles: só apps activas e sem restrição.
            format!(
                "SELECT {SELECT_COLS} FROM platform_app_registry r \
                 WHERE {ACTIVE_FILTER} \
                   AND NOT EXISTS (SELECT 1 FROM platform_app_allowed_roles WHERE app_id = r.app_id) \
                 ORDER BY r.name ASC LIMIT ?1"
            )
        } else {
            let placeholders: String = (2..=roles.len() + 1)
                .map(|i| format!("?{i}"))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "SELECT {SELECT_COLS} FROM platform_app_registry r \
                 WHERE {ACTIVE_FILTER} \
                   AND ( NOT EXISTS (SELECT 1 FROM platform_app_allowed_roles WHERE app_id = r.app_id) \
                      OR EXISTS ( \
                          SELECT 1 FROM platform_app_allowed_roles \
                          WHERE app_id = r.app_id AND role_id IN ({placeholders}) \
                      ) ) \
                 ORDER BY r.name ASC LIMIT ?1"
            )
        };

        let mut params_vals: Vec<Value> = vec![Value::Integer(limit as i64)];
        for r in roles {
            params_vals.push(Value::Text(r.as_str().to_owned()));
        }
        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vals.iter().map(|v| v as &dyn rusqlite::ToSql).collect();

        let mut stmt = self.conn.prepare(&sql)?;
        let app_rows: Vec<AppRow> = stmt
            .query_map(params_refs.as_slice(), row_to_app_row)?
            .collect::<Result<Vec<_>, _>>()?;

        self.build_registrations(app_rows)
    }

    fn get(&self, id: &AppId) -> Result<Option<AppRegistration>, Self::Error> {
        let row_opt = self.conn
            .query_row(
                &format!("SELECT {SELECT_COLS} FROM platform_app_registry WHERE app_id = ?1"),
                params![id.as_str()],
                row_to_app_row,
            )
            .optional()?;

        let Some(row) = row_opt else { return Ok(None); };

        let state_history  = load_transitions_for(&self.conn, &row.app_id)?;
        let allowed_roles  = load_roles_for(&self.conn, &row.app_id)?;
        build_registration(row, state_history, allowed_roles).map(Some)
    }

    fn list(
        &self,
        filter: &AppRegistryFilter,
        limit: usize,
    ) -> Result<Vec<AppRegistration>, Self::Error> {
        let state_str = filter.state.as_ref().map(AppState::as_str);
        let name_pattern = filter.name_contains.as_ref()
            .map(|n| format!("%{}%", escape_like(&n.to_lowercase())));

        let sql = format!(
            "SELECT {SELECT_COLS} FROM platform_app_registry r \
             WHERE (?1 IS NULL OR (\
                 SELECT state FROM platform_app_state_transitions \
                 WHERE app_id = r.app_id ORDER BY transitioned_at DESC LIMIT 1\
             ) = ?1) \
             AND (?2 IS NULL OR domain     = ?2) \
             AND (?3 IS NULL OR owner      = ?3) \
             AND (?4 IS NULL OR visibility = ?4) \
             AND (?5 IS NULL OR lower(name) LIKE ?5 ESCAPE '\\') \
             ORDER BY name ASC LIMIT ?6"
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let app_rows: Vec<AppRow> = stmt
            .query_map(
                params![
                    state_str,
                    filter.domain.as_deref(),
                    filter.owner.as_deref(),
                    filter.visibility.as_ref().map(AppVisibility::as_str),
                    name_pattern.as_deref(),
                    limit as i64,
                ],
                row_to_app_row,
            )?
            .collect::<Result<Vec<_>, _>>()?;

        self.build_registrations(app_rows)
    }

    fn state_history(&self, id: &AppId) -> Result<Vec<AppStateTransition>, Self::Error> {
        let exists: bool = self.conn.query_row(
            "SELECT COUNT(*) > 0 FROM platform_app_registry WHERE app_id = ?1",
            params![id.as_str()],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(RegistryError::AppNotFound(id.as_str().to_owned()).into());
        }
        load_transitions_for(&self.conn, id.as_str())
    }

    fn exists(&self, id: &AppId) -> Result<bool, Self::Error> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM platform_app_registry WHERE app_id = ?1",
            params![id.as_str()],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}

impl RegistrySqliteStore {
    /// Histórico de mudanças de roles de uma app, mais recente primeiro.
    /// Cada registo corresponde a uma chamada a `set_allowed_roles`.
    pub fn role_change_log(
        &self,
        app_id: &AppId,
    ) -> Result<Vec<RoleChangeRecord>, RegistrySqliteError> {
        let mut stmt = self.conn.prepare(
            "SELECT roles_json, set_by, set_at FROM platform_app_role_changes \
             WHERE app_id = ?1 ORDER BY set_at DESC, change_id DESC",
        )?;
        let raw: Vec<(String, String, String)> = stmt
            .query_map(params![app_id.as_str()], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        raw.into_iter()
            .map(|(roles_json, set_by, set_at)| {
                let role_strs: Vec<String> = serde_json::from_str(&roles_json)?;
                let roles = role_strs
                    .into_iter()
                    .map(|s| RoleId::new(s).map_err(|e| {
                        RegistrySqliteError::Domain(RegistryError::Storage(e.to_string()))
                    }))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(RoleChangeRecord { roles, set_by, set_at: decode_datetime(&set_at) })
            })
            .collect()
    }

    /// Histórico de mudanças de metadados de uma app, mais recente primeiro.
    /// Uma linha por campo efectivamente alterado em cada `update_metadata`.
    pub fn metadata_change_log(
        &self,
        app_id: &AppId,
    ) -> Result<Vec<MetadataChangeRecord>, RegistrySqliteError> {
        let mut stmt = self.conn.prepare(
            "SELECT field, old_value, new_value, changed_by, changed_at \
             FROM platform_app_metadata_changes \
             WHERE app_id = ?1 ORDER BY changed_at DESC, change_id DESC",
        )?;
        let rows = stmt
            .query_map(params![app_id.as_str()], |row| {
                Ok(MetadataChangeRecord {
                    field:      row.get(0)?,
                    old_value:  row.get(1)?,
                    new_value:  row.get(2)?,
                    changed_by: row.get(3)?,
                    changed_at: decode_datetime(&row.get::<_, String>(4)?),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Carrega transições e roles para uma lista de AppRow em 2 queries batch.
    fn build_registrations(
        &self,
        app_rows: Vec<AppRow>,
    ) -> Result<Vec<AppRegistration>, RegistrySqliteError> {
        if app_rows.is_empty() {
            return Ok(Vec::new());
        }

        let n = app_rows.len();
        let ph: String = (1..=n).map(|i| format!("?{i}")).collect::<Vec<_>>().join(", ");
        let id_strs: Vec<String> = app_rows.iter().map(|r| r.app_id.clone()).collect();
        let id_params: Vec<&dyn rusqlite::ToSql> =
            id_strs.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

        // Transições
        let mut stmt = self.conn.prepare(&format!(
            "SELECT app_id, state, transitioned_at, transitioned_by, reason \
             FROM platform_app_state_transitions \
             WHERE app_id IN ({ph}) ORDER BY app_id, transitioned_at ASC"
        ))?;
        let raw_t: Vec<(String, String, String, String, Option<String>)> = stmt
            .query_map(id_params.as_slice(), |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut by_transitions: std::collections::HashMap<String, Vec<AppStateTransition>> =
            std::collections::HashMap::new();
        for (aid, s, at, by, reason) in raw_t {
            let state = AppState::from_str(&s)?;
            by_transitions.entry(aid).or_default().push(AppStateTransition {
                state, transitioned_at: decode_datetime(&at), transitioned_by: by, reason,
            });
        }

        // Roles
        let mut stmt2 = self.conn.prepare(&format!(
            "SELECT app_id, role_id FROM platform_app_allowed_roles WHERE app_id IN ({ph})"
        ))?;
        let raw_r: Vec<(String, String)> = stmt2
            .query_map(id_params.as_slice(), |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        let mut by_roles: std::collections::HashMap<String, Vec<RoleId>> =
            std::collections::HashMap::new();
        for (aid, rid) in raw_r {
            let role_id = RoleId::new(rid).map_err(|e| {
                RegistrySqliteError::Domain(RegistryError::Storage(e.to_string()))
            })?;
            by_roles.entry(aid).or_default().push(role_id);
        }

        app_rows
            .into_iter()
            .map(|row| {
                let aid = row.app_id.clone();
                let history = by_transitions.remove(&aid).unwrap_or_default();
                let roles   = by_roles.remove(&aid).unwrap_or_default();
                build_registration(row, history, roles)
            })
            .collect()
    }
}

// ─── Testes ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use core_rh::{RhError, Role, RoleRepository};
    use tempfile::tempdir;

    // ── Mocks de RoleRepository para testes ──────────────────────────────────
    //
    // O AppRegistryService valida roles via `get()`, distinguindo:
    //   None              → RoleNotFound
    //   Some(!is_active)  → RoleInactive
    //   Some(is_active)   → ok

    /// Todos os roles existem e estão activos.
    struct AllRolesActive;

    impl RoleRepository for AllRolesActive {
        type Error = RhError;
        fn get(&self, id: &RoleId) -> Result<Option<Role>, RhError> {
            Ok(Some(Role::new(id.as_str(), "Mock Role", None, true)?))
        }
        fn list_active(&self) -> Result<Vec<Role>, RhError> { Ok(vec![]) }
        fn exists_and_active(&self, _: &RoleId) -> Result<bool, RhError> { Ok(true) }
        fn upsert(&self, _: &Role) -> Result<(), RhError> { Ok(()) }
        fn deactivate(&self, _: &RoleId) -> Result<(), RhError> { Ok(()) }
    }

    /// Nenhum role existe.
    struct NoRolesExist;

    impl RoleRepository for NoRolesExist {
        type Error = RhError;
        fn get(&self, _: &RoleId) -> Result<Option<Role>, RhError> { Ok(None) }
        fn list_active(&self) -> Result<Vec<Role>, RhError> { Ok(vec![]) }
        fn exists_and_active(&self, _: &RoleId) -> Result<bool, RhError> { Ok(false) }
        fn upsert(&self, _: &Role) -> Result<(), RhError> { Ok(()) }
        fn deactivate(&self, _: &RoleId) -> Result<(), RhError> { Ok(()) }
    }

    /// O role existe mas está inactivo.
    struct InactiveRoles;

    impl RoleRepository for InactiveRoles {
        type Error = RhError;
        fn get(&self, id: &RoleId) -> Result<Option<Role>, RhError> {
            Ok(Some(Role::new(id.as_str(), "Inactive Role", None, false)?))
        }
        fn list_active(&self) -> Result<Vec<Role>, RhError> { Ok(vec![]) }
        fn exists_and_active(&self, _: &RoleId) -> Result<bool, RhError> { Ok(false) }
        fn upsert(&self, _: &Role) -> Result<(), RhError> { Ok(()) }
        fn deactivate(&self, _: &RoleId) -> Result<(), RhError> { Ok(()) }
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn open_tmp() -> (tempfile::TempDir, RegistrySqliteStore) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.db");
        let store = RegistrySqliteStore::open(
            &SqliteRelationalConfig::read_write_create(&path),
        ).unwrap();
        (dir, store)
    }

    fn aid(s: &str) -> AppId { AppId::new(s).unwrap() }
    fn rid(s: &str) -> RoleId { RoleId::new(s).unwrap() }

    fn req(id: &str, domain: &str) -> RegisterAppRequest {
        RegisterAppRequest {
            id:            aid(id),
            name:          format!("App {id}"),
            version:       "1.0.0".into(),
            owner:         "equipa".into(),
            domain:        domain.into(),
            description:   None,
            capabilities:  vec![],
            visibility:    AppVisibility::Internal,
            allowed_roles: vec![],
            registered_by: "admin".into(),
        }
    }

    fn now() -> DateTime<Utc> { Utc::now() }

    /// Transita uma app Draft → Active (para testes de list_for_roles, que só devolve apps activas).
    fn activate(store: &RegistrySqliteStore, id: &str) {
        store.transition(
            &TransitionStateRequest {
                app_id:          aid(id),
                to_state:        AppState::Active,
                transitioned_by: "admin".into(),
                reason:          None,
            },
            now() + chrono::Duration::milliseconds(1),
        ).unwrap();
    }

    // ── Testes básicos ───────────────────────────────────────────────────────

    #[test]
    fn register_and_get() {
        let (_dir, store) = open_tmp();
        store.register(&req("app-x", "rh"), now()).unwrap();
        let app = store.get(&aid("app-x")).unwrap().unwrap();
        assert_eq!(app.current_state(), Some(&AppState::Draft));
        assert!(app.allowed_roles.is_empty());
    }

    #[test]
    fn register_with_roles_stored() {
        let (_dir, store) = open_tmp();
        let mut r = req("app-r", "rh");
        r.allowed_roles = vec![rid("gestor_rh"), rid("admin")];
        store.register(&r, now()).unwrap();

        let app = store.get(&aid("app-r")).unwrap().unwrap();
        assert_eq!(app.allowed_roles.len(), 2);
        assert!(app.allowed_roles.contains(&rid("gestor_rh")));
        assert!(app.allowed_roles.contains(&rid("admin")));
    }

    #[test]
    fn set_allowed_roles_replaces_previous() {
        let (_dir, store) = open_tmp();
        let mut r = req("app-s", "rh");
        r.allowed_roles = vec![rid("role_a")];
        store.register(&r, now()).unwrap();

        store.set_allowed_roles(&aid("app-s"), &[rid("role_b"), rid("role_c")], "admin", now()).unwrap();

        let app = store.get(&aid("app-s")).unwrap().unwrap();
        assert!(!app.allowed_roles.contains(&rid("role_a")), "role_a deve ter sido removido");
        assert!(app.allowed_roles.contains(&rid("role_b")));
        assert!(app.allowed_roles.contains(&rid("role_c")));
    }

    #[test]
    fn set_allowed_roles_empty_removes_all() {
        let (_dir, store) = open_tmp();
        let mut r = req("app-e", "rh");
        r.allowed_roles = vec![rid("role_x")];
        store.register(&r, now()).unwrap();

        store.set_allowed_roles(&aid("app-e"), &[], "admin", now()).unwrap();

        let app = store.get(&aid("app-e")).unwrap().unwrap();
        assert!(app.allowed_roles.is_empty());
    }

    #[test]
    fn list_for_roles_returns_unrestricted_and_matching() {
        let (_dir, store) = open_tmp();
        // App sem restrição
        store.register(&req("app-open", "rh"), now()).unwrap();
        // App restrita a gestor_rh
        let mut r2 = req("app-restricted", "rh");
        r2.allowed_roles = vec![rid("gestor_rh")];
        store.register(&r2, now()).unwrap();
        // App restrita a outro role
        let mut r3 = req("app-other", "rh");
        r3.allowed_roles = vec![rid("finance")];
        store.register(&r3, now()).unwrap();
        // Só apps activas aparecem no menu
        activate(&store, "app-open");
        activate(&store, "app-restricted");
        activate(&store, "app-other");

        let visible = store.list_for_roles(&[rid("gestor_rh")], 100).unwrap();
        let ids: Vec<&str> = visible.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"app-open"),       "app sem restrição deve aparecer");
        assert!(ids.contains(&"app-restricted"), "app com role do utilizador deve aparecer");
        assert!(!ids.contains(&"app-other"),     "app com role diferente não deve aparecer");
    }

    #[test]
    fn list_for_roles_empty_returns_only_unrestricted() {
        let (_dir, store) = open_tmp();
        store.register(&req("app-open", "rh"), now()).unwrap();
        let mut r2 = req("app-gated", "rh");
        r2.allowed_roles = vec![rid("admin")];
        store.register(&r2, now()).unwrap();
        activate(&store, "app-open");
        activate(&store, "app-gated");

        let visible = store.list_for_roles(&[], 100).unwrap();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id.as_str(), "app-open");
    }

    #[test]
    fn list_for_roles_excludes_non_active_apps() {
        let (_dir, store) = open_tmp();
        // app-draft permanece em Draft; app-live é activada
        store.register(&req("app-draft", "rh"), now()).unwrap();
        store.register(&req("app-live", "rh"), now()).unwrap();
        activate(&store, "app-live");

        // Suspender uma app activada — deixa de ser navegável
        store.register(&req("app-susp", "rh"), now()).unwrap();
        activate(&store, "app-susp");
        store.transition(
            &TransitionStateRequest {
                app_id: aid("app-susp"), to_state: AppState::Suspended,
                transitioned_by: "admin".into(), reason: None,
            },
            now() + chrono::Duration::milliseconds(5),
        ).unwrap();

        let visible = store.list_for_roles(&[], 100).unwrap();
        let ids: Vec<&str> = visible.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, vec!["app-live"], "só apps Active aparecem no menu");
    }

    #[test]
    fn list_with_name_contains_filter() {
        let (_dir, store) = open_tmp();
        let mut r1 = req("app-rh", "rh");   r1.name = "Gestão RH".into();
        let mut r2 = req("app-doc", "rh");  r2.name = "Documental".into();
        store.register(&r1, now()).unwrap();
        store.register(&r2, now()).unwrap();

        let result = store.list(
            &AppRegistryFilter { name_contains: Some("gestão".into()), ..Default::default() },
            100,
        ).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id.as_str(), "app-rh");
    }

    #[test]
    fn service_rejects_unknown_role() {
        use domain_registry::AppRegistryService;
        let (_dir, store) = open_tmp();
        let svc = AppRegistryService::new(store, NoRolesExist);
        let mut r = req("app-u", "rh");
        r.allowed_roles = vec![rid("unknown_role")];
        let err = svc.register(r, now()).unwrap_err();
        assert!(err.to_string().contains("unknown_role"));
    }

    #[test]
    fn service_accepts_valid_roles() {
        use domain_registry::AppRegistryService;
        let (_dir, store) = open_tmp();
        let svc = AppRegistryService::new(store, AllRolesActive);
        let mut r = req("app-v", "rh");
        r.allowed_roles = vec![rid("gestor_rh")];
        svc.register(r, now()).unwrap();
        let app = svc.get(&aid("app-v")).unwrap().unwrap();
        assert_eq!(app.allowed_roles.len(), 1);
    }

    #[test]
    fn service_rejects_inactive_role_distinctly() {
        use domain_registry::AppRegistryService;
        let (_dir, store) = open_tmp();
        let svc = AppRegistryService::new(store, InactiveRoles);
        let mut r = req("app-i", "rh");
        r.allowed_roles = vec![rid("role_desativado")];
        let err = svc.register(r, now()).unwrap_err();
        let msg = err.to_string();
        // Distingue de RoleNotFound: a mensagem indica que o role está inactivo.
        assert!(msg.contains("inactivo"), "esperava erro de role inactivo, obtido: {msg}");
        assert!(msg.contains("role_desativado"));
    }

    #[test]
    fn set_allowed_roles_records_audit_trail() {
        let (_dir, store) = open_tmp();
        // register cria a baseline (roles iniciais vazios) no log.
        store.register(&req("app-audit", "rh"), now()).unwrap();

        let t1 = now() + chrono::Duration::seconds(1);
        store.set_allowed_roles(&aid("app-audit"), &[rid("role_a")], "alice", t1).unwrap();
        let t2 = now() + chrono::Duration::seconds(2);
        store.set_allowed_roles(&aid("app-audit"), &[rid("role_b"), rid("role_c")], "bob", t2).unwrap();

        let log = store.role_change_log(&aid("app-audit")).unwrap();
        assert_eq!(log.len(), 3, "baseline do registo + duas mudanças");
        // Mais recente primeiro
        assert_eq!(log[0].set_by, "bob");
        assert_eq!(log[0].roles.len(), 2);
        assert!(log[0].roles.contains(&rid("role_b")));
        assert_eq!(log[1].set_by, "alice");
        assert_eq!(log[1].roles, vec![rid("role_a")]);
        // Entrada de baseline criada no registo
        assert_eq!(log[2].set_by, "admin");
        assert!(log[2].roles.is_empty(), "baseline sem restrição de role");
    }

    #[test]
    fn register_creates_audit_baseline_with_initial_roles() {
        let (_dir, store) = open_tmp();
        let mut r = req("app-base", "rh");
        r.allowed_roles = vec![rid("gestor_rh")];
        store.register(&r, now()).unwrap();

        // O registo deixa sempre a origem do histórico de acesso no log.
        let log = store.role_change_log(&aid("app-base")).unwrap();
        assert_eq!(log.len(), 1, "baseline criada no registo");
        assert_eq!(log[0].set_by, "admin");
        assert_eq!(log[0].roles, vec![rid("gestor_rh")]);
    }

    // ── Retry sob SQLITE_BUSY ─────────────────────────────────────────────────

    fn busy_err() -> RegistrySqliteError {
        // 5 = SQLITE_BUSY no código primário.
        RegistrySqliteError::Sqlite(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(5),
            Some("database is busy".into()),
        ))
    }

    #[test]
    fn is_busy_detects_sqlite_busy() {
        assert!(is_busy(&busy_err()));
        assert!(!is_busy(&RegistrySqliteError::Domain(RegistryError::AppNotFound("x".into()))));
    }

    #[test]
    fn with_busy_retry_succeeds_first_try() {
        let mut calls = 0;
        let r: Result<i32, _> = with_busy_retry(|| { calls += 1; Ok(7) });
        assert_eq!(r.unwrap(), 7);
        assert_eq!(calls, 1, "sem retries quando a primeira tentativa passa");
    }

    #[test]
    fn with_busy_retry_does_not_retry_non_busy_errors() {
        let mut calls = 0;
        let r: Result<(), _> = with_busy_retry(|| {
            calls += 1;
            Err(RegistrySqliteError::Domain(RegistryError::AppNotFound("x".into())))
        });
        assert!(r.is_err());
        assert_eq!(calls, 1, "erro não-busy não deve ser repetido");
    }

    #[test]
    fn with_busy_retry_recovers_after_transient_busy() {
        let mut calls = 0;
        let r: Result<&str, _> = with_busy_retry(|| {
            calls += 1;
            if calls < 3 { Err(busy_err()) } else { Ok("ok") }
        });
        assert_eq!(r.unwrap(), "ok");
        assert_eq!(calls, 3, "duas falhas busy seguidas de sucesso");
    }

    #[test]
    fn with_busy_retry_exhausts_and_returns_busy() {
        let mut calls = 0;
        let r: Result<(), _> = with_busy_retry(|| { calls += 1; Err(busy_err()) });
        assert!(r.is_err());
        assert!(is_busy(&r.unwrap_err()));
        assert_eq!(calls, 6, "1 inicial + 5 retries");
    }

    #[test]
    fn name_contains_escapes_like_wildcards() {
        let (_dir, store) = open_tmp();
        let mut r1 = req("app-pct", "rh");  r1.name = "Taxa 50% anual".into();
        let mut r2 = req("app-doc", "rh");  r2.name = "Documental".into();
        store.register(&r1, now()).unwrap();
        store.register(&r2, now()).unwrap();

        // Pesquisar "%" literal deve encontrar apenas a app com "%" no nome,
        // não todas as apps (que seria o comportamento se % fosse wildcard).
        let result = store.list(
            &AppRegistryFilter { name_contains: Some("%".into()), ..Default::default() },
            100,
        ).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id.as_str(), "app-pct");
    }

    #[test]
    fn register_and_get_existing_tests() {
        let (_dir, store) = open_tmp();
        assert!(store.get(&aid("nao-existe")).unwrap().is_none());
        store.register(&req("app-y", "doc"), now()).unwrap();
        assert!(store.exists(&aid("app-y")).unwrap());
    }

    #[test]
    fn transition_records_new_state() {
        let (_dir, store) = open_tmp();
        store.register(&req("app-z", "rh"), now()).unwrap();
        store.transition(
            &TransitionStateRequest {
                app_id: aid("app-z"), to_state: AppState::Active,
                transitioned_by: "admin".into(), reason: None,
            },
            now() + chrono::Duration::milliseconds(1),
        ).unwrap();
        let app = store.get(&aid("app-z")).unwrap().unwrap();
        assert_eq!(app.current_state(), Some(&AppState::Active));
    }

    #[test]
    fn list_with_state_and_domain_filter() {
        let (_dir, store) = open_tmp();
        store.register(&req("app-a", "rh"), now()).unwrap();
        store.register(&req("app-b", "doc"), now()).unwrap();
        store.transition(
            &TransitionStateRequest {
                app_id: aid("app-a"), to_state: AppState::Active,
                transitioned_by: "admin".into(), reason: None,
            },
            now() + chrono::Duration::milliseconds(1),
        ).unwrap();

        let active = store.list(
            &AppRegistryFilter { state: Some(AppState::Active), ..Default::default() }, 100,
        ).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id.as_str(), "app-a");
    }

    #[test]
    fn update_metadata_version() {
        let (_dir, store) = open_tmp();
        store.register(&req("app-u", "rh"), now()).unwrap();
        store.update_metadata(&UpdateAppMetadataRequest {
            app_id: aid("app-u"), version: Some("2.0.0".into()),
            description: None, capabilities: None, visibility: None,
            owner: None, updated_by: "admin".into(),
        }, now()).unwrap();
        let app = store.get(&aid("app-u")).unwrap().unwrap();
        assert_eq!(app.version, "2.0.0");
    }

    #[test]
    fn update_metadata_records_audit_per_changed_field() {
        let (_dir, store) = open_tmp();
        store.register(&req("app-m", "rh"), now()).unwrap(); // version 1.0.0, owner "equipa"
        let t = now() + chrono::Duration::seconds(1);

        store.update_metadata(&UpdateAppMetadataRequest {
            app_id:       aid("app-m"),
            version:      Some("2.0.0".into()),
            description:  Some(Some("Nova descrição".into())),
            capabilities: None,
            visibility:   None,
            owner:        Some("nova-equipa".into()),
            updated_by:   "deploy-bot".into(),
        }, t).unwrap();

        let log = store.metadata_change_log(&aid("app-m")).unwrap();
        // 3 campos efectivamente alterados: version, description, owner
        assert_eq!(log.len(), 3);
        assert!(log.iter().all(|c| c.changed_by == "deploy-bot"));

        let by_field = |f: &str| log.iter().find(|c| c.field == f).unwrap();
        let v = by_field("version");
        assert_eq!(v.old_value.as_deref(), Some("1.0.0"));
        assert_eq!(v.new_value.as_deref(), Some("2.0.0"));
        let d = by_field("description");
        assert_eq!(d.old_value, None); // era None
        assert_eq!(d.new_value.as_deref(), Some("Nova descrição"));
        let o = by_field("owner");
        assert_eq!(o.old_value.as_deref(), Some("equipa"));
        assert_eq!(o.new_value.as_deref(), Some("nova-equipa"));
    }

    #[test]
    fn update_metadata_does_not_log_unchanged_values() {
        let (_dir, store) = open_tmp();
        store.register(&req("app-noop", "rh"), now()).unwrap(); // version já é 1.0.0

        // "Alterar" a versão para o mesmo valor — não deve gerar entrada de auditoria.
        store.update_metadata(&UpdateAppMetadataRequest {
            app_id: aid("app-noop"), version: Some("1.0.0".into()),
            description: None, capabilities: None, visibility: None,
            owner: None, updated_by: "admin".into(),
        }, now()).unwrap();

        assert!(store.metadata_change_log(&aid("app-noop")).unwrap().is_empty(),
            "valor inalterado não deve gerar auditoria");
    }

    #[test]
    fn update_metadata_empty_request_is_noop() {
        let (_dir, store) = open_tmp();
        store.register(&req("app-empty", "rh"), now()).unwrap();
        store.update_metadata(&UpdateAppMetadataRequest {
            app_id: aid("app-empty"), version: None, description: None,
            capabilities: None, visibility: None, owner: None,
            updated_by: "admin".into(),
        }, now()).unwrap();
        assert!(store.metadata_change_log(&aid("app-empty")).unwrap().is_empty());
    }

    #[test]
    fn build_registrations_loads_roles_in_batch() {
        let (_dir, store) = open_tmp();
        let mut r1 = req("app-1", "rh");  r1.allowed_roles = vec![rid("role_a")];
        let mut r2 = req("app-2", "rh");  r2.allowed_roles = vec![rid("role_b")];
        store.register(&r1, now()).unwrap();
        store.register(&r2, now()).unwrap();

        let all = store.list(&AppRegistryFilter::default(), 100).unwrap();
        assert_eq!(all.len(), 2);
        let roles_1: Vec<&str> = all.iter().find(|a| a.id.as_str()=="app-1")
            .unwrap().allowed_roles.iter().map(|r| r.as_str()).collect();
        let roles_2: Vec<&str> = all.iter().find(|a| a.id.as_str()=="app-2")
            .unwrap().allowed_roles.iter().map(|r| r.as_str()).collect();
        assert_eq!(roles_1, vec!["role_a"]);
        assert_eq!(roles_2, vec!["role_b"]);
    }
}
