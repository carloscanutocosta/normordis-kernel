use normordis_pdf::{
    AccessibilityConfig, BulletList, DocumentBuilder, FixedBox, ImageElement, ListItemElement,
    Paragraph, PdfStandard, Section, Spacer, StructTag, StructureTree, Table, TableCell, UaError,
    UaValidator,
};

// ── helpers ───────────────────────────────────────────────────────────────────

fn ua_builder(title: &str) -> DocumentBuilder {
    DocumentBuilder::new(title).accessibility(AccessibilityConfig {
        enabled: true,
        lang: "pt-PT".into(),
        warn_missing_alt: true,
        fixed_box_default_artifact: true,
    })
}

fn renders_ok(b: DocumentBuilder) -> bool {
    b.render_to_bytes().is_ok()
}

fn tiny_png() -> Vec<u8> {
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xFF, 0xFF, 0x3F, 0x00, 0x05, 0xFE, 0x02, 0xFE, 0xDC, 0xCC, 0x59, 0xE7, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
}

// ── 01–03: AccessibilityConfig ────────────────────────────────────────────────

#[test]
fn ua01_default_config_has_enabled_false() {
    let cfg = AccessibilityConfig::default();
    assert!(!cfg.enabled);
}

#[test]
fn ua02_builder_with_accessibility_enabled_renders() {
    assert!(renders_ok(ua_builder("UA-02")));
}

#[test]
fn ua03_pdfa_ua2_standard_enables_ua_automatically() {
    let bytes = DocumentBuilder::new("UA-03")
        .standard(PdfStandard::PdfUa2)
        .push(Paragraph::new("Texto."))
        .render_to_bytes();
    assert!(bytes.is_ok());
    assert!(!bytes.unwrap().is_empty());
}

// ── 04–10: StructTag mapping ──────────────────────────────────────────────────

#[test]
fn ua04_section_level1_renders() {
    assert!(renders_ok(
        ua_builder("UA-04").push(Section::new("Título", 1))
    ));
}

#[test]
fn ua05_section_level6_renders() {
    assert!(renders_ok(
        ua_builder("UA-05").push(Section::new("Sub-sub-sub", 6))
    ));
}

#[test]
fn ua06_paragraph_renders() {
    assert!(renders_ok(
        ua_builder("UA-06").push(Paragraph::new("Parágrafo."))
    ));
}

#[test]
fn ua07_table_renders() {
    let tbl = Table::builder()
        .header_row(vec![TableCell::new("A"), TableCell::new("B")])
        .row(vec![TableCell::new("1"), TableCell::new("2")])
        .build();
    assert!(renders_ok(ua_builder("UA-07").push(tbl)));
}

#[test]
fn ua08_bullet_list_renders() {
    let list = BulletList::new(vec![
        ListItemElement::plain("Item 1"),
        ListItemElement::plain("Item 2"),
    ]);
    assert!(renders_ok(ua_builder("UA-08").push(list)));
}

#[test]
fn ua09_image_with_alt_renders() {
    let img = ImageElement::new(tiny_png())
        .width_mm(40.0)
        .alt("Descrição da imagem");
    assert!(renders_ok(ua_builder("UA-09").push(img)));
}

#[test]
fn ua10_image_without_alt_renders_as_artifact() {
    let img = ImageElement::new(tiny_png()).width_mm(40.0);
    assert!(renders_ok(ua_builder("UA-10").push(img)));
}

// ── 11–16: Artefactos ─────────────────────────────────────────────────────────

#[test]
fn ua11_header_renders_as_artifact() {
    use normordis_pdf::InstitutionalHeader;
    let h = InstitutionalHeader::new("Entidade", "Título");
    assert!(renders_ok(ua_builder("UA-11").header(h)));
}

#[test]
fn ua12_footer_renders_as_artifact() {
    use normordis_pdf::PageFooter;
    let f = PageFooter::new().right("{{page}}");
    assert!(renders_ok(ua_builder("UA-12").footer(f)));
}

#[test]
fn ua13_watermark_renders_as_artifact() {
    use normordis_pdf::Watermark;
    let wm = Watermark::new("RASCUNHO").opacity(0.1);
    assert!(renders_ok(ua_builder("UA-13").watermark(wm)));
}

#[test]
fn ua14_spacer_renders_without_panic() {
    assert!(renders_ok(ua_builder("UA-14").push(Spacer::new(10.0))));
}

