use serde::{Deserialize, Serialize};

/// Configuration for PDF/UA-2 accessibility conformance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityConfig {
    /// Enable PDF/UA-2 structure tree generation. Default: false.
    pub enabled: bool,
    /// Document language for the PDF catalog (BCP 47). Default: "pt-PT".
    pub lang: String,
    /// Warn instead of error when an image has no alt text. Default: true.
    pub warn_missing_alt: bool,
    /// Treat FixedBox elements as Artifact unless they have an explicit role. Default: true.
    pub fixed_box_default_artifact: bool,
}

impl Default for AccessibilityConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            lang: "pt-PT".into(),
            warn_missing_alt: true,
            fixed_box_default_artifact: true,
        }
    }
}

/// PDF structure element tags (ISO 32000-2 §14.8.4 + PDF/UA-2 new tags).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructTag {
    // Grouping elements
    Document,
    DocumentFragment,
    Part,
    Sect,
    Div,
    // Block-level elements
    P,
    H,
    H1, H2, H3, H4, H5, H6,
    BlockQuote,
    Caption,
    Index,
    TOC,
    TOCI,
    Aside,
    // List elements
    L,
    LI,
    Lbl,
    LBody,
    // Table elements
    Table,
    TR,
    TH,
    TD,
    THead,
    TBody,
    TFoot,
    // Inline elements
    Span,
    Em,
    Strong,
    Link,
    Annot,
    Ruby,
    RB, RT, RP,
    Warichu,
    // Illustration elements
    Figure,
    Formula,
    Form,
    // Note elements
    FENote,
    Note,
    // Code
    Code,
    // Artifact (decorative content)
    Artifact,
}

impl StructTag {
    pub fn pdf_name(&self) -> &'static str {
        match self {
            Self::Document => "Document",
            Self::DocumentFragment => "DocumentFragment",
            Self::Part => "Part",
            Self::Sect => "Sect",
            Self::Div => "Div",
            Self::P => "P",
            Self::H => "H",
            Self::H1 => "H1",
            Self::H2 => "H2",
            Self::H3 => "H3",
            Self::H4 => "H4",
            Self::H5 => "H5",
            Self::H6 => "H6",
            Self::BlockQuote => "BlockQuote",
            Self::Caption => "Caption",
            Self::Index => "Index",
            Self::TOC => "TOC",
            Self::TOCI => "TOCI",
            Self::Aside => "Aside",
            Self::L => "L",
            Self::LI => "LI",
            Self::Lbl => "Lbl",
            Self::LBody => "LBody",
            Self::Table => "Table",
            Self::TR => "TR",
            Self::TH => "TH",
            Self::TD => "TD",
            Self::THead => "THead",
            Self::TBody => "TBody",
            Self::TFoot => "TFoot",
            Self::Span => "Span",
            Self::Em => "Em",
            Self::Strong => "Strong",
            Self::Link => "Link",
            Self::Annot => "Annot",
            Self::Ruby => "Ruby",
            Self::RB => "RB",
            Self::RT => "RT",
            Self::RP => "RP",
            Self::Warichu => "Warichu",
            Self::Figure => "Figure",
            Self::Formula => "Formula",
            Self::Form => "Form",
            Self::FENote => "FENote",
            Self::Note => "Note",
            Self::Code => "Code",
            Self::Artifact => "Artifact",
        }
    }
}

/// Type of PDF artifact for screen reader classification.
#[derive(Debug, Clone, Copy)]
pub enum ArtifactType {
    Header,
    Footer,
    Background,
    Decorative,
}

// ── Structure tree events (flat list, collected during rendering) ─────────────

/// A single event in the structure tree event log.
#[derive(Debug, Clone)]
pub enum StructEvent {
    /// Begin a structure element group.
    BeginGroup { tag: StructTag, alt: Option<String> },
    /// Reference a marked content sequence (MCID on a page).
    ContentRef { mcid: u32, page_idx: usize },
    /// End the current structure element group.
    EndGroup,
}

/// Structure tree events collected during rendering.
///
/// The flat event list is converted to a PDF structure tree during `finish()`.
pub struct StructureTree {
    pub events: Vec<StructEvent>,
}

impl StructureTree {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn begin_group(&mut self, tag: StructTag, alt: Option<String>) {
        self.events.push(StructEvent::BeginGroup { tag, alt });
    }

    pub fn add_content_ref(&mut self, mcid: u32, page_idx: usize) {
        self.events.push(StructEvent::ContentRef { mcid, page_idx });
    }

    pub fn end_group(&mut self) {
        self.events.push(StructEvent::EndGroup);
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Default for StructureTree {
    fn default() -> Self {
        Self::new()
    }
}

// ── UA-2 validation ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum UaError {
    NoStructureTree,
    MissingAlt { element_index: usize },
    NoDocumentLanguage,
    /// Structure tree root is not Document or DocumentFragment.
    InvalidRootTag(String),
}

#[derive(Debug, Clone)]
pub enum UaWarning {
    MissingAlt { element_index: usize },
    HeadingLevelSkipped { from: u8, to: u8 },
    FixedBoxAsArtifact { x_mm: f64, y_mm: f64 },
}

pub struct UaValidator {
    pub warnings: Vec<UaWarning>,
    pub errors: Vec<UaError>,
}

impl UaValidator {
    pub fn new() -> Self {
        Self { warnings: Vec::new(), errors: Vec::new() }
    }

    pub fn validate(tree: Option<&StructureTree>, lang: &str) -> Self {
        let mut v = Self::new();
        if lang.is_empty() {
            v.errors.push(UaError::NoDocumentLanguage);
        }
        match tree {
            None => v.errors.push(UaError::NoStructureTree),
            Some(t) if t.is_empty() => v.errors.push(UaError::NoStructureTree),
            Some(t) => {
                if let Some(StructEvent::BeginGroup { tag, .. }) = t.events.first() {
                    if *tag != StructTag::Document && *tag != StructTag::DocumentFragment {
                        v.errors.push(UaError::InvalidRootTag(tag.pdf_name().to_string()));
                    }
                }
            }
        }
        v
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn report(&self) {
        for w in &self.warnings {
            eprintln!("PDF/UA-2 WARNING: {:?}", w);
        }
        for e in &self.errors {
            eprintln!("PDF/UA-2 ERROR: {:?}", e);
        }
    }
}

impl Default for UaValidator {
    fn default() -> Self {
        Self::new()
    }
}
