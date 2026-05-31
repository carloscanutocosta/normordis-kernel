use serde_json::Value;

use super::{
    data::NdtData,
    model::{BodyElement, NdtDocument},
    resolver, TemplateError,
};
use crate::{
    elements::{
        fixed_image::{FixedImageBox, ImageFit},
        fixed_line::FixedLineElement as FixedLine,
        fixed_text::{FixedTextBox, VerticalAlign},
        image::ImageElement,
        list::{BulletList, ListItemElement, OrderedList},
        page_break::PageBreakElement,
        paragraph::{Paragraph, ParagraphContent, TextRun},
        section::Section,
        spacer::Spacer,
        table::Table,
        Element,
    },
    layout::{BorderStyle, BoxBorder, FixedBox, OverflowPolicy, TextAlign},
    richtext::{self},
    styles::{DocumentStyle, RgbColor},
};

/// Convert an `NdtDocument` + `NdtData` into a flat list of renderable elements.
pub fn render_template(
    doc: &NdtDocument,
    data: &NdtData,
    style: &DocumentStyle,
) -> Result<Vec<Box<dyn Element>>, TemplateError> {
    render_body(&doc.body, doc, data, style)
}

fn render_body(
    body: &[BodyElement],
    doc: &NdtDocument,
    data: &NdtData,
    style: &DocumentStyle,
) -> Result<Vec<Box<dyn Element>>, TemplateError> {
    let mut elements: Vec<Box<dyn Element>> = Vec::new();

    for item in body {
        match item {
            BodyElement::Paragraph(p) => {
                let text = resolver::resolve_string(&p.text, data);
                let alignment = parse_alignment(p.alignment.as_deref());
                let mut para = Paragraph::new(text).align(alignment);
                if p.bold.unwrap_or(false) {
                    para = para.bold();
                }
                if p.italic.unwrap_or(false) {
                    para = para.italic();
                }
                if let Some(fs) = p.font_size {
                    para = para.font_size(fs);
                }
                if let Some(indent) = p.indent_mm {
                    para.indent_left_mm = indent;
                }
                elements.push(Box::new(para));
            }

            BodyElement::Heading(h) => {
                let text = resolver::resolve_string(&h.text, data);
                let level = h.level.unwrap_or(1).clamp(1, 3);
                elements.push(Box::new(Section::new(text, level)));
            }

            BodyElement::RichText(rt) => {
                let json = if rt.source.as_deref() == Some("placeholder") {
                    match resolver::resolve_value(&rt.content, data) {
                        Some(Value::String(s)) => s,
                        _ => resolver::resolve_string(&rt.content, data),
                    }
                } else {
                    resolver::resolve_string(&rt.content, data)
                };

                let ncrtf_doc = richtext::parse_ncrtf(&json)
                    .map_err(|e| TemplateError::RenderError(e.to_string()))?;
                let mut els = richtext::ncrtf_to_elements(&ncrtf_doc, style);
                elements.append(&mut els);
            }

            BodyElement::Table(t) => {
                let headers: Vec<String> = t
                    .headers
                    .as_deref()
                    .unwrap_or(&[])
                    .iter()
                    .map(|h| resolver::resolve_string(h, data))
                    .collect();
                let rows: Vec<crate::elements::table::TableRow> = t
                    .rows
                    .as_deref()
                    .unwrap_or(&[])
                    .iter()
                    .map(|row| {
                        let cells: Vec<String> = row
                            .iter()
                            .map(|c| resolver::resolve_string(c, data))
                            .collect();
                        crate::elements::table::TableRow::plain(cells)
                    })
                    .collect();
                let mut table = Table::new(headers, rows);
                if let Some(widths) = &t.col_widths {
                    table = table.col_widths(widths.clone());
                }
                elements.push(Box::new(table));
            }

            BodyElement::List(l) => {
                let list_type = l.list_type.as_deref().unwrap_or("bullet");
                let items: Vec<ListItemElement> = l
                    .items
                    .iter()
                    .map(|text| ListItemElement {
                        indent: 0,
                        runs: vec![TextRun::plain(resolver::resolve_string(text, data))],
                    })
                    .collect();
                match list_type {
                    "ordered" => elements.push(Box::new(OrderedList { start: 1, items })),
                    _ => elements.push(Box::new(BulletList { items })),
                }
            }

            BodyElement::Image(_img) => {
                // TODO: decode base64/asset src
                elements.push(Box::new(ImageElement::new(vec![])));
            }

            BodyElement::Spacer(s) => {
                elements.push(Box::new(Spacer::new(s.height_mm)));
            }

            BodyElement::HorizontalRule => {
                elements.push(Box::new(Spacer::new(2.0)));
            }

            BodyElement::PageBreak => {
                elements.push(Box::new(PageBreakElement));
            }

            BodyElement::FixedText(ft) => {
                let text = resolver::resolve_string(&ft.text, data);
                let overflow = parse_overflow(ft.overflow.as_deref());
                let alignment = parse_alignment(ft.alignment.as_deref());
                elements.push(Box::new(FixedTextBox {
                    text_box: FixedBox {
                        x_mm: ft.x_mm,
                        y_mm: ft.y_mm,
                        width_mm: ft.width_mm,
                        height_mm: ft.height_mm,
                        overflow,
                        border: None,
                        background: None,
                        padding_mm: ft.padding_mm.unwrap_or(2.0),
                        z_index: 0,
                        ua_role: None,
                        ua_alt: None,
                    },
                    content: ParagraphContent::Plain(text),
                    alignment,
                    font_size: ft.font_size,
                    vertical_align: VerticalAlign::Top,
                }));
            }

            BodyElement::FixedImage(fi) => {
                elements.push(Box::new(FixedImageBox {
                    image_box: FixedBox {
                        x_mm: fi.x_mm,
                        y_mm: fi.y_mm,
                        width_mm: fi.width_mm,
                        height_mm: fi.height_mm,
                        overflow: OverflowPolicy::Truncate,
                        border: None,
                        background: None,
                        padding_mm: 0.0,
                        z_index: 0,
                        ua_role: None,
                        ua_alt: None,
                    },
                    data: vec![], // TODO: decode src
                    fit: ImageFit::Contain,
                }));
            }

            BodyElement::FixedLine(fl) => {
                let color = fl
                    .color
                    .as_deref()
                    .and_then(RgbColor::from_hex)
                    .unwrap_or_else(|| RgbColor::new(0.0, 0.0, 0.0));
                elements.push(Box::new(FixedLine::new(
                    fl.x1_mm, fl.y1_mm, fl.x2_mm, fl.y2_mm, color,
                )));
            }

            BodyElement::FixedBox(fb) => {
                let text = resolver::resolve_string(fb.content.as_deref().unwrap_or(""), data);
                let overflow = parse_overflow(fb.overflow.as_deref());
                let alignment = parse_alignment(fb.alignment.as_deref());
                let border = fb
                    .border_color
                    .as_deref()
                    .and_then(RgbColor::from_hex)
                    .map(|c| BoxBorder {
                        width_mm: fb.border_width_mm.unwrap_or(0.3),
                        color: c,
                        style: BorderStyle::Solid,
                    });
                let background = fb.background.as_deref().and_then(RgbColor::from_hex);
                elements.push(Box::new(FixedTextBox {
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
                    content: ParagraphContent::Plain(text),
                    alignment,
                    font_size: None,
                    vertical_align: VerticalAlign::Top,
                }));
            }

            BodyElement::ZoneRef(zr) => {
                let zones = doc
                    .zones
                    .as_ref()
                    .ok_or_else(|| TemplateError::ZoneNotFound {
                        name: zr.zone.clone(),
                    })?;
                let zone = zones
                    .get(&zr.zone)
                    .ok_or_else(|| TemplateError::ZoneNotFound {
                        name: zr.zone.clone(),
                    })?;
                let mut zone_els = render_body(&zone.elements, doc, data, style)?;
                elements.append(&mut zone_els);
            }

            BodyElement::Conditional(cond) => {
                let branch = if evaluate_condition(
                    &cond.condition,
                    cond.operator.as_deref(),
                    &cond.value,
                    data,
                ) {
                    &cond.then
                } else {
                    &cond.else_branch
                };
                let mut branch_els = render_body(branch, doc, data, style)?;
                elements.append(&mut branch_els);
            }

            BodyElement::Repeat(rep) => {
                let items_key = rep.items.trim_matches(|c| c == '{' || c == '}');
                let items_value = data.get(items_key);
                let item_var = rep.item_var.as_deref().unwrap_or("item");

                if let Some(Value::Array(arr)) = items_value {
                    for item_val in arr {
                        let mut item_data = data.clone();
                        item_data
                            .data
                            .insert(item_var.to_string(), item_val.clone());

                        // Flatten nested object fields under item_var prefix
                        if let Value::Object(map) = item_val {
                            for (k, v) in map {
                                item_data.data.insert(format!("{item_var}.{k}"), v.clone());
                            }
                        }

                        let mut iter_els = render_body(&rep.elements, doc, &item_data, style)?;
                        elements.append(&mut iter_els);
                    }
                }
            }

            BodyElement::Include(_inc) => {
                // TODO: v0.8.0 — load external NDT file and render recursively
            }

            BodyElement::FootnoteRef(fref) => {
                use crate::elements::footnote::{FootnoteMarkStyle, FootnoteRef};
                let style = match fref.mark_style.as_deref() {
                    Some("alpha") => FootnoteMarkStyle::Alpha,
                    Some("symbol") => FootnoteMarkStyle::Symbol,
                    _ => FootnoteMarkStyle::Numeric,
                };
                elements.push(Box::new(FootnoteRef::new(fref.number).with_style(style)));
            }

            BodyElement::Toc(toc_el) => {
                use crate::elements::toc::TableOfContents;
                let mut toc = TableOfContents::new();
                if let Some(ref t) = toc_el.title {
                    toc = toc.title(t.clone());
                }
                if let Some(lvl) = toc_el.max_level {
                    toc = toc.max_level(lvl);
                }
                if let Some(ref lc) = toc_el.leader_char {
                    if let Some(c) = lc.chars().next() {
                        toc = toc.dot_leader(c);
                    }
                }
                elements.push(Box::new(toc));
            }

            BodyElement::AcroformField(af) => {
                use crate::elements::form::{
                    CheckBoxDef, ComboBoxDef, FieldRect, FormField, TextFieldDef,
                };
                let rect = FieldRect {
                    x_mm: af.rect.x_mm,
                    y_mm: af.rect.y_mm,
                    width_mm: af.rect.width_mm,
                    height_mm: af.rect.height_mm,
                };
                let field = match af.field_type.as_str() {
                    "check_box" => FormField::CheckBox(CheckBoxDef {
                        name: af.name.clone(),
                        checked_by_default: af.checked_by_default.unwrap_or(false),
                        tooltip: af.tooltip.clone(),
                        rect,
                    }),
                    "combo_box" => FormField::ComboBox(ComboBoxDef {
                        name: af.name.clone(),
                        options: af.options.clone().unwrap_or_default(),
                        default_value: None,
                        editable: false,
                        tooltip: af.tooltip.clone(),
                        rect,
                        font_size: af.font_size.unwrap_or(11.0),
                    }),
                    _ => FormField::TextField(TextFieldDef {
                        name: af.name.clone(),
                        default_value: None,
                        tooltip: af.tooltip.clone(),
                        multiline: false,
                        max_length: af.max_length,
                        readonly: false,
                        required: af.required.unwrap_or(false),
                        rect,
                        font_size: af.font_size.unwrap_or(11.0),
                    }),
                };
                elements.push(Box::new(field));
            }
        }
    }

    Ok(elements)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_alignment(s: Option<&str>) -> TextAlign {
    match s {
        Some("center") => TextAlign::Center,
        Some("justify") => TextAlign::Justify,
        _ => TextAlign::Left,
    }
}

fn parse_overflow(s: Option<&str>) -> OverflowPolicy {
    match s {
        Some("clip") => OverflowPolicy::Clip,
        Some("shrink") => OverflowPolicy::Shrink,
        Some("overflow") => OverflowPolicy::Overflow,
        _ => OverflowPolicy::Truncate,
    }
}

fn evaluate_condition(
    key: &str,
    operator: Option<&str>,
    expected: &Option<Value>,
    data: &NdtData,
) -> bool {
    let plain_key = key.trim_matches(|c| c == '{' || c == '}');
    let actual = data.get(plain_key);

    match operator.unwrap_or("exists") {
        "exists" => actual.is_some(),
        "empty" => {
            actual.is_none()
                || actual.is_none_or(|v| match v {
                    Value::String(s) => s.is_empty(),
                    Value::Array(a) => a.is_empty(),
                    Value::Null => true,
                    _ => false,
                })
        }
        "eq" => {
            if let (Some(a), Some(e)) = (actual, expected.as_ref()) {
                values_equal(a, e)
            } else {
                false
            }
        }
        "neq" => {
            if let (Some(a), Some(e)) = (actual, expected.as_ref()) {
                !values_equal(a, e)
            } else {
                actual.is_none()
            }
        }
        "gt" => {
            if let (Some(a), Some(e)) = (
                actual.and_then(|v| v.as_f64()),
                expected.as_ref().and_then(|v| v.as_f64()),
            ) {
                a > e
            } else {
                false
            }
        }
        "lt" => {
            if let (Some(a), Some(e)) = (
                actual.and_then(|v| v.as_f64()),
                expected.as_ref().and_then(|v| v.as_f64()),
            ) {
                a < e
            } else {
                false
            }
        }
        _ => false,
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(s1), Value::String(s2)) => s1 == s2,
        (Value::Bool(b1), Value::Bool(b2)) => b1 == b2,
        (Value::Number(n1), Value::Number(n2)) => n1.as_f64() == n2.as_f64(),
        _ => a == b,
    }
}
