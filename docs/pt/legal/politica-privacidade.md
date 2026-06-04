---
title: "Normordis Kernel — Política de Privacidade"
type: legal
framework: RGPD
status: draft
version: 0.1.0
date: 2026-06-03
lang: pt
audience: [executive, technical, auditor]
approved_by: ""
related:
  - docs/pt/legal/registo-actividades-tratamento.md
  - docs/pt/legal/declaracao-conformidade.md
  - docs/pt/compliance/rgpd.md
---

# Normordis Kernel — Política de Privacidade

## Nota Importante — Leia Antes de Continuar

O Normordis Kernel é uma **plataforma de software** destinada a organizações da Administração Pública — não é uma aplicação de utilizador final. Se está a utilizar um serviço ou sistema que usa o Normordis Kernel como componente interno, **o responsável pelo tratamento dos seus dados é a organização que disponibiliza esse serviço**, não os autores do kernel.

Este documento serve dois propósitos:

1. **Política de plataforma** — descreve como o kernel trata dados pessoais enquanto componente técnico.
2. **Modelo base** — fornece às organizações integradoras uma estrutura de política de privacidade adaptável ao seu contexto, marcando com `[integrador]` os campos a personalizar.

---

## 1. Identidade do Responsável pelo Tratamento

O Normordis Kernel actua tipicamente como **subcontratante** (*data processor*): processa dados pessoais por conta e sob instrução das organizações que o integram. O **responsável pelo tratamento** (*data controller*) é a organização que disponibiliza o serviço ao utilizador.

| Campo | Kernel (subcontratante) | Organização integradora (responsável) |
|-------|------------------------|---------------------------------------|
| Designação | Normordis Kernel | `[integrador]` |
| Contacto | carloscanutocosta@gmail.com | `[integrador]` |
| Encarregado de Protecção de Dados (EPD) | — | `[integrador — se aplicável]` |
| Website / portal | — | `[integrador]` |

Se não souber quem é o responsável pelo tratamento dos seus dados, contacte a organização que lhe fornece o serviço.

---

## 2. Âmbito desta Política

Esta política aplica-se ao tratamento de dados pessoais efectuado pelo Normordis Kernel no contexto do seu funcionamento como componente de software. Cobre as seguintes actividades de tratamento (detalhadas no [Registo de Actividades de Tratamento](registo-actividades-tratamento.md)):

| Actividade | Descrição resumida |
|------------|-------------------|
| AT-01 — Gestão de utilizadores | Dados de identidade e atribuições de colaboradores |
| AT-02 — Auditoria de operações | Registo pseudonimizado de operações de controlo |
| AT-03 — Estrutura organizacional | Posições e cargos (com identificador opaco) |
| AT-04 — Ciclo de vida documental | Autores e destinatários de documentos |
| AT-05 — Telemetria de uso | Eventos de utilização pseudonimizados |
| AT-06 — Logs de diagnóstico | Logs técnicos com identificador opaco |
| AT-07 — Validação de identificadores | NIF, IBAN, e-mail — apenas em memória, sem persistência |

---

## 3. Que Dados Tratamos e Porquê

### 3.1 Dados de identidade e função (AT-01)

**Dados:** nome, endereço de correio electrónico, NIF, cargo, função, datas de atribuição.

**Porquê:** para identificar quem pode usar o sistema, que funções desempenha e quais as responsabilidades associadas. Sem estes dados, não é possível atribuir responsabilidades, gerar evidências de controlo ou rastrear operações a um responsável.

**Base jurídica:** execução de contrato de trabalho ou função pública (Art. 6.1.b/e RGPD); obrigação legal de controlo interno (Art. 6.1.c).

### 3.2 Registo de auditoria (AT-02)

**Dados:** identificador opaco do actor (`ActorId` — um código sem significado por si só), data e hora da operação, tipo de controlo, resultado.

