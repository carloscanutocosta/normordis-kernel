# Regras de Negócio — core-config

`core-config` define configuração declarativa e validável. Não executa I/O, não
cria directórios, não abre bases de dados e não instancia adapters.

---

## AppConfig

### CFG-R01 — app_name obrigatório e limitado

`options.app_name` é obrigatório e tem limite máximo de 128 caracteres.

### CFG-R02 — environment canónico

`options.environment` usa apenas `dev`, `test` ou `prod`.

### CFG-R03 — paths relativos e seguros

`paths.*` deve ser caminho relativo, não vazio e sem componentes `..`.
Path traversal é rejeitado como falha de segurança, não de negócio.

### CFG-R04 — Sem campos desconhecidos

Campos desconhecidos em `AppConfig`, `PathsConfig` e `AppOptions` são rejeitados.
`additionalProperties: false` é enforced no schema.

---

## MiniKernelProfile

### CFG-R05 — Nomes de profile únicos

Perfis de storage devem ter nomes únicos dentro de `MiniKernelProfile`.

### CFG-R06 — default_profile deve existir

`default_profile` deve referenciar um profile com esse nome declarado em `profiles`.

### CFG-R07 — Storage em memória sem path nem cifragem

Storage em memória não pode ter `database_path` nem `encrypted = true`.

### CFG-R08 — Storage SQLite com path

Storage SQLite deve ter `database_path` declarado.

### CFG-R09 — Crypto activo quando storage está cifrado

Se algum storage estiver cifrado, `crypto.enabled` deve ser `true`.

### CFG-R10 — Auditoria activa com storage de propósito audit

Auditoria activa deve referenciar um storage profile existente com propósito `audit`.
