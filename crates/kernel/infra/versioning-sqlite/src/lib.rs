/*!
 * Versionamento SemVer por módulo, persistido em SQLite.
 *
 * Cada módulo (workspace, mini-app, etc.) tem a sua própria linha de versão.
 * Cada bump gera um registo no changelog com resumo e "what's new".
 *
 * Schema:
 *   app_version        — versão actual por AppName
 *   version_changelog  — histórico imutável de todos os bumps
 */

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use support_errors::{Component, ErrorCode, MiniError};
use thiserror::Error;

// ─── erros ───────────────────────────────────────────────────────────────────

pub const VERSIONING_COMPONENT: &str = "support-versioning-sqlite";
pub const SQLITE_ERROR: &str = "MINI.VERSIONING.SQLITE_ERROR";
pub const APP_NOT_FOUND: &str = "MINI.VERSIONING.APP_NOT_FOUND";
pub const EMPTY_SUMMARY: &str = "MINI.VERSIONING.EMPTY_SUMMARY";
pub const EMPTY_WHATS_NEW: &str = "MINI.VERSIONING.EMPTY_WHATS_NEW";
pub const INVALID_BUMP_TYPE: &str = "MINI.VERSIONING.INVALID_BUMP_TYPE";

#[derive(Debug, Error)]
pub enum VersioningError {
    #[error("erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("módulo '{0}' não encontrado")]
    AppNotFound(String),
    #[error("resumo da alteração não pode estar vazio")]
    EmptySummary,
    #[error("campo what's new não pode estar vazio")]
    EmptyWhatsNew,
    #[error("tipo de bump inválido: '{0}' — use 'major', 'minor' ou 'patch'")]
    InvalidBumpType(String),
}

impl VersioningError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Sqlite(_) => SQLITE_ERROR,
            Self::AppNotFound(_) => APP_NOT_FOUND,
            Self::EmptySummary => EMPTY_SUMMARY,
            Self::EmptyWhatsNew => EMPTY_WHATS_NEW,
            Self::InvalidBumpType(_) => INVALID_BUMP_TYPE,
        }
    }

    pub fn public_message(&self) -> &'static str {
        match self {
            Self::Sqlite(_) => "database operation failed",
            Self::AppNotFound(_) => "module not found",
            Self::EmptySummary => "change summary cannot be empty",
            Self::EmptyWhatsNew => "what's new field cannot be empty",
            Self::InvalidBumpType(_) => "invalid bump type — use major, minor or patch",
        }
    }

    pub fn to_mini_error(&self) -> MiniError {
        MiniError::new(
            ErrorCode::new(self.code())
                .expect("support-versioning-sqlite error codes must be valid"),
            Component::new(VERSIONING_COMPONENT)
                .expect("support-versioning-sqlite component must be valid"),
            self.public_message(),
        )
    }
}

impl From<VersioningError> for MiniError {
    fn from(value: VersioningError) -> Self {
        value.to_mini_error()
    }
}

// ─── tipos públicos ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AppVersion {
    pub app_name: String,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub updated_at_utc: String,
}

