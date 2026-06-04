pub mod context;
pub mod email_verification;
pub mod error;
pub mod hash;
pub mod issue;
pub mod manifest;
pub mod normalized;
pub mod report;
pub mod result;
pub mod rules;
pub mod status;
pub mod validators;

// ── context ───────────────────────────────────────────────────────────────────
pub use context::ValidationContext;

// ── error ─────────────────────────────────────────────────────────────────────
pub use error::{
    ValidationError, FILE_NOT_FOUND, FILE_READ_FAILED, HASH_FAILED, INVALID_INPUT,
    INVALID_PATH_ENCODING, INVALID_RULE, JSON_FAILED, MANIFEST_FAILED, NORMALIZATION_FAILED,
    NOT_REGULAR_FILE, OPERATION_FAILED, UNSAFE_FILE_TYPE, VALIDATION_COMPONENT,
};

// ── email verification port ──────────────────────────────────────────────────
pub use email_verification::{
    EmailAttachment, EmailDeliveryError, EmailDeliveryEvidence, EmailDeliveryPort, EmailMessage,
    EmailRouteEvidence, EmailRouteStatus, EmailVerificationError, EmailVerificationPort,
};

// ── hash ──────────────────────────────────────────────────────────────────────
pub use hash::{sha256_bytes, sha256_file};

// ── issue ─────────────────────────────────────────────────────────────────────
pub use issue::{ValidationIssue, ValidationSeverity};

// ── manifest ──────────────────────────────────────────────────────────────────
pub use manifest::{manifest_file, ManifestEntry, ManifestList};

// ── normalized ────────────────────────────────────────────────────────────────
pub use normalized::Normalized;

// ── report ────────────────────────────────────────────────────────────────────
pub use report::ValidationReport;

// ── result ────────────────────────────────────────────────────────────────────
pub use result::{RuleOutcome, ValidationResult};

// ── rules ─────────────────────────────────────────────────────────────────────
pub use rules::{
    CC_CHECKSUM, CC_FORMAT, CP_FORMAT, DATE_FORMAT_INVALID, DATE_RANGE_INVALID, EMAIL_FORMAT,
    HASH_SHA256_FORMAT, IBAN_FORMAT, JSON_OBJECT, JSON_REQUIRED_FIELD, MIME_FORMAT, NIF_CHECKSUM,
    NIF_FORMAT, NISS_CHECKSUM, NISS_FORMAT, NUMERIC_RANGE_INVALID, PHONE_PT_FORMAT, SEMVER_FORMAT,
    STATE_TRANSITION_INVALID, STRING_MAX_LENGTH, STRING_MIN_LENGTH, STRING_REQUIRED, UUID_FORMAT,
};

// ── status ────────────────────────────────────────────────────────────────────
pub use status::ValidationStatus;

// ── validators ────────────────────────────────────────────────────────────────
pub use validators::{
    max_length, min_length, normalize_cc, normalize_cp, normalize_iban, normalize_nif,
    normalize_niss, normalize_phone_pt, require_field, require_object, required, validate_cc,
    validate_cp, validate_date_range, validate_datetime_range, validate_email, validate_iban,
    validate_in_range, validate_mime, validate_nif, validate_niss, validate_phone_pt,
    validate_semver, validate_sha256_hex, validate_state_transition, validate_uuid,
};
