# MAN.md

## Nome

support-address

## Tipo

Biblioteca headless de modelo, parsing e formatacao de moradas

## Objetivo

Representar codigos postais e candidatos de morada, validar formato `cp4-cp3` e formatar uma morada postal pronta para enderecamento.

## Ambito

- parsing e validacao de codigo postal
- preservacao de multiplos resultados para o mesmo codigo
- formatacao postal com `CRLF`, ignorando campos vazios

## Fora de ambito

- lookup SQLite
- UI de selecao
- edicao de referencias postais
- geocodificacao
- preenchimento automatico de porta, andar ou complemento
- escolha do candidato a usar

## Contrato publico

- `PostalCode`
- `AddressCandidate`
- `AddressError`
- `parse_postal_code`
- `validate_postal_parts`

O adapter SQLite vive em `kernel/infra/address-sqlite`.

## Invariantes

- o codigo postal tem formato `NNNN-NNN`
- a biblioteca nao decide qual candidato deve ser escolhido
- a formatacao postal ignora campos vazios e nao duplica separadores

## Estado

Production-ready interno/controlado para parsing e formatacao postal pura.

## Ultima revisao

2026-05-14

- separado do adapter SQLite, agora localizado em `kernel/infra/address-sqlite`
