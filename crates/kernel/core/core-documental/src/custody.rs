//! Agregado central de custódia documental — código de validação, tipo documental,
//! conteúdo imutável, proveniência, política de retenção, estados custodiais e relações.

use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{AuthoritySnapshot, DocumentalError, TemplateId};

// ── ValidationCode ────────────────────────────────────────────────────────────

/// Alfabeto Crockford Base32 — exclui I, L, O, U para eliminar ambiguidade de leitura.
const CROCKFORD: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn is_crockford(b: u8) -> bool {
    CROCKFORD.contains(&b)
}

fn validate_validation_code_format(code: &str) -> Result<(), DocumentalError> {
    let parts: Vec<&str> = code.split('-').collect();
    if parts.len() != 4
        || parts[0] != "NORD"
        || parts[1].len() != 4
        || parts[2].len() != 4
        || parts[3].len() != 2
    {
        return Err(DocumentalError::InvalidIdentifier {
            field: "validation_code".into(),
            reason: "formato inválido — esperado NORD-XXXX-XXXX-XX".into(),
        });
    }
    for part in &parts[1..] {
        for &b in part.as_bytes() {
            if !is_crockford(b) {
                return Err(DocumentalError::InvalidIdentifier {
                    field: "validation_code".into(),
                    reason: format!(
                        "carácter '{}' inválido — use Crockford Base32 (sem I, L, O, U)",
                        b as char
                    ),
                });
            }
        }
    }
    Ok(())
}

/// Código público de validação documental — análogo ao ATCUD português.
///
/// Formato: `NORD-XXXX-XXXX-XX` com alfabeto Crockford Base32 (32 símbolos sem I, L, O, U).
/// Combinações possíveis: 32^10 ≈ 1,1 × 10^15 — suficiente para séculos de emissão pública.
///
/// Gerado no momento de entrada em custódia; imutável após atribuição.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ValidationCode(pub String);

impl ValidationCode {
    /// Valida e aceita um código no formato `NORD-XXXX-XXXX-XX` (Crockford Base32).
    pub fn new(code: impl Into<String>) -> Result<Self, DocumentalError> {
        let code = code.into().trim().to_uppercase();
        if code.is_empty() {
            return Err(DocumentalError::EmptyField("validation_code".into()));
        }
        validate_validation_code_format(&code)?;
        Ok(Self(code))
    }

