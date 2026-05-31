//! Basic document with flow elements.
//! Run: cargo run --example 01_basic_document -p normordis-pdf

use normordis_pdf::*;

fn main() -> Result<()> {
    let pdf = DocumentBuilder::new("Basic Document")
        .header(
            InstitutionalHeader::new("Câmara Municipal de Exemplo", "Relatório de Teste")
                .with_reference("REF/2026/001")
                .with_date("25 de Abril de 2026"),
        )
        .footer(PageFooter::with_page_numbers())
        .push(Section::new("1. Introdução", 1))
        .push(
            Paragraph::new(
                "Este é um documento de exemplo gerado pelo normordis-pdf v1.0.0. \
                 Demonstra o modo de layout Flow com quebra automática de linhas e páginas.",
            )
            .align(TextAlign::Justify),
        )
        .push(Spacer::new(5.0))
        .push(Section::new("2. Tabela de Dados", 1))
        .push(Table::new(
            vec!["Campo".into(), "Valor".into()],
            vec![
                TableRow::plain(vec![
                    "Entidade".into(),
                    "Câmara Municipal de Exemplo".into(),
                ]),
                TableRow::plain(vec!["Data".into(), "25 de Abril de 2026".into()]),
                TableRow::plain(vec!["Referência".into(), "REF/2026/001".into()]),
            ],
        ))
        .render_to_bytes()?;

    let out = std::env::temp_dir().join("normaxis_basic.pdf");
    std::fs::write(&out, &pdf)?;
    println!("PDF gerado: {} ({} bytes)", out.display(), pdf.len());
    Ok(())
}
