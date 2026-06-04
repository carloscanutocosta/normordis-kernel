# Política de Segurança — normordis-kernel

> Política de CVD conforme **CRA (UE 2024/2847) art. 13–14**, **EN ISO/IEC 30111:2019** e **EN ISO/IEC 29147:2018**.

## Versões Suportadas

| Versão  | Suporte de segurança |
|---------|----------------------|
| `0.x` (branch `main`) | ✔ Activo |
| Versões anteriores | Não recebe correcções |

As correcções de segurança são aplicadas exclusivamente na versão activa. Versões anteriores não recebem backports.

---

## Reportar uma Vulnerabilidade

**Não abrir issues públicas para vulnerabilidades de segurança.**

### Canal preferencial — GitHub Private Security Advisory

**[▶ Reportar uma vulnerabilidade](https://github.com/carloscanutocosta/normordis-kernel/security/advisories/new)**

Este canal garante confidencialidade total até à divulgação coordenada (CVD).

### Canal alternativo — Email

**carloscanutocosta@gmail.com**  
Assunto: `[SECURITY] normordis-kernel — <descrição breve>`

Incluir no relatório:

- Descrição clara do problema e do impacto potencial
- Passos para reproduzir (ou prova de conceito mínima)
- Versão afectada (output de `cargo metadata --no-deps`)
- Sugestão de correcção, se existir

---

## SLAs de Resposta (alinhados com CRA art. 14)

| Severidade | Acuse de recepção | Avaliação inicial | Resolução / Mitigação |
|---|---|---|---|
| **Crítica** (CVSS ≥ 9.0) | 24 horas | 72 horas | 7 dias |
| **Alta** (CVSS 7.0–8.9) | 48 horas | 5 dias úteis | 30 dias |
| **Média** (CVSS 4.0–6.9) | 5 dias úteis | 10 dias úteis | 90 dias |
| **Baixa** (CVSS < 4.0) | 10 dias úteis | — | Próxima versão planeada |

---

## Processo de Divulgação Coordenada (CVD)

1. **Recepção** — acusamos recepção no prazo acima indicado.
2. **Triagem** — avaliamos impacto, reproduzimos e atribuímos CVSS.
3. **Correcção** — desenvolvemos e testamos a correcção em branch privada.
4. **Embargo** — período de embargo de até 90 dias (negociável conforme severidade).
5. **Divulgação** — publicamos GitHub Security Advisory com CVE, versão corrigida e crédito.
6. **Notificação ENISA / CERT.PT** — para vulnerabilidades activamente exploradas, notificamos a ENISA e o CERT.PT em ≤ 24 horas após confirmação (CRA art. 14 n.º 1).

Referências normativas: EN ISO/IEC 30111:2019, EN ISO/IEC 29147:2018, CRA (UE 2024/2847) art. 13–14, NIS2 (UE 2022/2555) / DL n.º 20/2025.

---

## Âmbito CVD

### Em âmbito

- Código Rust no workspace `normordis-kernel` (todos os crates em `crates/`)
- Configurações de CI/CD com impacto de segurança
- Dependências directas listadas em `Cargo.toml`

### Fora de âmbito

- Vulnerabilidades em dependências transitivas sem impacto demonstrável (reportar ao maintainer original)
- Engenharia social / phishing
- Ataques físicos

---

## Advisories com Isenção (VEX)

Os advisories RustSec actualmente isentos estão documentados com análise de exploitabilidade em [`security/VEX.cdx.json`](security/VEX.cdx.json) (CycloneDX VEX), conforme CRA art. 13 n.º 6.

As vulnerabilidades confirmadas são corrigidas antes de qualquer divulgação pública. O crédito ao investigador é dado nas notas de release, salvo pedido de anonimato.

---

## Modelo de Confiança em Release

Cada release do `normordis-kernel` passa por um portão de segurança de 4 passos antes de ser promovida a `main`. O orquestrador é `scripts/security/release-gate.ps1`.

### 1. Integridade do Código-Fonte

`scripts/security/generate-manifest.ps1` / `generate-manifest.sh`

Gera `MANIFEST.sha256` e `MANIFEST.json` com hashes SHA-256 de todos os ficheiros fonte, excluindo artefactos reconstruíveis. O manifesto é gravado em `artifacts/trust/` e pode ser verificado de forma independente:

```sh
# Linux / macOS
sha256sum -c artifacts/trust/MANIFEST.sha256

# PowerShell
.\scripts\security\verify-manifest.ps1
```

### 2. Auditoria de Dependências

`scripts/security/audit-deps.ps1` / `audit-deps.sh`

Todas as dependências Rust são verificadas contra a [RustSec Advisory Database](https://rustsec.org/). A release é bloqueada se existir qualquer vulnerabilidade com CVE ou RUSTSEC atribuído.

```sh
cargo audit
```

O relatório JSON é gravado em `artifacts/trust/audit-report.json`.

### 3. Conformidade de Licenças

`scripts/security/check-licenses.ps1` / `check-licenses.sh`

A política de licenças está definida em `deny.toml` na raiz do repositório. São **permitidas** licenças compatíveis com EUPL-1.2 (MIT, Apache-2.0, BSD-*, ISC, MPL-2.0). São **proibidas** GPL-2.0, GPL-3.0, AGPL-3.0 e LGPL-*.

```sh
cargo deny check licenses
```

### 4. Build Determinístico

`scripts/build-release.ps1`

A release é compilada com `cargo build --release --workspace` após `cargo fmt --check` e `cargo clippy -- -D warnings`, sem flags não documentadas ou patches não divulgados.

### Relatório Final

O `release-gate.ps1` produz `artifacts/trust/release-report.json` com o git SHA do commit, timestamps, versão do compilador e resultado de cada passo. Este ficheiro acompanha qualquer distribuição de artefactos.

---

## Integração Contínua

O CI em `.github/workflows/ci.yml` valida automaticamente em cada push/PR:

| Job | Plataforma | O que verifica |
|-----|-----------|----------------|
| `fmt` | Linux | Formatação (`cargo fmt --check`) |
| `check-windows` | Windows | Compilação + Clippy (`-D warnings`) |
| `check-linux` | Linux | Compilação + Clippy (`-D warnings`) — valida agnóstico de plataforma |
| `test-windows` | Windows | Suite de testes completa |
| `test-linux` | Linux | Suite de testes completa |

`RUSTFLAGS="-D warnings"` está activo em todos os jobs — qualquer warning é tratado como erro de compilação.

---

## Ferramentas de Segurança

| Ferramenta    | Versão mínima | Instalação                           |
|---------------|---------------|--------------------------------------|
| `cargo-audit` | 0.21          | `cargo install cargo-audit --locked` |
| `cargo-deny`  | 0.16          | `cargo install cargo-deny --locked`  |

Os scripts de PowerShell instalam estas ferramentas automaticamente se não estiverem presentes.

---

## Dependências com Código Criptográfico

O `normordis-kernel` inclui as seguintes crates com implementações criptográficas:

| Crate              | Algoritmos           | Auditada |
|--------------------|----------------------|----------|
| `ed25519-dalek`    | Ed25519              | Sim      |
| `p256`, `p384`     | ECDSA P-256 / P-384  | Sim      |
| `rsa`              | RSA-OAEP / PSS       | Sim      |
| `chacha20poly1305` | ChaCha20-Poly1305    | Sim      |
| `argon2`           | Argon2id             | Sim      |
| `sha2`             | SHA-256 / SHA-512    | Sim      |

Estas crates são mantidas pela comunidade RustCrypto e sujeitas a revisão periódica pelo projecto. Qualquer substituição ou adição de crates criptográficas requer aprovação explícita no PR.

---

## Licença

Este projecto está licenciado sob a [EUPL-1.2](LICENSE). A EUPL-1.2 é compatível com GPL-2.0, GPL-3.0, AGPL-3.0, LGPL, MPL-2.0 e outras licenças aprovadas pela OSI conforme o Apêndice da licença.
