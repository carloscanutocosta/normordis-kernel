use std::collections::{HashMap, HashSet};
use std::io::Write as IoWrite;

use flate2::write::ZlibEncoder;
use flate2::Compression;
use pdf_writer::{
    types::{CidFontType, FontFlags, OutputIntentSubtype, StructRole, SystemInfo},
    Chunk, Content, Date, Filter, Name, Pdf, Rect, Ref, Str, TextStr,
};

// sRGB IEC61966-2.1 v2 ICC profile (HP/Microsoft, 3144 bytes).
const SRGB_V2_ICC: &[u8] = include_bytes!("srgb_v2.icc");
use subsetter::GlyphRemapper;
use ttf_parser::{Face, GlyphId};

use super::{FontRef, PdfBackend};
use crate::{fonts::ShapedGlyph, styles::RgbColor, NormaxisPdfError};

// Minimal identity ToUnicode CMap — maps GID = Unicode code point.
const IDENTITY_TOUNICODE: &[u8] = b"\
/CIDInit /ProcSet findresource begin\n\
12 dict begin\n\
begincmap\n\
/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n\
/CMapName /Adobe-Identity-UCS def\n\
/CMapType 2 def\n\
1 begincodespacerange\n\
<0000> <FFFF>\n\
endcodespacerange\n\
1 beginbfrange\n\
<0000> <FFFF> <0000>\n\
endbfrange\n\
endcmap\n\
CMapName currentdict /CMap defineresource pop\n\
end\n\
end\n";

// Font refs are allocated in embed_font(); objects are written in finish()
// (after glyph usage is fully collected) but are merged into the PDF buffer
// BEFORE the pages chunk so readers see fonts before page content.
struct FontEntry {
    name: String,    // PDF resource name ("F0", "F1", ...)
    pdf_ref: Ref,    // Type0 font — referenced from page resources
    stream_ref: Ref, // Font program stream
    desc_ref: Ref,   // FontDescriptor
    cmap_ref: Ref,   // ToUnicode CMap
    cid_ref: Ref,    // CIDFont (descendant)
    family_name: String,
    bytes: Vec<u8>, // Full TTF bytes; subsetted in finish()
    bold: bool,
    italic: bool,
    // Metrics from the full font (stable across subsetting)
    ascent: f32,
    descent: f32,
    cap_height: f32,
    bbox: Rect,
}

/// A heading recorded for inclusion in the PDF /Outlines (bookmarks) tree.
struct OutlineEntry {
    title: String,
    level: u8,       // 1 = H1, 2 = H2, 3 = H3
    page_idx: usize, // 0-based page index
    y_pt: f32,       // baseline y in PDF points (bottom-origin)
}

/// Flat node used when building the outline tree hierarchy.
struct OutlineNode {
    entry_idx: usize,
    parent: Option<usize>,
    children: Vec<usize>,
}

fn count_outline_descendants(nodes: &[OutlineNode], ni: usize) -> usize {
    let mut total = nodes[ni].children.len();
    for &ci in &nodes[ni].children {
        total += count_outline_descendants(nodes, ci);
    }
    total
}

/// A link annotation pending on the current page (collected during rendering).
struct PendingLink {
    rect_pt: (f32, f32, f32, f32), // (x1, y1, x2, y2) in PDF points
    dest_title: String,
    dest_page_estimate: u32, // 1-based, from TocEntry
}

/// A link annotation whose dict must be written in finish() once all page refs
/// are known.
struct DeferredLinkAnnot {
    annot_ref: Ref,
    rect_pt: (f32, f32, f32, f32),
    dest_title: String,
    dest_page_estimate: u32,
}

struct InternalSignatureConfig {
    reason: String,
    location: String,
    reserved_bytes: usize,
    widget_ref: Ref,  // Form widget annotation (on page 1)
    sig_val_ref: Ref, // Signature value dict (ByteRange + Contents)
}

pub struct PdfWriterBackend {
    // Catalog is deferred to finish() so PDF/A metadata can be included.
    pdf: Option<Pdf>,
    catalog_ref: Ref,
    title: String,
    // Page content buffered here so finish() writes fonts before pages.
    pages_chunk: Chunk,
    content: Content,
    page_refs: Vec<Ref>,
    current_page_ref: Option<Ref>,
    pages_ref: Ref,
    alloc: Ref,
    page_width_pt: f32,
    page_height_pt: f32,
    fonts: Vec<FontEntry>,
    used_glyphs: HashMap<u32, HashSet<u16>>,
    /// Font indices (into `fonts`) referenced on the page being built.
    current_page_fonts: HashSet<u32>,
    compression: u32,
    /// When true, finish() emits XMP metadata and sRGB OutputIntent (PDF/A).
    pdfa: bool,
    /// XMP pdfaid:part value — 1 for PDF/A-1b, 2 for PDF/A-2b / PDF/UA-2.
    pdfa_part: u8,
    /// When set, finish() writes signature field placeholders into the output.
    signature: Option<InternalSignatureConfig>,
    /// Ref of page 1 (set in flush_current_page when page_index == 0).
    page1_ref: Option<Ref>,
    /// Cache of opacity level (0–255) → allocated ExtGState Ref.
    /// Objects are written in finish(); names are referenced in page resources.
    opacity_gs: HashMap<u8, Ref>,
    /// When Some, finish() links StructTreeRoot in the catalog.
    struct_tree_root_ref: Option<Ref>,
    /// Document language for PDF/UA-2 catalog /Lang entry.
    ua_lang: String,
    /// When true, finish() emits PDF 2.0 header + pdfuaid XMP + trailer ID.
    pdfu2: bool,
    /// Section headings collected during rendering for the /Outlines tree.
    outlines: Vec<OutlineEntry>,
    /// Link annotations accumulated for the page currently being rendered.
    /// Flushed (with pre-allocated Refs) into deferred_links on page close.
    current_page_links: Vec<PendingLink>,
    /// Link annotation dicts deferred to finish() so page Refs are available.
    deferred_links: Vec<DeferredLinkAnnot>,
}

fn mm_to_pt(mm: f64) -> f32 {
    (mm * 72.0 / 25.4) as f32
}

/// XMP for PDF/A-1b or PDF/A-2b (no UA markers — pdfuaid is not a predefined PDF/A schema).
fn build_xmp_pdfa(title: &str, part: u8) -> String {
    let safe_title = xml_escape(title);
    format!(
        "<?xpacket begin=\"\u{FEFF}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
         <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n  \
           <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n    \
             <rdf:Description rdf:about=\"\"\n      \
               xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\"\n      \
               xmlns:dc=\"http://purl.org/dc/elements/1.1/\"\n      \
               xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\">\n      \
               <pdfaid:part>{part}</pdfaid:part>\n      \
               <pdfaid:conformance>B</pdfaid:conformance>\n      \
               <dc:format>application/pdf</dc:format>\n      \
               <dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">{safe_title}</rdf:li></rdf:Alt></dc:title>\n      \
               <xmp:CreatorTool>normordis-pdf</xmp:CreatorTool>\n      \
               <xmp:CreateDate>2026-01-01T00:00:00Z</xmp:CreateDate>\n    \
             </rdf:Description>\n  \
           </rdf:RDF>\n\
         </x:xmpmeta>\n\
         <?xpacket end=\"w\"?>"
    )
}

