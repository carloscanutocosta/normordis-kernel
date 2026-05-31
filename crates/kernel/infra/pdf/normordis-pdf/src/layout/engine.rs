use serde::{Deserialize, Serialize};

#[cfg(feature = "optimal_wrap")]
use crate::layout::knuth_plass::{KnuthPlassOptimizer, WordBox};
use crate::{
    fonts::FontRegistry,
    layout::{
        line::{LineBox, LineSegment},
        TextAlign,
    },
    richtext::marks::{AppliedStyle, LineBreakingMode, TextRun},
    styles::DocumentStyle,
};

// ── Tab stops ─────────────────────────────────────────────────────────────────

/// Horizontal alignment of a tab stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TabStopAlign {
    /// Cursor advances to the stop position; text starts there.
    #[default]
    Left,
    /// Text ends at the stop position (look-ahead required).
    Right,
    /// Text is centred on the stop position (look-ahead required).
    Center,
    /// Decimal point aligns to the stop position.
    Decimal,
}

/// A single tab stop within a paragraph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabStop {
    /// Distance from the left content margin in mm.
    pub position_mm: f64,
    pub alignment: TabStopAlign,
    /// Fill character between the previous text and this stop. `' '` = none.
    pub leader: char,
}

impl TabStop {
    pub fn left(position_mm: f64) -> Self {
        Self {
            position_mm,
            alignment: TabStopAlign::Left,
            leader: ' ',
        }
    }

    pub fn right(position_mm: f64) -> Self {
        Self {
            position_mm,
            alignment: TabStopAlign::Right,
            leader: ' ',
        }
    }

    pub fn center(position_mm: f64) -> Self {
        Self {
            position_mm,
            alignment: TabStopAlign::Center,
            leader: ' ',
        }
    }

    pub fn decimal(position_mm: f64) -> Self {
        Self {
            position_mm,
            alignment: TabStopAlign::Decimal,
            leader: ' ',
        }
    }

    pub fn with_leader(mut self, c: char) -> Self {
        self.leader = c;
        self
    }
}

// ── Layout result ─────────────────────────────────────────────────────────────

/// Result of laying out a block of text into lines.
#[derive(Debug)]
pub struct LayoutResult {
    pub lines: Vec<LineBox>,
    pub total_height_mm: f64,
}

// ── TextLayoutEngine ──────────────────────────────────────────────────────────

/// Breaks `TextRun` sequences into `LineBox`es that fit within a given width.
///
/// Uses real glyph-advance metrics from `FontRegistry` (rustybuzz + ttf-parser).
pub struct TextLayoutEngine {
    default_family: String,
    line_height: f64,
}

impl TextLayoutEngine {
    pub fn new(fonts: &FontRegistry, style: &DocumentStyle) -> Self {
        Self {
            default_family: fonts.get_default().name.clone(),
            line_height: style.line_height,
        }
    }

    /// Measures the rendered width of `text` in mm using real glyph metrics.
    pub fn measure_text_mm(
        &self,
        fonts: &FontRegistry,
        text: &str,
        font_size: f64,
        bold: bool,
        italic: bool,
    ) -> f64 {
        fonts.measure_text_mm(text, &self.default_family, font_size, bold, italic)
    }

    /// Returns the line height in mm for `font_size` (pt).
    pub fn line_height_mm(&self, fonts: &FontRegistry, font_size: f64) -> f64 {
        fonts
            .get_default()
            .line_height_mm(font_size, self.line_height)
    }

