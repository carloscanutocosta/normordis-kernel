use crate::{
    AppProfile, AuditProfile, ConfigError, CryptoProfile, LoggingProfile, MiniKernelProfile,
    RuntimeProfile, StorageBackend, StorageProfile, StorageProfiles, StoragePurpose,
};
use std::collections::HashSet;

const MAX_APP_ID_LEN: usize = 128;
const MAX_DISPLAY_NAME_LEN: usize = 255;
const MAX_PROFILE_NAME_LEN: usize = 64;
const MAX_KEY_ID_LEN: usize = 128;
const MAX_STORAGE_PROFILE_NAME_LEN: usize = 64;
const MAX_NAMESPACE_LEN: usize = 128;

pub fn validate_profile(profile: &MiniKernelProfile) -> Result<(), ConfigError> {
    validate_app_profile(&profile.app)?;
    validate_runtime_profile(&profile.runtime)?;
    validate_storage_profiles(&profile.storage)?;
    validate_crypto_profile(&profile.crypto)?;
    validate_logging_profile(&profile.logging)?;
    validate_audit_profile(&profile.audit, &profile.storage)?;

    if profile
        .storage
        .profiles
        .iter()
        .any(|storage| storage.encrypted)
        && !profile.crypto.enabled
    {
        return Err(ConfigError::InconsistentProfile {
            reason: "encrypted storage requires crypto.enabled = true".to_owned(),
        });
    }

    Ok(())
}

pub fn validate_app_profile(profile: &AppProfile) -> Result<(), ConfigError> {
    if is_blank(&profile.app_id) {
        return Err(ConfigError::InvalidAppProfile {
            reason: "app_id is required".to_owned(),
        });
    }

    if profile.app_id.len() > MAX_APP_ID_LEN {
        return Err(ConfigError::InvalidAppProfile {
            reason: format!("app_id exceeds maximum length of {MAX_APP_ID_LEN}"),
        });
    }

    if !profile
        .app_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
    {
        return Err(ConfigError::InvalidAppProfile {
            reason: "app_id contains invalid characters".to_owned(),
        });
    }

    if is_blank(&profile.display_name) {
        return Err(ConfigError::InvalidAppProfile {
            reason: "display_name is required".to_owned(),
        });
    }

    if profile.display_name.len() > MAX_DISPLAY_NAME_LEN {
        return Err(ConfigError::InvalidAppProfile {
            reason: format!("display_name exceeds maximum length of {MAX_DISPLAY_NAME_LEN}"),
        });
    }

    Ok(())
}

pub fn validate_runtime_profile(profile: &RuntimeProfile) -> Result<(), ConfigError> {
    if is_blank(&profile.profile_name) {
        return Err(ConfigError::InvalidRuntimeProfile {
            reason: "profile_name is required".to_owned(),
        });
    }

    if contains_whitespace(&profile.profile_name) {
        return Err(ConfigError::InvalidRuntimeProfile {
            reason: "profile_name cannot contain spaces".to_owned(),
        });
    }

    if profile.profile_name.len() > MAX_PROFILE_NAME_LEN {
        return Err(ConfigError::InvalidRuntimeProfile {
            reason: format!("profile_name exceeds maximum length of {MAX_PROFILE_NAME_LEN}"),
        });
    }

    Ok(())
}

pub fn validate_storage_profiles(storage: &StorageProfiles) -> Result<(), ConfigError> {
    if storage.profiles.is_empty() {
        return Err(ConfigError::MissingStorageProfile {
            name: storage.default_profile.clone(),
        });
    }

    if is_blank(&storage.default_profile) {
        return Err(ConfigError::MissingStorageProfile {
            name: storage.default_profile.clone(),
        });
    }

    let mut names = HashSet::new();
    for profile in &storage.profiles {
        validate_storage_profile(profile)?;
        if !names.insert(profile.name.clone()) {
            return Err(ConfigError::DuplicateStorageProfile {
                name: profile.name.clone(),
            });
        }
    }

    if storage.profile(&storage.default_profile).is_none() {
        return Err(ConfigError::MissingStorageProfile {
            name: storage.default_profile.clone(),
        });
    }

    Ok(())
}

pub fn validate_storage_profile(profile: &StorageProfile) -> Result<(), ConfigError> {
    if is_blank(&profile.name) {
        return Err(ConfigError::InvalidStorageProfile {
            reason: "name is required".to_owned(),
        });
    }

    if contains_whitespace(&profile.name) || profile.name.contains(':') {
        return Err(ConfigError::InvalidStorageProfile {
            reason: "name cannot contain spaces or ':'".to_owned(),
        });
    }

    if profile.name.len() > MAX_STORAGE_PROFILE_NAME_LEN {
        return Err(ConfigError::InvalidStorageProfile {
            reason: format!("name exceeds maximum length of {MAX_STORAGE_PROFILE_NAME_LEN}"),
        });
    }

    if profile.backend == StorageBackend::Memory && profile.encrypted {
        return Err(ConfigError::InvalidStorageProfile {
            reason: "memory storage cannot be encrypted".to_owned(),
        });
    }

    match profile.backend {
        StorageBackend::Sqlite => {
            let path = profile.database_path.as_ref().ok_or_else(|| {
                ConfigError::InvalidStorageProfile {
                    reason: "sqlite storage requires database_path".to_owned(),
                }
            })?;

            if path.as_os_str().is_empty() {
                return Err(ConfigError::InvalidStorageProfile {
                    reason: "sqlite storage requires non-empty database_path".to_owned(),
                });
            }

            validate_storage_purpose(&profile.purpose)
        }
        StorageBackend::Memory if profile.database_path.is_some() => {
            Err(ConfigError::InvalidStorageProfile {
                reason: "memory storage cannot define database_path".to_owned(),
            })
        }
        _ => validate_storage_purpose(&profile.purpose),
    }
}