impl AppVersion {
    pub fn semver(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VersionChangelogEntry {
    pub id: i64,
    pub app_name: String,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub summary: String,
    pub whats_new: String,
    pub bump_type: String,
    pub created_at_utc: String,
}

impl VersionChangelogEntry {
    pub fn semver(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BumpType {
    Major,
    Minor,
    Patch,
}

impl BumpType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BumpType::Major => "major",
            BumpType::Minor => "minor",
            BumpType::Patch => "patch",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, VersioningError> {
        match s.to_lowercase().as_str() {
            "major" => Ok(BumpType::Major),
            "minor" => Ok(BumpType::Minor),
            "patch" => Ok(BumpType::Patch),
            other => Err(VersioningError::InvalidBumpType(other.to_string())),
        }
    }
}

// ─── store ────────────────────────────────────────────────────────────────────

pub struct VersioningSqliteStore {
    db_path: PathBuf,
}

const MIGRATION: &str = r#"
CREATE TABLE IF NOT EXISTS app_version (
    AppName      TEXT PRIMARY KEY,
    Major        INTEGER NOT NULL DEFAULT 0,
    Minor        INTEGER NOT NULL DEFAULT 1,
    Patch        INTEGER NOT NULL DEFAULT 0,
    UpdatedAtUtc TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS version_changelog (
    Id           INTEGER PRIMARY KEY AUTOINCREMENT,
    AppName      TEXT    NOT NULL,
    Major        INTEGER NOT NULL,
    Minor        INTEGER NOT NULL,
    Patch        INTEGER NOT NULL,
    Summary      TEXT    NOT NULL,
    WhatsNew     TEXT    NOT NULL,
    BumpType     TEXT    NOT NULL CHECK(BumpType IN ('major','minor','patch')),
    CreatedAtUtc TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_changelog_app ON version_changelog(AppName, Id DESC);
"#;

impl VersioningSqliteStore {
    pub fn new(db_path: impl Into<PathBuf>) -> Self {
        Self {
            db_path: db_path.into(),
        }
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    fn open(&self) -> Result<Connection, VersioningError> {
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                    Some(e.to_string()),
                )
            })?;
        }
        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch(MIGRATION)?;
        Ok(conn)
    }

    /// Garante que o módulo existe na tabela; cria-o a 0.1.0 se não existir.
    /// Devolve `true` se foi criado agora, `false` se já existia.
    pub fn ensure_app(
        &self,
        app_name: &str,
        initial_major: u32,
        initial_minor: u32,
        initial_patch: u32,
    ) -> Result<bool, VersioningError> {
        let conn = self.open()?;
        let existing: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM app_version WHERE AppName = ?1",
                params![app_name],
                |row| row.get(0),
            )
            .optional()?;
        if existing.is_some() {
            return Ok(false);
        }
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO app_version (AppName, Major, Minor, Patch, UpdatedAtUtc) VALUES (?1,?2,?3,?4,?5)",
            params![app_name, initial_major, initial_minor, initial_patch, now],
        )?;
        // Regista a versão inicial no changelog
        conn.execute(
            r#"INSERT INTO version_changelog
               (AppName, Major, Minor, Patch, Summary, WhatsNew, BumpType, CreatedAtUtc)
               VALUES (?1,?2,?3,?4,'Versão inicial','Primeira versão registada.','patch',?5)"#,
            params![app_name, initial_major, initial_minor, initial_patch, now],
        )?;
        Ok(true)
    }

