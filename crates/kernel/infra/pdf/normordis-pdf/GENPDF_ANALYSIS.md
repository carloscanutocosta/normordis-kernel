# genpdf Analysis — Reference for normaxis-pdf

Source studied: `genpdf 0.2.0` (MIT, last updated 2022).
Repository: https://gitlab.com/YuriSizov/genpdf-rs

---

## 1. Text Measurement (`fonts.rs` / `render.rs`)

### Formula (genpdf)

genpdf uses `rusttype`. At font initialisation it computes a normalised scale:

```rust
let glyph_height = (v_metrics.ascent - v_metrics.descent) / units_per_em;
let scale = rusttype::Scale::uniform(glyph_height);
```

For Liberation Sans: ascent ≈ 1854, descent ≈ −434, upm = 2048 →
`glyph_height ≈ 2288/2048 ≈ 1.117`

Then per character:

```rust
let advance_width = glyph.scaled(scale).h_metrics().advance_width;
Mm::from(printpdf::Pt(advance_width * font_size_pt))
```

Full chain for glyph with A font-units advance at font_size_pt:
`width_mm = A/upm * glyph_height * font_size_pt * 25.4/72`

Because `glyph_height > 1.0` for fonts where `ascent − descent > upm`, genpdf
overmeasures by ≈ 11% for Liberation Sans. This is a known limitation of the
genpdf approach (using unnormalised glyph metrics as the scale factor).

### Formula (normaxis-pdf, corrected)

```rust
let scale = ab_glyph::PxScale::from(font_size as f32);
let advance: f32 = chars.map(|c| font.h_advance(font.glyph_id(c))).sum();
advance / 72.0 * 25.4
```

`PxScale::from(font_size)` sets "font_size abstract units per em". Dividing by
72 converts those units (PDF points at 72 pt/inch) to inches, then ×25.4 gives
mm. This matches the canonical PDF formula: `A/upm * font_size_pt * 25.4/72`.

### Bug found and fixed ❌ → ✅

**Previous code used `/96.0` instead of `/72.0`**, undercounting all text widths
by 25% (ratio 72/96 = 0.75). This caused:
- Word overlapping in rendered output (words positioned too close together)
- Incorrect line wrapping (more words per line than visually fit)
- Wrong page-break estimation

**Fix applied in `src/fonts.rs`**: changed both `measure_text_mm` and
`line_height_mm` to divide by `72.0`.

### Kerning ⚠️

genpdf applies pair kerning via `rusttype::pair_kerning`, included in both
width measurement and PDF rendering (`write_positioned_codepoints`).

normaxis-pdf: **no kerning**. For typical institutional documents with body text
this is acceptable (kerning correction < 3% per word pair), but for display
fonts or large-size text it may be noticeable.

### Comparison summary

| Aspect | genpdf | normaxis-pdf | Status |
|---|---|---|---|
| Advance width formula | `A/upm × glyph_h × fs × 25.4/72` | `A/upm × fs × 25.4/72` | ✅ normaxis-pdf is more accurate |
| Divisor for pt→mm | 72 (via `Pt::into()`) | 72 (after fix) | ✅ Equivalent |
| Kerning | Yes (pair kerning) | No | ⚠️ Minor gap |
| Missing glyph fallback | No (renders empty glyph) | No | ✅ Same |
| Bold/italic measurement | Separate font file per variant | Separate `FontVariant` per variant | ✅ Equivalent |

---

## 2. Text Wrapping (`wrap.rs`)

### Algorithm (genpdf)

`Words` iterator — splits each `StyledString` at the **first single space**:

```rust
let n = s.s.find(' ').map(|i| i + 1).unwrap_or_else(|| s.s.len());
```

This yields tokens that **include the trailing space** as part of the word
(e.g. `"hello "`, `"world"`). Width includes the space.

`Wrapper` iterator — greedy line packing:

1. For each word, check `x + word_w > max_width`
2. If overflow: try to hyphenate (optional feature). If no hyphen possible AND
   remaining word still wider than max_width → discard (TODO: error).
3. Flush current line, start new line with the overflowing word.
4. On exhaustion: flush final line.

Lines are yielded as `(Vec<StyledCow>, delta)` where `delta` accounts for
bytes added by hyphen insertion.

Justification is **not** implemented in `Wrapper` — it's the caller's
responsibility (each Element handles its own inter-word spacing).

### Algorithm (normaxis-pdf)

`layout_runs` — greedy word-level packing across `TextRun` boundaries:

1. Tokenise via `split_whitespace()` — strips all whitespace, yields bare words.
2. Measure each word with its run's style (bold/italic).
3. Track `x_cursor`; add `space_w` between consecutive words (measured once).
4. Overflow → flush line, start new line with the overflowing word.
5. Oversized single word → force-add to prevent infinite loop.
6. On last line → `is_last = true` → no justification stretch.
7. Justification in `build_line`: `inter_word = (max_width − words_total) / (n−1)`.

### Comparison

