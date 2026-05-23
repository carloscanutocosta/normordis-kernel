pub mod app;
pub mod audit;
pub mod crypto;
pub mod error;
pub mod local;
pub mod logging;
pub mod profile;
pub mod runtime;
pub mod storage;
pub mod validate;

// ── app ───────────────────────────────────────────────────────────────────────
pub use app::{AppProfile, Environment};

// ── audit ─────────────────────────────────────────────────────────────────────
pub use audit::{AuditProfile, DEFAULT_AUDIT_NAMESPACE, DEFAULT_AUDIT_STORAGE_PROFILE};

// ── crypto ────────────────────────────────────────────────────────────────────
pub use crypto::CryptoProfile;

// ── error ─────────────────────────────────────────────────────────────────────
pub use error::{
    ConfigError, CONFIG_COMPONENT, DUPLICATE_STORAGE_PROFILE, INCONSISTENT_PROFILE,
    INVALID_APP_PROFILE, INVALID_AUDIT_PROFILE, INVALID_CRYPTO_PROFILE,
    INVALID_LOGGING_PROFILE, INVALID_RUNTIME_PROFILE, INVALID_STORAGE_PROFILE,
    MISSING_STORAGE_PROFILE,
};

// ── local ─────────────────────────────────────────────────────────────────────
pub use local::{
    app_config_to_json_string, load_app_config_from_json_str, resolve_paths,
    validate_app_config, AppConfig, AppOptions, PathsConfig, ResolvedPaths,
};

// ── logging ───────────────────────────────────────────────────────────────────
pub use logging::{
    LoggingProfile, DEFAULT_LOG_FILE_NAME, DEFAULT_MAX_FILE_SIZE_MB, DEFAULT_MAX_FILES,
    DEFAULT_RETENTION_DAYS,
};

// ── profile ───────────────────────────────────────────────────────────────────
pub use profile::MiniKernelProfile;

// ── runtime ───────────────────────────────────────────────────────────────────
pub use runtime::RuntimeProfile;

// ── storage ───────────────────────────────────────────────────────────────────
pub use storage::{StorageBackend, StorageProfile, StorageProfiles, StoragePurpose};

// ── validate ──────────────────────────────────────────────────────────────────
pub use validate::{
    validate_app_profile, validate_audit_profile, validate_crypto_profile,
    validate_logging_profile, validate_profile, validate_runtime_profile,
    validate_storage_profile, validate_storage_profiles,
};
