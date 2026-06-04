# Manual: core-validation

## Propósito e fronteira

O `core-validation` é o mecanismo estrutural de confiança formal do NORMORDIS.

Responde à pergunta:

> "Este artefacto está formalmente válido, íntegro, coerente com o contrato aplicável e em condições de ser tratado, auditado, arquivado ou decidido?"

Não responde a:

> "Este sujeito tem direito? Este procedimento deve ser deferido? Esta interpretação legal é correta?"

Essas perguntas pertencem ao domínio, ao procedimento administrativo ou à decisão humana.

**Formulação canónica da fronteira:**

> O `core-validation` não decide o conteúdo da Administração; garante que aquilo que é produzido, transmitido ou aceite pelo sistema cumpre as condições estruturais mínimas para poder ser tratado, auditado, arquivado ou decidido.

---

## Modelos canónicos

### `ValidationSeverity`

Peso de uma regra de validação.

| Variante  | Significado                                       | Equivalente funcional |
|-----------|---------------------------------------------------|-----------------------|
| `Error`   | Falha bloqueante — impede progressão              | blocking              |
| `Warning` | Aviso — permite progressão com justificação       | warning               |
| `Info`    | Observação — apenas regista, não condiciona       | info                  |

A distinção entre `Error` e `Warning` é intencional: um `Warning` com override deve ser auditado;
um `Error` não pode ser ultrapassado sem intervenção explícita e registo.

### `ValidationStatus`

Estado de execução de uma validação individual. Distinto de `ValidationSeverity` (que descreve
o peso da regra) — este descreve o resultado de a ter corrido.

| Variante         | Significado                                                               |
|------------------|---------------------------------------------------------------------------|
| `Passed`         | A regra correu e o artefacto satisfaz os critérios.                       |
| `Failed`         | A regra correu e o artefacto não satisfaz os critérios (equivale a Error).|
| `Warning`        | A regra correu com reservas (equivale a Warning).                         |
| `Skipped`        | A regra foi intencionalmente ignorada por condição legítima.              |
| `NotApplicable`  | A regra não se aplica ao artefacto em questão.                            |
| `Overridden`     | Uma validação bloqueante foi ultrapassada com justificação registada.     |
| `ExecutionError` | A validação não pôde ser executada (erro de infra ou dependência ausente).|

**Distinção crítica:** `Failed` significa que a regra correu e concluiu negativamente.
`ExecutionError` significa que a própria validação não foi possível executar.
Para evidência COSO, são situações distintas — a primeira é um resultado de controlo;
a segunda é uma falha de monitorização.

### `ValidationIssue`

Resultado de uma regra aplicada a um campo ou artefacto.

```rust
pub struct ValidationIssue {
    pub rule_id: String,       // ID canónico da regra (ex: "validation.nif.checksum")
    pub field: Option<String>, // Campo afectado, se aplicável
    pub severity: ValidationSeverity,
    pub message: String,       // Mensagem técnica
}
```

Construtores de conveniência: `ValidationIssue::error(...)`, `::warning(...)`, `::info(...)`.

### `ValidationReport`

Acumulador in-process de resultados de validação.

```rust
pub struct ValidationReport {
    pub valid: bool,
    pub issues: Vec<ValidationIssue>,
}
```

`valid` é `true` se e só se não existir nenhum issue com `ValidationSeverity::Error`.
Warnings e Info não invalidam o report.

Operações principais:

```rust
let mut report = ValidationReport::ok();
report.push(issue);
report.merge(other_report);
report.is_valid(); // → bool
```

### `ValidationContext`

Contexto de execução de uma validação institucional. Identifica quem validou, quando,
em que âmbito e com que motor. Permite que o `ValidationResult` seja reprodutível e auditável.

```rust
pub struct ValidationContext {
    pub actor_id: Option<String>,
    pub scope: Option<String>,
    pub timestamp_rfc3339: String, // RFC 3339, fornecido pelo chamador
    pub engine_version: Option<String>,
}
```

O crate não gera timestamps — o chamador fornece. Isto mantém o crate sem relógio
e torna os testes determinísticos.

Builder:

```rust
let ctx = ValidationContext::new("2026-06-03T14:00:00Z")
    .with_actor("svc_ingest")
    .with_scope("uo_001")
    .with_engine_version("0.3.0");
```

### `ValidationResult`

Artefacto institucional de validação. Difere de `ValidationReport` em que tem identidade
própria, contexto de execução, resultados por regra e estado global calculado.
Pode ser armazenado, emitido como evento ou referenciado por `core-audit` como evidência
de atividade de controlo.

