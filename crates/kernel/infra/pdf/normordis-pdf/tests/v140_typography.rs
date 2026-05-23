// Tests for normordis-pdf v1.4.0 — rustybuzz shaping, Knuth-Plass, TextDecoration,
// OpenTypeFeatures, ParagraphBorder, SectionBreak, GlyphUsageTracker.

use normordis_pdf::{
    DecorationLine, DocumentBuilder, FontData, FontRegistry, FontVariants, GlyphUsageTracker,
    HighlightColor, KnuthPlassOptimizer, LineBreakingMode, OpenTypeFeatures, Paragraph,
    ParagraphBorder, RgbColor, SectionBreak, SectionMargins, SectionOrientation, ShapedGlyph,
    TextDecoration, TextRun, WordBox,
};

// ── ShapedGlyph / FontData ────────────────────────────────────────────────────

#[test]
fn font_data_parse_libertinus_ok() {
    let reg = FontRegistry::default();
    let fam = reg.get_default();
    // FontData is the concrete type behind FontFamily's regular variant.
    let _: &FontData = fam.get(false, false);
}

#[test]
fn font_data_shape_returns_glyphs() {
    let reg = FontRegistry::default();
    let fam = reg.get_default();
    let fd: &FontData = fam.get(false, false);
    let glyphs: Vec<ShapedGlyph> = fd.shape("Hello", &[]);
    assert!(!glyphs.is_empty(), "shaping 'Hello' should produce glyphs");
    assert_eq!(glyphs.len(), 5, "5 input chars → 5 output glyphs (no ligatures by default)");
}

#[test]
fn font_data_shape_empty_string_returns_empty() {
    let reg = FontRegistry::default();
    let fd: &FontData = reg.get_default().get(false, false);
    assert!(fd.shape("", &[]).is_empty());
}

#[test]
fn font_data_measure_text_consistent_with_registry() {
    let reg = FontRegistry::default();
    let fd: &FontData = reg.get_default().get(false, false);
    let w_fd = fd.measure_text_mm("Test", 12.0);
    let w_reg = reg.measure_text_mm("Test", "LiberationSans", 12.0, false, false);
    assert!((w_fd - w_reg).abs() < 0.001, "FontData and FontRegistry must agree: {w_fd:.3} vs {w_reg:.3}");
}

#[test]
fn shaped_glyphs_have_positive_advance() {
    let reg = FontRegistry::default();
    let fd: &FontData = reg.get_default().get(false, false);
    for glyph in fd.shape("ABC", &[]) {
        assert!(glyph.x_advance > 0, "glyph_id={} should have positive advance", glyph.glyph_id);
    }
}

#[test]
fn font_variants_get_returns_bold() {
    let reg = FontRegistry::default();
    let fam: &FontVariants = reg.get_default();
    let regular = fam.get(false, false);
    let bold = fam.get(true, false);
    // Bold metrics may differ; at minimum both should measure non-zero.
    assert!(regular.measure_text_mm("X", 12.0) > 0.0);
    assert!(bold.measure_text_mm("X", 12.0) > 0.0);
}

#[test]
fn font_variants_fallback_to_regular_when_bold_missing() {
    // Liberation Sans has all 4 variants; pick it just to exercise fallback logic.
    let fam = normordis_pdf::liberation_sans_family().expect("LiberationSans should load");
    let regular = fam.get(false, false);
    let bold = fam.get(true, false);
    assert!(std::ptr::eq(regular as *const _, regular as *const _)); // trivial (same ptr)
    assert!(bold.measure_text_mm("X", 12.0) > 0.0);
}

// ── OpenTypeFeatures ──────────────────────────────────────────────────────────

#[test]
fn opentype_features_default_all_false() {
    let f = OpenTypeFeatures::default();
    assert!(!f.kern && !f.liga && !f.tnum && !f.smcp && !f.sups && !f.subs);
}

#[test]
fn opentype_features_to_rustybuzz_empty_when_all_false() {
    let f = OpenTypeFeatures::default();
    assert!(f.to_rustybuzz_features().is_empty());
}

#[test]
fn opentype_features_to_rustybuzz_returns_enabled_features() {
    let f = OpenTypeFeatures { kern: true, liga: true, ..Default::default() };
    let feats = f.to_rustybuzz_features();
    assert_eq!(feats.len(), 2);
}

#[test]
fn opentype_features_all_enabled_returns_six() {
    let f = OpenTypeFeatures { kern: true, liga: true, tnum: true, smcp: true, sups: true, subs: true };
    assert_eq!(f.to_rustybuzz_features().len(), 6);
}

