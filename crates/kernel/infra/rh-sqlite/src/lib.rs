use adapter_sqlite::{
    open_relational_connection, run_relational_migrations, SqliteRelationalConfig,
};
use core_rh::{
    resolve_current_user, RhError, Role, RoleId, RoleRepository, UserContext, UserIdentity,
    UserRole,
};
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

pub const RH_SQLITE_MIGRATIONS: &[&str] = &[
    r#"
    CREATE TABLE IF NOT EXISTS local_user (
        UserId TEXT PRIMARY KEY,
        Username TEXT NOT NULL UNIQUE,
        DisplayName TEXT NOT NULL,
        Email TEXT,
        Role TEXT NOT NULL,
        IsActive INTEGER NOT NULL DEFAULT 1
    );

    CREATE TABLE IF NOT EXISTS current_user_context (
        ContextId INTEGER PRIMARY KEY CHECK (ContextId = 1),
        UserId TEXT NOT NULL,
        FOREIGN KEY (UserId) REFERENCES local_user(UserId)
    );

    CREATE INDEX IF NOT EXISTS idx_local_user_username
    ON local_user (Username);
    "#,
    // Migração 2 — catálogo de roles funcionais
    r#"
    CREATE TABLE IF NOT EXISTS platform_roles (
        role_id     TEXT NOT NULL PRIMARY KEY,
        name        TEXT NOT NULL,
        description TEXT,
        is_active   INTEGER NOT NULL DEFAULT 1
    );

    CREATE INDEX IF NOT EXISTS idx_platform_roles_active
        ON platform_roles (is_active);
    "#,
];

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

#[derive(Debug)]
pub struct UsersSqliteStore {
    conn: Connection,
}

impl UsersSqliteStore {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, UsersSqliteError> {
        let conn = open_relational_connection(config)?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn from_connection(conn: Connection) -> Result<Self, UsersSqliteError> {
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn migrate(&self) -> Result<(), UsersSqliteError> {
        run_relational_migrations(&self.conn, RH_SQLITE_MIGRATIONS)?;
        Ok(())
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn upsert_user(&self, user: &UserIdentity) -> Result<(), UsersSqliteError> {
        user.validate()?;
        self.conn.execute(
            r#"
            INSERT INTO local_user (UserId, Username, DisplayName, Email, Role, IsActive)
            VALUES (?1, ?2, ?3, ?4, ?5, 1)
            ON CONFLICT(UserId) DO UPDATE SET
                Username = excluded.Username,
                DisplayName = excluded.DisplayName,
                Email = excluded.Email,
                Role = excluded.Role,
                IsActive = 1
            "#,
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
            r#"
            INSERT INTO local_user (UserId, Username, DisplayName, Email, Role, IsActive)
            VALUES (?1, ?2, ?3, ?4, ?5, 1)
            ON CONFLICT(UserId) DO UPDATE SET
                Username = excluded.Username,
                DisplayName = excluded.DisplayName,
                Email = excluded.Email,
                Role = excluded.Role,
                IsActive = 1
            "#,
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
            r#"
            INSERT INTO current_user_context (ContextId, UserId)
            VALUES (1, ?1)
            ON CONFLICT(ContextId) DO UPDATE SET
                UserId = excluded.UserId
            "#,
            [user_id],
        )?;
        Ok(())
    }

    pub fn clear_current_user_in_tx(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
        tx.execute("DELETE FROM current_user_context", [])?;
        Ok(())
    }

    pub fn get_user_by_id(&self, user_id: &str) -> Result<Option<UserIdentity>, UsersSqliteError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT UserId, Username, DisplayName, Email, Role
            FROM local_user
            WHERE UserId = ?1 AND IsActive = 1
            "#,
        )?;
        let user = stmt.query_row([user_id], decode_user).optional()?;
        Ok(user)
    }

