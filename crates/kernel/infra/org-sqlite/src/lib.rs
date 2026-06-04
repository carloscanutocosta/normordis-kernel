//! Adaptador SQLite para `core-org`.
//!
//! ## Arquitectura de acesso à BD
//!
//! `OrgSqliteStore` envolve uma `Connection` em `Arc<Mutex<>>`, seguindo o padrão
//! dos restantes adapters do kernel (security-sqlite, metrics-sqlite). Isto torna
//! o store `Send + Sync + Clone` e thread-safe.
//!
//! - **Leituras e escritas simples**: executadas sob o `Mutex`.
//! - **Operações multi-passo** (ex.: `deactivate`, que verifica filhos/posições
//!   antes de escrever): envolvidas numa transacção `IMMEDIATE` para garantir
//!   atomicidade e eliminar a janela TOCTOU.
//!
//! Usa os helpers de `adapter-sqlite` (`open_relational_connection`,
//! `run_relational_migrations`) para abertura e migração.

use adapter_sqlite::{
    open_relational_connection, run_relational_migrations, SqliteRelationalConfig,
};
use chrono::{NaiveDate, TimeZone, Utc};
use core_audit::{
    AuditActor, AuditEvent, AuditOutcome, AuditStore, AuditTarget, ControlCategory,
    ControlDefinition, ControlExecution, ControlExecutionResult, ControlRegistryStore,
    ControlSeverity,
};
use core_org::{
    Competency, CompetencyId, CompetencyRepository, Delegation, DelegationId, DelegationRepository,
    InstrumentKind, LegalInstrument, LegalInstrumentId, LegalInstrumentRepository, OrgAddress,
    OrgAuditEvent, OrgAuditOutbox, OrgAuditPort, OrgContacts, OrgDomainEvent, OrgDomainEventPort,
    OrgError, OrgEventOutcome, OrgLevel, OrgPage, OrgPosition, OrgPositionId,
    OrgPositionRepository, OrgPositionStatus, OrgUnit, OrgUnitId, OrgUnitRepository, OrgUnitStatus,
    PagedResult, PositionKind,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::sync::{Arc, Mutex, MutexGuard};
use thiserror::Error;

// ── Migrations ────────────────────────────────────────────────────────────────
//
// CONTRATO DE MIGRAÇÃO — **append-only**.
//   Cada entrada é imutável depois de existir: NUNCA editar uma migração já
//   presente (o runner é hash-tracked; editar in-place não actualiza bases já
//   migradas). Mudanças de schema entram SEMPRE como nova entrada no fim, com
//   `ALTER TABLE ADD COLUMN` (colunas) ou `CREATE TABLE` (tabelas novas).
//   M0 é a baseline de entidades; M1 acrescenta OCC a competencies/delegations
//   e a infraestrutura de outbox (auditoria + eventos de domínio).

pub const ORG_SQLITE_MIGRATIONS: &[&str] = &[
    // ── M0 — baseline de entidades ────────────────────────────────────────────
    r#"
    CREATE TABLE IF NOT EXISTS legal_instruments (
        instrument_id   TEXT PRIMARY KEY,
        kind            TEXT NOT NULL,
        reference       TEXT NOT NULL,
        date            TEXT NOT NULL,
        description     TEXT NOT NULL,
        effective_from  TEXT NOT NULL,
        effective_until TEXT
    );

    CREATE TABLE IF NOT EXISTS org_units (
        unit_id         TEXT PRIMARY KEY,
        short_name      TEXT NOT NULL,
        full_name       TEXT NOT NULL,
        service_code    TEXT,
        level           INTEGER NOT NULL CHECK (level >= 1),
        parent_id       TEXT REFERENCES org_units(unit_id),
        created_by      TEXT REFERENCES legal_instruments(instrument_id),
        legal_reference TEXT,
        valid_from      TEXT NOT NULL,
        valid_until     TEXT,
        status          TEXT NOT NULL DEFAULT 'active',
        email           TEXT,
        phone           TEXT,
        fax             TEXT,
        rua             TEXT,
        numero          TEXT,
        porta           TEXT,
        local           TEXT,
        cp4             TEXT,
        cp3             TEXT,
        localidade      TEXT,
        version         INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE IF NOT EXISTS org_positions (
        position_id     TEXT PRIMARY KEY,
        code            TEXT NOT NULL UNIQUE,
        title           TEXT NOT NULL,
        kind            TEXT NOT NULL DEFAULT 'outro',
        substitutes     TEXT REFERENCES org_positions(position_id),
        status          TEXT NOT NULL DEFAULT 'active',
        unit_id         TEXT NOT NULL REFERENCES org_units(unit_id),
        created_by      TEXT NOT NULL REFERENCES legal_instruments(instrument_id),
        valid_from      TEXT NOT NULL,
        valid_until     TEXT,
        version         INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE IF NOT EXISTS competencies (
        competency_id   TEXT PRIMARY KEY,
        code            TEXT NOT NULL,
        description     TEXT NOT NULL,
        scope           TEXT NOT NULL,
        assigned_to     TEXT NOT NULL REFERENCES org_positions(position_id),
        granted_by      TEXT NOT NULL REFERENCES legal_instruments(instrument_id),
        valid_from      TEXT NOT NULL,
        valid_until     TEXT
    );

    CREATE TABLE IF NOT EXISTS delegations (
        delegation_id   TEXT PRIMARY KEY,
        competency_id   TEXT NOT NULL REFERENCES competencies(competency_id),
        from_position   TEXT NOT NULL REFERENCES org_positions(position_id),
        to_position     TEXT NOT NULL REFERENCES org_positions(position_id),
        instrument_id   TEXT NOT NULL REFERENCES legal_instruments(instrument_id),
        valid_from      TEXT NOT NULL,
        valid_until     TEXT
    );

    CREATE INDEX IF NOT EXISTS idx_org_units_level     ON org_units (level);
    CREATE INDEX IF NOT EXISTS idx_org_units_parent    ON org_units (parent_id);
    CREATE INDEX IF NOT EXISTS idx_org_units_valid     ON org_units (valid_from, valid_until);
    CREATE INDEX IF NOT EXISTS idx_org_units_name      ON org_units (short_name, full_name);
    CREATE INDEX IF NOT EXISTS idx_org_positions_unit  ON org_positions (unit_id, valid_from, valid_until);
    CREATE INDEX IF NOT EXISTS idx_org_positions_kind  ON org_positions (kind, status);
    CREATE INDEX IF NOT EXISTS idx_org_positions_subs  ON org_positions (substitutes);
    CREATE INDEX IF NOT EXISTS idx_competencies_pos    ON competencies (assigned_to, valid_from, valid_until);
    CREATE INDEX IF NOT EXISTS idx_delegations_to      ON delegations (to_position, valid_from, valid_until);
    "#,
    // ── M1 — OCC em competencies/delegations + outbox (auditoria + eventos) ─────
    // Aditiva: ALTER para colunas em tabelas existentes, CREATE para tabelas novas.
    // `delivered`: 0 = pendente, 1 = entregue, 2 = dead-letter (esgotou tentativas).
    r#"
    ALTER TABLE competencies ADD COLUMN version INTEGER NOT NULL DEFAULT 0;
    ALTER TABLE delegations  ADD COLUMN version INTEGER NOT NULL DEFAULT 0;

    CREATE TABLE IF NOT EXISTS org_audit_outbox (
        seq         INTEGER PRIMARY KEY AUTOINCREMENT,
        event_json  TEXT NOT NULL,
        created_at  TEXT NOT NULL,
        delivered   INTEGER NOT NULL DEFAULT 0,
        attempts    INTEGER NOT NULL DEFAULT 0,
        last_error  TEXT
    );

    CREATE TABLE IF NOT EXISTS org_domain_outbox (
        seq         INTEGER PRIMARY KEY AUTOINCREMENT,
        event_json  TEXT NOT NULL,
        created_at  TEXT NOT NULL,
        delivered   INTEGER NOT NULL DEFAULT 0,
        attempts    INTEGER NOT NULL DEFAULT 0,
        last_error  TEXT
    );

    CREATE INDEX IF NOT EXISTS idx_org_audit_outbox_pending  ON org_audit_outbox (delivered, seq);
    CREATE INDEX IF NOT EXISTS idx_org_domain_outbox_pending ON org_domain_outbox (delivered, seq);
    "#,
];

/// Tentativas de entrega antes de um evento ser marcado dead-letter (`delivered=2`).
pub const MAX_OUTBOX_ATTEMPTS: i64 = 5;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum OrgSqliteError {
    #[error(transparent)]
    Adapter(#[from] support_errors::MiniError),
    #[error("erro SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("data inválida: {0}")]
    InvalidDate(String),
    #[error("tipo de instrumento desconhecido: {0}")]
    UnknownInstrumentKind(String),
    #[error("estado desconhecido: {0}")]
    UnknownStatus(String),
    #[error("tipo de cargo desconhecido: {0}")]
    UnknownPositionKind(String),
}

impl From<OrgSqliteError> for OrgError {
    fn from(e: OrgSqliteError) -> Self {
        OrgError::OperationFailed(e.to_string())
    }
}

// ── Store ─────────────────────────────────────────────────────────────────────

/// Adaptador SQLite para `core-org`. Thread-safe e clonável via `Arc<Mutex<Connection>>`.
#[derive(Clone)]
pub struct OrgSqliteStore {
    conn: Arc<Mutex<Connection>>,
}

impl OrgSqliteStore {
    pub fn open(config: &SqliteRelationalConfig) -> Result<Self, OrgSqliteError> {
        let conn = open_relational_connection(config)?;
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn from_connection(conn: Connection) -> Result<Self, OrgSqliteError> {
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn migrate(&self) -> Result<(), OrgSqliteError> {
        let conn = self.lock_raw()?;
        run_relational_migrations(&conn, ORG_SQLITE_MIGRATIONS)?;
        Ok(())
    }

    fn lock_raw(&self) -> Result<MutexGuard<'_, Connection>, OrgSqliteError> {
        self.conn
            .lock()
            .map_err(|_| OrgSqliteError::Sqlite(rusqlite::Error::InvalidQuery))
    }

    /// Bloqueia a ligação para uma operação. Erro se o mutex estiver envenenado.
    fn lock(&self) -> Result<MutexGuard<'_, Connection>, OrgError> {
        self.conn
            .lock()
            .map_err(|_| OrgError::OperationFailed("connection mutex poisoned".into()))
    }
}

// ── Helpers de conversão ──────────────────────────────────────────────────────

fn date_to_str(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

fn str_to_date(s: &str) -> Result<NaiveDate, OrgSqliteError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| OrgSqliteError::InvalidDate(s.to_string()))
}

fn opt_str_to_date(s: Option<String>) -> Result<Option<NaiveDate>, OrgSqliteError> {
    s.as_deref().map(str_to_date).transpose()
}

fn kind_to_str(k: &InstrumentKind) -> String {
    match k {
        InstrumentKind::Portaria => "portaria".into(),
        InstrumentKind::Despacho => "despacho".into(),
        InstrumentKind::Deliberacao => "deliberacao".into(),
        InstrumentKind::RegulamentoOrganico => "regulamento_organico".into(),
        InstrumentKind::Outro(s) => format!("outro:{s}"),
    }
}

fn str_to_kind(s: &str) -> Result<InstrumentKind, OrgSqliteError> {
    match s {
        "portaria" => Ok(InstrumentKind::Portaria),
        "despacho" => Ok(InstrumentKind::Despacho),
        "deliberacao" => Ok(InstrumentKind::Deliberacao),
        "regulamento_organico" => Ok(InstrumentKind::RegulamentoOrganico),
        other if other.starts_with("outro:") => Ok(InstrumentKind::Outro(other[6..].to_string())),
        other => Err(OrgSqliteError::UnknownInstrumentKind(other.to_string())),
    }
}

fn status_to_str(s: &OrgUnitStatus) -> &'static str {
    match s {
        OrgUnitStatus::Active => "active",
        OrgUnitStatus::Suspended => "suspended",
        OrgUnitStatus::Extinct => "extinct",
    }
}

fn str_to_status(s: &str) -> Result<OrgUnitStatus, OrgSqliteError> {
    match s {
        "active" => Ok(OrgUnitStatus::Active),
        "suspended" => Ok(OrgUnitStatus::Suspended),
        "extinct" => Ok(OrgUnitStatus::Extinct),
        other => Err(OrgSqliteError::UnknownStatus(other.to_string())),
    }
}

fn pos_status_to_str(s: &OrgPositionStatus) -> &'static str {
    match s {
        OrgPositionStatus::Active => "active",
        OrgPositionStatus::Suspended => "suspended",
        OrgPositionStatus::Extinct => "extinct",
    }
}

fn str_to_pos_status(s: &str) -> Result<OrgPositionStatus, OrgSqliteError> {
    OrgPositionStatus::from_str(s).ok_or_else(|| OrgSqliteError::UnknownStatus(s.to_string()))
}

fn pos_kind_to_str(k: &PositionKind) -> String {
    k.as_str()
}

fn str_to_pos_kind(s: &str) -> Result<PositionKind, OrgSqliteError> {
    PositionKind::from_str(s).ok_or_else(|| OrgSqliteError::UnknownPositionKind(s.to_string()))
}

/// Mapeia erros genéricos para OrgError::OperationFailed.
fn op<E: std::fmt::Display>(e: E) -> OrgError {
    OrgError::OperationFailed(e.to_string())
}

// ── OrgUnit SELECT (22 colunas) ───────────────────────────────────────────────

const UNIT_SELECT: &str = "unit_id, short_name, full_name, service_code, level, parent_id, \
     created_by, legal_reference, valid_from, valid_until, status, \
     email, phone, fax, rua, numero, porta, local, cp4, cp3, localidade, version";

const UNIT_SELECT_U: &str =
    "u.unit_id, u.short_name, u.full_name, u.service_code, u.level, u.parent_id, \
     u.created_by, u.legal_reference, u.valid_from, u.valid_until, u.status, \
     u.email, u.phone, u.fax, u.rua, u.numero, u.porta, u.local, u.cp4, u.cp3, u.localidade, u.version";

type UnitRow = (
    String,
    String,
    String,
    Option<String>,
    i64,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    i64,
);

fn row_to_unit(r: UnitRow) -> Result<OrgUnit, OrgError> {
    let (
        id_s,
        short_name,
        full_name,
        service_code,
        level_n,
        parent_s,
        created_by_s,
        legal_reference,
        from_s,
        until_s,
        status_s,
        email,
        phone,
        fax,
        rua,
        numero,
        porta,
        local,
        cp4,
        cp3,
        localidade,
        version_n,
    ) = r;
    Ok(OrgUnit {
        id: OrgUnitId(id_s),
        short_name,
        full_name,
        service_code,
        level: OrgLevel::new(level_n as u8)?,
        parent_id: parent_s.map(OrgUnitId),
        created_by: created_by_s.map(LegalInstrumentId),
        legal_reference,
        valid_from: str_to_date(&from_s).map_err(op)?,
        valid_until: opt_str_to_date(until_s).map_err(op)?,
        status: str_to_status(&status_s).map_err(op)?,
        contacts: OrgContacts {
            email,
            phone,
            fax,
            address: OrgAddress {
                rua,
                numero,
                porta,
                local,
                cp4,
                cp3,
                localidade,
            },
        },
        version: version_n as u32,
    })
}

macro_rules! read_unit_row {
    ($r:expr) => {{
        (
            $r.get::<_, String>(0)?,
            $r.get::<_, String>(1)?,
            $r.get::<_, String>(2)?,
            $r.get::<_, Option<String>>(3)?,
            $r.get::<_, i64>(4)?,
            $r.get::<_, Option<String>>(5)?,
            $r.get::<_, Option<String>>(6)?,
            $r.get::<_, Option<String>>(7)?,
            $r.get::<_, String>(8)?,
            $r.get::<_, Option<String>>(9)?,
            $r.get::<_, String>(10)?,
            $r.get::<_, Option<String>>(11)?,
            $r.get::<_, Option<String>>(12)?,
            $r.get::<_, Option<String>>(13)?,
            $r.get::<_, Option<String>>(14)?,
            $r.get::<_, Option<String>>(15)?,
            $r.get::<_, Option<String>>(16)?,
            $r.get::<_, Option<String>>(17)?,
            $r.get::<_, Option<String>>(18)?,
            $r.get::<_, Option<String>>(19)?,
            $r.get::<_, Option<String>>(20)?,
            $r.get::<_, i64>(21)?,
        )
    }};
}

fn collect_units(
    rows: impl Iterator<Item = Result<UnitRow, rusqlite::Error>>,
) -> Result<Vec<OrgUnit>, OrgError> {
    let mut result = Vec::new();
    for row in rows {
        result.push(row_to_unit(row.map_err(op)?)?);
    }
    Ok(result)
}

// ── OrgPosition SELECT (11 colunas) ──────────────────────────────────────────

const POS_SELECT: &str = "position_id, code, title, kind, substitutes, status, \
     unit_id, created_by, valid_from, valid_until, version";

type PosRow = (
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    String,
    Option<String>,
    i64,
);

fn row_to_position(r: PosRow) -> Result<OrgPosition, OrgError> {
    let (id_s, code, title, kind_s, subs_s, status_s, unit_s, created_s, from_s, until_s, ver) = r;
    Ok(OrgPosition {
        id: OrgPositionId(id_s),
        code,
        title,
        kind: str_to_pos_kind(&kind_s).map_err(op)?,
        substitutes: subs_s.map(OrgPositionId),
        status: str_to_pos_status(&status_s).map_err(op)?,
        unit_id: OrgUnitId(unit_s),
        created_by: LegalInstrumentId(created_s),
        valid_from: str_to_date(&from_s).map_err(op)?,
        valid_until: opt_str_to_date(until_s).map_err(op)?,
        version: ver as u32,
    })
}

macro_rules! read_pos_row {
    ($r:expr) => {{
        (
            $r.get::<_, String>(0)?,
            $r.get::<_, String>(1)?,
            $r.get::<_, String>(2)?,
            $r.get::<_, String>(3)?,
            $r.get::<_, Option<String>>(4)?,
            $r.get::<_, String>(5)?,
            $r.get::<_, String>(6)?,
            $r.get::<_, String>(7)?,
            $r.get::<_, String>(8)?,
            $r.get::<_, Option<String>>(9)?,
            $r.get::<_, i64>(10)?,
        )
    }};
}

fn collect_positions(
    rows: impl Iterator<Item = Result<PosRow, rusqlite::Error>>,
) -> Result<Vec<OrgPosition>, OrgError> {
    let mut result = Vec::new();
    for row in rows {
        result.push(row_to_position(row.map_err(op)?)?);
    }
    Ok(result)
}

// ── Helpers de unit params (partilhado por create/update) ─────────────────────

macro_rules! unit_params {
    ($u:expr) => {{
        let c = &$u.contacts;
        let a = &c.address;
        (
            $u.id.as_str().to_string(),
            $u.short_name.clone(),
            $u.full_name.clone(),
            $u.service_code.clone(),
            $u.level.as_u8() as i64,
            $u.parent_id.as_ref().map(|p| p.as_str().to_string()),
            $u.created_by.as_ref().map(|cb| cb.as_str().to_string()),
            $u.legal_reference.clone(),
            date_to_str($u.valid_from),
            $u.valid_until.map(date_to_str),
            status_to_str(&$u.status).to_string(),
            c.email.clone(),
            c.phone.clone(),
            c.fax.clone(),
            a.rua.clone(),
            a.numero.clone(),
            a.porta.clone(),
            a.local.clone(),
            a.cp4.clone(),
            a.cp3.clone(),
            a.localidade.clone(),
        )
    }};
}

// ── State writes reutilizáveis (Connection ou Transaction via Deref) ──────────

fn outbox_insert(conn: &Connection, event: &OrgAuditEvent) -> Result<(), OrgError> {
    let json = serde_json::to_string(event).map_err(op)?;
    conn.execute(
        "INSERT INTO org_audit_outbox (event_json, created_at, delivered) VALUES (?1, ?2, 0)",
        params![json, event.occurred_at.to_rfc3339()],
    )
    .map_err(op)?;
    Ok(())
}

fn domain_outbox_insert(conn: &Connection, event: &OrgDomainEvent) -> Result<(), OrgError> {
    let json = serde_json::to_string(event).map_err(op)?;
    conn.execute(
        "INSERT INTO org_domain_outbox (event_json, created_at, delivered)
         VALUES (?1, ?2, 0)",
        params![json, Utc::now().to_rfc3339()],
    )
    .map_err(op)?;
    Ok(())
}

fn unit_insert(conn: &Connection, u: &OrgUnit) -> Result<(), OrgError> {
    let (id, sn, fn_, sc, lv, pi, cb, lr, vf, vu, st, em, ph, fx, ru, nu, po, lo, c4, c3, lc) =
        unit_params!(u);
    let affected = conn
        .execute(
            "INSERT OR IGNORE INTO org_units
                 (unit_id, short_name, full_name, service_code, level, parent_id,
                  created_by, legal_reference, valid_from, valid_until, status,
                  email, phone, fax, rua, numero, porta, local, cp4, cp3, localidade)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21)",
            params![
                id, sn, fn_, sc, lv, pi, cb, lr, vf, vu, st, em, ph, fx, ru, nu, po, lo, c4, c3, lc
            ],
        )
        .map_err(op)?;
    if affected == 0 {
        return Err(OrgError::AlreadyExists(u.id.as_str().into()));
    }
    Ok(())
}

