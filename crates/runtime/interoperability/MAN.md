# Manual: support-interoperability

## Enquadramento

`support-interoperability` e a camada transversal, substituivel e headless para
orquestrar interoperabilidade nas mini-apps. O crate existe para que as apps nao
dependam diretamente de adapters concretos de exportacao, preservando a
separacao entre contrato (`core-exports`), capacidades transversais
(`support-interoperability`) e materializacao tecnica (`infra-export`).

## Contrato publico

- `ExportRequestBuilder` constroi `ExportMaterializationRequest`.
- `ExportAuthorizationContext` identifica actor, finalidade e correlacao.
- `ExportAuthorizationPolicy` autoriza ou rejeita um pedido.
- `InteroperabilityExportService` valida contexto/pedido, aplica a policy e
  chama `ExportMaterializerPort`.

## Como usar

1. Converter dados da mini-app para `TabularRow`.
2. Construir o pedido com `ExportRequestBuilder`.
3. Criar um contexto com actor, purpose e correlation_id.
4. Injetar uma policy de autorizacao real do host.
5. Injetar um materializer concreto que implemente `ExportMaterializerPort`.

## Invariantes e regras

- `actor`, `purpose` e `correlation_id` sao obrigatorios.
- `snapshot_id`, `output_ref`, `columns` e `rows` sao obrigatorios.
- A autorizacao e sempre executada antes de chamar o materializer.
- O crate nao materializa ficheiros nem faz I/O concreto.

## Limitacoes atuais

- Nao inclui uma policy RBAC real; o host deve fornece-la.
- Nao emite audit events por si so; o runtime/host deve compor audit quando
  necessario.
- A normalizacao tabular e intencionalmente simples.

## ToDo

- Adicionar policies compostas por claims/roles.
- Adicionar validadores de perfis formais de interoperabilidade.
- Adicionar builders para datasets multi-tabela quando `core-exports` suportar.