**Porquê:** por obrigação legal de controlo interno e para permitir que organismos fiscalizadores (Tribunal de Contas) verifiquem a legalidade e regularidade das operações. Este registo é **pseudonimizado por design**: o `ActorId` não identifica directamente nenhuma pessoa — a ligação entre o código e o nome do colaborador existe apenas nos dados de AT-01.

**Base jurídica:** obrigação legal de controlo interno (Art. 6.1.c — Lei 98/97, COSO, INTOSAI GOV 9100).

**Nota de imutabilidade:** os registos de auditoria são **imutáveis por razões legais** — não podem ser alterados ou apagados sem comprometer a integridade da cadeia de evidência exigida pelo Tribunal de Contas. O apagamento de dados pessoais (AT-01) não elimina os registos de auditoria; elimina a ligação entre o `ActorId` e a pessoa — tornando o registo anónimo mas preservando a evidência de controlo.

### 3.3 Estrutura organizacional (AT-03)

**Dados:** identificador opaco de quem ocupa cada posição, período de ocupação.

**Porquê:** para modelar a hierarquia de autoridade, as substituições legais e as responsabilidades de supervisão — elementos exigidos pelos quadros de controlo interno.

**Base jurídica:** exercício de funções públicas (Art. 6.1.e); obrigação legal (Art. 6.1.c).

### 3.4 Documentos institucionais (AT-04)

**Dados:** identidade do autor, destinatários, signatários e outros intervenientes em documentos produzidos ou recebidos.

**Porquê:** para gerir o ciclo de vida de documentos institucionais, garantir a rastreabilidade documental e cumprir as obrigações arquivísticas.

**Base jurídica:** exercício de funções públicas (Art. 6.1.e); obrigação legal de gestão de arquivo (Art. 6.1.c — DL 447/88).

**Nota:** o conteúdo dos documentos pode incluir dados pessoais de terceiros (cidadãos, fornecedores). A base jurídica aplicável a esses dados depende do tipo de documento e é avaliada pela organização integradora.

### 3.5 Telemetria e diagnóstico (AT-05 e AT-06)

**Dados:** identificador opaco do utilizador, tipo de operação, timestamps. Logs técnicos podem conter identificadores de sessão.

**Porquê:** para monitorizar o desempenho do sistema, detectar anomalias e diagnosticar problemas técnicos. Os dados são usados exclusivamente para fins operacionais internos.

**Base jurídica:** interesse legítimo na operação técnica segura do sistema (Art. 6.1.f).

### 3.6 Validação de identificadores (AT-07)

**Dados:** NIF, IBAN, endereço de correio electrónico — processados **apenas em memória** durante a validação.

**Porquê:** para verificar se os dados introduzidos são formalmente válidos antes de os persistir.

**Nota:** estes dados não são armazenados pelo módulo de validação. Não existe registo de validações.

---

## 4. Com Quem Partilhamos os Dados

O Normordis Kernel **não transmite dados pessoais para terceiros**. Opera em modo local — toda a persistência é feita em SQLite no servidor ou dispositivo da organização, sem envio de dados para serviços externos, cloud ou outros sistemas.

Excepções planeadas (integração futura com serviços portugueses):
- **iAP/ARTE** — validação em tempo real de NIF/NIPC (quando activada pela organização)
- **CMD/Autenticação.Gov** — autenticação forte e assinatura qualificada (quando activada)

Quando estas integrações forem activadas, os dados transmitidos e as bases jurídicas serão documentados pelo integrador no seu RAT.

---

## 5. Quanto Tempo Conservamos os Dados

| Actividade | Prazo indicativo | Fundamento |
|------------|-----------------|------------|
| AT-01 — Utilizadores | Duração da relação + `[integrador]` | Legislação laboral/estatuto |
| AT-02 — Auditoria | Mínimo 10 anos (recomendado) | Prescrição de responsabilidade financeira (TC) |
| AT-03 — Estrutura org. | Duração do mandato + `[integrador]` | Responsabilidade de cargo |
| AT-04 — Documentos | Conforme tabela de selecção MEF-DGLAB | DL 447/88, Portaria 412/2001 |
| AT-05 — Telemetria | ≤ 90 dias | Minimização de dados |
| AT-06 — Logs | ≤ 30 dias | Minimização de dados |
| AT-07 — Validação | Não aplicável (só em memória) | — |

