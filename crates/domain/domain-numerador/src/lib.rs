use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ─── Shared value types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResetPolicy {
    Never,
    Yearly,
    Monthly,
    Daily,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormatPart {
    Literal(String),
    DocumentTypeCode,
    SeriesCode,
    ScopeCode,
    ServiceCode,
    Period,
    Sequence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NumberFormat {
    pub separator: String,
    pub parts: Vec<FormatPart>,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

pub fn period_key(reset_policy: &ResetPolicy, as_of_date: NaiveDate) -> String {
    match reset_policy {
        ResetPolicy::Never => "GLOBAL".to_string(),
        ResetPolicy::Yearly => as_of_date.year().to_string(),
        ResetPolicy::Monthly => format!("{:04}-{:02}", as_of_date.year(), as_of_date.month()),
        ResetPolicy::Daily => format!(
            "{:04}-{:02}-{:02}",
            as_of_date.year(),
            as_of_date.month(),
            as_of_date.day()
        ),
    }
}

/// Renders a formatted number directly from an NNS sequence.
///
/// `SeriesCode` and `ServiceCode` render as empty strings — they have no NNS
/// equivalent and are kept in the enum only for historical data compatibility.
pub fn format_nns_number(
    sequence: &NumberingSequence,
    entity_id: &str,
    as_of_date: NaiveDate,
    value: u64,
) -> String {
    let seq_str = format!("{:0width$}", value, width = sequence.padding);
    let period = period_key(&sequence.reset_policy, as_of_date);
    let type_code = match &sequence.kind {
        NumberingKind::Document => sequence.document_type.as_deref().unwrap_or(""),
        NumberingKind::Procedure => sequence.procedure_type.as_deref().unwrap_or(""),
    };
    sequence
        .format
        .parts
        .iter()
        .map(|part| match part {
            FormatPart::Literal(v) => v.clone(),
            FormatPart::DocumentTypeCode => type_code.to_string(),
            FormatPart::ScopeCode => entity_id.to_string(),
            FormatPart::Period => period.clone(),
            FormatPart::Sequence => seq_str.clone(),
            FormatPart::SeriesCode | FormatPart::ServiceCode => String::new(),
        })
        .collect::<Vec<_>>()
        .join(&sequence.format.separator)
}

// ─── NNS Enums ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NumberingKind {
    Document,
    Procedure,
}

impl NumberingKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Procedure => "procedure",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "document" => Some(Self::Document),
            "procedure" => Some(Self::Procedure),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignedStatus {
    Assigned,
    Void,
    Cancelled,
}

impl AssignedStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Assigned => "assigned",
            Self::Void => "void",
            Self::Cancelled => "cancelled",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "assigned" => Some(Self::Assigned),
            "void" => Some(Self::Void),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Void | Self::Cancelled)
    }
}

// ─── NNS Value types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetRef {
    pub id: String,
    pub target_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorRef {
    pub id: String,
    pub name: Option<String>,
}

/// Metadados opcionais transportados com cada atribuição NNS.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssignmentMetadata {
    pub subject: Option<String>,
    pub recipient: Option<String>,
    pub classification_code: Option<String>,
    pub notes: Option<String>,
}

/// Filtro para listagem/contagem de atribuições NNS.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssignmentFilter {
    pub sequence_id: Option<String>,
    pub kind: Option<NumberingKind>,
    pub period_key: Option<String>,
    pub status: Option<AssignedStatus>,
    pub assigned_after: Option<DateTime<Utc>>,
    pub assigned_before: Option<DateTime<Utc>>,
}

// ─── NNS Aggregate: NumberingSequence ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumberingSequence {
    pub sequence_id: String,
    pub kind: NumberingKind,
    pub document_type: Option<String>,
    pub procedure_type: Option<String>,
    pub entity_id: String,
    pub org_unit_id: Option<String>,
    pub padding: usize,
    pub reset_policy: ResetPolicy,
    pub format: NumberFormat,
    pub valid_from: NaiveDate,
    pub valid_to: Option<NaiveDate>,
}

