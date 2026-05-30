# core-rh

Domínio de identidade e gestão de utilizadores do Mini-Kernel RS.

## Responsabilidade

- Identificador canónico de utilizador (`UserId`) com validação de formato.
- Perfil de utilizador (`UserProfile`) com papel sistémico, roles aplicacionais e referência orgânica opcional.
- Papéis sistémicos (`UserRole`: Utilizador, Auditor, Administrator) com serialização canónica e aliases de parse.
- Catálogo gerido de roles funcionais: `RoleId` (identificador validado), `Role` (com `is_active`), `RoleRepository` (port implementado por `rh-sqlite`).
  O catálogo é a fonte de verdade que impede roles arbitrários nas apps — o `AppRegistryService` valida sempre contra ele.
- Identidade operacional (`UserIdentity`) e contexto de sessão corrente (`CurrentSession`).
- Metadados de autoria (`AuthorMetadata`) usados por outros crates para rastrear quem praticou um acto.
- Referência leve a unidade orgânica (`OrgUnitRef`) sem dependência directa de `core-org`.
- Bridge para `core-audit`: `audit_actor_from_user` converte um `UserProfile` num `AuditActor`.
- Funções de validação de primitivos: `validate_user_id_value`, `validate_username`, `validate_optional_email`, etc.

## Não responsabilidade

- Não conhece SQLite, filesystem, Tauri ou UI.
- Não implementa autenticação, LDAP, OAuth, OIDC, SSO, passwords ou tokens.
- Não gere sessões persistentes — `CurrentSession` é um valor em memória.
- Não detém a estrutura orgânica completa — usa `OrgUnitRef` como referência leve; a hierarquia pertence a `core-org`.
- Não implementa RBAC hierárquico — papéis sistémicos são simples e sem herança.
- Não persiste o catálogo de roles — a persistência pertence a `rh-sqlite`.

## Exemplo mínimo

```rust
use core_rh::{UserId, UserProfile, UserRole, UserIdentity};

let user_id = UserId::new("joao.silva")?;
let profile = UserProfile::new(
    user_id,
    "joao.silva",
    "João Silva",
    Some("joao@example.com".into()),
    UserRole::Utilizador,
    vec![],
    None,
)?;

let identity = UserIdentity::try_from(profile)?;
let meta = identity.author_metadata(); // AuthorMetadata { actor_id: "joao.silva", actor_name: "João Silva" }
```
