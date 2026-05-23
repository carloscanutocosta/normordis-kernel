# NAME

support-clock

# SYNOPSIS

Biblioteca headless para abstracao de tempo UTC e clocks testaveis.

# DESCRIPTION

`support-clock` fornece uma abstracao minima sobre o tempo atual, permitindo usar um clock real (`SystemClock`) ou um clock fixo (`FixedClock`) em testes e fluxos deterministas.

# PUBLIC CONTRACT

## Tipos publicos

- `Clock`
- `SystemClock`
- `FixedClock`

## Funcoes e metodos publicos

- `Clock::now`
- `FixedClock::new`
- `now_utc`

# INVARIANTS

- todos os instantes sao UTC
- `FixedClock` devolve sempre o mesmo instante
- a biblioteca nao mantem estado global mutavel

# STATUS

Proposto

# LAST REVIEW

2026-03-25
