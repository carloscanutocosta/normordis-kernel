# Guia de Conformance

Uma implementação diz-se **conforme** quando passa as três camadas de validação
para cada fixture do seu domínio:

| Camada | O que testa | Quando falha |
|--------|-------------|--------------|
| **1 — Schema** | Forma e tipo dos dados em JSON | Campo obrigatório ausente, tipo errado, enum inválido, pattern violado |
| **2 — Desserialização** | Mapeamento JSON → tipo nativo | Campo impossível de representar no modelo (ex: UUID malformado que serde rejeita) |
| **3 — Validação nativa** | Invariantes de negócio além da forma | Strings só de espaços, datas invertidas, listas vazias, regras contextuais |
| **4 — Round-trip** | Tipo nativo → JSON → schema (guarda de drift) | O tipo serializa um campo que o schema não cobre |

A camada 1 é executável por qualquer linguagem via JSON Schema.
As camadas 2, 3 e 4 requerem implementação nativa — este documento descreve
exactamente o que cada implementação deve verificar.

---

## Regra de cobertura

Nenhum ficheiro `.json` em `fixtures/valid/` ou `fixtures/invalid/` pode existir
sem estar registado no runner de conformance. Se um fixture existir em disco mas
não tiver teste correspondente, o runner deve falhar.

---

## Mapeamento fixture → schema

### Fixtures válidas

| Ficheiro (`fixtures/valid/`)              | Schema                                          |
|-------------------------------------------|-------------------------------------------------|
| `audit-event-minimal.json`                | `schemas/core/audit/audit-event.schema.json`         |
| `audit-event-with-control.json`           | `schemas/core/audit/audit-event.schema.json`         |
| `control-definition-auth.json`            | `schemas/core/audit/control-definition.schema.json`  |
| `control-execution-passed.json`           | `schemas/core/audit/control-execution.schema.json`   |
| `control-execution-dispensed.json`        | `schemas/core/audit/control-execution.schema.json`   |
| `config-app-config-basic.json`            | `schemas/core/config/app-config.schema.json`         |
| `org-unit-root.json`                      | `schemas/core/org/org-unit.schema.json`              |
| `rh-role-active.json`                     | `schemas/core/rh/role.schema.json`                   |
| `security-policy-strict.json`             | `schemas/core/security/policy.schema.json`           |
| `validation-result-passed.json`           | `schemas/core/validation/validation-result.schema.json` |
| `ingest-source-config-bundle.json`        | `schemas/core/ingest/source.schema.json`             |
| `exports-source-ref-config-profile.json`  | `schemas/core/exports/source-ref.schema.json`        |
| `support-mini-error.json`                 | `schemas/support/mini-error.schema.json`        |
| `support-technical-id.json`               | `schemas/support/technical-id.schema.json`      |
| `support-utc-timestamp.json`              | `schemas/support/utc-timestamp.schema.json`     |
| `support-postal-code.json`                | `schemas/support/postal-code.schema.json`       |
| `support-release-notes.json`              | `schemas/support/release-notes.schema.json`     |
| `support-normalization-case.json`         | `schemas/support/normalization-case.schema.json` |
| `support-storage-key.json`                | `schemas/support/storage-key.schema.json`       |
| `support-encrypted-payload.json`          | `schemas/support/encrypted-payload.schema.json` |
| `support-log-event.json`                  | `schemas/support/log-event.schema.json`         |
| `support-webauthn-challenge.json`         | `schemas/support/webauthn-challenge.schema.json` |
| `support-pdf-render-request.json`         | `schemas/support/render-request.schema.json`    |
| `support-typst-render-request.json`       | `schemas/support/render-request.schema.json`    |
| `support-docx-to-typst-request.json`      | `schemas/support/render-request.schema.json`    |
| `support-backup-archive-ref.json`         | `schemas/support/backup-archive-ref.schema.json` |
| `support-public-error.json`               | `schemas/support/public-error.schema.json`       |
| `audit-record-event-request.json`         | `schemas/core/audit/record-audit-event-request.schema.json` |
| `audit-chain-link-genesis.json`           | `schemas/core/audit/audit-chain-link.schema.json` |
| `audit-chain-link-chained.json`           | `schemas/core/audit/audit-chain-link.schema.json` |
| `audit-chain-report-verified.json`        | `schemas/core/audit/audit-chain-report.schema.json` |
| `audit-export-manifest.json`              | `schemas/core/audit/audit-export-manifest.schema.json` |
| `signed-audit-export-manifest.json`       | `schemas/core/audit/signed-audit-export-manifest.schema.json` |
| `audit-event-maximal.json`                | `schemas/core/audit/audit-event.schema.json` |
| `control-definition-maximal.json`         | `schemas/core/audit/control-definition.schema.json` |
| `org-unit-maximal.json`                   | `schemas/core/org/org-unit.schema.json` |
| `org-position-active.json`                | `schemas/core/org/org-position.schema.json` |
| `org-position-outro.json`                 | `schemas/core/org/org-position.schema.json` |
| `org-competency.json`                     | `schemas/core/org/competency.schema.json` |
| `org-delegation.json`                     | `schemas/core/org/delegation.schema.json` |
| `org-legal-instrument.json`               | `schemas/core/org/legal-instrument.schema.json` |
| `rh-user-identity.json`                   | `schemas/core/rh/user-identity.schema.json` |
| `rh-person-assignment.json`               | `schemas/core/rh/person-assignment.schema.json` |
| `rh-user-profile-full.json`               | `schemas/core/rh/user-profile.schema.json` |
| `rh-user-profile-minimal.json`            | `schemas/core/rh/user-profile.schema.json` |
| `validation-report-clean.json`            | `schemas/core/validation/validation-report.schema.json` |
| `validation-report-with-issues.json`      | `schemas/core/validation/validation-report.schema.json` |
| `security-auth-level.json`                | `schemas/core/security/auth-level.schema.json` |
| `security-classification.json`            | `schemas/core/security/resource-classification.schema.json` |
| `security-sod-rule.json`                  | `schemas/core/security/sod-rule.schema.json` |
| `security-context-full.json`              | `schemas/core/security/security-context.schema.json` |
| `security-context-minimal.json`           | `schemas/core/security/security-context.schema.json` |
| `exports-tabular-dataset.json`            | `schemas/core/exports/tabular-dataset.schema.json` |
| `exports-materialization-request.json`    | `schemas/core/exports/export-materialization-request.schema.json` |