/// XMP for PDF/UA-2 standalone (PDF 2.0, pdfuaid namespace, no pdfaid).
fn build_xmp_pdfu2(title: &str) -> String {
    let safe_title = xml_escape(title);
    format!(
        "<?xpacket begin=\"\u{FEFF}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
         <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n  \
           <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n    \
             <rdf:Description rdf:about=\"\"\n      \
               xmlns:pdfuaid=\"http://www.aiim.org/pdfua/ns/id/\"\n      \
               xmlns:dc=\"http://purl.org/dc/elements/1.1/\"\n      \
               xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\">\n      \
               <pdfuaid:part>2</pdfuaid:part>\n      \
               <pdfuaid:rev>2024</pdfuaid:rev>\n      \
               <dc:format>application/pdf</dc:format>\n      \
               <dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">{safe_title}</rdf:li></rdf:Alt></dc:title>\n      \
               <xmp:CreatorTool>normordis-pdf</xmp:CreatorTool>\n      \
               <xmp:CreateDate>2026-01-01T00:00:00Z</xmp:CreateDate>\n    \
             </rdf:Description>\n  \
           </rdf:RDF>\n\
         </x:xmpmeta>\n\
         <?xpacket end=\"w\"?>"
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Deterministic 16-byte file identifier for the PDF trailer /ID array.
/// Derived from the document title via SHA-256; both IDs are identical
/// (new and original are the same for a freshly generated file).
fn pdfu2_file_id(title: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(title.as_bytes());
    hash[..16].to_vec()
}

impl PdfWriterBackend {
    pub fn new(title: &str, compression: u32) -> Self {
        let mut pdf = Pdf::new();
        let mut alloc = Ref::new(1);

        // catalog_ref is allocated now but written in finish() so PDF/A metadata
        // can be included in the same catalog dict.
        let catalog_ref = alloc.bump();
        let pages_ref = alloc.bump();

        let info_ref = alloc.bump();
        let mut info = pdf.document_info(info_ref);
        info.producer(TextStr("normordis-pdf"));
        info.creator(TextStr(title));
        info.creation_date(Date::new(2026).month(1).day(1));
        drop(info);

        Self {
            pdf: Some(pdf),
            catalog_ref,
            title: title.to_string(),
            pages_chunk: Chunk::new(),
            content: Content::new(),
            page_refs: Vec::new(),
            current_page_ref: None,
            pages_ref,
            alloc,
            page_width_pt: mm_to_pt(210.0),
            page_height_pt: mm_to_pt(297.0),
            fonts: Vec::new(),
            used_glyphs: HashMap::new(),
            current_page_fonts: HashSet::new(),
            compression,
            pdfa: false,
            pdfa_part: 1,
            signature: None,
            page1_ref: None,
            opacity_gs: HashMap::new(),
            struct_tree_root_ref: None,
            ua_lang: String::new(),
            pdfu2: false,
            outlines: Vec::new(),
            current_page_links: Vec::new(),
            deferred_links: Vec::new(),
        }
    }

    /// Enable PDF/A conformance. `part` is 1 for PDF/A-1b, 2 for PDF/A-2b.
    pub fn set_pdfa(&mut self, part: u8) {
        self.pdfa = true;
        self.pdfa_part = part;
    }

    /// Enable PDF/UA-2 mode: PDF 2.0 header, pdfuaid XMP, trailer ID.
    pub fn set_pdfu2(&mut self) {
        self.pdfu2 = true;
    }

    /// Enable signature preparation mode.
    ///
    /// Must be called before the first `new_page()` so the widget annotation
    /// can be placed on page 1. After `finish()`, call
    /// `signing::extract_prepared()` on the raw bytes to obtain a
    /// `PreparedPdf` with patched `ByteRange` and the `Contents` offset.
    pub fn set_signature(&mut self, reason: &str, location: &str, reserved_bytes: usize) {
        let widget_ref = self.alloc.bump();
        let sig_val_ref = self.alloc.bump();
        self.signature = Some(InternalSignatureConfig {
            reason: reason.to_string(),
            location: location.to_string(),
            reserved_bytes,
            widget_ref,
            sig_val_ref,
        });
    }

    /// Register a font variant. Refs are allocated now so page resources can
    /// reference them; all PDF objects are written in `finish()`, in a chunk
    /// that is merged before the pages content.
    pub fn embed_font(
        &mut self,
        bytes: &[u8],
        family_name: &str,
        bold: bool,
        italic: bool,
    ) -> crate::Result<FontRef> {
        let font_idx = self.fonts.len() as u32;
        let font_name = format!("F{font_idx}");

        let face = Face::parse(bytes, 0)
            .map_err(|e| NormaxisPdfError::FontLoadError(format!("ttf-parser: {e}")))?;

        let ascent = face.ascender() as f32;
        let descent = face.descender() as f32;
        let cap_height = face.capital_height().unwrap_or(face.ascender()) as f32;
        let bb = face.global_bounding_box();
        let bbox = Rect::new(
            bb.x_min as f32,
            bb.y_min as f32,
            bb.x_max as f32,
            bb.y_max as f32,
        );

        let stream_ref = self.alloc.bump();
        let desc_ref = self.alloc.bump();
        let cmap_ref = self.alloc.bump();
        let cid_ref = self.alloc.bump();
        let pdf_ref = self.alloc.bump(); // Type0

        self.fonts.push(FontEntry {
            name: font_name,
            pdf_ref,
            stream_ref,
            desc_ref,
            cmap_ref,
            cid_ref,
            family_name: family_name.to_string(),
            bytes: bytes.to_vec(),
            bold,
            italic,
            ascent,
            descent,
            cap_height,
            bbox,
        });

        Ok(FontRef(font_idx))
    }

    /// Encode UTF-8 text as 2-byte big-endian GIDs and record them for
    /// subsetting.
    fn text_to_gid_bytes(&mut self, font_ref: FontRef, text: &str) -> Vec<u8> {
        let bytes = &self.fonts[font_ref.0 as usize].bytes;
        let face = match Face::parse(bytes, 0) {
            Ok(f) => f,
            Err(_) => {
                let mut out = Vec::with_capacity(text.len() * 2);
                for ch in text.chars() {
                    let cp = ch as u32;
                    out.push((cp >> 8) as u8);
                    out.push((cp & 0xFF) as u8);
                }
                return out;
            }
        };
        self.current_page_fonts.insert(font_ref.0);
        let used = self.used_glyphs.entry(font_ref.0).or_default();
        let mut out = Vec::with_capacity(text.len() * 2);
        for ch in text.chars() {
            let gid = face.glyph_index(ch).map(|g| g.0).unwrap_or(0);
            used.insert(gid);
            out.push((gid >> 8) as u8);
            out.push((gid & 0xFF) as u8);
        }
        out
    }

    fn compress(&self, data: &[u8]) -> crate::Result<Vec<u8>> {
        let level = Compression::new(self.compression);
        let mut enc = ZlibEncoder::new(Vec::new(), level);
        enc.write_all(data)
            .and_then(|_| enc.finish())
            .map_err(|e| NormaxisPdfError::RenderError(format!("flate2: {e}")))
    }
}

fn compress_bytes(data: &[u8], level: u32) -> Vec<u8> {
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::new(level));
    use std::io::Write;
    enc.write_all(data).ok();
    enc.finish().unwrap_or_else(|_| data.to_vec())
}

impl PdfWriterBackend {
    /// Flush the current page's content stream and page dict into `pages_chunk`
    /// (not directly into `pdf`). This allows `finish()` to write font objects
    /// first, then append pages — giving the correct object order.
    fn flush_current_page(&mut self) -> crate::Result<()> {
        let page_ref = match self.current_page_ref.take() {
            Some(r) => r,
            None => return Ok(()),
        };

        // 0-based index of the page being flushed (page_refs already has this ref).
        let page_index = self.page_refs.len().saturating_sub(1);

        // Track page 1 ref so finish() can wire up the sig widget's /P entry.
        if page_index == 0 {
            self.page1_ref = Some(page_ref);
        }

        // Extract widget ref (Copy) before mutably borrowing pages_chunk.
        let sig_widget = if page_index == 0 {
            self.signature.as_ref().map(|s| s.widget_ref)
        } else {
            None
        };

        // Pre-allocate link annotation refs so they can appear in the page /Annots array.
        let n_links = self.current_page_links.len();
        let link_refs: Vec<Ref> = (0..n_links).map(|_| self.alloc.bump()).collect();
        let pending_links = std::mem::take(&mut self.current_page_links);

        let raw = std::mem::replace(&mut self.content, Content::new()).finish();
        let content_ref = self.alloc.bump();

        let data: std::borrow::Cow<[u8]> = if self.compression > 0 {
            std::borrow::Cow::Owned(self.compress(&raw)?)
        } else {
            std::borrow::Cow::Borrowed(&raw)
        };

        // Write content stream → pages_chunk
        if self.compression > 0 {
            self.pages_chunk
                .stream(content_ref, &data)
                .filter(Filter::FlateDecode);
        } else {
            self.pages_chunk.stream(content_ref, &data);
        }

        // Write page dict → pages_chunk
        let mut page = self.pages_chunk.page(page_ref);
        page.parent(self.pages_ref)
            .media_box(Rect::new(0.0, 0.0, self.page_width_pt, self.page_height_pt))
            .contents(content_ref);

        // Combined annotations: sig widget (page 1 only) + any link annotations.
        {
            let mut annot_refs: Vec<Ref> = Vec::new();
            if let Some(wref) = sig_widget {
                annot_refs.push(wref);
            }
            annot_refs.extend_from_slice(&link_refs);
            if !annot_refs.is_empty() {
                page.annotations(annot_refs);
            }
        }

        // Resources — fonts + ExtGState entries
        {
            let mut res = page.resources();
            {
                let mut fdict = res.fonts();
                for (idx, entry) in self.fonts.iter().enumerate() {
                    if self.current_page_fonts.contains(&(idx as u32)) {
                        fdict.pair(Name(entry.name.as_bytes()), entry.pdf_ref);
                    }
                }
            }
            if !self.opacity_gs.is_empty() {
                let mut gsdict = res.ext_g_states();
                for (&opacity_u8, &gs_ref) in &self.opacity_gs {
                    let name = format!("GS{opacity_u8}");
                    gsdict.pair(Name(name.as_bytes()), gs_ref);
                }
            }
        }

        self.current_page_fonts.clear();

        // Move pending links to the deferred list now that the page dict is sealed.
        for (link, aref) in pending_links.into_iter().zip(link_refs.into_iter()) {
            self.deferred_links.push(DeferredLinkAnnot {
                annot_ref: aref,
                rect_pt: link.rect_pt,
                dest_title: link.dest_title,
                dest_page_estimate: link.dest_page_estimate,
            });
        }

        Ok(())
    }

    fn subset_font(bytes: &[u8], used: &HashSet<u16>) -> (Vec<u8>, GlyphRemapper) {
        let mut remapper = GlyphRemapper::new();
        remapper.remap(0); // always keep .notdef
        for &gid in used {
            remapper.remap(gid);
        }
        let subsetted = subsetter::subset(bytes, 0, &remapper).unwrap_or_else(|_| bytes.to_vec());
        (subsetted, remapper)
    }

    /// Compute per-CID advance widths (in 1/1000 em units) for the `/W` array.
    fn compute_cid_widths(font_bytes: &[u8], used: &HashSet<u16>) -> Vec<(u16, f32)> {
        let face = match Face::parse(font_bytes, 0) {
            Ok(f) => f,
            Err(_) => return vec![],
        };
        let upem = face.units_per_em() as f32;
        let mut ws: Vec<(u16, f32)> = used
            .iter()
            .map(|&gid| {
                let adv = face.glyph_hor_advance(GlyphId(gid)).unwrap_or(upem as u16) as f32;
                (gid, adv / upem * 1000.0)
            })
            .collect();
        ws.sort_unstable_by_key(|&(g, _)| g);
        ws
    }

    /// Build a CIDToGIDMap stream: `map[cid * 2] = new_gid (big-endian)`.
    /// Maps the original GID (used as character code / CID via Identity-H) to
    /// the remapped GID in the subsetted font.
    fn build_cid_to_gid_map(used: &HashSet<u16>, remapper: &GlyphRemapper) -> Vec<u8> {
        let max_cid = used.iter().copied().max().unwrap_or(0) as usize;
        let mut map = vec![0u8; (max_cid + 1) * 2];
        if let Some(new) = remapper.get(0) {
            map[0] = (new >> 8) as u8;
            map[1] = (new & 0xFF) as u8;
        }
        for &old_gid in used {
            if let Some(new_gid) = remapper.get(old_gid) {
                let pos = old_gid as usize * 2;
                if pos + 1 < map.len() {
                    map[pos] = (new_gid >> 8) as u8;
                    map[pos + 1] = (new_gid & 0xFF) as u8;
                }
            }
        }
        map
    }
}

impl PdfBackend for PdfWriterBackend {
    fn draw_text(
        &mut self,
        text: &str,
        x_mm: f64,
        y_mm: f64,
        font_size_pt: f64,
        font_ref: FontRef,
        color: &RgbColor,
        letter_spacing_pt: f32,
    ) -> crate::Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        let gid_bytes = self.text_to_gid_bytes(font_ref, text);
        let font_name = self.fonts[font_ref.0 as usize].name.clone();
        let x_pt = mm_to_pt(x_mm);
        let y_pt = mm_to_pt(y_mm);
        let fs = font_size_pt as f32;

        self.content
            .set_fill_rgb(color.r as f32, color.g as f32, color.b as f32);

        if letter_spacing_pt != 0.0 {
            self.content.set_char_spacing(letter_spacing_pt);
        }

        self.content.begin_text();
        self.content.set_font(Name(font_name.as_bytes()), fs);
        self.content
            .set_text_matrix([1.0, 0.0, 0.0, 1.0, x_pt, y_pt]);
        self.content.show(Str(&gid_bytes));
        self.content.end_text();

        if letter_spacing_pt != 0.0 {
            self.content.set_char_spacing(0.0);
        }

        Ok(())
    }

    fn draw_shaped_glyphs(
        &mut self,
        glyphs: &[ShapedGlyph],
        x_mm: f64,
        y_mm: f64,
        font_size_pt: f64,
        font_ref: FontRef,
        color: &RgbColor,
    ) -> crate::Result<()> {
        if glyphs.is_empty() {
            return Ok(());
        }
        self.current_page_fonts.insert(font_ref.0);
        let used = self.used_glyphs.entry(font_ref.0).or_default();
        let mut gid_bytes = Vec::with_capacity(glyphs.len() * 2);
        for g in glyphs {
            used.insert(g.glyph_id);
            gid_bytes.push((g.glyph_id >> 8) as u8);
            gid_bytes.push((g.glyph_id & 0xFF) as u8);
        }

        let font_name = self.fonts[font_ref.0 as usize].name.clone();
        let x_pt = mm_to_pt(x_mm);
        let y_pt = mm_to_pt(y_mm);
        let fs = font_size_pt as f32;

        self.content
            .set_fill_rgb(color.r as f32, color.g as f32, color.b as f32);
        self.content.begin_text();
        self.content.set_font(Name(font_name.as_bytes()), fs);
        self.content
            .set_text_matrix([1.0, 0.0, 0.0, 1.0, x_pt, y_pt]);
        self.content.show(Str(&gid_bytes));
        self.content.end_text();

        Ok(())
    }

    fn draw_line(
        &mut self,
        x1_mm: f64,
        y1_mm: f64,
        x2_mm: f64,
        y2_mm: f64,
        width_pt: f32,
        color: &RgbColor,
    ) -> crate::Result<()> {
        self.content
            .set_stroke_rgb(color.r as f32, color.g as f32, color.b as f32)
            .set_line_width(width_pt)
            .move_to(mm_to_pt(x1_mm), mm_to_pt(y1_mm))
            .line_to(mm_to_pt(x2_mm), mm_to_pt(y2_mm))
            .stroke();
        Ok(())
    }

    fn draw_rect(
        &mut self,
        x_mm: f64,
        y_mm: f64,
        width_mm: f64,
        height_mm: f64,
        fill: &RgbColor,
    ) -> crate::Result<()> {
        self.content
            .set_fill_rgb(fill.r as f32, fill.g as f32, fill.b as f32)
            .rect(
                mm_to_pt(x_mm),
                mm_to_pt(y_mm),
                mm_to_pt(width_mm),
                mm_to_pt(height_mm),
            )
            .fill_nonzero();
        Ok(())
    }

    fn draw_rect_stroked(
        &mut self,
        x_mm: f64,
        y_mm: f64,
        width_mm: f64,
        height_mm: f64,
        fill: &RgbColor,
        stroke: &RgbColor,
        stroke_pt: f32,
    ) -> crate::Result<()> {
        self.content
            .set_fill_rgb(fill.r as f32, fill.g as f32, fill.b as f32)
            .set_stroke_rgb(stroke.r as f32, stroke.g as f32, stroke.b as f32)
            .set_line_width(stroke_pt)
            .rect(
                mm_to_pt(x_mm),
                mm_to_pt(y_mm),
                mm_to_pt(width_mm),
                mm_to_pt(height_mm),
            )
            .fill_nonzero_and_stroke();
        Ok(())
    }

    fn draw_text_rotated(
        &mut self,
        text: &str,
        cx_mm: f64,
        cy_mm: f64,
        font_size_pt: f64,
        font_ref: FontRef,
        color: &RgbColor,
        angle_deg: f64,
        half_width_mm: f64,
    ) -> crate::Result<()> {
        let gid_bytes = self.text_to_gid_bytes(font_ref, text);
        let font_name = self.fonts[font_ref.0 as usize].name.clone();
        let cx_pt = mm_to_pt(cx_mm);
        let cy_pt = mm_to_pt(cy_mm);
        let hw_pt = mm_to_pt(half_width_mm);
        let fs = font_size_pt as f32;
        let angle_rad = (-angle_deg).to_radians();
        let cos_a = angle_rad.cos() as f32;
        let sin_a = angle_rad.sin() as f32;

        self.content.save_state();
        self.content
            .transform([cos_a, sin_a, -sin_a, cos_a, cx_pt, cy_pt]);
        self.content
            .set_fill_rgb(color.r as f32, color.g as f32, color.b as f32);
        self.content.begin_text();
        self.content.set_font(Name(font_name.as_bytes()), fs);
        self.content
            .set_text_matrix([1.0, 0.0, 0.0, 1.0, -hw_pt, 0.0]);
        self.content.show(Str(&gid_bytes));
        self.content.end_text();
        self.content.restore_state();
        Ok(())
    }

    fn new_page(&mut self, width_mm: f64, height_mm: f64) -> crate::Result<()> {
        self.flush_current_page()?;
        self.page_width_pt = mm_to_pt(width_mm);
        self.page_height_pt = mm_to_pt(height_mm);
        let page_ref = self.alloc.bump();
        self.page_refs.push(page_ref);
        self.current_page_ref = Some(page_ref);
        self.content = Content::new();
        Ok(())
    }

    fn finish(&mut self) -> crate::Result<Vec<u8>> {
        self.flush_current_page()?;

        // ── Prepare subsetted font data ────────────────────────────────────────
        struct PreparedFont {
            stream_ref: Ref,
            desc_ref: Ref,
            cmap_ref: Ref,
            cid_ref: Ref,
            pdf_ref: Ref,
            family_name: String,
            bold: bool,
            italic: bool,
            ascent: f32,
            descent: f32,
            cap_height: f32,
            bbox: Rect,
            subsetted: Vec<u8>,
            subsetted_len: i32,
            cid_to_gid: Vec<u8>,
            tounicode: Vec<u8>,
            cid_widths: Vec<(u16, f32)>,
        }

        let empty_set: HashSet<u16> = HashSet::new();
        let compression = self.compression;
        let prepared: Vec<PreparedFont> = self
            .fonts
            .iter()
            .enumerate()
            .filter(|(idx, _)| {
                self.used_glyphs
                    .get(&(*idx as u32))
                    .map_or(false, |s| !s.is_empty())
            })
            .map(|(idx, e)| {
                let used = self.used_glyphs.get(&(idx as u32)).unwrap_or(&empty_set);
                let (subsetted_raw, remapper) = Self::subset_font(&e.bytes, used);
                let cid_to_gid_raw = Self::build_cid_to_gid_map(used, &remapper);
                let tounicode_raw = generate_to_unicode_cmap(&e.bytes, used);
                let cid_widths = Self::compute_cid_widths(&e.bytes, used);
                let subsetted_len = subsetted_raw.len() as i32;
                let (subsetted, cid_to_gid, tounicode) = if compression > 0 {
                    (
                        compress_bytes(&subsetted_raw, compression),
                        compress_bytes(&cid_to_gid_raw, compression),
                        compress_bytes(&tounicode_raw, compression),
                    )
                } else {
                    (subsetted_raw, cid_to_gid_raw, tounicode_raw)
                };
                PreparedFont {
                    stream_ref: e.stream_ref,
                    desc_ref: e.desc_ref,
                    cmap_ref: e.cmap_ref,
                    cid_ref: e.cid_ref,
                    pdf_ref: e.pdf_ref,
                    family_name: e.family_name.clone(),
                    bold: e.bold,
                    italic: e.italic,
                    ascent: e.ascent,
                    descent: e.descent,
                    cap_height: e.cap_height,
                    bbox: e.bbox,
                    subsetted,
                    subsetted_len,
                    cid_to_gid,
                    tounicode,
                    cid_widths,
                }
            })
            .collect();

        // Allocate CIDToGIDMap refs before any pdf borrows.
        let ctg_refs: Vec<Ref> = prepared.iter().map(|_| self.alloc.bump()).collect();

        // ── Write font objects into `pdf` FIRST (before pages content) ─────────
        // Chunk::extend() adjusts byte offsets, so pdf will have correct xref.
        for (prep, &ctg_ref) in prepared.iter().zip(ctg_refs.iter()) {
            let mut flags = FontFlags::empty();
            flags |= FontFlags::NON_SYMBOLIC;
            if prep.italic {
                flags |= FontFlags::ITALIC;
            }
            if prep.bold {
                flags |= FontFlags::FORCE_BOLD;
            }

            let pdf = self.pdf.as_mut().expect("pdf not finished");

            {
                let mut s = pdf.stream(prep.stream_ref, &prep.subsetted);
                s.pair(Name(b"Length1"), prep.subsetted_len);
                if compression > 0 {
                    s.filter(Filter::FlateDecode);
                }
            }

            pdf.font_descriptor(prep.desc_ref)
                .name(Name(prep.family_name.as_bytes()))
                .flags(flags)
                .bbox(prep.bbox)
                .italic_angle(if prep.italic { -12.0 } else { 0.0 })
                .ascent(prep.ascent)
                .descent(prep.descent)
                .cap_height(prep.cap_height)
                .stem_v(80.0)
                .font_file2(prep.stream_ref);

            {
                let mut s = pdf.stream(prep.cmap_ref, &prep.tounicode);
                if compression > 0 {
                    s.filter(Filter::FlateDecode);
                }
            }

            // CIDToGIDMap: old GID (= CID via Identity-H) → new GID in subsetted font.
            {
                let mut s = pdf.stream(ctg_ref, &prep.cid_to_gid);
                if compression > 0 {
                    s.filter(Filter::FlateDecode);
                }
            }

            {
                let mut cid_f = pdf.cid_font(prep.cid_ref);
                cid_f.subtype(CidFontType::Type2);
                cid_f.base_font(Name(prep.family_name.as_bytes()));
                cid_f.system_info(SystemInfo {
                    registry: Str(b"Adobe"),
                    ordering: Str(b"Identity"),
                    supplement: 0,
                });
                cid_f.default_width(1000.0);
                cid_f.font_descriptor(prep.desc_ref);
                cid_f.cid_to_gid_map_stream(ctg_ref);
                if !prep.cid_widths.is_empty() {
                    let sorted = &prep.cid_widths;
                    let mut ws = cid_f.widths();
                    let mut i = 0;
                    while i < sorted.len() {
                        let run_first = sorted[i].0;
                        let mut j = i + 1;
                        while j < sorted.len() && sorted[j].0 == sorted[j - 1].0 + 1 {
                            j += 1;
                        }
                        ws.consecutive(run_first, sorted[i..j].iter().map(|&(_, w)| w));
                        i = j;
                    }
                }
            }

            pdf.type0_font(prep.pdf_ref)
                .base_font(Name(prep.family_name.as_bytes()))
                .encoding_predefined(Name(b"Identity-H"))
                .descendant_font(prep.cid_ref)
                .to_unicode(prep.cmap_ref);
        }

        // ── Merge pages content AFTER fonts ───────────────────────────────────
        // extend() adjusts all byte offsets in pages_chunk by the current
        // length of pdf's buffer, so the xref will be correct.
        let pages_chunk = std::mem::replace(&mut self.pages_chunk, Chunk::new());
        let pdf = self.pdf.as_mut().expect("pdf not finished");
        pdf.extend(&pages_chunk);

        // ── Write pages tree ──────────────────────────────────────────────────
        let page_count = self.page_refs.len() as i32;
        let page_refs: Vec<Ref> = self.page_refs.clone();
        pdf.pages(self.pages_ref)
            .count(page_count)
            .kids(page_refs.into_iter());

        // ── Signature objects ─────────────────────────────────────────────────
        // Extract all sig data before taking another &mut pdf borrow.
        struct SigSnap {
            widget_ref: Ref,
            sig_val_ref: Ref,
            reserved_bytes: usize,
            reason: String,
            location: String,
        }
        let sig_snap: Option<SigSnap> = self.signature.as_ref().map(|s| SigSnap {
            widget_ref: s.widget_ref,
            sig_val_ref: s.sig_val_ref,
            reserved_bytes: s.reserved_bytes,
            reason: s.reason.clone(),
            location: s.location.clone(),
        });
        let page1_ref = self.page1_ref;

        if let Some(ref snap) = sig_snap {
            let p1 = page1_ref.expect("signature requires at least one rendered page");
            let pdf = self.pdf.as_mut().expect("pdf not finished");

            // Invisible signature widget annotation (rect [0 0 0 0] on page 1).
            {
                let mut d = pdf.indirect(snap.widget_ref).dict();
                d.pair(Name(b"Type"), Name(b"Annot"));
                d.pair(Name(b"Subtype"), Name(b"Widget"));
                d.pair(Name(b"FT"), Name(b"Sig"));
                d.pair(Name(b"T"), TextStr("Sig1"));
                d.pair(Name(b"Rect"), Rect::new(0.0, 0.0, 0.0, 0.0));
                d.pair(Name(b"P"), p1);
                d.pair(Name(b"V"), snap.sig_val_ref);
                d.pair(Name(b"F"), 4i32);
            }

            // Signature value dict with ByteRange + Contents placeholders.
            // ByteRange placeholder: exactly 36 bytes → patched by extract_prepared().
            // Contents placeholder:  Str(&[0x80; N]) → hex "<8080...>" → patched by embed_signature().
            {
                let contents_placeholder = vec![0x80u8; snap.reserved_bytes];
                let mut d = pdf.indirect(snap.sig_val_ref).dict();
                d.pair(Name(b"Type"), Name(b"Sig"));
                d.pair(Name(b"Filter"), Name(b"Adobe.PPKLite"));
                d.pair(Name(b"SubFilter"), Name(b"adbe.pkcs7.detached"));
                d.insert(Name(b"ByteRange"))
                    .array()
                    .item(0i32)
                    .item(1_111_111_111i32)
                    .item(1_222_222_222i32)
                    .item(1_333_333_333i32);
                d.pair(Name(b"Contents"), Str(&contents_placeholder));
                d.pair(Name(b"Reason"), TextStr(&snap.reason));
                d.pair(Name(b"Location"), TextStr(&snap.location));
                d.pair(Name(b"M"), TextStr("D:20260101000000Z"));
            }
        }

        // ── ExtGState objects for opacity ─────────────────────────────────────
        for (&opacity_u8, &gs_ref) in &self.opacity_gs {
            let alpha = opacity_u8 as f32 / 255.0;
            let pdf = self.pdf.as_mut().expect("pdf not finished");
            pdf.ext_graphics(gs_ref)
                .non_stroking_alpha(alpha)
                .stroking_alpha(alpha);
        }

        // ── PDF Outlines (bookmarks) ──────────────────────────────────────────
        // Build title → (page_idx, y_pt) lookup BEFORE taking outlines (used for
        // deferred link annotation resolution further below).
        let heading_lookup: HashMap<String, (usize, f32)> = self
            .outlines
            .iter()
            .map(|e| (e.title.clone(), (e.page_idx, e.y_pt)))
            .collect();

        let outline_root_ref: Option<Ref> = {
            let entries = std::mem::take(&mut self.outlines);
            if entries.is_empty() {
                None
            } else {
                // Build flat node tree (parent/children relationships)
                let mut nodes: Vec<OutlineNode> = Vec::new();
                let mut level_last: [Option<usize>; 4] = [None; 4]; // idx 1..=3 used

                for (ei, entry) in entries.iter().enumerate() {
                    let lv = (entry.level as usize).clamp(1, 3);
                    let parent = if lv == 1 { None } else { level_last[lv - 1] };
                    let ni = nodes.len();
                    nodes.push(OutlineNode {
                        entry_idx: ei,
                        parent,
                        children: vec![],
                    });
                    if let Some(p) = parent {
                        nodes[p].children.push(ni);
                    }
                    level_last[lv] = Some(ni);
                    for l in (lv + 1)..4 {
                        level_last[l] = None;
                    }
                }

                let root_children: Vec<usize> = nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| n.parent.is_none())
                    .map(|(i, _)| i)
                    .collect();

                let outline_root = self.alloc.bump();
                let node_refs: Vec<Ref> = nodes.iter().map(|_| self.alloc.bump()).collect();
                let page_refs_snap: Vec<Ref> = self.page_refs.clone();

                let pdf = self.pdf.as_mut().expect("pdf not finished");

                // Write outline root dict
                {
                    let mut d = pdf.indirect(outline_root).dict();
                    d.pair(Name(b"Type"), Name(b"Outlines"));
                    d.pair(Name(b"Count"), nodes.len() as i32);
                    if let Some(&f) = root_children.first() {
                        d.pair(Name(b"First"), node_refs[f]);
                    }
                    if let Some(&l) = root_children.last() {
                        d.pair(Name(b"Last"), node_refs[l]);
                    }
                }

                // Write each outline item
                for (ni, node) in nodes.iter().enumerate() {
                    let entry = &entries[node.entry_idx];
                    let my_ref = node_refs[ni];

                    let siblings: &[usize] = if let Some(p) = node.parent {
                        &nodes[p].children
                    } else {
                        &root_children
                    };
                    let sib_pos = siblings.iter().position(|&i| i == ni).unwrap_or(0);
                    let parent_ref = node.parent.map(|p| node_refs[p]).unwrap_or(outline_root);
                    let page_ref = page_refs_snap
                        .get(entry.page_idx)
                        .copied()
                        .unwrap_or_else(|| *page_refs_snap.first().unwrap());
                    let desc_count = count_outline_descendants(&nodes, ni) as i32;

                    let mut d = pdf.indirect(my_ref).dict();
                    d.pair(Name(b"Title"), TextStr(&entry.title));
                    d.pair(Name(b"Parent"), parent_ref);
                    if sib_pos > 0 {
                        d.pair(Name(b"Prev"), node_refs[siblings[sib_pos - 1]]);
                    }
                    if sib_pos + 1 < siblings.len() {
                        d.pair(Name(b"Next"), node_refs[siblings[sib_pos + 1]]);
                    }
                    if !node.children.is_empty() {
                        d.pair(Name(b"First"), node_refs[node.children[0]]);
                        d.pair(Name(b"Last"), node_refs[*node.children.last().unwrap()]);
                        d.pair(Name(b"Count"), desc_count);
                    }
                    // [page /XYZ 0 y 0] — navigate to heading position (zoom = 0 = unchanged)
                    d.insert(Name(b"Dest"))
                        .array()
                        .item(page_ref)
                        .item(Name(b"XYZ"))
                        .item(0.0f32)
                        .item(entry.y_pt)
                        .item(0.0f32);
                }

                Some(outline_root)
            }
        };

        // ── Deferred link annotations ─────────────────────────────────────────
        {
            let deferred_links = std::mem::take(&mut self.deferred_links);
            if !deferred_links.is_empty() {
                let page_refs_snap: Vec<Ref> = self.page_refs.clone();
                let pdf = self.pdf.as_mut().expect("pdf not finished");

                for dl in &deferred_links {
                    let (dest_page_idx, dest_y_pt) = heading_lookup
                        .get(&dl.dest_title)
                        .copied()
                        .unwrap_or_else(|| (dl.dest_page_estimate.saturating_sub(1) as usize, 0.0));

                    let page_ref = page_refs_snap
                        .get(dest_page_idx)
                        .copied()
                        .unwrap_or_else(|| *page_refs_snap.first().unwrap());

                    let (x1, y1, x2, y2) = dl.rect_pt;

                    let mut annot = pdf.indirect(dl.annot_ref).dict();
                    annot.pair(Name(b"Type"), Name(b"Annot"));
                    annot.pair(Name(b"Subtype"), Name(b"Link"));
                    annot.pair(Name(b"Rect"), Rect::new(x1, y1, x2, y2));
                    annot
                        .insert(Name(b"Border"))
                        .array()
                        .item(0.0f32)
                        .item(0.0f32)
                        .item(0.0f32);
                    {
                        let mut act = annot.insert(Name(b"A")).dict();
                        act.pair(Name(b"S"), Name(b"GoTo"));
                        act.insert(Name(b"D"))
                            .array()
                            .item(page_ref)
                            .item(Name(b"XYZ"))
                            .item(0.0f32)
                            .item(dest_y_pt)
                            .item(0.0f32);
                    }
                }
            }
        }

        // ── Catalog (deferred so PDF/A metadata + AcroForm refs are available) ─
        {
            let pdfa = self.pdfa;
            let pdfu2 = self.pdfu2;

            // Allocate metadata refs before the catalog borrow.
            let (icc_ref, meta_ref) = if pdfa || pdfu2 {
                let r1 = if pdfa { Some(self.alloc.bump()) } else { None };
                let r2 = self.alloc.bump();
                (r1, Some(r2))
            } else {
                (None, None)
            };

            let pdf = self.pdf.as_mut().expect("pdf not finished");

            // PDF/UA-2: PDF 2.0 header + trailer file ID (ISO 14289-2 §6.1).
            if pdfu2 {
                pdf.set_version(2, 0);
                let file_id = pdfu2_file_id(&self.title);
                pdf.set_file_id((file_id.clone(), file_id));
            }

            let ua_active = !self.ua_lang.is_empty();
            if let Some(mr) = meta_ref {
                if pdfa {
                    if let Some(ir) = icc_ref {
                        pdf.icc_profile(ir, SRGB_V2_ICC).n(3);
                    }
                    let xmp = build_xmp_pdfa(&self.title, self.pdfa_part);
                    pdf.metadata(mr, xmp.as_bytes());
                } else if pdfu2 {
                    let xmp = build_xmp_pdfu2(&self.title);
                    pdf.metadata(mr, xmp.as_bytes());
                }
            }

            let mut cat = pdf.catalog(self.catalog_ref);
            cat.pages(self.pages_ref);

            if let Some(mr) = meta_ref {
                cat.metadata(mr);
            }

            if let Some(ref snap) = sig_snap {
                let mut acro = cat.insert(Name(b"AcroForm")).dict();
                acro.pair(Name(b"SigFlags"), 3i32);
                acro.insert(Name(b"Fields")).array().item(snap.widget_ref);
            }

            if let (Some(ir), true) = (icc_ref, pdfa) {
                cat.output_intents()
                    .push()
                    .subtype(OutputIntentSubtype::PDFA)
                    .output_condition_identifier(TextStr("sRGB IEC61966-2.1"))
                    .dest_output_profile(ir);
            }

            // PDF/UA-2: MarkInfo, /Lang, ViewerPreferences, StructTreeRoot
            if ua_active {
                cat.mark_info().marked(true);
                cat.lang(TextStr(&self.ua_lang));
                // ISO 14289-2 §6.9: viewer must display document title (not filename).
                cat.viewer_preferences().display_doc_title(true);
            }
            if let Some(str_root_ref) = self.struct_tree_root_ref {
                cat.struct_tree_root().child(str_root_ref);
            }
            if let Some(or) = outline_root_ref {
                cat.pair(Name(b"Outlines"), or);
                cat.pair(Name(b"PageMode"), Name(b"UseOutlines"));
            }
        }

        let pdf = self.pdf.take().expect("pdf not finished");
        Ok(pdf.finish())
    }

    fn save_state(&mut self) {
        self.content.save_state();
    }

    fn restore_state(&mut self) {
        self.content.restore_state();
    }

    fn set_opacity(&mut self, opacity: f64) -> crate::Result<()> {
        let opacity_u8 = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
        if !self.opacity_gs.contains_key(&opacity_u8) {
            let gs_ref = self.alloc.bump();
            self.opacity_gs.insert(opacity_u8, gs_ref);
        }
        let name = format!("GS{opacity_u8}");
        self.content.set_parameters(Name(name.as_bytes()));
        Ok(())
    }

    fn reset_opacity(&mut self) {
        if !self.opacity_gs.contains_key(&255u8) {
            let gs_ref = self.alloc.bump();
            self.opacity_gs.insert(255u8, gs_ref);
        }
        self.content.set_parameters(Name(b"GS255"));
    }

    fn begin_tagged_content(&mut self, tag_name: &[u8], mcid: u32) {
        self.content
            .begin_marked_content_with_properties(Name(tag_name))
            .properties()
            .identify(mcid as i32);
    }

    fn end_tagged_content(&mut self) {
        self.content.end_marked_content();
    }

    fn begin_artifact_content(&mut self) {
        self.content.begin_marked_content(Name(b"Artifact"));
    }

    fn current_page_idx(&self) -> usize {
        self.page_refs.len().saturating_sub(1)
    }

    fn add_outline_entry(&mut self, title: &str, level: u8, page_idx: usize, y_mm: f64) {
        self.outlines.push(OutlineEntry {
            title: title.to_string(),
            level,
            page_idx,
            y_pt: mm_to_pt(y_mm),
        });
    }

    fn add_link_annotation(
        &mut self,
        x1_mm: f64,
        y1_mm: f64,
        x2_mm: f64,
        y2_mm: f64,
        dest_title: &str,
        dest_page_estimate: u32,
    ) {
        self.current_page_links.push(PendingLink {
            rect_pt: (
                mm_to_pt(x1_mm),
                mm_to_pt(y1_mm),
                mm_to_pt(x2_mm),
                mm_to_pt(y2_mm),
            ),
            dest_title: dest_title.to_string(),
            dest_page_estimate,
        });
    }

    fn write_structure_tree(&mut self, events: &[crate::compliance::ua::StructEvent], lang: &str) {
        use crate::compliance::ua::{StructEvent, StructTag};

        self.ua_lang = lang.to_string();
        if events.is_empty() {
            return;
        }
        let page_refs: Vec<Ref> = self.page_refs.clone();

        // Wrap in a Document root if the events don't already start with Document
        let has_document_root = matches!(
            events.first(),
            Some(StructEvent::BeginGroup {
                tag: StructTag::Document,
                ..
            })
        );

        // Allocate root ref
        let root_ref = self.alloc.bump();
        self.struct_tree_root_ref = Some(root_ref);

        if has_document_root {
            // Write events starting from the existing Document root
            let mut idx = 0usize;
            write_struct_element(
                events,
                &mut idx,
                root_ref, // StructTreeRoot is the logical parent
                &page_refs,
                &mut self.alloc,
                &mut self.pages_chunk,
            );
        } else {
            // Wrap all events in an implicit Document element
            let doc_elem_ref = root_ref; // reuse root_ref as Document element

            let mut child_refs: Vec<Ref> = Vec::new();
            let mut mcid_children: Vec<(u32, Ref)> = Vec::new();
            let mut idx = 0usize;

            while idx < events.len() {
                match &events[idx] {
                    StructEvent::BeginGroup { .. } => {
                        let child_ref = write_struct_element(
                            events,
                            &mut idx,
                            doc_elem_ref,
                            &page_refs,
                            &mut self.alloc,
                            &mut self.pages_chunk,
                        );
                        child_refs.push(child_ref);
                    }
                    StructEvent::ContentRef { mcid, page_idx } => {
                        let pr = page_refs
                            .get(*page_idx)
                            .copied()
                            .unwrap_or_else(|| page_refs[0]);
                        mcid_children.push((*mcid, pr));
                        idx += 1;
                    }
                    StructEvent::EndGroup => {
                        idx += 1;
                    }
                }
            }

            // Write Document element
            let mut elem = self.pages_chunk.struct_element(doc_elem_ref);
            elem.kind(StructRole::Document);
            elem.parent(root_ref); // parent is the StructTreeRoot placeholder ref
            if !child_refs.is_empty() || !mcid_children.is_empty() {
                let mut kids = elem.children();
                for cr in &child_refs {
                    kids.struct_element(*cr);
                }
                for (mcid, pr) in &mcid_children {
                    kids.marked_content_ref()
                        .marked_content_id(*mcid as i32)
                        .page(*pr);
                }
            }
        }
    }
}

