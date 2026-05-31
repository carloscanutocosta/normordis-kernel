use crate::{
    compliance::ua::AccessibilityConfig,
    document::{CompressionLevel, Document, PdfStandard},
    elements::{
        fixed_image::{FixedImageBox, ImageFit},
        fixed_line::FixedLineElement,
        fixed_text::{FixedTextBox, VerticalAlign},
        footer::{PageFooter, SectionedFooter},
        form::FormField,
        header::{InstitutionalHeader, SectionedHeader},
        paragraph::ParagraphContent,
        Element,
    },
    fonts::FontRegistry,
    layout::{FixedBox, TextAlign},
    richtext,
    signing::{SignatureConfig, SignatureOptions},
    styles::{DocumentStyle, RgbColor, SecurityClassification, TraceabilityMetadata, Watermark},
    template, NormaxisPdfError, Result, VERSION,
};

/// Fluent builder for constructing and rendering a PDF document.
///
/// `DocumentBuilder` is the primary entry point for generating PDF documents.
/// It supports both programmatic construction and declarative NDT templates.
///
/// # Example
///
/// ```rust
/// use normordis_pdf::{DocumentBuilder, Section, Paragraph, TextAlign};
///
/// let pdf = DocumentBuilder::new("My Document")
///     .push(Section::new("Introduction", 1))
///     .push(Paragraph::new("Some text.").align(TextAlign::Justify))
///     .render_to_bytes()
///     .unwrap();
///
/// assert!(!pdf.is_empty());
/// ```
pub struct DocumentBuilder {
    title: String,
    style: DocumentStyle,
    fonts: FontRegistry,
    header: Option<Box<dyn Element>>,
    sectioned_header: Option<SectionedHeader>,
    footer: Option<Box<dyn Element>>,
    sectioned_footer: Option<SectionedFooter>,
    watermark: Option<Watermark>,
    elements: Vec<Box<dyn Element>>,
    footnotes: Vec<(u32, Vec<String>)>,
    next_footnote_number: u32,
    compression: CompressionLevel,
    standard: PdfStandard,
    traceability: Option<TraceabilityMetadata>,
    accessibility: AccessibilityConfig,
}

