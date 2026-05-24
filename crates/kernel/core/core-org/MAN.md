# Manual do módulo core-org

## Objetivo

`core-org` é o núcleo de domínio para estrutura orgânica institucional no
Mini-Kernel RS. Cobre a hierarquia de unidades orgânicas, cargos, competências,
delegações e os instrumentos jurídicos que fundamentam todas as alterações
estruturais. Exporta tipos, invariantes e ports de persistência.

## Contrato público

### Tipos principais

```rust
// Unidades orgânicas
OrgUnit             // agregado central — hierarquia + validade temporal
OrgUnitId           // identificador da unidade
OrgLevel            // nível hierárquico (1–5; 1 = topo)
OrgUnitStatus       // Active | Suspended | Extinct
OrgAddress          // endereço postal (CP4-CP3)
OrgContacts         // email, telefone, fax, endereço

// Posições
OrgPosition         // cargo abstracto — independente de quem o ocupa
OrgPositionId

// Competências
Competency          // autoridade jurídica para praticar actos administrativos
CompetencyId

// Delegações
Delegation          // delegação de competência com validade temporal
DelegationId

// Instrumentos jurídicos
LegalInstrument     // instrumento que fundamenta alterações estruturais
LegalInstrumentId
InstrumentKind      // Portaria | Despacho | Deliberacao | RegulamentoOrganico | Outro(String)

// Erro
OrgError
```

### Ports de persistência

```rust
OrgUnitRepository        // CRUD + hierarquia + desactivação
OrgPositionRepository    // CRUD por unidade
LegalInstrumentRepository// save + listagens por vigência
CompetencyRepository     // save + list_for_position_at
DelegationRepository     // save + get_effective_at
```

## Hierarquia de unidades

```
Nível 1 (raiz)
  └─ Nível 2
       └─ Nível 3
            └─ Nível 4
                 └─ Nível 5 (folha máxima)
```

Regras:
- `OrgLevel` aceita valores 1–5; valores fora desse intervalo são rejeitados.
- Unidade de nível 1 não tem `parent_id`. Unidades de nível 2–5 obrigam `parent_id`.
- O nível do pai deve ser exatamente `level - 1`; esta verificação é responsabilidade
  do service layer (que deve carregar o pai e comparar).
- Ciclos hierárquicos são detectados por `OrgUnit::validate_parent_chain`, que recebe
  a cadeia de ancestrais fornecida pelo chamador.

## Máquina de estados de OrgUnit

```
Active ──→ Suspended ──→ Active
  │              │
  └──→ Extinct   └──→ Extinct
       (terminal)       (terminal)
```

Regras:
- `Extinct` é estado terminal — nenhuma transição é válida a partir dele.
- A transição é validada por `OrgUnitStatus::can_transition_to`.
- `OrgUnit::transition_status` devolve o próximo status sem o aplicar ao agregado;
  a persistência é responsabilidade do adapter.

## Serialização de InstrumentKind

O formato canónico de texto, usado tanto no domínio como nos adapters SQLite:

| Variante              | String                    |
|-----------------------|---------------------------|
| `Portaria`            | `"portaria"`              |
| `Despacho`            | `"despacho"`              |
| `Deliberacao`         | `"deliberacao"`           |
| `RegulamentoOrganico` | `"regulamento_organico"`  |
| `Outro(s)`            | `"outro:{s}"`             |

Os adapters SQLite devem usar `InstrumentKind::as_str()` / `str::parse::<InstrumentKind>()`
em vez de implementar funções locais de conversão, garantindo consistência.

## Delegações

- Uma delegação não transfere a competência — o delegante mantém-na.
- `Delegation::validate_can_delegate` recebe as competências activas do `from_position`
  na data relevante; o chamador deve obtê-las via `CompetencyRepository::list_for_position_at`.
- `from_position` e `to_position` não podem ser a mesma posição.
- Toda a delegação requer instrumento jurídico próprio (`instrument_id`).

## Invariantes

- `OrgLevel` aceita apenas 1–5; `OrgLevel::new` rejeita valores fora do intervalo.
- `OrgUnit::validate` rejeita `short_name` ou `full_name` vazios, ranges temporais
  inválidos (`valid_until <= valid_from`) e inconsistências nível/pai.
