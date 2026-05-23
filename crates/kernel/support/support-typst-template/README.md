# support-typst-template

Helpers headless para templates Typst.

## Responsabilidade

- Substituir marcadores simples `{{chave}}`.
- Extrair texto simples de fonte Typst para previews, pesquisa ou fallback.
- Renderizar texto simples a partir de template + variaveis.
- Ler ficheiros `.typ` quando o host fornece um path.

## Nao responsabilidade

- Nao compila Typst.
- Nao gera PDF.
- Nao conhece fontes, OOXML, filesystem de aplicacao, Tauri ou UI.
- Nao define semantica documental.

## Exemplo minimo

```rust
let text = support_typst_template::render_text(
    "= Declaração\n\nEu, *{{nome}}*.",
    &[("nome", "Ana Silva")],
);
assert_eq!(text, "Declaração\n\nEu, Ana Silva.");
```