### Fixtures inválidas

| Ficheiro (`fixtures/invalid/`)                     | Schema violado                                  |
|----------------------------------------------------|-------------------------------------------------|
| `audit-event-missing-actor.json`                   | `schemas/core/audit/audit-event.schema.json`         |
| `audit-event-missing-occurred-at.json`             | `schemas/core/audit/audit-event.schema.json`         |
| `audit-event-non-utc-offset.json`                  | `schemas/core/audit/audit-event.schema.json`         |
| `audit-event-blank-actor-id.json`                  | `schemas/core/audit/audit-event.schema.json`         |
| `control-definition-legacy-control-id.json`        | `schemas/core/audit/control-definition.schema.json`  |
| `control-execution-invalid-result.json`            | `schemas/core/audit/control-execution.schema.json`   |
| `control-execution-dispensed-missing-notes.json`   | `schemas/core/audit/control-execution.schema.json`   |
| `config-app-config-path-traversal.json`            | `schemas/core/config/app-config.schema.json`         |
| `org-unit-child-without-parent.json`               | `schemas/core/org/org-unit.schema.json`              |
| `rh-role-id-with-space.json`                       | `schemas/core/rh/role.schema.json`                   |
| `security-policy-empty-rules.json`                 | `schemas/core/security/policy.schema.json`           |
| `validation-result-empty-target.json`              | `schemas/core/validation/validation-result.schema.json` |
| `ingest-source-missing-kind.json`                  | `schemas/core/ingest/source.schema.json`             |
| `exports-source-ref-blank-subject.json`            | `schemas/core/exports/source-ref.schema.json`        |
| `support-mini-error-bad-code.json`                 | `schemas/support/mini-error.schema.json`        |
| `support-technical-id-not-uuid.json`               | `schemas/support/technical-id.schema.json`      |
| `support-utc-timestamp-offset.json`                | `schemas/support/utc-timestamp.schema.json`     |
| `support-postal-code-bad-cp3.json`                 | `schemas/support/postal-code.schema.json`       |
| `support-release-notes-empty-version.json`         | `schemas/support/release-notes.schema.json`     |
| `support-normalization-case-unknown-op.json`       | `schemas/support/normalization-case.schema.json` |
| `support-storage-key-path-token.json`              | `schemas/support/storage-key.schema.json`       |
| `support-encrypted-payload-missing-ciphertext.json` | `schemas/support/encrypted-payload.schema.json` |
| `support-log-event-bad-level.json`                 | `schemas/support/log-event.schema.json`         |
| `support-webauthn-challenge-missing-user.json`     | `schemas/support/webauthn-challenge.schema.json` |
| `support-pdf-render-request-empty-source.json`     | `schemas/support/render-request.schema.json`    |
| `support-typst-render-request-bad-kind.json`       | `schemas/support/render-request.schema.json`    |
| `support-docx-to-typst-request-empty-source.json`  | `schemas/support/render-request.schema.json`    |
| `support-backup-archive-ref-bad-hash.json`         | `schemas/support/backup-archive-ref.schema.json` |
| `support-public-error-bad-code.json`               | `schemas/support/public-error.schema.json`       |
| `org-position-missing-title.json`                  | `schemas/core/org/org-position.schema.json` |
| `org-delegation-missing-instrument.json`           | `schemas/core/org/delegation.schema.json` |
| `org-legal-instrument-invalid-kind.json`           | `schemas/core/org/legal-instrument.schema.json` |
| `rh-user-identity-invalid-role.json`               | `schemas/core/rh/user-identity.schema.json` |
| `rh-person-assignment-missing-basis.json`          | `schemas/core/rh/person-assignment.schema.json` |
| `rh-user-profile-invalid-role.json`                | `schemas/core/rh/user-profile.schema.json` |
| `validation-report-missing-valid.json`             | `schemas/core/validation/validation-report.schema.json` |
| `security-auth-level-unknown.json`                 | `schemas/core/security/auth-level.schema.json` |
| `security-classification-unknown.json`             | `schemas/core/security/resource-classification.schema.json` |
| `security-sod-rule-missing-blocked.json`           | `schemas/core/security/sod-rule.schema.json` |
| `security-context-missing-auth-level.json`         | `schemas/core/security/security-context.schema.json` |
| `exports-tabular-dataset-row-not-object.json`      | `schemas/core/exports/tabular-dataset.schema.json` |
| `exports-materialization-request-invalid-format.json` | `schemas/core/exports/export-materialization-request.schema.json` |
| `support-storage-key-too-long.json`                | `schemas/support/storage-key.schema.json` (STOR-R03: > 256 chars) |
| `support-crypto-argon2id-low-memory.json`          | `schemas/support/encrypted-payload.schema.json` (CRYPT-R03: memory_kib < 64) |

