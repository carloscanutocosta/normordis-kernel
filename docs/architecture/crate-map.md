# normordis-kernel — Mapa de Crates

Estado: Activo  
Versão: v0.3.0  
Actualizado: 2026-05-23

---

## 1. Objectivo

Inventariar e classificar todos os crates do workspace `normordis-kernel` por
camada arquitectural, responsabilidade e estado.

---

## 2. Camadas lógicas

```
Foundation / Core      — semântica de domínio, portos e invariantes
Support                — primitivos e contratos técnicos transversais
Infra / Adapters       — adapters concretos (SQLite, PDF, OS)
Domain transversal     — domínios com semântica institucional forte
Runtime / Bootstrap    — composição e ciclo de vida
Fachada pública        — API unificada para consumidores
```

---

## 3. Core — `crates/kernel/core/`

| Crate | Responsabilidade |
|-------|-----------------|
| `core-audit` | Auditoria append-only com cadeia de hashes verificável |
| `core-config` | Configuração de perfis de deployment (dev, staging, prod) |
| `core-validation` | Validadores canónicos: NIF, IBAN, email, UUID, datas |
| `core-rh` | Identidade, autenticação e contexto de utilizador |
| `core-org` | Unidades orgânicas, posições e hierarquia institucional |
| `core-security` | Políticas de acesso e autorização |
| `core-documental` | Ciclo de vida documental, templates NDT e log de eventos |
| `core-exports` | Exportação de dados e snapshots auditáveis |
| `core-ingest` | Entrada e registo de documentos externos |
| `core-metrics` | Métricas de runtime da plataforma |

**Regra:** nenhum crate `core/` depende de `infra/`. Declara ports/traits;
não implementa adapters.

---

## 4. Support — `crates/kernel/support/`

| Crate | Responsabilidade |
|-------|-----------------|
| `support-auth` | Contratos de autenticação e sessão |
| `support-errors` | Erros canónicos transversais (`MiniError`, `PublicError`, `ERRORS.json`) |
| `support-backup` | Contratos e operações de backup |
| `support-crypto` | Primitivos criptográficos (`XChaCha20-Poly1305`, `Argon2id`, `KeyProvider`) |
| `support-storage` | Contratos genéricos de storage (headless, sem SQLite) |
| `support-logging` | Diagnóstico técnico local em JSONL |
| `support-normalization` | Normalização e validações determinísticas |
| `support-typst-template` | Contratos de templates Typst |
| `support-pdf` | Contratos de geração PDF |
| `support-clock` | Abstracção de tempo (testável e determinista) |
| `support-ids` | Geração de identificadores únicos (UUID v4) |
| `support-address` | Normalização e validação de moradas |
| `support-versioning` | Compatibilidade e migrações de esquema |
| `support-docx-to-typst` | Conversão DOCX → Typst |

---

## 5. Infra — `crates/kernel/infra/`

### Adapters SQLite

| Crate | Responsabilidade |
|-------|-----------------|
| `adapter-sqlite` | Adapter SQLite base (writer queue, retry, mixed-mode SQLCipher) |
| `rh-sqlite` | Persistência SQLite de `core-rh` |
| `org-sqlite` | Persistência SQLite de `core-org` |
| `security-sqlite` | Persistência SQLite de `core-security` |
| `documental-sqlite` | Persistência SQLite de `core-documental` |
| `metrics-sqlite` | Persistência SQLite de `core-metrics` |
| `exports-sqlite` | Persistência SQLite de `core-exports` |
| `address-sqlite` | Persistência SQLite de `support-address` |
| `versioning-sqlite` | Persistência SQLite de `support-versioning` |
| `storage-sqlite` | Adapter SQLite para `support-storage` |
| `numerador-sqlite` | Persistência SQLite de `domain-numerador` |
| `mef-sqlite` | Persistência SQLite de `domain-mef` |

### Adapters de sistema

| Crate | Responsabilidade |
|-------|-----------------|
| `secrets` | Gestão de segredos (DPAPI em Windows, fallback portável) |
| `files` | Operações de filesystem controladas |
| `adapter-scanner` | Digitalização e entrada de documentos |
| `rh-security-bridge` | Bridge entre `core-rh` e `core-security` |

### Serviços infra

| Crate | Responsabilidade |
|-------|-----------------|
| `services/signing` | Serviço de assinatura de documentos |
| `services/export` | Serviço de exportação |
| `services/backup` | Serviço de backup |
| `services/ingest-scanner` | Serviço de ingestão via scanner |

### Bootstrap

| Crate | Responsabilidade |
|-------|-----------------|
| `runtime-bootstrap` | Composição de infra do kernel (incluindo audit.db dedicado) |
| `app-bootstrap` | Arranque e preparação do ambiente local |

### Pipeline PDF

| Crate | Responsabilidade |
|-------|-----------------|
| `pdf/render-typst` | Renderização Typst para PDF |
| `pdf/normordis-pdf` | Geração PDF institucional |
| `pdf/documentos-pdf` | Pipeline PDF de documentos específicos |
| `pdf/pdf-pipeline` | Orquestrador assíncrono do pipeline PDF |

---

## 6. Domain transversal — `crates/domain/`

| Crate | Responsabilidade |
|-------|-----------------|
| `domain-numerador` | Numeração sequencial de documentos institucionais |
| `domain-mef` | Classificação orçamental MEF com tabela temporal e diploma legal |

---

## 7. Runtime — `crates/runtime/`

| Crate | Responsabilidade |
|-------|-----------------|
| `miniapp-runtime` | Contexto partilhado e ciclo de vida de mini-apps |
| `interoperability` | Contratos de interoperabilidade entre runtimes |

---

## 8. Fachada pública — `crates/normordis-kernel/`

| Crate | Responsabilidade |
|-------|-----------------|
| `normordis-kernel` | API pública unificada — re-exporta módulos de `core/`, `domain/` e `runtime/` |

Os adapters de `infra/` não são expostos na fachada — são detalhes de
implementação injectados pelo bootstrap das apps consumidoras.

---

## 9. Regras de evolução

1. Novos crates nascem na camada correcta (`core/`, `support/` ou `infra/`).
2. Não mover crates existentes sem necessidade concreta.
3. Cada crate deve ser testável isoladamente.
4. Novas dependências seguem `security/DEPENDENCY_POLICY.md`.
5. Crates com `unsafe` são aprovados explicitamente em `security/ALLOWLISTS.md`.
