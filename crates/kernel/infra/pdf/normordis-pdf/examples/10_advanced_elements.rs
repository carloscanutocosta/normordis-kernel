//! Demonstrates v1.5.0 advanced elements: footnotes, TOC, nested tables,
//! AcroForm fields, and Liberation Serif/Mono fonts.
//! Run: cargo run --example 10_advanced_elements -p normordis-pdf

use normordis_pdf::{
    elements::footer::PageFooter, elements::header::InstitutionalHeader, CheckBoxDef, ComboBoxDef,
    DocumentBuilder, DocumentStyle, FieldRect, FormField, NamedStyle, Paragraph, Section, Spacer,
    Table, TableCell, TableOfContents, TextFieldDef, TextRun,
};

fn main() -> normordis_pdf::Result<()> {
    let mut builder = DocumentBuilder::new("Elementos Avançados v1.5.0");

    // Register footnotes (must be done before build)
    let note1 = builder.add_footnote(vec![
        "Diário da República, 1.ª série, n.º 42, de 1 de Março de 2026.".to_string(),
    ]);
    let note2 = builder.add_footnote(vec![
        "Aprovado por deliberação do executivo municipal em 15/01/2026.".to_string(),
    ]);

    // Custom style for code / monospace text
    let mut style = DocumentStyle::default();
    style.named_styles.insert(
        "codigo".into(),
        NamedStyle {
            extends: Some("normal".into()),
            font_family: Some("LiberationMono".into()),
            font_size: Some(10.0),
            ..Default::default()
        },
    );
    style.named_styles.insert(
        "classico".into(),
        NamedStyle {
            extends: Some("normal".into()),
            font_family: Some("LiberationSerif".into()),
            ..Default::default()
        },
    );

    let pdf = builder
        .style(style)
        .header(InstitutionalHeader::new("NORMAXIS", "Demonstração v1.5.0"))
        .footer(PageFooter::new().right("{{page}} / {{total_pages}}"))
        // ── TOC ───────────────────────────────────────────────────────
        .push(
            TableOfContents::new()
                .title("Índice")
                .max_level(3)
                .dot_leader('.'),
        )
        .push(Spacer::new(8.0))
        // ── Fontes ────────────────────────────────────────────────────
        .push(Section::new("1. Fontes Liberation", 1))
        .push(Paragraph::new(
            "Este texto usa Liberation Sans (equivalente Arial/Calibri). \
             Ideal para documentos institucionais modernos.",
        ))
        .push(
            Paragraph::new(
                "Este texto usa Liberation Serif (equivalente Times New Roman). \
             Adequado para textos formais e jurídicos.",
            )
            .style("classico"),
        )
        .push(Paragraph::new("struct Config { font: &'static str, size: f64 }").style("codigo"))
        // ── Footnotes ─────────────────────────────────────────────────
        .push(Spacer::new(4.0))
        .push(Section::new("2. Notas de Rodapé", 1))
        .push(Paragraph::from_runs(
            vec![
                TextRun::plain("Conforme legislação vigente"),
                TextRun::footnote_ref(note1),
                TextRun::plain(", o procedimento é aprovado"),
                TextRun::footnote_ref(note2),
                TextRun::plain("."),
            ],
            normordis_pdf::TextAlign::Left,
            None,
        ))
        // ── Tabela aninhada ───────────────────────────────────────────
        .push(Spacer::new(4.0))
        .push(Section::new("3. Tabela com Célula Aninhada", 1))
        .push(
            Table::builder()
                .row(vec![TableCell::new("Campo"), TableCell::new("Dados")])
                .row(vec![
                    TableCell::new("Entidade"),
                    TableCell::new("Câmara Municipal de Lisboa"),
                ])
                .row(vec![
                    TableCell::new("Contactos"),
                    TableCell::new("").nested_table(
                        Table::builder()
                            .row(vec![
                                TableCell::new("Email"),
                                TableCell::new("geral@cm-lisboa.pt"),
                            ])
                            .row(vec![TableCell::new("Tel."), TableCell::new("21 000 0000")])
                            .build(),
                    ),
                ])
                .build(),
        )
        // ── Formulário ────────────────────────────────────────────────
        .push(Spacer::new(4.0))
        .push(Section::new("4. Campos de Formulário (AcroForm)", 1))
        .push(Paragraph::new("Nome do requerente:"))
        .form_field(FormField::TextField(TextFieldDef {
            name: "nome_requerente".into(),
            default_value: None,
            tooltip: Some("Nome completo".into()),
            multiline: false,
            max_length: Some(100),
            readonly: false,
            required: true,
            rect: FieldRect {
                x_mm: 25.0,
                y_mm: 130.0,
                width_mm: 120.0,
                height_mm: 8.0,
            },
            font_size: 11.0,
        }))
        .push(Paragraph::new("Aceita os termos e condições:"))
        .form_field(FormField::CheckBox(CheckBoxDef {
            name: "aceita_termos".into(),
            checked_by_default: false,
            tooltip: Some("Marque para aceitar".into()),
            rect: FieldRect {
                x_mm: 25.0,
                y_mm: 112.0,
                width_mm: 5.0,
                height_mm: 5.0,
            },
        }))
        .push(Paragraph::new("Categoria:"))
        .form_field(FormField::ComboBox(ComboBoxDef {
            name: "categoria".into(),
            options: vec!["Tipo A".into(), "Tipo B".into(), "Tipo C".into()],
            default_value: Some("Tipo A".into()),
            editable: false,
            tooltip: None,
            rect: FieldRect {
                x_mm: 25.0,
                y_mm: 95.0,
                width_mm: 60.0,
                height_mm: 8.0,
            },
            font_size: 10.0,
        }))
        .render_to_bytes()?;

    let out = std::env::temp_dir().join("normaxis_advanced_elements.pdf");
    std::fs::write(&out, &pdf)?;
    println!("PDF gerado: {} ({} bytes)", out.display(), pdf.len());
    println!("Verificar visualmente:");
    println!("  [] TOC com dot leaders e números de página correctos");
    println!("  [] Footnotes no fundo da página com numeração ¹ ²");
    println!("  [] Liberation Serif visualmente diferente do Sans");
    println!("  [] Tabela aninhada dentro de célula");
    println!("  [] Campos de formulário visíveis (placeholders azuis)");
    Ok(())
}
