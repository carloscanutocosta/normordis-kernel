use normordis_pdf::{
    parse_ndt, parse_ndt_data, serialize_ndt_json, serialize_ndt_toml,
    template::{check_version_compatibility, validator, TemplateError},
    DocumentBuilder, DocumentStyle,
};

const MINIMAL_NDT: &str = r#"{"ndt":"1.0.0","body":[]}"#;
const MINIMAL_DATA: &str = r#"{"ndt_data":"1.0.0","data":{}}"#;

// ── 1. parse_ndt valid ────────────────────────────────────────────────────────

#[test]
fn parse_ndt_valid_json_returns_ok() {
    assert!(parse_ndt(MINIMAL_NDT).is_ok());
}

// ── 2. parse_ndt invalid ──────────────────────────────────────────────────────

#[test]
fn parse_ndt_invalid_json_returns_err() {
    let result = parse_ndt("{not valid json");
    assert!(result.is_err(), "expected error for invalid JSON");
    assert!(matches!(result.unwrap_err(), TemplateError::JsonError(_)));
}

// ── 3. check_version_compatibility same version ───────────────────────────────

#[test]
fn version_compatibility_exact_match_ok() {
    assert!(check_version_compatibility("1.0.0").is_ok());
}

// ── 4. check_version_compatibility different minor ────────────────────────────

#[test]
fn version_compatibility_minor_ok() {
    assert!(check_version_compatibility("1.5.0").is_ok());
}

// ── 5. check_version_compatibility future major → error ───────────────────────

#[test]
fn version_compatibility_major_mismatch_err() {
    // Engine is 2.x — a template with major 3 must be rejected.
    let result = check_version_compatibility("3.0.0");
    assert!(result.is_err(), "future major version must be rejected");
    assert!(matches!(result.unwrap_err(), TemplateError::IncompatibleVersion { .. }));
}

#[test]
fn version_compatibility_v200_is_accepted() {
    // NDT 2.0.0 must now be accepted (engine is 2.x).
    assert!(check_version_compatibility("2.0.0").is_ok());
}

// ── 6. validate required placeholder absent ───────────────────────────────────

#[test]
fn validate_missing_required_placeholder_err() {
    use std::collections::HashMap;
    use normordis_pdf::template::model::PlaceholderDef;

    let mut defs = HashMap::new();
    defs.insert(
        "name".to_string(),
        PlaceholderDef {
            placeholder_type: Some("string".into()),
            required: Some(true),
            default: None,
            description: None,
            pattern: None,
            min: None,
            max: None,
        },
    );

    let data = parse_ndt_data(MINIMAL_DATA).unwrap();
    let result = validator::validate(&defs, &data);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TemplateError::MissingPlaceholder { .. }));
}

// ── 7. validate pattern mismatch → error ─────────────────────────────────────

#[test]
fn validate_pattern_mismatch_err() {
    use std::collections::HashMap;
    use normordis_pdf::template::model::PlaceholderDef;

    let mut defs = HashMap::new();
    defs.insert(
        "code".to_string(),
        PlaceholderDef {
            placeholder_type: None,
            required: Some(true),
            default: None,
            description: None,
            pattern: Some(r"^\d{4}$".into()),
            min: None,
            max: None,
        },
    );

    let data_json = r#"{"ndt_data":"1.0.0","data":{"code":"abc"}}"#;
    let data = parse_ndt_data(data_json).unwrap();
    let result = validator::validate(&defs, &data);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TemplateError::InvalidPlaceholder { .. }));
}

// ── 8. resolve_string replaces placeholder ────────────────────────────────────

#[test]
fn resolve_string_replaces_placeholder() {
    use normordis_pdf::template::resolver;

    let data = parse_ndt_data(r#"{"ndt_data":"1.0.0","data":{"name":"Maria"}}"#).unwrap();
    let result = resolver::resolve_string("Olá {{name}}!", &data);
    assert_eq!(result, "Olá Maria!");
}

// ── 9. resolve_string unknown key left as-is ──────────────────────────────────

#[test]
fn resolve_string_unknown_key_preserved() {
    use normordis_pdf::template::resolver;

    let data = parse_ndt_data(MINIMAL_DATA).unwrap();
    let result = resolver::resolve_string("Hello {{name}}", &data);
    assert_eq!(result, "Hello {{name}}");
}

// ── 10. resolve_string nested key ────────────────────────────────────────────

#[test]
fn resolve_string_nested_key() {
    use normordis_pdf::template::resolver;

    let json = r#"{"ndt_data":"1.0.0","data":{"obj":{"field":"world"}}}"#;
    let data = parse_ndt_data(json).unwrap();
    let result = resolver::resolve_string("Hello {{obj.field}}", &data);
    assert_eq!(result, "Hello world");
}

// ── 11. render_template conditional true branch ───────────────────────────────

#[test]
fn render_template_conditional_true_branch() {
    let template = r#"{
        "ndt": "1.0.0",
        "body": [{
            "type": "conditional",
            "condition": "show_title",
            "operator": "exists",
            "then": [{"type": "heading", "text": "Título", "level": 1}],
            "else": []
        }]
    }"#;
    let data = r#"{"ndt_data":"1.0.0","data":{"show_title":true}}"#;
    let style = DocumentStyle::default();

    let doc = parse_ndt(template).unwrap();
    let data = parse_ndt_data(data).unwrap();
    let els = normordis_pdf::template::render_template(&doc, &data, &style).unwrap();
    assert_eq!(els.len(), 1, "then-branch should produce 1 element");
}