```rust
pub struct ValidationResult {
    pub validation_id: String,
    pub target_type: String,
    pub target_id: String,
    pub context: Option<ValidationContext>,
    pub overall_status: ValidationStatus,
    pub outcomes: Vec<RuleOutcome>,
}
```

Construção a partir de um `ValidationReport` acumulado:

```rust
let result = ValidationResult::from_report(
    "val_2026_000123",
    "DocumentInstance",
    "doc_abc123",
    Some(context),
    &report,
);
```

O mapeamento de `ValidationSeverity` para `ValidationStatus` é:
`Error → Failed`, `Warning → Warning`, `Info → Passed`.
O `overall_status` é determinado pelo issue de maior severidade.

### `RuleOutcome`

Resultado da execução de uma regra individual dentro de um `ValidationResult`.

```rust
pub struct RuleOutcome {
    pub rule_id: String,
    pub status: ValidationStatus,
    pub message: Option<String>,
}
```

Construtores: `RuleOutcome::passed(...)`, `::failed(...)`, `::skipped(...)`,
`::not_applicable(...)`, `::overridden(...)`.

### `Normalized<T>`

Preserva o valor original e o valor normalizado quando existe normalização relevante.
Garante que a normalização não é silenciosa.

```rust
pub struct Normalized<T> {
    pub original: String,
    pub normalized: T,
}
```

---

## Validadores disponíveis

### Identificadores portugueses

| Função                          | Módulo  | Descrição                                                                 |
|---------------------------------|---------|---------------------------------------------------------------------------|
| `validate_nif(field, val)`      | `nif`   | NIF: 9 dígitos, primeiro dígito válido, checksum MOD 11.                 |
| `normalize_nif(val)`            | `nif`   | Remove whitespace, devolve `Normalized<String>`.                          |
| `validate_niss(field, val)`     | `niss`  | NISS: 11 dígitos, categoria, posição auxiliar, checksum.                  |
| `normalize_niss(val)`           | `niss`  | Remove whitespace, devolve `Normalized<String>`.                          |
| `validate_cc(field, val)`       | `cc`    | CC: 8 dígitos + 1 letra + 1 dígito, checksum Luhn alfanumérico.          |
| `normalize_cc(val)`             | `cc`    | Remove whitespace e hífenes, uppercase, devolve `Normalized<String>`.    |
| `validate_iban(field, val)`     | `iban`  | IBAN: forma mínima (15–34 chars), MOD 97.                                |
| `normalize_iban(val)`           | `iban`  | Remove whitespace, aplica uppercase.                                      |
| `validate_cp(field, val)`       | `cp`    | Código postal PT `DDDD-DDD`; aceita espaço como separador alternativo.   |
| `normalize_cp(val)`             | `cp`    | Normaliza espaço para hífen, devolve `Normalized<String>`.                |
| `validate_phone_pt(field, val)` | `phone` | Telefone PT: 9 dígitos, prefixo 2/3/7/8/9; strip `+351`/`00351`.        |
| `normalize_phone_pt(val)`       | `phone` | Remove formatting e prefixo internacional, devolve `Normalized<String>`. |

#### Algoritmo NIF (PT)

Dígitos 1–8 multiplicados pelos pesos `[9, 8, 7, 6, 5, 4, 3, 2]`.
Dígito de controlo = `11 - (soma % 11)`; se resultado ≥ 10, controlo = 0.
Primeiro dígito válido: `1, 2, 3, 5, 6, 7, 8, 9`.

#### Algoritmo NISS (PT)

Dígitos 1–9 multiplicados pelos pesos `[29, 23, 19, 17, 13, 11, 7, 5, 3]`.
Controlo = `9 - ((soma - 1) % 9)` — resultado ∈ {1..=9}.
Posição 10 deve ser `'0'`; posição 11 deve igualar o controlo.
Categorias válidas para o primeiro dígito: `1, 2, 3, 5, 6, 7, 9`.

#### Algoritmo CC (PT) — Cartão de Cidadão

Formato: 8 dígitos + 1 letra maiúscula + 1 dígito de controlo = 10 caracteres normalizados.
Algoritmo de controlo: os primeiros 9 caracteres (dígitos e letra) são convertidos para
valores numéricos (dígito → valor, letra A=10, B=11, ... Z=35), multiplicados pelos pesos
alternados `[1, 2, 1, 2, 1, 2, 1, 2, 1]` com redução de dois dígitos (se produto ≥ 10,
soma os algarismos do produto). O dígito de controlo é `10 - (soma % 10)`, sendo 10 → 0.
**Nota de verificação:** este algoritmo baseia-se na especificação técnica do IRN.
O primeiro dígito deve ser validado contra o conjunto de séries válidas em produção.