fn unit_update_occ(conn: &Connection, u: &OrgUnit) -> Result<(), OrgError> {
    let (id, sn, fn_, sc, lv, pi, cb, lr, vf, vu, st, em, ph, fx, ru, nu, po, lo, c4, c3, lc) =
        unit_params!(u);
    let id_clone = id.clone();
    let affected = conn
        .execute(
            "UPDATE org_units SET
                 short_name=?2, full_name=?3, service_code=?4,
                 level=?5, parent_id=?6, created_by=?7, legal_reference=?8,
                 valid_from=?9, valid_until=?10, status=?11,
                 email=?12, phone=?13, fax=?14,
                 rua=?15, numero=?16, porta=?17, local=?18,
                 cp4=?19, cp3=?20, localidade=?21,
                 version=version+1
             WHERE unit_id=?1 AND version=?22",
            params![
                id,
                sn,
                fn_,
                sc,
                lv,
                pi,
                cb,
                lr,
                vf,
                vu,
                st,
                em,
                ph,
                fx,
                ru,
                nu,
                po,
                lo,
                c4,
                c3,
                lc,
                u.version as i64
            ],
        )
        .map_err(op)?;
    if affected == 0 {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM org_units WHERE unit_id=?1",
                params![id_clone],
                |r| r.get(0),
            )
            .map_err(op)?;
        return if exists > 0 {
            Err(OrgError::VersionConflict(u.id.as_str().into()))
        } else {
            Err(OrgError::UnitNotFound(u.id.as_str().into()))
        };
    }
    Ok(())
}

fn unit_upsert(conn: &Connection, u: &OrgUnit) -> Result<(), OrgError> {
    let (id, sn, fn_, sc, lv, pi, cb, lr, vf, vu, st, em, ph, fx, ru, nu, po, lo, c4, c3, lc) =
        unit_params!(u);
    conn.execute(
        "INSERT INTO org_units
             (unit_id, short_name, full_name, service_code, level, parent_id,
              created_by, legal_reference, valid_from, valid_until, status,
              email, phone, fax, rua, numero, porta, local, cp4, cp3, localidade)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21)
         ON CONFLICT(unit_id) DO UPDATE SET
             short_name=excluded.short_name, full_name=excluded.full_name,
             service_code=excluded.service_code, level=excluded.level,
             parent_id=excluded.parent_id, created_by=excluded.created_by,
             legal_reference=excluded.legal_reference,
             valid_from=excluded.valid_from, valid_until=excluded.valid_until,
             status=excluded.status,
             email=excluded.email, phone=excluded.phone, fax=excluded.fax,
             rua=excluded.rua, numero=excluded.numero, porta=excluded.porta,
             local=excluded.local, cp4=excluded.cp4, cp3=excluded.cp3,
             localidade=excluded.localidade",
        params![
            id, sn, fn_, sc, lv, pi, cb, lr, vf, vu, st, em, ph, fx, ru, nu, po, lo, c4, c3, lc
        ],
    )
    .map_err(op)?;
    Ok(())
}

fn unit_deactivate(
    conn: &Connection,
    id: &OrgUnitId,
    valid_until: NaiveDate,
) -> Result<(), OrgError> {
    let children: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM org_units WHERE parent_id=?1 AND status='active'",
            params![id.as_str()],
            |r| r.get(0),
        )
        .map_err(op)?;
    if children > 0 {
        return Err(OrgError::CannotDeactivateWithActiveChildren);
    }
    let date_s = date_to_str(valid_until);
    let positions: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM org_positions
             WHERE unit_id=?1 AND (valid_until IS NULL OR valid_until > ?2)",
            params![id.as_str(), &date_s],
            |r| r.get(0),
        )
        .map_err(op)?;
    if positions > 0 {
        return Err(OrgError::CannotDeactivateWithActivePositions);
    }
    let affected = conn
        .execute(
            "UPDATE org_units SET valid_until=?1, status='extinct', version=version+1
             WHERE unit_id=?2 AND status != 'extinct'",
            params![date_s, id.as_str()],
        )
        .map_err(op)?;
    if affected == 0 {
        return Err(OrgError::UnitNotFound(id.as_str().into()));
    }
    Ok(())
}

fn pos_insert(conn: &Connection, p: &OrgPosition) -> Result<(), OrgError> {
    let affected = conn
        .execute(
            "INSERT OR IGNORE INTO org_positions
                 (position_id, code, title, kind, substitutes, status,
                  unit_id, created_by, valid_from, valid_until)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                p.id.as_str(),
                p.code,
                p.title,
                pos_kind_to_str(&p.kind),
                p.substitutes.as_ref().map(|s| s.as_str()),
                pos_status_to_str(&p.status),
                p.unit_id.as_str(),
                p.created_by.as_str(),
                date_to_str(p.valid_from),
                p.valid_until.map(date_to_str),
            ],
        )
        .map_err(op)?;
    if affected == 0 {
        return Err(OrgError::AlreadyExists(p.id.as_str().into()));
    }
    Ok(())
}

