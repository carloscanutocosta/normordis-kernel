// Tests for normordis-pdf v1.3.0 — Named Styles, Tab Stops, CellPadding, TableStyle

use std::collections::HashMap;

use normordis_pdf::{
    BulletList, CellPadding, DocumentBuilder, DocumentStyle, ListItemElement, NamedStyle,
    NormaxisPdfError, Paragraph, Section, Spacer, StyleResolver,
    TabStop, TabStopAlign, Table, TableCell, TableRow, TableStyle, TextAlign, TextRun,
    default_named_styles,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn default_doc() -> DocumentStyle {
    DocumentStyle::default()
}

fn resolver_for<'a>(
    styles: &'a HashMap<String, NamedStyle>,
    doc: &'a DocumentStyle,
) -> StyleResolver<'a> {
    StyleResolver::new(styles, doc)
}

fn empty_styles() -> HashMap<String, NamedStyle> {
    HashMap::new()
}

fn renders_ok(builder: DocumentBuilder) {
    let result = builder.render_to_bytes();
    assert!(result.is_ok(), "render_to_bytes failed: {:?}", result.err());
    let bytes = result.unwrap();
    assert!(bytes.len() > 1024, "PDF too small: {} bytes", bytes.len());
}

// ── StyleResolver — Built-in styles ──────────────────────────────────────────

#[test]
fn resolve_normal_returns_body_defaults() {
    let doc = default_doc();
    let styles = empty_styles();
    let resolver = resolver_for(&styles, &doc);
    let r = resolver.resolve("normal").unwrap();
    assert_eq!(r.font_size, doc.font_size_body);
    assert!(!r.bold);
    assert!(!r.italic);
}

#[test]
fn resolve_heading_1_is_bold_and_uses_title_size() {
    let doc = default_doc();
    let styles = empty_styles();
    let resolver = resolver_for(&styles, &doc);
    let r = resolver.resolve("heading_1").unwrap();
    assert_eq!(r.font_size, doc.font_size_title);
    assert!(r.bold);
    assert!(r.space_before_mm > 0.0);
    assert!(r.space_after_mm > 0.0);
}

#[test]
fn resolve_heading_2_inherits_bold_from_heading_1() {
    let doc = default_doc();
    let styles = empty_styles();
    let resolver = resolver_for(&styles, &doc);
    let r = resolver.resolve("heading_2").unwrap();
    assert!(r.bold, "heading_2 should inherit bold from heading_1");
    assert_eq!(r.font_size, doc.font_size_section);
}

#[test]
fn resolve_heading_3_inherits_chain_heading_1_via_heading_2() {
    let doc = default_doc();
    let styles = empty_styles();
    let resolver = resolver_for(&styles, &doc);
    let r = resolver.resolve("heading_3").unwrap();
    assert!(r.bold, "heading_3 should inherit bold via heading_2 → heading_1");
    assert_eq!(r.font_size, doc.font_size_body);
}

#[test]
fn resolve_caption_is_italic_and_centered() {
    let doc = default_doc();
    let styles = empty_styles();
    let resolver = resolver_for(&styles, &doc);
    let r = resolver.resolve("caption").unwrap();
    assert!(r.italic);
    assert_eq!(r.alignment, TextAlign::Center);
    assert_eq!(r.font_size, doc.font_size_small);
}

#[test]
fn resolve_table_header_is_bold() {
    let doc = default_doc();
    let styles = empty_styles();
    let resolver = resolver_for(&styles, &doc);
    let r = resolver.resolve("table_header").unwrap();
    assert!(r.bold);
    assert_eq!(r.space_before_mm, 0.0);
    assert_eq!(r.space_after_mm, 0.0);
}

#[test]
fn resolve_table_body_not_bold() {
    let doc = default_doc();
    let styles = empty_styles();
    let resolver = resolver_for(&styles, &doc);
    let r = resolver.resolve("table_body").unwrap();
    assert!(!r.bold);
}

// ── StyleResolver — User styles ───────────────────────────────────────────────

#[test]
fn user_style_overrides_builtin() {
    let mut styles = HashMap::new();
    styles.insert("normal".into(), NamedStyle {
        font_size: Some(14.0),
        bold: Some(true),
        ..Default::default()
    });
    let doc = default_doc();
    let resolver = resolver_for(&styles, &doc);
    let r = resolver.resolve("normal").unwrap();
    assert_eq!(r.font_size, 14.0);
    assert!(r.bold);
}

#[test]
fn user_style_inherits_from_builtin() {
    let mut styles = HashMap::new();
    styles.insert("my_style".into(), NamedStyle {
        extends: Some("caption".into()),
        font_size: Some(8.0),
        ..Default::default()
    });
    let doc = default_doc();
    let resolver = resolver_for(&styles, &doc);
    let r = resolver.resolve("my_style").unwrap();
    assert_eq!(r.font_size, 8.0);
    assert!(r.italic, "should inherit italic from caption");
    assert_eq!(r.alignment, TextAlign::Center, "should inherit center alignment");
}

