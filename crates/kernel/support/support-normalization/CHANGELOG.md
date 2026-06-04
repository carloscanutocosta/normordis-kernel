# CHANGELOG

## [Unreleased]

### Added

- crate `support-normalization`
- funcoes de normalizacao e validacao de texto
- funcoes de numeros e extenso em portugues para valores monetarios
- normalizacao de datas e validacao de NIF/e-mail
- parsing monetario exacto por string (`money_str_to_cents`, `money_str_to_words_eur`)
- normalizacao Unicode NFC/NFD/NFKC/NFKD
- normalizacao IDNA/punycode de dominios de email
- parsing monetario exacto por `rust_decimal::Decimal`
- validacao estrutural mais estrita de email
- parsing numerico deterministico com rejeicao de letras e grouping invalido
- documentacao de contrato production-ready interno/controlado
