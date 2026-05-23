use support_errors::{Component, ErrorCode, MiniError};

pub const SECRETS_COMPONENT: &str = "infra-secrets";
pub const PROTECT_FAILED: &str = "MINI.SECRETS.PROTECT_FAILED";
pub const UNPROTECT_FAILED: &str = "MINI.SECRETS.UNPROTECT_FAILED";
pub const WEAK_PASSPHRASE: &str = "MINI.SECRETS.WEAK_PASSPHRASE";
pub const UNSUPPORTED_PLATFORM: &str = "MINI.SECRETS.UNSUPPORTED_PLATFORM";

pub fn secret_error(code: &str, message: &str) -> MiniError {
    MiniError::new(
        ErrorCode::new(code).expect("infra-secrets error codes must be valid"),
        Component::new(SECRETS_COMPONENT).expect("infra-secrets component must be valid"),
        message,
    )
}