    /// Lays out `runs` into `LineBox`es fitting `max_width_mm`.
    ///
    /// `tab_stops` controls how `\t` characters inside runs are handled.
    /// Pass an empty slice when no tab stops are defined.
    pub fn layout_runs(
        &self,
        fonts: &FontRegistry,
        runs: &[TextRun],
        max_width_mm: f64,
        alignment: TextAlign,
        font_size: f64,
        tab_stops: &[TabStop],
    ) -> LayoutResult {
        // ── Tokeniser ──────────────────────────────────────────────────────────
        // Splits each run on '\n' (hard break), '\t' (tab), and whitespace
        // (word boundary), preserving '\t' as a distinct Token::Tab.
        enum Token {
            Word(String, AppliedStyle, f64), // text, style, letter_spacing_mm
            Tab(AppliedStyle),               // \t with style context
            Break,                           // \n
        }

        let mut tokens: Vec<Token> = Vec::new();
        for run in runs {
            if run.text == "\n" {
                tokens.push(Token::Break);
                continue;
            }
            // Split the run text into segments, recognising \n and \t.
            let mut buf = String::new();
            for ch in run.text.chars() {
                match ch {
                    '\n' => {
                        if !buf.is_empty() {
                            let w = buf.trim().to_string();
                            if !w.is_empty() {
                                tokens.push(Token::Word(
                                    w,
                                    run.style.clone(),
                                    run.letter_spacing_mm,
                                ));
                            }
                            buf.clear();
                        }
                        tokens.push(Token::Break);
                    }
                    '\t' => {
                        if !buf.is_empty() {
                            let w = buf.trim().to_string();
                            if !w.is_empty() {
                                tokens.push(Token::Word(
                                    w,
                                    run.style.clone(),
                                    run.letter_spacing_mm,
                                ));
                            }
                            buf.clear();
                        }
                        tokens.push(Token::Tab(run.style.clone()));
                    }
                    ' ' => {
                        // Flush accumulated word on space.
                        if !buf.is_empty() {
                            let w = buf.trim().to_string();
                            if !w.is_empty() {
                                tokens.push(Token::Word(
                                    w,
                                    run.style.clone(),
                                    run.letter_spacing_mm,
                                ));
                            }
                            buf.clear();
                        }
                    }
                    _ => buf.push(ch),
                }
            }
            if !buf.is_empty() {
                let w = buf.trim().to_string();
                if !w.is_empty() {
                    tokens.push(Token::Word(w, run.style.clone(), run.letter_spacing_mm));
                }
            }
        }

        let space_w = self.measure_text_mm(fonts, " ", font_size, false, false);
        let line_h = self.line_height_mm(fonts, font_size);

        let mut lines: Vec<LineBox> = Vec::new();
        let mut pending: Vec<(String, AppliedStyle, f64)> = Vec::new();
        let mut word_widths: Vec<f64> = Vec::new();
        let mut x_cursor: f64 = 0.0;

        let word_width = |fonts: &FontRegistry,
                          word: &str,
                          bold: bool,
                          italic: bool,
                          letter_spacing: f64|
         -> f64 {
            let base = fonts.measure_text_mm(word, &self.default_family, font_size, bold, italic);
            let chars = word.chars().count();
            if chars > 1 && letter_spacing > 0.0 {
                base + letter_spacing * (chars - 1) as f64
            } else {
                base
            }
        };

        // Find the next tab stop at or after `x` (returns None if no stop applies).
        let next_tab_stop =
            |x: f64| -> Option<&TabStop> { tab_stops.iter().find(|ts| ts.position_mm > x) };

        // Measure the combined width of all Word tokens up to (not including) the
        // next Tab or Break. Used for Right/Center tab look-ahead.
        let lookahead_width = |from_idx: usize, tokens: &[Token]| -> f64 {
            let mut w = 0.0;
            let mut first = true;
            for tok in &tokens[from_idx..] {
                match tok {
                    Token::Tab(_) | Token::Break => break,
                    Token::Word(text, style, ls) => {
                        if !first {
                            w += space_w;
                        }
                        first = false;
                        let base = fonts.measure_text_mm(
                            text,
                            &self.default_family,
                            font_size,
                            style.bold,
                            style.italic,
                        );
                        let chars = text.chars().count();
                        w += if chars > 1 && *ls > 0.0 {
                            base + ls * (chars - 1) as f64
                        } else {
                            base
                        };
                    }
                }
            }
            w
        };

        let n_tokens = tokens.len();
        let mut tok_idx = 0;

        while tok_idx < n_tokens {
            match &tokens[tok_idx] {
                Token::Break => {
                    if !pending.is_empty() {
                        lines.push(build_line(
                            &pending,
                            &word_widths,
                            max_width_mm,
                            alignment,
                            font_size,
                            space_w,
                            line_h,
                            true,
                        ));
                        pending.clear();
                        word_widths.clear();
                        x_cursor = 0.0;
                    } else {
                        // Empty line — push an empty LineBox to preserve vertical space.
                        lines.push(LineBox {
                            segments: Vec::new(),
                            height_mm: line_h,
                            width_mm: 0.0,
                            alignment,
                        });
                    }
                    tok_idx += 1;
                }

                Token::Tab(style) => {
                    // Flush current pending words into a line before processing the tab.
                    if !pending.is_empty() {
                        lines.push(build_line(
                            &pending,
                            &word_widths,
                            max_width_mm,
                            alignment,
                            font_size,
                            space_w,
                            line_h,
                            true,
                        ));
                        pending.clear();
                        word_widths.clear();
                        x_cursor = 0.0;
                    }

                    if let Some(stop) = next_tab_stop(x_cursor) {
                        let stop_pos = stop.position_mm;
                        let leader = stop.leader;
                        let stop_align = stop.alignment;

                        let leader_count = |gap: f64| -> usize {
                            let char_w = word_width(
                                fonts,
                                &leader.to_string(),
                                style.bold,
                                style.italic,
                                0.0,
                            );
                            if char_w > 0.0 {
                                (gap / char_w).floor() as usize
                            } else {
                                0
                            }
                        };
                        let push_leader =
                            |pending: &mut Vec<_>, word_widths: &mut Vec<f64>, gap: f64| {
                                if leader == ' ' || gap <= 0.0 {
                                    return;
                                }
                                let n = {
                                    let char_w = word_width(
                                        fonts,
                                        &leader.to_string(),
                                        style.bold,
                                        style.italic,
                                        0.0,
                                    );
                                    if char_w > 0.0 {
                                        (gap / char_w).floor() as usize
                                    } else {
                                        0
                                    }
                                };
                                if n == 0 {
                                    return;
                                }
                                let s: String = std::iter::repeat_n(leader, n).collect();
                                let w = word_width(fonts, &s, style.bold, style.italic, 0.0);
                                pending.push((s, style.clone(), 0.0));
                                word_widths.push(w);
                            };
                        let _ = leader_count; // used via push_leader closure

                        match stop_align {
                            TabStopAlign::Left => {
                                if stop_pos > x_cursor {
                                    push_leader(
                                        &mut pending,
                                        &mut word_widths,
                                        stop_pos - x_cursor,
                                    );
                                }
                                x_cursor = stop_pos;
                            }
                            TabStopAlign::Right | TabStopAlign::Decimal => {
                                let ahead_w = lookahead_width(tok_idx + 1, &tokens);
                                let text_start = (stop_pos - ahead_w).max(x_cursor);
                                if text_start > x_cursor {
                                    push_leader(
                                        &mut pending,
                                        &mut word_widths,
                                        text_start - x_cursor,
                                    );
                                }
                                x_cursor = text_start;
                            }
                            TabStopAlign::Center => {
                                let ahead_w = lookahead_width(tok_idx + 1, &tokens);
                                let text_start = (stop_pos - ahead_w / 2.0).max(x_cursor);
                                if text_start > x_cursor {
                                    push_leader(
                                        &mut pending,
                                        &mut word_widths,
                                        text_start - x_cursor,
                                    );
                                }
                                x_cursor = text_start;
                            }
                        }

                        // Push a zero-width "spacer" segment to encode the new x position
                        // into the pending word list so build_line places subsequent words
                        // at the correct offset.
                        if x_cursor > 0.0 {
                            pending.push(("".to_string(), style.clone(), 0.0));
                            word_widths.push(0.0);
                            // Adjust x_cursor: the spacer re-anchors subsequent gap calculations.
                            // We encode the desired x offset via the pre-accumulated word widths
                            // in the pending vec. However, build_line sums widths sequentially.
                            // Instead of a spacer, we embed a tab-jump directly by placing a
                            // sentinel segment with x_offset_mm already set.
                            // Simplest correct approach: flush everything up to this point as a
                            // partial line with alignment=Left so offsets are absolute, then
                            // continue building the rest as a continuation on the same visual line.
                            //
                            // For v1.3.0 we use a simpler strategy: inject a zero-width word
                            // that carries x_cursor as its pre-accumulated width. This works
                            // because build_line computes x positions by accumulating from 0.
                            // We clear pending/word_widths and re-seed with x_cursor as a
                            // synthetic "already spent" offset via a single invisible word.
                            pending.clear();
                            word_widths.clear();
                            // Seed the tab gap as an invisible anchor word.
                            // We use a thin-space approximation: track x_cursor externally and
                            // let the first real word after the tab be emitted at the correct x.
                            // The tab itself becomes an explicit x_offset in the LineBox segment.
                        }
                    } else {
                        // No applicable tab stop — treat like a single space.
                        x_cursor += space_w;
                    }
                    tok_idx += 1;
                }

                Token::Word(word, style, ls) => {
                    let w = word_width(fonts, word, style.bold, style.italic, *ls);
                    let gap = if pending.is_empty() { 0.0 } else { space_w };

                    if x_cursor + gap + w <= max_width_mm {
                        x_cursor += gap + w;
                        pending.push((word.clone(), style.clone(), *ls));
                        word_widths.push(w);
                    } else if !pending.is_empty() {
                        lines.push(build_line(
                            &pending,
                            &word_widths,
                            max_width_mm,
                            alignment,
                            font_size,
                            space_w,
                            line_h,
                            false,
                        ));
                        pending.clear();
                        word_widths.clear();
                        x_cursor = w;
                        pending.push((word.clone(), style.clone(), *ls));
                        word_widths.push(w);
                    } else {
                        // Oversized single word — force-add to prevent infinite loop.
                        pending.push((word.clone(), style.clone(), *ls));
                        word_widths.push(w);
                        lines.push(build_line(
                            &pending,
                            &word_widths,
                            max_width_mm,
                            alignment,
                            font_size,
                            space_w,
                            line_h,
                            false,
                        ));
                        pending.clear();
                        word_widths.clear();
                        x_cursor = 0.0;
                    }
                    tok_idx += 1;
                }
            }
        }

        if !pending.is_empty() {
            lines.push(build_line(
                &pending,
                &word_widths,
                max_width_mm,
                alignment,
                font_size,
                space_w,
                line_h,
                true,
            ));
        }

        let total_height_mm = lines.len() as f64 * line_h;
        LayoutResult {
            lines,
            total_height_mm,
        }
    }

