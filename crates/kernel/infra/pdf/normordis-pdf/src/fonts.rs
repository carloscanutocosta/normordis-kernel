use std::collections::HashMap;
use std::path::Path;

use crate::{NormaxisPdfError, Result};

// Libertinus Serif — kept for backward compatibility.
const LIBERTINUS_SERIF_REGULAR: &[u8] =
    include_bytes!("../assets/fonts/LibertinusSerif-Regular.ttf");
const LIBERTINUS_SERIF_BOLD: &[u8] =
    include_bytes!("../assets/fonts/LibertinusSerif-Bold.ttf");
const LIBERTINUS_SERIF_ITALIC: &[u8] =
    include_bytes!("../assets/fonts/LibertinusSerif-Italic.ttf");
const LIBERTINUS_SERIF_BOLD_ITALIC: &[u8] =
    include_bytes!("../assets/fonts/LibertinusSerif-BoldItalic.ttf");

// Liberation Sans — metrically identical to Arial/Calibri.
const LIBERATION_SANS_REGULAR: &[u8] =
    include_bytes!("../assets/fonts/LiberationSans-Regular.ttf");
const LIBERATION_SANS_BOLD: &[u8] =
    include_bytes!("../assets/fonts/LiberationSans-Bold.ttf");
const LIBERATION_SANS_ITALIC: &[u8] =
    include_bytes!("../assets/fonts/LiberationSans-Italic.ttf");
const LIBERATION_SANS_BOLD_ITALIC: &[u8] =
    include_bytes!("../assets/fonts/LiberationSans-BoldItalic.ttf");

// Liberation Serif — Times New Roman equivalent.
const LIBERATION_SERIF_REGULAR: &[u8] =
    include_bytes!("../assets/fonts/LiberationSerif-Regular.ttf");
const LIBERATION_SERIF_BOLD: &[u8] =
    include_bytes!("../assets/fonts/LiberationSerif-Bold.ttf");
const LIBERATION_SERIF_ITALIC: &[u8] =
    include_bytes!("../assets/fonts/LiberationSerif-Italic.ttf");
const LIBERATION_SERIF_BOLD_ITALIC: &[u8] =
    include_bytes!("../assets/fonts/LiberationSerif-BoldItalic.ttf");

// Liberation Mono — Courier New equivalent.
const LIBERATION_MONO_REGULAR: &[u8] =
    include_bytes!("../assets/fonts/LiberationMono-Regular.ttf");
const LIBERATION_MONO_BOLD: &[u8] =
    include_bytes!("../assets/fonts/LiberationMono-Bold.ttf");
const LIBERATION_MONO_ITALIC: &[u8] =
    include_bytes!("../assets/fonts/LiberationMono-Italic.ttf");
const LIBERATION_MONO_BOLD_ITALIC: &[u8] =
    include_bytes!("../assets/fonts/LiberationMono-BoldItalic.ttf");

// ── ShapedGlyph ───────────────────────────────────────────────────────────────

/// Single glyph produced by the rustybuzz shaping pipeline.
#[derive(Debug, Clone)]
pub struct ShapedGlyph {
    pub glyph_id: u16,
    /// Horizontal advance in font design units.
    pub x_advance: i32,
    pub x_offset: i32,
    pub y_offset: i32,
    /// UTF-8 byte index of the source cluster in the input string.
    pub cluster: u32,
}

// ── FontData ──────────────────────────────────────────────────────────────────

/// A single font variant — raw TTF/OTF bytes + parsed metrics.
///
/// Replaces `FontVariant` from v1.3.x.  Use [`FontVariants`] to group the four
/// weight/style variants of a family together.
pub struct FontData {
    pub bytes: Vec<u8>,
    pub units_per_em: u16,
}

