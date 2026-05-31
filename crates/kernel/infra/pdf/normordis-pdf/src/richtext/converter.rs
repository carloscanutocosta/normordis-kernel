use base64::Engine as _;

use crate::{
    elements::{
        fixed_text::{FixedTextBox, VerticalAlign},
        image::{ImageAlignment, ImageElement},
        list::{BulletList, CheckList, CheckListItem, ListItemElement, OrderedList},
        page_break::PageBreakElement,
        paragraph::{Paragraph, ParagraphContent, TextRun},
        section::Section,
        spacer::HorizontalRuleElement,
        table::{Table, TableCell, TableStyle},
        Element,
    },
    layout::{BorderStyle, BoxBorder, FixedBox, OverflowPolicy, TextAlign},
    richtext::{
        marks::{AppliedStyle, MarkValue},
        model::{
            Block, FixedBoxBlock, HeadingBlock, ImageAlign, ImageBlock, Inline, ListType,
            NcrtfDocument, ParagraphBlock, TableBlock,
        },
    },
    styles::{DocumentStyle, RgbColor},
};

// Dark navy used for hyperlinks.
const LINK_COLOR: RgbColor = RgbColor {
    r: 0.0,
    g: 0.2,
    b: 0.6,
};
// Mid-grey used for blockquote text.
const BLOCKQUOTE_COLOR: RgbColor = RgbColor {
    r: 0.47,
    g: 0.47,
    b: 0.47,
};