fn pos_update_occ(conn: &Connection, p: &OrgPosition) -> Result<(), OrgError> {
    let affected = conn
        .execute(
            "UPDATE org_positions SET
                 code=?2, title=?3, kind=?4, substitutes=?5, status=?6,
                 unit_id=?7, created_by=?8, valid_from=?9, valid_until=?10,
                 version=version+1
             WHERE position_id=?1 AND version=?11",
            params![
                p.id.as_str(),
                p.code,
                p.title,
                pos_kind_to_str(&p.kind),
                p.substitutes.as_ref().map(|s| s.as_str()),
                pos_status_to_str(&p.status),
                p.unit_id.as_str(),
                p.created_by.as_str(),
                date_to_str(p.valid_from),
                p.valid_until.map(date_to_str),
                p.version as i64,
            ],
        )
        .map_err(op)?;
    if affected == 0 {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM org_positions WHERE position_id=?1",
                params![p.id.as_str()],
                |r| r.get(0),
            )
            .map_err(op)?;
        return if exists > 0 {
            Err(OrgError::VersionConflict(p.id.as_str().into()))
        } else {
            Err(OrgError::PositionNotFound(p.id.as_str().into()))
        };
    }
    Ok(())
}

fn pos_upsert(conn: &Connection, p: &OrgPosition) -> Result<(), OrgError> {
    conn.execute(
        "INSERT INTO org_positions
             (position_id, code, title, kind, substitutes, status,
              unit_id, created_by, valid_from, valid_until)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
         ON CONFLICT(position_id) DO UPDATE SET
             code=excluded.code, title=excluded.title,
             kind=excluded.kind, substitutes=excluded.substitutes,
             status=excluded.status, unit_id=excluded.unit_id,
             created_by=excluded.created_by,
             valid_from=excluded.valid_from, valid_until=excluded.valid_until",
        params![
            p.id.as_str(),
            p.code,
            p.title,
            pos_kind_to_str(&p.kind),
            p.substitutes.as_ref().map(|s| s.as_str()),
            pos_status_to_str(&p.status),
            p.unit_id.as_str(),
            p.created_by.as_str(),
            date_to_str(p.valid_from),
            p.valid_until.map(date_to_str),
        ],
    )
    .map_err(op)?;
    Ok(())
}

fn pos_deactivate(
    conn: &Connection,
    id: &OrgPositionId,
    valid_until: NaiveDate,
) -> Result<(), OrgError> {
    let affected = conn
        .execute(
            "UPDATE org_positions SET valid_until=?1, status='extinct', version=version+1
             WHERE position_id=?2 AND status != 'extinct'",
            params![date_to_str(valid_until), id.as_str()],
        )
        .map_err(op)?;
    if affected == 0 {
        return Err(OrgError::PositionNotFound(id.as_str().into()));
    }
    Ok(())
}

fn comp_upsert(conn: &Connection, c: &Competency) -> Result<(), OrgError> {
    conn.execute(
        "INSERT INTO competencies
             (competency_id, code, description, scope, assigned_to, granted_by,
              valid_from, valid_until)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
         ON CONFLICT(competency_id) DO UPDATE SET
             code=excluded.code, description=excluded.description,
             scope=excluded.scope, assigned_to=excluded.assigned_to,
             granted_by=excluded.granted_by,
             valid_from=excluded.valid_from, valid_until=excluded.valid_until",
        params![
            c.id.as_str(),
            c.code,
            c.description,
            c.scope,
            c.assigned_to.as_str(),
            c.granted_by.as_str(),
            date_to_str(c.valid_from),
            c.valid_until.map(date_to_str),
        ],
    )
    .map_err(op)?;
    Ok(())
}

fn comp_insert(conn: &Connection, c: &Competency) -> Result<(), OrgError> {
    let affected = conn
        .execute(
            "INSERT OR IGNORE INTO competencies
                 (competency_id, code, description, scope, assigned_to, granted_by,
                  valid_from, valid_until)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                c.id.as_str(),
                c.code,
                c.description,
                c.scope,
                c.assigned_to.as_str(),
                c.granted_by.as_str(),
                date_to_str(c.valid_from),
                c.valid_until.map(date_to_str),
            ],
        )
        .map_err(op)?;
    if affected == 0 {
        return Err(OrgError::AlreadyExists(c.id.as_str().into()));
    }
    Ok(())
}

fn comp_update_occ(conn: &Connection, c: &Competency) -> Result<(), OrgError> {
    let affected = conn
        .execute(
            "UPDATE competencies SET
                 code=?2, description=?3, scope=?4, assigned_to=?5, granted_by=?6,
                 valid_from=?7, valid_until=?8, version=version+1
             WHERE competency_id=?1 AND version=?9",
            params![
                c.id.as_str(),
                c.code,
                c.description,
                c.scope,
                c.assigned_to.as_str(),
                c.granted_by.as_str(),
                date_to_str(c.valid_from),
                c.valid_until.map(date_to_str),
                c.version as i64,
            ],
        )
        .map_err(op)?;
    if affected == 0 {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM competencies WHERE competency_id=?1",
                params![c.id.as_str()],
                |r| r.get(0),
            )
            .map_err(op)?;
        return if exists > 0 {
            Err(OrgError::VersionConflict(c.id.as_str().into()))
        } else {
            Err(OrgError::CompetencyNotFound(c.id.as_str().into()))
        };
    }
    Ok(())
}

fn deleg_upsert(conn: &Connection, d: &Delegation) -> Result<(), OrgError> {
    conn.execute(
        "INSERT INTO delegations
             (delegation_id, competency_id, from_position, to_position,
              instrument_id, valid_from, valid_until)
         VALUES (?1,?2,?3,?4,?5,?6,?7)
         ON CONFLICT(delegation_id) DO UPDATE SET
             competency_id=excluded.competency_id,
             from_position=excluded.from_position,
             to_position=excluded.to_position,
             instrument_id=excluded.instrument_id,
             valid_from=excluded.valid_from, valid_until=excluded.valid_until",
        params![
            d.id.as_str(),
            d.competency_id.as_str(),
            d.from_position.as_str(),
            d.to_position.as_str(),
            d.instrument_id.as_str(),
            date_to_str(d.valid_from),
            d.valid_until.map(date_to_str),
        ],
    )
    .map_err(op)?;
    Ok(())
}

fn deleg_insert(conn: &Connection, d: &Delegation) -> Result<(), OrgError> {
    let affected = conn
        .execute(
            "INSERT OR IGNORE INTO delegations
                 (delegation_id, competency_id, from_position, to_position,
                  instrument_id, valid_from, valid_until)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                d.id.as_str(),
                d.competency_id.as_str(),
                d.from_position.as_str(),
                d.to_position.as_str(),
                d.instrument_id.as_str(),
                date_to_str(d.valid_from),
                d.valid_until.map(date_to_str),
            ],
        )
        .map_err(op)?;
    if affected == 0 {
        return Err(OrgError::AlreadyExists(d.id.as_str().into()));
    }
    Ok(())
}

fn deleg_update_occ(conn: &Connection, d: &Delegation) -> Result<(), OrgError> {
    let affected = conn
        .execute(
            "UPDATE delegations SET
                 competency_id=?2, from_position=?3, to_position=?4,
                 instrument_id=?5, valid_from=?6, valid_until=?7, version=version+1
             WHERE delegation_id=?1 AND version=?8",
            params![
                d.id.as_str(),
                d.competency_id.as_str(),
                d.from_position.as_str(),
                d.to_position.as_str(),
                d.instrument_id.as_str(),
                date_to_str(d.valid_from),
                d.valid_until.map(date_to_str),
                d.version as i64,
            ],
        )
        .map_err(op)?;
    if affected == 0 {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM delegations WHERE delegation_id=?1",
                params![d.id.as_str()],
                |r| r.get(0),
            )
            .map_err(op)?;
        return if exists > 0 {
            Err(OrgError::VersionConflict(d.id.as_str().into()))
        } else {
            Err(OrgError::DelegationNotFound(d.id.as_str().into()))
        };
    }
    Ok(())
}

// ── Outbox transaccional + drain idempotente ──────────────────────────────────

impl OrgSqliteStore {
    /// Executa uma escrita de estado e enfileira o evento de auditoria e (se
    /// presente) o evento de domínio nos outboxes, tudo numa transacção `IMMEDIATE`.
    fn audited_tx<F>(
        &self,
        event: &OrgAuditEvent,
        domain: Option<&OrgDomainEvent>,
        state_write: F,
    ) -> Result<(), OrgError>
    where
        F: FnOnce(&Connection) -> Result<(), OrgError>,
    {
        let mut conn = self.lock()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(op)?;
        state_write(&tx)?;
        outbox_insert(&tx, event)?;
        if let Some(d) = domain {
            domain_outbox_insert(&tx, d)?;
        }
        tx.commit().map_err(op)?;
        Ok(())
    }

    /// Lógica de drenagem partilhada pelos dois outboxes.
    ///
    /// Resiliente a *poison messages*: uma mensagem que falha vê o seu contador
    /// `attempts` incrementado e, ao atingir `MAX_OUTBOX_ATTEMPTS`, é movida para
    /// dead-letter (`delivered = 2`) — **sem bloquear** as restantes. A ordem de
    /// entrega não é crítica (a sequência da cadeia de hashes é atribuída no
    /// destino, não aqui).
    fn drain_table<F>(&self, table: &str, mut deliver: F) -> Result<usize, OrgError>
    where
        F: FnMut(&str) -> Result<(), OrgError>,
    {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT seq, event_json, attempts FROM {table}
                 WHERE delivered = 0 ORDER BY seq"
            ))
            .map_err(op)?;
        let pending: Vec<(i64, String, i64)> = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .map_err(op)?
            .collect::<Result<_, _>>()
            .map_err(op)?;
        drop(stmt);

        let mut delivered = 0usize;
        for (seq, json, attempts) in pending {
            match deliver(&json) {
                // Entregue, ou já presente no destino (idempotência) → marca entregue.
                Ok(()) | Err(OrgError::AlreadyExists(_)) => {
                    conn.execute(
                        &format!("UPDATE {table} SET delivered = 1 WHERE seq = ?1"),
                        params![seq],
                    )
                    .map_err(op)?;
                    delivered += 1;
                }
                // Falha de entrega: incrementa tentativas; dead-letter ao esgotar.
                // Continua (não bloqueia a fila numa mensagem envenenada).
                Err(e) => {
                    let next = attempts + 1;
                    let delivered_flag = if next >= MAX_OUTBOX_ATTEMPTS { 2 } else { 0 };
                    conn.execute(
                        &format!(
                            "UPDATE {table} SET attempts = ?1, delivered = ?2, last_error = ?3
                             WHERE seq = ?4"
                        ),
                        params![next, delivered_flag, e.to_string(), seq],
                    )
                    .map_err(op)?;
                }
            }
        }
        Ok(delivered)
    }

    fn count_where(&self, table: &str, delivered: i64) -> Result<u64, OrgError> {
        let conn = self.lock()?;
        let n: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE delivered = ?1"),
                params![delivered],
                |r| r.get(0),
            )
            .map_err(op)?;
        Ok(n as u64)
    }
}

impl OrgAuditOutbox for OrgSqliteStore {
    fn enqueue_audit(&self, event: &OrgAuditEvent) -> Result<(), OrgError> {
        let conn = self.lock()?;
        outbox_insert(&conn, event)
    }

    fn drain_audit_outbox(&self, audit: &dyn OrgAuditPort) -> Result<usize, OrgError> {
        self.drain_table("org_audit_outbox", |json| {
            let event: OrgAuditEvent = serde_json::from_str(json).map_err(op)?;
            audit.record(&event)
        })
    }

    fn drain_domain_outbox(&self, events: &dyn OrgDomainEventPort) -> Result<usize, OrgError> {
        self.drain_table("org_domain_outbox", |json| {
            let event: OrgDomainEvent = serde_json::from_str(json).map_err(op)?;
            events.publish(event)
        })
    }

    fn pending_audit_count(&self) -> Result<u64, OrgError> {
        self.count_where("org_audit_outbox", 0)
    }

    fn dead_letter_audit_count(&self) -> Result<u64, OrgError> {
        self.count_where("org_audit_outbox", 2)
    }

    fn pending_domain_count(&self) -> Result<u64, OrgError> {
        self.count_where("org_domain_outbox", 0)
    }

    fn dead_letter_domain_count(&self) -> Result<u64, OrgError> {
        self.count_where("org_domain_outbox", 2)
    }
}

// ── LegalInstrumentRepository ─────────────────────────────────────────────────

impl LegalInstrumentRepository for OrgSqliteStore {
    fn get(&self, id: &LegalInstrumentId) -> Result<Option<LegalInstrument>, OrgError> {
        let conn = self.lock()?;
        let row = conn
            .query_row(
                "SELECT instrument_id, kind, reference, date, description,
                    effective_from, effective_until
                 FROM legal_instruments WHERE instrument_id = ?1",
                params![id.as_str()],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, String>(5)?,
                        r.get::<_, Option<String>>(6)?,
                    ))
                },
            )
            .optional()
            .map_err(op)?;
        let Some((id_s, kind_s, reference, date_s, description, from_s, until_s)) = row else {
            return Ok(None);
        };
        Ok(Some(LegalInstrument {
            id: LegalInstrumentId(id_s),
            kind: str_to_kind(&kind_s).map_err(op)?,
            reference,
            date: str_to_date(&date_s).map_err(op)?,
            description,
            effective_from: str_to_date(&from_s).map_err(op)?,
            effective_until: opt_str_to_date(until_s).map_err(op)?,
        }))
    }

    fn list(&self) -> Result<Vec<LegalInstrument>, OrgError> {
        self.query_instruments("ORDER BY effective_from")
    }

    fn list_effective_at(&self, date: NaiveDate) -> Result<Vec<LegalInstrument>, OrgError> {
        let date_s = date_to_str(date);
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT instrument_id, kind, reference, date, description,
                    effective_from, effective_until
             FROM legal_instruments
             WHERE effective_from <= ?1 AND (effective_until IS NULL OR effective_until > ?1)
             ORDER BY effective_from",
            )
            .map_err(op)?;
        let rows = stmt
            .query_map(params![date_s], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, Option<String>>(6)?,
                ))
            })
            .map_err(op)?;
        self.map_instrument_rows(rows)
    }

    fn save(&self, i: &LegalInstrument) -> Result<(), OrgError> {
        i.validate()?;
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO legal_instruments
                 (instrument_id, kind, reference, date, description, effective_from, effective_until)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(instrument_id) DO UPDATE SET
                 kind=excluded.kind, reference=excluded.reference,
                 date=excluded.date, description=excluded.description,
                 effective_from=excluded.effective_from,
                 effective_until=excluded.effective_until",
            params![
                i.id.as_str(), kind_to_str(&i.kind), i.reference,
                date_to_str(i.date), i.description,
                date_to_str(i.effective_from), i.effective_until.map(date_to_str),
            ],
        ).map_err(op)?;
        Ok(())
    }
}

