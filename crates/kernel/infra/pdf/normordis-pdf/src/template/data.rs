use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

/// Input data bound to an NDT template.
#[derive(Debug, Clone, Deserialize)]
pub struct NdtData {
    pub ndt_data: String,
    pub template_id: Option<String>,
    pub template_version: Option<String>,
    pub data: HashMap<String, Value>,
}

impl NdtData {
    /// Resolve a potentially nested key like `"obj.field"` into the
    /// corresponding JSON value.  Returns `None` if any segment is missing.
    pub fn get(&self, key: &str) -> Option<&Value> {
        let mut parts = key.splitn(2, '.');
        let first = parts.next()?;
        let rest = parts.next();

        let root = self.data.get(first)?;

        match rest {
            None => Some(root),
            Some(tail) => resolve_nested(root, tail),
        }
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.get(key).map(|v| match v {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        })
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| v.as_bool())
    }

    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| v.as_f64())
    }
}

fn resolve_nested<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let mut parts = key.splitn(2, '.');
    let first = parts.next()?;
    let rest = parts.next();

    let child = value.as_object()?.get(first)?;

    match rest {
        None => Some(child),
        Some(tail) => resolve_nested(child, tail),
    }
}
