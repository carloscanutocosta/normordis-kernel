# Documentação — normordis-kernel

Documentação arquitetural, decisões de design e políticas de governança do `normordis-kernel`.

## Estrutura

```
docs/
├── _template.md  — Template canónico para documentos de compliance e legal
├── pt/           — Documentação em Português (língua primária)
│   ├── compliance/   — Como o kernel implementa cada framework normativo
│   └── legal/        — Requisitos legais e normativos aplicáveis
├── en/           — English translations (pendente)
│   ├── compliance/
│   └── legal/
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

## Conformidade (compliance by design)

- [Visão geral de conformidade](pt/compliance/overview.md) — estratégia global, matriz de conformidade, glossário
- [Requisitos legais e normativos](pt/legal/requirements.md) — catálogo completo: RGPD, NIS2, AI Act, CRA, SNC-AP, COSO, EIF, e outros
- [Registo de Actividades de Tratamento](pt/legal/registo-actividades-tratamento.md) — RAT RGPD Art. 30.º — AT-01 a AT-07, direitos dos titulares, modelo para integradores
- [Declaração de Conformidade](pt/legal/declaracao-conformidade.md) — declaração formal por framework, estado de conformidade, evidências técnicas
- [Política de Privacidade](pt/legal/politica-privacidade.md) — tratamento de dados pessoais, direitos dos titulares, modelo para integradores
- [Política de Segurança](pt/legal/politica-seguranca.md) — governação, SDLC, criptografia, gestão de vulnerabilidades e incidentes
- [Componentes de Terceiros](pt/legal/terceiros.md) — SBOM legível, licenças, advisories activos documentados
- [Controlo de Exportação](pt/legal/controlo-exportacao.md) — Regulamento 2021/821, isenções open source, sanções, obrigações do integrador
- [Conformidade COSO](pt/compliance/coso.md) — mapeamento detalhado dos 17 princípios COSO, crate a crate
- [Conformidade SNC-AP](pt/compliance/snc-ap.md) — mapeamento domain-mef, contabilidade orçamental/financeira/analítica, NCPs
- [Conformidade RGPD](pt/compliance/rgpd.md) — privacy by design, arquitectura ActorId/dados pessoais, resolução COSO↔RGPD
- [Conformidade NIS2 / Cibersegurança](pt/compliance/nis2.md) — Art. 21.º completo, supply chain security, criptografia XChaCha20/Argon2id
- [Conformidade AI Act](pt/compliance/ai-act.md) — posicionamento kernel/IA, Arts. 9.º–17.º, modelo de logging para IA de alto risco
- [Conformidade eIDAS 2](pt/compliance/eidas.md) — assinaturas qualificadas, CMD, EUDIW, fluxo PAdES, services/signing
- [Conformidade CRA + ISO 27001](pt/compliance/seguranca-informacao.md) — segurança por design, SBOM, Anexo A ISO 27001
- [Conformidade DGA](pt/compliance/dga.md) — reutilização de dados públicos, neutralidade, exportação auditada
- [Conformidade INTOSAI GOV 9100](pt/compliance/intosai.md) — accountability, transparência, Tribunal de Contas
- [Conformidade de Interoperabilidade](pt/compliance/interoperabilidade.md) — EIF 12 princípios, Interoperable Europe Act, iAP, crate interoperability
- [Conformidade Arquivística](pt/compliance/arquivistica.md) — visão document-centric AP, MEF-DGLAB, MoReq2010, ciclo de vida documental
- [Conformidade WCAG / A11Y](pt/compliance/wcag.md) — modelo kernel / normordis-core-ui, POUR mapping, PDF/A acessível

## Trabalho planeado

- [TODO.md](TODO.md) — itens de versão futura organizados por módulo

## Relação com outros documentos

| Ficheiro | Propósito |
|----------|-----------|
| [SECURITY.md](../SECURITY.md) | Política pública de segurança e disclosure |
| [security/](../security/) | Trust Baseline operacional e políticas formais |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | Guia de contribuição |
| [CHANGELOG.md](../CHANGELOG.md) | Histórico de versões |
