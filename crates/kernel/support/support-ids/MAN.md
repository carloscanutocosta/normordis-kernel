# NAME

support-ids

# SYNOPSIS

Biblioteca headless para geracao de identificadores tecnicos unicos.

# DESCRIPTION

`support-ids` fornece um tipo leve para IDs tecnicos (`TechnicalId`) e helpers para gerar novos identificadores UUID v4 como valor tipado ou `String`.

# PUBLIC CONTRACT

## Tipos publicos

- `TechnicalId`

## Funcoes e metodos publicos

- `TechnicalId::new`
- `TechnicalId::as_str`
- `new_id`
- `new_id_string`

# INVARIANTS

- os IDs gerados usam UUID v4
- `new_id_string` devolve o mesmo formato textual de `TechnicalId`
- a biblioteca nao define semantica funcional para o identificador

# STATUS

Proposto

# LAST REVIEW

2026-03-25