    /// Gera um código aleatório no formato canónico NORD (Crockford Base32).
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        let mut random_chars = |n: usize| -> String {
            (0..n)
                .map(|_| CROCKFORD[rng.gen_range(0..32usize)] as char)
                .collect()
        };
        let g1 = random_chars(4);
        let g2 = random_chars(4);
        let g3 = random_chars(2);
        Self(format!("NORD-{g1}-{g2}-{g3}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn validate(&self) -> Result<(), DocumentalError> {
        if self.0.trim().is_empty() {
            return Err(DocumentalError::EmptyField("validation_code".into()));
        }
        validate_validation_code_format(&self.0)
    }
}

// ── DocumentTypeCode ──────────────────────────────────────────────────────────

/// Código normalizado de tipo documental.
///
/// Normalizado para minúsculas sem espaços no intake.
/// Constantes para tipos nativos NORMORDIS (conforme MEF e classificação documental da AP).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentTypeCode(pub String);

impl DocumentTypeCode {
    pub fn new(code: impl Into<String>) -> Result<Self, DocumentalError> {
        let raw = code.into();
        let code = raw.trim().to_lowercase();
        if code.is_empty() {
            return Err(DocumentalError::EmptyField("document_type".into()));
        }
        if code.len() > 64 {
            return Err(DocumentalError::InvalidIdentifier {
                field: "document_type".into(),
                reason: "não pode exceder 64 caracteres".into(),
            });
        }
        if !code
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
        {
            return Err(DocumentalError::InvalidIdentifier {
                field: "document_type".into(),
                reason: "só pode conter ASCII alfanumérico, hífen ou underscore".into(),
            });
        }
        Ok(Self(code))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub const OFICIO: &'static str = "oficio";
    pub const INFORMACAO: &'static str = "informacao";
    pub const PARECER: &'static str = "parecer";
    pub const NOTIFICACAO: &'static str = "notificacao";
    pub const DECLARACAO: &'static str = "declaracao";
    pub const CERTIDAO: &'static str = "certidao";
    pub const DESPACHO: &'static str = "despacho";
    pub const REQUERIMENTO: &'static str = "requerimento";
    pub const EXTERNO: &'static str = "externo";
    pub const CONTRATO: &'static str = "contrato";
    pub const FATURA: &'static str = "fatura";
}

// ── DocumentContent ───────────────────────────────────────────────────────────

/// Conteúdo estruturado imutável de um documento em custódia.
///
/// Armazenado como texto JSON opaco — o domínio não interpreta a estrutura.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentContent(pub String);

impl DocumentContent {
    pub fn new(json_text: impl Into<String>) -> Result<Self, DocumentalError> {
        let text = json_text.into();
        if text.trim().is_empty() {
            return Err(DocumentalError::EmptyField("content".into()));
        }
        Ok(Self(text))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ── RetentionPolicy ───────────────────────────────────────────────────────────

/// Classe de retenção conforme RADA (Regulamento Arquivístico para a Administração Pública).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "class")]
pub enum RetentionClass {
    /// Conservação permanente — o documento nunca pode ser destruído.
    Permanent,
    /// Conservação temporária com período definido em anos.
    Temporary { years: u32 },
}

impl RetentionClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Permanent => "permanent",
            Self::Temporary { .. } => "temporary",
        }
    }
}

/// Política de retenção documental institucional.
///
/// `expires_at` é computado no momento da custódia (`custodied_at + years`) e
/// armazenado para lookup eficiente. `class` é o valor autoritativo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub class: RetentionClass,
    /// Data de expiração pré-computada — `None` para conservação permanente.
    pub expires_at: Option<DateTime<Utc>>,
}

impl RetentionPolicy {
    pub fn permanent() -> Self {
        Self {
            class: RetentionClass::Permanent,
            expires_at: None,
        }
    }

    /// Cria política temporária com `expires_at = custodied_at + years`.
    pub fn temporary(years: u32, custodied_at: DateTime<Utc>) -> Self {
        use chrono::Duration;
        let expires_at = custodied_at + Duration::days(years as i64 * 365);
        Self {
            class: RetentionClass::Temporary { years },
            expires_at: Some(expires_at),
        }
    }

    pub fn is_permanent(&self) -> bool {
        matches!(self.class, RetentionClass::Permanent)
    }

    pub fn is_expired(&self, at: DateTime<Utc>) -> bool {
        self.expires_at.map(|e| at > e).unwrap_or(false)
    }
}

// ── DocumentOrigin ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentOrigin {
    Normordis,
    Email,
    Scanner,
    Upload,
    Api,
    Interop,
    Legacy,
}

impl DocumentOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Normordis => "normordis",
            Self::Email => "email",
            Self::Scanner => "scanner",
            Self::Upload => "upload",
            Self::Api => "api",
            Self::Interop => "interop",
            Self::Legacy => "legacy",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "normordis" => Some(Self::Normordis),
            "email" => Some(Self::Email),
            "scanner" => Some(Self::Scanner),
            "upload" => Some(Self::Upload),
            "api" => Some(Self::Api),
            "interop" => Some(Self::Interop),
            "legacy" => Some(Self::Legacy),
            _ => None,
        }
    }
}

impl TryFrom<&str> for DocumentOrigin {
    type Error = DocumentalError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
            .ok_or_else(|| DocumentalError::OperationFailed(format!("origem desconhecida: {s}")))
    }
}

// ── EntryChannel ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntryChannel(pub String);