impl OrgSqliteStore {
    fn query_instruments(&self, suffix: &str) -> Result<Vec<LegalInstrument>, OrgError> {
        let conn = self.lock()?;
        let sql = format!(
            "SELECT instrument_id, kind, reference, date, description, effective_from, effective_until
             FROM legal_instruments {suffix}"
        );
        let mut stmt = conn.prepare(&sql).map_err(op)?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, Option<String>>(6)?,
                ))
            })
            .map_err(op)?;
        self.map_instrument_rows(rows)
    }

    fn map_instrument_rows(
        &self,
        rows: impl Iterator<
            Item = Result<
                (
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    Option<String>,
                ),
                rusqlite::Error,
            >,
        >,
    ) -> Result<Vec<LegalInstrument>, OrgError> {
        let mut result = Vec::new();
        for row in rows {
            let (id_s, kind_s, reference, date_s, description, from_s, until_s) =
                row.map_err(op)?;
            result.push(LegalInstrument {
                id: LegalInstrumentId(id_s),
                kind: str_to_kind(&kind_s).map_err(op)?,
                reference,
                date: str_to_date(&date_s).map_err(op)?,
                description,
                effective_from: str_to_date(&from_s).map_err(op)?,
                effective_until: opt_str_to_date(until_s).map_err(op)?,
            });
        }
        Ok(result)
    }
}

// ── OrgUnitRepository ─────────────────────────────────────────────────────────

impl OrgUnitRepository for OrgSqliteStore {
    fn get(&self, id: &OrgUnitId) -> Result<Option<OrgUnit>, OrgError> {
        let conn = self.lock()?;
        let row = conn
            .query_row(
                &format!("SELECT {UNIT_SELECT} FROM org_units WHERE unit_id = ?1"),
                params![id.as_str()],
                |r| Ok(read_unit_row!(r)),
            )
            .optional()
            .map_err(op)?;
        row.map(row_to_unit).transpose()
    }

    fn get_at_date(&self, id: &OrgUnitId, date: NaiveDate) -> Result<Option<OrgUnit>, OrgError> {
        let d = date_to_str(date);
        let conn = self.lock()?;
        let row = conn
            .query_row(
                &format!(
                    "SELECT {UNIT_SELECT} FROM org_units
                     WHERE unit_id = ?1 AND valid_from <= ?2
                       AND (valid_until IS NULL OR valid_until > ?2)"
                ),
                params![id.as_str(), d],
                |r| Ok(read_unit_row!(r)),
            )
            .optional()
            .map_err(op)?;
        row.map(row_to_unit).transpose()
    }

    fn list_active_at(&self, date: NaiveDate) -> Result<Vec<OrgUnit>, OrgError> {
        let d = date_to_str(date);
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {UNIT_SELECT} FROM org_units
             WHERE status = 'active' AND valid_from <= ?1
               AND (valid_until IS NULL OR valid_until > ?1)
             ORDER BY level, short_name"
            ))
            .map_err(op)?;
        let rows = stmt
            .query_map(params![d], |r| Ok(read_unit_row!(r)))
            .map_err(op)?;
        collect_units(rows)
    }

    fn list_by_level(&self, level: OrgLevel) -> Result<Vec<OrgUnit>, OrgError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {UNIT_SELECT} FROM org_units WHERE level = ?1 ORDER BY short_name"
            ))
            .map_err(op)?;
        let rows = stmt
            .query_map(params![level.as_u8() as i64], |r| Ok(read_unit_row!(r)))
            .map_err(op)?;
        collect_units(rows)
    }

    fn list_children(&self, parent_id: &OrgUnitId) -> Result<Vec<OrgUnit>, OrgError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {UNIT_SELECT} FROM org_units WHERE parent_id = ?1 ORDER BY short_name"
            ))
            .map_err(op)?;
        let rows = stmt
            .query_map(params![parent_id.as_str()], |r| Ok(read_unit_row!(r)))
            .map_err(op)?;
        collect_units(rows)
    }

    fn search_by_name(&self, term: &str, page: OrgPage) -> Result<PagedResult<OrgUnit>, OrgError> {
        let pattern = format!("%{term}%");
        let conn = self.lock()?;
        let total: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM org_units
                 WHERE status != 'extinct' AND (short_name LIKE ?1 OR full_name LIKE ?1)",
                params![pattern],
                |r| r.get::<_, i64>(0),
            )
            .map_err(op)? as u64;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {UNIT_SELECT} FROM org_units
             WHERE status != 'extinct' AND (short_name LIKE ?1 OR full_name LIKE ?1)
             ORDER BY level, short_name LIMIT ?2 OFFSET ?3"
            ))
            .map_err(op)?;
        let rows = stmt
            .query_map(
                params![pattern, page.limit as i64, page.offset as i64],
                |r| Ok(read_unit_row!(r)),
            )
            .map_err(op)?;
        let items = collect_units(rows)?;
        Ok(PagedResult::new(items, total, page))
    }

    fn hierarchy_at(&self, id: &OrgUnitId, date: NaiveDate) -> Result<Vec<OrgUnit>, OrgError> {
        let d = date_to_str(date);
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(&format!(
                "WITH RECURSIVE chain AS (
                 SELECT {UNIT_SELECT} FROM org_units
                 WHERE unit_id = ?1 AND valid_from <= ?2
                   AND (valid_until IS NULL OR valid_until > ?2)
                 UNION ALL
                 SELECT {UNIT_SELECT_U} FROM org_units u
                 INNER JOIN chain c ON u.unit_id = c.parent_id
                 WHERE u.valid_from <= ?2 AND (u.valid_until IS NULL OR u.valid_until > ?2)
             )
             SELECT * FROM chain ORDER BY level DESC"
            ))
            .map_err(op)?;
        let rows = stmt
            .query_map(params![id.as_str(), d], |r| Ok(read_unit_row!(r)))
            .map_err(op)?;
        collect_units(rows)
    }

    fn list_subtree(&self, root_id: &OrgUnitId, date: NaiveDate) -> Result<Vec<OrgUnit>, OrgError> {
        let d = date_to_str(date);
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(&format!(
                "WITH RECURSIVE subtree AS (
                 SELECT {UNIT_SELECT} FROM org_units
                 WHERE unit_id = ?1 AND valid_from <= ?2
                   AND (valid_until IS NULL OR valid_until > ?2)
                 UNION ALL
                 SELECT {UNIT_SELECT_U} FROM org_units u
                 INNER JOIN subtree s ON u.parent_id = s.unit_id
                 WHERE u.valid_from <= ?2 AND (u.valid_until IS NULL OR u.valid_until > ?2)
             )
             SELECT * FROM subtree ORDER BY level, short_name"
            ))
            .map_err(op)?;
        let rows = stmt
            .query_map(params![root_id.as_str(), d], |r| Ok(read_unit_row!(r)))
            .map_err(op)?;
        collect_units(rows)
    }

    fn full_tree_at(&self, date: NaiveDate) -> Result<Vec<OrgUnit>, OrgError> {
        let d = date_to_str(date);
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {UNIT_SELECT} FROM org_units
             WHERE status != 'extinct' AND valid_from <= ?1
               AND (valid_until IS NULL OR valid_until > ?1)
             ORDER BY level, short_name"
            ))
            .map_err(op)?;
        let rows = stmt
            .query_map(params![d], |r| Ok(read_unit_row!(r)))
            .map_err(op)?;
        collect_units(rows)
    }

    fn create(&self, u: &OrgUnit) -> Result<(), OrgError> {
        u.validate()?;
        let conn = self.lock()?;
        unit_insert(&conn, u)
    }

    fn update(&self, u: &OrgUnit) -> Result<(), OrgError> {
        u.validate()?;
        let conn = self.lock()?;
        unit_update_occ(&conn, u)
    }

    fn save(&self, u: &OrgUnit) -> Result<(), OrgError> {
        u.validate()?;
        let conn = self.lock()?;
        unit_upsert(&conn, u)
    }

    fn deactivate(&self, id: &OrgUnitId, valid_until: NaiveDate) -> Result<(), OrgError> {
        // Operação multi-passo: BEGIN IMMEDIATE garante atomicidade dos guardas.
        let mut conn = self.lock()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(op)?;
        unit_deactivate(&tx, id, valid_until)?;
        tx.commit().map_err(op)
    }

    fn create_audited(
        &self,
        u: &OrgUnit,
        event: &OrgAuditEvent,
        domain: Option<&OrgDomainEvent>,
    ) -> Result<(), OrgError> {
        u.validate()?;
        self.audited_tx(event, domain, |tx| unit_insert(tx, u))
    }

    fn update_audited(
        &self,
        u: &OrgUnit,
        event: &OrgAuditEvent,
        domain: Option<&OrgDomainEvent>,
    ) -> Result<(), OrgError> {
        u.validate()?;
        self.audited_tx(event, domain, |tx| unit_update_occ(tx, u))
    }

    fn save_audited(
        &self,
        u: &OrgUnit,
        event: &OrgAuditEvent,
        domain: Option<&OrgDomainEvent>,
    ) -> Result<(), OrgError> {
        u.validate()?;
        self.audited_tx(event, domain, |tx| unit_upsert(tx, u))
    }

    fn deactivate_audited(
        &self,
        id: &OrgUnitId,
        valid_until: NaiveDate,
        event: &OrgAuditEvent,
        domain: Option<&OrgDomainEvent>,
    ) -> Result<(), OrgError> {
        self.audited_tx(event, domain, |tx| unit_deactivate(tx, id, valid_until))
    }
}

// ── OrgPositionRepository ─────────────────────────────────────────────────────

impl OrgPositionRepository for OrgSqliteStore {
    fn get(&self, id: &OrgPositionId) -> Result<Option<OrgPosition>, OrgError> {
        let conn = self.lock()?;
        let row = conn
            .query_row(
                &format!("SELECT {POS_SELECT} FROM org_positions WHERE position_id=?1"),
                params![id.as_str()],
                |r| Ok(read_pos_row!(r)),
            )
            .optional()
            .map_err(op)?;
        row.map(row_to_position).transpose()
    }

    fn find_by_code(&self, code: &str) -> Result<Option<OrgPosition>, OrgError> {
        let conn = self.lock()?;
        let row = conn
            .query_row(
                &format!("SELECT {POS_SELECT} FROM org_positions WHERE code=?1"),
                params![code],
                |r| Ok(read_pos_row!(r)),
            )
            .optional()
            .map_err(op)?;
        row.map(row_to_position).transpose()
    }

    fn list_for_unit(&self, unit_id: &OrgUnitId) -> Result<Vec<OrgPosition>, OrgError> {
        self.query_positions("unit_id=?1 ORDER BY code", params![unit_id.as_str()])
    }

    fn list_for_unit_at(
        &self,
        unit_id: &OrgUnitId,
        date: NaiveDate,
    ) -> Result<Vec<OrgPosition>, OrgError> {
        let d = date_to_str(date);
        self.query_positions(
            "unit_id=?1 AND valid_from<=?2 AND (valid_until IS NULL OR valid_until>?2) ORDER BY code",
            params![unit_id.as_str(), d],
        )
    }

    fn list_by_kind(
        &self,
        kind: &PositionKind,
        date: NaiveDate,
    ) -> Result<Vec<OrgPosition>, OrgError> {
        let d = date_to_str(date);
        let k = pos_kind_to_str(kind);
        self.query_positions(
            "kind=?1 AND status='active' AND valid_from<=?2 AND (valid_until IS NULL OR valid_until>?2) ORDER BY code",
            params![k, d],
        )
    }

    fn list_for_unit_and_kind(
        &self,
        unit_id: &OrgUnitId,
        kind: &PositionKind,
        date: NaiveDate,
    ) -> Result<Vec<OrgPosition>, OrgError> {
        let d = date_to_str(date);
        let k = pos_kind_to_str(kind);
        self.query_positions(
            "unit_id=?1 AND kind=?2 AND status='active'
             AND valid_from<=?3 AND (valid_until IS NULL OR valid_until>?3) ORDER BY code",
            params![unit_id.as_str(), k, d],
        )
    }

    fn list_all_at(&self, date: NaiveDate) -> Result<Vec<OrgPosition>, OrgError> {
        let d = date_to_str(date);
        self.query_positions(
            "status='active' AND valid_from<=?1 AND (valid_until IS NULL OR valid_until>?1)
             ORDER BY unit_id, code",
            params![d],
        )
    }

