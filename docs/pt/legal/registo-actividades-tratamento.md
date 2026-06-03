---
title: "Normordis Kernel — Registo de Actividades de Tratamento (RAT)"
type: legal
framework: RGPD
status: draft
version: 0.1.0
date: 2026-06-03
lang: pt
audience: [executive, technical, auditor]
approved_by: ""
related:
  - docs/pt/compliance/rgpd.md
  - docs/pt/compliance/overview.md
  - docs/pt/legal/requirements.md
---

# Normordis Kernel — Registo de Actividades de Tratamento (RAT)

## Enquadramento Legal

O Art. 30.º do Regulamento (UE) 2016/679 (RGPD) obriga os responsáveis pelo tratamento e os subcontratantes a manter um registo das actividades de tratamento de dados pessoais sob a sua responsabilidade.

**Posicionamento do kernel:** O Normordis Kernel actua tipicamente como **subcontratante** (*processor*) — processa dados pessoais por conta das organizações que o utilizam, as quais são os responsáveis pelo tratamento (*controllers*). Cada organização integradora deve adaptar este RAT ao seu contexto específico, completando as secções marcadas com `[a preencher pelo integrador]`.

Este documento cumpre simultaneamente dois propósitos:
1. **RAT de plataforma** — regista as actividades de tratamento inerentes ao funcionamento do kernel como componente de software.
2. **Modelo base para integradores** — serve de ponto de partida para o RAT completo de cada deployment.

---

## Identificação das Entidades

| Campo | Kernel (subcontratante) | Integrador (responsável) |
|-------|------------------------|--------------------------|
| Designação | Normordis Kernel | `[a preencher pelo integrador]` |
| Responsável | carloscanutocosta@gmail.com | `[a preencher pelo integrador]` |
| Encarregado de Protecção de Dados (EPD) | — | `[a preencher pelo integrador]` |
| Contacto | carloscanutocosta@gmail.com | `[a preencher pelo integrador]` |

---

## Actividades de Tratamento

### AT-01 — Gestão de Utilizadores e Identidade

| Campo | Descrição |
|-------|-----------|
| **Designação** | Gestão do ciclo de vida de utilizadores e colaboradores |
| **Crate(s)** | `core-rh` — `UserRepository`, `UserService`, `PersonAssignment` |
| **Titular dos dados** | Utilizadores do sistema; colaboradores da organização |
| **Categorias de dados pessoais** | Nome; endereço de correio electrónico; NIF; cargo/função; data de início e fim de atribuição; identificador opaco (`ActorId`) |
| **Categorias especiais (Art. 9.º)** | Não aplicável |
| **Finalidade** | Autenticação e identificação; atribuição de funções e responsabilidades; rastreabilidade de operações |
| **Base jurídica** | Art. 6.1.e — exercício de funções públicas; Art. 6.1.b — execução de contrato de trabalho |
| **Destinatários** | Nenhum terceiro; dados processados localmente |
| **Transferências para países terceiros** | Não aplicável — execução local (SQLite) |
| **Prazo de conservação** | `[a definir pelo integrador]` — recomendado: duração da relação + período legal de retenção |
| **Medidas de segurança** | SQLite cifrado (`XChaCha20-Poly1305`); `ActorId` opaco nos logs de auditoria; gestão de secrets via DPAPI |
| **Observações** | Os dados pessoais de AT-01 nunca entram nos eventos de auditoria (AT-02). A ligação entre `ActorId` e pessoa reside exclusivamente neste módulo. |

---

### AT-02 — Auditoria de Operações

