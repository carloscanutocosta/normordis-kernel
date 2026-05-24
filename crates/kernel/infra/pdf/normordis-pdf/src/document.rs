use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    backend::pdf_writer_backend::PdfWriterBackend,
    backend::{FontRef, PdfBackend},
    compliance::ua::{AccessibilityConfig, StructTag, StructureTree, UaValidator},
    elements::{
        footer::{PageFooter, SectionedFooter},
        footnote::FootnoteAccumulator,
        header::SectionedHeader,
        toc::TocEntry,
        Element, LayoutMode, RenderContext,
    },
    layout::{PageFlow, TextLayoutEngine},
    page::PageLayout,
    styles::{SecurityClassification, TraceabilityMetadata, Watermark},
    NormaxisPdfError, Result,
};

/// PDF conformance standard for the output document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PdfStandard {
    /// Standard PDF 1.7 — no conformance requirements.
    #[default]
    Pdf17,
    /// PDF/A-1b — ISO 19005-1, long-term archival.
    PdfA1b,
    /// PDF/A-2b — ISO 19005-2 (uses same sRGB+XMP as A-1b in this implementation).
    PdfA2b,
    /// PDF/UA-2 — ISO 14289-2:2024 (PDF 2.0 + accessibility structure tree).
    /// Standalone standard: does NOT imply PDF/A conformance.
    PdfUa2,
}

impl PdfStandard {
    pub fn is_pdfa(self) -> bool {
        matches!(self, Self::PdfA1b | Self::PdfA2b)
    }

    pub fn is_pdfu2(self) -> bool {
        matches!(self, Self::PdfUa2)
    }

    pub fn xmp_part(self) -> u8 {
        match self {
            Self::PdfA1b => 1,
            Self::PdfA2b => 2,
            Self::PdfUa2 | Self::Pdf17 => 0,
        }
    }
}

/// Controls zlib compression level applied to PDF content streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionLevel {
    None,
    Fast,
    #[default]
    Default,
    Best,
}

impl CompressionLevel {
    pub fn to_zlib_level(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Fast => 1,
            Self::Default => 6,
            Self::Best => 9,
        }
    }
}

/// Internal document representation.
pub struct Document {
    pub(crate) title: String,
    pub(crate) style: crate::styles::DocumentStyle,
    pub(crate) fonts: crate::fonts::FontRegistry,
    pub(crate) header: Option<Box<dyn Element>>,
    pub(crate) sectioned_header: Option<SectionedHeader>,
    pub(crate) footer: Option<Box<dyn Element>>,
    pub(crate) sectioned_footer: Option<SectionedFooter>,
    pub(crate) watermark: Option<Watermark>,
    pub(crate) elements: Vec<Box<dyn Element>>,
    #[allow(dead_code)]
    pub(crate) footnotes: Vec<(u32, Vec<String>)>,
    #[allow(dead_code)]
    pub(crate) toc_entries: Option<Vec<TocEntry>>,
    pub(crate) compression: CompressionLevel,
    pub(crate) standard: PdfStandard,
    pub(crate) signature: Option<crate::signing::SignatureOptions>,
    pub(crate) traceability: Option<TraceabilityMetadata>,
    pub(crate) accessibility: AccessibilityConfig,
}

impl Document {
    /// First pass: estimate section positions for the TOC.
    fn collect_toc_entries_pass(&self) -> Vec<TocEntry> {
        let layout = PageLayout::from_style(&self.style);
        let hdr_h = if let Some(ref h) = self.header {
            h.estimated_height_mm()
        } else {
            0.0
        };
        let mut cursor_y = layout.page_height_mm - layout.margin_top_mm - hdr_h;
        let mut page = 1u32;
        let mut entries = Vec::new();

        for element in &self.elements {
            if let LayoutMode::Flow = element.layout_mode() {
                let h = element.estimated_height_mm();
                if cursor_y - h < layout.margin_bottom_mm {
                    page += 1;
                    cursor_y = layout.page_height_mm - layout.margin_top_mm - hdr_h;
                }
                cursor_y -= h;
            }
            if let Some((level, title)) = element.as_section_info() {
                entries.push(TocEntry {
                    level,
                    title: title.to_string(),
                    page_number: page,
                });
            }
        }
        entries
    }

