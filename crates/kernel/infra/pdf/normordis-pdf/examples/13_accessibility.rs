//! Demonstrates v2.1.0 PDF/UA-2 accessibility (ISO 14289-2:2024).
//! Run: cargo run --example 13_accessibility -p normordis-pdf

use normordis_pdf::{
    AccessibilityConfig, BulletList, DocumentBuilder, FixedBox, ImageElement, InstitutionalHeader,
    ListItemElement, OrderedList, PageFooter, Paragraph, PdfStandard, Result, Section, Spacer,
    StructTag, Table, TableCell, TextAlign,
};

fn main() -> Result<()> {
    let out_dir = std::env::temp_dir();

    // ── PDF/UA-2 completo ─────────────────────────────────────────────
    let pdf = DocumentBuilder::new("Relatório de Actividades — Acessível")
        .accessibility(AccessibilityConfig {
            enabled: true,
            lang: "pt-PT".into(),
            warn_missing_alt: true,
            fixed_box_default_artifact: true,
        })
        .standard(PdfStandard::PdfUa2)
        .header(
            InstitutionalHeader::new("Câmara Municipal de Lisboa", "Relatório Anual 2026")
                .with_reference("REL/2026/001")
                .with_date("1 de Janeiro de 2026"),
        )
        .footer(PageFooter::new().right("{{page}} / {{total_pages}}"))
        .push(Section::new("1. Introdução", 1))
        .push(Paragraph::new(
            "Este relatório apresenta as actividades desenvolvidas durante 2026. \
             Destina-se a leitores internos e ao público em geral.",
        ))
        .push(Spacer::new(4.0))
        .push(Section::new("2. Resultados", 1))
        .push(Section::new("2.1 Urbanismo", 2))
        .push(Paragraph::new(
            "A divisão de urbanismo emitiu 84 licenças em 2026.",
        ))
        .push(
            ImageElement::new(placeholder_image())
                .width_mm(60.0)
                .alt("Gráfico de barras: licenças emitidas por trimestre em 2026"),
        )
        .push(Paragraph::new(
            "A legenda do gráfico está disponível na tabela seguinte.",
        ))
        .push(
            Table::builder()
                .header_row(vec![
                    TableCell::new("Trimestre"),
                    TableCell::new("Licenças"),
                ])
                .row(vec![TableCell::new("T1"), TableCell::new("18")])
                .row(vec![TableCell::new("T2"), TableCell::new("24")])
                .row(vec![TableCell::new("T3"), TableCell::new("22")])
                .row(vec![TableCell::new("T4"), TableCell::new("20")])
                .build(),
        )
        .push(Spacer::new(4.0))
        .push(Section::new("2.2 Obras Públicas", 2))
        .push(BulletList::new(vec![
            ListItemElement::plain("Requalificação da Avenida da Liberdade"),
            ListItemElement::plain("Pavimentação da Rua do Ouro"),
            ListItemElement::plain("Reabilitação da Praça do Comércio"),
        ]))
        .push(Spacer::new(4.0))
        .push(Section::new("3. Conclusão", 1))
        .push(Paragraph::new(
            "Os resultados de 2026 foram positivos em todas as áreas de actuação.",
        ))
        .push(OrderedList::new(vec![
            ListItemElement::plain("Objectivo de urbanismo alcançado (105%)"),
            ListItemElement::plain("Obras públicas dentro do orçamento"),
        ]))
        // FixedBox com role — não tratada como Artifact
        .fixed_text(
            FixedBox {
                x_mm: 150.0,
                y_mm: 20.0,
                width_mm: 50.0,
                height_mm: 10.0,
                ua_role: Some(StructTag::Caption),
                ua_alt: None,
                ..Default::default()
            },
            "Documento confidencial",
            TextAlign::Right,
        )
        .render_to_bytes()?;

    let out_path = out_dir.join("normaxis_accessible.pdf");
    std::fs::write(&out_path, &pdf)?;

    println!("PDF/UA-2: {} ({} KB)", out_path.display(), pdf.len() / 1024);
    println!("  Structure tree: Document > H1/H2/P/Table/Figure/L/LI");
    println!("  Watermark e header/footer: Artifact (nao lidos por screen readers)");
    println!("  Verificar com PAC 2024 ou veraPDF --flavour ua2");

    Ok(())
}

fn placeholder_image() -> Vec<u8> {
    // Minimal 1×1 white PNG for illustration
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77,
        0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, // IDAT chunk
        0x54, 0x08, 0xD7, 0x63, 0xF8, 0xFF, 0xFF, 0x3F, 0x00, 0x05, 0xFE, 0x02, 0xFE, 0xDC, 0xCC,
        0x59, 0xE7, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, // IEND chunk
        0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
}
