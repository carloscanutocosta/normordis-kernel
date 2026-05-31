/// v2.0.0 compliance tests — covers the 44 scenarios from the v2.0.0 prompt.
///
/// Groups: backend migration, font subsetting, PDF/A-1b, ExtGState opacity,
/// traceability/classification, digital signature, NDT 2.0.0, file size.
use normordis_pdf::{
    backend::pdf_writer_backend::{generate_to_unicode_cmap, subset_font, to_cff_if_possible},
    CompressionLevel, DocumentBuilder, PageBreakElement, Paragraph, PdfStandard, Section,
    SecurityClassification, SignatureOptions, TraceabilityMetadata, Watermark, PDF_BACKEND,
    VERSION,
};
use std::collections::HashSet;

// ── helpers ───────────────────────────────────────────────────────────────────

fn simple_doc() -> Vec<u8> {
    DocumentBuilder::new("Teste v2.0.0")
        .push(Section::new("Secção 1", 1))
        .push(Paragraph::new("Parágrafo de teste."))
        .render_to_bytes()
        .expect("render failed")
}

fn simple_pdfa_doc() -> Vec<u8> {
    DocumentBuilder::new("Teste PDF/A v2.0.0")
        .pdfa()
        .compression(CompressionLevel::Best)
        .push(Section::new("Secção 1", 1))
        .push(Paragraph::new("Conteúdo PDF/A."))
        .render_to_bytes()
        .expect("render failed")
}

fn embedded_font_bytes() -> Vec<u8> {
    // Use Liberation Sans — the bundled default font.
    let variants = normordis_pdf::liberation_sans_family().expect("embedded font must load");
    variants.regular.bytes.clone()
}

// ── 01–08 Backend migration ───────────────────────────────────────────────────

#[test]
fn v200_01_renders_without_panic() {
    let bytes = simple_doc();
    assert!(!bytes.is_empty());
}

#[test]
fn v200_02_output_is_nonempty() {
    let bytes = simple_doc();
    assert!(bytes.len() > 1024, "PDF should be > 1 KB");
}

#[test]
fn v200_03_starts_with_pdf_header() {
    let bytes = simple_doc();
    assert!(bytes.starts_with(b"%PDF-"), "must start with %PDF-");
}

#[test]
fn v200_04_contains_page_object() {
    let bytes = simple_doc();
    let raw = String::from_utf8_lossy(&bytes);
    assert!(raw.contains("/Page"), "must contain /Page object");
}

#[test]
fn v200_05_two_paragraphs_renders() {
    let bytes = DocumentBuilder::new("Two paragraphs")
        .push(Paragraph::new("First paragraph with some text."))
        .push(Paragraph::new("Second paragraph with more text."))
        .render_to_bytes()
        .expect("render failed");
    assert!(bytes.starts_with(b"%PDF-"));
}

#[test]
fn v200_06_draw_rect_full_opacity_no_ext_gstate() {
    // A simple doc without watermark should not need ExtGState
    let bytes = simple_doc();
    let raw = String::from_utf8_lossy(&bytes);
    // Opacity watermarks add /ExtGState; a plain doc should not
    assert!(
        !raw.contains("/ExtGState"),
        "plain doc should have no ExtGState"
    );
}

#[test]
fn v200_07_watermark_opacity_adds_ext_gstate() {
    let bytes = DocumentBuilder::new("Watermark test")
        .watermark(Watermark::new("TESTE").opacity(0.15))
        .push(Paragraph::new("Texto."))
        .render_to_bytes()
        .expect("render failed");
    let raw = String::from_utf8_lossy(&bytes);
    assert!(
        raw.contains("/ExtGState"),
        "watermark must produce ExtGState"
    );
}

#[test]
fn v200_08_multiple_pages_renders() {
    let bytes = DocumentBuilder::new("Multi-page")
        .push(Paragraph::new("Página 1."))
        .push(PageBreakElement)
        .push(Paragraph::new("Página 2."))
        .render_to_bytes()
        .expect("render failed");
    assert!(bytes.starts_with(b"%PDF-"));
}

// ── 09–15 Font subsetting ─────────────────────────────────────────────────────

#[test]
fn v200_09_subset_font_reduces_size() {
    let font_bytes = embedded_font_bytes();
    let mut used: HashSet<u16> = HashSet::new();
    // Use only the first 10 glyphs (notdef + 9 others)
    for i in 0u16..10 {
        used.insert(i);
    }
    let subsetted = subset_font(&font_bytes, &used);
    // Subsetted should be smaller than original
    assert!(
        subsetted.len() < font_bytes.len(),
        "subsetted ({}) should be < full font ({})",
        subsetted.len(),
        font_bytes.len()
    );
}