impl FontData {
    /// Parse a font from raw bytes (e.g., `include_bytes!`).
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        let face = ttf_parser::Face::parse(&bytes, 0)
            .map_err(|e| NormaxisPdfError::FontLoadError(e.to_string()))?;
        let units_per_em = face.units_per_em();
        Ok(Self { bytes, units_per_em })
    }

    /// Shape `text` with the given OpenType features and return per-glyph metrics.
    ///
    /// Creates an ephemeral `rustybuzz::Face` per call to avoid lifetime issues.
    pub fn shape(&self, text: &str, features: &[rustybuzz::Feature]) -> Vec<ShapedGlyph> {
        let face = match rustybuzz::Face::from_slice(&self.bytes, 0) {
            Some(f) => f,
            None => return vec![],
        };
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(text);
        let output = rustybuzz::shape(&face, features, buffer);
        let infos = output.glyph_infos();
        let positions = output.glyph_positions();
        infos
            .iter()
            .zip(positions.iter())
            .map(|(info, pos)| ShapedGlyph {
                glyph_id: info.glyph_id as u16,
                x_advance: pos.x_advance,
                x_offset: pos.x_offset,
                y_offset: pos.y_offset,
                cluster: info.cluster,
            })
            .collect()
    }

    /// Advance width of `text` in mm at `font_size` points (72 pt/inch).
    pub fn measure_text_mm(&self, text: &str, font_size: f64) -> f64 {
        if text.is_empty() {
            return 0.0;
        }
        let glyphs = self.shape(text, &[]);
        let total_advance: i32 = glyphs.iter().map(|g| g.x_advance).sum();
        let advance_pts =
            (total_advance as f64 / self.units_per_em as f64) * font_size;
        advance_pts / 72.0 * 25.4
    }

    /// Line height in mm for `font_size` (pt) and a multiplier.
    pub fn line_height_mm(&self, font_size: f64, multiplier: f64) -> f64 {
        (font_size / 72.0 * 25.4) * multiplier
    }
}

impl Clone for FontData {
    fn clone(&self) -> Self {
        Self {
            bytes: self.bytes.clone(),
            units_per_em: self.units_per_em,
        }
    }
}

impl std::fmt::Debug for FontData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontData")
            .field("bytes_len", &self.bytes.len())
            .field("units_per_em", &self.units_per_em)
            .finish()
    }
}

// ── FontVariants ──────────────────────────────────────────────────────────────

/// A font family with up to four weight/style variants.
///
/// Replaces `FontFamily` from v1.3.x.
#[derive(Debug, Clone)]
pub struct FontVariants {
    pub name: String,
    pub regular: FontData,
    pub bold: Option<FontData>,
    pub italic: Option<FontData>,
    pub bold_italic: Option<FontData>,
}

impl FontVariants {
    /// Load a family from raw bytes (e.g., `include_bytes!`).
    pub fn from_bytes(
        name: impl Into<String>,
        regular: Vec<u8>,
        bold: Option<Vec<u8>>,
        italic: Option<Vec<u8>>,
        bold_italic: Option<Vec<u8>>,
    ) -> Result<Self> {
        Ok(Self {
            name: name.into(),
            regular: FontData::from_bytes(regular)?,
            bold: bold.map(FontData::from_bytes).transpose()?,
            italic: italic.map(FontData::from_bytes).transpose()?,
            bold_italic: bold_italic.map(FontData::from_bytes).transpose()?,
        })
    }

    /// Load a family from TTF/OTF file paths.
    pub fn from_files(
        name: impl Into<String>,
        regular: &Path,
        bold: Option<&Path>,
        italic: Option<&Path>,
        bold_italic: Option<&Path>,
    ) -> Result<Self> {
        let read = |p: &Path| -> Result<Vec<u8>> {
            std::fs::read(p).map_err(NormaxisPdfError::IoError)
        };
        Self::from_bytes(
            name,
            read(regular)?,
            bold.map(read).transpose()?,
            italic.map(read).transpose()?,
            bold_italic.map(read).transpose()?,
        )
    }

    /// Returns the best available variant for `(bold, italic)`, falling back to regular.
    pub fn get(&self, bold: bool, italic: bool) -> &FontData {
        match (bold, italic) {
            (true, true) => self
                .bold_italic
                .as_ref()
                .or(self.bold.as_ref())
                .or(self.italic.as_ref())
                .unwrap_or(&self.regular),
            (true, false) => self.bold.as_ref().unwrap_or(&self.regular),
            (false, true) => self.italic.as_ref().unwrap_or(&self.regular),
            (false, false) => &self.regular,
        }
    }

    /// Alias for [`get`](Self::get) — kept for backward compatibility with v1.3.x callers.
    pub fn get_variant(&self, bold: bool, italic: bool) -> &FontData {
        self.get(bold, italic)
    }

