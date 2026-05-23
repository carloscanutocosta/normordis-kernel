use serde_json::{Map, Value};

use crate::config::LoggingConfig;
use crate::event::LogEvent;

const REDACTED: &str = "[REDACTED]";
const TRUNCATED: &str = "[TRUNCATED]";
const SENSITIVE_KEY_FRAGMENTS: &[&str] = &[
    "password",
    "passphrase",
    "secret",
    "token",
    "key",
    "ciphertext",
    "plaintext",
    "payload",
    "authorization",
    "cookie",
];

pub fn sanitize_event(mut event: LogEvent, config: &LoggingConfig) -> LogEvent {
    event.component = sanitize_text(&event.component, 128);
    if let Some(code) = event.code.take() {
        event.code = Some(sanitize_text(&code, 128));
    }
    event.message = sanitize_text(&event.message, config.max_message_chars);
    if let Some(details) = event.details.take() {
        event.details = Some(limit_details(
            redact_value(details),
            config.max_details_bytes,
        ));
    }
    event
}

fn sanitize_text(value: &str, max_chars: usize) -> String {
    let sanitized = value.replace(['\r', '\n'], " ");
    truncate_chars(&sanitized, max_chars)
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }

    let mut output = value.chars().take(max_chars).collect::<String>();
    output.push_str(TRUNCATED);
    output
}

fn redact_value(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(redact_map(map)),
        Value::Array(items) => Value::Array(items.into_iter().map(redact_value).collect()),
        Value::String(value) => Value::String(sanitize_text(&value, 512)),
        other => other,
    }
}

fn redact_map(map: Map<String, Value>) -> Map<String, Value> {
    map.into_iter()
        .map(|(key, value)| {
            if is_sensitive_key(&key) {
                (key, Value::String(REDACTED.to_owned()))
            } else {
                (key, redact_value(value))
            }
        })
        .collect()
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    SENSITIVE_KEY_FRAGMENTS
        .iter()
        .any(|fragment| key.contains(fragment))
}

fn limit_details(value: Value, max_bytes: usize) -> Value {
    match serde_json::to_vec(&value) {
        Ok(bytes) if bytes.len() <= max_bytes => value,
        Ok(_) => Value::String(TRUNCATED.to_owned()),
        Err(_) => Value::String(TRUNCATED.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;

    use super::*;
    use crate::level::LogLevel;

    #[test]
    fn redacts_sensitive_details() {
        let mut config = LoggingConfig::new("logs", "app.log");
        config.max_details_bytes = 1024;
        let event = LogEvent {
            ts: Utc::now(),
            level: LogLevel::Info,
            component: "runtime".to_owned(),
            code: None,
            message: "ok".to_owned(),
            details: Some(json!({
                "password": "secret",
                "nested": {"token": "abc", "safe": "value"}
            })),
        };

        let sanitized = sanitize_event(event, &config);
        let details = sanitized.details.unwrap();

        assert_eq!(details["password"], REDACTED);
        assert_eq!(details["nested"]["token"], REDACTED);
        assert_eq!(details["nested"]["safe"], "value");
    }

    #[test]
    fn truncates_long_message() {
        let mut config = LoggingConfig::new("logs", "app.log");
        config.max_message_chars = 4;
        let event = LogEvent::new(LogLevel::Info, "runtime", "abcdef");

        let sanitized = sanitize_event(event, &config);

        assert_eq!(sanitized.message, format!("abcd{TRUNCATED}"));
    }
}