### Scenario fixtures (Camada 4 — invariantes inter-registo)

Ficheiros em `fixtures/scenarios/` descrevem sequências de registos que verificam
invariantes que não são detectáveis por fixture individual.

| Ficheiro | Invariante | Resultado esperado |
|---------|------------|-------------------|
| `audit-chain-valid.json` | CHAIN-R01 + CHAIN-R02 | Passa todas as invariantes |
| `audit-chain-broken-sequence.json` | CHAIN-R01 violado (sequence repetida) | Falha |
| `audit-chain-broken-hash.json` | CHAIN-R02 violado (previous_hash errado) | Falha |

### Fixtures de camada 3 (passam schema, falham validação nativa)

| Ficheiro (`fixtures/invalid/`)                     | Tipo validado                         |
|----------------------------------------------------|---------------------------------------|
| `control-definition-inverted-dates.json`           | `ControlDefinition` (valid_to < valid_from) |
| `org-unit-inverted-dates.json`                     | `OrgUnit` (valid_until < valid_from)  |
| `org-delegation-self.json`                         | `Delegation` (from == to)             |
| `rh-person-assignment-inverted-dates.json`         | `PersonAssignment` (valid_until < valid_from) |

---

## Validação nativa por domínio

Esta secção documenta o que a **camada 3** deve verificar em cada domínio.
O JSON Schema (camada 1) já captura forma e tipo; a validação nativa captura
invariantes que o schema não consegue expressar.