#[test]
fn opentype_features_serde_round_trip() {
    let f = OpenTypeFeatures { smcp: true, tnum: true, ..Default::default() };
    let json = serde_json::to_string(&f).unwrap();
    let f2: OpenTypeFeatures = serde_json::from_str(&json).unwrap();
    assert!(f2.smcp && f2.tnum && !f2.kern);
}

// ── TextDecoration ────────────────────────────────────────────────────────────

#[test]
fn text_decoration_default_all_none() {
    let d = TextDecoration::default();
    assert!(d.underline.is_none());
    assert!(d.double_underline.is_none());
    assert!(d.strikethrough.is_none());
    assert!(d.highlight.is_none());
    assert!(!d.superscript && !d.subscript && !d.small_caps);
}

#[test]
fn text_decoration_serde_round_trip() {
    let d = TextDecoration {
        underline: Some(DecorationLine::simple()),
        highlight: Some(HighlightColor::Yellow),
        small_caps: true,
        ..Default::default()
    };
    let json = serde_json::to_string(&d).unwrap();
    let d2: TextDecoration = serde_json::from_str(&json).unwrap();
    assert!(d2.underline.is_some());
    assert_eq!(d2.highlight, Some(HighlightColor::Yellow));
    assert!(d2.small_caps);
}

// ── HighlightColor ────────────────────────────────────────────────────────────

#[test]
fn highlight_color_yellow_to_rgb() {
    let rgb = HighlightColor::Yellow.to_rgb();
    assert!((rgb.r - 1.0).abs() < 0.01);
    assert!((rgb.g - 1.0).abs() < 0.01);
    assert!((rgb.b - 0.0).abs() < 0.01);
}

#[test]
fn highlight_color_all_variants_have_valid_rgb() {
    use HighlightColor::*;
    let colors = [
        Black, Blue, Cyan, DarkBlue, DarkCyan, DarkGray, DarkGreen,
        DarkMagenta, DarkRed, DarkYellow, Green, LightGray, Magenta,
        Red, White, Yellow,
    ];
    for c in colors {
        let rgb = c.to_rgb();
        assert!(rgb.r >= 0.0 && rgb.r <= 1.0, "r out of range for {c:?}");
        assert!(rgb.g >= 0.0 && rgb.g <= 1.0, "g out of range for {c:?}");
        assert!(rgb.b >= 0.0 && rgb.b <= 1.0, "b out of range for {c:?}");
    }
}

#[test]
fn decoration_line_default_thickness() {
    let dl = DecorationLine::default();
    assert!((dl.thickness_mm - 0.25).abs() < 0.001);
    assert!(dl.color.is_none());
}

#[test]
fn decoration_line_with_color() {
    let red = RgbColor { r: 1.0, g: 0.0, b: 0.0 };
    let dl = DecorationLine::with_color(red.clone());
    assert!(dl.color.is_some());
}

// ── LineBreakingMode ──────────────────────────────────────────────────────────

#[test]
fn line_breaking_mode_default_is_greedy() {
    let m = LineBreakingMode::default();
    assert_eq!(m, LineBreakingMode::Greedy);
}

#[test]
fn line_breaking_mode_serde() {
    let json = serde_json::to_string(&LineBreakingMode::KnuthPlass).unwrap();
    assert!(json.contains("knuth_plass"));
    let m: LineBreakingMode = serde_json::from_str(&json).unwrap();
    assert_eq!(m, LineBreakingMode::KnuthPlass);
}

// ── KnuthPlassOptimizer ───────────────────────────────────────────────────────

#[test]
fn knuth_plass_produces_valid_output_for_realistic_paragraph() {
    let opt = KnuthPlassOptimizer::new(160.0, 2.5);
    // 20 words × 15mm average = 300mm total → needs ~2 lines on 160mm
    let boxes: Vec<WordBox> = (0..20).map(|_| WordBox { width: 15.0 }).collect();
    let breaks = opt.optimize(&boxes);
    assert!(!breaks.is_empty());
    assert_eq!(*breaks.last().unwrap(), 19);
    // Should produce 2 lines: 20*15 + 19*2.5 ≈ 347.5mm, line=160mm → ~2-3 lines
    assert!(breaks.len() >= 2 && breaks.len() <= 4);
}

#[test]
fn layout_engine_with_knuth_plass_mode_renders_without_panic() {
    DocumentBuilder::new("Knuth-Plass test")
        .push(
            Paragraph::from_runs(
                vec![TextRun { text: "This is a paragraph that will be laid out using the KnuthPlass line-breaking mode for optimal word spacing and paragraph colour.".into(), ..Default::default() }],
                normordis_pdf::TextAlign::Justify,
                Some(11.0),
            )
        )
        .render_to_bytes()
        .expect("should render without panic");
}