pub fn validate_crypto_profile(profile: &CryptoProfile) -> Result<(), ConfigError> {
    if profile.enabled {
        let key_id =
            profile
                .key_id
                .as_deref()
                .ok_or_else(|| ConfigError::InvalidCryptoProfile {
                    reason: "key_id is required when crypto is enabled".to_owned(),
                })?;

        if is_blank(key_id) || contains_whitespace(key_id) || key_id.contains(':') {
            return Err(ConfigError::InvalidCryptoProfile {
                reason: "key_id cannot be empty or contain spaces or ':'".to_owned(),
            });
        }

        if key_id.len() > MAX_KEY_ID_LEN {
            return Err(ConfigError::InvalidCryptoProfile {
                reason: format!("key_id exceeds maximum length of {MAX_KEY_ID_LEN}"),
            });
        }
    }

    Ok(())
}

pub fn validate_logging_profile(profile: &LoggingProfile) -> Result<(), ConfigError> {
    if !profile.enabled {
        return Ok(());
    }

    let log_dir =
        profile
            .log_dir
            .as_ref()
            .ok_or_else(|| ConfigError::InvalidLoggingProfile {
                reason: "log_dir is required when logging is enabled".to_owned(),
            })?;

    if log_dir.as_os_str().is_empty() {
        return Err(ConfigError::InvalidLoggingProfile {
            reason: "log_dir cannot be empty when logging is enabled".to_owned(),
        });
    }

    if is_blank(&profile.file_name)
        || profile.file_name.contains('/')
        || profile.file_name.contains('\\')
    {
        return Err(ConfigError::InvalidLoggingProfile {
            reason: "file_name is required and cannot contain path separators".to_owned(),
        });
    }

    if profile.max_file_size_mb == 0 {
        return Err(ConfigError::InvalidLoggingProfile {
            reason: "max_file_size_mb must be greater than zero".to_owned(),
        });
    }

    if profile.max_files == 0 {
        return Err(ConfigError::InvalidLoggingProfile {
            reason: "max_files must be greater than zero".to_owned(),
        });
    }

    if profile.retention_days < 1 {
        return Err(ConfigError::InvalidLoggingProfile {
            reason: "retention_days must be at least one".to_owned(),
        });
    }

    Ok(())
}

pub fn validate_audit_profile(
    profile: &AuditProfile,
    storage: &StorageProfiles,
) -> Result<(), ConfigError> {
    if !profile.enabled {
        return Ok(());
    }

    if is_blank(&profile.namespace)
        || contains_whitespace(&profile.namespace)
        || profile.namespace.contains(':')
    {
        return Err(ConfigError::InvalidAuditProfile {
            reason: "namespace is required and cannot contain spaces or ':'".to_owned(),
        });
    }

    if profile.namespace.len() > MAX_NAMESPACE_LEN {
        return Err(ConfigError::InvalidAuditProfile {
            reason: format!("namespace exceeds maximum length of {MAX_NAMESPACE_LEN}"),
        });
    }

    if is_blank(&profile.storage_profile) {
        return Err(ConfigError::InvalidAuditProfile {
            reason: "storage_profile is required".to_owned(),
        });
    }

    if contains_whitespace(&profile.storage_profile) || profile.storage_profile.contains(':') {
        return Err(ConfigError::InvalidAuditProfile {
            reason: "storage_profile cannot contain spaces or ':'".to_owned(),
        });
    }

    let audit_storage = storage
        .profile(&profile.storage_profile)
        .ok_or_else(|| ConfigError::MissingStorageProfile {
            name: profile.storage_profile.clone(),
        })?;

    if audit_storage.purpose != StoragePurpose::Audit {
        return Err(ConfigError::InvalidAuditProfile {
            reason: "audit storage_profile must have StoragePurpose::Audit".to_owned(),
        });
    }

    Ok(())
}

fn validate_storage_purpose(purpose: &StoragePurpose) -> Result<(), ConfigError> {
    if let StoragePurpose::Other(value) = purpose {
        if is_blank(value) {
            return Err(ConfigError::InvalidStorageProfile {
                reason: "other storage purpose cannot be empty".to_owned(),
            });
        }
    }

    Ok(())
}

fn contains_whitespace(value: &str) -> bool {
    value.chars().any(char::is_whitespace)
}

fn is_blank(value: &str) -> bool {
    value.trim().is_empty()
}
