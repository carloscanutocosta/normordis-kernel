---
title: "Normordis Kernel — Conformidade RGPD"
type: compliance
framework: RGPD
status: draft
version: 0.1.0
date: 2026-06-03
lang: pt
audience: [executive, technical, auditor]
approved_by: ""
related:
  - docs/pt/compliance/overview.md
  - docs/pt/compliance/coso.md
  - docs/pt/legal/requirements.md
  - docs/pt/legal/registo-actividades-tratamento.md
---

# Normordis Kernel — Conformidade RGPD

## Declaração de Conformidade RGPD

O Normordis Kernel implementa **privacy by design** (Art. 25.º RGPD) como restrição arquitectural de primeira classe: a protecção de dados pessoais não é uma camada adicionada ao sistema, é uma propriedade do seu design. Dois invariantes fundamentais materializam esta declaração:

1. **Dados pessoais nunca entram em eventos de auditoria directamente** — `AuditEvent` referencia actores por `ActorId` (UUID opaco), não por dados pessoais (nome, e-mail, NIF). A associação entre `ActorId` e pessoa reside exclusivamente em `core-rh`, o único crate com autoridade sobre dados pessoais.

2. **Cifra em repouso por defeito** — toda a persistência local usa SQLite cifrado com `XChaCha20-Poly1305`. Não existe via de acesso a dados pessoais sem a chave de cifra.

Este design resolve a principal tensão entre RGPD e COSO: o direito ao apagamento (Art. 17.º) é satisfeito anonimizando o `ActorId` em `core-rh`, sem destruir a cadeia de evidência de auditoria.

---

## Sumário Executivo

O Regulamento (UE) 2016/679 (RGPD) impõe obrigações de protecção de dados pessoais a qualquer sistema que os trate. O Normordis Kernel, enquanto plataforma de suporte a aplicações da Administração Pública, processa dados de utilizadores, pessoas e titulares de cargos — todos dados pessoais na acepção do RGPD.

A estratégia do kernel é de **minimização e separação**: dados pessoais concentram-se em `core-rh` com ciclo de vida controlado; os restantes crates referenciam actores por identificador opaco. Esta arquitectura permite cumprir simultaneamente os requisitos de auditoria COSO (evidência imutável) e os direitos dos titulares RGPD (apagamento, portabilidade, rectificação).

## Âmbito

Cobre o mapeamento entre os requisitos do RGPD e a implementação do Normordis Kernel versão `0.1.x`. Não cobre:

- Tratamento de dados pessoais nas aplicações consumidoras — responsabilidade do integrador
- Registo de Actividades de Tratamento (RAT) — obrigação do responsável pelo tratamento (integrador), suportada mas não implementada pelo kernel
- Avaliação de Impacto sobre a Protecção de Dados (AIPD) — responsabilidade do integrador para sistemas de alto risco
- Gestão de consentimento — responsabilidade da aplicação consumidora

## Referências Normativas

| Ref. | Diploma / Norma | Artigo / Secção | Obrigação |
|------|----------------|-----------------|-----------|
| [RGPD-5] | Regulamento (UE) 2016/679 | Art. 5.º | Princípios relativos ao tratamento de dados pessoais |
| [RGPD-25] | Regulamento (UE) 2016/679 | Art. 25.º | Protecção de dados desde a concepção e por defeito |
| [RGPD-30] | Regulamento (UE) 2016/679 | Art. 30.º | Registos das actividades de tratamento (RAT) |
| [RGPD-32] | Regulamento (UE) 2016/679 | Art. 32.º | Segurança do tratamento — medidas técnicas e organizativas |
| [RGPD-33] | Regulamento (UE) 2016/679 | Art. 33.º–34.º | Notificação de violações de dados à autoridade e ao titular |
| [RGPD-35] | Regulamento (UE) 2016/679 | Art. 35.º | Avaliação de impacto sobre a protecção de dados (AIPD) |
| [RGPD-17] | Regulamento (UE) 2016/679 | Art. 15.º–22.º | Direitos dos titulares dos dados |

---

## Arquitectura de Privacidade do Kernel

### Separação de responsabilidades por dados pessoais

```
core-rh  ← único crate com autoridade sobre dados pessoais
    │
    │  Armazena: nome, e-mail, NIF, cargo, datas
    │  Emite:    ActorId (UUID opaco) como referência externa
    │
    ▼
core-audit, core-org, core-config, ...
    │
    │  Referenciam actores apenas por ActorId
    │  Nunca armazenam dados pessoais directamente
    │
    ▼
AuditEvent { actor: ActorId, ... }  ← sem dados pessoais
```

Esta arquitectura de separação tem uma consequência directa: **os logs de auditoria são pseudonimizados por design**. Um `ActorId` sem acesso a `core-rh` é um UUID sem significado — não identifica nenhum titular.

### Tensão COSO ↔ RGPD e como é resolvida

O audit trail append-only (requisito COSO, P1/P8) cria tensão com o direito ao apagamento (RGPD, Art. 17.º): como apagar evidências imutáveis?

