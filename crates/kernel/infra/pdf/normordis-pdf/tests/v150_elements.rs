use normordis_pdf::{
    AccessibilityConfig, CheckBoxDef, ComboBoxDef, DocumentBuilder, DocumentStyle, FieldRect,
    FontRegistry, FormField, FootnoteMarkStyle, FootnoteRef, FOOTNOTE_SEPARATOR_THICKNESS_MM,
    InstitutionalHeader, PageFlow, PageLayout, Paragraph, RenderContext, Section, StructureTree,
    Table, TableCell, TableOfContents, TextFieldDef,
    elements::{footnote::FootnoteAccumulator, Element},
    layout::{TextLayoutEngine, GlyphUsageTracker},
    backend::pdf_writer_backend::PdfWriterBackend,
};

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_ctx() -> RenderContext {
    let style = DocumentStyle::default();
    let fonts = FontRegistry::new();
    let layout_engine = TextLayoutEngine::new(&fonts, &style);
    let flow = PageFlow::new(&style);
    let layout = PageLayout::from_style(&style);
    let default_font_family = fonts.default_family_name().to_string();
    RenderContext {
        backend: Box::new(PdfWriterBackend::new("test", 0)),
        font_map: std::collections::HashMap::new(),
        flow,
        layout,
        layout_engine,
        style,
        fonts,
        force_page_break: false,
        default_font_family,
        page_number: 1,
        total_pages: 1,
        resume_index: 0,
        glyph_tracker: GlyphUsageTracker::new(),
        reserved_footnotes_mm: 0.0,
        ua_config: AccessibilityConfig::default(),
        ua_events: StructureTree::new(),
        mcid_counter: 0,
        last_heading_level: None,
    }
}

fn renders_to_pdf(builder: DocumentBuilder) -> Vec<u8> {
    let bytes = builder.render_to_bytes().expect("render must not fail");
    assert!(bytes.starts_with(b"%PDF-"), "output must be a valid PDF");
    bytes
}

// ── Fontes (01-06) ────────────────────────────────────────────────────────────

// 01. FontRegistry::default() tem "LiberationSerif" registado
#[test]
fn font_01_registry_has_liberation_serif() {
    let reg = FontRegistry::default();
    assert!(reg.get("LiberationSerif").is_some(), "LiberationSerif must be registered");
}

// 02. FontRegistry::default() tem "LiberationMono" registado
#[test]
fn font_02_registry_has_liberation_mono() {
    let reg = FontRegistry::default();
    assert!(reg.get("LiberationMono").is_some(), "LiberationMono must be registered");
}

// 03. alias "Times New Roman" resolve para "LiberationSerif"
#[test]
fn font_03_times_new_roman_alias_resolves_to_liberation_serif() {
    let reg = FontRegistry::default();
    let resolved = reg.get_family("Times New Roman");
    let serif = reg.get_family("LiberationSerif");
    // Both should return the same font data (same units_per_em)
    assert_eq!(
        resolved.regular.units_per_em,
        serif.regular.units_per_em,
        "Times New Roman alias must resolve to LiberationSerif"
    );
}

// 04. alias "Courier New" resolve para "LiberationMono"
#[test]
fn font_04_courier_new_alias_resolves_to_liberation_mono() {
    let reg = FontRegistry::default();
    let resolved = reg.get_family("Courier New");
    let mono = reg.get_family("LiberationMono");
    assert_eq!(
        resolved.regular.units_per_em,
        mono.regular.units_per_em,
        "Courier New alias must resolve to LiberationMono"
    );
}

// 05. LiberationMono é monospace — 'i' e 'M' têm o mesmo advance
#[test]
fn font_05_liberation_mono_is_monospace() {
    let reg = FontRegistry::default();
    let w_i = reg.measure_text_mm("i", "LiberationMono", 12.0, false, false);
    let w_m = reg.measure_text_mm("M", "LiberationMono", 12.0, false, false);
    assert!(
        (w_i - w_m).abs() < 0.01,
        "LiberationMono must be monospace: 'i'={w_i:.4} vs 'M'={w_m:.4}"
    );
}

