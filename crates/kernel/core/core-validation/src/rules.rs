// ── Identificadores formais ───────────────────────────────────────────────────
pub const STRING_REQUIRED: &str = "validation.string.required";
pub const STRING_MIN_LENGTH: &str = "validation.string.min_length";
pub const STRING_MAX_LENGTH: &str = "validation.string.max_length";
pub const SEMVER_FORMAT: &str = "validation.semver.format";
pub const CP_FORMAT: &str = "validation.cp.format";
pub const PHONE_PT_FORMAT: &str = "validation.phone_pt.format";
pub const MIME_FORMAT: &str = "validation.mime.format";
pub const NUMERIC_RANGE_INVALID: &str = "validation.range.out_of_range";
pub const EMAIL_FORMAT: &str = "validation.email.format";
pub const UUID_FORMAT: &str = "validation.uuid.format";
pub const NIF_FORMAT: &str = "validation.nif.format";
pub const NIF_CHECKSUM: &str = "validation.nif.checksum";
pub const NISS_FORMAT: &str = "validation.niss.format";
pub const NISS_CHECKSUM: &str = "validation.niss.checksum";
pub const CC_FORMAT: &str = "validation.cc.format";
pub const CC_CHECKSUM: &str = "validation.cc.checksum";
pub const IBAN_FORMAT: &str = "validation.iban.format";

// ── JSON / payload ────────────────────────────────────────────────────────────
pub const JSON_OBJECT: &str = "validation.json.object";
pub const JSON_REQUIRED_FIELD: &str = "validation.json.required_field";

// ── Integridade ───────────────────────────────────────────────────────────────
pub const HASH_SHA256_FORMAT: &str = "validation.hash.sha256_format";

// ── Coerência estrutural ──────────────────────────────────────────────────────
pub const DATE_FORMAT_INVALID: &str = "validation.coherence.date_format";
pub const DATE_RANGE_INVALID: &str = "validation.coherence.date_range";
pub const DATETIME_OFFSET_MISMATCH: &str = "validation.coherence.datetime_offset_mismatch";
pub const STATE_TRANSITION_INVALID: &str = "validation.coherence.state_transition";