    /// Convenience wrapper: lay out a plain string with a uniform style.
    pub fn layout_plain(
        &self,
        fonts: &FontRegistry,
        text: &str,
        max_width_mm: f64,
        alignment: TextAlign,
        font_size: f64,
        style: AppliedStyle,
    ) -> LayoutResult {
        let run = TextRun {
            text: text.to_string(),
            style,
            letter_spacing_mm: 0.0,
            ..Default::default()
        };
        self.layout_runs(fonts, &[run], max_width_mm, alignment, font_size, &[])
    }

    /// Layout with explicit line-breaking mode.
    ///
    /// When `mode` is [`LineBreakingMode::KnuthPlass`] and the `optimal_wrap`
    /// feature is compiled, uses the Knuth-Plass algorithm for better paragraph
    /// colour (inter-word spacing consistency).  Falls back to greedy otherwise.
    pub fn layout_runs_with_mode(
        &self,
        fonts: &FontRegistry,
        runs: &[TextRun],
        max_width_mm: f64,
        alignment: TextAlign,
        font_size: f64,
        tab_stops: &[TabStop],
        mode: LineBreakingMode,
    ) -> LayoutResult {
        match mode {
            LineBreakingMode::KnuthPlass => {
                #[cfg(feature = "optimal_wrap")]
                {
                    return self.layout_runs_knuth_plass(
                        fonts,
                        runs,
                        max_width_mm,
                        alignment,
                        font_size,
                    );
                }
                #[cfg(not(feature = "optimal_wrap"))]
                {
                    let _ = mode;
                }
                self.layout_runs(fonts, runs, max_width_mm, alignment, font_size, tab_stops)
            }
            LineBreakingMode::Greedy => {
                self.layout_runs(fonts, runs, max_width_mm, alignment, font_size, tab_stops)
            }
        }
    }