### Formato e strings

| Função                              | Módulo   | Descrição                                                                  |
|-------------------------------------|----------|----------------------------------------------------------------------------|
| `required(field, val)`              | `string` | Campo não vazio após trim.                                                 |
| `min_length(field, val, min)`       | `string` | Comprimento mínimo (conta chars Unicode, não bytes).                       |
| `max_length(field, val, max)`       | `string` | Comprimento máximo (conta chars Unicode, não bytes).                       |
| `validate_email(field, val)`        | `email`  | Forma estrutural: sem espaços, contém `@`.                                 |
| `validate_uuid(field, val)`         | `uuid`   | Qualquer UUID válido (v1/v3/v4/v5/v7/nil) via crate `uuid`.               |
| `validate_semver(field, val)`       | `semver` | Semver 2.0: `MAJOR.MINOR.PATCH[-pre][+build]`; zeros à esquerda rejeitados em MAJOR/MINOR/PATCH e em identificadores numéricos de prerelease. |
| `validate_mime(field, val)`         | `mime`   | MIME type `type/subtype`; sem espaços nem parâmetros; chars `[A-Za-z0-9-]` no type e `[A-Za-z0-9-.+]` no subtype. |
| `validate_in_range(field, val, min, max)` | `range` | `val ∈ [min, max]` (inclusive); rejeita NaN, infinito e bounds inválidos. |

### JSON / payload

| Função                                 | Módulo | Descrição                            |
|----------------------------------------|--------|--------------------------------------|
| `require_object(field, val)`           | `json` | O `Value` deve ser um objeto JSON.   |
| `require_field(field, object, key)`    | `json` | O objeto deve conter o campo `key`.  |

### Integridade

| Função                                 | Módulo        | Descrição                                                             |
|----------------------------------------|---------------|-----------------------------------------------------------------------|
| `validate_sha256_hex(field, val)`      | `hash_format` | 64 caracteres hexadecimais lowercase.                                 |
| `sha256_bytes(data)`                   | (root)        | SHA-256 de `&[u8]` → hex lowercase String.                           |
| `sha256_file(path)`                    | (root)        | SHA-256 de ficheiro regular em streaming (não carrega em memória).    |
| `manifest_file(path)`                  | (root)        | `ManifestEntry { path, size, sha256 }` para um ficheiro.             |
| `ManifestList::from_entries(entries)`  | (root)        | Agrega `Vec<ManifestEntry>`, ordena por `path`, calcula `list_hash`. |
| `ManifestList::from_paths(paths)`      | (root)        | Constrói `ManifestList` a partir de paths de ficheiros.               |

`ManifestList.list_hash` é o SHA-256 da representação JSON canónica das entradas ordenadas
por `path`. Determinístico independentemente da ordem de inserção — pode ser usado como
identificador de integridade do pacote completo.

Política de filesystem:

- `sha256_file` e `manifest_file` usam `symlink_metadata` e rejeitam symlinks.
- Em Windows, reparse points são rejeitados quando detectados pela API standard.
- `manifest_file` rejeita paths que não sejam representáveis em UTF-8, para evitar
  manifests com substituição lossy.
- Paths no manifest usam `/` como separador canónico.

### Verificação e entrega externa de email

O `core-validation` define portos de email sem depender de rede, DNS ou providers.
As implementações concretas vivem em infra.

| Tipo | Descrição |
|------|-----------|
| `EmailVerificationPort` | Porto hexagonal para verificar rota DNS/MX de email. |
| `EmailRouteEvidence` | Evidência com domínio ASCII, estado, MX encontrados e fallback A/AAAA. |
| `EmailRouteStatus` | `MxFound`, `AddressFallbackFound` ou `NoRoute`. |
| `EmailDeliveryPort` | Porto hexagonal para envio de email. |
| `EmailMessage` | Mensagem com destinatários, assunto, corpo text/html e anexos base64. |
| `EmailDeliveryEvidence` | Evidência de provider, id externo quando disponível e destinatários aceites. |

Implementações concretas vivem em infra, por exemplo
`email-infra::DnsEmailVerifier` e `email-infra::GraphEmailSender`.

### Coerência estrutural

