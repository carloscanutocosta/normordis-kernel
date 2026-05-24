#![allow(clippy::result_large_err)]

use adapter_sqlite::{
    open_relational_connection, run_relational_migrations, SqliteRelationalConfig,
};
use core_rh::{resolve_current_user, RhError, UserContext, UserIdentity, UserRole};
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

pub const RH_SQLITE_MIGRATIONS: &[&str] = &[r#"
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
"#];

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
