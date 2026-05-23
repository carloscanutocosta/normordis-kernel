use normordis_pdf::{CompressionLevel, DocumentBuilder, Paragraph, Section};

fn pdfa_doc() -> Vec<u8> {
    DocumentBuilder::new("Teste PDF/A")
        .pdfa()
        .push(Section::new("Secção 1", 1))
        .push(Paragraph::new("Parágrafo de teste para PDF/A-1b."))
        .render_to_bytes()
        .expect("render failed")
}

fn plain_doc() -> Vec<u8> {
    DocumentBuilder::new("Teste Plain")
        .push(Section::new("Secção 1", 1))
        .push(Paragraph::new("Parágrafo de teste."))
        .render_to_bytes()
        .expect("render failed")
}

// ── Basic structure ───────────────────────────────────────────────────────────

#[test]
fn pdfa_01_renders_without_error() {
    let bytes = pdfa_doc();
    assert!(!bytes.is_empty());
}

#[test]
fn pdfa_02_starts_with_pdf_header() {
    let bytes = pdfa_doc();
    assert!(bytes.starts_with(b"%PDF-"), "must start with %PDF-");
}

#[test]
fn pdfa_03_contains_xmp_namespace() {
    let bytes = pdfa_doc();
    let raw = String::from_utf8_lossy(&bytes);
    assert!(
        raw.contains("http://www.aiim.org/pdfa/ns/id/"),
        "XMP must contain pdfaid namespace"
    );
}

#[test]
fn pdfa_04_pdfaid_part_1() {
    let bytes = pdfa_doc();
    let raw = String::from_utf8_lossy(&bytes);
    assert!(raw.contains("<pdfaid:part>1</pdfaid:part>"), "must declare PDF/A part 1");
}

#[test]
fn pdfa_05_pdfaid_conformance_b() {
    let bytes = pdfa_doc();
    let raw = String::from_utf8_lossy(&bytes);
    assert!(
        raw.contains("<pdfaid:conformance>B</pdfaid:conformance>"),
        "must declare conformance level B"
    );
}

#[test]
fn pdfa_06_output_intent_present() {
    let bytes = pdfa_doc();
    let raw = String::from_utf8_lossy(&bytes);
    assert!(
        raw.contains("GTS_PDFA1") || raw.contains("OutputIntent"),
        "must contain OutputIntent / GTS_PDFA1"
    );
}

#[test]
fn pdfa_07_srgb_icc_identifier() {
    let bytes = pdfa_doc();
    let raw = String::from_utf8_lossy(&bytes);
    assert!(
        raw.contains("sRGB IEC61966-2.1"),
        "OutputIntent must identify the sRGB IEC61966-2.1 profile"
    );
}

#[test]
fn pdfa_08_metadata_stream_type() {
    let bytes = pdfa_doc();
    let raw = String::from_utf8_lossy(&bytes);
    assert!(raw.contains("/Type /Metadata"), "must contain a /Type /Metadata stream");
}

#[test]
fn pdfa_09_metadata_subtype_xml() {
    let bytes = pdfa_doc();
    let raw = String::from_utf8_lossy(&bytes);
    assert!(raw.contains("/Subtype /XML"), "metadata stream must have /Subtype /XML");
}

#[test]
fn pdfa_10_xmp_xpacket_wrapper() {
    let bytes = pdfa_doc();
    let raw = String::from_utf8_lossy(&bytes);
    assert!(raw.contains("<?xpacket"), "XMP must be wrapped in xpacket PI");
    assert!(raw.contains("<?xpacket end="), "XMP must close with xpacket end PI");
}

// ── ICC profile ───────────────────────────────────────────────────────────────

#[test]
fn pdfa_11_icc_profile_bytes_present() {
    let bytes = pdfa_doc();
    // sRGB v2 ICC profile starts with specific 4-byte size field (0x00000C48 = 3144)
    let profile_header: [u8; 4] = [0x00, 0x00, 0x0C, 0x48];
    let found = bytes.windows(4).any(|w| w == profile_header);
    assert!(found, "sRGB ICC profile header bytes not found in PDF");
}

