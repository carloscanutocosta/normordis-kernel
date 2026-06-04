# MAN — rh-security-bridge

## Objectivo

`rh-security-bridge` liga dados de RH/organização ao domínio de autorização de
`core-security`. Implementa memberships principal→role e validação de âmbito
orgânico para decisões contextuais.

## Responsabilidade

- Persistir atribuições de roles com validade temporal.
- Revogar memberships sem apagar histórico.
- Implementar `RoleMembershipRepository`.
- Implementar `OrgScopeValidator` consultando afectações RH.
- Resolver opcionalmente principal IAM/técnico para pessoa RH.

## Não-responsabilidade

- Não decide autorização.
- Não autentica principals.
- Não sincroniza automaticamente AD/LDAP/Entra ID.
- Não modela cargos, contratos ou organigramas; consulta a tabela RH existente.

## Contrato público

```rust
pub struct RhSecurityBridgeStore;
pub struct MemberId(pub String);

impl RhSecurityBridgeStore {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, RhSecurityBridgeError>;
    pub fn from_connection(conn: rusqlite::Connection) -> Result<Self, RhSecurityBridgeError>;
    pub fn migrate(&self) -> Result<(), RhSecurityBridgeError>;

    pub fn assign_principal_to_role(
        &self,
        principal_id: &str,
        role_id: &RoleId,
        assigned_by: &str,
        valid_from: DateTime<Utc>,
        valid_to: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> Result<MemberId, RhSecurityBridgeError>;

    pub fn revoke_membership(
        &self,
        member_id: &MemberId,
        revoked_by: &str,
        now: DateTime<Utc>,
    ) -> Result<(), RhSecurityBridgeError>;

    pub fn link_principal_to_person(
        &self,
        principal_id: &str,
        person_id: &str,
        linked_by: &str,
        now: DateTime<Utc>,
    ) -> Result<(), RhSecurityBridgeError>;
}

impl RoleMembershipRepository for RhSecurityBridgeStore;
impl OrgScopeValidator for RhSecurityBridgeStore;
```

## Identidade principal → pessoa RH

`OrgScopeValidator` consulta `person_assignment`. Em produção, o principal de IAM
pode não ser o `person_id` RH. Para isso, usar `link_principal_to_person()`.

Quando não existe ligação explícita, o adapter mantém fallback compatível:
`principal_id` é usado como `person_id`. Este fallback deve ser uma convenção
deliberada, não uma suposição escondida.

## Migrações

O adapter usa `_rh_security_bridge_migrations` com nomes estáveis:

1. `rh_security_bridge_001_role_members`: memberships de roles.
2. `rh_security_bridge_002_principal_person_links`: ligação principal→pessoa.

## Integração

```rust
let repo = SecuritySqliteStore::open(&security_config)?;
let audit = SecuritySqliteStore::open(&security_config)?;
let roles = RhSecurityBridgeStore::open(&rh_config)?;

let svc = SecurityService::with_all(repo, audit, roles);
```

Para `OrgScopeValidator`, a base de dados deve conter a tabela `person_assignment`
do domínio RH, com `person_id`, `unit_id`, `valid_from` e `valid_until`.

## Invariantes

- Memberships revogados ficam marcados; não são removidos.
- `valid_from <= now` e `valid_to > now` determinam vigência.
- `valid_to = None` significa sem expiração.
- `MemberId` é gerado internamente.
- A validação de scope usa data civil UTC no formato `YYYY-MM-DD`, compatível com
  as afectações RH existentes.

## Limites actuais

- Roles são strings opacas; a existência do role é governada fora do adapter.
- Não há importação/sincronização batch.
- Não há notificação automática de expiração.
- A consulta de `person_assignment` é dependente do schema RH documentado.

## Validação

```sh
cargo test -p rh-security-bridge
cargo clippy -p rh-security-bridge --all-targets -- -D warnings
```