    fn find_effective_substitute(
        &self,
        position_id: &OrgPositionId,
        date: NaiveDate,
    ) -> Result<Option<OrgPosition>, OrgError> {
        let d = date_to_str(date);
        let conn = self.lock()?;
        let row = conn
            .query_row(
                &format!(
                    "SELECT {POS_SELECT} FROM org_positions
                     WHERE substitutes=?1 AND status='active'
                       AND valid_from<=?2 AND (valid_until IS NULL OR valid_until>?2)
                     ORDER BY code LIMIT 1"
                ),
                params![position_id.as_str(), d],
                |r| Ok(read_pos_row!(r)),
            )
            .optional()
            .map_err(op)?;
        row.map(row_to_position).transpose()
    }

    fn create(&self, p: &OrgPosition) -> Result<(), OrgError> {
        p.validate()?;
        let conn = self.lock()?;
        pos_insert(&conn, p)
    }

    fn update(&self, p: &OrgPosition) -> Result<(), OrgError> {
        p.validate()?;
        let conn = self.lock()?;
        pos_update_occ(&conn, p)
    }

    fn save(&self, p: &OrgPosition) -> Result<(), OrgError> {
        p.validate()?;
        let conn = self.lock()?;
        pos_upsert(&conn, p)
    }

    fn deactivate(&self, id: &OrgPositionId, valid_until: NaiveDate) -> Result<(), OrgError> {
        let conn = self.lock()?;
        pos_deactivate(&conn, id, valid_until)
    }

    fn create_audited(
        &self,
        p: &OrgPosition,
        event: &OrgAuditEvent,
        domain: Option<&OrgDomainEvent>,
    ) -> Result<(), OrgError> {
        p.validate()?;
        self.audited_tx(event, domain, |tx| pos_insert(tx, p))
    }

    fn update_audited(
        &self,
        p: &OrgPosition,
        event: &OrgAuditEvent,
        domain: Option<&OrgDomainEvent>,
    ) -> Result<(), OrgError> {
        p.validate()?;
        self.audited_tx(event, domain, |tx| pos_update_occ(tx, p))
    }

    fn deactivate_audited(
        &self,
        id: &OrgPositionId,
        valid_until: NaiveDate,
        event: &OrgAuditEvent,
        domain: Option<&OrgDomainEvent>,
    ) -> Result<(), OrgError> {
        self.audited_tx(event, domain, |tx| pos_deactivate(tx, id, valid_until))
    }
}

impl OrgSqliteStore {
    fn query_positions(
        &self,
        where_order: &str,
        params: impl rusqlite::Params,
    ) -> Result<Vec<OrgPosition>, OrgError> {
        let conn = self.lock()?;
        let sql = format!("SELECT {POS_SELECT} FROM org_positions WHERE {where_order}");
        let mut stmt = conn.prepare(&sql).map_err(op)?;
        let rows = stmt
            .query_map(params, |r| Ok(read_pos_row!(r)))
            .map_err(op)?;
        collect_positions(rows)
    }
}

// ── CompetencyRepository ──────────────────────────────────────────────────────

const COMP_SELECT: &str = "competency_id, code, description, scope, assigned_to, \
     granted_by, valid_from, valid_until, version";

type CompRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    i64,
);

fn row_to_competency(r: CompRow) -> Result<Competency, OrgError> {
    let (id_s, code, description, scope, assigned_s, granted_s, from_s, until_s, ver) = r;
    Ok(Competency {
        id: CompetencyId(id_s),
        code,
        description,
        scope,
        assigned_to: OrgPositionId(assigned_s),
        granted_by: LegalInstrumentId(granted_s),
        valid_from: str_to_date(&from_s).map_err(op)?,
        valid_until: opt_str_to_date(until_s).map_err(op)?,
        version: ver as u32,
    })
}

macro_rules! read_comp_row {
    ($r:expr) => {{
        (
            $r.get::<_, String>(0)?,
            $r.get::<_, String>(1)?,
            $r.get::<_, String>(2)?,
            $r.get::<_, String>(3)?,
            $r.get::<_, String>(4)?,
            $r.get::<_, String>(5)?,
            $r.get::<_, String>(6)?,
            $r.get::<_, Option<String>>(7)?,
            $r.get::<_, i64>(8)?,
        )
    }};
}

impl CompetencyRepository for OrgSqliteStore {
    fn get(&self, id: &CompetencyId) -> Result<Option<Competency>, OrgError> {
        let conn = self.lock()?;
        let row = conn
            .query_row(
                &format!("SELECT {COMP_SELECT} FROM competencies WHERE competency_id=?1"),
                params![id.as_str()],
                |r| Ok(read_comp_row!(r)),
            )
            .optional()
            .map_err(op)?;
        row.map(row_to_competency).transpose()
    }

    fn list_for_position_at(
        &self,
        position_id: &OrgPositionId,
        date: NaiveDate,
    ) -> Result<Vec<Competency>, OrgError> {
        let d = date_to_str(date);
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {COMP_SELECT} FROM competencies
                 WHERE assigned_to=?1 AND valid_from<=?2
                   AND (valid_until IS NULL OR valid_until>?2)
                 ORDER BY code"
            ))
            .map_err(op)?;
        let rows = stmt
            .query_map(params![position_id.as_str(), d], |r| Ok(read_comp_row!(r)))
            .map_err(op)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row_to_competency(row.map_err(op)?)?);
        }
        Ok(result)
    }

    fn save(&self, c: &Competency) -> Result<(), OrgError> {
        c.validate()?;
        let conn = self.lock()?;
        comp_upsert(&conn, c)
    }

    fn update(&self, c: &Competency) -> Result<(), OrgError> {
        c.validate()?;
        let conn = self.lock()?;
        comp_update_occ(&conn, c)
    }

    fn create_audited(&self, c: &Competency, event: &OrgAuditEvent) -> Result<(), OrgError> {
        c.validate()?;
        self.audited_tx(event, None, |tx| comp_insert(tx, c))
    }

    fn update_audited(&self, c: &Competency, event: &OrgAuditEvent) -> Result<(), OrgError> {
        c.validate()?;
        self.audited_tx(event, None, |tx| comp_update_occ(tx, c))
    }
}

// ── DelegationRepository ──────────────────────────────────────────────────────

const DELEG_SELECT: &str = "delegation_id, competency_id, from_position, to_position, \
     instrument_id, valid_from, valid_until, version";

type DelegRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    i64,
);

fn row_to_delegation(r: DelegRow) -> Result<Delegation, OrgError> {
    let (id_s, comp_s, from_s, to_s, instr_s, vfrom_s, vuntil_s, ver) = r;
    Ok(Delegation {
        id: DelegationId(id_s),
        competency_id: CompetencyId(comp_s),
        from_position: OrgPositionId(from_s),
        to_position: OrgPositionId(to_s),
        instrument_id: LegalInstrumentId(instr_s),
        valid_from: str_to_date(&vfrom_s).map_err(op)?,
        valid_until: opt_str_to_date(vuntil_s).map_err(op)?,
        version: ver as u32,
    })
}

macro_rules! read_deleg_row {
    ($r:expr) => {{
        (
            $r.get::<_, String>(0)?,
            $r.get::<_, String>(1)?,
            $r.get::<_, String>(2)?,
            $r.get::<_, String>(3)?,
            $r.get::<_, String>(4)?,
            $r.get::<_, String>(5)?,
            $r.get::<_, Option<String>>(6)?,
            $r.get::<_, i64>(7)?,
        )
    }};
}

impl DelegationRepository for OrgSqliteStore {
    fn get(&self, id: &DelegationId) -> Result<Option<Delegation>, OrgError> {
        let conn = self.lock()?;
        let row = conn
            .query_row(
                &format!("SELECT {DELEG_SELECT} FROM delegations WHERE delegation_id=?1"),
                params![id.as_str()],
                |r| Ok(read_deleg_row!(r)),
            )
            .optional()
            .map_err(op)?;
        row.map(row_to_delegation).transpose()
    }

    fn get_effective_at(
        &self,
        to_position: &OrgPositionId,
        date: NaiveDate,
    ) -> Result<Vec<Delegation>, OrgError> {
        let d = date_to_str(date);
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {DELEG_SELECT} FROM delegations
                 WHERE to_position=?1 AND valid_from<=?2
                   AND (valid_until IS NULL OR valid_until>?2)"
            ))
            .map_err(op)?;
        let rows = stmt
            .query_map(params![to_position.as_str(), d], |r| Ok(read_deleg_row!(r)))
            .map_err(op)?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row_to_delegation(row.map_err(op)?)?);
        }
        Ok(result)
    }

    fn save(&self, d: &Delegation) -> Result<(), OrgError> {
        d.validate()?;
        let conn = self.lock()?;
        deleg_upsert(&conn, d)
    }

    fn update(&self, d: &Delegation) -> Result<(), OrgError> {
        d.validate()?;
        let conn = self.lock()?;
        deleg_update_occ(&conn, d)
    }

    fn create_audited(&self, d: &Delegation, event: &OrgAuditEvent) -> Result<(), OrgError> {
        d.validate()?;
        self.audited_tx(event, None, |tx| deleg_insert(tx, d))
    }

    fn update_audited(&self, d: &Delegation, event: &OrgAuditEvent) -> Result<(), OrgError> {
        d.validate()?;
        self.audited_tx(event, None, |tx| deleg_update_occ(tx, d))
    }
}

// ── OrgAuditAdapter ───────────────────────────────────────────────────────────

/// Implementa `OrgAuditPort` usando `core-audit`.
///
/// Para cada `OrgAuditEvent`:
/// 1. grava um `AuditEvent` na cadeia de hashes, com:
///    - `event_id` = identidade estável do evento (idempotência na reentrega);
///    - `event_type` = `"org.<entity_kind_lower>.<action>"`;
///    - `outcome` = `Success`/`Failure` (evidência COSO de sucesso **e** falha);
///    - `control_id` = controlo COSO primário;
///    - `details_json` = payload (snapshot ou contexto da falha);
/// 2. se um `ControlRegistryStore` estiver configurado e houver `control_id`,
///    grava também uma `ControlExecution` (`Passed`/`Failed`), com `execution_id`
///    determinístico (`<event_id>:<control_id>`) — idempotente na reentrega.
///
/// A reentrega de um evento já gravado (`DuplicateEvent` /
/// `DuplicateControlExecution`) é tratada como sucesso, suportando o drain
/// at-least-once do outbox sem duplicar evidência.
pub struct OrgAuditAdapter {
    store: Arc<dyn AuditStore>,
    controls: Option<Arc<dyn ControlRegistryStore>>,
}

impl OrgAuditAdapter {
    /// Apenas grava `AuditEvent` (sem `ControlExecution`).
    pub fn new(store: Arc<dyn AuditStore>) -> Self {
        Self {
            store,
            controls: None,
        }
    }

    /// Grava `AuditEvent` **e** `ControlExecution` no registo de controlos.
    pub fn with_controls(
        store: Arc<dyn AuditStore>,
        controls: Arc<dyn ControlRegistryStore>,
    ) -> Self {
        Self {
            store,
            controls: Some(controls),
        }
    }
}

impl OrgAuditPort for OrgAuditAdapter {
    fn record(&self, event: &OrgAuditEvent) -> Result<(), OrgError> {
        let event_type = format!(
            "org.{}.{}",
            event.entity_kind.to_lowercase().replace(' ', "_"),
            event.action.as_str(),
        );
        let outcome = match event.outcome {
            OrgEventOutcome::Success => AuditOutcome::Success,
            OrgEventOutcome::Failure => AuditOutcome::Failure,
        };
        let audit_event = AuditEvent::with_id_and_time(
            event.event_id.clone(),
            event_type,
            AuditActor {
                actor_id: event.actor.clone(),
                actor_type: Some("user".into()),
                actor_name: None,
            },
            AuditTarget {
                target_type: event.entity_kind.clone(),
                target_id: event.entity_id.clone(),
            },
            event.occurred_at,
            outcome,
            event.control_id.clone(),
            event.payload.clone(),
        )
        .map_err(|e| OrgError::OperationFailed(e.to_string()))?;

        match self.store.record(&audit_event) {
            Ok(()) => {}
            // Já gravado: idempotente (reentrega do outbox).
            Err(core_audit::AuditError::DuplicateEvent) => return Ok(()),
            Err(e) => return Err(OrgError::OperationFailed(e.to_string())),
        }

        if let (Some(controls), Some(control_id)) = (&self.controls, &event.control_id) {
            let result = if event.outcome.is_success() {
                ControlExecutionResult::Passed
            } else {
                ControlExecutionResult::Failed
            };
            let execution = ControlExecution::with_id_and_time(
                format!("{}:{}", event.event_id, control_id),
                control_id.clone(),
                event.event_id.clone(),
                event.occurred_at,
                result,
                None,
                None,
            )
            .map_err(|e| OrgError::OperationFailed(e.to_string()))?;
            match controls.record_execution(&execution) {
                Ok(()) | Err(core_audit::AuditError::DuplicateControlExecution) => {}
                Err(e) => return Err(OrgError::OperationFailed(e.to_string())),
            }
        }
        Ok(())
    }
}

// ── Catálogo de controlos COSO do domínio org ─────────────────────────────────