#[test]
fn pdfa_12_icc_n3_colour_components() {
    // /N 3 must appear near the ICC stream (3 colour components for RGB)
    let bytes = pdfa_doc();
    let raw = String::from_utf8_lossy(&bytes);
    assert!(raw.contains("/N 3"), "ICC profile must declare /N 3 (RGB)");
}

// ── Content unchanged ─────────────────────────────────────────────────────────

#[test]
fn pdfa_13_title_in_xmp() {
    let bytes = pdfa_doc();
    let raw = String::from_utf8_lossy(&bytes);
    assert!(
        raw.contains("Teste PDF/A"),
        "document title must appear in XMP metadata"
    );
}

#[test]
fn pdfa_14_size_under_600kb() {
    // PDF/A overhead (XMP + ICC) should stay modest on top of ~113KB base.
    let bytes = pdfa_doc();
    assert!(
        bytes.len() < 600_000,
        "PDF/A doc should be under 600KB, got {} bytes",
        bytes.len()
    );
}

#[test]
fn pdfa_15_plain_doc_has_no_xmp() {
    // Non-PDF/A path must not emit XMP metadata.
    let bytes = plain_doc();
    let raw = String::from_utf8_lossy(&bytes);
    assert!(
        !raw.contains("pdfaid:part"),
        "plain doc must not contain pdfaid XMP namespace"
    );
}

#[test]
fn pdfa_16_pdfa_doc_larger_than_plain() {
    // PDF/A adds ICC profile + XMP, so it must be larger than the plain equivalent.
    let pdfa_size  = pdfa_doc().len();
    let plain_size = plain_doc().len();
    assert!(
        pdfa_size > plain_size,
        "PDF/A ({pdfa_size}) should be larger than plain ({plain_size})"
    );
}

// ── XMP content ──────────────────────────────────────────────────────────────

#[test]
fn pdfa_17_dc_format_application_pdf() {
    let bytes = pdfa_doc();
    let raw = String::from_utf8_lossy(&bytes);
    assert!(
        raw.contains("<dc:format>application/pdf</dc:format>"),
        "XMP must declare dc:format = application/pdf"
    );
}

#[test]
fn pdfa_18_xmp_creator_tool() {
    let bytes = pdfa_doc();
    let raw = String::from_utf8_lossy(&bytes);
    assert!(
        raw.contains("<xmp:CreatorTool>normordis-pdf</xmp:CreatorTool>"),
        "XMP must declare CreatorTool = normordis-pdf"
    );
}

// ── XML special chars in title ────────────────────────────────────────────────

#[test]
fn pdfa_19_title_xml_escaping() {
    let bytes = DocumentBuilder::new("A & B <test>")
        .pdfa()
        .push(Paragraph::new("body"))
        .render_to_bytes()
        .expect("render failed");
    let raw = String::from_utf8_lossy(&bytes);
    // Escaped form must appear in the XMP stream.
    assert!(raw.contains("A &amp; B &lt;test&gt;"), "title must be XML-escaped in XMP");
    // XMP section must not have the unescaped & immediately before "B <test>".
    // (The PDF info dict legitimately contains the raw bytes; we scope to XMP.)
    let xmp_start = raw.find("<?xpacket").expect("xpacket PI not found");
    let xmp_end   = raw.rfind("?>").expect("xpacket end not found") + 2;
    let xmp_section = &raw[xmp_start..xmp_end];
    assert!(
        !xmp_section.contains("A & B"),
        "unescaped ampersand must not appear inside the XMP packet"
    );
}

// ── Compression compatibility ─────────────────────────────────────────────────

#[test]
fn pdfa_20_pdfa_works_with_best_compression() {
    let bytes = DocumentBuilder::new("Comprimido")
        .pdfa()
        .compression(CompressionLevel::Best)
        .push(Paragraph::new("Texto."))
        .render_to_bytes()
        .expect("render failed");
    assert!(bytes.starts_with(b"%PDF-"));
    let raw = String::from_utf8_lossy(&bytes);
    assert!(raw.contains("pdfaid:part"));
}