/// Convert a parsed `NcrtfDocument` into a flat list of `normordis-pdf` elements.
pub fn ncrtf_to_elements(doc: &NcrtfDocument, _style: &DocumentStyle) -> Vec<Box<dyn Element>> {
    let mut elements: Vec<Box<dyn Element>> = Vec::new();

    for block in &doc.blocks {
        match block {
            Block::Heading(h) => elements.push(heading_to_section(h)),
            Block::Paragraph(p) => elements.push(paragraph_block_to_element(p)),
            Block::List(l) => match l.list_type {
                ListType::Bullet => {
                    let items = l
                        .children
                        .iter()
                        .map(|li| ListItemElement {
                            indent: li.indent.unwrap_or(0),
                            runs: inlines_to_runs(&li.children),
                        })
                        .collect();
                    elements.push(Box::new(BulletList { items }));
                }
                ListType::Ordered => {
                    let items = l
                        .children
                        .iter()
                        .map(|li| ListItemElement {
                            indent: li.indent.unwrap_or(0),
                            runs: inlines_to_runs(&li.children),
                        })
                        .collect();
                    elements.push(Box::new(OrderedList { start: 1, items }));
                }
                ListType::Checklist => {
                    let items = l
                        .children
                        .iter()
                        .map(|li| CheckListItem {
                            checked: li.checked.unwrap_or(false),
                            indent: li.indent.unwrap_or(0),
                            runs: inlines_to_runs(&li.children),
                        })
                        .collect();
                    elements.push(Box::new(CheckList { items }));
                }
            },
            Block::Table(t) => {
                table_block_to_elements(t, &mut elements);
            }
            Block::Blockquote(bq) => {
                let runs = inlines_to_runs_colored(&bq.children, &BLOCKQUOTE_COLOR);
                let mut p = Paragraph::from_runs(runs, TextAlign::Left, None);
                p.indent_left_mm = 8.0;
                p.indent_right_mm = 8.0;
                elements.push(Box::new(p));
                if let Some(ref attr) = bq.attribution {
                    let run = TextRun {
                        text: format!("— {attr}"),
                        style: AppliedStyle {
                            italic: true,
                            color: Some("#787878".into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    };
                    let mut attr_para = Paragraph::from_runs(vec![run], TextAlign::Right, None);
                    attr_para.indent_right_mm = 8.0;
                    elements.push(Box::new(attr_para));
                }
            }
            Block::CodeBlock(cb) => {
                let run = TextRun {
                    text: cb.code.clone(),
                    style: AppliedStyle {
                        code: true,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                elements.push(Box::new(Paragraph::from_runs(
                    vec![run],
                    TextAlign::Left,
                    None,
                )));
            }
            Block::Image(img) => elements.push(image_block_to_element(img)),
            Block::HorizontalRule => {
                elements.push(Box::new(HorizontalRuleElement));
            }
            Block::PageBreak => elements.push(Box::new(PageBreakElement)),
            Block::FixedBox(fb) => elements.push(fixed_box_to_element(fb)),
        }
    }

    elements
}

// ── Block helpers ─────────────────────────────────────────────────────────────

fn heading_to_section(h: &HeadingBlock) -> Box<dyn Element> {
    let text = inlines_to_text(&h.children);
    let level = h.level.clamp(1, 3);
    Box::new(Section::new(text, level))
}

fn paragraph_block_to_element(p: &ParagraphBlock) -> Box<dyn Element> {
    let runs = inlines_to_runs(&p.children);
    let alignment = convert_alignment(p.alignment.as_ref());
    let mut para = Paragraph::from_runs(runs, alignment, None);
    if let Some(ref name) = p.style {
        para.style_ref = Some(name.clone());
    }
    if let Some(level) = p.indent {
        para.indent_left_mm = level as f64 * 10.0;
    }
    Box::new(para)
}

/// Converts an NCRTF table block, prepending a caption paragraph if present.
fn table_block_to_elements(t: &TableBlock, out: &mut Vec<Box<dyn Element>>) {
    // Caption → small italic paragraph above the table.
    if let Some(ref cap) = t.caption {
        let run = TextRun {
            text: cap.clone(),
            style: AppliedStyle {
                italic: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut cap_para = Paragraph::from_runs(vec![run], TextAlign::Left, None);
        cap_para.space_after_mm = Some(1.0);
        out.push(Box::new(cap_para));
    }

    let mut builder = Table::builder().table_style(TableStyle::grid());

    for hrow in &t.head {
        let cells: Vec<TableCell> = hrow
            .cells
            .iter()
            .map(|c| ncrtf_cell_to_table_cell(c, true))
            .collect();
        builder = builder.header_row(cells);
    }

    for brow in &t.body {
        let cells: Vec<TableCell> = brow
            .cells
            .iter()
            .map(|c| ncrtf_cell_to_table_cell(c, false))
            .collect();
        builder = builder.row(cells);
    }

    if let Some(widths) = &t.col_widths {
        builder = builder.col_widths(widths.clone());
    }

    out.push(Box::new(builder.build()));
}

fn ncrtf_cell_to_table_cell(
    c: &crate::richtext::model::TableCell,
    force_header: bool,
) -> TableCell {
    let text = inlines_to_text(&c.children);
    let is_header = c.header || force_header;
    let alignment = convert_alignment(c.alignment.as_ref());
    let col_span = c.col_span.unwrap_or(1).max(1) as u16;
    let row_span = c.row_span.unwrap_or(1).max(1) as u16;

    let mut cell = TableCell::new(text)
        .align(alignment)
        .col_span(col_span)
        .row_span(row_span);
    if is_header {
        cell.style_ref = Some("table_header".into());
    }
    cell
}

fn image_block_to_element(img: &ImageBlock) -> Box<dyn Element> {
    let data = decode_data_uri(&img.src);

    let alignment = match img.alignment.as_ref().unwrap_or(&ImageAlign::Center) {
        ImageAlign::Left => ImageAlignment::Left,
        ImageAlign::Center => ImageAlignment::Center,
        ImageAlign::Right => ImageAlignment::Right,
    };

    let mut element = ImageElement::new(data).align(alignment);
    if let Some(cap) = &img.caption {
        element = element.caption(cap.to_owned());
    }
    if let Some(alt) = &img.alt {
        element = element.alt(alt.to_owned());
    }
    if let Some(pct) = img.width_percent {
        element.width_percent = Some(pct);
    }
    Box::new(element)
}

// ── Inline helpers ────────────────────────────────────────────────────────────

/// Extract plain text from a slice of inline nodes.
pub fn inlines_to_text(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .map(|i| match i {
            Inline::Text(t) => t.text.clone(),
            Inline::Link(l) => inlines_to_text(&l.children),
            Inline::HardBreak => "\n".to_string(),
            Inline::FootnoteRef(r) => r.number.to_string(),
        })
        .collect()
}

/// Convert a slice of inline nodes into `TextRun`s.
pub fn inlines_to_runs(inlines: &[Inline]) -> Vec<TextRun> {
    inlines_to_runs_colored(inlines, &RgbColor::new(0.0, 0.0, 0.0))
}

/// Convert inline nodes to `TextRun`s, overriding the text color with `base_color`
/// for nodes that don't already carry an explicit `color` mark.
fn inlines_to_runs_colored(inlines: &[Inline], base_color: &RgbColor) -> Vec<TextRun> {
    let is_black = base_color.r == 0.0 && base_color.g == 0.0 && base_color.b == 0.0;
    let color_hex = if is_black {
        None
    } else {
        Some(format!(
            "#{:02X}{:02X}{:02X}",
            (base_color.r * 255.0) as u8,
            (base_color.g * 255.0) as u8,
            (base_color.b * 255.0) as u8,
        ))
    };

    let mut runs = Vec::new();
    for inline in inlines {
        match inline {
            Inline::Text(t) => {
                let marks: &[MarkValue] = t.marks.as_deref().unwrap_or(&[]);
                let mut style = AppliedStyle::from(marks);
                // Apply base_color only when the node has no explicit color mark.
                if style.color.is_none() {
                    style.color = color_hex.clone();
                }
                runs.push(TextRun {
                    text: t.text.clone(),
                    style,
                    opentype: t.opentype_features.clone().unwrap_or_default(),
                    ..Default::default()
                });
            }
            Inline::Link(l) => {
                // Links: underline + dark navy, recursively convert children.
                for mut run in inlines_to_runs(&l.children) {
                    run.style.underline = true;
                    if run.style.color.is_none() {
                        run.style.color = Some(format!(
                            "#{:02X}{:02X}{:02X}",
                            (LINK_COLOR.r * 255.0) as u8,
                            (LINK_COLOR.g * 255.0) as u8,
                            (LINK_COLOR.b * 255.0) as u8,
                        ));
                    }
                    runs.push(run);
                }
            }
            Inline::HardBreak => {
                runs.push(TextRun {
                    text: "\n".into(),
                    ..Default::default()
                });
            }
            Inline::FootnoteRef(r) => {
                runs.push(TextRun::footnote_ref(r.number));
            }
        }
    }
    runs
}

fn convert_alignment(a: Option<&TextAlign>) -> TextAlign {
    match a {
        Some(align) => *align,
        None => TextAlign::Left,
    }
}

fn fixed_box_to_element(fb: &FixedBoxBlock) -> Box<dyn Element> {
    let overflow = match fb.overflow.as_deref() {
        Some("clip") => OverflowPolicy::Clip,
        Some("shrink") => OverflowPolicy::Shrink,
        Some("overflow") => OverflowPolicy::Overflow,
        _ => OverflowPolicy::Truncate,
    };

    let border = fb.border.as_ref().map(|b| {
        let style = match b.style.as_deref() {
            Some("dashed") => BorderStyle::Dashed,
            Some("dotted") => BorderStyle::Dotted,
            _ => BorderStyle::Solid,
        };
        BoxBorder {
            width_mm: b.width_mm,
            color: RgbColor::from_hex(&b.color).unwrap_or(RgbColor::new(0.0, 0.0, 0.0)),
            style,
        }
    });

    let background = fb.background.as_deref().and_then(RgbColor::from_hex);

    Box::new(FixedTextBox {
        text_box: FixedBox {
            x_mm: fb.x_mm,
            y_mm: fb.y_mm,
            width_mm: fb.width_mm,
            height_mm: fb.height_mm,
            overflow,
            border,
            background,
            padding_mm: fb.padding_mm.unwrap_or(2.0),
            z_index: 0,
            ua_role: None,
            ua_alt: None,
        },
        content: ParagraphContent::Runs(inlines_to_runs(&fb.children)),
        alignment: convert_alignment(fb.alignment.as_ref()),
        font_size: None,
        vertical_align: VerticalAlign::Top,
    })
}

// ── Image decoding ────────────────────────────────────────────────────────────

/// Decode a `data:image/...;base64,<payload>` URI.
/// Returns an empty vec for asset references or malformed URIs.
fn decode_data_uri(src: &str) -> Vec<u8> {
    if let Some(pos) = src.find(";base64,") {
        let payload = &src[pos + 8..];
        base64::engine::general_purpose::STANDARD
            .decode(payload.trim())
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}