### core-audit

#### `AuditEvent`

| Invariante | Detalhe |
|-----------|---------|
| `event_id` não vazio após trim | Rejeitar strings só de espaços |
| `event_type` não vazio após trim | Rejeitar strings só de espaços; máx. 256 chars |
| `actor.actor_id` não vazio após trim | Rejeitar strings só de espaços |
| `actor.actor_name` não vazio após trim (se presente) | Rejeitar strings só de espaços |
| `actor.actor_type` não vazio após trim (se presente) | Rejeitar strings só de espaços |
| `target.target_type` não vazio após trim | Rejeitar strings só de espaços |
| `target.target_id` não vazio após trim | Rejeitar strings só de espaços |
| `control_id` não vazio após trim (se presente) | Rejeitar strings só de espaços; máx. 64 chars |
| `details_json` máx. 65 536 bytes serializados | Rejeitar payloads excessivos |

#### `ControlDefinition`

| Invariante | Detalhe |
|-----------|---------|
| `control_id` não vazio após trim | Máx. 64 chars |
| `name` não vazio após trim | Máx. 256 chars |
| `version` não vazio | — |
| `valid_to > valid_from` (se `valid_to` presente) | Rejeitar se igual ou anterior |
| Cada entrada de `implemented_by` não vazia após trim | Máx. 128 chars por entrada |
| Cada entrada de `references` não vazia após trim | Máx. 128 chars por entrada |

#### `ControlExecution`

| Invariante | Detalhe |
|-----------|---------|
| `execution_id` não vazio após trim | — |
| `control_id` não vazio após trim | Máx. 64 chars |
| `event_id` não vazio após trim | — |
| Se `result == "dispensed"` → `notes` obrigatório | Regra contextual não expressável em schema básico |
| `notes` máx. 1 024 chars (se presente) | — |

---

### core-config

O domínio config valida em cascata: `AppConfig` delega para sub-profiles.

#### `AppConfig` → `AppProfile`

| Invariante | Detalhe |
|-----------|---------|
| `app_id` não vazio | Máx. 128 chars; apenas `[a-zA-Z0-9_-]` |
| `display_name` não vazio | Máx. 255 chars |

#### `AppConfig` → `RuntimeProfile`

| Invariante | Detalhe |
|-----------|---------|
| `profile_name` não vazio | Sem espaços; máx. 64 chars |

#### `AppConfig` → `StorageProfiles`