    pub fn get_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserIdentity>, UsersSqliteError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT UserId, Username, DisplayName, Email, Role
            FROM local_user
            WHERE Username = ?1 AND IsActive = 1
            "#,
        )?;
        let user = stmt.query_row([username], decode_user).optional()?;
        Ok(user)
    }

    pub fn list_users(&self) -> Result<Vec<UserIdentity>, UsersSqliteError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT UserId, Username, DisplayName, Email, Role
            FROM local_user
            WHERE IsActive = 1
            ORDER BY DisplayName ASC, Username ASC
            "#,
        )?;
        let rows = stmt.query_map([], decode_user)?;
        let mut users = Vec::new();
        for row in rows {
            users.push(row?);
        }
        Ok(users)
    }

    pub fn deactivate_user(&self, user_id: &str) -> Result<(), UsersSqliteError> {
        let changed = self.conn.execute(
            r#"
            UPDATE local_user
            SET IsActive = 0
            WHERE UserId = ?1
            "#,
            [user_id],
        )?;
        if changed == 0 {
            return Err(UsersSqliteError::UserNotFound(user_id.to_string()));
        }
        self.conn.execute(
            "DELETE FROM current_user_context WHERE UserId = ?1",
            [user_id],
        )?;
        Ok(())
    }

    pub fn set_current_user(&self, user_id: &str) -> Result<UserContext, UsersSqliteError> {
        let user = self
            .get_user_by_id(user_id)?
            .ok_or_else(|| UsersSqliteError::UserNotFound(user_id.to_string()))?;
        self.conn.execute(
            r#"
            INSERT INTO current_user_context (ContextId, UserId)
            VALUES (1, ?1)
            ON CONFLICT(ContextId) DO UPDATE SET
                UserId = excluded.UserId
            "#,
            [user_id],
        )?;
        Ok(resolve_current_user(user)?)
    }

    pub fn clear_current_user(&self) -> Result<(), UsersSqliteError> {
        self.conn.execute("DELETE FROM current_user_context", [])?;
        Ok(())
    }

    pub fn resolve_current_user(&self) -> Result<UserContext, UsersSqliteError> {
        let user_id: String = self
            .conn
            .query_row(
                "SELECT UserId FROM current_user_context WHERE ContextId = 1",
                [],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(UsersSqliteError::MissingCurrentUser)?;

        let user = self
            .get_user_by_id(&user_id)?
            .ok_or(UsersSqliteError::MissingCurrentUser)?;
        Ok(resolve_current_user(user)?)
    }
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

// ─── RoleRepository impl ──────────────────────────────────────────────────────

impl RoleRepository for UsersSqliteStore {
    type Error = UsersSqliteError;

    fn get(&self, id: &RoleId) -> Result<Option<Role>, Self::Error> {
        // Lê os campos brutos no closure e constrói o Role fora dele, para que
        // um RhError de validação se propague como UsersSqliteError::Rh sem
        // ter de ser empacotado num rusqlite::Error.
        let raw: Option<(String, String, Option<String>, i64)> = self
            .conn
            .query_row(
                "SELECT role_id, name, description, is_active \
                 FROM platform_roles WHERE role_id = ?1",
                params![id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;

        match raw {
            Some((rid, name, desc, active)) => {
                let role = Role::new(rid, name, desc, active != 0)?;
                Ok(Some(role))
            }
            None => Ok(None),
        }
    }

    fn list_active(&self) -> Result<Vec<Role>, Self::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT role_id, name, description, is_active \
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
        let found: bool = self.conn.query_row(
            "SELECT COUNT(*) > 0 FROM platform_roles \
             WHERE role_id = ?1 AND is_active = 1",
            params![id.as_str()],
            |row| row.get(0),
        )?;
        Ok(found)
    }

    fn upsert(&self, role: &Role) -> Result<(), Self::Error> {
        role.validate().map_err(UsersSqliteError::Rh)?;
        self.conn.execute(
            "INSERT INTO platform_roles (role_id, name, description, is_active) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(role_id) DO UPDATE SET \
                 name        = excluded.name, \
                 description = excluded.description, \
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
        let changed = self.conn.execute(
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