| Campo | Descrição |
|-------|-----------|
| **Designação** | Registo imutável de operações de controlo interno |
| **Crate(s)** | `core-audit` — `AuditStore`, `AuditEvent`, outbox, dead-letter |
| **Titular dos dados** | Utilizadores que executam operações auditáveis |
| **Categorias de dados pessoais** | `ActorId` (identificador opaco — pseudónimo do utilizador); timestamp da operação; resultado do controlo (`outcome`) |
| **Categorias especiais (Art. 9.º)** | Não aplicável |
| **Finalidade** | Controlo interno (COSO); evidência para auditoria (Tribunal de Contas); rastreabilidade de responsabilidade; detecção de desvios |
| **Base jurídica** | Art. 6.1.c — obrigação legal (Lei 98/97, COSO, INTOSAI); Art. 6.1.e — exercício de funções públicas |
| **Destinatários** | Auditores internos; Tribunal de Contas (quando solicitado); `[a completar pelo integrador]` |
| **Transferências para países terceiros** | Não aplicável — execução local |
| **Prazo de conservação** | Mínimo: duração do período de responsabilidade financeira legalmente exigido (recomendado: 10 anos para evidências do Tribunal de Contas) |
| **Medidas de segurança** | Append-only (sem UPDATE/DELETE); hash encadeado (adulteração detectável); SQLite cifrado; `ActorId` pseudonimizado |
| **Observações** | O log de auditoria é **pseudonimizado por design**: `ActorId` não é dados pessoais directos. O apagamento de dados pessoais (AT-01) não destrói a evidência de auditoria — apenas remove a ligação entre `ActorId` e pessoa. Ver [docs/pt/compliance/rgpd.md](../compliance/rgpd.md). |

---

### AT-03 — Estrutura Organizacional

| Campo | Descrição |
|-------|-----------|
| **Designação** | Modelação da estrutura orgânica, posições e substituições legais |
| **Crate(s)** | `core-org` — `OrgAuditAdapter`, `PositionKind`, `substitutes` |
| **Titular dos dados** | Titulares de cargos e posições na organização |
| **Categorias de dados pessoais** | `ActorId` associado a posição; datas de início e fim de ocupação |
| **Categorias especiais (Art. 9.º)** | Não aplicável |
| **Finalidade** | Definição de estrutura de autoridade e responsabilidade; substituição legal de funções; base para controlos COSO |
| **Base jurídica** | Art. 6.1.e — exercício de funções públicas; Art. 6.1.c — obrigação legal (estrutura orgânica obrigatória) |
| **Destinatários** | Nenhum terceiro; processamento local |
| **Transferências para países terceiros** | Não aplicável |
| **Prazo de conservação** | `[a definir pelo integrador]` — recomendado: duração do mandato + período de responsabilidade |
| **Medidas de segurança** | SQLite cifrado; OCC (Optimistic Concurrency Control) — alterações sem conflitos silenciosos; audit trail de mudanças via `OrgAuditAdapter` |

---

### AT-04 — Ciclo de Vida Documental

| Campo | Descrição |
|-------|-----------|
| **Designação** | Gestão de documentos institucionais — criação, tramitação, assinatura e arquivo |
| **Crate(s)** | `core-documental`, `core-ingest`, `services/signing`, `domain-numerador` |
| **Titular dos dados** | Autores, destinatários, signatários e destinatários de documentos |
| **Categorias de dados pessoais** | Nome e `ActorId` do autor; destinatários (nome, e-mail, NIF quando aplicável); dados constantes do conteúdo do documento |
| **Categorias especiais (Art. 9.º)** | Depende do conteúdo documental — `[a avaliar pelo integrador]` |
| **Finalidade** | Produção e gestão de documentação institucional; arquivo; conformidade arquivística; assinatura qualificada |
| **Base jurídica** | Art. 6.1.e — exercício de funções públicas; Art. 6.1.c — obrigação legal (DL 447/88, Portaria 412/2001) |
| **Destinatários** | Destinatários dos documentos; arquivo institucional; `[a completar pelo integrador]` |
| **Transferências para países terceiros** | Não aplicável — execução local |
| **Prazo de conservação** | Conforme tabela de selecção arquivística aplicável (MEF-DGLAB, Portaria 412/2001) — `[a definir pelo integrador]` |
| **Medidas de segurança** | SQLite cifrado; estados de documento imutáveis após finalização; assinatura qualificada PAdES; audit trail de operações |

