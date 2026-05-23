# support-logging

Logging tecnico local do Mini-Kernel RS.

## Objetivo

Registar diagnostico operacional local em ficheiros `JSON Lines` para ajudar em
troubleshooting tecnico.

## Regra fundamental

```text
support-logging = diagnostico tecnico operacional
core-audit = evidencia institucional auditavel
```

Estes conceitos nunca devem ser misturados.

## Responsabilidade

- Escrever eventos tecnicos em `.jsonl`.
- Registar erros tecnicos, warnings e diagnostico local.
- Rotacionar ficheiros por tamanho.
- Aplicar retencao por dias.
- Filtrar por nivel minimo.
- Limitar tamanho de mensagens e `details`.
- Redigir campos sensiveis comuns em `details`.
- Integrar com `support-errors::MiniError` sem expor detalhes internos.

## Nao responsabilidade

- Nao e auditoria institucional.
- Nao e cadeia de custodia.
- Nao e evidencia juridica.
- Nao persiste em SQLite.
- Nao depende de Tauri/UI.
- Nao contem logica de dominio.

## Validacao

```text
cargo test -p support-logging
```

Stress test concorrente de 1 minuto:

```text
cargo test -p support-logging --test stress_tests -- --ignored --nocapture
```
