use std::collections::HashMap;

use regex::Regex;

use super::{data::NdtData, model::PlaceholderDef, TemplateError};

/// Validate all placeholder definitions against the provided data.
pub fn validate(
    placeholders: &HashMap<String, PlaceholderDef>,
    data: &NdtData,
) -> Result<(), TemplateError> {
    for (name, def) in placeholders {
        let required = def.required.unwrap_or(false);
        let value = data.get(name);

        if value.is_none() {
            if required && def.default.is_none() {
                return Err(TemplateError::MissingPlaceholder { name: name.clone() });
            }
            continue; // optional or has default — skip further checks
        }

        let val = value.unwrap();

        // Pattern check (string values only)
        if let Some(pattern) = &def.pattern {
            if let Some(s) = val.as_str() {
                let re = Regex::new(pattern).map_err(|e| TemplateError::InvalidPlaceholder {
                    name: name.clone(),
                    reason: format!("invalid pattern regex: {e}"),
                })?;
                if !re.is_match(s) {
                    return Err(TemplateError::InvalidPlaceholder {
                        name: name.clone(),
                        reason: format!("value '{s}' does not match pattern '{pattern}'"),
                    });
                }
            }
        }

        // Numeric range checks
        if let Some(min) = def.min {
            if let Some(n) = val.as_f64() {
                if n < min {
                    return Err(TemplateError::InvalidPlaceholder {
                        name: name.clone(),
                        reason: format!("value {n} is below minimum {min}"),
                    });
                }
            }
        }
        if let Some(max) = def.max {
            if let Some(n) = val.as_f64() {
                if n > max {
                    return Err(TemplateError::InvalidPlaceholder {
                        name: name.clone(),
                        reason: format!("value {n} exceeds maximum {max}"),
                    });
                }
            }
        }

        // Type check
        if let Some(expected_type) = &def.placeholder_type {
            let actual_type = json_type_name(val);
            let ok = match expected_type.as_str() {
                "string" => val.is_string(),
                "number" => val.is_number(),
                "boolean" => val.is_boolean(),
                "array" => val.is_array(),
                "object" => val.is_object(),
                "ncrtf" => val.is_string(),
                _ => true,
            };
            if !ok {
                return Err(TemplateError::PlaceholderTypeMismatch {
                    name: name.clone(),
                    expected: expected_type.clone(),
                    got: actual_type.to_string(),
                });
            }
        }
    }
    Ok(())
}

fn json_type_name(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}