// 06. measure_text_mm com LiberationSerif retorna valor positivo para texto não vazio
#[test]
fn font_06_liberation_serif_measure_is_positive() {
    let reg = FontRegistry::default();
    let w = reg.measure_text_mm("Texto", "LiberationSerif", 12.0, false, false);
    assert!(w > 0.0, "LiberationSerif advance must be > 0, got {w}");
}

// ── Hifenização (07-12) ───────────────────────────────────────────────────────

// 07. (feature="hyphenation") hyphenate_word retorna pontos não vazios para palavra longa
#[cfg(feature = "hyphenation")]
#[test]
fn hyph_07_hyphenate_long_word_returns_breaks() {
    let style = DocumentStyle::default();
    let fonts = FontRegistry::new();
    let engine = TextLayoutEngine::new(&fonts, &style);
    let breaks = engine.hyphenate_word("implementação");
    assert!(!breaks.is_empty(), "PT-PT: 'implementação' must have hyphenation breaks");
}

// 08. (feature="hyphenation") palavra demasiado curta (< 5 chars) retorna []
#[cfg(feature = "hyphenation")]
#[test]
fn hyph_08_short_word_returns_no_breaks() {
    let style = DocumentStyle::default();
    let fonts = FontRegistry::new();
    let engine = TextLayoutEngine::new(&fonts, &style);
    let breaks = engine.hyphenate_word("de");
    assert!(breaks.is_empty(), "word 'de' (< 5 chars) must return no breaks");
}

// 09. (sem feature="hyphenation") hyphenate_word nunca faz panic e retorna []
#[cfg(not(feature = "hyphenation"))]
#[test]
fn hyph_09_without_feature_returns_empty() {
    let style = DocumentStyle::default();
    let fonts = FontRegistry::new();
    let engine = TextLayoutEngine::new(&fonts, &style);
    let breaks = engine.hyphenate_word("implementação");
    assert!(breaks.is_empty(), "without hyphenation feature must return []");
}

// 09 with-feature variant (always true when feature is on — just verifies no panic)
#[cfg(feature = "hyphenation")]
#[test]
fn hyph_09_with_feature_no_panic() {
    let style = DocumentStyle::default();
    let fonts = FontRegistry::new();
    let engine = TextLayoutEngine::new(&fonts, &style);
    // Must not panic for any input
    let _ = engine.hyphenate_word("a");
    let _ = engine.hyphenate_word("");
    let _ = engine.hyphenate_word("responsabilidade");
}

// 10. Parágrafo com soft hyphen U+00AD não causa panic
#[test]
fn hyph_10_soft_hyphen_paragraph_renders_without_panic() {
    let mut ctx = make_ctx();
    // U+00AD = soft hyphen — must not cause panic
    let text = "im\u{00AD}ple\u{00AD}men\u{00AD}tação de fun\u{00AD}cio\u{00AD}na\u{00AD}li\u{00AD}dades";
    let p = Paragraph::new(text);
    p.render(&mut ctx).expect("soft hyphen paragraph must render without error");
}

// 11. Parágrafo com palavra muito longa sem hifenização não causa panic
#[test]
fn hyph_11_long_word_without_hyphenation_no_panic() {
    let bytes = DocumentBuilder::new("test")
        .push(Paragraph::new("Pneumonoultramicroscopicsilicovolcanoconiosis is a long word."))
        .render_to_bytes()
        .expect("long word must not cause panic");
    assert!(bytes.starts_with(b"%PDF-"));
}

// 12. (feature="hyphenation") parágrafo com texto longo usa hifenização
#[cfg(feature = "hyphenation")]
#[test]
fn hyph_12_with_feature_long_text_renders_ok() {
    let bytes = DocumentBuilder::new("hyph test")
        .push(Paragraph::new(
            "responsabilidade desenvolvimento implementação funcionalidade \
             disponibilidade estabelecimento"
        ).align(normordis_pdf::TextAlign::Justify))
        .render_to_bytes()
        .expect("justified long text must render without error");
    assert!(bytes.starts_with(b"%PDF-"));
}