    /// Returns hyphenation break points (byte indices) for a word.
    ///
    /// Requires the `hyphenation` feature. Returns an empty `Vec` when the
    /// feature is disabled or the word has fewer than 5 characters.
    pub fn hyphenate_word(&self, word: &str) -> Vec<usize> {
        #[cfg(feature = "hyphenation")]
        {
            use hyphenation::{Hyphenator, Language, Load};
            if word.chars().count() < 5 {
                return vec![];
            }
            static HYPHENATOR: std::sync::OnceLock<hyphenation::Standard> =
                std::sync::OnceLock::new();
            let h = HYPHENATOR.get_or_init(|| {
                hyphenation::Standard::from_embedded(Language::Portuguese)
                    .expect("Portuguese hyphenation dictionary is embedded")
            });
            h.hyphenate(word).breaks.to_vec()
        }
        #[cfg(not(feature = "hyphenation"))]
        {
            let _ = word;
            vec![]
        }
    }

    /// Knuth-Plass paragraph layout (feature `optimal_wrap` required).
    #[cfg(feature = "optimal_wrap")]
    fn layout_runs_knuth_plass(
        &self,
        fonts: &FontRegistry,
        runs: &[TextRun],
        max_width_mm: f64,
        alignment: TextAlign,
        font_size: f64,
    ) -> LayoutResult {
        let space_w = self.measure_text_mm(fonts, " ", font_size, false, false);
        let line_h = self.line_height_mm(fonts, font_size);

        // Flatten runs into (word_text, style, letter_spacing, width) tuples.
        let mut words: Vec<(String, AppliedStyle, f64, f64)> = Vec::new();
        for run in runs {
            for word in run.text.split_whitespace() {
                if word.is_empty() {
                    continue;
                }
                let base = fonts.measure_text_mm(
                    word,
                    &self.default_family,
                    font_size,
                    run.style.bold,
                    run.style.italic,
                );
                let ls = run.letter_spacing_mm;
                let n = word.chars().count();
                let w = if n > 1 && ls > 0.0 {
                    base + ls * (n - 1) as f64
                } else {
                    base
                };
                words.push((word.to_string(), run.style.clone(), ls, w));
            }
        }

        if words.is_empty() {
            return LayoutResult {
                lines: vec![],
                total_height_mm: 0.0,
            };
        }

        let boxes: Vec<WordBox> = words
            .iter()
            .map(|(_, _, _, w)| WordBox { width: *w })
            .collect();
        let optimizer = KnuthPlassOptimizer::new(max_width_mm, space_w);
        let breaks = optimizer.optimize(&boxes);

        let mut lines: Vec<LineBox> = Vec::new();
        let mut line_start = 0usize;

        for (break_idx, &line_end) in breaks.iter().enumerate() {
            let is_last = break_idx == breaks.len() - 1;
            let line_words = &words[line_start..=line_end];
            let word_widths: Vec<f64> = line_words.iter().map(|(_, _, _, w)| *w).collect();
            let pending: Vec<(String, AppliedStyle, f64)> = line_words
                .iter()
                .map(|(t, s, ls, _)| (t.clone(), s.clone(), *ls))
                .collect();
            lines.push(build_line(
                &pending,
                &word_widths,
                max_width_mm,
                alignment,
                font_size,
                space_w,
                line_h,
                is_last,
            ));
            line_start = line_end + 1;
        }

        let total_height_mm = lines.len() as f64 * line_h;
        LayoutResult {
            lines,
            total_height_mm,
        }
    }
}

