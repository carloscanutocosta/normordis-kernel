# Manual do módulo core-rh

## Objetivo

`core-rh` é o núcleo de domínio de identidade e gestão de utilizadores no
Mini-Kernel RS. Responde a "quem fez...?" num contexto COSO de auditoria e
controlo interno. Cobre identificadores de utilizador, perfis funcionais, papéis
sistémicos, sessões locais, referências orgânicas e o bridge para `core-audit`.

## Contrato público

### Tipos principais

```rust
// Utilizador
UserId              // identificador canónico — alfanum + _ - . , máx 128 chars
UserProfile         // perfil completo: username, display_name, email, role, roles, org_unit
UserRole            // papel sistémico: Utilizador | Auditor | Administrator
Role                // role aplicacional com role_id e display_name

// Identidade operacional
UserIdentity        // snapshot operacional do utilizador autenticado
UserContext         // utilizador corrente resolvido
AuthorMetadata      // autoria simples: actor_id + actor_name

// Sessão
CurrentUser         // wrapper de UserProfile para contexto de utilizador corrente
CurrentSession      // sessão local com UUID v4 e timestamp UTC

// Referência orgânica
OrgUnitRef          // referência leve a unidade orgânica (sem hierarquia)

// Bridge audit
audit_actor_from_user(user: &UserProfile) -> core_audit::AuditActor

// Erro
RhError
```

### Funções de validação exportadas

```rust
validate_user_id_value(value: &str)     -> Result<(), RhError>
validate_role_id(value: &str)           -> Result<(), RhError>
validate_username(value: &str)          -> Result<(), RhError>
validate_optional_email(value: Option<&str>) -> Result<(), RhError>
validate_required_display_name(field, value, error) -> Result<(), RhError>
validate_org_unit_id(value: &str)       -> Result<(), RhError>

USER_ID_MAX_LENGTH: usize  // = 128
```

## Regras de validação

### UserId

- Não pode estar vazio nem conter espaços.
- Aceita apenas caracteres ASCII alfanuméricos e `_`, `-`, `.`.
- Comprimento máximo: 128 caracteres.

### UserProfile

- `username`: obrigatório, sem espaços.
- `display_name`: obrigatório, não pode estar vazio.
- `email`: opcional; se presente e não vazio, deve ter formato estrutural válido.
  Uma string vazia é tratada como ausência de email (sem erro).
- `roles`: pode estar vazia.

### Role

- `role_id`: obrigatório, sem espaços.
- `display_name`: obrigatório.

### OrgUnitRef

- `org_unit_id`: obrigatório; não resolve hierarquia nem valida existência.

## UserRole — serialização canónica

| Variante        | `as_str()`        | aliases em `parse()`              |
|-----------------|-------------------|------------------------------------|
| `Utilizador`    | `"utilizador"`    | `"standard"` (case-insensitive)   |
| `Auditor`       | `"auditor"`       | `"supervisor"` (case-insensitive) |
| `Administrator` | `"administrator"` | —                                 |

- `UserRole::from_str(s)` — aceita apenas o valor canónico exacto; devolve `Option`.
- `UserRole::parse(s)` — aceita aliases e é case-insensitive; devolve `Result`.
- `TryFrom<&str> for UserRole` — wraps `from_str`, devolve `Err(RhError::InvalidRole)`.

Os adapters SQLite devem usar `as_str()` / `from_str()` para garantir consistência.
`parse` é para input do utilizador (formulários, importações).

## Bridge para core-audit

```rust
pub fn audit_actor_from_user(user: &UserProfile) -> core_audit::AuditActor {
    // actor_id   = user.user_id
    // actor_name = Some(user.display_name)
    // actor_type = Some("user")
}
```

O service layer deve chamar `audit_actor_from_user` ao construir eventos de auditoria.
`UserIdentity::audit_actor()` é uma conveniência que combina `to_profile()` + `audit_actor_from_user`.

## Invariantes

- `CurrentSession::new` gera sempre um UUID v4 não nulo e timestamp UTC.
- `UserProfile::validate` é chamado em `UserProfile::new` e em `CurrentUser::new`.
- `UserIdentity::validate` revalida `UserId` via `UserId::new` — garante que identidades
  construídas manualmente (não via `TryFrom<UserProfile>`) são igualmente validadas.