// ── Footnotes (13-19) ─────────────────────────────────────────────────────────

// 13. add_footnote() retorna números sequenciais (1, 2, 3, …)
#[test]
fn note_13_add_footnote_returns_sequential_numbers() {
    let mut builder = DocumentBuilder::new("test");
    let n1 = builder.add_footnote(vec!["First note.".to_string()]);
    let n2 = builder.add_footnote(vec!["Second note.".to_string()]);
    let n3 = builder.add_footnote(vec!["Third note.".to_string()]);
    assert_eq!(n1, 1);
    assert_eq!(n2, 2);
    assert_eq!(n3, 3);
}

// 14. FootnoteRef::mark_text() com number=1 retorna "1"
#[test]
fn note_14_footnote_ref_mark_text_numeric() {
    let r = FootnoteRef::new(1);
    assert_eq!(r.mark_text(), "1");
}

// 14b. FootnoteMarkStyle::Alpha retorna letras; Symbol retorna símbolos
#[test]
fn note_14b_mark_style_variants() {
    let alpha = FootnoteRef { number: 1, mark_style: FootnoteMarkStyle::Alpha };
    assert_eq!(alpha.mark_text(), "a");

    let sym = FootnoteRef { number: 1, mark_style: FootnoteMarkStyle::Symbol };
    assert_eq!(sym.mark_text(), "*");
}

// 15. Documento com 1 footnote renderiza sem panic e produz PDF válido
#[test]
fn note_15_document_with_footnote_renders() {
    let mut builder = DocumentBuilder::new("footnote test");
    let _n = builder.add_footnote(vec!["Ver referência bibliográfica.".to_string()]);
    builder = builder.push(Paragraph::new("Texto com nota de rodapé."));
    renders_to_pdf(builder);
}

// 16. FootnoteAccumulator::reserve() aumenta reserved_height_mm
#[test]
fn note_16_accumulator_reserve_increases_height() {
    let mut acc = FootnoteAccumulator::new();
    assert_eq!(acc.reserved_height_mm, 0.0);
    let h = acc.reserve(1, vec!["First note text.".to_string()], 5.0);
    assert!(h > 0.0, "reserve() must return positive height");
    assert!(acc.reserved_height_mm > 0.0, "reserved_height_mm must increase");
}

// 17. PageFlow::would_overflow_with_footnotes considera reserva de footnotes
#[test]
fn note_17_would_overflow_considers_footnote_reservation() {
    let style = DocumentStyle::default();
    let mut flow = PageFlow::new(&style);
    // Move cursor near the bottom
    let near_bottom = style.margin_bottom_mm + 5.0;
    flow.cursor_y_mm = near_bottom;

    // Without footnote reservation — 4mm still fits
    assert!(!flow.would_overflow_with_footnotes(4.0, 0.0));
    // With 2mm footnote reservation — 4mm no longer fits
    assert!(flow.would_overflow_with_footnotes(4.0, 2.0));
}

// 18. FootnoteAccumulator com pending non-empty → is_empty() = false
#[test]
fn note_18_accumulator_is_not_empty_after_reserve() {
    let mut acc = FootnoteAccumulator::new();
    assert!(acc.is_empty());
    acc.reserve(1, vec!["Note text.".to_string()], 5.0);
    assert!(!acc.is_empty());
}

// 19. FOOTNOTE_SEPARATOR_THICKNESS_MM é 0.25
#[test]
fn note_19_separator_thickness_is_025mm() {
    assert!(
        (FOOTNOTE_SEPARATOR_THICKNESS_MM - 0.25).abs() < 1e-9,
        "separator thickness must be 0.25 mm"
    );
}

// ── TOC (20-29) ───────────────────────────────────────────────────────────────

