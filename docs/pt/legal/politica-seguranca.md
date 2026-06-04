---
title: "Normordis Kernel — Política de Segurança"
type: legal
framework: [NIS2, CRA, ISO27001]
status: draft
version: 0.1.0
date: 2026-06-03
lang: pt
audience: [executive, technical, auditor]
approved_by: ""
related:
  - docs/pt/compliance/nis2.md
  - docs/pt/compliance/seguranca-informacao.md
  - docs/pt/legal/declaracao-conformidade.md
---

# Normordis Kernel — Política de Segurança

> Para procedimentos operacionais de reporte de vulnerabilidades e gates de release, ver [SECURITY.md](../../../../SECURITY.md).

---

## 1. Objectivos de Segurança

O Normordis Kernel compromete-se com os seguintes objectivos de segurança, tratados como restrições arquitecturais de primeira classe:

| Objectivo | Compromisso |
|-----------|-------------|
| **Confidencialidade** | Dados em repouso cifrados com `XChaCha20-Poly1305`; segredos geridos via DPAPI/portável; nunca expostos em logs |
| **Integridade** | Registos de auditoria imutáveis com hash encadeado; `cargo audit` impede integração de código com vulnerabilidades conhecidas |
| **Disponibilidade** | Dead-letter garante que nenhum evento de controlo é perdido em falha parcial; backup auditável via `services/backup` |
| **Rastreabilidade** | Toda a operação auditável tem actor, timestamp e resultado — não existe acção sem evidência |
| **Segurança da cadeia de abastecimento** | Trust Baseline (ADR-NK-006) verificável; política de dependências formal; SBOM em `Cargo.lock` |

---

## 2. Âmbito

Esta política aplica-se ao Normordis Kernel enquanto componente de software e à sua cadeia de abastecimento. Cobre:

- O código-fonte e artefactos de build do kernel
- As dependências directas e transitivas do kernel
- O processo de desenvolvimento, integração e release
- O ciclo de vida de vulnerabilidades identificadas

Não cobre:
- A infraestrutura de deployment das organizações integradoras
- Aplicações construídas sobre o kernel
- Segurança de rede ou de sistemas operativos

---

## 3. Governação de Segurança

### 3.1 Responsabilidades

| Papel | Responsabilidade |
|-------|-----------------|
| **Responsável técnico** | Aprovação de alterações de segurança; gestão de vulnerabilidades; disclosure |
| **Contribuidores** | Cumprimento de práticas de secure coding; reporte de vulnerabilidades detectadas |
| **Integradores** | Avaliação de segurança do kernel no contexto do seu sistema; gestão de incidentes nos seus deployments |

### 3.2 Documentos de governação

| Documento | Localização | Propósito |
|-----------|-------------|-----------|
| Trust Baseline | `docs/adr/ADR-NK-006` | Política formal de confiança verificável |
| Política de dependências | `security/DEPENDENCY_POLICY.md` | Critérios de aceitação de novas dependências |
| Allowlists | `security/ALLOWLISTS.md` | `unsafe` e excepções aprovadas explicitamente |
| Procedimentos operacionais | `SECURITY.md` | Reporte de vulnerabilidades, release gate, CI |

---

## 4. Gestão de Vulnerabilidades

### 4.1 Detecção contínua

O kernel detecta automaticamente vulnerabilidades em três camadas, em cada commit:

```
Commit
  │
  ├─► cargo audit      — CVEs em dependências (RustSec Advisory DB)
  │     Bloqueante: integração rejeitada se CVE presente
  │
  ├─► cargo deny       — Licenças, duplicados, advisories (deny.toml)
  │     Bloqueante: política de licenças e advisories aplicada
  │
  └─► clippy -D warnings — Problemas de código (inclui alguns de segurança)
        Bloqueante: qualquer warning é erro de compilação
```

### 4.2 Classificação de vulnerabilidades

