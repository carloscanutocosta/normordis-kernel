# TODO — normordis-kernel

Itens de trabalho planeado para versões futuras, organizados por módulo.

**Convenção:**
- `[P1]` — prioritário, próxima iteração relevante
- `[P2]` — importante, sem urgência imediata
- `[P3]` — desejável, implementar quando o contexto de uso for claro
- Cada item tem: o quê, porquê, e quando faz sentido atacar

Itens de linha (bug fixes, refactors pequenos) vivem em comentários `// TODO:` no código.
Este ficheiro é para trabalho de nível arquitectural ou de feature.

---

## core-rh

### [P2] RoleService com evidência COSO

**O quê:** `RoleRepository.upsert` e `deactivate` são operações de catálogo sem rastro
de auditoria. Faltam variantes `upsert_audited` e `deactivate_audited` no port, e um
`RoleService<R: RoleRepository + RhAuditOutbox>` que emita `COSO.RH.UPSERT_ROLE` e
`COSO.RH.DEACTIVATE_ROLE`, à semelhança de `UserService`.

**Pré-condição:** `RoleRepository` usa `type Error: std::error::Error` (associated type),
ao contrário de `UserRepository` e `PersonAssignmentRepository` que devolvem `RhError`
directamente. Antes de implementar `RoleService`, decidir se `RoleRepository` migra
para o padrão consistente (`RhError` directo). Se não migrar, o bound do serviço
será `R::Error: Into<RhError>`, o que complica o serviço sem benefício real.

**Porquê:** roles são catálogo administrativo, menos sensíveis que utilizadores e
afetações; por isso foram deixados para depois. Num sistema legal e audit-compliant
by design, criação e desactivação de roles também deve ser auditável.

**Quando:** quando `workspace-governance` começar a gerir roles funcionais directamente,
e não antes — o momento de uso concreto definirá a API melhor do que especulação.

---

### [P3] Temporal role assignments (UserRoleAssignment)

**O quê:** `UserProfile.roles: Vec<Role>` é um snapshot estático do momento de leitura.
Não existe registo de quando um utilizador adquiriu ou perdeu um role. A entidade
necessária é:

```rust
pub struct UserRoleAssignment {
    pub id: UserRoleAssignmentId,
    pub user_id: UserId,
    pub role_id: RoleId,
    pub basis: String,        // despacho que conferiu o role
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    pub version: u32,         // OCC
}
```

Padrão: igual a `PersonAssignment` — port `UserRoleAssignmentRepository` com
`_audited` variants, `UserRoleAssignmentService` com evidência COSO.

**Porquê:** para responder a "que roles tinha o utilizador X em 14/03/2025?",
o sistema actualmente não tem resposta. Para rastreabilidade COSO de autorização,
esta informação é necessária quando roles determinam o que um actor pode fazer.

**Quando:** quando houver um caso de uso concreto que exija rastreabilidade temporal
de roles — não before. `workspace-governance` é o candidato mais provável.

---

## core-audit

### [P2] AuditFacade — ponto de entrada unificado

**O quê:** uma struct `AuditFacade<A, C>` que compõe `AuditService<A>` e
`ControlRegistryService<C>` num único ponto de entrada, com um método
`record_with_controls(request, controls)` para o fluxo mais comum.

**Porquê:** o fluxo "registar evento + registar execuções de controlos" exige
hoje gerir dois serviços e dois stores separados. Para um developer que
programa sobre o NORMORDIS, isto cria fricção no ponto de entrada mais comum.

**Proposta mínima:**
```rust
pub struct AuditFacade<A: AuditStore, C: ControlRegistryStore> { ... }

impl<A, C> AuditFacade<A, C> {
    pub fn record_with_controls(
        &self,
        request: RecordAuditEventRequest,
        controls: Vec<(String, ControlExecutionResult)>,
    ) -> Result<AuditEvent, AuditError>;

    pub fn audit(&self) -> &AuditService<A>;
    pub fn registry(&self) -> &ControlRegistryService<C>;
}
```

**Quando:** depois de `workspace-governance` usar ambos os serviços em cenários
reais. Os padrões de uso concretos definirão a API melhor do que especulação
antecipada.

---

### [P3] Adapter SQLite para ControlRegistryStore

