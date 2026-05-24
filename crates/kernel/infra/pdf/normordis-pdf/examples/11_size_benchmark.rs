//! Size benchmark — compares output sizes across compression levels.
//! Run: cargo run --example 11_size_benchmark -p normordis-pdf

use normordis_pdf::{
    elements::footer::PageFooter, elements::header::InstitutionalHeader, CompressionLevel,
    DocumentBuilder, Paragraph, Section, Spacer, Table, TableCell,
};

fn build_document(compression: CompressionLevel) -> normordis_pdf::Result<Vec<u8>> {
    DocumentBuilder::new("Relatório de Teste — Benchmark de Tamanho")
        .compression(compression)
        .header(
            InstitutionalHeader::new("Câmara Municipal de Exemplo", "Relatório Anual")
                .with_reference("REF/2026/001")
                .with_date("30 de Abril de 2026"),
        )
        .footer(
            PageFooter::new()
                .left("REF/2026/001")
                .right("{{page}} / {{total_pages}}"),
        )
        .push(Section::new("1. Introdução", 1))
        .push(Paragraph::new(
            "Este documento serve como benchmark para medir o tamanho de \
             ficheiro gerado pelo normordis-pdf em diferentes configurações \
             de compressão. O objectivo é atingir tamanhos comparáveis \
             aos gerados pelo Typst (~37KB) após a implementação completa \
             de font subsetting na v2.0.0.",
        ))
        .push(Spacer::new(4.0))
        .push(Section::new("2. Tabela de dados", 1))
        .push(
            Table::builder()
                .row(vec![TableCell::new("Campo"), TableCell::new("Valor")])
                .row(vec![
                    TableCell::new("Entidade"),
                    TableCell::new("Câmara Municipal"),
                ])
                .row(vec![
                    TableCell::new("Referência"),
                    TableCell::new("REF/2026/001"),
                ])
                .row(vec![TableCell::new("Data"), TableCell::new("30-04-2026")])
                .row(vec![TableCell::new("Versão"), TableCell::new("1.5.1")])
                .row(vec![
                    TableCell::new("Compressão"),
                    TableCell::new(&format!("{compression:?}")),
                ])
                .build(),
        )
        .push(Spacer::new(4.0))
        .push(Section::new("3. Conclusão", 1))
        .push(Paragraph::new(
            "Com font subsetting completo (v2.0.0) a redução esperada é de \
             ~600KB adicionais, atingindo ~37–50KB por documento — equivalente ao Typst.",
        ))
        .render_to_bytes()
}

fn main() -> normordis_pdf::Result<()> {
    println!("normordis-pdf — Benchmark de Tamanho de Ficheiro v1.5.1");
    println!("{}", "═".repeat(51));
    println!();

    let levels = [
        ("None (sem compressão)", CompressionLevel::None),
        ("Fast (zlib nível 1)", CompressionLevel::Fast),
        ("Default (zlib nível 6)", CompressionLevel::Default),
        ("Best (zlib nível 9)", CompressionLevel::Best),
    ];

    let mut results: Vec<(&str, usize, Vec<u8>)> = Vec::new();

    for (label, level) in &levels {
        let bytes = build_document(*level)?;
        let kb = bytes.len() / 1024;
        println!("{:<30} {:>6} KB", label, kb);
        results.push((label, bytes.len(), bytes));
    }

    let none_size = results[0].1;
    let best_size = results[3].1;
    let reduction_pct = 100 - (best_size * 100 / none_size);

    println!();
    println!("Redução None -> Best: {}%", reduction_pct);
    println!("(Font subsetting em v2.0.0 reduzirá ~600KB adicionais)");
    println!();

    let out_dir = std::env::temp_dir();
    for (label, _size, bytes) in &results {
        let suffix = label.split(' ').next().unwrap_or("out").to_lowercase();
        let path = out_dir.join(format!("normaxis_bench_{suffix}.pdf"));
        std::fs::write(&path, bytes)?;
        println!("Guardado: {} ({} KB)", path.display(), bytes.len() / 1024);
    }

    Ok(())
}