| Função                                                  | Módulo      | Descrição                                                              |
|---------------------------------------------------------|-------------|------------------------------------------------------------------------|
| `validate_date_range(field, start, end)`                | `coherence` | `start ≤ end` em `YYYY-MM-DD`; valida formato, mês [1–12] e dia por mês (com ano bissexto). |
| `validate_datetime_range(field, start, end)`            | `coherence` | `start ≤ end` em RFC 3339; comparação correcta em UTC (cross-offset).  |
| `validate_state_transition(field, from, to, allowed)`   | `coherence` | Par `(from, to)` na lista de transições permitidas.                    |

#### Comportamento de offset em `validate_datetime_range`

A comparação é feita em UTC via `chrono::DateTime::parse_from_rfc3339` —
correcta para qualquer combinação de offsets. `"2026-01-01T10:00:00+01:00"` e
`"2026-01-01T09:00:00Z"` são considerados iguais (mesmo instante). Fractional
seconds são aceites. Leap seconds (segundo 60) são aceites per RFC 3339.

---

## Pipeline de validação típico

```
artefacto recebido
    ↓
ValidationReport::ok()          // acumulador vazio
    ↓
validators::* aplicados         // cada validator produz um ValidationReport
    ↓
report.merge(...)               // acumulação
    ↓
report.is_valid()               // verificação imediata (in-process)
    ↓
ValidationResult::from_report() // artefacto institucional (para persistência/evento)
    ↓
result.is_blocking()            // decide progressão
    ↓
[se blocking] rejeitar ou suspender processamento
[se warning]  registar e continuar com justificação
[se passed]   aceitar e prosseguir
```

---

## Constantes de regras

As constantes canónicas de `rule_id` estão em `core_validation::rules`.

### Identificadores e strings

| Constante            | Valor                                 |
|----------------------|---------------------------------------|
| `STRING_REQUIRED`    | `"validation.string.required"`        |
| `STRING_MIN_LENGTH`  | `"validation.string.min_length"`      |
| `STRING_MAX_LENGTH`  | `"validation.string.max_length"`      |
| `EMAIL_FORMAT`       | `"validation.email.format"`           |
| `UUID_FORMAT`        | `"validation.uuid.format"`            |
| `SEMVER_FORMAT`      | `"validation.semver.format"`          |
| `MIME_FORMAT`        | `"validation.mime.format"`            |
| `NIF_FORMAT`         | `"validation.nif.format"`             |
| `NIF_CHECKSUM`       | `"validation.nif.checksum"`           |
| `NISS_FORMAT`        | `"validation.niss.format"`            |
| `NISS_CHECKSUM`      | `"validation.niss.checksum"`          |
| `CC_FORMAT`          | `"validation.cc.format"`              |
| `CC_CHECKSUM`        | `"validation.cc.checksum"`            |
| `IBAN_FORMAT`        | `"validation.iban.format"`            |
| `CP_FORMAT`          | `"validation.cp.format"`              |
| `PHONE_PT_FORMAT`    | `"validation.phone_pt.format"`        |

### JSON / payload

| Constante            | Valor                                  |
|----------------------|----------------------------------------|
| `JSON_OBJECT`        | `"validation.json.object"`             |
| `JSON_REQUIRED_FIELD`| `"validation.json.required_field"`     |

### Integridade

| Constante            | Valor                               |
|----------------------|-------------------------------------|
| `HASH_SHA256_FORMAT` | `"validation.hash.sha256_format"`   |

### Range numérico

| Constante              | Valor                            |
|------------------------|----------------------------------|
| `NUMERIC_RANGE_INVALID`| `"validation.range.out_of_range"`|

### Coerência estrutural

| Constante                   | Valor                                          |
|-----------------------------|------------------------------------------------|
| `DATE_FORMAT_INVALID`       | `"validation.coherence.date_format"`           |
| `DATE_RANGE_INVALID`        | `"validation.coherence.date_range"`            |
| `STATE_TRANSITION_INVALID`  | `"validation.coherence.state_transition"`      |

---

## Erros canónicos

Componente: `core-validation`.

