# Política de Dependências — normordis-kernel

Estado: Draft v0.1.0.

## Objetivo

Definir regras mínimas para introdução, revisão e evidência de dependências
Rust no `normordis-kernel`.

## Âmbito

Aplica-se a todas as dependências declaradas nos `Cargo.toml` do workspace,
incluindo dependências directas e transitivas.

## Regras mínimas

- Dependências pinadas por versão semântica explícita (sem `*` ou ranges abertos).
- `Cargo.lock` não é versionado (convenção de biblioteca), mas deve ser
  regenerado deterministicamente em CI a partir de versões fixas.
- Novas dependências requerem revisão quanto a: manutenção activa, origem,
  licença compatível com EUPL-1.2, superfície de execução e necessidade real.
- Crates com código `unsafe` explícito devem ser documentadas em
  `ALLOWLISTS.md` com justificação.
- Build scripts (`build.rs`) de dependências só são aprovados quando
  estritamente necessários (ex: `rusqlite` — bundled SQLite).
- Dependências abandonadas (sem release há mais de 2 anos ou sem repositório
  verificável) exigem justificação ou substituição.
- Dependências críticas (criptografia, storage, serialização) devem ter
  alternativa ou mitigação documentada.
- `cargo audit` sem vulnerabilidades é condição de release.
- `cargo deny check licenses` conforme `deny.toml` é condição de release.

## Categorias de dependências

### Dependências directas (Cargo.toml)

Toda a adição de dependência directa deve ser acompanhada, no PR, de:

1. Justificação da necessidade
2. Verificação de licença (compatível com EUPL-1.2)
3. Estado de manutenção (última release, issues abertas)
4. Presença ou ausência de código `unsafe`

### Dependências criptográficas

As crates com implementações criptográficas são sujeitas a critério adicional:

- Preferencialmente do ecossistema [RustCrypto](https://github.com/RustCrypto)
- Com auditoria conhecida ou processo de revisão documentado
- Sem implementações próprias de primitivos — sempre via crate estabelecida
- Listadas explicitamente em `CRYPTOGRAPHY.md`

### Dependências transitivas

As dependências transitivas são monitorizadas via:

- `cargo audit` — vulnerabilidades conhecidas (RustSec)
- `cargo deny` — licenças e crates duplicadas
- `cargo tree` — para inspecção manual quando necessário

## Evidência esperada

- `Cargo.toml` com versões explícitas
- `artifacts/trust/audit-report.json` em cada release
- `artifacts/trust/license-report.txt` em cada release
- Justificação em PR para dependências sensíveis ou com `unsafe`

## Relação com NORMORDIS

Dependências afectam directamente a cadeia de confiança dos artefactos
institucionais que o kernel produz. Esta política reduz alterações implícitas
e facilita auditoria independente.
