---
title: "Normordis Kernel — Conformidade Data Governance Act (DGA)"
type: compliance
framework: DGA
status: draft
version: 0.1.0
date: 2026-06-03
lang: pt
audience: [executive, technical, auditor]
approved_by: ""
related:
  - docs/pt/compliance/overview.md
  - docs/pt/compliance/rgpd.md
  - docs/pt/compliance/nis2.md
  - docs/pt/compliance/interoperabilidade.md
  - docs/pt/legal/requirements.md
---

# Normordis Kernel — Conformidade Data Governance Act (DGA)

## Sumário Executivo

O Regulamento (UE) 2022/868 (Data Governance Act — DGA) estabelece um quadro europeu para a partilha e reutilização de dados, com três pilares: reutilização de dados protegidos do sector público, serviços de intermediação de dados, e altruísmo de dados.

Para o Normordis Kernel, o DGA é relevante principalmente no primeiro pilar: o kernel suporta aplicações da Administração Pública que gerem e disponibilizam dados públicos — e deve garantir que a sua arquitectura não cria obstáculos à reutilização legítima de dados, e que exporta dados em formatos abertos e interoperáveis.

O kernel não é um serviço de intermediação de dados nem uma organização de altruísmo de dados — esses papéis são das aplicações consumidoras ou de entidades externas.

## Âmbito

Cobre os requisitos do DGA aplicáveis ao Normordis Kernel como plataforma de suporte a aplicações da AP versão `0.1.x`. Não cobre:

- Serviços de intermediação de dados (Arts. 10.º–15.º) — papel das aplicações integradoras
- Organizações de altruísmo de dados (Arts. 16.º–25.º) — entidades externas ao kernel
- Espaços de dados europeus (Art. 26.º–31.º) — aplicável a nível de sector/domínio

## Referências Normativas

| Ref. | Diploma / Norma | Artigo / Secção | Obrigação |
|------|----------------|-----------------|-----------|
| [DGA] | Regulamento (UE) 2022/868, de 30 de maio de 2022 | Art. 3.º–9.º, 10.º | Condições de reutilização de dados protegidos do sector público; neutralidade |
| [DATA-ACT] | Regulamento (UE) 2023/2854 | Art. 23.º–31.º | Interoperabilidade de dados — complementar ao DGA |
| [LADA] | Lei n.º 26/2016, de 22 de agosto | Art. 15.º | Reutilização de documentos administrativos — enquadramento nacional |
| [RGPD] | Regulamento (UE) 2016/679 | Art. 5.º | Tratamento de dados pessoais no contexto de reutilização |

---

## Mapeamento por Artigo DGA

### Arts. 3.º–5.º — Condições de Reutilização de Dados Protegidos

O DGA permite a reutilização de dados do sector público que estejam protegidos por direitos (segredos comerciais, direitos de propriedade intelectual, dados pessoais) sob condições específicas. O kernel deve garantir que a gestão e exportação de dados respeita estas condições.

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Exportação controlada e auditada | `core-exports` | Snapshots auditáveis com registo de quem exportou e quando | Implementado |
| Audit trail de acesso a dados | `core-audit` | `AuditEvent` por cada operação de acesso/exportação | Implementado |
| Classificação de dados por tipo | `domain-registry` | Registo de domínios com metadados de classificação | Implementado (base) |
| Controlo de acesso a dados protegidos | `core-security` | Políticas de acesso por recurso/tipo | Implementado (base) |
| Formatos abertos e interoperáveis | `core-exports` | Exportação em formatos abertos (JSON, CSV) | Implementado |
| Marcação de dados pessoais vs. não-pessoais | `core-validation` + `core-rh` | Separação ActorId / dados pessoais | Implementado |

### Art. 10.º — Serviços de Intermediação de Dados — Requisitos de Neutralidade

O Art. 10.º exige que os serviços de intermediação sejam tecnologicamente neutros — não criem dependência de fornecedor, não limitem a portabilidade, não usem os dados para fins próprios.

