use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::{Cursor, Read};
use std::path::Path;
use thiserror::Error;
use zip::ZipArchive;

#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("erro de I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("erro ao ler o ficheiro DOCX: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("erro ao analisar XML: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("word/document.xml não encontrado no ficheiro DOCX")]
    MissingDocument,
}

/// Converte um ficheiro `.docx` para código-fonte Typst.
pub fn convert_docx_file(path: impl AsRef<Path>) -> Result<String, ConvertError> {
    let bytes = std::fs::read(path.as_ref())?;
    convert_docx_bytes(&bytes)
}

/// Converte bytes de um `.docx` para código-fonte Typst.
pub fn convert_docx_bytes(bytes: &[u8]) -> Result<String, ConvertError> {
    let xml = extract_document_xml(bytes)?;
    let blocks = parse_blocks(&xml)?;
    Ok(blocks_to_typst(blocks))
}

fn extract_document_xml(bytes: &[u8]) -> Result<String, ConvertError> {
    let cursor = Cursor::new(bytes);
    let mut zip = ZipArchive::new(cursor)?;
    let mut xml_file = zip
        .by_name("word/document.xml")
        .map_err(|_| ConvertError::MissingDocument)?;
    let mut xml = String::new();
    xml_file.read_to_string(&mut xml)?;
    Ok(xml)
}

#[derive(Debug, Clone, PartialEq, Default)]
enum ParaStyle {
    #[default]
    Normal,
    Heading(u8),
    ListBullet,
    ListNumber,
}

#[derive(Debug, Clone)]
struct RunData {
    text: String,
    bold: bool,
}

#[derive(Debug)]
enum Block {
    Para {
        style: ParaStyle,
        runs: Vec<RunData>,
    },
}

fn local_name(full_name: &[u8]) -> &[u8] {
    if let Some(pos) = full_name.iter().rposition(|&b| b == b':') {
        &full_name[pos + 1..]
    } else {
        full_name
    }
}