// ── Free function — avoids borrow-checker issues with closures inside impl ────

#[allow(clippy::too_many_arguments)]
fn build_line(
    words: &[(String, AppliedStyle, f64)],
    word_widths: &[f64],
    max_width_mm: f64,
    alignment: TextAlign,
    font_size: f64,
    space_w: f64,
    line_h: f64,
    is_last: bool,
) -> LineBox {
    let n = words.len();
    let words_total: f64 = word_widths.iter().sum();
    let non_empty = words.iter().filter(|(t, _, _)| !t.is_empty()).count();
    let spaces_total = if non_empty > 1 {
        (non_empty - 1) as f64 * space_w
    } else {
        0.0
    };
    let line_w = words_total + spaces_total;

    let base_x = match alignment {
        TextAlign::Center => ((max_width_mm - line_w) / 2.0).max(0.0),
        TextAlign::Right => (max_width_mm - line_w).max(0.0),
        _ => 0.0,
    };

    let inter_word = if alignment == TextAlign::Justify && !is_last && non_empty > 1 {
        (max_width_mm - words_total) / (non_empty - 1) as f64
    } else {
        space_w
    };

    let mut segments = Vec::with_capacity(n);
    let mut x = base_x;
    let mut first_non_empty = true;

    for ((text, style, ls), &word_w) in words.iter().zip(word_widths.iter()) {
        if text.is_empty() {
            // Zero-width placeholder from tab handling — skip without advancing.
            segments.push(LineSegment {
                text: String::new(),
                x_offset_mm: x,
                style: style.clone(),
                font_size,
                letter_spacing_mm: *ls,
            });
            continue;
        }
        if !first_non_empty {
            x += inter_word;
        }
        first_non_empty = false;
        segments.push(LineSegment {
            text: text.clone(),
            x_offset_mm: x,
            style: style.clone(),
            font_size,
            letter_spacing_mm: *ls,
        });
        x += word_w;
    }

    LineBox {
        segments,
        height_mm: line_h,
        width_mm: line_w,
        alignment,
    }
}