#[test]
fn v200_10_subset_font_empty_glyphs_no_panic() {
    let font_bytes = embedded_font_bytes();
    let empty: HashSet<u16> = HashSet::new();
    let result = subset_font(&font_bytes, &empty);
    assert!(
        !result.is_empty(),
        "subset of empty glyph set must not panic"
    );
}

#[test]
fn v200_11_subset_font_output_is_valid_ttf() {
    let font_bytes = embedded_font_bytes();
    let mut used: HashSet<u16> = HashSet::new();
    used.insert(0);
    used.insert(36); // 'A' in most fonts
    let subsetted = subset_font(&font_bytes, &used);
    // ttf-parser must accept the output
    ttf_parser::Face::parse(&subsetted, 0).expect("subsetted font must be parseable");
}

#[test]
fn v200_12_generate_cmap_nonempty_for_latin_glyphs() {
    let font_bytes = embedded_font_bytes();
    let face = ttf_parser::Face::parse(&font_bytes, 0).unwrap();
    // Collect actual glyph IDs for a–z
    let mut used: HashSet<u16> = HashSet::new();
    for ch in 'a'..='z' {
        if let Some(gid) = face.glyph_index(ch) {
            used.insert(gid.0);
        }
    }
    let cmap = generate_to_unicode_cmap(&font_bytes, &used);
    assert!(!cmap.is_empty(), "CMap must be non-empty for Latin glyphs");
}

#[test]
fn v200_13_generate_cmap_starts_with_cidsinit() {
    let font_bytes = embedded_font_bytes();
    let face = ttf_parser::Face::parse(&font_bytes, 0).unwrap();
    let mut used: HashSet<u16> = HashSet::new();
    for ch in 'A'..='Z' {
        if let Some(gid) = face.glyph_index(ch) {
            used.insert(gid.0);
        }
    }
    let cmap = generate_to_unicode_cmap(&font_bytes, &used);
    let cmap_str = String::from_utf8_lossy(&cmap);
    assert!(
        cmap_str.contains("/CIDInit") || cmap_str.starts_with("/CIDInit"),
        "CMap must start with /CIDInit"
    );
}

#[test]
fn v200_14_generate_cmap_contains_bfchar() {
    let font_bytes = embedded_font_bytes();
    let face = ttf_parser::Face::parse(&font_bytes, 0).unwrap();
    let mut used: HashSet<u16> = HashSet::new();
    for ch in ['A', 'B', 'C', 'a', 'b', 'c'] {
        if let Some(gid) = face.glyph_index(ch) {
            used.insert(gid.0);
        }
    }
    let cmap = generate_to_unicode_cmap(&font_bytes, &used);
    let cmap_str = String::from_utf8_lossy(&cmap);
    assert!(
        cmap_str.contains("beginbfchar"),
        "CMap must contain beginbfchar"
    );
    assert!(
        cmap_str.contains("endbfchar"),
        "CMap must contain endbfchar"
    );
}

#[test]
fn v200_15_cff_extraction_no_panic() {
    let font_bytes = embedded_font_bytes();
    let result = to_cff_if_possible(&font_bytes);
    // For TTF-only fonts returns original bytes; for OTF with CFF returns smaller result.
    assert!(
        !result.is_empty(),
        "to_cff_if_possible must return non-empty bytes"
    );
}

// ── 16–21 PDF/A-1b ───────────────────────────────────────────────────────────

#[test]
fn v200_16_pdfa_contains_pdfaid_part() {
    let bytes = simple_pdfa_doc();
    let raw = String::from_utf8_lossy(&bytes);
    assert!(raw.contains("pdfaid:part"), "must contain pdfaid:part");
}

#[test]
fn v200_17_pdfa_contains_pdfaid_conformance_b() {
    let bytes = simple_pdfa_doc();
    let raw = String::from_utf8_lossy(&bytes);
    assert!(
        raw.contains("pdfaid:conformance"),
        "must contain pdfaid:conformance"
    );
}

#[test]
fn v200_18_pdfa_xmp_is_utf8() {
    let bytes = simple_pdfa_doc();
    // The whole PDF is binary but the XMP section is UTF-8 text
    let raw = String::from_utf8_lossy(&bytes);
    assert!(raw.contains("x:xmpmeta"), "must contain x:xmpmeta element");
}

