# Trust Baseline v0.1 — normordis-kernel

Estado: Draft v0.1.0.

## Objetivo

Definir uma baseline inicial de confiança verificável para o `normordis-kernel`.
A confiança não é assumida; deve ser suportada por evidência reprodutível:
manifests SHA-256, auditoria de dependências, conformidade de licenças,
provenance e políticas formais.

## Âmbito

Esta baseline aplica-se ao repositório `normordis-kernel` e define os padrões
mínimos de segurança para o kernel Rust. Os repositórios consumidores
(miniapps, aplicações institucionais) devem adoptar uma baseline equivalente,
com os ajustamentos necessários ao seu ecossistema.

## Regras mínimas

- `Cargo.lock` não é versionado (biblioteca); regenerado deterministicamente em CI.
- `MANIFEST.sha256` e `MANIFEST.json` gerados em cada release.
- `cargo audit` sem vulnerabilidades antes de promover para `main`.
- `cargo deny check licenses` conforme `deny.toml` em cada release.
- Actions e ferramentas de CI pinadas por versão (sem `@latest`).
- Secrets não são necessários para o pipeline base.
- Decisões de segurança com impacto transversal documentadas aqui.

## Evidência esperada

- `artifacts/trust/MANIFEST.sha256`
- `artifacts/trust/MANIFEST.json`
- `artifacts/trust/audit-report.json`
- `artifacts/trust/license-report.txt`
- `artifacts/trust/release-report.json`

## Relação com NORMORDIS

O `normordis-kernel` é a plataforma base do ecossistema NORMORDIS. Esta
baseline estabelece o padrão mínimo de confiança que qualquer artefacto
produzido pelo kernel deve satisfazer antes de ser consumido por aplicações
institucionais.

## Documentos

| Ficheiro | Propósito |
|----------|-----------|
| [TRUST_GRAPH.md](TRUST_GRAPH.md) | Grafo de confiança: nós, arestas e relações |
| [THREAT_MODEL.md](THREAT_MODEL.md) | Activos, adversários e fronteiras de confiança |
| [CRYPTOGRAPHY.md](CRYPTOGRAPHY.md) | Decisões de design criptográfico e algoritmos |
| [DEPENDENCY_POLICY.md](DEPENDENCY_POLICY.md) | Regras para introdução e revisão de dependências Rust |
| [PROVENANCE_POLICY.md](PROVENANCE_POLICY.md) | Origem e rastreabilidade dos artefactos |
| [RUNTIME_INTEGRITY.md](RUNTIME_INTEGRITY.md) | Verificação de integridade em runtime |
| [ALLOWLISTS.md](ALLOWLISTS.md) | Allowlists e blocklists de dependências e licenças |
| [WINDOWS_SETUP.md](WINDOWS_SETUP.md) | Configuração recomendada em Windows 11 / PowerShell 7 |
| [ADVISORIES.md](ADVISORIES.md) | Histórico de advisories e CVEs aplicáveis |
| [audits/](audits/) | Relatórios de auditorias externas |

## Relação com outros ficheiros

```
SECURITY.md              ← política pública (disclosure, reporting)
deny.toml                ← política de licenças (cargo deny)
security/
  README.md              ← esta baseline
  TRUST_GRAPH.*          ← grafo estruturado de confiança
  THREAT_MODEL.md        ← o quê e quem se protege
  CRYPTOGRAPHY.md        ← como se protege (cripto)
  DEPENDENCY_POLICY.md   ← confiança nas dependências
  PROVENANCE_POLICY.md   ← rastreabilidade dos artefactos
  RUNTIME_INTEGRITY.md   ← integridade em execução
  ALLOWLISTS.md          ← listas explícitas de aprovação
  WINDOWS_SETUP.md       ← setup local Windows
  ADVISORIES.md          ← histórico de incidentes
  audits/                ← evidências de revisão externa
scripts/security/
  release-gate.ps1       ← portão operacional de release
  audit-deps.*           ← cargo audit automatizado
  check-licenses.*       ← cargo deny automatizado
  generate-manifest.*    ← geração de manifesto SHA-256
  verify-manifest.*      ← verificação de manifesto
```
