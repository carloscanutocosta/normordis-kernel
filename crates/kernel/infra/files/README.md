# support-files

Biblioteca headless para layout tecnico de diretorias e nomes tecnicos de ficheiros.

## Capacidades

- resolucao de `FileLayout`
- criacao de `apps/.database`, `apps/.assets`, `tmp` e `apps/.logs`
- limpeza automatica de ficheiros em `tmp` com mais de 7 dias
- geracao de nomes tecnicos sanitizados

## Contrato

Consultar `MAN.md`.

## Testes

```bash
cargo test -p support-files
```