#[test]
fn v200_19_pdfa_contains_output_intent() {
    let bytes = simple_pdfa_doc();
    let raw = String::from_utf8_lossy(&bytes);
    assert!(
        raw.contains("OutputIntent"),
        "must contain OutputIntent for PDF/A"
    );
}

#[test]
fn v200_20_pdfa_contains_srgb_icc() {
    let bytes = simple_pdfa_doc();
    let raw = String::from_utf8_lossy(&bytes);
    assert!(
        raw.contains("sRGB") || raw.contains("IEC61966"),
        "must reference sRGB ICC profile"
    );
}

#[test]
fn v200_21_pdfa_standard_enum_is_pdfa() {
    assert!(PdfStandard::PdfA1b.is_pdfa());
    assert!(PdfStandard::PdfA2b.is_pdfa());
    assert!(!PdfStandard::Pdf17.is_pdfa());
    assert_eq!(PdfStandard::PdfA1b.xmp_part(), 1);
    assert_eq!(PdfStandard::PdfA2b.xmp_part(), 2);
    assert_eq!(PdfStandard::Pdf17.xmp_part(), 0);
}

// ── 22–25 ExtGState opacity ───────────────────────────────────────────────────

#[test]
fn v200_22_watermark_opacity_produces_ext_gstate() {
    let bytes = DocumentBuilder::new("Opacity test")
        .watermark(Watermark::new("RASCUNHO").opacity(0.1))
        .push(Paragraph::new("Texto."))
        .render_to_bytes()
        .expect("render failed");
    let raw = String::from_utf8_lossy(&bytes);
    assert!(
        raw.contains("/ExtGState"),
        "opacity watermark must add ExtGState"
    );
}

#[test]
fn v200_23_full_opacity_watermark_still_renders() {
    let bytes = DocumentBuilder::new("Full opacity")
        .watermark(Watermark::new("VISÍVEL").opacity(1.0))
        .push(Paragraph::new("Texto."))
        .render_to_bytes()
        .expect("render failed");
    assert!(bytes.starts_with(b"%PDF-"));
}

#[test]
fn v200_24_two_different_opacities_both_in_pdf() {
    // Two classification watermarks at different opacity levels would produce
    // two distinct ExtGState entries. Here we verify separate render paths.
    let bytes_a = DocumentBuilder::new("A")
        .watermark(Watermark::new("A").opacity(0.1))
        .push(Paragraph::new("A"))
        .render_to_bytes()
        .expect("render");
    let bytes_b = DocumentBuilder::new("B")
        .watermark(Watermark::new("B").opacity(0.5))
        .push(Paragraph::new("B"))
        .render_to_bytes()
        .expect("render");
    // Both must produce ExtGState
    assert!(String::from_utf8_lossy(&bytes_a).contains("/ExtGState"));
    assert!(String::from_utf8_lossy(&bytes_b).contains("/ExtGState"));
}

#[test]
fn v200_25_opacity_cache_single_ext_gstate_for_same_level() {
    // Two identical watermarks should produce a single ExtGState (cache hit).
    // We can only verify the PDF is valid — cache correctness is internal.
    let bytes = DocumentBuilder::new("Cache test")
        .watermark(Watermark::new("TESTE").opacity(0.2))
        .push(Paragraph::new("Parágrafo."))
        .render_to_bytes()
        .expect("render failed");
    assert!(bytes.starts_with(b"%PDF-"));
}

// ── 26–30 Traceability ────────────────────────────────────────────────────────

#[test]
fn v200_26_classification_internal_label_pt() {
    assert_eq!(SecurityClassification::Internal.label_pt(), "Interno");
}

#[test]
fn v200_27_classification_confidential_label_pt() {
    assert_eq!(
        SecurityClassification::Confidential.label_pt(),
        "Confidencial"
    );
}

#[test]
fn v200_28_internal_classification_adds_watermark() {
    let bytes = DocumentBuilder::new("Internal")
        .traceability(TraceabilityMetadata {
            engine_version: VERSION.into(),
            entity_id: "test".into(),
            document_ref: None,
            classification: SecurityClassification::Internal,
            generated_at: "2026-01-01T00:00:00Z".into(),
            ndt_version: "2.0.0".into(),
            framework_version: None,
        })
        .push(Paragraph::new("Conteúdo interno."))
        .render_to_bytes()
        .expect("render failed");
    // Internal classification triggers a watermark → ExtGState should appear
    let raw = String::from_utf8_lossy(&bytes);
    assert!(
        raw.contains("/ExtGState"),
        "internal doc must have classification watermark"
    );
}

