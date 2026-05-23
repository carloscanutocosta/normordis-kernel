# Política de Provenance — normordis-kernel

Estado: Draft v0.1.0.

## Objetivo

Definir como a origem e o processo de produção dos artefactos do
`normordis-kernel` devem ser registados e verificáveis.

## Âmbito

Aplica-se a artefactos de build, release, manifests e outputs produzidos
pelo CI nos repositórios do ecossistema NORMORDIS.

## Regras mínimas

- Builds executados exclusivamente a partir de fonte Git versionada.
- Artefactos publicados têm hash SHA-256 verificável (`MANIFEST.sha256`).
- O git SHA do commit consta obrigatoriamente no `release-report.json`.
- A versão do compilador Rust (`rustc --version`) consta no relatório de release.
- Quando disponível, GitHub Artifact Attestations deve ser usado.
- A ausência de atestação formal não bloqueia builds no MVP, mas deve ficar
  visível no sumário do workflow.
- Artefactos não dependem de secrets externos para a baseline inicial.

## Cadeia de provenance

```
commit SHA (git)
    │
    ▼
GitHub Actions runner (versão fixada)
    │  rustc + cargo (versão estável fixada)
    ▼
build.artifact
    │
    ├─► MANIFEST.sha256   (integridade do source)
    ├─► audit-report.json (estado de segurança das deps)
    ├─► license-report.txt (conformidade de licenças)
    └─► release-report.json (agrega git SHA, rustc, timestamps)
```

## Informação mínima por release

O `release-report.json` gerado pelo `release-gate.ps1` deve conter:

| Campo | Descrição |
|-------|-----------|
| `git.commit` | SHA completo do commit |
| `git.branch` | Branch de origem (`main`) |
| `environment.rust` | Versão do `rustc` |
| `environment.cargo` | Versão do `cargo` |
| `generated_at` | Timestamp ISO 8601 UTC |
| `steps[*].status` | Resultado de cada passo do portão |

## Evidência esperada

- `artifacts/trust/MANIFEST.sha256`
- `artifacts/trust/MANIFEST.json`
- `artifacts/trust/release-report.json`
- Atestação de provenance GitHub quando o ambiente a suportar

## Relação com NORMORDIS

Provenance permite ligar um artefacto do kernel ao commit exacto, ao pipeline
e ao contexto que o produziram — essencial para auditar a cadeia de confiança
de documentos e registos institucionais que dependem do kernel.