#[test]
fn user_style_two_level_inheritance() {
    let mut styles = HashMap::new();
    styles.insert("base".into(), NamedStyle {
        font_size: Some(10.0),
        bold: Some(true),
        space_after_mm: Some(5.0),
        ..Default::default()
    });
    styles.insert("derived".into(), NamedStyle {
        extends: Some("base".into()),
        italic: Some(true),
        ..Default::default()
    });
    let doc = default_doc();
    let resolver = resolver_for(&styles, &doc);
    let r = resolver.resolve("derived").unwrap();
    assert_eq!(r.font_size, 10.0);
    assert!(r.bold);
    assert!(r.italic);
    assert_eq!(r.space_after_mm, 5.0);
}

#[test]
fn cycle_detection_returns_error() {
    let mut styles = HashMap::new();
    styles.insert("a".into(), NamedStyle { extends: Some("b".into()), ..Default::default() });
    styles.insert("b".into(), NamedStyle { extends: Some("a".into()), ..Default::default() });
    let doc = default_doc();
    let resolver = resolver_for(&styles, &doc);
    let err = resolver.resolve("a").unwrap_err();
    assert!(matches!(err, NormaxisPdfError::StyleCycleError(_)));
}

#[test]
fn unknown_style_returns_error() {
    let doc = default_doc();
    let styles = empty_styles();
    let resolver = resolver_for(&styles, &doc);
    let err = resolver.resolve("nonexistent_xyz").unwrap_err();
    assert!(matches!(err, NormaxisPdfError::UnknownStyle(_)));
}

#[test]
fn default_named_styles_has_builtin_entries() {
    let doc = default_doc();
    let styles = default_named_styles(&doc);
    for name in &["normal", "heading_1", "heading_2", "heading_3", "caption", "table_header", "table_body", "footnote", "toc_1", "toc_2", "toc_3"] {
        assert!(styles.contains_key(*name), "missing builtin: {}", name);
    }
}

// ── Paragraph style rendering ─────────────────────────────────────────────────

#[test]
fn paragraph_with_style_caption_renders() {
    renders_ok(DocumentBuilder::new("test")
        .push(Paragraph::new("Figure caption.").style("caption")));
}

#[test]
fn paragraph_with_user_style_renders() {
    let mut styles = HashMap::new();
    styles.insert("intro".into(), NamedStyle {
        extends: Some("normal".into()),
        font_size: Some(13.0),
        italic: Some(true),
        space_before_mm: Some(6.0),
        space_after_mm: Some(6.0),
        ..Default::default()
    });
    let doc = DocumentStyle { named_styles: styles, ..default_doc() };
    renders_ok(DocumentBuilder::new("test")
        .style(doc)
        .push(Paragraph::new("Intro paragraph.").style("intro")));
}

#[test]
fn paragraph_space_before_suppressed_at_top_of_page() {
    // Should not panic or overflow — space_before suppressed at page top.
    renders_ok(DocumentBuilder::new("test")
        .push(Paragraph::new("First paragraph.").space_before(50.0)));
}

#[test]
fn paragraph_space_after_explicit_renders() {
    renders_ok(DocumentBuilder::new("test")
        .push(Paragraph::new("A").space_after(10.0))
        .push(Paragraph::new("B")));
}

// ── Section style rendering ───────────────────────────────────────────────────

#[test]
fn section_level_1_uses_heading_1_builtin() {
    renders_ok(DocumentBuilder::new("test").push(Section::new("Title", 1)));
}

#[test]
fn section_level_2_uses_heading_2_builtin() {
    renders_ok(DocumentBuilder::new("test").push(Section::new("Subtitle", 2)));
}

#[test]
fn section_with_explicit_style_ref_renders() {
    renders_ok(DocumentBuilder::new("test")
        .push(Section::new("Captioned Heading", 1).style("caption")));
}

// ── Tab stops ─────────────────────────────────────────────────────────────────

#[test]
fn tab_stop_left_factory() {
    let ts = TabStop::left(60.0);
    assert_eq!(ts.position_mm, 60.0);
    assert_eq!(ts.alignment, TabStopAlign::Left);
    assert_eq!(ts.leader, ' ');
}

#[test]
fn tab_stop_right_with_leader() {
    let ts = TabStop::right(120.0).with_leader('.');
    assert_eq!(ts.position_mm, 120.0);
    assert_eq!(ts.alignment, TabStopAlign::Right);
    assert_eq!(ts.leader, '.');
}

#[test]
fn paragraph_with_tab_stop_renders() {
    let p = Paragraph::from_runs(
        vec![TextRun::plain("Label\t42")],
        TextAlign::Left,
        None,
    )
    .tab_stop(TabStop::right(140.0).with_leader('.'));
    renders_ok(DocumentBuilder::new("test").push(p));
}

