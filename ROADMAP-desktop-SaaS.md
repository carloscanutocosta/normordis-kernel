# ROADMAP desktop + SaaS

## Objetivo

Preparar o `normordis-kernel` para ser consumido por aplicações NORMORDIS em dois
runtimes principais:

- desktop: Rust + Tauri 2 + SQLite/local storage;
- SaaS/web: Rust HTTP server + PostgreSQL/MongoDB/object storage.

O mesmo domínio, os mesmos contratos e os mesmos casos de uso devem servir ambos
os modelos. Tauri commands e HTTP handlers devem ser apenas adapters de entrada.

Este roadmap assume que os contratos canónicos estão a ser implementados em
`normordis-spec` e que o kernel deve alinhar-se com esses contratos sem absorver
detalhes de UI, HTTP, Tauri, browser, cloud provider ou sistema operativo.

O documento é deliberadamente ambicioso. Não implica executar tudo de uma vez nem
antecipar complexidade sem necessidade. A função principal é dar uma linha de ação
que permita evoluir o kernel sem fechar portas e sem obrigar a refactorizações
complexas quando o alvo SaaS/API ficar mais próximo.

## Objetivo estratégico

O objetivo estratégico é manter:

- um único `normordis-kernel`;
- um único `normordis-core-ui`;
- múltiplos hosts/adapters finos para desktop, SaaS/web e futuros runtimes;
- o mesmo código fonte a correr em Windows e Linux sempre que a diferença de
  plataforma não seja a própria funcionalidade.

O que deve ser único:

- domínio institucional;
- contratos `normordis-spec`;
- DTOs canónicos;
- casos de uso e regras transversais;
- ports e erros públicos;
- componentes, shells e primitives visuais do `normordis-core-ui`;
- UX partilhada quando o fluxo funcional for o mesmo.

O que pode variar por runtime:

- transporte: Tauri commands, HTTP, jobs ou mensageria;
- persistência: SQLite local, PostgreSQL, MongoDB ou outro backend;
- storage de ficheiros: filesystem local, object storage, MinIO/S3;
- autenticação: local/session desktop, OIDC/OAuth2/JWT/cookies em SaaS;
- tenancy, deployment, configuração e gestão de secrets;
- linguagem de serviços adjacentes, quando apropriado, desde que os contratos e as
  regras institucionais permaneçam no kernel/spec.

`normordis-core-ui` já cumpre o papel de biblioteca visual agnóstica: fornece
componentes e shells reutilizáveis, sem regras de negócio nem conhecimento das
apps consumidoras. A variabilidade desktop/SaaS deve ficar fora do core-ui, em
clients/adapters e bootstrap da app host.

As apps desktop Rust + Tauri já demonstraram que é possível correr em Windows e
Linux sem alteração ao código fonte, mantendo a UX baseada em `normordis-core-ui`
igual entre plataformas. Essa propriedade deve ser tratada como objetivo de
qualidade e sinal de fronteiras arquiteturais saudáveis.

Regra orientadora:

```text
um kernel comum + um core-ui comum + adapters por runtime
```

Evitar o objetivo perigoso:

```text
um único crate que conhece todos os runtimes concretos
```

## Rust, Go e fronteiras tecnológicas

O ecossistema NORMORDIS deve usar a melhor ferramenta para cada caso de uso, mas
sem quebrar a fronteira do kernel.

Regra base:

```text
Rust para o kernel institucional.
Go para serviços, workers e ferramentas adjacentes quando fizer sentido.
```

O `normordis-kernel` deve permanecer uma biblioteca Rust: domínio, invariantes,
ports, DTOs, use cases críticos, erros públicos e integração com contratos
`normordis-spec`. Introduzir Go dentro do kernel implicaria FFI/bindings, builds
mais complexos, packaging multiplataforma mais frágil e maior risco de duplicação
de regras de negócio.

Go é adequado como tecnologia de ecossistema em componentes ao lado do kernel,
por exemplo:

- gateways HTTP simples;
- workers de ingestão;
- serviços de sincronização;
- agentes locais;
- CLIs operacionais;
- watchers de filesystem;
- pipelines de import/export;
- bridges com sistemas externos;
- health checks, probes e ferramentas DevOps;
- serviços cloud pequenos e fáceis de distribuir.

Fronteira saudável:

```text
normordis-kernel (Rust)
  domínio, invariantes, contratos, use cases críticos

serviços/ferramentas Go
  operação, integração, workers, gateways, automação

normordis-spec
  contrato comum entre Rust, Go, TypeScript e futuros consumidores
```

Se um serviço Go começar a conter regra institucional, essa regra deve ser
promovida para o kernel ou formalizada em `normordis-spec`. O componente Go deve
ficar como adapter, host, worker ou ferramenta operacional.

## Princípios

- O domínio não conhece runtime, sistema operativo, base de dados, protocolo HTTP,
  Tauri ou browser.
- Regras institucionais vivem em `core`, `domain` ou application services
  headless, nunca em handlers Tauri/HTTP.
- Persistência, storage, autenticação, configuração e transporte são adapters.
- Desktop e SaaS chamam os mesmos casos de uso quando a regra de negócio é comum.
- Estado institucional nunca pertence ao frontend.
- `normordis-spec` é a referência para DTOs, schemas, erros públicos e cenários de
  conformidade.
- Serviços Go podem existir no ecossistema, mas não devem ser fonte autoritativa
  de regras institucionais transversais.

## Estado atual

- O kernel compila em Windows e Linux.
- As `normordis-apps` Rust + Tauri já compilaram e correram em Surface Go 1 com
  Ubuntu 26.04, dando evidência prática de portabilidade desktop Linux.
- A UX baseada em `normordis-core-ui` mantém paridade visual e funcional entre
  Windows e Linux.
- A execução desktop Windows/Linux sem alteração ao código fonte é uma propriedade
  a preservar em novas funcionalidades.
- `core-*` concentra domínio, modelos e ports.
- `infra-*` contém vários adapters concretos, sobretudo SQLite e serviços locais.
- `crates/runtime/miniapp-runtime` já faz orquestração headless mínima para
  criação documental e auditoria, mas ainda não é um runtime completo para
  desktop/SaaS.
- `spec-conformance` valida parte dos contratos de `normordis-spec`.
- Existem tipos `*Request`, `*Response`, filtros e paginação em alguns crates, mas
  ainda não há uma camada DTO canónica, consistente e transversal para Tauri/HTTP.
- Falta uma camada pública, estável e abrangente de application services/DTOs para
  ser chamada indistintamente por Tauri commands e HTTP handlers.
- Ainda não existe um conjunto formal de application services headless por domínio
  com use cases públicos, estáveis e preparados para desktop/SaaS.
- Ainda não existe um `KernelRuntime`/`NormordisRuntime` com builder agnóstico para
  composição explícita de ports, stores, clock, ids, configuração, identidade,
  tenant e serviços.
- Ainda não existem adapters de entrada Tauri/HTTP finos e exemplares que provem
  a chamada dos mesmos use cases a partir de desktop e web.

## Leitura arquitetural do trabalho existente

O trabalho existente facilita a abstração desktop/web. A separação atual entre
`core`, `support`, `infra`, `runtime` e `spec-conformance` já cria o terreno certo
para que uma app Tauri e uma app SaaS consumam o mesmo núcleo sem duplicar regras
de negócio.

Pontos que já ajudam:

- `core-*` concentra domínio, modelos, invariantes e ports.
- `support-*` reúne capacidades técnicas genéricas e headless.
- `infra-*` já materializa adapters concretos sem contaminar o domínio.
- `core-documental` já expõe ports como `DocumentCustodyRepository`,
  `DocumentEventLog` e `AttachmentStore`.
- `miniapp-runtime` já demonstra uma primeira orquestração headless sobre
  documental, utilizador, orgânica e auditoria.
- `normordis-spec` e `spec-conformance` criam uma base forte para contratos comuns
  entre desktop, HTTP APIs e futuros clients.

Isto significa que não é necessário recomeçar nem redesenhar tudo. O próximo salto
deve ser incremental: formalizar DTOs, use cases e runtime builder agnósticos,
guiados pelos contratos da spec.

## Cuidados de planeamento futuro

O maior risco é transformar a camada de runtime numa mistura de domínio,
application services e adapters concretos. Para evitar isso:

- Não colocar dependências de `tauri`, `axum`, `actix`, browser, PostgreSQL,
  MongoDB ou cloud provider em crates de domínio ou application services
  headless.
