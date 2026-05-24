/// Example 08 — Portuguese text spacing validation
///
/// Reproduces typical content from miniapp-declaracao-consentimento-pf.
/// Open the output PDF and verify: words must be separated by visible spaces;
/// Portuguese characters (ã, ç, é, ó, ú, â) must render correctly.
use normordis_pdf::{
    AppliedStyle, BulletList, DocumentBuilder, ListItemElement, OrderedList, Paragraph, Section,
    Spacer, TextAlign, TextRun,
};

fn bold_run(text: &str) -> TextRun {
    TextRun {
        text: text.into(),
        style: AppliedStyle {
            bold: true,
            ..Default::default()
        },
        letter_spacing_mm: 0.0,
        ..Default::default()
    }
}
fn plain_run(text: &str) -> TextRun {
    TextRun::plain(text)
}

fn main() {
    let pdf = DocumentBuilder::new("Espaçamento Português — Libertinus Serif")
        // Validate space in plain left-aligned text
        .push(Paragraph::new("Classificação documental: 350.10.100").font_size(9.0))
        .push(Spacer::new(4.0))
        // Centred bold title with Portuguese characters
        .push(Paragraph::from_runs(
            vec![bold_run(
                "Declaração de Consentimento Informado para Acesso e \
                 Utilização de Credenciais no Portal das Finanças",
            )],
            TextAlign::Center,
            Some(12.0),
        ))
        .push(Spacer::new(6.0))
        // Justified mixed bold/plain body — key stress test for word spacing
        .push(Paragraph::from_runs(
            vec![
                plain_run("Eu, "),
                bold_run("João António Ferreira da Silva"),
                plain_run(", titular do Número de Identificação Fiscal "),
                bold_run("123 456 789"),
                plain_run(", residente em "),
                bold_run("Rua das Acácias, n.º 12, 1100-001 Lisboa"),
                plain_run(
                    ", na qualidade de titular dos dados pessoais e para efeitos do \
                     disposto no Regulamento (UE) 2016/679 (RGPD), bem como da \
                     legislação fiscal aplicável, declaro prestar o meu consentimento \
                     expresso ao funcionário da Repartição de Finanças de Lisboa.",
                ),
            ],
            TextAlign::Justify,
            None,
        ))
        .push(Spacer::new(4.0))
        // Ordered list with Portuguese text
        .push(OrderedList::new(vec![ListItemElement {
            indent: 0,
            runs: vec![
                plain_run(
                    "Preencher, validar e submeter a minha declaração de rendimentos (IRS) \
                     referente ao ano fiscal de ",
                ),
                bold_run("2024"),
                plain_run(";"),
            ],
        }]))
        .push(Spacer::new(6.0))
        // Section heading
        .push(Section::new("Cláusulas de Limitação e Responsabilidade", 2))
        // Bullet list — tests spacing across lines
        .push(BulletList::new(vec![
            ListItemElement::plain(
                "O presente consentimento é prestado ao abrigo do Artigo 6.º, n.º 1, \
                 alínea a) do RGPD, sendo o tratamento de dados pautado pelos princípios \
                 da licitude, lealdade, transparência e limitação das finalidades.",
            ),
            ListItemElement::plain(
                "O profissional compromete-se a assegurar a confidencialidade, integridade \
                 e segurança dos dados pessoais tratados, nos termos do RGPD.",
            ),
            ListItemElement::plain(
                "O consentimento ora prestado é limitado no tempo, extinguindo-se \
                 automaticamente com a submissão da declaração de IRS.",
            ),
        ]))
        .push(Spacer::new(6.0))
        // Final paragraph — right margin validation
        .push(
            Paragraph::new(
                "Afirmo que compreendi integralmente o conteúdo, alcance e riscos \
                 associados à presente autorização, prestando o meu consentimento de \
                 forma livre, específica, informada e inequívoca, conforme exigido pelo \
                 Regulamento Geral sobre a Proteção de Dados.",
            )
            .align(TextAlign::Justify),
        )
        .push(Spacer::new(12.0))
        // Alphabet check row for all Portuguese characters
        .push(Section::new("Verificação de Caracteres Portugueses", 2))
        .push(Paragraph::new(
            "Maiúsculas: À Á Â Ã Ä Ç È É Ê Ë Ì Í Î Ï Ò Ó Ô Õ Ö Ù Ú Û Ü",
        ))
        .push(Paragraph::new(
            "Minúsculas: à á â ã ä ç è é ê ë ì í î ï ò ó ô õ ö ù ú û ü",
        ))
        .render_to_bytes()
        .expect("render deve funcionar");

    let out = "crates/normordis-pdf/examples/output/08_portuguese_spacing.pdf";
    std::fs::create_dir_all("crates/normordis-pdf/examples/output").unwrap();
    std::fs::write(out, &pdf).expect("write OK");
    println!("Written {} bytes to {out}", pdf.len());
}