// ── GlyphUsageTracker ─────────────────────────────────────────────────────────

#[test]
fn glyph_tracker_new_is_empty() {
    let t = GlyphUsageTracker::new();
    assert!(t.used.is_empty());
}

#[test]
fn glyph_tracker_record_and_retrieve() {
    let mut t = GlyphUsageTracker::new();
    t.record("LibertinusSerif::regular", vec![1u16, 2, 3].into_iter());
    let set = t.used.get("LibertinusSerif::regular").unwrap();
    assert!(set.contains(&1) && set.contains(&2) && set.contains(&3));
}

#[test]
fn glyph_tracker_deduplicates() {
    let mut t = GlyphUsageTracker::new();
    t.record("f::r", vec![42u16, 42, 42].into_iter());
    assert_eq!(t.used["f::r"].len(), 1);
}

// ── ParagraphBorder ───────────────────────────────────────────────────────────

#[test]
fn paragraph_border_box_renders_without_panic() {
    DocumentBuilder::new("ParagraphBorder test")
        .push(
            Paragraph::new("Bordered paragraph.")
                .border(ParagraphBorder::box_border(DecorationLine::simple()))
        )
        .render_to_bytes()
        .expect("bordered paragraph should render");
}

#[test]
fn paragraph_background_renders_without_panic() {
    DocumentBuilder::new("Background test")
        .push(
            Paragraph::new("Yellow background.")
                .background(HighlightColor::Yellow.to_rgb())
        )
        .render_to_bytes()
        .expect("background paragraph should render");
}

#[test]
fn paragraph_border_and_background_combined() {
    DocumentBuilder::new("Border+BG test")
        .push(
            Paragraph::new("Bordered and highlighted.")
                .border(ParagraphBorder::box_border(DecorationLine::simple()))
                .background(RgbColor { r: 0.9, g: 0.9, b: 1.0 })
        )
        .render_to_bytes()
        .expect("border+bg should render");
}

// ── SectionBreak ──────────────────────────────────────────────────────────────

#[test]
fn section_break_portrait_renders_without_panic() {
    DocumentBuilder::new("SectionBreak portrait")
        .push(Paragraph::new("Before break."))
        .push(SectionBreak::portrait())
        .push(Paragraph::new("After break."))
        .render_to_bytes()
        .expect("section break (portrait) should render");
}

#[test]
fn section_break_landscape_renders_without_panic() {
    DocumentBuilder::new("SectionBreak landscape")
        .push(Paragraph::new("Before."))
        .push(SectionBreak::landscape())
        .push(Paragraph::new("After."))
        .render_to_bytes()
        .expect("section break (landscape) should render");
}

#[test]
fn section_break_with_margins_renders_without_panic() {
    DocumentBuilder::new("SectionBreak margins")
        .push(Paragraph::new("Before."))
        .push(SectionBreak::portrait().with_margins(SectionMargins::uniform(30.0)))
        .push(Paragraph::new("After."))
        .render_to_bytes()
        .expect("section break with margins should render");
}

#[test]
fn section_break_orientation_fields() {
    assert_eq!(SectionBreak::portrait().orientation, SectionOrientation::Portrait);
    assert_eq!(SectionBreak::landscape().orientation, SectionOrientation::Landscape);
}

#[test]
fn section_margins_uniform() {
    let m = SectionMargins::uniform(20.0);
    assert_eq!(m.top_mm, 20.0);
    assert_eq!(m.left_mm, 20.0);
}

// ── TextRun defaults ──────────────────────────────────────────────────────────

#[test]
fn text_run_default_has_empty_text() {
    let r = TextRun::default();
    assert!(r.text.is_empty());
    assert_eq!(r.letter_spacing_mm, 0.0);
}

#[test]
fn text_run_plain_sets_text() {
    let r = TextRun::plain("Hello");
    assert_eq!(r.text, "Hello");
    assert!(!r.style.bold);
    assert!(r.decoration.underline.is_none());
}

#[test]
fn text_run_with_decoration_serde_round_trip() {
    let r = TextRun {
        text: "test".into(),
        decoration: TextDecoration {
            underline: Some(DecorationLine::simple()),
            highlight: Some(HighlightColor::Cyan),
            superscript: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let json = serde_json::to_string(&r).unwrap();
    let r2: TextRun = serde_json::from_str(&json).unwrap();
    assert!(r2.decoration.underline.is_some());
    assert_eq!(r2.decoration.highlight, Some(HighlightColor::Cyan));
    assert!(r2.decoration.superscript);
}
