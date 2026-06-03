---
title: "Normordis Kernel — Aviso de Controlo de Exportação"
type: legal
framework: [CRA]
status: draft
version: 0.1.0
date: 2026-06-03
lang: pt
audience: [executive, technical, auditor]
approved_by: ""
related:
  - docs/pt/legal/terceiros.md
  - docs/pt/legal/declaracao-conformidade.md
  - docs/pt/compliance/seguranca-informacao.md
---

# Normordis Kernel — Aviso de Controlo de Exportação

> **Aviso:** Este documento é informativo e não constitui aconselhamento jurídico. Para operações de exportação específicas, consulte um especialista em direito do comércio internacional ou a autoridade nacional competente.

---

## 1. Contexto

O Normordis Kernel inclui componentes de software criptográfico — cifra simétrica (`XChaCha20-Poly1305`), derivação de chave (`Argon2id`), assinaturas digitais (`Ed25519`, `RSA`, `ECDSA P-256/P-384`) e funções de hash (`SHA-256/SHA-512`). Software com capacidades criptográficas está sujeito a regulamentação de controlo de exportação em várias jurisdições.

Este documento identifica o enquadramento regulamentar aplicável, a classificação do kernel, as isenções relevantes e as restrições conhecidas.

---

## 2. Quadro Regulamentar

### 2.1 União Europeia — Regulamento de Dupla Utilização

**Instrumento principal:** Regulamento (UE) 2021/821 do Parlamento Europeu e do Conselho, de 20 de maio de 2021 (refundição do Regulamento de Dupla Utilização)

Este regulamento controla a exportação, a corretagem, a assistência técnica e o trânsito de **bens de dupla utilização** — produtos, software e tecnologia que podem ser utilizados tanto para fins civis como militares ou de proliferação.

O software criptográfico é abrangido pela **Categoria 5, Parte 2 — Segurança da Informação** da Lista de Controlo da UE (Anexo I do Regulamento).

### 2.2 Portugal — Autoridade Nacional Competente

A autoridade nacional competente para controlos de exportação em Portugal é a **Direção-Geral das Atividades Económicas (DGAE)** do Ministério da Economia.

### 2.3 Sanções internacionais

Para além dos controlos de dupla utilização, as exportações estão sujeitas a **regimes de sanções** da UE, geridos pelo Conselho da UE e executados pela DGAE em Portugal. Os regimes de sanções actuais incluem restrições para a Rússia, Belarus, Irão, Coreia do Norte e outros.

---

## 3. Classificação do Kernel

### 3.1 Categoria aplicável

O Normordis Kernel contém software com capacidades criptográficas que pode enquadrar-se na categoria **5D002** da lista de dupla utilização — *"Software concebido ou modificado para utilizar criptografia"*.

### 3.2 Características relevantes para classificação

| Característica | Valor |
|---------------|-------|
| Propósito principal | Plataforma de gestão e controlo interno para AP — a criptografia é **anciliar**, não o produto principal |
| Algoritmos | Apenas algoritmos públicos, padronizados e amplamente adoptados (NIST, IETF) |
| Implementações | Código aberto (RustCrypto, projecto público com revisões de segurança) |
| Licença | EUPL-1.2 — licença de serviço público europeu, publicada abertamente |
| Comprimento de chave | AES-equivalente: XChaCha20 (256 bits); RSA: 2048–4096 bits; Ed25519: 256 bits |

### 3.3 Nota sobre o propósito da criptografia

A criptografia no kernel serve exclusivamente fins de **protecção de dados em repouso** e **integridade de evidências de controlo** — não de comunicações secretas, evasão de vigilância ou outros fins que as regulamentações de dupla utilização visam controlar primariamente. Esta distinção é relevante para a avaliação de risco de exportação.

---

## 4. Isenções Aplicáveis

### 4.1 Software de código aberto disponível ao público

A **Nota Técnica da Categoria 5 Parte 2** do Regulamento 2021/821 exclui do controlo o software que seja:

> *"Do domínio público"* — tecnologia ou software disponibilizado sem restrições à sua disseminação posterior.

O Normordis Kernel é publicado sob EUPL-1.2 e disponível publicamente. As implementações criptográficas utilizadas (RustCrypto) são igualmente código aberto disponível publicamente sem restrições de acesso.

### 4.2 Autorização Geral de Exportação EU001

O Regulamento 2021/821, Art. 26.º, prevê **autorizações gerais de exportação** que dispensam autorização individual para certas transferências. A **EU001** cobre exportações para determinados destinos (AUS, CAN, JP, NZ, NO, CHE, GBR, USA) de muitos itens de dupla utilização de baixo risco.

### 4.3 Transferências intra-UE

As transferências de software entre Estados-Membros da UE não são exportações na acepção do Regulamento 2021/821 e estão fora do seu âmbito de controlo.

---

## 5. Restrições Conhecidas

As seguintes restrições aplicam-se independentemente da classificação de dupla utilização:

| Restrição | Fundamento | Países/Entidades afectadas |
|-----------|-----------|---------------------------|
| Sanções UE — embargo total ou parcial | Regulamentos de sanções do Conselho da UE | Rússia, Belarus, Irão, Coreia do Norte, Myanmar, Sudão, Venezuela, Cuba, Síria (e outros — ver lista actualizada) |
| Entidades listadas | Regulamento UE de controlos de exportação | Entidades em listas de restrição da UE, ONU ou EUA |
| Uso militar | Art. 4.º Regulamento 2021/821 | Qualquer destino se houver suspeita de uso militar |

> **Atenção:** Os regimes de sanções são actualizados frequentemente. Antes de qualquer exportação ou transferência, consultar a lista actualizada em **eur-lex.europa.eu** e o portal da DGAE.

---

## 6. Obrigações do Integrador

Os integradores que redistribuam ou exportem produtos que incluam o Normordis Kernel devem:

1. **Avaliar** se o produto final se enquadra nas categorias de controlo de exportação da jurisdição de origem e de destino
2. **Verificar** que o destino não está sujeito a sanções ou embargos aplicáveis
3. **Obter** as autorizações de exportação necessárias antes de qualquer transferência para destinos fora da UE
4. **Manter** registos das exportações efectuadas (obrigação regulamentar)
5. **Consultar** a DGAE ou um especialista jurídico para exportações para destinos de risco elevado ou para usos especializados

Para redistribuição **dentro da UE** (entre Estados-Membros), não existem obrigações de controlo de exportação ao abrigo do Regulamento 2021/821.

---

## 7. Algoritmos Criptográficos — Referência Rápida

Para avaliação de conformidade com regulamentações de exportação de outras jurisdições (ex: EAR dos EUA):

| Algoritmo | Tipo | Comprimento de chave | Uso no kernel |
|-----------|------|---------------------|---------------|
| XChaCha20-Poly1305 | Cifra simétrica autenticada | 256 bits | Dados em repouso (SQLite) |
| Argon2id | Derivação de chave | — (parâmetros configuráveis) | Derivação de chaves de cifra |
| Ed25519 | Assinatura digital | 256 bits (curva) | Assinatura de documentos |
| RSA | Assinatura / verificação | 2048–4096 bits | Verificação de assinaturas JWT/RSA |
| ECDSA P-256 | Assinatura digital | 256 bits (curva) | Assinaturas de curva elíptica |
| ECDSA P-384 | Assinatura digital | 384 bits (curva) | Assinaturas de curva elíptica |
| SHA-256 / SHA-512 | Hash criptográfico | 256 / 512 bits output | Integridade de logs, hashes |

Todos os algoritmos são **públicos, padronizados e de uso generalizado** — não são algoritmos proprietários ou desenvolvidos para fins de restrição de exportação.

---

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial — Regulamento 2021/821, isenções, sanções, obrigações do integrador |