A resolução é arquitectural:

```
Pedido de apagamento de utilizador U
    │
    ├─► core-rh: anonimiza ActorId de U
    │     - nome → "[apagado]"
    │     - e-mail → hash irreversível ou nulo
    │     - NIF → nulo
    │     - ActorId → mantido (UUID permanece válido mas sem ligação a pessoa)
    │
    └─► core-audit: inalterado
          - AuditEvents com actor=ActorId(U) permanecem intactos
          - A evidência de controlo é preservada (COSO)
          - O ActorId já não é atribuível a uma pessoa (RGPD)
```

O resultado: a evidência de "alguém fez X" é preservada para efeitos de auditoria, mas "quem" deixa de ser determinável — satisfazendo simultaneamente COSO e RGPD.

---

## Mapeamento por Princípio RGPD (Art. 5.º)

### a) Licitude, lealdade e transparência

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Audit trail de operações de tratamento | `core-audit` | `AuditEvent` por cada operação sobre dados pessoais | Implementado |
| Documentação de interface | `docs/` | ADRs, architecture overview — base para informação ao titular | Implementado |
| Gestão de consentimento | — | Responsabilidade da app integradora | Planeado (contratos) |

### b) Limitação das finalidades

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Separação de responsabilidades | `core-rh` (dados pessoais isolados) | Dados pessoais não fluem para outros crates | Implementado |
| `control_id` por finalidade | `core-audit` | Cada operação tem uma finalidade de controlo explícita | Implementado (base) |

### c) Minimização dos dados

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Validação sem armazenamento | `core-validation` | NIF, IBAN, e-mail validados sem persistência desnecessária | Implementado |
| `ActorId` como referência opaca | `core-audit`, `core-org` | Outros crates não têm acesso a dados pessoais de actores | Implementado |
| Campos obrigatórios mínimos em `core-rh` | `core-rh` | Apenas dados necessários à função são recolhidos | Implementado (base) |

### d) Exactidão

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Validação canónica de dados pessoais | `core-validation` | NIF, IBAN, e-mail, datas — validação antes de persistência | Implementado |
| OCC em actualizações | `core-org` | Conflitos de actualização detectados explicitamente | Implementado |
| Rectificação auditada | `core-rh` | Alterações a dados pessoais geram `AuditEvent` | Implementado |

### e) Limitação da conservação

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Suporte a prazos de retenção | — | Contratos de retenção a definir (integrador define prazos) | Planeado |
| Apagamento controlado via anonimização | `core-rh` | Ver secção "Tensão COSO ↔ RGPD" | Implementado (base) |

### f) Integridade e confidencialidade

| Mecanismo | Crate | Implementação | Status |
|-----------|-------|---------------|--------|
| Cifra em repouso | `infra` (SQLite) | `XChaCha20-Poly1305` — dados ilegíveis sem chave | Implementado |
| Gestão de secrets | `secrets` | DPAPI (Windows) ou fallback portável; secrets nunca em logs | Implementado |
| Hash encadeado de eventos | `core-audit` | Integridade do log verificável e adulteração detectável | Implementado |
| Chaves geridas por `support-crypto` | `support-crypto` | `Argon2id` para derivação de chaves, `XChaCha20-Poly1305` | Implementado |

---

## Segurança do Tratamento (Art. 32.º)

O Art. 32.º exige medidas técnicas e organizativas adequadas ao risco. O kernel implementa:

| Medida (Art. 32.º) | Implementação | Status |
|--------------------|---------------|--------|
| Pseudonimização | `ActorId` como referência opaca; dados pessoais em `core-rh` isolado | Implementado |
| Cifra | SQLite `XChaCha20-Poly1305` em repouso; `Argon2id` para derivação | Implementado |
| Confidencialidade e integridade contínuas | Cifra + hash chain + append-only | Implementado |
| Capacidade de restauro | `services/backup` — backup controlado | Implementado |
| Testes regulares | CI pipeline — `cargo audit`, testes obrigatórios por commit | Implementado |
| Avaliação de risco contínua | `core-audit` dead-letter, `ControlOutcome::Failed` | Implementado |

---

## Direitos dos Titulares (Arts. 15.º–22.º)

| Direito | Artigo | Suporte no kernel | Status |
|---------|--------|------------------|--------|
| Direito de acesso | Art. 15.º | `core-exports` — exportação de dados do titular | Implementado (base) |
| Direito de rectificação | Art. 16.º | `core-rh` — actualização auditada de dados pessoais | Implementado |
| Direito ao apagamento | Art. 17.º | `core-rh` — anonimização que preserva audit trail (ver arquitectura) | Implementado (base) |
| Direito à limitação | Art. 18.º | — | Planeado |
| Direito à portabilidade | Art. 20.º | `core-exports` — exportação em formatos abertos | Implementado (base) |
| Direito de oposição | Art. 21.º | — | Planeado (responsabilidade do integrador) |

---

