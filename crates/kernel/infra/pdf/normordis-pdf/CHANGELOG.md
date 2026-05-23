# Changelog

All notable changes to `normordis-pdf` are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) — versions follow [Semantic Versioning](https://semver.org/).

---

## [Unreleased] — 2026-05-09

### Added

- **PDF bookmarks / document outline** — `Outline` / `OutlineItem` structs; `DocumentBuilder::outline()` builder; PDF `/Outlines` dict written in `finish()`; headings auto-registered when `outline_headings: true`
- **TOC clickable GoTo links** — TOC entries rendered as `/Link` annotations with `/GoTo` destinations pointing to the target page; `TocEntry::dest_ref` field
- **`InstitutionalHeader::render()`** — standardised institutional page header element with logo slot, entity name, and document reference line

### Changed

- **Crate renamed** from `normaxis-pdf` to `normordis-pdf`; package, Rust crate name (`normordis_pdf`), and all `use` paths updated across the workspace

### Fixed

- **Widow / orphan control** — `PageFlow` detects when fewer than `min_lines` lines would remain at the bottom of a page and forces a page break before the paragraph

---

## [2.4.1] — 2026-05-08

### Added

- **`pdfuaid` XMP namespace** — `build_xmp_pdfu2()` now emits `pdfuaid:part=2 pdfuaid:rev=2024` in the XMP metadata packet, required for PDF/UA-2 conformance
- **Artifact marking for decorative content** — watermark, header, footer, and decorative rules are wrapped in `/Artifact` marked-content sequences when UA is enabled
- **`LI` → `Lbl` + `LBody` structure** — list items now emit proper PDF/UA-2 sub-structure (`/Lbl` for the bullet/number, `/LBody` for the text content)

### Fixed

- **`InvalidRootTag("P")`** validation error — `StructureTree` root is now always `/Document`; was emitting `/P` when the first flow element was a paragraph

---

## [2.4.0] — 2026-05-06

### Added

- **`PdfStandard::PdfUa2`** — ISO 14289-2:2024 accessibility standard; standalone (does not imply PDF/A)
- **`AccessibilityConfig`** — `{ enabled: bool, lang: String }`; attached via `DocumentBuilder::accessibility()`; auto-enabled when `standard(PdfUa2)` is set
- **`StructureTree`** — event-driven structure tree builder: `begin_group`, `end_group`, `tag_content`; emits `/StructTreeRoot` and `/ParentTree` in `finish()`
- **`StructTag`** — 37 PDF role variants (`Document`, `H1`–`H6`, `P`, `L`, `LI`, `Table`, `TH`, `TD`, `Figure`, `Artifact`, …) with `Serialize` + `Deserialize`
- **`UaValidator`** — validates the structure tree before serialisation; reports warnings for missing `Alt` text on figures, empty headings, etc.
- **Tagged elements** — `Section` (H1–H6), `Paragraph` (P), `BulletList` / `OrderedList` / `CheckList` (L / LI), `Table` (Table / THead / TBody / TR / TH / TD), `ImageElement` (Figure), header / footer / watermark (Artifact), `FixedTextBox` / `FixedImageBox` (configurable role)
- **`FixedBox::ua_role` / `ua_alt`** — serde fields + `.role()` / `.alt()` builder methods for fixed-position elements
- **NDF 1.1.0 pipeline** — `compile_ndt()`, `render_ndf()`, `parse_ndf()`, `verify_ndf()`, `NdfDocument`, `NdfIntegrity` (SHA-256 / JCS), `NdfRevision`
- **JCS canonical JSON** — `jcs::canonicalise()` with UTF-16 key ordering; requires `serde_json` feature `preserve_order`
- **NCRTF 1.3.0 fixes** — `MarkValue::SmallCaps`, `MarkValue::UnderlineColor`, `MarkValue::StrikethroughColor`, `ParagraphBlock::style`, `TextNode::opentype_features`
- **`NdtOutput::accessibility`** — optional `AccessibilityConfig` in NDT 2.0 `output` block; `"pdf_ua2"` standard key
- **`examples/13_accessibility.rs`** — full PDF/UA-2 example
- **`tests/v210_accessibility.rs`** — 34 tests; **`tests/formats.rs`** — 42 NDF tests

### Changed

- `serde_json` overridden with `features = ["preserve_order"]` for JCS canonical key ordering
- `ENGINE_NDT_VERSION` bumped to `"2.1.0"`; version check accepts `template_major ≤ engine_major`
- `NormaxisPdfError` gains `NdfIntegrityError`, `NdfAuditError`, `NdfRevisionError`, `NdfCompileError`, `AccessibilityError`, `SerdeError` variants

---

## [2.3.0] — 2026-05-05

### Added

- **Digital signature two-phase API** — `DocumentBuilder::render_prepared_for_signing(opts)` returns a `PreparedPdf`; `PreparedPdf::embed_signature(pkcs7_der)` splices the PKCS#7 DER blob in-place
- **`SignatureOptions`** — `{ reason, location, reserved_bytes }`; defaults: "Assinado digitalmente", "Portugal", 8192
- **`PreparedPdf`** — `bytes_to_sign()`, `byte_range() -> (u64, u64, u64, u64)`, `embed_signature(pkcs7_der)`
- **ByteRange placeholder** — `[0 1111111111 1222222222 1333333333]` (36 bytes, space-padded); patched in-place after render
- **`SignatureConfig`** / **`SignatureField`** — higher-level config with visual field geometry; `DocumentBuilder::sign(config)` → `SigningBuilder`
- **`sign_pdf(prepared, config, pkcs7_der)`** — public convenience function
- **`tests/v230_signing.rs`** — signing test suite

---

## [2.2.0] — 2026-05-05

### Added

- **PDF/A-1b conformance** (`PdfStandard::PdfA1b`) — opt-in via `DocumentBuilder::standard(PdfStandard::PdfA1b)` or `.pdfa()` alias
- **XMP metadata packet** — `build_xmp_pdfa(title, part)` emits `pdfaid:part` / `pdfaid:conformance=B`; embedded as `/Metadata` stream on the catalog
- **sRGB ICC OutputIntent** — `srgb_icc_output_intent()` writes the sRGB v2 ICC profile as `/OutputIntent` (required by PDF/A)
- **`PdfStandard::PdfA2b`** — ISO 19005-2; same XMP/sRGB path with `part=2`
- **`DocumentBuilder::pdfa()`** kept as alias for `standard(PdfStandard::PdfA1b)`
- **`tests/v200_compliance.rs`** — 44 compliance tests

### Changed

- `Document.pdfa: bool` replaced by `Document.standard: PdfStandard`

---

## [2.1.1] — 2026-05-01

### Fixed

- **PDF object ordering for Adobe Acrobat** — content streams and font objects now written in separate `Chunk`s; fixes "file does not begin with '%PDF-'" error in Acrobat when opening subsetted PDFs

---

## [2.1.0] — 2026-05-01

### Added

- **pdf-writer 0.12 backend** (Eixo A) — complete replacement of printpdf with pdf-writer (same backend as Typst); `PdfWriterBackend` implements `PdfBackend` trait; low-level Op-based content streams replaced by `Content` builder API
- **Font subsetting** (Eixo B) — `subsetter 0.2.3` + `GlyphRemapper`; per-page glyph tracking via `GlyphUsageTracker`; `CIDToGIDMap` stream; `ToUnicode` CMap generated from BMP scan (U+0020–U+024F + punctuation); ~97% file size reduction
- **`subset_font(bytes, used_glyphs)`** — public free function in `backend::pdf_writer_backend`
- **`to_cff_if_possible(bytes)`** — extracts CFF table from OTF if present
- **`generate_to_unicode_cmap(bytes, used_glyphs)`** — real reverse GID→Unicode CMap
- **`PdfBackend` trait** — `embed_font`, `new_page`, `draw_text`, `draw_text_rotated`, `set_opacity`, `reset_opacity`, `begin_tagged_content`, `end_tagged_content`, `begin_artifact_content`, `write_structure_tree`, `finish`
- **Eixo E (real ExtGState opacity)** — `set_opacity(f64)` / `reset_opacity()` use PDF ExtGState; `opacity_gs: HashMap<u8, Ref>` cache in backend; watermark uses real opacity instead of color simulation
- **Eixo F (`TraceabilityMetadata`)** — `SecurityClassification { Public, Internal, Confidential, Reserved }`, `TraceabilityMetadata`; `DocumentBuilder::traceability(meta)`; auto-watermark when classification is non-Public
- **NDT 2.0.0 `output` + `signature` fields** — `NdtOutput`, `NdtSignature`, `NdtSignatureField` in template model; `push_ndt()` wires `output.standard` → `PdfStandard`, `output.compression` → `CompressionLevel`, `output.classification` → `TraceabilityMetadata`
- **`pub const PDF_BACKEND: &str = "pdf-writer"`** exported from `lib.rs`
- **`examples/12_compliance.rs`** — PDF/A-1b + traceability + NDT 2.0 output
- **379 tests**, 0 failures

### Changed

- `ENGINE_NDT_VERSION` bumped to `"2.0.0"`; version check accepts `template_major ≤ engine_major`

---

## [1.5.1] — 2026-04-30

### Added

- **`lopdf` compression pipeline** — post-processes the raw PDF bytes through lopdf's compression pass; reduces typical output from ~831 KB to ~280 KB (~66%)
- **`ndt-tools pdf-inspect` subcommand** — inspects a PDF file and reports object count, page count, embedded fonts, and compression status

---

## [1.5.0] — 2026-04-30

### Added

- **Footnotes** — `FootnoteRef` inline node in NCRTF; `FootnoteAccumulator` collects refs per page and renders them in a ruled section at the bottom of each page
- **TOC two-pass layout** — `collect_toc_entries_pass()` estimates section positions before the main render so the TOC has accurate page numbers
- **AcroForm visual placeholders** — `SignaturePlaceholder` element draws a visible dashed rectangle for signature fields; written to `/AcroForm` in the catalog
- **Nested tables** — `TableCell` content accepts another `Table` as a cell element
- **Liberation Serif** and **Liberation Mono** embedded at compile time (joining Liberation Sans)
- **`hyphenation` feature flag** — optional; embeds `hyphenation 0.8` with `embed_all` language data when enabled; plugs into `TextLayoutEngine`

### Changed

- `NDT_VERSION` bumped to `"1.5.0"`; `NCRTF_VERSION` bumped to `"1.2.0"`

---

## [1.4.1] — 2026-04-29

### Added

- **`compat_mode` in `NdtMeta`** — `Option<String>` field; when set to `"1.0"` the engine relaxes version checks for legacy templates

### Fixed

- **`dotx2ndt` phantom paragraph suppression** — trailing empty paragraphs generated by Word's style definitions no longer appear in the NDT skeleton output

---

## [1.4.0] — 2026-04-29

### Added

- **Knuth-Plass line breaking** — `TextLayoutEngine` upgraded from greedy wrapping to the full TeX Knuth-Plass algorithm via `rustybuzz 0.20`; better justification, fewer rivers
- **`TextDecoration`** — per-run decorations: `Underline`, `Strikethrough`, `Highlight(color)`, `Superscript`, `Subscript`, `SmallCaps`; rendered in `draw_text` for all backends
- **`ParagraphBorder`** — box or side borders around a `Paragraph` block; configurable line width, color, and padding
- **`SectionBreak`** — flow element that forces a page break and optionally resets the page counter
- **`ndt-tools` CLI v0.1.0** — standalone binary crate (`tools/ndt-tools`): `validate`, `render`, `inspect` subcommands

### Changed

- `rustybuzz` replaces ad-hoc glyph advance measurement; glyph metrics are now shaped correctly for complex scripts
- `NDT_VERSION` bumped to `"1.4.0"`

---

## [1.3.1] — 2026-04-28

### Added

- **Libertinus Serif** embedded as the default body font (replaces Liberation Sans as default); Liberation Sans retained for UI / sans-serif use

### Fixed

- **Word-join bug with Portuguese characters** — `TextLayoutEngine` was splitting words at `ã`, `ç`, `é`, and other non-ASCII letters when measuring break opportunities; fixed by using Unicode word-boundary rules instead of ASCII whitespace detection

---

## [1.3.0] — 2026-04-26

### Added

- **Named styles** — `NamedStyle` struct with optional fields; style inheritance chain via `extends`; cycle detection returns `StyleCycleError`; unknown name returns `UnknownStyle`
- **7 built-in styles** — `normal`, `heading_1`, `heading_2`, `heading_3`, `caption`, `table_header`, `table_body`; accessible without declaring in `DocumentStyle.named_styles`
- **`StyleResolver`** — resolves a named style to a fully-populated `ResolvedStyle` (no `Option` fields); `StyleResolver::new(styles, doc_style).resolve("name")`
- **`DocumentStyle.named_styles`** — `HashMap<String, NamedStyle>` for user-defined styles; user styles override built-ins with the same name
- **`Paragraph` — named style support** — `.style("caption")` builder method; `style_ref` field; resolved values used as defaults for `font_size`, `bold`, `italic`, `alignment`, `indent_*`, `space_before/after`
- **`space_before_mm` / `space_after_mm`** — per-paragraph spacing in mm; suppressed at top of page (matching Word behaviour); builder methods `.space_before(mm)` and `.space_after(mm)`
- **Tab stops** — `TabStop { position_mm, alignment, leader }` with factory methods `.left()`, `.right()`, `.center()`, `.decimal()`, `.with_leader(char)`; `TabStopAlign { Left, Right, Center, Decimal }`; added to `Paragraph` via `.tab_stop(stop)` builder; `\t` characters in `TextRun` text are processed by the layout engine
- **`PageFlow::is_top_of_page()`** — returns `true` when cursor is within 0.5 mm of the top margin; used to suppress `space_before`
- **`Section` — named style support** — `.style("heading_1")` builder method; `style_ref` field; defaults to `heading_1/2/3` built-ins by level
- **`TableStyle`** — named table style struct with `outer_border`, `inner_border`, `header_background`, `stripe_color`; factory methods `TableStyle::grid()`, `.bordered()`, `.striped()`, `.plain()`
- **`CellPadding`** — per-edge cell insets `{ top_mm, bottom_mm, left_mm, right_mm }`; default 1/1/2/2 mm; factory methods `.uniform(mm)`, `.horizontal_vertical(h, v)`; `TableCell.padding` field; `.padding(p)` builder method
- **`Table::with_table_style()`** — builder method to apply a `TableStyle`
- **Example `07_named_styles`** — demonstrates all v1.3.0 features
- **30 new tests** in `tests/v130_styles.rs`
- **`tools/dotx2ndt`** — CLI tool that extracts Word `.dotx` style definitions and generates an NDT-compatible named-styles JSON skeleton

### Changed

- `TextLayoutEngine::layout_runs` gains a `tab_stops: &[TabStop]` parameter; all internal callers pass `&[]` — no behaviour change for existing code
- `Section::render` now uses `StyleResolver` instead of hardcoded level-based sizing; output is visually identical for the default built-in styles
- `NDT_VERSION` bumped to `"1.3.0"`; `NCRTF_VERSION` bumped to `"1.1.0"`

---

## [1.2.0] — 2026-04-26

### Added

- **Table pagination** — tables spanning multiple pages no longer silently truncate rows; header rows are re-printed on each continuation page
- **List pagination** — `BulletList`, `OrderedList`, and `CheckList` span multiple pages correctly
- **Kerning** — optional pair kerning via `ab_glyph`; enable with feature flag `kerning`
- **`col_span` / `row_span`** in `TableCell` — merged-header tables; builder methods `.col_span(n)` and `.row_span(n)`
- **Z-index for Fixed Box** — `FixedBox::z_index: i32` (default 0); higher z-index renders on top; sorted before each page flush
- **Character spacing** — `TextRun::letter_spacing_mm: f64` (default 0.0); added to glyph-advance measurement and line wrapping
- **Per-cell borders** — `CellBorders`, `CellBorder`, `BorderLineStyle` (Solid / Dashed / Dotted / None) per table cell edge
- **Paragraph indentation** — `indent_left_mm`, `indent_right_mm`, `indent_first_line_mm`; builder methods `.indent_left()`, `.indent_right()`, `.indent_first_line()`
- **`TextAlign::Right`** reintroduced — right-aligned dates, numeric columns, letter headings; no breaking changes
- **`RenderResult`** — `Element::render` now returns `crate::Result<RenderResult>` with `has_more` flag; enables multi-page element continuation
- **`TableBuilder`** — fluent builder via `Table::builder()` with `header_row()`, `row()`, `stripe()`, `col_widths()`, `build()`
- **Example `06_advanced_layout`** — demonstrates all v1.2.0 features (indentation, Right alignment, col_span, multi-page table and list)
- **25 new tests** in `tests/v120_advanced.rs` covering all feature areas

### Fixed

- **`Paragraph::estimated_height_mm`** — replaced hardcoded `10.0` with character-width heuristic based on actual font size and content length

### Changed

- `Element::render` signature: `fn render(&self, ctx: &mut RenderContext) -> crate::Result<RenderResult>` (was `-> crate::Result<()>`); all simple elements return `RenderResult::done()` — no behaviour change
- `NDT_VERSION` bumped to `"1.2.0"` (backwards compatible with 1.0.0 and 1.1.0)

---

## [1.1.0] — 2026-04-25

### Added

- **`SectionedHeader`** — per-page-type institutional headers (`first_page`, `odd_pages`, `even_pages`) via `DocumentBuilder::sectioned_header()`
- **`SectionedFooter`** — per-page-type footers (`first_page`, `odd_pages`, `even_pages`) via `DocumentBuilder::sectioned_footer()`; `all_pages()` convenience builder
- **`PageFooter` text columns** — `.left()`, `.center()`, `.right()` builder methods; all columns accept runtime fields; separator line and column layout now rendered
- **`RowHeight` enum** — `Auto` / `AtLeast(f64)` / `Exact(f64)` row height control for `TableRow`; builder methods `height_exact()` and `height_at_least()`
- **`TableRow` / `TableCell` types** — `TableRow::plain(Vec<String>)` for simple construction; `TableRow::new(Vec<TableCell>)` for rich construction
- **Runtime calculated fields** — `{{page}}`, `{{total_pages}}`, `{{today}}`, `{{now}}` resolved in all footer text columns via `RuntimeContext` and `resolve_runtime_fields()`
- **`Watermark` struct** — diagonal text watermark on every page via `DocumentBuilder::watermark()`; configurable `text`, `opacity`, `color`, `font_size`, `angle_deg`
- **Two-pass page count** — `render_to_bytes` now estimates total pages before the main render so `{{total_pages}}` is accurate
- **`RenderContext` page tracking** — `page_number` and `total_pages` fields propagated to all elements
- **Example `05_fidelity`** — demonstrates all v1.1.0 features (sectioned header/footer, watermark, exact row heights, runtime fields)
- **13 new tests** in `tests/v110_fidelity.rs` covering all five feature areas

### Changed

- `Table::new` rows parameter changed from `Vec<Vec<String>>` to `Vec<TableRow>` — use `TableRow::plain()` for migration
- `NDT_VERSION` bumped to `"1.1.0"` (`ENGINE_NDT_VERSION` constant)
- `chrono` added as a workspace dependency (used by `RuntimeContext` for date/time fields)

---

## [1.0.0] — 2026-04-25

First stable release. API is considered stable from this version onwards.

### Added

- **NDT v1.0.0 template engine** (`src/template/`) — parse, validate, resolve, and render JSON-driven document templates with 16 body element types
  - `BodyElement` enum: `Paragraph`, `Heading`, `RichText`, `Table`, `List`, `Image`, `Spacer`, `HorizontalRule`, `PageBreak`, `FixedText`, `FixedImage`, `FixedLine`, `FixedBox`, `ZoneRef`, `Conditional`, `Repeat`, `Include`
  - `ConditionalElement` with operators: `exists`, `empty`, `eq`, `neq`, `gt`, `lt`
  - `RepeatElement` for list-driven repetition
  - Nested key resolution (`obj.field` syntax in data)
  - Placeholder type validation (`string`, `ncrtf`)
  - `TemplateError` enum with 8 variants for structured error reporting
- **`DocumentBuilder::push_ndt(template, data)`** — renders an NDT template into the document builder pipeline
- **`DocumentBuilder::push_ncrtf(json)`** — renders NCRTF rich text directly
- **Liberation Sans fonts** embedded at compile time (Regular, Bold, Italic, Bold Italic) — no system fonts required
- **`ab_glyph` integration** — real glyph advance-width metrics for accurate text layout
- **`Orientation` enum** (`Portrait` / `Landscape`) added to `DocumentStyle`
- **`elements::fixed` module** — unified re-export for all fixed-position element types (`FixedTextBox`, `FixedImageBox`, `FixedLineElement`, `VerticalAlign`, `ImageFit`)
- **`ListItem` type alias** for `ListItemElement`
- **Version constants**: `VERSION`, `NDT_VERSION`, `NCRTF_VERSION` available from crate root
- **4 runnable examples** with matching NDT templates:
  - `01_basic_document` — flow document
  - `02_ncrtf_document` — NCRTF rich text
  - `03_ndt_template` — NDT template with runtime data
  - `04_mixed_layout` — flow + fixed box (office letter)
- **4 bundled NDT templates**: `relatorio-simples`, `oficio-nacional`, `certidao-generica`, `formulario-generico`
- Comprehensive crate-level doc comments with Quick Start and NDT examples

### Changed

- `TextAlign` (canonical enum) replaces the previous split `Alignment` (layout) + `TextAlignment` (paragraph/fixed) — all public APIs now use `TextAlign`
- `TextLayoutEngine` methods now take `&FontRegistry` as first parameter, eliminating self-referential struct issues
- `FontRegistry` implements `Default` and embeds Liberation Sans automatically
- `FontVariant` implements `Clone` via re-parsing from stored bytes
- `NormaxisPdfError` gains a `Template(String)` variant
- Crate version is now managed independently (`version = "1.0.0"`) rather than inheriting workspace version
- Publication metadata added to `Cargo.toml` (repository, keywords, categories, description)

### Removed

- `Alignment` enum (replaced by `TextAlign`)
- `TextAlignment` enum (replaced by `TextAlign`)
- Local `TextAlign` in `richtext::model` (consolidated into `layout::TextAlign`)

---

## [0.7.0] — 2026-04-24

### Added

- `FixedBox` layout type with `OverflowPolicy::Shrink` (auto-reduce font size to fit)
- `VerticalAlign` for fixed text boxes (Top / Middle / Bottom)
- `BorderStyle` / `BoxBorder` for table and fixed box borders
- `PageFlow` struct for cursor and page management

---

## [0.6.0] — 2026-04-24

### Added

- NCRTF v1.0 parser (`push_ncrtf`) with inline marks (bold, italic, underline, strikethrough, code)
- `RichText` flow element
- `TextRun` / `AppliedStyle` types for styled inline text

---

## [0.5.0] — 2026-04-24

### Added

- `Table` flow element with configurable headers, rows, and column widths
- `List` flow element (bullet / ordered / checklist) with `ListItem` / `ListItemElement`

---

## [0.4.0] — 2026-04-24

### Added

- `FixedTextBox`, `FixedImageBox`, `FixedLineElement` for absolute-coordinate positioning
- Mixed layout mode (flow + fixed in one document)

---

## [0.3.0] — 2026-04-23

### Added

- `TextLayoutEngine` with real glyph metrics via `FontRegistry`
- Word-wrap, line-break, and justification logic
- `LayoutResult` / `LineBox` / `LineSegment` types

---

## [0.2.0] — 2026-04-23

### Added

- `FontRegistry` with `FontFamily` / `FontVariant`
- TTF loading via `printpdf` + `ab_glyph` glyph advance metrics

---

## [0.1.0] — 2026-04-22

### Added

- Initial scaffold: `DocumentBuilder`, `Document`, `PageLayout`
- `Paragraph`, `Section` (heading), `Spacer`, `PageBreak`, `HorizontalRule` flow elements
- `DocumentStyle` with `PageSize`, `RgbColor`
- printpdf 0.9 Op-based renderer
