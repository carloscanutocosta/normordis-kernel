//! # normordis-pdf
//!
//! Institutional PDF generation library for NORMAXIS mini-apps.
//!
//! Generates professional PDF documents for Portuguese public administration,
//! with support for:
//! - Flow and Fixed Box layout modes
//! - NORMAXIS Canonical Rich Text Format (NCRTF v1.1)
//! - NORMAXIS Document Template format (NDT v1.3.0)
//! - Named paragraph/table styles with inheritance (equivalent to Word Styles)
//! - Tab stops (left, right, center, decimal) with leader characters
//! - TTF/OTF font loading with real glyph metrics (rustybuzz + ttf-parser)
//! - Left, Justify, Center, Right text alignment
//!
//! ## Quick Start
//!
//! ```rust
//! use normordis_pdf::{DocumentBuilder, Section, Paragraph, TextAlign};
//!
//! let pdf = DocumentBuilder::new("Annual Report")
//!     .push(Section::new("1. Introduction", 1))
//!     .push(Paragraph::new("Document body text.").align(TextAlign::Justify))
//!     .render_to_bytes()?;
//! # Ok::<(), normordis_pdf::NormaxisPdfError>(())
//! ```
//!
//! ## Named Styles
//!
//! ```rust
//! use normordis_pdf::{DocumentBuilder, Paragraph, Section};
//!
//! let pdf = DocumentBuilder::new("Styled Document")
//!     .push(Section::new("Introduction", 1))
//!     .push(Paragraph::new("Caption text.").style("caption"))
//!     .render_to_bytes()?;
//! # Ok::<(), normordis_pdf::NormaxisPdfError>(())
//! ```
//!
//! ## NDT Templates
//!
//! ```rust
//! use normordis_pdf::DocumentBuilder;
//!
//! let data = r#"{"ndt_data":"1.0.0","data":{"entity":"Câmara Municipal"}}"#;
//! let template = r#"{"ndt":"1.0.0","body":[{"type":"paragraph","text":"{{entity}}"}]}"#;
//!
//! let pdf = DocumentBuilder::new("Ofício")
//!     .push_ndt(template, data)?
//!     .render_to_bytes()?;
//! # Ok::<(), normordis_pdf::NormaxisPdfError>(())
//! ```

// ── Modules ───────────────────────────────────────────────────────────────────

pub mod error;
pub mod styles;
pub mod fonts;
pub mod page;
pub mod backend;
pub mod document;
pub mod builder;
pub mod layout;
pub mod elements;
pub mod richtext;
pub mod template;
pub mod signing;
pub mod ndf;
pub mod compliance;

// ── Error handling ────────────────────────────────────────────────────────────

pub use error::{NormaxisPdfError, Result};

// ── Digital signing ───────────────────────────────────────────────────────────

pub use signing::{PreparedPdf, SignatureConfig, SignatureField, SignatureOptions, sign_pdf};

// ── Styles ────────────────────────────────────────────────────────────────────

pub use styles::{
    default_named_styles, DocumentStyle, NamedStyle, Orientation, PageSize, ResolvedStyle,
    RgbColor, SecurityClassification, StyleResolver, TraceabilityMetadata, Watermark,
};

// ── Fonts ─────────────────────────────────────────────────────────────────────

pub use fonts::{
    liberation_sans_family, liberation_serif_family, liberation_mono_family,
    FontData, FontVariants, ShapedGlyph,
    // v1.3.x backward-compatibility aliases
    FontFamily, FontVariant,
    FontRegistry,
};

// ── Page ─────────────────────────────────────────────────────────────────────

pub use page::PageLayout;

// ── Layout ───────────────────────────────────────────────────────────────────

pub use layout::{
    AppliedStyle, BorderStyle, BoxBorder, DecorationLine, FixedBox, GlyphUsageTracker,
    HighlightColor, KnuthPlassOptimizer, LayoutResult, LineBox, LineBreakingMode,
    LineSegment, OpenTypeFeatures, OverflowPolicy, PageFlow, TabStop, TabStopAlign,
    TextAlign, TextDecoration, TextLayoutEngine, TextRun, WordBox,
};

