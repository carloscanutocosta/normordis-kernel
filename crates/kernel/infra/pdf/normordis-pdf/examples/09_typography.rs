/// Example 09 — v1.4.0 Typography showcase
///
/// Demonstrates: rustybuzz shaping, TextDecoration, OpenTypeFeatures,
/// HighlightColor, ParagraphBorder, SectionBreak, LineBreakingMode.
use normordis_pdf::{
    AppliedStyle, DecorationLine, DocumentBuilder, HighlightColor, Paragraph, ParagraphBorder,
    RgbColor, SectionBreak, SectionMargins, TextAlign, TextDecoration, TextRun,
};

fn main() {
    let out_dir = std::path::Path::new("crates/normordis-pdf/examples/output");
    std::fs::create_dir_all(out_dir).ok();

    let pdf = DocumentBuilder::new("normordis-pdf v1.4.0 — Typography")
        // ── Section 1: Plain paragraph baseline ───────────────────────────────
        .push(Paragraph::new("1. Baseline paragraph with rustybuzz shaping.").bold())
        .push(Paragraph::new(
            "Libertinus Serif is now shaped via rustybuzz (HarfBuzz in Rust), \
             enabling GPOS kerning, standard ligatures (fi, fl, ff), and \
             OpenType feature overrides per text run. The advance-width \
             computation is based on actual glyph metrics rather than a \
             heuristic character-width estimate.",
        ))
        // ── Section 2: Text decoration ────────────────────────────────────────
        .push(Paragraph::new("2. Text decoration.").bold())
        .push(Paragraph::from_runs(
            vec![
                TextRun {
                    text: "Normal, ".into(),
                    ..Default::default()
                },
                TextRun {
                    text: "underlined, ".into(),
                    style: AppliedStyle::default(),
                    decoration: TextDecoration {
                        underline: Some(DecorationLine::simple()),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                TextRun {
                    text: "strikethrough, ".into(),
                    style: AppliedStyle::default(),
                    decoration: TextDecoration {
                        strikethrough: Some(DecorationLine::simple()),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                TextRun {
                    text: "highlighted yellow".into(),
                    style: AppliedStyle::default(),
                    decoration: TextDecoration {
                        highlight: Some(HighlightColor::Yellow),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                TextRun {
                    text: ".".into(),
                    ..Default::default()
                },
            ],
            TextAlign::Left,
            Some(11.0),
        ))
        // ── Section 3: Paragraph background and border ────────────────────────
        .push(Paragraph::new("3. Paragraph background and border.").bold())
        .push(
            Paragraph::new("This paragraph has a light-blue background fill.")
                .background(RgbColor {
                    r: 0.85,
                    g: 0.92,
                    b: 1.0,
                })
                .align(TextAlign::Left),
        )
        .push(
            Paragraph::new("This paragraph has a box border with default padding.")
                .border(ParagraphBorder::box_border(DecorationLine::simple()))
                .align(TextAlign::Left),
        )
        .push(
            Paragraph::new("Background AND border — useful for callout boxes.")
                .background(RgbColor {
                    r: 1.0,
                    g: 0.97,
                    b: 0.85,
                })
                .border(ParagraphBorder::box_border(DecorationLine::with_color(
                    RgbColor {
                        r: 0.8,
                        g: 0.6,
                        b: 0.0,
                    },
                )))
                .align(TextAlign::Left),
        )
        // ── Section 4: Section break ──────────────────────────────────────────
        .push(Paragraph::new("4. Section break (portrait → new page).").bold())
        .push(Paragraph::new(
            "Content before the section break. The next section starts on a new page.",
        ))
        .push(SectionBreak::portrait().with_margins(SectionMargins::symmetric(20.0, 20.0)))
        // ── After section break ───────────────────────────────────────────────
        .push(Paragraph::new("After the section break — new page.").bold())
        .push(Paragraph::new(
            "This content appears on the page started by the SectionBreak element. \
             The page margins were narrowed to 20 mm on all sides for this section.",
        ));

    match pdf.render_to_bytes() {
        Ok(bytes) => {
            let path = out_dir.join("09_typography.pdf");
            std::fs::write(&path, bytes).expect("write 09_typography.pdf");
            println!("Written: {}", path.display());
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}
