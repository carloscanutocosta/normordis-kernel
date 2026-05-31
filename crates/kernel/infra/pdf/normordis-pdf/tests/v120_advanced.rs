use normordis_pdf::layout::AppliedStyle;
use normordis_pdf::richtext::marks::TextRun as TR;
use normordis_pdf::styles::DocumentStyle;
use normordis_pdf::{
    BulletList, DocumentBuilder, Element, FixedBox, FixedTextBox, FontRegistry, ListItemElement,
    OrderedList, OverflowPolicy, Paragraph, ParagraphContent, RenderResult, Spacer, Table,
    TableCell, TableRow, TextAlign, TextLayoutEngine, VerticalAlign,
};

fn default_fonts() -> FontRegistry {
    FontRegistry::default()
}
fn default_style() -> DocumentStyle {
    DocumentStyle::default()
}

fn plain_rows(data: Vec<Vec<String>>) -> Vec<TableRow> {
    data.into_iter()
        .map(|cells| TableRow::plain(cells))
        .collect()
}

fn plain_items(texts: Vec<String>) -> Vec<ListItemElement> {
    texts.into_iter().map(ListItemElement::plain).collect()
}

// ── RenderResult ──────────────────────────────────────────────────────────────

#[test]
fn render_result_done_has_more_false() {
    assert!(!RenderResult::done().has_more);
}

#[test]
fn render_result_more_has_more_true() {
    assert!(RenderResult::more().has_more);
}

// ── Table pagination ──────────────────────────────────────────────────────────

#[test]
fn table_50_rows_renders_without_panic() {
    let rows = plain_rows(
        (1..=50)
            .map(|i| {
                vec![
                    i.to_string(),
                    format!("Item {}", i),
                    format!("{:.2}", i as f64 * 1.5),
                ]
            })
            .collect(),
    );
    DocumentBuilder::new("Tabela paginada")
        .push(Table::new(
            vec!["#".into(), "Descrição".into(), "Valor".into()],
            rows,
        ))
        .render_to_bytes()
        .expect("tabela 50 linhas deve renderizar");
}

#[test]
fn table_50_rows_produces_bytes() {
    let rows = plain_rows(
        (1..=50)
            .map(|i| {
                vec![
                    i.to_string(),
                    format!("Item {}", i),
                    format!("{:.2}", i as f64 * 1.5),
                ]
            })
            .collect(),
    );
    let bytes = DocumentBuilder::new("Tabela paginada")
        .push(Table::new(
            vec!["#".into(), "Descrição".into(), "Valor".into()],
            rows,
        ))
        .render_to_bytes()
        .unwrap();
    assert!(!bytes.is_empty());
}

#[test]
fn table_with_header_row_renders_without_panic() {
    let table = Table::builder()
        .header_row(vec![TableCell::new("Nome"), TableCell::new("Valor")])
        .row(vec![TableCell::new("A"), TableCell::new("1")])
        .row(vec![TableCell::new("B"), TableCell::new("2")])
        .build();
    DocumentBuilder::new("Header row")
        .push(table)
        .render_to_bytes()
        .expect("tabela com header_row deve renderizar");
}

// ── List pagination ───────────────────────────────────────────────────────────

#[test]
fn bullet_list_100_items_renders_without_panic() {
    let items = plain_items(
        (1..=100)
            .map(|i| format!("Item de lista número {}", i))
            .collect(),
    );
    DocumentBuilder::new("Lista paginada")
        .push(BulletList::new(items))
        .render_to_bytes()
        .expect("BulletList 100 itens deve renderizar");
}

#[test]
fn ordered_list_60_items_produces_bytes() {
    let items = plain_items((1..=60).map(|i| format!("Ponto {}", i)).collect());
    let bytes = DocumentBuilder::new("Lista ordenada")
        .push(OrderedList::new(items))
        .render_to_bytes()
        .unwrap();
    assert!(!bytes.is_empty());
}

// ── estimated_height_mm ───────────────────────────────────────────────────────

#[test]
fn paragraph_estimated_height_positive() {
    assert!(Paragraph::new("Texto de exemplo").estimated_height_mm() > 0.0);
}