- Não transformar `miniapp-runtime` num saco de adapters; se evoluir, deve
  evoluir como orquestração/use cases agnósticos.
- Não duplicar regra institucional em Tauri commands e HTTP handlers.
- Introduzir DTOs canónicos antes de criar muitos endpoints/commands.
- Pensar transações cedo, sobretudo para documento + evento + auditoria + anexos
  + numeração.
- Pensar multi-tenancy antes do SaaS real; acrescentar `TenantContext` tarde pode
  obrigar a rever queries, índices, auditoria, numeração e storage.
- Evitar paths reais e detalhes de filesystem no domínio; usar ports como
  `AttachmentStore`, `FileStore` ou `ObjectStore`.
- Separar estado institucional de estado de UI e de estado de trabalho.
- Decidir com cuidado a fronteira sync/async dos use cases e ports, porque Tauri e
  servidores web tendem a operar em runtimes async, enquanto alguns ports atuais
  são síncronos.
- Não aceitar divergências Windows/Linux no código fonte como solução normal. Se
  uma diferença de plataforma for inevitável, deve ficar isolada num adapter,
  documentada e validada.
- Não usar Go para contornar lacunas do kernel. Se a lacuna for uma regra de
  domínio ou contrato institucional, corrigir primeiro no kernel/spec.

Regra prática: desktop e SaaS podem divergir em adapters, bootstrap,
autenticação, storage e transporte; não devem divergir nas regras de negócio
transversais.

## Frentes transversais para estabilidade SaaS

Estas frentes não precisam de ser implementadas todas no início. Devem, porém,
orientar decisões estruturais para que o kernel continue estável, manutenível e
preparado para servir como backend API/SaaS.

### Versionamento e compatibilidade

Objetivo: garantir que apps consumidoras não quebram por alterações invisíveis.

Planear:

- política SemVer para crates públicos;
- versionamento de `normordis-spec`;
- versionamento de DTOs e APIs (`v1`, `v2`, etc.);
- política de depreciação;
- changelog de breaking changes;
- compatibilidade de erros públicos;
- compatibilidade de migrações de dados.

### API pública e OpenAPI

Objetivo: tornar o kernel consumível por APIs documentadas e testáveis.

Planear:

- OpenAPI para endpoints SaaS/web;
- schemas JSON alinhados com `normordis-spec`;
- exemplos de requests/responses;
- envelopes de erro públicos;
- paginação, filtros e ordenação consistentes;
- requisitos de autenticação/autorização por endpoint;
- testes de contrato API.

### Segurança aplicacional

Objetivo: tornar o modo SaaS seguro por desenho, não por remendo tardio.

Planear:

- autenticação;
- autorização;
- RBAC/ABAC;
- scopes/permissões;
- isolamento por tenant;
- rate limiting;
- proteção contra enumeração de dados;
- threat model;
- gestão de secrets;
- política de logs sem dados sensíveis.

### Conformidade AP, privacidade e acessibilidade

Objetivo: garantir que as decisões técnicas não dificultam requisitos típicos da
Administração Pública.

Planear:

- minimização de dados pessoais;
- classificação de dados sensíveis;
- rastreabilidade de acessos;
- retenção e eliminação conforme política aplicável;
- exportação de evidência;
- acessibilidade da UX consumidora via `normordis-core-ui`;
- compatibilidade com arquivo, integridade e prova documental;
- documentação de decisões com impacto legal/institucional.

### Multi-tenancy como fundação

Objetivo: evitar que o isolamento SaaS seja acrescentado tarde demais.

Planear impacto de `TenantContext` em:

- repositories;
- auditoria;
- numeração;
- documentos e anexos;
- exports;
- logs e métricas;
- backups/restores;
- permissões;
- quotas e limites.

### Migrações e evolução de dados

Objetivo: permitir que o backend viva anos sem bloquear evolução funcional.

Planear:

- schema migrations;
- estratégia forward-only ou rollback;
- migrações de dados documentais;
- versionamento de templates e documentos;
- compatibilidade entre versões de app/API;
- jobs de manutenção;
- validação pós-migração.

### Observabilidade operacional

Objetivo: tornar o mesmo use case observável em desktop, web e SaaS.

Planear:

- logs estruturados;
- tracing;
- métricas;
- correlation id;
- audit id;
- tenant id seguro nos logs;
- health checks;
- readiness checks;
- dashboards e alerting.

### Backup, restore e continuidade

Objetivo: cumprir expectativas institucionais de preservação, recuperação e
evidência.

Planear:

- backups automáticos;
- restore testado;
- recuperação por tenant;
- retenção;
- integridade;
- exportação de evidência;
- plano de continuidade;
- testes periódicos de recuperação.

### Performance, limites e quotas

Objetivo: evitar APIs sem limites explícitos.

Planear:

- tamanho máximo de anexos;
- paginação obrigatória;
- limites de exportação;
- timeouts;
- quotas por tenant;
- concorrência;
- operações longas fora do request/response síncrono;
- critérios mínimos de carga quando aplicável.

### Jobs e processamento assíncrono

Objetivo: preparar operações longas ou reprocessáveis sem contaminar o domínio.

Planear:

- filas;
- workers;
- retries;
- idempotência;
- dead-letter;
- reprocessamento;
- geração de PDF/export;
- ingestão;
- sincronizações;
- uso eventual de Go em workers/ferramentas sem duplicar regra institucional.

### CI/CD e matriz de plataformas

Objetivo: preservar portabilidade e qualidade com checks automáticos.

Planear:

- `cargo fmt --all --check`;
- `cargo check --workspace`;
- `cargo test --workspace`;
- `cargo clippy --workspace`;
- `cargo audit`;
- `cargo deny`;
- CI Linux;
- CI Windows;
- validação WSL/Linux headless;
- build de container quando existir API/SaaS.

### Packaging e deployment

Objetivo: tornar o backend API/SaaS operável em Linux/cloud.

Planear:

- imagem container;
- configuração por ambiente;
- secrets;
- migrações no deploy;
- readiness;
- graceful shutdown;
- logs em stdout/stderr;
- artefactos versionados;
- rollback operacional.

### Política de manutenção

Objetivo: manter o kernel simples de evoluir.

Planear:

- crates pequenos e com ownership claro;
- `README.md` e `MAN.md` alinhados;
- ADRs para decisões estruturais;
- checklist de breaking changes;
- testes de contrato;
- exemplos mínimos de integração;
- regra de não promover para o kernel sem evidência de transversalidade.

## Sequência de implementação recomendada

Esta sequência evita tentar construir SaaS completo antes de estabilizar o kernel.
As fases são cumulativas, mas podem ser executadas de forma incremental por
domínio.

### Fase A - Base operacional e contratos

- Milestone 0: portabilidade operacional Linux/WSL.
- Milestone 1: contratos canónicos em `normordis-spec`.
- Milestone 2: DTOs Rust alinhados com a spec.

Objetivo: estabilizar a linguagem comum antes de criar superfícies de runtime.

### Fase B - Núcleo consumível por hosts

- Milestone 3: application services headless.
- Milestone 4: runtime builder agnóstico.
- Milestone 5: transações e unidade de trabalho.

Objetivo: permitir que qualquer host chame os mesmos casos de uso sem conhecer
detalhes internos do domínio.

### Fase C - Provas de integração

- Milestone 6: desktop runtime/Tauri fino.
- Milestone 7: web/API runtime fino.
- Milestone 8: multi-tenancy e isolamento SaaS.
- Milestone 9: frontend universal.

Objetivo: provar a simetria desktop/web. O runtime web pode começar como API
single-tenant ou ambiente controlado, mas SaaS multi-tenant de produção só deve
ser considerado depois da Milestone 8.

### Fase D - Operação SaaS

- Milestone 10: observabilidade e release mínimo.
- Milestone 11: API pública e contrato SaaS.
- Milestone 12: segurança SaaS e autorização.
- Milestone 13: migrações, backup e continuidade.
- Milestone 14: jobs, workers e operações longas.
- Milestone 15: packaging, deploy e operação Linux/cloud.

Objetivo: transformar o backend em serviço operável, versionado e manutenível.

## Arquitetura alvo

```text
normordis-spec
  schemas, DTOs, códigos de erro, cenários de conformidade

core/domain
  entidades, value objects, invariantes, ports

application/runtime headless
  casos de uso, transações, DTOs Rust, conversão de erros públicos

infra/adapters
  SQLite, PostgreSQL, MongoDB, filesystem, object storage, audit store

desktop host
  Tauri commands finos -> application services

web/SaaS host
  HTTP routes finas -> application services

frontend
  normordis-core-ui agnóstico + AppClient da app host
  TauriClient ou HttpClient selecionado no build/bootstrap
```

