use serde::{Deserialize, Serialize};
use std::time::Duration;

// ── Dispositivo ────────────────────────────────────────────────────────────────

/// Localização e configuração de acesso a um scanner eSCL.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScannerDevice {
    /// Nome legível do dispositivo (ex.: "HP LaserJet MFP M234sdwe").
    pub name: String,
    /// Hostname ou endereço IP do dispositivo.
    pub host: String,
    /// Porta HTTP/HTTPS (tipicamente 80 ou 443).
    pub port: u16,
    /// `true` para HTTPS (`_uscans._tcp`), `false` para HTTP (`_uscan._tcp`).
    pub uses_https: bool,
    /// Caminho base do serviço eSCL (tipicamente `"/eSCL"`).
    pub base_path: String,
    /// Aceitar certificados TLS auto-assinados (uso em redes locais de confiança).
    pub danger_accept_invalid_certs: bool,
}

impl ScannerDevice {
    /// Cria um `ScannerDevice` para HTTP não encriptado (redes locais).
    pub fn http(host: impl Into<String>, port: u16) -> Self {
        Self {
            name: String::new(),
            host: host.into(),
            port,
            uses_https: false,
            base_path: "/eSCL".into(),
            danger_accept_invalid_certs: false,
        }
    }

    /// URL base do dispositivo, sem caminho eSCL.
    pub fn base_url(&self) -> String {
        let scheme = if self.uses_https { "https" } else { "http" };
        format!("{}://{}:{}", scheme, self.host, self.port)
    }

    /// URL completa para um endpoint eSCL (ex.: `"/ScannerCapabilities"`).
    pub fn escl_url(&self, endpoint: &str) -> String {
        format!("{}{}{}", self.base_url(), self.base_path, endpoint)
    }
}

// ── Configuração do cliente ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ScannerClientConfig {
    /// Timeout de ligação TCP (default: 5s).
    pub connect_timeout: Duration,
    /// Timeout máximo de espera pelo documento (default: 120s).
    pub job_timeout: Duration,
    /// Intervalo entre tentativas de obter o documento (default: 1s).
    pub poll_interval: Duration,
}

impl Default for ScannerClientConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(5),
            job_timeout: Duration::from_secs(120),
            poll_interval: Duration::from_secs(1),
        }
    }
}

// ── Capacidades ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ScanCapabilities {
    pub make_model: String,
    pub version: String,
    /// Capacidades do vidro plano.
    pub platen: Option<InputCapabilities>,
    /// Capacidades do ADF (alimentador automático), se disponível.
    pub adf: Option<InputCapabilities>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InputCapabilities {
    /// Largura máxima em 1/300 de polegada (ThreeHundredthsOfInches).
    pub max_width: u32,
    /// Altura máxima em 1/300 de polegada.
    pub max_height: u32,
    pub supported_resolutions: Vec<u32>,
    pub supported_formats: Vec<ScanFormat>,
    pub supported_color_modes: Vec<ColorMode>,
    pub supported_intents: Vec<ScanIntent>,
}

impl InputCapabilities {
    pub fn supports_format(&self, fmt: &ScanFormat) -> bool {
        self.supported_formats.contains(fmt)
    }

    pub fn supports_color_mode(&self, mode: &ColorMode) -> bool {
        self.supported_color_modes.contains(mode)
    }

    pub fn supports_resolution(&self, dpi: u32) -> bool {
        self.supported_resolutions.contains(&dpi)
    }
}

// ── Enumerações ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanFormat {
    Pdf,
    Jpeg,
    Png,
    Tiff,
}