O kernel enquadra-se aqui como plataforma de suporte a serviços de intermediação:

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| API pública sem vendor lock-in | `normordis-kernel` (facade) | Contratos tipados em Rust; sem dependência proprietária | Implementado |
| Portabilidade de dados | `core-exports` | Exportação estruturada em formatos abertos | Implementado |
| Sem uso de dados para fins do kernel | Arquitectura | Kernel não transmite dados para terceiros; execução local | Implementado |
| Interoperabilidade técnica | `interoperability` | Contratos entre runtimes sem acoplamento | Implementado |

### Arts. 23.º–31.º — Espaços de Dados Europeus

Os espaços de dados europeus (saúde, mobilidade, energia, agricultura, indústria, administração pública, etc.) impõem requisitos de interoperabilidade semântica e técnica.

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Classificação funcional interoperável | `domain-mef` | MEF nacional — taxonomia compatível com classificações europeias | Implementado |
| Metadados de dados abertos (DCAT) | — | Suporte a catálogo DCAT para datasets exportados | Planeado |
| Espaço de dados da AP (AGORA/SEMIC) | — | Alinhamento com vocabulários SEMIC | Planeado |

---

## Matriz de Conformidade

| Requisito DGA | Crate / Módulo | Evidência | Status |
|--------------|---------------|-----------|--------|
| Exportação controlada e auditada (Art. 4.º) | `core-exports`, `core-audit` | Snapshots auditáveis com AuditEvent | Implementado |
| Formatos abertos e interoperáveis (Art. 5.º) | `core-exports` | JSON, CSV — sem formatos proprietários | Implementado |
| Neutralidade tecnológica (Art. 10.º) | `normordis-kernel` (facade) | API tipada sem vendor lock-in | Implementado |
| Portabilidade de dados (Art. 10.º) | `core-exports` | Exportação estruturada | Implementado |
| Sem uso de dados para fins próprios (Art. 10.º) | Arquitectura local | Execução local; sem transmissão para terceiros | Implementado |
| Controlo de acesso a dados protegidos (Art. 3.º) | `core-security` | Políticas de acesso por recurso | Implementado (base) |
| Classificação de dados (Art. 3.º–5.º) | `domain-registry` | Metadados de classificação | Implementado (base) |
| Catálogo DCAT para datasets (Art. 8.º) | — | Exportação de metadados DCAT | Planeado |
| Alinhamento SEMIC / espaços de dados (Arts. 23.º–31.º) | — | Vocabulários semânticos europeus | Planeado |

## Limitações e Exclusões Conhecidas

- **Intermediação de dados**: o kernel não implementa serviços de intermediação de dados (Arts. 10.º–15.º). Esta funcionalidade, quando necessária, é responsabilidade da aplicação consumidora.
- **Altruísmo de dados**: fora do âmbito do kernel.
- **DCAT**: o kernel exporta dados em formatos abertos mas não gera automaticamente metadados DCAT para catalogação; está planeado.
- **Dados pessoais**: a reutilização de dados que contenham dados pessoais está sujeita ao RGPD mesmo sob o DGA. Ver [docs/pt/compliance/rgpd.md](rgpd.md).

## Glossário

| Termo | Definição |
|-------|-----------|
| **DCAT** | Data Catalog Vocabulary (W3C) — vocabulário para catalogação de datasets interoperáveis |
| **DGA** | Data Governance Act — Regulamento (UE) 2022/868; quadro europeu de partilha e reutilização de dados |
| **Espaço de dados** | Infra-estrutura sectorial para partilha de dados entre organizações com regras comuns (saúde, mobilidade, AP, etc.) |
| **Intermediação de dados** | Serviço que facilita a partilha de dados entre titulares e utilizadores; sujeito ao Art. 10.º DGA |
| **SEMIC** | Semantic Interoperability Community — comunidade europeia que publica vocabulários e ontologias para serviços públicos |

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial — Arts. 3.º–10.º DGA, exportação auditada, neutralidade tecnológica |