    pub fn render_to_bytes(mut self) -> Result<Vec<u8>> {
        // TOC first pass
        let toc_data = self.collect_toc_entries_pass();
        if !toc_data.is_empty() {
            for element in &mut self.elements {
                element.inject_toc_entries(&toc_data);
            }
        }

        // Auto-apply classification watermark when traceability is set and non-public.
        if self.watermark.is_none() {
            if let Some(ref trace) = self.traceability {
                if trace.classification != SecurityClassification::Public {
                    self.watermark = Some(
                        Watermark::new(trace.classification.label_pt())
                            .opacity(0.08)
                            .color(trace.classification.watermark_color()),
                    );
                }
            }
        }

        let Document {
            title,
            style,
            fonts,
            header,
            sectioned_header,
            footer,
            sectioned_footer,
            watermark,
            elements,
            footnotes: _,
            toc_entries: _,
            compression,
            standard,
            signature,
            traceability: _,
            accessibility,
        } = self;

        // First-pass page count
        let total_pages = {
            let mut flow = PageFlow::new(&style);
            let mut pages = 1u32;
            let hdr_h = header_height_mm(&header, &sectioned_header, 1);
            flow.advance(hdr_h);
            for element in &elements {
                if let LayoutMode::Flow = element.layout_mode() {
                    let h = element.estimated_height_mm();
                    if flow.would_overflow(h) {
                        flow.new_page();
                        pages += 1;
                        flow.advance(header_height_mm(
                            &header,
                            &sectioned_header,
                            flow.page_number,
                        ));
                    }
                    flow.advance(h);
                }
            }
            pages
        };

        let (pw, ph) = style.page_size.dimensions_mm();
        let layout = PageLayout::from_style(&style);

        // If PdfUa2 is requested, enable accessibility automatically
        let accessibility = if standard == PdfStandard::PdfUa2 && !accessibility.enabled {
            AccessibilityConfig {
                enabled: true,
                ..accessibility
            }
        } else {
            accessibility
        };

        // ── Set up backend ────────────────────────────────────────────────────
        let mut backend = PdfWriterBackend::new(&title, compression.to_zlib_level());
        if standard.is_pdfa() {
            backend.set_pdfa(standard.xmp_part());
        }
        if standard.is_pdfu2() {
            backend.set_pdfu2();
        }
        if let Some(ref sig) = signature {
            backend.set_signature(&sig.reason, &sig.location, sig.reserved_bytes);
        }

        // Embed fonts
        let mut font_map: HashMap<String, FontRef> = HashMap::new();
        for (family_name, family) in fonts.families() {
            if let Ok(fr) = backend.embed_font(&family.regular.bytes, family_name, false, false) {
                font_map.insert(format!("{family_name}::regular"), fr);
            }
            if let Some(ref v) = family.bold {
                if let Ok(fr) = backend.embed_font(&v.bytes, family_name, true, false) {
                    font_map.insert(format!("{family_name}::bold"), fr);
                }
            }
            if let Some(ref v) = family.italic {
                if let Ok(fr) = backend.embed_font(&v.bytes, family_name, false, true) {
                    font_map.insert(format!("{family_name}::italic"), fr);
                }
            }
            if let Some(ref v) = family.bold_italic {
                if let Ok(fr) = backend.embed_font(&v.bytes, family_name, true, true) {
                    font_map.insert(format!("{family_name}::bold_italic"), fr);
                }
            }
        }

        // Open first page
        backend.new_page(pw, ph)?;

        let default_font_family = fonts.default_family_name().to_string();
        let layout_engine = TextLayoutEngine::new(&fonts, &style);
        let flow = PageFlow::new(&style);

        let ua_enabled = accessibility.enabled;
        let ua_lang = accessibility.lang.clone();

        let mut ctx = RenderContext {
            backend: Box::new(backend),
            font_map,
            flow,
            layout,
            layout_engine,
            style,
            fonts,
            force_page_break: false,
            default_font_family,
            page_number: 1,
            total_pages,
            resume_index: 0,
            glyph_tracker: crate::layout::GlyphUsageTracker::new(),
            reserved_footnotes_mm: 0.0,
            ua_config: accessibility,
            ua_events: StructureTree::new(),
            mcid_counter: 0,
            last_heading_level: None,
        };

        // Fixed elements are deferred until end of page (sorted by z_index)
        let mut fixed_pending: Vec<(i32, &dyn Element)> = Vec::new();
        let mut footnote_acc = FootnoteAccumulator::new();

        // PDF/UA-2: wrap all content in a /Document root structure element
        if ua_enabled {
            ctx.ua_events.begin_group(StructTag::Document, None);
        }

        // Render watermark and header on the first page
        render_watermark_if_any(&watermark, &mut ctx, pw, ph);
        render_header_for_page(&header, &sectioned_header, &mut ctx);

        for element in &elements {
            match element.layout_mode() {
                LayoutMode::Flow => {
                    if ctx.force_page_break {
                        ctx.force_page_break = false;
                        flush_page(
                            &mut ctx,
                            &mut fixed_pending,
                            &mut footnote_acc,
                            &footer,
                            &sectioned_footer,
                            &watermark,
                            &header,
                            &sectioned_header,
                            pw,
                            ph,
                        )?;
                    }

                    ctx.reset_resume();
                    loop {
                        let result = element.render(&mut ctx)?;
                        if !result.has_more {
                            break;
                        }
                        flush_page(
                            &mut ctx,
                            &mut fixed_pending,
                            &mut footnote_acc,
                            &footer,
                            &sectioned_footer,
                            &watermark,
                            &header,
                            &sectioned_header,
                            pw,
                            ph,
                        )?;
                    }
                }
                LayoutMode::Fixed(ref fb) => {
                    fixed_pending.push((fb.z_index, element.as_ref()));
                }
            }
        }

        // Final page: render fixed elements sorted by z, then footnotes, then footer
        fixed_pending.sort_by_key(|(z, _)| *z);
        for (_, elem) in &fixed_pending {
            let _ = elem.render(&mut ctx);
        }
        fixed_pending.clear();

        footnote_acc.render_pending(&mut ctx)?;
        render_footer_for_page(&footer, &sectioned_footer, &mut ctx);

        // ── PDF/UA-2: validate + write structure tree before finalising ───────
        if ua_enabled {
            ctx.ua_events.end_group(); // close the /Document root
            let events = std::mem::take(&mut ctx.ua_events.events);
            let tree_for_validation = StructureTree {
                events: events.clone(),
            };
            let validator = UaValidator::validate(Some(&tree_for_validation), &ua_lang);
            validator.report();
            ctx.backend.write_structure_tree(&events, &ua_lang);
        }

        // Finalise
        ctx.backend.finish()
    }