| Invariante | Detalhe |
|-----------|---------|
| Lista `profiles` não vazia | Pelo menos um profile |
| `default_profile` referencia um profile existente | Rejeitar referências a nomes inexistentes |
| Nomes de profiles únicos | Sem duplicados na lista |
| Backend `memory`: não pode ser encriptado; não pode ter `database_path` | Invariante de consistência |
| Backend `sqlite`: `database_path` obrigatório e não vazio | — |
| `database_path` não pode conter traversal de caminho (`..`) | Invariante de segurança — rejeitar `../` ou `..\` |

#### `AppConfig` → `CryptoProfile` (se `enabled`)

| Invariante | Detalhe |
|-----------|---------|
| `key_id` não vazio | Sem espaços nem `:`; máx. 128 chars |

#### `AppConfig` → `LoggingProfile` (se `enabled`)

| Invariante | Detalhe |
|-----------|---------|
| `log_dir` não vazio | — |
| `file_name` sem separadores de caminho (`/` ou `\`) | Apenas nome simples, sem path |
| `max_file_size_mb > 0` | — |
| `max_files > 0` | — |
| `retention_days >= 1` | — |

#### `AppConfig` → `AuditProfile` (se `enabled`)

| Invariante | Detalhe |
|-----------|---------|
| `namespace` não vazio | Sem espaços nem `:`; máx. 128 chars |
| `storage_profile` não vazio | Sem espaços nem `:`; deve existir na lista de storage profiles com `purpose = audit` |

---

### core-org

#### `OrgUnit`

| Invariante | Detalhe |
|-----------|---------|
| `short_name` não vazio após trim | — |
| `full_name` não vazio após trim | — |
| `valid_until > valid_from` (se `valid_until` presente) | — |
| Unidade de nível 1 não tem `parent_id` | Raiz hierárquica |
| Unidade de nível > 1 tem `parent_id` | Toda a não-raiz tem pai |
| `contacts.email` formato válido (se presente) | `local@domain.tld` |
| `contacts.phone` mín. 7 dígitos (se presente) | — |
| `contacts.fax` mín. 7 dígitos (se presente) | — |
| `contacts.address.cp4` exactamente 4 dígitos (se presente) | Código postal português |
| `contacts.address.cp3` exactamente 3 dígitos (se presente) | Código postal português |

#### `OrgPosition`

| Invariante | Detalhe |
|-----------|---------|
| `code` não vazio após trim | — |
| `title` não vazio após trim | — |
| `substitutes` ≠ `id` (se presente) | Auto-substituição proibida |
| `valid_until > valid_from` (se `valid_until` presente) | — |

#### `Competency`

| Invariante | Detalhe |
|-----------|---------|
| `code` não vazio após trim | — |
| `description` não vazio após trim | — |
| `scope` não vazio após trim | — |
| `valid_until > valid_from` (se `valid_until` presente) | — |

#### `Delegation`

| Invariante | Detalhe |
|-----------|---------|
| `from_position ≠ to_position` | Não se pode delegar para si próprio |
| `valid_until > valid_from` (se `valid_until` presente) | — |

---

### core-rh

#### `Role`

| Invariante | Detalhe |
|-----------|---------|
| `id` (RoleId) não vazio | — |
| `id` sem espaços | Invariante de identificador |
| `name` não vazio após trim | — |

#### `UserProfile`

| Invariante | Detalhe |
|-----------|---------|
| `user_id` não vazio | Sem espaços; máx. 128 chars; apenas `[a-zA-Z0-9_\-.]` |
| `username` não vazio | Sem espaços |
| `display_name` não vazio | — |
| `email` formato válido (se presente) | RFC 5321; sem espaços |

#### `PersonAssignment`

| Invariante | Detalhe |
|-----------|---------|
| `position_id` não vazio | — |
| `unit_id` não vazio | — |
| `valid_until > valid_from` (se `valid_until` presente) | — |

---

### core-security

#### `Policy`

| Invariante | Detalhe |
|-----------|---------|
| `policy_id` não vazio após trim | — |
| `version` não vazio após trim | — |
| `rules` lista não vazia | Pelo menos uma regra |
| Cada `rule.code` não vazio após trim | — |
| `valid_until > valid_from` (se ambos presentes) | — |

---

### core-validation

#### `ValidationResult`

O tipo é um registo de resultado — não tem invariantes de validação nativa
além das garantidas pelo schema. A camada 3 limita-se a confirmar que o tipo
desserializa sem erro.

---

### core-ingest

#### `IngestSource`

| Invariante | Detalhe |
|-----------|---------|
| `kind` não vazio após trim | Categoria da fonte (ex: `file_bundle`, `api_push`) |
| `subject_id` não vazio após trim | Identificador da entidade ingerida |
| `version` não vazio após trim | Versão do protocolo ou schema da fonte |

---

### core-exports

#### `SourceRef`

| Invariante | Detalhe |
|-----------|---------|
| `kind` não vazio após trim | Tipo de referência de exportação |
| `subject_id` não vazio após trim | Identificador do sujeito exportado |
| `version` não vazio após trim | Versão do schema de exportação |

---

## Resolução de $ref locais

Todos os `$ref` com URI `https://normordis.local/...` devem ter um schema
correspondente com esse `$id` disponível. O teste `all_local_schema_refs_resolve`
do runner Rust verifica isto automaticamente para todos os schemas da spec.

Implementações Go devem registar os schemas de suporte no `Compiler` antes de
compilar o schema principal — ver exemplo na secção "Camada 1" acima.