## Milestone 0 - Portabilidade operacional Linux/WSL

Objetivo: garantir que o kernel e o fluxo de desenvolvimento vivem confortavelmente
em Linux antes de evoluir para Ubuntu Server headless e SaaS.

Esta milestone não é um refactor arquitetural. É uma validação operacional para
reduzir risco em tooling, paths, permissões, SQLite, PDF/fonts, scripts e ambiente
headless.

Entregáveis:

- Projeto transportado/clonado para o filesystem Linux do WSL, preferencialmente
  fora de `/mnt/c`.
- Toolchain Linux documentada: Rust stable, `cargo`, `rustfmt`, `clippy`, `git`,
  `gh` quando aplicável, `pkg-config` e bibliotecas nativas necessárias.
- Checks base executados em WSL:
  - `cargo fmt --all --check`
  - `cargo check --workspace`
  - `cargo test --workspace`
- Registo de diferenças Windows vs Linux que afetem:
  - paths e separadores;
  - permissões de ficheiros;
  - diretórios temporários;
  - SQLite;
  - geração/renderização PDF;
  - fonts;
  - scripts PowerShell sem equivalente shell.
- Documento curto de setup WSL/Linux para desenvolvimento.
- Plano de repetição dos mesmos checks em Ubuntu Server headless.

Critérios de aceitação:

- O workspace compila e testa em WSL sem depender de paths Windows.
- O código fonte usado em Windows e Linux mantém-se comum; exceções por plataforma
  são isoladas em adapters e justificadas.
- Os testes não assumem separadores Windows, permissões Windows ou ambiente
  gráfico.
- Dependências nativas necessárias estão identificadas.
- Scripts críticos de desenvolvimento/release têm alternativa Linux ou limitação
  documentada.
- A evidência existente das `normordis-apps` em Ubuntu 26.04 permanece tratada
  como prova de paridade desktop, não como substituto dos checks headless do
  kernel.

## Milestone 1 - Contratos canónicos em normordis-spec

Objetivo: garantir que desktop e SaaS falam a mesma linguagem.

Entregáveis:

- Schemas para requests/responses dos fluxos principais.
- Schemas para filtros, paginação e envelopes de erro.
- Catálogo de códigos de erro públicos estáveis.
- Cenários de conformidade para sucesso e falha.
- Fixtures para documental, auditoria, ingest, exports e identidade/contexto.

Critérios de aceitação:

- `spec-conformance` cobre os contratos novos.
- Nenhum erro público expõe paths, queries, segredos, dados pessoais desnecessários
  ou detalhes internos.
- Os contratos distinguem estado de UI, estado de trabalho e estado institucional.

## Milestone 2 - DTOs Rust alinhados com a spec

Objetivo: o kernel expor tipos serializáveis estáveis para consumo por Tauri e
HTTP.

Esta milestone deve começar por um inventário: distinguir tipos de domínio,
pedidos internos, requests de adapters e DTOs verdadeiramente públicos. Nem todo
`*Request` existente é automaticamente um DTO canónico.

Entregáveis:

- Inventário dos tipos existentes com nomes como `*Request`, `*Response`,
  `*Filter`, `Page<T>` e equivalentes.
- Classificação de cada tipo:
  - domínio interno;
  - port/application request;
  - adapter request;
  - DTO público alinhado com `normordis-spec`.
- Requests e responses canónicos por domínio.
- Tipos comuns para paginação, filtros, ordenação e correlation id.
- `PublicError`/`PublicErrorResponse` alinhado com `support-errors`.
- Round-trip JSON coberto por testes.
- Mapeamento documentado entre schemas `normordis-spec` e tipos Rust.
- Convenção de nomes para evitar ambiguidade entre domínio e transporte, por
  exemplo `DocumentResponse`, `AcceptDocumentRequest`, `PublicErrorResponse`,
  `ListDocumentsFilter`.

Critérios de aceitação:

- DTOs não expõem tipos internos desnecessários.
- DTOs não dependem de Tauri, Axum, Actix ou frameworks frontend.
- `spec-conformance` valida serialização/desserialização dos DTOs relevantes.
- Um handler Tauri e uma route HTTP conseguem usar os mesmos DTOs sem conversões
  divergentes fora da camada de entrada.

## Milestone 3 - Application services headless

Objetivo: criar casos de uso reutilizáveis por desktop e SaaS.

Estado atual: não existe ainda uma camada abrangente de application services
headless. O `miniapp-runtime` é uma prova inicial de orquestração, mas ainda não
define uma superfície completa e estável de use cases para consumo por Tauri/HTTP.

Entregáveis iniciais:

- `accept_document`
- `get_document`
- `list_documents`
- `assign_document_number`
- `attach_document_file`
- `archive_document`
- `record_access`
- `export_document_package`

Cada use case deve:

- receber DTOs e contexto de ator/tenant quando aplicável;
- validar input com tipos do domínio;
- chamar ports, repositories e stores;
- produzir eventos documentais e auditoria;
- devolver DTOs ou erro público estável.

Entregáveis estruturais:

- Definir localização da camada: `crates/runtime/*`, `crates/kernel/application/*`
  ou crates de application service por domínio.
- Separar claramente:
  - domínio puro;
  - DTO público;
  - request interno de use case;
  - adapter de entrada;
  - adapter de persistência/storage.
- Criar testes com repositories/stores fake ou in-memory.
- Documentar para cada use case:
  - input;
  - output;
  - ports usados;
  - eventos emitidos;
  - auditoria registada;
  - garantias transacionais;
  - erros públicos possíveis.

Critérios de aceitação:

- Handlers Tauri/HTTP conseguem chamar os use cases sem conhecer domínio interno.
- Regras de negócio não são duplicadas nos hosts.
- Testes unitários cobrem invariantes, erros e eventos gerados.

## Milestone 4 - Runtime builder agnóstico

Objetivo: montar serviços e dependências sem acoplar a Tauri ou HTTP.

Estado atual: ainda não existe um runtime builder canónico. A composição de
dependencies continua distribuída por crates e testes, sem um ponto único que uma
app desktop ou SaaS possa instanciar de forma previsível.

Entregáveis:

- `KernelRuntime` ou `NormordisRuntime`.
- `RuntimeBuilder` com injeção explícita de ports.
- `RuntimeConfig` independente de ambiente concreto.
- Contextos de execução: `ActorContext`, `RequestContext`, `TenantContext`.
- Conversão centralizada de erros internos para erros públicos.
- Perfis de composição esperados:
  - desktop local;
  - SaaS/web;
  - testes/in-memory;
  - futuro headless worker.
- Contratos para capacidades comuns:
  - clock;
  - id generation;
  - audit store;
  - documental repositories;
  - attachment/file/object store;
  - config;
  - actor/session context;
  - tenant context;
  - unit of work/transações.

Critérios de aceitação:

- O builder permite composição desktop e SaaS.
- O crate de runtime não depende de `tauri`, `axum`, `actix`, `rusqlite` ou
  `sqlx` diretamente, salvo em crates/adapters próprios.
- Testes usam fakes/in-memory adapters.

## Milestone 5 - Transações e unidade de trabalho

Objetivo: garantir atomicidade em fluxos institucionais.

Fluxos críticos:

- documento + evento documental;
- documento + número + evento + auditoria;
- anexo + metadados + evento + auditoria;
- exportação + manifesto + auditoria;
- alterações multi-entidade em contexto SaaS.

Entregáveis:

- Port `UnitOfWork` ou padrão transacional equivalente.
- Contrato para commits/rollbacks.
- Implementação SQLite inicial.
- Estratégia documentada para PostgreSQL/MongoDB quando aplicável.

Critérios de aceitação:

- Use cases críticos não fazem escritas parciais.
- Testes simulam falha a meio do fluxo.
- Erros de transação são convertidos para erros públicos estáveis.

## Milestone 6 - Desktop runtime

Objetivo: tornar o consumo via Tauri 2 direto e fino.

Estado atual: ainda não há adapter Tauri exemplar neste repositório que prove a
integração fim a fim. As apps Tauri consumidoras já demonstram portabilidade, mas
o kernel ainda deve documentar/fornecer o padrão de integração fino.

Entregáveis:

- Exemplo ou crate host com Tauri commands.
- Bootstrap local com SQLite, filesystem local e configuração local.
- Estratégia de armazenamento de drafts e anexos.
- Mapeamento de erros para respostas Tauri serializáveis.
- Documentação de integração para apps desktop.
- Exemplo de comando fino:
  - desserializa DTO;
  - obtém runtime/contexto;
  - chama use case;
  - converte erro para `PublicErrorResponse`;
  - não contém regra de negócio.

Critérios de aceitação:

- Tauri commands só extraem input, chamam o runtime e devolvem output.
- O domínio não conhece paths reais nem APIs Tauri.
- O runtime desktop funciona offline-first.

## Milestone 7 - SaaS/web runtime

Objetivo: criar um adapter HTTP/API fino, adequado a deployment Linux/cloud, sem
deixar endpoints ditarem o domínio.

Estado atual: ainda não há adapter HTTP exemplar neste repositório que prove a
integração fim a fim. A camada web/SaaS deve nascer depois dos DTOs, use cases e
runtime builder, para evitar que endpoints ditem o domínio.

Nota de sequência: esta milestone pode começar como runtime web/API controlado. A
prontidão SaaS multi-tenant depende da Milestone 8.

Entregáveis:

- Exemplo ou crate host com handlers Axum ou Actix.
- Bootstrap server com configuração por ambiente.
- Integração com PostgreSQL/MongoDB conforme decisão por domínio.
- Storage de ficheiros/anexos via object storage local/cloud.
- Autenticação via adapter: OIDC, OAuth2, JWT ou cookies `HttpOnly`.
- `TenantContext` previsto na fronteira de request, mesmo que a primeira prova seja
  single-tenant.
- Exemplo de handler fino:
  - desserializa DTO;
  - resolve actor/tenant/correlation id;
  - chama use case;
  - converte erro para resposta HTTP pública;
  - não contém regra de negócio.

Critérios de aceitação:

- HTTP handlers não contêm regra institucional.
- Nenhuma query ou store institucional opera sem tenant quando o modo SaaS
  multi-tenant estiver ativo.
- Auditoria, numeração e documentos são tenant-scoped.

## Milestone 8 - Multi-tenancy e isolamento SaaS

Objetivo: suportar SaaS sem risco de mistura institucional.

Entregáveis:

- `TenantId` e `TenantContext`.
- Ports tenant-aware.
- Estratégia de isolamento: schema por tenant, coluna `tenant_id`, base separada
  ou combinação documentada.
- Testes de isolamento entre tenants.
- Auditoria tenant-scoped.

Critérios de aceitação:

- Não é possível listar, consultar, exportar ou auditar dados de outro tenant.
- Erros não revelam existência de dados noutro tenant.
- Índices e constraints consideram tenant quando aplicável.

## Milestone 9 - Frontend universal

Objetivo: permitir uma UX React/TypeScript única.

Entregáveis esperados nos projetos de app:

- Interface `AppClient`.
- `TauriClient`.
- `HttpClient`.
- Seleção de client num ponto único de bootstrap/build.
- Tipos TypeScript gerados ou sincronizados a partir de `normordis-spec`.

Critérios de aceitação:

- A UI não contém checks runtime espalhados, como `window.__TAURI__`.
- Componentes React consomem apenas `AppClient`.
- Desktop e web partilham fluxos e mensagens de erro.

## Milestone 10 - Observabilidade e release mínimo

Objetivo: preparar operação local e cloud com confiança.

Entregáveis:

- Correlation id em requests, eventos e erros.
- Logging estruturado sem dados sensíveis.
- Métricas runtime-agnósticas.
- Política mínima de secrets por adapter.
- Checks de release: `cargo check --workspace`, `cargo test --workspace`,
  `cargo audit`, `cargo deny`, SBOM e manifestos quando aplicável.

Critérios de aceitação:

- Logs e erros públicos não expõem paths, queries, segredos ou dados pessoais
  desnecessários.
- O mesmo use case é observável em desktop e SaaS.
- Pipeline de release documentado e reproduzível em Linux.

## Milestone 11 - API pública e contrato SaaS

Objetivo: preparar a API web do host SaaS de forma documentada, versionada e
alinhada com `normordis-spec`.

Entregáveis:

- Convenção de versionamento de API.
- OpenAPI inicial para os fluxos mínimos.
- Exemplos de requests/responses.
- Envelopes de erro públicos.
- Política de paginação, filtros e ordenação.
- Testes de contrato API.

Critérios de aceitação:

- A API não expõe modelos internos desnecessários.
- OpenAPI, DTOs Rust e `normordis-spec` estão alinhados.
- Breaking changes são identificáveis e documentáveis.

## Milestone 12 - Segurança SaaS e autorização

Objetivo: desenhar autenticação/autorização como fronteira de host/adapter, sem
misturar protocolos no domínio.

Entregáveis:

- Modelo de `ActorContext` e permissões.
- Estratégia RBAC/ABAC.
- Política de scopes por use case/API.
- Integração planeada com OIDC/OAuth2/JWT/cookies `HttpOnly`.
- Threat model inicial.
- Política de proteção contra enumeração de dados.

Critérios de aceitação:

- Use cases recebem contexto de ator resolvido, não tokens/protocolos brutos.
- Erros de autorização não revelam dados indevidos.
- Permissões são testáveis sem servidor HTTP real.

## Milestone 13 - Migrações, backup e continuidade

Objetivo: preparar operação de longa duração, com evolução segura de dados e
recuperação testável.

Entregáveis:

- Estratégia de migrations por backend.
- Política de compatibilidade de dados.
- Plano de backup/restore.
- Verificação de integridade pós-restore.
- Estratégia de recuperação por tenant.
- Documentação de retenção e evidência.

Critérios de aceitação:

- Migrações críticas têm testes ou validação documentada.
- Restore deixa o sistema funcional e auditável.
- Backups não quebram isolamento por tenant.

## Milestone 14 - Jobs, workers e operações longas

Objetivo: separar operações longas do request/response síncrono.

Entregáveis:

- Modelo de job.
- Contratos de idempotência.
- Estratégia de retries e dead-letter.
- Workers para PDF/export/ingest quando aplicável.
- Convenção para serviços Go adjacentes, se usados.

Critérios de aceitação:

- Reprocessamento não duplica efeitos institucionais.
- Jobs não contêm regra de negócio que pertença ao kernel.
- Falhas são auditáveis e observáveis.

## Milestone 15 - Packaging, deploy e operação Linux/cloud

Objetivo: tornar o backend API/SaaS operável em ambiente Linux/cloud.

Entregáveis:

- Container build.
- Configuração por ambiente.
- Gestão de secrets.
- Readiness/health checks.
- Graceful shutdown.
- Logs estruturados em stdout/stderr.
- Pipeline CI/CD com matriz Linux/Windows quando aplicável.

Critérios de aceitação:

- O serviço arranca em Linux headless sem dependências implícitas de desktop.
- Configuração e secrets não são compilados no binário.
- Deploy e rollback têm procedimento documentado.

## Decisões em aberto

- Nome e localização final da camada de application services:
  `crates/runtime/*`, `crates/kernel/application/*` ou crates por domínio.
- Se `miniapp-runtime` deve ser renomeado para refletir melhor a função de
  orquestração headless.
- Backend SaaS primário: PostgreSQL, MongoDB ou combinação por domínio.
- Papel de Go no ecossistema: serviços operacionais, workers e ferramentas, sem
  duplicar domínio do kernel.
- Estratégia principal de multi-tenancy.
- Se os clients TypeScript serão gerados diretamente de `normordis-spec` ou
  mantidos manualmente com testes de contrato.
- Política formal de versionamento/depreciação para API, DTOs e specs.
- Estratégia OpenAPI: gerar a partir de código, gerar a partir da spec ou manter
  como contrato validado.
- Estratégia de jobs/workers e eventual tecnologia de fila.
- Estratégia de packaging/deployment para SaaS.

## Próximos passos recomendados

1. Fechar os contratos `normordis-spec` para os fluxos documentais mínimos.
2. Alinhar DTOs Rust e testes `spec-conformance`.
3. Extrair `miniapp-runtime` para uma camada de use cases mais explícita ou
   evoluí-lo com um `KernelRuntime` agnóstico.
4. Implementar `accept_document` e `get_document` como prova vertical.
5. Criar dois adapters de entrada mínimos: um Tauri command e uma route HTTP,
   ambos chamando exatamente o mesmo use case.
