use normordis_pdf::{
    AccessibilityConfig, DocumentBuilder, DocumentStyle, Element, FixedBox, FixedTextBox,
    FontRegistry, LayoutMode, OverflowPolicy, PageFlow, PageLayout, ParagraphContent,
    RenderContext, Spacer, StructureTree, TextAlign, TextLayoutEngine, VerticalAlign,
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
        glyph_tracker: normordis_pdf::GlyphUsageTracker::new(),
        reserved_footnotes_mm: 0.0,
        ua_config: AccessibilityConfig::default(),
        ua_events: StructureTree::new(),
        mcid_counter: 0,
        last_heading_level: None,
    }
}

fn default_fixed_box() -> FixedBox {
    FixedBox {
        x_mm: 10.0,
        y_mm: 100.0,
        width_mm: 60.0,
        height_mm: 20.0,
        padding_mm: 2.0,
        ..Default::default()
    }
}

// ── 1. inner_width_mm subtracts padding × 2 ──────────────────────────────────

#[test]
fn inner_width_subtracts_padding() {
    let b = FixedBox { width_mm: 60.0, padding_mm: 2.0, ..Default::default() };
    assert_eq!(b.inner_width_mm(), 56.0);
}

// ── 2. inner_height_mm subtracts padding × 2 ─────────────────────────────────

#[test]
fn inner_height_subtracts_padding() {
    let b = FixedBox { height_mm: 20.0, padding_mm: 2.0, ..Default::default() };
    assert_eq!(b.inner_height_mm(), 16.0);
}

// ── 3. FixedTextBox::layout_mode returns Fixed, not Flow ─────────────────────

#[test]
fn fixed_text_box_layout_mode_is_fixed() {
    let el = FixedTextBox {
        text_box: default_fixed_box(),
        content: ParagraphContent::Plain("hello".into()),
        alignment: TextAlign::Left,
        font_size: None,
        vertical_align: VerticalAlign::Top,
    };
    assert!(
        matches!(el.layout_mode(), LayoutMode::Fixed(_)),
        "expected LayoutMode::Fixed"
    );
}

// ── 4. FixedTextBox::estimated_height_mm returns 0.0 ─────────────────────────

#[test]
fn fixed_text_box_estimated_height_is_zero() {
    let el = FixedTextBox {
        text_box: default_fixed_box(),
        content: ParagraphContent::Plain("hello".into()),
        alignment: TextAlign::Left,
        font_size: None,
        vertical_align: VerticalAlign::Top,
    };
    assert_eq!(el.estimated_height_mm(), 0.0);
}

// ── 5. Flow cursor unchanged after Fixed element render ───────────────────────

#[test]
fn fixed_element_does_not_advance_cursor() {
    use normordis_pdf::Element;

    let mut ctx = make_ctx();
    let y_before = ctx.flow.cursor_y_mm;

    let el = FixedTextBox {
        text_box: default_fixed_box(),
        content: ParagraphContent::Plain("test text".into()),
        alignment: TextAlign::Left,
        font_size: None,
        vertical_align: VerticalAlign::Top,
    };

    el.render(&mut ctx).expect("render should not fail");

    assert_eq!(
        ctx.flow.cursor_y_mm, y_before,
        "Fixed element must not advance the flow cursor"
    );
}

// ── 6. Flow continues from same cursor after Fixed element ────────────────────

#[test]
fn flow_cursor_unaffected_by_preceding_fixed_element() {
    use normordis_pdf::Element;

    let mut ctx = make_ctx();

    // Advance cursor with a flow element.
    let spacer = Spacer::new(10.0);
    spacer.render(&mut ctx).unwrap();
    let y_after_spacer = ctx.flow.cursor_y_mm;

    // Render a Fixed element.
    let fixed_el = FixedTextBox {
        text_box: default_fixed_box(),
        content: ParagraphContent::Plain("fixed content".into()),
        alignment: TextAlign::Left,
        font_size: None,
        vertical_align: VerticalAlign::Top,
    };
    fixed_el.render(&mut ctx).unwrap();

    // Cursor must be exactly where the spacer left it.
    assert_eq!(
        ctx.flow.cursor_y_mm, y_after_spacer,
        "cursor must remain at the post-spacer position after Fixed element"
    );
}

