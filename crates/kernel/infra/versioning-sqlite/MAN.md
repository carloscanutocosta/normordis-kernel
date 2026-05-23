# MAN — versioning-sqlite

## Objectivo

Persistência SQLite de versões semânticas de módulos de aplicação. Permite registar, consultar e bumpar versões SemVer com histórico de changelog.

---

## Contrato público

```rust
pub struct VersioningSqliteStore {
    db_path: PathBuf,
}

impl VersioningSqliteStore {
    pub fn open(db_path: impl Into<PathBuf>) -> Result<Self, VersioningError>;

    /// Regista o módulo se ainda não existir (idempotente).
    pub fn ensure_app(&self, app_name: &str, initial_version: &str) -> Result<(), VersioningError>;

    /// Versão actual do módulo. None se o módulo não existir.
    pub fn get_version(&self, app_name: &str) -> Result<Option<AppVersion>, VersioningError>;

    /// Lista todas as versões do módulo (mais recente primeiro).
    pub fn list_versions(&self, app_name: &str) -> Result<Vec<AppVersion>, VersioningError>;

    /// Incrementa a versão e regista no changelog.
    pub fn bump_version(
        &self,
        app_name: &str,
        bump_type: BumpType,
        description: &str,
    ) -> Result<AppVersion, VersioningError>;

    /// Garante que o módulo está na versão mínima. Devolve erro se inferior.
    pub fn ensure_min_version(
        &self,
        app_name: &str,
        min_version: &str,
    ) -> Result<(), VersioningError>;

    /// Lista entradas do changelog para um módulo.
    pub fn list_changelog(&self, app_name: &str) -> Result<Vec<VersionChangelogEntry>, VersioningError>;

    /// Lista todo o changelog (todos os módulos).
    pub fn list_all_changelog(&self) -> Result<Vec<VersionChangelogEntry>, VersioningError>;
}

pub enum BumpType { Major, Minor, Patch }

pub struct AppVersion {
    pub app_name: String,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub bumped_at: DateTime<Utc>,
}

impl AppVersion {
    pub fn as_semver(&self) -> String; // ex: "1.2.3"
}

pub struct VersionChangelogEntry {
    pub app_name: String,
    pub from_version: Option<String>,
    pub to_version: String,
    pub bump_type: String,
    pub description: String,
    pub bumped_at: DateTime<Utc>,
}
```

---

## Schema

```sql
CREATE TABLE IF NOT EXISTS app_version (
    AppName  TEXT PRIMARY KEY,
    Major    INTEGER NOT NULL DEFAULT 0,
    Minor    INTEGER NOT NULL DEFAULT 0,
    Patch    INTEGER NOT NULL DEFAULT 0,
    BumpedAt TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS version_changelog (
    Id          TEXT PRIMARY KEY,
    AppName     TEXT NOT NULL REFERENCES app_version(AppName),
    FromVersion TEXT,
    ToVersion   TEXT NOT NULL,
    BumpType    TEXT NOT NULL,   -- 'Major' | 'Minor' | 'Patch'
    Description TEXT NOT NULL,
    BumpedAt    TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_changelog_app ON version_changelog(AppName, BumpedAt DESC);
```

---

## Como usar

```rust
use versioning_sqlite::{VersioningSqliteStore, BumpType};

let store = VersioningSqliteStore::open("platform.db")?;

// Registar módulo (idempotente)
store.ensure_app("mef", "0.1.0")?;

// Bump de versão
let v = store.bump_version("mef", BumpType::Minor, "Adicionado suporte a diploma")?;
println!("Nova versão: {}", v.as_semver()); // "0.2.0"

// Garantir versão mínima antes de operar
store.ensure_min_version("mef", "0.2.0")?;

// Consultar histórico
for entry in store.list_changelog("mef")? {
    println!("{} → {} ({}): {}", 
        entry.from_version.as_deref().unwrap_or("init"),
        entry.to_version, entry.bump_type, entry.description);
}
```

---

## Invariantes

- `ensure_app` é idempotente — não sobrescreve se o módulo já existir.
- `bump_version` incrementa atomicamente: Major zera Minor e Patch; Minor zera Patch.
- `ensure_min_version` compara semanticamente (não lexicograficamente).
- A ligação SQLite é aberta e fechada em cada operação (via `db_path`) — sem estado de ligação persistente.

---

## Limites actuais

- Uma ligação por operação — pode ser lento em uso intensivo.
- Sem suporte a pré-releases (`1.0.0-alpha.1`).
- Sem validação de formato SemVer em `initial_version` ou `min_version`.

---

## ToDo

- [ ] Pool de ligações ou ligação persistente opcional.
- [ ] Validação de SemVer na entrada.
- [ ] Suporte a pré-releases e build metadata.
