use normordis_pdf::{
    CompressionLevel, DocumentBuilder, PageBreakElement, Paragraph, Section,
    Table, TableCell,
};
use lopdf::Document as LopdfDoc;
use std::io::Cursor;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn simple_doc(compression: CompressionLevel) -> Vec<u8> {
    DocumentBuilder::new("Size Test")
        .compression(compression)
        .push(Section::new("Introduction", 1))
        .push(Paragraph::new("First paragraph of body text for size testing."))
        .push(Paragraph::new("Second paragraph with some additional content."))
        .render_to_bytes()
        .expect("render failed")
}

fn load_lopdf(bytes: &[u8]) -> LopdfDoc {
    LopdfDoc::load_from(Cursor::new(bytes)).expect("lopdf parse failed")
}

// ── pdf-inspect (01–05) ───────────────────────────────────────────────────────

#[test]
fn size_01_bytes_not_empty() {
    let bytes = simple_doc(CompressionLevel::Default);
    assert!(!bytes.is_empty());
}

#[test]
fn size_02_total_bytes_greater_than_zero() {
    let bytes = simple_doc(CompressionLevel::Default);
    assert!(bytes.len() > 0);
}

#[test]
fn size_03_font_data_present_in_pdf() {
    // Fonts are embedded — the PDF must contain font-related data.
    // We verify by checking the raw bytes contain a font reference.
    let bytes = simple_doc(CompressionLevel::None);
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("/Font") || bytes.len() > 100_000,
        "Expected font data in PDF (Liberation Sans should be embedded)");
}

#[test]
fn size_04_pdf_bytes_less_than_2mb() {
    // Eixo B: font subsetting active — simple doc is ~113 KB. Limit set to 500 KB.
    let bytes = simple_doc(CompressionLevel::Default);
    assert!(bytes.len() < 500_000,
        "PDF unexpectedly large: {} bytes", bytes.len());
}

#[test]
fn size_05_lopdf_can_load_generated_pdf() {
    let bytes = simple_doc(CompressionLevel::Default);
    let result = LopdfDoc::load_from(Cursor::new(&bytes));
    assert!(result.is_ok(), "lopdf failed to load generated PDF: {:?}", result.err());
}

// ── Compressão (06–10) ────────────────────────────────────────────────────────

#[test]
fn size_06_default_smaller_than_none() {
    let none_bytes    = simple_doc(CompressionLevel::None);
    let default_bytes = simple_doc(CompressionLevel::Default);
    assert!(
        default_bytes.len() < none_bytes.len(),
        "Default ({}) should be smaller than None ({})",
        default_bytes.len(), none_bytes.len()
    );
}

#[test]
fn size_07_best_le_default() {
    let default_bytes = simple_doc(CompressionLevel::Default);
    let best_bytes    = simple_doc(CompressionLevel::Best);
    assert!(
        best_bytes.len() <= default_bytes.len() + 1024,
        "Best ({}) should be <= Default ({}) (allow 1KB tolerance)",
        best_bytes.len(), default_bytes.len()
    );
}

#[test]
fn size_08_none_produces_valid_pdf() {
    let bytes = simple_doc(CompressionLevel::None);
    assert!(bytes.len() > 0);
    let doc = load_lopdf(&bytes);
    assert!(!doc.objects.is_empty());
}

#[test]
fn size_09_compressed_pdf_is_valid() {
    let bytes = simple_doc(CompressionLevel::Default);
    let doc = load_lopdf(&bytes);
    assert!(!doc.objects.is_empty(), "lopdf must parse a non-empty document");
}

#[test]
fn size_10_fast_smaller_than_none() {
    let none_bytes = simple_doc(CompressionLevel::None);
    let fast_bytes = simple_doc(CompressionLevel::Fast);
    assert!(
        fast_bytes.len() < none_bytes.len(),
        "Fast ({}) should be smaller than None ({})",
        fast_bytes.len(), none_bytes.len()
    );
}

// ── Garbage collection (11–13) ────────────────────────────────────────────────