impl ScanFormat {
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Pdf => "application/pdf",
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Tiff => "image/tiff",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Tiff => "tiff",
        }
    }

    pub(crate) fn from_mime(mime: &str) -> Option<Self> {
        let base = mime.split(';').next().unwrap_or("").trim();
        match base {
            "application/pdf" => Some(Self::Pdf),
            "image/jpeg" => Some(Self::Jpeg),
            "image/png" => Some(Self::Png),
            "image/tiff" => Some(Self::Tiff),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorMode {
    BlackAndWhite1,
    Grayscale8,
    Rgb24,
}

impl ColorMode {
    pub(crate) fn as_escl(&self) -> &'static str {
        match self {
            Self::BlackAndWhite1 => "BlackAndWhite1",
            Self::Grayscale8 => "Grayscale8",
            Self::Rgb24 => "RGB24",
        }
    }

    pub(crate) fn from_escl(s: &str) -> Option<Self> {
        match s.trim() {
            "BlackAndWhite1" => Some(Self::BlackAndWhite1),
            "Grayscale8" => Some(Self::Grayscale8),
            "RGB24" => Some(Self::Rgb24),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanSource {
    Platen,
    Adf,
    AdfDuplex,
}

impl ScanSource {
    pub(crate) fn as_escl(&self) -> &'static str {
        match self {
            Self::Platen => "Platen",
            Self::Adf => "Feeder",
            Self::AdfDuplex => "Feeder",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanIntent {
    Document,
    Photo,
    TextAndGraphic,
    BusinessCard,
    Preview,
}

impl ScanIntent {
    pub(crate) fn as_escl(&self) -> &'static str {
        match self {
            Self::Document => "Document",
            Self::Photo => "Photo",
            Self::TextAndGraphic => "TextAndGraphic",
            Self::BusinessCard => "BusinessCard",
            Self::Preview => "Preview",
        }
    }

    pub(crate) fn from_escl(s: &str) -> Option<Self> {
        match s.trim() {
            "Document" => Some(Self::Document),
            "Photo" => Some(Self::Photo),
            "TextAndGraphic" => Some(Self::TextAndGraphic),
            "BusinessCard" => Some(Self::BusinessCard),
            "Preview" => Some(Self::Preview),
            _ => None,
        }
    }
}

// ── Definições de página ───────────────────────────────────────────────────────

/// Região de scan em ThreeHundredthsOfInches (1 unidade = 1/300 polegada).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanRegion {
    pub x_offset: u32,
    pub y_offset: u32,
    pub width: u32,
    pub height: u32,
}

impl ScanRegion {
    /// A4 portrait (210 × 297 mm → 2480 × 3508 unidades a 300 dpi).
    pub const A4_PORTRAIT: Self = Self { x_offset: 0, y_offset: 0, width: 2480, height: 3508 };
    /// A4 landscape.
    pub const A4_LANDSCAPE: Self = Self { x_offset: 0, y_offset: 0, width: 3508, height: 2480 };
    /// Letter portrait (8.5 × 11 polegadas).
    pub const LETTER_PORTRAIT: Self = Self { x_offset: 0, y_offset: 0, width: 2550, height: 3300 };
}

// ── Pedido de scan ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSettings {
    pub source: ScanSource,
    pub format: ScanFormat,
    pub intent: ScanIntent,
    pub color_mode: ColorMode,
    /// Resolução em DPI (ex.: 75, 150, 200, 300, 600).
    pub resolution: u32,
    pub region: ScanRegion,
}

impl Default for ScanSettings {
    fn default() -> Self {
        Self {
            source: ScanSource::Platen,
            format: ScanFormat::Pdf,
            intent: ScanIntent::Document,
            color_mode: ColorMode::Grayscale8,
            resolution: 300,
            region: ScanRegion::A4_PORTRAIT,
        }
    }
}

// ── Estado do dispositivo ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScannerState {
    Idle,
    Processing,
    Stopped,
    Unknown(String),
}

impl ScannerState {
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Idle)
    }

    pub(crate) fn from_escl(s: &str) -> Self {
        match s.trim() {
            "Idle" => Self::Idle,
            "Processing" => Self::Processing,
            "Stopped" => Self::Stopped,
            other => Self::Unknown(other.to_string()),
        }
    }
}

// ── Resultado ─────────────────────────────────────────────────────────────────

/// Documento digitalizado pronto a persistir ou processar.
#[derive(Debug, Clone)]
pub struct ScannedDocument {
    pub format: ScanFormat,
    pub data: Vec<u8>,
    /// Mime-type conforme declarado pelo dispositivo no Content-Type da resposta.
    pub content_type: String,
}
