# Manual: support-typst-template

## Contrato publico

- `substitute_vars(source, vars)` substitui marcadores `{{chave}}`.
- `extract_plain_text(source)` remove diretivas e markup Typst basico.
- `render_text(source, vars)` combina substituicao e extracao.
- `load_typst_file(path)` le um ficheiro `.typ`.

## Como usar

Usa este crate quando a app precisa de manipular templates Typst sem compilar
PDF. Para rendering real, usa `render-typst`.

## Invariantes e regras

- A substituicao e textual e deterministica.
- Marcadores ausentes permanecem inalterados.
- A extracao de texto e conservadora e orientada a previews simples.
- O crate e headless e nao depende de Typst engine.

## Limitacoes atuais

- Nao e parser Typst completo.
- Nao avalia expressoes Typst.
- Nao valida variaveis obrigatorias.

## ToDo

- Adicionar validacao opcional de variaveis obrigatorias.
- Expor lista de marcadores encontrados.
- Melhorar extracao para blocos Typst complexos.
