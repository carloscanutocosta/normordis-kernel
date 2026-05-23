# Documentação — normordis-kernel

Documentação arquitetural, decisões de design e políticas de governança do `normordis-kernel`.

## Estrutura

```
docs/
├── adr/          — Architecture Decision Records
├── architecture/ — Visão geral e mapa de crates
├── governance/   — Políticas transversais (supply chain, confiança)
└── pdf/          — Pipeline e formatos documentais PDF/Typst
```

## Architecture Decision Records (ADR)

| ADR | Título | Estado |
|-----|--------|--------|
| [ADR-NK-001](adr/ADR-NK-001-kernel-architecture.md) | Formalização do normordis-kernel como camada autónoma | Aceite |
| [ADR-NK-002](adr/ADR-NK-002-crate-layers.md) | Estrutura de crates `core/`, `support/` e `infra/` | Aceite |
| [ADR-NK-003](adr/ADR-NK-003-support-errors.md) | Erros canónicos e `support-errors` | Aceite |
| [ADR-NK-004](adr/ADR-NK-004-encrypted-storage.md) | Escritas cifradas em storage SQLite | Aceite |
| [ADR-NK-005](adr/ADR-NK-005-document-numbering.md) | Numeração alocada apenas na finalização do documento | Aceite |
| [ADR-NK-006](adr/ADR-NK-006-trust-baseline.md) | Trust Baseline v0.1 — confiança verificável contínua | Aceite |

## Arquitectura

- [Visão geral](architecture/overview.md) — definição, camadas lógicas e princípios
- [Mapa de crates](architecture/crate-map.md) — inventário e classificação de todos os crates

## Governança

- [Supply Chain Trust](governance/SUPPLY_CHAIN_TRUST.md) — SBOM, provenance e confiança contínua

## PDF & Formatos Documentais

- [Pipeline PDF](pdf/README.md) — ADRs e modelos de execução do pipeline Typst
- [NCRTF](pdf/NCRTF.md) — Canonical Rich Text Format
- [NDF](pdf/NDF.md) — NORMAXIS Document Format
- [NDT](pdf/NDT.md) — NORMAXIS Document Template

## Relação com outros documentos

| Ficheiro | Propósito |
|----------|-----------|
| [SECURITY.md](../SECURITY.md) | Política pública de segurança e disclosure |
| [security/](../security/) | Trust Baseline operacional e políticas formais |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | Guia de contribuição |
| [CHANGELOG.md](../CHANGELOG.md) | Histórico de versões |