// ── Builder / Document ───────────────────────────────────────────────────────

pub use builder::{DocumentBuilder, SigningBuilder};
pub use backend::{FontRef, ImageRef, PdfBackend};
pub use backend::pdf_writer_backend::encode_for_identity_h;
pub use document::{CompressionLevel, Document, PdfStandard};

// ── Elements — Flow ──────────────────────────────────────────────────────────

pub use elements::{
    footnote::{FootnoteMarkStyle, FootnoteRef, FOOTNOTE_SEPARATOR_THICKNESS_MM},
    footer::{PageFooter, SectionedFooter},
    form::{
        CheckBoxDef, ComboBoxDef, FieldRect, FormField, ListBoxDef,
        RadioButtonDef, TextFieldDef,
    },
    header::{InstitutionalHeader, SectionedHeader},
    image::ImageElement,
    list::{BulletList, CheckList, CheckListItem, ListItem, ListItemElement, OrderedList},
    page_break::PageBreakElement,
    paragraph::{Paragraph, ParagraphBorder, ParagraphContent},
    section::Section,
    section_break::{Orientation as SectionOrientation, SectionBreak, SectionMargins},
    spacer::{HorizontalRuleElement, Spacer},
    table::{
        BorderLineStyle, CellBorder, CellBorders, CellPadding, RowHeight,
        Table, TableBuilder, TableCell, TableRow, TableStyle,
    },
    toc::{TableOfContents, TocEntry},
    Element, LayoutMode, RenderContext, RenderResult,
};

// ── Elements — Fixed ─────────────────────────────────────────────────────────

pub use elements::fixed::{FixedImageBox, FixedLineElement, FixedTextBox, ImageFit, VerticalAlign};

// ── Rich text ────────────────────────────────────────────────────────────────

pub use richtext::{ncrtf_to_elements, parse_ncrtf, NcrtfDocument};

// ── Templates ────────────────────────────────────────────────────────────────

pub use template::{
    parse_ndt, parse_ndt_data, render as render_ndt,
    serialize_ndt_json, serialize_ndt_toml,
    NdtDocument, TemplateError,
    ENGINE_NDT_DATA_VERSION, ENGINE_NDT_VERSION,
    resolve_runtime_fields, RuntimeContext,
    check_version_compatibility,
};

// ── NDT 2.0.0 types ───────────────────────────────────────────────────────────

pub use template::model::{NdtOutput, NdtSignature, NdtSignatureField};

// ── NDF pipeline ─────────────────────────────────────────────────────────────

pub use template::{
    compile_ndt, parse_ndf, render_ndf, render_ndf_prepared_for_signing,
    verify_ndf, CompileOptions,
};

// ── NDF types ─────────────────────────────────────────────────────────────────

pub use ndf::{
    canonical_hash,
    Actor, AuditEvent, EventType,
    IntegrityFailure, IntegrityReport,
    NdfAudit, NdfDocument, NdfIntegrity,
    NdfMeta, NdfMetaNumbering, NdfOrigin, NdfOutput, NdfRevision, NdfRevisionRef, NdfSignature,
};

// ── NCRTF 1.3.0 types ────────────────────────────────────────────────────────

pub use richtext::marks::MarkValue as NcrtfMark;
pub use richtext::model::ImageBlock as NcrtfImage;

// ── Version constants ─────────────────────────────────────────────────────────

/// Version of the normordis-pdf library.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// ── Accessibility / PDF/UA-2 ─────────────────────────────────────────────────

pub use compliance::ua::{
    AccessibilityConfig, ArtifactType, StructEvent, StructTag, StructureTree,
    UaError, UaValidator, UaWarning,
};

/// NDT format version supported by this release.
pub const NDT_VERSION: &str = "2.1.0";

/// PDF backend crate powering the output engine.
pub const PDF_BACKEND: &str = "pdf-writer";

/// NDF format version produced by this release.
pub const NDF_VERSION: &str = ndf::NDF_VERSION;

/// NCRTF format version supported by this release.
pub const NCRTF_VERSION: &str = richtext::NCRTF_VERSION;