| Código canónico                      | Situação                                           |
|--------------------------------------|----------------------------------------------------|
| `MINI.VALIDATION.INVALID_INPUT`      | Input inválido para operação de validação.         |
| `MINI.VALIDATION.INVALID_RULE`       | Regra de validação inválida ou desconhecida.       |
| `MINI.VALIDATION.NORMALIZATION_FAILED` | Falha no processo de normalização.               |
| `MINI.VALIDATION.JSON_FAILED`        | Falha em operação sobre JSON.                      |
| `MINI.VALIDATION.OPERATION_FAILED`   | Falha genérica de operação.                        |
| `MINI.VALIDATION.FILE_NOT_FOUND`     | Ficheiro não encontrado.                           |
| `MINI.VALIDATION.NOT_REGULAR_FILE`   | Path não corresponde a ficheiro regular.           |
| `MINI.VALIDATION.UNSAFE_FILE_TYPE`   | Path é symlink, reparse point ou tipo proibido.    |
| `MINI.VALIDATION.INVALID_PATH_ENCODING` | Path não é representável em UTF-8 canónico.     |
| `MINI.VALIDATION.FILE_READ_FAILED`   | Falha de leitura de ficheiro.                      |
| `MINI.VALIDATION.MANIFEST_FAILED`    | Falha na geração de manifesto.                     |
| `MINI.VALIDATION.HASH_FAILED`        | Falha no cálculo de hash.                          |

Todos os erros são convertíveis para `MiniError` via `ValidationError::to_mini_error()`
ou `From<ValidationError> for MiniError`.

---

## Alinhamento COSO

O `core-validation` contribui para quatro dos cinco componentes COSO:

| Componente COSO          | Contributo do core-validation                                             |
|--------------------------|---------------------------------------------------------------------------|
| Ambiente de controlo     | Regras formais de validação uniformes, versionadas e auditáveis.          |
| Avaliação de risco       | Identificação de falhas, inconsistências e artefactos incompletos.        |
| Atividades de controlo   | Bloqueios (`Failed`), avisos auditáveis (`Warning`), registos (`Info`).   |
| Informação e comunicação | `ValidationResult` estruturado, com `rule_id`, `status` e `message`.     |
| Monitorização            | `ValidationStatus` distingue `Failed` / `ExecutionError` / `Skipped` — cada um com leitura de controlo distinta. |

**O que o core-validation não faz em nome do COSO:**
Não emite eventos para `core-audit` diretamente. A emissão de eventos auditáveis
é responsabilidade do chamador (serviço, use case ou adapter) que usa o `ValidationResult`
como entrada para um `AuditRecord`. O crate produz a evidência; não a armazena.

---

## Integração com outros cores

### core-documental

O `core-validation` valida a estrutura formal de instâncias documentais:
- `template_id` presente e em formato UUID.
- `template_version` presente.
- `payload_hash` em formato SHA-256 hexadecimal.
- Transições de estado documental permitidas (`validate_state_transition`).
- Existência de campos obrigatórios no payload (`require_field`).

Não valida se o teor do documento é juridicamente adequado.

### core-ingest

O `core-validation` valida a entrada de artefactos no sistema:
- Hash SHA-256 calculado e conforme (`sha256_bytes`, `validate_sha256_hex`).
- Manifesto de pacote calculado (`manifest_file`).
- Metadados mínimos presentes (`required`, `require_field`).
- Identificadores de origem com formato válido.

O resultado do scan antimalware (de adapter externo) pode ser validado quanto
ao formato (presença de `scan_status`, `engine_id`, `engine_version`) — o
core-validation não executa o scan.

### core-export

O `core-validation` valida pacotes de exportação antes de serem selados:
- Manifesto multi-ficheiro via `ManifestList` (`from_paths`, `list_hash`).
- Hashes de cada ficheiro no pacote.
- Formato dos metadados de exportação.

### core-audit

O `core-validation` **não depende** de `core-audit`. A relação é inversa:
o chamador usa o `ValidationResult` como entrada para emitir um `AuditRecord`
quando a validação é institucional. O crate não persiste evidência; produz-a.

### core-security

O `core-validation` valida formato de tokens e identificadores (UUID, hash);
não decide permissões nem autorizações. Quem pode fazer o quê pertence ao `core-security`.

---

## Edge vs central

No modelo edge/central do NORMORDIS, o `core-validation` tem dois papéis:

**Edge (miniPC/SF):**
- Valida estrutura e calcula hashes localmente.
- Aceita trabalho em fila com validação provisória.
- Prepara `ValidationResult` local para eventual revalidação.

**Central (backend):**
- Revalida contra versões oficiais de regras e schemas.
- Confirma políticas actualizadas.
- Aceita o artefacto como definitivo após validação central.

**Princípio:**

> Tudo o que é validado no edge é provisoriamente confiável;
> só se torna institucionalmente definitivo após revalidação central.

---