Os prazos definitivos são definidos pela organização integradora em conformidade com a sua tabela de selecção arquivística e legislação sectorial.

---

## 6. Os Seus Direitos

Nos termos do RGPD, tem os seguintes direitos relativamente aos seus dados pessoais:

| Direito | Artigo RGPD | Como exercer no contexto do kernel |
|---------|------------|-------------------------------------|
| **Acesso** | Art. 15.º | Solicitar à organização integradora exportação dos seus dados |
| **Rectificação** | Art. 16.º | Solicitar correcção de dados incorrectos à organização |
| **Apagamento** | Art. 17.º | Solicitar à organização — note que o registo de auditoria (AT-02) é imutável por lei; o apagamento traduz-se em anonimização do identificador |
| **Limitação** | Art. 18.º | Solicitar à organização integradora |
| **Portabilidade** | Art. 20.º | Solicitar exportação dos dados em formato aberto à organização |
| **Oposição** | Art. 21.º | Solicitar à organização integradora |

**Como exercer:** contacte directamente a organização que disponibiliza o serviço — ela é o responsável pelo tratamento. Se não souber como contactá-la, consulte o portal ou sistema que utiliza.

**Direito de reclamação:** tem o direito de apresentar reclamação à autoridade de controlo competente:

> **Comissão Nacional de Protecção de Dados (CNPD)**
> Rua de São Bento, 148-3.º, 1200-821 Lisboa
> geral@cnpd.pt | www.cnpd.pt

---

## 7. Segurança dos Dados

O Normordis Kernel implementa as seguintes medidas técnicas de segurança para proteger os dados pessoais:

| Medida | Implementação |
|--------|---------------|
| **Cifra em repouso** | SQLite com `XChaCha20-Poly1305` — dados ilegíveis sem chave |
| **Pseudonimização** | `ActorId` opaco nos registos de auditoria — identidade separada dos eventos |
| **Gestão de segredos** | DPAPI (Windows) / fallback portável — chaves nunca em logs |
| **Integridade** | Hash encadeado no registo de auditoria — adulteração detectável |
| **Minimização** | AT-07 sem persistência; AT-05/06 com prazos curtos |
| **Controlo de acesso** | Políticas de acesso por recurso (`core-security`) |

---

## 8. Decisões Automatizadas

O Normordis Kernel **não toma decisões automatizadas** com base em dados pessoais que produzam efeitos jurídicos ou que afectem significativamente os titulares (Art. 22.º RGPD). É uma plataforma de suporte — qualquer decisão é tomada pelos sistemas ou pessoas que o utilizam.

Se uma aplicação construída sobre o kernel incorporar lógica de decisão automatizada, o integrador deve avaliar os requisitos do Art. 22.º RGPD e do AI Act no contexto dessa aplicação.

---

## 9. Transferências Internacionais

O Normordis Kernel **não efectua transferências de dados pessoais para países fora do Espaço Económico Europeu (EEE)**. Toda a persistência é local.

---

## 10. Alterações a Esta Política

Esta política pode ser actualizada para reflectir alterações ao kernel, ao quadro legal aplicável ou às práticas de tratamento de dados. A versão e data de cada revisão estão registadas no histórico abaixo. A versão em vigor é sempre a disponível no repositório do projecto.

---

## 11. Contacto

Para questões relacionadas com esta política de privacidade enquanto política de plataforma:

**Normordis Kernel — Responsável técnico**
carloscanutocosta@gmail.com

Para questões relacionadas com os seus dados pessoais num serviço concreto que utilize o kernel, contacte a organização responsável por esse serviço.

---

## Histórico de Revisões

| Versão | Data | Autor | Alteração |
|--------|------|-------|-----------|
| 0.1.0 | 2026-06-03 | carloscanutocosta | Versão inicial — política de plataforma e modelo para integradores |
