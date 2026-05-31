//! normordis-pdf v1.1.0 fidelity example — SectionedHeader, SectionedFooter,
//! exact RowHeight, runtime fields and diagonal Watermark.
//! Run: cargo run --example 05_fidelity -p normordis-pdf

use normordis_pdf::*;

fn main() -> Result<()> {
    // ── Document ──────────────────────────────────────────────────────────────

    let pdf = DocumentBuilder::new("Acta n.º 5/2026 — Reunião Ordinária")
        // SectionedHeader: dedicated first-page header, then alternating
        .sectioned_header(
            SectionedHeader::new()
                .first_page(InstitutionalHeader::new(
                    "Câmara Municipal de Exemplo",
                    "ACTA DE REUNIÃO — EXEMPLAR",
                ))
                .odd_pages(InstitutionalHeader::new(
                    "CME",
                    "Acta n.º 5/2026 | continuação",
                ))
                .even_pages(InstitutionalHeader::new(
                    "CME",
                    "Acta n.º 5/2026 | continuação",
                )),
        )
        // SectionedFooter with runtime fields
        .sectioned_footer(
            SectionedFooter::new()
                .first_page(
                    PageFooter::new()
                        .left("CONFIDENCIAL — uso interno")
                        .right("{{page}} / {{total_pages}}"),
                )
                .all_pages(
                    PageFooter::new()
                        .left("Câmara Municipal de Exemplo")
                        .center("{{today}}")
                        .right("{{page}} / {{total_pages}}"),
                ),
        )
        // Diagonal watermark
        .watermark(Watermark::new("RASCUNHO").opacity(0.10))
        // ── Body ──────────────────────────────────────────────────────────────
        .push(Spacer::new(4.0))
        .push(Section::new("1. Presentes", 1))
        .push(
            Paragraph::new(
                "Reuniu a Câmara Municipal de Exemplo em sessão ordinária, \
                 estando presentes os membros abaixo identificados.",
            )
            .align(TextAlign::Justify),
        )
        .push(Spacer::new(3.0))
        // Table with exact row heights (v1.1.0)
        .push(
            Table::new(
                vec!["Nome".into(), "Cargo".into(), "Presença".into()],
                vec![
                    TableRow::plain(vec![
                        "Ana Ferreira".into(),
                        "Presidente".into(),
                        "Presente".into(),
                    ])
                    .height_exact(8.0),
                    TableRow::plain(vec![
                        "Bruno Costa".into(),
                        "Vereador".into(),
                        "Presente".into(),
                    ])
                    .height_exact(8.0),
                    TableRow::plain(vec![
                        "Carla Mendes".into(),
                        "Vereadora".into(),
                        "Ausente (justificado)".into(),
                    ])
                    .height_at_least(8.0),
                ],
            )
            .col_widths(vec![50.0, 30.0, 20.0]),
        )
        .push(Spacer::new(4.0))
        .push(Section::new("2. Ordem de trabalhos", 1))
        .push(
            Paragraph::new(
                "Foram aprovados por unanimidade os seguintes pontos da ordem de trabalhos:",
            )
            .align(TextAlign::Justify),
        )
        .push(BulletList {
            items: vec![
                ListItemElement {
                    indent: 0,
                    runs: vec![TextRun::plain("Aprovação da acta da reunião anterior")],
                },
                ListItemElement {
                    indent: 0,
                    runs: vec![TextRun::plain(
                        "Apreciação do relatório de actividades do 1.º trimestre",
                    )],
                },
                ListItemElement {
                    indent: 0,
                    runs: vec![TextRun::plain(
                        "Deliberação sobre o protocolo de cedência de instalações",
                    )],
                },
            ],
        })
        .push(Spacer::new(4.0))
        .push(Section::new("3. Deliberações", 1))
        .push(
            Paragraph::new(
                "Foram aprovadas por unanimidade todas as deliberações constantes \
                 da ordem de trabalhos, sem votos contra nem abstenções.",
            )
            .align(TextAlign::Justify),
        )
        .render_to_bytes()?;

    let out = std::env::temp_dir().join("normaxis_fidelity_v110.pdf");
    std::fs::write(&out, &pdf)?;
    println!("PDF v1.1.0 gerado: {} ({} bytes)", out.display(), pdf.len());
    assert!(pdf.starts_with(b"%PDF"), "output must be a PDF file");
    assert!(pdf.len() > 10_000, "PDF must be at least 10 KB");
    Ok(())
}