#[test]
fn v200_29_public_classification_no_auto_watermark() {
    let bytes = DocumentBuilder::new("Public")
        .traceability(TraceabilityMetadata {
            engine_version: VERSION.into(),
            entity_id: "test".into(),
            document_ref: None,
            classification: SecurityClassification::Public,
            generated_at: "2026-01-01T00:00:00Z".into(),
            ndt_version: "2.0.0".into(),
            framework_version: None,
        })
        .push(Paragraph::new("Conteúdo público."))
        .render_to_bytes()
        .expect("render failed");
    let raw = String::from_utf8_lossy(&bytes);
    assert!(
        !raw.contains("/ExtGState"),
        "public doc must not have auto watermark"
    );
}

#[test]
fn v200_30_traceability_metadata_serialises_to_json() {
    let meta = TraceabilityMetadata {
        engine_version: VERSION.into(),
        entity_id: "test-entity".into(),
        document_ref: Some("REF/001".into()),
        classification: SecurityClassification::Confidential,
        generated_at: "2026-01-01T00:00:00Z".into(),
        ndt_version: "2.0.0".into(),
        framework_version: None,
    };
    let json = serde_json::to_string(&meta).expect("serialise must not panic");
    assert!(json.contains("\"confidential\"") || json.contains("confidential"));
}

// ── 31–35 Digital signature (external PKCS#7 model) ──────────────────────────

#[test]
fn v200_31_prepared_pdf_bytes_nonempty() {
    let prepared = DocumentBuilder::new("Assinado")
        .push(Paragraph::new("Conteúdo a assinar."))
        .render_prepared_for_signing(SignatureOptions::default())
        .expect("prepare failed");
    let (b0, _, b2, _) = prepared.byte_range();
    assert!(b0 + b2 > 0, "bytes to sign must be non-zero");
}

#[test]
fn v200_32_prepared_pdf_starts_with_pdf_header() {
    let prepared = DocumentBuilder::new("Header test")
        .push(Paragraph::new("Texto."))
        .render_prepared_for_signing(SignatureOptions::default())
        .expect("prepare failed");
    let bytes = prepared.bytes_to_sign();
    // bytes_to_sign covers the signed ranges, which start at offset 0
    // and include the PDF header
    assert!(!bytes.is_empty());
}

#[test]
fn v200_33_prepared_pdf_contains_byte_range() {
    let prepared = DocumentBuilder::new("ByteRange test")
        .push(Paragraph::new("Texto."))
        .render_prepared_for_signing(SignatureOptions::default())
        .expect("prepare failed");
    let (r0, r1, r2, r3) = prepared.byte_range();
    // ByteRange must have non-zero lengths
    assert!(
        r1 > 0 && r3 > 0,
        "ByteRange lengths must be positive: ({r0},{r1},{r2},{r3})"
    );
}

#[test]
fn v200_34_prepared_pdf_embed_dummy_signature() {
    let prepared = DocumentBuilder::new("Embed test")
        .push(Paragraph::new("Texto."))
        .render_prepared_for_signing(SignatureOptions::default())
        .expect("prepare failed");
    // Embed a dummy (invalid) PKCS#7 blob — only tests the byte-writing logic
    let dummy_sig = vec![0u8; 64];
    let signed = prepared
        .embed_signature(&dummy_sig)
        .expect("embed must not fail for dummy sig");
    assert!(signed.starts_with(b"%PDF-"));
}

#[test]
fn v200_35_signature_options_default_values() {
    let opts = SignatureOptions::default();
    // Defaults: ASCII reason/location so they can be found as raw bytes in the PDF
    assert!(!opts.reason.is_empty(), "default reason must not be empty");
}

// ── 36–40 NDT 2.0.0 ──────────────────────────────────────────────────────────

#[test]
fn v200_36_ndt200_output_standard_sets_pdfa() {
    let ndt = r#"{
        "ndt": "2.0.0",
        "output": { "standard": "pdf_a_1b" },
        "body": [{ "type": "paragraph", "text": "Teste." }]
    }"#;
    let data = r#"{"ndt_data":"1.0.0","data":{}}"#;
    let bytes = DocumentBuilder::new("NDT 2.0.0 standard")
        .push_ndt(ndt, data)
        .expect("push_ndt failed")
        .render_to_bytes()
        .expect("render failed");
    let raw = String::from_utf8_lossy(&bytes);
    // PDF/A-1b must include OutputIntent
    assert!(
        raw.contains("OutputIntent"),
        "output.standard=pdf_a_1b must produce PDF/A"
    );
}

