# support-address

Biblioteca headless para modelo, parsing e formatacao transversal de moradas e codigos postais.

## Capacidades

- parsing de `cp4-cp3`
- modelo `PostalCode`
- modelo `AddressCandidate`
- formatacao postal pronta para enderecamento

## Fora de escopo

- lookup SQLite
- geocodificacao
- UI de selecao
- decisao de dominio sobre qual candidato escolher

O lookup SQLite vive em `kernel/infra/address-sqlite`.

## Contrato

Consultar `MAN.md`.

## Testes

```bash
cargo test -p support-address
```