---

## Implementação Rust

```sh
cargo test -p spec-conformance
```

O runner em `crates/spec-conformance/tests/contract_conformance.rs` executa
as três camadas para cada fixture registado. O teste `all_valid_contract_fixtures_are_mapped`
falha se existir um ficheiro em `fixtures/valid/` sem entrada no runner, e vice-versa.

---

## Implementação Go

### Dependências recomendadas

```go
// JSON Schema (suporta Draft 2020-12, resolução de $ref)
github.com/santhosh-tekuri/jsonschema/v6

// Para UUIDs
github.com/google/uuid
```

### Tabela de fixtures

O equivalente Go do `ContractSchema` do runner Rust. A fonte de verdade
do mapeamento completo é `crates/spec-conformance/tests/contract_conformance.rs`;
o teste de cobertura abaixo garante que esta tabela não diverge do disco.

```go
type fixtureEntry struct {
    fixture string // caminho relativo a normordis-spec/
    schema  string // caminho do schema relativo a normordis-spec/
}

// Excerto — manter sincronizado com fixtures/valid/ (o teste de cobertura falha se divergir).
var validFixtures = []fixtureEntry{
    {"fixtures/valid/audit-event-minimal.json", "schemas/core/audit/audit-event.schema.json"},
    {"fixtures/valid/audit-event-maximal.json", "schemas/core/audit/audit-event.schema.json"},
    {"fixtures/valid/security-context-full.json", "schemas/core/security/security-context.schema.json"},
    {"fixtures/valid/rh-user-profile-full.json", "schemas/core/rh/user-profile.schema.json"},
    // … restantes entradas conforme a tabela "Mapeamento fixture → schema" acima
}
```

### Estrutura do teste

```go
// conformance_test.go
package conformance_test

import (
    "bytes"
    "encoding/json"
    "os"
    "path/filepath"
    "strings"
    "testing"

    "github.com/santhosh-tekuri/jsonschema/v6"
)

// specRoot devolve o caminho absoluto para normordis-spec/.
// Assumindo que o teste corre na raiz do repo Go, ajustar conforme necessário.
func specRoot(t *testing.T) string {
    // ex: ../normordis-spec se o repo Go for irmão de normordis-kernel
    root := os.Getenv("NORMORDIS_SPEC_PATH")
    if root == "" {
        t.Fatal("NORMORDIS_SPEC_PATH não definido")
    }
    return root
}

// loadFixture lê e faz parse de um fixture JSON.
func loadFixture(t *testing.T, specRoot, relative string) any {
    t.Helper()
    data, err := os.ReadFile(filepath.Join(specRoot, relative))
    if err != nil {
        t.Fatalf("não foi possível ler %s: %v", relative, err)
    }
    var v any
    if err := json.Unmarshal(data, &v); err != nil {
        t.Fatalf("JSON inválido em %s: %v", relative, err)
    }
    return v
}
```

### Camada 1 — validação de schema

```go
func buildValidator(t *testing.T, specRoot, schemaPath string) *jsonschema.Schema {
    t.Helper()
    c := jsonschema.NewCompiler()
    // Registar schemas de suporte para resolução de $ref.
    // O URI registado deve coincidir com o "$id" do schema referenciado.
    for _, sup := range []string{
        "schemas/core/audit/audit-actor.schema.json",
        "schemas/core/audit/audit-target.schema.json",
        "schemas/core/audit/audit-outcome.schema.json",
        "schemas/core/audit/control-category.schema.json",
    } {
        data, _ := os.ReadFile(filepath.Join(specRoot, sup))
        _ = c.AddResource("https://normordis.local/spec/"+sup, bytes.NewReader(data))
    }
    schema, err := c.Compile(filepath.Join(specRoot, schemaPath))
    if err != nil {
        t.Fatalf("falha a compilar schema %s: %v", schemaPath, err)
    }
    return schema
}

func TestValidFixturesPassSchema(t *testing.T) {
    root := specRoot(t)
    for _, entry := range validFixtures {
        t.Run(entry.fixture, func(t *testing.T) {
            v := buildValidator(t, root, entry.schema)
            instance := loadFixture(t, root, entry.fixture)
            if err := v.Validate(instance); err != nil {
                t.Errorf("%s deveria ser válido: %v", entry.fixture, err)
            }
        })
    }
}
```

