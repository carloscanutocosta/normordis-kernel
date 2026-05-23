pub mod error;
pub mod hash;
pub mod issue;
pub mod manifest;
pub mod normalized;
pub mod report;
pub mod rules;
pub mod validators;

// ── error ─────────────────────────────────────────────────────────────────────
pub use error::{
    ValidationError, VALIDATION_COMPONENT, FILE_NOT_FOUND, FILE_READ_FAILED, HASH_FAILED,
    INVALID_INPUT, INVALID_RULE, JSON_FAILED, MANIFEST_FAILED, NORMALIZATION_FAILED,
    NOT_REGULAR_FILE, OPERATION_FAILED,
};

// ── hash ──────────────────────────────────────────────────────────────────────
pub use hash::{sha256_bytes, sha256_file};

// ── issue ─────────────────────────────────────────────────────────────────────
pub use issue::{ValidationIssue, ValidationSeverity};

// ── manifest ──────────────────────────────────────────────────────────────────
pub use manifest::{manifest_file, ManifestEntry};

// ── normalized ────────────────────────────────────────────────────────────────
pub use normalized::Normalized;

// ── report ────────────────────────────────────────────────────────────────────
pub use report::ValidationReport;

// ── rules ─────────────────────────────────────────────────────────────────────
pub use rules::{
    EMAIL_FORMAT, IBAN_FORMAT, JSON_OBJECT, JSON_REQUIRED_FIELD, NIF_CHECKSUM, NIF_FORMAT,
    STRING_MAX_LENGTH, STRING_REQUIRED, UUID_FORMAT,
};

// ── validators ────────────────────────────────────────────────────────────────
pub use validators::{
    max_length, normalize_iban, normalize_nif, require_field, require_object, required,
    validate_email, validate_iban, validate_nif, validate_uuid,
};