| Severidade | Critério | Tempo de resolução |
|------------|----------|-------------------|
| **Crítica** | CVSSv3 ≥ 9.0; exploração activa confirmada | 7 dias |
| **Alta** | CVSSv3 7.0–8.9; impacto significativo em confidencialidade ou integridade | 30 dias |
| **Média** | CVSSv3 4.0–6.9; impacto limitado ou exploração complexa | 90 dias |
| **Baixa** | CVSSv3 < 4.0 | Próxima release regular |

### 4.3 Ciclo de vida de vulnerabilidade

```
Detecção (interna via CI ou reporte externo)
    │
    ▼
Triagem — confirmação, classificação de severidade (5 dias úteis)
    │
    ▼
Desenvolvimento de correcção — em branch privado
    │
    ▼
Coordenação com reportador (se externo) — acordo sobre disclosure
    │
    ▼
Release de segurança — correcção aplicada à versão activa
    │
    ▼
Divulgação pública — CHANGELOG, advisory, crédito ao reportador
    │
    ▼
Notificação a integradores conhecidos (se impacto crítico)
```

### 4.4 Obrigações CRA (Art. 13.º e 14.º)

O CRA impõe obrigações específicas sobre vulnerabilidades activamente exploradas:

| Obrigação CRA | Implementação | Prazo |
|---------------|---------------|-------|
| Art. 13.1 — sem vulnerabilidades conhecidas no momento de release | `cargo audit` bloqueante no release gate | Contínuo |
| Art. 13.6 — política de gestão de vulnerabilidades | Esta política + `DEPENDENCY_POLICY.md` | Em vigor |
| Art. 14.1 — notificação à ENISA para vulnerabilidades activamente exploradas | `[a implementar — processo formal]` | 24 horas após conhecimento |
| Art. 14.2 — notificação aos utilizadores afectados | `SECURITY.md` + CHANGELOG | Com a correcção |

### 4.5 Obrigações NIS2 (Art. 23.º) para integradores

Os integradores que usem o kernel em sistemas sujeitos à NIS2 devem:
- Avaliar o impacto de vulnerabilidades do kernel nos seus sistemas
- Notificar o CNCS para incidentes significativos no prazo de 24 horas (notificação prévia) e 72 horas (notificação completa)
- Manter o kernel actualizado para a versão suportada

---

## 5. Ciclo de Vida Seguro de Desenvolvimento (SDLC)

### 5.1 Gates obrigatórios em cada integração (CI)

Nenhuma alteração pode ser integrada sem passar todos os gates:

| Gate | Ferramenta | O que verifica |
|------|-----------|----------------|
| Formatação | `cargo fmt --check` | Consistência de código |
| Análise estática | `cargo clippy -D warnings` | Problemas de código; warnings como erros |
| Testes | `cargo test --workspace` | Regressões funcionais e de segurança |
| Vulnerabilidades | `cargo audit` | CVEs em dependências |
| Política de dependências | `cargo deny` | Licenças, advisories, duplicados |

### 5.2 Gate de release (4 passos)

Antes de qualquer promoção a `main`, o `scripts/security/release-gate.ps1` executa:

1. **Integridade do código-fonte** — `MANIFEST.sha256` com hashes de todos os ficheiros fonte
2. **Auditoria de dependências** — `cargo audit`; relatório gravado em `artifacts/trust/`
3. **Conformidade de licenças** — `cargo deny check licenses`
4. **Build determinístico** — `cargo build --release --workspace` após fmt e clippy

O relatório `artifacts/trust/release-report.json` (git SHA, timestamps, versão do compilador, resultados) acompanha cada distribuição.

### 5.3 Práticas de secure coding

| Prática | Implementação |
|---------|---------------|
| `unsafe` bloqueado por defeito | Requer aprovação explícita em `ALLOWLISTS.md` e revisão de PR |
| Dependências criptográficas auditadas | Apenas crates RustCrypto auditadas (ver tabela em `SECURITY.md`) |
| Novas dependências requerem aprovação | `DEPENDENCY_POLICY.md` — critérios de maturidade, manutenção, histórico |
| Secrets nunca em código | `support-crypto` e `secrets` — abstracção; sem hardcode de chaves |

---

## 6. Política Criptográfica

