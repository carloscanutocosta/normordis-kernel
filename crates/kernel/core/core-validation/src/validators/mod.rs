pub mod email;
pub mod iban;
pub mod json;
pub mod nif;
pub mod string;
pub mod uuid;

pub use email::validate_email;
pub use iban::{normalize_iban, validate_iban};
pub use json::{require_field, require_object};
pub use nif::{normalize_nif, validate_nif};
pub use string::{max_length, required};
pub use uuid::validate_uuid;