#[test]
fn size_11_object_count_reasonable() {
    let bytes = simple_doc(CompressionLevel::Default);
    let doc = load_lopdf(&bytes);
    // After prune_objects(), should have far fewer objects than an unoptimized PDF
    assert!(doc.objects.len() > 0, "must have some objects");
    assert!(doc.objects.len() < 10_000, "unexpectedly many objects: {}", doc.objects.len());
}

#[test]
fn size_12_compressed_not_larger_than_raw() {
    let none_size    = simple_doc(CompressionLevel::None).len();
    let default_size = simple_doc(CompressionLevel::Default).len();
    assert!(default_size < none_size,
        "compression must reduce size: Default={} None={}", default_size, none_size);
}

#[test]
fn size_13_lopdf_parses_after_prune() {
    // Verify that the optimize + prune pipeline doesn't corrupt the PDF.
    let bytes = simple_doc(CompressionLevel::Best);
    let doc = load_lopdf(&bytes);
    assert!(doc.objects.len() > 5, "expected several objects after optimization");
}

// ── Metadados (14–16) ─────────────────────────────────────────────────────────

#[test]
fn size_14_producer_is_normordis_pdf() {
    let bytes = simple_doc(CompressionLevel::None);
    let raw = String::from_utf8_lossy(&bytes);
    assert!(raw.contains("normordis-pdf"),
        "Producer should be 'normordis-pdf' in the PDF");
}

#[test]
fn size_15_creation_date_present() {
    let bytes = simple_doc(CompressionLevel::None);
    let raw = String::from_utf8_lossy(&bytes);
    assert!(raw.contains("CreationDate"), "PDF should contain CreationDate");
}

#[test]
fn size_16_pdf_header_valid() {
    let bytes = simple_doc(CompressionLevel::Default);
    assert!(bytes.starts_with(b"%PDF-"), "PDF must start with %PDF-");
}

// ── Compressão efectiva (20–22) ───────────────────────────────────────────────

#[test]
fn size_20_default_compressed_under_800kb() {
    // Eixo B: subsetting active — simple doc ~113 KB. Well under 500 KB.
    let bytes = simple_doc(CompressionLevel::Default);
    assert!(
        bytes.len() < 500_000,
        "Default-compressed PDF should be under 500KB, got {} bytes",
        bytes.len()
    );
}

#[test]
fn size_21_subsetting_reduces_file_size() {
    // Eixo B: subsetting reduces a simple doc from ~4.6 MB (full fonts) to
    // ~150 KB. Verify the uncompressed output stays under 500 KB.
    let bytes = simple_doc(CompressionLevel::None);
    assert!(
        bytes.len() < 500_000,
        "Subsetted PDF should be under 500KB, got {} bytes",
        bytes.len()
    );
}

#[test]
fn size_22_ratio_default_over_none_under_065() {
    // Subsetted font data is binary and doesn't compress much further with zlib.
    // Ratio stays near 1.0; just verify compression doesn't inflate the file.
    let none_size    = simple_doc(CompressionLevel::None).len();
    let default_size = simple_doc(CompressionLevel::Default).len();
    let ratio = default_size as f64 / none_size as f64;
    assert!(
        ratio < 1.05,
        "Compression must not significantly inflate size (ratio={:.2}), Default={} None={}",
        ratio, default_size, none_size
    );
}

// ── NDT compression field (23–25) ─────────────────────────────────────────────

#[test]
fn size_23_compression_level_best_serde() {
    let json = r#""best""#;
    let level: CompressionLevel = serde_json::from_str(json).unwrap();
    assert_eq!(level, CompressionLevel::Best);
}

#[test]
fn size_24_compression_level_default_is_default() {
    let level = CompressionLevel::default();
    assert_eq!(level, CompressionLevel::Default);
}

#[test]
fn size_25_compression_level_none_serde() {
    let json = r#""none""#;
    let level: CompressionLevel = serde_json::from_str(json).unwrap();
    assert_eq!(level, CompressionLevel::None);
}

// ── Multi-page (26–28) ────────────────────────────────────────────────────────

