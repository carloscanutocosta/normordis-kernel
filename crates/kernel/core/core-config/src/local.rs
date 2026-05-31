use crate::{ConfigError, Environment};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct PathsConfig {
    pub database_dir: PathBuf,
    pub data_dir: PathBuf,
    pub artifacts_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub logs_dir: PathBuf,
}

impl Default for PathsConfig {
    fn default() -> Self {
        Self {
            database_dir: PathBuf::from("database"),
            data_dir: PathBuf::from("assets"),
            artifacts_dir: PathBuf::from("artifacts"),
            temp_dir: PathBuf::from("tmp"),
            logs_dir: PathBuf::from("logs"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct AppOptions {
    pub app_name: String,
    pub environment: Environment,
}

impl Default for AppOptions {
    fn default() -> Self {
        Self {
            app_name: "miniapp".to_string(),
            environment: Environment::Dev,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    pub paths: PathsConfig,
    pub options: AppOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPaths {
    pub database_dir: PathBuf,
    pub data_dir: PathBuf,
    pub artifacts_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub logs_dir: PathBuf,
}

pub fn load_app_config_from_json_str(json: &str) -> Result<AppConfig, ConfigError> {
    serde_json::from_str::<AppConfig>(json).map_err(|err| ConfigError::MalformedJson {
        reason: format!("invalid app config json: {err}"),
    })
}

pub fn load_validated_app_config_from_json_str(json: &str) -> Result<AppConfig, ConfigError> {
    let config = load_app_config_from_json_str(json)?;
    validate_app_config(&config)?;
    Ok(config)
}

pub fn app_config_to_json_string(config: &AppConfig) -> Result<String, ConfigError> {
    serde_json::to_string_pretty(config).map_err(|err| ConfigError::MalformedJson {
        reason: format!("failed to serialize app config json: {err}"),
    })
}

const MAX_APP_NAME_LEN: usize = 128;

pub fn validate_app_config(config: &AppConfig) -> Result<(), ConfigError> {
    if config.options.app_name.trim().is_empty() {
        return Err(ConfigError::InvalidAppProfile {
            reason: "app_name is required".to_string(),
        });
    }

    if config.options.app_name.len() > MAX_APP_NAME_LEN {
        return Err(ConfigError::InvalidAppProfile {
            reason: format!("app_name exceeds maximum length of {MAX_APP_NAME_LEN}"),
        });
    }

    validate_path(&config.paths.data_dir, "data_dir")?;
    validate_path(&config.paths.database_dir, "database_dir")?;
    validate_path(&config.paths.artifacts_dir, "artifacts_dir")?;
    validate_path(&config.paths.temp_dir, "temp_dir")?;
    validate_path(&config.paths.logs_dir, "logs_dir")?;

    Ok(())
}

pub fn resolve_paths(base_dir: impl AsRef<Path>, paths: &PathsConfig) -> ResolvedPaths {
    let base = base_dir.as_ref();
    ResolvedPaths {
        database_dir: base.join(&paths.database_dir),
        data_dir: base.join(&paths.data_dir),
        artifacts_dir: base.join(&paths.artifacts_dir),
        temp_dir: base.join(&paths.temp_dir),
        logs_dir: base.join(&paths.logs_dir),
    }
}

fn validate_path(path: &Path, field: &'static str) -> Result<(), ConfigError> {
    if path.as_os_str().is_empty() {
        return Err(ConfigError::InvalidAppProfile {
            reason: format!("{field} cannot be empty"),
        });
    }
    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) => {
                return Err(ConfigError::InvalidAppProfile {
                    reason: format!("{field} must be a relative path"),
                });
            }
            Component::ParentDir => {
                return Err(ConfigError::InvalidAppProfile {
                    reason: format!("{field} must not traverse parent directories"),
                });
            }
        }
    }
    Ok(())
}
