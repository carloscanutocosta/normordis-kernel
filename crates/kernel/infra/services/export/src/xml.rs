use std::fs;

use crate::csv::value_to_cell;
use crate::{ExportRequest, Result};

pub struct XmlExportAdapter;

impl XmlExportAdapter {
    pub fn render(req: &ExportRequest) -> String {
        let columns = req.effective_columns();
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push('<');
        xml.push_str(&xml_name(req.root_name(), "export"));
        xml.push_str(" snapshot_id=\"");
        xml.push_str(&escape_attr(&req.snapshot_id));
        xml.push_str("\">\n");

        for row in req.effective_rows() {
            xml.push_str("  <row>\n");
            for column in &columns {
                let tag = xml_name(column, "field");
                xml.push_str("    <");
                xml.push_str(&tag);
                xml.push('>');
                xml.push_str(&escape_text(&value_to_cell(row.get(column))));
                xml.push_str("</");
                xml.push_str(&tag);
                xml.push_str(">\n");
            }
            xml.push_str("  </row>\n");
        }

        xml.push_str("</");
        xml.push_str(&xml_name(req.root_name(), "export"));
        xml.push_str(">\n");
        xml
    }

    pub fn export(req: &ExportRequest) -> Result<()> {
        if let Some(parent) = req.output_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(&req.output_path, Self::render(req))?;
        Ok(())
    }
}

fn xml_name(raw: &str, fallback: &str) -> String {
    let mut name = String::new();
    for (idx, ch) in raw.trim().chars().enumerate() {
        let valid = ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.';
        if valid && !(idx == 0 && ch.is_ascii_digit()) {
            name.push(ch);
        } else if valid {
            name.push('_');
            name.push(ch);
        } else {
            name.push('_');
        }
    }
    if name.is_empty() {
        fallback.into()
    } else {
        name
    }
}

pub(crate) fn escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(value: &str) -> String {
    escape_text(value)
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
