# core-config

`core-config` e o core de configuracao local do Mini-Kernel RS.

## Objetivo

Declarar e validar perfis de configuracao que o runtime desktop precisa para instanciar storage, crypto, logging e auditoria.

## Responsabilidade

- Modelar `MiniKernelProfile` e os subperfis de app, runtime, storage, crypto, logging e audit.
- Modelar `AppConfig`, `PathsConfig` e `AppOptions` para compatibilidade de bootstrap local simples.
- Validar consistencia entre storage cifrado, crypto ativa e perfis auditaveis.
- Validar configuracao local minima de paths e app name sem fazer I/O.
- Preparar multiplos storage profiles locais, incluindo varias SQLite separadas.
- Serializar e desserializar perfis como dados, com validacao explicita apos carregamento.

## Nao responsabilidade

- Nao abre bases de dados.
- Nao cria adapters.
- Nao cria diretorios ou ficheiros.
- Nao cria, carrega ou grava ficheiros JSON de configuracao; isso pertence ao bootstrap/infra.
- Nao carrega segredos.
- Nao executa wiring de runtime.

## Exemplo minimo

```rust
use core_config::MiniKernelProfile;

let profile = MiniKernelProfile::dev_default("data");
profile.validate()?;
# Ok::<(), core_config::ConfigError>(())
```