impl NumberingSequence {
    pub fn validate(&self) -> Result<(), NumeradorDomainError> {
        if self.sequence_id.trim().is_empty() {
            return Err(NumeradorDomainError::EmptyField("sequence_id"));
        }
        if self.entity_id.trim().is_empty() {
            return Err(NumeradorDomainError::EmptyField("entity_id"));
        }
        match &self.kind {
            NumberingKind::Document => {
                if self
                    .document_type
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty()
                {
                    return Err(NumeradorDomainError::EmptyField("document_type"));
                }
            }
            NumberingKind::Procedure => {
                if self
                    .procedure_type
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty()
                {
                    return Err(NumeradorDomainError::EmptyField("procedure_type"));
                }
            }
        }
        if self.padding == 0 {
            return Err(NumeradorDomainError::InvalidPadding);
        }
        if let Some(valid_to) = self.valid_to {
            if valid_to <= self.valid_from {
                return Err(NumeradorDomainError::InvalidDateRange);
            }
        }
        Ok(())
    }

    pub fn is_active_at(&self, date: NaiveDate) -> bool {
        date >= self.valid_from && self.valid_to.map(|to| date < to).unwrap_or(true)
    }
}

// ─── NNS Request / response types ────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AssignNumberRequest {
    pub kind: NumberingKind,
    pub target: TargetRef,
    pub document_type: Option<String>,
    pub procedure_type: Option<String>,
    pub entity_id: String,
    pub org_unit_id: Option<String>,
    pub actor: ActorRef,
    pub requested_at: Option<DateTime<Utc>>,
    pub correlation_id: Option<String>,
    pub metadata: AssignmentMetadata,
}

