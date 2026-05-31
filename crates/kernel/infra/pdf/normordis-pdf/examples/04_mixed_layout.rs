//! Document combining Flow and Fixed Box elements (office letter style).
//! Run: cargo run --example 04_mixed_layout -p normordis-pdf

use normordis_pdf::*;

fn main() -> Result<()> {
    let pdf = DocumentBuilder::new("Ofício")
        // Fixed elements — do not affect the flow cursor
        .fixed_text(
            FixedBox {
                x_mm: 20.0,
                y_mm: 257.0,
                width_mm: 120.0,
                height_mm: 8.0,
                ..Default::default()
            },
            "CÂMARA MUNICIPAL DE EXEMPLO",
            TextAlign::Left,
        )
        .fixed_line(
            20.0,
            252.0,
            190.0,
            252.0,
            RgbColor {
                r: 0.0,
                g: 0.2,
                b: 0.6,
            },
        )
        .fixed_text(
            FixedBox {
                x_mm: 110.0,
                y_mm: 185.0,
                width_mm: 80.0,
                height_mm: 7.0,
                ..Default::default()
            },
            "João Silva",
            TextAlign::Left,
        )
        .fixed_text(
            FixedBox {
                x_mm: 110.0,
                y_mm: 178.0,
                width_mm: 80.0,
                height_mm: 6.0,
                ..Default::default()
            },
            "Rua de Exemplo, 42",
            TextAlign::Left,
        )
        // Flow content
        .push(Spacer::new(50.0))
        .push(Section::new("Assunto: Teste de layout misto", 2))
        .push(Spacer::new(6.0))
        .push(Paragraph::new("Exmo.(a) Senhor(a),"))
        .push(Spacer::new(4.0))
        .push(
            Paragraph::new(
                "Em resposta à vossa comunicação, informamos que o pedido foi \
                 recebido e encontra-se em análise pelos serviços competentes.",
            )
            .align(TextAlign::Justify),
        )
        .render_to_bytes()?;

    let out = std::env::temp_dir().join("normaxis_mixed.pdf");
    std::fs::write(&out, &pdf)?;
    println!("PDF gerado: {} ({} bytes)", out.display(), pdf.len());
    Ok(())
}
