# Manual: support-normalization

## Propósito e fronteira

O `support-normalization` é uma biblioteca headless de normalização e validação
auxiliar para o NORMORDIS Kernel.

Centraliza operações transversais sobre texto, números, datas, dinheiro, NIF e
email que são úteis em formulários, documentos e validadores estruturais.

Não decide regras de negócio, não valida existência real de entidades externas e
não substitui validadores canónicos de domínio quando estes existem.

## Responsabilidade

- Normalizar whitespace e transformar strings para pesquisa.
- Normalizar Unicode em NFC/NFD/NFKC/NFKD.
- Remover diacríticos por decomposição Unicode preservando a capitalização base.
- Capitalizar texto e nomes portugueses.
- Extrair dígitos ou caracteres alfanuméricos.
- Fazer parsing numérico permissivo mas determinístico.
- Converter dinheiro para cêntimos por caminho exacto (`money_str_to_cents`) ou
  por conveniência `f64` (`money_to_cents`).
- Escrever números e montantes em euros por extenso em pt-PT.
- Normalizar datas simples para ISO `YYYY-MM-DD`.
- Validar forma estrutural mínima de NIF e email.

## Não responsabilidade

- Localização multilingue completa.
- Regras fiscais, legais ou bancárias.
- Validação de existência real de email, domínio, NIF ou pessoa.
- Parsing monetário livre de todos os formatos humanos possíveis.
- Formatação contabilística, arredondamentos legais ou regras de moeda fora de EUR.
- UI, Tauri, SQLite, I/O ou integração com serviços externos.

## Contrato público

### Erros

```rust
pub enum NormalizationError {
    InvalidDate(String),
    InvalidMoney,
    InvalidNumber,
    NumberOutOfRange,
}
```

### Texto

| Função | Contrato |
|--------|----------|
| `normalize_whitespace(value)` | Colapsa whitespace Unicode em espaços simples e remove extremidades. |
| `trim_to_none(value)` | Devolve `None` se a string for vazia após trim. |
| `normalize_unicode_nfc(value)` | Normalização Unicode NFC. |
| `normalize_unicode_nfd(value)` | Normalização Unicode NFD. |
| `normalize_unicode_nfkc(value)` | Normalização Unicode NFKC. |
| `normalize_unicode_nfkd(value)` | Normalização Unicode NFKD. |
| `strip_diacritics(value)` | Remove marcas combinantes após NFD, preservando maiúsculas/minúsculas base. |
| `normalize_for_lookup(value)` | Remove diacríticos, normaliza whitespace e converte para lowercase. |
| `capitalize_first(value)` | Capitaliza o primeiro char Unicode. |
| `title_case(value)` | Normaliza whitespace e capitaliza cada palavra. |
| `normalize_portuguese_name(value)` | Capitaliza nomes e mantém partículas portuguesas em minúsculas quando não iniciais. |
| `digits_only(value)` | Mantém apenas dígitos ASCII. |
| `letters_and_digits_only(value)` | Mantém apenas caracteres Unicode alfanuméricos. |

### Números

| Função | Contrato |
|--------|----------|
| `parse_i64_loose(value)` | Aceita sinal opcional e separadores de milhar consistentes (` `, `.`, `,`). Rejeita letras e grouping inválido. |
| `parse_f64_loose(value)` | Aceita decimal `.` ou `,` e milhares consistentes. Quando há `.` e `,`, o último separador é decimal. |
| `round_to_places(value, places)` | Arredonda `f64`; valores não finitos são devolvidos sem alteração. |

Regras de parsing:

- `"1 234 567"`, `"1.234.567"` e `"1,234,567"` são inteiros válidos.
- `"1.234,56"`, `"1,234.56"` e `"1 234,56"` são decimais válidos.
- `"abc123"`, `"12.34"` como inteiro, `"12..34"` e `"1,23,4"` são rejeitados.
- O parser é estrutural; não aplica localização regional implícita.

### Dinheiro EUR

