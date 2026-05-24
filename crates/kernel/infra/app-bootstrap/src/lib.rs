#![allow(clippy::result_large_err)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use adapter_sqlite::SqliteRelationalConfig;
use core_config::{
    app_config_to_json_string, load_app_config_from_json_str, validate_app_config, AppConfig,
};
use documental_sqlite::{DocumentalSqliteError, DocumentalSqliteStore};
use files::{ensure_directories, prune_stale_temp_files, resolve_layout, FileLayout, FilesError};
use rh_sqlite::{UsersSqliteError, UsersSqliteStore};
use support_versioning::{FileReleaseNotesStore, VersioningError};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapOptions {
    pub documental_db_file_name: String,
    pub rh_db_file_name: String,
    pub config_file_name: String,
}

impl Default for BootstrapOptions {
    fn default() -> Self {
        Self {
            documental_db_file_name: "documental.db".to_string(),
            rh_db_file_name: "rh.db".to_string(),
            config_file_name: "app-config.json".to_string(),
        }
    }
}

#[derive(Debug)]
pub struct AppBootstrapRuntime {
    pub config: AppConfig,
    pub config_path: PathBuf,
    pub layout: FileLayout,
    pub documental_db_path: PathBuf,
    pub rh_db_path: PathBuf,
    pub release_notes_path: PathBuf,
    pub documental_store: DocumentalSqliteStore,
    pub users_store: UsersSqliteStore,
}

#[derive(Debug, Error)]
pub enum AppBootstrapError {
    #[error(transparent)]
    Config(#[from] core_config::ConfigError),
    #[error("erro de IO ao materializar configuração local: {0}")]
    ConfigIo(#[from] std::io::Error),
    #[error(transparent)]
    Files(#[from] FilesError),
    #[error(transparent)]
    DocumentalSqlite(#[from] DocumentalSqliteError),
    #[error(transparent)]
    UsersSqlite(#[from] UsersSqliteError),
    #[error(transparent)]
    Versioning(#[from] VersioningError),
    #[error("nome do ficheiro de base de dados vazio")]
    EmptyDbFileName,
    #[error("nome do ficheiro de configuração vazio")]
    EmptyConfigFileName,
}

pub fn load_from_json_file(path: impl AsRef<Path>) -> Result<AppConfig, AppBootstrapError> {
    let json = std::fs::read_to_string(path).map_err(AppBootstrapError::ConfigIo)?;
    let config = load_app_config_from_json_str(&json)?;
    validate_app_config(&config)?;
    Ok(config)
}

pub fn save_to_json_file(
    config: &AppConfig,
    path: impl AsRef<Path>,
) -> Result<(), AppBootstrapError> {
    validate_app_config(config)?;
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(AppBootstrapError::ConfigIo)?;
        }
    }
    let json = app_config_to_json_string(config)?;
    std::fs::write(path, json).map_err(AppBootstrapError::ConfigIo)?;
    Ok(())
}

pub fn ensure_json_file(
    path: impl AsRef<Path>,
    default_config: &AppConfig,
) -> Result<AppConfig, AppBootstrapError> {
    let path = path.as_ref();
    if path.exists() {
        return load_from_json_file(path);
    }

    save_to_json_file(default_config, path)?;
    Ok(default_config.clone())
}

pub fn bootstrap_local_app(
    base_dir: impl AsRef<Path>,
    config: AppConfig,
    options: BootstrapOptions,
) -> Result<AppBootstrapRuntime, AppBootstrapError> {
    if options.documental_db_file_name.trim().is_empty() {
        return Err(AppBootstrapError::EmptyDbFileName);
    }
    if options.rh_db_file_name.trim().is_empty() {
        return Err(AppBootstrapError::EmptyDbFileName);
    }
    if options.config_file_name.trim().is_empty() {
        return Err(AppBootstrapError::EmptyConfigFileName);
    }

    let config_path = base_dir.as_ref().join(&options.config_file_name);
    let config = ensure_json_file(&config_path, &config)?;

    let layout = resolve_layout(base_dir, &config.paths);
    ensure_directories(&layout)?;
    prune_stale_temp_files(&layout.temp_dir, Duration::from_secs(7 * 24 * 60 * 60))?;

    let documental_db_path = layout.database_dir.join(&options.documental_db_file_name);
    // open() already runs migrations internally
    let documental_store = DocumentalSqliteStore::open(
        &SqliteRelationalConfig::read_write_create(&documental_db_path),
    )?;
    let rh_db_path = layout.database_dir.join(&options.rh_db_file_name);
    let users_store =
        UsersSqliteStore::open(&SqliteRelationalConfig::read_write_create(&rh_db_path))?;
    users_store.migrate()?;
    let release_notes_path = layout.data_dir.join("release-notes.json");
    FileReleaseNotesStore::new(&release_notes_path).ensure_exists(env!("CARGO_PKG_VERSION"))?;

    Ok(AppBootstrapRuntime {
        config,
        config_path,
        layout,
        documental_db_path,
        rh_db_path,
        release_notes_path,
        documental_store,
        users_store,
    })
}