fn parse_blocks(xml: &str) -> Result<Vec<Block>, quick_xml::Error> {
    let mut reader = Reader::from_str(xml);

    let mut blocks: Vec<Block> = Vec::new();
    let mut in_body = false;
    let mut in_para = false;
    let mut in_run = false;
    let mut in_rpr = false;
    let mut in_ppr = false;
    let mut in_text = false;

    let mut para_style = ParaStyle::default();
    let mut para_runs: Vec<RunData> = Vec::new();
    let mut run_bold = false;
    let mut run_text = String::new();

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let ename = e.name();
                let name = local_name(ename.as_ref());
                match name {
                    b"body" => in_body = true,
                    b"p" if in_body => {
                        in_para = true;
                        para_style = ParaStyle::default();
                        para_runs = Vec::new();
                    }
                    b"pPr" if in_para => in_ppr = true,
                    b"pStyle" if in_ppr => {
                        for attr in e.attributes().flatten() {
                            let key = local_name(attr.key.as_ref());
                            if key == b"val" {
                                let val = std::str::from_utf8(&attr.value).unwrap_or("");
                                para_style = detect_style(val);
                            }
                        }
                    }
                    b"r" if in_para => {
                        in_run = true;
                        run_bold = false;
                        run_text = String::new();
                    }
                    b"rPr" if in_run => in_rpr = true,
                    b"b" if in_rpr => run_bold = true,
                    b"t" if in_run => in_text = true,
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let ename = e.name();
                let name = local_name(ename.as_ref());
                match name {
                    b"pStyle" if in_ppr => {
                        for attr in e.attributes().flatten() {
                            let key = local_name(attr.key.as_ref());
                            if key == b"val" {
                                let val = std::str::from_utf8(&attr.value).unwrap_or("");
                                para_style = detect_style(val);
                            }
                        }
                    }
                    b"b" if in_rpr => run_bold = true,
                    b"br" if in_run => run_text.push('\n'),
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_text {
                    if let Ok(text) = e.unescape() {
                        run_text.push_str(&text);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let ename = e.name();
                let name = local_name(ename.as_ref());
                match name {
                    b"body" => in_body = false,
                    b"p" if in_para => {
                        blocks.push(Block::Para {
                            style: para_style.clone(),
                            runs: std::mem::take(&mut para_runs),
                        });
                        in_para = false;
                    }
                    b"pPr" => in_ppr = false,
                    b"r" if in_run => {
                        let text = std::mem::take(&mut run_text);
                        if !text.is_empty() {
                            para_runs.push(RunData {
                                text,
                                bold: run_bold,
                            });
                        }
                        in_run = false;
                    }
                    b"rPr" => in_rpr = false,
                    b"t" => in_text = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e),
            _ => {}
        }
        buf.clear();
    }

    Ok(blocks)
}

fn detect_style(val: &str) -> ParaStyle {
    match val {
        "Heading1" | "Ttulo1" | "Título1" | "1" => ParaStyle::Heading(1),
        "Heading2" | "Ttulo2" | "Título2" | "2" => ParaStyle::Heading(2),
        "Heading3" | "Ttulo3" | "Título3" | "3" => ParaStyle::Heading(3),
        s if s.starts_with("Heading") => {
            let n = s.trim_start_matches("Heading").parse().unwrap_or(2);
            ParaStyle::Heading(n)
        }
        "ListBullet" | "ListParagraph" | "ListaBullet" => ParaStyle::ListBullet,
        "ListNumber" | "ListaNumerada" => ParaStyle::ListNumber,
        _ => ParaStyle::Normal,
    }
}

fn blocks_to_typst(blocks: Vec<Block>) -> String {
    let mut output = String::new();

    output.push_str("#set document(title: \"Documento convertido de DOCX\")\n");
    output.push_str("#set page(paper: \"a4\", margin: 2.5cm)\n");
    output.push_str("#set text(font: \"Linux Libertine\", size: 11pt, lang: \"pt\")\n");
    output.push_str("#set par(justify: true)\n");
    output.push('\n');

    let mut prev_blank = false;

    for block in blocks {
        let Block::Para { style, runs } = block;

        let text = runs_to_typst_inline(&runs);

        if text.trim().is_empty() {
            if !prev_blank {
                output.push('\n');
                prev_blank = true;
            }
            continue;
        }
        prev_blank = false;

        let line = match style {
            ParaStyle::Heading(1) => format!("= {text}"),
            ParaStyle::Heading(2) => format!("== {text}"),
            ParaStyle::Heading(3) => format!("=== {text}"),
            ParaStyle::Heading(n) => format!("{} {text}", "=".repeat(n as usize)),
            ParaStyle::ListBullet => format!("- {text}"),
            ParaStyle::ListNumber => format!("+ {text}"),
            ParaStyle::Normal => text,
        };

        output.push_str(&line);
        output.push('\n');
    }

    output
}

fn runs_to_typst_inline(runs: &[RunData]) -> String {
    // Merge consecutive runs with same bold state, then emit
    let mut result = String::new();
    let mut i = 0;
    while i < runs.len() {
        let bold = runs[i].bold;
        let mut merged = runs[i].text.clone();
        i += 1;
        while i < runs.len() && runs[i].bold == bold {
            merged.push_str(&runs[i].text);
            i += 1;
        }
        if merged.trim().is_empty() {
            result.push_str(&merged);
        } else if bold {
            result.push('*');
            result.push_str(&merged);
            result.push('*');
        } else {
            result.push_str(&merged);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_minimal_docx_xml() {
        // Minimal valid word/document.xml fragment
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
      <w:r><w:t>Título Principal</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:rPr><w:b/></w:rPr><w:t>Texto em negrito</w:t></w:r>
      <w:r><w:t xml:space="preserve"> texto normal</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:pStyle w:val="ListBullet"/></w:pPr>
      <w:r><w:t>Item de lista</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let blocks = parse_blocks(xml).unwrap();
        let typst = blocks_to_typst(blocks);

        assert!(typst.contains("= Título Principal"));
        assert!(typst.contains("*Texto em negrito*"));
        assert!(typst.contains("- Item de lista"));
    }

    #[test]
    fn preserves_placeholder_markers() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t xml:space="preserve">Eu, </w:t></w:r>
      <w:r><w:rPr><w:b/></w:rPr><w:t>{{nome}}</w:t></w:r>
      <w:r><w:t xml:space="preserve">, NIF {{NIF}}.</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let blocks = parse_blocks(xml).unwrap();
        let typst = blocks_to_typst(blocks);

        assert!(typst.contains("{{nome}}"));
        assert!(typst.contains("{{NIF}}"));
    }
}
