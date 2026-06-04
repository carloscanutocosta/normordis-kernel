# MAN — security-sqlite

## Objectivo

`security-sqlite` é o adapter SQLite de `core-security`. Implementa
`SecurityPolicyRepository` e `SecurityAuditLog`, persistindo políticas, delegações
temporais, revogações em cascata e decisões de autorização.

## Responsabilidade

- Executar schema/migrações do adapter.
- Persistir políticas soberanas versionadas.
- Persistir delegações com vigência, recurso opcional, condições e cadeia `granted_via`.
- Revogar delegações em cascata por CTE recursiva.
- Registar decisões de autorização para auditoria operacional com cadeia de hash local.

## Não-responsabilidade

- Não decide autorização; essa lógica pertence a `core-security`.
- Não autentica principals.
- Não fornece WORM/hash chain/assinatura probatória forte.
- Não faz purge/retention automático.
- Não gere roles; isso pertence a `rh-security-bridge` ou outro adapter.

## Contrato público

```rust
pub struct SecuritySqliteStore;

impl SecuritySqliteStore {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, SecuritySqliteError>;
    pub fn from_connection(conn: rusqlite::Connection) -> Result<Self, SecuritySqliteError>;
    pub fn migrate(&self) -> Result<(), SecuritySqliteError>;
    pub fn verify_audit_chain(&self) -> Result<bool, SecuritySqliteError>;
}

impl SecurityPolicyRepository for SecuritySqliteStore;
impl SecurityAuditLog for SecuritySqliteStore;

pub const SECURITY_SQLITE_MIGRATIONS: &[&str];
```

## Migrações

O adapter usa uma tabela local `_security_sqlite_migrations` com nomes estáveis,
independentes do texto SQL. Campos adicionados por `ALTER TABLE` são aplicados
apenas quando a coluna ainda não existe, evitando falhas em bases já migradas.

Migrações actuais:

1. `security_sqlite_001_base`: `security_policies`, `security_delegations` e índices base.
2. `security_sqlite_002_delegation_chain`: coluna `granted_via` e índice de cadeia.
3. `security_sqlite_003_auth_decisions`: `security_auth_decisions` e índices.
4. `security_sqlite_004_policy_validity`: `valid_from` e `valid_to` em políticas.
5. `security_sqlite_005_evidence_level`: `evidence_level` em decisões.
6. `security_sqlite_006_audit_hash_chain`: `previous_hash` e `entry_hash`.

## Invariantes

- `open()` executa migrações antes de devolver o store.
- Políticas revogadas não são eliminadas.
- Delegações revogadas não são eliminadas.
- `list_delegations()` devolve apenas delegações activas no instante fornecido.
- `revoke_delegation()` revoga a delegação raiz e descendentes via `granted_via`.
- Decisões de autorização são inseridas, não actualizadas pela API pública.
- Cada nova decisão guarda `previous_hash` e `entry_hash` SHA-256 sobre payload canónico.
- `verify_audit_chain()` detecta alteração local de payload ou quebra de cadeia.

## Integração recomendada

```rust
let repo = SecuritySqliteStore::open(&config)?;
let audit = SecuritySqliteStore::open(&config)?;
let svc = SecurityService::with_audit(repo, audit);
```

Para produção, manter `SecurityRuntimePolicy::production()` e garantir que o
adapter de auditoria está disponível antes de aceitar operações sensíveis.

## Limites actuais

- A cadeia de hash local aumenta integridade operacional, mas não substitui WORM,
  assinatura externa, retenção legal ou custódia probatória em `core-audit`.
- Usa `Arc<Mutex<rusqlite::Connection>>`; para carga concorrente alta considerar
  pool ou adapter assíncrono dedicado.
- Sem encriptação própria da base de dados.
- Sem política de retenção/purge.

## Validação

```sh
cargo test -p security-sqlite
cargo clippy -p security-sqlite --all-targets -- -D warnings
```