#[test]
fn paragraph_with_no_tab_stop_and_tab_char_renders() {
    // \t with no tab stops defined — should not panic, treated as space.
    let p = Paragraph::from_runs(
        vec![TextRun::plain("A\tB\tC")],
        TextAlign::Left,
        None,
    );
    renders_ok(DocumentBuilder::new("test").push(p));
}

// ── CellPadding ───────────────────────────────────────────────────────────────

#[test]
fn cell_padding_default_values() {
    let p = CellPadding::default();
    assert_eq!(p.top_mm, 1.0);
    assert_eq!(p.bottom_mm, 1.0);
    assert_eq!(p.left_mm, 2.0);
    assert_eq!(p.right_mm, 2.0);
}

#[test]
fn cell_padding_uniform() {
    let p = CellPadding::uniform(5.0);
    assert_eq!(p.top_mm, 5.0);
    assert_eq!(p.bottom_mm, 5.0);
    assert_eq!(p.left_mm, 5.0);
    assert_eq!(p.right_mm, 5.0);
}

#[test]
fn cell_padding_horizontal_vertical() {
    let p = CellPadding::horizontal_vertical(3.0, 1.5);
    assert_eq!(p.left_mm, 3.0);
    assert_eq!(p.right_mm, 3.0);
    assert_eq!(p.top_mm, 1.5);
    assert_eq!(p.bottom_mm, 1.5);
}

#[test]
fn table_with_cell_padding_renders() {
    let table = Table::builder()
        .header_row(vec![
            TableCell::new("Col A").padding(CellPadding::uniform(4.0)),
            TableCell::new("Col B").padding(CellPadding::uniform(4.0)),
        ])
        .row(vec![
            TableCell::new("Value 1").padding(CellPadding::horizontal_vertical(3.0, 2.0)),
            TableCell::new("Value 2").padding(CellPadding::horizontal_vertical(3.0, 2.0)),
        ])
        .build();
    renders_ok(DocumentBuilder::new("test").push(table));
}

// ── TableStyle ────────────────────────────────────────────────────────────────

#[test]
fn table_style_grid_renders() {
    let table = Table::new(
        vec!["A".into(), "B".into()],
        vec![TableRow::plain(vec!["1".into(), "2".into()])],
    )
    .with_table_style(TableStyle::grid());
    renders_ok(DocumentBuilder::new("test").push(table));
}

#[test]
fn table_style_bordered_renders() {
    let table = Table::new(
        vec!["X".into()],
        vec![TableRow::plain(vec!["Y".into()])],
    )
    .with_table_style(TableStyle::bordered());
    renders_ok(DocumentBuilder::new("test").push(table));
}

#[test]
fn table_style_striped_renders() {
    let rows: Vec<TableRow> = (1..=4)
        .map(|i| TableRow::plain(vec![format!("Row {}", i)]))
        .collect();
    let table = Table::new(vec!["N".into()], rows)
        .with_table_style(TableStyle::striped());
    renders_ok(DocumentBuilder::new("test").push(table));
}

#[test]
fn table_style_plain_renders() {
    let table = Table::new(
        vec!["X".into()],
        vec![TableRow::plain(vec!["Y".into()])],
    )
    .with_table_style(TableStyle::plain());
    renders_ok(DocumentBuilder::new("test").push(table));
}

// ── Integration — full document with all v1.3.0 features ─────────────────────

#[test]
fn full_v130_document_renders() {
    let mut styles = HashMap::new();
    styles.insert("intro".into(), NamedStyle {
        extends: Some("normal".into()),
        italic: Some(true),
        space_before_mm: Some(4.0),
        space_after_mm: Some(4.0),
        ..Default::default()
    });
    let doc = DocumentStyle { named_styles: styles, ..default_doc() };

    let table = Table::new(
        vec!["Estilo".into(), "Descrição".into()],
        vec![
            TableRow::plain(vec!["caption".into(), "Legenda".into()]),
            TableRow::plain(vec!["normal".into(), "Corpo".into()]),
        ],
    )
    .with_table_style(TableStyle::grid())
    .col_widths(vec![40.0, 60.0]);

    let tab_p = Paragraph::from_runs(
        vec![TextRun::plain("Item\t99")],
        TextAlign::Left,
        None,
    )
    .tab_stop(TabStop::right(130.0).with_leader('.'));

    renders_ok(
        DocumentBuilder::new("v1.3.0 Integration Test")
            .style(doc)
            .push(Section::new("Título", 1))
            .push(Paragraph::new("Intro.").style("intro"))
            .push(Paragraph::new("Caption.").style("caption"))
            .push(Spacer::new(4.0))
            .push(tab_p)
            .push(Spacer::new(4.0))
            .push(table)
            .push(Spacer::new(4.0))
            .push(BulletList::new(vec![
                ListItemElement::plain("Feature A"),
                ListItemElement::plain("Feature B"),
            ])),
    );
}
