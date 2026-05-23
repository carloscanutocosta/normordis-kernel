# Guia de Contribuição — normordis-kernel

## Pré-requisitos

| Componente | Versão | Instalação |
|-----------|--------|-----------|
| Rust (rustup) | stable | [rustup.rs](https://rustup.rs) |
| PowerShell | 7.4+ | [aka.ms/powershell](https://aka.ms/powershell) |
| cargo-audit | 0.21+ | `cargo install cargo-audit --locked` |
| cargo-deny | 0.16+ | `cargo install cargo-deny --locked` |

Ver [security/WINDOWS_SETUP.md](security/WINDOWS_SETUP.md) para configuração detalhada em Windows 11.

---

## Estratégia de branches

| Branch | Propósito |
|--------|-----------|
| `main` | Código estável, protegido — apenas via PR aprovado com CI verde |
| `devel` | Branch de trabalho — base para novas funcionalidades e correcções |
| `feature/*` | Funcionalidades isoladas, baseadas em `devel` |
| `fix/*` | Correcções de bugs, baseadas em `devel` |

O fluxo é sempre: `feature/* → devel → main`.

---

## Processo de contribuição

### 1. Preparar o ambiente

```powershell
git clone https://github.com/carloscanutocosta/normordis-kernel
cd normordis-kernel
cargo check --workspace
cargo test --workspace
```

### 2. Criar branch de trabalho

```powershell
git checkout devel
git pull origin devel
git checkout -b feature/nome-da-funcionalidade
```

### 3. Desenvolver

- Manter a arquitectura hexagonal: `core/` sem dependências de `infra/`
- Sem código `unsafe` sem aprovação explícita em `security/ALLOWLISTS.md`
- Sem dependências criptográficas fora do inventário em `security/CRYPTOGRAPHY.md`
- Sem dependências com licença proibida (ver `deny.toml`)

### 4. Verificar antes do PR

```powershell
# Formatação + clippy + testes
.\scripts\build-release.ps1 -SkipBuild

# Portão de segurança (audit + licenças + manifesto)
.\scripts\security\release-gate.ps1 -SkipBuild
```

O CI rejeita PRs com warnings de compilação (`-D warnings`), falhas de teste, vulnerabilidades RustSec ou violações de licença.

### 5. Abrir PR

- Base: `devel`
- Título conciso (< 70 caracteres)
- Descrever o quê e o porquê (não o como — o código mostra o como)
- Referenciar issues relacionadas

---

## Convenções de código

### Rust

- Edição 2021, `rustfmt` com configuração padrão
- `clippy --all-targets -- -D warnings` sem supressões não documentadas
- Sem `unwrap()` / `expect()` em código de produção — usar `?` ou tipos de erro explícitos
- Erros definidos com `thiserror`; sem `anyhow` no kernel

### Commits

Formato livre mas descritivo. Incluir o contexto do porquê quando não é óbvio.
Não incluir `Co-Authored-By` gerado por ferramentas.

### Dependências

Toda a nova dependência directa requer no PR:
1. Justificação da necessidade
2. Verificação de licença (compatível com EUPL-1.2)
3. Estado de manutenção

Ver [security/DEPENDENCY_POLICY.md](security/DEPENDENCY_POLICY.md).

---

## Alterações a ficheiros de segurança

Qualquer alteração a `security/`, `deny.toml` ou `SECURITY.md` requer
descrição explícita no PR do impacto na política de confiança.

Alterações a `security/THREAT_MODEL.md` ou `security/CRYPTOGRAPHY.md`
requerem discussão antes de merge.

---

## Questões e suporte

Abrir uma issue no GitHub com o máximo de contexto possível.
Para vulnerabilidades de segurança, ver [SECURITY.md](SECURITY.md) — não usar issues públicas.
