# MAN.md

## Nome

support-normalization

## Tipo

Biblioteca headless de utilitarios de normalizacao em `crates/kernel/support`

## Objetivo

Centralizar funcoes transversais para tratamento de texto, numeros, datas e validacoes comuns em formularios e documentos.

## Ambito

- normalizacao de strings
- capitalizacao
- normalizacao de nomes portugueses
- parsing e limpeza de numeros
- extenso de numeros e valores monetarios em euros
- normalizacao de datas
- validacao de NIF e e-mail

## Fora de ambito

- UI
- localizacao completa multilingue
- validacao fiscal ou legal exaustiva para todos os paises

## Contrato publico

- `NormalizationError`
- `normalize_whitespace`
- `trim_to_none`
- `strip_diacritics`
- `normalize_for_lookup`
- `capitalize_first`
- `title_case`
- `normalize_portuguese_name`
- `digits_only`
- `letters_and_digits_only`
- `parse_i64_loose`
- `parse_f64_loose`
- `round_to_places`
- `money_to_cents`
- `number_to_words_pt`
- `money_to_words_eur`
- `money_cents_to_words_eur`
- `normalize_date_to_iso`
- `is_valid_nif`
- `is_valid_email`

## Invariantes

- as funcoes sao puras e agnosticas da UI
- a normalizacao de datas devolve sempre `yyyy-mm-dd`
- o extenso monetario usa euros e centimos
- a normalizacao de nomes portugueses preserva em minusculas os elementos de ligacao usuais

## Estado

Proposto

## Ultima revisao

2026-05-12

- movida para `crates/kernel/support/support-normalization`
