# MAN — domain-mef

## Objectivo

Domínio da classificação MEF (Matriz de Estrutura Funcional). Define os tipos e o port de repositório para a hierarquia funcional portuguesa com versionamento temporal e rastreabilidade a diploma legal.

---

## Contrato público

### Erros

```rust
pub enum MefError {
    EmptyCode,
    EmptyDiplomaRef,
    EmptyChangedBy,
    NotFound(String),
    AlreadyActive(String),
    // variantes do adaptador (via From<MefError> no tipo de erro do store)
}
```

### Código MEF

```rust
/// Código hierárquico validado (trim, não vazio).
pub struct MefCode(String);

impl MefCode {
    pub fn new(code: impl Into<String>) -> Result<Self, MefError>;
    pub fn as_str(&self) -> &str;
}

impl Display for MefCode { ... }    // ex: "0401.04.01"
```

### Referência a diploma legal

```rust
pub struct DiplomaRef {
    pub reference: String,          // ex: "Portaria n.º 1258/2009, de 15 de outubro"
    pub date: Option<String>,       // ISO YYYY-MM-DD, ex: "2009-10-15"
}

impl DiplomaRef {
    pub fn new(reference: impl Into<String>) -> Result<Self, MefError>;
    pub fn with_date(self, date: impl Into<String>) -> Self;
}
```

### Entrada MEF

```rust
pub struct MefEntry {
    pub code: MefCode,
    pub label: String,
    pub parent_code: Option<MefCode>,
    pub is_usable: bool,            // false = nó de agrupamento (não atribuível)
    pub effective_from: DateTime<Utc>,
    pub effective_to: Option<DateTime<Utc>>,  // None = versão activa
    pub changed_by: String,
    pub change_reason: Option<String>,
    pub diploma: Option<DiplomaRef>,
}

impl MefEntry {
    pub fn is_root(&self) -> bool;    // parent_code.is_none()
    pub fn is_active(&self) -> bool;  // effective_to.is_none()
}
```

### Pedido de upsert

```rust
pub struct UpsertMefEntryRequest {
    pub code: MefCode,
    pub label: String,
    pub parent_code: Option<MefCode>,
    pub is_usable: bool,
    pub changed_by: String,
    pub change_reason: Option<String>,
    pub diploma: Option<DiplomaRef>,
}

impl UpsertMefEntryRequest {
    /// Valida: `changed_by` não pode estar vazio.
    pub fn validate(&self) -> Result<(), MefError>;
}
```

### Port de repositório

```rust
pub trait MefRepository {
    type Error: From<MefError>;

    // ─── Leitura ─────────────────────────────────────────────────

    /// Todas as entradas activas (effective_to IS NULL), ordenadas por código.
    fn get_current(&self) -> Result<Vec<MefEntry>, Self::Error>;

    /// Entradas activas num instante passado — reconstituição histórica.
    fn get_at(&self, timestamp: DateTime<Utc>) -> Result<Vec<MefEntry>, Self::Error>;

    /// Versão activa de uma entrada. None se não existir ou estiver desactivada.
    fn get_entry(&self, code: &MefCode) -> Result<Option<MefEntry>, Self::Error>;

    /// Histórico completo de uma entrada (todas as versões), do mais recente ao mais antigo.
    fn get_history(&self, code: &MefCode) -> Result<Vec<MefEntry>, Self::Error>;

    /// Caminho hierárquico da raiz até ao código (inclusivo), versão activa de cada nó.
    fn resolve_path(&self, code: &MefCode) -> Result<Vec<MefEntry>, Self::Error>;

    // ─── Escrita ──────────────────────────────────────────────────

    /// Insere ou actualiza uma entrada com registo de auditoria e diploma.
    /// Idempotente se label, parent e is_usable forem iguais à versão activa.
    fn upsert_entry(&self, request: &UpsertMefEntryRequest) -> Result<(), Self::Error>;

    /// Desactiva uma entrada (marca effective_to = agora). Idempotente.
    fn deactivate_entry(
        &self,
        code: &MefCode,
        changed_by: &str,
        change_reason: Option<&str>,
        diploma: Option<&DiplomaRef>,
    ) -> Result<(), Self::Error>;
}
```

---

## Como usar

### Consulta da tabela actual

```rust
use domain_mef::{MefRepository, MefCode};

let entries = repo.get_current()?;
for entry in &entries {
    println!("{} — {} {}", 
        entry.code, 
        entry.label,
        if entry.is_usable { "" } else { "(agrupamento)" });
}
```

### Resolver classificação de um documento

```rust
let code = MefCode::new("0401.04.01")?;
let path = repo.resolve_path(&code)?;
// path[0] = raiz (ex: "04"), path[last] = "0401.04.01"
let breadcrumb: Vec<_> = path.iter().map(|e| e.label.as_str()).collect();
println!("{}", breadcrumb.join(" > "));
```

### Reconstituição histórica

```rust
let at_2020 = DateTime::parse_from_rfc3339("2020-06-01T00:00:00Z")?.into();
let historic = repo.get_at(at_2020)?;
// Tabela MEF tal como estava em 2020
```

### Inserção de nova versão por diploma

```rust
use domain_mef::{UpsertMefEntryRequest, MefCode, DiplomaRef};

repo.upsert_entry(&UpsertMefEntryRequest {
    code: MefCode::new("0401.04.01")?,
    label: "Liquidação de impostos directos".into(),
    parent_code: Some(MefCode::new("0401.04")?),
    is_usable: true,
    changed_by: "admin".into(),
    change_reason: Some("Actualização pela Portaria n.º 55/2024".into()),
    diploma: Some(DiplomaRef::new("Portaria n.º 55/2024")?.with_date("2024-03-15")),
})?;
```

### Desactivação por abolição

```rust
repo.deactivate_entry(
    &MefCode::new("0401.99")?,
    "admin",
    Some("Código abolido"),
    Some(&DiplomaRef::new("Portaria n.º 55/2024")?),
)?;
```

---

## Invariantes

- `MefCode` não pode ser vazio — `MefCode::new("")` devolve `MefError::EmptyCode`.
- `DiplomaRef::reference` não pode ser vazio — devolve `MefError::EmptyDiplomaRef`.
- `UpsertMefEntryRequest::changed_by` não pode ser vazio — `validate()` devolve `MefError::EmptyChangedBy`.
- `resolve_path` devolve o caminho da raiz até ao código: `path[0]` é a raiz, `path[last]` é o código pedido.
- `upsert_entry` é idempotente: só cria nova versão se `label`, `parent_code` ou `is_usable` mudaram.
- `deactivate_entry` é idempotente: se já estiver desactivado, não faz nada.

---

## Limites actuais

- Sem validação de existência do `parent_code` no repositório (delegada ao adaptador).
- Sem validação de que `is_usable = false` implica que o código tem filhos.
- Sem suporte a reactivação de entradas desactivadas (seria um novo `upsert_entry`).

---

## ToDo

- [ ] Validação de que o `parent_code` existe antes do upsert.
- [ ] Método `list_children(code)` para enumerar filhos directos.
- [ ] Exportação da tabela MEF em formato tabulado (CSV/JSON) para publicação externa.
