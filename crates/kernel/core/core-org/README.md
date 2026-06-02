# core-org

Domínio de estrutura orgânica institucional — hierarquia, cargos, competências,
delegações e instrumentos jurídicos. Núcleo puro sem dependências de infra.

## Responsabilidade

| Conceito | Descrição |
|---|---|
| `OrgUnit` | Unidade orgânica com validade temporal, nível hierárquico ilimitado e OCC |
| `OrgPosition` | Cargo abstracto (tipo, substituto legal, status); ocupação é `core-rh` |
| `Competency` | Autoridade jurídica para praticar actos, associada a uma posição |
| `Delegation` | Delegação temporária de competência entre posições |
| `LegalInstrument` | Portaria / Despacho / Deliberação que fundamenta cada alteração |
| `OrgUnitService` | Orquestra invariantes + auditoria + eventos de domínio para unidades |
| `OrgPositionService` | Orquestra invariantes + auditoria + eventos de domínio para posições |
| `OrgAuditPort` | Porto secundário (driven) de auditoria — implementado pelo adaptador infra |
| `OrgDomainEventPort` | Porto secundário (driven) de eventos de domínio — notifica outros bounded contexts (`core-rh`) |

## Não-responsabilidade

- Não conhece SQLite, filesystem, Tauri ou UI.
- Não decide quem ocupa uma posição (`core-rh`).
- Não decide autorização de operações (`core-security`).
- Não gera IDs — passados pelo chamador.

## Modelo de domínio

```
LegalInstrument ─── fundamenta ──→ OrgUnit (hierarquia ilimitada, OCC)
                                        │ tem ──→ OrgPosition (PositionKind, substitutes, OCC)
                                                      │ tem ──→ Competency
                                                      └──────── Delegation (para outra OrgPosition)
```

## Camada de serviço

O serviço é o único ponto de entrada para escritas em produção. Cada operação,
por esta ordem: (1) valida invariantes, (2) persiste no repositório, (3) emite um
`OrgAuditEvent` (com payload do estado resultante), (4) publica um `OrgDomainEvent`.

Cada serviço tem **três parâmetros genéricos** — repositório (`R`), auditoria (`A`)
e eventos de domínio (`E`):

```rust
use core_org::{OrgUnitService, OrgNoopAudit, OrgNoopDomainEvents};

// Em produção: substituir os Noop pelos adaptadores reais
// (ex.: OrgAuditAdapter de org-sqlite + um publisher de eventos).
let svc = OrgUnitService::new(repo, OrgNoopAudit, OrgNoopDomainEvents);

svc.create(unit, "joao.silva")?;  // validate_strict + hierarquia + auditoria + evento
svc.suspend(&id, "joao.silva")?;  // transition_status + update + auditoria + evento
svc.deactivate(&id, date, "x")?; // transition_status + deactivate + auditoria + evento
```

## Portos secundários (hexagonal)

`core-org` define os contratos; os adaptadores infra implementam-nos sem que o
domínio conheça `core-audit`, mensageria ou SQLite:

```
core-org ──define──→ OrgAuditPort ─────────┐
         ──define──→ OrgDomainEventPort ──┐ │
                                          │ ↑
org-sqlite ──implementa OrgAuditAdapter ──┼─┘ ──usa──→ core-audit::AuditStore
                                          │
camada de app ──implementa publisher ─────┘ ──→ core-rh, barramento, Tauri…
```

### Auditoria com payload

`OrgAuditEvent` transporta `actor`, `action`, `entity_kind`, `entity_id`,
`occurred_at` e um `payload: Option<serde_json::Value>` com o estado resultante
(snapshot/delta). O `OrgAuditAdapter` (em org-sqlite) converte-o para
`core_audit::AuditEvent` (`event_type = "org.<entidade>.<acção>"`,
`outcome = Success`) e grava-o no `AuditStore` com cadeia de hashes.

### Eventos de domínio

`OrgDomainEvent` cobre o ciclo de vida (`UnitCreated`, `PositionCreated`,
`UnitStatusChanged`, `PositionDeactivated`, …). Permite que `core-rh` reaja à
criação de uma posição (para criar a ocupação) sem acoplamento directo entre
domínios. `OrgNoopDomainEvents` descarta os eventos em testes e contextos sem
integração configurada.

## Invariantes garantidas pelo serviço

- `validate_strict` — exige `created_by` ou `legal_reference` em modo operacional.
- Nível hierárquico = pai.nível + 1 exactamente (sem buracos).
- Detecção de ciclos hierárquicos (via cadeia de ancestrais).
- Detecção de ciclos de substituição (posição A substitui B que substitui A).
- Máquina de estados: Active ↔ Suspended → Extinct (terminal).
- OCC: `update` falha com `VersionConflict` se a versão não coincidir.

## Validações de contacto

Email (formato básico), telefone (≥ 7 dígitos), CP4 (4 dígitos), CP3 (3 dígitos).

## Exemplo mínimo

```rust
use core_org::{
    OrgLevel, OrgUnit, OrgUnitId, OrgUnitStatus, OrgContacts,
    OrgUnitService, OrgNoopAudit, OrgNoopDomainEvents,
};
use chrono::NaiveDate;

let unit = OrgUnit {
    id: OrgUnitId::new("unit-sf-beja")?,
    short_name: "SF Beja".into(),
    full_name: "Serviço de Finanças de Beja".into(),
    service_code: Some("0312".into()),
    level: OrgLevel::new(3)?,
    parent_id: Some(OrgUnitId::new("unit-df-beja")?),
    contacts: OrgContacts::default(),
    created_by: None,
    legal_reference: Some("Portaria n.º 150/2024".into()),
    valid_from: NaiveDate::from_ymd_opt(2024, 5, 1).unwrap(),
    valid_until: None,
    status: OrgUnitStatus::Active,
    version: 0,
};

let svc = OrgUnitService::new(repo, OrgNoopAudit, OrgNoopDomainEvents);
svc.create(unit, "admin")?;
```

## Testes

```sh
cargo test -p core-org
```

61 testes de domínio. A integração (repositório + `OrgAuditAdapter` ligado ao
`core-audit`) é coberta por 21 testes em `org-sqlite`.