- `OrgPosition::validate` rejeita `code` ou `title` vazios e ranges temporais inválidos.
- `Competency::validate` rejeita `code`, `description` ou `scope` vazios e ranges temporais.
- `Delegation::validate` rejeita `from_position == to_position` e ranges temporais inválidos.
- `LegalInstrument::validate` rejeita `reference` ou `description` vazios e ranges temporais.
- `OrgUnitRepository::create` deve rejeitar com `AlreadyExists` se a unidade já existe.
- `OrgUnitRepository::deactivate` deve verificar filhos e posições activas antes de extinguir.
- `AttachmentStore` e `NdfArchive` não pertencem a este crate.

## Decisões de design

### Posições independentes de ocupantes

`OrgPosition` é abstracta — não contém referência a quem ocupa o cargo.
A ocupação de uma posição por uma pessoa é responsabilidade de `core-rh`, que
referencia `OrgPositionId` sem criar dependência circular.
Esta separação preserva a estrutura orgânica mesmo quando não há ocupantes,
e permite que `core-org` seja usado sem `core-rh`.

### Instrumentos jurídicos como base legal obrigatória

Toda entidade com validade temporal em `core-org` deve referenciar o instrumento
jurídico que a criou ou modificou. `OrgUnit.created_by` é `Option<LegalInstrumentId>`
apenas para permitir importação de dados históricos sem instrumento formal registado.
Em código novo, deve sempre ser preenchido.

### Delegação não transfere competência

Decisão explícita de direito administrativo português: o delegante não abdica da
sua competência ao delegar. `validate_can_delegate` verifica apenas que o delegante
detém efectivamente a competência; não a remove após delegação.

### Sem service layer no crate

`core-org` exporta apenas o modelo de domínio e os ports. A orquestração
(ex: criar unidade com validação de hierarquia em transação) é responsabilidade
do service layer da app.

## Erros

`OrgError` cobre todos os erros de domínio. Os mais relevantes:

| Variante                                | Situação                                                        |
|-----------------------------------------|-----------------------------------------------------------------|
| `InvalidLevel(String)`                  | Nível hierárquico fora do intervalo 1–5                         |
| `InvalidTemporalRange`                  | `valid_until <= valid_from`                                     |
| `CircularHierarchy`                     | Hierarquia circular detectada por `validate_parent_chain`       |
| `InconsistentLevel`                     | Nível inconsistente com presença/ausência de `parent_id`        |
| `ExtinctUnit`                           | Operação proibida em unidade extinta                            |
| `CannotDeactivateWithActiveChildren`    | Tentativa de desactivar unidade com filhos activos              |
| `CannotDeactivateWithActivePositions`   | Tentativa de desactivar unidade com posições activas            |
| `AlreadyExists(String)`                 | Criação de entidade já existente                                |
| `EmptyField(String)`                    | Campo obrigatório vazio                                         |
| `OperationFailed(String)`               | Erro de operação genérico (inclui delegação inválida)           |

`OrgError` implementa `From<OrgError> for MiniError` para conversão pelo service layer.

## Dependências

```
support-errors — MiniError, ErrorCode, Component
```

`core-org` não depende de `core-rh`, `core-documental`, `core-audit`, SQLite,
filesystem, Tauri ou UI.

## Análise de completude

### O que está implementado

- Agregado `OrgUnit` com hierarquia, validade temporal e máquina de estados.
- Posições orgânicas abstractas com validade temporal.
- Competências com validade temporal e verificação de eficácia.
- Delegações com validação de posse de competência pelo delegante.
- Instrumentos jurídicos como base legal com `InstrumentKind` extensível.
- Ports para todos os conceitos acima.
- `as_str`, `from_str`, `TryFrom<&str>` para `OrgUnitStatus` e `InstrumentKind`.
- 36 testes unitários de invariantes de domínio.

### Lacunas conhecidas

- `OrgUnitRepository` não tem método de contagem ou listagem paginada.
- Não há validação de que o nível do pai é exactamente `level - 1` no domínio;
  esta verificação recai sobre o service layer.
- `OrgPosition` não tem `status` próprio — a "desactivação" é feita via `valid_until`.
- Sem `OrgOrgService` — a orquestração transaccional é responsabilidade do caller.
- `OrgUnitRepository::deactivate` não tem contrato formal para a data de transição
  de status; os adapters devem documentar o comportamento.

## ToDo

- Contrato de listagem paginada em `OrgUnitRepository` (com filtros por status e data).
- Validação no domínio de que `parent.level == self.level - 1` (requer acesso ao pai).
- `OrgPosition::status` para ciclo de vida explícito de posições.
- `OrgOrgService` ou equivalente para orquestração transaccional.
- Avaliar se `OrgUnit.created_by` deve ser `LegalInstrumentId` (obrigatório) em vez de
  `Option<LegalInstrumentId>`, quebrando a compatibilidade com dados históricos.