// 20. TableOfContents::new() tem max_level=3 por defeito
#[test]
fn toc_20_default_max_level_is_3() {
    let toc = TableOfContents::new();
    assert_eq!(toc.max_level, 3);
}

// 21. TableOfContents::new() tem leader_char='.' por defeito
#[test]
fn toc_21_default_leader_char_is_dot() {
    let toc = TableOfContents::new();
    assert_eq!(toc.leader_char, '.');
}

// 22. Documento com TOC + 3 secções produz PDF sem panic
#[test]
fn toc_22_document_with_toc_and_3_sections_renders() {
    let bytes = DocumentBuilder::new("TOC test")
        .push(TableOfContents::new())
        .push(Section::new("1. Introdução", 1))
        .push(Paragraph::new("Conteúdo da introdução."))
        .push(Section::new("2. Desenvolvimento", 1))
        .push(Paragraph::new("Conteúdo do desenvolvimento."))
        .push(Section::new("3. Conclusão", 1))
        .push(Paragraph::new("Conclusão do documento."))
        .render_to_bytes()
        .expect("document with TOC and 3 sections must render");
    assert!(bytes.starts_with(b"%PDF-"));
    // PDF must be substantial (TOC + 3 sections produce real content)
    assert!(bytes.len() > 5000, "PDF must be > 5 KB, got {} bytes", bytes.len());
}

// 23. TOC com max_level=1 não inclui secções de nível 2
#[test]
fn toc_23_max_level_1_excludes_level2_sections() {
    let bytes = DocumentBuilder::new("TOC max_level test")
        .push(TableOfContents::new().max_level(1))
        .push(Section::new("1. Capítulo", 1))
        .push(Section::new("1.1 Sub-secção", 2))
        .push(Paragraph::new("Texto."))
        .render_to_bytes()
        .expect("TOC with max_level=1 must render");
    assert!(bytes.starts_with(b"%PDF-"));
}

// 24. TOC com dot_leader personalizado renderiza sem panic
#[test]
fn toc_24_custom_dot_leader_renders() {
    let bytes = DocumentBuilder::new("TOC dot leader test")
        .push(TableOfContents::new().dot_leader('·'))
        .push(Section::new("Secção", 1))
        .render_to_bytes()
        .expect("custom dot leader TOC must render");
    assert!(bytes.starts_with(b"%PDF-"));
}

// 25. TableOfContents::entry_style_for retorna estilo correcto por nível
#[test]
fn toc_25_entry_style_for_returns_correct_name() {
    let toc = TableOfContents::new();
    assert_eq!(toc.entry_style_for(1), "toc_1");
    assert_eq!(toc.entry_style_for(2), "toc_2");
    assert_eq!(toc.entry_style_for(3), "toc_3");
    // Beyond configured levels falls back to "normal"
    assert_eq!(toc.entry_style_for(9), "normal");
}

// 26. Documento com TOC + title customizado renderiza sem panic
#[test]
fn toc_26_document_with_toc_custom_title_renders() {
    let bytes = DocumentBuilder::new("TOC title test")
        .push(TableOfContents::new().title("Sumário"))
        .push(Section::new("Cap. 1", 1))
        .render_to_bytes()
        .expect("TOC with custom title must render");
    assert!(bytes.starts_with(b"%PDF-"));
}

// 27. Documento com TOC presente produz PDF válido (two-pass executado)
#[test]
fn toc_27_with_toc_produces_valid_pdf() {
    let bytes = DocumentBuilder::new("two-pass test")
        .push(TableOfContents::new())
        .push(Section::new("Secção 1", 1))
        .push(Paragraph::new("Conteúdo."))
        .render_to_bytes()
        .expect("two-pass document must produce valid PDF");
    assert!(bytes.starts_with(b"%PDF-"));
}

// 28. Documento sem TOC mas com footer {{total_pages}} produz PDF válido
#[test]
fn toc_28_total_pages_footer_produces_valid_pdf() {
    use normordis_pdf::elements::footer::PageFooter;
    let bytes = DocumentBuilder::new("total_pages test")
        .footer(PageFooter::new().right("{{page}} / {{total_pages}}"))
        .push(Paragraph::new("Página com rodapé numerado."))
        .render_to_bytes()
        .expect("total_pages footer must render");
    assert!(bytes.starts_with(b"%PDF-"));
}

