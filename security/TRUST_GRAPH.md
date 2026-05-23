# Trust Graph — normordis-kernel

Estado: Draft v0.1.0.

## Objetivo

Descrever, de forma simples e validável, as relações de confiança entre fonte,
pipeline, dependências, artefactos, manifests, auditoria e release do
`normordis-kernel`.

## Âmbito

O grafo cobre o caminho mínimo de confiança: código fonte, GitHub Actions,
política de licenças, auditoria de dependências, build, manifesto de
integridade, relatório de release e promoção a `main`.

## Regras mínimas

- Cada nó tem identificador estável e tipo declarado.
- Cada aresta declara origem, destino e relação semântica.
- O grafo em JSON (`TRUST_GRAPH.json`) é validável pelo schema em
  `schemas/trust-graph.schema.json`.
- Alterações estruturais ao grafo devem ser documentadas no PR.

## Grafo de Confiança

```
source.git
  │  triggers
  ▼
github.actions ◄── cargo.lock.ci (constrains)
  │                deny.toml (governs)
  │  produces
  ▼
build.artifact
  │
  ├──── is-audited-by ──► cargo.audit.report
  │                           │ supports-release
  ├──── is-checked-by ──► cargo.deny.report
  │                           │ supports-release
  ├──── is-hashed-by ──► manifest.sha256
  │                           │ supports-release
  └──► release.report (agrega todos)
           │ gates-promotion
           ▼
       release.artifact (tag em main)
```

## Evidência esperada

- `security/TRUST_GRAPH.json` — grafo estruturado em JSON
- `security/schemas/trust-graph.schema.json` — schema de validação
- `artifacts/trust/audit-report.json` — saída do `cargo audit`
- `artifacts/trust/license-report.txt` — saída do `cargo deny`
- `artifacts/trust/MANIFEST.sha256` — manifesto de integridade
- `artifacts/trust/release-report.json` — relatório final do `release-gate.ps1`

## Relação com NORMORDIS

O trust graph permite auditar como um artefacto do kernel deriva de fonte
versionada, passou por verificação automática e satisfaz a política de
segurança antes de ser consumido por aplicações institucionais.