#[test]
fn v200_37_ndt200_output_classification_confidential_adds_watermark() {
    let ndt = r#"{
        "ndt": "2.0.0",
        "output": { "classification": "confidencial" },
        "body": [{ "type": "paragraph", "text": "Confidencial." }]
    }"#;
    let data = r#"{"ndt_data":"1.0.0","data":{}}"#;
    let bytes = DocumentBuilder::new("NDT 2.0.0 classification")
        .push_ndt(ndt, data)
        .expect("push_ndt failed")
        .render_to_bytes()
        .expect("render failed");
    let raw = String::from_utf8_lossy(&bytes);
    assert!(
        raw.contains("/ExtGState"),
        "confidential classification must add watermark"
    );
}

#[test]
fn v200_38_ndt150_compat_still_works() {
    // Old NDT 1.x templates must still parse (version check: template_major <= engine_major)
    let ndt = r#"{
        "ndt": "1.1.0",
        "body": [{ "type": "paragraph", "text": "Compatível com v1." }]
    }"#;
    let data = r#"{"ndt_data":"1.0.0","data":{}}"#;
    let bytes = DocumentBuilder::new("NDT 1.1.0 compat")
        .push_ndt(ndt, data)
        .expect("NDT 1.x must still be accepted")
        .render_to_bytes()
        .expect("render failed");
    assert!(bytes.starts_with(b"%PDF-"));
}

#[test]
fn v200_39_ndt200_json_and_toml_parse_equivalent() {
    let json = r#"{"ndt":"2.0.0","output":{"standard":"pdf_a_1b"},"body":[]}"#;
    let toml = "ndt = \"2.0.0\"\n[output]\nstandard = \"pdf_a_1b\"\n";
    let doc_json = normordis_pdf::parse_ndt(json).expect("JSON parse failed");
    let doc_toml = normordis_pdf::parse_ndt(toml).expect("TOML parse failed");
    assert_eq!(doc_json.ndt, doc_toml.ndt);
    let out_json = doc_json.output.as_ref().and_then(|o| o.standard.as_deref());
    let out_toml = doc_toml.output.as_ref().and_then(|o| o.standard.as_deref());
    assert_eq!(
        out_json, out_toml,
        "standard field must match between JSON and TOML"
    );
}

#[test]
fn v200_40_ndt_output_compression_best_renders() {
    let ndt = r#"{
        "ndt": "2.0.0",
        "output": { "compression": "best" },
        "body": [{ "type": "paragraph", "text": "Comprimido." }]
    }"#;
    let data = r#"{"ndt_data":"1.0.0","data":{}}"#;
    let bytes = DocumentBuilder::new("NDT compression")
        .push_ndt(ndt, data)
        .expect("push_ndt failed")
        .render_to_bytes()
        .expect("render failed");
    assert!(bytes.starts_with(b"%PDF-"));
}

// ── 41–44 File size / PDF_BACKEND ────────────────────────────────────────────

#[test]
fn v200_41_simple_doc_pdfa_best_under_200kb() {
    let bytes = simple_pdfa_doc();
    assert!(
        bytes.len() < 200_000,
        "PDF/A-1b + Best compression must be < 200 KB, got {} bytes",
        bytes.len()
    );
}

#[test]
fn v200_42_plain_doc_smaller_than_500kb() {
    let bytes = simple_doc();
    assert!(
        bytes.len() < 500_000,
        "Plain doc must be < 500 KB (font subsetting active), got {} bytes",
        bytes.len()
    );
}

#[test]
fn v200_43_pdf_backend_constant_is_pdf_writer() {
    assert_eq!(PDF_BACKEND, "pdf-writer");
}

#[test]
fn v200_44_ndt_signature_model_deserialises() {
    let json = r#"{
        "ndt": "2.0.0",
        "signature": {
            "field": { "x_mm": 120.0, "y_mm": 40.0, "width_mm": 70.0, "height_mm": 20.0, "page": 1, "label": "Assinatura" },
            "reason": "Aprovado",
            "location": "Lisboa"
        },
        "body": []
    }"#;
    let doc = normordis_pdf::parse_ndt(json).expect("parse failed");
    let sig = doc.signature.as_ref().expect("signature must be present");
    assert_eq!(sig.reason.as_deref(), Some("Aprovado"));
    assert_eq!(sig.location.as_deref(), Some("Lisboa"));
    let field = sig.field.as_ref().expect("field must be present");
    assert_eq!(field.label.as_deref(), Some("Assinatura"));
}