impl EntryChannel {
    pub fn new(channel: impl Into<String>) -> Result<Self, DocumentalError> {
        let channel = channel.into();
        if channel.trim().is_empty() {
            return Err(DocumentalError::EmptyField("entry_channel".into()));
        }
        Ok(Self(channel))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ── DocumentId ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct DocumentId(pub String);

impl DocumentId {
    pub fn new(id: impl Into<String>) -> Result<Self, DocumentalError> {
        let id = id.into();
        validate_document_id(&id)?;
        Ok(Self(id))
    }

    pub fn validate(&self) -> Result<(), DocumentalError> {
        validate_document_id(&self.0)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn from_existing(id: impl Into<String>) -> Result<Self, DocumentalError> {
        Self::new(id)
    }
}

impl<'de> Deserialize<'de> for DocumentId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let id = String::deserialize(deserializer)?;
        Self::new(id).map_err(serde::de::Error::custom)
    }
}

fn validate_document_id(id: &str) -> Result<(), DocumentalError> {
    if id.trim().is_empty() {
        return Err(DocumentalError::EmptyField("document_id".into()));
    }
    if id != id.trim() {
        return Err(DocumentalError::InvalidIdentifier {
            field: "document_id".into(),
            reason: "não pode ter espaços no início ou no fim".into(),
        });
    }
    if id.len() > 128 {
        return Err(DocumentalError::InvalidIdentifier {
            field: "document_id".into(),
            reason: "não pode exceder 128 bytes".into(),
        });
    }
    if id == "." || id == ".." || id.contains("..") {
        return Err(DocumentalError::InvalidIdentifier {
            field: "document_id".into(),
            reason: "não pode conter segmentos de navegação".into(),
        });
    }
    if id.contains('/') || id.contains('\\') {
        return Err(DocumentalError::InvalidIdentifier {
            field: "document_id".into(),
            reason: "não pode conter separadores de caminho".into(),
        });
    }
    if !id
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(DocumentalError::InvalidIdentifier {
            field: "document_id".into(),
            reason: "só pode conter ASCII alfanumérico, hífen, underscore ou ponto".into(),
        });
    }
    Ok(())
}

// ── DocumentStatus ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    Active,
    Archived,
    Revoked,
    Superseded,
}

impl DocumentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
            Self::Revoked => "revoked",
            Self::Superseded => "superseded",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "archived" => Some(Self::Archived),
            "revoked" => Some(Self::Revoked),
            "superseded" => Some(Self::Superseded),
            _ => None,
        }
    }

    /// Transições custodiais válidas:
    /// - `Active` → `Archived` | `Revoked` | `Superseded`
    /// - `Archived` → `Revoked`
    pub fn can_transition_to(&self, next: &DocumentStatus) -> bool {
        use DocumentStatus::*;
        matches!(
            (self, next),
            (Active, Archived) | (Active, Revoked) | (Active, Superseded) | (Archived, Revoked)
        )
    }
}

impl TryFrom<&str> for DocumentStatus {
    type Error = DocumentalError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s).ok_or_else(|| {
            DocumentalError::OperationFailed(format!("estado custodial desconhecido: {s}"))
        })
    }
}

// ── DocumentRelation ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    ReplyTo,
    References,
    Supersedes,
    Annuls,
    AnnexDocument,
}

impl RelationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReplyTo => "reply_to",
            Self::References => "references",
            Self::Supersedes => "supersedes",
            Self::Annuls => "annuls",
            Self::AnnexDocument => "annex_document",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "reply_to" => Some(Self::ReplyTo),
            "references" => Some(Self::References),
            "supersedes" => Some(Self::Supersedes),
            "annuls" => Some(Self::Annuls),
            "annex_document" => Some(Self::AnnexDocument),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentRelation {
    pub relation_type: RelationType,
    pub from_id: DocumentId,
    pub to_id: DocumentId,
    pub established_at: DateTime<Utc>,
}

impl DocumentRelation {
    pub fn validate(&self) -> Result<(), DocumentalError> {
        self.from_id.validate()?;
        self.to_id.validate()?;
        Ok(())
    }
}

// ── IntakeSpec ────────────────────────────────────────────────────────────────

/// Especificação de intake — todos os parâmetros necessários para aceitar um documento em custódia.
///
/// Usar com `DocumentCustody::accept()` para garantir que os invariantes de domínio
/// são validados no momento de construção.
#[derive(Debug, Clone)]
pub struct IntakeSpec {
    pub id: DocumentId,
    pub document_type: DocumentTypeCode,
    pub validation_code: ValidationCode,
    pub origin: DocumentOrigin,
    pub entry_channel: EntryChannel,
    pub authority: AuthoritySnapshot,
    pub content: Option<DocumentContent>,
    pub template_id: Option<TemplateId>,
    pub template_version: Option<String>,
    pub retention_policy: RetentionPolicy,
    pub received_at: DateTime<Utc>,
    pub custodied_at: DateTime<Utc>,
}

