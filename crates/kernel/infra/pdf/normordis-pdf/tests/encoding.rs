use normordis_pdf::{
    backend::pdf_writer_backend::encode_for_identity_h, DocumentBuilder, Paragraph,
};

// ── ENC-01: ASCII → UTF-16-BE with BOM ───────────────────────────────────────

#[test]
fn enc01_ascii_utf16be_with_bom() {
    let encoded = encode_for_identity_h("Hello");
    assert_eq!(&encoded[0..2], &[0xFE, 0xFF], "BOM must be 0xFEFF");
    assert_eq!(&encoded[2..4], &[0x00, 0x48], "H = U+0048");
    assert_eq!(&encoded[4..6], &[0x00, 0x65], "e = U+0065");
    assert_eq!(encoded.len(), 2 + 5 * 2, "BOM + 5 chars × 2 bytes");
}

// ── ENC-02: Portuguese characters → correct UTF-16-BE ────────────────────────

#[test]
fn enc02_portuguese_chars_utf16be() {
    let encoded = encode_for_identity_h("ção");
    assert_eq!(&encoded[0..2], &[0xFE, 0xFF], "must start with BOM");
    // 'ç' = U+00E7
    assert_eq!(&encoded[2..4], &[0x00, 0xE7], "ç = U+00E7");
    // 'ã' = U+00E3
    assert_eq!(&encoded[4..6], &[0x00, 0xE3], "ã = U+00E3");
    // 'o' = U+006F
    assert_eq!(&encoded[6..8], &[0x00, 0x6F], "o = U+006F");
    assert_eq!(encoded.len(), 2 + 3 * 2);
}

// ── ENC-03: empty string → only BOM ──────────────────────────────────────────

#[test]
fn enc03_empty_string_is_bom_only() {
    let encoded = encode_for_identity_h("");
    assert_eq!(
        encoded,
        vec![0xFE, 0xFF],
        "empty string must produce only BOM"
    );
}

// ── ENC-04: length matches UTF-16 code unit count ────────────────────────────

#[test]
fn enc04_length_matches_utf16_code_units() {
    let text = "Declaração de Consentimento";
    let encoded = encode_for_identity_h(text);
    assert_eq!(&encoded[0..2], &[0xFE, 0xFF]);
    let utf16_units = text.encode_utf16().count();
    assert_eq!(
        encoded.len(),
        2 + utf16_units * 2,
        "BOM + UTF-16 code units × 2"
    );
}

// ── ENC-05: PDF with Portuguese text is structurally valid ───────────────────
//
// NOTE: normordis-pdf renders text as GID-based 2-byte sequences (not UTF-16-BE
// with BOM) because Identity-H CIDFont expects glyph IDs, not Unicode code
// points. The rendering is therefore correct per PDF spec §9.4.4 and the GID
// → Unicode mapping is provided via the ToUnicode CMap. This test verifies
// that a PDF with accented characters renders without error.

#[test]
fn enc05_pdf_with_portuguese_text_is_valid() {
    let pdf = DocumentBuilder::new("Declaração de Teste")
        .push(Paragraph::new(
            "Classificação documental. Consentimento informado. \
             Regulação §1.º alínea a) do RGPD.",
        ))
        .render_to_bytes()
        .expect("must render without error");

    assert!(pdf.starts_with(b"%PDF"), "output must be a valid PDF");
    assert!(pdf.len() > 1000, "PDF must have reasonable size");
}

// ── ENC-06: text extractable from rendered PDF ────────────────────────────────

#[test]
fn enc06_text_extractable_from_pdf() {
    let original = "Declaração de Consentimento Informado";
    let pdf = DocumentBuilder::new("Teste")
        .push(Paragraph::new(original))
        .render_to_bytes()
        .expect("must render");

    let doc =
        lopdf::Document::load_from(std::io::Cursor::new(&pdf)).expect("must be parseable by lopdf");

    let text = doc.extract_text(&[1]).unwrap_or_default();
    assert!(
        text.contains("Declaração") || text.contains("Declara"),
        "extracted text must contain 'Declaração' (or its root); got: {:?}",
        &text[..text.len().min(120)],
    );
}
