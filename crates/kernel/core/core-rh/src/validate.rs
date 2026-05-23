//! Funções de validação de campos primitivos de `core-rh` (user_id, username, email, etc.).

use crate::RhError;
use core_validation::validators::string;

pub const USER_ID_MAX_LENGTH: usize = 128;

pub fn validate_user_id_value(value: &str) -> Result<(), RhError> {
    if !string::required("user_id", value).is_valid()
        || !string::max_length("user_id", value, USER_ID_MAX_LENGTH).is_valid()
        || value.chars().any(char::is_whitespace)
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
    {
        return Err(RhError::InvalidUserId);
    }

    Ok(())
}

pub fn validate_role_id(value: &str) -> Result<(), RhError> {
    if !string::required("role_id", value).is_valid() || value.chars().any(char::is_whitespace) {
        return Err(RhError::InvalidRole);
    }

    Ok(())
}

pub fn validate_username(value: &str) -> Result<(), RhError> {
    if !string::required("username", value).is_valid() || value.chars().any(char::is_whitespace) {
        return Err(RhError::InvalidProfile);
    }

    Ok(())
}

pub fn validate_optional_email(value: Option<&str>) -> Result<(), RhError> {
    let Some(value) = value else {
        return Ok(());
    };

    if value.trim().is_empty() {
        return Ok(());
    }

    if !core_validation::validators::email::validate_email("email", value).is_valid() {
        return Err(RhError::InvalidProfile);
    }

    Ok(())
}

pub fn validate_required_display_name(
    field: &'static str,
    value: &str,
    error: RhError,
) -> Result<(), RhError> {
    if !string::required(field, value).is_valid() {
        return Err(error);
    }

    Ok(())
}

pub fn validate_org_unit_id(value: &str) -> Result<(), RhError> {
    if !string::required("org_unit_id", value).is_valid() {
        return Err(RhError::InvalidOrgRef);
    }

    Ok(())
}

pub fn validate_position_id(value: &str) -> Result<(), RhError> {
    if !string::required("position_id", value).is_valid() {
        return Err(RhError::InvalidOrgRef);
    }
    Ok(())
}

pub fn validate_competency_id(value: &str) -> Result<(), RhError> {
    if !string::required("competency_id", value).is_valid() {
        return Err(RhError::InvalidOrgRef);
    }
    Ok(())
}