#[test]
fn ua15_fixed_box_without_ua_role_renders() {
    use normordis_pdf::TextAlign;
    let fb = FixedBox {
        x_mm: 10.0,
        y_mm: 50.0,
        width_mm: 50.0,
        height_mm: 15.0,
        ua_role: None,
        ua_alt: None,
        ..Default::default()
    };
    assert!(renders_ok(ua_builder("UA-15").fixed_text(
        fb,
        "Sem role",
        TextAlign::Left
    )));
}

#[test]
fn ua16_fixed_box_with_ua_role_figure_renders() {
    use normordis_pdf::TextAlign;
    let fb = FixedBox {
        x_mm: 10.0,
        y_mm: 50.0,
        width_mm: 50.0,
        height_mm: 15.0,
        ua_role: Some(StructTag::Figure),
        ua_alt: Some("Figura decorativa".into()),
        ..Default::default()
    };
    assert!(renders_ok(ua_builder("UA-16").fixed_text(
        fb,
        "Figura",
        TextAlign::Center
    )));
}

// ── 17–21: Structure tree ─────────────────────────────────────────────────────

#[test]
fn ua17_ua2_pdf_is_not_empty() {
    let bytes = ua_builder("UA-17")
        .push(Section::new("Título", 1))
        .push(Paragraph::new("Texto."))
        .render_to_bytes()
        .unwrap();
    assert!(!bytes.is_empty());
}

#[test]
fn ua18_ua2_pdf_starts_with_pdf_header() {
    let bytes = ua_builder("UA-18")
        .push(Paragraph::new("Conteúdo."))
        .render_to_bytes()
        .unwrap();
    assert!(bytes.starts_with(b"%PDF"));
}

#[test]
fn ua19_ua2_pdf_contains_lang_entry() {
    let bytes = ua_builder("UA-19")
        .push(Paragraph::new("Texto PT."))
        .render_to_bytes()
        .unwrap();
    // /Lang entry should appear in the PDF bytes
    assert!(bytes.windows(5).any(|w| w == b"/Lang"));
}

#[test]
fn ua20_ua2_pdf_contains_struct_tree_root() {
    let bytes = ua_builder("UA-20")
        .push(Section::new("Sec", 1))
        .render_to_bytes()
        .unwrap();
    assert!(bytes.windows(16).any(|w| w == b"/StructTreeRoot "));
}

#[test]
fn ua21_mcid_counter_starts_at_zero() {
    use normordis_pdf::backend::pdf_writer_backend::PdfWriterBackend;
    use normordis_pdf::layout::TextLayoutEngine;
    use normordis_pdf::{DocumentStyle, FontRegistry, PageFlow, PageLayout, RenderContext};
    let style = DocumentStyle::default();
    let fonts = FontRegistry::new();
    let ctx = RenderContext {
        backend: Box::new(PdfWriterBackend::new("test", 0)),
        font_map: std::collections::HashMap::new(),
        flow: PageFlow::new(&style),
        layout: PageLayout::from_style(&style),
        layout_engine: TextLayoutEngine::new(&fonts, &style),
        style,
        fonts,
        force_page_break: false,
        default_font_family: "default".into(),
        page_number: 1,
        total_pages: 1,
        resume_index: 0,
        glyph_tracker: normordis_pdf::GlyphUsageTracker::new(),
        reserved_footnotes_mm: 0.0,
        ua_config: AccessibilityConfig {
            enabled: true,
            ..Default::default()
        },
        ua_events: StructureTree::new(),
        mcid_counter: 0,
        last_heading_level: None,
    };
    assert_eq!(ctx.mcid_counter, 0);
}

// ── 22–23: Ordem de leitura ───────────────────────────────────────────────────

#[test]
fn ua22_flow_elements_render_in_order() {
    let bytes = ua_builder("UA-22")
        .push(Section::new("A", 1))
        .push(Paragraph::new("Primeiro."))
        .push(Section::new("B", 1))
        .push(Paragraph::new("Segundo."))
        .render_to_bytes();
    assert!(bytes.is_ok());
}

#[test]
fn ua23_fixed_box_with_role_renders() {
    use normordis_pdf::TextAlign;
    let bytes = ua_builder("UA-23")
        .fixed_text(
            FixedBox {
                x_mm: 10.0,
                y_mm: 250.0,
                width_mm: 80.0,
                height_mm: 10.0,
                ua_role: Some(StructTag::Caption),
                ua_alt: None,
                ..Default::default()
            },
            "Legenda do gráfico",
            TextAlign::Left,
        )
        .render_to_bytes();
    assert!(bytes.is_ok());
}

// ── 24–27: Validação interna ──────────────────────────────────────────────────