impl DocumentBuilder {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            style: DocumentStyle::default(),
            fonts: FontRegistry::default(),
            header: None,
            sectioned_header: None,
            footer: None,
            sectioned_footer: None,
            watermark: None,
            elements: Vec::new(),
            footnotes: Vec::new(),
            next_footnote_number: 1,
            compression: CompressionLevel::Default,
            standard: PdfStandard::Pdf17,
            traceability: None,
            accessibility: AccessibilityConfig::default(),
        }
    }

    /// Adds a footnote definition and returns its auto-assigned number (1-based).
    ///
    /// Pass the text lines that make up the footnote content.
    /// The returned number should be used with [`TextRun::footnote_ref`] to place
    /// the inline reference mark in the body text.
    pub fn add_footnote(&mut self, texts: Vec<impl Into<String>>) -> u32 {
        let number = self.next_footnote_number;
        self.next_footnote_number += 1;
        self.footnotes
            .push((number, texts.into_iter().map(|s| s.into()).collect()));
        number
    }

    /// Adds an AcroForm field element (renders as a visual placeholder).
    pub fn form_field(mut self, field: FormField) -> Self {
        self.elements.push(Box::new(field));
        self
    }

    /// Override the document style (page size, margins, colours, etc.).
    pub fn style(mut self, style: DocumentStyle) -> Self {
        self.style = style;
        self
    }

    /// Provide a pre-populated font registry.
    pub fn fonts(mut self, fonts: FontRegistry) -> Self {
        self.fonts = fonts;
        self
    }

    /// Set the institutional header rendered on each page.
    pub fn header(mut self, header: InstitutionalHeader) -> Self {
        self.header = Some(Box::new(header));
        self.sectioned_header = None;
        self
    }

    /// Set a sectioned header (different header per page type).
    /// Replaces any previously set header.
    pub fn sectioned_header(mut self, h: SectionedHeader) -> Self {
        self.sectioned_header = Some(h);
        self.header = None;
        self
    }

    /// Set the footer rendered at the bottom of each page.
    pub fn footer(mut self, footer: PageFooter) -> Self {
        self.footer = Some(Box::new(footer));
        self.sectioned_footer = None;
        self
    }

    /// Set a sectioned footer (different footer per page type).
    /// Replaces any previously set footer.
    pub fn sectioned_footer(mut self, f: SectionedFooter) -> Self {
        self.sectioned_footer = Some(f);
        self.footer = None;
        self
    }

    /// Sets the output PDF compression level (default: `CompressionLevel::Default`).
    ///
    /// Use `CompressionLevel::None` to disable compression for debugging.
    /// Use `CompressionLevel::Best` for smallest archival files.
    pub fn compression(mut self, level: CompressionLevel) -> Self {
        self.compression = level;
        self
    }

    /// Set the PDF conformance standard.
    ///
    /// Use [`PdfStandard::PdfA1b`] for long-term archival (ISO 19005-1).
    /// [`PdfStandard::Pdf17`] is the default (no conformance requirements).
    pub fn standard(mut self, s: PdfStandard) -> Self {
        self.standard = s;
        self
    }

    /// Enable PDF/A-1b conformance (shorthand for `.standard(PdfStandard::PdfA1b)`).
    ///
    /// Adds an XMP metadata stream and an sRGB `OutputIntent` to the catalog,
    /// satisfying the core requirements of ISO 19005-1 (PDF/A-1b).
    pub fn pdfa(self) -> Self {
        self.standard(PdfStandard::PdfA1b)
    }

    /// Attach traceability metadata (CRA/NIS2 compliance).
    ///
    /// When `classification` is non-`Public` and no explicit watermark is set,
    /// a classification watermark is applied automatically to every page.
    pub fn traceability(mut self, meta: TraceabilityMetadata) -> Self {
        self.traceability = Some(meta);
        self
    }

    /// Configure PDF/UA-2 accessibility (ISO 14289-2:2024).
    ///
    /// Enable tagged PDF output, set the document language, and control
    /// warnings for missing alt text.  Use [`PdfStandard::PdfUa2`] together
    /// with this to emit a fully conformant PDF/UA-2 document.
    pub fn accessibility(mut self, config: AccessibilityConfig) -> Self {
        self.accessibility = config;
        self
    }

    /// Configure the document for digital signature preparation.
    ///
    /// Use [`DocumentBuilder::render_prepared_for_signing`] to produce a
    /// [`PreparedPdf`] whose bytes can be signed externally.
    /// `config` provides metadata (reason, location) embedded in the signature field.
    ///
    /// [`PreparedPdf`]: crate::PreparedPdf
    pub fn sign(self, config: SignatureConfig) -> SigningBuilder {
        SigningBuilder {
            inner: self,
            config,
        }
    }

    /// Adds a diagonal text watermark to every page.
    ///
    /// # Example
    ///
    /// ```rust
    /// use normordis_pdf::{DocumentBuilder, Watermark};
    ///
    /// let pdf = DocumentBuilder::new("Draft")
    ///     .watermark(Watermark::new("RASCUNHO").opacity(0.12))
    ///     .render_to_bytes()?;
    /// # Ok::<(), normordis_pdf::NormaxisPdfError>(())
    /// ```
    pub fn watermark(mut self, wm: Watermark) -> Self {
        self.watermark = Some(wm);
        self
    }

    /// Append any element to the document body.
    pub fn push(mut self, element: impl Element + 'static) -> Self {
        self.elements.push(Box::new(element));
        self
    }

    /// Render the document to a PDF byte vector.
    pub fn render_to_bytes(self) -> Result<Vec<u8>> {
        Document {
            title: self.title,
            style: self.style,
            fonts: self.fonts,
            header: self.header,
            sectioned_header: self.sectioned_header,
            footer: self.footer,
            sectioned_footer: self.sectioned_footer,
            watermark: self.watermark,
            elements: self.elements,
            footnotes: self.footnotes,
            toc_entries: None,
            compression: self.compression,
            standard: self.standard,
            signature: None,
            traceability: self.traceability.clone(),
            accessibility: self.accessibility,
        }
        .render_to_bytes()
    }

    /// Adds a fixed-position text box.  Does not affect the flow cursor.
    pub fn fixed_text(
        mut self,
        box_def: FixedBox,
        content: impl Into<String>,
        alignment: TextAlign,
    ) -> Self {
        self.elements.push(Box::new(FixedTextBox {
            text_box: box_def,
            content: ParagraphContent::Plain(content.into()),
            alignment,
            font_size: None,
            vertical_align: VerticalAlign::Top,
        }));
        self
    }

    /// Adds a fixed-position image.  Does not affect the flow cursor.
    pub fn fixed_image(mut self, box_def: FixedBox, data: Vec<u8>, fit: ImageFit) -> Self {
        self.elements.push(Box::new(FixedImageBox {
            image_box: box_def,
            data,
            fit,
        }));
        self
    }

    /// Adds a fixed-position decorative line.  Does not affect the flow cursor.
    pub fn fixed_line(mut self, x1: f64, y1: f64, x2: f64, y2: f64, color: RgbColor) -> Self {
        self.elements
            .push(Box::new(FixedLineElement::new(x1, y1, x2, y2, color)));
        self
    }

    /// Parse an NDT template + data JSON and append all resulting elements.
    ///
    /// For NDT 2.0.0 templates the `output` block is also applied:
    /// - `output.standard` → sets the PDF conformance standard
    /// - `output.compression` → sets the compression level
    /// - `output.classification` → attaches a `TraceabilityMetadata` (unless one is already set)
    pub fn push_ndt(mut self, template_json: &str, data_json: &str) -> Result<Self> {
        let doc = template::parse_ndt(template_json)
            .map_err(|e| NormaxisPdfError::Template(e.to_string()))?;
        let data = template::parse_ndt_data(data_json)
            .map_err(|e| NormaxisPdfError::Template(e.to_string()))?;

        template::check_version_compatibility(&doc.ndt)
            .map_err(|e| NormaxisPdfError::Template(e.to_string()))?;

        if let Some(ref placeholders) = doc.placeholders {
            template::validator::validate(placeholders, &data)
                .map_err(|e| NormaxisPdfError::Template(e.to_string()))?;
        }

        // Apply NDT 2.0.0 output settings.
        if let Some(ref output) = doc.output {
            if let Some(ref std_str) = output.standard {
                self.standard = match std_str.as_str() {
                    "pdf_a_1b" | "pdf_a1b" => PdfStandard::PdfA1b,
                    "pdf_a_2b" | "pdf_a2b" => PdfStandard::PdfA2b,
                    "pdf_ua2" | "pdf_ua_2" => PdfStandard::PdfUa2,
                    _ => PdfStandard::Pdf17,
                };
            }
            if let Some(ref comp_str) = output.compression {
                self.compression = match comp_str.as_str() {
                    "none" => CompressionLevel::None,
                    "fast" => CompressionLevel::Fast,
                    "best" => CompressionLevel::Best,
                    _ => CompressionLevel::Default,
                };
            }
            // NDT 2.1.0: granular accessibility config
            if !self.accessibility.enabled {
                if let Some(ref acc) = output.accessibility {
                    self.accessibility = acc.clone();
                }
            }

            if self.traceability.is_none() {
                if let Some(ref class_str) = output.classification {
                    let classification = parse_classification(class_str);
                    if classification != SecurityClassification::Public {
                        self.traceability = Some(TraceabilityMetadata {
                            engine_version: VERSION.into(),
                            framework_version: None,
                            entity_id: String::new(),
                            document_ref: output.document_ref.clone(),
                            classification,
                            generated_at: String::new(),
                            ndt_version: doc.ndt.clone(),
                        });
                    }
                }
            }
        }

        let elements = template::render_template(&doc, &data, &self.style)
            .map_err(|e| NormaxisPdfError::Template(e.to_string()))?;
        for el in elements {
            self.elements.push(el);
        }
        Ok(self)
    }

    /// Parse an NCRTF JSON string and append all resulting elements to the document.
    pub fn push_ncrtf(mut self, json: &str) -> Result<Self> {
        let doc = richtext::parse_ncrtf(json)?;
        let elements = richtext::ncrtf_to_elements(&doc, &self.style);
        for el in elements {
            self.elements.push(el);
        }
        Ok(self)
    }

    /// Render the document and write it to a file.
    pub fn render_to_file(self, path: impl AsRef<std::path::Path>) -> Result<()> {
        Document {
            title: self.title,
            style: self.style,
            fonts: self.fonts,
            header: self.header,
            sectioned_header: self.sectioned_header,
            footer: self.footer,
            sectioned_footer: self.sectioned_footer,
            watermark: self.watermark,
            elements: self.elements,
            footnotes: self.footnotes,
            toc_entries: None,
            compression: self.compression,
            standard: self.standard,
            signature: None,
            traceability: self.traceability.clone(),
            accessibility: self.accessibility,
        }
        .render_to_file(path)
    }

    /// Render and return a [`PreparedPdf`] ready for external PKCS#7 signing.
    ///
    /// The returned value exposes the byte ranges that must be signed and an
    /// [`PreparedPdf::embed_signature`] method to inject the DER-encoded
    /// PKCS#7 blob and obtain the final signed PDF bytes.
    pub fn render_prepared_for_signing(
        self,
        opts: SignatureOptions,
    ) -> Result<crate::signing::PreparedPdf> {
        Document {
            title: self.title,
            style: self.style,
            fonts: self.fonts,
            header: self.header,
            sectioned_header: self.sectioned_header,
            footer: self.footer,
            sectioned_footer: self.sectioned_footer,
            watermark: self.watermark,
            elements: self.elements,
            footnotes: self.footnotes,
            toc_entries: None,
            compression: self.compression,
            standard: self.standard,
            signature: None,
            traceability: self.traceability,
            accessibility: self.accessibility,
        }
        .render_prepared_for_signing(opts)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_classification(s: &str) -> SecurityClassification {
    match s.to_ascii_lowercase().as_str() {
        "interno" | "internal" => SecurityClassification::Internal,
        "confidencial" | "confidential" => SecurityClassification::Confidential,
        "reservado" | "reserved" => SecurityClassification::Reserved,
        _ => SecurityClassification::Public,
    }
}

// ── SigningBuilder ────────────────────────────────────────────────────────────

/// A builder pre-configured with a [`SignatureConfig`].
///
/// Obtained via [`DocumentBuilder::sign`].
pub struct SigningBuilder {
    inner: DocumentBuilder,
    config: SignatureConfig,
}

impl SigningBuilder {
    /// Render and return a [`PreparedPdf`] using the stored [`SignatureConfig`].
    pub fn render_prepared_for_signing(self) -> Result<crate::signing::PreparedPdf> {
        let opts = self.config.to_options();
        self.inner.render_prepared_for_signing(opts)
    }

    /// Render, sign with the provided PKCS#7 DER blob, and return the signed PDF.
    pub fn render_signed(self, pkcs7_der: &[u8]) -> Result<Vec<u8>> {
        let config = self.config.clone();
        let prepared = self.render_prepared_for_signing()?;
        crate::signing::sign_pdf(prepared, &config, pkcs7_der)
    }
}