/// Recursively writes a structure element and its children to the chunk.
/// Returns the Ref of the written element.
fn write_struct_element(
    events: &[crate::compliance::ua::StructEvent],
    idx: &mut usize,
    parent_ref: Ref,
    page_refs: &[Ref],
    alloc: &mut Ref,
    chunk: &mut Chunk,
) -> Ref {
    use crate::compliance::ua::StructEvent;

    let (tag, alt) = match &events[*idx] {
        StructEvent::BeginGroup { tag, alt } => (tag.clone(), alt.clone()),
        _ => {
            *idx += 1;
            return alloc.bump(); // should not happen
        }
    };
    *idx += 1;

    let elem_ref = alloc.bump();

    // Collect children first (recursive)
    let mut struct_children: Vec<Ref> = Vec::new();
    let mut mcid_children: Vec<(u32, Ref)> = Vec::new();

    loop {
        if *idx >= events.len() {
            break;
        }
        match &events[*idx] {
            StructEvent::EndGroup => {
                *idx += 1;
                break;
            }
            StructEvent::BeginGroup { .. } => {
                let child_ref =
                    write_struct_element(events, idx, elem_ref, page_refs, alloc, chunk);
                struct_children.push(child_ref);
            }
            StructEvent::ContentRef { mcid, page_idx } => {
                let pr = page_refs
                    .get(*page_idx)
                    .copied()
                    .unwrap_or_else(|| *page_refs.first().unwrap_or(&Ref::new(1)));
                mcid_children.push((*mcid, pr));
                *idx += 1;
            }
        }
    }

    // Write this element (children already written)
    let mut elem = chunk.struct_element(elem_ref);

    // Map our StructTag to pdf-writer's StructRole (or custom name)
    match tag_to_struct_role(&tag) {
        Some(role) => {
            elem.kind(role);
        }
        None => {
            elem.custom_kind(Name(tag.pdf_name().as_bytes()));
        }
    }

    elem.parent(parent_ref);

    if let Some(ref alt_text) = alt {
        elem.alt(TextStr(alt_text));
    }

    if !struct_children.is_empty() || !mcid_children.is_empty() {
        let mut kids = elem.children();
        for cr in &struct_children {
            kids.struct_element(*cr);
        }
        for (mcid, pr) in &mcid_children {
            kids.marked_content_ref()
                .marked_content_id(*mcid as i32)
                .page(*pr);
        }
    }

    elem_ref
}