O kernel usa exclusivamente criptografia de estado da arte, seleccionada com base nos seguintes critérios: auditada por terceiros, sem patents pendentes, resistente a ataques futuros conhecidos.

| Uso | Algoritmo | Crate | Justificação |
|-----|-----------|-------|--------------|
| Cifra autenticada (dados em repouso) | `XChaCha20-Poly1305` | `chacha20poly1305` (RustCrypto) | AEAD; resistente a nonce reutilizado; auditada |
| Derivação de chave | `Argon2id` | `argon2` (RustCrypto) | Resistente a GPU/ASIC; recomendado NIST/OWASP |
| Assinatura digital | `Ed25519` | `ed25519-dalek` (RustCrypto) | Curva de alta segurança; rápida; auditada |
| Hash criptográfico | `SHA-256 / SHA-512` | `sha2` (RustCrypto) | Standard NIST; sem colisões conhecidas |
| Gestão de segredos | DPAPI (Windows) | `secrets` | Integração com OS; chaves nunca em disco sem cifra |

**Proibido:**
- Algoritmos deprecados: MD5, SHA-1, DES, 3DES, RC4
- Cifra sem autenticação: AES-CBC sem MAC
- Chaves hardcoded no código-fonte
- Geração de números aleatórios não criptográficos para fins de segurança

---

## 7. Gestão de Incidentes de Segurança

### 7.1 O que constitui um incidente de segurança

| Tipo | Exemplos |
|------|---------|
| Vulnerabilidade no código | Buffer overflow, injecção, bypass de autenticação |
| Vulnerabilidade em dependência | CVE em crate do ecossistema Rust |
| Comprometimento da cadeia de abastecimento | Dependência maliciosa; supply chain attack |
| Violação de dados | Acesso não autorizado a dados persistidos |
| Falha de integridade | Hash chain inválido; evidência de adulteração |

### 7.2 Resposta a incidentes

```
Incidente detectado ou reportado
    │
    ├─► Registo imediato no dead-letter / SECURITY.md
    │
    ▼
Avaliação de impacto (24h)
    ├─► Que versões são afectadas?
    ├─► Que dados podem estar comprometidos?
    └─► Integradores a notificar?
    │
    ▼
Contenção e correcção
    ├─► Patch em branch privado
    └─► Coordenação com reportador
    │
    ▼
Release de segurança + divulgação controlada
    │
    ▼
Post-mortem e actualização desta política (se necessário)
```

### 7.3 Contacto de segurança

**Email:** carloscanutocosta@gmail.com
**Tempo de confirmação:** 5 dias úteis
**Tempo de resolução:** 30 dias (ver tabela de severidade em 4.2)

Para vulnerabilidades de severidade crítica: indicar `[URGENTE - SEGURANÇA]` no assunto.

---

## 8. Gestão da Cadeia de Abastecimento

A segurança da cadeia de abastecimento é gerida através de múltiplas camadas complementares:

| Camada | Mecanismo | Frequência |
|--------|-----------|------------|
| Inventário de dependências | `Cargo.lock` versionado | Por commit |
| CVEs em dependências | `cargo audit` (RustSec) | Por commit (CI) |
| Política de licenças e advisories | `cargo deny` + `deny.toml` | Por commit (CI) |
| Critérios de aceitação | `DEPENDENCY_POLICY.md` | Por nova dependência |
| Excepções aprovadas | `ALLOWLISTS.md` | Por excepção (revisão manual) |
| Trust Baseline | `ADR-NK-006` | Por release |

Para avaliação da cadeia de abastecimento por integradores (NIS2 Art. 29.º, CRA Art. 13.3), os artefactos disponíveis são: `Cargo.lock`, `artifacts/trust/audit-report.json`, `artifacts/trust/release-report.json`, e esta política.

---

## 9. Revisão desta Política

Esta política é revista:
- A cada release major do kernel
- Após qualquer incidente de segurança significativo
- Quando o quadro legal aplicável for alterado (NIS2, CRA, etc.)
- No mínimo anualmente

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial — governação, SDLC, criptografia, incidentes, supply chain |
