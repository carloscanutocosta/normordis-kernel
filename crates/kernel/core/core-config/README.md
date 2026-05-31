# core-config

Declara e valida perfis de configuração do Mini-Kernel RS — sem I/O, sem dependências de runtime.

## Responsabilidade

- Modelar `MiniKernelProfile` e os subperfis: app, runtime, storage, crypto, logging e audit.
- Modelar `AppConfig` / `PathsConfig` / `AppOptions` para bootstrap local simples.
- Validar consistência cruzada entre perfis (ex: storage cifrado exige crypto activa).
- Serializar e desserializar perfis como dados puros; a validação é sempre explícita.

## Não responsabilidade

- Não abre bases de dados nem cria adapters.
- Não cria directórios nem ficheiros.
- Não carrega nem grava ficheiros de configuração — responsabilidade do `app-bootstrap`.
- Não carrega segredos nem executa wiring de runtime.

## Início rápido

### Desenvolvimento

```rust
use core_config::MiniKernelProfile;

let profile = MiniKernelProfile::dev_default("data");
profile.validate()?;
# Ok::<(), core_config::ConfigError>(())
```

### Produção

```rust
use core_config::MiniKernelProfile;

let profile = MiniKernelProfile::prod("data", "prod-secret-key-2026");
profile.validate()?;
# Ok::<(), core_config::ConfigError>(())
```

### Testes

```rust
use core_config::MiniKernelProfile;

let profile = MiniKernelProfile::test_memory();
profile.validate()?;
# Ok::<(), core_config::ConfigError>(())
```

### Configuração local simples (AppConfig)

```rust
use core_config::{AppConfig, validate_app_config, resolve_paths};

let config = AppConfig::default();
validate_app_config(&config)?;
let paths = resolve_paths("/app/data", &config.paths);
# Ok::<(), core_config::ConfigError>(())
```

## Documentação de referência

Ver [MAN.md](MAN.md) para a especificação completa de tipos, regras de validação,
garantias de segurança, formato JSON e códigos de erro.
