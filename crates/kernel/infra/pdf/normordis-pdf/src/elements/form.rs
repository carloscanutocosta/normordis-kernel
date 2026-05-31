use serde::{Deserialize, Serialize};

use super::{Element, LayoutMode, RenderContext, RenderResult};
use crate::{
    layout::{FixedBox, OverflowPolicy},
    styles::RgbColor,
};

// ── FieldRect ─────────────────────────────────────────────────────────────────

/// Position and size of a form field on the page (mm from bottom-left corner).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldRect {
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
}

// ── Field definitions ─────────────────────────────────────────────────────────

/// A single-line or multi-line text input field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextFieldDef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    pub multiline: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    pub readonly: bool,
    pub required: bool,
    pub rect: FieldRect,
    pub font_size: f64,
}

/// A boolean checkbox field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckBoxDef {
    pub name: String,
    pub checked_by_default: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    pub rect: FieldRect,
}

/// One option in a radio button group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadioButtonDef {
    /// Group name — all buttons with the same `group_name` are mutually exclusive.
    pub group_name: String,
    /// Value this button represents when selected.
    pub value: String,
    pub selected_by_default: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    pub rect: FieldRect,
}

/// A drop-down selection field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComboBoxDef {
    pub name: String,
    pub options: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    pub editable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    pub rect: FieldRect,
    pub font_size: f64,
}

/// A scrollable list selection field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBoxDef {
    pub name: String,
    pub options: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    pub multi_select: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooltip: Option<String>,
    pub rect: FieldRect,
    pub font_size: f64,
}

// ── FormField ─────────────────────────────────────────────────────────────────

/// An interactive AcroForm field.
///
/// Note: full AcroForm interactivity requires v2.0.0 (pdf-writer). This version
/// renders a visible placeholder rectangle with the field name label.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "field_type", rename_all = "snake_case")]
pub enum FormField {
    TextField(TextFieldDef),
    CheckBox(CheckBoxDef),
    RadioButton(RadioButtonDef),
    ComboBox(ComboBoxDef),
    ListBox(ListBoxDef),
}

impl FormField {
    fn rect(&self) -> &FieldRect {
        match self {
            FormField::TextField(d) => &d.rect,
            FormField::CheckBox(d) => &d.rect,
            FormField::RadioButton(d) => &d.rect,
            FormField::ComboBox(d) => &d.rect,
            FormField::ListBox(d) => &d.rect,
        }
    }

    fn name(&self) -> &str {
        match self {
            FormField::TextField(d) => &d.name,
            FormField::CheckBox(d) => &d.name,
            FormField::RadioButton(d) => &d.group_name,
            FormField::ComboBox(d) => &d.name,
            FormField::ListBox(d) => &d.name,
        }
    }
}

impl Element for FormField {
    fn layout_mode(&self) -> LayoutMode {
        let r = self.rect();
        LayoutMode::Fixed(FixedBox {
            x_mm: r.x_mm,
            y_mm: r.y_mm,
            width_mm: r.width_mm,
            height_mm: r.height_mm,
            z_index: 0,
            overflow: OverflowPolicy::Clip,
            border: None,
            background: None,
            padding_mm: 0.0,
            ua_role: None,
            ua_alt: None,
        })
    }

    fn estimated_height_mm(&self) -> f64 {
        0.0
    }

    fn render(&self, ctx: &mut RenderContext) -> crate::Result<RenderResult> {
        let r = self.rect();
        let name = self.name().to_string();

        let (fill_r, fill_g, fill_b) = match self {
            FormField::TextField(_) | FormField::ComboBox(_) | FormField::ListBox(_) => {
                (0.93_f64, 0.96_f64, 1.0_f64)
            }
            FormField::CheckBox(_) | FormField::RadioButton(_) => (1.0_f64, 1.0_f64, 1.0_f64),
        };

        let fill = RgbColor {
            r: fill_r,
            g: fill_g,
            b: fill_b,
        };
        let stroke = RgbColor {
            r: 0.4,
            g: 0.4,
            b: 0.8,
        };
        ctx.backend.draw_rect_stroked(
            r.x_mm,
            r.y_mm,
            r.width_mm,
            r.height_mm,
            &fill,
            &stroke,
            0.5,
        )?;

        match self {
            FormField::CheckBox(d) if d.checked_by_default => {
                render_checkmark(ctx, r)?;
            }
            FormField::RadioButton(d) if d.selected_by_default => {
                render_radio_dot(ctx, r)?;
            }
            _ => {}
        }

        // Render field name label inside the box.
        if let Some(font_ref) = ctx.get_font_ref(false, false) {
            let label_fs = 7.0_f64;
            let label_color = RgbColor {
                r: 0.3,
                g: 0.3,
                b: 0.6,
            };
            let text_x = r.x_mm + 1.5 * 25.4 / 72.0;
            let text_y = r.y_mm + r.height_mm / 2.0 - label_fs * 25.4 / 72.0 / 2.0;
            ctx.draw_text(&name, text_x, text_y, label_fs, font_ref, &label_color)?;
        }

        Ok(RenderResult::done())
    }
}

fn render_checkmark(ctx: &mut RenderContext, r: &FieldRect) -> crate::Result<()> {
    let cx = r.x_mm + r.width_mm * 0.25;
    let cy = r.y_mm + r.height_mm * 0.5;
    let cr = r.width_mm.min(r.height_mm) * 0.3;
    let color = RgbColor {
        r: 0.0,
        g: 0.5,
        b: 0.0,
    };
    let width_pt = 1.0_f32;
    // Two-segment tick mark
    ctx.backend
        .draw_line(cx - cr * 0.3, cy, cx, cy - cr * 0.5, width_pt, &color)?;
    ctx.backend
        .draw_line(cx, cy - cr * 0.5, cx + cr, cy + cr * 0.8, width_pt, &color)?;
    Ok(())
}

fn render_radio_dot(ctx: &mut RenderContext, r: &FieldRect) -> crate::Result<()> {
    let dot_size = r.width_mm.min(r.height_mm) * 0.4;
    let dx = r.x_mm + r.width_mm / 2.0 - r.width_mm * 0.2;
    let dy = r.y_mm + r.height_mm / 2.0 - r.height_mm * 0.2;
    let color = RgbColor {
        r: 0.2,
        g: 0.2,
        b: 0.8,
    };
    ctx.backend.draw_rect(dx, dy, dot_size, dot_size, &color)?;
    Ok(())
}
