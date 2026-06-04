# support-normalization

Biblioteca headless de normalização, validação auxiliar e transformação de inputs
para o NORMORDIS Kernel.

## Responsabilidade

- Normalização de texto, whitespace, Unicode, diacríticos e nomes portugueses.
- Parsing numérico permissivo mas determinístico.
- Conversão exacta de dinheiro EUR para cêntimos e extenso em pt-PT.
- Normalização de datas simples para `YYYY-MM-DD`.
- Validação estrutural auxiliar de NIF e email com domínio IDNA/punycode.

## Não responsabilidade

- UI, Tauri, SQLite, I/O ou serviços externos.
- Validação fiscal, legal, DNS/MX ou bancária.
- Localização multilingue completa.
- Regras contabilísticas ou arredondamentos legais.

## Exemplo mínimo

```rust
use support_normalization::{money_str_to_cents, normalize_for_lookup};

assert_eq!(normalize_for_lookup("  Órgão   Público "), "orgao publico");
assert_eq!(money_str_to_cents("1.234,56").unwrap(), 123456);
```

## Testes

```powershell
cargo test -p support-normalization
```

Ver [MAN.md](MAN.md) para contrato público, invariantes, limitações e estado de produção.
