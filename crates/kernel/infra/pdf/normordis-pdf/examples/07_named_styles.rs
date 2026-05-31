// Example 07 — Named Styles, Tab Stops, and CellPadding
//
// Demonstrates:
//   - Built-in named styles applied to Paragraph and Section
//   - User-defined named style with inheritance
//   - space_before_mm / space_after_mm per paragraph
//   - Tab stops (left, right) with leader characters
//   - TableStyle presets (grid, bordered, striped)
//   - CellPadding per cell

use std::collections::HashMap;

use normordis_pdf::{
    BulletList, CellPadding, DocumentBuilder, DocumentStyle, ListItemElement, NamedStyle,
    Paragraph, RgbColor, Section, Spacer, TabStop, Table, TableCell, TableRow, TableStyle,
    TextAlign, TextRun,
};

fn main() {
    let pdf_bytes = build_pdf().expect("PDF generation failed");
    let path = "examples/output/07_named_styles.pdf";
    std::fs::create_dir_all("examples/output").unwrap();
    std::fs::write(path, &pdf_bytes).unwrap();
    println!("Written {} bytes to {}", pdf_bytes.len(), path);
}

fn build_pdf() -> normordis_pdf::Result<Vec<u8>> {
    // ── User-defined styles ───────────────────────────────────────────────────
    let mut user_styles = HashMap::new();

    // "intro" — extends "normal", italic, extra spacing
    user_styles.insert(
        "intro".into(),
        NamedStyle {
            extends: Some("normal".into()),
            font_size: Some(12.0),
            italic: Some(true),
            space_before_mm: Some(4.0),
            space_after_mm: Some(6.0),
            ..Default::default()
        },
    );

    // "sidebar" — extends "normal", smaller, right-aligned, institutional blue
    user_styles.insert(
        "sidebar".into(),
        NamedStyle {
            extends: Some("normal".into()),
            font_size: Some(9.5),
            alignment: Some(TextAlign::Right),
            color: Some(RgbColor::new(0.0, 0.2, 0.6)),
            space_before_mm: Some(2.0),
            space_after_mm: Some(2.0),
            ..Default::default()
        },
    );

    let doc_style = DocumentStyle {
        margin_top_mm: 20.0,
        margin_bottom_mm: 20.0,
        margin_left_mm: 25.0,
        margin_right_mm: 20.0,
        named_styles: user_styles,
        ..DocumentStyle::default()
    };

    // ── Paragraphs with named styles ──────────────────────────────────────────
    let intro_p = Paragraph::new(
        "Este documento demonstra o sistema de estilos nomeados do normordis-pdf v1.3.0, \
         equivalente aos Paragraph Styles do Microsoft Word.",
    )
    .style("intro");

    let sidebar_p = Paragraph::new("Nota: os estilos são resolvidos em tempo de renderização.")
        .style("sidebar");

    let caption_p =
        Paragraph::new("Quadro 1 — Resultado dos testes de estilo v1.3.0.").style("caption");

    // Explicit spacing (not via style)
    let spaced_p = Paragraph::new(
        "Espaçamento explícito: 8 mm antes, 6 mm depois (definido na instância, não via estilo).",
    )
    .space_before(8.0)
    .space_after(6.0);

    // ── Tab stops — right-aligned page numbers with dot leader ────────────────
    let tab1 = Paragraph::from_runs(
        vec![TextRun::plain("Constituição da República\t1976")],
        TextAlign::Left,
        Some(10.5),
    )
    .tab_stop(TabStop::right(140.0).with_leader('.'));

    let tab2 = Paragraph::from_runs(
        vec![TextRun::plain("Código Civil\t1966")],
        TextAlign::Left,
        Some(10.5),
    )
    .tab_stop(TabStop::right(140.0).with_leader('.'));

    let tab3 = Paragraph::from_runs(
        vec![TextRun::plain("Código do Processo Civil\t1961")],
        TextAlign::Left,
        Some(10.5),
    )
    .tab_stop(TabStop::right(140.0).with_leader('.'));

    let tab4 = Paragraph::from_runs(
        vec![TextRun::plain("Código Penal\t1982")],
        TextAlign::Left,
        Some(10.5),
    )
    .tab_stop(TabStop::right(140.0).with_leader('.'));

    // ── Bullet list ───────────────────────────────────────────────────────────
    let features = BulletList::new(vec![
        ListItemElement::plain("NamedStyle — herança via extends, propriedades opcionais"),
        ListItemElement::plain(
            "StyleResolver — resolução em tempo de renderização, deteção de ciclos",
        ),
        ListItemElement::plain("TabStop — left / right / center / decimal com leader char"),
        ListItemElement::plain("CellPadding — insets por célula (top/bottom/left/right)"),
        ListItemElement::plain("TableStyle — grid / bordered / striped / plain"),
    ]);

    // ── Grid table ────────────────────────────────────────────────────────────
    let grid_table = Table::new(
        vec!["Estilo".into(), "Descrição".into(), "Desde".into()],
        vec![
            TableRow::plain(vec![
                "heading_1".into(),
                "Título principal".into(),
                "v1.3.0".into(),
            ]),
            TableRow::plain(vec![
                "heading_2".into(),
                "Subtítulo".into(),
                "v1.3.0".into(),
            ]),
            TableRow::plain(vec![
                "caption".into(),
                "Legenda de figura/tabela".into(),
                "v1.3.0".into(),
            ]),
            TableRow::plain(vec![
                "normal".into(),
                "Corpo de texto".into(),
                "v1.3.0".into(),
            ]),
            TableRow::plain(vec![
                "table_header".into(),
                "Cabeçalho de tabela".into(),
                "v1.3.0".into(),
            ]),
            TableRow::plain(vec![
                "table_body".into(),
                "Célula de tabela".into(),
                "v1.3.0".into(),
            ]),
        ],
    )
    .with_table_style(TableStyle::grid())
    .col_widths(vec![30.0, 50.0, 20.0]);

    // ── Bordered table with generous CellPadding ──────────────────────────────
    let bordered_table = Table::builder()
        .header_row(vec![
            TableCell::new("Propriedade").padding(CellPadding::horizontal_vertical(4.0, 3.0)),
            TableCell::new("Tipo").padding(CellPadding::horizontal_vertical(4.0, 3.0)),
            TableCell::new("Descrição").padding(CellPadding::horizontal_vertical(4.0, 3.0)),
        ])
        .row(vec![
            TableCell::new("space_before_mm").padding(CellPadding::uniform(3.0)),
            TableCell::new("Option<f64>").padding(CellPadding::uniform(3.0)),
            TableCell::new("Espaço antes do parágrafo").padding(CellPadding::uniform(3.0)),
        ])
        .row(vec![
            TableCell::new("space_after_mm").padding(CellPadding::uniform(3.0)),
            TableCell::new("Option<f64>").padding(CellPadding::uniform(3.0)),
            TableCell::new("Espaço depois do parágrafo").padding(CellPadding::uniform(3.0)),
        ])
        .row(vec![
            TableCell::new("tab_stops").padding(CellPadding::uniform(3.0)),
            TableCell::new("Vec<TabStop>").padding(CellPadding::uniform(3.0)),
            TableCell::new("Tab stops deste parágrafo").padding(CellPadding::uniform(3.0)),
        ])
        .col_widths(vec![35.0, 25.0, 40.0])
        .build()
        .with_table_style(TableStyle::bordered());

    // ── Striped table ─────────────────────────────────────────────────────────
    let striped_table = Table::new(
        vec!["#".into(), "Conteúdo".into()],
        (1..=6_u32)
            .map(|i| {
                TableRow::plain(vec![
                    format!("{}", i),
                    format!("Linha de exemplo número {}", i),
                ])
            })
            .collect(),
    )
    .with_table_style(TableStyle::striped())
    .col_widths(vec![15.0, 85.0]);

    // ── Assemble document ─────────────────────────────────────────────────────
    DocumentBuilder::new("normordis-pdf v1.3.0 — Named Styles Demo")
        .style(doc_style)
        .push(Section::new("1. Estilos Nomeados", 1))
        .push(intro_p)
        .push(sidebar_p)
        .push(spaced_p)
        .push(Spacer::new(4.0))
        .push(features)
        .push(Spacer::new(8.0))
        .push(Section::new("2. Tab Stops", 2))
        .push(Spacer::new(2.0))
        .push(tab1)
        .push(tab2)
        .push(tab3)
        .push(tab4)
        .push(Spacer::new(8.0))
        .push(Section::new("3. Estilos de Tabela", 2))
        .push(Section::new("3a. Grid", 3))
        .push(Spacer::new(2.0))
        .push(grid_table)
        .push(Spacer::new(4.0))
        .push(Section::new("3b. Bordered + CellPadding", 3))
        .push(Spacer::new(2.0))
        .push(bordered_table)
        .push(Spacer::new(4.0))
        .push(Section::new("3c. Striped", 3))
        .push(Spacer::new(2.0))
        .push(striped_table)
        .push(Spacer::new(8.0))
        .push(Section::new("4. Caption Style", 2))
        .push(Spacer::new(2.0))
        .push(caption_p)
        .render_to_bytes()
}