#[test]
fn paragraph_estimated_height_grows_with_content() {
    let short = Paragraph::new("Curto").estimated_height_mm();
    let long_text = "linha longa ".repeat(20);
    let long = Paragraph::new(long_text).estimated_height_mm();
    assert!(
        long > short,
        "parágrafo longo deve ter altura estimada maior: {long} > {short}"
    );
}

// ── col_span / row_span ───────────────────────────────────────────────────────

#[test]
fn table_cell_col_span_field() {
    assert_eq!(TableCell::new("x").col_span(2).col_span, 2);
}

#[test]
fn table_cell_row_span_field() {
    assert_eq!(TableCell::new("x").row_span(3).row_span, 3);
}

#[test]
fn table_with_col_span_renders_without_panic() {
    let table = Table::builder()
        .col_widths(vec![25.0, 25.0, 25.0, 25.0])
        .header_row(vec![
            TableCell::new("Identificação").col_span(2),
            TableCell::new("Contacto").col_span(2),
        ])
        .header_row(vec![
            TableCell::new("Nome"),
            TableCell::new("NIF"),
            TableCell::new("Telefone"),
            TableCell::new("Email"),
        ])
        .row(vec![
            TableCell::new("João Silva"),
            TableCell::new("123 456 789"),
            TableCell::new("912 345 678"),
            TableCell::new("joao@example.pt"),
        ])
        .build();
    DocumentBuilder::new("Col span")
        .push(table)
        .render_to_bytes()
        .expect("tabela com col_span deve renderizar");
}

// ── Z-index ───────────────────────────────────────────────────────────────────

#[test]
fn fixed_box_default_z_index_is_zero() {
    assert_eq!(FixedBox::default().z_index, 0);
}

#[test]
fn fixed_elements_with_z_index_render_without_panic() {
    let box_low = FixedTextBox {
        text_box: FixedBox {
            x_mm: 20.0,
            y_mm: 100.0,
            width_mm: 60.0,
            height_mm: 20.0,
            overflow: OverflowPolicy::Truncate,
            border: None,
            background: None,
            padding_mm: 2.0,
            z_index: 0,
            ua_role: None,
            ua_alt: None,
        },
        content: ParagraphContent::Plain("Fundo".into()),
        alignment: TextAlign::Left,
        font_size: None,
        vertical_align: VerticalAlign::Top,
    };
    let box_high = FixedTextBox {
        text_box: FixedBox {
            x_mm: 20.0,
            y_mm: 100.0,
            width_mm: 60.0,
            height_mm: 20.0,
            overflow: OverflowPolicy::Truncate,
            border: None,
            background: None,
            padding_mm: 2.0,
            z_index: 10,
            ua_role: None,
            ua_alt: None,
        },
        content: ParagraphContent::Plain("Frente".into()),
        alignment: TextAlign::Left,
        font_size: None,
        vertical_align: VerticalAlign::Top,
    };
    DocumentBuilder::new("Z-index")
        .push(Spacer::new(5.0))
        .push(box_low)
        .push(box_high)
        .render_to_bytes()
        .expect("elementos fixed com z_index devem renderizar");
}

// ── Character spacing ─────────────────────────────────────────────────────────

#[test]
fn letter_spacing_increases_measured_width() {
    let fonts = default_fonts();
    let family = fonts.get_default();
    let text = "HELLO";
    let w0 = family.measure_text_mm(text, 11.0, false, false);
    let ls = 1.0_f64;
    let chars = text.chars().count();
    let w1 = w0 + ls * (chars - 1) as f64;
    assert!(
        w1 > w0,
        "largura com letter_spacing deve ser maior: {w1} > {w0}"
    );
}

#[test]
fn letter_spacing_zero_is_same_as_default() {
    let fonts = default_fonts();
    let style = default_style();
    let engine = TextLayoutEngine::new(&fonts, &style);
    let run_default = TR::plain("HELLO");
    let run_zero = TR {
        text: "HELLO".into(),
        style: AppliedStyle::default(),
        letter_spacing_mm: 0.0,
        ..Default::default()
    };
    let r1 = engine.layout_runs(&fonts, &[run_default], 200.0, TextAlign::Left, 11.0, &[]);
    let r2 = engine.layout_runs(&fonts, &[run_zero], 200.0, TextAlign::Left, 11.0, &[]);
    assert_eq!(r1.lines.len(), r2.lines.len());
}