impl AssignNumberRequest {
    pub fn validate(&self) -> Result<(), NumeradorDomainError> {
        if self.target.id.trim().is_empty() {
            return Err(NumeradorDomainError::EmptyField("target.id"));
        }
        if self.target.target_type.trim().is_empty() {
            return Err(NumeradorDomainError::EmptyField("target.target_type"));
        }
        if self.entity_id.trim().is_empty() {
            return Err(NumeradorDomainError::EmptyField("entity_id"));
        }
        if self.actor.id.trim().is_empty() {
            return Err(NumeradorDomainError::EmptyField("actor.id"));
        }
        match &self.kind {
            NumberingKind::Document => {
                if self
                    .document_type
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty()
                {
                    return Err(NumeradorDomainError::EmptyField("document_type"));
                }
            }
            NumberingKind::Procedure => {
                if self
                    .procedure_type
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty()
                {
                    return Err(NumeradorDomainError::EmptyField("procedure_type"));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ChangeStatusRequest {
    pub kind: NumberingKind,
    pub target: TargetRef,
    pub actor: ActorRef,
    pub reason: String,
    pub correlation_id: Option<String>,
}

impl ChangeStatusRequest {
    pub fn validate(&self) -> Result<(), NumeradorDomainError> {
        if self.target.id.trim().is_empty() {
            return Err(NumeradorDomainError::EmptyField("target.id"));
        }
        if self.actor.id.trim().is_empty() {
            return Err(NumeradorDomainError::EmptyField("actor.id"));
        }
        if self.reason.trim().is_empty() {
            return Err(NumeradorDomainError::EmptyField("reason"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignedNumber {
    pub numbering_ref: String,
    pub kind: NumberingKind,
    pub target: TargetRef,
    pub number_value: String,
    pub sequence_id: String,
    pub sequence_value: u64,
    pub period_key: String,
    pub assigned_at: DateTime<Utc>,
    pub assigned_by: ActorRef,
    pub status: AssignedStatus,
    pub correlation_id: Option<String>,
    pub metadata: AssignmentMetadata,
}

// ─── NNS Error ────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum NumeradorDomainError {
    #[error("campo obrigatório vazio: {0}")]
    EmptyField(&'static str),
    #[error("padding inválido: deve ser > 0")]
    InvalidPadding,
    #[error("data de fim anterior ou igual à data de início")]
    InvalidDateRange,
    #[error("série não encontrada: {0}")]
    SequenceNotFound(String),
    #[error("série inactiva ou fora do período: {0}")]
    SequenceInactive(String),
    #[error("atribuição não encontrada para alvo: {0}")]
    AssignmentNotFound(String),
    #[error("transição de estado inválida: número já está '{0}'")]
    InvalidStatusTransition(String),
    #[error("erro de armazenamento: {0}")]
    Storage(String),
}

// ─── NNS Port traits ──────────────────────────────────────────────────────────

pub trait NumberingStore {
    fn assign(
        &mut self,
        request: &AssignNumberRequest,
        assigned_at: DateTime<Utc>,
        numbering_ref: &str,
    ) -> Result<AssignedNumber, NumeradorDomainError>;

    fn change_status(
        &mut self,
        request: &ChangeStatusRequest,
        status: AssignedStatus,
        changed_at: DateTime<Utc>,
    ) -> Result<AssignedNumber, NumeradorDomainError>;

    fn get_by_target(
        &self,
        kind: &NumberingKind,
        target_id: &str,
    ) -> Result<Option<AssignedNumber>, NumeradorDomainError>;

    fn list_assignments(
        &self,
        filter: &AssignmentFilter,
        limit: usize,
    ) -> Result<Vec<AssignedNumber>, NumeradorDomainError>;

    fn count_assignments(&self, filter: &AssignmentFilter) -> Result<u64, NumeradorDomainError>;
}

pub trait NumberingSequenceRepository {
    fn get(&self, sequence_id: &str) -> Result<Option<NumberingSequence>, NumeradorDomainError>;
    fn upsert(&self, sequence: &NumberingSequence) -> Result<(), NumeradorDomainError>;
    fn list(&self) -> Result<Vec<NumberingSequence>, NumeradorDomainError>;
    fn find_active_for(
        &self,
        kind: &NumberingKind,
        entity_id: &str,
        document_type: Option<&str>,
        procedure_type: Option<&str>,
        as_of: NaiveDate,
    ) -> Result<Option<NumberingSequence>, NumeradorDomainError>;
}

// ─── NNS Domain service ───────────────────────────────────────────────────────

pub struct NumeradorService<S: NumberingStore> {
    store: S,
}

impl<S: NumberingStore> NumeradorService<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub fn assign_number(
        &mut self,
        request: &AssignNumberRequest,
        now: DateTime<Utc>,
    ) -> Result<AssignedNumber, NumeradorDomainError> {
        request.validate()?;
        let numbering_ref = format!("num-{}", uuid::Uuid::new_v4());
        self.store.assign(request, now, &numbering_ref)
    }

    pub fn void_number(
        &mut self,
        request: &ChangeStatusRequest,
        now: DateTime<Utc>,
    ) -> Result<AssignedNumber, NumeradorDomainError> {
        request.validate()?;
        self.store.change_status(request, AssignedStatus::Void, now)
    }

    pub fn cancel_number(
        &mut self,
        request: &ChangeStatusRequest,
        now: DateTime<Utc>,
    ) -> Result<AssignedNumber, NumeradorDomainError> {
        request.validate()?;
        self.store
            .change_status(request, AssignedStatus::Cancelled, now)
    }

    pub fn get_by_target(
        &self,
        kind: &NumberingKind,
        target_id: &str,
    ) -> Result<Option<AssignedNumber>, NumeradorDomainError> {
        self.store.get_by_target(kind, target_id)
    }

    pub fn list_assignments(
        &self,
        filter: &AssignmentFilter,
        limit: usize,
    ) -> Result<Vec<AssignedNumber>, NumeradorDomainError> {
        self.store.list_assignments(filter, limit)
    }

    pub fn count_assignments(
        &self,
        filter: &AssignmentFilter,
    ) -> Result<u64, NumeradorDomainError> {
        self.store.count_assignments(filter)
    }
}
