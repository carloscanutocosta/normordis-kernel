use normordis_pdf::{
    fonts::FontRegistry,
    layout::{PageFlow, TextAlign, TextLayoutEngine},
    richtext::marks::AppliedStyle,
    styles::DocumentStyle,
};

fn reg() -> FontRegistry {
    FontRegistry::default()
}

fn engine() -> TextLayoutEngine {
    TextLayoutEngine::new(&reg(), &DocumentStyle::default())
}

// ── 1. measure_text_mm returns a positive value ───────────────────────────────

#[test]
fn measure_single_char_is_positive() {
    let r = reg();
    let e = engine();
    let w = e.measure_text_mm(&r, "A", 12.0, false, false);
    assert!(w > 0.0, "expected positive width, got {w}");
}

// ── 2. Short text fits in one line ────────────────────────────────────────────

#[test]
fn layout_plain_short_text_one_line() {
    let r = reg();
    let e = engine();
    let result = e.layout_plain(&r, "Hello", 200.0, TextAlign::Left, 11.0, AppliedStyle::default());
    assert_eq!(result.lines.len(), 1, "short text should produce exactly 1 line");
}

// ── 3. Long text wraps into multiple lines ────────────────────────────────────

#[test]
fn layout_plain_long_text_wraps() {
    let r = reg();
    let e = engine();
    let text = "The quick brown fox jumps over the lazy dog near the river bank";
    let result = e.layout_plain(&r, text, 30.0, TextAlign::Left, 11.0, AppliedStyle::default());
    assert!(
        result.lines.len() > 1,
        "expected multiple lines for narrow column, got {}",
        result.lines.len()
    );
}

// ── 4. Oversized single word stays on one line (no infinite loop) ─────────────

#[test]
fn layout_plain_oversized_word_does_not_loop() {
    let r = reg();
    let e = engine();
    let result = e.layout_plain(&r, "Supercalifragilistic", 1.0, TextAlign::Left, 11.0, AppliedStyle::default());
    assert_eq!(
        result.lines.len(),
        1,
        "oversized word should produce 1 line, got {}",
        result.lines.len()
    );
}

// ── 5. layout_runs with mixed styles produces correct segments ────────────────

#[test]
fn layout_runs_mixed_styles_segments() {
    use normordis_pdf::richtext::marks::TextRun;

    let r = reg();
    let e = engine();
    let runs = vec![
        TextRun {
            text: "Normal".to_string(),
            style: AppliedStyle::default(),
            letter_spacing_mm: 0.0,
            ..Default::default()
        },
        TextRun {
            text: "Bold".to_string(),
            style: AppliedStyle { bold: true, ..Default::default() },
            letter_spacing_mm: 0.0,
            ..Default::default()
        },
    ];

    let result = e.layout_runs(&r, &runs, 200.0, TextAlign::Left, 11.0, &[]);
    assert!(!result.lines.is_empty(), "should produce at least one line");

    let first_line = &result.lines[0];
    assert_eq!(
        first_line.segments.len(),
        2,
        "expected 2 segments (one per run), got {}",
        first_line.segments.len()
    );
    assert!(first_line.segments[1].style.bold, "second segment should be bold");
}

// ── 6. PageFlow::would_overflow when cursor is below threshold ────────────────

#[test]
fn page_flow_overflow_detected() {
    let style = DocumentStyle::default();
    let mut flow = PageFlow::new(&style);
    flow.cursor_y_mm = style.margin_bottom_mm + 1.0;
    assert!(
        flow.would_overflow(2.0),
        "2mm content should overflow with only 1mm remaining"
    );
}

// ── 7. PageFlow::new_page resets cursor and increments page_number ────────────

#[test]
fn page_flow_new_page_resets() {
    let style = DocumentStyle::default();
    let mut flow = PageFlow::new(&style);
    let initial_page = flow.page_number;
    flow.advance(100.0);
    let y_before = flow.cursor_y_mm;

    flow.new_page();

    assert_eq!(flow.page_number, initial_page + 1, "page_number should increment");
    assert!(
        flow.cursor_y_mm > y_before,
        "cursor_y should reset upward after new_page"
    );
}

// ── 8. Paragraph with long text renders without panic ─────────────────────────

#[test]
fn paragraph_long_text_renders_without_panic() {
    use normordis_pdf::DocumentBuilder;

    let long_text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
        Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
        Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris \
        nisi ut aliquip ex ea commodo consequat.";

    let bytes = DocumentBuilder::new("Long paragraph test")
        .push(normordis_pdf::Paragraph::new(long_text))
        .render_to_bytes()
        .expect("render should succeed");

    assert!(bytes.starts_with(b"%PDF-"), "output must be a valid PDF");
}
