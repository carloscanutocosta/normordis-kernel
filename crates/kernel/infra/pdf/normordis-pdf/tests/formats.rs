use normordis_pdf::{
    compile_ndt, parse_ndf, parse_ncrtf, render_ndf, verify_ndf,
    Actor, AuditEvent, CompileOptions, EventType,
    NcrtfImage, NcrtfMark, NdfRevision,
    canonical_hash,
};
use normordis_pdf::ndf::jcs;
use serde_json::json;

// ── helpers ───────────────────────────────────────────────────────────────────

fn minimal_ndt() -> &'static str {
    r#"{"ndt":"1.1.0","meta":{"title":"Test Document"},"body":[{"type":"paragraph","text":"Hello {{name}}."}]}"#
}

fn minimal_data_json() -> &'static str {
    r#"{"ndt_data":"1.0.0","data":{"name":"World"}}"#
}

fn minimal_data() -> normordis_pdf::template::NdtData {
    serde_json::from_str(minimal_data_json()).unwrap()
}

fn simple_ndf() -> normordis_pdf::NdfDocument {
    compile_ndt(minimal_ndt(), &minimal_data(), CompileOptions::default()).unwrap()
}

fn now_ts() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ── 01–05: JCS / RFC 8785 ────────────────────────────────────────────────────

#[test]
fn fmt_01_jcs_sorts_keys_by_utf16() {
    let val = json!({ "z": 1, "a": 2, "m": 3 });
    let c = jcs::canonicalise(&val);
    let keys: Vec<&str> = c.as_object().unwrap().keys().map(|s| s.as_str()).collect();
    assert_eq!(keys, vec!["a", "m", "z"]);
}

#[test]
fn fmt_02_jcs_preserves_strings() {
    let val = json!({ "k": "caf\u{00e9}" });
    let c = jcs::canonicalise(&val);
    assert_eq!(c["k"].as_str().unwrap(), "café");
}

#[test]
fn fmt_03_jcs_idempotent() {
    let val = json!({ "c": 3, "a": 1, "b": { "z": 9, "a": 1 } });
    let once = jcs::canonicalise(&val);
    let twice = jcs::canonicalise(&once);
    assert_eq!(once, twice);
}

#[test]
fn fmt_04_jcs_preserves_array_order() {
    let val = json!([3, 1, 2]);
    assert_eq!(jcs::canonicalise(&val), json!([3, 1, 2]));
}

#[test]
fn fmt_05_canonical_hash_deterministic() {
    let val = json!({"b": 2, "a": 1});
    let h1 = canonical_hash(&val);
    let h2 = canonical_hash(&val);
    assert_eq!(h1, h2);
    assert!(h1.starts_with("sha256:"), "hash must start with 'sha256:'");
}

// ── 06–10: NdfIntegrity ──────────────────────────────────────────────────────

#[test]
fn fmt_06_integrity_compute_ok() {
    let ndf = simple_ndf();
    assert!(!ndf.integrity.content_hash.is_empty());
    assert!(!ndf.integrity.styles_hash.is_empty());
    assert!(!ndf.integrity.payload_hash.is_empty());
    assert!(!ndf.integrity.ndf_hash.is_empty());
    assert_eq!(ndf.integrity.algorithm, "sha256");
}

#[test]
fn fmt_07_content_hash_matches_canonical_hash_of_content() {
    let ndf = simple_ndf();
    let expected = canonical_hash(&ndf.content);
    assert_eq!(ndf.integrity.content_hash, expected);
}

#[test]
fn fmt_08_styles_hash_matches_canonical_hash_of_styles() {
    let ndf = simple_ndf();
    let expected = canonical_hash(&ndf.styles);
    assert_eq!(ndf.integrity.styles_hash, expected);
}