    pub fn render_to_file(self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let bytes = self.render_to_bytes()?;
        std::fs::write(path, bytes).map_err(NormaxisPdfError::IoError)
    }

    /// Render with signature placeholders and return a [`PreparedPdf`] ready
    /// for external PKCS#7 signing via [`PreparedPdf::embed_signature`].
    pub fn render_prepared_for_signing(
        self,
        opts: crate::signing::SignatureOptions,
    ) -> Result<crate::signing::PreparedPdf> {
        let reserved = opts.reserved_bytes;
        let bytes = Document {
            signature: Some(opts),
            ..self
        }
        .render_to_bytes()?;
        crate::signing::extract_prepared(bytes, reserved)
    }
}

// ── Page helpers ──────────────────────────────────────────────────────────────

fn header_height_mm(
    header: &Option<Box<dyn Element>>,
    sectioned: &Option<SectionedHeader>,
    page: u32,
) -> f64 {
    if let Some(ref sh) = sectioned {
        sh.resolve(page)
            .map(|h| h.estimated_height_mm())
            .unwrap_or(0.0)
    } else if let Some(ref h) = header {
        h.estimated_height_mm()
    } else {
        0.0
    }
}

#[allow(clippy::too_many_arguments)]
fn flush_page(
    ctx: &mut RenderContext,
    fixed_pending: &mut Vec<(i32, &dyn Element)>,
    footnote_acc: &mut FootnoteAccumulator,
    footer: &Option<Box<dyn Element>>,
    sectioned_footer: &Option<SectionedFooter>,
    watermark: &Option<Watermark>,
    header: &Option<Box<dyn Element>>,
    sectioned_header: &Option<SectionedHeader>,
    pw: f64,
    ph: f64,
) -> Result<()> {
    // Footnotes before closing page
    footnote_acc.render_pending(ctx)?;
    ctx.reserved_footnotes_mm = 0.0;

    // Fixed elements sorted by z
    fixed_pending.sort_by_key(|(z, _)| *z);
    for (_, elem) in fixed_pending.iter() {
        let _ = elem.render(ctx);
    }
    fixed_pending.clear();

    render_footer_for_page(footer, sectioned_footer, ctx);

    // Open new page
    ctx.backend.new_page(pw, ph)?;
    ctx.flow.new_page();
    ctx.page_number = ctx.flow.page_number;
    ctx.mcid_counter = 0;

    render_watermark_if_any(watermark, ctx, pw, ph);
    render_header_for_page(header, sectioned_header, ctx);
    Ok(())
}

