use serde_json::Value;

/// Canonicalises a JSON value per RFC 8785 (JSON Canonicalization Scheme).
///
/// - Object keys sorted by UTF-16 code unit sequence (§3.2.3)
/// - Arrays preserved in order
/// - Strings preserved as-is (no NFC normalisation)
/// - No whitespace
///
/// Requires serde_json compiled with `preserve_order` feature so that
/// `serde_json::Map` respects insertion order during serialisation.
pub fn canonicalise(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted = serde_json::Map::new();
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_by(|a, b| {
                let a16: Vec<u16> = a.encode_utf16().collect();
                let b16: Vec<u16> = b.encode_utf16().collect();
                a16.cmp(&b16)
            });
            for k in keys {
                sorted.insert(k.clone(), canonicalise(&map[k]));
            }
            Value::Object(sorted)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(canonicalise).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sorts_keys_by_utf16() {
        let val = json!({ "z": 1, "a": 2, "m": 3 });
        let c = canonicalise(&val);
        let keys: Vec<&str> = c.as_object().unwrap().keys().map(|s| s.as_str()).collect();
        assert_eq!(keys, vec!["a", "m", "z"]);
    }

    #[test]
    fn does_not_modify_strings() {
        let val = json!({ "key": "caf\u{00e9}" });
        let c = canonicalise(&val);
        assert_eq!(c["key"].as_str().unwrap(), "café");
    }

    #[test]
    fn nested_objects_sorted() {
        let val = json!({ "z": { "b": 1, "a": 2 }, "a": 3 });
        let c = canonicalise(&val);
        let outer: Vec<_> = c.as_object().unwrap().keys().collect();
        assert_eq!(outer, vec!["a", "z"]);
        let inner: Vec<_> = c["z"].as_object().unwrap().keys().collect();
        assert_eq!(inner, vec!["a", "b"]);
    }

    #[test]
    fn arrays_preserved_in_order() {
        let val = json!([3, 1, 2]);
        assert_eq!(canonicalise(&val), json!([3, 1, 2]));
    }

    #[test]
    fn idempotent() {
        let val = json!({ "c": 3, "a": 1, "b": 2 });
        let once = canonicalise(&val);
        let twice = canonicalise(&once);
        assert_eq!(once, twice);
    }
}