fn tag_to_struct_role(tag: &crate::compliance::ua::StructTag) -> Option<StructRole> {
    use crate::compliance::ua::StructTag;
    match tag {
        StructTag::Document => Some(StructRole::Document),
        StructTag::Part => Some(StructRole::Part),
        StructTag::Sect => Some(StructRole::Sect),
        StructTag::Div => Some(StructRole::Div),
        StructTag::BlockQuote => Some(StructRole::BlockQuote),
        StructTag::Caption => Some(StructRole::Caption),
        StructTag::TOC => Some(StructRole::TOC),
        StructTag::TOCI => Some(StructRole::TOCI),
        StructTag::Index => Some(StructRole::Index),
        StructTag::P => Some(StructRole::P),
        StructTag::H1 => Some(StructRole::H1),
        StructTag::H2 => Some(StructRole::H2),
        StructTag::H3 => Some(StructRole::H3),
        StructTag::H4 => Some(StructRole::H4),
        StructTag::H5 => Some(StructRole::H5),
        StructTag::H6 => Some(StructRole::H6),
        StructTag::L => Some(StructRole::L),
        StructTag::LI => Some(StructRole::LI),
        StructTag::Lbl => Some(StructRole::Lbl),
        StructTag::LBody => Some(StructRole::LBody),
        StructTag::Table => Some(StructRole::Table),
        StructTag::TR => Some(StructRole::TR),
        StructTag::TH => Some(StructRole::TH),
        StructTag::TD => Some(StructRole::TD),
        StructTag::THead => Some(StructRole::THead),
        StructTag::TBody => Some(StructRole::TBody),
        StructTag::TFoot => Some(StructRole::TFoot),
        StructTag::Span => Some(StructRole::Span),
        StructTag::Code => Some(StructRole::Code),
        StructTag::Link => Some(StructRole::Link),
        StructTag::Annot => Some(StructRole::Annot),
        StructTag::Figure => Some(StructRole::Figure),
        StructTag::Formula => Some(StructRole::Formula),
        StructTag::Form => Some(StructRole::Form),
        StructTag::Note => Some(StructRole::Note),
        StructTag::Ruby => Some(StructRole::Ruby),
        StructTag::Warichu => Some(StructRole::Warichu),
        StructTag::RB => Some(StructRole::RB),
        StructTag::RT => Some(StructRole::RT),
        StructTag::RP => Some(StructRole::RP),
        // PDF 2.0 tags not in pdf-writer 0.12 StructRole → custom_kind
        _ => None,
    }
}