    /// Devolve a versão actual do módulo.
    pub fn get_version(&self, app_name: &str) -> Result<AppVersion, VersioningError> {
        let conn = self.open()?;
        conn.query_row(
            "SELECT AppName, Major, Minor, Patch, UpdatedAtUtc FROM app_version WHERE AppName = ?1",
            params![app_name],
            |row| {
                Ok(AppVersion {
                    app_name: row.get(0)?,
                    major: row.get::<_, u32>(1)?,
                    minor: row.get::<_, u32>(2)?,
                    patch: row.get::<_, u32>(3)?,
                    updated_at_utc: row.get(4)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| VersioningError::AppNotFound(app_name.to_string()))
    }

    /// Lista todos os módulos registados, ordenados por nome.
    pub fn list_versions(&self) -> Result<Vec<AppVersion>, VersioningError> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(
            "SELECT AppName, Major, Minor, Patch, UpdatedAtUtc FROM app_version ORDER BY AppName",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(AppVersion {
                app_name: row.get(0)?,
                major: row.get::<_, u32>(1)?,
                minor: row.get::<_, u32>(2)?,
                patch: row.get::<_, u32>(3)?,
                updated_at_utc: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Incrementa a versão do módulo e regista o changelog.
    /// Semântica:
    ///   major → major+1 . 0 . 0
    ///   minor → major   . minor+1 . 0
    ///   patch → major   . minor   . patch+1
    pub fn bump_version(
        &self,
        app_name: &str,
        bump_type: BumpType,
        summary: &str,
        whats_new: &str,
    ) -> Result<AppVersion, VersioningError> {
        let summary = summary.trim();
        let whats_new = whats_new.trim();
        if summary.is_empty() {
            return Err(VersioningError::EmptySummary);
        }
        if whats_new.is_empty() {
            return Err(VersioningError::EmptyWhatsNew);
        }

        let current = self.get_version(app_name)?;
        let (new_major, new_minor, new_patch) = match bump_type {
            BumpType::Major => (current.major + 1, 0, 0),
            BumpType::Minor => (current.major, current.minor + 1, 0),
            BumpType::Patch => (current.major, current.minor, current.patch + 1),
        };
        let now = Utc::now().to_rfc3339();

        let conn = self.open()?;
        conn.execute(
            "UPDATE app_version SET Major=?1, Minor=?2, Patch=?3, UpdatedAtUtc=?4 WHERE AppName=?5",
            params![new_major, new_minor, new_patch, now, app_name],
        )?;
        conn.execute(
            r#"INSERT INTO version_changelog
               (AppName, Major, Minor, Patch, Summary, WhatsNew, BumpType, CreatedAtUtc)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8)"#,
            params![
                app_name,
                new_major,
                new_minor,
                new_patch,
                summary,
                whats_new,
                bump_type.as_str(),
                now
            ],
        )?;

        Ok(AppVersion {
            app_name: app_name.to_string(),
            major: new_major,
            minor: new_minor,
            patch: new_patch,
            updated_at_utc: now,
        })
    }

    /// Garante que o módulo está na versão mínima indicada.
    /// Se a versão actual for inferior, efectua exactamente um bump para a atingir.
    /// Idempotente: não faz nada se já estiver na versão correcta ou superior.
    /// Devolve `true` se o bump foi efectuado.
    pub fn ensure_min_version(
        &self,
        app_name: &str,
        target_major: u32,
        target_minor: u32,
        target_patch: u32,
        summary: &str,
        whats_new: &str,
    ) -> Result<bool, VersioningError> {
        let current = self.get_version(app_name)?;
        let cur = (current.major, current.minor, current.patch);
        let tgt = (target_major, target_minor, target_patch);
        if cur >= tgt {
            return Ok(false);
        }
        let bump_type = if target_major > current.major {
            BumpType::Major
        } else if target_minor > current.minor {
            BumpType::Minor
        } else {
            BumpType::Patch
        };
        self.bump_version(app_name, bump_type, summary, whats_new)?;
        Ok(true)
    }

    /// Lista o changelog de um módulo, do mais recente para o mais antigo.
    pub fn list_changelog(
        &self,
        app_name: &str,
    ) -> Result<Vec<VersionChangelogEntry>, VersioningError> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(
            r#"SELECT Id, AppName, Major, Minor, Patch, Summary, WhatsNew, BumpType, CreatedAtUtc
               FROM version_changelog
               WHERE AppName = ?1
               ORDER BY Id DESC"#,
        )?;
        let rows = stmt.query_map(params![app_name], |row| {
            Ok(VersionChangelogEntry {
                id: row.get(0)?,
                app_name: row.get(1)?,
                major: row.get::<_, u32>(2)?,
                minor: row.get::<_, u32>(3)?,
                patch: row.get::<_, u32>(4)?,
                summary: row.get(5)?,
                whats_new: row.get(6)?,
                bump_type: row.get(7)?,
                created_at_utc: row.get(8)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Lista todo o changelog de todos os módulos, do mais recente para o mais antigo.
    pub fn list_all_changelog(&self) -> Result<Vec<VersionChangelogEntry>, VersioningError> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(
            r#"SELECT Id, AppName, Major, Minor, Patch, Summary, WhatsNew, BumpType, CreatedAtUtc
               FROM version_changelog
               ORDER BY Id DESC"#,
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(VersionChangelogEntry {
                id: row.get(0)?,
                app_name: row.get(1)?,
                major: row.get::<_, u32>(2)?,
                minor: row.get::<_, u32>(3)?,
                patch: row.get::<_, u32>(4)?,
                summary: row.get(5)?,
                whats_new: row.get(6)?,
                bump_type: row.get(7)?,
                created_at_utc: row.get(8)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}

// ─── testes ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn store_in_tmp() -> (tempfile::TempDir, VersioningSqliteStore) {
        let dir = tempdir().unwrap();
        let store = VersioningSqliteStore::new(dir.path().join("versioning.db"));
        (dir, store)
    }

    #[test]
    fn ensure_app_creates_at_initial_version() {
        let (_dir, store) = store_in_tmp();
        let created = store.ensure_app("workspace", 0, 1, 0).unwrap();
        assert!(created);
        let v = store.get_version("workspace").unwrap();
        assert_eq!(v.semver(), "0.1.0");
    }

    #[test]
    fn ensure_app_idempotent() {
        let (_dir, store) = store_in_tmp();
        store.ensure_app("workspace", 0, 1, 0).unwrap();
        let second = store.ensure_app("workspace", 0, 1, 0).unwrap();
        assert!(!second, "segunda chamada não deve recriar");
    }

    #[test]
    fn bump_patch_increments_patch() {
        let (_dir, store) = store_in_tmp();
        store.ensure_app("workspace", 0, 1, 0).unwrap();
        let v = store
            .bump_version(
                "workspace",
                BumpType::Patch,
                "Fix bug",
                "Correcção de bug X",
            )
            .unwrap();
        assert_eq!(v.semver(), "0.1.1");
    }

    #[test]
    fn bump_minor_resets_patch() {
        let (_dir, store) = store_in_tmp();
        store.ensure_app("workspace", 0, 1, 3).unwrap();
        let v = store
            .bump_version(
                "workspace",
                BumpType::Minor,
                "Nova funcionalidade",
                "Adicionado Y",
            )
            .unwrap();
        assert_eq!(v.semver(), "0.2.0");
    }

    #[test]
    fn bump_major_resets_minor_and_patch() {
        let (_dir, store) = store_in_tmp();
        store.ensure_app("workspace", 0, 5, 2).unwrap();
        let v = store
            .bump_version(
                "workspace",
                BumpType::Major,
                "Breaking change",
                "Migração Z",
            )
            .unwrap();
        assert_eq!(v.semver(), "1.0.0");
    }

    #[test]
    fn changelog_is_ordered_newest_first() {
        let (_dir, store) = store_in_tmp();
        store.ensure_app("ws", 0, 1, 0).unwrap();
        store
            .bump_version("ws", BumpType::Patch, "p1", "w1")
            .unwrap();
        store
            .bump_version("ws", BumpType::Patch, "p2", "w2")
            .unwrap();
        let log = store.list_changelog("ws").unwrap();
        // changelog inclui entrada inicial + 2 bumps = 3 entradas
        assert_eq!(log.len(), 3);
        assert_eq!(log[0].summary, "p2");
        assert_eq!(log[1].summary, "p1");
    }

    #[test]
    fn multiple_apps_independent() {
        let (_dir, store) = store_in_tmp();
        store.ensure_app("workspace", 0, 1, 0).unwrap();
        store.ensure_app("consent-pf", 0, 1, 0).unwrap();
        store
            .bump_version("workspace", BumpType::Minor, "Nova tab", "Adicionada tab X")
            .unwrap();
        let ws = store.get_version("workspace").unwrap();
        let pf = store.get_version("consent-pf").unwrap();
        assert_eq!(ws.semver(), "0.2.0");
        assert_eq!(pf.semver(), "0.1.0");
    }

    #[test]
    fn empty_summary_is_rejected() {
        let (_dir, store) = store_in_tmp();
        store.ensure_app("ws", 0, 1, 0).unwrap();
        let err = store
            .bump_version("ws", BumpType::Patch, "  ", "ok")
            .unwrap_err();
        assert!(matches!(err, VersioningError::EmptySummary));
    }

    #[test]
    fn unknown_app_returns_error() {
        let (_dir, store) = store_in_tmp();
        let err = store.get_version("nao-existe").unwrap_err();
        assert!(matches!(err, VersioningError::AppNotFound(_)));
    }
}
