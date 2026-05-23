# support-pdf

Interface mínima de renderização PDF — define o trait `PdfRenderer` e o tipo de erro `PdfError`.

## Objectivo

Fornece o contrato abstracto para renderização PDF sem impor nenhuma engine concreta. Permite que o domínio dependa apenas desta interface e que a engine (Typst, wkhtmltopdf, normordis-pdf, etc.) seja injectada como implementação.

## Posição arquitectural

`crates/kernel/support` — primitiva técnica headless. Sem dependências de infraestrutura. É a base sobre a qual assentam `documentos-pdf` e outros adapters PDF.

## Responsabilidade

- Definir o trait `PdfRenderer` com método `render(source: &str) → Result<Vec<u8>, PdfError>`.
- Definir `PdfError` como tipo de erro opaco (string).

## Não-responsabilidade

- Não renderiza PDFs — apenas define a interface.
- Não conhece Typst, NDT, HTML nem qualquer formato de fonte.
- Não faz gestão de fontes, cache nem fila de jobs.

## Exemplo mínimo

```rust
use support_pdf::{PdfRenderer, PdfError};

struct MyRenderer;
impl PdfRenderer for MyRenderer {
    fn render(&self, source: &str) -> Result<Vec<u8>, PdfError> {
        // engine concreta aqui
        todo!()
    }
}
```

## Validação

```sh
cargo test -p support-pdf
```