/// Controlos COSO específicos do domínio orgânico, referenciados pelas operações
/// de `core-org` (módulo `core_org::controls`). Distintos do catálogo transversal
/// de 50 controlos (`core_audit::builtin_control_catalog`) — controlos de domínio
/// vivem no respectivo módulo. Uma aplicação deve registá-los no
/// `ControlRegistryService` no arranque, a par do catálogo base.
pub fn org_control_catalog() -> Vec<ControlDefinition> {
    use core_org::controls::*;
    let valid_from = Utc
        .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
        .single()
        .expect("valid catalog date");
    let def = |id: &str, name: &str, description: &str, category, severity| ControlDefinition {
        control_id: id.to_string(),
        name: name.to_string(),
        description: Some(description.to_string()),
        category,
        severity,
        owner: None,
        implemented_by: vec!["@core-org".to_string()],
        references: vec!["COSO".to_string()],
        version: "1.0.0".to_string(),
        valid_from,
        valid_to: None,
        active: true,
    };
    vec![
        def(
            CTRL_ORG_UNIT_CHANGE,
            "Alteração de unidade orgânica fundamentada",
            "Verifica que a criação ou alteração de uma unidade orgânica é fundamentada \
             por instrumento jurídico ou referência legal, com hierarquia consistente.",
            ControlCategory::Auth,
            ControlSeverity::High,
        ),
        def(
            CTRL_ORG_UNIT_LIFECYCLE,
            "Ciclo de vida de unidade orgânica controlado",
            "Verifica que a extinção/suspensão de uma unidade respeita a máquina de \
             estados e os guardas (sem filhos nem posições activas).",
            ControlCategory::Integrity,
            ControlSeverity::High,
        ),
        def(
            CTRL_ORG_POSITION_CHANGE,
            "Definição de cargo fundamentada",
            "Verifica que a criação/alteração de um cargo é fundamentada e não introduz \
             ciclos de substituição.",
            ControlCategory::Auth,
            ControlSeverity::High,
        ),
        def(
            CTRL_ORG_POSITION_LIFECYCLE,
            "Ciclo de vida de cargo controlado",
            "Verifica que a extinção/suspensão de um cargo respeita a máquina de estados.",
            ControlCategory::Integrity,
            ControlSeverity::Medium,
        ),
        def(
            CTRL_ORG_COMPETENCY,
            "Atribuição de competência fundamentada",
            "Verifica que a atribuição de competência (autoridade jurídica para actos) \
             a um cargo é fundamentada e temporalmente válida.",
            ControlCategory::Auth,
            ControlSeverity::High,
        ),
    ]
}

// ── Convenience façade ────────────────────────────────────────────────────────

impl OrgSqliteStore {
    pub fn list_active_at(&self, date: NaiveDate) -> Result<Vec<OrgUnit>, OrgError> {
        OrgUnitRepository::list_active_at(self, date)
    }
    pub fn list_by_level(&self, level: OrgLevel) -> Result<Vec<OrgUnit>, OrgError> {
        OrgUnitRepository::list_by_level(self, level)
    }
    pub fn list_children(&self, parent_id: &OrgUnitId) -> Result<Vec<OrgUnit>, OrgError> {
        OrgUnitRepository::list_children(self, parent_id)
    }
    pub fn list_subtree(
        &self,
        root_id: &OrgUnitId,
        date: NaiveDate,
    ) -> Result<Vec<OrgUnit>, OrgError> {
        OrgUnitRepository::list_subtree(self, root_id, date)
    }
    pub fn full_tree_at(&self, date: NaiveDate) -> Result<Vec<OrgUnit>, OrgError> {
        OrgUnitRepository::full_tree_at(self, date)
    }
    pub fn search_by_name(
        &self,
        term: &str,
        page: OrgPage,
    ) -> Result<PagedResult<OrgUnit>, OrgError> {
        OrgUnitRepository::search_by_name(self, term, page)
    }
    pub fn save_instrument(&self, i: &LegalInstrument) -> Result<(), OrgError> {
        LegalInstrumentRepository::save(self, i)
    }
    pub fn get_instrument(
        &self,
        id: &LegalInstrumentId,
    ) -> Result<Option<LegalInstrument>, OrgError> {
        LegalInstrumentRepository::get(self, id)
    }
    pub fn list_instruments_at(&self, date: NaiveDate) -> Result<Vec<LegalInstrument>, OrgError> {
        LegalInstrumentRepository::list_effective_at(self, date)
    }
    pub fn create_unit(&self, u: &OrgUnit) -> Result<(), OrgError> {
        OrgUnitRepository::create(self, u)
    }
    pub fn update_unit(&self, u: &OrgUnit) -> Result<(), OrgError> {
        OrgUnitRepository::update(self, u)
    }
    pub fn save_unit(&self, u: &OrgUnit) -> Result<(), OrgError> {
        OrgUnitRepository::save(self, u)
    }
    pub fn get_unit(&self, id: &OrgUnitId) -> Result<Option<OrgUnit>, OrgError> {
        OrgUnitRepository::get(self, id)
    }
    pub fn create_position(&self, p: &OrgPosition) -> Result<(), OrgError> {
        OrgPositionRepository::create(self, p)
    }
    pub fn update_position(&self, p: &OrgPosition) -> Result<(), OrgError> {
        OrgPositionRepository::update(self, p)
    }
    pub fn save_position(&self, p: &OrgPosition) -> Result<(), OrgError> {
        OrgPositionRepository::save(self, p)
    }
    pub fn get_position(&self, id: &OrgPositionId) -> Result<Option<OrgPosition>, OrgError> {
        OrgPositionRepository::get(self, id)
    }
    pub fn find_position_by_code(&self, code: &str) -> Result<Option<OrgPosition>, OrgError> {
        OrgPositionRepository::find_by_code(self, code)
    }
    pub fn find_effective_substitute(
        &self,
        position_id: &OrgPositionId,
        date: NaiveDate,
    ) -> Result<Option<OrgPosition>, OrgError> {
        OrgPositionRepository::find_effective_substitute(self, position_id, date)
    }
    pub fn list_all_positions_at(&self, date: NaiveDate) -> Result<Vec<OrgPosition>, OrgError> {
        OrgPositionRepository::list_all_at(self, date)
    }
    pub fn list_positions_for_unit_and_kind(
        &self,
        unit_id: &OrgUnitId,
        kind: &PositionKind,
        date: NaiveDate,
    ) -> Result<Vec<OrgPosition>, OrgError> {
        OrgPositionRepository::list_for_unit_and_kind(self, unit_id, kind, date)
    }
    pub fn save_competency(&self, c: &Competency) -> Result<(), OrgError> {
        CompetencyRepository::save(self, c)
    }
    pub fn save_delegation(&self, d: &Delegation) -> Result<(), OrgError> {
        DelegationRepository::save(self, d)
    }
    pub fn deactivate_unit(&self, id: &OrgUnitId, valid_until: NaiveDate) -> Result<(), OrgError> {
        OrgUnitRepository::deactivate(self, id, valid_until)
    }
    pub fn deactivate_position(
        &self,
        id: &OrgPositionId,
        valid_until: NaiveDate,
    ) -> Result<(), OrgError> {
        OrgPositionRepository::deactivate(self, id, valid_until)
    }
    pub fn get_effective_at(
        &self,
        to_position: &OrgPositionId,
        date: NaiveDate,
    ) -> Result<Vec<Delegation>, OrgError> {
        DelegationRepository::get_effective_at(self, to_position, date)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use core_org::{
        OrgAuditPort, OrgNoopAudit, OrgNoopDomainEvents, OrgPositionService, OrgUnitService,
    };
    use tempfile::NamedTempFile;

    fn store() -> OrgSqliteStore {
        let tmp = NamedTempFile::new().unwrap();
        OrgSqliteStore::open(&SqliteRelationalConfig::read_write_create(tmp.path())).unwrap()
    }

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn instr(id: &str) -> LegalInstrument {
        LegalInstrument {
            id: LegalInstrumentId(id.into()),
            kind: InstrumentKind::Portaria,
            reference: format!("Portaria {id}/2020"),
            date: date(2020, 1, 1),
            description: "teste".into(),
            effective_from: date(2020, 1, 1),
            effective_until: None,
        }
    }

    fn unit(id: &str) -> OrgUnit {
        OrgUnit {
            id: OrgUnitId(id.into()),
            short_name: format!("UN {id}"),
            full_name: format!("Unidade {id}"),
            service_code: None,
            level: OrgLevel::new(1).unwrap(),
            parent_id: None,
            created_by: None,
            legal_reference: None,
            valid_from: date(2020, 1, 1),
            valid_until: None,
            status: OrgUnitStatus::Active,
            contacts: OrgContacts::default(),
            version: 0,
        }
    }

    fn position(id: &str, unit_id: &str, instr_id: &str) -> OrgPosition {
        OrgPosition {
            id: OrgPositionId(id.into()),
            code: id.to_uppercase(),
            title: format!("Cargo {id}"),
            kind: PositionKind::Tecnico,
            substitutes: None,
            status: OrgPositionStatus::Active,
            unit_id: OrgUnitId(unit_id.into()),
            created_by: LegalInstrumentId(instr_id.into()),
            valid_from: date(2020, 1, 1),
            valid_until: None,
            version: 0,
        }
    }

    // ── Repositório directo ───────────────────────────────────────────────────

    #[test]
    fn unit_round_trip() {
        let s = store();
        s.save_unit(&unit("u1")).unwrap();
        let loaded = s.get_unit(&OrgUnitId("u1".into())).unwrap().unwrap();
        assert_eq!(loaded.version, 0);
    }

    #[test]
    fn unit_occ_update_ok() {
        let s = store();
        s.save_unit(&unit("u1")).unwrap();
        let fetched = s.get_unit(&OrgUnitId("u1".into())).unwrap().unwrap();
        let updated = OrgUnit {
            short_name: "Actualizado".into(),
            ..fetched
        };
        s.update_unit(&updated).unwrap();
        let reloaded = s.get_unit(&OrgUnitId("u1".into())).unwrap().unwrap();
        assert_eq!(reloaded.short_name, "Actualizado");
        assert_eq!(reloaded.version, 1);
    }

    #[test]
    fn unit_occ_version_conflict() {
        let s = store();
        s.save_unit(&unit("u1")).unwrap();
        let u1 = s.get_unit(&OrgUnitId("u1".into())).unwrap().unwrap();
        let u2 = u1.clone();
        s.update_unit(&u1).unwrap();
        assert!(matches!(
            s.update_unit(&u2).unwrap_err(),
            OrgError::VersionConflict(_)
        ));
    }

    #[test]
    fn deactivate_unit_atomico() {
        let s = store();
        s.save_unit(&unit("u1")).unwrap();
        s.deactivate_unit(&OrgUnitId("u1".into()), date(2025, 1, 1))
            .unwrap();
        let loaded = s.get_unit(&OrgUnitId("u1".into())).unwrap().unwrap();
        assert!(loaded.is_extinct());
    }

    #[test]
    fn position_round_trip() {
        let s = store();
        s.save_instrument(&instr("i1")).unwrap();
        s.save_unit(&unit("u1")).unwrap();
        s.save_position(&position("p1", "u1", "i1")).unwrap();
        let loaded = s
            .get_position(&OrgPositionId("p1".into()))
            .unwrap()
            .unwrap();
        assert_eq!(loaded.status, OrgPositionStatus::Active);
        assert_eq!(loaded.version, 0);
    }

    #[test]
    fn position_occ_conflict() {
        let s = store();
        s.save_instrument(&instr("i1")).unwrap();
        s.save_unit(&unit("u1")).unwrap();
        s.save_position(&position("p1", "u1", "i1")).unwrap();
        let p1 = s
            .get_position(&OrgPositionId("p1".into()))
            .unwrap()
            .unwrap();
        let p2 = p1.clone();
        s.update_position(&p1).unwrap();
        assert!(matches!(
            s.update_position(&p2).unwrap_err(),
            OrgError::VersionConflict(_)
        ));
    }

    #[test]
    fn find_position_by_code() {
        let s = store();
        s.save_instrument(&instr("i1")).unwrap();
        s.save_unit(&unit("u1")).unwrap();
        s.save_position(&position("p1", "u1", "i1")).unwrap();
        assert!(s.find_position_by_code("P1").unwrap().is_some());
        assert!(s.find_position_by_code("NAOEXISTE").unwrap().is_none());
    }

    #[test]
    fn list_all_positions_at() {
        let s = store();
        s.save_instrument(&instr("i1")).unwrap();
        s.save_unit(&unit("u1")).unwrap();
        s.save_unit(&unit("u2")).unwrap();

        let mut p1 = position("p1", "u1", "i1");
        p1.kind = PositionKind::Chefia;
        let mut p2 = position("p2", "u2", "i1");
        p2.kind = PositionKind::Adjunto;
        s.save_position(&p1).unwrap();
        s.save_position(&p2).unwrap();

        let all = s.list_all_positions_at(date(2025, 1, 1)).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn list_positions_for_unit_and_kind() {
        let s = store();
        s.save_instrument(&instr("i1")).unwrap();
        s.save_unit(&unit("u1")).unwrap();

        let mut chefe = position("chefe", "u1", "i1");
        chefe.kind = PositionKind::Chefia;
        let mut adj = position("adj", "u1", "i1");
        adj.kind = PositionKind::Adjunto;
        s.save_position(&chefe).unwrap();
        s.save_position(&adj).unwrap();

        let chefes = s
            .list_positions_for_unit_and_kind(
                &OrgUnitId("u1".into()),
                &PositionKind::Chefia,
                date(2025, 1, 1),
            )
            .unwrap();
        assert_eq!(chefes.len(), 1);
        assert_eq!(chefes[0].id.as_str(), "chefe");
    }

    #[test]
    fn find_effective_substitute() {
        let s = store();
        s.save_instrument(&instr("i1")).unwrap();
        s.save_unit(&unit("u1")).unwrap();

        let mut chefe = position("chefe", "u1", "i1");
        chefe.kind = PositionKind::Chefia;
        s.save_position(&chefe).unwrap();

        let mut adj = position("adj", "u1", "i1");
        adj.kind = PositionKind::Adjunto;
        adj.substitutes = Some(OrgPositionId("chefe".into()));
        s.save_position(&adj).unwrap();

        let sub = s
            .find_effective_substitute(&OrgPositionId("chefe".into()), date(2025, 1, 1))
            .unwrap();
        assert_eq!(sub.unwrap().id.as_str(), "adj");
    }

    #[test]
    fn hierarchy_at_returns_chain() {
        let s = store();
        let root = unit("root");
        let mut child = unit("child");
        child.level = OrgLevel::new(2).unwrap();
        child.parent_id = Some(OrgUnitId("root".into()));
        s.save_unit(&root).unwrap();
        s.save_unit(&child).unwrap();
        let chain = s
            .hierarchy_at(&OrgUnitId("child".into()), date(2025, 1, 1))
            .unwrap();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].id.as_str(), "child");
    }

