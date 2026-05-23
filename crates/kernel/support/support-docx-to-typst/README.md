# support-docx-to-typst

Conversor de documentos DOCX para código Typst — extracção de blocos de texto e estilos para renderização posterior.

## Objectivo

Converte ficheiros DOCX (Microsoft Word) em código fonte Typst, preservando a estrutura de parágrafos, cabeçalhos, listas e formatação básica (negrito). Adequado para migração de templates legacy para o ecossistema Typst.

## Posição arquitectural

`crates/kernel/support` — utilitário técnico sem dependências de domínio. Sem estado próprio, sem persistência.

## Responsabilidade

- Ler ficheiros DOCX a partir do disco ou de bytes em memória.
- Extrair blocos de conteúdo (parágrafos, cabeçalhos, listas).
- Detectar estilos DOCX (`Heading1`–`Heading6`, `ListBullet`, `ListNumber`, `Normal`).
- Converter para código Typst válido com formatação equivalente.

## Não-responsabilidade

- Não renderiza PDFs — use `documentos-pdf` para isso.
- Não preserva imagens, tabelas, caixas de texto nem macros VBA.
- Não faz round-trip (Typst → DOCX).

## Exemplo mínimo

```rust
use support_docx_to_typst::convert_docx_file;

let typst_source = convert_docx_file("template.docx")?;
println!("{typst_source}");
```

## Validação

```sh
cargo test -p support-docx-to-typst
```
