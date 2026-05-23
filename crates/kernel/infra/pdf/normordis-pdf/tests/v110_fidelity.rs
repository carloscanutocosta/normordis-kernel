use normordis_pdf::{
    DocumentBuilder, InstitutionalHeader, PageFooter, RowHeight, SectionedFooter, SectionedHeader,
    TableRow, Watermark,
};
use normordis_pdf::template::resolver::{resolve_runtime_fields, RuntimeContext};

// ── SectionedHeader resolution ────────────────────────────────────────────────

#[test]
fn sectioned_header_resolve_first_page_when_set() {
    let h = SectionedHeader::new()
        .first_page(InstitutionalHeader::new("Entidade", "Primeira"))
        .odd_pages(InstitutionalHeader::new("Entidade", "Ímpar"));
    assert_eq!(h.resolve(1).map(|h| h.document_title.as_str()), Some("Primeira"));
}

#[test]
fn sectioned_header_resolve_first_page_falls_back_to_odd() {
    let h = SectionedHeader::new()
        .odd_pages(InstitutionalHeader::new("Entidade", "Ímpar"));
    assert_eq!(h.resolve(1).map(|h| h.document_title.as_str()), Some("Ímpar"));
}

#[test]
fn sectioned_header_resolve_even_page_with_even_set() {
    let h = SectionedHeader::new()
        .odd_pages(InstitutionalHeader::new("E", "Ímpar"))
        .even_pages(InstitutionalHeader::new("E", "Par"));
    assert_eq!(h.resolve(2).map(|h| h.document_title.as_str()), Some("Par"));
}

#[test]
fn sectioned_header_resolve_even_page_falls_back_to_odd() {
    let h = SectionedHeader::new()
        .odd_pages(InstitutionalHeader::new("E", "Ímpar"));
    assert_eq!(h.resolve(2).map(|h| h.document_title.as_str()), Some("Ímpar"));
}

#[test]
fn sectioned_header_resolve_odd_page() {
    let h = SectionedHeader::new()
        .odd_pages(InstitutionalHeader::new("E", "Ímpar"))
        .even_pages(InstitutionalHeader::new("E", "Par"));
    assert_eq!(h.resolve(3).map(|h| h.document_title.as_str()), Some("Ímpar"));
}

// ── RowHeight ─────────────────────────────────────────────────────────────────

#[test]
fn row_height_exact_preserved() {
    let row = TableRow::new(vec![]).height_exact(12.0);
    assert!(matches!(row.height, RowHeight::Exact(h) if (h - 12.0).abs() < 0.001));
}

// ── RuntimeContext / resolve_runtime_fields ───────────────────────────────────

#[test]
fn resolve_page_and_total() {
    let ctx = RuntimeContext::new(2, 5);
    assert_eq!(resolve_runtime_fields("{{page}} / {{total_pages}}", &ctx), "2 / 5");
}

#[test]
fn resolve_today_is_non_empty() {
    let ctx = RuntimeContext::new(1, 1);
    let today = resolve_runtime_fields("{{today}}", &ctx);
    assert!(!today.is_empty(), "{{today}} should not be empty");
    assert!(!today.contains("{{"), "{{today}} should be resolved");
}

#[test]
fn resolve_text_without_fields_unchanged() {
    let ctx = RuntimeContext::new(1, 1);
    let text = "Este texto não tem campos calculados.";
    assert_eq!(resolve_runtime_fields(text, &ctx), text);
}

// ── Watermark ─────────────────────────────────────────────────────────────────

#[test]
fn watermark_default_values() {
    let wm = Watermark::default();
    assert_eq!(wm.text, "RASCUNHO");
    assert!((wm.opacity - 0.10).abs() < 0.001);
}

#[test]
fn document_with_watermark_renders_without_panic() {
    let pdf = DocumentBuilder::new("Rascunho")
        .watermark(Watermark::new("RASCUNHO").opacity(0.10))
        .render_to_bytes()
        .expect("should render");
    assert!(pdf.starts_with(b"%PDF"));
    assert!(pdf.len() > 1_000);
}

// ── Footer with {{total_pages}} ───────────────────────────────────────────────

#[test]
fn document_with_total_pages_in_footer_renders_without_panic() {
    let pdf = DocumentBuilder::new("Paginado")
        .footer(
            PageFooter::new()
                .right("{{page}} / {{total_pages}}"),
        )
        .render_to_bytes()
        .expect("should render");
    assert!(pdf.starts_with(b"%PDF"));
    assert!(pdf.len() > 1_000);
}

// ── SectionedFooter ───────────────────────────────────────────────────────────

#[test]
fn document_with_sectioned_footer_renders_without_panic() {
    let pdf = DocumentBuilder::new("Seccionado")
        .sectioned_footer(
            SectionedFooter::new().all_pages(
                PageFooter::new()
                    .left("REF/2026/001")
                    .right("{{page}} / {{total_pages}}"),
            ),
        )
        .render_to_bytes()
        .expect("should render");
    assert!(pdf.starts_with(b"%PDF"));
    assert!(pdf.len() > 1_000);
}