## Limitações declaradas

### Limitações de design (por fronteira)

- Não implementa JSON Schema completo (draft-07 ou superior).
- Não valida existência real de email, domínio, conta bancária ou sujeito fiscal.
- Não valida regras substantivas de negócio (elegibilidade, enquadramento, regime).
- Não persiste `ValidationReport` nem `ValidationResult`.
- Não persiste manifests.
- Não assina digitalmente manifests nem `ValidationResult`.
- Não emite eventos para `core-audit` — responsabilidade do chamador.
- Não integra UI, Tauri, SQLite nem qualquer infra.

### Limitações técnicas activas

- Não implementa `ValidationRuleDescriptor` / rule registry — as regras são constantes
  sem metadados (scope, versão, vigência, fundamento). Isto limita a governabilidade
  das regras em runtime e a sua rastreabilidade institucional.
- Não implementa mensagens em camadas (utilizador / técnica / auditável) — existe
  apenas um campo `message` por issue.
- Não implementa `ValidationOverride` como tipo de primeira classe — existe
  `RuleOutcome::overridden()` mas sem `override_id`, `actor_id`, `timestamp`,
  `justification` para evidência COSO completa.
- Sem mitigação completa de TOCTOU quando o host valida paths em directórios
  mutáveis entre metadata e abertura do ficheiro.
- Algoritmo de checksum do CC baseado na especificação técnica do IRN;
  requer validação contra casos reais de emissão antes de uso probatório.
- Verificação operacional de email requer adaptador externo via `EmailVerificationPort`;
  o core não executa DNS por si.

---

## Estado de produção

**Estado:** production-ready interno/controlado — apto para validação estrutural
e integridade determinística nas miniapps.

**Reserva:** uso probatório em cenários de arquivo formal requer verificação externa
do algoritmo de checksum do CC contra casos reais de emissão e controlo do risco
TOCTOU pelo host quando paths vierem de directórios mutáveis.

---

## Roadmap

| Versão     | Âmbito                                                                           |
|------------|----------------------------------------------------------------------------------|
| `v0.3 atual` | Validação estrutural e semântica completa, integridade SHA-256, `ValidationResult`, NISS, CC, CP, telefone PT, MIME, semver, range, `ManifestList`, coerência com leap year. |
| `v0.4`     | `ValidationRuleDescriptor` e rule registry com metadados (scope, versão, vigência). |
| `v0.5`     | Mensagens em camadas (utilizador / técnica / auditável) por issue.               |
| `v0.6`     | `ValidationOverride` como tipo de primeira classe com evidência COSO.            |
| `v0.7`     | Hardening adicional: mitigação host-level de TOCTOU e políticas de sandbox de ficheiros. |

---

## Exemplos

### Validação estrutural simples

```rust
use core_validation::validators::{nif, iban, string};

let mut report = string::required("nome", "Alice");
report.merge(nif::validate_nif("nif", "501964843"));
report.merge(iban::validate_iban("iban", "PT50 0002 0123 1234 5678 9015 4"));

assert!(report.is_valid());
```

### Resultado institucional

```rust
use core_validation::{ValidationResult, ValidationContext, validators::nif};

let report = nif::validate_nif("nif", "501964843");

let result = ValidationResult::from_report(
    "val_2026_000123",
    "Pessoa",
    "pessoa_abc",
    Some(
        ValidationContext::new("2026-06-03T14:00:00Z")
            .with_actor("svc_onboarding")
            .with_scope("uo_rh")
    ),
    &report,
);

assert!(result.allows_progression());
```

### Integridade de ficheiro

```rust
use core_validation::{sha256_bytes, manifest_file, validate_sha256_hex};

let hash = sha256_bytes(b"payload");
assert!(validate_sha256_hex("payload_hash", &hash).is_valid());

let entry = manifest_file("/caminho/para/ficheiro.pdf").unwrap();
// entry.sha256: hash SHA-256 do ficheiro
// entry.size: tamanho em bytes
// entry.path: path normalizado com '/' como separador
```

### Coerência estrutural

```rust
use core_validation::validators::coherence;

// Data de início não pode ser posterior à data de fim
let report = coherence::validate_date_range("vigencia", "2026-01-01", "2026-12-31");
assert!(report.is_valid());

// Transições de estado permitidas
let allowed = [("rascunho", "em_revisao"), ("em_revisao", "aprovado")];
let report = coherence::validate_state_transition("estado", "rascunho", "em_revisao", &allowed);
assert!(report.is_valid());
```
