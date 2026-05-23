# support-clock

Biblioteca headless para abstracao de tempo UTC e clocks testaveis.

## Capacidades

- `Clock` como contrato minimo
- `SystemClock` para tempo real
- `FixedClock` para testes deterministas
- helper `now_utc`

## Contrato

Consultar `MAN.md`.

## Testes

```bash
cargo test -p support-clock
```
