# ingest-scanner

Serviço de ingestão de documentos digitalizados — converte scans em documentos custodiados com limpeza automática.

## Objectivo

Recebe ficheiros digitalizados (imagem ou PDF de scan), aplica um processo de limpeza configurável e persiste o resultado como documento custodiado no repositório documental. Adequado para integrar digitalizadores físicos ou uploads de scans.

## Posição arquitectural

`crates/kernel/infra/services` — serviço de infraestrutura de alto nível. Orquestra `DocumentCustodyRepository` e um `Scanner` (limpeza de imagem). Não tem estado próprio.

## Responsabilidade

- Receber `ScanIngestRequest` com os bytes do scan e metadados.
- Aplicar o scanner de limpeza configurado (`AlwaysCleanScanner` por omissão).
- Persistir o documento digitalizado via `DocumentCustodyRepository`.
- Devolver referência ao documento criado.

## Não-responsabilidade

- Não reconhece texto (OCR) — o scan é guardado como blob opaco.
- Não classifica automaticamente o documento (sem MEF automático).
- Não numera o documento — use `numerador-sqlite` para isso.

## Exemplo mínimo

```rust
use ingest_scanner::{ingest_scanned_document, ScanIngestRequest, AlwaysCleanScanner};

let request = ScanIngestRequest {
    filename: "scan-2025-001.pdf".into(),
    data: pdf_bytes,
    scanned_by: "user-1".into(),
    ..Default::default()
};
let doc_id = ingest_scanned_document(&request, &custody_repo, &AlwaysCleanScanner)?;
```

## Validação

```sh
cargo test -p ingest-scanner
```
