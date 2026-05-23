# Modelo de Ameaças — normordis-kernel

**Versão:** 0.3.x  
**Última revisão:** 2026-05-23  
**Âmbito:** crates do kernel (`crates/kernel/`) e domínio transversal (`crates/domain/`)

---

## 1. Activos Protegidos

| Activo | Classificação | Descrição |
|--------|--------------|-----------|
| Chaves privadas de assinatura | Crítico | Chaves Ed25519 / ECDSA usadas para assinar documentos e registos de auditoria |
| Segredos em repouso | Crítico | Chaves e tokens cifrados via DPAPI (Windows) ou equivalente |
| Cadeia de auditoria | Alto | Sequência de registos com hashes encadeados (`core-audit`) — integridade e não-repúdio |
| Assinaturas de documentos | Alto | Assinaturas criptográficas associadas a documentos (`core-documental`) |
| Metadados de recursos humanos | Alto | Dados pessoais geridos por `core-rh` |
| Dados de configuração | Médio | Perfis de runtime, credenciais de storage, parâmetros de cifra |
| Código-fonte do kernel | Médio | Integridade verificada via `MANIFEST.sha256` em cada release |

---

## 2. Fronteiras de Confiança

```
┌─────────────────────────────────────────────────────┐
│  App consumidora (Tauri / servidor HTTP)            │  ← não confiável por omissão
│    │                                                 │
│    ▼ chamadas à API pública normordis-kernel         │
├─────────────────────────────────────────────────────┤
│  Fachada normordis-kernel (crates/normordis-kernel) │  ← fronteira de validação
│    │                                                 │
│    ▼ uso interno                                     │
├─────────────────────────────────────────────────────┤
│  Kernel (core / support / infra)                    │  ← confiável (código auditado)
│    │                                                 │
│    ▼ I/O                                             │
├─────────────────────────────────────────────────────┤
│  Sistema operativo / storage (SQLite, DPAPI, FS)    │  ← parcialmente confiável
└─────────────────────────────────────────────────────┘
```

**Princípio:** o kernel confia no OS para operações criptográficas de baixo nível (RNG, DPAPI) mas não confia em dados vindos de fora da fronteira de validação sem verificação.

---

## 3. Adversários Considerados

### 3.1 Dependência Comprometida (supply chain)

- **Capacidade:** código malicioso introduzido numa crate transitiva
- **Impacto potencial:** exfiltração de chaves, backdoor em operações criptográficas
- **Mitigação:** `cargo audit` em CI e release gate; `cargo deny` bloqueia licenças não aprovadas; `Cargo.lock` excluído do repositório mas regenerado deterministicamente em CI

### 3.2 Aplicação Consumidora Maliciosa

- **Capacidade:** app com acesso à API do kernel que tenta aceder a dados de outros contextos ou elevar privilégios
- **Impacto potencial:** acesso não autorizado a documentos ou auditoria
- **Mitigação:** a fachada expõe apenas operações com validação de contexto; o kernel não armazena estado entre sessões de app; isolamento por instância de runtime

### 3.3 Acesso Físico ao Dispositivo (dispositivo roubado)

- **Capacidade:** acesso ao sistema de ficheiros, incluindo a base de dados SQLite e ficheiros de configuração
- **Impacto potencial:** leitura de dados em repouso
- **Mitigação:** cifra de storage via `adapter-sqlite` (SQLCipher); segredos cifrados via DPAPI (`infra-secrets`); chaves não armazenadas em plaintext

### 3.4 Adulteração do Código em Distribuição

- **Capacidade:** substituição de binários ou fontes por versões modificadas
- **Impacto potencial:** backdoor silencioso em produção
- **Mitigação:** `MANIFEST.sha256` gerado e assinado em cada release; build determinístico documentado; CI publica o hash do commit e do artefacto

### 3.5 Comprometimento de Credenciais de Desenvolvimento

- **Capacidade:** acesso ao repositório GitHub ou ambiente de build
- **Impacto potencial:** introdução de código malicioso em `main`
- **Mitigação:** branch `main` protegido (PRs obrigatórios, CI deve passar); `devel` como branch de trabalho; revisão de código antes de merge

---

## 4. Fora de Âmbito

Os seguintes cenários estão **explicitamente fora** do modelo de ameaças do kernel:

- Comprometimento do sistema operativo do host (root/SYSTEM comprometido)
- Ataques físicos ao hardware (cold boot, DMA attacks)
- Side-channels criptográficos ao nível do CPU (Spectre/Meltdown — responsabilidade do OS/CPU)
- Gestão de identidade e autenticação dos utilizadores finais (responsabilidade da app consumidora)

---

## 5. Superfície de Ataque

| Ponto de entrada | Validação aplicada |
|-----------------|-------------------|
| API pública da fachada | Validação de tipos e invariantes em `core-validation` |
| Leitura de storage (SQLite) | Deserialização tipada; sem `unsafe` em código de leitura |
| Ficheiros de configuração | Validação de schema em `core-config` |
| Operações criptográficas | Crates RustCrypto auditadas; sem implementações próprias de primitivos |
| IPC / serialização JSON | `serde` com tipos estritos; sem `serde_json::Value` não tipado em paths críticos |

---

## 6. Revisão

Este documento deve ser revisto:

- A cada alteração de arquitectura que afecte fronteiras de confiança
- Antes de qualquer release major (`x.0.0`)
- Após qualquer incidente de segurança
- Anualmente, mesmo sem alterações de arquitectura