| Aspect | genpdf | normaxis-pdf | Status |
|---|---|---|---|
| Tokenisation | Split at first space (trailing space kept) | `split_whitespace` (stripped) | ✅ Same result for normal text |
| Style per token | Preserved via `StyledStr` | Preserved via `(word, AppliedStyle)` | ✅ Equivalent |
| Greedy vs optimal | Greedy | Greedy | ✅ Same |
| Infinite-loop guard | Implicit (word wider than page → discard) | Explicit force-add | ✅ Both safe |
| Leading/trailing spaces | Preserved in token | Stripped | ⚠️ Edge case: multiple consecutive spaces collapse |
| Justification | Caller's responsibility | Inline in `build_line` | ✅ normaxis-pdf handles it correctly |
| Hyphenation | Optional feature | Not implemented | ⚠️ Minor gap for Portuguese text |

---

## 3. Page Flow

### Cursor model (genpdf)

genpdf does not have a `PageFlow` struct. Instead, each `Element::render` call
receives an `Area` (origin + size, upper-left origin in the element's coordinate
system). The element reports how much height it consumed via `RenderResult.size`
and whether it has more content via `RenderResult.has_more`.

Overflow handling: the `Document` render loop re-calls `element.render` on a
new page when `has_more == true`. Elements must track their own render progress
(e.g., which line they last rendered).

`Area::transform_position` converts from upper-left origin to printpdf's
lower-left origin: `y_printpdf = page_height − y_element`.

### Cursor model (normaxis-pdf)

`PageFlow` tracks `cursor_y_mm` in printpdf coordinates (bottom-left origin,
starts near the top of the page, decreases as content is added).

```rust
pub fn would_overflow(&self, height_mm: f64) -> bool {
    self.cursor_y_mm - height_mm < self.margin_bottom_mm
}
pub fn advance(&mut self, height_mm: f64) {
    self.cursor_y_mm -= height_mm;
}
```

Page breaks are detected in `Document::render_to_bytes` before each flow
element is rendered. Elements do not report `has_more` — they are rendered
in one shot.

### Comparison

| Aspect | genpdf | normaxis-pdf | Status |
|---|---|---|---|
| Origin convention | Upper-left (element space), auto-converted | Lower-left (printpdf native) | ✅ Both correct |
| Overflow detection | Element re-entry (`has_more`) | Pre-check in render loop | ✅ Equivalent for flow elements |
| Multi-page elements | Supported (element tracks own state) | Not supported (single render) | ⚠️ Large tables/lists can't span pages |
| Header/footer injection | Via `PageDecorator` trait | Inline in `render_to_bytes` | ✅ Equivalent |
| Coordinate conversion | `page_height − y` per draw call | Implicit (cursor starts at top) | ✅ Equivalent |

---

## 4. Recommended Patches for normaxis-pdf

### Critical — fix before next release

| # | Issue | File | Fix |
|---|---|---|---|
| C1 | `/96.0` should be `/72.0` in measurement formulas | `src/fonts.rs` | **Done in this session** |
| C2 | `Op::SetTextCursor` is Td (relative) — multiple calls within BT/ET accumulate offsets | `src/elements/paragraph.rs`, `list.rs`, `fixed_text.rs`, `footer.rs` | **Done in this session** |

### Important — v1.1.x

| # | Issue | File | Recommendation |
|---|---|---|---|
| I1 | Multi-page elements: tables and lists that exceed one page are silently truncated | `src/elements/table.rs`, `list.rs` | Detect overflow in `render`, emit page break, continue on new page |
| I2 | No kerning | `src/fonts.rs` | Use `ab_glyph`'s glyph pair metrics; add `kern_between(id_a, id_b)` |

### Minor — v1.2.0+

| # | Issue | File | Recommendation |
|---|---|---|---|
| M1 | `split_whitespace` collapses multiple consecutive spaces | `src/layout/engine.rs` | Use single-space split, preserve trailing space widths |
| M2 | No hyphenation | `src/layout/engine.rs` | Add optional `hyphenation` crate integration (feature flag) |
| M3 | `estimated_height_mm` hardcoded to `10.0` in `Paragraph` | `src/elements/paragraph.rs` | Use actual `layout_runs` height with current content width |

---

## 5. Algorithms Worth Porting

### `Area::text_section` — single text cursor per BT/ET block

genpdf uses ONE `begin_text_section` per text block and then uses
`add_line_break()` (PDF `T*`) to advance to the next line. This keeps the PDF
stream compact and avoids the relative-Td accumulation problem.

For normaxis-pdf: the current fix (one BT/ET per segment) is correct and safe.
A further optimisation would be to group consecutive same-style segments on the
same line under one BT/ET and use `Tj`/`TJ` for the whole run, switching fonts
only when the style changes.

### `Wrapper` — token-level style preservation

genpdf's `StyledStr` type carries style alongside the string slice, preserving
bold/italic per token across run boundaries. normaxis-pdf's `(String, AppliedStyle)`
tuple in `layout_runs` achieves the same result. ✅ No change needed.

### `RenderResult.has_more` — re-entrant element rendering

This pattern allows any element to span multiple pages. Porting it to
normaxis-pdf would require changing the `Element::render` signature to return
`has_more: bool` and require elements to track their rendering cursor. Worth
considering for v1.2.0 when table pagination becomes necessary.