## Matriz de Conformidade

| Requisito RGPD | Crate / Módulo | Evidência | Status |
|---------------|---------------|-----------|--------|
| Privacy by design (Art. 25.º) | Arquitectura (separação core-rh / ActorId) | ActorId como referência opaca; dados pessoais isolados | Implementado |
| Cifra em repouso (Art. 32.º) | `infra` (SQLite), `support-crypto` | XChaCha20-Poly1305, Argon2id | Implementado |
| Gestão de secrets (Art. 32.º) | `secrets` | DPAPI / fallback; sem exposição em logs | Implementado |
| Pseudonimização (Art. 32.º) | `core-audit` (ActorId) | Logs sem dados pessoais directos | Implementado |
| Integridade do log (Art. 32.º) | `core-audit` | Hash encadeado, append-only | Implementado |
| Backup e restauro (Art. 32.º) | `services/backup` | Backup controlado | Implementado |
| Rectificação auditada (Art. 16.º) | `core-rh` | Alterações geram AuditEvent | Implementado |
| Apagamento por anonimização (Art. 17.º) | `core-rh` | ActorId desassociado de pessoa | Implementado (base) |
| Exportação de dados do titular (Art. 15.º, 20.º) | `core-exports` | Exportação estruturada | Implementado (base) |
| Validação de dados pessoais (Art. 5.º.d) | `core-validation` | NIF, IBAN, e-mail, datas | Implementado |
| Audit trail de operações de tratamento (Art. 5.º) | `core-audit` | AuditEvent por operação | Implementado |
| RAT — Registo de Actividades de Tratamento (Art. 30.º) | — | — | Planeado |
| Notificação de violações (Art. 33.º) | — | Dead-letter como detecção; notificação a implementar | Planeado |
| AIPD — Avaliação de Impacto (Art. 35.º) | — | Responsabilidade do integrador; suporte técnico disponível | Planeado |
| Gestão de consentimento (Art. 6.º–7.º) | — | Responsabilidade da app integradora | Planeado |
| Limitação de conservação (Art. 5.º.e) | — | Contratos de retenção a definir | Planeado |

## Limitações e Exclusões Conhecidas

- **RAT**: o kernel gera eventos auditáveis de operações de tratamento mas não implementa um Registo de Actividades de Tratamento (Art. 30.º) dedicado. O integrador é o responsável pelo tratamento e deve manter o RAT.
- **Consentimento**: a gestão de consentimento (Art. 6.º–7.º) é responsabilidade da aplicação consumidora. O kernel não implementa mecanismos de consentimento.
- **Transferências internacionais**: o kernel opera localmente; não efectua transferências de dados para países terceiros. Aplicações que o façam devem avaliar os requisitos dos Arts. 44.º–49.º.
- **Direito à limitação (Art. 18.º)**: não implementado — requer mecanismo de marcação de dados como "limitados" sem os apagar.
- **AIPD**: a avaliação de impacto é responsabilidade do integrador para sistemas de alto risco; o kernel fornece a documentação técnica necessária (este documento e os ADRs).
- **Apagamento nos logs de backup**: os backups gerados por `services/backup` podem conter dados pessoais não anonimizados — o processo de anonimização deve incluir os backups.

## Glossário

| Termo | Definição |
|-------|-----------|
| **AIPD** | Avaliação de Impacto sobre a Protecção de Dados — obrigação RGPD Art. 35.º para tratamentos de alto risco |
| **ActorId** | Identificador opaco (UUID) que referencia um actor nas operações de auditoria, sem conter dados pessoais directos |
| **Anonimização** | Processo irreversível que remove a possibilidade de identificar um titular; dados anonimizados saem do âmbito do RGPD |
| **CNPD** | Comissão Nacional de Protecção de Dados — autoridade de controlo portuguesa |
| **Privacy by design** | Princípio RGPD Art. 25.º — a protecção de dados é integrada no design do sistema, não adicionada posteriormente |
| **Privacy by default** | Princípio RGPD Art. 25.º — por defeito, apenas os dados mínimos necessários são tratados |
| **Pseudonimização** | Substituição de dados identificativos por um identificador artificial (ex: ActorId); dados pseudonimizados continuam no âmbito do RGPD |
| **RAT** | Registo de Actividades de Tratamento — Art. 30.º RGPD; documento que lista todas as operações de tratamento de dados pessoais |
| **Responsável pelo tratamento** | Entidade que determina as finalidades e meios de tratamento; tipicamente a organização que usa o kernel, não o kernel em si |
| **RGPD** | Regulamento Geral sobre a Proteção de Dados — Regulamento (UE) 2016/679 |
| **Subcontratante** | Entidade que trata dados pessoais por conta do responsável; o kernel pode ser considerado subcontratante dependendo do modelo de uso |
| **Titular dos dados** | Pessoa singular identificada ou identificável a quem os dados dizem respeito |

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial — arquitectura de privacidade, separação ActorId/dados pessoais, resolução tensão COSO↔RGPD |