- `audit_actor_from_user` assume que o perfil é válido; usa `expect` interno — nunca
  deve receber um perfil inválido.
- Não existem passwords, tokens ou segredos no modelo público.

## Decisões de design

### OrgUnitRef sem dependência de core-org

`core-rh` usa `OrgUnitRef` (id + display_name opcional) em vez de referenciar
`core-org::OrgUnit` directamente. Esta decisão evita dependência circular e
permite que `core-rh` seja consumido por crates que não conhecem `core-org`.
A resolução da hierarquia orgânica completa é responsabilidade do service layer.

### AuthorMetadata como tipo transitório

`AuthorMetadata` (`actor_id` + `actor_name`) existe para componentes que ainda não
migraram para `core_audit::AuditActor`. A migração deve ser gradual — à medida
que os contratos documentais forem modernizados, os consumidores devem passar a
usar `AuditActor` directamente via `audit_actor_from_user`.

### UserRole sem RBAC hierárquico

Os três papéis sistémicos (Utilizador, Auditor, Administrator) são intencionalmente
simples. `roles: Vec<Role>` permite roles aplicacionais adicionais, mas não implementa
herança nem hierarquia. A decisão adiada: avaliar RBAC hierárquico apenas quando
houver necessidade documentada de negócio.

### Email vazio tratado como ausência

`validate_optional_email` aceita `Some("")` como válido (trata como ausência).
Esta decisão facilita dados históricos importados com campo email em branco, sem
forçar conversão para `None` no caller.

## Erros

`RhError` cobre todos os erros de domínio:

| Variante              | Situação                                              |
|-----------------------|-------------------------------------------------------|
| `InvalidUserId`       | UserId vazio, com espaços, chars inválidos ou longo   |
| `InvalidRole`         | role_id vazio ou com espaços; display_name vazio      |
| `InvalidProfile`      | username/display_name inválidos; email mal formado    |
| `InvalidOrgRef`       | org_unit_id vazio                                     |
| `InvalidSession`      | session_id nulo ou perfil inválido na sessão          |
| `OperationFailed(s)`  | Erro de operação genérico                             |

`RhError` implementa `From<RhError> for MiniError` para conversão pelo service layer.

## Dependências

```
support-errors   — MiniError, ErrorCode, Component
core-validation  — validators de string e email
core-audit       — AuditActor (bridge em audit.rs)
```

`core-rh` não depende de `core-org`, `core-documental`, SQLite, filesystem, Tauri ou UI.

## Análise de completude

### O que está implementado

- `UserId` com validação rigorosa de formato.
- `UserProfile` com validação de todos os campos.
- `UserRole` com serialização canónica, aliases de parse e `TryFrom<&str>`.
- `Role` aplicacional com validação.
- `OrgUnitRef` como referência leve.
- `UserIdentity` / `UserContext` / `AuthorMetadata` para identidade operacional.
- `CurrentUser` / `CurrentSession` com UUID v4 e timestamp UTC.
- Bridge `audit_actor_from_user` para `core-audit`.
- Funções de validação exportadas para reutilização por crates consumidores.
- 32 testes unitários + 21 testes de integração pré-existentes.

### Lacunas conhecidas

- Sem persistência no crate — adapter `rh-sqlite` vive em `kernel/infra`.
- `AuthorMetadata` é tipo transitório; migração gradual para `AuditActor` pendente.
- Sem política formal para utilizadores técnicos/sistema (daemons, importações).
- `CurrentSession` não tem TTL nem mecanismo de expiração.
- Sem validação de unicidade de `username` — responsabilidade do adapter de persistência.

## ToDo

- Migrar consumidores de `AuthorMetadata` para `AuditActor` quando os contratos
  documentais forem modernizados.
- Definir política para utilizadores técnicos/sistema (actor de sistema vs. actor humano).
- Avaliar múltiplos papéis operacionais por utilizador se houver necessidade de RBAC.
- Definir proveniência de sessão (`session_id`) nos eventos de `core-audit` quando
  exigido pelo modelo de auditoria.
- Criar adapter de autenticação local/externa em infra, mantendo segredos fora do core.
