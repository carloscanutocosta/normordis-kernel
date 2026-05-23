# infra-export

Adapters de infraestrutura para exportacao tabular em SQLite, CSV, XML e XLSX.
Este componente existe como peca tecnica de interoperabilidade para cumprir
exigencias portuguesas e europeias de portabilidade, reutilizacao e acesso
maquinavel a dados institucionais.

## Responsabilidade

- Materializar `ExportRequest` em ficheiros tecnicos de exportacao.
- Implementar `core_exports::ExportMaterializerPort` para o contrato comum de
  interoperabilidade.
- Produzir plano e resultado com artefacto, path e hash SHA-256 do payload.
- Aceitar linhas tabulares explicitas ou derivar uma linha de um `ExportSnapshot`
  de `core-exports`.
- Criar diretorios de destino quando necessario.
- Disponibilizar formatos comuns para troca entre sistemas publicos, auditoria,
  arquivo, reporting e integracao com ferramentas de escritorio.

## Nao responsabilidade

- Nao define semantica institucional de snapshots nem o contrato comum de
  materializacao; isso pertence a `core-exports`.
- Nao decide autorizacao, auditoria ou wiring final do runtime.
- Nao substitui o adapter persistente `exports-sqlite` de snapshots/audit.
- Nao certifica, por si so, conformidade legal completa; fornece a camada
  tecnica reutilizavel para essa conformidade ser composta pelo host/runtime.

## Exemplo minimo

```rust
use infra_export::{Config, Exporter, ExportRequest, ExportRow, Provider, RuntimeBinding};
use serde_json::json;

let mut row = ExportRow::new();
row.insert("id".into(), json!("A-1"));
row.insert("total".into(), json!(42));

let req = ExportRequest {
    snapshot_id: "exp:demo:1".into(),
    snapshot: None,
    rows: vec![row],
    columns: vec!["id".into(), "total".into()],
    root_name: Some("export".into()),
    sheet_name: Some("Dados".into()),
    table_name: Some("export_rows".into()),
    output_path: "target/demo.csv".into(),
};

let binding = RuntimeBinding {
    default_provider: Provider::Csv,
    csv: Some(Config::new(Provider::Csv)),
    xml: None,
    sqlite: None,
    xlsx: None,
};

let result = binding.open_exporter()?.export(&req)?;
assert_eq!(result.provider, Provider::Csv);
# Ok::<(), infra_export::ExportAdapterError>(())
```
