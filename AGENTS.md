# AGENTS.md - normordis-kernel

Guia local para agentes de coding que trabalhem neste repositório.

Os agentes devem tratar este ficheiro como instruções de projeto. Quando houver
conflito com instruções explícitas do utilizador ou do sistema, essas instruções
prevalecem.

## 1. Comunicação e estilo

- Responder ao utilizador em pt-PT.
- Escrever documentação funcional e técnica em pt-PT.
- Manter identificadores de código em inglês, salvo quando o domínio exigir
  terminologia portuguesa ou siglas institucionais.
- Escrever código claro, pequeno e testável. Comentários devem explicar intenção,
  invariantes ou decisões pouco óbvias; não repetir o que o código já diz.
- Ser direto sobre lacunas reais, riscos arquiteturais, limitações atuais e testes
  que não foram executados.

## 2. Identidade do repositório

`normordis-kernel` é o kernel de plataforma do ecossistema NORMORDIS: Rust puro,
agnóstico de plataforma e de runtime, consumido pelas aplicações como biblioteca.
O kernel não deve conhecer UI, Tauri, Node.js, frameworks de apresentação ou uma
app host concreta.

Estrutura principal:

- `crates/kernel/core/*`: lógica de domínio, semântica institucional reutilizável,
  modelos e portos. Não deve depender de `infra`.
- `crates/kernel/support/*`: capacidades técnicas transversais e headless
  (erros, ids, clock, storage, crypto, logging, PDF, etc.).
- `crates/kernel/infra/*`: adaptadores concretos e materialização técnica
  (SQLite, filesystem, scanner, assinatura, serviços infra).
- `crates/domain/*`: domínios transversais reutilizáveis fora da fachada principal.
- `crates/runtime/*`: contexto, bootstrap e interoperabilidade de mini-apps.
- `crates/normordis-kernel/*`: fachada pública unificada.

Objetivo prático:

- manter apps leves, compostas e focadas no fluxo de uso;
- promover capacidades comuns para crates reutilizáveis apenas quando houver
  evidência de transversalidade;
- impedir que regras específicas de uma app contaminem bibliotecas partilhadas.

## 3. Ordem mínima de leitura

Antes de propor ou implementar alterações, ler apenas o contexto necessário,
por esta ordem:

1. `README.md` da raiz, quando relevante para o pedido.
2. `Cargo.toml` e `README.md` do crate afetado.
3. `MAN.md` do crate afetado quando a mudança toca contrato público,
   invariantes, persistência, erros, limites, integração ou comportamento
   observável.
4. Testes do crate, módulo ou fluxo afetado.
5. Implementação atual.

Se a alteração mexer em comportamento, persistência, shape de dados, API pública
ou wiring entre crates, a leitura do `MAN.md` e dos testes existentes é
obrigatória.

## 4. Regras de arquitetura

- Seguir Clean Architecture, Hexagonal Architecture, DDD, SOLID e Clean Code como
  heurísticas práticas, sem criar abstrações cerimoniais.
- As dependências devem fluir de fora para dentro: `infra` pode depender de
  `core`; `core` não pode depender de `infra`.
- Usar `core` para domínio puro, contratos e portos.
- Usar `support` para capacidades técnicas genéricas, sem regras de negócio de
  apps concretas.
- Usar `infra` para I/O, persistência, integrações externas e adaptadores.
- O wiring final entre `core`, `support`, `infra` e apps deve viver no
  runtime/bootstrap ou host apropriado.
- Preferir composição entre crates pequenos a expandir um crate com
  responsabilidades pouco relacionadas.
- Não criar novos componentes estruturais diretamente em `crates/` quando
  pertencerem ao kernel.
- Não migrar crates existentes em massa sem necessidade concreta e documentada.
- Se uma necessidade for específica de uma mini-app, implementar primeiro na app;
  promover para o kernel só quando a reutilização for clara.

## 5. Convenções por área

### Documental e PDF

- Pensar em definições, instâncias, eventos, artefactos, anexos, estado,
  proveniência, integridade e rastreabilidade.
- Não reduzir o domínio documental a exportação de ficheiros.
- Evitar workflows demasiado específicos sem justificar impacto transversal.
- Em PDF, preservar compatibilidade, acessibilidade e fidelidade quando já
  cobertas por testes.

### SQLite e persistência

- Bibliotecas SQLite genéricas não devem conter queries de domínio.
- Queries de domínio pertencem nos crates de persistência do respetivo domínio.
- Mudanças de schema, migração, índices, transações ou semântica de persistência
  exigem testes e atualização de documentação no mesmo conjunto de alterações.