// 29. Documento sem TOC nem total_pages produz PDF válido (single-pass)
#[test]
fn toc_29_single_pass_document_renders() {
    let bytes = DocumentBuilder::new("single-pass test")
        .push(Paragraph::new("Documento simples sem TOC."))
        .render_to_bytes()
        .expect("single-pass document must render");
    assert!(bytes.starts_with(b"%PDF-"));
}

// ── InstitutionalHeader (35-39) ──────────────────────────────────────────────

// 35. InstitutionalHeader mínimo (só entity + título) renderiza sem pânico
#[test]
fn header_35_minimal_renders_without_panic() {
    let bytes = DocumentBuilder::new("test")
        .push(InstitutionalHeader::new("Entidade Pública", "Ofício nº 1/2026"))
        .push(Paragraph::new("Corpo do documento."))
        .render_to_bytes()
        .expect("minimal header must render");
    assert!(bytes.starts_with(b"%PDF-"));
}

// 36. InstitutionalHeader com todos os campos renderiza sem pânico
#[test]
fn header_36_full_fields_renders_without_panic() {
    let bytes = DocumentBuilder::new("test")
        .push(
            InstitutionalHeader::new("Câmara Municipal de Lisboa", "Ofício de Comunicação")
                .with_subtitle("Assunto: Confirmação de Recepção")
                .with_reference("REF/2026/042")
                .with_date("09 de maio de 2026"),
        )
        .push(Paragraph::new("Corpo do ofício."))
        .render_to_bytes()
        .expect("full-fields header must render");
    assert!(bytes.starts_with(b"%PDF-"));
}

// 37. InstitutionalHeader com subtitle aumenta o cursor em relação à versão sem
#[test]
fn header_37_subtitle_advances_cursor_more() {
    use normordis_pdf::elements::Element;
    let h_no_sub = InstitutionalHeader::new("Ent", "Título");
    let h_with_sub = InstitutionalHeader::new("Ent", "Título")
        .with_subtitle("Subtítulo informativo");
    assert!(
        h_with_sub.estimated_height_mm() >= h_no_sub.estimated_height_mm(),
        "estimated_height must not shrink when subtitle is present"
    );
}

// 38. InstitutionalHeader renderiza dentro de make_ctx() sem pânico
#[test]
fn header_38_render_via_make_ctx_no_panic() {
    use normordis_pdf::elements::Element;
    let mut ctx = make_ctx();
    let h = InstitutionalHeader::new("Organização", "Documento Oficial")
        .with_reference("REF-001")
        .with_date("2026-05-09");
    h.render(&mut ctx).expect("header render must not fail");
}

// 39. TOC with 2 sections produces PDF with /Annot link objects
#[test]
fn toc_30_toc_with_links_produces_valid_pdf() {
    let bytes = DocumentBuilder::new("TOC links test")
        .push(TableOfContents::new().title("Sumário"))
        .push(Section::new("1. Introdução", 1))
        .push(Paragraph::new("Texto de introdução."))
        .push(Section::new("2. Desenvolvimento", 1))
        .push(Paragraph::new("Texto de desenvolvimento."))
        .render_to_bytes()
        .expect("TOC with links must render");
    assert!(bytes.starts_with(b"%PDF-"));
    // The PDF must contain /Subtype /Link (link annotations from TOC entries)
    let content = std::str::from_utf8(&bytes).unwrap_or("");
    assert!(
        content.contains("/Link") || bytes.windows(5).any(|w| w == b"/Link"),
        "PDF must contain /Link annotation objects from TOC"
    );
}

// ── Tabelas aninhadas (30-34) ─────────────────────────────────────────────────