**O quê:** `ControlRegistrySqliteStore` em `adapter-audit-sqlite`, a par de
`AuditSqliteStore`.

**Porquê:** `StorageControlRegistryStore` carrega índices inteiros em memória.
Para volumes acima de ~100K registos, ou para produção, é necessário um backend
relacional com índices SQL eficientes.

**Quando:** quando `workspace-governance` ou qualquer app de produção precisar
de escalar o Control Registry.

---

## core-validation

### [P2] ValidationRuleDescriptor e rule registry

**O quê:** uma struct `ValidationRuleDescriptor` com metadados formais de cada
regra (rule_id, name, scope, type, severity, mode, version, valid_from, description,
fundamento legal) e um `RuleRegistry` que as agrega e permite consulta por id e scope.

**Porquê:** actualmente as regras são constantes de string sem metadados. Sem registry
não é possível responder a "que versão desta regra estava vigente em 2026-01-15?" nem
auditar mudanças de política de validação ao longo do tempo. O registry é o que
transforma regras de validação em controlos governáveis.

**Quando:** quando `workspace-governance` ou `core-documental` precisar de consultar
regras activas para um scope específico. O consumidor concreto definirá o contrato
certo — não antes.

---

### [P2] Mensagens em camadas por ValidationIssue

**O quê:** adicionar `user_message: Option<String>` e `audit_message: Option<String>`
a `ValidationIssue`, a par do `message` técnico existente. Cada camada serve uma
audiência distinta: utilizador final (simples, sem jargão técnico), técnica (actual),
auditável (formal, para registo institucional).