// ── DocumentCustody ───────────────────────────────────────────────────────────

/// Agregado central de custódia documental.
///
/// Representa um documento já finalizado em guarda institucional.
/// Todos os campos de identidade, proveniência e conteúdo são primitivos ou
/// newtypes próprios do bounded context — sem dependência de tipos externos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentCustody {
    pub id: DocumentId,
    pub document_type: DocumentTypeCode,
    /// Número administrativo — atribuído exactamente uma vez após intake, se aplicável.
    pub document_number: Option<String>,
    /// Código de validação público — imutável após atribuição no intake.
    pub validation_code: ValidationCode,
    pub template_id: Option<TemplateId>,
    pub template_version: Option<String>,
    pub origin: DocumentOrigin,
    pub entry_channel: EntryChannel,
    /// Snapshot de autoridade de quem aceitou a custódia — imutável.
    pub authority: AuthoritySnapshot,
    /// Conteúdo estruturado imutável em JSON opaco.
    pub content: Option<DocumentContent>,
    pub status: DocumentStatus,
    /// Política de retenção conforme RADA — determina período de conservação obrigatório.
    pub retention_policy: RetentionPolicy,
    pub received_at: DateTime<Utc>,
    pub custodied_at: DateTime<Utc>,
}

impl DocumentCustody {
    /// Construtor canónico — valida todos os invariantes no momento de construção.
    ///
    /// Usar sempre este método para novos documentos. Struct literal directo só deve
    /// ser usado por adapters ao reconstituir documentos a partir da base de dados.
    pub fn accept(spec: IntakeSpec) -> Result<Self, DocumentalError> {
        let doc = Self {
            id: spec.id,
            document_type: spec.document_type,
            document_number: None,
            validation_code: spec.validation_code,
            template_id: spec.template_id,
            template_version: spec.template_version,
            origin: spec.origin,
            entry_channel: spec.entry_channel,
            authority: spec.authority,
            content: spec.content,
            status: DocumentStatus::Active,
            retention_policy: spec.retention_policy,
            received_at: spec.received_at,
            custodied_at: spec.custodied_at,
        };
        doc.validate()?;
        Ok(doc)
    }

    pub fn validate(&self) -> Result<(), DocumentalError> {
        self.id.validate()?;
        if self.document_type.0.trim().is_empty() {
            return Err(DocumentalError::EmptyField("document_type".into()));
        }
        self.validation_code.validate()?;
        if self.entry_channel.0.trim().is_empty() {
            return Err(DocumentalError::EmptyField("entry_channel".into()));
        }
        Ok(())
    }

    pub fn is_in_active_custody(&self) -> bool {
        matches!(self.status, DocumentStatus::Active)
    }

    pub fn is_retention_expired(&self, at: DateTime<Utc>) -> bool {
        self.retention_policy.is_expired(at)
    }

    /// Valida a transição custodial e devolve o novo estado.
    pub fn transition_to(&self, next: DocumentStatus) -> Result<DocumentStatus, DocumentalError> {
        if !self.status.can_transition_to(&next) {
            return Err(DocumentalError::InvalidStatusTransition(
                self.status.as_str().to_string(),
                next.as_str().to_string(),
            ));
        }
        Ok(next)
    }

    /// Valida a atribuição de número — só permitida em custódia activa e sem número anterior.
    pub fn assign_number(&self, number: &str) -> Result<(), DocumentalError> {
        if !self.is_in_active_custody() {
            return Err(DocumentalError::DocumentImmutable);
        }
        if self.document_number.is_some() {
            return Err(DocumentalError::NumberAlreadyAssigned);
        }
        if number.trim().is_empty() {
            return Err(DocumentalError::EmptyDocumentNumber);
        }
        Ok(())
    }
}