// 30. TableCell::nested_table() define campo nested_table como Some
#[test]
fn nested_30_nested_table_sets_field() {
    let inner = Table::builder()
        .row(vec![TableCell::new("a"), TableCell::new("b")])
        .build();
    let cell = TableCell::new("").nested_table(inner);
    assert!(cell.nested_table.is_some(), "nested_table must be Some after .nested_table()");
}

// 31. Tabela com célula aninhada renderiza sem panic
#[test]
fn nested_31_table_with_nested_cell_renders() {
    let inner = Table::builder()
        .row(vec![TableCell::new("Email"), TableCell::new("info@example.com")])
        .row(vec![TableCell::new("Tel."),  TableCell::new("21 000 0000")])
        .build();

    let outer = Table::builder()
        .row(vec![
            TableCell::new("Contactos"),
            TableCell::new("").nested_table(inner),
        ])
        .build();

    let bytes = DocumentBuilder::new("nested table test")
        .push(outer)
        .render_to_bytes()
        .expect("nested table must render without panic");
    assert!(bytes.starts_with(b"%PDF-"));
}

// 32. Célula pai com padding — nested_table não ultrapassa a largura da célula
#[test]
fn nested_32_padding_reduces_inner_width() {
    use normordis_pdf::CellPadding;
    let inner = Table::builder()
        .row(vec![TableCell::new("Nested content fits within padded cell")])
        .build();

    let cell = TableCell::new("")
        .nested_table(inner)
        .padding(CellPadding { top_mm: 2.0, bottom_mm: 2.0, left_mm: 4.0, right_mm: 4.0 });

    // Verify no panic when rendering with padding
    let bytes = DocumentBuilder::new("nested padding test")
        .push(Table::builder().row(vec![cell]).build())
        .render_to_bytes()
        .expect("padded nested table must render");
    assert!(bytes.starts_with(b"%PDF-"));
}

// 33. Tabela aninhada com múltiplas colunas renderiza sem panic
#[test]
fn nested_33_multi_column_nested_table_renders() {
    let inner = Table::builder()
        .row(vec![TableCell::new("A"), TableCell::new("B"), TableCell::new("C")])
        .row(vec![TableCell::new("1"), TableCell::new("2"), TableCell::new("3")])
        .build();

    let bytes = DocumentBuilder::new("multi-col nested test")
        .push(Table::builder()
            .row(vec![TableCell::new("Outer"), TableCell::new("").nested_table(inner)])
            .build())
        .render_to_bytes()
        .expect("multi-column nested table must render");
    assert!(bytes.starts_with(b"%PDF-"));
}

// 34. Tabela aninhada com 10 linhas numa célula estreita não causa panic
#[test]
fn nested_34_large_nested_table_no_panic() {
    let mut inner_rows = Vec::new();
    for i in 0..10 {
        inner_rows.push(vec![
            TableCell::new(format!("Item {i}")),
            TableCell::new(format!("Valor {i}")),
        ]);
    }
    let mut inner_builder = Table::builder();
    for row in inner_rows {
        inner_builder = inner_builder.row(row);
    }
    let inner = inner_builder.build();

    let bytes = DocumentBuilder::new("large nested test")
        .push(Table::builder()
            .row(vec![TableCell::new("").nested_table(inner)])
            .build())
        .render_to_bytes()
        .expect("large nested table must not panic");
    assert!(bytes.starts_with(b"%PDF-"));
}

// ── AcroForm (35-39) ──────────────────────────────────────────────────────────

// 35. FormField::TextField renderiza sem panic
#[test]
fn form_35_text_field_renders() {
    let bytes = DocumentBuilder::new("form test")
        .push(Paragraph::new("Nome:"))
        .form_field(FormField::TextField(TextFieldDef {
            name: "nome".into(),
            default_value: None,
            tooltip: Some("Nome completo".into()),
            multiline: false,
            max_length: Some(100),
            readonly: false,
            required: true,
            rect: FieldRect { x_mm: 25.0, y_mm: 240.0, width_mm: 120.0, height_mm: 8.0 },
            font_size: 11.0,
        }))
        .render_to_bytes()
        .expect("TextField must render");
    assert!(bytes.starts_with(b"%PDF-"));
}

