use normordis_pdf::{DocumentBuilder, Paragraph, PreparedPdf, Section, SignatureOptions};

fn prepared_doc() -> PreparedPdf {
    DocumentBuilder::new("Documento Assinado")
        .push(Section::new("Secção 1", 1))
        .push(Paragraph::new("Conteúdo para assinatura digital."))
        .render_prepared_for_signing(SignatureOptions::default())
        .expect("render_prepared_for_signing failed")
}

// ── Basic structure ───────────────────────────────────────────────────────────

#[test]
fn sig_01_render_prepared_succeeds() {
    let _ = prepared_doc();
}

#[test]
fn sig_02_raw_bytes_not_empty() {
    let p = prepared_doc();
    assert!(!p.raw_bytes().is_empty());
}

#[test]
fn sig_03_starts_with_pdf_header() {
    let p = prepared_doc();
    assert!(p.raw_bytes().starts_with(b"%PDF-"));
}

#[test]
fn sig_04_byterange_patched_in_raw_bytes() {
    let p = prepared_doc();
    let raw = String::from_utf8_lossy(p.raw_bytes());
    // Placeholder must be gone
    assert!(
        !raw.contains("1111111111"),
        "ByteRange placeholder must be replaced in prepared PDF"
    );
    // /ByteRange must be present with real values
    assert!(raw.contains("/ByteRange"), "/ByteRange must be present");
}

#[test]
fn sig_05_contents_placeholder_still_in_raw_bytes() {
    // Before embed_signature() the /Contents field still has placeholder data
    let p = prepared_doc();
    let raw = p.raw_bytes();
    // The Contents field should contain a hex string starting with '<'
    // (it's not been patched yet)
    assert!(
        raw.windows(b"/Contents <".len())
            .any(|w| w == b"/Contents <"),
        "/Contents hex placeholder must be present"
    );
}

#[test]
fn sig_06_sig_filter_adobe_ppklite() {
    let p = prepared_doc();
    let raw = String::from_utf8_lossy(p.raw_bytes());
    assert!(
        raw.contains("/Adobe.PPKLite"),
        "/Filter /Adobe.PPKLite must be present"
    );
}

#[test]
fn sig_07_subfilter_pkcs7_detached() {
    let p = prepared_doc();
    let raw = String::from_utf8_lossy(p.raw_bytes());
    assert!(
        raw.contains("adbe.pkcs7.detached"),
        "/SubFilter /adbe.pkcs7.detached must be present"
    );
}

#[test]
fn sig_08_sig_type_present() {
    let p = prepared_doc();
    let raw = String::from_utf8_lossy(p.raw_bytes());
    assert!(
        raw.contains("/Type /Sig"),
        "/Type /Sig must be present in sig value dict"
    );
}

#[test]
fn sig_09_acroform_in_catalog() {
    let p = prepared_doc();
    let raw = String::from_utf8_lossy(p.raw_bytes());
    assert!(raw.contains("/AcroForm"), "Catalog must contain /AcroForm");
}

#[test]
fn sig_10_sig_flags_3() {
    let p = prepared_doc();
    let raw = String::from_utf8_lossy(p.raw_bytes());
    assert!(
        raw.contains("/SigFlags"),
        "/SigFlags must be present in AcroForm"
    );
}

// ── ByteRange / bytes_to_sign ─────────────────────────────────────────────────

#[test]
fn sig_11_byte_range_tuple_valid() {
    let p = prepared_doc();
    let (r1_start, r1_len, r2_start, r2_len) = p.byte_range();
    assert_eq!(r1_start, 0, "range1 must start at 0");
    assert!(r1_len > 0, "range1 must have positive length");
    assert!(r2_start > r1_len, "range2 must start after range1 ends");
    assert!(r2_len > 0, "range2 must have positive length");
    let total = r1_len + (r2_start - r1_len - 1) + 1 + r2_len; // with Contents bytes
    assert!(
        total <= p.raw_bytes().len() as u64 + 10,
        "ranges must not exceed file size"
    );
}

#[test]
fn sig_12_byte_ranges_non_overlapping() {
    let p = prepared_doc();
    let (_, r1_len, r2_start, _) = p.byte_range();
    // There is a gap between range1 end and range2 start (the /Contents hex).
    assert!(
        r2_start > r1_len,
        "Contents placeholder must create a gap between ranges"
    );
}

