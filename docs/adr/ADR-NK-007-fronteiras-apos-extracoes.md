# ADR-NK-007 — Fronteiras do kernel após as extrações e congelamento do repositório

Estado: Aceite  
Âmbito: normordis-kernel · Arquitetura · Fronteiras entre repositórios  
Autor: Carlos Costa  
Data: 2026-10-18  
Versão: v1.0.0  
Origem: documento de trabalho «Normordis — decisões de arquitetura e primeiro marco» (18 de outubro de 2026)

---

## Contexto

O `normordis-kernel` começou por ser o único projeto do ecossistema e continha
tudo: interface, formatos documentais, composição PDF e domínio institucional.
À medida que cada componente mostrou identidade própria, foi autonomizado:

| Componente | Origem | Autonomizado como | Razão |
|---|---|---|---|
| Interface | kernel | `normordis-core-ui` | Trazia dependências Node.js para dentro do kernel |
| Composição PDF | kernel | `Normordis/normordis-pdf` | Carácter próprio e uso independente; publicado no crates.io |
| Formatos documentais | via `normordis-pdf` | `normordis-formats` | Contrato documental, não implementação de um motor |

Extrair um componente não o retirou do sítio onde nasceu. A análise de 18 de
outubro de 2026 (commit `80f65f2`, v0.3.0) encontrou os seguintes resíduos:

- **Cópia interna de `normordis-pdf` 2.5.1** em `crates/kernel/infra/pdf/normordis-pdf`,
  imposta a todo o workspace por `[patch.crates-io]`, enquanto o repositório
  canónico `Normordis/normordis-pdf` está na 3.0.1 (PDF/A-4f + PDF/UA-2 por
  omissão, veraPDF em CI). São duas fontes de verdade divergentes.
- **Via Typst** (`render-typst`, `pdf-pipeline`, `documentos-pdf`,
  `support-typst-template`, `support-pdf`) e `support-docx-to-typst`: um segundo
  motor de produção PDF.
- **Tipos NDF/NDT** em `core-documental` (`ndf_core.rs`, `NdfArchive`, templates
  NDT) e em `core-doc-types` (`TemplateRef`, `EngineRef`).

Na mesma data foram tomadas decisões de arquitetura que fixam o lugar de cada
componente:

- ODF (OASIS, ODF 1.3 e seguintes) é o formato do conteúdo editável; não são
  desenvolvidos os formatos documentais próprios NDF e NRTF.
- Documentos completos são editados no Collabora Online, integrado pelo
  `core-editor` através de WOPI.
- Há dois percursos de produção PDF com um motor de composição comum:
  documentos editados (OTT → ODT → LibreOffice) e documentos gerados
  (dados → template NDT → `normordis-pdf`). O NDT mantém-se apenas como formato
  de template de entrada, não como contrato documental.
- A base das aplicações é composta por `kernel-institutional`, `core-ui`,
  `core-editor`, `core-formats` e `core-pdf`, com `normordis-pdf` como motor.
- O Typst fica congelado: suporta apenas PDF/UA-1, incompatível com PDF/A-4,
  e não cumpre o alvo PDF/A-4 + PDF/UA-2.

O repositório não tem consumidores externos conhecidos, o que permite reiniciar
sem custos de compatibilidade.

---

## Decisão

### 1. Congelar este repositório

O `normordis-kernel` fica congelado no estado do commit `80f65f2` (v0.3.0) como
referência histórica e fonte de reaproveitamento:

- sem novas funcionalidades;
- apenas correções de documentação e de segurança até ao arquivamento;
- arquivado no GitHub quando o novo repositório do kernel substituir a sua função.

### 2. Novo repositório do kernel

O conteúdo institucional transita para um novo repositório público
(nome definitivo a fixar: `kernel-institutional` ou equivalente), criado desde o
início com licença EUPL-1.2, política de segurança, proteção do ramo principal e
verificações de CI.

### 3. Destino de cada área

| Área | Linhas (aprox.) | Destino |
|---|---|---|
| `kernel/core/*` | 32 700 | Novo kernel |
| `kernel/support/*`, exceto PDF/Typst | 6 700 | Novo kernel |
| `kernel/infra/*`, exceto PDF | 34 700 | Novo kernel (assinatura CC/CMD/PKCS#11 servirá o PAdES do `core-pdf`) |
| `domain/*`, `runtime/*`, fachada, `spec-conformance` | 4 400 | Novo kernel |
| `normordis-spec` | — | Novo kernel; schemas NDF revistos, NDT mantido como template |
| Cópia de `normordis-pdf` e `[patch.crates-io]` | 20 700 | Não transitam; usa-se a versão publicada de `Normordis/normordis-pdf` |
| Via Typst e `support-docx-to-typst` | 1 900 | Não transitam; preservadas apenas neste repositório |

### 4. Remodelações na transição

- A semântica arquivística de `ndf_core.rs` (classificação, prazos de
  conservação, forma de contagem, destino final, consulta à Lista Consolidada)
  é preservada como modelo de metadados institucionais, com nome que não remeta
  para o NDF.
- `NdfArchive` generaliza-se para arquivo write-once de representações
  (ODT, PDF).
- `DocumentPackage`, `TemplateRef` e `EngineRef` alinham-se com OTT/ODT, NDT e o
  `ndfpkg`.

### 5. Regra para extrações futuras

Um componente autonomizado passa a ser consumido como dependência publicada ou
versionada. A extração só termina quando a cópia original é removida do
repositório de onde saiu. Não se mantêm cópias internas nem substituições por
`[patch]`.

---

## Consequências

### Positivas

- Uma única fonte de verdade por componente.
- O novo kernel fica restrito ao domínio institucional, com cerca de 78% do
  código atual reaproveitado.
- O histórico das extrações fica registado e explica a forma dos repositórios
  a quem chegue ao projeto.
- Os resíduos passam a uma lista finita e verificável.

### Negativas

- O histórico Git do novo repositório começa no reinício; o histórico anterior
  fica neste repositório arquivado.
- A transição exige rever dependências internas entre crates ao mover
  `core-documental` e `core-doc-types`.

---

## Alternativas consideradas

### 1. Limpar este repositório no lugar

Manteria o histórico contínuo, mas arrastaria os resíduos e a configuração
pensada para um repositório privado. Sem consumidores externos, o custo de
compatibilidade de um novo repositório é nulo. Rejeitada.

### 2. Manter a cópia de `normordis-pdf` no kernel

Preservaria a compilação atual sem alterações, mas mantém duas fontes de
verdade, com a cópia já uma versão maior atrás da publicada (2.5.1 face a 3.0.1). Rejeitada.

---

## Notas sobre ADR anteriores

- **ADR-NK-001:** a afirmação de que o kernel é privado deixou de ser verdadeira;
  o repositório é público.
- **ADR-NK-002:** o diagrama `core → support → infra` sugere que `support`
  depende de `infra`. O grafo real de dependências (verificado em 2026-10-18)
  é o inverso — `infra` depende de `support` e de `core` — e não tem violações
  de camada. O diagrama deve ser lido como ordem de abstração, não de dependência.