    #[test]
    fn search_by_name_paginado() {
        let s = store();
        for i in 0..5u32 {
            let mut u = unit(&format!("u{i}"));
            u.full_name = format!("Serviço de Finanças {i}");
            s.save_unit(&u).unwrap();
        }
        let result = s.search_by_name("Finanças", OrgPage::first(3)).unwrap();
        assert_eq!(result.total, 5);
        assert_eq!(result.items.len(), 3);
        assert!(result.has_more());
    }

    // ── Serviços ──────────────────────────────────────────────────────────────

    fn unit_svc(
        s: OrgSqliteStore,
    ) -> OrgUnitService<OrgSqliteStore, impl OrgAuditPort, impl core_org::OrgDomainEventPort> {
        OrgUnitService::new(s, OrgNoopAudit, OrgNoopDomainEvents)
    }

    fn pos_svc(
        s: OrgSqliteStore,
    ) -> OrgPositionService<OrgSqliteStore, impl OrgAuditPort, impl core_org::OrgDomainEventPort>
    {
        OrgPositionService::new(s, OrgNoopAudit, OrgNoopDomainEvents)
    }

    #[test]
    fn service_create_sem_instrumento_rejeita() {
        let svc = unit_svc(store());
        let u = unit("u1");
        assert!(matches!(
            svc.create(u, "admin"),
            Err(OrgError::EmptyField(_))
        ));
    }

    #[test]
    fn service_create_com_legal_reference_ok() {
        let svc = unit_svc(store());
        let mut u = unit("u1");
        u.legal_reference = Some("Portaria n.º 1/2024".into());
        assert!(svc.create(u, "admin").is_ok());
    }

    #[test]
    fn service_deactivate_vai_por_maquina_de_estados() {
        let s = store();
        let mut u = unit("u1");
        u.legal_reference = Some("Port. 1/2024".into());
        s.save_unit(&u).unwrap();
        s.deactivate_unit(&u.id, date(2025, 1, 1)).unwrap();

        let svc = unit_svc(s);
        let err = svc
            .deactivate(&u.id, date(2025, 6, 1), "admin")
            .unwrap_err();
        assert!(matches!(err, OrgError::OperationFailed(_)));
    }

    #[test]
    fn service_suspend_e_reactivate() {
        let s = store();
        let mut u = unit("u1");
        u.legal_reference = Some("Port. 1/2024".into());
        s.save_unit(&u).unwrap();
        let svc = unit_svc(s);

        svc.suspend(&u.id, "admin").unwrap();
        let stored = OrgUnitRepository::get(&svc.repo, &u.id).unwrap().unwrap();
        assert_eq!(stored.status, OrgUnitStatus::Suspended);

        svc.reactivate(&u.id, "admin").unwrap();
        let stored2 = OrgUnitRepository::get(&svc.repo, &u.id).unwrap().unwrap();
        assert_eq!(stored2.status, OrgUnitStatus::Active);
    }

    #[test]
    fn position_service_detects_substitution_cycle() {
        let s = store();
        s.save_instrument(&instr("i1")).unwrap();
        s.save_unit(&unit("u1")).unwrap();

        let mut chefe = position("chefe", "u1", "i1");
        chefe.kind = PositionKind::Chefia;
        s.save_position(&chefe).unwrap();

        let mut adj = position("adj", "u1", "i1");
        adj.kind = PositionKind::Adjunto;
        adj.substitutes = Some(OrgPositionId("chefe".into()));
        s.save_position(&adj).unwrap();

        let svc = pos_svc(s);
        let mut chefe_update =
            OrgPositionRepository::get(&svc.repo, &OrgPositionId("chefe".into()))
                .unwrap()
                .unwrap();
        chefe_update.substitutes = Some(OrgPositionId("adj".into()));
        assert!(matches!(
            svc.update(chefe_update, "admin"),
            Err(OrgError::SubstitutionCycle)
        ));
    }

    #[test]
    fn position_service_suspend_e_reactivate() {
        let s = store();
        s.save_instrument(&instr("i1")).unwrap();
        s.save_unit(&unit("u1")).unwrap();
        s.save_position(&position("p1", "u1", "i1")).unwrap();

        let svc = pos_svc(s);
        svc.suspend(&OrgPositionId("p1".into()), "admin").unwrap();
        let stored = OrgPositionRepository::get(&svc.repo, &OrgPositionId("p1".into()))
            .unwrap()
            .unwrap();
        assert_eq!(stored.status, OrgPositionStatus::Suspended);

        svc.reactivate(&OrgPositionId("p1".into()), "admin")
            .unwrap();
        let stored2 = OrgPositionRepository::get(&svc.repo, &OrgPositionId("p1".into()))
            .unwrap()
            .unwrap();
        assert_eq!(stored2.status, OrgPositionStatus::Active);
    }

    #[test]
    fn store_is_clone_e_pode_ser_compartilhado() {
        let s = store();
        let s2 = s.clone(); // OrgSqliteStore: Clone
        let mut u = unit("u1");
        u.legal_reference = Some("Port. 1/2024".into());
        s.save_unit(&u).unwrap();
        // s2 pode ler a mesma base de dados
        assert!(s2.get_unit(&u.id).unwrap().is_some());
    }

    // ── OrgAuditAdapter end-to-end ────────────────────────────────────────────

    use core_audit::{
        AuditChainReport, AuditError, AuditEvent as CAuditEvent, AuditExportManifest, AuditTarget,
    };
    use std::sync::Mutex as StdMutex;

    /// Mock de AuditStore que captura os eventos registados.
    #[derive(Default)]
    struct CapturingAudit {
        events: StdMutex<Vec<CAuditEvent>>,
    }

    impl AuditStore for CapturingAudit {
        fn record(&self, event: &CAuditEvent) -> Result<(), AuditError> {
            self.events.lock().unwrap().push(event.clone());
            Ok(())
        }
        fn get(&self, _id: &str) -> Result<Option<CAuditEvent>, AuditError> {
            Ok(None)
        }
        fn list_by_actor(
            &self,
            _a: &str,
            _l: usize,
            _o: usize,
        ) -> Result<Vec<CAuditEvent>, AuditError> {
            Ok(self.events.lock().unwrap().clone())
        }
        fn list_by_target(
            &self,
            _t: &AuditTarget,
            _l: usize,
            _o: usize,
        ) -> Result<Vec<CAuditEvent>, AuditError> {
            Ok(self.events.lock().unwrap().clone())
        }
        fn list_all(&self, _l: usize, _o: usize) -> Result<Vec<CAuditEvent>, AuditError> {
            Ok(self.events.lock().unwrap().clone())
        }
        fn list_by_date_range(
            &self,
            _f: chrono::DateTime<chrono::Utc>,
            _t: chrono::DateTime<chrono::Utc>,
            _l: usize,
            _o: usize,
        ) -> Result<Vec<CAuditEvent>, AuditError> {
            Ok(self.events.lock().unwrap().clone())
        }
        fn verify_chain(&self) -> Result<AuditChainReport, AuditError> {
            Err(AuditError::OperationFailed)
        }
        fn verify_chain_since(&self, _s: u64) -> Result<AuditChainReport, AuditError> {
            Err(AuditError::OperationFailed)
        }
        fn verify_chain_from_checkpoint(
            &self,
            _s: u64,
            _h: &str,
        ) -> Result<AuditChainReport, AuditError> {
            Err(AuditError::OperationFailed)
        }
        fn export_manifest(&self) -> Result<AuditExportManifest, AuditError> {
            Err(AuditError::OperationFailed)
        }
    }

    #[test]
    fn audit_adapter_regista_evento_de_criacao() {
        let capturing = Arc::new(CapturingAudit::default());
        let adapter = OrgAuditAdapter::new(capturing.clone());

        let svc = OrgUnitService::new(store(), adapter, OrgNoopDomainEvents);
        let mut u = unit("u1");
        u.legal_reference = Some("Portaria n.º 1/2024".into());
        svc.create(u, "joao.silva").unwrap();

        let events = capturing.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        let ev = &events[0];
        assert_eq!(ev.event_type, "org.orgunit.created");
        assert_eq!(ev.actor.actor_id, "joao.silva");
        assert_eq!(ev.target.target_type, "OrgUnit");
        assert_eq!(ev.target.target_id, "u1");
        // payload presente com os dados da unidade
        let payload = ev.details_json.as_ref().unwrap();
        assert_eq!(payload["short_name"], "UN u1");
        assert_eq!(payload["level"], 1);
    }

