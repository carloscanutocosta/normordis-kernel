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
