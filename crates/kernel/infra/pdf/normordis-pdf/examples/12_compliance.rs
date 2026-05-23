//! Demonstrates v2.0.0: PDF/A-1b, font subsetting, opacity, and traceability.
//! Run: cargo run --example 12_compliance -p normordis-pdf

use normordis_pdf::{
    BulletList, CompressionLevel, DocumentBuilder, InstitutionalHeader,
    ListItemElement, PageFooter, Paragraph, PdfStandard, RgbColor, Result, Section,
    SecurityClassification, Spacer, TraceabilityMetadata, Watermark, NDT_VERSION,
    PDF_BACKEND, VERSION,
};

fn main() -> Result<()> {
    let out_dir = std::env::temp_dir();

    // ── PDF/A-1b com traceabilidade ───────────────────────────────────
    let pdf_a = DocumentBuilder::new("Acta n.º 1/2026")
        .standard(PdfStandard::PdfA1b)
        .compression(CompressionLevel::Best)
        .traceability(TraceabilityMetadata {
            engine_version: VERSION.into(),
            entity_id: "cm-lisboa".into(),
            document_ref: Some("ACT/2026/001".into()),
            classification: SecurityClassification::Internal,
            generated_at: "2026-01-01T00:00:00Z".into(),
            ndt_version: NDT_VERSION.into(),
            framework_version: None,
        })
        .header(
            InstitutionalHeader::new("Câmara Municipal de Lisboa", "Acta n.º 1/2026")
                .with_reference("ACT/2026/001")
                .with_date("29 de Abril de 2026"),
        )
        .footer(PageFooter::new()
            .left("ACT/2026/001 — INTERNO")
            .right("Pág. {{page}} / {{total_pages}}"))
        .push(Section::new("1. Abertura da Sessão", 1))
        .push(Paragraph::new(
            "Reuniu a Câmara Municipal de Lisboa, em sessão ordinária, \
             na sala de reuniões dos Paços do Concelho, pelas 10h00."
        ))
        .push(Spacer::new(4.0))
        .push(Section::new("2. Ordem do Dia", 1))
        .push(BulletList::new(vec![
            ListItemElement::plain("Aprovação da acta anterior"),
            ListItemElement::plain("Ponto 1: Aprovação do Orçamento Municipal 2027"),
            ListItemElement::plain("Ponto 2: Deliberações diversas"),
        ]))
        .render_to_bytes()?;

    let path_a = out_dir.join("normordis_pdfa.pdf");
    std::fs::write(&path_a, &pdf_a)?;
    println!("PDF/A-1b:       {} ({} KB)", path_a.display(), pdf_a.len() / 1024);

    // ── Opacidade real na marca de água ───────────────────────────────
    let pdf_opacity = DocumentBuilder::new("Rascunho")
        .watermark(
            Watermark::new("RASCUNHO")
                .opacity(0.15)
                .color(RgbColor { r: 0.8, g: 0.0, b: 0.0 })
                .font_size(80.0),
        )
        .push(Paragraph::new("Este documento usa opacidade real via ExtGState."))
        .push(Paragraph::new(
            "A marca de água RASCUNHO é renderizada com alfa 0.15 sem \
             simulação de cor — funciona correctamente sobre qualquer fundo."
        ))
        .render_to_bytes()?;

    let path_o = out_dir.join("normaxis_opacity.pdf");
    std::fs::write(&path_o, &pdf_opacity)?;
    println!("Opacidade real: {} ({} KB)", path_o.display(), pdf_opacity.len() / 1024);

    // ── Classificação automática via NDT 2.0.0 output block ──────────
    let ndt_template = r#"{
        "ndt": "2.0.0",
        "meta": { "title": "Ofício Confidencial" },
        "output": {
            "standard": "pdf_a_1b",
            "compression": "best",
            "classification": "confidencial",
            "document_ref": "OFC/2026/007"
        },
        "body": [
            { "type": "paragraph", "text": "Este ofício é {{classificacao}}." }
        ]
    }"#;
    let ndt_data = r#"{"ndt_data":"1.0.0","data":{"classificacao":"confidencial"}}"#;

    let pdf_ndt = DocumentBuilder::new("Ofício Confidencial")
        .push_ndt(ndt_template, ndt_data)?
        .render_to_bytes()?;

    let path_n = out_dir.join("normaxis_ndt200.pdf");
    std::fs::write(&path_n, &pdf_ndt)?;
    println!("NDT 2.0.0:      {} ({} KB)", path_n.display(), pdf_ndt.len() / 1024);

    println!("\nChecklist visual:");
    println!("  □ PDF/A: verificar com veraPDF — zero erros");
    println!("  □ Marca de água INTERNO em azul translúcido (classif. auto)");
    println!("  □ Marca de água RASCUNHO com opacidade real");
    println!("  □ NDT 2.0.0: marca de água CONFIDENCIAL automática");
    println!("  □ PDF backend: {}", PDF_BACKEND);

    Ok(())
}
