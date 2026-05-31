use normordis_pdf::{DocumentBuilder, InstitutionalHeader, PageFooter, Paragraph, Section, Spacer};

#[test]
fn render_empty_document_produces_valid_pdf() {
    let bytes = DocumentBuilder::new("Test Document")
        .render_to_bytes()
        .expect("render_to_bytes should succeed for an empty document");

    assert!(!bytes.is_empty(), "PDF bytes must not be empty");
    assert!(
        bytes.starts_with(b"%PDF-"),
        "output must start with PDF magic bytes"
    );
}

#[test]
fn render_document_with_elements() {
    let bytes = DocumentBuilder::new("Document with Elements")
        .header(InstitutionalHeader::new(
            "Entidade de Teste",
            "Título do Documento",
        ))
        .footer(PageFooter::with_page_numbers())
        .push(Section::new("1. Introdução", 1))
        .push(Paragraph::new("Este é um parágrafo de teste."))
        .push(Spacer::new(10.0))
        .push(Section::new("2. Conclusão", 1))
        .push(Paragraph::new("Fim do documento."))
        .render_to_bytes()
        .expect("render_to_bytes should succeed");

    assert!(!bytes.is_empty());
    assert!(bytes.starts_with(b"%PDF-"));
}