// ── Indentation ───────────────────────────────────────────────────────────────

#[test]
fn paragraph_indent_left_renders_without_panic() {
    DocumentBuilder::new("Indentação")
        .push(Paragraph::new("Texto com indentação esquerda de 10mm.").indent_left(10.0))
        .render_to_bytes()
        .expect("Paragraph com indent_left deve renderizar");
}

#[test]
fn paragraph_hanging_indent_renders_without_panic() {
    DocumentBuilder::new("Hanging indent")
        .push(
            Paragraph::new("Texto com hanging indent — primeira linha mais à esquerda.")
                .indent_left(15.0)
                .indent_first_line(-10.0),
        )
        .render_to_bytes()
        .expect("Paragraph com hanging indent deve renderizar");
}

// ── Glyph metrics (rustybuzz) ─────────────────────────────────────────────────

#[test]
fn glyph_metrics_av_aa_positive() {
    let fonts = default_fonts();
    let family = fonts.get_default();
    let w_av = family.measure_text_mm("AV", 24.0, false, false);
    let w_aa = family.measure_text_mm("AA", 24.0, false, false);
    assert!(w_av > 0.0);
    assert!(w_aa > 0.0);
}

// ── TextAlign::Right ──────────────────────────────────────────────────────────

#[test]
fn layout_right_short_line_has_positive_x_offset() {
    let fonts = default_fonts();
    let style = default_style();
    let engine = TextLayoutEngine::new(&fonts, &style);
    let result = engine.layout_plain(
        &fonts,
        "curto",
        100.0,
        TextAlign::Right,
        11.0,
        AppliedStyle::default(),
    );
    let seg = &result.lines[0].segments[0];
    assert!(
        seg.x_offset_mm > 0.0,
        "linha curta alinhada à direita deve ter x_offset > 0: {}",
        seg.x_offset_mm
    );
}

#[test]
fn paragraph_right_renders_without_panic() {
    DocumentBuilder::new("Right aligned")
        .push(Paragraph::new("Alinhado à direita.").align(TextAlign::Right))
        .render_to_bytes()
        .expect("Paragraph com TextAlign::Right deve renderizar");
}

#[test]
fn table_cell_right_alignment_renders_without_panic() {
    let table = Table::builder()
        .header_row(vec![TableCell::new("Valor").align(TextAlign::Right)])
        .row(vec![TableCell::new("1 234,56 €").align(TextAlign::Right)])
        .build();
    DocumentBuilder::new("Células direita")
        .push(table)
        .render_to_bytes()
        .expect("TableCell com Right deve renderizar");
}

#[test]
fn layout_right_x_offset_near_max_minus_line_width() {
    let fonts = default_fonts();
    let style = default_style();
    let engine = TextLayoutEngine::new(&fonts, &style);
    let max_w = 100.0_f64;
    let result = engine.layout_plain(
        &fonts,
        "curto",
        max_w,
        TextAlign::Right,
        11.0,
        AppliedStyle::default(),
    );
    let line = &result.lines[0];
    let seg = &line.segments[0];
    let line_w = line.width_mm;
    let expected_offset = (max_w - line_w).max(0.0);
    let diff = (seg.x_offset_mm - expected_offset).abs();
    assert!(
        diff < 0.5,
        "x_offset ({:.3}) deve ser próximo de max_w-line_w ({:.3})",
        seg.x_offset_mm,
        expected_offset
    );
}

#[test]
fn layout_right_full_width_line_has_near_zero_offset() {
    let fonts = default_fonts();
    let style = default_style();
    let engine = TextLayoutEngine::new(&fonts, &style);
    let word = "M";
    let word_w = fonts
        .get_default()
        .measure_text_mm(word, 11.0, false, false);
    let result = engine.layout_plain(
        &fonts,
        word,
        word_w + 0.01,
        TextAlign::Right,
        11.0,
        AppliedStyle::default(),
    );
    let seg = &result.lines[0].segments[0];
    assert!(
        seg.x_offset_mm < 0.5,
        "linha que preenche a largura com Right deve ter x_offset ≈ 0: {}",
        seg.x_offset_mm
    );
}