fn render_header_for_page(
    header: &Option<Box<dyn Element>>,
    sectioned: &Option<SectionedHeader>,
    ctx: &mut RenderContext,
) {
    let page = ctx.page_number;
    if let Some(ref sh) = sectioned {
        if let Some(hdr) = sh.resolve(page) {
            let _ = hdr.render(ctx);
        }
    } else if let Some(ref hdr) = header {
        let _ = hdr.render(ctx);
    }
}

fn render_footer_for_page(
    footer: &Option<Box<dyn Element>>,
    sectioned: &Option<SectionedFooter>,
    ctx: &mut RenderContext,
) {
    let page = ctx.page_number;
    let footer_ref: Option<&PageFooter> = if let Some(ref sf) = sectioned {
        sf.resolve(page)
    } else {
        None
    };

    if let Some(f) = footer_ref {
        let saved = ctx.flow.cursor_y_mm;
        ctx.flow.cursor_y_mm = ctx.style.margin_bottom_mm + f.estimated_height_mm();
        let _ = f.render(ctx);
        ctx.flow.cursor_y_mm = saved;
        return;
    }

    if let Some(ref f) = footer {
        let h = f.estimated_height_mm();
        let saved = ctx.flow.cursor_y_mm;
        ctx.flow.cursor_y_mm = ctx.style.margin_bottom_mm + h;
        let _ = f.render(ctx);
        ctx.flow.cursor_y_mm = saved;
    }
}

fn render_watermark_if_any(
    watermark: &Option<Watermark>,
    ctx: &mut RenderContext,
    page_width_mm: f64,
    page_height_mm: f64,
) {
    let Some(wm) = watermark else { return };
    let Some(font_ref) = ctx.get_font_ref(false, false) else {
        return;
    };

    let cx_mm = page_width_mm / 2.0;
    let cy_mm = page_height_mm / 2.0;
    let half_w = ctx
        .fonts
        .get_default()
        .measure_text_mm(&wm.text, wm.font_size, false, false)
        / 2.0;

    if ctx.ua_config.enabled {
        ctx.backend.begin_artifact_content();
    }

    // Use real ExtGState opacity — no color simulation.
    let _ = ctx.backend.set_opacity(wm.opacity);
    let _ = ctx.backend.draw_text_rotated(
        &wm.text,
        cx_mm,
        cy_mm,
        wm.font_size,
        font_ref,
        &wm.color,
        wm.angle_deg,
        half_w,
    );
    ctx.backend.reset_opacity();

    if ctx.ua_config.enabled {
        ctx.backend.end_tagged_content();
    }
}
