# Changelog — normordis-kernel

Todas as alterações relevantes são documentadas neste ficheiro.
Formato baseado em [Keep a Changelog](https://keepachangelog.com/pt/1.0.0/).
O projecto segue [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Adicionado
- Política de segurança e Trust Baseline v0.1 (`security/`)
- Scripts de portão de release (`scripts/security/release-gate.ps1`)
- Auditoria de dependências e conformidade de licenças em CI
- SBOM CycloneDX gerado em CI e em release
- Workflow de release automatizado com provenance attestation
- `deny.toml` — política formal de licenças compatíveis com EUPL-1.2

---

## [0.3.0] — 2026-05-23

### Adicionado
- Workspace inicial do `normordis-kernel` extraído do ecossistema `mini-apps-rusty`
- Crate de fachada `normordis-kernel` com API pública unificada
- Módulos: `rh`, `org`, `audit`, `documental`, `validation`, `security`, `config`, `metrics`, `exports`, `ingest`, `numerador`, `mef`, `runtime`, `errors`, `ids`, `clock`
- CI com validação em Windows e Linux (fmt, check, clippy, test)
- Scripts de build release (`scripts/build-release.ps1`)
- Scripts de backup e restore (`scripts/backup/`)
- Licença EUPL-1.2
- Suporte a `support-backup` e `infra-backup`
- Adaptador SQLite com modo misto (SQLCipher + retry)
- Domínios transversais: `domain-numerador`, `domain-mef`
- Runtime de mini-apps: `miniapp-runtime`, `interoperability`

---

[Unreleased]: https://github.com/carloscanutocosta/normordis-kernel/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/carloscanutocosta/normordis-kernel/releases/tag/v0.3.0