// ── 7. OverflowPolicy::Shrink reduces font size for overflowing content ───────

#[test]
fn shrink_policy_reduces_font_size() {
    let ctx = make_ctx();

    // Very small box: 10mm × 5mm, tiny inner area.
    let small_box = FixedBox {
        x_mm: 0.0,
        y_mm: 0.0,
        width_mm: 10.0,
        height_mm: 5.0,
        padding_mm: 0.0,
        overflow: OverflowPolicy::Shrink,
        ..Default::default()
    };

    let el = FixedTextBox {
        text_box: small_box,
        content: ParagraphContent::Plain(
            "This is a long text that definitely does not fit at the default body font size"
                .into(),
        ),
        alignment: TextAlign::Left,
        font_size: Some(12.0),
        vertical_align: VerticalAlign::Top,
    };

    let effective = el.effective_font_size(&ctx);
    assert!(
        effective < 12.0,
        "expected font size < 12.0, got {effective}"
    );
    assert!(effective >= 6.0, "font size must not drop below 6.0");
}

// ── 8. VerticalAlign::Middle computes correct Y start ────────────────────────

#[test]
fn vertical_align_middle_offset() {
    let fb = FixedBox {
        x_mm: 0.0,
        y_mm: 10.0,
        width_mm: 50.0,
        height_mm: 20.0,
        padding_mm: 0.0,
        ..Default::default()
    };
    // inner_y_top_mm = 10 + 20 - 0 = 30; inner_height = 20
    let el = FixedTextBox {
        text_box: fb,
        content: ParagraphContent::Plain("x".into()),
        alignment: TextAlign::Center,
        font_size: None,
        vertical_align: VerticalAlign::Middle,
    };

    let content_h = 4.0; // simulated content height
    let y_start = el.content_y_start_mm(content_h);
    // expected: 30 - (20 - 4) / 2 = 30 - 8 = 22
    assert!(
        (y_start - 22.0).abs() < 0.001,
        "expected y_start ≈ 22.0, got {y_start}"
    );
}

// ── 9. NCRTF fixed_box block deserialises correctly ──────────────────────────

#[test]
fn ncrtf_fixed_box_deserialises() {
    let json = r#"{
        "ncrtf": "1.0",
        "meta": {},
        "blocks": [
            {
                "type": "fixed_box",
                "x_mm": 20.0,
                "y_mm": 257.0,
                "width_mm": 120.0,
                "height_mm": 15.0,
                "overflow": "truncate",
                "padding_mm": 2.0,
                "alignment": "left",
                "children": []
            }
        ]
    }"#;

    let doc = normordis_pdf::parse_ncrtf(json).expect("should parse");
    assert_eq!(doc.blocks.len(), 1);
    // Block variant should be FixedBox
    assert!(
        matches!(doc.blocks[0], normordis_pdf::richtext::model::Block::FixedBox(_)),
        "block should be FixedBox variant"
    );
}

// ── 10. ncrtf_to_elements maps fixed_box to FixedTextBox (LayoutMode::Fixed) ─

#[test]
fn ncrtf_fixed_box_converts_to_fixed_element() {
    let json = r#"{
        "ncrtf": "1.0",
        "meta": {},
        "blocks": [
            {
                "type": "fixed_box",
                "x_mm": 20.0,
                "y_mm": 257.0,
                "width_mm": 120.0,
                "height_mm": 15.0,
                "children": [
                    { "type": "text", "text": "Header text", "marks": [] }
                ]
            }
        ]
    }"#;

    let style = DocumentStyle::default();
    let doc = normordis_pdf::parse_ncrtf(json).unwrap();
    let elements = normordis_pdf::ncrtf_to_elements(&doc, &style);

    assert_eq!(elements.len(), 1);
    assert!(
        matches!(elements[0].layout_mode(), LayoutMode::Fixed(_)),
        "converted element should have LayoutMode::Fixed"
    );

    // Full render to PDF must succeed.
    let bytes = DocumentBuilder::new("fixed box ncrtf test")
        .push_ncrtf(json)
        .unwrap()
        .render_to_bytes()
        .unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
}