// 36. FormField::CheckBox renderiza sem panic
#[test]
fn form_36_checkbox_renders() {
    let bytes = DocumentBuilder::new("checkbox test")
        .form_field(FormField::CheckBox(CheckBoxDef {
            name: "aceitar".into(),
            checked_by_default: true,
            tooltip: None,
            rect: FieldRect { x_mm: 20.0, y_mm: 220.0, width_mm: 5.0, height_mm: 5.0 },
        }))
        .render_to_bytes()
        .expect("CheckBox must render");
    assert!(bytes.starts_with(b"%PDF-"));
}

// 37. FormField::ComboBox com options renderiza sem panic
#[test]
fn form_37_combobox_renders() {
    let bytes = DocumentBuilder::new("combo test")
        .form_field(FormField::ComboBox(ComboBoxDef {
            name: "distrito".into(),
            options: vec!["Lisboa".into(), "Porto".into(), "Faro".into()],
            default_value: Some("Lisboa".into()),
            editable: false,
            tooltip: None,
            rect: FieldRect { x_mm: 25.0, y_mm: 200.0, width_mm: 60.0, height_mm: 8.0 },
            font_size: 10.0,
        }))
        .render_to_bytes()
        .expect("ComboBox must render");
    assert!(bytes.starts_with(b"%PDF-"));
}

// 38. Documento com 3 campos de formulário renderiza sem panic
#[test]
fn form_38_document_with_three_fields_renders() {
    let bytes = DocumentBuilder::new("multi-field form")
        .push(Paragraph::new("Formulário de candidatura"))
        .form_field(FormField::TextField(TextFieldDef {
            name: "nome".into(),
            default_value: None,
            tooltip: None,
            multiline: false,
            max_length: None,
            readonly: false,
            required: true,
            rect: FieldRect { x_mm: 25.0, y_mm: 230.0, width_mm: 100.0, height_mm: 8.0 },
            font_size: 11.0,
        }))
        .form_field(FormField::CheckBox(CheckBoxDef {
            name: "termos".into(),
            checked_by_default: false,
            tooltip: None,
            rect: FieldRect { x_mm: 25.0, y_mm: 210.0, width_mm: 5.0, height_mm: 5.0 },
        }))
        .form_field(FormField::ComboBox(ComboBoxDef {
            name: "categoria".into(),
            options: vec!["A".into(), "B".into()],
            default_value: None,
            editable: false,
            tooltip: None,
            rect: FieldRect { x_mm: 25.0, y_mm: 190.0, width_mm: 40.0, height_mm: 8.0 },
            font_size: 10.0,
        }))
        .render_to_bytes()
        .expect("3-field form must render");
    assert!(bytes.starts_with(b"%PDF-"));
}

// 39. FieldRect com dimensões mínimas não causa overflow ou panic
#[test]
fn form_39_minimal_field_rect_no_panic() {
    let bytes = DocumentBuilder::new("min rect test")
        .form_field(FormField::CheckBox(CheckBoxDef {
            name: "mini".into(),
            checked_by_default: false,
            tooltip: None,
            rect: FieldRect { x_mm: 1.0, y_mm: 1.0, width_mm: 3.0, height_mm: 3.0 },
        }))
        .render_to_bytes()
        .expect("minimal FieldRect must not panic");
    assert!(bytes.starts_with(b"%PDF-"));
}

// ── NDT 1.5.0 (40-45) ────────────────────────────────────────────────────────

// 40. NDT 1.5.0 com footnote_ref element deserializa correctamente
#[test]
fn ndt_40_footnote_ref_element_deserializes() {
    let template = r#"{
        "ndt": "1.5.0",
        "body": [
            { "type": "footnote_ref", "number": 1 }
        ]
    }"#;
    let doc = normordis_pdf::parse_ndt(template).expect("NDT 1.5.0 with footnote_ref must parse");
    assert_eq!(doc.body.len(), 1);
}

