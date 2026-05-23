use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use support_errors::{Component, ErrorCode, MiniError};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseNotes {
    pub version: String,
    pub novidades: Vec<String>,
    pub problemas_conhecidos: Vec<String>,
    pub updated_at_utc: DateTime<Utc>,
}

impl ReleaseNotes {
    pub fn new(version: impl Into<String>, now: DateTime<Utc>) -> Self {
        Self {
            version: version.into(),
            novidades: Vec::new(),
            problemas_conhecidos: Vec::new(),
            updated_at_utc: now,
        }
    }

    pub fn validate(&self) -> Result<(), VersioningError> {
        if self.version.trim().is_empty() {
            return Err(VersioningError::EmptyVersion);
        }
        if self.novidades.iter().any(|x| x.trim().is_empty()) {
            return Err(VersioningError::EmptyNovelty);
        }
        if self
            .problemas_conhecidos
            .iter()
            .any(|x| x.trim().is_empty())
        {
            return Err(VersioningError::EmptyKnownIssue);
        }
        Ok(())
    }
}

pub const VERSIONING_COMPONENT: &str = "support-versioning";
pub const EMPTY_VERSION: &str = "MINI.VERSIONING.EMPTY_VERSION";
pub const EMPTY_NOVELTY: &str = "MINI.VERSIONING.EMPTY_NOVELTY";
pub const EMPTY_KNOWN_ISSUE: &str = "MINI.VERSIONING.EMPTY_KNOWN_ISSUE";
pub const IO_ERROR: &str = "MINI.VERSIONING.IO_ERROR";
pub const JSON_ERROR: &str = "MINI.VERSIONING.JSON_ERROR";

#[derive(Debug, Error)]
pub enum VersioningError {
    #[error("versão vazia")]
    EmptyVersion,
    #[error("novidade vazia")]
    EmptyNovelty,
    #[error("problema conhecido vazio")]
    EmptyKnownIssue,
    #[error("erro de IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("erro JSON: {0}")]
    Json(#[from] serde_json::Error),
}

impl VersioningError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptyVersion => EMPTY_VERSION,
            Self::EmptyNovelty => EMPTY_NOVELTY,
            Self::EmptyKnownIssue => EMPTY_KNOWN_ISSUE,
            Self::Io(_) => IO_ERROR,
            Self::Json(_) => JSON_ERROR,
        }
    }

    pub fn public_message(&self) -> &'static str {
        match self {
            Self::EmptyVersion => "version cannot be empty",
            Self::EmptyNovelty => "novelty entry cannot be empty",
            Self::EmptyKnownIssue => "known issue entry cannot be empty",
            Self::Io(_) => "file operation failed",
            Self::Json(_) => "JSON serialization failed",
        }
    }

    pub fn to_mini_error(&self) -> MiniError {
        MiniError::new(
            ErrorCode::new(self.code()).expect("support-versioning error codes must be valid"),
            Component::new(VERSIONING_COMPONENT)
                .expect("support-versioning component must be valid"),
            self.public_message(),
        )
    }
}

impl From<VersioningError> for MiniError {
    fn from(value: VersioningError) -> Self {
        value.to_mini_error()
    }
}

#[derive(Debug, Clone)]
pub struct FileReleaseNotesStore {
    path: PathBuf,
}

impl FileReleaseNotesStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn ensure_exists(
        &self,
        default_version: impl Into<String>,
    ) -> Result<ReleaseNotes, VersioningError> {
        if self.path.exists() {
            return self.load();
        }

        let notes = ReleaseNotes::new(default_version, Utc::now());
        self.save(&notes)?;
        Ok(notes)
    }

    pub fn load(&self) -> Result<ReleaseNotes, VersioningError> {
        let json = fs::read_to_string(&self.path)?;
        let notes = serde_json::from_str::<ReleaseNotes>(&json)?;
        notes.validate()?;
        Ok(notes)
    }

    pub fn save(&self, notes: &ReleaseNotes) -> Result<(), VersioningError> {
        notes.validate()?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(notes)?;
        fs::write(&self.path, json)?;
        Ok(())
    }

    pub fn set_version(
        &self,
        version: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Result<ReleaseNotes, VersioningError> {
        let mut notes = self.load()?;
        notes.version = version.into();
        notes.updated_at_utc = now;
        self.save(&notes)?;
        Ok(notes)
    }

    pub fn add_novidade(
        &self,
        novidade: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Result<ReleaseNotes, VersioningError> {
        let mut notes = self.load()?;
        notes.novidades.push(novidade.into());
        notes.updated_at_utc = now;
        self.save(&notes)?;
        Ok(notes)
    }

    pub fn add_problema_conhecido(
        &self,
        problema: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Result<ReleaseNotes, VersioningError> {
        let mut notes = self.load()?;
        notes.problemas_conhecidos.push(problema.into());
        notes.updated_at_utc = now;
        self.save(&notes)?;
        Ok(notes)
    }
}