---

### AT-05 — Telemetria de Uso e Métricas

| Campo | Descrição |
|-------|-----------|
| **Designação** | Recolha de eventos de uso e métricas operacionais para monitorização do sistema |
| **Crate(s)** | `domain-telemetry`, `core-metrics` |
| **Titular dos dados** | Utilizadores cujas acções geram eventos de uso |
| **Categorias de dados pessoais** | `ActorId` (pseudónimo); timestamp; tipo de operação; identificadores de recurso acedido |
| **Categorias especiais (Art. 9.º)** | Não aplicável |
| **Finalidade** | Monitorização operacional; detecção de anomalias; avaliação de eficácia de controlos (COSO C5); suporte à monitorização AI Act Art. 72.º |
| **Base jurídica** | Art. 6.1.e — exercício de funções públicas; interesse legítimo na monitorização do sistema |
| **Destinatários** | Administradores do sistema; `[a completar pelo integrador]` |
| **Transferências para países terceiros** | Não aplicável |
| **Prazo de conservação** | `[a definir pelo integrador]` — recomendado: 90 dias para telemetria operacional |
| **Medidas de segurança** | `ActorId` pseudonimizado; dados agregados onde possível; SQLite cifrado |

---

### AT-06 — Logs de Diagnóstico Técnico

| Campo | Descrição |
|-------|-----------|
| **Designação** | Logs técnicos para diagnóstico e resolução de problemas |
| **Crate(s)** | `support-logging` |
| **Titular dos dados** | Utilizadores cujas sessões geram erros ou eventos de diagnóstico |
| **Categorias de dados pessoais** | `ActorId` (quando presente em contexto de erro); timestamps; identificadores de sessão |
| **Categorias especiais (Art. 9.º)** | Não aplicável |
| **Finalidade** | Diagnóstico técnico; resolução de erros; suporte operacional |
| **Base jurídica** | Art. 6.1.f — interesse legítimo na operação técnica do sistema |
| **Destinatários** | Equipa técnica de suporte; `[a completar pelo integrador]` |
| **Transferências para países terceiros** | Não aplicável |
| **Prazo de conservação** | `[a definir pelo integrador]` — recomendado: 30 dias; nunca superior ao prazo de AT-02 |
| **Medidas de segurança** | Logs em formato JSONL local; secrets nunca registados; `ActorId` em vez de dados pessoais directos; ficheiros cifrados em repouso |

---

### AT-07 — Validação de Identificadores Pessoais

| Campo | Descrição |
|-------|-----------|
| **Designação** | Validação de identificadores pessoais como NIF, IBAN, endereço de correio electrónico |
| **Crate(s)** | `core-validation`, `support-address` |
| **Titular dos dados** | Titulares dos identificadores validados |
| **Categorias de dados pessoais** | NIF; IBAN; endereço de correio electrónico; morada (temporariamente em memória durante validação) |
| **Categorias especiais (Art. 9.º)** | Não aplicável |
| **Finalidade** | Verificação de exactidão de dados antes de persistência; conformidade com Art. 5.1.d RGPD |
| **Base jurídica** | Art. 6.1.e — exercício de funções públicas; obrigação de exactidão de dados |
| **Destinatários** | Nenhum — validação executada localmente sem transmissão |
| **Transferências para países terceiros** | Não aplicável — validação local, sem chamadas externas |
| **Prazo de conservação** | Não aplicável — dados apenas em memória durante validação; não persistidos por este módulo |
| **Medidas de segurança** | Validação em memória; sem persistência de dados pelo módulo de validação; sem transmissão externa |

---

## Sumário de Actividades