    /// Advance width of `text` in mm using the appropriate variant.
    pub fn measure_text_mm(&self, text: &str, font_size: f64, bold: bool, italic: bool) -> f64 {
        self.get(bold, italic).measure_text_mm(text, font_size)
    }

    /// Line height in mm for `font_size` (pt) and a multiplier.
    pub fn line_height_mm(&self, font_size: f64, line_height_multiplier: f64) -> f64 {
        self.regular.line_height_mm(font_size, line_height_multiplier)
    }
}

// Backward-compatibility aliases for v1.3.x callers.
pub type FontFamily = FontVariants;
pub type FontVariant = FontData;

// ── FontRegistry ──────────────────────────────────────────────────────────────

/// Registry of font families available in the document.
///
/// `FontRegistry::default()` embeds Liberation Sans, Serif, and Mono, with
/// common Word font aliases pre-configured.
#[derive(Debug, Clone)]
pub struct FontRegistry {
    families: HashMap<String, FontVariants>,
    /// Maps alias name → canonical family name (e.g. "Arial" → "LiberationSans").
    aliases: HashMap<String, String>,
    default_family: String,
    monospace_family: Option<String>,
}

impl FontRegistry {
    /// Creates a registry with the default embedded fonts (Liberation Sans/Serif/Mono).
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty registry with no registered fonts.
    pub fn empty() -> Self {
        Self {
            families: HashMap::new(),
            aliases: HashMap::new(),
            default_family: String::new(),
            monospace_family: None,
        }
    }

    /// Register a font family from embedded `&'static [u8]` bytes.
    ///
    /// If this is the first family registered, it becomes the default.
    pub fn register_embedded(
        &mut self,
        name: &str,
        regular: &'static [u8],
        bold: Option<&'static [u8]>,
        italic: Option<&'static [u8]>,
        bold_italic: Option<&'static [u8]>,
    ) {
        let family = FontVariants::from_bytes(
            name,
            regular.to_vec(),
            bold.map(|b| b.to_vec()),
            italic.map(|b| b.to_vec()),
            bold_italic.map(|b| b.to_vec()),
        )
        .expect("embedded font bytes must be valid");
        if self.default_family.is_empty() {
            self.default_family = name.to_string();
        }
        self.families.insert(name.to_string(), family);
    }

    /// Map `alias` to an already-registered family name.
    ///
    /// Subsequent calls to `get_family(alias)` will resolve to the target family.
    pub fn add_alias(&mut self, alias: &str, target: &str) {
        self.aliases.insert(alias.to_string(), target.to_string());
    }

    /// Returns a family by name, resolving aliases transparently.
    ///
    /// Falls back to the default family when neither the name nor any alias matches.
    pub fn get_family(&self, name: &str) -> &FontVariants {
        if let Some(fam) = self.families.get(name) {
            return fam;
        }
        if let Some(target) = self.aliases.get(name) {
            if let Some(fam) = self.families.get(target.as_str()) {
                return fam;
            }
        }
        self.get_default()
    }

    /// Register a font family. Replaces any existing family with the same name.
    pub fn register(&mut self, family: FontVariants) {
        let name = family.name.clone();
        if self.default_family.is_empty() {
            self.default_family = name.clone();
        }
        self.families.insert(name, family);
    }

    pub fn set_default(&mut self, name: &str) -> Result<()> {
        if self.families.contains_key(name) {
            self.default_family = name.to_string();
            Ok(())
        } else {
            Err(NormaxisPdfError::FontLoadError(format!(
                "font family '{name}' not registered"
            )))
        }
    }

    pub fn set_monospace(&mut self, name: &str) -> Result<()> {
        if self.families.contains_key(name) {
            self.monospace_family = Some(name.to_string());
            Ok(())
        } else {
            Err(NormaxisPdfError::FontLoadError(format!(
                "font family '{name}' not registered"
            )))
        }
    }

