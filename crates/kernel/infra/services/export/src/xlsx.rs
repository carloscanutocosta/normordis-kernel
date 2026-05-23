use std::fs::{self, File};
use std::io::{Seek, Write};

use zip::write::SimpleFileOptions;

use crate::csv::value_to_cell;
use crate::xml::escape_text;
use crate::{ExportRequest, Result};

pub struct XlsxExportAdapter;

impl XlsxExportAdapter {
    pub fn export(req: &ExportRequest) -> Result<()> {
        if let Some(parent) = req.output_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let file = File::create(&req.output_path)?;
        let mut zip = zip::ZipWriter::new(file);
        write_xlsx_package(&mut zip, req)?;
        zip.finish()?;
        Ok(())
    }
}

fn write_xlsx_package<W: Write + Seek>(
    zip: &mut zip::ZipWriter<W>,
    req: &ExportRequest,
) -> Result<()> {
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    write_file(zip, options, "[Content_Types].xml", CONTENT_TYPES)?;
    write_file(zip, options, "_rels/.rels", RELS)?;
    write_file(
        zip,
        options,
        "xl/workbook.xml",
        &workbook_xml(req.sheet_name()),
    )?;
    write_file(zip, options, "xl/_rels/workbook.xml.rels", WORKBOOK_RELS)?;
    write_file(zip, options, "xl/worksheets/sheet1.xml", &sheet_xml(req))?;
    Ok(())
}

fn write_file<W: Write + Seek>(
    zip: &mut zip::ZipWriter<W>,
    options: SimpleFileOptions,
    path: &str,
    content: &str,
) -> Result<()> {
    zip.start_file(path, options)?;
    zip.write_all(content.as_bytes())?;
    Ok(())
}

fn workbook_xml(sheet_name: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="{}" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>"#,
        escape_text(&sheet_name.chars().take(31).collect::<String>())
    )
}

fn sheet_xml(req: &ExportRequest) -> String {
    let columns = req.effective_columns();
    let rows = req.effective_rows();
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
"#,
    );

    xml.push_str("    <row r=\"1\">");
    for (idx, column) in columns.iter().enumerate() {
        write_cell(&mut xml, idx + 1, 1, column);
    }
    xml.push_str("</row>\n");

    for (row_idx, row) in rows.iter().enumerate() {
        let excel_row = row_idx + 2;
        xml.push_str(&format!("    <row r=\"{excel_row}\">"));
        for (col_idx, column) in columns.iter().enumerate() {
            write_cell(
                &mut xml,
                col_idx + 1,
                excel_row,
                &value_to_cell(row.get(column)),
            );
        }
        xml.push_str("</row>\n");
    }

    xml.push_str("  </sheetData>\n</worksheet>\n");
    xml
}

fn write_cell(xml: &mut String, col: usize, row: usize, value: &str) {
    xml.push_str(&format!(
        "<c r=\"{}{}\" t=\"inlineStr\"><is><t>{}</t></is></c>",
        column_name(col),
        row,
        escape_text(value)
    ));
}

fn column_name(mut col: usize) -> String {
    let mut name = String::new();
    while col > 0 {
        col -= 1;
        name.insert(0, (b'A' + (col % 26) as u8) as char);
        col /= 26;
    }
    name
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
</Types>"#;

const RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#;

const WORKBOOK_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#;