// ── 12. render_template conditional false branch ──────────────────────────────

#[test]
fn render_template_conditional_false_branch() {
    let template = r#"{
        "ndt": "1.0.0",
        "body": [{
            "type": "conditional",
            "condition": "missing_key",
            "operator": "exists",
            "then": [{"type": "heading", "text": "Hidden", "level": 1}],
            "else": [{"type": "paragraph", "text": "Visible"}]
        }]
    }"#;
    let style = DocumentStyle::default();

    let doc = parse_ndt(template).unwrap();
    let data = parse_ndt_data(MINIMAL_DATA).unwrap();
    let els = normordis_pdf::template::render_template(&doc, &data, &style).unwrap();
    assert_eq!(els.len(), 1, "else-branch should produce 1 element");
}

// ── 13. render_template repeat ────────────────────────────────────────────────

#[test]
fn render_template_repeat_produces_n_elements() {
    let template = r#"{
        "ndt": "1.0.0",
        "body": [{
            "type": "repeat",
            "items": "{{items}}",
            "item_var": "item",
            "elements": [{"type": "paragraph", "text": "{{item}}"}]
        }]
    }"#;
    let data = r#"{"ndt_data":"1.0.0","data":{"items":["a","b","c"]}}"#;
    let style = DocumentStyle::default();

    let doc = parse_ndt(template).unwrap();
    let ndt_data = parse_ndt_data(data).unwrap();
    let els = normordis_pdf::template::render_template(&doc, &ndt_data, &style).unwrap();
    assert_eq!(els.len(), 3, "repeat should produce 3 elements for 3 items");
}

// ── 14. render_template zone_ref injects zone elements ───────────────────────

#[test]
fn render_template_zone_ref_injects_zone() {
    let template = r#"{
        "ndt": "1.0.0",
        "zones": {
            "footer_zone": {
                "elements": [
                    {"type": "spacer", "height_mm": 5.0},
                    {"type": "paragraph", "text": "Rodapé"}
                ]
            }
        },
        "body": [{"type": "zone_ref", "zone": "footer_zone"}]
    }"#;
    let style = DocumentStyle::default();

    let doc = parse_ndt(template).unwrap();
    let data = parse_ndt_data(MINIMAL_DATA).unwrap();
    let els = normordis_pdf::template::render_template(&doc, &data, &style).unwrap();
    assert_eq!(els.len(), 2, "zone should inject its 2 elements");
}

// ── 15. push_ndt in DocumentBuilder renders to valid PDF ─────────────────────

#[test]
fn push_ndt_builder_renders_ok() {
    let template = r#"{
        "ndt": "1.0.0",
        "body": [
            {"type": "heading", "text": "{{title}}", "level": 1},
            {"type": "paragraph", "text": "{{body}}"}
        ]
    }"#;
    let data = r#"{"ndt_data":"1.0.0","data":{"title":"Teste NDT","body":"Conteúdo do documento."}}"#;

    let pdf = DocumentBuilder::new("NDT Test")
        .push_ndt(template, data)
        .expect("push_ndt should succeed")
        .render_to_bytes()
        .expect("render should succeed");

    assert!(!pdf.is_empty(), "PDF should not be empty");
    assert!(pdf.starts_with(b"%PDF"), "output should start with %PDF");
}

// ── 16. parse_ndt TOML auto-detect ───────────────────────────────────────────

#[test]
fn parse_ndt_toml_returns_ok() {
    let toml = r#"
ndt = "1.0.0"

[[body]]
type = "paragraph"
text = "Olá mundo"
"#;
    let doc = parse_ndt(toml).expect("TOML NDT should parse");
    assert_eq!(doc.ndt, "1.0.0");
    assert_eq!(doc.body.len(), 1);
}

// ── 17. parse_ndt invalid TOML returns TomlError ──────────────────────────────

#[test]
fn parse_ndt_invalid_toml_returns_err() {
    let result = parse_ndt("ndt = this is not valid toml !!!!");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TemplateError::TomlError(_)));
}

// ── 18. serialize_ndt_json round-trip ─────────────────────────────────────────

#[test]
fn serialize_ndt_json_round_trip() {
    let doc = parse_ndt(MINIMAL_NDT).unwrap();
    let json = serialize_ndt_json(&doc).expect("JSON serialization should succeed");
    let doc2 = parse_ndt(&json).expect("re-parsed document should be valid");
    assert_eq!(doc.ndt, doc2.ndt);
}

// ── 19. serialize_ndt_toml round-trip ────────────────────────────────────────

#[test]
fn serialize_ndt_toml_round_trip() {
    let json = r#"{"ndt":"1.0.0","meta":{"title":"Teste"},"body":[{"type":"paragraph","text":"Body text"}]}"#;
    let doc = parse_ndt(json).unwrap();
    let toml_str = serialize_ndt_toml(&doc).expect("TOML serialization should succeed");
    let doc2 = parse_ndt(&toml_str).expect("TOML round-trip should parse back");
    assert_eq!(doc.ndt, doc2.ndt);
    assert_eq!(doc.body.len(), doc2.body.len());
    assert_eq!(
        doc.meta.as_ref().and_then(|m| m.title.as_deref()),
        doc2.meta.as_ref().and_then(|m| m.title.as_deref()),
    );
}