**Porquê:** actualmente existe um único campo `message` que serve todas as audiências.
Para NORMORDIS, a mensagem que aparece ao utilizador ("Não é possível submeter: falta
a versão do template") deve ser distinta da mensagem técnica ("DOC.TEMPLATE_VERSION
missing in DocumentInstance") e da mensagem auditável ("Submissão bloqueada por ausência
de template_version obrigatório — rule DOC.TEMPLATE_VERSION.REQUIRED@1.0").

**Quando:** quando existir código de UI ou de serviço que consuma `ValidationIssue`
de forma diferenciada por audiência. É uma breaking change em `ValidationIssue` — todos
os validators precisam de ser actualizados. Sem consumidor concreto é prematura.

---

### [P3] ValidationOverride como tipo de primeira classe

**O quê:** uma struct `ValidationOverride` com `override_id`, `rule_id`, `actor_id`,
`justification` e `timestamp_rfc3339`, produzida pelo serviço que autoriza ultrapassar
uma validação bloqueante. O `RuleOutcome::overridden()` actual aceita apenas uma string
de justificação sem identidade nem timestamp.

**Porquê:** para evidência COSO de overrides, não chega registar "foi ultrapassado" —
é necessário saber quem autorizou, quando e com que fundamento. Sem este tipo, um
override não tem evidência auditável suficiente para cenários probatórios.

**Quando:** quando um serviço real precisar de registar um override com evidência COSO
completa. O tipo pertence à camada de serviço que chama `core-validation` — o crate
de validação produz a evidência, o serviço cria o override. A fronteira exacta vai ser
clara quando houver um caso de uso concreto.

### [P3] ULID format validator

**O quê:** `validate_ulid(field, value)` em `validators/ulid.rs`.
ULID (Universally Unique Lexicographically Sortable Identifier): 26 caracteres
em base32 Crockford (`[0-9A-HJKMNP-TV-Z]`), 10 chars de timestamp + 16 chars aleatórios.
Não requer dependência externa — a validação é estrutural (comprimento e charset).

**Porquê:** ULIDs são ordenáveis por timestamp e são uma alternativa natural a UUIDs em
sistemas de eventos. Se o kernel adoptar ULIDs para IDs de eventos de auditoria ou
outbox, o validator é necessário imediatamente. Sem esse consumidor, é prematuro.

**Quando:** quando um crate do kernel definir IDs como ULID em vez de UUID.

---

### [P3] Número de telefone internacional (E.164)

**O quê:** `validate_phone_e164(field, value)` — valida formato E.164:
`+` seguido de 7 a 15 dígitos (ex: `"+351912345678"`, `"+12025551234"`).
Complemento ao `validate_phone_pt` já existente para contextos multi-país
(notificações internacionais, contactos de representantes estrangeiros).

**Porquê:** E.164 é o formato canónico para números de telefone em sistemas
de notificação (SMS, VoIP). O `validate_phone_pt` cobre o caso nacional;
E.164 cobre o caso internacionalizado.

**Quando:** quando existir um canal de notificação ou formulário que aceite
números de telefone de múltiplos países.

---

## core-communications (futuro)

### [P2] Avaliar promoção dos portos de email para core-communications

**O quê:** `core-validation` passou a expor portos técnicos mínimos de email
(`EmailVerificationPort`, `EmailDeliveryPort`) porque o crate já concentra a
validação/verificação estrutural de email. Quando surgirem fluxos reais de
notificação, avaliar a criação de `crates/kernel/core/core-communications` para
concentrar contratos e semântica institucional de comunicações.

O eventual `core-communications` deve cobrir apenas o que for domínio transversal:

- mensagens institucionais como artefactos identificáveis;
- destinatários, canais, prioridades e estado de entrega;
- evidência de envio, tentativa, falha, retry e confirmação;
- políticas de consentimento, retenção, classificação e auditoria;
- portos de delivery (`EmailDeliveryPort`, SMS/push quando existirem);
- integração com `core-audit` via chamador, sem depender dele directamente.

**Critério core vs support vs infra:**

- `core-communications` só deve existir se houver semântica institucional:
  mensagem como artefacto de domínio, estado, destinatários, canal, evidência,
  retry, retenção, política, consentimento/classificação e auditoria.
- `support-*` é adequado apenas para primitivas técnicas puras e headless, sem
  estado institucional: normalização de endereços, templates de corpo, tipos
  auxiliares, serialização ou validações estruturais reutilizáveis.
- `infra/*` contém sempre mecanismos concretos de entrega e verificação: SMTP,
  Microsoft Graph, provider HTTP, DNS/MX, credenciais, TLS, rede e integração
  com serviços externos.

Regra curta: **semântica institucional = core; primitiva técnica = support;
mecanismo externo concreto = infra**.

**Porquê:** envio de email é transversal a muitas apps, mas promover cedo demais
pode criar um core genérico e cerimonial. A separação actual permite usar Graph/DNS
já em infra, sem impedir uma promoção limpa para `core-communications` quando os
casos de uso provarem a necessidade.

**Quando:** quando pelo menos duas apps precisarem de workflows de comunicação com
estado/auditoria, ou quando houver requisitos formais de notificações, recibos,
retries e evidência institucional.

---

## workspace-governance (mini-apps-rusty)

### [P1] Implementar app workspace-governance

**O quê:** a primeira app de referência sobre o kernel NORMORDIS, em
`mini-apps-rusty`. Usa `core-audit`, `ControlRegistryService`,
`domain-registry` e `domain-telemetry`.

**Porquê:** é o primeiro cenário de uso real que vai validar (ou refutar)
as APIs actuais dos crates do kernel. É aqui que padrões como `AuditFacade`
se justificarão — ou não.

**Quando:** próxima iteração activa do projecto.

---

## core-metrics (futuro)

### [P3] Criar core-metrics para Balanced Scorecard

**O quê:** um crate `core-metrics` que consome `ConformanceSummary` do
`core-audit` e produz métricas estruturadas para dashboards institucionais
(taxa de conformidade por categoria, evolução temporal, etc.).

**Porquê:** `ConformanceSummary.conformance_rate()` já produz o número base.
O próximo passo natural é agregar por categoria COSO, por período, e alimentar
um Balanced Scorecard de controlo interno.

**Quando:** depois de `workspace-governance` estar a produzir execuções reais,
para ter dados com que trabalhar.

---

## Concluído (referência)

| Item | Versão | Notas |
|---|---|---|
| `AuditOutcome` + `control_id` em `AuditEvent` | devel | COSO alignment |
| Control Registry (50 controlos base) | devel | `ControlDefinition`, `ControlExecution`, `builtin_control_catalog()` |
| Documentação enterprise-grade `core-audit` | devel | `//!`, `README.md`, `MAN.md` actualizados |
