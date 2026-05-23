# MAN — domain-numerador

## Objectivo

Domínio NNS (Numeração Normalizada de Sequências). Define os tipos, ports e serviço para criação e gestão de séries de numeração configuráveis, com atribuição atómica e formatação de números normalizados.

---

## Contrato público

### Tipos de sequência

```rust
pub struct NumberingSequence {
    pub series_id: String,
    pub scope: String,         // ex: "DGF", "global"
    pub kind: NumberingKind,
    pub reset_policy: ResetPolicy,
    pub format_parts: Vec<FormatPart>,
    pub is_active: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

pub enum NumberingKind {
    Sequential,    // contador global, sem reset
    PeriodReset,   // contador por período (ano, mês)
}

pub enum ResetPolicy {
    Never,
    Yearly,   // period_key = "2025"
    Monthly,  // period_key = "2025-01"
}
```

### Formato de número

```rust
pub struct NumberFormat {
    pub parts: Vec<FormatPart>,
}

pub enum FormatPart {
    Literal(String),                  // ex: "OF", "/"
    Separator(String),                // alias semântico para Literal
    Counter { width: u32, pad: char }, // ex: 4 dígitos com '0' → "0042"
    Year,                             // ano de 4 dígitos: "2025"
    Month,                            // mês de 2 dígitos: "05"
}

/// Calcula a chave de período para uma data e política de reset.
pub fn period_key(date: &DateTime<Utc>, policy: &ResetPolicy) -> String;

/// Formata um número NNS com os parts e o número de sequência.
pub fn format_nns_number(parts: &[FormatPart], seq: u64, date: &DateTime<Utc>) -> String;
```

### Atribuição

```rust
pub struct AssignNumberRequest {
    pub series_id: String,
    pub assigned_by: String,
    pub target: Option<TargetRef>,
    pub actor: Option<ActorRef>,
    pub subject: Option<String>,
    pub recipient: Option<String>,
    pub classification_code: Option<String>,
    pub notes: Option<String>,
}

pub struct AssignedNumber {
    pub assignment_id: String,
    pub series_id: String,
    pub period_key: String,
    pub sequence_number: u64,
    pub formatted_number: String,
    pub assigned_at: DateTime<Utc>,
    pub assigned_by: String,
    pub target: Option<TargetRef>,
}

pub struct TargetRef {
    pub kind: String,   // ex: "oficio", "despacho", "processo"
    pub id: String,
}

pub struct ActorRef {
    pub kind: String,   // ex: "user", "system"
    pub id: String,
}
```

### Metadados e filtros

```rust
pub struct AssignmentMetadata {
    pub subject: Option<String>,
    pub recipient: Option<String>,
    pub classification_code: Option<String>,
    pub notes: Option<String>,
}

pub struct AssignmentFilter {
    pub series_id: Option<String>,
    pub period_key: Option<String>,
    pub target_kind: Option<String>,
    pub target_id: Option<String>,
    pub assigned_by: Option<String>,
}

pub enum AssignedStatus {
    Active,
    Voided { reason: String },
}

pub struct ChangeStatusRequest {
    pub assignment_id: String,
    pub new_status: AssignedStatus,
    pub changed_by: String,
    pub reason: Option<String>,
}
```

### Ports

```rust
pub trait NumberingStore: Send + Sync {
    fn assign(&self, req: &AssignNumberRequest, series: &NumberingSequence, date: &DateTime<Utc>)
        -> Result<AssignedNumber, NumeradorError>;
    fn get_assignment(&self, assignment_id: &str) -> Result<Option<AssignedNumber>, NumeradorError>;
    fn list_assignments(&self, filter: &AssignmentFilter) -> Result<Vec<AssignedNumber>, NumeradorError>;
    fn change_status(&self, req: &ChangeStatusRequest) -> Result<(), NumeradorError>;
}

pub trait NumberingSequenceRepository: Send + Sync {
    fn save_series(&self, series: &NumberingSequence) -> Result<(), NumeradorError>;
    fn get_series(&self, series_id: &str) -> Result<Option<NumberingSequence>, NumeradorError>;
    fn list_series(&self, scope: Option<&str>) -> Result<Vec<NumberingSequence>, NumeradorError>;
    fn deactivate_series(&self, series_id: &str) -> Result<(), NumeradorError>;
}
```

### Serviço de domínio

```rust
pub struct NumeradorService<S> where S: NumberingStore + NumberingSequenceRepository { ... }

impl<S: NumberingStore + NumberingSequenceRepository> NumeradorService<S> {
    pub fn new(store: S) -> Self;

    /// Atribui o próximo número da série. Devolve erro se a série não existir ou estiver inactiva.
    pub fn assign(&self, req: &AssignNumberRequest) -> Result<AssignedNumber, NumeradorError>;

    /// Cria ou substitui uma série de numeração.
    pub fn save_series(&self, series: &NumberingSequence) -> Result<(), NumeradorError>;

    /// Lista todas as séries, opcionalmente filtradas por scope.
    pub fn list_series(&self, scope: Option<&str>) -> Result<Vec<NumberingSequence>, NumeradorError>;
}
```

---

## Como usar

### Criar série e atribuir número

```rust
use domain_numerador::{
    NumeradorService, NumberingSequence, NumberingKind, ResetPolicy,
    FormatPart, AssignNumberRequest, TargetRef,
};

let svc = NumeradorService::new(my_store);

// Criar série
svc.save_series(&NumberingSequence {
    series_id: "oficio-dgf".into(),
    scope: "DGF".into(),
    kind: NumberingKind::PeriodReset,
    reset_policy: ResetPolicy::Yearly,
    format_parts: vec![
        FormatPart::Literal("OF".into()),
        FormatPart::Separator("/".into()),
        FormatPart::Counter { width: 4, pad: '0' },
        FormatPart::Separator("/".into()),
        FormatPart::Year,
    ],
    is_active: true,
    created_by: "system".into(),
    created_at: Utc::now(),
})?;

// Atribuir número
let assigned = svc.assign(&AssignNumberRequest {
    series_id: "oficio-dgf".into(),
    assigned_by: "user-1".into(),
    target: Some(TargetRef { kind: "oficio".into(), id: "doc-42".into() }),
    subject: Some("Notificação de prazo".into()),
    classification_code: Some("0401.04.01".into()),
    ..Default::default()
})?;

println!("{}", assigned.formatted_number); // "OF/0001/2025"
```

### Calcular period_key

```rust
use domain_numerador::{period_key, ResetPolicy};

let key = period_key(&Utc::now(), &ResetPolicy::Monthly);
// "2025-05"
```

---

## Invariantes

- Séries inactivas (`is_active = false`) não aceitam novas atribuições.
- O `period_key` é determinístico dado a data e a política — permite reconstituição histórica.
- `format_nns_number` é puro (sem side effects) — pode ser chamado a qualquer momento para formatar.
- O `assignment_id` é gerado pelo store (UUID) — único por atribuição.

---

## Limites actuais

- Sem suporte a reversão de atribuições (números são permanentes).
- Sem validação de unicidade de `series_id` no domínio (delegada ao store).
- Sem suporte a numeração por sub-série (ex.: série dentro de série).

---

## ToDo

- [ ] Suporte a sub-séries (ex.: "OF/DGF/0001/2025" com divisão por unidade).
- [ ] Método `preview_next` para pré-visualizar o próximo número sem atribuir.
- [ ] Exportação de relatório de atribuições por período.