// ── Public font-tools API (Eixo B) ───────────────────────────────────────────

/// Subsets a TTF/OTF font to the glyphs in `used_glyphs` and returns the
/// reduced font bytes.  Falls back to the full font if subsetting fails.
///
/// Uses the `subsetter` crate (same algorithm as the internal renderer).
pub fn subset_font(font_bytes: &[u8], used_glyphs: &std::collections::HashSet<u16>) -> Vec<u8> {
    let mut remapper = subsetter::GlyphRemapper::new();
    remapper.remap(0); // always keep .notdef
    for &gid in used_glyphs {
        remapper.remap(gid);
    }
    subsetter::subset(font_bytes, 0, &remapper).unwrap_or_else(|_| font_bytes.to_vec())
}

/// Extracts the CFF (Compact Font Format) table from an OTF font, if present.
///
/// CFF outlines are ~30 % smaller than TrueType outlines.  For TTF-only fonts
/// (no `CFF ` table) the original bytes are returned unchanged.
pub fn to_cff_if_possible(font_bytes: &[u8]) -> Vec<u8> {
    let face = match Face::parse(font_bytes, 0) {
        Ok(f) => f,
        Err(_) => return font_bytes.to_vec(),
    };
    // Tag "CFF " — note trailing space
    if let Some(cff_data) = face.raw_face().table(ttf_parser::Tag::from_bytes(b"CFF ")) {
        return cff_data.to_vec();
    }
    font_bytes.to_vec()
}