    #[test]
    fn audit_adapter_regista_transicao_de_estado() {
        let capturing = Arc::new(CapturingAudit::default());
        let s = store();
        let mut u = unit("u1");
        u.legal_reference = Some("Port. 1/2024".into());
        s.save_unit(&u).unwrap();

        let svc = OrgUnitService::new(
            s,
            OrgAuditAdapter::new(capturing.clone()),
            OrgNoopDomainEvents,
        );
        svc.suspend(&u.id, "admin").unwrap();

        let events = capturing.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "org.orgunit.status_changed");
        assert_eq!(
            events[0].details_json.as_ref().unwrap()["status"],
            "suspended"
        );
    }

    // ── Gap 1: evidência de FALHA ─────────────────────────────────────────────

    #[test]
    fn falha_de_criacao_gera_evento_failure() {
        let capturing = Arc::new(CapturingAudit::default());
        let svc = OrgUnitService::new(
            store(),
            OrgAuditAdapter::new(capturing.clone()),
            OrgNoopDomainEvents,
        );
        // Sem created_by nem legal_reference → validate_strict falha.
        let err = svc.create(unit("u1"), "joao").unwrap_err();
        assert!(matches!(err, OrgError::EmptyField(_)));

        let events = capturing.events.lock().unwrap();
        assert_eq!(events.len(), 1, "a falha deve gerar evidência");
        assert_eq!(events[0].event_type, "org.orgunit.created");
        assert_eq!(events[0].outcome, AuditOutcome::Failure);
        // payload contém o código de erro
        let payload = events[0].details_json.as_ref().unwrap();
        assert_eq!(payload["error_code"], "MINI.ORG.EMPTY_FIELD");
    }

    // ── Gap 2: ControlExecution ───────────────────────────────────────────────

    #[derive(Default)]
    struct CapturingControls {
        executions: StdMutex<Vec<ControlExecution>>,
    }

    impl ControlRegistryStore for CapturingControls {
        fn define_control(&self, _d: &ControlDefinition) -> Result<(), AuditError> {
            Ok(())
        }
        fn get_control(&self, _id: &str) -> Result<Option<ControlDefinition>, AuditError> {
            Ok(None)
        }
        fn list_controls(
            &self,
            _l: usize,
            _o: usize,
        ) -> Result<Vec<ControlDefinition>, AuditError> {
            Ok(vec![])
        }
        fn list_controls_by_category(
            &self,
            _c: ControlCategory,
            _l: usize,
            _o: usize,
        ) -> Result<Vec<ControlDefinition>, AuditError> {
            Ok(vec![])
        }
        fn record_execution(&self, e: &ControlExecution) -> Result<(), AuditError> {
            self.executions.lock().unwrap().push(e.clone());
            Ok(())
        }
        fn list_executions_by_control(
            &self,
            _c: &str,
            _l: usize,
            _o: usize,
        ) -> Result<Vec<ControlExecution>, AuditError> {
            Ok(self.executions.lock().unwrap().clone())
        }
        fn list_executions_by_event(&self, _e: &str) -> Result<Vec<ControlExecution>, AuditError> {
            Ok(self.executions.lock().unwrap().clone())
        }
    }

    #[test]
    fn adapter_grava_control_execution_passed_em_sucesso() {
        let audit = Arc::new(CapturingAudit::default());
        let controls = Arc::new(CapturingControls::default());
        let svc = OrgUnitService::new(
            store(),
            OrgAuditAdapter::with_controls(audit.clone(), controls.clone()),
            OrgNoopDomainEvents,
        );
        let mut u = unit("u1");
        u.legal_reference = Some("Portaria 1/2024".into());
        svc.create(u, "joao").unwrap();

        let execs = controls.executions.lock().unwrap();
        assert_eq!(execs.len(), 1);
        assert_eq!(execs[0].control_id, "CTRL-ORG-UNIT-001");
        assert_eq!(execs[0].result, ControlExecutionResult::Passed);
        // a execução liga ao mesmo event_id do AuditEvent
        let events = audit.events.lock().unwrap();
        assert_eq!(execs[0].event_id, events[0].event_id);
    }

    #[test]
    fn adapter_grava_control_execution_failed_em_falha() {
        let audit = Arc::new(CapturingAudit::default());
        let controls = Arc::new(CapturingControls::default());
        let svc = OrgUnitService::new(
            store(),
            OrgAuditAdapter::with_controls(audit, controls.clone()),
            OrgNoopDomainEvents,
        );
        let _ = svc.create(unit("u1"), "joao").unwrap_err(); // falha validate_strict

        let execs = controls.executions.lock().unwrap();
        assert_eq!(execs.len(), 1);
        assert_eq!(execs[0].result, ControlExecutionResult::Failed);
    }

    // ── Gap 3: outbox transaccional sem perda de evidência ────────────────────

    /// Porto de auditoria que falha sempre a entrega.
    struct FailingAudit;
    impl OrgAuditPort for FailingAudit {
        fn record(&self, _e: &OrgAuditEvent) -> Result<(), OrgError> {
            Err(OrgError::OperationFailed("entrega indisponível".into()))
        }
    }

    #[test]
    fn outbox_preserva_evidencia_quando_entrega_falha() {
        let s = store();
        let mut u = unit("u1");
        u.legal_reference = Some("Port. 1/2024".into());
        let ev = OrgAuditEvent::new(
            "joao",
            core_org::OrgAuditAction::Created,
            "OrgUnit",
            "u1",
            chrono::Utc::now(),
            OrgEventOutcome::Success,
            Some("CTRL-ORG-UNIT-001".into()),
            None,
        );
        // Escrita atómica: estado + evento no outbox.
        OrgUnitRepository::create_audited(&s, &u, &ev, None).unwrap();

        // Estado persistido mesmo sem entrega.
        assert!(s.get_unit(&u.id).unwrap().is_some());
        // Evidência capturada e por entregar.
        assert_eq!(s.pending_audit_count().unwrap(), 1);

        // Entrega falha → evidência permanece (sem perda).
        let delivered = s.drain_audit_outbox(&FailingAudit).unwrap();
        assert_eq!(delivered, 0);
        assert_eq!(s.pending_audit_count().unwrap(), 1);

        // Entrega recupera → drena e marca entregue.
        let capturing = CapturingAudit::default();
        let adapter = OrgAuditAdapter::new(Arc::new(capturing));
        let delivered = s.drain_audit_outbox(&adapter).unwrap();
        assert_eq!(delivered, 1);
        assert_eq!(s.pending_audit_count().unwrap(), 0);

        // Idempotente: nova drenagem não reentrega.
        assert_eq!(s.drain_audit_outbox(&adapter).unwrap(), 0);
    }

    #[test]
    fn create_audited_falha_nao_deixa_estado_nem_evidencia() {
        let s = store();
        let dup = unit("dup");
        let ev = OrgAuditEvent::new(
            "x",
            core_org::OrgAuditAction::Created,
            "OrgUnit",
            "dup",
            chrono::Utc::now(),
            OrgEventOutcome::Success,
            None,
            None,
        );
        OrgUnitRepository::create_audited(&s, &dup, &ev, None).unwrap();
        assert_eq!(s.pending_audit_count().unwrap(), 1);

        // Segunda criação falha (AlreadyExists) → transação reverte, sem novo outbox.
        let err = OrgUnitRepository::create_audited(&s, &dup, &ev, None).unwrap_err();
        assert!(matches!(err, OrgError::AlreadyExists(_)));
        assert_eq!(
            s.pending_audit_count().unwrap(),
            1,
            "a falha não deve acrescentar evento ao outbox"
        );
    }

    // ── Gap 4: CompetencyService / DelegationService ──────────────────────────

    fn competency(id: &str, pos: &str, instr_id: &str) -> Competency {
        Competency {
            id: CompetencyId(id.into()),
            code: id.to_uppercase(),
            description: "Assinar".into(),
            scope: "Nível 1".into(),
            assigned_to: OrgPositionId(pos.into()),
            granted_by: LegalInstrumentId(instr_id.into()),
            valid_from: date(2020, 1, 1),
            valid_until: None,
            version: 0,
        }
    }

    #[test]
    fn competency_service_cria_com_auditoria() {
        use core_org::CompetencyService;
        let capturing = Arc::new(CapturingAudit::default());
        let s = store();
        s.save_instrument(&instr("i1")).unwrap();
        s.save_unit(&unit("u1")).unwrap();
        s.save_position(&position("p1", "u1", "i1")).unwrap();

        let svc = CompetencyService::new(s, OrgAuditAdapter::new(capturing.clone()));
        svc.create(competency("c1", "p1", "i1"), "admin").unwrap();

        let events = capturing.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "org.competency.created");
        assert_eq!(events[0].control_id.as_deref(), Some("CTRL-ORG-COMP-001"));
    }

    #[test]
    fn competency_service_occ_conflito() {
        use core_org::CompetencyService;
        let s = store();
        s.save_instrument(&instr("i1")).unwrap();
        s.save_unit(&unit("u1")).unwrap();
        s.save_position(&position("p1", "u1", "i1")).unwrap();
        s.save_competency(&competency("c1", "p1", "i1")).unwrap();

        let svc = CompetencyService::new(s, OrgNoopAudit);
        let c1 = competency("c1", "p1", "i1"); // version 0
        svc.update(c1.clone(), "admin").unwrap(); // → version 1
                                                  // segunda com version 0 → conflito
        assert!(matches!(
            svc.update(c1, "admin").unwrap_err(),
            OrgError::VersionConflict(_)
        ));
    }

    #[test]
    fn delegation_service_cria_com_verificacao_de_competencia() {
        use core_org::DelegationService;
        let s = store();
        s.save_instrument(&instr("i1")).unwrap();
        s.save_unit(&unit("u1")).unwrap();
        s.save_position(&position("pa", "u1", "i1")).unwrap();
        s.save_position(&position("pb", "u1", "i1")).unwrap();
        s.save_competency(&competency("c1", "pa", "i1")).unwrap();

        let deleg = Delegation {
            id: DelegationId("d1".into()),
            competency_id: CompetencyId("c1".into()),
            from_position: OrgPositionId("pa".into()),
            to_position: OrgPositionId("pb".into()),
            instrument_id: LegalInstrumentId("i1".into()),
            valid_from: date(2024, 1, 1),
            valid_until: None,
            version: 0,
        };
        let capturing = Arc::new(CapturingAudit::default());
        let svc = DelegationService::new(s, OrgAuditAdapter::new(capturing.clone()));

        let comp = CompetencyId("c1".into());
        // pa detém a competência c1 → ok
        svc.create(deleg.clone(), &[&comp], "admin").unwrap();
        assert_eq!(
            capturing.events.lock().unwrap()[0].event_type,
            "org.delegation.created"
        );

        // pa NÃO detém c2 → falha + evidência de falha
        let mut d2 = deleg;
        d2.id = DelegationId("d2".into());
        let other = CompetencyId("c2".into());
        let err = svc.create(d2, &[&other], "admin").unwrap_err();
        assert!(matches!(err, OrgError::OperationFailed(_)));
        let events = capturing.events.lock().unwrap();
        assert_eq!(events.last().unwrap().outcome, AuditOutcome::Failure);
    }

    // ── Gap 1: migração aditiva idempotente ───────────────────────────────────

    #[test]
    fn migracao_idempotente_e_com_colunas_novas() {
        let s = store();
        // Re-migrar é seguro (idempotente).
        s.migrate().unwrap();

        // Coluna version em competencies/delegations (M1 via ALTER) funciona.
        s.save_instrument(&instr("i1")).unwrap();
        s.save_unit(&unit("u1")).unwrap();
        s.save_position(&position("p1", "u1", "i1")).unwrap();
        s.save_competency(&competency("c1", "p1", "i1")).unwrap();
        assert_eq!(
            CompetencyRepository::get(&s, &CompetencyId("c1".into()))
                .unwrap()
                .unwrap()
                .version,
            0
        );
        // Tabelas de outbox (M1) existem.
        assert_eq!(s.pending_audit_count().unwrap(), 0);
        assert_eq!(s.pending_domain_count().unwrap(), 0);
        assert_eq!(s.dead_letter_audit_count().unwrap(), 0);
    }

    // ── Gap 2: dead-letter / poison message ───────────────────────────────────

    /// Falha a entrega de eventos cujo actor começa por "poison".
    struct SelectiveFailingAudit {
        captured: StdMutex<Vec<String>>,
    }
    impl OrgAuditPort for SelectiveFailingAudit {
        fn record(&self, e: &OrgAuditEvent) -> Result<(), OrgError> {
            if e.actor.starts_with("poison") {
                return Err(OrgError::OperationFailed("destino rejeita".into()));
            }
            self.captured.lock().unwrap().push(e.entity_id.clone());
            Ok(())
        }
    }

    fn audit_ev(actor: &str, id: &str) -> OrgAuditEvent {
        OrgAuditEvent::new(
            actor,
            core_org::OrgAuditAction::Created,
            "OrgUnit",
            id,
            chrono::Utc::now(),
            OrgEventOutcome::Success,
            None,
            None,
        )
    }

    #[test]
    fn poison_message_vai_para_dead_letter_sem_bloquear_a_fila() {
        let s = store();
        let port = SelectiveFailingAudit {
            captured: StdMutex::new(vec![]),
        };

        // Um envenenado (sempre falha) à frente de um saudável.
        s.enqueue_audit(&audit_ev("poison", "bad")).unwrap();
        s.enqueue_audit(&audit_ev("joao", "good")).unwrap();

        // Primeira drenagem: o saudável é entregue apesar do envenenado à frente.
        let delivered = s.drain_audit_outbox(&port).unwrap();
        assert_eq!(delivered, 1);
        assert_eq!(
            port.captured.lock().unwrap().as_slice(),
            &["good".to_string()]
        );
        assert_eq!(s.pending_audit_count().unwrap(), 1); // o envenenado continua pendente

        // Drenar até esgotar tentativas → dead-letter, sem nunca bloquear.
        for _ in 0..MAX_OUTBOX_ATTEMPTS {
            s.drain_audit_outbox(&port).unwrap();
        }
        assert_eq!(s.pending_audit_count().unwrap(), 0);
        assert_eq!(s.dead_letter_audit_count().unwrap(), 1);
    }

    // ── Gap 3: outbox de eventos de domínio (sem perda) ───────────────────────

    #[derive(Default)]
    struct CapturingDomain {
        events: StdMutex<Vec<OrgDomainEvent>>,
    }
    impl OrgDomainEventPort for CapturingDomain {
        fn publish(&self, event: OrgDomainEvent) -> Result<(), OrgError> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }
    }

    struct FailingDomain;
    impl OrgDomainEventPort for FailingDomain {
        fn publish(&self, _e: OrgDomainEvent) -> Result<(), OrgError> {
            Err(OrgError::OperationFailed("barramento indisponível".into()))
        }
    }

    #[test]
    fn servico_captura_e_entrega_evento_de_dominio() {
        let domain = Arc::new(CapturingDomain::default());
        let svc = OrgUnitService::new(store(), OrgNoopAudit, DomainArc(domain.clone()));
        let mut u = unit("u1");
        u.legal_reference = Some("Port. 1/2024".into());
        svc.create(u, "joao").unwrap();

        let events = domain.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], OrgDomainEvent::UnitCreated { .. }));
    }

    /// Wrapper para usar Arc<CapturingDomain> como OrgDomainEventPort por valor.
    struct DomainArc(Arc<CapturingDomain>);
    impl OrgDomainEventPort for DomainArc {
        fn publish(&self, event: OrgDomainEvent) -> Result<(), OrgError> {
            self.0.publish(event)
        }
    }

    #[test]
    fn evento_de_dominio_sem_perda_quando_publish_falha() {
        let s = store();
        let svc = OrgUnitService::new(s, OrgNoopAudit, FailingDomain);
        let mut u = unit("u1");
        u.legal_reference = Some("Port. 1/2024".into());
        // create tem sucesso (estado + outbox commitados); publish falha no deliver.
        svc.create(u.clone(), "joao").unwrap();

        // Estado persistido; evento de domínio retido (sem perda).
        assert!(OrgUnitRepository::get(&svc.repo, &u.id).unwrap().is_some());
        assert_eq!(svc.repo.pending_domain_count().unwrap(), 1);

        // Recuperação: drenar para um porto saudável entrega o evento.
        let capturing = CapturingDomain::default();
        let n = svc.repo.drain_domain_outbox(&capturing).unwrap();
        assert_eq!(n, 1);
        assert_eq!(svc.repo.pending_domain_count().unwrap(), 0);
    }

    // ── Gap 4: drainer supervisionado ─────────────────────────────────────────

    #[test]
    fn drainer_run_once_entrega_ambos_os_outboxes() {
        use core_org::OrgOutboxDrainer;
        let s = store();
        // Captura evidência + evento sem entregar (portos que falham no deliver inline).
        let mut u = unit("u1");
        u.legal_reference = Some("Port. 1/2024".into());
        let ev = audit_ev("joao", "u1");
        let domain = OrgDomainEvent::UnitUpdated {
            id: OrgUnitId("u1".into()),
        };
        s.save_unit(&u).unwrap();
        OrgUnitRepository::update_audited(&s, &u, &ev, Some(&domain)).unwrap();
        assert_eq!(s.pending_audit_count().unwrap(), 1);
        assert_eq!(s.pending_domain_count().unwrap(), 1);

        let drainer = OrgOutboxDrainer::new(s, OrgNoopAudit, CapturingDomain::default());
        let stats = drainer.run_once().unwrap();
        assert_eq!(stats.audit_delivered, 1);
        assert_eq!(stats.domain_delivered, 1);
        assert_eq!(stats.audit_pending, 0);
        assert_eq!(stats.domain_pending, 0);
        assert!(!stats.has_dead_letters());
    }
}