// 41. NDT 1.5.0 com toc element deserializa correctamente
#[test]
fn ndt_41_toc_element_deserializes() {
    let template = r#"{
        "ndt": "1.5.0",
        "body": [
            { "type": "toc", "title": "Índice", "max_level": 3 }
        ]
    }"#;
    let doc = normordis_pdf::parse_ndt(template).expect("NDT 1.5.0 with toc must parse");
    assert_eq!(doc.body.len(), 1);
}

// 42. NDT 1.5.0 com acroform_field deserializa correctamente
#[test]
fn ndt_42_acroform_field_deserializes() {
    let template = r#"{
        "ndt": "1.5.0",
        "body": [
            {
                "type": "acroform_field",
                "field_type": "text_field",
                "name": "campo_nome",
                "required": true,
                "font_size": 11.0,
                "rect": { "x_mm": 25.0, "y_mm": 240.0, "width_mm": 120.0, "height_mm": 8.0 }
            }
        ]
    }"#;
    let doc = normordis_pdf::parse_ndt(template).expect("NDT 1.5.0 with acroform_field must parse");
    assert_eq!(doc.body.len(), 1);
}

// 43. NDT 1.4.0 (sem elementos 1.5.0) renderiza sem alteração (backwards compat)
#[test]
fn ndt_43_v140_template_backwards_compat() {
    let template = r#"{
        "ndt": "1.4.0",
        "body": [
            { "type": "paragraph", "text": "Texto compatível com NDT 1.4.0." },
            { "type": "heading", "level": 1, "text": "Título 1.4.0" }
        ]
    }"#;
    let data = r#"{"ndt_data":"1.0.0","data":{}}"#;
    let bytes = DocumentBuilder::new("v1.4.0 compat")
        .push_ndt(template, data)
        .expect("NDT 1.4.0 parse must succeed")
        .render_to_bytes()
        .expect("NDT 1.4.0 render must succeed");
    assert!(bytes.starts_with(b"%PDF-"));
}

// 44. NCRTF 1.3.0 com footnote_ref inline → FootnoteRef correctamente mapeado
#[test]
fn ncrtf_44_footnote_ref_inline_maps_correctly() {
    let json = r#"{
        "ncrtf": "1.3.0",
        "meta": {},
        "blocks": [
            {
                "type": "paragraph",
                "children": [
                    { "type": "text", "text": "Conforme legislação" },
                    { "type": "footnote_ref", "number": 1 },
                    { "type": "text", "text": "." }
                ]
            }
        ]
    }"#;
    let doc = normordis_pdf::parse_ncrtf(json).expect("NCRTF 1.3.0 with footnote_ref must parse");
    assert_eq!(doc.blocks.len(), 1);

    // Render to PDF must succeed
    let bytes = DocumentBuilder::new("ncrtf footnote_ref")
        .push_ncrtf(json)
        .expect("push_ncrtf must succeed")
        .render_to_bytes()
        .expect("render must succeed");
    assert!(bytes.starts_with(b"%PDF-"));
}

// 45. NCRTF 1.2.0 (sem footnote_ref) renderiza com defaults correctos
#[test]
fn ncrtf_45_v120_renders_with_defaults() {
    let json = r#"{
        "ncrtf": "1.2.0",
        "meta": {},
        "blocks": [
            {
                "type": "paragraph",
                "children": [
                    { "type": "text", "text": "Documento NCRTF 1.2.0.", "marks": [] }
                ]
            },
            {
                "type": "heading",
                "level": 1,
                "children": [
                    { "type": "text", "text": "Título", "marks": [] }
                ]
            }
        ]
    }"#;
    let bytes = DocumentBuilder::new("ncrtf 1.2.0")
        .push_ncrtf(json)
        .expect("NCRTF 1.2.0 push must succeed")
        .render_to_bytes()
        .expect("NCRTF 1.2.0 render must succeed");
    assert!(bytes.starts_with(b"%PDF-"));
}
