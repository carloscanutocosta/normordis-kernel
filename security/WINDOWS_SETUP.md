# Windows Setup — normordis-kernel

Estado: Draft v0.1.0.

## Objetivo

Definir a forma recomendada de executar a Trust Baseline v0.1 em
Windows 11 com PowerShell 7, sem assumir Linux, WSL ou ferramentas GNU.

## Âmbito

Aplica-se à execução local dos scripts de segurança (`scripts/security/`),
ao build release (`scripts/build-release.ps1`) e à geração de evidência
para revisão.

## Regras mínimas

- PowerShell 7 como shell principal para todos os scripts de segurança.
- Não assumir WSL para validações locais.
- Não depender de utilitários GNU — usar `Get-FileHash` para SHA-256.
- Scripts Bash (`.sh`) mantidos como compatibilidade para CI Linux.
- Caminhos relativos sempre à raiz do repositório.

---

## Requisitos mínimos

| Componente | Versão mínima | Instalação |
|-----------|--------------|------------|
| Windows | 11 | — |
| PowerShell | 7.4+ | [aka.ms/powershell](https://aka.ms/powershell) |
| Git for Windows | 2.44+ | [git-scm.com](https://git-scm.com/) |
| Rust (rustup) | stable | [rustup.rs](https://rustup.rs) |
| cargo-audit | 0.21+ | `cargo install cargo-audit --locked` |
| cargo-deny | 0.16+ | `cargo install cargo-deny --locked` |
| 7-Zip | 23+ | [7-zip.org](https://www.7-zip.org/) (opcional, para backup) |

---

## Execution Policy

Recomendação para desenvolvimento local:

```powershell
Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned
```

Em ambientes com política restrita, execução pontual:

```powershell
pwsh -NonInteractive -ExecutionPolicy Bypass -File scripts/security/release-gate.ps1
```

---

## Execução local dos scripts de segurança

### Release gate completo (recomendado antes de qualquer tag)

```powershell
# A partir da raiz do repositório
.\scripts\security\release-gate.ps1
```

### Passos individuais

```powershell
# 1. Gerar manifesto de integridade
.\scripts\security\generate-manifest.ps1

# 2. Verificar manifesto existente
.\scripts\security\verify-manifest.ps1

# 3. Auditar dependências (RustSec)
.\scripts\security\audit-deps.ps1 -UpdateDb

# 4. Verificar conformidade de licenças
.\scripts\security\check-licenses.ps1
```

### Build release completo

```powershell
.\scripts\build-release.ps1
.\scripts\build-release.ps1 -SkipTests   # iteração rápida
.\scripts\build-release.ps1 -WithDocs    # gerar documentação HTML
```

### Backup do repositório

```powershell
.\scripts\backup\full-repo-backup.ps1
.\scripts\backup\full-repo-restore.ps1 -List
```

---

## Integração VSCode

O terminal recomendado é PowerShell 7. Definir no `.vscode/settings.json`
(não versionado):

```json
{
  "terminal.integrated.defaultProfile.windows": "PowerShell"
}
```

Executar sempre a partir da raiz do repositório para garantir caminhos
relativos estáveis no `MANIFEST.sha256`.

---

## Integração GitHub Actions

O CI em `.github/workflows/ci.yml` usa runners Windows e Linux em paralelo,
validando o agnóstico de plataforma do kernel:

- `check-windows` / `test-windows` — runner `windows-latest`
- `check-linux` / `test-linux` — runner `ubuntu-latest`
- `RUSTFLAGS="-D warnings"` activo em todos os jobs

---

## Evidência esperada após execução local

```
artifacts/trust/
  MANIFEST.sha256       ← integridade do source
  MANIFEST.json         ← idem, formato estruturado
  audit-report.json     ← cargo audit (RustSec)
  license-report.txt    ← cargo deny check licenses
  release-report.json   ← sumário do release-gate
```