    /// Iterates over all registered families.
    pub fn families(&self) -> impl Iterator<Item = (&str, &FontVariants)> {
        self.families.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Name of the default font family.
    pub fn default_family_name(&self) -> &str {
        &self.default_family
    }

    /// Returns the named family, or `None` if not registered (does NOT resolve aliases).
    pub fn get(&self, name: &str) -> Option<&FontVariants> {
        self.families.get(name)
    }

    /// Returns the default font family (always valid).
    pub fn get_default(&self) -> &FontVariants {
        self.families
            .get(&self.default_family)
            .expect("default_family must be registered")
    }

    /// Returns the monospace family, or the default if none is set.
    pub fn get_monospace(&self) -> &FontVariants {
        self.monospace_family
            .as_deref()
            .and_then(|n| self.families.get(n))
            .unwrap_or_else(|| self.get_default())
    }

    /// Measures text using the named family (resolves aliases, falls back to default).
    pub fn measure_text_mm(
        &self,
        text: &str,
        family: &str,
        font_size: f64,
        bold: bool,
        italic: bool,
    ) -> f64 {
        self.get_family(family).measure_text_mm(text, font_size, bold, italic)
    }
}

impl FontRegistry {
    /// Load a [`FontRegistry`] from a directory of TTF/OTF font files.
    ///
    /// Files are grouped into families by the stem prefix before the last `-`.
    /// Variant suffixes recognised (case-insensitive): `Regular`, `Bold`,
    /// `Italic`, `BoldItalic`.  At least one family with a `Regular` variant
    /// must exist; unknown-suffix files are silently skipped.
    pub fn from_dir(dir: &Path) -> crate::Result<FontRegistry> {
        let mut map: HashMap<String, [Option<Vec<u8>>; 4]> = HashMap::new();

        let entries = std::fs::read_dir(dir).map_err(NormaxisPdfError::IoError)?;
        for entry in entries {
            let path = entry.map_err(NormaxisPdfError::IoError)?.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !matches!(ext.to_lowercase().as_str(), "ttf" | "otf") {
                continue;
            }
            let stem = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let dash = match stem.rfind('-') {
                Some(p) => p,
                None => continue,
            };
            let family = stem[..dash].to_string();
            let variant = stem[dash + 1..].to_lowercase();
            let bytes = std::fs::read(&path).map_err(NormaxisPdfError::IoError)?;
            let slot = map.entry(family).or_insert([None, None, None, None]);
            match variant.as_str() {
                "regular" => slot[0] = Some(bytes),
                "bold" => slot[1] = Some(bytes),
                "italic" => slot[2] = Some(bytes),
                "bolditalic" | "bold_italic" => slot[3] = Some(bytes),
                _ => {}
            }
        }

        let mut families = HashMap::new();
        let mut default_family = String::new();
        for (name, [regular, bold, italic, bold_italic]) in map {
            if let Some(reg_bytes) = regular {
                let fam =
                    FontVariants::from_bytes(name.clone(), reg_bytes, bold, italic, bold_italic)?;
                if default_family.is_empty() {
                    default_family = name.clone();
                }
                families.insert(name, fam);
            }
        }

        if families.is_empty() {
            return Err(NormaxisPdfError::FontLoadError(format!(
                "no valid font families found in {}",
                dir.display()
            )));
        }

        Ok(FontRegistry { families, aliases: HashMap::new(), default_family, monospace_family: None })
    }

