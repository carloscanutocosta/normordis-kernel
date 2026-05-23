# core-org

Domínio de estrutura orgânica institucional do Mini-Kernel RS.

## Responsabilidade

- Hierarquia de unidades orgânicas com validade temporal e instrumentos jurídicos.
- Cargos e posições orgânicas — abstractos, independentes de quem os ocupa.
- Competências: autoridade jurídica para praticar actos administrativos, com validade temporal.
- Delegações de competência entre posições, com instrumento jurídico próprio.
- Instrumentos jurídicos (Portaria, Despacho, Deliberação, RegulamentoOrgânico, Outro) como base legal de todas as alterações estruturais.
- Ports de persistência (hexagonal): `OrgUnitRepository`, `OrgPositionRepository`, `LegalInstrumentRepository`, `CompetencyRepository`, `DelegationRepository`.

## Não responsabilidade

- Não conhece SQLite, filesystem, Tauri ou UI.
- Não contém lógica de autorização (quem pode fazer o quê) — responsabilidade do service layer.
- Não gera identificadores — os IDs são gerados externamente e passados ao domínio.
- Não sabe quem ocupa uma posição — a ocupação é responsabilidade de `core-rh`.
- Não implementa queries complexas — as listagens com filtros pertencem ao adapter.

## Exemplo mínimo

```rust
use core_org::{OrgLevel, OrgUnit, OrgUnitId, OrgUnitStatus};
use chrono::NaiveDate;

let unit = OrgUnit {
    id: OrgUnitId::new("unit-sf-beja")?,
    short_name: "SF Beja".into(),
    full_name: "Serviço de Finanças de Beja".into(),
    service_code: Some("0312".into()),
    level: OrgLevel::new(3)?,
    parent_id: Some(OrgUnitId::new("unit-df-beja")?),
    contacts: Default::default(),
    created_by: None,
    legal_reference: Some("Portaria n.º 150/2024".into()),
    valid_from: NaiveDate::from_ymd_opt(2024, 5, 1).unwrap(),
    valid_until: None,
    status: OrgUnitStatus::Active,
};

unit.validate()?; // valida invariantes de domínio
```
