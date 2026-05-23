# Política de Segurança em Release — normordis-kernel

Este diretório contém os scripts que implementam a **política de segurança** aplicada antes de qualquer release do `normordis-kernel`. O objetivo é garantir integridade do código-fonte, ausência de vulnerabilidades conhecidas nas dependências e conformidade de licenças com a EUPL-1.2.

---

## Visão Geral do Pipeline

```
source code
    │
    ▼
[1] generate-manifest   →  MANIFEST.sha256 + MANIFEST.json (integridade SHA-256)
    │
    ▼
[2] audit-deps          →  audit-report.json        (RustSec — CVEs e RUSTSEC)
    │
    ▼
[3] check-licenses      →  license-report.txt        (cargo deny — EUPL-1.2)
    │
    ▼
[4] cargo build --release                            (artefactos compilados)
    │
    ▼
[5] release-report.json                              (sumário final com git SHA)
```

Todos os artefactos são gravados em `artifacts/trust/` (configurável via `-OutputDir` ou `TRUST_OUT_DIR`).

---

## `release-gate.ps1` — Orquestrador Master

Executa o pipeline completo num único comando. **Uso recomendado antes de qualquer tag de release.**

```powershell
# Pipeline completo (requer internet para audit)
.\release-gate.ps1

# Sem build (apenas verificações de source + deps)
.\release-gate.ps1 -SkipBuild

# Sem internet (CI offline ou air-gapped)
.\release-gate.ps1 -SkipAudit

# Actualizar a base de dados RustSec antes de auditar
.\release-gate.ps1 -UpdateAdvisoryDb

# Não falhar em crates sem manutenção (apenas vulnerabilidades bloqueiam)
.\release-gate.ps1 -AllowUnmaintained

# Pasta de saída personalizada
.\release-gate.ps1 -OutputDir "D:\release\trust"
```

### Parâmetros

| Parâmetro            | Predefinição        | Descrição |
|----------------------|---------------------|-----------|
| `-OutputDir`         | `artifacts/trust`   | Pasta de saída para relatórios |
| `-SkipAudit`         | `false`             | Ignora auditoria de deps (sem internet) |
| `-SkipLicenses`      | `false`             | Ignora verificação de licenças |
| `-SkipBuild`         | `false`             | Ignora `cargo build --release` |
| `-UpdateAdvisoryDb`  | `false`             | Actualiza a advisory-db RustSec antes de auditar |
| `-AllowUnmaintained` | `false`             | Não falha por crates sem manutenção |

### Exit codes

| Código | Significado |
|--------|-------------|
| `0`    | Todos os passos passaram — release aprovada |
| `1`    | Um ou mais passos falharam — release bloqueada |

---

## Scripts Individuais

### `generate-manifest.ps1` / `generate-manifest.sh`

Gera `MANIFEST.sha256` e `MANIFEST.json` com hashes SHA-256 de todos os ficheiros fonte, excluindo artefactos reconstruíveis (`target/`, `.logs/`, `artifacts/`).

```powershell
# Saída padrão: artifacts/trust/
.\generate-manifest.ps1

# Pasta personalizada
.\generate-manifest.ps1 -OutputDir "D:\release\trust"

# Via variável de ambiente
$env:TRUST_OUT_DIR = "D:\release\trust"; .\generate-manifest.ps1
```

```sh
# Linux/macOS
./generate-manifest.sh
./generate-manifest.sh /tmp/release/trust
TRUST_OUT_DIR=/tmp/release/trust ./generate-manifest.sh
```

---

### `verify-manifest.ps1` / `verify-manifest.sh`

Verifica o `MANIFEST.sha256` existente. Útil para validar integridade após transferência ou antes de assinar.

```powershell
.\verify-manifest.ps1
.\verify-manifest.ps1 -VerboseOk    # imprime "OK" por ficheiro
.\verify-manifest.ps1 -ManifestPath "D:\release\trust\MANIFEST.sha256"
```

```sh
./verify-manifest.sh
./verify-manifest.sh /tmp/release/trust/MANIFEST.sha256
```

**Exit codes:** `0` = tudo OK · `1` = falha de hash · `2` = manifesto não encontrado

---

### `audit-deps.ps1` / `audit-deps.sh`

Audita as dependências Rust contra a [RustSec Advisory Database](https://rustsec.org/). Instala `cargo-audit` automaticamente se necessário.

```powershell
.\audit-deps.ps1
.\audit-deps.ps1 -UpdateDb          # actualiza advisory-db antes
.\audit-deps.ps1 -AllowWarnings     # não falha apenas por "unmaintained"
```

```sh
./audit-deps.sh
UPDATE_DB=1 ./audit-deps.sh
ALLOW_WARNINGS=1 ./audit-deps.sh
```

Gera `audit-report.json` em JSON estruturado.

---

### `check-licenses.ps1` / `check-licenses.sh`

Verifica conformidade de licenças usando [`cargo deny`](https://embarkstudios.github.io/cargo-deny/), configurado em `deny.toml` na raiz do repositório.

```powershell
.\check-licenses.ps1
.\check-licenses.ps1 -CheckAll      # inclui advisories + bans
```

```sh
./check-licenses.sh
CHECK_ALL=1 ./check-licenses.sh
```

Gera `license-report.txt` com o output completo do `cargo deny`.

---

## `deny.toml` — Configuração de Licenças

O ficheiro `deny.toml` na raiz do repositório define a política de licenças. As licenças **permitidas** incluem MIT, Apache-2.0, BSD-*, ISC, MPL-2.0 e EUPL-1.2. As licenças **proibidas** incluem GPL-2.0, GPL-3.0, AGPL-3.0 e LGPL-*.

Para adicionar uma excepção a uma crate específica:

```toml
[[licenses.exceptions]]
name = "nome-da-crate"
version = "*"
allow = ["MIT"]
```

---

## Ferramentas Necessárias

| Ferramenta    | Instalação                          | Usado por |
|---------------|-------------------------------------|-----------|
| `cargo-audit` | `cargo install cargo-audit --locked` | `audit-deps` |
| `cargo-deny`  | `cargo install cargo-deny --locked`  | `check-licenses` |
| `sha256sum`   | Built-in (Linux) / via Git Bash      | `verify-manifest.sh` |
| `7-Zip`       | https://www.7-zip.org/              | (backup scripts) |

Os scripts de PowerShell instalam `cargo-audit` e `cargo-deny` automaticamente se não estiverem presentes.

---

## Artefactos Gerados (`artifacts/trust/`)

| Ficheiro               | Gerado por          | Conteúdo |
|------------------------|---------------------|----------|
| `MANIFEST.sha256`      | `generate-manifest` | Hashes SHA-256 (formato `sha256sum -c`) |
| `MANIFEST.json`        | `generate-manifest` | Hashes em JSON estruturado |
| `audit-report.json`    | `audit-deps`        | Relatório JSON do `cargo audit` |
| `license-report.txt`   | `check-licenses`    | Output do `cargo deny check licenses` |
| `release-report.json`  | `release-gate`      | Sumário completo com git SHA e timestamps |

---

## Integração com CI

Os scripts shell (`.sh`) são usados directamente no GitHub Actions:

```yaml
- name: Security gate
  run: |
    chmod +x scripts/security/*.sh
    scripts/security/audit-deps.sh
    scripts/security/check-licenses.sh
    scripts/security/generate-manifest.sh
```

O CI em `.github/workflows/ci.yml` já inclui `cargo deny check` como parte do pipeline de validação.