    /// Load a [`FontRegistry`] populated with all fonts found on the host system.
    ///
    /// Requires the `system-fonts` feature flag.
    #[cfg(feature = "system-fonts")]
    pub fn from_system() -> crate::Result<FontRegistry> {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();

        let mut map: HashMap<String, [Option<Vec<u8>>; 4]> = HashMap::new();

        for face in db.faces() {
            let family = match face.families.first() {
                Some((f, _)) if !f.is_empty() => f.clone(),
                _ => continue,
            };
            let is_bold = face.weight.0 >= 600;
            let is_italic =
                matches!(face.style, fontdb::Style::Italic | fontdb::Style::Oblique);
            let bytes = match db.with_face_data(face.id, |data, _| data.to_vec()) {
                Some(b) => b,
                None => continue,
            };
            let slot = map.entry(family).or_insert([None, None, None, None]);
            let idx = match (is_bold, is_italic) {
                (false, false) => 0,
                (true, false) => 1,
                (false, true) => 2,
                (true, true) => 3,
            };
            if slot[idx].is_none() {
                slot[idx] = Some(bytes);
            }
        }

        let mut families = HashMap::new();
        let mut default_family = String::new();
        for (name, [regular, bold, italic, bold_italic]) in map {
            if let Some(reg_bytes) = regular {
                let fam =
                    FontVariants::from_bytes(name.clone(), reg_bytes, bold, italic, bold_italic)?;
                if default_family.is_empty() {
                    default_family = name.clone();
                }
                families.insert(name, fam);
            }
        }

        if families.is_empty() {
            return Err(NormaxisPdfError::FontLoadError(
                "no system fonts found".to_string(),
            ));
        }

        Ok(FontRegistry { families, aliases: HashMap::new(), default_family, monospace_family: None })
    }
}

impl Default for FontRegistry {
    /// Creates a registry with Liberation Sans (default), Serif, and Mono embedded,
    /// plus common Word font aliases pre-configured.
    fn default() -> Self {
        let mut registry = FontRegistry::empty();

        // Liberation Sans — default (metrically identical to Arial/Calibri)
        registry.register_embedded(
            "LiberationSans",
            LIBERATION_SANS_REGULAR,
            Some(LIBERATION_SANS_BOLD),
            Some(LIBERATION_SANS_ITALIC),
            Some(LIBERATION_SANS_BOLD_ITALIC),
        );

        // Liberation Serif — Times New Roman equivalent
        registry.register_embedded(
            "LiberationSerif",
            LIBERATION_SERIF_REGULAR,
            Some(LIBERATION_SERIF_BOLD),
            Some(LIBERATION_SERIF_ITALIC),
            Some(LIBERATION_SERIF_BOLD_ITALIC),
        );

        // Liberation Mono — Courier New equivalent
        registry.register_embedded(
            "LiberationMono",
            LIBERATION_MONO_REGULAR,
            Some(LIBERATION_MONO_BOLD),
            Some(LIBERATION_MONO_ITALIC),
            Some(LIBERATION_MONO_BOLD_ITALIC),
        );

        // Libertinus Serif — also available for explicit use
        registry.register_embedded(
            "LibertinusSerif",
            LIBERTINUS_SERIF_REGULAR,
            Some(LIBERTINUS_SERIF_BOLD),
            Some(LIBERTINUS_SERIF_ITALIC),
            Some(LIBERTINUS_SERIF_BOLD_ITALIC),
        );

        // Word font aliases → Liberation equivalents
        registry.add_alias("Arial",           "LiberationSans");
        registry.add_alias("Calibri",         "LiberationSans");
        registry.add_alias("Helvetica",       "LiberationSans");
        registry.add_alias("Times New Roman", "LiberationSerif");
        registry.add_alias("Cambria",         "LiberationSerif");
        registry.add_alias("Georgia",         "LiberationSerif");
        registry.add_alias("Courier New",     "LiberationMono");
        registry.add_alias("Consolas",        "LiberationMono");

        let _ = registry.set_default("LiberationSans");
        let _ = registry.set_monospace("LiberationMono");
        registry
    }
}

/// Pre-built [`FontVariants`] for Liberation Sans from embedded bytes.
pub fn liberation_sans_family() -> crate::Result<FontVariants> {
    FontVariants::from_bytes(
        "LiberationSans",
        LIBERATION_SANS_REGULAR.to_vec(),
        Some(LIBERATION_SANS_BOLD.to_vec()),
        Some(LIBERATION_SANS_ITALIC.to_vec()),
        Some(LIBERATION_SANS_BOLD_ITALIC.to_vec()),
    )
}

/// Pre-built [`FontVariants`] for Liberation Serif from embedded bytes.
pub fn liberation_serif_family() -> crate::Result<FontVariants> {
    FontVariants::from_bytes(
        "LiberationSerif",
        LIBERATION_SERIF_REGULAR.to_vec(),
        Some(LIBERATION_SERIF_BOLD.to_vec()),
        Some(LIBERATION_SERIF_ITALIC.to_vec()),
        Some(LIBERATION_SERIF_BOLD_ITALIC.to_vec()),
    )
}

/// Pre-built [`FontVariants`] for Liberation Mono from embedded bytes.
pub fn liberation_mono_family() -> crate::Result<FontVariants> {
    FontVariants::from_bytes(
        "LiberationMono",
        LIBERATION_MONO_REGULAR.to_vec(),
        Some(LIBERATION_MONO_BOLD.to_vec()),
        Some(LIBERATION_MONO_ITALIC.to_vec()),
        Some(LIBERATION_MONO_BOLD_ITALIC.to_vec()),
    )
}
