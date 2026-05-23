# Manual: infra-export

## Enquadramento

`infra-export` e um componente estrutural de interoperabilidade do Mini-Kernel
RS. O objetivo e permitir que dados institucionais possam ser materializados em
formatos abertos ou amplamente suportados por ecossistemas publicos e privados:
CSV, XML, SQLite e XLSX. Isto apoia requisitos portugueses e europeus de
portabilidade, reutilizacao, auditabilidade e troca entre sistemas, mantendo a
semantica de dominio separada em `core-exports`.

## Contrato publico

- `RuntimeBinding::open_exporter()` valida o provider selecionado e devolve um
  `RuntimeExporter`.
- `Exporter::build_plan()` valida o pedido e calcula um artefacto esperado com
  hash `sha256:<hex>` do payload canonico do pedido.
- `Exporter::export()` escreve exatamente um artefacto no `output_path`.
- `RuntimeExporter` implementa `core_exports::ExportMaterializerPort`, permitindo
  uso atraves de `support-interoperability`.
- Providers suportados: `csv`, `xml`, `sqlite`, `xlsx`.
- O resultado de cada export inclui hash SHA-256 para verificacao posterior de
  integridade do payload que originou o artefacto.

## Como usar

1. Construir um `ExportRequest` com `snapshot_id`, `output_path` e `rows` ou
   `snapshot`.
2. Definir `columns` para controlar ordem/selecionar campos. Quando vazio, as
   colunas sao inferidas das linhas por ordem alfabetica.
3. Selecionar provider em `RuntimeBinding`.
4. Chamar `export`.

## Invariantes e regras

- `snapshot_id` e `output_path` sao obrigatorios.
- O pedido precisa de `rows` ou `snapshot`.
- O unico algoritmo de hash aceite na configuracao e SHA-256.
- CSV segue escaping RFC 4180 basico.
- XML sanitiza nomes de elementos e escapa texto/atributos.
- SQLite cria uma tabela tabular com colunas `TEXT` e regista metadata em
  `export_metadata`.
- XLSX e um pacote OOXML minimo com uma worksheet e strings inline.
- A ordem de colunas e explicita quando `columns` e fornecido; isto deve ser
  preferido em contratos de interoperabilidade para evitar ambiguidades.

## Limitacoes atuais

- Valores complexos (`array`/`object`) sao exportados como JSON numa celula.
- XLSX nao aplica estilos, tipos numericos nativos nem multiplas folhas.
- SQLite e pensado para artefacto de export tabular, nao para persistencia
  canonica de snapshots/audit. Para esse caso usa `exports-sqlite`.
- O crate fornece garantias tecnicas de formato e integridade, mas nao substitui
  matriz legal, politica de retencao, classificacao de informacao ou validacao
  normativa feita pelo host.

## ToDo

- Adicionar styles opcionais para XLSX.
- Suportar multiplas tabelas/folhas por pedido.
- Adicionar streaming para datasets grandes.
- Adicionar perfis formais de interoperabilidade por caso de uso legal/setorial.
