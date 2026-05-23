# AGENTS.md - mini-apps-rusty

Instrucoes locais para agentes que trabalhem nesta workspace Rust.

## 1. Linguagem e comunicacao

- Responder ao utilizador em pt-pt.
- Escrever documentacao funcional e tecnica em pt-pt.
- O código deve ser anotado segundo as melhores práticas (google style) em pt-pt.  
- Manter identificadores de codigo em ingles, salvo quando o proprio dominio exigir outra convencao.
- Ser direto sobre lacunas reais, riscos arquiteturais e limitacoes atuais.

## 2. Enquadramento do repositorio

Este repositorio e uma workspace Rust para mini-apps locais.

- `crates/kernel/core/*`: semantica institucional/local reutilizavel, modelos de dominio e ports de dominio.
- `crates/kernel/support/*`: contratos, primitives e capacidades tecnicas transversais headless.
- `crates/kernel/infra/*`: adapters concretos e materializacao tecnica.
- `crates/support-*`: capacidades transversais, headless e reutilizaveis.
- `apps/*`: hosts e mini-apps concretas.
- A app de referencia atual e `apps/cli`.

Objetivo pratico:

- extrair capacidades comuns para crates reutilizaveis
- manter as apps leves, compostas e focadas no fluxo de uso
- evitar que regras especificas de uma mini-app contaminem bibliotecas transversais

## 3. Ordem minima de leitura

Antes de propor ou implementar alteracoes, ler apenas o contexto minimo relevante, por esta ordem:

1. `README.md` da raiz, se relevante para o pedido.
2. `README.md` do crate afetado.
3. `MAN.md` do crate afetado, quando o pedido mexe em contrato publico, invariantes, persistencia, erros, limites ou integracao.
4. Testes do crate ou da app afetada.
5. Implementacao atual.

Se a alteracao mexer em comportamento, persistencia, shape de dados, API publica ou wiring entre crates, a leitura do `MAN.md` e dos testes existentes e obrigatoria.

## 4. Regras de arquitetura

- Novos componentes estruturais do Mini-Kernel RS devem nascer em `crates/kernel/core`, `crates/kernel/support` ou `crates/kernel/infra`.
- Usar `crates/kernel/core` para semantica institucional/local reutilizavel, modelos de dominio e ports de dominio.
- Usar `crates/kernel/support` para contratos, primitives e capacidades tecnicas transversais headless.
- Usar `crates/kernel/infra` para adapters concretos, como SQLite, filesystem ou outros mecanismos externos.
- Nao criar novos componentes estruturais diretamente em `crates/` quando pertencerem ao Mini-Kernel RS.
- Nao migrar crates existentes em massa sem necessidade concreta e documentada.
- O wiring final entre `core`, `support`, `infra` e apps deve viver no runtime/bootstrap ou host apropriado.
- Tratar `crates/support-*` como bibliotecas transversais e agnosticas da mini-app consumidora.
- Nao colocar regras especificas de `miniapps-cli` dentro de crates `support-*`, a menos que a abstracao seja claramente transversal.
- Preferir composicao entre crates pequenos em vez de expandir um crate com responsabilidades pouco relacionadas.
- Preservar a separacao entre:
  - modelo/contrato transversal
  - persistencia transversal
  - bootstrap/runtime
  - host/app concreta
- Se a necessidade for especifica de uma mini-app, implementar primeiro na app. So promover para `crates/` quando houver evidencia de reutilizacao.

## 5. Convencoes por area

### `crates/support-documental*`

- Pensar em definicoes, instancias, eventos, artefactos, anexos, estado e proveniencia.
- Nao reduzir o problema documental a exportacao de ficheiros.
- Evitar assumir workflows demasiado especificos sem justificar impacto transversal.

### `crates/support-sqlite` e crates persistentes