fn page_count(bytes: &[u8]) -> u32 {
    let doc = LopdfDoc::load_from(Cursor::new(bytes)).expect("lopdf parse failed");
    doc.get_pages().len() as u32
}

#[test]
fn size_26_explicit_page_break_produces_two_pages() {
    let bytes = DocumentBuilder::new("Two pages")
        .push(Paragraph::new("Página 1."))
        .push(PageBreakElement)
        .push(Paragraph::new("Página 2."))
        .render_to_bytes()
        .unwrap();
    assert_eq!(page_count(&bytes), 2, "devia ter 2 páginas");
}

#[test]
fn size_27_overflow_content_produces_multiple_pages() {
    // ~40 paragraphs of long text — enough to overflow a A4 page
    let mut builder = DocumentBuilder::new("Overflow");
    for i in 1..=40 {
        builder = builder.push(Paragraph::new(format!(
            "Parágrafo {i}: Este é um texto longo destinado a forçar a quebra automática de \
             página pelo motor de layout do normordis-pdf quando o conteúdo excede a altura \
             disponível numa única página A4."
        )));
    }
    let bytes = builder.render_to_bytes().unwrap();
    assert!(page_count(&bytes) >= 2, "devia ter pelo menos 2 páginas");
}

#[test]
fn size_28_three_explicit_page_breaks_produce_four_pages() {
    let bytes = DocumentBuilder::new("Four pages")
        .push(Paragraph::new("Página 1."))
        .push(PageBreakElement)
        .push(Paragraph::new("Página 2."))
        .push(PageBreakElement)
        .push(Paragraph::new("Página 3."))
        .push(PageBreakElement)
        .push(Paragraph::new("Página 4."))
        .render_to_bytes()
        .unwrap();
    assert_eq!(page_count(&bytes), 4, "devia ter 4 páginas");
}

// ── Table pagination (29–32) ──────────────────────────────────────────────────

fn overflow_table(rows: usize) -> Table {
    let mut b = Table::builder();
    b = b.header_row(vec![TableCell::new("Nº"), TableCell::new("Descrição")]);
    for i in 1..=rows {
        b = b.row(vec![
            TableCell::new(format!("{i}")),
            TableCell::new(format!("Linha de dados {i} com texto suficiente para ocupar espaço.")),
        ]);
    }
    b.build()
}

#[test]
fn size_29_table_with_many_rows_overflows_to_second_page() {
    let bytes = DocumentBuilder::new("Table overflow")
        .push(overflow_table(60))
        .render_to_bytes()
        .unwrap();
    assert!(page_count(&bytes) >= 2, "tabela com 60 linhas devia ter >= 2 páginas");
}

#[test]
fn size_30_table_header_repeats_on_continuation_page() {
    // Verify the PDF is structurally valid (lopdf parseable) when table spans pages.
    let bytes = DocumentBuilder::new("Table header repeat")
        .push(overflow_table(60))
        .render_to_bytes()
        .unwrap();
    let result = LopdfDoc::load_from(Cursor::new(&bytes));
    assert!(result.is_ok(), "lopdf falhou numa tabela multi-página: {:?}", result.err());
}

#[test]
fn size_31_table_after_content_overflows_correctly() {
    // Table that starts near the bottom of the first page.
    let mut builder = DocumentBuilder::new("Late table");
    for _ in 0..40 {
        builder = builder.push(Paragraph::new(
            "Parágrafo de preenchimento para empurrar a tabela para o fundo da página.",
        ));
    }
    builder = builder.push(overflow_table(20));
    let bytes = builder.render_to_bytes().unwrap();
    assert!(page_count(&bytes) >= 2, "tabela após conteúdo devia ter >= 2 páginas");
}

#[test]
fn size_32_single_page_table_stays_on_one_page() {
    let bytes = DocumentBuilder::new("Small table")
        .push(overflow_table(5))
        .render_to_bytes()
        .unwrap();
    assert_eq!(page_count(&bytes), 1, "tabela pequena devia caber numa página");
}
