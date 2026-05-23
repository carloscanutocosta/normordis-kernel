use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostalCode {
    pub cp4: String,
    pub cp3: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressCandidate {
    pub postal_code_id: i64,
    pub postal_code: String,
    pub postal_designation: Option<String>,
    pub locality_name: Option<String>,
    pub artery_type: Option<String>,
    pub artery_title: Option<String>,
    pub artery_name: Option<String>,
    pub artery_local: Option<String>,
    pub section: Option<String>,
}

impl AddressCandidate {
    pub fn display_label(&self) -> String {
        let mut parts = Vec::new();

        let mut artery = Vec::new();
        if let Some(value) = non_empty(self.artery_type.as_deref()) {
            artery.push(value.to_string());
        }
        if let Some(value) = non_empty(self.artery_title.as_deref()) {
            artery.push(value.to_string());
        }
        if let Some(value) = non_empty(self.artery_name.as_deref()) {
            artery.push(value.to_string());
        }
        if !artery.is_empty() {
            parts.push(artery.join(" "));
        }

        if let Some(value) = non_empty(self.artery_local.as_deref()) {
            parts.push(value.to_string());
        }
        if let Some(value) = non_empty(self.section.as_deref()) {
            parts.push(value.to_string());
        }
        if let Some(value) = non_empty(self.locality_name.as_deref()) {
            parts.push(value.to_string());
        }

        if parts.is_empty() {
            self.postal_code.clone()
        } else {
            parts.join(", ")
        }
    }

    pub fn format_postal_address(&self) -> String {
        let mut first_line_parts = Vec::new();
        let mut artery = Vec::new();

        if let Some(value) = non_empty(self.artery_type.as_deref()) {
            artery.push(value.to_string());
        }
        if let Some(value) = non_empty(self.artery_title.as_deref()) {
            artery.push(value.to_string());
        }
        if let Some(value) = non_empty(self.artery_name.as_deref()) {
            artery.push(value.to_string());
        }
        if !artery.is_empty() {
            first_line_parts.push(artery.join(" "));
        }
        if let Some(value) = non_empty(self.artery_local.as_deref()) {
            first_line_parts.push(value.to_string());
        }
        if let Some(value) = non_empty(self.locality_name.as_deref()) {
            first_line_parts.push(value.to_string());
        }

        let mut second_line_parts = vec![self.postal_code.clone()];
        if let Some(value) = non_empty(self.postal_designation.as_deref()) {
            second_line_parts.push(value.to_string());
        }

        let first_line = first_line_parts.join(", ");
        let second_line = second_line_parts.join(" ");
        if first_line.is_empty() {
            second_line
        } else {
            format!("{first_line}\r\n{second_line}")
        }
    }
}

#[derive(Debug, Error)]
pub enum AddressError {
    #[error("codigo postal invalido: {0}")]
    InvalidPostalCode(String),
}

pub fn parse_postal_code(value: &str) -> Result<PostalCode, AddressError> {
    let value = value.trim();
    let (cp4, cp3) = value
        .split_once('-')
        .ok_or_else(|| AddressError::InvalidPostalCode(value.to_string()))?;
    validate_postal_parts(cp4, cp3)?;
    Ok(PostalCode {
        cp4: cp4.to_string(),
        cp3: cp3.to_string(),
    })
}

pub fn validate_postal_parts(cp4: &str, cp3: &str) -> Result<(), AddressError> {
    if cp4.len() != 4
        || cp3.len() != 3
        || !cp4.chars().all(|ch| ch.is_ascii_digit())
        || !cp3.chars().all(|ch| ch.is_ascii_digit())
    {
        return Err(AddressError::InvalidPostalCode(format!("{cp4}-{cp3}")));
    }
    Ok(())
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