- Manter a biblioteca de SQLite generica.
- Queries de dominio pertencem nos crates de persistencia do respetivo dominio, nao em `support-sqlite`.
- Sempre que houver mudancas de schema ou persistencia, atualizar testes e documentacao no mesmo conjunto de alteracoes.

### `crates/support-miniapp-runtime` e `crates/support-app-bootstrap`

- Devem orquestrar capacidades transversais, nao absorver regras de negocio da app.
- `bootstrap` deve devolver um runtime tecnico coerente e previsivel.
- Evitar acoplamento desnecessario a uma unica app host.

### `apps/*`

- Hosts podem compor defaults, comandos, fluxos e naming especificos.
- A app deve consumir contratos e crates transversais existentes antes de criar logica paralela.

## 6. Documentacao obrigatoria

- Todos os novos componentes/crates devem incluir `README.md` e `MAN.md` junto ao respetivo `Cargo.toml`.
- Qualquer alteracao relevante num crate deve manter `README.md`, `MAN.md` e testes alinhados com a realidade implementada.
- `README.md` deve ser curto e orientado a objetivo/uso, incluindo responsabilidade, nao responsabilidade e exemplo minimo quando aplicavel.
- `MAN.md` deve refletir a verdade atual do contrato publico, como usar, invariantes, limites, integracao, limitacoes atuais e ToDo de implementacoes futuras.
- Sempre que um novo `ErrorCode` canonico for introduzido no Mini-Kernel RS, atualizar `crates/kernel/support/support-errors/ERRORS.json`.
- O catalogo de erros documenta codigos estaveis; nao deve listar mensagens dinamicas, detalhes internos, paths, queries, dados pessoais ou segredos.
- Se uma alteracao mudar persistencia, comportamento observavel, erros, workflow, ou API publica, atualizar o `MAN.md` no mesmo change set.
- Nao usar `MAN.md` como changelog.

## 7. Regras de implementacao

- Nao programar contra suposicoes quando o contrato atual puder ser lido primeiro.
- Nao deixar `TODO`, placeholders, pseudocodigo ou APIs falsas em codigo que se pretende ativo.
- Favorecer tipos explicitos, erros tipados e testes pequenos focados no comportamento.
- Preservar backward compatibility razoavel nos crates transversais, salvo quando a mudanca for deliberada e documentada.
- Se encontrares um desvio entre documentacao e implementacao, corrigir ou sinalizar explicitamente.

## 8. Convencoes de qualidade

- Correr testes relevantes apos alteracoes, idealmente `cargo test` do crate afetado e do fluxo impactado.
- Quando o pedido toca na integracao transversal, preferir validar com `cargo test` na workspace.
- Nao introduzir estado global mutavel sem necessidade clara.
- Nao introduzir dependencia de UI/framework visual em crates `support-*`.
- Quando uma abstracao ainda so existe em memoria mas o projeto sugere persistencia real, sinalizar isso explicitamente.

## 9. Raciocinio antes de alterar

Antes de fazer mudancas nao triviais, explicitar de forma curta:

- que crate e dono da alteracao
- se a necessidade e transversal ou especifica da app
- que contrato ou testes sao a referencia principal
- que documentacao precisa de ser atualizada

## 10. Fluxo de entrega

1. Ler o contexto minimo relevante.
2. Resumir a abordagem estrutural.
3. Implementar.
4. Atualizar documentacao relevante.
5. Executar testes/checks adequados.
6. Entregar com resumo curto, riscos e validacao.

## 11. Checklist rapida por modulo

Quando alterares um crate ou app:

- Ler `README.md` e `MAN.md` relevantes antes de editar.
- Confirmar se a mudanca e transversal ou especifica da mini-app.
- Verificar impacto em comportamento, persistencia, wiring, erros e testes.
- Atualizar `MAN.md` se a verdade funcional/tecnica mudou.
- Atualizar testes no mesmo conjunto de alteracoes.
- Garantir que a app host continua fina e que as libs continuam reutilizaveis.
