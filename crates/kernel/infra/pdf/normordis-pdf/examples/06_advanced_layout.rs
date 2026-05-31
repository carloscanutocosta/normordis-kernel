//! normordis-pdf v1.2.0 — advanced layout features.
//! Run: cargo run --example 06_advanced_layout -p normordis-pdf

use normordis_pdf::*;

fn main() -> Result<()> {
    let pdf = DocumentBuilder::new("Layout Avançado v1.2.0")
        .header(
            InstitutionalHeader::new("Câmara Municipal de Exemplo", "Demonstração v1.2.0")
                .with_reference("DEMO/2026/001"),
        )
        .footer(PageFooter::new().right("{{page}} / {{total_pages}}"))
        // ── Indentação ────────────────────────────────────────────────────────
        .push(Section::new("1. Indentação de parágrafo", 1))
        .push(
            Paragraph::new(
                "Este parágrafo tem indentação esquerda de 10mm. \
                 O texto corre dentro das margens ajustadas — útil para \
                 citações, listas e blocos de conteúdo com hierarquia visual.",
            )
            .align(TextAlign::Justify)
            .indent_left(10.0),
        )
        .push(Spacer::new(4.0))
        .push(
            Paragraph::new(
                "Parágrafo com indentação esquerda de 10mm e primeira linha com \
                 mais 5mm adicionais — padrão comum em documentos formais portugueses \
                 para iniciar novos parágrafos após ponto final.",
            )
            .align(TextAlign::Justify)
            .indent_left(10.0)
            .indent_first_line(5.0),
        )
        .push(Spacer::new(4.0))
        .push(
            Paragraph::new(
                "Hanging indent: a primeira linha começa 5mm à esquerda das \
                 linhas seguintes — usado em bibliografias e listas de referências.",
            )
            .align(TextAlign::Justify)
            .indent_left(15.0)
            .indent_first_line(-10.0),
        )
        // ── TextAlign::Right ──────────────────────────────────────────────────
        .push(Spacer::new(6.0))
        .push(Section::new("2. Alinhamento à direita", 1))
        .push(Paragraph::new("Lisboa, 25 de Abril de 2026").align(TextAlign::Right))
        .push(Paragraph::new("Referência: DEMO/2026/001").align(TextAlign::Right))
        // ── Tabela com col_span ───────────────────────────────────────────────
        .push(Spacer::new(6.0))
        .push(Section::new(
            "3. Tabela com células mescladas (col_span)",
            1,
        ))
        .push(
            Table::builder()
                .col_widths(vec![25.0, 25.0, 25.0, 25.0])
                .header_row(vec![
                    TableCell::new("Identificação").col_span(2),
                    TableCell::new("Contacto").col_span(2),
                ])
                .header_row(vec![
                    TableCell::new("Nome"),
                    TableCell::new("NIF"),
                    TableCell::new("Telefone"),
                    TableCell::new("Email"),
                ])
                .row(vec![
                    TableCell::new("João Silva"),
                    TableCell::new("123 456 789"),
                    TableCell::new("912 345 678"),
                    TableCell::new("joao@example.pt"),
                ])
                .row(vec![
                    TableCell::new("Maria Santos"),
                    TableCell::new("987 654 321"),
                    TableCell::new("913 456 789"),
                    TableCell::new("maria@example.pt"),
                ])
                .stripe()
                .build(),
        )
        // ── Paginação automática de tabela ───────────────────────────────────
        .push(Spacer::new(6.0))
        .push(Section::new(
            "4. Tabela com paginação automática (40 linhas)",
            1,
        ))
        .push(
            Table::new(
                vec!["#".into(), "Descrição".into(), "Valor (€)".into()],
                (1..=40)
                    .map(|i| {
                        TableRow::plain(vec![
                            i.to_string(),
                            format!("Item de demonstração número {i}"),
                            format!("{:.2}", i as f64 * 12.5),
                        ])
                    })
                    .collect(),
            )
            .stripe(),
        )
        // ── Paginação de lista ────────────────────────────────────────────────
        .push(Spacer::new(6.0))
        .push(Section::new(
            "5. Lista com paginação automática (30 itens)",
            1,
        ))
        .push(BulletList::new(
            (1..=30)
                .map(|i| {
                    ListItemElement::plain(format!(
                        "Item de lista paginado número {i} — conteúdo de demonstração"
                    ))
                })
                .collect(),
        ))
        .render_to_bytes()?;

    let out_path = std::env::temp_dir().join("normaxis_advanced.pdf");
    std::fs::write(&out_path, &pdf)?;
    println!("PDF gerado: {} ({} bytes)", out_path.display(), pdf.len());

    Ok(())
}
