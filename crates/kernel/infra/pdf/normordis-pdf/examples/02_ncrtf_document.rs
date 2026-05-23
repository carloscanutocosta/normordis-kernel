//! Document built from NCRTF rich text JSON.
//! Run: cargo run --example 02_ncrtf_document -p normordis-pdf

use normordis_pdf::*;

const NCRTF_CONTENT: &str = r#"{
  "ncrtf": "1.0",
  "blocks": [
    {
      "type": "heading",
      "level": 1,
      "children": [{"type": "text", "text": "Título do Relatório", "marks": []}]
    },
    {
      "type": "paragraph",
      "alignment": "justify",
      "children": [
        {"type": "text", "text": "Texto com ", "marks": []},
        {"type": "text", "text": "negrito", "marks": ["bold"]},
        {"type": "text", "text": " e ", "marks": []},
        {"type": "text", "text": "itálico", "marks": ["italic"]},
        {"type": "text", "text": " no mesmo parágrafo.", "marks": []}
      ]
    },
    {
      "type": "list",
      "list_type": "bullet",
      "children": [
        {"indent": 0, "children": [{"type": "text", "text": "Primeiro item", "marks": []}]},
        {"indent": 0, "children": [{"type": "text", "text": "Segundo item", "marks": []}]}
      ]
    }
  ]
}"#;

fn main() -> Result<()> {
    let pdf = DocumentBuilder::new("NCRTF Document")
        .push_ncrtf(NCRTF_CONTENT)?
        .render_to_bytes()?;

    let out = std::env::temp_dir().join("normaxis_ncrtf.pdf");
    std::fs::write(&out, &pdf)?;
    println!("PDF gerado: {} ({} bytes)", out.display(), pdf.len());
    Ok(())
}