/// Generates a ToUnicode CMap stream that maps glyph IDs to Unicode characters.
///
/// Scans the font's cmap table (BMP range U+0020–U+024F plus punctuation and
/// currency symbols) to build a reverse GID→Unicode map, then emits a PDF
/// `bfchar` CMap for only the glyphs in `used_glyphs`.
///
/// Falls back to the identity CMap if the font cannot be parsed.
pub fn generate_to_unicode_cmap(
    font_bytes: &[u8],
    used_glyphs: &std::collections::HashSet<u16>,
) -> Vec<u8> {
    if used_glyphs.is_empty() {
        return IDENTITY_TOUNICODE.to_vec();
    }

    let face = match Face::parse(font_bytes, 0) {
        Ok(f) => f,
        Err(_) => return IDENTITY_TOUNICODE.to_vec(),
    };

    // Build reverse map GID → char by scanning common BMP ranges.
    // Covers Basic Latin, Latin-1 Supplement, Latin Extended-A/B,
    // General Punctuation, Currency Symbols — sufficient for Portuguese text.
    let scan_ranges: &[(u32, u32)] = &[
        (0x0020, 0x024F), // Basic Latin → Latin Extended-B
        (0x2000, 0x206F), // General Punctuation
        (0x20A0, 0x20CF), // Currency Symbols
        (0x2100, 0x214F), // Letterlike Symbols
    ];
    let mut reverse: std::collections::HashMap<u16, char> =
        std::collections::HashMap::with_capacity(used_glyphs.len() * 2);

    for &(lo, hi) in scan_ranges {
        for cp in lo..=hi {
            if let Some(c) = char::from_u32(cp) {
                if let Some(gid) = face.glyph_index(c) {
                    reverse.entry(gid.0).or_insert(c);
                }
            }
        }
    }

    let mut mappings: Vec<(u16, char)> = used_glyphs
        .iter()
        .filter_map(|&gid| reverse.get(&gid).map(|&c| (gid, c)))
        .collect();
    mappings.sort_unstable_by_key(|&(gid, _)| gid);

    if mappings.is_empty() {
        return IDENTITY_TOUNICODE.to_vec();
    }

    let mut cmap = String::with_capacity(512);
    cmap.push_str("/CIDInit /ProcSet findresource begin\n");
    cmap.push_str("12 dict begin\n");
    cmap.push_str("begincmap\n");
    cmap.push_str("/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n");
    cmap.push_str("/CMapName /Adobe-Identity-UCS def\n");
    cmap.push_str("/CMapType 2 def\n");
    cmap.push_str("1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n");
    cmap.push_str(&format!("{} beginbfchar\n", mappings.len()));
    for (gid, ch) in &mappings {
        // Write Unicode as UTF-16BE hex (BMP-only — surrogate pairs not needed here)
        let cp = *ch as u32;
        if cp <= 0xFFFF {
            cmap.push_str(&format!("<{gid:04X}> <{cp:04X}>\n"));
        } else {
            // Encode as surrogate pair
            let cp = cp - 0x10000;
            let hi = 0xD800 + (cp >> 10);
            let lo = 0xDC00 + (cp & 0x3FF);
            cmap.push_str(&format!("<{gid:04X}> <{hi:04X}{lo:04X}>\n"));
        }
    }
    cmap.push_str("endbfchar\n");
    cmap.push_str("endcmap\n");
    cmap.push_str("CMapName currentdict /CMap defineresource pop\n");
    cmap.push_str("end\nend\n");
    cmap.into_bytes()
}

/// Encodes a text string as UTF-16-BE with a leading BOM (0xFE 0xFF).
///
/// Per PDF spec §9.4.4, strings in Type0 CIDFont streams must be UTF-16-BE
/// with BOM when the encoding is Identity-H.  The normordis-pdf rendering
/// pipeline uses GID-based encoding via `text_to_gid_bytes()` (more precise
/// for subsetted fonts); this function is provided as a utility for callers
/// that need raw UTF-16-BE encoding (e.g., XMP metadata or external PDF tools).
///
/// # Example
/// ```
/// use normordis_pdf::encode_for_identity_h;
/// let enc = encode_for_identity_h("Olá");
/// assert_eq!(&enc[0..2], &[0xFE, 0xFF]); // BOM
/// ```
pub fn encode_for_identity_h(text: &str) -> Vec<u8> {
    let mut bytes: Vec<u8> = vec![0xFE, 0xFF];
    for unit in text.encode_utf16() {
        bytes.push((unit >> 8) as u8);
        bytes.push((unit & 0xFF) as u8);
    }
    bytes
}