- Não expor paths absolutos, queries sensíveis, dados pessoais ou segredos em
  erros públicos.

### Runtime e bootstrap

- Runtime/bootstrap deve orquestrar capacidades transversais, não absorver regras
  de negócio.
- O bootstrap deve devolver um runtime técnico coerente, previsível e testável.
- Evitar acoplamento a uma única app host.

### Segurança, auditoria e erros

- Preservar invariantes de auditoria append-only, cadeias de hash e verificações
  de integridade.
- Erros públicos devem ser tipados e estáveis.
- Ao introduzir um novo `ErrorCode` canónico, atualizar
  `crates/kernel/support/support-errors/ERRORS.json`.
- O catálogo de erros documenta códigos estáveis; não deve listar mensagens
  dinâmicas, detalhes internos, paths, queries, dados pessoais ou segredos.

## 6. Documentação obrigatória

- Novos componentes/crates devem incluir `README.md` e `MAN.md` junto ao
  respetivo `Cargo.toml`.
- `README.md` deve ser curto e orientado a objetivo/uso: responsabilidade,
  não responsabilidade e exemplo mínimo quando aplicável.
- `MAN.md` deve refletir a verdade atual do contrato público: como usar,
  invariantes, limites, integração, erros, limitações atuais e trabalhos futuros.
- Não usar `MAN.md` como changelog.
- Se uma alteração mudar persistência, comportamento observável, erros, workflow,
  API pública ou wiring, atualizar o `MAN.md` no mesmo change set.
- Se documentação e implementação divergirem, corrigir a divergência ou
  sinalizar explicitamente.

## 7. Implementação

- Não programar contra suposições quando o contrato atual puder ser lido.
- Não deixar `TODO`, placeholders, pseudocódigo ou APIs falsas em código ativo.
- Favorecer tipos explícitos, erros tipados e testes pequenos focados em
  comportamento.
- Preservar backward compatibility razoável nos crates transversais, salvo quando
  a mudança for deliberada, documentada e testada.
- Não introduzir estado global mutável sem necessidade clara.
- Não introduzir dependências de UI ou framework visual em `core` ou `support`.
- Para parsing, serialização e persistência, preferir APIs estruturadas a
  manipulação ad hoc de strings.
- Manter mudanças focadas no pedido; evitar refactors oportunistas sem benefício
  direto.

## 8. Qualidade e validação

Escolher os checks pelo risco da alteração:

- Mudança local e pequena: `cargo test -p <crate>` ou teste específico.
- Mudança em contratos, módulos transversais ou wiring: `cargo test --workspace`.
- Mudança de formatação ampla: `cargo fmt --all`.
- Antes de PR/release, preferir o pipeline documentado no `README.md`:
  `cargo check --workspace`, `cargo test --workspace`,
  `.\scripts\build-release.ps1` quando aplicável.

Quando não for possível correr um check relevante, dizer isso no resumo final e
explicar o risco residual.

## 9. Git e segurança do trabalho

- Antes de editar, verificar o estado do worktree quando a tarefa envolver commit,
  push, PR ou alterações amplas.
- Nunca reverter alterações do utilizador sem pedido explícito.
- Não usar comandos destrutivos como `git reset --hard`, limpeza recursiva ou
  checkout de ficheiros alterados sem autorização clara.
- Se o worktree tiver mudanças misturadas, separar o escopo ou perguntar antes de
  fazer staging/commit.
- Commits devem ter mensagem curta, concreta e em pt-PT ou inglês consistente com
  o histórico.

## 10. Raciocínio antes de alterar

Antes de mudanças não triviais, explicitar de forma curta:

- que crate/módulo é dono da alteração;
- se a necessidade é transversal ou específica de app;
- que contrato, `MAN.md` ou testes são a referência principal;
- que documentação precisa de atualização;
- que checks serão executados.

## 11. Fluxo de entrega

1. Ler o contexto mínimo relevante.
2. Confirmar ownership e abordagem estrutural.
3. Implementar a menor alteração correta.
4. Atualizar documentação relevante.
5. Executar testes/checks adequados.
6. Entregar com resumo curto, validação e riscos restantes.

## 12. Checklist rápida

Antes de terminar uma tarefa:

- O código respeita a direção das dependências?
- A mudança é transversal ou específica da app certa?
- `README.md`, `MAN.md`, erros e testes continuam alinhados?
- Persistência, comportamento observável, API pública e wiring foram considerados?
- Os testes relevantes foram executados ou o motivo para não os executar ficou
  claro?
- O worktree não contém alterações acidentais?
