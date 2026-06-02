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
| `CompetencyService` | Atribuição de competências com OCC + auditoria |
| `DelegationService` | Delegação de competências (verifica posse) com OCC + auditoria |
| `OrgAuditPort` | Porto secundário (driven) de auditoria — implementado pelo adaptador infra |
| `OrgAuditOutbox` | Porto de evidência transaccional (captura atómica + drain idempotente) |
| `OrgDomainEventPort` | Porto secundário (driven) de eventos de domínio — notifica outros bounded contexts (`core-rh`) |
| `controls` | Identificadores dos controlos COSO evidenciados (`CTRL-ORG-*`, `CTRL-AUTH-004`) |

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

O serviço é o único ponto de entrada para escritas governadas. Cada operação,
por esta ordem: (1) valida invariantes, (2) persiste o estado **e** captura a
evidência (`OrgAuditEvent`) no outbox **atomicamente**, (3) entrega a evidência
ao `OrgAuditPort` (drain idempotente), (4) publica um `OrgDomainEvent`.

Cada serviço de agregado tem **três parâmetros genéricos** — repositório (`R`),
auditoria (`A`) e eventos de domínio (`E`):

```rust
use core_org::{OrgUnitService, OrgNoopAudit, OrgNoopDomainEvents};

// Em produção: substituir os Noop pelos adaptadores reais
// (ex.: OrgAuditAdapter de org-sqlite + um publisher de eventos).
let svc = OrgUnitService::new(repo, OrgNoopAudit, OrgNoopDomainEvents);

svc.create(unit, "joao.silva")?;  // validate_strict + hierarquia + evidência + evento
svc.suspend(&id, "joao.silva")?;  // transition_status + OCC + evidência + evento
svc.deactivate(&id, date, "x")?; // transition_status + guardas + evidência + evento
```

`CompetencyService` e `DelegationService` (dois genéricos: `R` + `A`) governam a
atribuição de competências e delegações com OCC e auditoria.

## Evidência COSO

A camada de serviço produz **evidência COSO-grade** — não apenas um trilho de
sucessos:

- **Sucesso E falha.** Uma operação rejeitada (validação, `VersionConflict`,
  guarda de extinção, ciclo de substituição) emite um `OrgAuditEvent` com
  `outcome = Failure` (e o código de erro no payload) **antes** de propagar o erro.
- **Ligação a controlos.** Cada operação refere o `control_id` COSO primário
  (`CTRL-ORG-UNIT-001`, `CTRL-ORG-POS-001`, `CTRL-ORG-COMP-001`, `CTRL-AUTH-004`…).
  O adaptador grava uma `ControlExecution` (`Passed`/`Failed`) ligada ao evento.
- **Captura atómica (outbox).** O evento de auditoria **e o evento de domínio**
  são escritos nos outboxes na mesma transacção do estado. Não há estado sem
  evidência, evidência sem estado, nem evento de integração perdido. A entrega
  aos portos é posterior, idempotente e **resiliente a poison messages** (uma
  mensagem que falha repetidamente vai para dead-letter sem bloquear a fila).
  Um `OrgOutboxDrainer` supervisionado faz a entrega diferida em background.

## Portos secundários (hexagonal)

`core-org` define os contratos; os adaptadores infra implementam-nos sem que o
domínio conheça `core-audit`, mensageria ou SQLite:

```
core-org ──define──→ OrgAuditOutbox (captura + drain) ─┐
         ──define──→ OrgAuditPort (entrega) ───────────┤
         ──define──→ OrgDomainEventPort ──────────────┐│
                                                       ↑│
org-sqlite ─ OrgSqliteStore: OrgAuditOutbox ───────────┘│
             OrgAuditAdapter: OrgAuditPort ──usa──→ core-audit (AuditStore + ControlRegistry)
camada de app ── publisher de eventos ─────────────────┘ ──→ core-rh, barramento, Tauri…
```

### Auditoria com payload, outcome e controlo

`OrgAuditEvent` transporta `event_id` (identidade estável), `actor`, `action`,
`entity_kind`, `entity_id`, `occurred_at`, `outcome` (Success/Failure),
`control_id` e `payload: Option<serde_json::Value>`. O `OrgAuditAdapter`
(em org-sqlite) converte-o para `core_audit::AuditEvent`
(`event_type = "org.<entidade>.<acção>"`) na cadeia de hashes, e grava a
`ControlExecution` correspondente no registo de controlos.

### Eventos de domínio

`OrgDomainEvent` cobre o ciclo de vida (`UnitCreated`, `PositionCreated`,
`UnitStatusChanged`, `PositionDeactivated`, …). Permite que `core-rh` reaja à
criação de uma posição (para criar a ocupação) sem acoplamento directo entre
domínios. São **capturados transaccionalmente** (no `org_domain_outbox`, junto
com o estado) e entregues com as mesmas garantias da auditoria — não se perdem se
o publisher falhar. `OrgNoopDomainEvents` descarta os eventos em testes.

### Drainer supervisionado

`OrgOutboxDrainer::new(repo, audit, events)` entrega ambos os outboxes:
`run_once()` devolve `DrainStats` (entregues, pendentes, dead-letter);
`run_forever(interval, on_tick)` corre num loop dedicado para monitorização.
A entrega inline (após cada operação) é best-effort; o drainer garante a
recuperação fiável e desacopla a latência do caminho de escrita.

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

61 testes de domínio. A integração (repositório, OCC, outbox transaccional de
auditoria **e de eventos de domínio**, dead-letter / poison messages, drainer,
`OrgAuditAdapter` ligado ao `core-audit` com `ControlExecution`, evidência de
falha, migração aditiva idempotente, serviços de competência e delegação) é
coberta por 34 testes em `org-sqlite`.

## Fronteiras e trabalho futuro

As seguintes ressalvas são **deliberadas** (fronteiras arquitecturais) ou
**afinações operacionais** — não comprometem a conformidade nem a integridade da
evidência, mas estão registadas para implementação futura:

1. **Autorização não verificada.** O `actor` das operações é aceite por confiança;
   o core-org não verifica se o actor *pode* realizar a operação (COSO `CTRL-AUTH-002`
   autorização, `CTRL-AUTH-005` segregação de funções). É fronteira de
   `core-security` / `core-rh`. *Futuro:* um porto `OrgAuthorizationPort` (driven)
   consultado pelo serviço antes de cada escrita, emitindo evidência de negação.
2. **`OrgOutboxDrainer::run_forever` usa `std::thread::sleep`.** Adequado a uma
   thread dedicada; num runtime async puro convém uma variante `async`.
   *Futuro:* feature opcional `tokio` com `run_forever_async`.
3. **Entrega inline sob o `Mutex`.** Cada operação tenta drenar de imediato (sob o
   lock da ligação). É correcto e sem perda, mas penaliza throughput.
   *Futuro:* tornar a entrega inline opcional (config) e delegar inteiramente no
   drainer supervisionado em cargas intensas.

Ver o TODO de projecto associado para o estado de cada item.