| Função | Contrato |
|--------|----------|
| `money_str_to_cents(value)` | Caminho exacto recomendado. Aceita euros com 0 a 2 casas decimais inequívocas. |
| `money_decimal_to_cents(value)` | Caminho exacto com `rust_decimal::Decimal`; rejeita escala superior a 2. |
| `money_to_cents(value)` | Conveniência para `f64`; arredonda a 2 casas por multiplicação por 100. |
| `money_cents_to_words_eur(cents)` | Escreve cêntimos inteiros por extenso em euros. |
| `money_str_to_words_eur(value)` | Converte string monetária exacta para extenso. |
| `money_to_words_eur(value)` | Conveniência `f64` para extenso. |

Para workflows financeiros, usar `money_str_to_cents` ou `money_cents_to_words_eur`.
`money_to_cents(f64)` existe para compatibilidade e inputs já numéricos, mas não
deve ser o caminho preferido para valores introduzidos por utilizadores.

`money_str_to_cents` rejeita formatos ambíguos como `"1,234"` ou `"1.234"`.
Usar `"1234"`, `"1 234"`, `"1234,56"`, `"1.234,56"` ou `"1,234.56"`.

### Datas

`normalize_date_to_iso(value)` aceita:

- `YYYY-MM-DD`
- `YYYY/MM/DD`
- `DD-MM-YYYY`
- `DD/MM/YYYY`

Devolve sempre `YYYY-MM-DD` ou `NormalizationError::InvalidDate`.

### Validações auxiliares

| Função | Contrato |
|--------|----------|
| `is_valid_nif(value)` | Valida NIF português por 9 dígitos, primeiro dígito permitido e checksum MOD 11. |
| `normalize_domain_to_ascii(domain)` | Normaliza domínio Unicode para IDNA/punycode ASCII lowercase. |
| `is_valid_email(value)` | Valida local-part ASCII permitido e domínio IDNA com labels válidos. |

`is_valid_email` não valida DNS, MX, existência real da caixa postal nem emails
com local-part internacionalizada. Para validação operacional completa, combinar
com o porto `core_validation::EmailVerificationPort` e um adaptador de infra.

## Invariantes

- O crate é puro, determinístico e agnóstico de UI/runtime.
- Não faz I/O.
- Não depende de infra nem de adapters.
- Datas normalizadas usam sempre `YYYY-MM-DD`.
- Dinheiro canónico interno usa cêntimos inteiros.
- Erros públicos não expõem dados sensíveis além do input de data inválida já
  fornecido pelo chamador.

## Limitações

- Remoção de diacríticos remove marcas combinantes após decomposição Unicode; alguns
  caracteres compatíveis ou ligaduras podem exigir NFKD/NFKC antes da chamada,
  conforme o caso de uso.
- `title_case` é estrutural e não implementa regras linguísticas completas.
- Extenso numérico suporta até `999_999_999_999`.
- Email suporta domínio IDNA/punycode; local-part continua ASCII estrutural.
- DNS/MX e confirmação real de mailbox pertencem a infra/adapters.
- `money_to_cents(f64)` é compatível, mas não é o caminho canónico para entrada
  monetária crítica.

## Estado de produção

**Estado:** production-ready interno/controlado para normalização transversal,
parsing estrutural determinístico e suporte a validadores do kernel.

**Reserva:** cenários financeiros formais devem usar cêntimos inteiros ou
`money_str_to_cents`; validação de email deve ser tratada como estrutural.

## Exemplos

```rust
use support_normalization::{normalize_for_lookup, parse_f64_loose};

assert_eq!(normalize_for_lookup("  Órgão   Público "), "orgao publico");
assert_eq!(parse_f64_loose("1.234,56"), Some(1234.56));
```

```rust
use support_normalization::{money_str_to_cents, money_cents_to_words_eur};

let cents = money_str_to_cents("1.234,56").unwrap();
assert_eq!(cents, 123456);
assert_eq!(
    money_cents_to_words_eur(cents).unwrap(),
    "mil, duzentos e trinta e quatro euros e cinquenta e seis cêntimos"
);
```