#[test]
fn sig_13_bytes_to_sign_covers_most_of_file() {
    let p = prepared_doc();
    let to_sign = p.bytes_to_sign();
    let file_size = p.raw_bytes().len();
    // bytes_to_sign should be file_size minus the Contents hex placeholder
    // (reserved_bytes * 2 + 2 for '<' and '>')
    let reserved_hex = SignatureOptions::default().reserved_bytes * 2 + 2;
    assert_eq!(
        to_sign.len(),
        file_size - reserved_hex,
        "bytes_to_sign must equal file size minus reserved /Contents hex"
    );
}

#[test]
fn sig_14_bytes_to_sign_starts_with_pdf_header() {
    let p = prepared_doc();
    let to_sign = p.bytes_to_sign();
    assert!(
        to_sign.starts_with(b"%PDF-"),
        "bytes_to_sign must start with %PDF-"
    );
}

// ── embed_signature ───────────────────────────────────────────────────────────

#[test]
fn sig_15_embed_empty_pkcs7_succeeds() {
    let p = prepared_doc();
    let dummy_pkcs7 = vec![0x30u8, 0x00u8]; // minimal DER sequence
    let signed = p
        .embed_signature(&dummy_pkcs7)
        .expect("embed_signature failed");
    assert!(signed.starts_with(b"%PDF-"));
}

#[test]
fn sig_16_embed_signature_patches_contents() {
    let p = prepared_doc();
    let dummy_pkcs7 = vec![0xAAu8, 0xBBu8, 0xCCu8];
    let signed = p
        .embed_signature(&dummy_pkcs7)
        .expect("embed_signature failed");
    // The hex encoding of 0xAA 0xBB 0xCC is "aabbcc"
    let raw = String::from_utf8_lossy(&signed);
    assert!(
        raw.contains("aabbcc"),
        "signed PDF must contain hex-encoded PKCS#7 bytes"
    );
}

#[test]
fn sig_17_embed_oversized_signature_errors() {
    let opts = SignatureOptions {
        reserved_bytes: 8,
        ..Default::default()
    };
    let p = DocumentBuilder::new("Test")
        .push(Paragraph::new("x"))
        .render_prepared_for_signing(opts)
        .expect("render failed");
    let oversized = vec![0u8; 100]; // larger than reserved 8 bytes
    assert!(
        p.embed_signature(&oversized).is_err(),
        "oversized signature must return Err"
    );
}

#[test]
fn sig_18_signed_pdf_size_unchanged() {
    let p = prepared_doc();
    let file_size = p.raw_bytes().len();
    let dummy_pkcs7 = vec![0x42u8; 100];
    let signed = p.embed_signature(&dummy_pkcs7).expect("embed failed");
    assert_eq!(
        signed.len(),
        file_size,
        "embed_signature must not change file size"
    );
}

// ── Reason / Location ─────────────────────────────────────────────────────────

#[test]
fn sig_19_custom_reason_in_prepared() {
    // Use pure ASCII reason so TextStr encodes it as a literal string
    let opts = SignatureOptions {
        reason: "Signed by director".into(),
        location: "Lisbon".into(),
        reserved_bytes: 8192,
    };
    let p = DocumentBuilder::new("Doc")
        .push(Paragraph::new("Text."))
        .render_prepared_for_signing(opts)
        .expect("render failed");
    let raw = String::from_utf8_lossy(p.raw_bytes());
    assert!(
        raw.contains("Signed by director"),
        "custom ASCII reason must appear verbatim in prepared PDF"
    );
    assert!(
        raw.contains("Lisbon"),
        "custom location must appear in prepared PDF"
    );
}

// ── Compatibility: PDF/A + signature ─────────────────────────────────────────

#[test]
fn sig_20_pdfa_and_signature_together() {
    let opts = SignatureOptions::default();
    let p = DocumentBuilder::new("PDF/A Assinado")
        .pdfa()
        .push(Paragraph::new("Texto."))
        .render_prepared_for_signing(opts)
        .expect("render failed");

    let raw = String::from_utf8_lossy(p.raw_bytes());
    // Both PDF/A and signature structures must be present
    assert!(
        raw.contains("pdfaid:part"),
        "PDF/A metadata must be present"
    );
    assert!(raw.contains("/AcroForm"), "AcroForm must be present");
    assert!(
        raw.contains("adbe.pkcs7.detached"),
        "sig subfilter must be present"
    );
}