### Camada 3 — validação nativa

Para cada tipo nativo, implementar uma função `Validate() error` que aplica
as invariantes descritas na secção anterior. Exemplo para `AuditEvent`:

```go
func (e *AuditEvent) Validate() error {
    if strings.TrimSpace(e.EventID) == "" {
        return errors.New("event_id não pode ser vazio")
    }
    if strings.TrimSpace(e.EventType) == "" {
        return errors.New("event_type não pode ser vazio")
    }
    if strings.TrimSpace(e.Actor.ActorID) == "" {
        return errors.New("actor.actor_id não pode ser vazio")
    }
    if e.Actor.ActorName != nil && strings.TrimSpace(*e.Actor.ActorName) == "" {
        return errors.New("actor.actor_name não pode ser só espaços")
    }
    if strings.TrimSpace(e.Target.TargetType) == "" {
        return errors.New("target.target_type não pode ser vazio")
    }
    if strings.TrimSpace(e.Target.TargetID) == "" {
        return errors.New("target.target_id não pode ser vazio")
    }
    if e.ControlID != nil && strings.TrimSpace(*e.ControlID) == "" {
        return errors.New("control_id não pode ser só espaços")
    }
    return nil
}
```

Para `ControlExecution`, a regra contextual `dispensed → notes obrigatório`:

```go
func (e *ControlExecution) Validate() error {
    // ... validações básicas ...
    if e.Result == "dispensed" && (e.Notes == nil || strings.TrimSpace(*e.Notes) == "") {
        return errors.New("notes é obrigatório quando result é 'dispensed'")
    }
    return nil
}
```

### Camada 4 — round-trip (guarda de drift)

Para cada tipo nativo, serializar uma instância desserializada de volta para JSON
e validar contra o schema. Se o tipo Go ganhar um campo que o schema não cobre,
este teste falha — exactamente como o `round_trip_rust_to_json_validates_schema`
do runner Rust.

```go
func TestRoundTripValidatesSchema(t *testing.T) {
    root := specRoot(t)
    for _, entry := range validFixtures {
        t.Run(entry.fixture, func(t *testing.T) {
            // 1. ler fixture e desserializar no tipo nativo (ex: AuditEvent)
            typed := decodeNative(t, root, entry) // serde-equivalente por tipo
            // 2. re-serializar
            reJSON, err := json.Marshal(typed)
            if err != nil {
                t.Fatalf("round-trip: marshal falhou: %v", err)
            }
            // 3. validar o JSON re-serializado contra o schema
            var generic any
            _ = json.Unmarshal(reJSON, &generic)
            v := buildValidator(t, root, entry.schema)
            if err := v.Validate(generic); err != nil {
                t.Errorf("DRIFT em %s: o tipo Go serializa campos fora do schema: %v",
                    entry.fixture, err)
            }
        })
    }
}
```

> Nota sobre `null`: em Rust, `Option<T>` sem `#[serde(skip_serializing_if)]`
> serializa como `null`. Os schemas que cobrem esses campos usam
> `"type": ["string", "null"]`. Em Go, campos `*T` com `omitempty` omitem o campo
> em vez de emitir `null` — confirmar que a serialização Go corresponde ao schema
> (ou ajustar as tags `json:` para reproduzir o comportamento Rust quando o
> contrato exigir `null` explícito).

### Cobertura de fixtures

O runner Go deve verificar que nenhum fixture fica por testar:

```go
func TestAllValidFixturesAreMapped(t *testing.T) {
    root := specRoot(t)
    entries, _ := os.ReadDir(filepath.Join(root, "fixtures/valid"))
    mapped := make(map[string]bool)
    for _, f := range validFixtures {
        mapped[filepath.Base(f.fixture)] = true
    }
    for _, e := range entries {
        if strings.HasSuffix(e.Name(), ".json") && !mapped[e.Name()] {
            t.Errorf("fixture %s não tem mapeamento de conformance", e.Name())
        }
    }
}
```