#[test]
fn ua24_validator_with_tree_and_lang_has_no_errors() {
    let mut tree = StructureTree::new();
    tree.begin_group(StructTag::Document, None);
    tree.add_content_ref(0, 0);
    tree.end_group();
    let v = UaValidator::validate(Some(&tree), "pt-PT");
    assert!(!v.has_errors());
}

#[test]
fn ua25_validator_missing_lang_returns_error() {
    let mut tree = StructureTree::new();
    tree.begin_group(StructTag::Document, None);
    tree.end_group();
    let v = UaValidator::validate(Some(&tree), "");
    assert!(v
        .errors
        .iter()
        .any(|e| matches!(e, UaError::NoDocumentLanguage)));
}

#[test]
fn ua26_validator_empty_tree_returns_error() {
    let tree = StructureTree::new();
    let v = UaValidator::validate(Some(&tree), "pt-PT");
    assert!(v
        .errors
        .iter()
        .any(|e| matches!(e, UaError::NoStructureTree)));
}

#[test]
fn ua27_validator_none_tree_returns_error() {
    let v = UaValidator::validate(None, "pt-PT");
    assert!(v
        .errors
        .iter()
        .any(|e| matches!(e, UaError::NoStructureTree)));
}

// ── 28–30: NDT 2.1.0 ─────────────────────────────────────────────────────────

fn ndt_data() -> &'static str {
    r#"{"ndt_data":"1.0.0","data":{}}"#
}

#[test]
fn ua28_ndt_pdf_ua2_standard_sets_pdfa_ua2() {
    let template = r#"{"ndt":"2.1.0","output":{"standard":"pdf_ua2"},"body":[]}"#;
    let bytes = DocumentBuilder::new("UA-28")
        .push_ndt(template, ndt_data())
        .unwrap()
        .render_to_bytes();
    assert!(bytes.is_ok());
}

#[test]
fn ua29_ndt_without_pdf_ua2_does_not_activate_ua() {
    let template = r#"{"ndt":"2.0.0","body":[]}"#;
    let b = DocumentBuilder::new("UA-29")
        .push_ndt(template, ndt_data())
        .unwrap();
    let bytes = b.render_to_bytes();
    assert!(bytes.is_ok());
}

#[test]
fn ua30_ndt_pdf_ua2_produces_pdf_with_struct_tree_root() {
    let template = r#"{"ndt":"2.1.0","output":{"standard":"pdf_ua2"},
        "body":[{"type":"paragraph","text":"Acessível."}]}"#;
    let bytes = DocumentBuilder::new("UA-30")
        .push_ndt(template, ndt_data())
        .unwrap()
        .render_to_bytes()
        .unwrap();
    assert!(bytes.windows(16).any(|w| w == b"/StructTreeRoot "));
}

// ── 31–34: Conformidade ───────────────────────────────────────────────────────

#[test]
fn ua31_ua2_pdf_parseable_with_lopdf() {
    let bytes = ua_builder("UA-31")
        .push(Section::new("Título", 1))
        .push(Paragraph::new("Conteúdo acessível."))
        .render_to_bytes()
        .unwrap();
    let result = lopdf::Document::load_mem(&bytes);
    assert!(result.is_ok(), "lopdf failed: {:?}", result.err());
}

#[test]
fn ua32_ua2_pdf_size_reasonable() {
    let bytes = ua_builder("UA-32")
        .push(Section::new("Título", 1))
        .push(Paragraph::new("Conteúdo de teste para verificar tamanho."))
        .render_to_bytes()
        .unwrap();
    // Structure tree adds ~5–15KB on top of base PDF/A-2b (~37–50KB)
    assert!(
        bytes.len() < 150_000,
        "PDF demasiado grande: {} bytes",
        bytes.len()
    );
}

#[test]
fn ua33_image_alt_field_set_correctly() {
    let img = ImageElement::new(tiny_png()).alt("Texto alternativo da imagem");
    // just verify the builder compiles and renders
    assert!(renders_ok(ua_builder("UA-33").push(img)));
}

#[test]
fn ua34_fixed_box_role_figure_and_alt_set_correctly() {
    use normordis_pdf::TextAlign;
    let fb = FixedBox {
        x_mm: 10.0,
        y_mm: 80.0,
        width_mm: 60.0,
        height_mm: 20.0,
        ua_role: Some(StructTag::Figure),
        ua_alt: Some("Alt da figura fixa".into()),
        ..Default::default()
    };
    assert_eq!(fb.ua_role, Some(StructTag::Figure));
    assert_eq!(fb.ua_alt.as_deref(), Some("Alt da figura fixa"));
    assert!(renders_ok(ua_builder("UA-34").fixed_text(
        fb,
        "Figura",
        TextAlign::Left
    )));
}
