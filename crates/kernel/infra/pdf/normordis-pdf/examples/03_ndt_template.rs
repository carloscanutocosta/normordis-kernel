//! Document built from an NDT template with runtime data.
//! Run: cargo run --example 03_ndt_template -p normordis-pdf

use normordis_pdf::*;

const TEMPLATE: &str = include_str!("templates/relatorio-simples.ndt.json");

fn main() -> Result<()> {
    let data = serde_json::json!({
        "ndt_data": "1.0.0",
        "data": {
            "entity_name": "Câmara Municipal de Exemplo",
            "document_title": "Relatório Anual 2025",
            "document_date": "25 de Abril de 2026",
            "reference": "REL/2026/001",
            "author": "Divisão de Planeamento",
            "body_content": "{\"ncrtf\":\"1.0\",\"blocks\":[{\"type\":\"paragraph\",\"alignment\":\"justify\",\"children\":[{\"type\":\"text\",\"text\":\"Conteúdo do relatório gerado em runtime.\",\"marks\":[]}]}]}"
        }
    })
    .to_string();

    let pdf = DocumentBuilder::new("Relatório")
        .push_ndt(TEMPLATE, &data)?
        .render_to_bytes()?;

    let out = std::env::temp_dir().join("normaxis_ndt.pdf");
    std::fs::write(&out, &pdf)?;
    println!("PDF gerado: {} ({} bytes)", out.display(), pdf.len());
    Ok(())
}