#[test]
fn fmt_09_payload_hash_covers_meta_styles_content() {
    let ndf = simple_ndf();
    let meta_val = serde_json::to_value(&ndf.meta).unwrap();
    let payload_val = json!({
        "content": ndf.content,
        "meta":    meta_val,
        "styles":  ndf.styles,
    });
    let expected = canonical_hash(&payload_val);
    assert_eq!(ndf.integrity.payload_hash, expected);
}

#[test]
fn fmt_10_ndf_hash_not_empty_and_different_from_payload_hash() {
    let ndf = simple_ndf();
    assert!(!ndf.integrity.ndf_hash.is_empty());
    // ndf_hash includes origin etc — must differ from payload_hash
    assert_ne!(ndf.integrity.ndf_hash, ndf.integrity.payload_hash);
}

// ── 11–18: compile_ndt ───────────────────────────────────────────────────────

#[test]
fn fmt_11_compile_ndt_json_ok() {
    let ndf = compile_ndt(minimal_ndt(), &minimal_data(), CompileOptions::default());
    assert!(ndf.is_ok(), "compile_ndt must return Ok: {:?}", ndf.err());
}

#[test]
fn fmt_12_compile_ndt_toml_ok() {
    let toml = r#"
ndt = "1.1.0"
[meta]
title = "TOML Doc"
[[body]]
type = "paragraph"
text = "Static TOML."
"#;
    let data: normordis_pdf::template::NdtData =
        serde_json::from_str(r#"{"ndt_data":"1.0.0","data":{}}"#).unwrap();
    let ndf = compile_ndt(toml, &data, CompileOptions::default());
    assert!(ndf.is_ok(), "compile_ndt TOML must return Ok: {:?}", ndf.err());
}

#[test]
fn fmt_13_compile_ndt_resolves_placeholders() {
    let ndf = simple_ndf();
    let content_str = serde_json::to_string(&ndf.content).unwrap();
    assert!(content_str.contains("World"), "{{name}} must be resolved to 'World'");
    assert!(!content_str.contains("{{name}}"), "raw placeholder must not remain");
}

#[test]
fn fmt_14_compile_ndt_missing_placeholder_errors() {
    let ndt = r#"{"ndt":"1.1.0","placeholders":{"required_field":{"required":true}},"body":[]}"#;
    let empty_data: normordis_pdf::template::NdtData =
        serde_json::from_str(r#"{"ndt_data":"1.0.0","data":{}}"#).unwrap();
    let result = compile_ndt(ndt, &empty_data, CompileOptions::default());
    assert!(result.is_err(), "missing required placeholder must return Err");
}

#[test]
fn fmt_15_compile_ndt_validate_resolved_false_allows_remaining() {
    let ndt = r#"{"ndt":"1.1.0","body":[{"type":"paragraph","text":"{{missing}}"}]}"#;
    let empty_data: normordis_pdf::template::NdtData =
        serde_json::from_str(r#"{"ndt_data":"1.0.0","data":{}}"#).unwrap();
    let opts = CompileOptions { validate_resolved: false, ..Default::default() };
    let result = compile_ndt(ndt, &empty_data, opts);
    assert!(result.is_ok(), "validate_resolved=false must not fail on remaining placeholders");
}

#[test]
fn fmt_16_compile_ndt_ndf_version() {
    let ndf = simple_ndf();
    assert_eq!(ndf.ndf, "1.1.0");
}

#[test]
fn fmt_17_compile_ndt_audit_has_one_generated_event() {
    let ndf = simple_ndf();
    assert_eq!(ndf.audit.events.len(), 1);
    assert_eq!(ndf.audit.events[0].event_type, EventType::DocumentGenerated);
    assert_eq!(ndf.audit.events[0].seq, 1);
}

#[test]
fn fmt_18_compile_ndt_content_hash_not_empty() {
    let ndf = simple_ndf();
    assert!(!ndf.integrity.content_hash.is_empty());
    assert!(ndf.integrity.content_hash.starts_with("sha256:"));
}

// ── 19–22: render_ndf / parse_ndf ────────────────────────────────────────────

#[test]
fn fmt_19_render_ndf_returns_bytes() {
    let ndf = simple_ndf();
    let json = ndf.to_canonical_json().unwrap();
    let pdf = render_ndf(&json);
    assert!(pdf.is_ok(), "render_ndf must return Ok: {:?}", pdf.err());
    assert!(!pdf.unwrap().is_empty());
}

#[test]
fn fmt_20_render_ndf_starts_with_pdf_header() {
    let ndf = simple_ndf();
    let json = ndf.to_canonical_json().unwrap();
    let pdf = render_ndf(&json).unwrap();
    assert!(pdf.starts_with(b"%PDF-"), "rendered bytes must start with %PDF-");
}

#[test]
fn fmt_21_parse_ndf_from_canonical_json_roundtrip() {
    let ndf = simple_ndf();
    let canonical = ndf.to_canonical_json().unwrap();
    let restored = parse_ndf(&canonical).unwrap();
    assert_eq!(restored.ndf, ndf.ndf);
    assert_eq!(restored.meta.title, ndf.meta.title);
    assert_eq!(restored.integrity.content_hash, ndf.integrity.content_hash);
}

#[test]
fn fmt_22_parse_ndf_from_pretty_json_roundtrip() {
    let ndf = simple_ndf();
    let pretty = ndf.to_pretty_json().unwrap();
    let restored = parse_ndf(&pretty).unwrap();
    assert_eq!(restored.integrity.ndf_hash, ndf.integrity.ndf_hash);
}

// ── 23–25: verify_ndf ────────────────────────────────────────────────────────

#[test]
fn fmt_23_verify_ndf_intact_all_valid() {
    let ndf = simple_ndf();
    let json = ndf.to_pretty_json().unwrap();
    let report = verify_ndf(&json).unwrap();
    assert!(report.all_valid, "integrity must be valid for unmodified NDF; failures: {:?}", report.failures);
}

#[test]
fn fmt_24_verify_ndf_tampered_content_fails() {
    let mut ndf = simple_ndf();
    ndf.content = json!([{"type":"paragraph","text":"TAMPERED"}]);
    let json = ndf.to_pretty_json().unwrap();
    let report = verify_ndf(&json).unwrap();
    assert!(!report.content_hash_valid, "tampered content must fail content_hash check");
    assert!(!report.all_valid);
}

#[test]
fn fmt_25_verify_ndf_tampered_styles_fails() {
    let mut ndf = simple_ndf();
    ndf.styles = json!({"font_size_body": 99.0});
    let json = ndf.to_pretty_json().unwrap();
    let report = verify_ndf(&json).unwrap();
    assert!(!report.styles_hash_valid, "tampered styles must fail styles_hash check");
}

// ── 26–29: NdfDocument::add_event ────────────────────────────────────────────

#[test]
fn fmt_26_add_event_correct_hash_ok() {
    let mut ndf = simple_ndf();
    let event = AuditEvent {
        seq: 0,
        event_type: EventType::DocumentReviewed,
        timestamp: now_ts(),
        actor: Actor::System { id: "test".into(), version: None, instance_id: None },
        content_hash: Some(ndf.integrity.content_hash.clone()),
        note: None,
        extra: Default::default(),
    };
    assert!(ndf.add_event(event).is_ok());
    assert_eq!(ndf.audit.events.len(), 2);
}

#[test]
fn fmt_27_add_event_wrong_hash_errors() {
    let mut ndf = simple_ndf();
    let event = AuditEvent {
        seq: 0,
        event_type: EventType::DocumentReviewed,
        timestamp: now_ts(),
        actor: Actor::System { id: "test".into(), version: None, instance_id: None },
        content_hash: Some("sha256:wronghash".into()),
        note: None,
        extra: Default::default(),
    };
    assert!(ndf.add_event(event).is_err(), "wrong hash must return Err");
}

#[test]
fn fmt_28_add_event_increments_seq() {
    let mut ndf = simple_ndf();
    assert_eq!(ndf.audit.events.last().unwrap().seq, 1);
    let event = AuditEvent {
        seq: 0,
        event_type: EventType::DocumentApproved,
        timestamp: now_ts(),
        actor: Actor::System { id: "x".into(), version: None, instance_id: None },
        content_hash: Some(ndf.integrity.content_hash.clone()),
        note: None,
        extra: Default::default(),
    };
    ndf.add_event(event).unwrap();
    assert_eq!(ndf.audit.events.last().unwrap().seq, 2);
}

#[test]
fn fmt_29_add_event_non_monotonic_timestamp_errors() {
    let mut ndf = simple_ndf();
    let past = "2000-01-01T00:00:00Z".to_string();
    let event = AuditEvent {
        seq: 0,
        event_type: EventType::DocumentReviewed,
        timestamp: past,
        actor: Actor::System { id: "x".into(), version: None, instance_id: None },
        content_hash: None,
        note: None,
        extra: Default::default(),
    };
    assert!(ndf.add_event(event).is_err(), "past timestamp must return Err");
}

// ── 30–33: NdfRevision::create_from ──────────────────────────────────────────

#[test]
fn fmt_30_revision_seq_is_2_for_original() {
    let original = simple_ndf();
    let actor = Actor::System { id: "test".into(), version: None, instance_id: None };
    let revised = NdfRevision::create_from(
        &original,
        json!([{"type":"paragraph","text":"Revised content."}]),
        actor,
        "Fix typo",
        None,
    ).unwrap();
    assert_eq!(revised.revision.as_ref().unwrap().revision_seq, 2);
}

#[test]
fn fmt_31_revision_does_not_modify_original() {
    let original = simple_ndf();
    let original_hash = original.integrity.content_hash.clone();
    let original_events = original.audit.events.len();

    let actor = Actor::System { id: "test".into(), version: None, instance_id: None };
    let _revised = NdfRevision::create_from(
        &original,
        json!([{"type":"paragraph","text":"Changed."}]),
        actor,
        "reason",
        None,
    ).unwrap();

    assert_eq!(original.integrity.content_hash, original_hash);
    assert_eq!(original.audit.events.len(), original_events);
}

#[test]
fn fmt_32_revision_of_matches_original_document_id() {
    let original = simple_ndf();
    let doc_id = original.audit.document_id.clone();
    let actor = Actor::System { id: "test".into(), version: None, instance_id: None };
    let revised = NdfRevision::create_from(
        &original,
        json!([{"type":"paragraph","text":"New."}]),
        actor,
        "reason",
        None,
    ).unwrap();
    assert_eq!(revised.revision.as_ref().unwrap().revision_of, doc_id);
}

#[test]
fn fmt_33_revised_content_hash_differs_from_original() {
    let original = simple_ndf();
    let actor = Actor::System { id: "test".into(), version: None, instance_id: None };
    let revised = NdfRevision::create_from(
        &original,
        json!([{"type":"paragraph","text":"Completely different content."}]),
        actor,
        "reason",
        None,
    ).unwrap();
    assert_ne!(
        revised.integrity.content_hash,
        original.integrity.content_hash,
        "revised document must have a different content_hash"
    );
}

// ── 34–42: NCRTF ─────────────────────────────────────────────────────────────

#[test]
fn fmt_34_parse_ncrtf_130_ok() {
    let json = r#"{
        "ncrtf": "1.3.0",
        "blocks": [
            {
                "type": "paragraph",
                "children": [
                    {"type": "text", "text": "Hello"},
                    {"type": "footnote_ref", "number": 1},
                    {"type": "hard_break"}
                ]
            }
        ]
    }"#;
    let doc = parse_ncrtf(json);
    assert!(doc.is_ok(), "parse_ncrtf 1.3.0 must return Ok: {:?}", doc.err());
    assert_eq!(doc.unwrap().ncrtf, "1.3.0");
}

#[test]
fn fmt_35_footnote_ref_inline_deserializes() {
    use normordis_pdf::richtext::model::Inline;
    let json = r#"{"ncrtf":"1.3.0","blocks":[{"type":"paragraph","children":[{"type":"footnote_ref","number":3}]}]}"#;
    let doc = parse_ncrtf(json).unwrap();
    let block = &doc.blocks[0];
    if let normordis_pdf::richtext::model::Block::Paragraph(p) = block {
        assert!(
            p.children.iter().any(|i| matches!(i, Inline::FootnoteRef(n) if n.number == 3)),
            "FootnoteRef number=3 must be deserialised"
        );
    } else {
        panic!("expected paragraph block");
    }
}

#[test]
fn fmt_36_soft_hyphen_preserved_in_text_node() {
    let json = r#"{"ncrtf":"1.3.0","blocks":[{"type":"paragraph","children":[{"type":"text","text":"im­ple­men­ta­ção"}]}]}"#;
    let doc = parse_ncrtf(json).unwrap();
    use normordis_pdf::richtext::model::{Block, Inline};
    if let Block::Paragraph(p) = &doc.blocks[0] {
        if let Inline::Text(t) = &p.children[0] {
            assert!(t.text.contains('\u{00AD}'), "soft hyphen U+00AD must be preserved");
        }
    }
}

#[test]
fn fmt_37_validate_src_data_uri_ok() {
    let img = NcrtfImage {
        src: "data:image/png;base64,abc123".into(),
        alt: None,
        caption: None,
        alignment: None,
        width_percent: None,
    };
    assert!(img.validate_src().is_ok());
}

#[test]
fn fmt_38_validate_src_https_errors() {
    let img = NcrtfImage {
        src: "https://example.com/logo.png".into(),
        alt: None,
        caption: None,
        alignment: None,
        width_percent: None,
    };
    assert!(img.validate_src().is_err(), "https:// src must return Err");
}

#[test]
fn fmt_39_mark_simple_bold_mark_type() {
    let m = NcrtfMark::Bold;
    assert_eq!(m.mark_type(), "bold");
}

#[test]
fn fmt_40_mark_parameterised_color_mark_type() {
    let m = NcrtfMark::Color("#CC0000".into());
    assert_eq!(m.mark_type(), "color");
}

#[test]
fn fmt_41_ncrtf_to_elements_with_footnote_ref_no_panic() {
    use normordis_pdf::richtext::ncrtf_to_elements;
    use normordis_pdf::styles::DocumentStyle;
    let json = r#"{
        "ncrtf": "1.3.0",
        "blocks": [
            {"type": "paragraph", "children": [
                {"type": "text", "text": "See note"},
                {"type": "footnote_ref", "number": 1}
            ]}
        ]
    }"#;
    let doc = parse_ncrtf(json).unwrap();
    let style = DocumentStyle::default();
    let elements = ncrtf_to_elements(&doc, &style);
    assert!(!elements.is_empty(), "must produce at least one element");
}

#[test]
fn fmt_42_ncrtf_meta_is_separate_from_ndt_meta() {
    // Verify at the type level that NcrtfMeta and NdtMeta are different structs.
    // NcrtfMeta has `custom: Option<HashMap<String, Value>>`
    // NdtMeta has `compat_mode: Option<u32>` — field not present in NcrtfMeta.
    use normordis_pdf::richtext::model::DocumentMeta;
    use normordis_pdf::template::model::NdtMeta;

    let ncrtf_meta: DocumentMeta = serde_json::from_str(
        r#"{"title":"Editorial only","author":"Editor"}"#,
    ).unwrap();
    assert_eq!(ncrtf_meta.title.as_deref(), Some("Editorial only"));

    let ndt_meta: NdtMeta = serde_json::from_str(
        r#"{"title":"Template title","compat_mode":16}"#,
    ).unwrap();
    assert_eq!(ndt_meta.compat_mode, Some(16));
    // The two structs exist independently — compiles only if they are separate types.
    drop(ncrtf_meta);
    drop(ndt_meta);
}
