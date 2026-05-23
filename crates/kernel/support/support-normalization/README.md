# support-normalization

Biblioteca headless com funcoes utilitarias transversais para normalizacao, validacao e transformacao de inputs.
Faz parte de `crates/kernel/support` por ser uma capacidade tecnica transversal
do Mini-Kernel RS.

Suporta:

- normalizacao de texto e espacos
- capitalizacao, title case e nomes portugueses
- parsing e limpeza de numeros
- extenso de numeros e montantes em euros
- normalizacao de datas para `yyyy-mm-dd`
- validacao de NIF e e-mail

## Testes

```bash
cargo test -p support-normalization
```