| ID | Actividade | Crate(s) | Dados pessoais | Base jurídica | Retenção |
|----|-----------|----------|---------------|---------------|----------|
| AT-01 | Gestão de utilizadores | `core-rh` | Nome, email, NIF, cargo | Art. 6.1.b/e | `[integrador]` |
| AT-02 | Auditoria de operações | `core-audit` | `ActorId` (pseudónimo) | Art. 6.1.c/e | ≥ 10 anos (TC) |
| AT-03 | Estrutura organizacional | `core-org` | `ActorId`, datas de cargo | Art. 6.1.c/e | `[integrador]` |
| AT-04 | Ciclo de vida documental | `core-documental` | Autor, destinatários | Art. 6.1.c/e | Tabela de selecção |
| AT-05 | Telemetria de uso | `domain-telemetry` | `ActorId`, eventos | Art. 6.1.e | ≤ 90 dias |
| AT-06 | Logs de diagnóstico | `support-logging` | `ActorId` (ocasional) | Art. 6.1.f | ≤ 30 dias |
| AT-07 | Validação de identificadores | `core-validation` | NIF, IBAN, email (memória) | Art. 6.1.e | N/A |

---

## Direitos dos Titulares

| Direito (RGPD) | Artigo | Mecanismo de exercício no kernel |
|---------------|--------|----------------------------------|
| Acesso | Art. 15.º | `core-exports` — exportação dos dados do titular por `ActorId` |
| Rectificação | Art. 16.º | `core-rh` — actualização auditada de dados de AT-01 |
| Apagamento | Art. 17.º | `core-rh` — anonimização de `ActorId`; AT-02 preservado (evidência de controlo) |
| Portabilidade | Art. 20.º | `core-exports` — exportação em formatos abertos |
| Limitação | Art. 18.º | `[a implementar pelo integrador]` |
| Oposição | Art. 21.º | `[a implementar pelo integrador]` |

---

## Transferências para Países Terceiros

**Não se aplicam transferências de dados pessoais para países terceiros.** O kernel opera exclusivamente em modo local (SQLite no dispositivo/servidor da organização), sem transmissão de dados para serviços externos ou cloud.

---

## Medidas de Segurança Transversais

| Medida | Implementação |
|--------|---------------|
| Cifra em repouso | SQLite com `XChaCha20-Poly1305` em todos os módulos |
| Pseudonimização | `ActorId` (UUID opaco) em logs de auditoria — dados pessoais isolados em AT-01 |
| Gestão de segredos | DPAPI (Windows) / fallback portável; nunca expostos em logs |
| Minimização | Validação sem persistência (AT-07); telemetria agregada quando possível (AT-05) |
| Integridade | Hash encadeado em AT-02; OCC em AT-03 |
| Controlo de acesso | `core-security` — políticas por recurso (em desenvolvimento) |

---

## Notas para Integradores

Este RAT cobre as actividades de tratamento **inerentes ao kernel**. Os integradores devem:

1. **Completar** os campos marcados `[a preencher pelo integrador]` com os dados da sua organização.
2. **Acrescentar** actividades de tratamento específicas das suas aplicações (não cobertas pelo kernel).
3. **Nomear** o Encarregado de Protecção de Dados (EPD) quando obrigatório.
4. **Rever** os prazos de conservação de acordo com a legislação sectorial aplicável.
5. **Avaliar** se o conteúdo de documentos em AT-04 contém categorias especiais de dados (Art. 9.º).
6. **Registar** este RAT junto da CNPD se exigido pela sua actividade.

---

## Limitações e Exclusões Conhecidas

- **Limitação de tratamento (Art. 18.º)**: não implementado no kernel; responsabilidade do integrador.
- **Oposição (Art. 21.º)**: não implementado no kernel; responsabilidade do integrador.
- **Categorias especiais (Art. 9.º)**: o kernel não trata categorias especiais por defeito. Se o conteúdo documental (AT-04) incluir dados de saúde, dados biométricos ou outros, o integrador deve avaliar a base jurídica do Art. 9.º.
- **Notificação de violações (Art. 33.º–34.º)**: o dead-letter em AT-02 detecta falhas técnicas; a notificação formal à CNPD e aos titulares é responsabilidade do responsável pelo tratamento (integrador).

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial — AT-01 a AT-07; sumário; direitos dos titulares; notas para integradores |
